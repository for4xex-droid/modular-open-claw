# Aiome + Nurture UI 全体改善計画（UI Overhaul Plan v6 — 実地レビュー反映・情報設計確定版）

- **作成日**: 2026-07-05（v2: /perfect-plan 5ゲート検証。v3: 体感品質・品質基盤監査で Phase U5 新設。v4: 課金ジャーニー検証+計画横断監査。v5: 全未決定分岐の解消 + 実装コントラクト（Appendix A）追加。**v6（2026-07-07）: ユーザー実地レビューで「まだ分かりにくい」と判定。§12 に即修正バグ（U0-B）と Phase U6（情報設計の確定・メニュー順序）を新設**）
- **ステータス**: **実装中**（2026-07-05 R1 完了 = U0/U1/U3/U4/U5 実装済み（OP-066: Jest 392 PASS / hex 0 / deep-scan 0。残: U2-4 variant 統合・U1-3 ギフト/ギルド）。**ただし U2 は「ラベル置換のみ完了・構造再編（5グループ化）は未完」に v6 で格下げ（§12.2）**。U0-B/U6 は未着手）
- **根拠**: サブエージェント3体による実コード棚卸し（UI構造 / 課金導線 / Generative UI 資産）+ 第4サブエージェントによるパス実在検証（Gate 1 相当）済み。全ファイルパス・行番号は 2026-07-05 時点で実在確認済み。`GET /api/v1/commerce/subscription/{agent_id}` は `commerce.rs` L432–445 に実在（U1-1 はフロント追加のみで成立）。
- **目的**: 以下4課題を解消し「AI に詳しくなくても使いこなせる UI」へ転換する。
  1. **課金メリットが伝わらない** — Pro の価値が 402 エラー発生まで不可視
  2. **機能が使いづらい** — 二重ナビ・専門用語・到達不能画面
  3. **ダサい** — トークン乖離・HEX直書き・インラインstyle 乱立
  4. **（v3 追加）信頼感が薄い** — fetch 失敗のサイレント無視・空白ローディング・フィードバック不在・キーボード/モバイル非対応
- **関連**: OP-057-R（決済→Pro 自動有効化、凍結中）、`biome_lenia_overhaul_plan.md` §13（Biome ブラッシュアップ）と同一バッチ推奨。

---

## 1. 現状診断（証拠付き）

### 1.1 情報設計の問題

| # | 症状 | 証拠 |
|---|---|---|
| S-1 | サイドバーは `viewMode === 'advanced'` のみ表示。beginner/intermediate は HomePage 4タブ（home/shop/world/settings）が全て | `App.tsx` L417 |
| S-2 | SetupWizard は `expert` を送信するがフロント型は `advanced` のみ → Expert 選択者にサイドバーが出ない | `SetupWizard.tsx` L392 / `types.ts` L203 |
| S-3 | `buzz-approval` タブが `isVisible()` の3配列いずれにも未登録 → **常に到達不能** | `App.tsx` L305–312, L499–505 |
| S-4 | チャット UI が2系統併存（HomePage の StoryFlow / サイドバーの AgentConsole） | `home/StoryFlow.tsx` / `AgentConsole.tsx`（639行） |
| S-5 | 用語が専門的（Biotope, Commune, Karma, Cortex, SoT）で機能実体が直感できない | `i18n/ja.json` トップレベルキー 50個 |
| S-6 | HomePage「world」タブのサブ項目が7個（p2p/dashboard/biome/map/trace/chronicle/demo）で選択基準がない | `home/HomePage.tsx` |

### 1.2 課金価値の不可視性

| # | 症状 | 証拠 |
|---|---|---|
| M-1 | **Pro 状態を表示する UI が存在しない**。サブスク状態を取得するフックがない（`useAgentIdentity` は eKYC のみ） | `hooks/useAgentIdentity.ts` |
| M-2 | アップグレード導線は 402 → `stripe-402-payment-required` イベント → ProUpgradeModal の**受動一本** | `lib/auth.ts` L54–57 / `ProUpgradeModal.tsx` L22–30 |
| M-3 | TTS SSE は `authenticatedFetch` 不使用 → Pro ゲートでも 402 イベント不発（モーダルが開かない） | `hooks/useTtsSse.ts` L57–60 |
| M-4 | ProUpgradeModal の機能リスト（英語固定・i18n 未対応）が実ゲート10箇所と不一致 | `ProUpgradeModal.tsx` / `routes/{buzz,commerce,treasure,lora_market,gift,syndicate,voice}.rs` |
| M-5 | KC 課金（Recharge / Buy Points）と Pro サブスクが同じ Checkout に流れ、UI 上区別されない | `VoiceStore.tsx` L186–189 / `NurtureDashboard.tsx` L143–149 |
| M-6 | Mock モードは未知 agent を `Active` 扱い → 開発・デモで Pro 体験（ロック→解放）が再現不能 | `libs/aiome-commerce/src/mock.rs` L248–257（`subscription_override` は存在するが env 経由の Free 固定は未実装） |
| M-7 | NurtureDashboard の「View Store」がスタブ URL | `NurtureDashboard.tsx` L153 |
| M-8 | **（v4）ProUpgradeModal に `agentId` が渡されていない** → Checkout Session 作成 API（`agent_id` 必須）が失敗し得る = **アプリ内唯一のアップグレード導線が機能しない疑い**（VoiceStore は `useAgentIdentity()` で正しく渡している） | `App.tsx` L782（`<ProUpgradeModal priceId={STRIPE_PRICE_ID} />`）/ `useCheckoutSession.ts` L25–32 |
| M-9 | **（v4）決済完了後の着地 UI が存在しない**: `success_url = window.location.href`（同一ページに黙って戻る）。AiaaOnboardingWizard は `/checkout/success` を指定するが React ルート未定義（404 相当） | `useCheckoutSession.ts` L25–32 / `AiaaOnboardingWizard.tsx` / `App.tsx`（Route なし） |
| M-10 | **（v4）「支払い→Pro」の閉ループが本番相当で存在しない**: Pro 判定は `get_subscription_status`（Stripe API 照会）のみ。LP Payment Link は agent 未紐付け・`register_license` は v1.0 封印・ライセンスキー入力 UI/API なし。閉ループ完成は OP-057-R（凍結・Safety-Critical）待ち | `auth.rs` L158–172 / `stripe/mod.rs` L644–656, L877–912 / [課金ジャーニー監査](49a390c4-2aff-44ae-b352-a0a9d0a97912) |

### 1.3 見た目（デザイン負債）

| # | 症状 | 証拠 |
|---|---|---|
| D-1 | Biome 系に HEX 直書き多数（`#00f0ff`, `#66ff99` 等、U-002 違反） | `lib/biome/BiomeGame.tsx` 他 / 既知 OP-029 |
| D-2 | `NurtureDashboard` 等がインライン style 主体でトークン粒度が粗い | `NurtureDashboard.tsx` L203–209 |
| D-3 | `DESIGN.md` と `tokens.css` の乖離（`--accent-rose` の値、font-main 定義） | `DESIGN.md` vs `src/styles/tokens.css` |
| D-4 | ProUpgradeModal が英語ハードコード（i18n 未対応） | `ProUpgradeModal.tsx` |

### 1.4 体感品質（フィードバック・状態表示）— v3 追加

| # | 症状 | 証拠 |
|---|---|---|
| Q-1 | **fetch 失敗がサイレント**（`console.error` のみで UI 無反応）: 失敗と「データゼロ」が区別不能 | `GraphView.tsx` L47–49, L147–148 / `DiagnosticsHistory.tsx` L65–66 / `SettingsPage.tsx` L74–77 / `ExpressionPipeline.tsx` L54–56 / `home/CharacterPanel.tsx` L42, L49, L64 |
| Q-2 | ローディング表示が6パターン混在（Skeleton / Loader2 / pulse テキスト / 自前回転 / `...` / なし）。`Suspense fallback={<div />}` の完全空白も | `App.tsx` L350–356 / `Timeline.tsx` L113–116 / `SkillVault.tsx` L123–126 / `NurtureDashboard.tsx` L208 |
| Q-3 | 空状態に「次の一手」CTA がない画面が多数（Timeline / SkillVault / DiagnosticsHistory / TreasureBox / PromptStatsView）。DiagnosticsHistory は空状態 UI 自体なし | `Timeline.tsx` L123–127 / `DiagnosticsHistory.tsx` L81–151 / `TreasureBox.tsx` L117–120 |
| Q-4 | SettingsPage の保存成功フィードバックなし（`saving` スピナーのみ）。Toast 採用は7画面に留まり、`alert()`/`window.confirm()` 直書きが残存 | `SettingsPage.tsx` L89–96 / `VaultSecretsManager.tsx` L98, L105, L115 / `BanDashboard.tsx` L96 |
| Q-5 | フォーム検証の統一度が低い（SetupWizard は高品質だが SettingsPage はフィールド検証ゼロ） | `SettingsPage.tsx` L577–596 |

**既存の再利用可能資産**（U5 はこれらの横展開であり新規発明ではない）: `common/Toast.tsx`（ToastProvider、ただし同時1件制限）/ `common/ConfirmModal.tsx` / `common/AiomeSkeleton.tsx` / `StatusPage.tsx` L78–93 のエラー+Retry パターン / `TreasureBox.tsx` L63–66 の loading+empty 分岐 / `ExpressionPipeline` の CTA 付き空状態。

### 1.5 品質基盤（a11y・レスポンシブ・性能）— v3 追加

| # | 症状 | 証拠 |
|---|---|---|
| F-1 | **a11y 基盤欠如**: `aria-*` 全体で21件・`tabIndex` 0件。モーダル3種すべてフォーカストラップ/`aria-modal` なし（Escape 対応は ProUpgradeModal のみ）。サイドバー `NavItem` が `<div onClick>`（キーボード到達不能） | `App.tsx` L789–798 / `ConfirmModal.tsx` / `AvatarViewerModal.tsx` |
| F-2 | **レスポンシブ未対応**: `App.css`/`tokens.css` に `@media` 0件。サイドバー常時280px、HomePage 左ペイン320px固定、world サブタブ `nowrap`。ブレークポイント定義なし（散在値 768/900/1024px） | `App.css` L179–196 / `home/HomePage.tsx` L218, L86–91 |
| F-3 | `prefers-reduced-motion` 対応が Fluid 背景のみ。Framer Motion 40+ ファイルに配慮なし | `fluid/useFluidConfig.ts` L13–21 |
| F-4 | ダークテーマ固定（`tokens.css` は Dark Palette 前提、切替 UI・`prefers-color-scheme` なし） | `styles/tokens.css` L1–14 |
| F-5 | `DioramaView`（three.js/@react-three/fiber）が静的 import でメインバンドル常駐（他24画面は lazy 済み）。Timeline 等の長リストは仮想化なし。`React.memo` は2件のみ | `App.tsx` L68 / `Timeline.tsx` L141– |
| F-6 | `animations.css` に未使用定義5種（`ani-float` / `h-hover` / `h-holographic` / `avatar-breath` / `neural-line-animate`）。hover 演出がCSS とインライン `onMouseEnter` で混在 | `styles/animations.css` / `home/HomePage.tsx` L267–268 |

### 1.6 Generative UI（A2UI）の未活用

