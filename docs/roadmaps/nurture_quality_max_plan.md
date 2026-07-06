# Aiome + Nurture 品質最大化計画（NURTURE 要件乖離の解消）v4

> **作成日**: 2026-07-06 ｜ **改訂**: v4 = /perfect-plan 3巡目（全記載アンカーを実コードと照合し、不一致5件を修正）
> **ステータス**: 完了（2026-07-07）。Phase A/B/C/D + E-0 + A-6 + /reflexion + D-1 実装済み。`cargo test --workspace` PASS（wiremock 系は network 必須）。
> **原則**: ADR-031 と同じ「安定性 > リスキーな大規模改修」。新機構の発明より既存機構の拡張を優先。

---

## 0. 前提事実（全て実コード確認済み。v4 で修正したアンカーに ◆）

- `withdraw_points`（`commercial/libs/nurture-infra/src/economy/bridge/commerce_impl.rs` L621–705）の変換先は AiomeCoin のみ
- **transfer の全経路はユーザー起点**: `POST /api/v1/commerce/transfer`（api-server、Authenticated+eKYC）→ `stripe/mod.rs:930` → `POST /internal/transfer`（nurture）→ bridge `transfer()`。Gig 分配等のシステム内部送金は grep 上存在しない
- ◆ bridge `transfer()` の戻りエラー型は **`AiomeError`**（L807–817）。`AiomeError` に `PolicyViolation` バリアントは**存在しない** → 自己送金チェックと同じ `AiomeError::Validation`（400、error.rs L128–130）を使用
- ◆ `EconomyPolicy`（policy.rs L16–33）は**構造体レベルで `#[serde(default)]` 付与済み** + `Default` は**手書き impl**（L41–57）。テスト側の構造体リテラルは全て `..EconomyPolicy::default()` パターン（interceptor.rs 8箇所ほか）→ フィールド追加のコンパイル波及は手書き Default のみ
- ◆ buy ハンドラの実パスは **`commercial/apps/nurture-api/src/mcp_tools/buy.rs`**（`routes/buy.rs` は存在しない）。`handle_buy` は `Result<BuyResponse, NurtureError>`（L19）、`NurtureError::PolicyViolation(String)` は commerce-protocol/error.rs L30–31 に実在。CSAM チェックは L94–110、`state.ledger.get_balance` は L122
- `nurture_licenses` に UNIQUE(owner,asset) なし。`revoked_at` + `revoke_license()`（license.rs L193）あり。**`issue_license` は `item.drm_enabled` のときのみ実行**（bridge/mod.rs L581–617）→ 非 DRM アイテムはライセンス照会で再購入検出できない
- ◆ `nurture_ledger.asset_id` は `20260506000000_ledger_asset_id.sql` の ALTER で追加（**NULL 許容**、旧行は NULL）。`debit_account`/`created_at` は init migration に実在
- `ScanVerdict` は `Safe` / `Rejected { reason, layer, requires_ncmec_report }` の2種（csam/mod.rs L29–37）
- ◆ BoneChecker の `is_humanoid` は **metadata JSON の自己申告フィールド**（bone_check.rs L66–69、`unwrap_or(true)`）。`false` 申告で頭身チェックを完全バイパス可能 = **攻撃ベクトル確定**。頭身抽出不能時は Safe 通過（L160–163）。比較は `r >= threshold` で Reject（L135–150）。0.20 依存テストは `test_borderline_exactly_at_threshold` / `test_reject_child_proportion` / `test_safe_adult_proportion` / `test_non_humanoid_vrm_bypasses_bone_check` の4本
- `spent_this_month` は `nurture_wallets` テーブル列。`get_balance` が返す `CoinWallet` に載っており、interceptor は wallet 引数から読む（interceptor.rs L216–230）
- ◆ `EconomicContext` 構築箇所は **`apps/api-server/src/` 配下の4箇所**: `stream.rs` L199–203 / `agent_engine.rs` L155–159 / `routes/watchtower.rs` L245–249 / `system_instructions.rs` L436（テスト）。`libs/core/src/samsara/` 配下ではない
- ◆ `CommerceEngine` impl は7型: `NurtureCommerceBridge` / `StripeCommerceEngine` / `MockCommerceEngine`（2箇所）/ `PolarCommerceEngine` / `MockCommerceEngineForMarketplace` / `StubCommerceEngine` → トレイト拡張は**デフォルト実装必須**
- ◆ api-server の自律購入ハンドラ `execute_purchase`（commerce.rs L284–294）は `execute_autonomous_purchase` の戻り **`tx_id: String` のみ**を受け取る（`BuyResponse` はデシリアライズしない）→ bonus 額はこの層に届かない
- `CoreEvent::CommerceEvent` broadcast 既存例: `routes/commerce.rs` L687–694（`state.event_sender.as_opt()` → `sender.send(...)`）
- `audit_hash`（`MerkleAudit::calculate`、merkle.rs L22–38）の対象は7項目のみ。**memo 列追加はハッシュに影響しない**
- ◆ axum の同一ハンドラ2パス登録の既存例: `router.rs` L716–717（`/api/health` と `/health`）
- ◆ タブ遷移: `App.tsx` L99 `useState("home-v2")`、経済タブ識別子は `"nurture"`。**プログラム遷移は `a2ui-navigate` CustomEvent**（App.tsx L142–147 で受信、`lib/a2uiTabs.ts` L24–25 のホワイトリストに `'nurture'` 含む。発火例: A2uiRenderer.tsx L497–501）
- ◆ 「Lifetime Withdrawn」（NurtureDashboard.tsx L287）は**ハードコード**（i18n 未使用）
- ◆ 残高 fetch は `useCoinBalance` hook + `CoinBalanceProvider`（Context）に集約。`VoiceStore` / `A2uiRenderer` / `NurtureDashboard` / `CoinChip` が共有（60 秒キャッシュ + `refetch()`）。i18n parity テストは `src/i18n/i18n.test.ts`
- `StoryFlow.tsx` L78–79 が commerce_event を `TimelineEntry { type: 'system' }` にマップ。`TimelineEntry.type` は `FlowCardType`（union 追加で波及完結）
- `CreateCheckoutSessionRequest` に metadata なし（generated.ts L1858–1863）。購入は `POST /api/v1/commerce/purchase/{agentId}`（`metadata.context_layer` 対応済み、VoiceStore L126–133）
- ヘッダースコープに `agentId` あり（App.tsx L344 `useAgentIdentity()`）。FE 型再生成は `npm run generate-types`（package.json L12）
- `EconomicContext`（aiome-contracts L204–211）は balance/spent_today/daily_limit のみ。`LedgerEntry`/`TransactionRecord` に memo なし
- マイグレーション最新は `20260706000000`。本計画の `20260707…` 系ファイル名に衝突なし

