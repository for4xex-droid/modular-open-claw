# 研究・外部リポジトリ由来メカニズムの仕分け（Triage v0.1）

- **ステータス**: Draft（仕分け完了・除去は未実行）
- **作成日**: 2026-08-08
- **判定軸**: Economy / OSS / Enterprise の3チャネル（[enterprise_edition_plan.md](enterprise_edition_plan.md)）のいずれかの**本番経路に結線され、実際に機能しているか**
- **調査方法**: 3系統の機械的調査（研究由来16メカニズムの結線追跡 / 既定無効・孤立機能の検出 / ルート直下残骸の参照照合）

---

## 判定サマリ

| 区分 | 件数 | 方針 |
|---|---|---|
| A. 即削除可（参照ゼロの残骸・死コード） | 約30件 | **削除実行済み（2026-08-08）**: A-1 trash / A-2 git rm / A-3 コード削除。cargo check + tsc 検証パス |
| B. 未結線の実装 | 4件 | **決定済み（2026-08-08）**: B-1 活性化 / B-2・B-4 保持 / B-3 は `gig_marketplace` **削除実行済み**・`lora_training` 蘇生・`seo_publish` / `p2p_federation` **結線完了**（[feature_flag_wiring_plan.md](feature_flag_wiring_plan.md)） |
| C. 実装済みだが既定無効・キー未設定で空回り | 約15件 | 削除せず、エディション別 feature/flag に整理（Phase 2 でゲート） |
| D. 本番結線済み・機能中 | 11件 | 保持 |

---

## A. 即削除可（参照ゼロ確認済み）

### A-1. ルート直下の未追跡残骸（`trash` で除去。git 影響なし）

| 対象 | 根拠 |
|---|---|
| `tla2tools.jar`（4MB 現物） | CI（`formal-verify.yml`）は都度ダウンロードするためローカル現物は不要 |
| `test_db`, `test_logic`（Mach-O バイナリ） | どこからも参照なし |
| `debug_shield.rs` | クレート外の実験ファイル |
| `scratch_scan.py`, `scratch_scan2.py` | 参照なし |
| `audit.json`, `audit2.json`, `full_hack_scan.txt`, `full_todo_scan.txt`, `errors.txt` | 過去スキャンの出力 |
| `test_output.log`, `test_pg.log`, `hub.log`, `node_a.log`, `node_b.log` | ログ残骸（swarm テスト再実行で再生成される） |
| `hub_test.db`, `samsara_hub.db`, `tests.db`, `database.sqlite` | テスト/ランタイム残骸（テスト・本番は各自のパスに再作成） |
| `test_forge.sh`, `build_check.sh`, `clippy_fix.sh` | CI/scripts から未呼び出し。`clippy_fix.sh` は危険なワンショット |
| `workspace_a/`, `workspace_b/`, `scratch/`, `.intent_tmp/`(空), `.temp_puppeteer/`, `.tmp/` | swarm テスト残骸・空ディレクトリ（`.intent_tmp` の**パス名概念**は `AIOME_DATA_DIR` 配下で本番使用のため、ルートの空現物のみ除去） |
| `USER.md.bak` | バックアップ残骸 |

### A-2. git 追跡中のワンショット（`git rm` で除去）

`test_db.rs`, `test_logic.rs`, `test_extract_types.rs`, `split_bootstrap.py`, `update_mocks.py`, `update_tests.py`, `generate_jwt.js`, `test_json.sh`
— いずれもワークスペース非所属・参照ゼロ。`update_*.py` はソースを書き換える危険なワンショットであり残置リスクの方が高い。

### A-3. 死んだ設定・死コード（コード変更を伴う）

| 対象 | 根拠 |
|---|---|
| `MCP_SKILL_MD_INJECTION`（`shared/config.rs` の `skill_md_injection` フィールド + `.env.example`） | 設定の読み手が存在しない |
| `PROMETHEUS_METRICS_ENABLED`（`.env.example` のみ） | コード参照ゼロ |
| `KANI_STUB_MODE`（`.env.example`。ADR-041 で廃止済み） | 実行時参照ゼロ（コメントのみ） |
| `libs/aiome-contracts/src/trellis2.rs` | `lib.rs` に `mod` 宣言がなくコンパイル単位外の孤立 DTO |

---

## B. 未結線の実装（削除可否の検証完了: 2026-08-08）

