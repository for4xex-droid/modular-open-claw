# Desktop 既定 InProcess 化計画（v1.3）— 製品品質最大化版

- **ステータス**: Human Q1–Q3 確定。**P0-pre…P3 ✅** + **CI sidecar 配線済**（2026-07-21）。次=P4 文書（任意）/ P5
- **OPEN**: **OP-088**（OP-062 完了。**再実装しない**）
- **Safety-Critical**: `src-tauri/src/lib.rs` / `tauri.conf.json` / capabilities / `.github/workflows`（CI）/ auth 近傍はフェーズごとに明示許可

## 0. 製品北星（これ以外は従属）

公式 Desktop を入れたユーザーが、**環境変数を知らなくても**「相棒＋経済が普通に動く」こと。  
開発者だけが知るフラグや、動いたつもり（課金・forget が沈黙）は製品バグとみなす。

| 品質軸 | ユーザーが感じること | 計画での担保 |
|--------|----------------------|--------------|
| **起動の単純さ** | ダブルクリックで動く。`NURTURE_IN_PROCESS` 不要 | P2 既定 InProcess |
| **経済の信頼性** | 課金・残高・forget が「たまに無視」されない | P1（JWT 外 `/internal` + self-URL）+ §1.3 Negative |
| **プロセスの単純さ** | タスクマネージャに謎の nurture-api がいない | P2 + P3（公式から sidecar 除外） |
| **失敗の正直さ** | 壊れたら分かる（偽成功しない） | Fail-Closed（secret/DRM/S3） |
| **逃げ道** | 隔離デバッグだけ明示スイッチ | `NURTURE_MODE=local` + 開発用ビルド |
| **配布の軽さ** | 余計なバイナリを抱えない | Q3 / P3 |

**Ship ゲート（製品として出す最小セット）**: **P0-pre → P0 → P1 → P2 → P3（公式 sidecar 除外まで）**。  
P4 文書は Ship 直後でも可。P5（磨き）は Ship 後。

---

## 1. Human 決定（2026-07-21・確定）

| # | 決定 | 確定内容 | 禁止・注意 |
|---|------|----------|------------|
| **Q1** | 経済経路 | **A' 自己 HTTP** + 既存 `internal_routes()` / `internal_auth_middleware` を **JWT `auth_middleware` の外**で `nest("/internal", …)`。`NURTURE_API_URL=http://127.0.0.1:3015` | **Plugin `merge_routes`（JWT 配下）に `/internal` を載せない**。「Plugin 側へ」≠ `nurture_routes`。CommerceEngine DI は後回し（ADR-012 D3） |
| **Q2** | Local 復帰 | **`NURTURE_MODE=local`**（正本）。`in_process` / `cloud` / `disabled` も同変数に寄せる | `FORCE_LOCAL` 単独語彙は作らない。設定 UI トグルは **P5** |
| **Q3** | 公式パッケージ | **nurture-api を外す**。開発は `--with-nurture-sidecar` | 同梱して非 spawn はしない（サイズ・誤解） |
| Q4 | obscura | 本計画外 | — |
| Q5 | OSS Desktop 別チャネル | defer（別 OP） | Ship を止めない |

**後回しで質が上がるが今はやらない（P5）**

| ID | 内容 | なぜ後 |
|----|------|--------|
| P5-a | 自己 HTTP → tower/oneshot 内部呼び出し（G11 本解消） | A' で Ship 可能。負荷で問題が出てから |
| P5-b | Settings UI で Mode 切替 | env 正本が先。UI は誤操作面が増える |
| P5-c | CommerceEngine DI（Q1-B） | ADR 改訂・二重台帳。差分最大 |
| P5-d | OSS / Economy 二系統パッケージ | 配布運用コスト |

---

## 2. 設計原則（品質用に短縮）

1. **偽成功禁止** — skip/warn だけの経済・RTBF は不合格  
2. **単一の正本** — Desktop 通常起動 = InProcess。Local は明示モードのみ  
3. **契約再利用** — S2S は既存 Bearer secret + OXP。新認証を発明しない  
4. **ビルド再利用** — `desktop_sidecar_manager.py` 拡張のみ。新オーケストレータ禁止  
5. **JWT と S2S を混ぜない** — G9  
6. **配布物 = 実行形態** — 公式に無いバイナリに依存しない（Q3）

---

## 3. 現行ギャップ（実装が解くもの・要約）

詳細アンカーは v1.2 監査を継承。Ship に直結するものだけ列挙する。

| ID | 問題 | 解くフェーズ |
|----|------|--------------|
| G10 | default `cloud-storage` → S3 無しで Plugin 失敗 | **P0-pre** |
| G1/G12 | InProcess で secret/DRM 未注入（scrub あり） | **P0** |
| G2/G7/G9 | `/internal` 欠落 or JWT 下だと S2S 死 | **P1** |
| G3/G8 | URL 欠落 skip・MCP 503 | **P1** |
| G6 | Local escape なし | **P2** |
| B1–B5/B11 | sidecar 二重同梱・externalBin・CSP :3020 | **P3** |
| G11 | 自己 HTTP デッドロック | **監視は P1 / 本解消は P5-a** |

