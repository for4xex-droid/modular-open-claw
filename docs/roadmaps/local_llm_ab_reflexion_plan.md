# Local LLM A/B + ViewMode /reflexion フォロー計画

> **作成**: 2026-07-13  
> **出典**: Local LLM A/B 実装 + `/reflexion`（Loop2 合格 96/100）の**残リスク**  
> **改訂**: 2026-07-19 — OP-081 git 分割コミット ✅（OPEN 2026-07-13）  
> **目的**: Pattern B 実機未検証・NT-3 目視を、NT-2 §8 と同型の台帳で後追い可能にする  
> **非目標**: Pattern A スタックの即時切替、本番 compose 変更、`.env` のコミット

---

## 0. 30 秒サマリ

| 層 | 状態 | 備考 |
|----|------|------|
| Pattern A（Docker Ollama + `gemma4:e4b`） | ✅ 実機確認済 | 現行 quickstart 既定 |
| Pattern B compose（`depends_on: !reset null`） | ✅ config + **実機 PASS** | LL-A / OP-080 ✅ 2026-07-13 |
| ViewModeProvider + テスト 15 件 | ✅ PASS | MC dist 反映済（quickstart） |
| disk_hygiene / local_llm_setup | ✅ スクリプト + bash -n | — |
| git コミット | ✅ OP-081 2026-07-13 | LL-B 完了 |
| NT-3 Biome 目視 | ❌ Human-only | 既存 **OP-002** → **LL-C** |
| Linux Pattern B | ○ 未検証 | macOS 前提。必要時 **LL-D** |

**/reflexion Loop2 でコード修正済み（残リスクではない）**: `depends_on: !reset null`、`pattern-b-up --force-recreate`、`disk_hygiene` の `cargo clean` 順序。

---

## 1. 正本・重複排除

| 文書 | 役割 |
|------|------|
| 本ファイル | 残リスク LL-A〜D の対応計画・DoD |
| [`QUICKSTART.md`](../guides/QUICKSTART.md) | Pattern A/B ユーザー手順 |
| [`scripts/local_llm_setup.sh`](../../scripts/local_llm_setup.sh) | 切替・衛生コマンド |
| [`remaining_work_foolproof_plan.md`](remaining_work_foolproof_plan.md) §2.H-2.5 | Human/Agent 入口（ポインタ） |
| [`OPEN.md`](../../OPEN.md) | **OP-080 / OP-081 / OP-082** のみ（手順は複製しない） |
| NT-3 目視 | [`foolproof` H-3](remaining_work_foolproof_plan.md) / **OP-002** |

---

## 2. /reflexion 残リスク → 対応計画

| ID | 残リスク | 影響 | 対応策 | 担当 | DoD / 検証 | OPEN |
|----|----------|------|--------|------|------------|------|
| **LL-A** | **Pattern B 実機** | — | ✅ 2026-07-13 | Human または Agent | Positive/Negative/A復帰 記録済（§4.4） | **OP-080** ✅ |
| **LL-B** | ViewMode / LLM compose+scripts / disk hygiene / CHANGELOG 等が**未コミット** | ロールバック困難・レビュー不能 | ユーザー「コミットしろ」承認後、論理単位で分割: (1) ViewModeProvider + test.tsx (2) `docker-compose.quickstart.native-ollama.yml` + `local_llm_setup.sh` (3) `disk_hygiene.sh` + `.gitignore` (4) docs。**.env / 秘密は除外** | Agent（ユーザー依頼時） | 各コミットで関連テスト PASS；`git status` クリーン（意図的 untracked 除く） | **OP-081** |
| **LL-C** | **NT-3 Biome 目視** | — | ✅ 2026-07-13 Human PASS（cockpit → ワールド + Negative） | **Human-only** | OPEN **OP-002** `[x]` | **OP-002** ✅ |
| **LL-D** | Pattern B が **Linux で `host.docker.internal` 未保証** | Linux 開発者が Pattern B 失敗 | macOS では現状 doc のみ。Linux 需要が出た PR で `extra_hosts` 追加 + VERIFICATION 1 行。今は実装しない | Agent（Linux 需要ゲート後） | Linux ホストで `pattern-b-up` → api healthy | **OP-082**（任意・低優先） |

---

## 3. 実行順序

```
LL-C（NT-3 目視） ── Human 並行可 ──→ OP-002 クローズ
LL-A（Pattern B 実機） ── macOS 需要時・NT-3 前後どちらでも可 ──→ OP-080 クローズ
LL-B（git 整理） ──ユーザー「コミットしろ」後──→ OP-081 クローズ
LL-D ── Linux 需要が出るまで保留──→ OP-082
```

**推奨**: 日常開発は **Pattern A 維持**。LL-A は Metal 検証や B 手順の回帰確認時のみ実行し、終了後は **必ず pattern-a-up で復帰**。

---

## 4. LL-A 詳細手順（正本）

### 4.1 Positive（Pattern B）

```bash
cd /path/to/aiome
./scripts/local_llm_setup.sh pattern-b-check
./scripts/local_llm_setup.sh pattern-b-up
docker inspect aiome-api-server --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -E '^OLLAMA_|^LLM_MODEL='
curl -sf http://localhost:3015/health
# 任意: MC ログイン後 chat 1 往復、または settings で Ollama 検出確認
```

### 4.2 Negative（競合注入）

```bash
docker compose -f docker-compose.quickstart.yml up -d ollama   # 意図的に container Ollama を起動
./scripts/local_llm_setup.sh pattern-b-up   # 失敗 or 誤バックエンドを観察・記録
docker stop aiome-ollama
./scripts/local_llm_setup.sh pattern-b-up   # 復帰確認
```

### 4.3 復帰（Pattern A）

```bash
./scripts/local_llm_setup.sh pattern-a-up
docker exec aiome-ollama ollama list | head -5
```

### 4.4 完了記録

```
LL-A / OP-080
日付: 2026-07-13
Pattern B: PASS（OLLAMA_HOST=host.docker.internal:11434 / OLLAMA_MODEL=gemma4:26b / health）
Negative 11434: 記録済 — aiome-ollama 併走時 dual-bind（OrbStack *:11434 + native 127.0.0.1:11434）。api 経由は native gemma4:26b 応答。stop 後 B 復帰 OK
Pattern A 復帰: PASS（OLLAMA_HOST=http://ollama:11434 / gemma4:e4b / health）
```

---

## 5. やらないこと

- Pattern B を本番 compose / GHCR イメージの既定にする  
- `.env` や Vault 秘密のコミット  
- LL-A 未完了を NT-6 公開の**必須ブロッカー**に格上げ（macOS quickstart は Pattern A で足りる）  
- LL-C（NT-3）を Agent 代理で PASS 扱いにする  

---

## 6. チェックリスト

```
[x] Loop2 修正: depends_on !reset null / force-recreate / disk_hygiene cargo clean
[x] Pattern A 実機 + gemma4:e4b pull
[x] LL-A Pattern B 実機（OP-080）— 2026-07-13
[ ] LL-B git 分割コミット（OP-081・ユーザー承認後）
[x] LL-C NT-3 目視（OP-002）— 2026-07-13 Human PASS
[ ] LL-D Linux extra_hosts（OP-082・任意）
```
