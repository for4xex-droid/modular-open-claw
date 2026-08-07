# Aiome Enterprise Edition — 切り出し計画（v0.1 / 計画フェーズ）

- **ステータス**: Draft（未承認・実装未着手）
- **作成日**: 2026-08-08（更新: 同日 — 仕分け結果と LoRA AutoTuner 活性化計画を統合）
- **前提**: 本書は計画書であり、コード変更は一切行っていない。実装には Human の明示的な「実装しろ」が必要。
- **関連**: [OP-089 OSS/Economy 二系統チャネル](op089_oss_economy_dual_channel_plan.md) / [value_10x_roadmap.md](value_10x_roadmap.md)（F-8 Agency）/ ADR-037（Cell-Based Architecture）/ [research_mechanism_triage.md](research_mechanism_triage.md)（研究由来メカニズム仕分け＋LoRA AutoTuner 活性化計画）

---

## 0. 目的

Aiome＋Nurture をユースケース別に整備し、**エンタープライズに特化した Aiome を独立プロダクトとして提供する**。
ただし「独立」= リポジトリのフォークではなく、**同一モノレポ内の第3ビルドチャネル（Enterprise）** として実現する。

## 1. 戦略決定（最重要）

### 採用案: 第3チャネル方式（フォークしない）

| 判断軸 | A. リポジトリをフォーク | B. 別バイナリ新設 | **C. 第3チャネル + Cargo feature（採用）** |
|---|---|---|---|
| セキュリティ修正の反映 | 二重適用（致命的） | 単一 | 単一 |
| 初期コスト | 低（コピーのみ） | 高 | 中 |
| 継続コスト | 極大（25万行の同期地獄） | 中 | **低** |
| 既存資産の再利用 | — | 部分的 | **OP-089 の仕組みをそのまま拡張** |

根拠:
1. **OP-089 で実証済み**: `scripts/desktop_sidecar_manager.py --channel {economy|oss}` と `--verify-channel-link`（cargo tree による Fail-Closed 依存検査）、CI の両チャネル link 検査が既に本番運用されている。`--channel enterprise` の追加は既存パターンの反復である。
2. **api-server は既に feature ゲート構造**（`nurture` / `nurture-cloud` / `dev-routes` / `dev-mock` / `demo`）を持ち、ルーター組み立ても bootstrap 層に集約されている。
3. エンタープライズの中核価値（Abyss Vault 鍵隔離・多層防御 L0-L4・監査ハッシュ鎖・WASM/Docker サンドボックス・CELL_ID 分離）は**コンシューマ機能と同じクレートの「安全層」として既に存在**しており、フォークで複製する意味がない。

### チャネル定義（OP-089 の表を拡張）

| | Economy（既定） | OSS（軽量） | **Enterprise（新設）** |
|---|---|---|---|
| api-server features | `nurture` | なし | `enterprise`（`consumer` を含まない） |
| commercial/ リンク | あり | なし | **なし（既定）**※ |
| コンシューマ機能（avatar/soul表現/biome/TTS/SNS/treasure） | あり | あり | **なし** |
| SSO(OIDC)・テナント管理・SIEM 監査出力 | なし | なし | **あり（新規開発）** |
| 配布形態 | Tauri Desktop | Tauri Desktop | **ヘッドレスサーバ（Docker/compose）中心** + 管理 UI |
| アセット名規約 | `AiomeOS-Economy-*` | `AiomeOS-OSS-*` | `AiomeOS-Enterprise-*` |

※ Nurture 経済圏（マーケット・コイン・DRM）はクリエイター/コンシューマ向け価値であり、エンタープライズ初期スコープから除外。OSS チャネルで「commercial/ 非リンクでも全体が動く」ことは実証済み。社内マーケット需要が出た場合のみ optional feature で再接続する。

## 2. 機能マップ（調査結果サマリ）

3つの調査（apps 棚卸し / libs・commercial 棚卸し / エディション・機能ゲート調査）の統合結果。

### 2a. 共通基盤（両エディションで共有 — 変更しない）

