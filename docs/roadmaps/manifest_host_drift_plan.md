# Manifest ホスト意味論ドリフト解消 + Seatbelt Spike（v1.3）

- **作成日**: 2026-07-31（v1.0） / **改訂**: v1.1 → v1.2 → **v1.3**（第3回実コード照合・照合収束）
- **ステータス**: **OP-097 ✅ / OP-098 Spike ✅ Residual（実装しない・2026-07-31）**。照合収束済み
- **ID**: **OP-097**（本線・薄い配線 ✅） / **OP-098**（Spike Residual ✅・実装しない）
- **継承**: OP-096（✅） / ADR-057 / [`autonomous_egress_defense_plan.md`](autonomous_egress_defense_plan.md) v1.3
- **目的**: PermissionManifest ホスト判定の**残ドリフトを最小差分で閉じる**。ライブ egress の再発明・別契約の統合はしない
- **正本関係**: `OPEN.md`。全体像は [`SECURITY_DESIGN.md`](../architecture/SECURITY_DESIGN.md)

---

## 0. 結論（v1.3）

### 照合サマリ（実装後・/reflexion 再検証）

| 主張 | 結果 |
|---|---|
| `allowed_domains.contains` 本番 | **0**（OP-097 後。実装前は L99 のみだった） |
| DomainBlocked | `host_permitted` 委譲済み（潜伏コメント付き） |
| `"network_request"` / `"fs_write"` の他クレート書込 0 | **PASS** |
| `host_permitted` 使用 | Bastion + code_mode + **constraint_checker** + contracts テスト |
| skill_handler は `execute_wasm_skill` + skill 名 | **PASS**（DomainBlocked 未到達は意図どおり） |
| SeatbeltProfile に domains なし / OP-098 Residual | **PASS**（§8） |

### 価値仮説（過大宣言禁止）

| 要求 | 答え |
|---|---|
| 実害 | DomainBlocked は本番ほぼ未到達（§1.2）。OP-097 は意味論の単一正本化。OP-096 の再実装ではない |
| 低運用 | `constraint_checker` **L99 の 1 条件**のみ。`fs_write` 分岐・dead action 削除はしない |
| 確実防衛 | ライブ正本は Bastion / code_mode / WASM（OP-096 済） |
| 誤統合防止 | 禁止表（§0）厳守 |

### やる

| 順 | Wave | ID | 内容 | 状態 |
|---|---|---|---|---|
| 1 | **P0** | OP-097-P0 | 本計画 **v1.3** 承認 | ✅ |
| — | — | — | OPEN 起票・ADR §5/§7 | ✅ 済 |
| 2 | **B** | OP-097-B | L99 `contains` → `host_permitted` + 潜伏コメント + 配線テスト最小 | ✅ |
| 3 | **C** | OP-097-C | CHANGELOG / RIPPLE / OPEN ✅ / 本計画完了 | ✅ |
| 4 | **S0** | OP-098-S0 | seatbelt Spike レポート（§8） | ✅ Residual |
| 5 | **S1** | OP-098-S1 | 実装 OP 起票 | **不要**（§8 結論） |

### やらない（再発明・誤統合・重複の禁止）

| 禁止 | 実コード根拠 |
|---|---|
| `host_permitted` の再実装／コピー | 正本 `libs/aiome-contracts/src/security.rs`。呼出は `aiome_core::security::host_permitted`（`libs/core/src/security.rs` が contracts を re-export） |
| contracts `security_tests` のケース複製 | 配線テストは **最小 2**（§2.2） |
| `execute_wasm_skill` にホスト／DomainBlocked を新設 | `skill_handler.rs` L223–224。防衛は WASM+Bastion |
| `"network_request"` / `"fs_write"` 分岐の削除や AgentRx 配線追加 | 両方とも本番未 emit（§1.2）。削除は別議論・本 OP 外 |
| `aiome_core::security::ConstitutionalValidator`（struct）の substring 公理を寄せる | `libs/core/src/security.rs` L26–34 |
| `aiome_core_contracts::traits::ConstitutionalValidator` や `DefaultConstitutionalValidator` を触る | **同名別物**（gig/workflow 用）。Manifest ホスト許可と無関係 |
| `commerce_helpers::validate_redirect_url*` | https・ALLOWED_ORIGINS・casefold・dev fail-open |
| `tool_call_router` 第二 SSRF / Manifest egress | 既存 `reason_code=ssrf` |
| `assert_resolved_url_safe` / `SecurityPolicy::validate_url` | workflow / ShieldClient。Manifest 外 |
| Vault / auth / Tauri / RuntimeJail 改名 | Safety-Critical / API 再発明 |
| seatbelt 実装（Spike なし） | `SeatbeltProfile` は boolean のみ |
| Fitness で `allowed_domains.contains` を rg ゲート化 | 偽陽性 |
| OP-096（Bastion / code_mode / WASM）の再実行 | ✅ 済 |

