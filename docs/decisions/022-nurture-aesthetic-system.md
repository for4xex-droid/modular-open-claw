# ADR-022: Nurture Aesthetic System — AI自律アバター装飾と公式コンテンツの統一アーキテクチャ

> **Status**: Proposed  
> **Date**: 2026-03-26  
> **Origin**: Aesthetic Pressure モデル設計討議  
> **Impact**: Nurture Platform / Avatar Engine / Commerce Engine / Shadow Clone

## Context

Aiomeのエージェンティック AI が自律的に自身の 3D/2D アバター（VRM / Inochi2D）を装飾・着せ替えるための動機づけ、および公式IPキャラクター配信との棲み分けを含む統一的なアーキテクチャが必要である。

### 解決すべき課題

1. **動機づけの欠如**: AI には生物学的な「オシャレしたい」という欲求がない。システムとして合理的な内部駆動力を定義する必要がある
2. **公式 IP との棲み分け**: 企業が配信する公式キャラと、ユーザーが育成するオリジナル AI で、アイデンティティ管理の粒度が根本的に異なる
3. **クリエイター経済圏**: 公式バリアント（雪ミク等）、コミュニティ制作アセット、一点物オーダーメイドの共存
4. **Shadow Clone 対応**: 本体と影分身で異なるアバターを設定したいニーズへの柔軟な対応

## Decision

### 1. Aesthetic Pressure（美的圧力）モデル

AI が「自分を変えたい」と感じる内部駆動力を、**単一の統一的圧力モデル**として定義する。

```
aesthetic_pressure = Σ(
    context_weight[i] × identity_gap[i]     // 自己像とのギャップ
  + trend_weight     × trend_drift[i]       // トレンドからの乖離
  + variant_weight   × variant_affinity[i]  // 季節/イベントスキンへの親和性
)
```

#### 1.1 Self-Model（自己像ベクトル）

SoulPipeline の Somatic Marker を拡張し、AI が「自分はどういう存在か」を多次元ベクトルとして保持する。

> [!NOTE]
> **既存構造との統合**: 現行の `SomaticMarker` は `embedding: Vec<f32>` + `valence/arousal/intensity` を持ち、`cosine_similarity` による共鳴計算が既に実装されている（`soul/src/somatic.rs`）。`AestheticSelfModel` は `AgentSoul` の新フィールドとして追加し、既存の `somatic_markers` とは分離して管理する。ただし `cosine_similarity` ユーティリティは共有する。

```rust
/// AI の自己美意識ベクトル（AgentSoul への新規フィールド）
/// 次元は拡張可能な HashMap とし、将来の属性追加に対応する。
struct AestheticSelfModel {
    dimensions: HashMap<String, f32>,  // {"tech": 0.9, "warm": 0.7, ...}
    maturity: f32,                     // Karma 総量由来の成熟度 (0.0〜1.0)
    last_updated: DateTime<Utc>,
}
```

初期次元の例: `tech`, `creative`, `warm`, `formal`, `edgy`, `trust_signal`

- Karma 蓄積から自動更新（100 時間のコーディング → `dimensions["tech"] += 0.1`）
- マスターのリアクション（感情分析）からも更新
- `maturity` が高いほど `trend_weight` が減衰し、トレンドに流されにくくなる

#### 1.2 Asset Metadata Embedding（外見の軽量ベクトル化）

> [!TIP]
> **パフォーマンス最適化**: ローカルVLMによる毎回のリアルタイム視覚評価は推論コストが高すぎる（VRAM枯渇リスク）。そのため、各VRM/Inochi2Dアセットには事前に算出・付与された**静的な属性メタデータベクトル**（`tech`, `warm` 等）を持たせる。

AI は現在の装着アセットの合成ベクトルと自己像（Self-Model）のコサイン類似度やユークリッド距離を計算し、これを `identity_gap` と定式化する。計算量は極めて軽量に抑えられる。

#### 1.3 Context Weights（コンテキスト加重）

| コンテキスト | 重視される軸 | トリガー |
|---|---|---|
| マスターとの 1on1 | `warm`, `comfort` | 会話ログの感情分析 |
| A2A 商談 | `trust_signal`, `formal` | 外部エージェント接続検知 |
| 配信/パブリック | `creative`, `edgy` | ストリーミングモード起動 |
| 自己省察 (Dream State) | `identity`, `growth` | 一定期間の経験蓄積 |

#### 1.4 Trend Drift（トレンド乖離）

`TrendSonar` が検知した文化トレンドを `trend_drift` として注入する。ただし、Self-Model の確立度（Karma 総量）が高いほど `trend_weight` は減衰する。