- `shared`（auth/JWT/RBAC, crypto, guardrails, sandbox, config, CELL_ID app_data）
- `key-proxy` + `abyss-vault`（鍵物理隔離）
- `infrastructure` の中核（llm/, job_queue/, skills/, security/, compliance/, audit_logger, workflow/, task_orchestrator/）
- `aiome-contracts` / `aiome-core-contracts` / `aiome-core`
- wasm-skills 3種、MCP 基盤、`aiome-migrate`

### 2b. コンシューマ専用（Enterprise では非リンク/無効化する対象）

| 機能塊 | クレート/モジュール | api-server ルート |
|---|---|---|
| アバター | `avatar-engine`（クレートごと optional 化） | `routes/avatar.rs`, `inochi2d.rs` |
| バイオーム | `biome-engine`（クレートごと optional 化） | `routes/biome.rs` |
| 人格・感情表現 | `soul` の somatic/expression 系、`core/expression` | `routes/soul.rs`, `expression.rs` |
| 音声/TTS | `infrastructure/tts`, whisper | `routes/voice.rs`, `whisper.rs` |
| SNS 投稿 | `infrastructure/publisher/`, `buzz/`, `channel_bridge/` | `routes/buzz.rs`, playbook(sns) |
| ゲーミフィケーション | agent_stats(level/exp), treasure, biome rarity | `routes/treasure.rs` |
| AI ギグ経済・転生 | gig_engine, samsara_engine, lora_marketplace | `routes/gig.rs`, `lora_market.rs`, `karma.rs`(federation) |
| Nurture 経済圏 | `commercial/*` 全体 | `nurture_mcp_proxy`, `commerce*`, `gift` |
| フェデレーション | `samsara-hub`, `aiome-node`(federation), commune | `routes/commune.rs`, `federation` |

**注意（切り離し難度）**: `infrastructure` → `soul` / `biome-engine` へのクレートレベル依存があるため、Phase 2 では「ルート層 + optional deps でのゲート」を先行し、`infrastructure` 内部の分離は段階的に行う（一括リファクタリングはしない）。

### 2c. エンタープライズ必須で既に存在するもの（磨くだけ）

- JWT + `Role`(Admin/User/System/Agent/Federated) + `Permission` の RBAC
- `AsyncAuditLogger` + SHA-256 ハッシュ鎖監査
- Abyss Vault / key-proxy / vault backend
- `CELL_ID` によるプロセス・DB 名前空間分離（マルチテナントの土台）
- compliance/（eKYC, BanStore, Quarantine, CSAM, GDPR/RTBF）
- OxiLean 形式検証ゲート、WASM/Docker サンドボックス

### 2d. エンタープライズのギャップ（新規開発が必要）

1. **SSO**: OIDC（Entra ID / Okta / Google Workspace）→ SAML は後続。現状 OAuth は MCP プロバイダ向けのみ
2. **テナント管理**: CELL_ID の上に「組織・メンバー・ロール割当」の管理 API/UI（F-8 Agency 計画と統合）
3. **監査エクスポート**: SIEM 連携（syslog/JSON Lines/S3）、保持ポリシー
4. **エディション一元化**: 散在する plan_id 文字列・WorkspacePersona・feature flags を単一の `Edition` 概念に統合
5. **管理コンソール enterprise ペルソナ**（既存 consumer/agency の WorkspaceMode を拡張）

## 3. フェーズ計画

各ステップに検証ポイントを明示する（Goal-Driven Execution）。

### Phase 0: 意思決定の固定（0.5日）
- [ ] ADR 新規作成「Enterprise は第3チャネルとして実装、フォーク禁止」（本書 §1 の表を根拠に）
- [ ] エンタープライズ初期スコープの確定（Nurture 除外、SSO は OIDC 先行、配布はヘッドレス優先）
- **検証**: ADR が `docs/decisions/` に存在し、Human 承認済みであること

### Phase 0.5: コードベース健全化（仕分け結果の実行 / 0.5〜1日）

> 詳細は [research_mechanism_triage.md](research_mechanism_triage.md)。Feature ゲート導入（Phase 2）の前に死コードを除去し、ゲート対象を最小化する。

