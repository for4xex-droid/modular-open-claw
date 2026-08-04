# 自律 Egress 防衛計画（低運用・確実防衛）（v1.3）

- **作成日**: 2026-07-31（v1.0） / **改訂**: v1.2 → **v1.3**（第4回実コード照合・実装拘束の精密化）
- **ステータス**: **実装完了（2026-07-31）** — P0–D ✅ / ADR-057 Accepted / OPEN OP-096 ✅
- **ID**: **OP-096**
- **目的**: ホスト FW Ask を本線から外し、**PermissionManifest ホスト許可意味論を Fail-Closed に統一**（既存 code_mode ロジックの抽出が本体）
- **継承**: OP-095／OP-075／OP-093／ADR-036／`PermissionManifest`／BastionGuard／code_mode fetch／workflow SSRF
- **正本関係**: 承認後 `OPEN.md`。全体像は [`SECURITY_DESIGN.md`](../architecture/SECURITY_DESIGN.md)

---

## 0. 結論

### 価値仮説

| 要求 | 答え |
|---|---|
| 低運用 | 人間 Ask なし。**純関数 1 個 + Bastion/code_mode 委譲**。日常 UI なし |
| 確実防衛 | Manifest 経路で空 `allowed_domains` ≠ 全許可（Bastion を code_mode に合わせる） |
| ホスト FW | 任意（OP-095 H1）。本線は製品内 |

### やる（v1.3 確定）

| 順 | Wave | ID | 内容 |
|---|---|---|---|
| 1 | **P0** | OP-096-P0 | 承認 + OP-095 H1 任意化 + `DEV_HOST_EGRESS` 1 文 |
| 2 | **A** | OP-096-A | ADR-057（案）: 意味論・配置・WASM `*`・非目標（commerce/seatbelt/router） |
| 3 | **B** | OP-096-B | `host_permitted` を **`libs/aiome-contracts/src/security.rs`** に追加 → code_mode / Bastion が利用 + P/N（テストは既存 `tests/security_tests.rs` へ追記） |
| 4 | **C** | OP-096-C | WASM `*` スキップ方針をコメント＋テスト。**それ以外の配線追加なし** |
| 5 | **D** | OP-096-D | 単体テスト必須化（Fitness rg **禁止**） |

### やらない（再発明・誤統合の禁止）

| 禁止 | 根拠（実体） |
|---|---|
| `commerce_helpers::validate_redirect_url*` と統合 | 類似の host/suffix 照合だが **別契約**（https・ALLOWED_ORIGINS・dev fail-open）。Safety-Critical commerce。**触らない・共通化しない** |
| tool_call_router へ egress / 第二 SSRF | 既存 `reason_code=ssrf` |
| workflow ↔ Manifest 統合 | `assert_resolved_url_safe` |
| seatbelt ドメイン allowlist | `SafeCommandBuilder` は boolean net のみ |
| OPEN_WEB 定数を Bastion 例外の中核に | Open-Web = SSRF 層 |
| Fitness で `allow_network`+空 domains を rg | 偽陽性で高運用 |
| constraint_checker を本 OP で書き換える | 現状は **完全一致** `contains(host)`（suffix/`*` なし）。寄せるのは **OP-097**（[`manifest_host_drift_plan.md`](manifest_host_drift_plan.md)） |
| ConstitutionalValidator 改変 | 過剰権限公理のみ。空 domains 非担当 |
| 製品 LS / ホスト FW 必須 / CapabilityToken / 新 audit バス | 維持 |
| marketplace_buy / OP-068 / auth・Vault・Tauri 論理 | 維持 |
| `RuntimeJail::check_network` のリネーム／シグネチャ変更 | トレイト API の再発明。host-or-url 受理は文書＋実装のみ |
| 新テストファイル／新テストハーネス | 既存 `libs/aiome-contracts/tests/security_tests.rs` に追記 |

### OP-095

D1/D2 維持。**H1 任意**。本線 = OP-096。

---

