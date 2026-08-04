# 開発ホスト Egress 衛生 導入計画（v1.2）

- **作成日**: 2026-07-31（v1.0） / **改訂**: v1.1 → **v1.2**（再照合: シンボル名・D2 挿入点・OS スコープ・seatbelt 混同排除）
- **ステータス**: **H0 承認済（2026-07-31）**。D1+D2 ✅。**残 = H1（Human 実機）**
- **ID**: **OP-095**（`OPEN.md` P2 起票済）
- **目的**: 安心・安全を損なわず、開発速度と本番価値（Live 課金・Vault・Agentic）を同時に最大化する
- **根拠**: macOS outbound 監視議論 + 2026-07-31 コード／docs **二回目**物理照合
- **正本関係**: タスク ID は承認後 `OPEN.md`。製品セキュリティ正本は [`SECURITY_DESIGN.md`](../architecture/SECURITY_DESIGN.md)。本計画は **ホスト層（darwin）のみ**

---

## 0. 結論（何をやるか / やらないか）

### 価値仮説

| 価値 | メカニズム |
|---|---|
| **安心** | Vault / Stripe Live を扱う **開発 Mac** で、未知外向きを人間が一度は見る |
| **安全** | 製品内制御（Immune / SSRF / Vault / MCP）の **補完**。代替ではない |
| **効率** | ガイド 1 枚 + 入口 1 行×2。製品コード・SECURITY_DESIGN・seatbelt 仕様の再記述をしない |
| **価値最大化** | OP-064 / Phase E / Upstream を止めず、秘密作業の摩擦だけ下げる |

### やる（v1.2 確定スコープ）

| 順 | Wave | ID | 内容 | 担当 | コード |
|---|---|---|---|---|---|
| 1 | **H0** | OP-095-H0 | 本計画 **v1.2** 承認 | Human | なし |
| 2 | **H1** | OP-095-H1 | **macOS のみ** LuLu（最低）または Little Snitch + P/N/R | **Human** | なし |
| 3 | **D1** | OP-095-D1 | `docs/guides/DEV_HOST_EGRESS.md`（ホスト運用のみ） | Agent | docs |
| 4 | **D2** | OP-095-D2 | 危険作業入口 **2 ファイル×各 1 行**（挿入点は §2 D2 で一意） | Agent | docs |

**最小クローズ** = H0+H1+D1+D2。

**OS スコープ**: H1 は **darwin / macOS**。Linux・Windows 開発機は本 OP の必須対象外（任意の OS 標準 FW は個人判断。ガイドに手順を増やさない）。

### やらない / 降格（確定）

| 項目 | 判定 | 根拠（実体） |
|---|---|---|
| Wave **A1** 棚卸し | **廃止**（§1 に固定） | SECURITY_DESIGN 再記述は重複 |
| Wave **P1** reason_code | **既定アウト** | OP-093 済（下記シンボル一覧） |
| 製品 FW / 新クレート | 禁止 | `SecurityPolicy`+`ShieldClient` / BastionGuard / MCP WL |
| README 必須更新 | 禁止 | D2 の 2 ファイルのみ接続 |
| `nt_gate.py` に FW ゲート | 禁止 | 非決定的・秘密非対象 |
| OPERATIONS §6 Monitoring | 接続禁止 | Shadow/gRPC 監視でありホスト FW ではない |
| MCP `marketplace_buy` | 触らない | NR-09 クローズ。`is_skill_whitelisted` に無し → `_ => false` |
| OP-068 / Upstream deny | 非吸収 | 別レーン |
| 製品同梱・CI ホスト FW | 禁止 | — |
| DEV_HOST に **seatbelt / sandbox-exec** ルール転記 | 禁止 | `libs/shared/src/sandbox/seatbelt.rs` は製品実行層。ホスト FW と別物 |
| `assert_url_safe` 名での文書化 | 禁止 | **実シンボルは `assert_resolved_url_safe`**（v1.1 誤記を v1.2 で訂正） |

### レーン分離