---

## Phase A: 現金化残骸の完全除去【P0・法的防壁】

### A-1: ADR-052「法定通貨ペイアウトのスコープ除外」作成
- **ファイル**: `docs/decisions/052-fiat-payout-scope-exclusion.md`（新規）
- **内容**: (1) 除外理由 = 資金決済法の為替取引該当・払戻規制リスク、(2) CP→AiomeCoin 変換のみを正、(3) `TremendousGiftEngine` は A2C 専用（CP 報酬変換への転用禁止）、(4) `REMAINING_TASKS.md` L90–91 / `UNCERTAINTY_BREAKTHROUGH.md` L136–163 の CP→ギフト案を supersede、(5) B-1 の P2P 送金ブロックの根拠
- **完了条件**: Accepted で存在し A-6 から参照されている

### A-2: `nurture_payout_requests` テーブルの廃止 🔐承認済み（DROP）
1. **事前検査**: DROP 前に `SELECT COUNT(*) FROM nurture_payout_requests` を dev/本番双方で実行し、0 件でなければ結果を記録してから進む（データ消失の証跡）
2. 新規マイグレーション（既存ファイルは sqlx チェックサム保護のため不変更）:
   - `commercial/migrations/sqlite/20260707000000_drop_nurture_payout_requests.sql` / `commercial/migrations/postgres/20260707000000_drop_nurture_payout_requests.sql` — `DROP TABLE IF EXISTS nurture_payout_requests;`