## 1. 実コード照合（第4回・2026-07-31）

### 1.1 経路別

| 経路 | 実体 | 空 domains + net | OP-096 |
|---|---|---|---|
| code_mode fetch | `skills/code_mode.rs` L221–226 | **Deny**（`*` / 完全一致 / `host.ends_with("."+d)`） | **アルゴリズム正本** |
| Bastion `check_network` | `bastion_guard.rs` L259–282 | 空→スキップ **Ok（Fail-Open）**、一致は `url.contains` | **host_permitted へ置換** |
| `check_network` 呼出 | `skills/mod.rs` L358 のみ | 引数は **URL ではなく `allowed_domains` の生文字列** | Bastion は **bare host を受理**必須（§2.2） |
| WASM `with_allowed_host` | 同 L355–361 | 空→未登録。`domain != "*"` で `*` スキップ | 方針維持（C） |
| router / workflow | SSRF / `assert_resolved_url_safe` | Manifest 外 | 非変更 |
| SafeCommandBuilder | boolean + Profile | domains 未参照 | Residual・スコープ外 |
| commerce redirect | `commerce_helpers.rs` L44–46 | 別契約（小文字・スキーム） | **統合禁止** |
| constraint_checker | L99 `allowed_domains.contains(host)` | 空+host→ DomainBlocked（完全一致のみ） | 本 OP 非変更 → **後続 OP-097**（[`manifest_host_drift_plan.md`](manifest_host_drift_plan.md) v1.3） |
| PermissionManifest 定義 | **`libs/aiome-contracts/src/security.rs`**（`aiome-core-contracts` が re-export） | — | 純関数の **配置先** |

### 1.2 第3–4回で潰した計画穴

| 穴 | 訂正 |
|---|---|
| Bastion が常に URL パースすると書いていた | 唯一の呼出は **bare domain**。`Url::parse("example.com")` 依存は **WASM 登録を壊す** → target→host 解決規則を明記 |
| 配置が「contracts または core」で曖昧 | **`aiome-contracts` の `security.rs` に固定**（Manifest 同居） |
| commerce の類似コード未記載 | **拒否リストに追加**（車輪統合の誘惑を遮断） |
| constraint_checker を「寄せてもよい」と緩い | **本 OP スコープ外**と明確化 → 寄せる作業は **OP-097** に分離済 |
| 大文字小文字 | code_mode は **ケースセンシティブ** → ADR/実装もそれに固定（commerce の to_lowercase を持ち込まない） |
| 空 domains Deny が WASM 現状を大きく変える過大期待 | 空リスト時はループ 0 回で元から host 未登録。**本 OP の実益は (1) Bastion 意味論の将来安全 (2) `contains`→host 一致への置換 (3) code_mode との単一正本** |
| テスト配置が未固定 | 既存 `aiome-contracts/tests/security_tests.rs` に追記（新ハーネス禁止） |
| トレイト改名の誘惑 | `check_network` 名は維持 |

### 1.3 本 OP のギャップ（変わらず）

| ID | 内容 |
|---|---|
| G1 | Bastion vs code_mode 意味論不一致（空 Allow vs Deny、`contains` vs host/`*`/suffix） |
| G2 | WASM が `*` を host 登録しない（意図的 Fail-Closed — ADR 明記） |
| G3 | Bastion Fail-Open の将来踏襲リスク |
| G4 | seatbelt ドメイン非対応（Residual・非目標） |

### 1.4 現状 WASM 登録と `check_network` の関係（過剰実装防止）

```text
for domain in allowed_domains:
  if domain != "*":
    check_network(domain)  // 引数 = リスト自身の要素（ほぼ自己照合）
    → Ok なら with_allowed_host(domain)
```

ここだけ見ると空→Deny 変更の **即時ユーザ影響は小さい**。それでも Bastion を直す理由は、トレイト実装が Fail-Open のままだと将来の URL 引数呼び出しで穴が残るため。**新レイヤや Open-Web 例外は不要**。

