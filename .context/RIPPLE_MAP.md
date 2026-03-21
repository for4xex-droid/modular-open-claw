# 🌊 Aiome Ripple Map (Phase 9)

このドキュメントは、Phase 9「サンドボックス強化」におけるコード変更の影響範囲（リップル効果）を追跡するためのものです。

## 1. コア依存チェーン

```mermaid
graph TD
    A[libs/infrastructure/src/security.rs] -->|BastionGuard| B[libs/infrastructure/src/skills/mod.rs]
    B -->|WasmSkillManager| C[libs/infrastructure/src/skill_arena.rs]
    B -->|Unit Tests| D[libs/infrastructure/src/skills/tests.rs]
    
    E[apps/api-server/src/router.rs] -->|Route Registration| F[apps/api-server/src/routes/voice.rs]
    E -->|Route Registration| G[apps/api-server/src/routes/avatar.rs]
```

## 2. 影響を受けるコンポーネント

### 🛡️ セキュリティ (security.rs)
- **変更内容**: `BastionGuard::safe_exec` に動的サンドボックス（gVisor/sandbox-exec）選択ロジックを追加。
- **波及効果**: 全てのシェル実行、WASMスキルのランタイム実行に影響。

### 🛠️ スキル管理 (skills/mod.rs)
- **変更内容**: `WasmSkillManager` がプロセス起動時に `BastionGuard` を介するように変更。
- **波及効果**: スキルの初期化、実行、バリデーションフローに影響。

### 🏟️ スキルアリーナ (skill_arena.rs)
- **変更内容**: なし（直接の変更は予定していないが、`WasmSkillManager` の動作変更によりテストが必要）。
- **確認事項**: 試合（Match）や評価（Evaluation）が制限された環境下で正しく動作するか。

### 🌐 API サーバー (router.rs)
- **変更内容**: ルーターを Public / Auth / High-Payload の3層に分離。`DefaultBodyLimit` を一部で無効化。
- **波及効果**: 全APIエンドポイントの認証・制限挙動に影響。

## 3. Phase 10 クリエイター機能の波及範囲

```mermaid
graph TD
    A[libs/core/src/expression/engine.rs] -->|TTS Provider| B[apps/api-server/src/routes/expression.rs]
    C[libs/core/src/lora/engine.rs] -->|LoRA Manager| D[apps/api-server/src/app_state.rs]
    D -->|State Injection| E[apps/api-server/src/router.rs]
    F[libs/infrastructure/src/soul_store.rs] -->|lora_hash storage| G[libs/soul/src/soul_pipeline.rs]
    H[libs/aiome-contracts/src/commerce.rs] -->|Stripe implementation| I[libs/infrastructure/src/mock_commerce.rs]
```

### 🎭 TTS / Expression (10.1a)
- **変更内容**: `ExpressionEngine` に TTS プロバイダー統合。
- **波及効果**: `routes/expression.rs`、UI の発話コンポーネントに影響。

### 🧠 LoRA / Soul (10.1b)
- **変更内容**: `LoraEngine` 新設、`SqliteSoulStore` スキーマ更新 (`lora_hash`)。
- **波及効果**: ソウル（人格）の生成・更新フロー、`AppState` 全体に影響。

### 💰 Voice DRM (10.2 / Expert Review Integrated)
- **変更内容**: 
    - `crypto.rs`: AES-256-GCM + Nonce 運用基盤の実装。
    - `abyss_voice_vault.rs`: `vault_keys` テーブルへの鍵永続化 (§CISO-1) とリストア実装。
    - `routes/voice.rs`: 暗号化パイプライン (100MB Limit) と Creator Auth (§SEC-4) 追加。
- **波及効果**: 
    - `main.rs`: `VoiceCoreDrm` の async 初期化に伴い起動フローが非同期化。
    - `router.rs`: `RequestBodyLimitLayer` によるメモリ DoS 防御の強化。
    - `contracts`: `VoiceKeyVault` trait の拡張。

