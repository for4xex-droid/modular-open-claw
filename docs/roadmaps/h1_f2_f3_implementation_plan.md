# 実装計画書 H1: F-2 Outcome Ledger / F-3 Skill Marketplace α

**作成日**: 2026-07-03（偵察による実在確認済み。行番号は当日時点 — 着手時はアンカー文字列で再特定すること）
**対象**: `value_10x_roadmap.md` F-2（ROI ダッシュボード）と F-3（署名付きスキル配布）
**性質**: 機能追加。F-2 は既存挙動を変えない。F-3 は import 経路に「署名検証」と「import 直後 dry-run」を追加する（意図的な厳格化）。
**実行順**: F-2 → F-3（独立だが、F-3 は Safety-Critical Zone に接するため人間レビュー体制を整えてから）。

---

## 0. 共通の安全網（各機能の着手前に必ず実行）

```bash
cd /Users/motista/Desktop/antigravity/aiome
git status                    # クリーンであること。汚れていれば中断・報告
git checkout -b feature/f2-outcome-ledger   # F-3 は feature/f3-skill-marketplace
cargo check --workspace --tests
cargo test -p infrastructure 2>&1 | grep "test result"     # ベースライン記録
cargo test -p api-server api_integration_tests 2>&1 | grep "test result"
cargo clippy -p infrastructure -p api-server --all-targets -- -D warnings
cd apps/management-console && npx jest 2>&1 | tail -4 && cd ../..
```

全 GREEN でなければ着手しない。以降、各項目の完了条件で「ベースラインと同数以上 PASS・failed 0」を確認する。

**共通ルール**: 1項目=1コミット（`feat(outcome): L-N ...` / `feat(market): M-N ...`）。完了条件を満たせなければ中断して報告。`cargo fmt` はコミット前に対象クレートへ実行（pre-commit フックが fmt/anti-pattern/gitleaks を強制する）。

---

# PART A: F-2 Outcome Ledger

## A-1. 現状理解（実行者への文脈共有）

「今月◯時間・¥◯分働いた」をホーム画面に常設表示する。**集計エンジンは新造せず**、以下の実在ソース3つの集約ビューとして実装する。

| ソース | 実在（確認済み） | 使い方 |
|---|---|---|
| ジョブ完了 | `jobs` テーブル。完了は `core_ops.rs` の `do_complete_job`（アンカー: `async fn do_complete_job`）で `status='Completed'` に更新。`get_job_count_since(since)` が既存（`traits.rs` の `JobQueue` trait） | 件数の正本 |
| ジョブのカテゴリ | `jobs.category` カラム（`fetch_recent_jobs` で取得可能） | 種別ごとの換算に使用 |
| LLM コスト | `prompt_evaluation_log`（provider/model/latency_ms/token_count_in/out/cost_usd/cache_hit/created_at）。集計 SQL の参照実装は `evaluation_logger.rs` の `get_all_provider_stats`（アンカー: `GROUP BY provider, model`） | 「かかったコスト」側の表示 |

重要な既知事実:
- `SkillArena::record_outcome()` は**インメモリ集計のみ**で、DB 書込は `save_stats()` 経由に限られる。よって成果イベントの正本には使えない。**F-2 では jobs テーブルを正本にする**。
- 換算係数（1タスク=◯分の節約等）は既存の key-value Settings（`system_settings` テーブル、`PUT /api/v1/settings`、`UpdateSettingsRequest { key, value, category }`）に載せる。専用テーブルは作らない。
- HomePage（`apps/management-console/src/components/home/HomePage.tsx`）は API を直接呼ばず `App.tsx` から props を受ける構造。ただし新ウィジェットは**自前フックで API を呼ぶ独立コンポーネント**として追加し、App.tsx の配線変更を最小化する（`useModelStatus.ts` のパターン踏襲）。
- CSV export の既存実装は api-server に**存在しない**（rg 0件）。文字列組み立てで自作する（依存クレート追加禁止）。

