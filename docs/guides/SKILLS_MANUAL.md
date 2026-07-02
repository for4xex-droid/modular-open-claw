# 🧠 Aiome スキル活用マニュアル (Skills Manual)

本プロジェクトには、AIアシスタントが「あなたの意図を汲み取り、最高品質のコードを爆速で生成する」ための専用スキルセット（`.agent/skills`）がインストールされています。

このマニュアルでは、各スキルの役割と、開発者がどのようにAIと協力してバイブコーディングを加速させるべきかを解説します。

---

## 🛠 プロジェクト専用スキル一覧

### 1. 🌌 CloudDoc Vibe Master (`clouddoc-vibe-master.md`)
**【役割】構造の守護者・解説員**
- **何をするか**: 大規模な Aiome の構造をドキュメント経由で完全に把握し、実装の「ノリ」を設計図レベルで同期させます。
- **活用シーン**: 
  - 「複雑な機能を追加したいけど、どこから手をつければいい？」と聞く。
  - 実装前に「既存のよく似たコードの構造をMermaidで図解して」と頼む。

### 2. ⚙️ Backend Architecture Patterns ([`backend-patterns.md`](../generic-patterns/backend-patterns.md))
**【役割】堅牢なバックエンドの設計士**
- **何をするか**: Rust Enterprise Templateに基づいた「クリーンアーキテクチャ」と「依存性逆転の原則」を徹底させます。
- **活用シーン**: 
  - `libs/core` にビジネスロジックを追加する際の実装パターンの統一。
  - `infrastructure` 層でのデータベースや外部APIの抽象化実装。
- **注記**: 汎用リファレンスのため `docs/guides/generic-patterns/` に移管済み。Aiome 固有規約は [`architecture-rules.md`](../../.agent/skills/architecture-rules.md) / [`docs-ui-ux-golden-rules.md`](../../.agent/skills/docs-ui-ux-golden-rules.md) を参照。

### 3. 🎨 Frontend System Patterns ([`frontend-patterns.md`](../generic-patterns/frontend-patterns.md))
**【役割】プレミアムUIの魔術師**
- **何をするか**: バニラCSSや最新のWebデザイン（グラスモーフィズム、ダークモード）を用い、一目で「プレミアム」と感じさせるUIを生成します。
- **活用シーン**: 
  - 新しいダッシュボードウィジェットのデザイン。
  - アセットやアニメーションを含めたモダンなインターフェース構築。
- **注記**: 汎用リファレンスのため `docs/guides/generic-patterns/` に移管済み。Aiome 固有規約は [`architecture-rules.md`](../../.agent/skills/architecture-rules.md) / [`docs-ui-ux-golden-rules.md`](../../.agent/skills/docs-ui-ux-golden-rules.md) を参照。

### 4. 📏 Coding Standards & Iron Principles ([`coding-standards.md`](../generic-patterns/coding-standards.md))
**【役割】品質の門番（鉄の掟）**
- **何をするか**: 本番コードでの `unwrap()` 禁止（テストは可）、`Result`型強制、`tokio`による非同期処理など、Rustのエンタープライズ品質を担保します。
- **活用シーン**: 
  - コード生成時の自動リント・セルフチェック。
  - 実行効率と安全性を両立したコードの記述。
- **注記**: 汎用リファレンスのため `docs/guides/generic-patterns/` に移管済み。Aiome 固有規約は [`architecture-rules.md`](../../.agent/skills/architecture-rules.md) / [`docs-ui-ux-golden-rules.md`](../../.agent/skills/docs-ui-ux-golden-rules.md) を参照。

### 5. 🛡 Security Guardrails (`security-guardrails.md`)
**【役割】防御のスペシャリスト**
- **何をするか**: SQLインジェクション、XSS、不適切な認証認可など、ボイラープレートに潜みがちなセキュリティ脆弱性を防ぎます。
- **活用シーン**: 
  - APIエンドポイントの実装時のバリデーションチェック。
  - 環境変数や秘匿情報の取り扱いに関するガードレール。

### 6. 📚 実績由来スキル群（2026-07-03 追加）

過去の障害・教訓（memory/ Lessons）から抽出された、再発防止のためのスキルです。

| スキル | 発動場面 |
|---|---|
| `api-route-wiring-check.md` | 新 API エンドポイント / トレイトメソッド追加時（router.rs 配線漏れ防止） |
| `sqlx-migration-safety.md` | DB スキーマ変更時（SQLite/Postgres 二重同期・適用済み SQL 編集禁止） |
| `stripe-mock-centralization.md` | Commerce モックの追加・変更時（一元定義・偽成功禁止） |
| `playwright-e2e-stabilization.md` | E2E テストが Flaky・ハングするとき（JWT/Suspense/Tokio 枯渇の切り分け） |
| `i18n-test-sync.md` | UI 文字列の t() 化・翻訳 JSON 変更時（テスト期待値のキー同期） |
| `r3f-three-shader-patterns.md` | Biome UI（R3F/シェーダー）変更時（alpha/DPR/instanceColor/ダブルバッファ） |

---

## 🚀 バイブコーディングを加速させる「AIへの頼み方」

スキルを最大限に引き出すために、以下のステップで進めることを推奨します。

1. **スキルの「指名」**: 
   > 「`clouddoc-vibe-master` を使って、今のプロジェクトの課金モデルを解析してから、新しいプランの追加計画を立てて」
   
2. **構造の「可視化と合意」**: 
   > 「実装を始める前に、スキルのパターンに基づいて依存関係のクラス図を出して。それで合ってれば実装して」
   
3. **継続的な「シンクロ」**: 
   > 「実装が終わったら、`generate_docs.py` を実行してドキュメントを最新にし、管理画面で確認できるようにして」

---

## 💎 運用の心得
- **スキルは進化する**: 開発が進む中で新しい「自分たちだけのパターン」が見つかったら、`.Skills` ファイルを更新してAIに学習させてください。
- **「ノリ」と「規律」の両立**: 爆速で書く（Vibe）けれど、スキル（規律）は守る。これが Aiome における最強の開発スタイルです。

---
> AIはスキルの「目」を通してあなたのコードを見、あなたの「手」となって魔法を形にします。
