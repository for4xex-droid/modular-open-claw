# OP-088 P5 — Ship 後品質積み上げ 詳細計画（v1.5）

- **親計画**: [`desktop_inprocess_default_plan.md`](desktop_inprocess_default_plan.md) v1.3（Ship P0-pre…P3 + P4 ✅）
- **OPEN**: OP-088 残任意 = **P5-b/c/d**（**P5-a ✅ 2026-07-21**）
- **日付**: 2026-07-21
- **ステータス**: P5-a Implemented。b/c/d は各サブIDの明示許可まで実装禁止
- **/perfect-plan**: v1.4 → **v1.5 再突合 PASS**（§8・C1 実行可能性）
- **Safety-Critical**: 都度「`P5-x` を実装しろ」必須

---

## 0. 目的と境界

Ship 済み Desktop（env なし InProcess）を壊さず、観測された痛み・UI・長期アーキだけを任意で磨く。

| しないこと | 理由 |
|------------|------|
| Ship 本線の再実装 | 沈黙経済・401 |
| Transport / Mutex / CommerceEngine adapter | 既存手段で足りる |
| `attempt_coin_charge_once` を全経路の唯一正本にする | 3重複 → 共通抽出（actor/power は引数） |
| **P5-c で AppState.commerce_engine だけ置換** | gig / docker / browser / marketplace が Factory `Arc` を既に保持（二重台帳） |
| P5-a で Stripe を触る / nurture-api 公式復帰 | 役割分担・Q3 |

### 0.1 サブID

| ID | 一言 | 着手条件 | 優先 |
|----|------|----------|------|
| **P5-a** | S2S プロセス内 dispatch（同期 G11） | ✅ 2026-07-21 | ★★★ |
| **P5-b** | Settings Mode（b-read → b-write） | Human 明示 | ★★ |
| **P5-c** | Plugin bridge を **生成時点から** 唯一の CommerceEngine に | 新 ADR + SC | ★ |
| **P5-d** | OSS/Economy 二系統 | 別 OP | ★ |

### 0.2 役割分担

| 経路 | 担当 |
|------|------|
| forget / monthly-limit / coin-charge(+DLQ) | **P5-a** |
| Stripe HTTP `/internal/*`（Factory） | **P5-c**（boot 並べ替えで消滅） |
| Polar / Mock | c 対象外 |
| MCP self SSE | 非対象 |

---

## 1. 再利用インベントリ

| 既存資産 | 使い方 |
|----------|--------|
| `internal_routes`（prefix なし） | oneshot path 正本（`/forget/:id`, `/coin-charge`, `/economy-policy/monthly-limit`） |
| `clone_s2s_router`（新・薄い） | 二重 `s2s_internal_service` 禁止 |
| `Router::clone().oneshot` | Mutex 禁止 |
| Bearer+OXP 3重複 | **`attach_s2s_headers(secret, actor, power)`** — actor/power は呼び出し側が渡す（下記 §3.1） |
| `get_http_client` | HTTP フォールバック正本 |
| `AiomePlugin::commerce_engine` + `get_agent_hooks` 型 | P5-c: `PluginRegistry::commerce_engine()` |
| `preflight.plugin_registry` in assemble | S2S clone（別引数任意） |
| `get_nurture_status` / `.nurture_drm_master_key` | b-read / b-write（`.nurture_mode` + gitignore） |
| `common.rs` **and** `system.rs` の AppState リテラル | 両方に `nurture_s2s: None` |

### 1.1 起動順序（P5-a / P5-c で意味が違う）

**現状:**

```text
init_core_services  → Factory + gig/docker/browser が commerce Arc を捕捉
register_in_process_plugins → s2s + Plugin bridge
assemble → marketplace も core.commerce_engine を捕捉
spawn_background_workers
create_router → nest /internal
```

| ID | 注入点 |
|----|--------|
| **P5-a** | assemble 内で `clone_s2s_router` → AppState（**spawn 前**） |
| **P5-c** | `create_plugin` は `auth_manager` + `event_sender`（core 内生成）が必要 → **core 全体より前の C1 は不可**。commerce ブロック直前（C1'）または core 二段化（C2）。AppState 後段置換は **禁止** |

### 1.2 trait

bridge / plugin = contracts re-export → adapter 不要。

---

## 2. G11 アンカー

| 経路 | G11 | 備考 |
|------|-----|------|
| forget | **高** | OXP actor=`aiome_system`, power=`1000` 固定 |
| monthly-limit | **高** | actor=`aiome-edge-node`, power=`oxilean_power`; URL 空=Ok skip / secret None=Err |
| coin-charge / DLQ | 低〜中 | actor=`aiome-edge-node`, power=引数 |
| Stripe | **高（残→c）** | a 非対象 |
| MCP SSE | 非対象 | |
| serve | concurrency_limit **無し** | HTTP self-call が危険 → oneshot 妥当 |

---

## 3. P5-a — プロセス内 S2S

### 3.1 設計

1. `clone_s2s_router` + `AppState.nurture_s2s: Option<Router>`
2. `nurture_s2s` モジュール:
   - `attach_s2s_headers(..., actor: &str, power: u32)` — **定数化しない**（上記3経路で異なる）
   - `post_internal`: handle あり → nest 前 path + oneshot / なし → `get_http_client` + `{url}/internal{path}`
3. relay / settings / auth を置換（auth=SC）
4. create_router は従来 `take`+nest

### 3.2 ステップ

