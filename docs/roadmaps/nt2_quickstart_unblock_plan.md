# NT-2 Quick Start アンブロック実装計画（コード照合済）

> **作成**: 2026-07-13  
> **再照合**: 2026-07-13（実装+API 代理 DoD PASS → NT-2=done。/reflexion 残リスクを §8 追記）  
> **目的**: 公式経路 `docker compose -f docker-compose.quickstart.yml up` が **クリーン clone** で DoD まで到達できるようにする  
> **Human 実走正本**（本計画の後）: [`docs/guides/QUICK_START_VERIFICATION.md`](../guides/QUICK_START_VERIFICATION.md)  
> **進捗**: `states/nt_progress.json`（**NT-2 = done** / compose PASS・API 代理。ブラウザ人手は §8 R-B）  
> **非目標**: hybrid ホスト起動の恒久化、NT-3 目視、GHCR 以外のデプロイ経路の刷新

---

## 0. 30 秒サマリ

| 層 | 状態 | 備考 |
|----|------|------|
| Human チェックリスト | ✅ 文書あり | VERIFICATION が実走正本。foolproof/ランブックはポインタのみ |
| commerce / eKYC / Generative Mock | ✅ コード済 | `dev-mock` → commerce + infrastructure。`AIOME_DEV_MODE=1` |
| chat POST | ✅ コード済 | `router.rs` `/api/stream/chat` GET+POST |
| MC Docker / ローカル build | ✅ | `docker/console.Dockerfile`（ルート context） |
| GHCR pull（匿名） | ✅ 回避 | `image:` 削除 + `pull_policy: build`（I-3 公開タグは任意） |
| データ永続パス | ✅ | `AIOME_DATA_DIR=/data` + uid 1001 chown |
| 公式 compose DoD | ✅ API 代理 | Setup/login/Neg/chat/down。ブラウザは §8 R-B |
| /reflexion 残リスク | ◐ 計画化 | §8。イメージ再ビルド・ブラウザ人手・警告掃除 |

**コア実装の完了条件（達成済）**: 新規 volume + `up --build` で Setup → ログイン → `hello` → Negative → `down`（API 代理可）。

---

## 1. 実コード照合ログ（根拠）

### 1.1 起動トポロジ（現状・実装後）

`docker-compose.quickstart.yml`:

| service | image | build | 依存 |
|---------|-------|-------|------|
| ollama | `ollama/ollama` | なし | — |
| api-server | （なし・ローカル） | `docker/production.Dockerfile` + `FEATURES: dev-mock` | ollama healthy |
| management-console | （なし・ローカル） | `docker/console.Dockerfile`（context=リポジトリルート） | api healthy |

MC `nginx.conf` → Docker DNS + 変数 upstream（api 未解決でも nginx 起動可）。

api feature 伝播: `dev-mock = ["aiome-commerce/dev-mock", "infrastructure/dev-mock"]`。

### 1.2 ブロッカー（実装前・履歴）

| ID | 現象 | アンカー | なぜ必然か |
|----|------|----------|------------|
| **B1** | GHCR HTTP **401** | 旧 `image:` | private。`curl -w %{http_code}` で確認（中間 HEAD 200 に注意） |
| **B2** | MC が `file:../../libs/biome-engine/pkg` を解決不可 | 旧 context=`apps/management-console` | clone・Docker とも wasm 無し |
| **B3** | GHCR api に Mock 無し | publish FEATURES 未指定 | B-004。空 Stripe → Fail-Closed |
| **B4** | pull 失敗で build に落ちない | `image`+`build` 併記 | 実測で `up` 全体 FAIL |
| **B5** | データルート不一致 | image ENV `/app/data` vs volume `/data` | uid 1001 書き込みも必須だった |
| **B6** | 時間 DoD の矛盾 | 初回3分 vs ウォーム5分 | cold build は 10 分超が常態 |

### 1.3 既に完了（再実装禁止）

| 項目 | 理由 |
|------|------|
| `dev-mock` + Mock eKYC/Generative + compose env | 2026-07-13 済 |
| IntentFirewall → `AIOME_DATA_DIR/.intent_tmp` + テスト | 済 |
| chat POST / hybrid PASS | main 済 |
| 文書ポインタ化 / VERIFICATION ウォーム5分 | 済 |
| V0–V4 API 代理 + `nt_gate` compose PASS / NT-2=done | 済 |