## A-2. 新規 API 契約（確定。変更しない）

| メソッド | パス | 応答 |
|---|---|---|
| GET | `/api/v1/outcomes/summary?period=30d` | `OutcomeSummaryResponse`（下記） |
| GET | `/api/v1/outcomes/export?period=30d` | `text/csv`（ヘッダ `category,count,minutes_saved,value_jpy`） |

```rust
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct OutcomeCategorySummary {
    pub category: String,
    pub completed_count: i64,
    pub minutes_saved: Option<f64>,   // 係数未設定なら None（虚偽の数字を出さない）
    pub value_jpy: Option<f64>,       // 同上
}
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct OutcomeSummaryResponse {
    pub period: String,
    pub total_completed: i64,
    pub total_minutes_saved: f64,     // 係数のある種別のみの合計
    pub total_value_jpy: f64,
    pub llm_cost_usd: f64,            // prompt_evaluation_log の SUM(cost_usd)
    pub categories: Vec<OutcomeCategorySummary>,
}
```

換算係数の Settings 規約: key=`outcome_rate_<category>`、value=`{"minutes_per_task": 30, "jpy_per_task": 1500}`（JSON 文字列）、category=`outcome`。

## A-3. 作業項目リスト（実行順）

### L-1: 集計クエリを JobQueue trait ではなく専用リポジトリ関数として追加（infrastructure）

- **対象**: `libs/infrastructure/src/outcome_ledger.rs`（新規）、`libs/infrastructure/src/lib.rs`（`pub mod outcome_ledger;` 1行）
- **変更**: `DatabasePool` を受け取り2クエリを発行する純関数モジュールを作る。**JobQueue trait には手を触れない**（trait 変更は全 Mock 同期が必要になり被害半径が大きい）。

```rust
pub struct CategoryCount { pub category: String, pub count: i64 }

/// 期間内の完了ジョブをカテゴリ別に集計
pub async fn completed_jobs_by_category(pool: &DatabasePool, since: chrono::DateTime<chrono::Utc>)
    -> Result<Vec<CategoryCount>, AiomeError>
{ /* SELECT category, COUNT(*) FROM jobs WHERE status='Completed' AND updated_at >= ? GROUP BY category
     — SQLite/PG 両対応は audit.rs の get_sqlite_pool_or_err パターンでよい（PG 側は OP-012 未検証のため SQLite 優先で可） */ }

/// 期間内の LLM 総コスト
pub async fn llm_cost_since(pool: &DatabasePool, since: ...) -> Result<f64, AiomeError>
{ /* SELECT COALESCE(SUM(cost_usd),0) FROM prompt_evaluation_log WHERE created_at >= ? */ }
```

- **テスト**（同ファイル `#[cfg(test)]`）: インメモリ SQLite に jobs/prompt_evaluation_log を作成し、`test_completed_jobs_by_category_filters_status_and_period` / `test_llm_cost_since_sums_only_period` の2本。
- **完了条件**: `cargo test -p infrastructure outcome_ledger` 2本 PASS、`cargo check --workspace --tests`、clippy。
- **リスク**: 低。`updated_at` の型（TEXT RFC3339）比較は既存 `get_job_count_since` の SQL（`created_at >= ?`）を踏襲。失敗時 `git checkout -- libs/infrastructure/src/`。
- **依存**: 項目0

### L-2: `/api/v1/outcomes/*` ハンドラ＋配線＋OpenAPI＋統合テスト（api-server）