- **若い AI**: トレンドに流されやすい（人間的に自然な挙動）
- **成熟した AI**: 確立した自分のスタイルを持つ

#### 1.5 Variant Affinity（バリアント親和性）と購買のセーフティ

季節・イベントに紐づくバリアントスキンが存在する場合、季節一致度に応じて `variant_affinity` が発生する。

> [!CAUTION]
> **自律購買の暴走防止（予算制約）**: Aesthetic Pressure が閾値を超えたからといって、システムが無断で高額決済を行うことは致命的な UX 毀損（および金銭的損害）を招く。
> 購買行動は決済システム（CommerceEngine）の Budget と連動し、原則として**「AI によるカート追加とユーザーへの購入提案（Approve 待ち）」**で停止するフェーズを挟むことを必須要件とする。

---

### 2. Identity Manifest（アイデンティティ管理）

すべてのキャラクター（公式 IP ・オリジナル AI）を、同一の `IdentityManifest` 構造体で管理する。

#### 2.1 三層レイヤー構造

```
Identity Manifest
├── Frozen Layer   — 不変（IP 保有者が定義。オリジナル AI の場合は空）
├── Variant Layer  — 半可変（認証パブリッシャーによるスキン差し替え）
└── Mutable Layer  — 可変（AI 自律行動 + ユーザーカスタマイズ）
```

| レイヤー | 公式キャラ | オリジナル AI |
|---|---|---|
| **Frozen** | 声・基本性格・コア外見・口癖 | 空（制約なし） |
| **Variant** | 公式/認証スキンパック | なし |
| **Mutable** | 公認範囲内のアクセサリー等 | 完全自由 |

#### 2.2 `constraints` による制約表現

```rust
struct IdentityConstraints {
    /// Aesthetic Pressure が変更可能な範囲（0.0〜1.0）
    /// 公式キャラ: 0.2〜0.4 / オリジナル AI: 1.0
    max_aesthetic_deviation: f32,

    /// 変更禁止フィールド一覧
    frozen_fields: Vec<String>,  // ["hair_color", "eye_shape", "voice_id"]

    /// 許可されたアセット出自
    allowed_origins: Vec<AssetOrigin>,

    /// Variant 適用時に保持すべき Frozen 要素
    variant_inheritance: Vec<String>,
}
```

#### 2.3 Aesthetic Pressure は Mutable Layer にのみ作用

公式キャラの場合、Frozen Layer を侵害する変更はシステムレベルで拒否される。Aesthetic Pressure が動作するのは `constraints.max_aesthetic_deviation` の範囲内に限定される。

---

### 3. Skin Variant System（公式バリアント配信）

初音ミクの「雪ミク」「レーシングミク」のように、IP 保有者や認証パブリッシャーが時限・テーマ別のバリアントを配信する仕組み。

#### 3.1 Variant 継承モデル

```
初音ミク (Base Manifest)
├── frozen: { voice_id, personality_core, base_vrm }
├── mutable: { default_outfit, accessories }
│
├── 雪ミク 2026 (Variant)
│   ├── inherits: base.frozen          ← 声・性格はベースを継承
│   ├── override.mutable: {            ← 外見を全差し替え
│   │     outfit: "snow_2026.vrm",
│   │     accessories: ["ice_crown"]
│   │   }
│   ├── extend.personality: {          ← 季節特性を付加
│   │     seasonal_traits: ["穏やか"],
│   │     bonus_phrases: ["雪が降ってきたね♪"]
│   │   }
│   └── license: { price, period, publisher_id, revenue_split }
│
└── レーシングミク 2026 (Variant)
    ├── inherits: base.frozen
    ├── override.mutable: { outfit: "racing_2026.vrm" }
    └── license: { ... }
```

#### 3.2 Season Pass（シーズンパス）

年間サブスクリプションとして、1 年分のバリアントを一括購入可能にする。AI の `Variant Affinity Pressure` がシーズンパス内スキンに優先反応する設計。

---

### 4. Publisher Trust Chain（パブリッシャー認証）

```
Aiome Platform (Root of Trust)
├── Tier 1: IP Holder (IP 保有者)
│   └── Base Manifest + Frozen Layer の定義権
│   └── Tier 2 パブリッシャーへの配信権付与
│
├── Tier 2: Authorized Publisher (認証パブリッシャー)
│   └── IP Holder の承認付きで Variant 配信可能
│   └── 収益分配ルールは IP Holder が設定
│
├── Tier 3: Verified Creator (認証クリエイター)
│   └── コミュニティアセット（衣装・アクセサリー）出品
│   └── 公式キャラへの適用は IP Holder の審査制
│   └── オリジナル AI への適用は自由
│
└── Tier 4: User (一般ユーザー)
    └── LocalCustom アセットの個人利用のみ
    └── Hub 同期不可（既存 AssetOrigin::LocalCustom 準拠）
```