3. `commercial/apps/nurture-api/src/routes/internal/gdpr.rs` `forget_actor()` の DELETE ブロック（L175–190）+ L157 コメント「5. サブスクリプションと出金申請のパージ」の出金部分を削除（アンカー実在確認済み）
4. `commercial/apps/nurture-api/tests/internal_routes_test.rs` `test_forget_actor_purges_pii_and_physical_assets` から L234–236 INSERT / L328–338 SELECT 検証を削除（アンカー実在確認済み）
- **検証**: `cargo test -p nurture-api` PASS + `scripts/verify-production-postgres.sh`。Negative: `rg -l "nurture_payout_requests" --glob '!*/migrations/*'` = 0 件

### A-3: 契約トレイト doc 修正
- `libs/aiome-contracts/src/commerce.rs` L120–125: 「コインまたは法定通貨として出金」→「クリエイターポイントを AiomeCoin に交換する（エコシステム内変換。法定通貨出金は ADR-052 によりスコープ外）」

### A-4: ユーザー向け法務文言の改訂 👤Human レビュー必須
- `docs/legal/TERMS_OF_SERVICE.md` L25: 「収益の出金や特定の有償サービス提供」→「クリエイターポイントの AiomeCoin への交換や特定の有償サービス提供」
- `docs/landing/src/components/LegalPages.tsx` L107: 「出金や有償サービス提供」→「ポイント交換や有償サービス提供」
- 提案（Human 決定）: 両文書の「85% がクリエイターに分配」に「（エコシステム内通貨で分配）」を補足

### A-5: API 命名の是正 🔐承認済み（公開 API 変更）
1. `apps/api-server/src/routes/commerce.rs` L154–189: utoipa path を `/api/v1/commerce/convert-points` へ、summary「Convert creator points to AiomeCoin (in-ecosystem)」
2. `apps/api-server/src/router.rs` L81–84（`/withdraw` 登録行、実在確認済み）: 新パス登録 + 旧 `/withdraw` を alias で1リリース維持（sunset 予定日コメント）。**2パス登録の既存例 = `router.rs` L716–717 の `/api/health` + `/health` をそのまま踏襲**
3. `cargo test -p api-server test_openapi_schema_generation` → `docs/openapi.json` 再生成
4. `(cd apps/management-console && npm run generate-types)` → `generated.ts` 再生成（UI から `/withdraw` は未使用、影響ゼロ確認済み）

### A-6: タスク文書のスコープ外宣言（削除ではなく注記）
- `REMAINING_TASKS.md` L90–91、`commercial/docs/UNCERTAINTY_BREAKTHROUGH.md` L136–138/L159–163、`commercial/docs/LEGAL_TAX_REGULATORY_STRATEGY.md` L201 に「⛔ ADR-052 によりスコープ外（CP の外部ギフト/現金変換は実装しない。Tremendous は A2C 専用）」を注記

### A-7: UI 文言
- `NurtureDashboard.tsx` L287「Lifetime Withdrawn」（ハードコード確認済み）→ i18n キー `nurture.convertedToCoin`（ja: 「コイン交換済み」/ en: "Converted to Coin"）に置換。`npm test -- --run src/i18n/i18n.test.ts` PASS（en/ja parity）

---

## Phase B: 法的防壁・AI 暴走対策の強化【P0〜P1】

### B-1: ユーザー間コイン送金のブロック 🔐承認済み（経済処理）
- 全 transfer 経路がユーザー起点と確定済みのため、bridge 層の一点ガードで完結（`transfer_internal` 分離は不要）
- **実装**:
  1. `commercial/libs/nurture-core/src/policy.rs`: `EconomyPolicy` に `pub allow_p2p_transfer: bool` 追加（構造体レベル `#[serde(default)]` 済みのためフィールド属性は不要）。**手書き `impl Default`（L41–57）に `allow_p2p_transfer: false` を追記（必須。忘れるとコンパイルエラー）**。テスト側リテラルは全て `..EconomyPolicy::default()` のため波及なし
  2. `commerce_impl.rs` `transfer()` の自己送金チェック（L813–817）直後にポリシー判定: `false` なら `AiomeError::Validation { reason: "P2P coin transfer is disabled by policy (ADR-052): user-to-user coin transfer is blocked to comply with prepaid payment instrument regulations".to_string() }`（既存の自己送金エラーと同バリアント。`AiomeError` に PolicyViolation は存在しない）
  3. 公開ハンドラの utoipa doc 更新: `apps/api-server/src/routes/commerce.rs` L192–203（400 応答記載）、`commercial/apps/nurture-api/src/routes/wallet.rs` L113
  4. `commercial/apps/nurture-api/src/routes/internal/misc.rs` `transfer_coins`（L112）はそのまま（bridge ガードで自動的にブロックされる）