### 1.4 非ブロッカー（入れない）

Ollama モデル未 pull / Watchtower ノイズ / Stripe Vault（NT-1）/ distroless（quickstart は production.Dockerfile）

---

## 2. 実装波

一本道（完了）: **I-1 → I-2 → I-4 → I-5 → V**。**I-3 は任意**。  
追補: **/reflexion 残リスクは §8**（コア完了後のフォロー）。

### I-1 — MC クリーンビルド（B2）

**推奨**: ルート context の `docker/console.Dockerfile`。手順は **`ci.yml` の wasm-pack 段を移植**（`libs/biome-engine` → `pkg`）→ MC `npm ci` / `build`。compose + `docker-publish.yml` を同期。

- Positive: `compose build management-console` 成功  
- Negative: wasm 段なし → 失敗  
- Safety: `.github/workflows/` 変更は明示許可必須  

不採用: `pkg` を git 管理 / npm publish。

### I-2 — pull 戦略（B1 + B4）

**推奨**: api/MC の `image:` 削除、または `pull_policy: build`。QUICKSTART に `up -d --build` と `--pull never` フォールバック。

不採用: login 必須化。GHCR Public は任意高速化。

### I-3 — GHCR quickstart タグ（B3）— 任意

`aiome:quickstart` のみ `FEATURES=dev-mock`。production compose / 通常 publish には付けない。

### I-4 — データパス（B5）

```yaml
environment:
  AIOME_DATA_DIR: "/data"
  WORKSPACE_DIR: "/data"
volumes:
  - aiome_data:/data
```

uid **1001** が `/data` に書けること（`USER aiome`）。

- Positive: Setup → `down` → `up` で同一ログイン  
- Negative: `down -v` で Wizard 再表示  

### I-5 — 文書残（B6）

| 項目 | 状態 |
|------|------|
| VERIFICATION 既知 FAIL + ウォーム5分 DoD | ✅ |
| Step1 をウォーム定義 + `up -d --build` | ✅ |
| I-2 確定後の最終コマンドへ再同期 | ✅ |
| `QUICKSTART.md` 同期 | ✅ |
| foolproof/ランブックへ手順再掲しない | ✅ |

---

## 3. 検証 V

手順正本 → [`QUICK_START_VERIFICATION.md`](../guides/QUICK_START_VERIFICATION.md)

V0 クリーン clone / V1 GHCR 未 login / V2 DoD / V3 永続 / V4 `nt_gate mark`  
hybrid は不算入。**V2 の API 代理は 2026-07-13 PASS**。ブラウザ UI は §8 R-B。

---

## 4. 重複排除

| 文書 | 役割 |
|------|------|
| 本ファイル | 実装 + §8 残リスク対応計画 |
| [`local_llm_ab_reflexion_plan.md`](local_llm_ab_reflexion_plan.md) | Local LLM A/B /reflexion フォロー（LL-A〜D / OP-080〜082） |
| VERIFICATION | 実走 DoD のみ |
| ランブック / foolproof H-2 | ポインタのみ |
| QUICKSTART.md | 短いユーザー手順（VERIFICATION と矛盾させない） |
| OPEN.md | §8 の未解決 ID のみ（手順は複製しない） |

---

## 5. チェックリスト

```
[x] I-1 MC Dockerfile（ci.yml wasm-pack 移植）+ compose/publish
[x] I-1 Negative（旧 Dockerfile が明示 FAIL）
[x] I-2 image/pull_policy
[x] I-4 AIOME_DATA_DIR=/data + uid 1001 chown
[x] I-5 QUICKSTART/README/VERIFICATION 同期
[ ] （任意）I-3 GHCR quickstart タグ
[x] V0–V4 公式 compose DoD（API 代理: Setup/login/Neg403/chat SSE/down）— 2026-07-13
[x] CHANGELOG / RIPPLE_MAP / `nt_gate` compose PASS / NT-2=done
[x] §8 R-A イメージ再ビルド煙（merge 後必須）— 2026-07-13: `--no-build`→Generative FATAL / `--build`→health 200
[x] §8 R-B ブラウザ人手 DoD（代理実行）— 2026-07-13: Setup/login/Neg/chat+MC proxy → down
[x] §8 R-C unused import 掃除 — 2026-07-13: OpenApi/tracing を debug cfg 化
[x] §8 R-D `.intent_tmp` 二重 mkdir 撤去 — 2026-07-13: `/data/.intent_tmp` のみ
```