- **A2UI 基盤は Phase 0 完了済み**: Rust validator/catalog（16コンポーネント）→ SSE `event: a2ui` → `A2uiRenderer.tsx`。feature flag `a2ui_generative_ui`。
- 未活用ポイント:
  - `updateComponents` / `deleteSurface` はプレースホルダのみ（動的更新不可）
  - `voiceStore` / `loraMarket` はスラッシュコマンド専用（validator 未登録 → LLM 生成経路で使えない）
  - **Pro 訴求・オンボーディング・ナビゲーション補助に A2UI が一切使われていない**

---

## 2. 設計原則（本計画の北極星）

1. **「操作する管理画面」から「対話で頼めるコンシェルジュ」へ** — 迷ったらチャットに聞けば A2UI がその場に UI を出す。サイドバーは「地図」、チャットは「ガイド」。
2. **Pro は「壁」ではなく「ショーウィンドウ」** — ロック機能は隠さず、グレーアウト+価値説明+1クリック導線で常時見せる。
3. **語彙は日常語ファースト** — 一次ラベルは日常語（例:「AIとはなす」）、専門語（Cortex 等）は括弧・ツールチップに降格。
4. **トークンは法律**（U-001/U-002 遵守）— 生値ゼロ、`tokens.css` が唯一の真実。
5. **モードは3段でなく2段** — beginner/intermediate/advanced の3段+expert 不整合を「Simple / Pro Cockpit」の2段に整理統合。
6. **（v3 追加）状態は必ず語る** — 全画面が「読込中・空・失敗」の3状態を明示的に表現する。サイレント失敗（console.error のみ）と空白フォールバックは品質バグとして扱う。空状態には必ず「次の一手」CTA を置く。

---

## 3. Phase 構成（優先順・依存順）

### Phase U0: 即効バグ修正 + 計測基盤（工数: 小 / 前提なし）

「計画の前に直すべき明白なバグ」。全て既存ファイルの小修正。

| ID | 内容 | 対象 | 検証 |
|---|---|---|---|
| U0-1 | 【決定】`buzz-approval` を `isVisible()` の `advanced` 配列に**登録**する（削除しない。Pro 機能の可視化 = U1 方針と整合） | `App.tsx` L305–312 | タブが advanced で表示される |
| U0-2 | 【決定】**両方やる（二重防御）**: (1) SetupWizard の送信値を `advanced` に統一 (2) `setup.rs` の `allowed_modes` で `expert` を受理し `advanced` として保存（既存 DB に `expert` が残っているため） | `SetupWizard.tsx` L392 / `routes/setup.rs` L87–92 | Expert 選択→サイドバー表示。既存 `expert` 保存値でも表示 |
| U0-3 | `useTtsSse` に 402 検知を追加（`stripe-402-payment-required` dispatch） | `hooks/useTtsSse.ts` L57–60 | Pro 未加入で TTS→モーダル表示 |
| U0-4 | `buzz-approval` 選択時の `<h2>` タイトル追加 | `App.tsx` L694–718 | ヘッダ表示 |
| U0-5 | HomePage バイオームカードの日本語ハードコード i18n 化 | `home/HomePage.tsx` L244–287 | en 切替で英語表示 |
| U0-6 | **（v4）ProUpgradeModal への `agentId` 配線**: `useAgentIdentity().agentId` を渡す（M-8。アプリ内唯一のアップグレード導線が Checkout 作成で失敗し得る重大バグ。VoiceStore と同パターンの1行修正） | `App.tsx` L782 | Free 状態で Upgrade → Checkout Session 作成 API が 200 を返す |

### Phase U1: Pro 価値の可視化（工数: 中 / U0 後、OP-057-R と同バッチ推奨）

**「課金メリットが伝わらない」への直接回答。** 新規は最小限、既存拡張を優先。

| ID | 内容 | 対象 |
|---|---|---|
| U1-1 | `useSubscriptionStatus` フック新設: `GET /api/v1/commerce/subscription/{agent_id}` を取得しコンテキスト供給（バックエンドは既存） | `hooks/`（新規1ファイル） |
| U1-2 | **PlanBadge**: ヘッダ常設の Free/Pro バッジ。Free 時はクリックで ProUpgradeModal を能動オープン。**実装パターンは既存 `EkycStatusBadge`（`components/character/`）を踏襲**（車輪の再発明防止） | `App.tsx` ヘッダ + 新規小コンポーネント |
| U1-3 | **ロック表示パターン**: Pro ゲート機能（Buzz 生成 / Treasure / LoRA 市場 / ギフト / ギルド / TTS）に鍵アイコン+「Pro で解放」ツールチップ。隠すのではなくグレーアウトで見せる。**視覚レイアウトは `ImmuneSystem.tsx` の Lock+説明パターン（L510, L544）を参考**（Pro 用ロック UI 自体は既存皆無 = 新規妥当） | 各対象コンポーネント（§1.2 M-4 の10箇所） |
| U1-4 | ProUpgradeModal 改修: i18n 化（ja/en）、機能リストを実ゲートと一致させる（Buzz 自動生成・AgentSense・LoRA 市場・ギフト・ギルド・TTS）、トリガー元機能名を表示（「◯◯は Pro 機能です」） | `ProUpgradeModal.tsx` |
| U1-5 | KC と Pro の分離表示: VoiceStore「Recharge」/ NurtureDashboard「Buy Points」に「KC チャージ」であることを明記し、Pro サブスクとは別セクション化 | `VoiceStore.tsx` / `NurtureDashboard.tsx` |
| U1-6 | Mock モードに `MOCK_SUBSCRIPTION_STATUS=free` 環境変数を追加し、開発・デモで Free 体験（ロック→解放の流れ）を再現可能に。既存の `subscription_override`（`mock.rs` AtomicU8）を env から初期化する形で最小実装 | `libs/aiome-commerce/src/mock.rs` / `core_services.rs`（Safety-Critical 外の Mock 部のみ） |
| U1-7 | 【決定】NurtureDashboard のスタブ「View Store」は**アプリ内 `store` タブへの遷移に差替**（`setActiveTab('store')` を props/コールバックで受け取る。外部 LP へ飛ばさない — アプリ内の文脈を切らない） | `NurtureDashboard.tsx` L153 |
| U1-8 | **（v4）決済後の着地体験**: `/checkout/success` ルートを新設（AiaaOnboardingWizard が既に URL 指定済みだが未実装 = M-9）。内容は「決済ありがとうございます + 状態を更新ボタン（subscription API 再 fetch）+ 反映されない場合の案内（`stripe-setup.md` 参照）」。`useCheckoutSession` の success_url もここへ変更。**決済ロジックには不介入**（表示+再取得のみ） | `App.tsx`（Route 追加）/ `hooks/useCheckoutSession.ts` L25–32 / 新規1コンポーネント |

> **U1 の効果測定**: 「Pro 導線の能動表示回数」がゼロ→常時1（バッジ）+ロック箇所N。402 に頼らない。
>
> **（v4）U1 の限界の明示**: 本番相当では「支払い→Pro」の閉ループ自体が OP-057-R（凍結）待ち（M-10）。U1 の責務は「見せる→ Checkout に正しく到達させる（U0-6）→決済後に迷子にしない（U1-8）」まで。自動有効化・Webhook・`register_license` 本番実装は Safety-Critical のため本計画では一切触れない。開発デモは Mock エンジン（`STRIPE_API_KEY` 未設定時に全 agent Active）+ U1-6 の Free 固定で成立させる。

### Phase U2: 情報設計の再編 — 「Simple / Cockpit」2モード制（工数: 大 / U0 後）

**「使いづらい」への直接回答。** ナビの二重構造と3段モードを解体する。

| ID | 内容 | 対象 |
|---|---|---|
| U2-1 | viewMode を `simple` / `cockpit` の2値に統合（`beginner`→`simple`、`intermediate`/`advanced`/`expert`→`cockpit` へマイグレーション。設定でいつでも切替可）。**既存の `useViewMode.ts`（localStorage `aiome_view_mode` + `GET/PUT /api/v1/settings`）を拡張し、取得時に旧値→新値の写像を挟む**。別系統の state 管理を新設しない | 波及全18箇所は §4.5 参照 |
| U2-2 | **Simple モード**: HomePage 4タブを維持しつつ「world」タブの7サブ項目を3つに集約（「AIのようす」=dashboard+trace+chronicle、「Biome」、「つながり」=p2p+map）。demo は初回のみ表示。**【決定】`biome` は Cockpit サイドバーの「観測」グループにも追加する**（現状 HomePage 専用で Cockpit から到達不能 = S-3 と同種の到達性バグ。`isVisible()` の advanced 配列 + サイドバー JSX に `biome` を追加） | `home/HomePage.tsx` / `App.tsx` |
| U2-3 | **Cockpit モード**: サイドバー26項目を5グループに再編成（下表）。1グループ7項目以内 | `App.tsx` サイドバー JSX |
| U2-4 | チャット統一: StoryFlow と AgentConsole の重複を解消。**前提工事として `useAgentChat` の Context 化が必須**（現状は各コンポーネント独立インスタンスでタブ切替時に履歴が分断 = 二重 UI 問題の根因）。レンダリングは `variant="timeline"\|"console"` の props 設計で統合（AgentConsole=Markdown+Mermaid、StoryFlow=FlowCard+Vitality タイムラインの差異を吸収）。Automations タブ（agency 専用）は Cockpit のみ露出 | `hooks/useAgentChat.ts`（Context 化）/ `home/StoryFlow.tsx` / `AgentConsole.tsx` / `home/FlowCard.tsx` |
| U2-5 | 一次ラベルの日常語化: i18n の nav 系キー（nav 28 / page 26）を「AIとはなす / ようすを見る / ふやす（経済） / まもる（安全） / 整える（設定）」系に。専門語はサブテキストへ。**キー名は維持し値のみ変更**（テストはキー文字列をアサートしているため、値変更なら Jest 修正ゼロ）。用語は `MESSAGING.md`（SSOT）と整合させる | `i18n/ja.json` / `en.json` |
| U2-6 | 【決定】未マウント `ModelSetupStep` は**削除**する（どこからも import されていない dead code。git 履歴から復元可能。組込が必要になった場合は U4-3 の動的ガイドが後継となるため常設ステップとしては不要） | `ModelSetupStep.tsx`（削除前に `rg "ModelSetupStep" src/` で参照ゼロを確認） |

**U2-3 サイドバー再編案（Cockpit）**

| グループ | 含む activeTab |
|---|---|
| ホーム | home-v2, agency |
| 対話・実行 | agent, workflow-builder, buzz-approval, artifacts |
| 観測 | dashboard, demo, karma, graph, causal, seo-pulse, status-page, prompt-stats |
| 経済（Nurture） | store, nurture, treasure 系 |
| 安全・基盤 | immune, ban-dashboard, audit, mcp-dashboard, vault, cortex, lora, expressions, commune, settings |

### Phase U3: デザイン統一（工数: 中 / U0–U2 と並行可）

