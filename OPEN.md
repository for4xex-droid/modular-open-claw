# 📋 OPEN.md — 未解決タスク台帳（Single Source of Truth）

**最終更新: 2026-07-03**

## 運用ルール

- 未解決タスクは**このファイルのみ**で管理する（`memory/` の Open は当日分の追記メモであり、翌日以降はここへ反映する）。
- 各行は `- [ ] **ID**: 内容（初出日）` 形式。解決時はチェックを付け「✅ 解決」セクションへ移し、解決日と根拠（コミット/CHANGELOG）を1行添える。四半期ごとに解決済みを削除する。
- 凍結タスクは「⏸️ 凍結」セクションで管理し、解除条件を明記する。

## 🔴 P0 / ブロッカー

- [ ] **OP-001**: v8.3 リリースタスク（17個）の実装と検証（2026-06-11）
- [ ] **OP-002**: BiomeBackground + alpha:false 修正の目視検証（ブラウザ確認）（2026-06-30）

## 🟠 P1 / 次期リリース

- [ ] **OP-010**: Stripe Customer Portal 統合 — クレート追加、ポータル URL 生成エンドポイント新設（2026-05-28、HANDOVER.md P1-1）
- [ ] **OP-011**: `execute_autonomous_purchase` の封印解除 — Nurture /internal/purchase へのプロキシ実装（2026-05-28、HANDOVER.md P1-4）
- [ ] **OP-012**: PostgreSQL 本番環境での統合デプロイ検証（BAN 統合含む）（2026-05-20 / 05-22）
- [ ] **OP-013**: Stripe E2E テストの実行と結合挙動の継続確認（P2-2）（2026-05-29）
- [ ] **OP-014**: CLI ツールを用いたローカル Keychain 移行動作検証（2026-06-22）

## 🟡 P2 / 継続課題（技術的負債は REMAINING_TASKS.md 2026-07-02 版から吸収）

- [ ] **OP-020**: Phase 2b（Tauri シェル）、Phase 4（経済接続）、Phase 5（Federation）未着手（2026-06-10、ロードマップ参照）
- [ ] **OP-021**: BAN 管理ダッシュボード UI の設計検討（2026-05-22）
- [ ] **OP-022**: CausalVisualizer（Trajectory Graph の UI 可視化）未着手（MEMORY.md Blind Spots より）
- [ ] **OP-023**: `infrastructure` コアに残存する一時的 `unwrap()` / ドキュメント警告のリファクタ（R-005 違反）（MEMORY.md Blind Spots より）
- [ ] **OP-024**: `tool_call_router.rs` 課金チェックの Fail-Closed 化（DB エラーを握り潰さず明示拒否）（MEMORY.md Phase 48 より）
- [ ] **OP-025**: `key-proxy` への Telemetry（`caller_id`）追加と Cross-Node Auth Reliability モニタリング（MEMORY.md Phase 4 より）
- [ ] **OP-026**: X Signal Probe 設定画面 UI（SettingsPage.tsx, settings.rs）（2026-04-07）
- [ ] **OP-027**: Stripe API 実装追加時の一元化モック拡充（2026-06-01）
- [ ] **OP-028**: フロントエンド `as any` 型キャスト4箇所の解消（WorkflowBuilder.tsx ×3, workflowConverter.ts ×1）（2026-07-02）
- [ ] **OP-029**: `biome-popup-entry.tsx` HEX カラー直書きの解消（U-002 違反、`var(--bg-primary)` へ置換）（2026-07-02）
- [x] **OP-050**: `skills/mod.rs`（1,134行）God Module の責務分解 → 2026-07-03 完了（599行に縮小、code_mode.rs / host_fns.rs / types.rs へ分離。refactor/skills-god-module ブランチ）
- [ ] **OP-051**: Error 型定義の統一（thiserror/anyhow 混在 10種類 → 3階層）（2026-07-02）
- [ ] **OP-052**: `deep-scan.sh` CRATES 設定修正（廃止済み `apps/watchtower` の除外）（2026-07-02）
- [x] **OP-053**: `skills/mod.rs` L163 `unwrap_or_else(|_| loop {})` の安全なエラー伝搬への修正（Dim 10 違反） → 2026-07-03 完了（DUMMY_REGEX 削除、`LazyLock<Option<Regex>>` 化）
- [ ] **OP-054**: JobQueue トレイトの API 乖離解消（補助メソッドのトレイト引き上げ or private 化）（2026-07-02）
- [x] **OP-055**: `immune_system.rs` 内 MockJQ（約700行）の共有化 → 2026-07-03 完了（新クレートではなく `infrastructure::testing::mock_jq` クレート内モジュールとして抽出）
- [ ] **OP-056**: フロント `useWorkflowApi` の `POST /api/v1/workflows/validate` とバックエンド `/api/v1/workflows/:id/validate` のパス不整合（F-1 実装時に発見、修正は未実施）（2026-07-03）
- [ ] **OP-057**: LP Stripe Payment Link（$9.99 Pro）決済とセルフホスト環境の Pro 有効化が自動接続されていない。ライセンスキー配布 or Customer Portal 連携の設計が必要（PR品質改善 M-2 で発見）（2026-07-03）
- [ ] **OP-058**: `ProUpgradeModal`（402→アップグレード導線）が実装・テスト済みだが App.tsx 未マウントでコンバージョン機会を損失。commerce 系 Safety-Critical Zone のため人間許可後にマウント（2026-07-03）
- [ ] **OP-059**: ハイブリッド価格（Pro $9.99＋KC 月次含み枠＋超過チャージ＋支出上限設定）のバックエンド実装。ユーザー採用決定済み（2026-07-03）。commerce 系 Safety-Critical Zone のため人間レビュー必須。実装完了まで LP/README に含み枠数値を記載しない

## 🔵 Upstream 待ち（scripts/watch_upstream_blockers.py で監視中）

- [ ] **OP-030**: serenity 0.13+ リリース待ち → `discord.rs` 改修で RUSTSEC-2026-0098 等を解除（Issue A）
- [ ] **OP-031**: bastion-core TLS/DNS 近代化（Issue A 完了後、idna 等 CVE 解除）（Issue B）
- [ ] **OP-032**: extism v1.22+ / wasmtime v43+ で Wasmtime CVE 解除（Issue C）
- [ ] **OP-033**: tauri v3.0.0+ で GTK4/unic CVE 解除（Issue D）
- [ ] **OP-034**: Tauri の `plist` 依存更新後、`.cargo/audit.toml` の quick-xml 無視設定（RUSTSEC-2026-0194/0195）を削除し `cargo update -p quick-xml`（2026-07-02）

## 🌱 Project-Nurture 側（経済・コンプライアンス）

- Nurture 側の残存タスク（TLA+ 形式仕様、VRAM 競合調停、On-memory DRM、Saga 補償設計、資金決済法対応、特商法表記、自律購買ポリシー、CP 報酬変換、コールドスタート対策等）は `REMAINING_TASKS.md` セクション3を参照（次回 Nurture 側スプリント時に本台帳または Nurture 側台帳へ正式移入する）。

## ⏸️ 凍結（解除条件つき）

- [ ] **OP-040**: OGP 画像（og:image）・プロモーション動画の埋め込み — **完全凍結**。解除条件: ユーザーから完成版ロゴ・音声素材の提供。仮画像・プレースホルダーでの代用は厳禁（HANDOVER.md より）

## ✅ 解決（直近のみ保持）

- [x] **OP-900**: 異常ファイル名 `memory/2026-04-07.md\`` の整理 — 2026-07-03 解決（Lessons を正規版へマージし memory/archive/ へ移動）
