---
name: design-catalog
description: 外部デザインシステムカタログの活用ガイドライン。ユーザーが「〇〇風のUI」を要求した場合にのみ参照する。
---

# Design Catalog Skill 🎨

外部の著名企業のデザインシステムを、AI エージェントが「インスピレーション」として活用するためのリファレンスカタログです。

## ⚠️ 使用条件（絶対遵守）

1. **Aiome Management Console には絶対に適用しない** — Aiome 自身の UI は `apps/management-console/DESIGN.md` と `tokens.css` が唯一の真実
2. ユーザーが明示的に「〇〇風」「〇〇のようなデザイン」と要求した場合にのみ参照する
3. 自動的にカタログを読み込むことはしない（コンテキストウィンドウの節約）
4. 商標・ブランドの「模倣」ではなく「インスピレーション元」として活用する
5. 出力物には「〇〇のデザインシステムを参考にしたスタイル」と明記を推奨

## 読み込みルール（トークン節約）

- **通常は `{name}.prompt-guide.md` を読む** — 色・タイポ・Agent Prompt Guide の要点のみ（2〜4KB）
- **詳細なレイアウト・コンポーネント・レスポンシブ設計が必要な場合のみ** `{name}.design.md`（full 版）をオンデマンドで読む
- full 版は変更せず参照専用。prompt-guide にない情報（セクション 4〜8 等）が必要なときだけ full を開く

## カタログ一覧

| 企業 | ファイル（通常） | Full 版（オンデマンド） | 特徴 |
|------|-----------------|------------------------|------|
| **Linear** | `.agent/design-catalog/linear.app.prompt-guide.md` | `linear.app.design.md` | ダークモードSaaS の模範。Inter Variable, 精密な余白, インディゴアクセント |
| **Stripe** | `.agent/design-catalog/stripe.prompt-guide.md` | `stripe.design.md` | 決済UIのゴールドスタンダード。紫グラデーション、weight-300 の繊細さ |
| **Vercel** | `.agent/design-catalog/vercel.prompt-guide.md` | `vercel.design.md` | モノクロ精密。Geist フォント、黒と白の極致 |
| **Notion** | `.agent/design-catalog/notion.prompt-guide.md` | `notion.design.md` | 温かいミニマリズム。セリフ見出し、柔らかい表面 |
| **Raycast** | `.agent/design-catalog/raycast.prompt-guide.md` | `raycast.design.md` | 生産性ランチャー。ダーククローム、鮮やかなグラデーション |
| **Supabase** | `.agent/design-catalog/supabase.prompt-guide.md` | `supabase.design.md` | OSSの開発者ツール。ダークエメラルド、コードファースト |
| **Apple** | `.agent/design-catalog/apple.prompt-guide.md` | `apple.design.md` | プレミアム余白。SF Pro、没入型画像レイアウト |
| **Spotify** | `.agent/design-catalog/spotify.prompt-guide.md` | `spotify.design.md` | ダークモード×鮮やかグリーン。太い書体、メディア中心 |
| **Airbnb** | `.agent/design-catalog/airbnb.prompt-guide.md` | `airbnb.design.md` | 温かいコーラル。写真重視、丸みのあるUI |
| **Framer** | `.agent/design-catalog/framer.prompt-guide.md` | `framer.design.md` | モーション主導。黒×青、デザインファースト |

## 使い方

```
ユーザー: 「Stripe風の決済ページを作って」

エージェントの手順:
1. `.agent/design-catalog/stripe.prompt-guide.md` を view_file で読み込む
2. 色パレット、タイポグラフィ要点、Example Prompts を参照
3. レイアウトグリッド・レスポンシブ・コンポーネント詳細が必要なら `stripe.design.md`（full）を追加読み込み
4. 新規プロジェクト用の CSS を生成（Aiome の tokens.css は使わない）
5. 出力に「Stripe のデザインシステムを参考にしたスタイル」と明記
```

## ライセンス

すべてのカタログファイル（prompt-guide および full 版）は [VoltAgent/awesome-design-md](https://github.com/VoltAgent/awesome-design-md) (MIT License) から取得。
各企業のデザイントークンは公開 CSS から抽出された値であり、商標やブランドガイドラインとは異なるものです。