検証方法: 全参照の機械的列挙＋B-1 はコンパイル実験（Positive: 除去状態で `cargo check --workspace --tests` パス / Negative: 結線済み `samsara_engine` を同手順で除去すると api-server で E0433 検知 / Revert: diff ゼロ復元）。
**教訓**: 単体クレートの `cargo check -p` では削除可否を証明できない（消費者は別クレートにいる）。必ずワークスペース全体で検証すること。

| # | 対象 | 検証結果 | 削除時の接点 |
|---|---|---|---|
| B-1 | **LoRA AutoTuner** | **削除可（コンパイル実験で実証済み）** | 3点のみ: `infrastructure/src/lora_autotuner.rs` / `lib.rs:236` の mod 宣言 / `tests/lora_integration_test.rs` |
| B-2 | **x402** | **削除可（読み手ゼロを確定）**。`x402_negotiator` は AppState への代入のみで読み取り箇所なし。`PaymentProof` 型も x402 専用 | 契約: `aiome-contracts/src/x402.rs`・core-contracts 再エクスポート / 実装: `aiome-commerce/src/x402.rs`・`x402_factory.rs`・`lib.rs` / api-server: `core_services.rs:90-110`・`bootstrap/mod.rs:107`・`state_assembly.rs:72`・`app_state.rs:103`・`api_integration_tests/common.rs:410` / **要修正**: `aiome-node/src/routes/agent_card.rs:53,93` が `protocol: "x402"` を外部広告（残すと虚偽広告） / env: `X402_*` 4変数 |
| B-3 | **UI ハリボテ4フラグ** | **削除可（UI のみの存在を確定）**。`seo_publish` / `lora_training` / `gig_marketplace` / `p2p_federation` はいずれも Rust 側 `is_feature_enabled` ゲートが存在しない。**`federation_v1_5`（backend metrics sync）は別機能で削除対象外** | `SettingsPage.tsx:607-635` の FeatureToggle 4ブロック / `i18n/en.json`・`ja.json` の対応キー（378, 380-382） / （任意）`routes/settings.rs:778-779` のテスト例示 |
| B-4 | **STT / Whisper** | **条件付き削除可**。本番呼び出しは MCP `transcribe` のみ。ただし `TranscriptionSegment` 型を `avatar-engine/lip_sync.rs` が利用するため**型の残置または独自型化が必須**（エンジン削除だけでは lip_sync が壊れる） | `infrastructure/whisper_transcription.rs` / `lib.rs:200` / contracts `traits.rs:133-168`（型は要移設判断） / api-server: `core_services.rs:697-709`・`bootstrap/mod.rs:137`・`state_assembly.rs:113`・`app_state.rs:10,137`・`mcp/server.rs`（登録・実行・whitelist） / `.env.example:351` / ※`routes/whisper.rs` は Soul モノローグ用で **STT 無関係・削除禁止** |

### B の確定判断（Human 決定: 2026-08-08）

| # | 決定 | 内容 |
|---|---|---|
| B-1 | **保持 → 活性化** | 削除せず、下記「LoRA AutoTuner 活性化実装計画」により本番結線する |
| B-2 | **保持** | GRAND_ROADMAP Phase 2（スウォーム経済）の部品として維持。エディション整理時に feature ゲート化 |
| B-3 | **一部削除・一部結線** | `gig_marketplace` トグルは**削除実行済み（2026-08-08）**。`seo_publish` / `p2p_federation`（→`federation_v1_5`）は**結線完了（2026-08-08）**。**`lora_training` トグルは削除せず、AutoTuner 自律再訓練の実ゲートとして結線（活性化計画の段階4）** |
| B-4 | **保持** | 既定無効のまま consumer feature ゲートへ編入（エンタープライズ計画 Phase 2） |

---

## LoRA AutoTuner 活性化実装計画（B-1 の結線）

**背景**: AutoTuner（停滞→LR倍増 / 発振→LR半減 / 過学習→rank・epoch削減）は実装・テスト済みだが、入力となる `loss_history` を生む訓練パイプラインの中核が未完成。`scripts/mlx_train.py` はスタブ（実訓練なし・loss 出力なし）で、`lora_training.rs` にも loss の解析・保存処理がない。`train()` 内の `from_params` 直前には「(or Autotuner)」というコメント付きの結線予定地が既に存在する。

