# リファクタリング計画書: `libs/infrastructure/src/skills/` モジュール

**作成日**: 2026-07-03
**対象**: `libs/infrastructure/src/skills/`（中心は `mod.rs` 1,135行の God Module）+ 関連する `immune_system.rs` の MockJQ
**対応する技術的負債**: OPEN.md OP-050（God Module 分解）、OP-053（`loop {}` パニック回避策）、OP-055（MockJQ 共有化）
**この計画の性質**: 動作を変えない構造改善が主。唯一の意図的な挙動変更は R-2（セキュリティ検査の統一・厳格化）のみ。

---

## 1. 現状理解（実行者への文脈共有）

### 1-1. このモジュールが何をしているか

Aiome は自律 AI エージェント OS であり、エージェントは「スキル」（WASM バイナリ）を動的にロード・実行できる。`libs/infrastructure/src/skills/` はそのスキル実行基盤である。

- **WASM サンドボックス実行**: `WasmSkillManager` が extism クレートで WASM プラグインをロードし、メモリ上限・タイムアウト・ファイルシステム/ネットワーク権限を絞って実行する。
- **TypeState パターン**: `UnverifiedSkill` → （dry-run 検証通過）→ `VerifiedSkill` の型昇格により、未検証スキルの実行をコンパイルレベルで阻止する。この安全設計は本計画で**絶対に壊してはならない**。
- **ホスト関数**: WASM 側から `host_exec`（シェル実行）・`host_write`(ファイル書込) を呼べる。それぞれ `BastionGuard` とパス検証で防御されている。
- **Code Mode JS ブリッジ**: `run_code_mode_js()` は `aiome.log/exec/writeFile/readFile/fetch` という5命令だけの疑似 JavaScript を正規表現で行単位に解釈するミニインタープリタ（本物の JS エンジンではない）。仕様は同ディレクトリの `code_mode.d.ts` に対応。
- **成熟度管理**: スキルは `Quarantined → Probation → Trusted → Veteran` の成熟度を持ち、SQLite の `skill_maturity` テーブルに永続化される。

### 1-2. ファイル構成（2026-07-03 時点）

| ファイル | 行数 | 役割 |
|---|---|---|
| `mod.rs` | 1,135 | **God Module**。型定義・正規表現・WasmSkillManager 本体・JS ブリッジが混在 |
| `skill_arena.rs` | 685 | スキルの並列実行と評価（今回触らない） |
| `forge.rs` | 492 | スキル生成（今回触らない） |
| `tests.rs` | 435 | `mod.rs` のユニットテスト（`#[cfg(test)] mod tests;` として mod.rs 末尾で宣言） |
| `discovery.rs` | 305 | Semantic Tool Discovery（`list_skills_with_metadata` を利用） |
| `hooks.rs` / `importer.rs` / `harness.rs` / `cleanroom.rs` / `actions_importer.rs` | 各73〜266 | 今回触らない |
| `code_mode.d.ts` | — | JS ブリッジの型定義ドキュメント（コードではない） |

### 1-3. `mod.rs` の内部レイアウト（行番号は現状のもの。R-1 以降の作業で下方向にズレるため、**各項目の着手時に必ず記載のアンカー文字列で再検索すること**）

| 行範囲 | 内容 | アンカー文字列 |
|---|---|---|
| 40-64 | `is_sensitive_path()` 機密パス検出ヘルパー | `pub(crate) fn is_sensitive_path` |
| 66-120 | `UnverifiedSkill` / `VerifiedSkill`（TypeState） | `pub struct UnverifiedSkill` |
| 122-160 | `SkillMetadata` / `SkillMaturity` | `pub struct SkillMetadata` |
| 162-193 | 静的正規表現5本 + `DUMMY_REGEX` | `static DUMMY_REGEX` |
| 195-385 | `WasmSkillManager` 定義・ビルダー・成熟度 DB・キャッシュ・一覧 | `pub struct WasmSkillManager` |
| 390-684 | `call_skill()`（約300行。host_exec/host_write クロージャ内包） | `pub async fn call_skill` |
| 686-694 | `get_metadata()` | `pub fn get_metadata` |
| 699-777 | `dry_run_skill()` | `pub async fn dry_run_skill` |
| 781-831 | `validate_skill_logic()` | `pub async fn validate_skill_logic` |
| 833-864 | `search_skill_in_knowledge()` | `pub async fn search_skill_in_knowledge` |
| 868-1131 | `run_code_mode_js()`（約265行の JS ブリッジ） | `pub async fn run_code_mode_js` |
| 1134-1135 | `#[cfg(test)] mod tests;` | — |