| ID | 内容 | 対象 | 検証 |
|---|---|---|---|
| U3-1 | Biome 系 HEX 直書きを `tokens.css` 参照へ全置換（OP-029 吸収） | `lib/biome/*.tsx` / `biome-popup-entry.tsx` | `python3 scripts/test_ui_hex_violations.py` 0 違反 |
| U3-2 | `DESIGN.md` ⇔ `tokens.css` の乖離解消（rose 値・フォント定義）。runSync で Nurture 側にも同期 | `DESIGN.md` / `src/styles/tokens.css` | 差分ゼロ |
| U3-3 | インライン style の CSS クラス化（NurtureDashboard を最初の対象に、カード/バッジ/テーブルの共通クラス抽出） | `commerce/*.tsx` → `App.css` or 新規 `commerce.css` | lint PASS |
| U3-4 | 共通 UI プリミティブの整備: `Card` / `StatCard` / `LockedOverlay` / `SectionHeader` に加え **`LoadingState` / `EmptyState` の計6つ**を新設（過剰な共通化はしない）。**新規発明ではなく既存資産の抽出・ラップとして実装**: `App.css` の `.stat-card`（L385–398）と `.card-hover`（L462–467）をそのまま参照。`LoadingState` は `AiomeSkeleton` + `App.tsx` Skeleton グリッド、`EmptyState` は `ArtifactVault` の `.empty-state`（dashed border+アイコン+2行）+ `ExpressionPipeline` の CTA 付きパターンから抽出。`config-card` / `glass-card` はクラス名だけ存在し CSS 実体がインライン style のため、この機会に共通 CSS へ吸収 | `components/ui/`（新規）+ `App.css` | 各画面で再利用、二重定義ゼロ。U5-3/U5-4 の前提部品 |

### Phase U4: Generative UI（A2UI）の実戦投入（工数: 大 / U1・U2 後）

**「Generative UI の使い道」への回答。** A2UI Phase 0 基盤（validator 16種 / SSE / A2uiRenderer）を「AI に詳しくないユーザーの案内役」として使う。

| ID | 使い道 | 実装内容 |
|---|---|---|
| U4-0 | **前提工事**: (1) `card` を catalog に登録（validator には登録済みだが catalog 未登録のため LLM が生成経路で使えない）(2) **（v4）`a2ui_generative_ui` flag はデフォルト無効**（`traits.rs` L508–514 `unwrap_or(false)`）かつ SettingsPage の Feature Flags にトグル未掲載 → トグルを追加（Settings API は `feature_flag.*` 対応済み = `settings.rs` L157–171、既存7トグルと同パターン） | `bootstrap/state_assembly.rs` L290–328 / `SettingsPage.tsx` L455–496 |
| U4-1 | **ナビゲーションコンシェルジュ**: チャットで「ログが見たい」→ A2UI `card` + `button`（action: タブ遷移）を生成。ユーザーはメニューを覚えなくてよい | `catalog` にナビ action 追加 / `routes/a2ui.rs` に遷移 action / フロントで `setActiveTab` 連携 |
| U4-2 | **Pro ショーケース**: Free ユーザーが Pro 機能に触れた文脈で、LLM が `treasureItem`/`card` で「あなたの使い方なら Pro でこれができる」パーソナライズ提案を生成（ダークパターン禁止: 頻度制限・閉じたら当日再表示しない） | catalog + ProUpgradeModal 連携 |
| U4-3 | **動的セットアップガイド**: SetupWizard 完了後の「次の一歩」を A2UI `timeline`/`form` で対話生成（固定チュートリアルの代替）。**UI シェル（ステップ遷移・スポットライト・localStorage 完了フラグ）は既存 `BiomeTutorial.tsx` のパターンを再利用**し、ツアー基盤をゼロから作らない | `constants/slashCommands.ts` パターン転用 + `lib/biome/BiomeTutorial.tsx` 参考 |
| U4-4 | **updateComponents / deleteSurface の実装**: Phase 0 のプレースホルダを実レンダリングに（進行中タスクの進捗カード自動更新に必須） | `A2uiRenderer.tsx` / `types.ts` |
| U4-5 | **voiceStore / loraMarket の validator 正式登録**: スラッシュコマンド専用 → LLM 生成経路でも安全に出せるように | `libs/infrastructure/src/a2ui/validator.rs` / `catalog` |
| U4-6 | **Nurture ウィジェット追加**: `walletWidget`（KC 残高）/ `marketplaceItem` を catalog に追加し、チャット内で経済操作を完結 | `a2ui/schema.rs` / `catalog` / `A2uiRenderer.tsx` |

> **A2UI 安全境界**: 新コンポーネントは必ず validator ホワイトリスト + XSS/SSRF ガード（`SECURITY_WHITEPAPER.md` §2.6）を通す。決済実行そのものは A2UI action に載せない（遷移まで）。ADR-035 の「A2UI はプレゼンテーション層」原則を維持。
>
> **U4 の正確な現状値（/perfect-plan 検証）**: validator ホワイトリストは 16 種だが **catalog 登録は 15 種（`card` のみ catalog 未登録）** → U4 着手時に `state_assembly.rs` L290–328 へ `card` を登録する（U4-0 として先行）。action 許可 prefix は `approve_job:` / `run_skill:` / `cancel_job:` の3種のみ（`routes/a2ui.rs` L60）で、ナビ遷移 action は完全新規。**（v4）加えて `a2ui_generative_ui` flag はデフォルト OFF かつ ON にする UI が存在しない** — U4 の全機能はこの flag の背後にあるため、U4-0 のトグル追加なしでは U4-1〜6 をユーザーが体験できない。

### Phase U5: 体感品質と品質基盤（v3 新設 / U5-1〜4 は工数小で U1 と並行可、U5-5〜8 は工数大）

**「プロダクトの質」を決める最後の1マイル。** 診断 §1.4（Q-1〜Q-5）/ §1.5（F-1〜F-6）への直接回答。全項目が既存資産の横展開であり、新規発明はゼロ。

#### U5-A: 体感品質（工数: 小〜中 / U3-4 の LoadingState/EmptyState が前提）

| ID | 内容 | 対象 | 検証 |
|---|---|---|---|
| U5-1 | **サイレント失敗の一括修正**: fetch 失敗時に `StatusPage.tsx` L78–93 のエラー+Retry パターンまたは Toast を表示。失敗と「データゼロ」を UI で区別 | `GraphView` / `DiagnosticsHistory` / `SettingsPage`（初回読込）/ `ExpressionPipeline` / `home/CharacterPanel` の5画面 | ネットワーク遮断（Negative Test）で各画面にエラー表示+リトライが出る |
| U5-2 | **操作フィードバック統一**: SettingsPage 保存成功に `showToast('success')` 追加（既存 Toast をそのまま利用）。`VaultSecretsManager` の `alert()` → Toast、`BanDashboard` の `window.confirm()`（英語直書き）→ 共通 `ConfirmModal` へ置換 | `SettingsPage.tsx` / `VaultSecretsManager.tsx` / `BanDashboard.tsx` | `alert(`/`window.confirm(` が src/ から消える（rg 0件） |
| U5-3 | **空状態 CTA テンプレート適用**: `EmptyState`（U3-4）を Timeline / SkillVault / DiagnosticsHistory / TreasureBox / PromptStatsView / BiotopeView に適用。各画面に「次の一手」（チャット開始・スキル入手・Pro 案内等）を配置。SkillVault は「フィルタ結果ゼロ」と「初回ゼロ」を区別 | 上記6画面 + `components/ui/EmptyState` | データゼロ状態で全画面に CTA が表示される |
| U5-4 | **ローディング統一**: 6パターン混在を `LoadingState`（U3-4）に集約。`Suspense fallback={<div />}`（完全空白）を Skeleton に差替 | `App.tsx` L350–356 / Q-2 記載の各画面 | 白画面・静止画面ゼロ。`t('loading')` テキストのみの画面ゼロ |
| U5-5 | **Toast の複数表示対応**: 現在同時1件制限（`Toast.tsx` L33–44）→ キュー化（最大3件）。U5-1/2 で採用画面が7→15+ に増えるための足回り | `common/Toast.tsx` | 連続操作で通知が握り潰されない |

#### U5-B: 品質基盤（工数: 大 / U2 完了後推奨 — ナビ構造確定後に a11y/レスポンシブを当てる）

| ID | 内容 | 対象 | 検証 |
|---|---|---|---|
| U5-6 | **a11y 基盤**: (1) 共通 Modal プリミティブ（フォーカストラップ・Escape・`role="dialog"`+`aria-modal`）を新設し ProUpgradeModal / ConfirmModal / AvatarViewerModal を移行 (2) `NavItem` を `<div onClick>` → `<button>` 化 (3) アイコンのみボタンへ `aria-label` 一括付与（`data-tooltip` 併存可） | `components/ui/Modal`（新規）/ `App.tsx` L789–798 / 各 icon button | axe-core スモーク（重大違反0）+ Tab キーのみで全ナビ到達 |
| U5-7 | **レスポンシブ最小対応**: `tokens.css` に `--bp-sm/md/lg` を定義（散在値 768/900/1024 を統一）。md 未満でサイドバーをオーバーレイドロワー化、HomePage 左ペイン 320px 固定を縦積みに、world サブタブを折返し可に | `tokens.css` / `App.css` / `home/HomePage.tsx` L218 | 375px 幅で横スクロールなし・全機能到達可 |
| U5-8 | **reduced-motion 全局対応**: `usePrefersReducedMotion` フック新設（`useFluidConfig.ts` L13–21 の判定を昇格）+ Framer Motion は `MotionConfig reducedMotion="user"` をルートに1箇所 | `main.tsx` or `App.tsx` / 新規フック | OS 設定 ON でタブ遷移・pulse が停止 |
| U5-9 | **性能**: `DioramaView` を `React.lazy` 化（three.js をメインバンドルから排除、他24画面は lazy 済みなので同パターン）。Timeline のリスト仮想化は**計測してから**（`vite build` の chunk 分析で效果を確認後に判断） | `App.tsx` L68 / `Timeline.tsx` | ビルド後メイン chunk サイズ減を数値で記録 |
| U5-10 | **animations.css 死活整理**: 未使用5種（`ani-float`/`h-hover`/`h-holographic`/`avatar-breath`/`neural-line-animate`）を削除 or 用途明記。hover 演出のインライン `onMouseEnter` 直書きを CSS クラスへ | `styles/animations.css` / `home/HomePage.tsx` L267–268 | 未使用定義 0（rg で全クラス使用箇所確認） |

> **やらないと決めたこと（v3）**: **ライトテーマ（F-4）は本計画のスコープ外**。tokens.css は Premium Dark 前提で設計されており、二重テーマ化は全画面再検証（工数大）の割に「AI に詳しくないユーザーに使いこなせる」という本計画の目的への寄与が薄い。将来の要望が実証されてから別計画で扱う。

---

## 4. 実行順序と委譲マップ

```
U0（バグ修正）──┬──> U1（Pro可視化）──────> U4（GenUI 実戦投入）
                └──> U2（情報再編）──┬──↗
                                     └──> U5-B（a11y/レスポンシブ — ナビ確定後）
U3（デザイン統一）は全フェーズと並行可（U3-4 は U5-A の前提部品）
U5-A（体感品質）は U3-4 完了後、U1 と並行可
OP-057-R（決済→Pro自動有効化）は U1 と同バッチが最効率（サブスク状態APIを共用）
```