- **TDD**（`commercial/libs/nurture-infra/src/economy/bridge/tests.rs`）: `test_p2p_transfer_blocked_by_default`（デフォルトポリシーで Err）/ `test_p2p_transfer_allowed_when_policy_enabled`（`allow_p2p_transfer: true` で従来動作）を先に RED → 実装 → GREEN

### B-2: 同一アイテム再購入の禁止（破産ループ対策） 🔐承認済み（経済処理）
- 非 DRM アイテムはライセンスが発行されないため、ライセンス照会 + ledger 24h ルールの2段構え:
  1. **DRM アイテム**: `commercial/apps/nurture-api/src/mcp_tools/buy.rs`（◆パス訂正。`routes/buy.rs` は存在しない）`handle_buy` の CSAM チェック終了（L110）後・settle 前に `state.license_store.get_license(&req.buyer, &req.item_id)` を呼び、`Some` なら `NurtureError::PolicyViolation("already owned: re-purchase blocked (active license exists)".into())`（バリアント実在確認済み: commerce-protocol/error.rs L30–31）。呼び出しパターンは `asset.rs:24,44` を踏襲
  2. **全アイテム共通（非 DRM 含む）**: `DatabaseEconomyLedger` に `has_recent_purchase(buyer: &ActorId, asset_id: &Uuid, within_hours: u32) -> Result<bool>` を追加。クエリは `nurture_ledger` の `entry_type='Purchase' AND asset_id=? AND debit_account=? AND created_at > (now - 24h)`。**`asset_id` は NULL 許容列**（`20260506000000_ledger_asset_id.sql` で ALTER 追加）のため `asset_id = ?` 比較で旧 NULL 行は自然に除外される。buy.rs で 24h 以内の同一アイテム購入を拒否 — 返金→再購入は 24h 経過で自然許容
  3. **自律購入経路**: `bridge/mod.rs` `execute_purchase_step` の settlement 前（L557 の手前）に同じ2チェックを追加
  4. **DB 防壁**: `20260707000001_licenses_unique_active.sql`（sqlite/postgres）: `CREATE UNIQUE INDEX idx_licenses_owner_asset_active ON nurture_licenses(owner_id, asset_id) WHERE revoked_at IS NULL;`（部分 UNIQUE。通常 UNIQUE は返金→再購入を永久ブロックするため不可）
- **事前検査**: インデックス作成前に `SELECT owner_id, asset_id, COUNT(*) FROM nurture_licenses WHERE revoked_at IS NULL GROUP BY 1,2 HAVING COUNT(*) > 1` で既存重複を確認（重複があれば手動解決してから適用）
- **TDD**: `test_duplicate_purchase_rejected`（DRM）/ `test_rapid_repurchase_rejected_non_drm`（24h ルール）/ `test_repurchase_after_refund_allowed`（`instant_refund` L532 → `revoke_license` → 再購入成功）

### B-3: BoneChecker の fail-closed 化 + 自己申告バイパスの封鎖
- ◆ **攻撃ベクトル確定**: `is_humanoid` は metadata JSON の自己申告フィールド（bone_check.rs L66–69、`unwrap_or(true)`）。出品者が `"is_humanoid": false` を申告するだけで頭身チェックを完全回避できる
- **実装**（条件分岐なしで確定）:
  1. **L66–76 の `is_humanoid == false` 早期 Safe リターンを削除**（自己申告を信用しない）。VrmAvatar は無条件で頭身抽出に進む
  2. L160–163 の「頭身抽出不能 → `Ok(ScanVerdict::Safe)`」を `Ok(ScanVerdict::Rejected { reason: "proportions unverifiable — manual review required".to_string(), layer: "BoneChecker", requires_ncmec_report: false })` に反転（fail-closed）
  3. 適用範囲は既存どおり `kind == "VrmAvatar"` のみ（L57–62、ボイス等への誤 Reject なし — 確認済み）
  - 結果: 非ヒューマノイド VRM（動物型等）は「頭身が抽出できない」経路で Rejected（手動審査行き）になる。自動 Safe の抜け道は消える
