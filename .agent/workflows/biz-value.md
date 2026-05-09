---
description: プロジェクト全体のビジネス価値を7軸で定量・定性評価し、次の一手を明確にする
---

# /biz-value - ビジネスバリュー・スキャナー 💎

プロジェクト全体を**7つの価値軸**から多角的に評価し、現在のポジションと改善ROIが最も高いアクションアイテムを特定するワークフローです。
投資判断やピボット判断の基礎データとして使用します。

## いつ使うか
- 定期的な現状把握（月1回推奨）
- 大きなフェーズを完了した直後
- 「今、何に集中すべきか？」を見失った時
- 外部パートナーとの協業を検討する前

---

## 実行手順

### Phase 1: データ収集 (Automated Metrics)

以下のコマンドでプロジェクトの定量データを自動収集します。

// turbo
```bash
echo "=== Crate Count ===" && ls -d libs/*/Cargo.toml apps/*/Cargo.toml 2>/dev/null | wc -l
echo "=== Total Lines of Rust ===" && find . -name "*.rs" -not -path "*/target/*" | xargs wc -l | tail -1
echo "=== Test Count ===" && grep -r "#\[test\]" --include="*.rs" -l | wc -l
echo "=== Feature Flags ===" && grep -r '^\[features\]' --include="Cargo.toml" -l | wc -l
echo "=== Git Commit Count ===" && git rev-list --count HEAD
echo "=== Contributors ===" && git shortlog -sn --all | wc -l
echo "=== Open TODOs/FIXMEs ===" && grep -rn "TODO\|FIXME\|HACK\|TEMPORARY" --include="*.rs" | wc -l
echo "=== ADR Count ===" && ls docs/decisions/*.md 2>/dev/null | wc -l
echo "=== Skill/MCP Count ===" && find . -name "*.skill" -o -name "mcp_*.rs" 2>/dev/null | wc -l
```

// turbo
```bash
bash scripts/deep-scan.sh 2>&1 | tail -20
```

### Phase 2: 7-Axis バリュー評価

収集したデータとコードベースの構造を基に、以下の7軸それぞれについて評価を行います。

---

#### V-1: 技術的成熟度 (Technical Maturity) 🔧
**測定ポイント:**
- フェーズ進捗率（CHANGELOG.md / planning_summary を参照）
- テストカバレッジ（テストファイル数 / 全ソースファイル数）
- deep-scan のエラー数（ゼロが理想）
- `TODO/FIXME` の残存数（技術的負債の指標）

**スコアリング基準:**
| スコア | 基準 |
|--------|------|
| 5/5 | フェーズ進捗 > 80%, deep-scan エラー = 0, TODO < 10 |
| 4/5 | フェーズ進捗 > 60%, deep-scan エラー = 0, TODO < 30 |
| 3/5 | フェーズ進捗 > 40%, deep-scan エラー ≤ 3 |
| 2/5 | フェーズ進捗 > 20%, deep-scan エラー ≤ 5 |
| 1/5 | それ以下 |

---

#### V-2: 防御可能性 (Moat / Defensibility) 🏰
**測定ポイント:**
- 独自アーキテクチャの深さ（他者が6ヶ月以内に再現できるか？）
- 特許可能な機構の数（AgentSoul, Karma System, Adaptive Immune System 等）
- OSS + ProD の二段階モデルの堅牢性
- ネットワーク効果の有無（マルチエージェント、Federation/P2P Hub 等）

**評価の視点:**
> 「もし明日、Google/OpenAI がこの領域に参入したら、Aiome の何が生き残るか？」

---

#### V-3: 市場適合性 (Product-Market Fit) 🎯
**測定ポイント:**
- Web 検索を使い、直近の競合プロダクト動向を調査する
  - 検索キーワード: `autonomous AI agent OS`, `AI agent framework 2026`, `virtual being platform`, `AI companion monetization`
- 競合との機能マトリクス比較表を作成する
- Aiome が持つ「競合にないもの」と「競合が持っているがAiomeにないもの」を明示する

**必須アクション:**
1. Web検索を実行し、少なくとも3つの競合プロダクトを特定する
2. 機能比較表を作成する（Aiome vs 競合A vs 競合B vs 競合C）

---

#### V-4: 収益化経路 (Monetization Readiness) 💰
**測定ポイント:**
- Commerce Engine の実装完了度
- 決済統合（Stripe等）の接続状態
- マネタイズモデルの多様性
  - Subscription? Token Economy? Marketplace? Gift Economy?
- 実際にお金が流れるパス（End-to-End）が存在するか？

**評価の視点:**
> 「明日 Public Beta を出した場合、Day 1 で課金できるか？」

---

#### V-5: 運用リスク (Operational Risk) ⚠️
**測定ポイント:**
- バスファクター（コントリビューター数、ドキュメント充実度）
- セキュリティ姿勢（deep-scan 結果、Bastion/Sentinel の完成度）
- 技術的負債の蓄積率（`TODO/FIXME` の増減トレンド）
- 依存クレートの脆弱性（`cargo audit` があれば実行）
- CI/CD パイプラインの成熟度