### 1-4. 外部からの利用箇所（公開 API 面 — 壊してはならない）

`infrastructure::skills::` パスで以下が参照されている。**再エクスポート（`pub use`）でパスを維持する限り、利用側の変更は不要**。

- `WasmSkillManager`（`new`, `with_db_pool`, `with_vault`, `with_limits`, `call_skill`, `dry_run_skill`, `list_skills`, `list_skills_with_metadata`, `get_metadata`, `hot_reload_skills`, `invalidate_cache`, `get_skill_maturity`, `promote_skill_maturity`, `run_code_mode_js`, `validate_skill_logic`, `search_skill_in_knowledge`）
- `UnverifiedSkill`（`verify`）、`VerifiedSkill`（`name`, `new_for_test`）、`SkillMetadata`、`SkillMaturity`

主な利用側ファイル:
- `apps/api-server/src/`: `skill_handler.rs`, `routes/skill.rs`, `routes/a2ui.rs`, `routes/agent.rs`, `system_instructions.rs`, `mcp/server.rs`, `agent_engine.rs`, `tool_call_processor.rs`, `bootstrap/core_services.rs`, `app_state.rs`, `api_integration_tests/{common,system}.rs`, `internal_services/dream.rs`
- `apps/aiome-node/src/`: `main.rs`, `mcp_server.rs`
- `libs/infrastructure/src/`: `skills/discovery.rs`, `oss_orchestrator.rs`
- `libs/infrastructure/tests/`: `deerflow_tdd.rs`, `mbt_quarantine.rs`

### 1-5. 既存テスト（ベースラインとして利用可能）

`libs/infrastructure/src/skills/tests.rs` に以下があり、**特性テストは概ね既に存在する**:
`test_wasm_skill_timeout`, `test_wasm_skill_incident_on_error`, `test_wasm_skill_config_injection`, `test_dry_run_call_validation`, `test_dry_run_missing_skill_error`, `test_hot_reload_skills`, `test_skill_verification_promotion`, `test_list_skills_with_metadata_auto_generation`, `test_list_skills_with_explicit_metadata`, `test_grpc_formal_proof_gate_empty_token_rejection`, `test_code_mode_js_success`, `test_code_mode_js_security_violation_exec`, `test_code_mode_js_exec_success`, `test_code_mode_js_path_traversal_rejection`, および `is_sensitive_path` の単体テスト群（`sensitive_path_tests` サブモジュール）。

### 1-6. プロジェクト固有の制約（必読）