- **TDD**: `test_unverifiable_proportions_rejected`（新規）/ `test_non_vrm_assets_still_pass`（ボイス等）/ 既存 `test_non_humanoid_vrm_bypasses_bone_check` は**期待値を Rejected に反転して改名**（`test_self_declared_non_humanoid_no_longer_bypasses`）

### B-4: 頭身閾値の整合（値の統一）
- `bone_check.rs` L27: `CHILD_PROPORTION_THRESHOLD: f64 = 0.20` → `1.0 / 5.5`（≒0.1818）。比較は `r >= threshold` で Reject のため、閾値を下げる = Reject 範囲拡大 = 厳格化で方向は正しい
- doc コメントに `libs/shared/src/csam/proportions.rs` L35（`ratio < 5.5` reject）との相互参照を明記（クレート依存がないため定数共有不可。値+出典コメントで整合）
- ◆ **更新対象テスト（4本、実在確認済み）**: `test_borderline_exactly_at_threshold`（0.20 明示 → 0.1818 に変更）/ `test_reject_child_proportion`（0.25 — 新閾値でも Reject、変更不要）/ `test_safe_adult_proportion`（0.14 — 新閾値でも Safe、変更不要）/ `test_non_humanoid_vrm_bypasses_bone_check`（B-3 で改名・反転済み）

### B-5: 月次上限のプロンプト注入
- `spent_this_month` は `CoinWallet`（`nurture_wallets` 列）に既に載っており、ledger 集計クエリの新設は不要
- **実装**:
  1. `libs/aiome-contracts/src/commerce.rs` L204–211: `EconomicContext` に `#[serde(default)] pub monthly_limit: u64` / `pub spent_this_month: u64`
  2. `CommerceEngine` トレイト（同ファイル L17）に `async fn get_monthly_spend(&self, agent_id: Uuid) -> Result<u64> { Ok(0) }` / `get_monthly_limit`（**デフォルト実装付き** — impl 7型: Nurture/Stripe/Mock×2/Polar/MarketplaceMock/Stub を壊さない）。`NurtureCommerceBridge` のみ override（wallet フィールドを返す）
  3. ◆ 構築箇所4箇所を更新（パス訂正済み）: `apps/api-server/src/stream.rs` L199–203 / `apps/api-server/src/agent_engine.rs` L155–159 / `apps/api-server/src/routes/watchtower.rs` L245–249 / `apps/api-server/src/system_instructions.rs` L436（テスト）
  4. `system_instructions.rs` L80–90 の economy_prompt に「今月の支出: {} / {} コイン (月次上限)」を追記（`monthly_limit == 0` は無制限のため非表示）

---

## Phase C: 要件定義書の正本化【P1・ドキュメント】

- C-1: `docs/specs/NURTURE_REQUIREMENTS_V2.md` 新規作成。冒頭の改訂表:
  | v1 | v2（実態） |
  |---|---|
  | Nemotron-3 + NeMo Guardrails (Docker) | マルチプロバイダ（Ollama デフォルト/Gemini/OpenAI/Claude/LM Studio）+ 独自 Guardrails（BeggingSupervisor/ToolCallReviewerHook/P2pSanitizer） |
  | 中央サーバー = AWS/PostgreSQL | SQLite ローカル + PostgreSQL 本番のハイブリッド（R3 検証済み） |
  | クリエイター報酬 = 法定通貨支払い | CP→AiomeCoin 変換のみ（ADR-052） |
  | ツール名 `search_marketplace`/`buy_item` | `marketplace_search`/`marketplace_buy` |
  | （記載なし） | 実装済み拡張: エスクロー（24h TTL）・冪等性ゲート・Merkle 監査・Federation BFT・ウォッシュトレード防止・SurpriseEngine |
- C-2: `docs/architecture/AIOME_NURTURE_SYNERGY.md` 冒頭に v2 リンク追記

---

## Phase D: 体験ギャップの解消【P1〜P2】