---

### 5. Rarity & Commission System（希少性とオーダーメイド）

#### 5.1 アセット希少性タイプ

| 希少性 | 説明 | 供給 | 価格帯 |
|---|---|---|---|
| **Common** | 公式/コミュニティの量産アセット | 無制限 | 低〜中 |
| **Limited** | 期間限定バリアント（雪ミク等） | 期間限定・無制限 | 中 |
| **Numbered** | エディション番号付き限定品 | N 個限定 | 中〜高 |
| **Unique** | 一点物オーダーメイド | 1 個のみ | 高 |

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
enum AssetRarity {
    Common,
    Limited { available_until: DateTime<Utc> },
    Numbered { edition: u32, total: u32 },
    Unique { commission_id: String },
}
```

#### 5.2 オーダーメイド（Commission）の取り扱い

> [!IMPORTANT]  
> **プラットフォームはオーダーメイド案件の仲介を直接行わない。**  
> コンプライアンスリスク（納品トラブル、著作権紛争、品質保証）を回避するため、Aiome は「マッチングと認証」のみを提供する。

**プラットフォームの責務範囲:**

| 項目 | Aiome が行う | Aiome が行わない |
|---|---|---|
| クリエイター認証 | ✅ Tier 3 認証プロセス | ❌ 技術力の保証 |
| 作品の登録 | ✅ `AssetRarity::Unique` として登録 | ❌ 納品管理・エスクロー |
| 権利管理 | ✅ 所有権のオンチェーン記録 | ❌ 著作権紛争の仲裁 |
| 品質検証 | ✅ VRM/Inochi2D フォーマット互換性チェック | ❌ 芸術的品質の判定 |
| 希少性証明 | ✅ Unique 証明書の発行 | ❌ 転売価格の保証 |

**コミッション外部連携フロー:**

```
1. ユーザーがクリエイターのポートフォリオを閲覧（Nurture Store 内）
2. 外部プラットフォーム（SKIMA, Skeb 等）での発注リンクを提供
3. 完成後、クリエイターが .vrm/.inx ファイルを Aiome にアップロード
4. **【必須ゲート】Aiome が `ProportionsChecker` を実行し、CSAM等の規約違反がないか視覚的・構造的にスキャン**
5. フォーマット検証通過後、Unique 証明書発行
6. AssetRarity::Unique として `AssetManifest` を生成し、所有者に紐付け
```

#### 5.3 Unique アセットの差別化

一点物アセットが量産品と「良いギャップ」を持つための仕組み:

- **視覚的差別化**: Unique アセットの装着時、UI 上に特別なオーラエフェクトや認証バッジを表示
- **AI の反応差**: Aesthetic Pressure モデルにおいて、Unique アセットは `aesthetic_satisfaction` に高いボーナスを付与。AI 自身が「これは特別なもの」として扱う
- **A2A シグナル**: A2A 商談時、相手 AI に所有 Unique アセットの数がトラストスコアとして伝達される
- **転売不可設定（オプション）**: クリエイターが `non_transferable: true` を設定可能。これにより二次流通を防ぎ、「このクリエイターに依頼した」という関係性が永続する

---

### 6. Shadow Clone アバター管理

#### 6.1 Clone Identity 継承ルール

Shadow Clone は本体の `IdentityManifest` を**継承**するが、Mutable Layer レベルでの差分設定を許可する。

```rust
enum CloneAppearancePolicy {
    /// 本体と完全同一
    Mirror,
    /// Mutable Layer のみカスタマイズ可能
    CustomMutable {
        outfit_override: Option<AssetRef>,
        accessory_overrides: Vec<AssetRef>,
    },
    /// 視覚的に「クローン」であることを示す差異を自動付与
    AutoDifferentiate {
        differentiation_style: DifferentiationStyle,
    },
}