- **AGENTS.md のルールが適用される**。特に: R-005（本番コードで `unwrap()`/`expect()` 禁止）、R-007（テストを通すために assert を緩める・消すこと禁止）、Preserve Intent（未使用警告は安易な削除でなく意図を保持）。
- **pre-push フックが `cargo fmt --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`cargo audit`・全テストを強制する**。コミットは通るが、プッシュ時に全体が検証される。
- crate 名は `infrastructure`（`cargo test -p infrastructure` で指定）。

---

## 2. 項目0: 安全網の構築（最初に必ず実行）

### 0-a. 作業前コミット

```bash
cd /Users/motista/Desktop/antigravity/aiome
git status   # 未コミットの変更が「ない」ことを確認。あれば中断してユーザーに報告
git checkout -b refactor/skills-god-module
git log -1 --oneline   # 開始点のコミットハッシュを作業メモに記録
```

### 0-b. ベースライン確認

```bash
cargo check --workspace --tests
cargo test -p infrastructure skills   # 上記 1-5 の全テストが PASS することを確認
cargo test -p infrastructure immune_system   # R-7 で触る範囲のベースライン
cargo clippy -p infrastructure --all-targets -- -D warnings
```

4コマンドすべてが成功しない場合、**リファクタリングを開始せず**、失敗内容をユーザーに報告して中断すること（開始前から壊れているものを直すのはこの計画のスコープ外）。

PASS したテストの一覧を `cargo test -p infrastructure skills 2>&1 | grep "test result"` の出力とともに記録し、以降の各項目の完了確認で同じ結果（passed 数が同数以上・failed 0）と比較する。

### 0-c. 追加特性テスト（R-2 の前提。この項目0の中で作成しコミットする）

R-2 で `host_write` のパス検査を厳格化する前に、**現状の挙動を固定するテストは作らない**（現状の緩い検査は仕様ではなくバグであるため）。代わりに R-2 の期待挙動を先に RED テストとして書く。仕様:

- **テスト名**: `test_host_write_blocks_all_sensitive_patterns`（`tests.rs` の `sensitive_path_tests` の近くに追加）
- **内容**: `is_sensitive_path` が `true` を返す代表パス（`.ssh/id_rsa`, `certs/server.pem`, `Cargo.toml`）について、`is_sensitive_path(Path::new(...))` が `true` であることを再確認する単体アサーション（既存テストの補強）。host_write クロージャ自体は WASM フィクスチャなしに直接テストできないため、**R-2 では「クロージャが `is_sensitive_path` を呼ぶこと」をコードレビューと既存テストの GREEN 維持で担保する**。
- この時点では既存の `is_sensitive_path` テストが GREEN のままであること。

---

## 3. 作業項目リスト（実行順）

> 共通ルール: 1項目 = 1コミット。各項目の完了条件をすべて満たしてからコミットし、次に進む。行番号がズレている場合は 1-3 のアンカー文字列で再特定する。

---

### R-1: デッドコード削除（未使用の `reqwest::Client::new()`）

- **対象**: `libs/infrastructure/src/skills/mod.rs:1068`（`run_code_mode_js` 内、`aiome.fetch` 処理ブロック）
- **問題**: L1068 で `let client = reqwest::Client::new();` を生成しているが、実際のリクエストは L877-881 で生成済みの `http_client`（タイムアウト30秒付き）を L1077 で使用しており、`client` は一切使われない。タイムアウトなしクライアントの誤用リスクを将来に残す。
- **変更**: L1068 の1行を削除する。他は一切変更しない。

```rust
// 変更前（L1067-1069 付近）
                    let client = reqwest::Client::new();
                    let req_method = match method.to_uppercase().as_str() {

// 変更後
                    let req_method = match method.to_uppercase().as_str() {
```

- **完了条件**: `cargo check --workspace --tests` 成功、`cargo test -p infrastructure skills` が項目0と同数 PASS（特に `test_code_mode_js_success` と `test_code_mode_js_exec_success`）、`cargo clippy -p infrastructure --all-targets -- -D warnings` 成功。
- **リスク**: ほぼゼロ（未使用変数の削除）。失敗時は `git checkout -- libs/infrastructure/src/skills/mod.rs` で戻す。
- **依存**: 項目0

---

### R-2: `host_write` の機密パス検査を `is_sensitive_path` に統一（挙動変更あり・意図的）

- **対象**: `libs/infrastructure/src/skills/mod.rs:604-614`（`call_skill` 内の `host_write_fn` クロージャ）
- **問題**: `is_sensitive_path()`（L40-64）のドキュメントコメントは「パターンの追加はこの関数のみで管理する（DRY 原則）」と明記しているのに、host_write クロージャは独自のインライン検査を持ち、**`.env` / `.git` / `security.json` の3パターンしか見ていない**。`id_rsa`, `id_ed25519`, `.ssh`, `cargo.toml`, `*.pem`, `*.key` が WASM スキルからの書込で**素通りする**。これは宣言された設計意図との乖離＝セキュリティ検査の穴。
- **変更**: インライン検査ブロック全体を `is_sensitive_path` 呼び出しに置換する。

```rust
// 変更前（L604-614）
                                    let mut is_sensitive = false;
                                    for component in final_path.components() {
                                        if let std::path::Component::Normal(c) = component {
                                            if let Some(s) = c.to_str() {
                                                if s.starts_with(".env") || s == ".git" || s == "security.json" {
                                                    is_sensitive = true;
                                                    break;
                                                }
                                            }
                                        }
                                    }

// 変更後（1行）
                                    let is_sensitive = is_sensitive_path(&final_path);
```

- 注意: このクロージャは `move` で `spawn_blocking` 内にあるが、`is_sensitive_path` はモジュールレベルの `pub(crate) fn` なのでキャプチャ不要でそのまま呼べる。
- **完了条件**: `cargo check --workspace --tests` 成功、`cargo test -p infrastructure skills` 全 PASS（`test_wasm_skill_*` 3件が GREEN のままであること = 正常系スキル実行が壊れていない）、項目0-c のテスト GREEN、clippy 成功。
- **リスク**: **挙動が厳格化される**（従来書けた `Cargo.toml` 類似名や `.pem` への WASM 書込がブロックされる）。これは意図した変更である。もし既存の WASM スキルテストがこれにより FAIL した場合は、**assert を緩めず**（R-007）、失敗内容をユーザーに報告して中断する。戻し方: このコミットを `git revert`。
- **依存**: 項目0（0-c のテスト）

---

### R-3: `DUMMY_REGEX` の `loop {}` 除去（OP-053）

- **対象**: `libs/infrastructure/src/skills/mod.rs:162-193`（静的正規表現6本）
- **問題**: `DUMMY_REGEX` は `unwrap_or_else(|_| loop {})` で「パニック検出を回避するための意図的な無限ループ」を持つ。万一評価されると CPU 100% で沈黙する最悪の失敗モード（OPEN.md OP-053 / AGENTS.md R-005 精神への違反）。また5本の正規表現がエラー時に `DUMMY_REGEX.clone()` へフォールバックする迂遠な構造。
- **変更**: `LazyLock<Option<Regex>>` 化して `DUMMY_REGEX` を完全に削除する。パターンはすべてコンパイル時固定の有効な正規表現なので `None` 分岐は実際には発生しないが、発生してもパニックせず「マッチしない」に縮退する。

```rust
// 変更前
#[allow(clippy::empty_loop)]
static DUMMY_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("a^").unwrap_or_else(|_| loop {}));

static LOG_REGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| match regex::Regex::new(r#"aiome\.log\((.*)\);"#) {
        Ok(r) => r,
        Err(_) => DUMMY_REGEX.clone(),
    });