---

#### V-6: エコシステムの広がり (Ecosystem Breadth) 🌐
**測定ポイント:**
- MCP ツール/サーバーの統合数
- Skill モジュールの数と多様性
- 外部 API 連携の数（Discord, Telegram, LLM プロバイダー等）
- プラグインアーキテクチャの拡張性（PluginRegistry の成熟度）
- フロントエンド（Tauri等）の完成度

---

#### V-7: ナラティブ価値 (Narrative Power) 📖
**測定ポイント:**
- 「Aiome とは何か？」を30秒で説明できるか
- 「なぜ今これが必要か？」への説得力のある回答があるか
- README / ビジョンドキュメントの品質
- デモ可能な動くプロトタイプの存在
- 「刺さる一言」（タグライン）の有無

**評価の視点:**
> 「エレベーターピッチで相手を "もっと聞きたい" と思わせられるか？」

---

### Phase 3: 総合レポート生成

以下の2つの形式で結果をアーティファクトとして出力します。

#### 1. Markdown サマリー (Provenance 用)
標準的な Markdown 形式で概要、スコア、主要アクションを記述します。

#### 2. インタラクティブ HTML レポート (閲覧用)
`biz_value_report.html` という名前で、以下の要素を含む HTML アーティファクトを生成してください。
- **レーダーチャート (SVG)**: 7軸のスコアを視覚化したレーダーチャート。
- **インタラクティブ・マトリクス**: 競合比較をタブやホバーエフェクトで詳細表示。
- **デザイントークン準拠**: `tokens.css` で定義されたカラー変数（`var(--accent-cyan)` 等）を使用し、Aiome のデザインシステムと一貫性を持たせること。
- **アニメーション**: レポート表示時にフェードインやバーの伸長アニメーション（CSS）を含めること。
- **Interactive Feedback (JS Bridge)**: 「ROI最大のアクション」の横に、修正作業をエージェントに自動実行させるためのボタンを配置してください。以下のHTML属性を使用して、Management Console のチャットへ直接プロンプトを送信できます：
  `<button class="aiome-feedback-btn" data-aiome-feedback="アクション [X] の詳細な実装計画を作成してください。" data-autosend="true">Auto-Plan Action X</button>`

```markdown
# 💎 Aiome ビジネスバリュー・レポート

> **評価日時**: YYYY-MM-DD HH:MM
> **対象**: プロジェクト全体 (commit: [hash])

## 📊 バリューレーダー

| # | 軸 | スコア | 前回比 | キーインサイト |
|---|:---|:---:|:---:|:---|
| V-1 | 技術的成熟度 | ⭐/5 | — | [一行サマリー] |
| V-2 | 防御可能性 | ⭐/5 | — | [一行サマリー] |
| V-3 | 市場適合性 | ⭐/5 | — | [一行サマリー] |
| V-4 | 収益化経路 | ⭐/5 | — | [一行サマリー] |
| V-5 | 運用リスク | ⭐/5 | — | [一行サマリー] |
| V-6 | エコシステム | ⭐/5 | — | [一行サマリー] |
| V-7 | ナラティブ | ⭐/5 | — | [一行サマリー] |

**総合バリュースコア**: ⭐ X.X / 5.0

## 🏆 最大の強み (Top Moats)
1. ...

## 🚨 最大のリスク (Critical Gaps)
1. ...

## 🎯 ROI最大のアクション (Next Best Actions)
| 優先度 | アクション | 期待効果 | 工数見積 |
|--------|-----------|---------|---------|
| 🔴 P0 | ... | V-X を Y→Z に改善 | ~N日 |
| 🟡 P1 | ... | ... | ... |

## 📈 競合比較マトリクス
| 機能 | Aiome | 競合A | 競合B | 競合C |
|------|:---:|:---:|:---:|:---:|
| ... | ✅/❌ | ... | ... | ... |

## 💡 戦略メモ
[自由記述: 気づき、仮説、次フェーズへの示唆]
```

---

## 🧠 エージェントへの絶対指示 (Agent Directive)

1. **忖度禁止**: スコアは甘くつけてはいけない。「投資家が見たら何と言うか？」の視点で容赦なく評価すること。
2. **数字で語る**: 可能な限り定量データ（コード行数、テスト数、コミット数等）を添えること。感覚的な評価のみは不可。
3. **競合調査は必須**: V-3 では必ず Web 検索を行い、最新の競合動向を反映すること。「検索しませんでした」は許容しない。
4. **アクションアイテムは具体的に**: 「セキュリティを強化する」のような曖昧な提案は不可。「Phase 8.2 の JWT/RBAC 統合を完了し、SPOF を解消する（推定工数: 3日）」のように具体化すること。
5. **前回比**: 過去のレポートが存在する場合、スコアの変動を `↑` `↓` `→` で表示し、改善/悪化の傾向を可視化すること。