---

## 1. 実コード照合

### 1.0 照合スタンプ

| 回 | 版 | 要点 |
|---|---|---|
| 1 | v1.1 | `network_request` 未到達・ADR §5 矛盾解消・seatbelt Residual |
| 2 | v1.2 | ConstitutionalValidator 名衝突 3 種・`fs_write` 非対象・配線テスト最小 2 |
| 3 | **v1.3** | 主張を rg 再検証して全 PASS。seatbelt 配線が Bastion **2 箇所**（~L207 / ~L345）。**照合収束** |

### 1.1 Manifest ホスト許可の到達マップ

| 経路 | 実体アンカー | 判定 | 本計画 |
|---|---|---|---|
| 純関数 | `aiome-contracts` `host_permitted` | 空 Deny / `*` / exact / suffix（`.` 必須）/ trim+junk | **正本・非変更** |
| Bastion | `bastion_guard.rs` `check_network` → `host_permitted` | Fail-Closed | **非変更** |
| code_mode | `skills/code_mode.rs` `aiome.fetch` | 同上 | **非変更** |
| WASM 列挙 | `skills/mod.rs` `wasm_hosts_for_extism` | trim 後 `*` スキップ | **非変更** |
| constraint_checker DomainBlocked | `host_permitted`（旧 `contains` 廃止） | Bastion/code_mode と同一意味論 | **OP-097 ✅** |
| constraint_checker `fs_write` | L113 | allow_filesystem_write のみ | **非変更**（ホスト無関係・未到達） |
| ConstitutionalValidator **struct** | `libs/core/src/security.rs` | Manifest エントリ公理 | **禁止** |
| ConstitutionalValidator **trait** / Default… | traits / `infrastructure::validator` | 別系統 | **禁止（混同防止）** |
| seatbelt | `seatbelt.rs` + Bastion McpServer 等 | boolean only | **OP-098 Spike** |
| commerce / router / workflow | 各ヘルパ | Manifest 外 | **禁止** |

`rg 'allowed_domains\.contains'` → 本番ヒットは **`constraint_checker.rs` L99 のみ**。

### 1.2 本番到達性

| 事実 | 根拠 |
|---|---|
| ConstraintChecker 本番呼出 | `skill_handler.rs` L306 のみ（+ chaos 実験は非 prod 経路） |
| step.action | 常に `"execute_wasm_skill"`（L223） |
| tool_name | **skill 名**（L224）。ホストではない |
| `"network_request"` / `"fs_write"` の他クレート書込 | **0**（checker 定義・単体テストのみ） |

含意:

- OP-097 はライブ egress 修正ではない  
- `execute_wasm_skill` へのホスト検査追加は OP-096 との**二重化＝再発明**  
- dead 分岐の削除も本 OP ではやらない（範囲爆発）

### 1.3 seatbelt（OP-098）

- Manifest `allowed_domains` → `SeatbeltProfile` 転写は **存在しない**（フィールド自体なし）  
- Bastion が `SeatbeltProfile` を組む箇所は **2つ**（`bastion_guard.rs` 付近 L207 系 / L345 系）。いずれも `allow_network` boolean のみ  
- Spike は両 call site を棚卸しすること（片方だけ見て「直した」と誤認しない）  
- 既定結論: **boolean 二重化で十分 → Residual 受容でクローズ可**

