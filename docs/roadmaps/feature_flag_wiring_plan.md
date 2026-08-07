# Feature Flag 結線実装計画 — seo_publish / federation_v1_5（v1.0）

- **ステータス**: Implemented（2026-08-08）
- **作成日**: 2026-08-08
- **由来**: [research_mechanism_triage.md](research_mechanism_triage.md) の「結線 vs 削除」検証（両方とも結線が有利と判定）
- **対象**: (1) `seo_publish` — WordPress 自動投稿のユーザー制御ゲート新設（既定 OFF / fail-closed） (2) `p2p_federation` — UI キーを実在ゲート `federation_v1_5` に統一

> 本書は「手順どおりに写せば完成する」ことを目標に、全変更を before/after で記載する。
> 行番号は 2026-08-08 時点。ズレていても before スニペットで一意に特定できる。

---

## 0. 検証済みの前提（すべて実コードで確認済み）

| # | 前提 | 根拠（ファイル:行） |
|---|---|---|
| P1 | `SettingsOps::is_feature_enabled` は既定実装があり、`feature_flag.{flag}` を読み **未設定なら false** | `libs/aiome-core-contracts/src/traits.rs:508-515` |
| P2 | `JobQueue` トレイトは `SettingsOps` を継承している | `traits.rs:696-708` |
| P3 | `Arc<dyn サブトレイト>` → `Arc<dyn SettingsOps>` の upcast は本リポジトリで使用実績あり（`CostCircuitBreaker::new(self.ops.clone())`、`ops: Arc<dyn CostOps>`、`CostOps: SettingsOps`） | `libs/infrastructure/src/llm/background.rs:64` / `job_queue/settings.rs:18` |
| P4 | `PublishPipeline` は `publishers: Vec<Box<dyn Publisher>>` のみを持つ。`run_job` が全 publisher の単一入口 | `libs/infrastructure/src/publisher/mod.rs:23-52` |
| P5 | `run_job` の本番呼び出し元は `SeoContentConductor::conduct` の1箇所のみ。**publish 失敗は warn ログで握られ、ジョブは継続する**（= ゲートで Err を返しても致命傷にならない） | `libs/infrastructure/src/task_orchestrator/seo_content.rs:240-248` |
| P6 | `PublishPipeline::new` の呼び出しは本番1（core_services:744）＋テスト6（seo_content.rs×5, common.rs:532）。テストはすべて `new(vec![])` | grep 検証済み |
| P7 | settings ルートは `feature_flag.` プレフィクスの任意キー（英数+_）を許可し、書き込み時に **moka キャッシュへ即同期**する | `apps/api-server/src/routes/settings.rs:158-183, 239-244` |
| P8 | `AppState::is_feature_enabled` はキャッシュ→JobQueue の順で読む。main.rs の federation ループは毎 tick（既定300s+jitter）で再評価する | `app_state.rs:209-236` / `main.rs:113-131` |
| P9 | `feature_flag.federation_v1_5` は migration で **既定 'true'** をシード済み | `libs/infrastructure/migrations/sqlite/20260506000000_federation_v1_feature_flag.sql` |
| P10 | UI トグルの書き込み経路（`updateSetting('feature_flag.*', v, 'feature_flags')`）は既に機能している。欠けているのは Rust 側の読み手だけ | `SettingsPage.tsx:607-630` |
| P11 | `shared::feature_flags` に定数を置く既存パターンがある（`FEDERATION_V1_5_FLAG` 等3定数） | `libs/shared/src/feature_flags.rs` |
| P12 | infrastructure クレートは shared に依存済み（定数 import 可） | `libs/infrastructure/Cargo.toml` |

---

## 1. 実装A: `seo_publish` ゲート新設

### 設計判断（確定）
- ゲート位置は **`PublishPipeline::run_job` 冒頭**（P4: 将来 publisher が増えても単一関門）
- `settings` は `Option<Arc<dyn SettingsOps>>` として保持。**None のときはゲートなし（従来挙動）** — テスト6箇所（P6）を無変更で通すため。本番組み立ては必ず Some を渡すので fail-closed が成立する
- フラグ未設定は P1 により false = **既定で投稿しない**。現在 `seo_content` ジョブの生産者が未配線のため、この既定変更による本番リグレッションはない

### Step S1: フラグ定数の追加

**ファイル**: `libs/shared/src/feature_flags.rs`（既存定数の下に追記）

```rust
/// SEO コンテンツの自動投稿（WordPress 等への外部送信）を許可するフラグ。
/// 未設定は無効（fail-closed）。Settings UI の「SEO Publishing」トグルが書き込む。
pub const SEO_PUBLISH_FLAG: &str = "seo_publish";
```

