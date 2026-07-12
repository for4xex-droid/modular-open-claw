# Quick Start 実走チェックリスト（G1 / R3-4）

**目的**: クリーン環境で README / QUICKSTART 手順どおりに「セットアップ → ログイン → チャット」まで到達できることを確認する。

**担当**: Human（または明示の代理実行）。実装ブロッカー修正は別計画。

**正本関係**:
- 本ファイル = **実走 DoD** の唯一の手順書
- 実装アンブロック = [`nt2_quickstart_unblock_plan.md`](../roadmaps/nt2_quickstart_unblock_plan.md)
- ユーザー向け短い手順 = [`QUICKSTART.md`](QUICKSTART.md)

> ### 現状（2026-07-13）
> アンブロック実装済み。**公式 compose DoD PASS**（API 代理 + **R-B 代理**: Setup/login/Neg/chat via API+MC proxy → `down`）。OPEN **OP-078/077/079** 閉じ。
> 旧 GHCR pull 依存は廃止。release 必須 demo env（`ALLOWED_ORIGINS` / `A2A_NODE_TOKEN` 等）を compose に同梱。Generative は `dev-mock` + `AIOME_DEV_MODE` の Mock（偽 ComfyUI は使わない）。
>
> **R-A ✅**: `--no-build`（旧イメージ）→ Generative FATAL。`up -d --build` → `/health` 200 + MockGenerative。
> **R-B ✅（代理）**: クリーン volume Setup → ログイン → 誤パスワード 403 → chat SSE（**API + MC nginx proxy**；実ブラウザ DevTools 目視は任意）→ `down`。
> **R-C/R-D ✅**: release unused import cfg 化；`/app/.intent_tmp` 二重 mkdir 撤去。

---

## 事前準備

- [x] Docker Desktop が起動している（代理実走 2026-07-13）
- [x] ポート `1420`（Management Console）が空いている
- [x] 固定名コンテナなし（quickstart 3 サービスで検証）
- [x] クリーン volume（`down -v` 後に再起動。新規 clone ディレクトリは代理では同一リポジトリを使用）

```bash
lsof -i :1420 || echo "1420 OK"
docker ps --format '{{.Names}}' | grep -E '^aiome-(ollama|api-server|mc)$' || echo "OK: no name clash"
# 衝突時: docker stop aiome-ollama aiome-api-server aiome-mc 2>/dev/null || true

git clone https://github.com/motivationstudio-llc/aiome.git aiome-quickstart-verify
cd aiome-quickstart-verify
docker compose -f docker-compose.quickstart.yml down -v 2>/dev/null || true
```

---

## Step 1 — 起動（ウォーム目標: 3 分以内）

```bash
docker compose -f docker-compose.quickstart.yml up -d --build
docker compose -f docker-compose.quickstart.yml ps
```

> コード変更後・他マシンの旧イメージがある場合は **`--build` 必須**（R-A）。`--no-build` での復帰確認は Negative としてのみ使う。

- [x] 全サービスが `running` / `healthy`（代理 2026-07-13・ウォーム）
- [x] **ウォーム**で healthy 到達（cold build・初回 pull は備考で除外）

---

## Step 2 — ダッシュボード到達（目標: 30 秒）

ブラウザ: [http://localhost:1420](http://localhost:1420)

- [x] MC `http://localhost:1420/` → HTML 200；`/api/v1/bootstrap/status`（MC proxy）→ `mode=setup`（クリーン時）
- [ ] コンソールに致命的 JS エラーがない（DevTools → Console）— **任意フォロー**（代理では未実施）

---

## Step 3 — 初期セットアップとログイン

クリーン環境では既存 DB がないため **Setup Wizard** が起動します（QUICKSTART.md 準拠）。

- [x] Setup 相当: `POST /api/v1/setup/init` 成功（代理；UI ウィザード画面の目視は任意）
- [x] 管理パスワード設定・完了（setup/init）
- [x] 設定したパスワードでログイン成功（`POST /api/v1/auth/token` + MC proxy 同経路）
- [x] 誤パスワード Negative → 403
- [ ] Home / **AIとはなす** UI 操作 — **任意フォロー**（API chat で代替済）

> 開発機の既存 `aiome.db` を流用した場合のみ、既定パスワード `SuperSecretPassword123!` でログインする（クリーン検証としては無効。必ず新規 clone + 新規ボリュームで実施すること）。
> ログインが弾かれる等のトラブル時は `docs/guides/LOCAL_LOGIN_VERIFICATION.md` の復旧手順を参照。

---

## Step 4 — チャット疎通（目標: 1 分）

- [x] `POST /api/stream/chat` hello → SSE 200（API + MC proxy）
- [x] ストリーミング開始（`event: text` / `turn_end`）
- [ ] UI のエラー toast 目視 — **任意フォロー**

---

## Step 5 — 停止

```bash
docker compose -f docker-compose.quickstart.yml down
```

- [x] 正常終了（exit code 0）（代理 2026-07-13）

---

## 合格基準（DoD）

| 項目 | 基準 |
|---|---|
| 所要時間 | **ウォーム**（イメージ/レイヤ既存）で clone〜Step 4 まで **5 分以内**。cold build・初回 GHCR/Ollama pull は備考で除外可 |
| ログイン | Setup Wizard で設定したパスワードで成功 |
| チャット | 送信可能（LLM 応答 or 設定促し） |
| 回帰 | `docker compose down` 後に再起動しても Step 3 再現（`down -v` しない） |

---

## 記録テンプレート（Human → OPEN.md / CHANGELOG 用）

```
日付: YYYY-MM-DD
環境: macOS / Linux / Windows (WSL)
所要時間: __ 分
結果: PASS / FAIL
備考: （LLM 設定、ネットワーク、スクリーンショット URL 等）
```

FAIL 時は `docker compose logs` の末尾 50 行を Issue / OPEN.md に添付してください。