- [x] カテゴリA（参照ゼロの残骸 約30件）の除去（**実行済み 2026-08-08**）
- [x] B-3 のうち `gig_marketplace` トグルの削除（**実行済み 2026-08-08**）
- [x] B-3 の `seo_publish` / `p2p_federation`: **結線完了（2026-08-08）**。手順書 [feature_flag_wiring_plan.md](feature_flag_wiring_plan.md)
- [ ] B-2（x402）・B-4（STT）は保持を確定済み — Phase 2 で feature ゲート下に編入
- **検証**: `cargo check --workspace --tests` + `tsc --noEmit` パス（2026-08-08）。`cargo test --workspace` は実行中

### Phase C1: LoRA AutoTuner 活性化（コンシューマ側整備 / 本計画と並行実施可 / 約3日）

> Human 決定（2026-08-08）: B-1 は削除ではなく活性化。詳細な4段階計画（訓練実体化 → loss 収集 → 提案結線 → 自律ループ化）と検証基準は [research_mechanism_triage.md](research_mechanism_triage.md) の「LoRA AutoTuner 活性化実装計画」を正本とする。

- [ ] 段階1: `mlx_train.py` の実訓練化＋`LOSS` 行出力（Grok 委譲）
- [ ] 段階2: `lora_training.rs` の loss パーサ＋JobQueue 永続化
- [ ] 段階3: `suggest_hyperparams()` の結線（優先順位: UI 指定 > 提案 > 既定）＋ダッシュボード表示
- [ ] 段階4: 停滞検知 → AutoTune 再訓練 → （それでも停滞なら）Samsara 転生の順序化。Settings `lora_training` トグルを実ゲート化
- **エディション帰属**: 成果物はすべて `consumer` feature 側。enterprise ビルドには含めない
- **検証**: 停滞履歴で次回 LR が2倍（Positive）/ 履歴3件未満で不変（境界）/ トグル OFF で自律再訓練が積まれない（Negative）

### Phase 1: 境界の機械的可視化（1〜2日 / Grok 委譲比率: 高）
- [ ] api-server 全ルート（60超）を `common / consumer / enterprise / dev` に分類した表を作成（`docs/architecture/EDITION_ROUTE_MATRIX.md`）
- [ ] `cargo tree` ベースの依存監査: `avatar-engine` / `biome-engine` / `soul` / `commercial/*` に到達する経路の全列挙
- [ ] `.env.example` の環境変数をエディション別に分類
- **検証**: 分類表の全ルートが実コード（`routes/mod.rs` のルーター登録）と 1:1 対応していることを機械的照合スクリプトで確認

### Phase 2: Feature ゲート導入（3〜5日 / Grok 委譲比率: 中）
- [ ] api-server に `consumer` feature を新設し、2b のルート群・依存クレートを `#[cfg(feature = "consumer")]` + optional deps でゲート。既定 (`default`) には `consumer` を含め、**既存ユーザーへの挙動変化ゼロ**を保証
- [ ] `enterprise` feature 新設（この時点では空でよい。ゲートの器を先に作る）
- [ ] CI にビルドマトリクス追加: `--features consumer,nurture`（=Economy）/ `--no-default-features`（=OSS 相当）/ `--no-default-features --features enterprise` の 3 構成で `cargo check --workspace --tests`
- **検証（Negative Test 必須）**:
  - Positive: consumer ビルドで `/api/avatar` が 200 系
  - Negative: enterprise ビルドで `/api/avatar`・`/api/biome`・`/api/buzz` が **404** であり、`cargo tree` に `avatar-engine` / `biome-engine` / `nurture-api` が**存在しない**こと
  - Revert: default ビルドが従来と完全同一の挙動であること（既存テスト全パス）