- **対象**: `apps/api-server/src/routes/outcome.rs`（新規）、`routes/mod.rs`、`router.rs`（workflow nest の近く）、`api.rs`
- **変更**:
  1. `summary` ハンドラ: `period` は `audit.rs` の `get_audit_prompt_stats` の parse ロジック（アンカー: `1–3650日 clamp`）をコピーして踏襲 → L-1 の2関数で集計 → Settings から `outcome_rate_<category>` を読み（`state.job_queue` の設定取得は `routes/settings.rs` の GET 実装を踏襲）、係数がある種別のみ `minutes_saved`/`value_jpy` を計算。
  2. `export` ハンドラ: summary と同じデータを CSV 文字列に組み立て、`([(header::CONTENT_TYPE, "text/csv")], body)` で返す。**PII 非含有**: 出力列は category/count/数値のみ（ジョブ本文・トピックは含めない）。
  3. `.route("/api/v1/outcomes/summary", get(...)).route("/api/v1/outcomes/export", get(...))` を `internal_router`（auth 必須側）へ配線。`api.rs` の `paths(...)` と `components(schemas(...))` に登録（4点セット遵守）。
  4. 統合テスト `api_integration_tests/outcome.rs`（`playbook.rs` を雛形に）: `test_outcome_routes_auth`（未認証401）/ `test_outcome_summary_counts_completed_jobs`（ジョブを enqueue→complete し件数一致）/ `test_outcome_summary_omits_value_without_rate`（係数未設定で `minutes_saved == null`）/ `test_outcome_export_has_no_pii`（CSV にトピック文字列が現れない）。
- **完了条件**: 統合テスト4本 PASS、既存統合テストがベースラインと同数、clippy。**Negative 配線確認**: ルート2行をコメントアウト→該当テスト FAIL→復元→GREEN（F-1 P-4 と同手順）。
- **リスク**: 中。テストでジョブを Completed にする方法は `core_ops.rs` の `complete_job` を直接呼ぶ（統合テストは `_state` を持っているため可能）。
- **依存**: L-1

### L-3: 換算係数の Settings UI（management-console）

- **対象**: `apps/management-console/src/components/SettingsPage.tsx`
- **変更**: 既存の `updateSetting(key, value, category)` ローカル関数（アンカー: `PUT ${API_BASE}/api/v1/settings`）をそのまま使い、「成果換算」セクションを追加。カテゴリ一覧は `GET /api/v1/outcomes/summary` の `categories[].category` から動的に取得し、各行に「1件あたり分」「1件あたり円」の数値入力 → 保存で `outcome_rate_<category>` に JSON 文字列を書く。
- **完了条件**: `npx tsc --noEmit` で新規エラーなし（既存 biome 系8件は無視）、`npx jest SettingsPage` が存在すれば GREEN 維持。
- **依存**: L-2

### L-4: HomePage ウィジェット＋フック＋テスト（management-console）

- **対象**: `src/hooks/useOutcomeSummary.ts`（新規）、`src/components/home/OutcomeLedgerWidget.tsx`（新規）、`src/components/home/HomePage.tsx`（ウィジェット1個の挿入のみ）
- **変更**:
  1. フック: `useModelStatus.ts` のパターン（`authenticatedFetch` + `components['schemas'][...]` 型。generated.ts が未再生成なら手書き interface で可、TODO コメント禁止・OP 登録で対応）で `GET /api/v1/outcomes/summary?period=30d` を取得。`refresh()` を公開。
  2. ウィジェット: 「今月: タスク N 件 / 節約 X 時間 / 相当額 ¥Y」。係数のない種別は件数のみ表示。スタイルは `var(--...)` トークンのインライン（HomePage 既存ウィジェットの慣例）。
  3. **リロードなし再計算**: Settings 保存後に `window.dispatchEvent(new CustomEvent('aiome_outcome_rates_updated'))` を SettingsPage から発火し、フックが listener で `refresh()` する（受け入れ基準2）。
  4. i18n: `home.outcome*` キーを ja/en に追加。
  5. テスト `OutcomeLedgerWidget.test.tsx`: 表示3項目 / 係数なし種別が金額非表示 / `aiome_outcome_rates_updated` 受信で再フェッチ、の3本（fetch は jest.mock）。