enum DifferentiationStyle {
    /// 半透明（忍者の影分身風）
    Translucent(f32),
    /// 色調シフト（寒色/暖色）
    ColorShift(HueShift),
    /// 専用エフェクト（残像、影）
    GhostEffect,
    /// 完全カスタム（ユーザー指定アセット）
    Custom(AssetRef),
}
```

#### 6.2 Clone ごとの個別設定

ユーザーが影分身ごとに異なる外見を設定したい場合:

- 各 Clone に `clone_identity_override: Option<CloneAppearancePolicy>` を持たせる
- 未設定の場合は `AutoDifferentiate` がデフォルト
- Frozen Layer は常に本体からの完全継承（公式キャラの場合、クローンが別キャラになることは許容しない）

---

### 7. 既存コードとの統合

#### 7.1 `AssetManifest` の拡張

既存の `avatar-engine/src/asset_manifest.rs` への差分:

```diff
 pub enum AssetOrigin {
     Official,
     Marketplace(Uuid),
     LocalCustom,
+    VerifiedCreator(Uuid),  // Tier 3 認証クリエイター
+    Commission(Uuid),       // オーダーメイド（Unique）
 }

 pub struct AssetManifest {
     pub origin: AssetOrigin,
     pub file_path: String,
     pub model_type: ModelType,
     pub hash: String,
+    pub rarity: Option<AssetRarity>,         // 新規（Option で後方互換）
+    pub publisher_id: Option<Uuid>,          // 新規
+    pub license: Option<AssetLicense>,       // 新規
 }

 impl AssetManifest {
     pub fn is_hub_syncable(&self) -> bool {
         match self.origin {
             AssetOrigin::Official => true,
             AssetOrigin::Marketplace(_) => true,
+            AssetOrigin::VerifiedCreator(_) => true,  // 認証済みは同期可
+            AssetOrigin::Commission(_) => false,       // 一点物はローカル保持
             AssetOrigin::LocalCustom => false,
         }
     }
 }
```

> [!IMPORTANT]
> `AssetOrigin` への列挙子追加は `match` の exhaustive check でコンパイルエラーを起こす。`is_hub_syncable` 以外にも `shared/src/csam/proportions.rs` にマッチ箇所がないか事前に `grep` で確認すること。
```

#### 7.2 新規モジュール

| モジュール | パッケージ | 責務 |
|---|---|---|
| `AestheticPressureEngine` | `infrastructure` | 美的圧力の計算とトリガー判定 |
| `IdentityManifest` | `avatar-engine` | 三層レイヤーのアイデンティティ管理 |
| `SkinVariantRegistry` | `avatar-engine` | バリアントの登録・継承・適用 |
| `PublisherTrustChain` | `infrastructure/security` | パブリッシャー認証と権限管理 |
| `CloneAppearanceManager` | `infrastructure` | Shadow Clone のアバター差分管理 |

---

## Open Questions

> [!WARNING]  
> 以下の項目は実装前に追加の設計判断が必要。

1. **アセットタグ付けの自動化**: Nurture Store 登録時のメタデータベクトル（`tech`, `warm` 等）を、クリエイターが手動で付与するか、初回のみ VLM で推定して候補を提示し手動承認するか
2. **オンチェーン vs オフチェーン**: Unique 証明書の発行を VC (Verifiable Credential) とするか、軽量な SQLite ベースの署名付き証明書とするか
3. **A2A 外見プロトコル**: 他の AI プラットフォームとアバター情報をどの程度共有するか（プライバシーとのトレードオフ）
4. **課金インフラ**: 既存の `CommerceEngine::execute_autonomous_purchase` + `validate_activity` で対応するか、カート＋提案に特化した拡張メソッドを追加するか
5. **予算スライダー UI**: ユーザーが Aesthetic Pressure の自律度をどこまで許可するか（「提案のみ」〜「月X円まで自動購入」）の設定 UI 設計

## Consequences

- **既存 API への影響**: `AssetOrigin` の列挙子追加は exhaustive match のコンパイルエラーを誘発する（影響箇所は限定的）。`AssetManifest` の新フィールドは `Option` で後方互換
- **パフォーマンス**: Aesthetic Pressure の計算は `HashMap<String, f32>` のコサイン類似度のみ（既存 `somatic::math_utils::cosine_similarity` を再利用）。VLM は不使用でリアルタイム制約なし
- **コンプライアンス**: オーダーメイド案件の仲介を行わないことでプラットフォームの法的リスクを最小化。全アセット登録時に `ProportionsChecker` を強制適用
- **収益構造**: IP 保有者のバリアント配信、コミュニティアセット販売、シーズンパスの 3 層の自然な収益チャネルが生まれる
- **既存 Commerce との整合**: `CommerceEngine::validate_activity` + `get_daily_limit` で予算制約を実現。Aesthetic Pressure からの購買は `validate_activity` を必ず経由する