---

## 4. Ship フェーズ（製品品質の本線）

### P0-pre — Desktop features（S3 依存切断） ✅ 2026-07-21

**許可**: 「Desktop InProcess P0-pre を実装しろ」（P0 に含める明示でも可）

| ID | 作業 | 品質検証 |
|----|------|----------|
| P0-pre-1 | ✅ `nurture` = `default-features=false` + `nurture-api/desktop` + `nurture-infra/stripe`。退避 `nurture-cloud` | `cargo tree … -i aws-sdk-s3` 不一致 / check PASS |
| P0-pre-2 | ✅ `desktop_sidecar_manager` コメント同期（コマンドは引き続き `--features nurture`） | |

---

### P0 — 起動硬化（まだ既定は変えない） ✅ 2026-07-21

**許可**: 「Desktop InProcess P0 を実装しろ」

| ID | 作業 | 品質検証 |
|----|------|----------|
| P0-1 | ✅ InProcess で secret + DRM 注入 + `NURTURE_IN_PROCESS=true`（**P1 で self-URL 追加**） | unit: `in_process_api_env` / DRM resolve |
| P0-2 | ✅ plugins debug 固定 DRM 廃止 → `require_drm_master_key` Fail-Closed | Negative: missing/empty DRM fail |
| P0-3 | ✅ unit: 注入分岐 + DRM persist | Tauri `in_process`/`resolve_drm` + api-server `require_drm` PASS |

**DoD**: 明示 InProcess で「起動＋Plugin」。**経済成功を主張しない。**  
**Out**: 既定フリップ、`/internal`、公式 sidecar 削除。

---

### P1 — 経済・RTBF・MCP（製品の心臓） ✅ 2026-07-21

**許可**: 「Desktop InProcess P1 を実装しろ」（Q1=A' 確定済）

```text
P1-1  ✅ JWT 外 nest_service("/internal", s2s_internal_service + internal_auth)
        ※ Plugin nurture_routes / merge_routes 配下に置かない。Plugin Router<()> は with_state 後 JWT merge
P1-2  ✅ InProcess 時 NURTURE_API_URL=http://127.0.0.1:3015（Tauri in_process_api_env）
P1-3  ✅ discovery 正本 = nurture-mcp プロキシ（両モード）。InProcess upstream=`/mcp` + JWT 転送。
        SSE endpoint 広告は InProcess=`/mcp/message`（plugin）。proxy がパス書換
P1-4  ✅ ADR-012 Amendment → Accepted（P2 ゲート通過済）
```

| 検証 | 内容 |
|------|------|
| Positive | unit: self-URL 注入 / MCP path·rewrite / S2S nest。既存 forget・coin-charge クライアント契約は self-URL で刺さる |
| Negative | ✅ 不正 secret → 401（`Invalid internal credentials`）。JWT 外 nest |
| §1.3 | URL 注入により settings/DLQ/relay の沈黙 skip 経路を閉鎖 |
| G11 | 監視継続。デッドロック出たら P5-a |

**ゲート**: P1 未達なら **P2 禁止**。加えて **ADR Amendment Accept** まで P2 禁止。

---

### P2 — 既定フリップ（北星の到達） ✅ 2026-07-21

**許可**: 「Desktop InProcess P2 を実装しろ」+ ADR Amendment Accept

| ID | 作業 |
|----|------|
| P2-1 | ✅ else → **InProcess**（`test_nurture_mode_default_is_in_process`） |
| P2-2 | ✅ `NURTURE_MODE` 正本 + 互換（DISABLED/CLOUD/IN_PROCESS） |
| P2-3 | ✅ tray hint + `.env.example`（通常は設定不要） |
| P2-4 | ✅ InProcess 中 :3020 応答で二重 Hook 警告 |

```text
1. MODE=disabled / NURTURE_DISABLED     → Disabled
2. MODE=cloud / NURTURE_CLOUD_URL       → Cloud
3. MODE=local                           → Local（escape・公式は sidecar 無しなので開発ビルド向け）
4. MODE=in_process / NURTURE_IN_PROCESS → InProcess（明示）
5. else                                 → InProcess  ← 製品既定
```

**品質注記**: 公式パッケージ（P3 後）では `MODE=local` は「sidecar が無い」ため失敗する。tray/ドキュメントで **開発用プロファイルが必要**と明示（ユーザーを迷子にしない）。

---

### P3 — 配布形態 = 実行形態（北星の完成） ✅ 2026-07-21

**許可**: 「Desktop InProcess P3 を実装しろ」（externalBin/CSP=T-003 同期済み）

