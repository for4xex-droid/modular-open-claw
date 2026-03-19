# 🗺️ Ripple Map — 変更影響範囲マップ

> **目的**: ファイルを変更する前にこのマップを参照し、影響が波及する先を事前に把握する。  
> **ルール**: 新規ファイル追加・構造体変更時は必ずこのマップを更新すること。

> [!IMPORTANT]
> **警告抑制ポリシー (Preserve Intent)**:
> 現在、未使用コードやインポートによる警告を `#![allow(...)]` で抑制している。
> モジュールの削除や大規模なリファクタリング時、これらの属性が「隠れた依存関係」となってコンテキストの喪失を招かないよう注意すること。詳細は `AGENTS.md` および `ADR 007` を参照。

*最終更新: 2026-03-20*

---

## libs/shared（共通基盤 — 全レイヤーが依存）

### crypto.rs — `derive_biome_key`
| 影響先 | 理由 |
|---|---|
| `api-server` (biome.rs) | `send_message` / `list_messages` での暗号化・復号 |
| `libs/core` (autonomous.rs) | `BiomeMessage` の P2P 通信保護 |

### guardrails.rs — `BeggingSupervisor`
| 影響先 | 理由 |
|---|---|
| `libs/core` (autonomous.rs) | AI 出力（おねだり・ダークパターン）の検証・遮断 |
| `api-server` (main.rs) | AppState 経由での利用（将来層） |

### auth.rs (AiomeCustomClaims)
| 影響先 | 理由 |
|---|---|
| `libs/infrastructure` (auth.rs) | トークン検証時の返り値として使用 |
| `api-server` (auth.rs) | `AuthenticatedUser` ラッパーの保持対象 |

---

## libs/aiome-contracts（インターフェース定義）

### commerce.rs — `GiftEngine`, `GiftRequest`
| 影響先 | 理由 |
|---|---|
| `libs/infrastructure` (commerce/gift.rs) | `TremendousGiftEngine` の具象実装 |
| `api-server` (main.rs) | `AppState` フィールド定義 |
| `api-server` (biome.rs) | `AutonomousBiomeEngine` への依存注入 |
| `libs/core` (autonomous.rs) | `AutonomousBiomeEngine` でのギフト送信実行 |

---

## libs/infrastructure（I/O実装層）

### commerce/gift.rs — `TremendousGiftEngine`
| 影響先 | 理由 |
|---|---|
| `api-server` (main.rs) | `AppState` でのインスタンス化 |
| `api_integration_tests.rs` | テスト用 Dummy/Mock 構築 |

### compliance/quarantine.rs — `QuarantineStore`
| 影響先 | 理由 |
|---|---|
| `api-server` (main.rs) | `AppState` でのインスタンス化・DB初期化 |
| `api-server` (routes/avatar.rs) | 非安全アセットの検疫保存ロジック |
| `api_integration_tests.rs` | テスト用 `MockQuarantineStore` 構築 |

---

## libs/soul（ドメイン層 — infrastructure に依存しない）

### model.rs — `AgentSoul`, `Experience`
| 影響先 | 理由 |
|---|---|
| `pipeline.rs` | `process_experience` の引数・フィールド参照 |
| `samsara_engine.rs` (infra) | `rebirth` で AgentSoul を構築 |
| `soul_store.rs` (infra) | `save_soul`/`load_soul` で全フィールドを JSON 永続化 |
| `soul_pipeline_tests.rs` | AgentSoul を生成してテスト |
| `routes/soul.rs` (api) | SoulStatusResponse で soul フィールドを読む |
| `main.rs` (api) | Worker ループで AgentSoul を操作 |

> [!CAUTION]
> **AgentSoul にフィールドを追加した場合**: `soul_store.rs` の INSERT/SELECT SQL、`serde` のシリアライズ、`api_integration_tests.rs` の AppState 初期化を必ず同期。

### pipeline.rs — `SoulPipeline`, `evaluate_trigger`, `process_experience`
| 影響先 | 理由 |
|---|---|
| `soul_adapter.rs` (infra) | `SoulDomainAdapter` トレイト実装 |
| `samsara_engine.rs` (infra) | `SamsaraEngine` トレイト実装 |
| `main.rs` (api) | `pipeline.process_experience()` 呼び出し |
| `soul_pipeline_tests.rs` | 全9テスト |

> [!CAUTION]
> **`SoulDomainAdapter` / `SamsaraEngine` のシグネチャ変更**: infrastructure 側の impl を必ず同期。テストの DummyAdapter/DummyEngine も更新必須。

### defense.rs — `Defense`, `DefenseTrigger`, `DefenseAction`
| 影響先 | 理由 |
|---|---|
| `pipeline.rs` | `evaluate_trigger`, reflex defense 生成 |
| `soul_adapter.rs` (infra) | `execute_defense` でアクション別処理 |
| `soul_store.rs` (infra) | `defenses_json` として JSON 永続化 |

> [!WARNING]
> **Defense 構造体のフィールド追加**: `serde` のデフォルト値がないとDB復元が壊れる。`#[serde(default)]` を検討。

### attachment.rs — `AttachmentModel`, `AttachmentStyle`
| 影響先 | 理由 |
|---|---|
| `pipeline.rs` | `update_from_experience` 呼び出し |
| `soul_store.rs` (infra) | `attachment_json` として永続化、SoulSnapshot |
| `routes/soul.rs` (api) | `attachment_style` 表示 |
| `build_system_instructions` (api) | SoulSnapshot 経由で表示 |

