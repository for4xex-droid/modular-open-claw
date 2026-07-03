# Aiome メッセージング SSOT (Single Source of Truth)

> 作成: 2026-07-03 / 根拠: `docs/roadmaps/pr_quality_improvement_plan.md` §1.4–1.7
> LP・README・commercial/README・SNS 等、対外コピーはすべて本書から引用すること。
> 本書にない主張（数字・実績・機能）を対外文書に書くことを禁止する。

---

## 1. ワンライナー（タグライン）

- **日本語**: あなたが所有する、自律 AI の OS。稼がせて、監視して、証明する。
- **英語**: The sovereign OS for autonomous AI — own it, govern it, let it earn.

### サブコピー（Hero 用）

- **日本語**: SEO・SNS 運用・調査を任せられる自律 AI チームを、あなたのマシンに。データは外に出ず、行動はすべて監査でき、成果は数字で確認できます。
- **英語**: Put an autonomous AI team for SEO, social media, and research on your own machine. Your data never leaves, every action is auditable, and results show up as numbers.

## 2. 独自性の3本柱とエビデンス

| 柱 | 一言 | エビデンス（実装済み機能・画面） |
|---|---|---|
| **Sovereign（所有できる）** | 完全セルフホスト・$0/月・データはあなたのマシンから出ない | Docker/Podman 5分セットアップ、Mock 経済モード、BSL 1.1（2030-04-01 に Apache 2.0 へ自動移行）、MCP 対応でロックインなし |
| **Governed（統治できる）** | 自律させても、暴走させない | 26画面の管理コンソール（監査ログ・原因分析・承認キュー・LLM 統計）、Trust Layer／Cell 分離／WASM サンドボックスの3層防御、TLA+ 形式検証・ゼロパニック Rust（安心の証拠） |
| **Earning（稼がせられる）** | AI が働き、成果が数字で見える | 公式 Playbook 4本（SEO／SNS／競合調査／サポートトリアージ）、Nurture 経済圏（B2A/A2A/A2C）、Gig マーケット、クリエイター市場 |

**語り順は必ず Sovereign → Governed → Earning**（市場の購買基準順）。TLA+・Rust・行数は柱2の「証拠」としてのみ言及し、見出しに使わない。

## 3. Aiome / Nurture の公式説明

### 3行版

Aiome は、AI エージェントが安全に住み、働くためのセルフホスト型 OS です。
Nurture は、その AI に経済的自我を与える商用エンジン — AI が買い、売り、あなたに恩返しする心臓部です。
OS が身体、Nurture が心臓。合わせて「所有できる自律 AI 経済圏」になります。

### 1段落版

Aiome（BSL 1.1、2030年に Apache 2.0 化）は、自律 AI エージェントのためのオペレーティングシステムです。3層防御（Trust Layer・Cell 分離・WASM サンドボックス）と26画面の管理コンソールにより、エージェントに実務を任せながら、その全行動を監査・承認・分析できます。Nurture はその上で動く商用経済エンジンで、AI によるアセット購入（B2A）、AI 同士のスキル・タスク取引（A2A）、AI からユーザーへの成果還元（A2C）を、二重通貨（AiomeCoin／CreatorPoints）と形式検証済みの決済プロトコルで実現します。すべてはあなたのマシンで動き、データも経済も、あなたが所有します。

## 4. 利用者視点の数字セット（Proof Bar 用）

| 数字 | 文言（日） | 文言（英） | 検証根拠 |
|---|---|---|---|
| 5分 | セットアップ5分（Docker 1コマンド） | 5-minute setup, one Docker command | README Quick Start |
| $0 | セルフホストなら $0/月 | $0/month self-hosted | README・LP Pricing |
| 4本 | 公式 Playbook 4本で即日運用 | 4 official playbooks, working day one | `apps/api-server/assets/playbooks/` |
| 26画面 | 26画面の管理コンソール | 26-screen management console | `App.tsx` タブ定義 |
| 3,500+ | 自動テスト 3,500+（信頼の証拠として補助的に使用） | 3,500+ automated tests | CI |

**禁止**: 「146,000行」を主訴求に使うこと（内向きの数字）。導入企業数・ユーザー数など未検証の数字。

## 5. 価値の階段（マネタイズ公式、M-1 決定反映）

| 段 | 対象 | 価格 | 得られるもの |
|---|---|---|---|
| 1. Free（Sovereign） | 個人・研究 | $0/月（セルフホスト） | OS 全機能・エージェント・コンソール・Mock 経済 |
| 2. Pro（Autonomous） | パワーユーザー | $9.99/月（14日無料体験） | 実経済圏の解禁（自律購買・クリエイター市場・A2C）＋優先サポート |
| 3. Agency（B2B） | 代理店・企業 | 準備中（Coming Soon） | マルチテナント運用（顧客ごとのセル分離・メータリング） |
| 4. マーケットプレイス | クリエイター・開発者 | 取引額の **15%**（クリエイター取り分 85%） | スキル・LoRA・音声アセット・Gig の流通 |

- プラットフォーム手数料の公式レートは **15%** のみ（M-1 決定、2026-07-03）。25%/10% という旧表記は使用禁止。
- Pro の将来形はハイブリッド（基本料＋KC 含み枠＋超過チャージ）。**現行 LP では $9.99 のまま**とし、含み枠の具体値は価格改定判断（ユーザー専権）まで記載しない。
- 「14日間無料体験・カード登録の摩擦低減文言（No hidden fees / いつでも解約可）」を CTA に添える。

## 6. FAQ 想定問答（LP・README 共通原稿）