---

## 2. 設計

### 2.1 純関数（唯一の新ロジック）

配置: **`libs/aiome-contracts/src/security.rs`**

```
pub fn host_permitted(host: &str, allowed_domains: &[String]) -> bool
  if allowed_domains.is_empty() → false
  for d in allowed_domains:
    if d == "*" OR d == host OR host.ends_with(&format!(".{d}")) → true
  false
```

- code_mode: URL から得た `host` を渡す（現状どおり）  
- 単体テスト: **`libs/aiome-contracts/tests/security_tests.rs` に追記**（infrastructure にアルゴリズムを残さない）  
- 空文字 `""` の domain エントリは許可に使わない（`ends_with(".")` 偶発を避ける。実装で `d.is_empty() → continue`）  
- **やらない**: async、必須 URL パーサ、新クレート、commerce からの呼び出し、トレイト改名

### 2.2 Bastion `check_network(target)`（メソッド名は維持）

```
if !system_internal && !allow_network → Err
if system_internal → Ok  // 拡大禁止
host = network_target_host(target):
  - scheme 付きで Url::parse 成功 → host_str（無ければ Err）
  - それ以外 → target を trim した文字列を host とみなす  // bare domain 用（現行 WASM 呼出）
if host.is_empty() → Err
if host_permitted(host, &allowed_domains) → Ok else Err
```

`url.contains(domain)` は削除。`network_target_host` は **infrastructure 側の私有関数**でよい（contracts に URL 依存を入れない）。

### 2.3 層分離

```
[aiome-contracts::host_permitted + Bastion + code_mode]  ← OP-096
[tool_call_router SSRF] / [workflow assert_resolved_url_safe]
[MCP command WL ADR-036] / [commerce redirect — 別契約]
[seatbelt boolean — Residual] / [Host FW OP-095 任意]
```

### 2.4 `*`

| 層 | 方針 |
|---|---|
| host_permitted | `*` → true |
| WASM `with_allowed_host` | `*` スキップ維持（列挙のみ）。`*` のみ manifest = WASM ネット無し |

---

## 3. Wave 詳細

### P0 — 承認

| DoD |
|---|
| **v1.3** 承認 |
| OPEN OP-096 |
| OP-095 H1 任意化 |
| `DEV_HOST_EGRESS.md` に本線 OP-096 を 1 文 |

### A — ADR-057（案）

空=Deny、§2.1–2.4、commerce/seatbelt/router/constraint **非目標**、Breaking は Bastion 旧 Fail-Open のみ。

### B — 実装

| 作業 | 検証 |
|---|---|
| `host_permitted` + `security_tests.rs` 追記 | 空/不一致/一致/suffix/`*`/空文字 domain |
| code_mode が関数を呼ぶ | 既存拒否メッセージ・挙動維持 |
| Bastion + bare host / URL 両対応 | WASM: 列挙 domain で `with_allowed_host` まで到達（Positive）。空 domains で check_network 単体は Deny（Negative） |
| `url.contains` 削除 | 部分一致の偶発許可が消える Negative |
| トレイト改名なし | `RuntimeJail::check_network` シグネチャ維持 |

Verification Protocol 必須。

### C — 薄い整合

WASM `*` コメント＋テスト 1。router/Open-Web/seatbelt **禁止**。

### D — 回帰

contracts/infrastructure の該当テストが CI で回ること。Fitness rg **禁止**。

---

## 4. 成功基準

1. code_mode と Bastion が同一 `host_permitted` を使用  
2. 空 domains → Bastion Deny；bare `example.com` でも列挙時 Ok（WASM 経路）  
3. SSRF / workflow / MCP WL / commerce redirect テストが本 OP で壊れない  
4. OP-095 H1 なしでクローズ可  
5. 新クレート・commerce 共通化・router 二重 SSRF なし  
6. Safety-Critical 非変更  

---

## 5. Red Team