| ID | 作業 | 品質への効き |
|----|------|----------------|
| P3-2 | ✅ 既定ビルド = api-server + key-proxy。`--with-nurture-sidecar` で Local 用 | 公式が軽い |
| P3-3 | ✅ externalBin / capabilities から nurture-api 削除。CSP **:3020** 除去（T-003） | 誤解・死んだポート削除 |
| P3-4 | ✅ `package.json` `sidecar:*` → Python | DX |
| P3-5 | ✅ `--check-all` 混入禁止 + **ci.yml `desktop-sidecar` 配線済**（build → check-core --forbid-nurture-sidecar） | 回帰で重いバイナリが戻らない |
| P3-6 | ✅ T-002/T-006 / desktop-sidecar.md / OPERATIONS_MANUAL | 「普通は設定不要」 |

**Ship 完了条件**: P0-pre〜P3 完了 + P1/P2 検証 PASS。

---

### P4 — 文書同期（Ship 直後可）

ADR-012 / SYNERGY / foolproof / wave / OPEN / CHANGELOG / RIPPLE / 「CI で check」嘘の解消。

---

### P5 — Ship 後の品質積み上げ（任意）

| ID | 内容 | 着手条件 |
|----|------|----------|
| P5-a | 自己 HTTP → 内部呼び出し（デッドロック本解消） | P1-g11 で問題観測 or 余裕 |
| P5-b | Settings の Mode UI | Human が「製品 UI で切り替えたい」 |
| P5-c | CommerceEngine DI | 別 ADR + SC |
| P5-d | OSS 別パッケージ | 別 OP |

---

## 5. 実行順（品質最大化）

```text
P0-pre → P0 → P1 【必須ゲート】→ P2 → P3  =  Ship（製品）
                                      ↓
                                    P4 文書
                                      ↓
                              （任意）P5 磨き
```

| 禁止 | 理由（製品） |
|------|----------------|
| P2 を P1 より先 | 全ユーザーに沈黙経済を配る |
| `/internal` を Plugin+JWT 下へ | RTBF/課金が 401（最悪の「壊れた製品」） |
| 公式に sidecar 同梱したまま既定 InProcess だけ | 「動かないバイナリ」を配る半端な質 |
| DI/新ビルドシステムを Ship に混ぜる | 遅延と回帰で質が落ちる |

---

## 6. エンドツーエンド受け入れ（Ship デモ）

1. **Positive（製品）**: env なし相当で起動 → nurture 子プロセスなし → 残高/課金相当 API 成功 → forget が Nurture 側まで届く  
2. **Negative**: DRM/secret 欠落で起動失敗 or 明確エラー。不正 internal secret → 401。settings が沈黙 Ok だけにならない  
3. **Escape（開発）**: `--with-nurture-sidecar` + `NURTURE_MODE=local` + **dev 用に `externalBin`/`capabilities` へ nurture-api を一時追加** → sidecar 起動・Hook 単一（公式 conf のままでは Tauri が sidecar を解決できない）  

4. **Revert**: フラグ無しに戻すと InProcess 既定  

---

## 7. 影響範囲

| 領域 | フェーズ |
|------|----------|
| Cargo desktop features | P0-pre |
| Tauri `lib.rs` | P0, P2（SC） |
| `plugins.rs` | P0 |
| api-server `router`（JWT 外 `/internal`） | P1（SC 近傍・明示許可） |
| forget / MCP / settings | P1 |
| `desktop_sidecar_manager` / tauri.conf / capabilities / CI | P3（SC） |
| **非対象（Ship）** | CommerceEngine DI、Stripe webhook 署名、OP-020-F5、obscura、OSS 二系統 |

---

## 8. 着手条件

| フェーズ | 許可フレーズ |
|----------|----------------|
| P0-pre | 「Desktop InProcess P0-pre を実装しろ」 |
| P0 | 「Desktop InProcess P0 を実装しろ」 |
| P1 | 「Desktop InProcess P1 を実装しろ」 |
| P2 | 「Desktop InProcess P2 を実装しろ」 |
| P3 | 「Desktop InProcess P3 を実装しろ」+ externalBin（+ CI 明示） |
| P5-* | 都度明示 |

Q1–Q3 は **回答済み**。P1/P2/P3 の「Q 待ち」は解除。

---

## 9. 旧版からの差分（v1.2 → v1.3）

- Human 決定を **確定表**に固定（A' / `NURTURE_MODE` / sidecar 除外）  
- 「Plugin 側」誤解を排除し **JWT 外**を製品制約として明記  
- **Ship = P0-pre…P3**（sidecar 除外まで本線）。文書は P4、磨きは P5  
- 品質北星・E2E デモ・「公式 Local は開発プロファイルが必要」を追加  
- 技術ギャップ表は Ship 直結のみに圧縮  

---

## 10. `/perfect-plan` 継承

v1.2 Round 2 の PASS 根拠（G9/G10/G12、reuse）を維持。v1.3 は製品スコープの最適化であり、技術ゲートの緩和ではない。