| 作業種別 | 担当 | 理由 |
|---|---|---|
| U0 全部・U3-1（HEX置換）・U1-4（i18n化）・U2-5（ラベル置換）・U5-2（alert/confirm 置換）・U5-3（EmptyState 適用）・U5-10（死活整理） | **低トークンサブエージェント**（composer 系） | 機械的置換・パターン作業 |
| U1-1/2/3（サブスク状態設計）・U2-1〜4（情報設計）・U4 全部・U5-6（Modal 設計）・U5-7（レスポンシブ設計） | メインエージェント | 設計判断・Safety 境界あり |
| U1-6（Mock 拡張） | メイン + 人間レビュー | commerce 隣接（Safety-Critical 境界確認） |
| U5-1（サイレント失敗修正）・U5-4/5（Loading/Toast） | メイン設計 → サブエージェント横展開 | パターン確立後は機械的 |
| 検証（lint / Jest / hex violations / axe スモーク） | 低トークンサブエージェント | 定型実行 |

### 4.5 U2 波及マップ（Gate 3 検証で確定・全数）

**viewMode リテラル比較はフロント18箇所**（型1 / 条件分岐14 / SetupWizard 3 / デフォルト2）。永続化は DB 専用カラムではなく key-value（`update_setting("view_mode", _, "ui")`）。

| レイヤ | 変更ファイル |
|---|---|
| フロント本体 | `types.ts` L203 / `hooks/useViewMode.ts`（写像はここに集約） / `App.tsx` L305–312, L417 / `SettingsPage.tsx` L231–235, L311, L393, L425, L447, L501 / `SetupWizard.tsx` L60, L367, L378, L392 |
| i18n | `ja.json` / `en.json` の `settings.viewMode_*`（L232–234）と `setup.*`（L329–333） |
| バックエンド | `routes/setup.rs` L87–92（`allowed_modes` 更新+旧値エイリアス）/ `routes/settings.rs`（PUT 時の enum 検証を追加 — 現状は任意文字列を保存可能） |
| テスト（Jest 3） | `useViewMode.test.ts` / `SettingsPage.test.tsx` / `App.test.tsx` L110–112 |
| テスト（E2E 2） | `e2e/helpers/auth-bypass.ts` L28 / `e2e/promo-clips.spec.ts` L108 |
| テスト（Rust） | `api_integration_tests/setup.rs` L23（`"expert"` 使用中） |
| ドキュメント | `.agent/skills/playwright-e2e-stabilization.md`（`aiome_view_mode` 記述）/ CHANGELOG |

i18n nav 日常語化（U2-5）の参照は本番3ファイル57箇所（`App.tsx` 55 / `McpDashboard.tsx` 1 / `BiotopeView.tsx` 1）。テストはキー文字列アサートのため**値のみ変更なら修正ゼロ**。

**（v4）i18n パリティの追認**: ja/en はトップレベル50キー・リーフ975キーで完全一致し、`i18n.test.ts` L22–33 が一致を強制 → U1-4/U2-5/U5-3 のキー追加は**必ず ja/en 同時**（テストが守ってくれる）。

### 4.6 他ロードマップとの境界と協調（v4・横断監査で確定）

**方針矛盾する ADR・計画はゼロ**（ADR-035 A2UI プレゼンテーション層原則とも整合）。要協調は以下の5点。

| 論点 | 内容 | 対応 |
|---|---|---|
| OP-029 二重起票 | OP-029（`biome-popup-entry.tsx` HEX）は U3-1 のサブセット | OPEN.md 側に「U3-1 に吸収、着手時クローズ」を注記（対応済み） |
| pr_quality の陳腐化 | `pr_quality_improvement_plan.md` M-D3 / `synergy_maximization_plan.md` W-7c の「ProUpgradeModal 未マウント」記述は OP-058 完了で解消済み | **U1 が Pro 訴求 UI の SSOT**。旧計画は歴史的診断として据え置き |
| implementation_plan P3 | P3「UI/UX 強化」（LoRA FE / Biome ダッシュボード / Settings Nurture パネル）は本計画 U1–U3 に包含 | P3 を単独実行しない（二重投資防止）。本計画が後継 |
| SkillVault の順序 | U5-3（EmptyState 基盤）→ `h1_f2_f3_implementation_plan.md` M-5（マーケットタブ）の順で実施 | 基盤→機能の順。統合は不要 |
| SetupWizard の順序 | U0-2（expert 修正）→ `f1_agent_playbooks` P-5（Playbook ステップ）→ U2-6 / U4-3 | 同一ファイルの改修順を固定し conflict 防止 |

Biome 側（`biome_lenia_overhaul_plan.md` Phase 4 / `biome_collection_beauty_plan.md`）には HEX・トークン・空状態の起票なし = U3-1/U5 との重複なし。Biome Phase 4 のチュートリアルは**ゲーム内**導線で、U4-3（アプリ全体ガイド）とはスコープ分離（シェルのパターン再利用のみ）。

---

## 5. 検証基準（Goal-Driven）

| フェーズ | 成功基準 |
|---|---|
| U0 | 全タブ到達可能・Expert→サイドバー表示・TTS 402→モーダル表示（Negative Test: Free 状態で TTS 実行） |
| U1 | Free 状態の全ゲート箇所でロック表示+モーダル能動オープン可。`npm run lint` / `npm test`（Jest）PASS |
| U2 | viewMode 2値でマイグレーション後も既存ユーザー設定が壊れない（旧値→新値の写像テスト） |
| U3 | `scripts/test_ui_hex_violations.py` 0 違反。DESIGN.md と tokens.css の diff ゼロ |
| U4 | LLM 生成 envelope が validator を通過して描画される E2E。不正 envelope（未登録 type / スクリプト注入）が**拒否される** Negative Test 必須 |
| U5-A | ネットワーク遮断 Negative Test で対象5画面全てにエラー+リトライ表示。`alert(`/`window.confirm(` rg 0件。データゼロ状態の全画面に CTA |
| U5-B | axe-core スモーク重大違反0・Tab キーのみで全ナビ到達・375px 幅で横スクロールなし・メイン chunk サイズ削減を数値記録 |

---

## 6. スコープ外（明示）

- 決済ロジック（`commerce.rs` / Webhook）の変更 — OP-057-R・Safety-Critical
- LP（docs/landing）の変更 — 本計画は管理コンソール + Nurture UI が対象
- 独立 `nurture-ui`（Project-Nurture 別リポジトリ）の新設 — tokens 同期（runSync）のみ本計画で維持
- 3D/VRM レンダリングパイプラインの改修（U-005 WebGL Safety 遵守で現状維持）
- **ライトテーマ対応（F-4）** — Premium Dark 前提の tokens 体系を二重化するコストに対し本計画の目的への寄与が薄い（§3 Phase U5 末尾に判断理由）
- **リスト仮想化の一律導入** — U5-9 で計測し効果が実証された画面のみ。`react-window` 等の依存追加は計測前に行わない

## 7. 悪魔の弁護人（自己検証・Gate 4）

1. **最悪シナリオ**: U2 の viewMode 統合で既存ユーザーの設定が読めなくなり全員 Simple に落ちる → マイグレーション写像 + フォールバックを U2-1 の受入条件に含めた。写像は `useViewMode.ts` の1箇所に集約し、バックエンド `setup.rs` でも旧値を受理する二重防御。
2. **見落とされがちな前提**: 「Pro 可視化すれば課金される」は未検証仮説。U1 はあくまで「伝わる」までを保証し、価格・訴求文言は MESSAGING.md（SSOT）側の管轄とする。
3. **やらない選択肢**: U4（GenUI）は延期可能。U0+U1 だけでも「伝わらない・使いづらい」の大半は解消するため、トークン予算が厳しい場合は U4 を Biome Phase 5 後へスライドする。
4. **（v2 追加）U2-4 の隠れコスト**: チャット統一は「薄いラッパ化」では済まず `useAgentChat` Context 化が前提工事。これを飛ばすと履歴分断が残り統一の意味がない。工数見積りは「大」のまま据え置きが正当。
5. **（v3 追加）U5 のスコープ膨張リスク**: 「品質」は際限がない。U5 は診断 Q-1〜Q-5 / F-1〜F-6 の**列挙済み証拠のみ**を対象とし、監査で検出されなかった画面への予防的適用は行わない。ライトテーマと一律仮想化は明示的に「やらない」と決めた（§6）。
6. **（v3 追加）U5-B を U2 の前にやる誘惑**: レスポンシブ・a11y をナビ再編（U2）前に当てると、再編で捨てる `<div>` 構造に工数を注ぐことになる。順序は U2 → U5-B を厳守する。

---

## 8. /perfect-plan 検証結果（2026-07-05・v2 反映済み）

**検証体制**: Gate 1（[車輪の再発明チェック](d2874bbe-25e9-442d-8dd6-2c9018fceb58)）と Gate 3（[波及分析](8531fa9c-3a25-491e-b0dd-5589ce61be6d)）は低トークンサブエージェントに委譲。Gate 2/4/5 はメインで実施。

### Gate 1: 構造スキャン — ⚠️→✅（PATCH 適用済み）
- ✅ 幻覚なし: 計画記載の全パス・行番号・API（`commerce.rs` L432 subscription 等）実在確認済み
- ⚠️→修正: U3-4 は新規発明でなく **既存 `.stat-card`/`.card-hover` の React ラップ**に変更。`config-card`/`glass-card` のインライン実体を共通 CSS へ吸収
- ⚠️→修正: catalog は 15 種（`card` 未登録）→ **U4-0（card 登録）を追加**
- ⚠️→修正: U4-3 は `BiomeTutorial.tsx` パターン再利用を明記
- ✅ 二重実装なし: `components/ui/`・`useSubscriptionStatus`・ナビ action・Pro ロック UI はいずれも未存在（新規妥当）

### Gate 2: 要件カバレッジ — ✅
- §2 経済台帳: 影響なし（表示のみ。決済実行は A2UI action に載せない）
- §3 MCP: 影響なし
- §4 セキュリティ: U4 は validator ホワイトリスト経由のみ。Negative Test を §5 に規定済み
- §5 法的リスク: U4-2 にダークパターン禁止条件（頻度制限・当日再表示なし）を明記済み
- §6 VRM/3D: スコープ外宣言済み（U-005 遵守）
- §7 A2C: TreasureBox のロック表示（U1-3）が該当。報酬ロジック自体は不変
- §8 P2P Hub: U2-2 で `CommuneDialogueView` を「つながり」に集約するが表示位置の変更のみ。Federation/CRDT 同期には不介入

### Gate 3: 依存関係 & 波及 — ⚠️→✅（§4.5 を追加）
- viewMode 18箇所・Jest 3・E2E 2・Rust テスト1・スキル文書1を全数列挙（§4.5）
- StoryFlow/AgentConsole 統一の前提工事（`useAgentChat` Context 化）を U2-4 に組込

### Gate 4: 悪魔の弁護人 — §7 に4項目（v2 で1項目追加）

### Gate 5: 実行順序 — ✅
- U0 → U1 → U4 / U0 → U2 → U4 の依存は妥当。U3 並行可も維持
- 修正1件: U4 冒頭に **U4-0（catalog へ card 登録）** を追加（U4-1〜6 の前提）
- U2-4（Context 化）は U2-1〜3 と独立して先行着手可能（並列化余地）

### 判定: ⚠️ PATCH → **✅ PASS（v2 で全修正適用済み）**

---

## 9. /perfect-plan 第2周検証結果（2026-07-05・v3 反映済み）— 「プロダクト品質最大化」観点

