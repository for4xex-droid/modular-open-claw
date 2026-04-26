# Grand Roadmap (v1.0 ~ v2.0)
> 更新日時: 2026-04-24
> 最終ブラッシュアップ: KarmaUXの重複解消とGap10の統合

本ドキュメントは、Aiome および Project-Nurture における、v1.0 本番リリース直前の「技術的負債完全掃討（Zero Debt Sweep）」から、リリース後の「次世代拡張（Next-Gen Integration）」に至るまでの全体ロードマップを定義した Single Source of Truth (SSOT) である。

## ✅ Phase 0: The Zero Debt Sweep (Pre-Release Blockers) [COMPLETED]
リリース前に絶対に解決しなければならない、深刻なセキュリティ・安定性に関わる残存負債。

1. **インフラ & セキュリティコア**
   - `AbyssVoiceVault` (FIXME L25): インメモリHashMapに復号鍵をリストアする暫定実装を廃止し、本番水準の HSM / セキュアエンクレーブ機構へ移行する（最も致命的なセキュリティギャップ）。
   - `docker_conductor.rs` (Gap C, G, I, J, K, L, N, R, S): コンテナへの環境変数を CLI 引数から ephemeral `env-file` 渡しへ変更。gRPC ヘルスチェック、ストリーム終了制御、イデムポテントなネットワーク生成とコンテナクリーンアップを完全実装。
   - `logging.rs` (SEC-4): ログ出力時の `Authorization` や `x_bearer_token` などの機密情報のマスキングを実装。
   - `a2a_grpc_client.rs` (Gap A): gRPC呼び出しにおけるAuthorizationヘッダーの確実な注入。

2. **ステート & レートリミット保護**
   - `jobs.rs` (Gap 11, 8, 4, 6) & `task_orchestrator`: `AwaitingInput` 解除時の Race Condition 防止（DB行ロック）と `execution_log` へのバイパスマーカー永続化。
   - `api_integration_tests.rs` (Gap 10): テスト環境における SQLite CHECK constraint migration の完全適用とテスト通過の保証。
   - `stripe.rs` (P0-1): Stripe API 呼び出しの全 `.send()` に対する30秒タイムアウト制約の適用。
   - `cost_breaker.rs` & `dynamic.rs` (GAP-6, Gap #4): セッション単位（例: 最大$0.5）のローカルコスト制限と `cost_usd` / `token` 計算ロジックの注入による課金暴走防御。
   - `bridge.rs` (N-FIX): `EconomyPolicy` のハードコードを廃止し、DB から動的ポリシーを取得・適用。
3. **コンプライアンス & RTBF (Right to be Forgotten)**
   - `security.rs` (`forget_actor`): 現在の「データベースの行削除のみ」の実装に加え、**BlobStorage / S3 上の物理アセット（VRMモデル、生成画像等）の完全削除**を実装する。これによりGDPR/忘れられる権利の重大な法的違反リスクを排除。

## 🔴 Phase 1: v1.0 Core Engine & Trust Layer (認証・認可の完全標準化)
法的・倫理的防壁を完成させ、コアエンジンとして安全に世に送り出すフェーズ。
*(注意: エコシステムの UX 破綻を防ぐため、商取引機能は Phase 2 に移行済)*

1. **CSAM 3層防壁の完成**
   - `csam/proportions.rs`: `gltf-rs` を用いた実際の VRM ボーン解析（頭身比チェック）を実装（骨格による児童ポルノ判定）。
   - *備考: `image_hash.rs` の PhotoDNA API連携は、Microsoftの承認が下りるまでGraceful Fallback（Mock）として運用。*
2. **本番デプロイメント検証**
   - P1-3: Tauri デスクトップビルドの検証（`api-server` + `management-console` シングルバイナリ化）。
   - P1-4: GitHub Actions PRトリガーへの Gitleaks CI/CD 完全統合。

## 🔵 Phase 2: v1.1 Creator Economy & Interaction
リリース後の次月スプリント。エージェントの「自我」と「商取引（マーケット・Karma）」の同時解禁。

1. **KarmaForge と Marketplace の統合解禁 (UX重複の解消)**
   - `shadow-worker` & `api-server` (P2-4): OxiLean Phase 2 (`ProofVerifierClient`) の統合。
   - `plugin.rs` (GAP-N, N2-C): OxiLean 形式検証の証明力シードと実データを `KarmaForge::cross_synthesize` に注入。
   - `plugin.rs` (Sprint-C): 上記の **Karma 報酬基盤が整った状態**で、初めて `marketplace_upload` MCPツールを解禁。これにより「アップロードしたのにKarmaが貰えない」UXの破綻を防止。
   - `merchant.rs` & `catalog.rs` (Phase 2): B2A (Business to Agent) マーチャント登録およびカタログ公開機能の実装。
   - `affiliate_adapter.rs`: アフィリエイトアダプタのモック廃止。
2. **Samsara & LiveSession (自己と対話の深化)**
   - `samsara_engine.rs` (GAP-2, 3, 4): 経験バッファの膨張を防ぐための Narrative Self（物語的自己）生成と記憶の蒸留（Distillation）。
   - `live_session.rs` (GAP-2, 4, 7): バックグラウンドでのセッション維持、音声/テキストのストリームTx/Rxの完全実装。
3. **A2UI インタラクション & ダッシュボード**
   - `A2uiRenderer.tsx` (P2-3): Action Button の `onClick` から `/api/v1/a2ui/action` コールバックの双方向ループ確立。
   - `EvaluationLogger` メトリクス（コスト・レイテンシ）のフロントエンド可視化 (P2-5)。

## 🟣 Phase 3: v2.0 Ecosystem Expansion
Aiome が完全に自律した分散経済圏を形成するための将来フェーズ。

1. **A2C ギフト・報酬経済**
   - `gift.rs` (TODO Phase 3): ギフト配送 MCP ツールの本稼働。
   - Tremendous API 連動によるクリエイターポイント（nurture_points）のリアル報酬（現金化・ギフトカード等）フロー確立 (P2-1)。
2. **AI Capabilities の拡張**
   - `VisionProvider` の実装（GPT-4V / LLaVA 等）。
   - `bootstrap.rs` & `heartbeat.rs` (Phase 4C): `TimesFM` (ForecastProvider) および `SLM` (Small Language Model) のメインプロセス注入による予測エコシステムの稼働。
3. **P2P & クロスプラットフォーム (Nurture Synergy)**
   - `aiome-node/src/main.rs` (MVP Stub): 現在の「Validator / GigEngine の MVP モック」を完全廃止し、本番水準の P2P ノード（SamsaraHub Federation）として昇格させる。
   - `SYNERGY.md (6-4)`: Minecraft MCP との連動によるゲーム内建築・行動の Karma 反映。
   - `SYNERGY.md (2C-7, 2D-6)`: デスクトップペット機能と PWA との連動による、Nurtureアセット（衣装等）の日常的な着せ替え・購入体験のシームレス化。
   - Tauri モバイル (iOS/Android) 展開。