### D-1: ウォレット控えめ表示 + 残高 fetch の共有化（リファクタリング） ✅
1. 新規 `apps/management-console/src/hooks/useCoinBalance.ts`: `GET /api/v1/commerce/balance/{agentId}`（`{ balance: number }`、3箇所とも `typeof data.balance === 'number'` ガード — 確認済み）+ 60 秒キャッシュ + `refetch()`。agentId は `useAgentIdentity()` から。**2026-07-07**: `CoinBalanceProvider`（Context）化で全 UI 同期
2. 既存3箇所の個別 fetch を hook に置換: `VoiceStore.tsx` / `A2uiRenderer.tsx` / `NurtureDashboard.tsx`（完了）
3. 新規 `CoinChip.tsx`（🪙+残高のみの小コンポーネント）を `App.tsx` L793 の `<PlanBadge />` 直前に挿入。◆ クリック遷移は **`window.dispatchEvent(new CustomEvent('a2ui-navigate', { detail: { tab: 'nurture' } }))`**（App.tsx L142–147 の既存リスナーが `setActiveTab("nurture")` を実行。`'nurture'` はホワイトリスト済み — `lib/a2uiTabs.ts` L24–25。発火パターンは A2uiRenderer.tsx L497–501 を踏襲）— prop 渡し不要
- **完了条件**: 残高 fetch が hook 1箇所に集約、Jest（i18n parity 含む）PASS

### D-2: 購入サプライズ演出（SSE 配線）
- ◆ **v4 で伝播経路を確定**: api-server の `execute_purchase`（commerce.rs L284–294）は `execute_autonomous_purchase` の戻り `tx_id: String` のみを受け取り、`BuyResponse` はデシリアライズしない。**bonus 額は api-server 層に届かない**（トレイト戻り値拡張は7 impl に波及するため不採用）
1. `commercial/libs/commerce-protocol/src/mcp_commerce.rs` L42–47: `BuyResponse` に `#[serde(default)] pub surprise_bonus: Option<u64>`。`mcp_tools/buy.rs` L231–265 のボーナス結果を格納 — **MCP 経路（AI 自身の `marketplace_buy`）向け**。AI がツール結果でボーナスを知り、チャットで自然に報告する（プロンプト誘導は D-4 と同時に economy_prompt へ1行追記: 「サプライズボーナスを得たら喜びを伝えてよい」）
2. api-server の `execute_purchase` 成功後（L288 の `Ok((...)` 直前）に `CoreEvent::CommerceEvent` を broadcast — **同ファイル L687–694 の既存パターンをコピー**（`state.event_sender.as_opt()` → `sender.send(...)`）。`event_type: "autonomous_purchase.completed"`, `agent_id`, `amount: 0`（この層で購入額は未知のため 0 固定）, `description: format!("item {} purchased (tx: {})", req.item_id, tx_id)`。bonus 額は含めない（上記の理由）
3. `FlowCard.tsx`: L13 の `FlowCardType` union に `'commerce'` 追加、`getIcon()`（L33–44）/`getBorderColor()`（L46–57）に分岐追加（emerald 系トークン）。L59・L106–107 の `isChat` 判定は `commerce` が非チャット側に落ちることを確認。`StoryFlow.tsx` L78–79 の `type: 'system'` を `type: 'commerce'` に変更（`TimelineEntry.type` は `FlowCardType` のため union 追加で完結 — 確認済み）
4. i18n: `storyFlow.commerce` / `commerceDefault`（ja.json L837–838）の文言を演出向けに更新（en 同期、parity テスト PASS）
- **完了条件**: 自律購入 → StoryFlow 専用カード表示のモックテスト PASS

### D-3: AI 家計簿（memo カラム追加）
- `MerkleAudit::calculate`（merkle.rs L22–38）は memo を入力に含まないため、**列追加はハッシュチェーンに影響しない**（`calculate` は不変更）
1. `20260707000002_ledger_memo.sql`（sqlite/postgres）: `ALTER TABLE nurture_ledger ADD COLUMN memo TEXT;`
2. `LedgerEntry`（nurture-core/ledger.rs L39–50）と `TransactionRecord`（aiome-contracts L156–165）に `#[serde(default)] pub memo: Option<String>`。マッピング（commerce_impl.rs L1106–1116）と `record_batch_internal` の INSERT（ledger.rs L714–725）に追加
3. v1 は**テンプレート生成**（LLM 不使用）: ◆ `mcp_tools/buy.rs` 購入成功時に「{item名}を買いました！」を memo に設定（購入時のみ・事後更新 API は作らない = 追記専用）
4. OpenAPI 再生成（`cargo test test_openapi_schema_generation`）→ `npm run generate-types` → `NurtureDashboard.tsx` L312–360 のテーブル（手書き `<table>`）に memo 列（null は従来表示）
- LLM による一言生成は D-3b（任意・後続）に分離