- **完了条件**: `npx jest Outcome` 3本 PASS、既存 jest スイート GREEN、`npx tsc --noEmit` 新規エラーなし。
- **依存**: L-2（L-3 と並行可）

### L-5: ドキュメント同期

- CHANGELOG [Unreleased] / OPEN.md（完了記録）/ value_10x_roadmap.md に F-2 実装済み注記 / `docs-sync-check.sh` PASS。
- **依存**: L-4

## A-4. F-2 やらないこと

1. **JobQueue trait / TrajectoryStore trait を変更しない**（Mock 同期の被害半径が大きい。集計は L-1 の独立モジュールで行う）。
2. **`record_outcome` の永続化改修をしない**（スキル別成果の DB 化は別課題。jobs テーブルが正本）。
3. Phase 5（Cognitive Observability）のトレース基盤を先取りしない。
4. 通貨換算・為替 API を呼ばない（`llm_cost_usd` は USD のまま表示）。
5. 専用テーブル・マイグレーションを追加しない（v1 は既存テーブルの読み取りのみ）。

---

# PART B: F-3 Skill Marketplace α

## B-1. 現状理解（実行者への文脈共有）

WASM スキルを**署名付きパッケージ**として export/import できるようにする。α版は「無償配布のみ」（法務前提: 特商法/資金決済法対応が完了するまで `CommodityKind::WasmSkill` の販売結線は**しない** — やらないこと参照）。

実在資産（確認済み）:
- **import パイプライン**: `POST /api/skills/import`（`routes/skill.rs`、アンカー: `SSRF Blocked`）→ 1MB 制限 → `SkillImporter` → `Cleanroom::process_import(manifest) -> PathBuf`（LLM 監査＋Forge）。
- **保存レイアウト**: `{wasm_storage}/{name}.wasm` + `{name}.meta.json`（`LocalSkillMetadata`: name/description/capabilities/inputs/outputs）。`resolver.resolve("wasm_storage")`。
- **Ed25519**: `libs/shared/src/auth.rs` の `JwtAuthManager`（`SigningKey::generate` / `from_private_key_b64` / `export_private_key_b64`、`ed25519_dalek`）。生署名の実例は `job_queue/swarm.rs` の `do_sign_swarm_payload`（アンカー: `SigningKey::from_bytes`）。検証の実例は `samsara-hub/src/auth.rs` の `verify_ed25519_signature`。
- **TypeState**: import されたスキルは実行時の `UnverifiedSkill::verify`（`skills/types.rs`）→ `dry_run_skill` で検証される。ただし **import 直後には dry-run されない**（実行時まで遅延）— 本計画 M-4 で import 直後 dry-run を追加する。
- **SkillVault.tsx**: filter に `'market'` が既に存在（アンカー: `'all' | 'my' | 'market'`）。一覧 API は `GET /api/skills`。
- `skill_maturity` テーブル: 未登録スキルは `Quarantined` 扱い（`get_skill_maturity`）。**import 品は必ず Quarantined から**（受け入れ基準1はこの既存挙動で満たされる。テストで固定する）。

## B-2. パッケージ形式 v1（確定。変更しない）

単一 JSON ファイル（`.aiomeskill.json`）:

```json
{
  "package_version": 1,
  "name": "<skill_name>",
  "meta": { /* LocalSkillMetadata と同形 */ },
  "wasm_base64": "<base64>",
  "wasm_sha256": "<hex>",
  "publisher_pubkey_b64": "<Ed25519 verifying key>",
  "signature_b64": "<Ed25519 signature over sha256(name + wasm_sha256 + meta_json_canonical)>"
}
```

- 署名対象は `format!("{}\n{}\n{}", name, wasm_sha256, serde_json::to_string(&meta))` の SHA-256 ダイジェスト。
- サイズ上限: wasm_base64 デコード後 10MB。name は `^[a-zA-Z0-9_-]{1,64}$`（既存 `proof_verifier.rs` のパストラバーサル検査と同水準）。