**方針**: 入力（loss）→ 保存 → 提案 → 自律化 の順に、各段階を独立に検証可能な形で実装する。

### 段階1: 訓練の実体化（mlx_train.py）
- `mlx_lm` の LoRA fine-tune を実装し、イテレーションごとに機械可読形式 `LOSS <iter> <value>` を stdout に出力
- `mlx_lm` 不在時は現行どおり STUB モード（CI 互換維持。ただし STUB でも合成 loss を出力しパーサをテスト可能にする）
- **検証**: STUB モードで `LOSS` 行が出力される / 実機（mlx_lm あり）でアダプタが実際に生成される

### 段階2: loss 収集（lora_training.rs）
- 既存の stdout ストリーム処理（P-21）に `LOSS` 行パーサを追加
- 完走時に `loss_history` と使用ハイパーパラメータをジョブメタデータ（JobQueue）へ model_family 単位で永続化
- **検証**: 訓練完了ジョブのメタデータに `loss_history` が記録される（ユニットテスト＋統合テスト）

### 段階3: 提案の結線（train() / routes/lora.rs / UI）
- `from_params` 直前で同一 model_family の前回走行の `loss_history`＋パラメータを取得し `suggest_hyperparams()` を呼ぶ
- **優先順位: UI 明示指定 > AutoTuner 提案 > 既定値**（ユーザー意思を上書きしない）
- API レスポンスに提案値と理由（stagnation / oscillation / overfitting）を含め、LoRA ダッシュボードに表示
- **検証（Goal-Driven）**: E2E — 停滞 loss 履歴を持つ状態で次回訓練の `learning_rate` が2倍になっている（Positive）/ 履歴3件未満では前回値のまま（境界）/ UI 明示指定時は提案が無視される（優先順位）

### 段階4: 自律ループ化（Dream / Watchtower / Settings）
- 既存の ScoreTracker 停滞検知（TimesFM プラトー予測 → Samsara 転生）の**手前**に「AutoTune 再訓練を先に試す」段階を挿入（転生はより破壊的な最終手段のため）
- Settings の `lora_training` トグル（現在無機能）を `is_feature_enabled("lora_training")` で自律再訓練のゲートとして結線。**既定 false**（ユーザーが明示的に有効化した場合のみ自律再訓練）
- **検証**: フラグ OFF で停滞検知しても再訓練ジョブが積まれない（Negative）/ ON で積まれる（Positive）

### 委譲・工数

| 段階 | 担当 | 見積 |
|---|---|---|
| 1 | Grok 4.5（Python 定型・メインがレビュー） | 0.5日 |
| 2 | メインエージェント（ストリーム処理・JobQueue 契約に接触） | 0.5日 |
| 3 | メインエージェント（API/UI 契約変更を含む） | 1日 |
| 4 | メインエージェント（進化ループは影響範囲大。RIPPLE_MAP 事前確認必須） | 1日 |

### エディションとの関係
自己進化系＝コンシューマ向け価値のため、成果はすべて **consumer feature 側**に属する（enterprise ビルドには含めない）。エンタープライズ計画 Phase 2 の feature ゲート導入時に、本計画で追加したコードも `consumer` ゲート下に置くこと。

---

## seo_publish / p2p_federation の結線 vs 削除 検証（2026-08-08）

### 検証結果

| 観点 | `seo_publish` | `p2p_federation` |
|---|---|---|
| 対応する実機能 | **あり**: WordPress 自動投稿パイプライン（`PublishPipeline` → key-proxy 経由 WP 投稿）。ただし現在 `seo_content` ジョブの**生産者が未配線**で実投稿は休眠中 | **あり**: `federation_v1_5` ゲート（`main.rs:120`）が karma / immune rules / arena の hub 定期 push & sync を制御。**migration で既定 ON** |
| 現在の制御手段 | env のみ（`WP_SDK_ENABLED` / `WP_API_URL` / `GEO_ENABLED`）。ユーザーが UI から止められない | ゲートは実在するが UI キーが `p2p_federation` で**名前不一致**のためトグルが無機能 |
| 結線コスト | 中（3〜5ファイル）: `PublishPipeline::run_job` に `SettingsOps` を DI し1ゲート追加 | **極小（1〜3ファイル）**: UI キーを `federation_v1_5` に統一するだけ（ゲートは実装済み） |
| 結線のメリット | **外部送信のキルスイッチ**。自律 SNS/CMS 投稿は Safety-Critical Zone #5 に該当し、ユーザーが UI で止められることは信頼の要件。将来 publisher が増えても `run_job` が単一関門 | **プライバシー制御**。karma・immune rules は行動データであり、hub への共有が**既定 ON なのに UI から止める手段がない**現状はダークパターンに近い |
| 削除のメリット | UI の正直さ（生産者未配線の間は「効かないトグル」の誤解を排除） | 同左 |

