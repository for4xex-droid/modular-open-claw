---
description: リリース前の最終セキュリティ・衛生チェック。push前に必ず実行すること。
---

# /release-preflight — リリース前 偏執狂レベル最終チェック

// turbo-all

## ステップ 1: シークレットスキャン
```bash
gitleaks detect -v 2>&1 | tail -5
```
Exit code 0 であることを確認。1件でも検出されたらリリースを中止する。

## ステップ 2: 追跡ファイルの衛生チェック
以下のコマンドを実行し、出力がゼロであることを確認する。1件でも出たら即座に `git rm --cached` で除去。

```bash
echo "=== .DS_Store ===" && (git ls-files | grep -i DS_Store || echo "OK") && \
echo "=== node_modules ===" && (git ls-files | grep node_modules/ | head -3 || echo "OK") && \
echo "=== .env files ===" && (git ls-files | grep -E '\.env$' || echo "OK") && \
echo "=== memory files ===" && (git ls-files | grep -E '^memory/|MEMORY\.md|\.agent.*/memory/' || echo "OK") && \
echo "=== database files ===" && (git ls-files | grep -E '\.(sqlite|sqlite3|db)$' || echo "OK") && \
echo "=== build artifacts ===" && (git ls-files | grep -E '\.(dylib|so|dll|node|tgz)$' || echo "OK") && \
echo "=== strategy docs ===" && (git ls-files | grep -iE 'master_blueprint|vision_manifesto|pitch_deck|buyout|valuation' || echo "OK") && \
echo "=== backup files ===" && (git ls-files | grep -E '\.bak$|\.orig$|\.swp$' || echo "OK") && \
echo "=== states/logs ===" && (git ls-files | grep -E '^states/|^logs/' || echo "OK")
```

## ステップ 3: ローカルパスの漏洩チェック
```bash
git ls-files | grep -v node_modules | xargs grep -rl "/Users/" 2>/dev/null || echo "OK: No local paths found"
```

## ステップ 4: 誤ったURLの検出
```bash
grep -rn "google/antigravity" README.md README_en.md 2>/dev/null || echo "OK: No wrong URLs"
```

## ステップ 5: ビルド検証
```bash
cargo check --workspace 2>&1 | tail -3
```

## ステップ 6: リポジトリサイズ確認
```bash
echo "Tracked files:" && git ls-files | wc -l && echo "Estimated size:" && git ls-files -z | xargs -0 du -ch 2>/dev/null | tail -1
```
700ファイル以下、30MB以下であることを確認する。

## ステップ 7: GitHub About セクション確認
GitHubリポジトリの About セクションに以下が設定されているか確認する（手動チェック）:
- Description: 適切な1行説明
- Website: 公式サイトURL（あれば）
- Topics: `ai`, `autonomous-agents`, `rust`, `agent-os`, `self-improving-ai` 等

## ステップ 8: LICENSE 整合性
```bash
head -1 LICENSE && grep -o "Apache\|BUSL\|MIT\|GPL" LICENSE | head -1
```
README のバッジ表示と一致していることを確認する。

## 判定
全ステップが OK であればリリース可能。1件でも NG があればリリースを中止し、修正後に再実行する。