### 🛡️ Voice DRM Expansion (Phase 11)
- **変更内容**:
    - `commerce_webhook.rs`: Stripe Webhook から `licenses` テーブルへの単一トランザクション化。
    - `registry.rs`: `licenses` 優先・Webhook フォールバックによる Dual-Read 所有権チェックと、Creator 所有判定の復元。
    - `audio_hasher.rs`: CSAM 検疫のための `tokio::task::spawn_blocking` を用いた音声ハッシュの実装とタイムアウト制御。
    - `abyss_voice_vault.rs`: `OnceCell` による Master Key 取得の遅延キャッシュ化。
    - `avatar-engine`: `LipSyncProvider` トレイトを新設し `VoiceKeyVault` から LipSync 責務を分離。
- **波及効果**:
    - `api_integration_tests.rs`: Voice DRM のラウンドトリップ（アップロード〜暗号化〜所有権検閲〜復号化）E2Eテストが追加され、システム全体の動作検証が確立。

```mermaid
graph TD
    A[crypto.rs] -->|AES-256-GCM| B[abyss_voice_vault.rs]
    B -->|Persistence| C[vault_keys table]
    A -->|Encryption| D[routes/voice.rs]
    D -->|Registration| E[VoiceCoreDrm]
    E -->|Lookup| B
    F[main.rs] -->|Async Init| E
    G[router.rs] -->|100MB Limit| D
```

### 🛡️ EKYC Persistence & Inochi2D Sync (Phase 14)
- **変更内容**: 
    - `ekyc_store.rs`: `EkycSessionStore` (SQLite) によるセッション ID の永続化。
    - `ekyc.rs`: `client_reference_id` ベースの Stripe フィルタリング実装。
    - `stream.rs`: SSE `avatar_expression` に `physics_override` フィールド追加と Resonance ブーストロジック実装。
    - `inochi2d.rs`: `AssetType::Inochi2D` 登録と `PathSandbox` 適用。
- **波及効果**: 
    - `main.rs`: `STRIPE_API_KEY` の Release ビルドにおける強制チェック (Fail-safe) 導入。
    - `router.rs`: `jwt_auth_middleware` の Inochi2D アップロードへの適用。
    - `api_integration_tests.rs`: eKYC セッション永続化と Inochi2D 物理同期情報の検証パスが確立。

```mermaid
graph TD
    A[Stripe Identity API] <-->|client_reference_id| B[StripeEkycEngine]
    B -->|session_id| C[EkycSessionStore]
    C -->|Persistence| D[ekyc_sessions table]
    E[AppState] -->|Injection| B
    E -->|Injection| C
    F[stream.rs] -->|Resonance| G[physics_override]
    G -->|SSE| H[Frontend / Inochi2D Sync]
    I[routes/inochi2d.rs] -->|PathSandbox| J[.inochi2d_assets/]
    I -->|Registry| K[AssetType::Inochi2D]
```

### 🛡️ Phase 16: EKYC Endpoints & Revenue Splitter
- **変更内容**: 
    - `auth.rs`: JWT バリデーションに `ekyc_verified` クレームの抽出を追加。
    - `gift.rs` / `commerce.rs`: `auth.ekyc_verified` による 403 Forbidden 遮断ロジックの追加（未確認ユーザーの経済活動ブロック）。
    - `splitter.rs`: Stripe Webhook から呼び出される `RevenueSplitter::split_revenue` (80/20分配) の実装と `revenue_splits` テーブルへの書き込み。
    - `commerce_webhook.rs`: Webhook 受信時のトランザクション内で Revenue Splitting → License Granting を一貫実行。
    - `main.rs`: 起動時に環境変数 (`STRIPE_API_KEY`, `JWT_PRIVATE_KEY_B64`, `SEARCH_API_KEY` 等) を読み込み直後に `std::env::remove_var` で即時消去 (Zeroize) するセキュリティ強化。
- **波及効果**: 
    - ユーザーは eKYC 完了までギフト送信とアセット購入が不可能になる。
    - クリエイターとプラットフォームの売上分配が自動化され、データベーストランザクションにて一貫性が担保される。

```mermaid
graph TD
    A[Stripe Webhook] -->|checkout.session.completed| B[commerce_webhook.rs]
    B -->|Transaction Start| C[RevenueSplitter]
    C -->|80/20 Split| D[revenue_splits]
    C -->|Grant| E[licenses]
    B -->|Transaction Commit| F[Successful DB Write]
```

---
*最終更新日: 2026-03-21* (Phase 16)