### 判定: **両方とも結線を推奨**（削除よりメリット大）

1. **`p2p_federation` → 結線（ほぼ無償）**: ゲートは既に本番で動いており、直すのはキー名だけ。既定 ON のデータ共有をユーザーが止められるようになるプライバシー価値は、削除の「正直さ」を大きく上回る。ラベルは実態に合わせ「Karma フェデレーション同期」へ変更（「P2P 全体」と誤解させない）。
2. **`seo_publish` → 結線（外部送信は fail-closed 原則）**: 生産者が未配線の今こそ、投稿経路の単一関門（`PublishPipeline::run_job`）にゲートを敷設する最適タイミング。既定は **OFF（fail-closed）** とし、外部送信はユーザーの明示的同意で開く設計にする。削除して将来再実装するより、休眠中に敷設する方が安全かつ安価。

### 実装時の要点（承認後）

**正本**: [feature_flag_wiring_plan.md](feature_flag_wiring_plan.md)（2026-08-08 作成。全前提を実コードで検証済み。before/after スニペット・テストコード・検証コマンド・落とし穴チェックリスト完備）

- `seo_publish`: `PublishPipeline` に `Option<Arc<dyn SettingsOps>>` を追加し `run_job` 冒頭でゲート、既定 OFF（fail-closed）。`JobQueue: SettingsOps` 継承（traits.rs:696-708）により `job_queue.clone()` で DI 可能
- `p2p_federation`: UI キーを `feature_flag.federation_v1_5` へ統一（backend 改名は migration・テスト波及のため不採用）。ラベルは「Karma フェデレーション同期」
- いずれも consumer feature 帰属（enterprise ビルドでは publisher / federation ごと除外）

---

## C. 保持するが整理対象（エディション計画 Phase 2 でゲート）

削除しない。キー未設定時は既に no-op であり「きちんと機能する形」を壊していないため、除去よりゲート整理が適切。

- **外部キー依存**: Discord / Telegram（Watchtower 橋）、X トレンド、WordPress 投稿、Firecrawl / Exa / BrightData MCP、fal.ai、Tremendous、Polar
- **サイドカー**: geo-optimizer / timesfm-sidecar（Docker 本番のみ・Tauri 非同梱 — 現状の配布実態と一致しており正常）
- **DB フィーチャーフラグ**: `federation_v1_5` / `a2ui_generative_ui` / `js_fallback`（既定 false の正常なゲート）
- **stripe-webhook-forwarder/**（Cloudflare Workers 単独。配布物外だが本番 Webhook 経路の部品）
- **要注意バグ（別途修正候補）**: `WP_SDK_ENABLED` が「変数の存在だけで true」になる実装（`.env.example` の `"false"` が効かない）

## D. 保持（本番結線済み・機能中）

Lenia/Biome、TimesFM、Society of Thought（Oracle 経由）、DreamState、Samsara Engine、OxiLean（証明書＋poller＋shadow-worker）、Kani（Dream/Aegis）、GEO、TrendSonar、Poincare GC（Watchtower）、TLA+（CI 専用）

※ HDC / Active Inference は**未実装**（GRAND_ROADMAP Phase 3 の計画記載のみ）のため、削除対象自体が存在しない。

---

## 実行時の検証プロトコル

1. **Positive**: 削除後 `cargo check --workspace --tests && cargo test --workspace` がベースラインと同等にパス
2. **Negative**: A-3 の削除対象（例: `trellis2`）を参照するコードを一時的に追加し、コンパイルが**失敗する**ことで「本当に孤立していた」ことを確認 → 戻す
3. **Revert & Report**: `git status` がクリーン（意図した削除のみ）であることを確認し報告

## ドキュメント同期（除去実行時）

CHANGELOG [Unreleased] / RIPPLE_MAP / `.env.example`（A-3 の3変数削除）