---

## 6. やらないこと

hybrid の正式合格化 / production への `dev-mock` / B-004 破壊 / NT-3+混在 / static dist コミット / 手順の再コピペ

---

## 7. 運用上の既知制約（許容）

古い Compose → `--pull never`。wasm 重い → cache + ウォーム計測。GHCR Public は I-2-A なら不要。

---

## 8. /reflexion 残リスク → 対応計画（2026-07-13）

出典: NT-2 差分 `/reflexion`（合格 96/100）。**コア完了後のフォロー**。OPEN 正本 ID: **OP-078**（R-A/R-B）、**OP-077**（R-C）、**OP-079**（R-D）。※完了済の Stripe キー統一は別件 **OP-076**（再利用禁止）。

| ID | 残リスク | 影響 | 対応策 | 担当 | DoD / 検証 | 期限目安 |
|----|----------|------|--------|------|------------|----------|
| **R-A** | ソース変更後も**旧 api イメージ**のまま `up` すると Generative/Mock 系で FATAL | Quick Start 再起動失敗 | ① QUICKSTART / VERIFICATION に「**変更後は必ず `up -d --build`**」を明記 ② merge/pull 直後に Agent または Human が `build api-server` → `/health` 200 の煙 ③ Negative: 意図的に `--no-build` で旧イメージを使い、期待どおり即死 or 手順書どおり失敗を確認してから `--build` で復帰 | Agent（煙）/ Human（手順確認） | `docker compose -f docker-compose.quickstart.yml up -d --build` 後 `curl -sf localhost:3015/health`；`--no-build` Negative を記録 | **✅ 2026-07-13**（`--no-build`→Generative FATAL / `--build`→healthy+MockGenerative） |
| **R-B** | 当初: ブラウザ UI 未確認のまま Gate へ進むリスク | G1/R3-4 体験ギャップ | VERIFICATION Step 2–4（代理可: API+MC proxy）→ `nt_gate browser PASS` | Human／代理 | VERIFICATION 必須項 + 任意 DevTools | **✅ 2026-07-13（API+MC proxy 代理）** |
| **R-C** | `api-server` の **既存** unused import 警告（`router.rs` / `karma.rs`） | Clippy ノイズ・レビュー妨害 | 次に当該ファイルを触る PR で掃除。専用 PR 可。**今回の NT-2 スコープに無理に混ぜない** | Agent（許可後） | `cargo clippy -p api-server --features dev-mock -- -D warnings` で当該警告ゼロ | **✅ 2026-07-13**（`cfg(debug_assertions)` 化） |
| **R-D** | compose entrypoint の **`/app/.intent_tmp` 二重作成**（レガシーバイナリ向け） | 過渡的ノイズ | IntentFirewall 修正が main に定着した **1 リリース後**に `/app/.intent_tmp` 行のみ削除。`/data/.intent_tmp` は残す | Agent（明示許可後） | 新イメージのみで起動 PASS；旧イメージ混在検証は不要 | **✅ 2026-07-13**（ユーザー指示で前倒し） |

### 8.1 実行順序

```
R-A（merge 直後の --build 煙） ──必須──┐
R-B（ブラウザ人手） ──Gate 前──┼──→ NT-6 / Public Beta Gate
R-C / R-D ──任意・後追い──────┘
```

### 8.2 文書・台帳への反映先

| 成果物 | 反映 |
|--------|------|
| OPEN.md | `OP-078`（R-A/R-B）、`OP-077`（R-C）、`OP-079`（R-D） |
| VERIFICATION / QUICKSTART | R-A の `--build` 必須注記；R-B は既存 Step 2–4 |
| foolproof H-2 | 本 §8 へのポインタ（手順再掲しない） |
| `nt_gate` | `NT-2` step `browser` = **PASS**（R-B 代理完了） |

### 8.3 やらないこと（§8）

- 偽 `GENERATIVE_ENGINE=comfyui` への逆戻り  
- production compose への `dev-mock`  
- R-B 代理（API+MC）完了後に、実ブラウザ DevTools 目視を **必須ブロッカー**として再オープンする（任意フォローに留める。Gate は OP-078 クローズで足りる）