| Step | 作業 | 検証 |
|------|------|------|
| a0 | 観測 or「余裕でやれ」 | 文書 |
| a1 | clone + AppState + **common.rs & system.rs** | compile |
| a2 | headers 抽出（actor/power 引数）+ coin-charge | relay unit |
| a3 | settings（Chesterton 維持が既定） | |
| a4 | auth forget（SC） | integration |
| a5 | 坏 secret → 401 | unit |
| a6 | docs | |

### 3.3 DoD

InProcess で3経路 TCP 無し可。Local/Cloud HTTP。Stripe/MCP 未変更。OXP actor/power の既存値が維持。

**実装記録（2026-07-21）**: a0–a6 ✅。`nurture_s2s` 5 unit + relay 2 PASS。settings の URL 空 skip（Chesterton）は維持。

---

## 4. P5-b — Mode UI

| 段階 | 内容 |
|------|------|
| b-read | `get_nurture_status` 表示 |
| b-write | `{data_dir}/.nurture_mode` + 再起動 + gitignore |

優先: env ≫ ファイル ≫ InProcess。再 spawn 禁止。

---

## 5. P5-c — Plugin CommerceEngine 正本化

### 5.1 ブロッカー

1. 新 ADR Accept  
2. **boot 分割/並べ替え設計**（下記）Accept  
3. SC  

### 5.2 設計（再発明禁止・二重台帳禁止）

**禁止**:
- assemble 後の `state.commerce_engine` だけ置換（gig/docker/browser/marketplace が旧 Arc 保持）
- `create_plugin` を `init_core_services` **全体より前**に移す（現状 `core.auth_manager` / `core.event_sender` 必須。`_event_sender` は未使用でも引数・auth 依存は残る → 署名改変で無理に前倒ししない）
- Bridge の欠けを埋めるため第2 Engine を新造

**依存事実**（`plugins.rs` → `create_plugin`）: `db.*` + `core.event_sender` + `core.auth_manager`。commerce Factory よりは前に置けるが、auth/event より前には置けない。

**本線（いずれか）**:

| 案 | 内容 | 採否 |
|----|------|------|
| **C1'（推奨）** | `init_core_services` 内: auth_manager + event_sender 生成直後に InProcess なら `create_plugin` / registry 登録 → **Factory スキップ** → 同じ `Arc` を gig/docker/browser と戻り値に渡す。assemble の marketplace もその Arc | **本線** |
| **C2** | `init_core_services` を明示二関数化（非 commerce → Plugin → commerce 依存）。C1' と同順だが境界がテストしやすい | 同等可 |
| ~~C1 全面前倒し~~ | boot_sequence で core より前に Plugin | **不可**（auth/event 未生成） |

Local/Cloud は現行 Factory。forget/DLQ は P5-a。二重 Hook Negative。  
**c0 追加**: `NurtureCommerceBridge` が Gig/Conductor が呼ぶ `CommerceEngine` メソッドを満たすかコード突合（足りなければ P5-c を分割 or 延期。新 Engine 禁止）。

### 5.3 ステップ

| Step | 作業 | 検証 |
|------|------|------|
| c0 | ADR + **C1'/C2** Accept + Bridge メソッドカバレッジ突合 | 文書 |
| c1 | core 内 C1'（または C2）+ `PluginRegistry::commerce_engine` | 単一 Arc（gig ≡ AppState ≡ marketplace） |
| c2 | InProcess で Stripe self-HTTP なし / Factory 非生成 | 統合 |
| c3 | 二重 Hook Negative | |
| c4 | docs | |

### 5.4 DoD

InProcess で `AppState` / gig / conductors / marketplace が **同一** `NurtureCommerceBridge` Arc。Factory Stripe 非生成。

---

## 6. P5-d — 別 OP

親 Q5。本 OP クローズ条件外。

---

## 7. 横断ルール

1. ID 指定 Scope Lock  
2. 偽成功禁止  
3. S2S 認証再発明禁止 / OXP actor·power の暗黙統一禁止  
4. Positive + Negative  
5. CHANGELOG / RIPPLE / 必要時 SYNERGY・OPERATIONS・README  

---

## 8. /perfect-plan（v1.5）

### Gate 1
- ✅ v1.4 資産維持（P5-a に新ギャップなし）  
- ✅ **C1 全面前倒しは不可**: `register_in_process_plugins` が `core.auth_manager` / `core.event_sender` 依存（`plugins.rs` / `create_plugin`）→ **C1'（commerce 直前）/ C2** に修正  
- ✅ `_event_sender` 未使用でも署名改変で前倒ししない（車輪・回帰）  
- ✅ Bridge カバレッジを c0 ゲートに追加（不足時の第2 Engine 禁止）  

### Gate 2–3
- ✅ a は現行 boot のまま。c のみ core 内順序変更  

### Gate 4
1. **最悪**: 偽 C1 で auth 未生成のまま Plugin → boot 破綻 → C1' に固定  
2. **前提**:「Plugin を boot 先頭へ」は依存上偽  
3. **やらないメリット**: P5-a だけで同期 G11 の大半は緩和。c は任意のまま正しい  

### Gate 5
- ✅ a →（b 並列）→ c（C1'）。**これ以上の計画 Round は証拠なき変更を避ける**（収穫逓減）  

### 判定
- [x] ✅ **PASS（v1.5）** — C1 実行可能性の誤認を実コードで修正。実装は ID 指定許可後。

---

## 9. 文書同期

| 文書 | 内容 |
|------|------|
| 本ファイル | **v1.5** |
| 親計画 / OPEN / CHANGELOG | v1.5 |

許可例: 「P5-a を実装しろ（観測ゲートは余裕でスキップしてよい）」 / 「P5-b-read を実装しろ」 / 「P5-c 用 ADR と boot 案 C1' を起草しろ」