| レーン | 本計画 |
|---|---|
| Human 衛生（macOS outbound） | **本線** |
| Docs（ガイド + 入口 1 行） | H0 後 Agent |
| 製品ランタイム | **スコープ外**（P1 は復活条件のみ） |
| SECURITY_DESIGN / Immune / Vault / seatbelt | **リンクのみ。改変・転記禁止** |

---

## 1. 実コード照合サマリ（2026-07-31・第2回）

> 旧 A1 の成果物。D1 はホスト手順のみ。製品側は本表へリンク（コピーしない）。

### 1.1 層マップ

```
[開発 Mac: LuLu / Little Snitch]     ← OP-095 唯一の新レイヤ（darwin）
        ↓
[供給] .npmrc ignore-scripts / CI npm audit / cargo deny·audit（OP-068 別）
[秘密] ALLOWED_VAULT_SECRETS + fetch_and_inject_secrets → key-proxy
[実行] BastionGuard + SandboxProfile + seatbelt（製品内・転記禁止）
[ツール] ToolCallRouter + Immune FC + OP-093 reason_code
[MCP]  is_skill_whitelisted
[URL]  SecurityPolicy::validate_url  ≠  WorkflowValidator::assert_resolved_url_safe
```

### 1.2 物理アンカー（第2回で訂正した箇所を含む）

| 資産 | 実体 | 事実（照合） | OP-095 |
|---|---|---|---|
| SSRF Shield | `libs/shared/src/security.rs` `validate_url` | loopback **8188/11434 のみ** + `trends.google.co.jp` + `COMMUNE_HUB_WHITELIST` + `block_private_ips(true)` | 改変禁止・ガイド非複製 |
| Shield 本体 | 同ファイル `ShieldClient` | 製品側ネットシールド既存 | ホスト FW と二重実装しない |
| wf_http SSRF | `libs/infrastructure/src/workflow/validator.rs` **`assert_resolved_url_safe`** → `check_resolved_url_ssrf`；実行時呼出 `task_orchestrator/workflow_runtime.rs` | **計画名 `assert_url_safe` は不存在**（w2 計画の設計時名称）。`SecurityPolicy::validate_url` は wf 経路で使わない | D1 注記は **実関数名**のみ |
| Tool deny | `apps/api-server/src/tool_call_router.rs` | 実コードの reason_code: `guardrail` `sentinel` `immune_db_error` `moe_culling` `path_traversal` `ssrf` `robots_txt` `hook_deny` `hook_ask` `mcp_suspended` `mcp_billing_db_error` `mcp_validate_denied` `hook_post_deny` `hook_post_ask` | 「reason_code 追加」P1 は再発明 |
| MCP WL | `apps/api-server/src/mcp/server.rs` L391–407 | `terminal_exec`/`fs_writer`/`forge_publish`→false。`marketplace_buy` 無し | NR-09 再開禁止 |
| Bastion | `bastion_guard.rs` | **Strict** = no net/write。ForgeBuild / BrowserAgent / Default 系は net 可。`check_network` = manifest | 「常に no net」禁止 |
| Seatbelt | `libs/shared/src/sandbox/seatbelt.rs` | `allow_network==false` で `deny network*`。LoRA 特例で 80/443 outbound | **DEV_HOST に書かない** |
| Shadow 5-Layer | SECURITY_DESIGN Phase 43 | DockerConductor + **BastionGuard Strict**（read-only / no net by default） | リンクのみ。ホスト FW と混同しない |
| Vault | `ALLOWED_VAULT_SECRETS` + `fetch_and_inject_secrets`（`KEY_PROXY_URL`） | key-proxy / abyss-vault が WL 参照 | ガイドに秘密値禁止。「exact endpoint」等の未検証スローガンをガイドに書かない |
| NPM | `apps/management-console/.npmrc` `ignore-scripts=true` + CI | 供給側ゲート既存 | FW=サプライチェーン完了と書かない |
| ホスト FW 文書 | guides / SECURITY_* / AGENTS / OPEN | **LuLu/LS・OP-095 記述ゼロ** | D1+D2 が埋める穴 |
| 危険作業① | `HUMAN_PUBLIC_BETA_RUNBOOK.md` **NT-1 Step A**（秘密 GUI/CLI） | Vault 操作入口 | D2-① **ここの直前のみ** |
| 危険作業② | `stripe-production-setup.md` 冒頭チェックリスト | OP-057-R/OP-084 | D2-② 任意推奨 1 行 |
| Vault 手順正本 | `api_key_rotation.md` | GUI 優先 | D2 非接続。D1 からリンク可 |
| nt_gate | `scripts/nt_gate.py` | Step0/hygiene | FW ゲート追加禁止 |