### instinct.rs — `Instinct`, `InstinctRule`
| 影響先 | 理由 |
|---|---|
| `samsara_engine.rs` (infra) | `distill` が Instinct を返す |
| `soul_store.rs` (infra) | `instinct_json`, SoulSnapshot.prompt_fragment |
| `main.rs` (api) | フォールバック distill |
| `build_system_instructions` (api) | prompt_fragment 注入 |

> [!WARNING]
> **Instinct にフィールド追加**: pipeline.rs テスト内の DummyEngine.distill() と soul_store.rs の永続化を同期。

### engine.rs — `SamsaraEngine` トレイト定義
| 影響先 | 理由 |
|---|---|
| `samsara_engine.rs` (infra) | impl 本体 |
| `pipeline.rs` テスト | DummyEngine impl |

### adapter.rs — `SoulDomainAdapter` トレイト定義
| 影響先 | 理由 |
|---|---|
| `soul_adapter.rs` (infra) | CoreDomainAdapter impl |
| `pipeline.rs` テスト | DummyAdapter impl |

### somatic.rs — `SomaticMarker`
| 影響先 | 理由 |
|---|---|
| `model.rs` | AgentSoul のフィールド |
| `pipeline.rs` | 予測時のバイアス計算 |
| `soul_store.rs` (infra) | JSONシリアライズ化 |

### predictive.rs — `PredictiveModel`
| 影響先 | 理由 |
|---|---|
| `model.rs` | AgentSoul のフィールド |
| `soul_adapter.rs` (infra) | 予測精度の更新 |
| `samsara_engine.rs` (infra) | Rebirth 時にリセット |

### anamnesis.rs — `AnamnesisProfile`
| 影響先 | 理由 |
|---|---|
| `model.rs` | AgentSoul のメタ認知フィールド |
| `samsara_engine.rs` (infra) | LLM蒸留によるプロフィール生成・継承 |
| `soul_store.rs` (infra) | JSONシリアライズ化 |

### error.rs — `SoulError`
| 影響先 | 理由 |
|---|---|
| `soul` 全域 | Error トレイト実装 |
| `infrastructure` 全域 | Result<T, SoulError> |

### lib.rs — パブリックエクスポート
| 影響先 | 理由 |
|---|---|
| `api-server`, `infrastructure` | クレート外からのモジュール可視性 |

---

## libs/infrastructure（インフラ層）

### soul_store.rs — `SqliteSoulStore`, `SoulSnapshot`
| 影響先 | 理由 |
|---|---|
| `main.rs` (api) | AppState に保持、Worker ループで使用 |
| `routes/soul.rs` (api) | `load_soul` 呼び出し |
| `routes/agent.rs` (api) | `get_snapshot` 呼び出し |
| `stream.rs` (api) | `get_snapshot` 呼び出し |
| `api_integration_tests.rs` | AppState 初期化に必要 |

> [!CAUTION]
> **SoulSnapshot にフィールド追加**: `get_snapshot`, `save_soul` のキャッシュ更新、`load_into_cache`、`build_system_instructions` の 4箇所を全て同期。

### samsara_engine.rs — `DefaultSamsaraEngine`
| 影響先 | 理由 |
|---|---|
| `main.rs` (api) | パイプライン構築時に生成 |

### soul_adapter.rs — `CoreDomainAdapter`
| 影響先 | 理由 |
|---|---|
| `main.rs` (api) | パイプライン構築時に生成 |

---

## apps/api-server（API層）

### main.rs — `AppState`, Worker ループ, ルーティング
| 影響先 | 理由 |
|---|---|
| 全 routes/*.rs | `State<AppState>` を受け取る |
| `api_integration_tests.rs` | AppState の全フィールドを模倣 |
| `stream.rs` | SSE 処理で AppState を使用 |

> [!CAUTION]
> **AppState にフィールド追加**: `api_integration_tests.rs` の初期化を **必ず** 同期。これを忘れると E0063 コンパイルエラー。

### routes/agent.rs — `build_system_instructions`, `trigger_agent_chat`
| 影響先 | 理由 |
|---|---|
| `stream.rs` | `build_system_instructions` を直接呼び出し |
| `watchtower.rs` | `build_system_instructions` を直接呼び出し |

> [!WARNING]
> **`build_system_instructions` のシグネチャ変更**: stream.rs と watchtower.rs の 2箇所を必ず同期。

---

## ⚡ 頻出カスケードパターン（過去の教訓）

| パターン | 波及先 | 防止策 |
|---|---|---|
| AgentSoul フィールド追加 | soul_store SQL, tests, API | SQL の INSERT/SELECT を先に確認 |
| トレイトシグネチャ変更 | infrastructure impl + test Dummy | `cargo check -p soul -p infrastructure --tests` |
| AppState フィールド追加 | api_integration_tests.rs | テストの初期化ブロックを先に確認 |
| `build_system_instructions` 引数変更 | agent.rs, stream.rs, watchtower.rs | 3箇所を grep で確認してから変更 |
| Defense/Instinct 構造体変更 | JSON永続化 (serde) | `#[serde(default)]` の有無を確認 |
| ルートハンドラ(`pub async fn`) 追加 | `deep-scan.sh (CC-6)` | 全てのAPIハンドラに `_auth: crate::auth::Authenticated` を必須とする |