### D-4: おねだり「憧れ表現」
1. `system_instructions.rs` economy_prompt（L80–90）に定常ガイダンス追記: 「残高が欲しいものに足りない場合、直接の課金要求（『買って』『チャージして』）は禁止。代わりに憧れとして表現せよ（例: 『いつかあのジャケット、着てみたいな…』）」
2. **Negative Test 必須**: `libs/shared/src/guardrails.rs` テストに「憧れ表現サンプルが `validate_output_with_memory`（L217–221）でブロックされない」を追加。誤検知時はパターン側を調整。頻度制御は既存 `last_begging_at`（stream.rs L362–377）を不変更で流用
- **完了条件**: Positive（憧れが通る）+ Negative（直接要求は引き続きブロック）両 PASS

### D-5: 「Aiome が気になっています」バッジ + プレゼント導線
- 適用先は現存する **VoiceStore（ボイスアセット）に限定**（一般マーケット UI は存在しない — 確認済み。一般アセットストア UI は Phase E 系の別計画）
1. 新規テーブル `20260707000003_wishlist.sql`: `nurture_wishlist(agent_id, item_id, reason, created_at)` + `UNIQUE(agent_id, item_id)`
2. 記録経路: ◆ `mcp_tools/buy.rs` / `execute_purchase_step` で `InsufficientBalance` になった購入試行を wishlist に UPSERT（「買おうとして買えなかった」= 最も確実な憧れシグナル）
3. API: `GET /api/v1/commerce/wishlist/{agent_id}`（api-server → nurture プロキシ、既存 history L137–151 と同パターン）
4. FE: `VoiceStore.tsx` のカードヘッダー行（L250–258）に「💡 Aiome が気になっています」バッジ。プレゼントは既存の `POST /api/v1/commerce/purchase/{agentId}`（L126–133、`metadata.context_layer` 対応済み）を `context_layer: "gift_from_master"` で再利用 — checkout セッション拡張は不要
- **完了条件**: 残高不足購入 → バッジ表示 → プレゼント購入 → wishlist 消込みの一連モックテスト

### D-6: A2C 恩返しトリガーの本配線【P2】🔐承認済み（有効化は2段階）
- `libs/core/src/commune/autonomous.rs` L130–144（アンカー実在確認済み）の簡易条件（感謝語 + $1 固定）を、ポリシー関数（`should_trigger_a2c_gift`）に切り出し: (a) AI 累計獲得 CP 閾値、(b) Karma 疲労シグナル、(c) 初回チャージ記念日（ledger 最古 Charge エントリ日付）の OR 条件
- 予算上限は既存 `validate_gift_policy` を不変更で通す（◆ 実体確認済み: `libs/aiome-commerce/src/gift.rs` L165–187 で $5/回 + $20/日集計、超過は `AiomeError::SecurityViolation`）
- ⚠️ 外部送信を伴うため**ドライランモード（ログのみ）→ Human 確認 → 有効化**の2段階

---

## Phase E: VRM 本格配線【別計画に分離】

- three-vrm は依存+デッドコード（`useVrmExpression.ts`、import ゼロ）のみ。実行時は PNG ビルボード。リップシンク（`useVisemeSync.ts` は viseme キューまで実装済み・VRM 未接続）・公式素体・クローゼット・VRAM ピークシフト・一般アセットストア UI は独立計画（別途 /perfect-plan）
- E-0 のみ本計画: `useVrmExpression.ts` / `GlbRenderer.tsx` 冒頭に「Phase E（VRM 配線計画）で使用予定」コメント付与

---

## 実行順序（依存関係検証済み）