### 1.3 ギャップ判定

| ID | ギャップ | ホストで足りる？ | 製品コード？ |
|---|---|---|---|
| G0 | 開発 Mac の未知 outbound 可視性なし | H1+D1 | No |
| G1 | 秘密作業入口にリマインダなし | D2 | No |
| G2 | 製品内「全 egress 一覧 UI」なし | ホスト FW で足りる | **作らない** |
| G3 | Bastion/seatbelt プロファイル一覧の人間向け再掲 | コード＋SECURITY_DESIGN | **本計画で新文書化しない** |

**結論**: 最小クローズに製品コード変更は不要。

### 1.4 拒否リスト（実装時）

| 禁止 | 既存正本 |
|---|---|
| SECURITY_DESIGN / Immune / SSRF / NPM 節のコピー | `SECURITY_DESIGN.md` |
| reason_code / ImmuneAlert 新バス | OP-093 / `tool_call_router.rs` |
| MCP 解禁・PurchasePolicy | NR-09 |
| compose へ API キー | ランブック §0.2 |
| ホスト CLI Vault＝本番正本 | ランブック: MC GUI → key-proxy |
| `validate_url` を wf_http に流用 | workflow SSRF は `assert_resolved_url_safe` |
| seatbelt プロファイルを DEV_HOST に転記 | `seatbelt.rs` |
| deny ignore を本 OP に混ぜる | OP-068 |
| NT 再採番・新 Human Wave | ランブック v1.6 |
| RIPPLE_MAP への長大セキュリティ地図 | D1 完了時の **1 行影響**のみ（ファイル追加時） |

---

## 2. Wave 詳細と DoD

### Wave H0 — 承認

| DoD | 状態 |
|---|---|
| Human が **v1.2** を承認 | ✅ 2026-07-31 |
| `OPEN.md` に附録 B で OP-095 追加 | ✅ |
| スコープ固定: H1→D1→D2 | ✅（D1/D2 先行完了・H1 残） |

---

### Wave H1 — ホスト導入（Human・macOS）

| ステップ | 内容 | 検証 |
|---|---|---|
| H1-1 | LuLu（最低）または Little Snitch | 起動・新規接続プロンプト可 |
| H1-2 | 既知開発ツールはカテゴリ Allow、**未知バイナリは Ask** | ルール固定 |
| H1-3 | 驚いた外向きメモ（プロセス+ホスト名のみ。秘密禁止） | 0 件可 |

**Positive**: `cargo fetch`、`gh`（使用時）、api-server / compose up が恒常ブロックされない。  
**Negative**: 未知バイナリ相当でプロンプトまたは拒否。  
**Revert**: テスト用一時ルール削除。

**学習で出やすいプロセス**: Docker Desktop / Colima、Cursor、`cargo`/`rustup`、`node`/`npm`、ブラウザ、`ollama`。

---

### Wave D1 — ガイド（Agent・docs only）

**成果物**: `docs/guides/DEV_HOST_EGRESS.md`

| 含める | 含めない |
|---|---|
| LuLu/LS・Ask・許可疲れ・H1 の P/N/R | Immune/SSRF/MCP/Bastion/seatbelt 仕様本文 |
| allowlist **カテゴリ**（附録 A） | 固定 IP、鍵、Vault 値 |
| 危険作業前「監視 ON」+ D2 入口へのリンク | README 必須、workflow、nt_gate |
| 「製品側 SSRF は別系統（`assert_resolved_url_safe` vs `validate_url`）。詳細は計画 §1.2」程度の **1 文** | 関数仕様の再掲 |

**DoD**:

- [x] Human がガイド単体で H1 再実行可 — [`DEV_HOST_EGRESS.md`](../guides/DEV_HOST_EGRESS.md) ✅ 2026-07-31
- [x] 製品制御は計画 §1.2 または SECURITY_DESIGN へのリンクのみ
- [x] CHANGELOG [Unreleased] Docs 1 行
- [x] `.env.example` 変更なし
- [x] `.context/RIPPLE_MAP.md` に影響 1 エントリ

---

### Wave D2 — 入口接続（最大 2 ファイル・挿入点一意）

| # | ファイル | 挿入点（一意） | 文面要件 |
|---|---|---|---|
| ① | `docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md` | **`### Step A — 秘密（推奨 GUI）` の直前**（「Step 0 DoD PASS」チェックの後） | チェック 1 行: Vault/秘密操作前にホスト outbound 監視 ON（LuLu/LS）→ `DEV_HOST_EGRESS.md`。§0.2 本文の改変・NT 再採番禁止 |
| ② | `docs/operations/stripe-production-setup.md` | 冒頭 OP-057-R/OP-084 チェックリスト | **任意推奨** 1 行のみ。既存 `[x]` の書き換え禁止 |

**やらない**: OPERATIONS §6、COMPLIANCE、SECURITY_WHITEPAPER、api_key_rotation 本文、live_billing 計画の再開。

**DoD**: ①②のどちらからでもガイドへ 1 ホップ — ✅ 2026-07-31（ランブック v1.7 + stripe-production-setup）。

---

### Wave P1 — 復活条件のみ（既定アウト）

1. H0–D2 完了  
2. §1.3 にない新ギャップを `path:line` で提示  
3. OP-093 / Immune / MCP WL / `SecurityPolicy` / `assert_resolved_url_safe` で塞げない理由  
4. Human が「P1 実装しろ」と明示  

許可されても: 新 audit バス禁止、auth/commerce/Vault/Tauri shell 禁止、ホスト FW 代替 UI 禁止。

---

## 3. 効率設計

| リスク | 対策 |
|---|---|
| 許可疲れ | 未知のみ Ask |
| ドキュメント肥大 | D1 1 枚 + D2 各 1 行 |
| 本線阻害 | Phase E / OP-064 / OP-068 と非連結 |
| 拡大解釈 | 「続けろ」≠ P1（AGENTS Scope Lock） |
| シンボル陳腐化 | ガイドは関数仕様を書かず計画 §1.2 に委譲 |

---

## 4. Red Team

| # | 失敗モード | 防御 |
|---|---|---|
| R1 | IDE/ブラウザ正規経路の漏洩 | 期待値＝未知バイナリ可視性 |
| R2 | 全部 Allow | 許可疲れプロトコル + D2 |
| R3 | ガイドに鍵・固定 IP・seatbelt 転記 | レビュー拒否 |
| R4 | P1 勝手実装 | 既定アウト + AGENTS |
| R5 | 誤シンボル名で実装者を迷わせる | §1.2 実名固定 |
| R6 | nt_gate/CI にホスト FW | 禁止 |
| R7 | Bastion/seatbelt とホスト FW を同一視 | 層マップ + 転記禁止 |
| R8 | D2 を §0.2 と Step A の両方に二重貼り | 挿入点は Step A 直前のみ |

---

## 5. 成功基準

1. Step A 直前と stripe-production-setup からガイド 1 ホップ  
2. H1 の P/N/R 実施済み（macOS）  
3. 製品コード・Vault・MCP WL・deny.toml・seatbelt 未変更  
4. SECURITY_DESIGN / seatbelt をガイドが複製していない  
5. 通常開発フローが恒常ブロックされない  

---

## 6. 実行順序（承認後）

```
H0 → OPEN OP-095
  → H1（Human・macOS）
  → D1（ガイド + RIPPLE 1 行 + CHANGELOG）
  → D2（Step A 直前 + stripe チェック 1 行）
  → OP-095 クローズ
```

---

## 7. Mission Control（v1.2）