### 1.4 ADR

Decision §5（OP-097 委譲例外）・§7（ConstitutionalValidator struct 非対象）は **済**。Follow-up は本計画 **v1.3**。

---

## 2. 設計

### 2.1 OP-097 — 差分

```rust
// constraint_checker.rs
use aiome_core::security::{host_permitted, PermissionManifest}; // PermissionManifest は既存

// action == "network_request" は現状 skill_handler からは emit されない（潜伏契約）。
// ホスト防衛のライブ正本は Bastion / WASM / code_mode（OP-096）。ここに二重検査を足さない。
if let Some(host) = &step.tool_name {
    if !host_permitted(host, &self.permission_manifest.allowed_domains) {
        // DomainBlocked ...
    }
}
```

- `allow_network` / `tool_name` 欠如の既存分岐は維持  
- 新クレート依存禁止  

### 2.2 テスト（重複禁止・最小）

| 層 | 置き場 | 内容 |
|---|---|---|
| アルゴリズム | `security_tests.rs` | **触らない** |
| 配線（必須） | `constraint_checker` tests | (1) suffix 許可 Positive (2) 空 domains Negative |
| 配線（任意） | 同 | `*` 許可 — contracts で十分なら省略可 |
| 回帰 | `test_network_access_denied` | 二重違反維持 |

新ファイル禁止。

### 2.3 意図的 Breaking（潜在契約のみ）

| ケース | 旧 | 新 |
|---|---|---|
| `["example.com"]` + `api.example.com` | Blocked | OK |
| `["*"]` + host | Blocked | OK |
| 空 + host | Blocked | Blocked |

本番 `execute_wasm_skill` の観測挙動は不変。

### 2.4 OP-098 — Spike

詳細は **§8（完了）**。結論: **Residual 受容・製品コード変更なし・S1 不起票**。

---

## 3. Wave 詳細

### P0 — 承認

Human が **v1.3** を承認。OPEN 正本リンクが本ファイルを指すこと。

### B — 実装 → 検証

1. 置換 + 潜伏コメント → `rg 'allowed_domains\.contains'` 本番 0  
2. 配線テスト ≥2 PASS  
3. diff に commerce / router / seatbelt / `libs/core/src/security.rs` / skills / bastion / validator が無い  
4. Verification Protocol: Positive（suffix）+ Negative（空）

### C — クローズ

OP-097 ✅。OP-098 は §8 Residual で ✅。

### S0 — OP-098 Spike ✅

§8 レポート。製品コード・`seatbelt.rs`・Bastion **非変更**。

---

## 4. 成功基準

1. DomainBlocked が `host_permitted` を呼ぶ（非複製） — **OP-097 ✅**  
2. ライブ経路（OP-096）を再変更しない — **維持**  
3. 禁止表のパスが OP-097 diff に無い — **維持**  
4. OP-098 Residual クローズ可 — **§8 ✅**  
5. Fitness 新ゲートなし — **維持**  

---

## 5. Red Team

| # | 失敗 | 防御 |
|---|---|---|
| R1 | ライブ egress を直したと過大報告 | §1.2 |
| R2 | `execute_wasm_skill` にホスト検査 | 禁止表 |
| R3 | trait/Default ConstitutionalValidator を struct と混同して改変 | §0 名衝突表 |
| R4 | `fs_write` / dead action 削除を同梱 | 禁止表 |
| R5 | commerce / seatbelt 実装を混ぜる | 禁止表 / §8 |
| R6 | アルゴリズムテスト二重管理 | §2.2 最小 2 |
| R7 | RuntimeJail 改名 | 禁止表 |
| R8 | seatbelt hostname allowlist で「直った」と誤認 | §8（DNS 非対応・偽安心） |

---

## 6. OPEN 文言（要約）

- **OP-097**: ✅ DomainBlocked → `host_permitted`  
- **OP-098**: ✅ Spike Residual（実装しない）。正本: §8  

---