## B-3. 作業項目リスト（実行順）

### M-1: 発行者鍵の管理（infrastructure / api-server bootstrap）

- **対象**: `libs/infrastructure/src/skills/package.rs`（新規）、bootstrap（`core_services.rs` の JWT 鍵ロード周辺、アンカー: `JWT_PRIVATE_KEY_B64`）
- **変更**: 環境変数 `SKILL_SIGNING_KEY_B64`（未設定なら export 機能は 503 を返す設計。**自動生成して黙って使う実装は禁止** — 鍵の出所が不明になる）。`package.rs` に `sign_package` / `verify_package` の純関数を実装（`ed25519_dalek`、`swarm.rs` の署名パターン踏襲）。
- **テスト**: `test_package_sign_verify_roundtrip` / `test_package_verify_rejects_tampered_wasm`（1バイト改ざんで Err）/ `test_package_rejects_oversize_and_bad_name` の3本を**実装と同一コミットで RED→GREEN**。
- **完了条件**: `cargo test -p infrastructure skills::package` 3本 PASS、clippy。
- **依存**: 項目0

### M-2: export API（api-server）

- **対象**: `routes/skill.rs` に `export_skill` ハンドラ追加、`router.rs`、`api.rs`
- **変更**: `GET /api/skills/:name/export` — name 検査（`proof_verifier.rs` の canonicalize + starts_with 検査をコピー）→ `{name}.wasm` と `{name}.meta.json` を読み → M-1 の `sign_package` → JSON 返却。`SKILL_SIGNING_KEY_B64` 未設定は 503（明示メッセージ）。
- **完了条件**: 統合テスト `test_skill_export_roundtrip`（既存フィクスチャ WASM を export→JSON スキーマ検証）PASS、Negative 配線確認、clippy。
- **依存**: M-1

### M-3: 署名検証つき import API（api-server）

- **対象**: `routes/skill.rs` に `import_skill_package` 追加（既存 `import_skill` = URL import は**変更しない**）
- **変更**: `POST /api/skills/import-package`（body = パッケージ JSON）→ B-2 の構造検査 → `verify_package`（失敗は 400 + **Aegis 監査ログ記録**。記録方法は `skill_arena.rs` の `insert_incident` 呼び出しパターンを踏襲）→ wasm/meta を `wasm_storage` へ書き込み（既存ファイルと衝突する name は 409）→ `hot_reload_skills` 相当でキャッシュ無効化。
- **統合テスト**: `test_import_package_registers_as_quarantined`（import 後 `get_skill_maturity == Quarantined`）/ `test_import_package_rejects_tampered_signature`（改ざんで 400 ＋ incidents テーブルに記録）/ `test_import_package_rejects_path_traversal_name`。
- **完了条件**: 3本 PASS、既存 skill テスト GREEN、clippy。
- **リスク**: 中。書込先は必ず canonicalize 済み `wasm_storage` 配下であることを starts_with で確認（M-2 と同じガード）。
- **依存**: M-1（M-2 と並行可）

### M-4: import 直後の自動 dry-run（意図的な厳格化）

- **対象**: M-3 の `import_skill_package` 末尾
- **変更**: 書込成功後に `state.wasm_skill_manager.dry_run_skill(&name, ...)` を実行（呼び出し形は `skill_handler.rs` の `execute_wasm_skill` 内の使用箇所を踏襲。アンカー: `dry_run_skill`）。失敗時は**書き込んだ .wasm/.meta.json を削除して 422** を返す（部分適用禁止）。dry-run 用入力はパッケージに `dry_run_payload` オプションフィールドを許可（未指定なら `{}`）。
- **統合テスト**: `test_import_package_dry_run_failure_rolls_back`（意図的に壊れた WASM を署名して import → 422 → ファイルが残っていない）。
- **完了条件**: 新テスト PASS、M-3 のテスト GREEN 維持、clippy。
- **依存**: M-3