**検証体制**: [体感品質監査](ecb6afc9-8f56-41cb-99a4-c9f28254b852)（ローディング/空状態/エラー/フィードバック/フォーム/初回体験）と[品質基盤監査](7fe7da22-0014-4c33-993b-b4406225c11a)（a11y/レスポンシブ/テーマ/motion/性能/タイポ）をサブエージェント2体に委譲。v2 計画との重複は監査側で除外済み。

### Gate 1: 構造スキャン — ✅（再利用資産を全数特定）
- U5 の全項目に既存資産あり: `Toast.tsx` / `ConfirmModal.tsx` / `AiomeSkeleton.tsx` / `StatusPage` エラー+Retry / `ExpressionPipeline` CTA 付き空状態。**新規発明ゼロで Phase U5 を構成**
- 発見済みの良い実装（`TreasureBox` の loading+empty 分岐、`SetupWizard` のリアルタイム検証）を横展開テンプレートに指定

### Gate 2: 要件カバレッジ — ✅
- v2 計画は「伝わる・使える・美しい」をカバーするが、**「信頼できる」（失敗が見える・操作が応答する）と「誰でも使える」（キーボード・モバイル・motion 配慮）が欠落**していた → §1.4（Q-1〜5）/ §1.5（F-1〜6）として診断に追加し、Phase U5 で回答
- NURTURE §4 セキュリティ: U5 は表示層のみで認証・入力検証ロジック不変。§5 法的リスク: a11y 改善はコンプライアンス上プラス方向のみ

### Gate 3: 依存関係 — ✅
- U5-A は U3-4（LoadingState/EmptyState プリミティブ）に依存 → U3-4 を6プリミティブに拡張して吸収
- U5-5（Toast キュー化）は U5-1/2 の採用画面拡大（7→15+）の前提 → 順序を明記
- U5-9（DioramaView lazy 化）は既存24画面と同パターンで独立実施可

### Gate 4: 悪魔の弁護人 — §7 に2項目追加（スコープ膨張・U5-B の実施順序）

### Gate 5: 実行順序 — ✅
- U5-A（工数小）は U3-4 後に U1 と並行可 / U5-B（工数大）は **U2 完了後**（ナビ構造確定前に a11y/レスポンシブを当てると手戻り）
- ライトテーマ（F-4）と一律仮想化は「やらない」判断でスコープ外に明記

### 判定: **✅ PASS（v3）** — 体感品質（Q-1〜5）と品質基盤（F-1〜6）を診断・フェーズ・検証基準・委譲マップまで一気通貫で組込済み

---

## 10. /perfect-plan 第3周検証結果（2026-07-05・v4 反映済み）— 課金ジャーニー一気通貫 + 計画横断監査

**検証体制**: [課金ジャーニー監査](49a390c4-2aff-44ae-b352-a0a9d0a97912)（支払い→Pro の実経路トレース）と[他ロードマップ重複検査](a4c01aaf-fb06-4a66-9ce6-0fb225b97ddd)（roadmaps 14件 + OPEN.md + ADR + DESIGN.md 突き合わせ）をサブエージェント2体に委譲。

### Gate 1/2: ジャーニー検証で発見した重大ギャップ — ⚠️→✅（PATCH 適用済み）
- **M-8（重大バグ）**: `App.tsx` L782 が ProUpgradeModal に `agentId` を渡しておらず、**アプリ内唯一のアップグレード導線が Checkout 作成で失敗し得る** → U0-6 として即修正枠に追加（VoiceStore と同パターンの1行）
- **M-9**: 決済完了後の着地 UI が皆無（同一ページに黙って戻る / `/checkout/success` はルート未定義で404相当） → U1-8 を追加（表示+状態再取得のみ、決済ロジック不介入）
- **M-10（構造的制約の明文化）**: 本番相当では「支払い→Pro」の閉ループ自体が存在しない（Pro 判定は Stripe API 照会のみ・LP Payment Link は agent 未紐付け・`register_license` は v1.0 封印・ライセンス入力 UI/API なし）。**U1 の責務境界を「見せる→正しく Checkout へ→迷子にしない」までと定義**し、自動有効化は OP-057-R（Safety-Critical・凍結）に委ねることを明記
- **U4 の隠れ前提**: `a2ui_generative_ui` flag はデフォルト OFF かつ ON にする UI が存在しない → U4-0 に SettingsPage トグル追加を組込（Settings API は対応済みのため小工数）
- 朗報: ja/en i18n は975キー完全一致+テスト強制 → U1-4/U2-5 の追加キーはテストが守る

### Gate 3: 計画横断（重複・矛盾） — ✅（§4.6 を追加）
- **方針矛盾する ADR・計画はゼロ**。Biome 2計画に HEX/トークン/空状態の起票なし（U3-1/U5 と重複なし）
- 要協調5点を §4.6 に確定: OP-029 は U3-1 に一本化 / pr_quality M-D3・synergy W-7c は陳腐化（U1 が SSOT）/ implementation_plan P3 は本計画が包含（単独実行しない）/ SkillVault は U5-3→h1 M-5 の順 / SetupWizard は U0-2→f1 P-5→U2-6/U4-3 の順
- DESIGN.md の rose 値は**文書内部でも矛盾**（L83 `#ff4d94` vs L200 `#f472b6`）— D-3/U3-2 の診断を追認

### Gate 4: 悪魔の弁護人（第3周の意地悪な問い）
1. **「U1 で Pro を見せても、買った人が Pro になれないなら逆効果では？」** — 正しい。だから U0-6（導線バグ修正）を U0 の即修正に昇格し、U1-8 で決済後の期待値管理（反映待ち・手動手順への案内）を行う。閉ループ完成前に「見せる」だけを先行させない順序に修正した。
2. **「計画が14ロードマップの15本目になるだけでは？」** — §4.6 で本計画が implementation_plan P3 の後継 SSOT であることを明記し、吸収・凍結関係を全て一覧化した。

### Gate 5: 実行順序 — ✅
- U0-6 は U0（即修正）へ、U1-8 は U1 内で U1-1（サブスク状態取得）の後。他フェーズへの影響なし

### 判定: **✅ PASS（v4）** — 3周の検証で「見せる UI」「使える UI」「信頼できる UI」に加え「**買える UI**（課金ジャーニーの成立）」までカバー。残る未検証仮説は「可視化→課金転換率」のみ（実装後の計測事項）

---

## 11. /perfect-plan 第4周検証結果（2026-07-05・v5 反映済み）— 実装可能性の確定

**検証体制**: [検証コマンド・ツーリング実在確認](a1401a62-7809-431a-9378-8e1fd4faeecf)と [API/コンポーネント契約の確定](53f029f6-ca59-489b-8687-fa2bc6a6af5a)をサブエージェント2体に委譲。「計画に従えば誰でも実装できる」ための曖昧さ排除が目的。

### Gate 1: 事実誤認の修正 — ⚠️→✅
- **計画中の「Vitest」は誤記 → Jest が正**（`package.json` L17 / `jest.config.js`）。全箇所修正済み
- `npm run lint` は **`tsc --noEmit`（型チェックのみ、ESLint ではない）**
- `GET /api/v1/settings?category=ui` の **category クエリはバックエンド未対応**（全件返却をクライアントで filter）— U2-1 実装者が「クエリが効かない」と混乱しないよう Appendix A に明記

### Gate 5: 未決定分岐の解消 — ⚠️→✅（全て【決定】に変換）
- U0-1: buzz-approval は**登録**（削除しない）/ U0-2: フロント統一+バックエンドエイリアスの**二重防御** / U1-7: **アプリ内 store タブ遷移**に差替 / U2-2: biome は **Cockpit「観測」グループにも追加** / U2-6: ModelSetupStep は**削除**
- 判断基準も併記（実装者が「なぜ」を理解して迷わないため）

### 新設: Appendix A（実装コントラクト）
- A.1 実装者が誤解しやすい前提事実 / A.2 新規フック・コンポーネントの型契約（実コード引用ベース）/ A.3 フェーズ別 DoD コマンド（コピペ可）/ A.4 導入が必要なツール

### 判定: **✅ PASS（v5）** — 未決定分岐ゼロ・契約明文化・DoD コマンド実在確認済み。実装フェーズへ移行可能な状態

---

## Appendix A: 実装コントラクト（v5 — 全て実コード引用ベース、推測なし）

### A.1 実装者が誤解しやすい前提事実

| # | 事実 | 証拠 |
|---|---|---|
| 1 | ユニットテストは **Jest**（Vitest ではない）。one-shot 実行は `npm test`（= `jest --testPathIgnorePatterns=e2e/`） | `package.json` L17 / `jest.config.js` |
| 2 | `npm run lint` = **`tsc --noEmit`**（型チェックのみ） | `package.json` |
| 3 | `GET /api/v1/settings` は **category クエリ非対応**（`?category=ui` を付けても全件返却。クライアントで `key === 'view_mode'` を find するのが既存実装） | `settings.rs` L46–59 / `useViewMode.ts` L19–27 |
| 4 | サブスク API のレスポンスは**プレーン JSON 文字列**（`"active"` / `"none"` 等 snake_case 9値。オブジェクトではない） | `libs/aiome-contracts/src/commerce.rs` L178–198（`#[serde(rename_all = "snake_case")]`）/ `generated.ts` L2428–2432 |
| 5 | サブスク API は **JWT 必須 + 他 agent の照会は 403**（`agent_id != auth.agent_id` を拒否） | `commerce.rs` L445–461 |
| 6 | `showToast` の引数順は **`(type, message)`**。type は `'success' \| 'error'` の2値のみ | `common/Toast.tsx` L11–44 |
| 7 | Feature Flag の settings key は **`feature_flag.<snake_case>`** 形式・category `feature_flags`・値は文字列 `"true"`/`"false"`（例: `feature_flag.seo_publish`） | `SettingsPage.tsx` L454–496 |
| 8 | E2E はローカルで **wasm-pack build → npm run build → npx playwright test** の3段（dist 必須。webServer が `cargo run -p api-server` を起動） | `playwright.config.ts` L48–69 / `.agent/skills/playwright-e2e-stabilization.md` |
| 9 | UI のみの変更でも **pre-commit で `cargo fmt --check`、pre-push で `make preflight`（Rust フル）が走る** | `scripts/install_hooks.sh` / `Makefile` L5 |
| 10 | `scripts/test_ui_hex_violations.py` のスキャン対象は `components/` `lib/` `App.tsx` のみ。**`biome-popup-entry.tsx`（OP-029 対象）はスコープ外** → U3-1 完了判定には `rg '#[0-9a-fA-F]{3,8}\|rgba?\(' apps/management-console/src/biome-popup-entry.tsx` を併用 | `scripts/test_ui_hex_violations.py` |

### A.2 新規フック・コンポーネントの契約

**U1-1 `useSubscriptionStatus`**（`hooks/useSubscriptionStatus.ts` 新規）

```typescript
type SubscriptionStatus = "active" | "cancelled" | "past_due" | "none"
  | "trialing" | "unpaid" | "incomplete" | "incomplete_expired"; // generated.ts L2428 の再利用可
interface UseSubscriptionStatusResult {
  status: SubscriptionStatus | null;  // null = 取得前 or agentId なし
  isPro: boolean;                     // status === 'active' || status === 'trialing'（auth.rs L158–172 の判定と同一）
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;       // U1-8 の「状態を更新」ボタンが使用
}
```