### Step S2: `PublishPipeline` にゲートを実装

**ファイル**: `libs/infrastructure/src/publisher/mod.rs`

**(S2-1) import 変更** — before:

```rust
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use aiome_core_contracts::traits::{Job, JobQueue, JobStatus, Publisher};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::info;
```

after（`SettingsOps` と `Arc` を追加）:

```rust
use aiome_core_contracts::error::AiomeError;
use aiome_core_contracts::llm::LlmProvider;
use aiome_core_contracts::traits::{Job, JobQueue, JobStatus, Publisher, SettingsOps};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
```

**(S2-2) struct とコンストラクタ** — before:

```rust
pub struct PublishPipeline {
    publishers: Vec<Box<dyn Publisher>>,
}

impl PublishPipeline {
    /// 新しいインスタンスを生成する
    pub fn new(publishers: Vec<Box<dyn Publisher>>) -> Self {
        Self { publishers }
    }
```

after:

```rust
pub struct PublishPipeline {
    publishers: Vec<Box<dyn Publisher>>,
    /// 外部送信ゲート。Some の場合、`feature_flag.seo_publish` が true でなければ投稿しない。
    /// None はテスト用（ゲートなし＝従来挙動）。本番組み立て（core_services）は必ず Some を渡すこと。
    settings: Option<Arc<dyn SettingsOps>>,
}

impl PublishPipeline {
    /// 新しいインスタンスを生成する
    pub fn new(publishers: Vec<Box<dyn Publisher>>) -> Self {
        Self {
            publishers,
            settings: None,
        }
    }

    /// 外部送信ゲート（SettingsOps）を接続する
    pub fn with_settings(mut self, settings: Arc<dyn SettingsOps>) -> Self {
        self.settings = Some(settings);
        self
    }
```

**(S2-3) `run_job` 冒頭にゲート** — before:

```rust
    pub async fn run_job(
        &self,
        platform: &str,
        content: &str,
        media_paths: &[PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        let publisher = self
```

after（`let publisher` の**前**にブロックを挿入）:

```rust
    pub async fn run_job(
        &self,
        platform: &str,
        content: &str,
        media_paths: &[PathBuf],
        metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        // Fail-Closed 外部送信ゲート: ユーザーが Settings で明示的に有効化しない限り投稿しない
        if let Some(settings) = &self.settings {
            if !settings
                .is_feature_enabled(shared::feature_flags::SEO_PUBLISH_FLAG)
                .await
            {
                info!(
                    "⏭️ [PublishPipeline] Publish to '{}' skipped: feature_flag.seo_publish is disabled (enable it in Settings → Feature Flags).",
                    platform
                );
                return Err(AiomeError::Infrastructure {
                    reason: "publish skipped: feature_flag.seo_publish is disabled".to_string(),
                });
            }
        }

        let publisher = self
```

> Err を返しても安全な根拠は P5（唯一の呼び出し元は warn して継続）。

### Step S3: 本番組み立てにsettingsを接続

**ファイル**: `apps/api-server/src/bootstrap/core_services.rs`

before（`:744` と `:793` 付近。中間の publishers 構築ブロックは**一切変更しない**）:

```rust
    let publish_pipeline = Arc::new(infrastructure::publisher::PublishPipeline::new({
```

```rust
        publishers
    }));
```

after（末尾の閉じだけ変更。`job_queue` は同関数冒頭 `:27` の `let job_queue = &db.job_queue;`）:

```rust
    let publish_pipeline = Arc::new(
        infrastructure::publisher::PublishPipeline::new({
```

```rust
            publishers
        })
        .with_settings(job_queue.clone()),
    );
```

> `job_queue.clone()`（`Arc<dyn JobQueue>`）は P2+P3 により `Arc<dyn SettingsOps>` へ upcast される。
> もし toolchain が upcast 係数を拒否した場合のフォールバック:
> `let settings_ops: Arc<dyn aiome_core_contracts::traits::SettingsOps> = job_queue.clone(); ... .with_settings(settings_ops)`

### Step S4: ゲートのユニットテスト追加（Positive / Negative / 既定）

**ファイル**: `libs/infrastructure/src/publisher/mod.rs` 末尾に新規 `#[cfg(test)] mod tests` を追加。

