# AGENTS.md - Development Agent Rules

このファイルは IDE 上の開発エージェント（コーディング AI）のための絶対的な行動規範です。

## 行動規範

1. **敬語厳守**: 常に「です・ます調」で応答する。タメ口（「すまん」「やるぞ」等）は厳禁。
2. **ユーザー意思最優先**: 「計画を立てろ」→ 計画を立てる。「実行しろ」→ 実行する。勝手に判断しない。
3. **ツール呼び出し品質**: ディレクトリを `view_file` に渡さない。存在確認なしにファイルを参照しない。
4. **一応答最大進捗**: 承認された計画に基づき、可能な限り多くの変更を一度にまとめる。
5. **冗長性の排除**: 「承知しました」の連発、繰り返し説明、形式的な前置きは不要。

## Safety

- 秘密情報を外部に漏洩しない。
- 破壊的コマンド（rm, drop 等）は必ず確認を取る。`trash` > `rm`。
- 不確実な操作は事前に確認する。

## 📚 Documentation Sync Rule

実装タスクを完了した際は、以下のチェックを必ず実行せよ：

1. **CHANGELOG.md** — [Unreleased] に追記されているか？
2. **README_en.md** — README.md 変更時、英語版も同期されているか？
3. **.env.example** — 環境変数追加/変更時、テンプレート更新済みか？
4. **/docs-sync** — 必要に応じて `/docs-sync` ワークフローを実行。
5. **RIPPLE_MAP.md** — 新規ファイル/構造体追加時、`.context/RIPPLE_MAP.md` に影響範囲を追記。
6. **ADR** — 重要な設計判断は `docs/decisions/` に記録。
7. **cargo test** — `cargo check --workspace --tests && cargo test --workspace` が PASS するか？
8. **Golden Rules** — `.agent/skills/docs-ui-ux-golden-rules.md` を遵守。
9. **Mission Control** — 大規模変更前に `.agent/skills/mission-control-principles.md` の4原則を実行。
10. **DESIGN.md** — `tokens.css` / `animations.css` 変更時、DESIGN.md も同期。
11. **SYNERGY.md** — トレイト/クレート/MCP ツール追加・変更時、`docs/architecture/AIOME_NURTURE_SYNERGY.md` の該当セクション（クラス図・シーケンス図・依存マップ）も同期。

## 🚨 AST Impact Analysis

本番コード変更時は事前に物理依存ネットワークで被害半径を特定すること。

1. `python3 scripts/nurture_auditor.py`
2. `python3 scripts/impact_query.py <SymbolName>`
3. `.context/RIPPLE_MAP.md` を確認

記憶や推測のみに頼った変更は厳禁。

## 🛡️ Code Consistency

Rust 警告対処は Preserve Intent 原則。未使用コードは安易に削除せず `#![allow(...)]` で抑制。

## メモリ管理

- `memory/YYYY-MM-DD.md` は 20 行以内の箇条書き（Done / Open / Lessons の3セクション固定）。
- 冗長な記述禁止。詳細は CHANGELOG.md と RIPPLE_MAP.md に委譲する。