- 実装: `useAgentIdentity().agentId`（`string | null`、マウント時1回）→ null なら fetch しない。`authenticatedFetch(\`${API_BASE}/api/v1/commerce/subscription/${agentId}\`)` → `res.json()` は文字列1個。エラー時は `error` に格納し throw しない（`useCheckoutSession` L13–93 と同スタイル）
- 供給: React Context（`SubscriptionProvider`）で `App.tsx` に1回マウント。PlanBadge / LockedOverlay / ProUpgradeModal が消費

**U1-2 `PlanBadge`**（`EkycStatusBadge.tsx` L10–33 パターン踏襲）

```typescript
interface PlanBadgeProps { onUpgradeClick: () => void; }
// isPro → emerald pill「Pro」/ Free → muted pill「Free · アップグレード」クリックで onUpgradeClick
// 色は var(--accent-emerald-10)/var(--accent-emerald) 等トークンのみ（EkycStatusBadge と同一手法）
```

**U1-3 `LockedOverlay`**（`components/ui/`、U3-4 の1つ）

```typescript
interface LockedOverlayProps {
  featureNameKey: string;   // i18n キー（例: 'pro.feature.buzz'）
  children: React.ReactNode; // グレーアウト対象
}
// isPro なら children をそのまま返す。Free なら opacity+pointer-events:none で覆い、
// Lock アイコン + t('pro.unlockHint') + クリックで ProUpgradeModal 能動オープン
```

**U2-1 viewMode 写像**（`useViewMode.ts` に集約）

```typescript
export type ViewMode = 'simple' | 'cockpit';
const migrate = (raw: string): ViewMode =>
  raw === 'beginner' ? 'simple'
  : ['intermediate', 'advanced', 'expert', 'cockpit'].includes(raw) ? 'cockpit'
  : raw === 'simple' ? 'simple'
  : 'simple'; // 未知値は安全側（Simple）へ
// 適用点は3つ: localStorage 読込時・GET /settings 読込時・デフォルト値
// 保存時は新値のみ書く（PUT { key:'view_mode', value:newMode, category:'ui' }）
```

**U4-0 flag トグル**: `FeatureToggle` 既存パターン（`SettingsPage.tsx` L601–626）をそのまま1個追加。key は **`feature_flag.a2ui_generative_ui`**・category `feature_flags`（`system_instructions.rs` L134 が読む flag 名と一致させること）

**U4-1 ナビ action**: バックエンド `routes/a2ui.rs` L59–66 の `valid_prefixes` に `"navigate:"` を追加し、値は `isVisible()` の登録済みタブ名のみ許可（ホワイトリスト検証）。フロント `A2uiRenderer.handleAction`（L303–328）で `navigate:` prefix を検出したら POST せず `setActiveTab(tab)` をローカル実行（サーバー round-trip 不要・XSS 面も安全）

**U5-3 `EmptyState`**（`ArtifactVault.tsx` L251–256 の構造を抽出）

```typescript
interface EmptyStateProps {
  icon: LucideIcon;        // 48px, opacity 0.5
  titleKey: string;        // i18n
  detailKey?: string;
  cta?: { labelKey: string; onClick: () => void }; // 「次の一手」
}
```

### A.3 フェーズ別 Definition of Done（コピペ可・全コマンド実在確認済み）

```bash
MC=/Users/motista/Desktop/antigravity/aiome/apps/management-console
REPO=/Users/motista/Desktop/antigravity/aiome
```

| フェーズ | 自動検証（全て PASS が DoD） | 手動/Negative |
|---|---|---|
| **U0** | `cd $MC && npm run lint && npm test && npm run build` | Expert 選択→サイドバー / Free で TTS→モーダル / Free で Upgrade→Checkout API 200 |
| **U1** | 上記 + `npm test -- --testPathPattern=i18n`（キー追加時）+ `MOCK_SUBSCRIPTION_STATUS=free cargo test -p aiome-commerce`（U1-6） | Free 状態で全ロック表示+能動モーダル |
| **U2** | `npm test -- --testPathPattern='useViewMode\|SettingsPage\|App\.test'` + 全体 Jest + `cargo test -p api-server`（setup 系）+ `npx playwright test e2e/promo-clips.spec.ts` | 旧3値→新2値の写像テスト（新規作成必須） |
| **U3** | `cd $REPO && python3 scripts/test_ui_hex_violations.py`（0違反）+ `rg '#[0-9a-fA-F]{3,8}\|rgba?\(' $MC/src/biome-popup-entry.tsx`（0件）+ `cd $MC && npm run lint:design && npm run sync:tokens && git diff --exit-code src/styles/tokens.css` | — |
| **U4** | `npm test -- --testPathPattern=A2uiRenderer` + `cargo test -p infrastructure a2ui`（validator Negative 含む） | 不正 envelope（未登録 type/注入）拒否の手動確認。**A2UI E2E spec は新規作成**（A.4） |
| **U5-A** | `rg 'alert\(\|window\.confirm\(' $MC/src`（0件）+ 対象画面の Jest + build | ネットワーク遮断（Playwright `route.abort()`）で5画面にエラー+Retry |
| **U5-B** | axe スモーク（A.4 導入後）+ `npm run build 2>&1 \| tee /tmp/build-sizes.log`（chunk 前後比較） | 375px 幅・Tab キーのみ・OS reduced-motion |

### A.4 導入が必要なツール（現状未整備 — 各フェーズ着手時に追加）

| ツール | 用途 | フェーズ |
|---|---|---|
| `@axe-core/playwright`（devDep 追加） | a11y スモーク spec | U5-6 着手時 |
| A2UI E2E spec（`e2e/a2ui.spec.ts` 新規） | U4 の描画+Negative E2E | U4 着手時 |
| ネットワーク遮断 spec（`e2e/error-states.spec.ts` 新規） | U5-1 の Negative Test | U5-1 着手時 |
| `rollup-plugin-visualizer`（任意） | U5-9 chunk 分析（`npm run build` stdout でも代替可） | U5-9 着手時 |
| `test_ui_hex_violations.py` の CI 組込（任意） | U3 ゲートの回帰防止 | U3 完了時 |

---

## 12. v6（2026-07-07）: ユーザー実地レビューの反映 — 即修正バグ + 情報設計の確定

**契機**: ローカル起動レビューで「本人ステータスの読み込みに失敗しました」エラーが多重表示され、かつ「各機能の使い方もセクション毎の名前も全て分かりにくい。メニューの順番も含めて改修せよ」との判定を受けた。v5 までの計画は「ラベルの日常語化（U2-5）」を完了扱いにしていたが、**実地では効果不十分**であることが確認された。

### 12.1 Phase U0-B: 即修正バグ（工数: 小 / 最優先）

| ID | 症状 | 根本原因（検証済み） | 修正案 | 備考 |
|---|---|---|---|---|
| U0-B1 | ホーム表示直後に「本人ステータスの読み込みに失敗しました」トーストが3連発 | `GET /api/v1/ekyc/status` が **HTTP 500**（実測: `Missing request extension: AuthenticatedUser`）。`router.rs` L597–600 で本ルートが `internal_router`（`auth_middleware` 層 = 検証のみ、extension 注入なし）に登録されているが、ハンドラ `get_ekyc_status_handler`（`avatar.rs` L172–174）は `jwt_auth_middleware` が注入する `Extension<AuthenticatedUser>` を要求するため必ず落ちる。旧パス `/api/avatar/ekyc-status`（L819–827）は `jwt_auth_middleware` を個別適用しており正常 | (a) `router.rs` L597 のルートに `jwt_auth_middleware` を route_layer で適用（旧パスと同形）、または (b) ハンドラを `Authenticated` extractor に書換。**(a) を推奨**（ハンドラ不変・差分1箇所） | **Safety-Critical（認証）隣接**。実装には明示的な「修正しろ」承認が必要 |
| U0-B2 | 同エラーが同時に3枚も積み上がる | `CharacterPanel.tsx` L40–58 が eKYC/soul の2 fetch で同一文言のトーストを出し、React StrictMode の二重マウントで倍加。失敗の区別も再試行手段もない | エラートーストの重複抑止（同一メッセージのデデュープ）+ CharacterPanel は「読み込み失敗」をパネル内表示+再試行ボタンに変更（U5-1 のパターン適用対象に追加） | フロントのみ |
| U0-B3 | eKYC ステータス確認のたびに Stripe セッションを新規作成している | `get_ekyc_status_handler` が status 確認なのに `create_verification_session` を呼ぶ（`STRIPE_API_KEY` 設定時は毎回実 API 呼び出し = 遅延・失敗要因） | status 確認とセッション作成の分離（`/api/v1/ekyc/session` は既に別在: `router.rs` L574–584）。status ハンドラから session 作成を除去 | Safety-Critical 隣接のため要承認 |
| U0-B4 | HEX/rgba 直書きの回帰（U-002 違反） | `python3 scripts/test_ui_hex_violations.py` が **RED**: `CoinChip.tsx` に rgba 直書き2件（2026-07-06 の D-1 残高統合実装で混入。U3-1 完了後の新規リグレッション） | rgba 2件を `tokens.css` の var() へ置換 | フロントのみ・サブエージェント委譲可 |

### 12.2 U2 の完了判定の訂正（v6 監査）

実コード確認（2026-07-07）の結果、U2 の実装状況は以下の通り:

- **完了**: viewMode 2値化（`isVisible()` は simple/cockpit の2配列）、一次ラベルの日常語化（`nav.*` 値の置換）
- **未完**: **U2-3 のサイドバー5グループ再編**。現状は `nav.section.synergyHub`（「ホーム」）に16項目、`nav.section.control`（「ツール」）に10項目の**2グループ・順序に設計意図なし**（`App.tsx` L485–707）
- **効果不十分**: U2-5 のラベルは平易語になったが、**互いに区別がつかない**（下表）。「ラベルを易しくする」だけでは「どれを押せば何が起きるか」は伝わらない

**現行ラベルの混同ペア（実地レビューで露呈した問題）**:

| 混同グループ | 現行ラベル | 実体 |
|---|---|---|
| 記録系3画面 | 「きろく」(karma) /「行動の記録」(audit) /「利用状況」(prompt-stats) | Karma年代記 / 監査ログ / プロンプト統計 |
| 状態系2画面 | 「ようすを見る」(dashboard) /「健康状態」(status-page) | Biotope総合ダッシュボード / システム稼働状態 |
| つながり系2画面 | 「つながり」(graph) /「AI同士の対話」(commune) | 共鳴マップ / P2P Commune |
| 意味不明ラベル | 「原因をたどる」(causal) /「デモ」(demo) /「ふやす」(nurture) /「整える」(settings) | 因果トレース / 自律デモ / Nurture経済 / 設定 |
| 二重ナビ | サイドバー「ホーム」内の HomePage にさらに home/shop/world/settings の4タブ | サイドバーと同名概念が入れ子（`HomePage.tsx` L58） |

### 12.3 Phase U6: 情報設計の確定 — メニュー順序・命名規約・画面自己説明（工数: 中 / U0-B 後）

**設計原則（v6 追加）**:
1. **並び順は「利用頻度 × 習熟段階」**: 毎日使う（話す・見る）→ 育てる → 広げる → 守る・設定 の順。アルファベットや実装順ではなく、ユーザーの一日の動線に沿う。
2. **ラベルは「動詞的ひらがな」をやめ「名詞（機能名）+サブテキスト」に**: 「ようすを見る」のような曖昧動詞は識別子として機能しない。一次ラベルは短い名詞、NavItem に1行サブテキスト（または tooltip）で「何ができるか」を必ず添える。
3. **1画面1責務・重複統合**: 区別できない画面は名前を変えるのではなく統合するか、親子関係（タブ）にする。
4. **全画面に自己説明ヘッダ**: 画面上部に「この画面でできること」1行説明を常設（i18n `page.*.description` を新設）。