## 7. 改訂履歴

| 版 | 日付 | 内容 |
|---|---|---|
| v1.0 | 2026-07-31 | 起票 |
| v1.1 | 2026-07-31 | 未到達・ADR・Residual・テスト重複禁止 |
| v1.2 | 2026-07-31 | ConstitutionalValidator 3 種・`fs_write`・配線テスト最小 2 |
| v1.3 | 2026-07-31 | 第3回 rg 照合全 PASS。seatbelt 2 call site。照合収束 |
| v1.3+S0 | 2026-07-31 | OP-098 Spike §8：Residual・S1 不起票・OPEN クローズ |

---

## 8. OP-098 Spike レポート（2026-07-31・コード変更なし）

### 8.1 調査質問への回答

| # | 質問 | 回答 |
|---|---|---|
| 1 | Bastion boolean + OS seatbelt で防衛目標は足りるか | **Yes**。ホスト許可の正本は OP-096（`host_permitted` / Bastion `check_network` / WASM 列挙 / code_mode）。seatbelt は subprocess の **粗粒度**（全許可 or 全拒否、LoRA のみ 80/443）。ドメイン粒度はアプリ層で既に担保 |
| 2 | hostname allowlist を seatbelt 言語で表現できるか | **実用的には No / 偽安心**。Seatbelt は connect 時点のフィルタで、**DNS 名を信頼できる許可単位にしない**設計が多い。`(remote tcp "host:*")` 等は best-effort で、CDN・解決後 IP・ドキュメント不足と相性が悪い。現行 Aiome は `allow_network=true` 時 `(allow default)` のまま **network deny を付けない**（ポート/ホスト列挙モデルではない） |
| 3 | Manifest `allowed_domains` 写経時の偽陰性／偽陽性 | **偽陰性**: `*`・suffix・trim を seatbelt 文字列に落とせず、許可不足で正当通信が死ぬ。**偽陽性**: DNS 非対応や `*:443` 級に広げると「Manifest 通り」と見せかけて広い egress。LoRA 既存の `*:80`/`*:443` はその典型 |
| 4 | 実装するなら最小差分と OS スコープ | macOS only（`sandbox-exec`）。`SeatbeltProfile` に domains 追加 → `generate_profile_str` 改変 → Bastion **両 call site** から転写。Linux `runsc` は別問題。コスト高・偽安心のため **実装しない** |

### 8.2 Bastion call site 棚卸し

| # | 関数 | 位置（目安） | Manifest domains | 挙動 |
|---|---|---|---|---|
| A | `safe_exec` 内サンドボックス組立 | `bastion_guard.rs` ~L205–227 | **未使用** | Strict / PythonForge·WasmRun / WasmBuild·ForgeBuild / McpServer（boolean のみ）/ `_` |
| B | `wrap_binary` | 同 ~L340–363 | **未使用** | LoraTraining（80/443）/ McpServer（boolean）/ BrowserAgent / `_` |

`SeatbeltProfile`（`libs/shared/src/sandbox/seatbelt.rs`）フィールド: `allow_network` / `allow_fs_write` / `is_lora_training` のみ。**domains フィールドなし**。

### 8.3 層分離（再確認・再発明禁止）

```
[host_permitted + Bastion check_network + WASM + code_mode]  ← ホスト許可正本（OP-096/097）
[seatbelt boolean / LoRA ポート]                             ← subprocess 粗粒度（本 Spike = Residual）
[Host FW OP-095 H1]                                          ← 任意の個人衛生
[commerce / router SSRF / workflow]                          ← 別契約・非統合
```

### 8.4 結論（Human ゲート不要・S1 不起票）

| 項目 | 決定 |
|---|---|
| 製品コード変更 | **しない** |
| 新 OP（seatbelt domains 実装） | **起票しない** |
| OPEN OP-098 | **Spike 完了・Residual で ✅** |
| 再検討トリガー | Apple が DNS 対応の公式 hostname allowlist を文書化した場合、または subprocess が Bastion `check_network` を迂回して任意 connect する実証バグが出た場合のみ |
