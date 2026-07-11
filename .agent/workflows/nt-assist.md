---
description: Human Public Beta（NT-1〜7）を1ステップずつ安全に進行。秘密は扱わない。
---

# /nt-assist — Human Public Beta アシスト

ランブック全文を人間に読ませず、**今やる1ステップだけ**を提示して進める。

**正本**: [`docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md`](../../docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md)  
**ゲート**: `python3 scripts/nt_gate.py`  
**進捗**: `states/nt_progress.json`（gitignore。雛形は `docs/guides/nt_progress.example.json`）

## 使用法

```text
/nt-assist
/nt-assist NT-1
/nt-assist 続けて
```

## 絶対禁止（違反したら即停止）

1. `sk_` / `whsec_` / パスワード / Vault 値をチャット・Issue・進捗 JSON に書かせない・受け取らない  
2. `docker-compose.production.yml` に `STRIPE_API_KEY=` を追加しない  
3. auth / commerce_webhook / key-proxy の「ついで修正」をしない  
4. 「公開してよい」なしに Release / タグを切らない  
5. Agent が本番ホストに秘密を入力するふりをしない（Human only）

## モード

| モード | Agent | Human |
|--------|-------|-------|
| `assist` | コピペ1塊・判定・`nt_gate`・進捗更新 | ホストで実行、「終わった」と返す |
| `human-only` | 待ちカード（画面操作の指示のみ） | Vault / Stripe Dashboard / 目視 / 決済 |
| `dry-run` | 読む・報告のみ | 確認 |

## 起動手順（毎回）

1. 進捗を読む:
   ```bash
   python3 scripts/nt_gate.py status
   ```
   `states/nt_progress.json` が無ければ雛形をコピーするよう案内:
   ```bash
   mkdir -p states && cp docs/guides/nt_progress.example.json states/nt_progress.json
   ```
2. ユーザーが NT を指定していなければ、進捗の `current` または未完了の最若番（NT-1→2→3→5→6、NT-4済、NT-7任意）を提案する。  
3. **今の1ステップだけ**を正本から要約して出す（825行を貼らない）。

## ステップ提示テンプレ（必ずこの形）

```markdown
### いま: NT-? / Step ?

**モード**: assist | human-only
**あなたがやること**（1つ）:
（ここにコマンド1塊 OR 画面操作1つ）

**終わったら**: 「終わった」またはゲート出力を貼る（秘密なし）
**Agent が次にやること**: 判定 → `nt_gate.py mark` → 次の1ステップ
```

## NT 別ルール

### NT-1（最重要）

順序: **0.1→0.2→0.3→0.4→0.5→A→B→C→D→Negative**

| Step | モード | Agent |
|------|--------|-------|
| 0.1–0.3 | assist | コピペ提示。**`restart` ではイメージは更新されない**と毎回注記 |
| 0.4 | assist | 実行後に `python3 scripts/nt_gate.py step0`（ホストで）。PASS まで次へ進まない |
| A / C(whsec) | **human-only** | 「まもる・整える→設定→Abyss Vault」でキー名だけ指示。値は受け取らない |
| B | assist | env 変数名のみ提示（値は Human） |
| C (restart) | assist | `restart` = 注入再読込。build ではない |
| D / Negative | human-only | Checkout・削除テストは Human。結果 Y/N のみ |

Step 0 完了条件: `nt_gate.py step0` exit 0。

### NT-2

assist。新規 clone + quickstart。事前にポート1420・container名衝突。Human がパスワード設定（Agent にパスワードを送らない）。

### NT-3 / NT-5

human-only。cockpit 必須。Agent は開き方だけ。PASS/FAIL を Human が宣言。

### NT-6

assist（開発機）。「NT-6 を実行しろ」相当。preflight は正本 / release-preflight。公開は **human-only 承認文**必須。  
CHANGELOG Unreleased 肥大は R5-2 を Agent に依頼する合図。

### NT-7

任意。human-only。連絡先を git に入れない。

## 進捗の更新

各ステップ判定後:

```bash
python3 scripts/nt_gate.py mark NT-1 0.4 PASS --note "distroless label true"
```

FAIL なら同じ Step の対処だけ再提示（ランブック Step 0.5）。先に進まない。

## ゲート（機械）

```bash
python3 scripts/nt_gate.py self-test   # 変更後・初回
python3 scripts/nt_gate.py hygiene
python3 scripts/nt_gate.py step0       # 本番ホスト想定。ローカルなら --skip-docker
python3 scripts/nt_gate.py step0 --skip-docker
```

## 完了時

該当 OPEN 項目の更新案を1行で出す（Agent が書いてよいか確認してから書く）:

- NT-1 → OP-057-R (1)  
- NT-3 → OP-002  
- NT-5 → OP-063  
- NT-6 → OP-070 / R5  
- NT-7 → OP-064（任意）