テクニック: publishers を空にしておけば、**ゲート通過＝「Publisher not found」エラー / ゲート遮断＝「publish skipped」エラー**でメッセージにより判別でき、モック publisher が不要。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use aiome_core_contracts::traits::SettingsOps;

    /// feature_flag.seo_publish に固定値を返すテスト用 SettingsOps
    #[derive(Debug)]
    struct FixedFlagSettings {
        seo_publish: Option<&'static str>,
    }

    #[async_trait]
    impl SettingsOps for FixedFlagSettings {
        async fn do_get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
            if key == "feature_flag.seo_publish" {
                return Ok(self.seo_publish.map(|s| s.to_string()));
            }
            Ok(None)
        }
        async fn do_set_setting(
            &self,
            _key: &str,
            _value: &str,
            _category: &str,
            _is_secret: bool,
        ) -> Result<(), AiomeError> {
            Ok(())
        }
        async fn do_get_all_settings(
            &self,
        ) -> Result<Vec<aiome_core_contracts::contracts::SystemSetting>, AiomeError> {
            Ok(vec![])
        }
    }

    fn pipeline_with_flag(value: Option<&'static str>) -> PublishPipeline {
        PublishPipeline::new(vec![])
            .with_settings(Arc::new(FixedFlagSettings { seo_publish: value }))
    }

    #[tokio::test]
    async fn test_run_job_blocked_when_flag_disabled() {
        // Negative: フラグ false → ゲートで遮断（"publish skipped" を含む）
        let p = pipeline_with_flag(Some("false"));
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("seo_publish"), "got: {err}");
    }

    #[tokio::test]
    async fn test_run_job_blocked_when_flag_unset() {
        // Fail-Closed: フラグ未設定 → 遮断
        let p = pipeline_with_flag(None);
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("seo_publish"), "got: {err}");
    }

    #[tokio::test]
    async fn test_run_job_passes_gate_when_flag_enabled() {
        // Positive: フラグ true → ゲート通過（publishers 空なので "Publisher not found" に到達）
        let p = pipeline_with_flag(Some("true"));
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Publisher not found"), "got: {err}");
    }

    #[tokio::test]
    async fn test_run_job_no_settings_keeps_legacy_behavior() {
        // 互換: settings 未接続（テスト構成）はゲートなし
        let p = PublishPipeline::new(vec![]);
        let err = p
            .run_job("wordpress", "content", &[], &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Publisher not found"), "got: {err}");
    }
}
```

> `SystemSetting` のパスが違う場合は `rg "struct SystemSetting" libs/aiome-core-contracts/` で正しい module を確認して合わせること。

### Step S5: 変更しないもの（誤爆防止）

- `core_services.rs:745-792` の publishers 構築ブロック（WP adapter / mock 分岐）
- `SeoContentConductor`（`geo_enabled` ゲートは既存のまま。二重ゲートになるが役割が違う: geo=品質、seo_publish=ユーザー同意）
- テスト6箇所の `PublishPipeline::new(vec![])`（S2 の Option 設計により無変更でコンパイル・パスする）
- Settings UI の `seo_publish` トグル（既に正しいキーで書いている。P10）

---

## 2. 実装B: `p2p_federation` → `federation_v1_5` キー統一

### 設計判断（確定）
- **UI を backend に合わせる**（方式1）。backend キーの改名（方式2）は migration・integration test・swarm スクリプトに波及するため採らない
- ラベルは実態（karma / immune rules / arena の hub 同期のみ。Commune WS 等は対象外）に合わせ「Karma Federation Sync」へ変更（誤解防止）
- migration の既定 'true'（P9）は互換のため**維持**。ユーザーが OFF にすると次 tick（≤300s+jitter）から push/sync が止まる（P7 のキャッシュ同期 + P8 の毎 tick 再評価）

### Step F1: SettingsPage のキーとラベル変更

**ファイル**: `apps/management-console/src/components/SettingsPage.tsx`（:619-624）

before:

```tsx
                        <FeatureToggle 
                            label={t('settings.ffP2pFederation', { defaultValue: 'P2P Federation' }) as string} 
                            current={getSetting('feature_flag.p2p_federation')} 
                            onUpdate={(v) => updateSetting('feature_flag.p2p_federation', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.p2p_federation'} 
                        />
```

after:

```tsx
                        <FeatureToggle 
                            label={t('settings.ffKarmaFederationSync', { defaultValue: 'Karma Federation Sync' }) as string} 
                            current={getSetting('feature_flag.federation_v1_5')} 
                            onUpdate={(v) => updateSetting('feature_flag.federation_v1_5', v, 'feature_flags')} 
                            saving={saving === 'feature_flag.federation_v1_5'} 
                        />
```

### Step F2: i18n キーの改名（en / ja 両方）

**ファイル**: `apps/management-console/src/i18n/en.json`（:380）

```json
    "ffP2pFederation": "P2P Federation",
```
→
```json
    "ffKarmaFederationSync": "Karma Federation Sync",
```

**ファイル**: `apps/management-console/src/i18n/ja.json`（:380）

```json
    "ffP2pFederation": "P2P フェデレーション",
```
→
```json
    "ffKarmaFederationSync": "Karma フェデレーション同期",
```

### Step F3: 残骸掃除（任意だが推奨）

- `apps/api-server/src/routes/settings.rs:779` のキー形式テスト例 `feature_flag.p2p_federation` を `feature_flag.federation_v1_5` へ変更（形式検証の例示なので機能影響なし）
- 完了後 `rg -n "p2p_federation" --glob '!docs/**' --glob '!node_modules/**'` が **0件**であることを確認

### 変更しないもの（誤爆防止）
- `main.rs:120` のゲート（既に正しい）
- `libs/shared/src/feature_flags.rs` の `FEDERATION_V1_5_FLAG` 定数
- migration SQL（改変・削除禁止。適用済み DB とズレる）
- `apps/api-server/src/api_integration_tests/auth.rs:38-43`（`federation_v1_5` を直接使用しており正しい）
- `tests/swarm_simulation_test.sh`（同上）

---

## 3. 実装順序と検証プロトコル

### 順序（コミット2つに分割推奨）

1. **コミット1**: 実装B（F1→F2→F3）— フロントのみ・低リスク
2. **コミット2**: 実装A（S1→S2→S3→S4）— Rust ゲート

### 検証（各コミット後に必須）

| # | コマンド | 期待結果 |
|---|---|---|
| V1 | `cd apps/management-console && npm run lint`（= `tsc --noEmit`） | exit 0 |
| V2 | `rg -n "p2p_federation\|ffP2pFederation" apps/ libs/` | 0件（コミット1後） |
| V3 | `cargo check --workspace --tests` | Finished（コミット2後） |
| V4 | `cargo test -p infrastructure publisher` | 新規4テスト PASS |
| V5 | `cargo test -p infrastructure --test '*' -- seo` + `cargo test -p api-server` | 既存 SEO/設定系テスト PASS |
| V6 | `cargo test --workspace` | ベースライン同等 |

### 手動 E2E（実装後の Positive / Negative / Revert — Verification Protocol 準拠）

前提: 開発環境ログインは Settings のパスワード欄に `SuperSecretPassword123!`（DB 再生成・上書きは厳禁）。

1. **Positive（federation）**: Settings → Feature Flags →「Karma フェデレーション同期」トグルが表示され、現在値（既定 ON）が見える。OFF に切替 → `system_settings` の `feature_flag.federation_v1_5` が `false` になり、次回 tick でログ `Running periodic federated metrics push & sync...` が**出ない**
2. **Negative（seo）**: `feature_flag.seo_publish` 未設定のまま `seo_content` ジョブを手動投入（または V4 のユニットテストで代替）→ ログ `⏭️ [PublishPipeline] Publish ... skipped` を確認
3. **Revert**: federation トグルを ON に戻し、次 tick で push/sync ログが再開することを確認

### ロールバック

- コミット単位で `git revert`。DB 状態はフラグ行のみ（`feature_flag.federation_v1_5` は元々存在、`feature_flag.seo_publish` は残っても読み手が消えるだけで無害）

---

## 4. 落とし穴チェックリスト（実装者は着手前に読むこと）

- [ ] `run_job` の Err はジョブを落とさない（P5）が、**エラーメッセージに "seo_publish" を含める**こと（テスト S4 が文字列で判別するため）
- [ ] `with_settings` を core_services **以外**で呼ばない（テストの互換性を守る）
- [ ] `shared::feature_flags::SEO_PUBLISH_FLAG` を使う。文字列リテラル `"seo_publish"` をゲートに直書きしない（定数と UI キーのドリフト防止）
- [ ] i18n は **en / ja 両方**を同時に変更（README_en 同期則と同様）
- [ ] migration ファイルは**絶対に**編集しない
- [ ] `SettingsPage.tsx` の他のトグル（`js_fallback` / `lora_training` / `intent_first_suggestion` 等）に触らない
- [ ] Safety-Critical Zone（auth / key-proxy / commerce webhook / tauri.conf.json）に**一切触れない**構成であることを diff で確認

## 5. DoD

- [x] V1〜V4 パス（`tsc --noEmit` / `rg p2p_federation` ソース 0 件 / `cargo check` / publisher 4テスト）
- [x] CHANGELOG / triage / enterprise_edition_plan Phase 0.5 更新
- [x] `.env.example` 変更なし
- [ ] 手動 E2E の 3 段階完了（ログイン後の UI 確認は Human）
- [x] `sync_mc_static.sh` で static アセット同期（ビルド成果物の旧キー除去）