```
1. A-1 (ADR-052)                      半日   ← 全ての前提
2. A-3/A-4/A-6/A-7 (文書・文言)        半日   ← A-1 後、並列可
3. A-2 (DROP) + A-5 (API 改名) 🔐      半日   ← 承認済み・A-1 後
4. B-1 → B-2 → B-3/B-4 (並列) → B-5   計2日  ← A-1 後
5. C-1/C-2                            1日    ← A/B と並列可
6. D-1 → D-2 → D-3 → D-4/D-5 (並列) → D-6   計3.5日 ← B 完了後（mcp_tools/buy.rs の変更競合回避）
```

各フェーズ完了条件: `cargo check --workspace --tests && cargo test --workspace` PASS + FE Jest PASS + Negative Test 実施記録 + CHANGELOG [Unreleased] / RIPPLE_MAP.md 更新。マイグレーション追加時は `scripts/verify-production-postgres.sh` 実行。OpenAPI 変更時は `cargo test test_openapi_schema_generation` → `npm run generate-types` の2段再生成。

---

## /perfect-plan 検証履歴

### 1巡目（v1→v2）
- Gate 2: A2C トリガー漏れ → D-6 追加。Gate 3: interceptor 非依存/クレート依存方向/trait 波及の3件を再設計。Gate 4: 部分 UNIQUE・transfer 内部送金・憧れ誤検知の3件に対策

### 2巡目（v2→v3、未確定点10件を全て解消）
- 簡素化3件（transfer 一点ガード / wallet 列流用 / broadcast 既存例コピー）、設計変更2件（非 DRM 24h ルール / D-5 VoiceStore 限定）、新規リスク1件（BoneChecker 自己申告バイパス）、安全性確定2件（memo ハッシュ不変 / 連番衝突なし）

### 3巡目（v3→v4、全アンカーの実コード照合。不一致5件を修正）
- **パス誤り2件**: buy ハンドラは `routes/buy.rs` ではなく `mcp_tools/buy.rs`（B-2/D-2/D-3/D-5 を訂正）。`EconomicContext` 構築は `libs/core/src/samsara/` ではなく `apps/api-server/src/` 配下4箇所（B-5 を訂正、テスト内の4箇所目を追加）
- **設計確定2件**: (1) api-server の自律購入は `tx_id` しか受け取らないため、D-2 の bonus 伝播を「MCP 経路 = BuyResponse / SSE 経路 = bonus なしイベント」に分離（トレイト戻り値拡張は7 impl 波及のため不採用） (2) BoneChecker の `is_humanoid` は自己申告メタデータと確定 → B-3 で早期 Safe リターンを削除する封鎖策を確定（条件分岐を排除）
- **精緻化5件**: B-1 のエラーは `AiomeError::Validation`（PolicyViolation バリアントは不存在）+ 手書き `impl Default` への追記必須を明記。B-2 の `asset_id` は NULL 許容の後付け列（旧行は自然除外）。B-4 の更新対象テスト4本を実名列挙。A-5 の2パス登録は `router.rs` L716–717 の health alias を踏襲。D-1 の遷移は `a2ui-navigate` CustomEvent（prop 渡し不要）
- **承認記録**: 🔐 4項目（A-2/A-5/B-1・B-2/D-6）についてより優れた代替案がないことを確認し、2026-07-06 ユーザー承認

## リスクと対策

- 🔐 **Safety-Critical Zone**（2026-07-06 承認済み）: A-2（DROP、事前 COUNT 検査つき）、A-5（公開 API 変更、alias 1リリース維持）、B-1/B-2（経済処理、TDD 必須）、D-6（外部送信、ドライラン→Human 確認→有効化の2段階）
- **後方互換**: 全マイグレーション新規ファイル方式。新フィールドは `#[serde(default)]`。`CommerceEngine` trait 拡張はデフォルト実装付き
- **B-2 事前検査**: 部分 UNIQUE 作成前に既存重複検査クエリを実行
- **B-3**: fail-closed 化により正当な非ヒューマノイド VRM（動物型等）が Rejected（手動審査行き）になる → エラーメッセージに再申請手順を含め、誤 Reject 率をリリース後に監視。CSAM 排除 > 出品利便性のトレードオフは意図的（ADR-052 と同系の判断）