// （EXEC_REGEX / WRITE_REGEX / READ_REGEX / FETCH_REGEX も同形）

// 変更後
static LOG_REGEX: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r#"aiome\.log\((.*)\);"#).ok());
// （残り4本も同形。DUMMY_REGEX と #[allow(clippy::empty_loop)] は削除）
```

- 利用側（`run_code_mode_js` 内の5箇所）も合わせて変更する:

```rust
// 変更前
if let Some(caps) = LOG_REGEX.captures(line) {

// 変更後
if let Some(caps) = LOG_REGEX.as_ref().and_then(|r| r.captures(line)) {
```

（`EXEC_REGEX` L946, `WRITE_REGEX` L964, `READ_REGEX` L1003, `FETCH_REGEX` L1037 も同様に5箇所すべて）

- **完了条件**: `cargo check --workspace --tests` 成功、`cargo test -p infrastructure skills` 全 PASS（特に `test_code_mode_js_success` / `test_code_mode_js_exec_success` / `test_code_mode_js_security_violation_exec` / `test_code_mode_js_path_traversal_rejection` の4件 = JS ブリッジの全経路）、clippy 成功、`rg -n "DUMMY_REGEX|empty_loop" libs/infrastructure/src/skills/` が0件。
- **リスク**: 低（正規表現は不変、呼び出し形のみ変更）。失敗時はコミットを `git revert`。
- **依存**: 項目0。**R-4 より先に行うこと**（R-4 はこの新形式のまま正規表現を移動するため）。

---

### R-4: JS ブリッジを `code_mode.rs` に分離

- **対象**: `mod.rs` の静的正規表現5本（R-3 適用後の L162 付近〜）と `run_code_mode_js()`（アンカー: `pub async fn run_code_mode_js`、約265行）
- **問題**: WASM サンドボックス管理と疑似 JS インタープリタという2つの責務が同居し、mod.rs 肥大化の主因の一つ。さらに `expand_vars` / `unquote` / `resolve_token` の3ヘルパーが**行処理ループの内側で毎行定義**されており（アンカー: `let expand_vars = |s: &str`）、可読性を損ねている。
- **変更**:
  1. `libs/infrastructure/src/skills/code_mode.rs` を新規作成。
  2. 正規表現5本（`LOG_REGEX`〜`FETCH_REGEX`）を移動。
  3. `run_code_mode_js` の本体を `pub(super) async fn run_code_mode_js_impl(manager: &WasmSkillManager, js_code: &str, manifest: &crate::security::PermissionManifest) -> Result<...>` として移動。`manager` からは `manager.allowed_root` を参照するため、`WasmSkillManager` のフィールドは `pub(crate)` にせず、**`allowed_root` の値を引数で渡す設計にしてもよい**（実装が簡単な方を選んでよいが、公開 API は変えないこと）。
  4. `expand_vars` / `unquote` / `resolve_token` はループの外・`code_mode.rs` のモジュールレベル関数（`fn`）に昇格させる。ロジックは1文字も変えない。
  5. `mod.rs` 側にはシグネチャ不変の委譲メソッドを残す:

```rust
// mod.rs（変更後）
pub mod code_mode;

impl WasmSkillManager {
    /// code_mode.d.ts に準拠した JavaScript コードを一括ロード・安全に実行する JS エンジニアブリッジ
    pub async fn run_code_mode_js(
        &self,
        js_code: &str,
        manifest: &crate::security::PermissionManifest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        code_mode::run_code_mode_js_impl(self, js_code, manifest).await
    }
}
```

- **完了条件**: `cargo check --workspace --tests` 成功、`cargo test -p infrastructure skills` 全 PASS（JS ブリッジ4テストを含む）、clippy 成功、`wc -l libs/infrastructure/src/skills/mod.rs` が 850行以下。
- **リスク**: 中（約300行の移動）。ヘルパーのループ外昇格で変数キャプチャがなくなるが、3ヘルパーとも `variables: &HashMap` を引数で受けているため純関数化は機械的に可能。失敗時はコミットを `git revert`。
- **依存**: R-3（新しい regex 形式を確定させてから移動する）

---

### R-5: ホスト関数ビルダーを `host_fns.rs` に分離

- **対象**: `call_skill()` 内の `host_exec_fn`（アンカー: `let host_exec_fn = Function::new`、約45行）と `host_write_fn`（アンカー: `let host_write_fn = Function::new`、約90行）
- **問題**: `call_skill` が約300行あり、その過半が2つのホスト関数クロージャの構築。サンドボックス実行フローの骨格が読めない。
- **変更**:
  1. `libs/infrastructure/src/skills/host_fns.rs` を新規作成。
  2. 2つのビルダー関数を作る（クロージャの中身はコピーで、ロジックは1文字も変えない）:

```rust
// host_fns.rs（スケッチ）
use crate::security::{BastionGuard, PermissionManifest};
use extism::{Function, UserData, Val, ValType};
use std::path::PathBuf;

pub(super) fn build_host_exec_fn(permissions: PermissionManifest) -> Function { /* 現 L497-539 の中身 */ }

pub(super) fn build_host_write_fn(
    permissions: PermissionManifest,
    allowed_root: PathBuf,   // canonicalize 済みを渡す
    vault_path: Option<PathBuf>,
) -> Function { /* 現 L550-639 の中身 */ }
```

  3. `call_skill` 側は `let functions = vec![host_fns::build_host_exec_fn(...), host_fns::build_host_write_fn(...)];` に縮める。
  4. 注意: `allowed_root` の `canonicalize` 失敗時に `return Err(...)` する既存動作（アンカー: `Failed to canonicalize allowed_root`）は、ビルダー呼び出し**前**に `call_skill` 側で行い、成功した `PathBuf` だけをビルダーに渡す形にする（エラーの発生タイミングと文言を変えない）。
  5. `dry_run_skill` 内のスタブ版ホスト関数（アンカー: `[Layer 3 Deterministic Tracer]` 直後の `Function::new` 2つ）も `build_noop_host_fns() -> Vec<Function>` として同ファイルへ移してよい（任意。やる場合も同一コミット内）。
- **完了条件**: `cargo check --workspace --tests` 成功、`cargo test -p infrastructure skills` 全 PASS（特に `test_wasm_skill_timeout` / `test_wasm_skill_incident_on_error` / `test_wasm_skill_config_injection` / `test_dry_run_call_validation`）、clippy 成功。
- **リスク**: 中。クロージャの `move` キャプチャを引数化する際に、`metadata.as_ref().map(...)` 由来の権限クローンの取り忘れに注意（現状 host_exec と host_write は**別々に** `metadata` から permissions をクローンしている。この二重クローンは温存してよい）。失敗時はコミットを `git revert`。
- **依存**: R-2（host_write の検査統一が済んだ最終形を移動するため）、R-4（mod.rs の行番号が安定してから）

---

### R-6: 型定義を `types.rs` に分離し、mod.rs を「マネージャ本体 + 再エクスポート」に確定

- **対象**: `UnverifiedSkill` / `VerifiedSkill` / `SkillMetadata` / `SkillMaturity`（アンカー: `pub struct UnverifiedSkill` 〜 `impl std::fmt::Display for SkillMaturity` まで）
- **問題**: God Module の残り。型と実行エンジンの分離で OP-050 を完了させる。
- **変更**:
  1. `libs/infrastructure/src/skills/types.rs` を新規作成し、上記4型と impl をそのまま移動（`is_sensitive_path` は host_fns / code_mode 両方から使うため mod.rs に残す）。
  2. `UnverifiedSkill::verify` は `WasmSkillManager` を引数に取るため `use super::WasmSkillManager;` を付ける。また `verify` には `#[requires(...)]` 属性（契約プログラミング）が付いているため、`use contracts::requires;` も types.rs へ移すこと（mod.rs 側で不要になったら mod.rs からは削除する）。
  3. `mod.rs` 冒頭に再エクスポートを置き、**外部パスを完全維持する**:

```rust
pub mod types;
pub use types::{SkillMaturity, SkillMetadata, UnverifiedSkill, VerifiedSkill};
```

  4. `VerifiedSkill::promote_internal` は `pub(crate)` のため、types.rs へ移しても crate 内参照（`UnverifiedSkill::verify`）は壊れないことを確認する。
- **完了条件**: `cargo check --workspace --tests` 成功（**利用側 api-server / aiome-node のコンパイルが通ること = 再エクスポートが正しい証明。api-server のテスト実行までは不要**）、`cargo test -p infrastructure skills` 全 PASS、clippy 成功、`wc -l libs/infrastructure/src/skills/mod.rs` が **600行以下**。
- **リスク**: 低〜中（純粋な移動 + 再エクスポート）。`derive` マクロの `serde` 参照が types.rs でも解決できるか（`serde::Serialize` はフルパス derive のため問題なし）。失敗時はコミットを `git revert`。
- **依存**: R-4, R-5（移動の衝突を避けるため最後に実施）

---

### R-7: `immune_system.rs` の MockJQ を `testing` モジュールへ抽出（OP-055）

- **対象**: `libs/infrastructure/src/immune_system.rs:352-886`（`#[cfg(test)] mod tests` 内の `struct MockJQ` と 14 個のトレイト impl、約530行）
- **問題**: JobQueue 系トレイト群のフル Mock 実装がテストモジュール内に埋没しており、他のテストから再利用できない（OPEN.md OP-055）。なお `libs/test-utils` クレートは**存在しない**。新クレート作成は Cargo workspace への影響が大きいため行わず、**同一クレート内のテスト支援モジュール**として抽出する。
- **変更**:
  1. `libs/infrastructure/src/testing/mod.rs` と `libs/infrastructure/src/testing/mock_jq.rs` を新規作成。
  2. `libs/infrastructure/src/lib.rs` に `#[cfg(test)] pub(crate) mod testing;` を追加（`lib.rs` は数行の追加のみ。他の宣言に触らない）。
  3. `MockJQ` 構造体と全トレイト impl を `mock_jq.rs` へそのまま移動し、`pub(crate)` にする。`rules` フィールドも `pub(crate)` にする。
  4. `immune_system.rs` のテストモジュールは `use crate::testing::mock_jq::MockJQ;` で参照する。テスト本体（`887`, `896`, `978` 行付近の `MockJQ { rules: ... }` 構築）は変更しない。
- **完了条件**: `cargo test -p infrastructure immune_system` が項目0と同数 PASS、`cargo check --workspace --tests` 成功、clippy 成功。
- **リスク**: 低（`#[cfg(test)]` ゲートのためリリースバイナリへの影響ゼロ）。impl が参照するヘルパー型（`aiome_core_contracts::traits::*`）の `use` を mock_jq.rs 側に漏れなく移すこと。失敗時はコミットを `git revert`。
- **依存**: 項目0のみ（R-1〜R-6 と独立。並行不可、必ず R-6 の後に着手する — 同一クレートのコンパイルエラー切り分けを単純にするため）

---

### 実行順トレース検証（作成者による事前確認済み）

R-1（削除のみ）→ R-2（検査統一。R-4 で移動する前の場所で修正するため行番号前提が単純）→ R-3（regex 形式確定）→ R-4（R-3 の新形式ごと JS ブリッジを移動。R-2 は host_write 側で無関係）→ R-5（R-2 適用済みの host_write を移動。R-4 完了により mod.rs が縮んで作業視界が良い）→ R-6（最後に型を移動し再エクスポート確定）→ R-7（別ファイル、独立）。
各項目は前の項目の成果物を前提とするが、**前の項目を覆す変更は存在しない**ことを確認済み。

---

## 4. やらないことリスト（実行者への禁止事項）

1. **エラー型の統一をしない**: `Box<dyn Error + Send + Sync>` を `AiomeError` 等へ置換したくなるが、これは OP-051（ワークスペース横断課題）であり本計画のスコープ外。シグネチャは1つも変えない。
2. **JS ブリッジの機能追加・仕様変更をしない**: 正規表現ベースの解釈は本物の JS パーサに置換したくなるが、禁止。5命令の挙動・エラーメッセージ文字列も変えない（エラーメッセージはテストと API 消費者が依存している可能性がある）。
3. **依存クレートの追加・更新・削除をしない**: `Cargo.toml` は R-7 でも変更不要（同一クレート内モジュール追加のため）。
4. **`skill_arena.rs` / `forge.rs` / `hooks.rs` / `importer.rs` / `harness.rs` / `cleanroom.rs` / `discovery.rs` に触らない**。
5. **TypeState パターンを緩めない**: `VerifiedSkill` のフィールドを `pub` にする、`promote_internal` を `pub` にする等は、検証バイパスを可能にするため絶対禁止。
6. **テストの assert を緩めない・削除しない・`#[ignore]` にしない**（AGENTS.md R-007）。テストが落ちたらプロダクションコード側を疑い、解決できなければ中断・報告。
7. **`apps/api-server` / `apps/aiome-node` 側のコードを変更しない**: 再エクスポートにより変更不要のはず。変更が必要になった時点で計画の前提が崩れているので中断・報告。
8. **フォーマット目的だけの無関係な変更をしない**: `cargo fmt` は変更したファイルに対してのみ実行。
9. **`git push` をしない**: 全項目完了後もプッシュはユーザーの判断に委ねる。
10. **`commercial/` 配下に触らない**。

---

## 5. 実行者への指示文（このままコピペして渡す）

```
docs/roadmaps/refactor_skills_module_plan.md のリファクタリング計画を実行してください。

ルール:
1. 計画書の「項目0」から順に、記載された実行順（R-1 → R-2 → R-3 → R-4 → R-5 → R-6 → R-7）で1項目ずつ実施する。順序の入れ替え禁止。
2. 1項目完了するごとに、その項目の「完了条件」のコマンドをすべて実行し、期待結果を満たすことを確認してから、その項目のみを1コミットとして記録する（コミットメッセージは「refactor(skills): <項目ID> <要約>」形式）。
3. 完了条件を満たせない場合は、それ以上進まず、失敗した項目ID・実行したコマンド・エラー出力全文を報告して中断する。自己判断での回避策（assertの緩和、#[ignore]、スコープ外ファイルの修正）は禁止。
4. 計画書の「やらないことリスト」を必ず先に読み、全項目で遵守する。
5. 行番号は作業により変動するため、各項目に記載の「アンカー文字列」で対象を再特定してから編集する。
6. すべて完了したら、各項目のコミットハッシュ一覧と、最終の cargo test -p infrastructure の test result 行を報告する。git push はしない。
```

---

## 付録: 全体ロールバック手順

- 全体を破棄する場合: `git checkout main && git branch -D refactor/skills-god-module`
- 特定項目のみ戻す場合: 各項目は独立コミットなので `git revert <該当コミット>`（ただし R-4 以降を revert する場合は依存関係の逆順で revert すること: R-6 → R-5 → R-4）