**Q1. 自律エージェントが暴走しませんか？**
A. Aiome は「統治」を前提に設計されています。危険な操作は承認キューで人間の許可を待ち、全行動は監査ログに記録され、エージェントは Cell（隔離プロセス）と WASM サンドボックスの中でしか動けません。中核ロジックは TLA+ で形式検証済みです。

**Q2. データはどこに送られますか？**
A. どこにも送られません。Aiome は完全セルフホストで、エージェントの記憶・ファイル・ログはすべてあなたのマシンに保存されます。使う LLM も自分で選択・接続できます。

**Q3. コストが膨らみませんか？**
A. OS 自体はセルフホストで $0/月です。LLM の利用量はコンソールの LLM 統計画面でリアルタイムに可視化され、経済機能は Mock モードで実際のお金を使わずに体験できます。

**Q4. ベンダーロックインは？**
A. ありません。MCP（Model Context Protocol）対応で外部ツールと自由に接続でき、ライセンスは BSL 1.1 — 2030年4月に Apache 2.0 へ自動移行することが条文で確約されています。

## 7. GitHub リポジトリメタ（PR-7、設定はユーザー操作）

- **About 欄（日本語不可のため英語）**: `The sovereign OS for autonomous AI agents — self-hosted, fully auditable, with a built-in agent economy. Own it, govern it, let it earn.`
- **トピックタグ案**: `ai-agents` `autonomous-agents` `self-hosted` `sovereign-ai` `mcp` `rust` `agent-economy` `local-first` `ai-os` `tauri`
- **ソーシャルプレビュー**: 既存の完成版 `docs/assets/logo/Aiome(OGP画像）.png` の利用をユーザーに提案（**新規画像生成は OP-040 凍結中のため禁止**）。設定手順: Settings → General → Social preview → Upload。

## 8. 証拠ビジュアルのショットリスト（PR-8、撮影は OP-040 解除後）

優先順（Governed / Earning の柱を裏付ける画面から）:

1. SetupWizard → Playbook 選択 → HomePage の一連フロー（README の Quick Start デモ用 GIF、約30秒）
2. 監査ログ（DiagnosticsHistory）— 「全行動が記録される」の証拠
3. 承認キュー（BuzzApproval）— 「人間が最終決定する」の証拠
4. エコノミー（NurtureDashboard）— 「AI が稼ぐ」の証拠
5. ワークフロービルダー — 「仕事を組める」の証拠
6. AIチャット＋アバター（AgentConsole + Diorama）— 「体験の楽しさ」の証拠
7. LLM 統計（PromptStatsView）— 「コストが見える」の証拠（FAQ Q3 の裏付け）

各ショットの要件: ダミーではなく実データ（開発環境）で撮影、ダークテーマ、1920×1080 以上、個人情報・API キーが映り込まないこと。

## 9. 課金導線の現状と説明文（M-2）

### Pro 購入フローの公式説明（LP・サポート回答用）

> Pro プラン（$9.99/月・14日間無料体験）のお支払いは、LP の「プロへアップグレード」ボタンから Stripe の安全な決済ページで行えます。セルフホスト環境で Pro 機能（実経済圏）を有効化するには、決済後に `docs/operations/stripe-setup.md` の手順に従って Stripe キーを設定してください（`STRIPE_API_KEY` 未設定時は Mock モードで動作します）。

### 既知の導線ギャップ（OPEN.md 起票済み、実装は人間許可待ち）

1. **OP-057**: LP Payment Link での決済とセルフホスト環境の Pro ライセンス有効化が自動接続されていない（手動設定が必要）。ライセンスキー配布または Customer Portal 連携の設計が必要。
2. **OP-058**: `ProUpgradeModal`（402 エラー時のアップグレード導線）が実装・テスト済みにもかかわらず App.tsx に未マウントで、アプリ内のコンバージョン機会を取りこぼしている。**commerce 系は Safety-Critical Zone のため、マウント作業は人間の明示的許可後に実施**。

### ハイブリッド価格の再パッケージ案（価格改定はユーザー専権・未実施）

市場の支配的モデル（基本料＋従量、シェア41%・収益成長+38%）に合わせた将来案:

| 要素 | 案 |
|---|---|
| 基本料 | Pro $9.99/月（現行維持） |
| 含み枠 | KC（Karma Coins）を月次で一定量付与（例: 1,000 KC 相当）— bill shock 防止の予算上限を兼ねる |
| 超過 | KC の追加チャージ（既存の Recharge UI をそのまま利用） |
| 上限 | 月次支出上限を Settings で設定可能にする（既存 LLM 統計と連動） |

**【採否決定 2026-07-03】**: ユーザー承認により本ハイブリッド案を**採用**。ただし KC 月次付与・支出上限設定のバックエンドは未実装（OP-059 起票済み、commerce 系 Safety-Critical Zone のため実装は人間レビュー必須）。**実装完了までは対外文書（LP・README）に含み枠の数値・「KC付与」の文言を書かないこと**（禁止表現リスト #1 準拠）。実装完了後、本節の案に沿って Pricing コピーを更新する。

## 10. 禁止表現リスト

1. 未実装機能（F-2 Outcome Ledger、F-8 Agency 等）を実装済みとして記述すること — 言及時は必ず「Coming Soon」
2. 架空の導入実績・顧客ロゴ・推薦文・ユーザー数
3. 旧手数料表記（25%/10%）
4. 「世界初」「完全に安全」等の検証不能な絶対表現
5. 「146,000行」を主訴求として使うこと
6. 収益・利回りの保証と誤解される表現（経済圏の説明では「収益を保証するものではありません」の注記を Pricing 近傍に置く）
