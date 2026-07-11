---
description: リリース前の最終セキュリティ・衛生チェック。push前に必ず実行すること。
---

# /release-preflight — リリース前 偏執狂レベル最終チェック

// turbo-all

## ステップ 0: ロールバック計画の存在確認
このリリースに失敗した場合の「引き返し方（Feature Flagの無効化、`git revert`、DBダウングレード等）」がイシューやPRに明記されているか確認すること。ない場合はリリースを停止。

## ステップ 0.5: Sovereign Verifier DAG チェック
```bash
python3 scripts/enforce_dag.py
```
Exit code 0 (DAG Topology is clean) であることを確認。違反があればリリースを中止する。

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
echo "=== strategy docs ===" && (git ls-files | grep -iE 'master_blueprint|vision_manifesto|pitch_deck|buyout|valuation' | grep -vi 'evaluation' || echo "OK") && \
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

## ステップ 5.5: リリースゲートテスト
`#[ignore]` でマークされたリリース前限定テスト（プレースホルダー検知等）を実行する。1件でも FAILED ならリリースを中止。
```bash
cargo test --workspace -- --ignored --skip sandbox --skip vendor 2>&1 | tail -10
```

## ステップ 6: リポジトリサイズ確認
```bash
echo "Tracked files:" && git ls-files | wc -l && echo "Estimated size:" && git ls-files -z | xargs -0 du -ch 2>/dev/null | tail -1
```
2500ファイル以下、75MB以下であることを確認する。

## ステップ 7: GitHub About セクション確認
GitHubリポジトリの About セクションに以下が設定されているか確認する（手動チェック）:
- Description: 適切な1行説明
- Website: 公式サイトURL（あれば）
- Topics: `ai-agents` `autonomous-agents` `self-hosted` `sovereign-ai` `mcp` `rust` `agent-economy` `local-first` `ai-os` `tauri`（MESSAGING §7 正本）

## ステップ 7.5: CHANGELOG [Unreleased] 肥大化チェック
```bash
awk '/^## \[Unreleased\]/{f=1;next} /^## \[/{f=0} f' CHANGELOG.md | wc -l
```
**200行を超えている場合**、リリース時に必ずバージョンセクション（`## [x.y.z] - YYYY-MM-DD`）へ切り出すこと。[Unreleased] の滞留はレビュー不能な変更履歴を生む。

## ステップ 8: LICENSE 整合性
```bash
head -1 LICENSE
grep -n "BUSL\|Business Source\|License-BUSL\|License-BSL" README.md | head -5
```
**PASS**: LICENSE 1行目が `Business Source License` で、README バッジが BUSL/BSL。  
**注意**: `grep -o Apache LICENSE | head -1` は使わない（Change License 行に誤ヒットする）。

## 判定
全ステップが OK であればリリース可能。1件でも NG があればリリースを中止し、修正後に再実行する。

## 🛑 共通の言い訳 (Anti-rationalization)

| エージェント(AI)のよくある言い訳 | 現実 (Reality) |
|----------------------|----------------|
| 「ステージングで動いたのでプリフライトはスキップします」 | 本番はデータもトラフィックも異なります。環境の差分は必ず悪さをします。 |
| 「時間がないのでDBバックアップは後回しで」 | データベースは一度壊れると復旧不可能です（特にマイグレーション時）。事前の準備こそが時間を救います。 |
| 「ロールバックは失敗を前提としているようで嫌です」 | シートベルトと同じです。使わないためのものであり、万一の際に命を救うためのものです。 |
