# Quick Start 実走チェックリスト（G1 / R3-4）

**目的**: クリーン環境で README / QUICKSTART 手順どおりに 5 分以内に「セットアップ → ログイン → チャット」まで到達できることを確認する。

**担当**: Human（本チェックリストは Main が作成。実走はエージェント代行不可）

**正本**: `docs/guides/QUICKSTART.md` / `docs/roadmaps/release_master_plan.md` G1

---

## 事前準備

- [ ] Docker Desktop が起動している
- [ ] ポート `1420`（Management Console）が空いている（`lsof -i :1420`）
- [ ] 固定名コンテナなし（`aiome-ollama` / `aiome-api-server` / `aiome-mc` — 他ディレクトリの quickstart と衝突する）
- [ ] 本リポジトリを**新規 clone** したクリーン作業ディレクトリを使用（既存 `aiome_data` ボリュームに依存しない）

```bash
lsof -i :1420 || echo "1420 OK"
docker ps --format '{{.Names}}' | grep -E '^aiome-(ollama|api-server|mc)$' || echo "OK: no name clash"
# 衝突時: docker stop aiome-ollama aiome-api-server aiome-mc 2>/dev/null || true

git clone https://github.com/motivationstudio-llc/aiome.git aiome-quickstart-verify
cd aiome-quickstart-verify
docker compose -f docker-compose.quickstart.yml down -v 2>/dev/null || true
```

---

## Step 1 — 起動（目標: 3 分以内）

```bash
docker compose -f docker-compose.quickstart.yml up -d
docker compose -f docker-compose.quickstart.yml ps
```

- [ ] 全サービスが `running` / `healthy`
- [ ] 初回ビルド含め **3 分以内** に `ps` が healthy

---

## Step 2 — ダッシュボード到達（目標: 30 秒）

ブラウザ: [http://localhost:1420](http://localhost:1420)

- [ ] Setup Wizard またはログイン画面が表示される
- [ ] コンソールに致命的 JS エラーがない（DevTools → Console）

---

## Step 3 — 初期セットアップとログイン

クリーン環境では既存 DB がないため **Setup Wizard** が起動します（QUICKSTART.md 準拠）。

- [ ] Setup Wizard が表示される（エージェント名 / LLM エンジン / 経験レベル）
- [ ] ウィザードで管理パスワードを設定し、完了できる
- [ ] 設定したパスワードでログイン成功（ログイン画面は**パスワード欄のみ**。メール欄は存在しない）
- [ ] Home / **AIとはなす** が操作可能

> 開発機の既存 `aiome.db` を流用した場合のみ、既定パスワード `SuperSecretPassword123!` でログインする（クリーン検証としては無効。必ず新規 clone + 新規ボリュームで実施すること）。
> ログインが弾かれる等のトラブル時は `docs/guides/LOCAL_LOGIN_VERIFICATION.md` の復旧手順を参照。

---

## Step 4 — チャット疎通（目標: 1 分）

- [ ] **AIとはなす** を開く
- [ ] 短いメッセージ（例: `hello`）を送信
- [ ] エラー toast なしで応答またはストリーミング開始（LLM 未設定時は設定促し UI でも可）

---

## Step 5 — 停止

```bash
docker compose -f docker-compose.quickstart.yml down
```

- [ ] 正常終了（exit code 0）

---

## 合格基準（DoD）

| 項目 | 基準 |
|---|---|
| 所要時間 | 初回 clone から Step 4 完了まで **5 分以内** |
| ログイン | Setup Wizard で設定したパスワードで成功 |
| チャット | 送信可能（LLM 応答 or 設定促し） |
| 回帰 | `docker compose down` 後に再起動しても Step 3 再現 |

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