| # | 失敗 | 防御 |
|---|---|---|
| R1 | seatbelt を直ったと誤認 | Residual |
| R2 | commerce ヘルパと統合 | 禁止表 |
| R3 | bare host を Url 必須にして WASM 破壊 | §2.2 |
| R4 | `url.contains` 残存 | B で削除 |
| R5 | Fitness rg | 禁止 |
| R6 | constraint を黙って suffix 化して挙動変更 | 本 OP 外 |
| R7 | ホスト FW 必須化 | P0 |
| R8 | system_internal / `*` WASM 無制限 | 拡大禁止・スキップ維持 |
| R9 | 空 Deny で「全部直った」と過大宣言 | §1.4。即時影響は限定的と明記 |
| R10 | RuntimeJail 改名・新 trait | 禁止表 |

---

## 6. 実行順

```
P0 → A → B（contracts 関数 → code_mode → Bastion）→ C → D → クローズ
```

---

## 7. Mission Control

| 原則 | 適用 |
|---|---|
| Deep Scan | §1.1 第4回。B 前に `check_network` / `host_permitted` / commerce_helpers を再 rg し誤統合を防止 |
| Ripple | Bastion 破壊半径は狭い。seatbelt/commerce **非接触**。WASM 空リスト挙動は実質不変 |
| Red Team | §5 |
| Drop-Dead | OPEN_WEB・Fitness rg・commerce 共通化・constraint 同梱・トレイト改名を GC |

---

## 8. /perfect-plan（v1.3）

| Gate | 結果 |
|---|---|
| 1 構造 | PASS — Manifest 同居の純関数のみ。トレイト改名なし |
| 2 再発明 | PASS — SSRF/commerce/seatbelt/新テスト基盤 非統合 |
| 3 正確性 | PASS — bare host・自己照合ループ・実益範囲を §1.4 で限定 |
| 4 運用 | PASS — Ask/UI/rg なし |
| 5 Safety | PASS — commerce/auth/Vault/Tauri 非変更 |
| 6 重複検証 | PASS — 第4回で v1.2 本線を再確認。差分は拘束の精密化のみ |
| 7 順序 | PASS |

**判定: ✅ PASS（v1.3 を実行正本とする。実装は P0 承認後）**

---

## 附録 A — OPEN 起票文案

```markdown
- [ ] **OP-096**: 自律 Egress 防衛 — `host_permitted`（aiome-contracts）抽出 + Bastion Fail-Closed 整合（code_mode 正本）。
  正本: [`autonomous_egress_defense_plan.md`](docs/roadmaps/autonomous_egress_defense_plan.md) **v1.3**。
  commerce redirect / router SSRF / seatbelt ドメイン / Fitness rg / ホスト FW 必須 / RuntimeJail 改名は非目標。OP-095 H1 任意。
```

## 附録 B — OP-095 更新文案（P0）

```markdown
- [x] **OP-095**: … D1/D2 ✅。**H1 は任意**（本線防衛は OP-096）。
```

## 附録 C — 版差分

### v1.0 → v1.1
code_mode 正本化、OPEN_WEB/Fitness rg/router egress 降格、seatbelt Residual、呼出 1 箇所の破壊半径是正。

### v1.1 → v1.2
配置を `aiome-contracts` に固定、bare host、commerce 統合禁止、constraint 本 OP 外、ケースセンシティブ固定。

### v1.2 → v1.3（本改訂・第4回照合）

| 変更 | 理由 |
|---|---|
| §1.4 で WASM 自己照合ループと即時影響の小ささを明記 | 過剰な新レイヤ追加を防止 |
| テストを既存 `security_tests.rs` に固定 | 新テスト基盤の再発明防止 |
| RuntimeJail 改名禁止 | トレイト API の車輪防止 |
| `network_target_host` を infra 私有に限定 | contracts へ URL 依存を入れない |
| 空文字 domain エントリを無視 | `ends_with(".")` 偶発許可の芽を摘む |
| OPEN 未起票を明記 | H0/P0 前の正常状態 |