### Phase 3: Enterprise チャネルの配布物化（2〜3日 / Grok 委譲比率: 中）
- [ ] `scripts/desktop_sidecar_manager.py` に `--channel enterprise` を追加し、`--verify-channel-link` の Fail-Closed 検査を拡張（enterprise = nurture-api 非リンク **かつ** avatar-engine 非リンク）
- [ ] `docker-compose.enterprise.yml` 新設（api-server[enterprise] + key-proxy + PostgreSQL。Ollama/クラウド LLM は既存 Pattern B のまま）
- [ ] `docs/guides/DESKTOP_CHANNELS.md` を 3 チャネル対応に更新、`docs/guides/ENTERPRISE_DEPLOYMENT.md` 新設
- **検証**: CI で 3 チャネルすべての link 検査がパス。enterprise compose がクリーン環境で起動し `/health` 応答

### Phase 4: ギャップ機能の開発（2〜4週 / Grok 委譲比率: 低）

> ⚠️ **Safety-Critical Zone**: `auth.rs` / `AiomeCustomClaims` / key-proxy はエージェント自律変更禁止領域。本フェーズの実装は項目ごとに Human の明示的な「実装しろ」を得てから着手する。

優先順（営業インパクト順）:
1. **OIDC SSO**（既存 `AuthManager` に IdP バックエンドを追加。JWT claims は OIDC 準拠形状のため親和性が高い）
2. **テナント管理 API/UI**（CELL_ID 分離の上に組織モデル。F-8 Agency の h3 計画と統合し二重実装を回避）
3. **監査 SIEM エクスポート**（`AsyncAuditLogger` に出力シンク追加。ハッシュ鎖はそのまま）
4. SAML（OIDC で満たせない案件が出たら着手）
- **検証**: 各機能ごとに Positive / Negative（例: 未割当テナントのユーザーが他セルの API に 403）/ Revert の 3 段階

### Phase 5: ドキュメント・整合同期（1日 / Grok 委譲比率: 高）
- [ ] CHANGELOG / README（+ README_en）/ RIPPLE_MAP / SYNERGY.md / .env.example の同期
- [ ] `AIOME_NURTURE_SYNERGY.md` にエディション境界図を追加
- **検証**: Documentation Sync Rule チェックリスト全項目

## 4. Grok 4.5 サブエージェント委譲方針

| タスク種別 | 担当 | 例 |
|---|---|---|
| 列挙・分類・照合（単純） | **Grok 4.5** | ルート×エディション分類表、cargo tree 経路列挙、env 変数分類、ドキュメント同期の下書き |
| 定型ゲート挿入（パターン反復） | **Grok 4.5**（メインがレビュー） | `#[cfg(feature)]` の機械的付与、CI マトリクス YAML、compose ファイル雛形 |
| アーキテクチャ判断・依存切断 | メインエージェント | optional deps 設計、bootstrap ルーター再構成、infrastructure 内部の soul 依存整理 |
| Safety-Critical Zone | **Human 許可 + メインエージェント** | OIDC SSO、テナント認可、key-proxy 変更 |

## 5. リスクと対策

| リスク | 対策 |
|---|---|
| `infrastructure` → `soul`/`biome-engine` の深い結合で feature ゲートが波及爆発する | Phase 2 はルート層ゲート優先。クレート分離は計測（cargo tree + ビルド時間）で効果が確認できた箇所のみ後続 OP 化 |
| feature フラグの組合せ爆発 | CI マトリクスを 3 構成に固定（自由な組合せを保証しない旨を ADR に明記） |
| F-8 Agency と Enterprise の二重開発 | Phase 4-2 で h3_f8_f10 計画と統合。Agency=マネージド/SMB、Enterprise=セルフホストという位置づけを ADR で固定 |
| コンシューマ既定ビルドのリグレッション | Phase 2 の Revert 検証（default = 従来挙動）を DoD に含める |

## 6. DoD（全体）

- [ ] 3 チャネル（Economy / OSS / Enterprise）が CI で恒常的に `cargo check --workspace --tests` パス
- [ ] Enterprise 成果物の依存ツリーに `avatar-engine` / `biome-engine` / `nurture-*` が存在しない（Fail-Closed 検査）
- [ ] Negative Test: enterprise ビルドでコンシューマ系ルートが 404
- [ ] OIDC SSO + テナント管理 + SIEM 監査出力が Positive/Negative/Revert の 3 段階検証済み
- [ ] Documentation Sync Rule 全項目完了