| ID | 内容 | 対象 | 検証 |
|---|---|---|---|
| U6-1 | **サイドバー5グループ再編+順序確定**（下表。U2-3 の未完部分を確定仕様で実施）。`nav.section.*` キーを5個に拡張 | `App.tsx` L485–707 / `i18n/ja.json`・`en.json` | 全項目が設計表の順に表示・Jest スナップショット更新 |
| U6-2 | **ラベル全面改定**（下表）: 名詞化+混同ペア解消。`nav.*` は値のみ変更（キー不変 = テスト影響最小、v4 §4.5 の原則踏襲） | `i18n/ja.json` / `en.json` | 混同ペア表の全行が別名になる。i18n パリティテスト PASS |
| U6-3 | **NavItem サブテキスト対応**: `NavItem` に `description?: string` を追加し、ホバー tooltip + サイドバー幅に応じた2行表示。`nav.desc.*` キー新設（26項目） | `App.tsx` L876（NavItem）/ i18n | 全 NavItem に説明が付く |
| U6-4 | **画面自己説明ヘッダ**: 各画面 `<h2>` 直下に1行説明（`page.desc.*`）。まず cockpit 26画面に一括適用 | `App.tsx` のタイトル描画部 + i18n | 全画面に説明表示 |
| U6-5 | **記録系3画面の統合**: 「きろく」「行動の記録」「利用状況」を単一「アクティビティ」画面のタブ（タイムライン / 監査ログ / 使用量）に統合。ルーティングは activeTab 互換を維持（旧値→新画面+タブへ写像）。**A2UI ナビ whitelist（`lib/a2uiTabs.ts` L9–37）と backend `validator.rs` L540（`navigate:audit` 使用例）にも写像を反映**（§13 Gate 3） | `Timeline.tsx` / `DiagnosticsHistory.tsx` / `PromptStatsView.tsx` + `App.tsx` / `lib/a2uiTabs.ts` / `libs/infrastructure/src/a2ui/validator.rs` | 3ラベルがサイドバーから1つに減る。旧 activeTab 値・`navigate:audit` でも到達可能 |
| U6-6 | **HomePage 二重ナビの解消**: HomePage 内 settings タブは Cockpit の設定画面へ誘導（重複実装を段階廃止）。world タブは「ワールド」サイドバー項目と統合方針を確定（simple モードの唯一の入口として残すのは可） | `home/HomePage.tsx` | サイドバーと HomePage で同名異物がなくなる |
| U6-7 | **demo の降格**: 「デモ」をサイドバー常設から外し、ホームの「はじめての方へ」カード（初回のみ）+ 設定内へ移動 | `App.tsx` / `home/HomePage.tsx` | cockpit 既定表示が1項目減る |
| U6-8 | **スタイル負債 第2波（おしゃれ化の実弾）**: (1) インライン style 最多5ファイル（`AgentConsole` 93 / `ImmuneSystem` 86 / `StatusPage` 74 / `SettingsPage` 69 / `SetupWizard` 66。components/ 全体で1,444箇所）を共通 CSS クラス + `components/ui/` プリミティブへ段階移行。**未作成プリミティブ Card / StatCard / SectionHeader（U3-4 の残り3/6）をここで新設** (2) **レスポンシブの実適用**: `--bp-*` トークンは定義済みだが `@media` は App.css L914 のドロワー1箇所+TSX 埋込4箇所のみ。しかも `var(--bp-*)` 未参照（リテラル 768px 直書き）。375px 対応を主要画面に展開 (3) **HEX/rgba の実残債 71件の解消**: `test_ui_hex_violations.py` は `.css` ファイルをスキャンしないため見逃しあり — 最悪は `WorkflowBuilder.css` の46件。スクリプトのスコープ拡張（.css 含む）+ 全置換 (4) `animations.css` 死活整理の残り（`.ani-slide-up` / `glowPulse` / `hudPulse` の3定義が使用0件） (5) **DESIGN.md ⇔ tokens.css の SSOT 再確立**: 5トークンの実乖離（font-main/font-display/bg-primary/bg-dark-sidebar/accent-emerald — §14 Gate 1）を、DESIGN.md を現行 tokens.css の実値へ更新する方向で解消。それまで `npm run sync:tokens` の実行禁止（UI 色の巻き戻りリスク） | 上記5ファイル / `WorkflowBuilder.css` / `App.css` / `styles/` / `DESIGN.md` / `scripts/test_ui_hex_violations.py` | インライン style 件数を計測して対象5ファイルで半減。拡張後の hex スクリプト GREEN（71→0）。375px で横スクロールなし。`git diff --exit-code src/styles/tokens.css`（sync 後に差分ゼロ） |

**U6-1 確定版サイドバー（cockpit・上から順に）**

| グループ（`nav.section.*`） | 項目（順序どおり） | 対応 activeTab |
|---|---|---|
| 1. ホーム | ホーム / AIとはなす /（agency モード時のみ）法人向け | home-v2, agent, agency |
| 2. そだてる | ワールド（Biome） / AI学習 / スキル / 知識ベース / AIの表現 | biome, lora, vault, cortex, expressions |
| 3. ようすを見る | ダッシュボード / アクティビティ（U6-5 統合後） / 共鳴マップ / 因果トレース / システム状態 | dashboard, karma(統合), graph, causal, status-page |
| 4. ひろげる | コイン・そだて経済 / ボイスショップ / SNS承認 / SEO / AI同士の対話 / ワークフロー / 外部ツール（MCP） / ファイル | nurture, store, buzz-approval, seo-pulse, commune, workflow-builder, mcp-dashboard, artifacts |
| 5. まもる・整える | セキュリティ / ルール / 設定 | immune, ban-dashboard, settings |

（simple モードは現行4項目 = home-v2 / agent / artifacts / settings を維持）

**U6-2 ラベル改定表（ja。en は同時改定）**

| activeTab | 現行 | 改定案（一次ラベル） | サブテキスト例（`nav.desc.*`） |
|---|---|---|---|
| dashboard | ようすを見る | ダッシュボード | AIの状態をひと目で確認 |
| karma | きろく | アクティビティ | AIの行動履歴とタイムライン |
| audit | 行動の記録 | （U6-5でアクティビティ内タブ「監査ログ」へ） | 操作の監査記録 |
| prompt-stats | 利用状況 | （U6-5でアクティビティ内タブ「使用量」へ） | プロンプト使用量の統計 |
| graph | つながり | 共鳴マップ | 知識と話題のつながりを図で見る |
| causal | 原因をたどる | 因果トレース | 「なぜこの行動をしたか」を追跡 |
| status-page | 健康状態 | システム状態 | サーバーと機能の稼働状況 |
| commune | AI同士の対話 | AIコミュニティ | 他のAIとの対話・共同作業 |
| nurture | ふやす | コイン・そだて | AiomeCoinの残高とそだて経済 |
| store | ボイス | ボイスショップ | 声の購入とギフト |
| seo-pulse | SEO | SEO発信 | 検索向け記事の自動発信 |
| buzz-approval | 承認待ち | SNS承認 | AIが作ったSNS投稿の承認 |
| workflow-builder | 仕事の流れ | ワークフロー | 自動化フローの作成 |
| mcp-dashboard | 外部ツール | 外部ツール（MCP） | 接続中の外部ツール管理 |
| immune | 安全 | セキュリティ | 脅威検知と防御の状況 |
| settings | 整える | 設定 | アカウントと動作の設定 |

（記載のない項目は現行ラベル維持。最終文言は `MESSAGING.md`（SSOT）と突き合わせて確定する）

### 12.4 実行順序と委譲（v6）

> **実装状況（2026-07-07 更新）**: U0-B 全件 + U6-1〜8 + **U6-9** 実装完了。検証: tsc / Jest 404 PASS / hex ゲート GREEN。U6-9: 「コインとポイント」改名、DioramaView データ画面非表示、NurtureDashboard i18n 化。

```
U0-B（バグ即修正 — U0-B1/B3 は人間承認必須、U0-B4 は委譲可）
  → ラベル承認ゲート（§13 Gate 4-1: 確定ラベル対照表のユーザー承認）
  → 第1弾: U6-1/2/3/4（グルーピング・命名・自己説明 — E2E 追随修正を同一 PR に含める）
  → 第2弾: U6-5/6/7（画面統合・二重ナビ解消 — a2uiTabs/validator.rs の写像込み）
  → 第3弾: U6-8（スタイル負債第2波 — 第1・2弾と並行可）
```

| 作業 | 担当 |
|---|---|
| U0-B4（rgba 置換）・U6-2 ラベル置換・U6-3/U6-4 の i18n キー一括追加・E2E ラベル追随・検証（lint/Jest/i18n パリティ/hex） | 低トークンサブエージェント |
| U0-B1/B3（認証隣接）・U6-1 グループ構造・U6-5 統合設計（a2uiTabs 写像含む） | メインエージェント（U0-B1/B3 は人間承認後） |
| U6-6/U6-7・U6-8 のインライン style 移行 | メイン設計 → サブエージェント横展開 |

### 12.5 検証基準（v6 追加分）

| 項目 | 成功基準 |
|---|---|
| U0-B1 | ログイン→ホーム表示でエラートースト0件。`curl`（JWT付き）で `/api/v1/ekyc/status` が 200。**Negative Test**: JWT なし→401、不正 JWT→401 |
| U0-B4 | `python3 scripts/test_ui_hex_violations.py` GREEN（0 violations） |
| U6-1/2 | サイドバーが5グループ・確定順で表示。混同ペア表の全行が別名。`npm run lint && npm test` PASS。**E2E 7 spec の英語ラベル hasText を追随修正 + nav へ data-testid 付与**（§13 Gate 3） |
| U6-5 | 旧 activeTab 値（karma/audit/prompt-stats）でアクセスしても新画面の該当タブが開く。`navigate:audit`（A2UI）でも到達可能。`cargo test -p infrastructure a2ui` PASS |
| U6-8 | インライン style 件数（rg 計測）が対象5ファイルで半減。375px 幅で横スクロールなし |
| 全体 | ユーザー実機レビューで「どのメニューが何か説明なしで分かる」ことの再判定 |

---

## 13. /perfect-plan 第5周検証結果（2026-07-07・v6 反映済み）— 実コードベース照合

**検証体制**: メインエージェントが Gate 1–5 を実コード照合で実施（i18n キー・A2UI whitelist・E2E ラベル依存・hex ゲート実走・インライン style 計測）。サイドバー構造インベントリはサブエージェント委譲済み（§12 の入力）。

### Gate 1: 構造スキャン — ⚠️→✅（PATCH 適用済み）