| 原則 | 適用 |
|---|---|
| Deep Scan | §1.2 を第2回照合で更新。実装前にシンボル再 `rg` |
| Ripple | D2=2 ファイル。CI/Tauri/compose/seatbelt 非接触 |
| Red Team | §4（R8 追加） |
| Drop-Dead | 誤シンボル・二重挿入・seatbelt 転記・A1/P1 既定を GC |

---

## 8. /perfect-plan + 再検証ゲート（v1.2）

| Gate | 結果 |
|---|---|
| 1 構造 | PASS — Human + docs のみ |
| 2 再発明 | PASS — OP-093/NR-09/Vault/SSRF/NPM/seatbelt 拒否 |
| 3 重複 | PASS — A1 吸収、D2 二重貼り禁止（R8） |
| 4 正確性 | PASS — `assert_resolved_url_safe` に訂正。呼出 `workflow_runtime.rs` 確認 |
| 5 接続先 | PASS — Step A 直前 + stripe 冒頭に一意化 |
| 6 OS | PASS — H1=macOS 必須のみ |
| 7 OPEN | PASS — 未起票を確認（H0 前の正常状態） |
| 8 順序 | PASS — H→D1→D2 |

**判定: ✅ PASS（v1.2 を実行正本とする。実装は H0 承認後）**

---

## 附録 A — allowlist カテゴリ種（D1 転記・H1 実測で更新）

| カテゴリ | 例 | 既定 |
|---|---|---|
| SCM | `github.com`, `api.github.com` | Allow |
| Rust | `crates.io`, `static.crates.io`, `index.crates.io` | Allow |
| JS | `registry.npmjs.org` | 使用時 Allow |
| コンテナ runtime | Docker Desktop / Colima | 開発時 Allow |
| レジストリ | Docker Hub / ghcr.io | 使用時 |
| IDE / Agent | Cursor 等 | 開発時 Allow・未知子プロセスは Ask |
| 決済（Human） | `api.stripe.com`, Dashboard | 秘密作業時 |
| LLM | 利用プロバイダ公式のみ | 設定時 |
| ローカル | `localhost`, `host.docker.internal`, Ollama | Allow |
| 未知バイナリ | 上記外 | **Ask** |

---

## 附録 B — OPEN 起票文案（承認後）

```markdown
- [ ] **OP-095**: 開発ホスト Egress 衛生（macOS: LuLu/Little Snitch + ガイド + 入口 1 行×2）。
  正本: [`dev_host_egress_hygiene_plan.md`](docs/roadmaps/dev_host_egress_hygiene_plan.md) **v1.2**。
  クローズ = H0+H1+D1+D2。A1 廃止（§1 照合済）。P1 既定アウト。
  D2 挿入: ランブック NT-1 Step A 直前 + stripe-production-setup 冒頭（任意推奨）。
  製品同梱・CI/nt_gate・SECURITY_DESIGN/seatbelt 転記・MCP/Vault 論理変更は禁止。
```

---

## 附録 C — 版差分

### v1.0 → v1.1
A1 廃止、P1 既定アウト、Bastion 記述訂正、D2 2 ファイル固定、README/nt_gate 抑制 等。

### v1.1 → v1.2（本改訂）

| 変更 | 理由（実コード） |
|---|---|
| `assert_url_safe` → **`assert_resolved_url_safe`** + `workflow_runtime.rs` 呼出 | `rg assert_url_safe` は計画文書のみ。実体は `validator.rs` L355 |
| D2 を「§0.2 または Step A」→ **Step A 直前のみ** | 二重貼り・曖昧挿入を防止（R8） |
| H1 を **macOS 必須**と明記 | LuLu/LS 前提。他 OS 手順肥大を防止 |
| seatbelt 転記禁止を追加 | `seatbelt.rs` は製品サンドボックス層 |
| reason_code を実コード列挙に更新 | `tool_call_router.rs` 再 grep |
| Vault「exact endpoint」をガイド禁止 | key-proxy 実装の未検証スローガン拡散防止 |
| OPEN 未起票を照合で確認 | H0 前の正常状態 |
| D1 DoD に RIPPLE_MAP 1 行 | AGENTS 新規ファイル規則（長大地図は禁止） |