### M-5: SkillVault「マーケット」タブの実体化（management-console）

- **対象**: `SkillVault.tsx`（filter `'market'` の表示内容）、`src/i18n/*`
- **変更**: market フィルタ選択時に「パッケージファイルの import」UI（ファイル選択 → JSON を `POST /api/skills/import-package`）と、自スキルの「export」ボタン（`GET /api/skills/:name/export` → Blob ダウンロード）を表示。422/400 のエラー本文（欠落・署名不正・dry-run 失敗）をそのまま表示。
- **テスト**: `SkillVault.test.tsx` が存在すれば GREEN 維持＋import 成功/署名エラー表示の2本追加。なければ新設2本。
- **完了条件**: jest GREEN、`npx tsc --noEmit` 新規エラーなし。
- **依存**: M-2, M-4

### M-6: 敵対テスト（受け入れ基準4）とドキュメント同期

- **対象**: `api_integration_tests/skill_market.rs`（新規）、CHANGELOG、OPEN.md、SECURITY_DESIGN.md、value_10x_roadmap.md
- **変更**: 「`is_sensitive_path` をバイパスしようとするスキル」の敵対テスト — 機密パス書込を試みる既存テストフィクスチャ WASM（`tests.rs` の `test_wasm_skill_incident_on_error` が使うフィクスチャの慣例に従う）を M-1 で署名 → import-package → 実行 → `host_write` でブロックされ incidents に記録されることを検証。SECURITY_DESIGN.md に「スキルパッケージ3層防御（署名・dry-run・実行時ガード）」を追記。
- **完了条件**: 敵対テスト PASS、`docs-sync-check.sh` PASS。
- **依存**: M-4

## B-4. F-3 やらないこと

1. **販売結線をしない**: `CommodityKind::WasmSkill` は既に enum 定義済みだが、Nurture 決済フロー（`commercial/` 配下、Safety-Critical）への接続は法務前提（特商法・資金決済法）完了までスコープ外。α版は無償配布のみ。
2. **`commercial/` 配下と `commerce.rs` に触らない**。
3. 既存 URL import（`import_skill`）の挙動を変えない（署名なし import の禁止は将来判断。併存させる）。
4. 公開レジストリ/ホスティングを作らない（配布はファイル授受。レジストリは F-9 スコープ）。
5. 依存クレート追加禁止（`ed25519_dalek`/`sha2`/`base64` はいずれも既存依存。ないものが必要になったら中断・報告）。
6. TypeState を緩めない（import 品を Trusted で登録する等は絶対禁止）。

---

## 5. 実行者への指示文（このままコピペ）

```
docs/roadmaps/h1_f2_f3_implementation_plan.md を実行してください。
1. PART A（F-2: L-1→L-5）を feature/f2-outcome-ledger ブランチで、PART B（F-3: M-1→M-6）を feature/f3-skill-marketplace ブランチで、この順に実施する。
2. 1項目=1コミット。各項目の完了条件コマンドを実行し満たしてからコミット（feat(outcome): L-N ... / feat(market): M-N ...）。
3. 満たせなければ中断し、項目ID・コマンド・エラー全文を報告。assert 緩和・#[ignore]・スコープ外修正は禁止。
4. 「やらないこと」を先に読み全項目で遵守。行番号はアンカー文字列で再特定。
5. 「既存実装を踏襲」と指定された箇所は必ず該当コードを読んでから書く（推測禁止）。
6. 完了後、コミットハッシュ一覧と最終テスト結果を報告。git push はしない。
```

## 6. ロールバック

各項目は独立コミット。`git revert` は依存の逆順（L-5→L-1 / M-6→M-1）。ブランチ全破棄は `git checkout main && git branch -D <branch>`。