- ⚠️→修正: **v6 冒頭のステータス行が事実誤認**だった。OPEN.md OP-066 により U0–U5 は 2026-07-05 R1 完了（Jest 392 PASS / hex 0 / deep-scan 0）。`components/ui/`（EmptyState / LoadingState / LockedOverlay / Modal）・`useSubscriptionStatus.tsx`・A2UI `navigate:` action はすべて**実装済みで実在確認**。ステータス行を修正済み
- ⚠️→発見: **hex ゲートが RED に回帰**（`CoinChip.tsx` rgba 2件、2026-07-06 D-1 実装で混入）→ U0-B4 として起票
- ⚠️→発見: **hex スクリプトに盲点** — `.css` ファイルをスキャンしないため、実残債は **71件**（最悪 `WorkflowBuilder.css` 46件、`ProUpgradeModal.tsx` 3件等）。「hex 0」の完了報告はスクリプトのスコープ内でのみ真。スクリプト拡張+全置換を U6-8 に組込
- ⚠️→発見: U3-4 の6プリミティブは **3/6 のみ実装**（LockedOverlay / LoadingState / EmptyState あり、Card / StatCard / SectionHeader なし）→ 残り3つは U6-8 で新設
- ✅ `nav.desc.*` / `page.desc.*` キーは ja.json に不存在（新設で衝突なし。`setup.*.desc` が同一パターンの先例）
- ✅ U6-5 の対象コンポーネント実在確認: audit=`DiagnosticsHistory.tsx`（「AuditLog」ではない — §12.3 の表記を修正済み）、karma=`Timeline.tsx`、prompt-stats=`PromptStatsView.tsx`
- ✅ NavItem は `App.tsx` L876 の単一定義（props: icon/label/active/onClick）。`description` prop 追加に衝突なし

### Gate 2: 要件カバレッジ（NURTURE クロスチェック） — ✅

- §2 経済台帳 / §6 VRM / §7 A2C / §8 P2P: 影響なし（U6 は表示・情報設計のみ。commune はラベル変更のみで Federation 不介入）
- §3 MCP: 影響なし
- §4 セキュリティ: **U0-B1/B3 が認証ミドルウェア隣接** — 人間承認ゲートを §12.1 に明記済み。U6-5 は A2UI ナビ whitelist（XSS/不正遷移ガード）に波及するため、旧値写像を whitelist 側にも実装（緩和ではなく等価変換であること）
- §5 法的リスク: ラベル変更・グルーピングのみ。ダークパターン要素なし

### Gate 3: 依存関係 & 波及 — ⚠️→✅（§12.3 に反映済み）

| 波及先 | 内容 | 対応 |
|---|---|---|
| `src/lib/a2uiTabs.ts` L9–37 | `A2UI_NAV_TABS` に karma/audit/prompt-stats が登録済み。U6-5 統合で不整合化 | U6-5 対象に追加済み（旧値は写像として維持） |
| `libs/infrastructure/src/a2ui/validator.rs` L540 | `navigate:audit` を使用（Rust 側）。フロント側も `A2uiRenderer.test.tsx` L113/L123・`e2e/a2ui.spec.ts` L12 が `navigate:audit` / `tab: 'audit'` を使用 | U6-5 対象に追加済み。`cargo test -p infrastructure a2ui` + `npm test -- A2uiRenderer` を DoD に追加 |
| E2E 7 spec が**英語ラベル hasText でナビを特定** | `a2ui.spec.ts`（'Agent Console'）/ `home_v2.spec.ts`（'Home v2(Beta)'）/ `demo.spec.ts`（'Synergy Demo'）/ `cortex_view.spec.ts` / `ui_fixes.spec.ts`（'AI Chat', 'Voice'）/ `screencast.spec.ts`（日英正規表現）/ `promo-clips.spec.ts`（`navigateToTab` ヘルパー） → U6-2 の en.json 改定で広範囲に破損 | U6-1/2 の DoD に **E2E 7 spec の追随修正**を明記（恒久対策: nav に data-testid 付与を U6-1 に含める） |
| Jest | nav ラベルは**キー名アサートのみ**（`'nav.biotope'` 等 — `t: (k) => k` モック）。日本語ラベル値のアサートなし | 値のみ変更なら Jest 影響ゼロ（v4 §4.5 の前提を実測で再確認） |
| activeTab 永続化 | localStorage / URL への activeTab 保存は**不存在**（rg 0件） | 旧値写像は a2ui-navigate 経路のみで足りる |

### Gate 4: 悪魔の弁護人

1. **最悪のシナリオ**: ラベル改定が3度目の空振り（v5「日常語化」→実地で不評→v6「名詞化」も外す）。→ **対策: U6-2 の実装前に「確定ラベル対照表」（§12.3 の表）をユーザーに提示し承認を得る承認ゲートを必須化**。文言の最終決定権はユーザーにあり、エージェントは実装のみ。
2. **見落とされた前提**: 「26画面を全部残す」前提が正しいとは限らない。メニューが分かりにくい根因は**数**でもある。→ U6-5/6/7 で cockpit 既定 26→23 項目に削減。それでも多い場合の次の一手（「その他」折りたたみ）は今回**不採用**と明示（隠すと到達不能バグ S-3 の再発リスク）。
3. **やらないメリット**: なし（ユーザーが明示的に改修を要求済み）。ただし**工数大の U6-5/6（画面統合）は第2弾に分離可能** — U0-B + U6-1/2/3/4（グルーピング・命名・自己説明）だけで体感は大きく改善するため、トークン・時間予算が厳しい場合は統合系を後続 PR に分割する。

### Gate 5: 実行順序 — ✅

- `U0-B（要承認・即修正）→ ラベル承認ゲート → U6-1/2/3/4（第1弾）→ U6-5/6/7（第2弾）→ U6-8（第3弾・並行可）` に確定。U6-2 の audit/prompt-stats ラベルは U6-5 の統合決定に従属（§12.3 で既に整理済み・二度手間なし）
- E2E 追随修正は U6-2 と**同一 PR**（分離すると main が RED になる）
- OPEN.md の既知ギャップとの競合なし（OP-002 Biome 目視は独立。OP-066 の残項目 U2-4 variant 統合は U6 と非干渉）

### 判定: **✅ PASS（v6 PATCH 適用済み）**

- 事実誤認2件（ステータス行・audit コンポーネント名）と波及漏れ3件（a2uiTabs / validator.rs / E2E ラベル依存）を計画に反映済み
- hex 回帰（U0-B4）を新規発見・起票
- 実装開始の前提条件: **(1) U0-B1/B3 の人間承認（認証隣接） (2) U6-2 確定ラベル対照表のユーザー承認**

---

## 14. /perfect-plan 第6周検証結果（2026-07-07）— §12/§13 自体の実在照合・実装可能性の最終確認

**検証体制**: §12/§13 に記載した行番号・前提・DoD コマンドをメインエージェントが実測（`lint:design` 実走・CSS 実読・i18n キー全数照合）。補助照合はサブエージェント委譲。

### Gate 1: 構造スキャン — ✅（U6 の土台は全て実在。新発見2件を注記）

| 検証項目 | 結果 |
|---|---|
| U6-4 の挿入点 | ✅ 全27タブのヘッダタイトルは `App.tsx` L764–790 の**単一 `<motion.h2>`** に集約されており、`page.*` キーも27個全て実在（ja.json）。説明の挿入点は1箇所+キー27個追加のみ |
| U6-3 の CSS 前提 | ✅ `.nav-item`（`App.css` L221–240）は **padding ベースで固定 height なし**。ただしラベル span は `white-space: nowrap`（L756–758）のため、2行表示にはマークアップ（説明用 span 追加）+ CSS の両方の小修正が必要。ブロッカーなし |
| U2-1 / U4-0 の完了 | ✅ `migrateViewMode`（`useViewMode.ts` L14–18、既定 cockpit）と `feature_flag.a2ui_generative_ui` トグル（`SettingsPage.tsx` L530–533）は実装済み |
| ⚠️ U3-2 の乖離は**部分未解消** | `--accent-rose`（#f472b6）と `--layout-sidebar-width`（280px）は一致。しかし**5トークンで実乖離が残存**: `--font-main` / `--font-display`（DESIGN.md は 'Artemis Inter' を含むが tokens.css は削除済み）、`--bg-primary`（#0b0b0f vs #0b0d14）、`--bg-dark-sidebar`、`--accent-emerald`（#10b981 vs #34d399）。tokens.css ヘッダは「Auto-generated from DESIGN.md. DO NOT EDIT DIRECTLY」と宣言しているのに直接編集されており、**SSOT の方向が崩壊**（`npm run sync:tokens` を実行すると現行 UI の色が巻き戻るリスク）。→ **U6-8 に「DESIGN.md を現行 tokens.css の実値に合わせて再生成し、SSOT 方向を再確立する」を追加** |
| ⚠️ 新発見1: `lint:design` が RED | `npm run lint:design`（`@google/design.md` linter）が **99 errors**。ただし内容は「rgba()/var() 値を hex でないためエラー扱い」という**リンタ側スキーマの厳格さによる偽陽性が大半**。Appendix A の U3 DoD からは**当面除外**し、DESIGN.md のスキーマ適合は独立タスクとする（U6 のブロッカーではない） |
| ⚠️ 新発見2: `data-testid` ゼロ | `App.tsx` に data-testid は **0件**。E2E の `navigateToTab`（`promo-clips.spec.ts` L25–31）も `.nav-item` + hasText 正規表現でナビを特定 → §13 の「data-testid 付与」は完全新規作業（既存慣習なし）。U6-1 の作業量に含めて見積もる |

### Gate 2: 要件カバレッジ — ✅（第5周から変更なし。U6 は表示層のみ、認証隣接は承認ゲート済み）

### Gate 3: 依存関係 — ✅（第5周で全数列挙済み。第6周の実測で追加の波及先は発見されず）

- E2E のラベル依存は `navigateToTab` ヘルパー1関数+個別 spec の hasText に集約されており、**ヘルパー修正1箇所で promo-clips 系はまとめて追随可能**（波及コストは §13 の見積りより小さい）

### Gate 4: 悪魔の弁護人（第6周の意地悪な問い）

1. **「検証6周は過剰では？ 分析麻痺に陥っていないか？」** — 正しい懸念。第6周の新発見は「lint:design の偽陽性」「data-testid 不在」の2件のみで**収穫逓減が明確**。本計画はこれ以上の検証周回を行わず、**2つの承認（U0-B1/B3・ラベル対照表）が得られ次第、実装に着手する**ことをここに固定する。
2. **「`lint:design` RED を放置してよいのか？」** — U6 のスコープでは可（UI 実装と独立した文書リンタの問題）。ただし Appendix A に「U3 DoD コマンド」として記載済みだったため、そのまま実行すると DoD 不能になる — 本周で DoD から除外済み。
3. **「サブエージェント委譲でトークンは本当に節約できているか？」** — 探索系は有効（棚卸し3件で本体コンテキストを消費せず）。ただし応答待ちが長い（10分超）ため、**実装フェーズでは「機械的置換の実行」のみ委譲し、検証はメインの直接実測を優先**する運用に調整する。

### Gate 5: 実行順序 — ✅

- §12.4 の3弾構成に変更なし。第6周の実測により第1弾（U6-1/2/3/4）の不確実性は解消（挿入点1箇所・キー27個・CSS ブロッカーなし・E2E はヘルパー集約）
- 実装順の最適化1件: **data-testid 付与（新規）を U6-1 の最初に行う** → 以降のラベル変更で E2E が壊れない状態を先に作る（E2E 追随を1回で済ませる）

### 判定: **✅ PASS（第6周・実装移行可）** — 検証は本周で打ち切り。残る前提は人間承認2件のみ

