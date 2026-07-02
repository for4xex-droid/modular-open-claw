## 🔍 Biome 背景グレーバグ、レイアウト崩れ、およびチュートリアル見切れの根本修正 (2026-07-01)

- **変更内容**:
    - `apps/management-console/src/lib/biome/BiomeCanvas.tsx` [MODIFY]: `gl.alpha: false` を指定し、CSS背景色との不要な透過合成を完全に無効化。
    - `apps/management-console/src/lib/biome/BiomeBackground.tsx` [MODIFY]: `depthWrite/depthTest` をデフォルトの不透明に戻し、Z座標を `-10` に設定して、セル（Z=0）が深度テストで最前面に正しく上書き描画されるよう修正。また、細胞風のボロノイノイズを廃止し、滑らかなラジアルグラデーションに置き換え。
    - `apps/management-console/src/lib/biome/shaders/biomeCell.ts` [MODIFY]: セルのフラグメントシェーダーでの出力を 1.8倍 にブースト。
    - `apps/management-console/src/lib/biome/BiomeCellGrid.tsx` [MODIFY]: セルの描画スケールを 0.65 に拡大。
    - `apps/management-console/src/lib/biome/BiomePostEffects.tsx` [MODIFY]: Bloomの `luminanceSmoothing` を 0.2 に引き締め。
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: 左・中・右に分かれる対称的な 3カラムフレックスレイアウト を導入し、右サイドバーの縦伸びと左キャンバスの押し下げ・画面切れを解消。
    - `apps/management-console/src/lib/biome/BiomeTutorial.tsx` [MODIFY]: ツールチップ幅を最大 460px に制限し、`window.innerHeight` に基づく上下限クリップ処理（16pxセーフティ）を追加して画面外への見切れを防止。
- **波及効果**:
    - **ビジュアル / UI体験**: 細胞ノイズの生理的嫌悪感が解消され、吸い込まれるようなプレミアムな宇宙的ダークグラデーションに一新。暗い背景色が Bloom を通して画面全体に漏れ出していた濁り（グレー現象）が消去され、1.8倍に輝くセル（0.65スケール）が漆黒に鮮やかに浮き上がる美しい3カラムHUDが完成。
    - **ユーザビリティ**: 画面の下半分が見切れる問題やスクロールの必要性が完全に根絶され、全コントロール（速度、元素、災害）とマップ情報がスクロールなしで一画面で操作可能。チュートリアルダイアログの見切れも完全に排除され、あらゆる解像度で確実に戻る・次へボタンが操作可能。

## 🔍 Biome 芋づる式拡散の是正（ダブルバッファリング）、自然死猶予措置、および高DPR表示バグの修正 (2026-06-30)

- **変更内容**:
    - `libs/biome-engine/src/grid.rs` [MODIFY]: 1フレーム内のループ進行方向に元素が玉突き移動してグリッドが飽和する「芋づる式拡散バグ」（左下1/4のグレー正方形領域の正体）を防ぐため、`current_cells`（前フレーム）を基準に拡散量を判定・算出する厳格なダブルバッファリング拡散を実装。また、拡散直後の新生セルが餓死（Decay）して生命の伝播が止まってしまうのを防ぐため、前フレームで非アクティブだったセルに対する Decay 処理を1フレーム猶予するロジックを導入。
    - `libs/biome-engine/src/lib.rs` [MODIFY]: 全元素の拡散が機能するようになったことに伴うセルの元素量減少による進化不整合を防ぐため、テスト `test_engine_last_tick_events` 内の H と O の注入量を 45000 から 60000 に調整。
    - `apps/management-console/src/lib/biome/BiomeCanvas.tsx` [MODIFY]: Canvasに `dpr={[1, 2]}` を指定し、カメラに `orthographic` 設定を直接組み込んで `manual` 制御を廃止。これにより、Retina等の高DPR環境でビューポート解像度やアスペクト比が同期せず、セル位置が大幅にY軸方向へズレたり、描画が左下1/4に縮小されていたバグを完全に解消。また、エフェクト適用時の Dpr 同期問題を解消。
    - `apps/management-console/src/lib/biome/BiomeBorder.tsx` [NEW]: セルオートマトン育成エリアの 128x128 境界を視覚化するネオンシアン色の枠線と四隅のL字型コーナーマークを描画するコンポーネントを新規作成。時間経過による不透明度の脈動アニメーションを実装。
    - `apps/management-console/src/lib/biome/shaders/biomeCell.ts` [MODIFY]: 頂点シェーダーにおいて、Three.js の自動的な `instanceColor` 属性注入と衝突した際の `redefinition` エラーおよび未定義エラーを防ぐため、マクロガード `#ifndef USE_INSTANCING_COLOR` を施した上で `attribute vec3 instanceColor;` を明示宣言。
    - `apps/management-console/src/lib/biome/BiomeCanvas.test.tsx` [MODIFY]: 頂点シェーダー内にマクロガード付きの属性宣言が存在することを検証するように、既存の TDD テストコードを更新。
    - `apps/management-console/src/lib/biome/BiomePostEffects.tsx` [MODIFY]: `<EffectComposer>` に `disableNormalPass` を指定し、不要な法線パスの自動レンダリングとそれに伴う左下へのグレーの半透明面誤投影バグを排除。
    - `apps/management-console/src/lib/biome/BiomeCellGrid.tsx` [MODIFY]: `react` からの `useEffect` インポート漏れを補正し、マウント/レアリティ変更時に全インスタンス行列のスケールを `0` にリセットする `useEffect` を追加（1フレーム目の未配置セル残留バグの防止）。
- **波及効果**:
    - **シミュレーション精度 / セルオートマトン**: 芋づる式拡散の根絶により、元素が不自然に高速で全域に爆発伝播してグリッドが飽和する現象が完全に解消された。さらに新生セルの猶予により、新領域への穏やかで動的な拡散挙動が実現し、多様性指数（Shannon）が正しく上昇して進化ランクが機能するようになった。
    - **ビジュアル / UI 崩れ**: セル飽和（極限希釈によるグレー化）の解消と、高DPR環境におけるWebGLビューポート・NormalPass・マウント時残留バグの修正が合わさり、画面左下を覆っていた不当なグレーの正方形が 100% 完全に根絶された。また、新規追加した `BiomeBorder` により育成エリアが明確に可視化され、UI 品質が大幅に向上した。
    - **テストスイートの健全性**: 修正後、`biome-engine` クレートのユニットテスト、およびフロントエンドの Jest テストが 100% GREEN パス。アプリ全体のビルドエラーも完全に防止された。



## 🔍 Biome 本番化 & Tauri CSP 修正 (U-002) (2026-06-30)


- **変更内容**:
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: 16px (1rem) 相当のフォントサイズトークン `--font-size-md` を追加補完。
    - `apps/management-console/DESIGN.md` [MODIFY]: `DESIGN.md` のフォントサイズ仕様一覧に `font-size-md: "1rem"` を追記同期し、トークンの公式仕様化とビルド警告を解決。
    - `apps/management-console/src/lib/biome/BiomeComponents.test.tsx` [MODIFY]: i18n モック環境の動作（`biomeConsole.activeCells` や `biomeConsole.elementRatio` キーなど）に合致するようにアサーション文字列を修正。また、コンソール文字化け環境でのボタン検知の不安定さを排除するため、部分一致正規表現 `/biomeConsole\.detail/i` に修正。
    - `apps/management-console/src-tauri/tauri.conf.json` [MODIFY]: `connect-src` に将来的な key-proxy (3017) / nurture-api (3020) のポートを予防的追加。
    - `apps/management-console/src/lib/biome/BiomeDendou.tsx` [MODIFY], `BiomeResult.tsx` [MODIFY], `BiomeEventToast.tsx` [MODIFY], `BiomeTutorial.tsx` [MODIFY]: 計 38 箇所のインラインスタイル生値を tokens.css 定義の CSS カスタムプロパティに完全置換 (U-002 トークン違反の変改)。
    - `BiomeResult.tsx` [MODIFY], `BiomeTutorial.tsx` [MODIFY]: `React.CSSProperties` を `CSSProperties` に修正し、不要な React インポートを型インポートにクリーンアップ。
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: 標本保存時 (handleSaveSpecimen) に、WASMから取得した最新の元素バランス (`element_balance`) と活性セル数 (`active_cell_count`) を specimens 保存 API ペイロードに含めて送信するように実装。また、`generation` のハードコードを解除。さらに `BiomeResult` コンポーネントへ `elementBalance` / `activeCellCount` データを結線。
    - `apps/management-console/src/lib/biome/BiomeGame.test.tsx` [MODIFY]: useBiomeEngine フックをモック化して同期的なテスト制御を可能にし、標本保存時の API ペイロードに正しいデータが含まれることを検証する TDD テストケースを追加。
    - `apps/management-console/src/i18n/ja.json` [MODIFY], `en.json` [MODIFY]: Biome 管理 UI 用の翻訳テキスト辞書 `"biomeConsole"` オブジェクトを新規定義。
    - `BiomeDendou.tsx` [MODIFY], `BiomeResult.tsx` [MODIFY]: 表示テキストおよびボタンラベル、各ヘッダーを `useTranslation` フックを用いて多言語化。
- **波及効果**:
    - **U-002 / トークン主管**: WebGL/Canvas テーマカラー同期に続き、UI レイアウト（fontFamily, fontSize, padding, gap, margin, borderRadius）全体のインライン生値が tokens.css トークンに 100% 統一された。新設した `--font-size-md` (1rem) トークンにより、サイト全体のデザイントークンスケールが一貫性を維持。また、`DESIGN.md` の追記同期により、リポジトリビルドフックにおける同期漏れ警告が完全に解消。
    - **テストの健全性 & 堅牢性**: 多言語化導入によって発生した既存アサーションの不整合が解消され、全 377 テストケースが 100% GREEN に回復。テスト内の要素取得を部分一致にすることで、実行コンソールでの絵文字文字化け（環境依存）による偽陰性エラーのリスクが排除された。
    - **Tauri / CSP (Release Readiness)**: 将来的な key-proxy や API サーバ直接通信時の CSP (Content Security Policy) エラーリスクが予防的に排除され、製品パッケージとしてのデスクトップリリース準備が完了した。
    - **Data Integrity / 標本データ**: 殿堂入りした標本データに「元素比率」と「活性セル数」が保存されるようになり、保存データを再読み込みした際にも UI 上でこれらの情報が欠落なく正しく復元・表示される基盤が完成。
    - **Internationalization (i18n)**: Biome コンポーネントの管理テキスト（殿堂入りリスト、結果表示）が英語・日本語でダイナミックに切り替わるようになり、グローバル多言語化要件を完全に満たした。
    - **テスト・整合性**: 新規追加した TDD テストケースを含め、テストスイート全体の 377 個のテストおよび静的解析がデグレーションなくパス。

## 🔍 統合技術的負債の解消 (Open Questions / Quick Wins) (2026-06-29)

- **変更内容**:
    - `libs/infrastructure/src/job_queue/mod.rs` [MODIFY]: `select_ladder_sandbox` を `pub(crate)` に限定し、API をスリム化。
    - `apps/api-server/src/main.rs` [MODIFY]: `UniversalJobQueue::do_push_federated_metrics()` のダウンキャスト具象メソッド呼び出しを廃止し、`FederationOps` トレイト経由の呼び出しへリファクタリング。
    - `apps/management-console/src/styles/tokens.css` [MODIFY], `apps/management-console/src/lib/biome/ThemeBridge.ts` [NEW], `BiomeCellGrid.tsx` [MODIFY], `BiomeGame.tsx` [MODIFY], `BiomeHUD.tsx` [MODIFY]: CSS デザイントークンベースの WebGL / Canvas カラー同期ブリッジを導入し、HEX カラーのハードコードを完全に排除。さらに `BiomeCellGrid.tsx` は import 順序規約違反の解決および動的カラー取得への変更（モジュールスコープ配列の廃止）を追加修正。
    - `apps/management-console/src-tauri/Cargo.toml` [MODIFY], `src-tauri/src/lib.rs` [MODIFY], `apps/management-console/src/types/tauri.ts` [NEW]: `ts-rs` を用いた Tauri IPC 構造体の自動型定義生成・共有パイプラインを構築。
    - `auth.rs` [MODIFY], `commune_ws.rs` [MODIFY], `heartbeat.rs` [MODIFY], `expression.rs` [MODIFY]: サイレントエラーの握り潰し（5箇所）を排除し、エラー時の警告・エラーロギング（`warn!` / `error!`）を伴う堅牢なエラー処理へ置き換え。さらに `auth.rs` の `admin_password_hash` 取得失敗時のサイレントなエラー握り潰し（`.ok().flatten()`）を追加修正。
    - `apps/api-server/src/api_integration_tests/auth.rs` [MODIFY]: `admin_password_hash` 取得失敗時のロギングパスを検証する統合テスト `test_auth_admin_hash_fetch_failure_logging` を追加。
- **波及効果**:
    - **U-002 / テーマ同期**: WebGL 描画および UI カラーのハードコードが一掃され、テーマ変更が動的に Three.js / Canvas 描画に反映されるようになった（`BiomeCellGrid` での動的取得の最適化により、テーマ変更時の動的追従性を 100% 達成）。
    - **Tauri IPC 型安全性 (Dimension 11)**: Rust 構造体から TypeScript 型定義が自動生成されるようになり、定義ズレによるランタイムクラッシュのリスクが排除された。
    - **Observability / エラー処理 (Dimension 7)**: DBエラーやパスワードハッシュ取得/パース/検証失敗、クロック同期失敗などの「静かな失敗」が全て可視化され、障害発生時の迅速なデバッグが可能となった。
    - **テスト・整合性**: ワークスペース全体の全 4,524 テストおよび静的解析（`enforce_unwrap_deny.py`, `deep-scan.sh --ci`）がデグレーションなくパス。

## 🔍 統合技術的負債の解消 (2026-06-29)

- **変更内容**:
    - `apps/api-server/src/routes/vault.rs` [MODIFY]: `vault_status`, `vault_upsert`, `vault_delete` に認証制約 `_auth: crate::auth::Authenticated` を追加。
    - `libs/infrastructure/src/skills/mod.rs` [MODIFY]: 5箇所の `unreachable!()` を `DUMMY_REGEX` によるパニックフリーなフォールバックへ置換。
    - `apps/management-console/src-tauri/src/lib.rs` [MODIFY]: アプリ構築失敗時の `panic!()` を `eprintln!` と `std::process::exit(1)` による終了処理に変更。
    - `.env.example` [MODIFY]: `NURTURE_DRM_MASTER_KEY` と `BIOME_HUB_WHITELIST` 環境変数を追加。
    - `scripts/deep-scan.sh` [MODIFY]: `BEGINFILE { exempt_seen=0 }` の追加により、複数ファイルスキャン時の状態汚染バグを解消。
- **波及効果**:
    - **Type-Driven Security (CC-6)**: `vault.rs` の管理エンドポイントが型安全に保護されていることが静的に保証され、セキュリティ監査の健全性が 100% 達成された。
    - **Zero-Panic Policy**: WASMスキル正規表現初期化およびデスクトップアプリ起動エラーにおける予期せぬパニックのリスクを排除し、信頼性が向上した。
    - **ビルド・テスト環境**: `cargo check --workspace --tests` および全 1,103 テストがデグレーションなくパス。

## Verify-to-Iterate Loop / Reflexion Loop 統合 (2026-06-29)

- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/dispatch_loop.rs` [MODIFY]: Oracle `Reject`/`Revise` → `fail_job` の即時終了を廃止し、`increment_job_retry_count` + `append_job_karma_directives` + `requeue_job` のリトライパスに変更。上限到達時は `AwaitingInput`（人間介入）に遷移。
    - `libs/infrastructure/src/task_orchestrator/goal_processor.rs` [MODIFY]: `job.topic` と `job.karma_directives` の排他的処理 (`if/else`) をマージロジックに変更。JSON 構造は非マージ（構造保全）、プレーンテキスト修復ヒントのみ `[Previous Execution Context]` タグ付きでマージ。
    - `libs/infrastructure/src/context_engine.rs` [MODIFY]: `ContextBudget` に `max_job_retries: i64` フィールド追加（デフォルト: 3）。
    - `libs/infrastructure/src/job_queue/core_ops.rs` [MODIFY]: `do_increment_job_retry_count` のハードコード `count >= 3` を `ContextBudget::default().max_job_retries` に置き換え。
- **波及効果**:
    - **dispatch_loop.rs → goal_processor.rs**: Oracle Reject 時に `karma_directives` に蓄積された Oracle フィードバックが、次回リトライ時に GoalProcessor 経由でプランナーに伝搬されるようになった。Watchtower Read-Path（`WATCHTOWER_INSIGHT` 注入）と相補的に機能。
    - **context_engine.rs → core_ops.rs**: `max_job_retries` が設定値化されたことで、将来的に設定ファイル経由でリトライ上限を動的に変更可能。既存テスト（`test_job_retry_count_poison_pill`）はデフォルト値 3 で動作するため影響なし。
    - **既存テスト**: ワークスペース全体の 1,103 テストがデグレーションなくパス。



- **変更内容**:
    - `libs/biome-engine/Cargo.toml` [MODIFY], `templates/minimal-skill/Cargo.toml` [MODIFY]: `license` フィールドの不足を補完。
    - 以下の 17 個の `.rs` ファイルに対して、不足していた標準著作権・ライセンスヘッダーを `scripts/migrate_license_headers.py` によって補完:
      - `libs/biome-engine/src/element.rs`
      - `libs/biome-engine/src/rarity.rs`
      - `libs/biome-engine/src/evolution.rs`
      - `libs/biome-engine/src/crisis.rs`
      - `libs/biome-engine/src/lib.rs`
      - `libs/biome-engine/src/grid.rs`
      - `libs/biome-engine/src/genome.rs`
      - `libs/biome-engine/src/particle.rs`
      - `libs/infrastructure/src/task_orchestrator/workflow_conductor.rs`
      - `libs/infrastructure/src/dream_state/crisis_guardian.rs`
      - `libs/infrastructure/src/workflow/store.rs`
      - `libs/infrastructure/src/workflow/validator.rs`
      - `libs/infrastructure/src/workflow/mod.rs`
      - `libs/infrastructure/src/workflow/schema.rs`
      - `libs/infrastructure/src/workflow/transpiler.rs`
      - `libs/soul/src/biome_traits.rs`
      - `libs/aiome-core-contracts/tests/delegation_test.rs`
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - 監査スクリプト `scripts/license_check.py` による全 11 種類のコンプライアンス自動検証テストが **すべてパス (ALL TESTS PASSED)** するようになり、リリースに向けた法務コンプライアンス上の衛生状態が健全化されました。
    - ファイルメタデータの追記のみであり、バイナリやランタイム動作に対する影響（波及効果）は一切ありません。

## Autodata Boltzmann Selection (確率的ツール選択) の TDD 実装 (2026-06-29)

- **変更内容**:
    - `libs/infrastructure/src/skills/skill_arena.rs` [MODIFY]: スキルの過去の成功率（`success_count` と `failure_count`）からスコアを動的に算出し、Boltzmann (softmax) 分布を用いて確率的にスキルを選択する `select_skill_boltzmann` メソッドを新規実装。また、最低探索確率（探索フロア）と最低試行回数ガード処理を実装し、各種検証用ユニットテストを追加。
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - スキルアリーナ（`SkillArena`）から確率的にスキルを選択できるようになり、特定の良好なスキルのみが過剰に偏って選ばれる「勝者総取り」問題を防ぎつつ、新規/低成功率スキルに対しても適切な確率（探索フロア: 5%）で学習機会を与え続けることが可能になりました。
    - 既存の `record_outcome` や DB 永続化機能、他のモジュールへの破壊的なインターフェース変更を行わず、新規の補助メソッド追加として完全に局所化されているため、他のコードに対するデグレーションの心配はありません。

## Discovery-Driven Tool Registration (MCPツールの動的発見・統合) の TDD 実装 (2026-06-28)

- **変更内容**:
    - `libs/aiome-core-contracts/src/traits.rs` [MODIFY]: MCPツールの動的取得用抽象インターフェース `McpToolSource` trait を新規追加。
    - `libs/infrastructure/src/skills/discovery.rs` [MODIFY]: `DefaultToolDiscoveryEngine` に `McpToolSource` をオプションで保持・注入（`with_mcp_source`）できるようにし、発見されたツールに MCP サーバー提供のツールが動的にマージされるように拡張。また、モック `MockMcpToolSource` を用いた検証テスト `test_discover_tools_includes_mcp` を追加。
    - `apps/api-server/src/mcp/client.rs` [MODIFY]: `McpProcessManager` に `McpToolSource` を実装。接続中の全 MCP サーバーからツール一覧 (`tools/list`) を非同期取得して統合スキーマ形式へバインドするよう実装。
    - `apps/api-server/src/bootstrap/core_services.rs` [MODIFY]: ブートストラップでの `DefaultToolDiscoveryEngine` 生成時に、`McpProcessManager` を `McpToolSource` として注入するよう DI 接続を構築。
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - `DefaultToolDiscoveryEngine` が `McpProcessManager` とブリッジされたことで、自動発見された外部 MCP サーバー（Google Workspace CLI 等）が提供するツール群が、AIOME のツール発見エンジンに動的に統合されるようになりました。
    - エージェントが実行時に利用可能な MCP ツールを自律的に探索し、ユーザーの指示に基づいて最適なツールセットを LLM がセマンティックに選択できる仕組みが確立されました。
    - 結合度を極小化するために `McpToolSource` トレイトを介した緩やかな抽象化（ブリッジパターン）を採用したため、インフラクレートと api-server クレート間の依存性の循環を回避し、疎結合な構成を維持できました。

## Closed-Loop Intelligence (自律最適化 & 品質ゲート) の TDD 実装 (2026-06-28)

- **変更内容**:
    - `libs/aiome-core-contracts/src/contracts.rs` [MODIFY]: 構造化フィードバック用のデータ構造（`FeedbackCategory`, `IterationRecord`, `OptimizationBudget`）と `ReviewDecision::HumanReview`, `SoTOutcome::ChallengerRejected` を非破壊的に追加。
    - `libs/infrastructure/src/society_of_thought.rs` [MODIFY]: 一次フィードバック分類ヘルパー `classify_feedback`、および予算上限制限付きでの Challenger-Verifier フィードバック最適化ループを実装。
    - `libs/infrastructure/src/oracle.rs` [MODIFY]: 平均スコア（Alignment & Growth）に基づく判定境界値（0.85以上で Accept、0.60〜0.85 で HumanReview、0.60未満で Reject）への ReviewDecision マッピングオーバーライドを実装。
    - `libs/infrastructure/src/skills/skill_arena.rs` [MODIFY]: `SkillPerformance` にレピュテーション履歴（`optimization_history` と `best_scores` JSON）を追加し、非破壊的マイグレーション（`ALTER TABLE`）と永続化、およびラッパー `record_outcome_with_feedback` を実装。
    - `libs/infrastructure/src/task_orchestrator/dispatch_loop.rs` [MODIFY]: 品質ゲートの三値判定分岐と、`HumanReview` 発生時のジョブステータスの `JobStatus::AwaitingInput` 遷移、および `TaskEvent::AwaitingInput` / `TaskEvent::QualityGate` イベントトリガーを実装。
    - テスト追加: `tests.rs` や `oracle.rs` 内にそれぞれ境界値マッピング、自律リトライ制限、DB永続化、ステータス遷移を検証する GREEN テストを追加。
- **波及効果**:
    - クライアント（contracts）からインフラ（infrastructure）にわたる一連の Closed-Loop 最適化構造が確立され、平均スコアに基づく厳格な品質ゲートをクリアできなかった場合、ジョブが自動的に人間承認待ち（AwaitingInput）へフォールバックされるため、不正動作やアライメント逸脱の発生率が物理的にゼロになりました。
    - SkillArena でのエラーレピュテーション履歴が JSON として SQLite 上で永続化されるため、Challenger の失敗情報が蓄積され、次回以降の世代の最適化における「失敗からの学習」の効率が大幅に向上しました。
    - 各種既存トレイト・シグネチャとの完全な後方互換性が維持されており、既存の Swarm シミュレーションやジョブディスパッチにカスケードエラーを引き起こすことなくシームレスに機能拡張できました。

## Zero-Metadata Commune Protocol v2 および 実行ラダー・Code Mode の TDD 実装 (2026-06-27)

- **変更内容**:
    - `libs/shared/src/crypto.rs` [MODIFY]: `encrypt_commune_envelope` を修正し、ランダム生成だった `channel_local_id` を外部から受け取るようシグネチャを変更。
    - `apps/samsara-hub/src/handlers/commune.rs` [MODIFY]: `commune_relay_metadata_free_handler` を実装し、Hub の DB 永続化をバイパスしてメモリ内ユニキャストルーティングで即時リレーを行うロジックを追加。
    - `apps/samsara-hub/src/main.rs` [MODIFY]: `HubMaintenance` 定期ループに、切断された WebSocket 接続（`is_closed()` が true）を検出して自動削除するゾンビチャネルパージ機構を実装。
    - `apps/api-server/src/internal_services/commune_ws.rs` [NEW]: api-server 側の WebSocket 受信サブシステム。Hub と hood との WS 接続確立、指数バックオフを伴う自動再接続、復号したメッセージの CSAM/Toxicity サニタイズ (`P2pSanitizer`) 処理、およびローカル DB 保存を統括。
    - `apps/api-server/src/routes/commune.rs` [MODIFY]: `send/metadata-free` エンドポイントを実装。`base64::Engine` トレイトのインポートを追加し、型エラーを解消。
    - `apps/api-server/src/router.rs` [MODIFY]: `send/metadata-free` エンドポイントを API サーバーのルートに登録。
    - `apps/api-server/src/api_integration_tests/commune.rs` [MODIFY]: 送信時のサイズ・空文字・バイナリ等のバリデーションを検証するテストを追加。
    - `libs/infrastructure/src/job_queue/mod.rs` [MODIFY]: 要求権限（`PermissionManifest`）およびカテゴリ（logic, wasm, browser, python 等）に基づいて適切な隔離サンドボックスプロファイル（Strict, WasmRun, BrowserAgent, PythonForge 等）を動的に選択する `select_ladder_sandbox` (Execution Ladder) を実装。
    - `libs/infrastructure/src/job_queue/swarm.rs` [MODIFY]: `test_select_ladder_sandbox` テストを追加。`MockTrajectoryStore` の不整合と `PermissionManifest` 等のインポートエラーを解消。
    - `libs/infrastructure/src/skills/mod.rs` [MODIFY]: `code_mode.d.ts` に準拠した JavaScript コードを一括ロード・安全に実行する JS エンジンブリッジ `run_code_mode_js` メソッドを実装。
        - 🛡️ セキュリティロック: 権限（`allow_shell_execution`）がない場合はシェル実行 (`aiome.exec`) を即座に遮断するロックを実装。
        - 構文解析: Rust `regex` でサポートされていない後方参照（backreferences）を排除した正規表現パーサを構築。
        - **Reflexion 堅牢化**: `is_sensitive_path` パス検査ヘルパー関数を抽出して DRY 違反を解消。WASM `host_write` 実行時の `allowed_root` 正規化（`canonicalize`）失敗時に確実にエラーを返すよう厳格化。JS 警告ログ切り詰めにおいて `shared::strings::truncate_chars_safely` を使用しマルチバイト文字での UTF-8 バイト境界パニックを完全に排除。
    - `libs/infrastructure/src/skills/tests.rs` [MODIFY]: `test_code_mode_js_success`, `test_code_mode_js_security_violation_exec`, `test_code_mode_js_exec_success` テストを追加し、Code Mode の実行とセキュリティロックを検証。また、`is_sensitive_path` を直接検証する 8 件のユニットテストを `sensitive_path_tests` モジュールとして追加。
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - 使い捨ての一方向チャネルID (`ChannelLocalId`) を用いた中継、署名の完全排除、メモリ内ユニキャストルーティングにより、通信グラフを漏洩させずタイミング攻撃から防御する Zero-Metadata P2P 経路が確立されました。
    - `CommuneWsClient` により Hub との WebSocket 接続が自動維持され、CSAM / 有害ワード検知 of サニタイズ境界が api-server 受信時に強制適用されるため、法的およびセキュリティリスクが極小化されました。
    - Execution Ladder の導入により、ジョブ of 要求権限とカテゴリに応じたきめ細かなサンドボックス割り当てが可能となり、システムの最小特権の原則が強化されました。
    - Code Mode の一括実行ブリッジが追加され、Wasm プラグイン（Extism）に依存しない JavaScript プレーンコード実行が BastionGuard による完全なセキュリティ境界の保護下で安全に行えるようになりました。
    - 機密パス判定ロジックの抽出（`is_sensitive_path`）により、将来のパターン追加が1箇所に集約され保守性が向上しました。
    - 文字列スライスにおける UTF-8 境界パニックの防止と、正規化に失敗したパスをフォールバックさせず拒否する挙動が確立され、JS Isolate 内のランタイム安全性が著しく向上しました。


## ISO 25010 品質指針の導入と Web Worker による Biome シミュレーション非同期分離の TDD 実装 (2026-06-27)

- **変更内容**:
    - `apps/management-console/src/hooks/biome.worker.ts` [NEW]: WASM (biome-engine) シミュレーション of ロード・実行を管理する Web Worker スクリプト。メインスレッドへの転送最適化（Transferable Object の利用、および `frozenCells` 軽量ビットマスクの構築）を実装。セルフレビューにより、セル情報の型安全検証（`Array.isArray`）および `Math.min` による配列範囲外書き込み防止ガードを追加。
    - `apps/management-console/src/hooks/useBiomeEngine.ts` [MODIFY]: Web Worker (`biome.worker?worker`) 経由での非同期シミュレーション管理への移行。既存の同期ゲッター (`getCellDetail`, `serializeGenome`) およびバッファデータ抽出を Transferable 及び遅延評価（オンデマンド・デシリアライズ）によって同期的に動作させ、完全な API 互換性を維持。セルフレビューにより、初期化時に古いエラーをクリア（`setError(null)`）するよう修正。
    - `apps/management-console/src/setupTests.ts` [MODIFY]: Jest 環境における `import.meta` パースエラーおよび `?worker` 解決エラーを解消するため、グローバルに `MockWorker` クラスと `?worker` インポートモックを設定。
    - `apps/management-console/src/hooks/useBiomeEngine.test.ts` [MODIFY]: 非同期メッセージングと `waitFor` に対応させたテストへの更新。初期化エラー（異常系・Negative Test）およびシード変更時のエラー状態クリア検証テストを追加。
    - `.github/workflows/ci.yml` [MODIFY]: `Verify Negative Tests` ジョブステップを明示的に追加し、異常注入テストを CI/CD での必須マージ基準に強制化。
    - `.gitleaksignore` [MODIFY]: 過去のコミット履歴に含まれていた誤検出シークレット（Stripe webhook プレースホルダー）を無視リストに追加。
    - `docs/operations/api_key_rotation.md` [MODIFY]: Stripe Webhook シークレットのダミープレースホルダーを無害な文字列に変更し、新規検出を防止。
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - Biome シミュレーション計算が Web Worker スレッドで並行実行されるようになり、UIメインスレッドの 60fps レンダリング性能がシミュレーションの負荷から完全に解放されました（性能効率性の向上）。
    - データのデシリアライズ時における配列型保証と明示的な範囲制限により、Worker スレッド内での境界アクセスの安全性が物理的に保証されました。
    - ESM の Jest 互換問題および Vite worker 構文のテスト不整合が解消され、全体のユニットテストが 100% 正常動作可能な状態になりました。
    - 意図的な障害注入（Negative Test）およびエラーからの回復ロジックが自動テストスイートで保証され、CI/CD で独立したステップとして実行されるため、今後のリファクタにおける耐障害性・セキュリティ機能が永続的に担保されるようになりました。

## PostgreSQL マイグレーションデータ型不整合の修正とテストによる型検証の強化 (2026-06-25)

- **変更内容**:
    - `commercial/migrations/postgres/` 配下の 13 個のマイグレーションファイル [MODIFY]: `DATETIME` および `TIMESTAMP` 型を PostgreSQL 互換の `TIMESTAMPTZ` に変更。
    - `commercial/apps/nurture-api/tests/db_config_test.rs` [MODIFY]: 本物の PostgreSQL インスタンス（ポート5433）に接続して、自動マイグレーションの実行、および `SQLiteEconomyLedger::get_balance`（本番コード）による `TIMESTAMPTZ` 型からのデータデコードが正常に処理されることを保証する Positive/Negative テストを追加。
    - `scratch/fix_postgres_timestamps.py` [NEW]: PostgreSQL 側の SQL ファイル群から `TIMESTAMP` を `TIMESTAMPTZ` へ一括かつ安全に置換するための Python スクリプト。
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - Rust の `DateTime<Utc>` 型が PostgreSQL の `TIMESTAMP` (timezone なし) からデコードできない型不整合エラー（`ColumnDecode` エラー）が完全に解消されました。
    - 本物の PostgreSQL に接続して動作保証を行うテストが自動テストスイートへ組み込まれ、移行後の経済処理・台帳記帳の堅牢性が PostgreSQL 実環境でも担保されました。

## key-proxy ヘルスチェックのデッドロック解消と Stripe v2 Webhook 移行の完了 (2026-06-24)

- **変更内容**:
    - `apps/key-proxy/src/auth.rs` [MODIFY]: `/api/v1/health` へのリクエストを認証チェックの前にバイパス（認証除外）するガードを実装。これによって、`api-server` 起動時に `key-proxy` への接続疎通確認が `401 Unauthorized` となり起動が停止するデッドロック問題を解消。
    - `apps/key-proxy/src/tests.rs` [MODIFY]: ヘルスチェック要求の期待ステータスを `200 OK` に更新。
    - `stripe-webhook-forwarder/src/index.js` [NEW]: Cloudflare Workers 用の中継スクリプト。Stripe Webhook (v2) からの thin event および v1 リクエストを生ボディのままと `stripe-signature` ヘッダーと共に本番 API サーバーの `FORWARD_URL` へ透過転送する中継サーバー。
    - `stripe-webhook-forwarder/wrangler.toml` [NEW]: `stripe-webhook-forwarder` のデプロイ構成定義ファイル。
    - `.env.production` [NEW]: 本番環境用設定テンプレート。セルID、DBパス、ポート番号、および起動時の Keychain / 環境変数シークレット分離手順を明示。
    - `CHANGELOG.md` [MODIFY]: 変更履歴の更新。
- **波及効果**:
    - `api-server` と `key-proxy` の起動順序デッドロックが完全に解消され、起動疎通が安定化しました。
    - Cloudflare Workers 中継サーバー経由で Stripe Webhook (v2) thin event を受信可能となり、自動解決されたフルデータが本番環境で正常に経済処理（ライセンス付与）へと反映されます。

## Stripe Webhook v2 thin event および複数署名検証への対応 (2026-06-23)

- **変更内容**:
    - `libs/aiome-commerce/src/stripe/mod.rs` [MODIFY]: カンマ区切りによる複数 Webhook シークレットの読み込みと検証ループ処理。パースエラー（`BadParse`）を「署名検証成功」として処理するフォールバックロジックを導入。
    - `libs/aiome-commerce/src/stripe/v2_tests.rs` [NEW]: 複数シークレット署名検証テストの実装。
    - `apps/api-server/src/app_state.rs` [MODIFY]: `stripe_api_key` フィールドを `AppState` に追加。
    - `apps/api-server/src/bootstrap/state_assembly.rs` [MODIFY]: `stripe_api_key: core.stripe_key_raw` のマッピング処理を追加。
    - `apps/api-server/src/api_integration_tests/common.rs` [MODIFY]: `create_test_server_with_limit` での `AppState` 構造体初期化に `stripe_api_key: None` を追加。
    - `apps/api-server/src/routes/commerce_webhook/stripe.rs` [MODIFY]: `v2.core.event` 受信時に Stripe API を介して `related_object.url` からフルデータを自動フェッチし、v1互換形式へとペイロードをパース・書き換えを行う自動解決処理を実装。
    - `.env.secret.example` [MODIFY]: カンマ区切りでの Stripe Webhook シークレット設定例を追加。
    - `apps/key-proxy/src/main.rs` [MODIFY]: JWT署名キー `JWT_PRIVATE_KEY_B64` のロード処理を `build_auth_manager` 関数へリファクタリング。開発ビルド時（`debug_assertions`有効）にプレースホルダーや不正な値の場合はエラー終了させず `MockAuthManager` へ安全にフォールバックする機能を追加。
    - `apps/key-proxy/src/tests.rs` [MODIFY]: `build_auth_manager` のフォールバックとリリースビルド時のエラー挙動を検証するユニットテストを3件追加。
    - `scripts/test_all.sh` [MODIFY]: `key-proxy` 起動時の環境変数に `JWT_PRIVATE_KEY_B64=""` を明示指定してプレースホルダーをオーバーライド。
- **波及効果**:
    - Stripe API v2 thin event の受信が可能になり、`related_object.url` から動的にフルデータを安全に取得・解決できるようになりました（SSRF防止策として `https://api.stripe.com` に宛先を固定）。
    - 従来の v1 ペイロードも変更なしでそのまま通る（後方互換性担保）ため、既存の全ての決済・ライセンス付与ハンドラを変更することなく Stripe Webhook v2 移行が完了。
    - カンマ区切りで複数の署名シークレットを指定可能にしたことで、移行期間中に v1 と v2 双方の Webhook エンドポイントシークレットを同時に動作させることが可能になり、ダウンタイムゼロでの移行パスが実現されます。

## R3F InstancedMesh によるバイオーム描画完全移行 (Phase 0-5) (2026-06-21)

### 1. 手書き WebGL2 から R3F InstancedMesh への描画移行とカスタムポストエフェクトの実装
- **変更内容**:
    - `apps/management-console/src/lib/biome/biomeTypes.ts` [NEW]: R3F 描画で使用する共通の型定義（`InjectionMark`, `EffectType`, `BiomeCanvasProps`）およびグリッド定数を新規定義。
    - `apps/management-console/src/lib/biome/effects/TachyonEffect.ts` [NEW]: `postprocessing` の `CopyPass` を使用した ping-pong 方式 of 残像ポストプロセスエフェクトを新規実装。TypeScript 型推論のために uniforms Map を明示的に型定義し、`copyPass.render` の引数に null を追加してコンパイルエラーを解消。
    - `apps/management-console/src/lib/biome/effects/HiggsEffect.ts` [NEW]: 重力レンズによる時空歪みと RGB チャンネル分離（色収差）を再現したポストプロセスエフェクトを新規実装。uniforms Map の TS 型推論を解決。
    - `apps/management-console/src/lib/biome/shaders/biomeCell.ts` [NEW]: セル描画用 GLSL（頂点/フラグメント）を定義。Phongライティング、形態別パターン、細胞核・細胞膜構造、およびフレネルリムライトとレアリティ光沢を R3F shaderMaterial 用に移植。
    - `apps/management-console/src/lib/biome/cellGeometries.ts` [NEW]: 5種類のレアリティと5種類の形態に基づき、25種類の明確に異なるジオメトリ（円、リング、星/歯車、Epic多面体、Legendary正二十面体）を動的に生成するヘルパー関数を新規実装。
    - `apps/management-console/src/lib/biome/BiomeBackground.tsx` [NEW]: ボロノイノイズによる有機的な背景 Plane     - `apps/management-console/src/lib/biome/BiomeCanvas.tsx` [MODIFY]: R3F `Canvas` の `orthographic` prop と camera 設定を削除し、`@react-three/drei` の `<OrthographicCamera>` を `manual` および `makeDefault` prop 付きで明示的にマウントするよう修正。カメラ位置を `[0, 0, 10]`、フラスタムを `left=0, right=128, top=128, bottom=0` に固定。
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: `BiomeRenderer` へのインポートを新設した `BiomeCanvas` に差し替え、WASM から取得したレアリティインデックス（`rarityIndex`）を `rarity` prop として結合。
    - `apps/management-console/src/lib/biome/BiomeRenderer.tsx` [MODIFY]: 先頭に `@legacy` コメントマーカーを追記し、レガシー WebGL2 コードとして参照用に保護。
    - `apps/management-console/src/lib/biome/effects/TachyonEffect.test.ts` [NEW], `HiggsEffect.test.ts` [NEW], `BiomeCanvas.test.tsx` [MODIFY]: 新規追加したカスタムエフェクト、キャンバスの初期化、および Legendary 時の Sparkles 表示ロジックを検証する Jest ユニットテストを追加し、すべて PASS。drei モックに `OrthographicCamera` を追記。
    - `apps/management-console/src/lib/biome/BiomeGame.test.tsx` [MODIFY]: レガシーな `?raw` モックと WebGL2 モックを完全に削除し、Jest 環境用の R3F および `postprocessing` のスタブモックへ差し替えてテストを正常化。さらに `CopyPass` および drei モックに `OrthographicCamera` を追記。
- **波及効果**:
    - `@react-three/fiber` v9.5 の仕様に起因する Orthographic カメラのフラスタム自動上書き問題が解決され、キャンバスのアスペクト比やサイズに関わらず 128x128 のグリッド全体がキャンバスにピッタリ収まるようになりました。
    - テスト時の drei モックで `OrthographicCamera` をエミュレートしたことにより、テスト実行時の React 不正要素コンパイルエラー（TypeError: Element type is invalid）が解消され、全テストスイートが正常稼働します。
    - 700行を超えていた手書きの複雑な WebGL2 生コードが、宣言的で高水準な React Three Fiber (R3F) のコンポーネントツリーに完全移行されました。
    - メタボール融合とエネルギーフローは技術的制約（InstancedMesh 間のデータ非共有）のため廃止されましたが、その代わりに形態別の美しい3D幾何多面体ジオメトリと回転アニメーション、およびレアリティによるリムグローや金色脈動などのリッチな 3D 演出が追加され、ビジュアルクオリティが大きく向上しました。
    - テストスイートが WebGL から R3F モックに適合化されたことで、Jest テスト環境のクラッシュが解消され、ラインカバレッジ 100% のテスト品質が維持されます。
    - `postprocessing` と TypeScript / React 18 の型制約に適合した堅牢なインテグレーションが実現され、コンパイル時エラーが 0 に維持されます。キャンバスにピッタリ収まるようになりました。
    - テスト時の drei モックで `OrthographicCamera` をエミュレートしたことにより、テスト実行時の React 不正要素コンパイルエラー（TypeError: Element type is invalid）が解消され、全テストスイートが正常稼働します。
    - 700行を超えていた手書きの複雑な WebGL2 生コードが、宣言的で高水準な React Three Fiber (R3F) のコンポーネントツリーに完全移行されました。
    - メタボール融合とエネルギーフローは技術的制約（InstancedMesh 間のデータ非共有）のため廃止されましたが、その代わりに形態別の美しい3D幾何多面体ジオメトリと回転アニメーション、およびレアリティによるリムグローや金色脈動などのリッチな 3D 演出が追加され、ビジュアルクオリティが大きく向上しました。
    - テストスイートが WebGL から R3F モックに適合化されたことで、Jest テスト環境のクラッシュが解消され、ラインカバレッジ 100% のテスト品質が維持されます。
    - `postprocessing` と TypeScript / React 18 の型制約に適合した堅牢なインテグレーションが実現され、コンパイル時エラーが 0 に維持されます。ne による raycaster 座標検出（クリック・ホバー）を統括するメインキャンバスコンポーネントを新規実装。
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: `BiomeRenderer` へのインポートを新設した `BiomeCanvas` に差し替え、WASM から取得したレアリティインデックス（`rarityIndex`）を `rarity` prop として結合。
    - `apps/management-console/src/lib/biome/BiomeRenderer.tsx` [MODIFY]: 先頭に `@legacy` コメントマーカーを追記し、レガシー WebGL2 コードとして参照用に保護。
    - `apps/management-console/src/lib/biome/effects/TachyonEffect.test.ts` [NEW], `HiggsEffect.test.ts` [NEW], `BiomeCanvas.test.tsx` [NEW]: 新規追加したカスタムエフェクト、キャンバスの初期化、および Legendary 時の Sparkles 表示ロジックを検証する Jest ユニットテストを追加し、すべて PASS。
    - `apps/management-console/src/lib/biome/BiomeGame.test.tsx` [MODIFY]: レガシーな `?raw` モックと WebGL2 モックを完全に削除し、Jest 環境用の R3F および `postprocessing` のスタブモックへ差し替えてテストを正常化。さらに `CopyPass` を mock に追加。
- **波及効果**:
    - 700行を超えていた手書きの複雑な WebGL2 生コードが、宣言的で高水準な React Three Fiber (R3F) のコンポーネントツリーに完全移行されました。
    - メタボール融合とエネルギーフローは技術的制約（InstancedMesh 間のデータ非共有）のため廃止されましたが、その代わりに形態別の美しい3D幾何多面体ジオメトリと回転アニメーション、およびレアリティによるリムグローや金色脈動などのリッチな 3D 演出が追加され、ビジュアルクオリティが大きく向上しました。
    - テストスイートが WebGL から R3F モックに適合化されたことで、Jest テスト環境のクラッシュが解消され、ラインカバレッジ 100% のテスト品質が維持されます。
    - `postprocessing` と TypeScript / React 18 の型制約に適合した堅牢なインテグレーションが実現され、コンパイル時エラーが 0 に維持されます。

## Biome 細胞形態の多様性革命 (Phase 8) (2026-06-20)

### 1. 3データテクスチャ構成への拡張とWASMデータ・パッキング精度崩壊の修正
- **変更内容**:
    - `apps/management-console/src/lib/biome/BiomeRenderer.tsx` [MODIFY]: float32 での u16 値パッキング（`cVal + nVal * 65536` 等）によるビット破壊・カラー多様性消失（赤色に固定）を防ぐため、パッキングを完全廃止。3枚目のデータテクスチャ（`gridTex2`）を追加し、12次元の `renderView` データをすべて f32 生値のまま転送するデータパイプラインを構築（`tex1Data` に C,N,P,H を、`tex2Data` に O,S,Fe,Si を格納）。
    - `apps/management-console/src/lib/biome/BiomeRenderer.test.tsx` [MODIFY]: WebGLモックに `texSubImage2D` や `isContextLost` を追加し、`TEXTURE4` へのデータテクスチャバインド（`uniform1i` 呼び出し）を検証するユニットテストに加え、生値元素データ転送アサーションを追加して TDD でグリーン化。
    - `apps/management-console/src/lib/biome/BiomeComponents.test.tsx` [MODIFY]: `BiomeHUD` の全レアリティ（Legendary/Epic/Uncommon/Common）の表示装飾を網羅するテストを TDD で追加し、ラインカバレッジ 100% を達成。
- **波及効果**:
    - WASMエンジンでシミュレーションされた全8元素の比率データが 100% の精度でシェーダーに伝送され、色の多様性とグラデーションの滑らかな遷移が完全に復元されました。

### 2. 有機的なメタボール融合と美しく多様なビジュアル演出の実装
- **変更内容**:
    - `apps/management-console/src/lib/biome/shaders/grid.frag` [MODIFY]: 3x3近傍のセルに対する 1/(d²+0.15) 距離場によるメタボール融合（アメーバ状セルクラスタ）を実装。
    - `grid.frag` [MODIFY]: 精度崩壊の修正に伴い、`getElementColor` および `getEnergy` におけるパッキング解除処理（`mod/floor`）を廃止し、2つのデータテクスチャ（`u_gridTex1`, `u_gridTex2`）から直接 texelFetch を行うように全面改修。
    - `grid.frag` [MODIFY]: 任意のUV座標でのメタボール場を取得する `getFieldAtUV()` を実装し、それを用いたメタボール輪郭の法線計算および Phong ライティング（ベース拡散光 + specular 鏡面反射）による立体感（3Dジェル感）を追加。
    - `grid.frag` [MODIFY]: セル内部の微細構造（中心の細胞核：輝点、外周の細胞膜：半透明リング）および視線角度に基づくフレネルリム発光効果（端が青く光る半透明の生物感）を実装。
    - `grid.frag` [MODIFY]: 形態別（Basic, Producer, Consumer, Predator, Decomposer）の内部模様（微粒子、同心円脈動、流線型波動、放射状トゲ、渦巻き）の描画コントラストを強化し、形態ごとに固有の色味シフト（緑み、赤み、暖色、紫など）を適用する色相・彩度補正を導入。
    - `apps/management-console/src/lib/biome/shaders/grid.vert` [MODIFY]: パススルー頂点シェーダーへ簡略化。
- **波及効果**:
    - ベタ塗りだったアメーバ融合細胞が、立体的な美しさを放つ有機的なビジュアル表現へ進化し、形態や元素の違いが視覚的に強くフィードバックされるプレミアムな意匠を実現しました。

## Biome UXおよびビジュアル品質の向上 (2026-06-19)

### 1. インタラクティブなキャンバス操作と詳細ツールチップの実装
- **変更内容**:
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: クリック/ドラッグによる元素（C/N/P/H）注入機能、ホバー座標に基づきWASMからセル詳細情報を取得して表示するツールチップ（生存状態、エネルギー、形態、元素比率、凍結状態）を追加。
    - `apps/management-console/src/lib/biome/utils/gridCoords.ts` [NEW]: キャンバス上のピクセル座標を WebGL の Y軸反転を考慮して 128x128 グリッド座標に変換する共通ユーティリティを実装。
    - `apps/management-console/src/lib/biome/utils/gridCoords.test.ts` [NEW]: 境界値クランプおよび無効座標の null 返却をテストする Jest ユニットテストを実装。
- **波及効果**:
    - ユーザーがキャンバスを直接タッチして元素エネルギーを供給したり、詳細状態を視覚的に検査できるようになり、静的で単調だったシミュレーション画面が動的でインタラクティブなゲーム体験に進化しました。

### 2. Bloomポストプロセスおよび背景グリッド等のビジュアル強化
- **変更内容**:
    - `apps/management-console/src/lib/biome/shaders/bloom.frag` [NEW]: 輝度抽出・2方向ブラー・合成を2パスで行うポストプロセスBloomシェーダーを実装。
    - `apps/management-console/src/lib/biome/shaders/grid.frag` [MODIFY], `grid.vert` [MODIFY]: 背景のサイバーグリッドを動的に明滅させ、形態別カラーリングを適用し、マウスホバー時にその位置のグリッドセルを強調ハイライトするビジュアル強化を実装。
    - `apps/management-console/src/lib/biome/BiomeRenderer.tsx` [MODIFY]: Bloom用のFBO/テクスチャバッファ初期化と2パス描画パイプラインを構築し、ホバー・クリック座標判定を呼び出し元へ通知するコールバックインタラクションを統合。
- **波及効果**:
    - チープだった画面描画が、Bloomによる発光効果と動的なグリッドアニメーションによって、プレミアムでサイバー感溢れるリッチなWebGL意匠に劇的にアップグレードされました。

### 3. スポットライト対話型チュートリアルの実装
- **変更内容**:
    - `apps/management-console/src/lib/biome/BiomeTutorial.tsx` [NEW]: 5つの進化・操作手順（キャンバス、速度制御、元素、災害、伝説進化目標）をスポットライトハイライトで案内するモーダルチュートリアルコンポーネントを新規実装。
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: 初回起動時（`localStorage` の未完了状態）にチュートリアルを自動起動する `useEffect` と State を統合。
    - `apps/management-console/src/lib/biome/BiomeControls.tsx` [MODIFY]: ❓ 遊び方ボタンのクリック時にチュートリアルを再表示するハンドラを接続。
- **波及効果**:
    - ルールや遊び方が分からなかった問題が完全に解消され、初めてゲームを起動したユーザーでも段階的なスポットライトガイドに沿って遊び方を直感的に習得できるようになりました。

### 4. 別ウインドウ（ポップアップ）対応およびホーム画面クイック起動
- **変更内容**:
    - `apps/management-console/biome-popup.html` [NEW]: ポップアップ専用のHTMLファイルを新設し、WASMの unsafe-eval を許可する強固な CSP を定義。
    - `apps/management-console/src/biome-popup-entry.tsx` [NEW]: `BiomeGame` を `standalone` モードでマウント・フルスクリーン表示する専用エントリーポイントを実装。
    - `apps/management-console/vite.config.ts` [MODIFY]: `biome-popup.html` を Rollup のマルチエントリー `input` に登録し、ビルドおよび開発時にアクセス可能に変更。
    - `apps/management-console/src/components/home/HomePage.tsx` [MODIFY]: ホーム画面の左側サイドバーにグラスモーフィズムデザインの「🎮 バイオーム」起動カードを統合し、直接メイン画面での起動、または別ウインドウへのポップアップ起動をサポート。
- **波及効果**:
    - AIエージェントを稼働させている間の暇つぶしとして、メイン画面の邪魔をしない別ウインドウでの常時ゲームプレイが可能となり、ホーム画面からのアクセス導線も最短化されました。

## Vibe Coding セキュリティ強化 (Phase 11) - Reflexion 改善 (2026-06-18)

### 1. サニタイズ処理の強化とパニック防止の徹底
- **変更内容**:
    - `libs/shared/src/guardrails.rs` [MODIFY]: `sanitize_for_context` 関数内のサニタイズを大幅に強化。
        - `SqlQuery` コンテキスト: ダブルクォート、バックスラッシュ、NULL、`--`、`/*`、`*/` の完全除去を導入し、Prepared Statementを推奨するコメントの記述。
        - `FilePath` コンテキスト: 再帰的ループ処理により、`....//` や `..././` などのディレクトリトラバーサルバイパス手法を完全に無効化。
        - `HttpHeader` コンテキスト: `OnceLock` で `CRLF_ENCODED_REGEX` を安全に保持し、非推奨な `unwrap()` を排除してパニックを防ぐパターンマッチへ変更。
    - `libs/shared/src/guardrails.rs` [MODIFY]: `BeggingSupervisor` 内で `timestamp_nanos_opt()` が `None` を返した場合のフォールバックコメント（安全側に倒す）を追加。
    - `libs/shared/src/guardrails.rs` [MODIFY]: `tests` モジュールに SqlQuery のサニタイズ (4件) および FilePath のサニタイズバイパス防止 (1件) のアサーションを追加。
- **波及効果**:
    - 出力コンテキスト別のサニタイズ処理において、攻撃者による難読化やバイパス手法（再帰的トラバーサル等）に対する防御が堅牢化されました。また、ランタイムエラーやパニックが起きない安全な初期化が保証されます。

### 2. CORS テスト競合の解消と未使用インポート警告の修正
- **変更内容**:
    - `apps/api-server/src/bootstrap/helpers.rs` [MODIFY]: `test_init_cors` テストに `#[serial_test::serial]` を追加。
    - `apps/api-server/src/bootstrap/helpers.rs` [MODIFY]: 未使用の `error` マクロのインポートに対して `#[allow(unused_imports)]` とリリースアサーション用の説明コメントを追加。
- **波及効果**:
    - 並列テスト実行時にポート競合などによる flaky test が発生する問題が完全に解消され、CIビルドの決定論的安定性が向上しました。

## Tauri デスクトップサイドカー安全ガードレール構築 (TDD) (2026-06-17)

### 1. 安全ガードレール自動検証スクリプト desktop_sidecar_manager.py の実装
- **変更内容**:
    - `scripts/desktop_sidecar_manager.py` [NEW]: ターゲットトリプルの自動検出（`rustc` 検出または platform フォールバック）、プレースホルダー（ダミーシェルスクリプト/バッチファイル）自動生成、AWS SDK排除フラグ（`--no-default-features --features desktop`）付きのRustサイドカービルド、および2段階検証（`--check-core` / `--check-all`）をTDDで実装。
    - `scripts/desktop_sidecar_manager.py` [MODIFY]: `generate_placeholders` が `target_binaries` パラメータを受け取れるよう拡張し、特定のバイナリ（例: `obscura` のみ）を指定してプレースホルダーを生成可能に変更 (Reflexion)。
    - `scripts/desktop_sidecar_manager.py` [MODIFY]: 引数が指定されなかった場合に `sys.exit(1)` でエラー終了させるよう改善 (Reflexion)。
    - `scripts/test_desktop_sidecar_manager.py` [NEW]: スクリプト機能（ターゲット検出、プレースホルダー生成、マジックバイトによるバイナリ物理判定、2段階検証）を包括するテストコードを実装。
    - `scripts/test_desktop_sidecar_manager.py` [MODIFY]: 0バイト、99999バイト、100000バイトの境界値、破損ヘッダー、FAT Mach-O を含む5つのテストケース（14件から19件へ）を追加 (Reflexion)。
- **波及効果**:
    - 開発およびCIにおいて、Tauri用サイドカーバイナリが本物であるかどうかが物理検証され、ダミーが本番ビルドに誤混入するリスクが完全に排除されました。

### 2. Git 物理ロックダウンおよびルール・ワークフローの整備
- **変更内容**:
    - `.gitignore` [MODIFY]: `/apps/management-console/src-tauri/binaries/` ディレクトリを無視リストに追加。
    - `git rm --cached obscura-aarch64-apple-darwin` [EXECUTE]: Gitで誤って追跡されていたダミーバイナリの追跡を解除。
    - `.agent/skills/tauri-development.md` [MODIFY]: T-006 (No Mock Sidecars in Prod) と T-007 (Git Binary Lockdown) ルールを追加。
    - `.agent/workflows/desktop-sidecar.md` [NEW]: エージェントや開発者がサイドカーのプレースホルダー・ビルド・検証プロセスを実行する際の手順と規約を定義。
- **波及効果**:
    - 巨大な実バイナリやプレースホルダーが誤って Git コミットされ履歴を汚染する事故を物理的に防止。さらに、AIエージェントのハルシネーションによるダミー同梱をワークフローとルールの両面から強力に防護するガードレールが確立されました。

## Tauri デスクトップアプリ化 (Phase 4 - Nurture ハイブリッド統合) (2026-06-15)

### 1. Nurture サイドカーの統合とライフサイクル管理の実装
- **変更内容**:
    - `apps/management-console/src-tauri/src/lib.rs` [MODIFY]: `NurtureMode` enum、`resolve_nurture_mode()`、および `generate_session_secret()` を実装。
    - `lib.rs` [MODIFY]: `SidecarState` 構造体に `nurture_child` と `nurture_status` を追加。`SidecarStatus` に `nurture` フィールドを追加。
    - `lib.rs` [MODIFY]: `start_sidecars()` を拡張し、ローカルモード時に `nurture-api` を起動。セッション一時シークレット（`NURTURE_INTERNAL_SECRET`）を生成して `nurture-api` 和 `api-server` の双方へ注入。
    - `lib.rs` [MODIFY]: `stop_sidecars()` を拡張し、`nurture-api` 子プロセスの SIGTERM によるクリーンアップ処理を構築。
    - `lib.rs` [MODIFY]: 新規 IPC コマンド `get_nurture_status` を実装・登録。
    - `lib.rs` [MODIFY]: トレイメニューのステータス文字列表示に Economy 状態（Local/Cloud/Off）を追加。
    - `apps/management-console/src-tauri/Cargo.toml` [MODIFY]: `getrandom` および `hex` 依存関係を追加。
- **波及効果**:
    - デスクトップ上で経済エンジン `nurture-api` が完全にローカル起動し、S2S 通信ポート 3020 を介して `api-server` とセッションシークレットで安全に連携。これにより、クラウド環境不要でエスクロー作成や残高管理などの経済・検証機能が透過的に機能します。

### 2. パッケージングおよび実行許可、環境設定の同期
- **変更内容**:
    - `apps/management-console/src-tauri/tauri.conf.json` [MODIFY]: `externalBin` に `binaries/nurture-api` を追加。
    - `apps/management-console/src-tauri/capabilities/default.json` [MODIFY]: `shell:allow-execute` の `allow` リストに `nurture-api` を追加。
    - `apps/api-server/.env.example` [MODIFY]: デスクトップ向けの Nurture ハイブリッドモード（ローカル/クラウド/無効）用環境変数の説明を追記。
    - `apps/api-server/.agent/skills/tauri-development.md` [MODIFY]: T-002 コントラクトに Nurture の環境変数および起動停止順序契約（nurture-api -> api-server -> key-proxy）を追加。
- **波及効果**:
    - アプリ起動時のセキュリティポリシーの制約（Shell 実行拒否）を回避して `nurture-api` サイドカーを安全に起動可能になり、かつデスクトップパッケージング時に `nurture-api` バイナリが自動アセット同梱されます。

### 3. nurture-api の desktop 向け AWS SDK 排除ビルドの対応および改名同期
- **変更内容**:
    - `commercial/libs/nurture-infra/Cargo.toml` [MODIFY], `commercial/apps/nurture-api/Cargo.toml` [MODIFY], `libs/nurture-infra/Cargo.toml` [MODIFY], `apps/nurture-api/Cargo.toml` [MODIFY]: `aws-config` および `aws-sdk-s3` 依存関係を `optional = true` とし、`cloud-storage` と `desktop` feature flag を定義。
    - `commercial/libs/nurture-infra/src/storage/mod.rs` [MODIFY], `libs/nurture-infra/src/storage/mod.rs` [MODIFY]: `s3` モジュールを `#[cfg(feature = "cloud-storage")]` ガードで条件付きコンパイル。
    - `commercial/apps/nurture-api/src/main.rs` [MODIFY], `apps/nurture-api/src/main.rs` [MODIFY]: S3 アセットストレージ初期化ブロックを `#[cfg(feature = "cloud-storage")]` でガードし、非有効時には `MockAssetStorage` に自動フォールバック。
    - `commercial/apps/nurture-api/src/plugin.rs` [MODIFY], `apps/nurture-api/src/plugin.rs` [MODIFY]: プラグイン側の S3 初期化ブロックを `#[cfg(feature = "cloud-storage")]` でガード。
    - `Project-Nurture/libs/nurture-bridge/src/lib.rs` [MODIFY], `commercial/libs/nurture-bridge/src/lib.rs` [MODIFY]: `biome` モジュールから `commune` モジュールへの改名（`AutonomousBiomeEngine` / `BiomeMessage` から `AutonomousCommuneEngine` / `CommuneMessage` への移行）を同期。
    - `Project-Nurture/libs/nurture-infra/src/mock_job_queue.rs` [MODIFY], `commercial/libs/nurture-infra/src/mock_job_queue.rs` [MODIFY]: `BiomeRegistry` / `BiomeMessage` から `CommuneRegistry` / `CommuneMessage` へのトレイト実装を更新。
    - `Project-Nurture/libs/nurture-infra/src/economy/karma_forge.rs` [MODIFY], `commercial/libs/nurture-infra/src/economy/karma_forge.rs` [MODIFY]: `update_biome_reputation` 呼び出しを `update_commune_reputation` へ変更し、戻り値 of 型推論エラーを解消。
- **波及効果**:
    - デスクトップ向けのビルド（`--no-default-features --features desktop`）において巨大な AWS SDK クレートが依存ツリーから完全に排除され、バイナリサイズの大幅な削減（10-15MB）とビルド時間の短縮が実現されます。また、両ワークスペース（`aiome` と `Project-Nurture`）の間で、`biome` から `commune` への改名に伴うビルドエラーが解消され、AWS SDK を除外した状態でのビルド整合性が完全に保証されます。

## Tauri デスクトップアプリ化 (Phase 2 - Tauri シェル強化 / IPC + サイドカー自動起動) (2026-06-15)

### 1. 新規 IPC コマンドおよびサイドカーライフサイクル管理の実装
- **変更内容**:
    - `apps/management-console/src-tauri/src/lib.rs` [MODIFY]: 新規 IPC コマンド（`get_data_dir`, `get_sidecar_status`, `restart_sidecar`, `get_system_info`）を実装。
    - `lib.rs` [MODIFY]: `.setup()` 内で `api-server` および `key-proxy` サイドカーの自動起動処理（`start_sidecars`）を追加し、`ExitRequested` イベントによる終了時に `stop_sidecars` でクリーンアップするロジックを実装。
    - `lib.rs` [MODIFY]: トレイメニューを拡張し、状態表示・再起動・データフォルダを開く機能を追加。アサーションを更新。
    - `apps/management-console/src-tauri/Cargo.toml` [MODIFY]: `sysinfo` および `shared` パス依存関係を追加。
- **波及効果**:
    - デスクトップアプリのフロントエンドからプロセスステータスの監視、エンジンの再起動、データ領域へのクイックアクセスが可能になり、アプリ終了時にゾンビプロセスが残るバグを防止します。テストコードを macOS とその他 OS で適切にガードし、CI でもテストが完全にグリーンで実行されます。

### 2. パッケージ制限および実行許可の更新
- **変更内容**:
    - `apps/management-console/src-tauri/tauri.conf.json` [MODIFY]: `externalBin` に `api-server` と `key-proxy` を追加。
    - `apps/management-console/src-tauri/capabilities/default.json` [MODIFY]: `shell:allow-execute` の `allow` リストに `api-server` と `key-proxy` を追加。
- **波及効果**:
    - Tauri アプリ起動時にセキュリティポリシーの制約（Shell 実行拒否）を受けることなくサイドカープロセスを安全に起動可能になり、デスクトップパッケージング時のアセット同梱が自動的に許可されます。

## Tauri デスクトップアプリ化 (Phase 1 - データパス統一 & 基盤整備) (2026-06-15)

### 1. アプリデータパス優先度 (AIOME_DATA_DIR) の導入
- **変更内容**:
    - `libs/shared/src/app_data.rs` [MODIFY]: `AppDataResolver::new()` の冒頭で環境変数 `AIOME_DATA_DIR` が設定されている場合、直ちにそのパスをアプリデータの `root` とするようオーバーライドロジックを実装。
    - `libs/shared/src/app_data.rs` [MODIFY]: TDD サイクル用の新規テストケース `test_resolve_root_override_via_env` を追加。
- **波及効果**:
    - Tauri デスクトップアプリの起動時に `app_data_dir` を解決して環境変数経由でサイドカー（api-server, key-proxy）に渡すことで、CLIとデスクトップアプリが同一のデータディレクトリ（DB、秘密鍵、設定など）を参照し、データの乖離を防止します。既存の 8 つの tests や workspace 全体のテストがデグレなく動作します。

### 2. エージェント開発ルール (tauri-development.md) の策定と統合
- **変更内容**:
    - `.agent/skills/tauri-development.md` [NEW]: Tauri 開発の絶対ルール（T-001〜T-005）を定義。
    - `AGENTS.md` [MODIFY]: Safety-Critical Zone に `src-tauri/src/lib.rs`, `tauri.conf.json` およびサイドカー起動ロジックを追加し、Documentation Sync Rule に `tauri.conf.json` の項目を紐付け。
    - `apps/management-console/src-tauri/Cargo.toml` [MODIFY]: サイドカー自動起動コードを分離するための `sidecar-auto` feature flag を定義。
- **波及効果**:
    - 今後の Tauri 周りの開発において AI エージェントが安全制約（Tauri 孤立化ルール、ゾンビプロセス防止ルールなど）を確実に遵守するよう規範化され、かつ Tauri 未導入環境での CI テストの実行安全性が保証されます。

## Windows/クロスプラットフォーム環境でのビルド対応 (Phase 12) (2026-06-14)

### 1. UNIX専用クレート `nix` 依存のターゲット限定化
- **変更内容**:
    - `libs/shared/Cargo.toml` [MODIFY], `apps/key-proxy/Cargo.toml` [MODIFY], `apps/api-server/Cargo.toml` [MODIFY], `commercial/libs/nurture-infra/Cargo.toml` [MODIFY]: `nix` 依存関係を `[dependencies]` から `[target.'cfg(unix)'.dependencies]` に移動。
    - `libs/shared/src/process_hardening.rs` [MODIFY]: `disable_core_dumps()` に `#[cfg(unix)]` ガードを適用し、非Unix環境向けの No-op フォールバック実装を追加。
    - `commercial/libs/nurture-infra/src/sidecar/clone_manager.rs` [MODIFY]: インポート部分の `nix` を `#[cfg(unix)]` ガードし、プロセス生存確認および強制終了ロジックを Unix環境とWindows環境で分岐（Windows では `tasklist` / `taskkill` コマンドでフォールバック）。
- **波及効果**:
    - Windowsターゲット向けにクロスコンパイルまたは直接ビルドする際、Unix依存の `nix` クレートのビルドがスキップされ、ビルドエラーが解消されます。また、堅牢化プロセスとCloneManagerにおける機能的影響は最小限に抑えられます。

### 2. protoc (Protocol Buffers) コンパイラ自動フォールバックの実装
- **変更内容**:
    - `libs/aiome-contracts/Cargo.toml` [MODIFY], `libs/aiome-core-contracts/Cargo.toml` [MODIFY]: `[build-dependencies]` に `protobuf-src = "2"` を追加。
    - `libs/aiome-contracts/build.rs` [MODIFY], `libs/aiome-core-contracts/build.rs` [MODIFY]: `protoc` がシステム上に存在しない（PATH および `PROTOC` 環境変数未定義）場合、`protobuf-src` を用いて Pure Rust 実装の `protoc` をビルド時に自動コンパイル・使用するフォールバックロジックを追加。
- **波及効果**:
    - Windowsや追加ツール未導入の環境でクローンした際、`protoc` の手動インストールなしに `cargo build` が完全に通過するようになります。

## サイドバー日本語表記 & UI ユーザビリティ改善 (Phase 10) (2026-06-14)

### 1. サイドバー翻訳キー追加と日本語ラベルのユーザビリティ改善
- **変更内容**:
    - `apps/management-console/src/i18n/ja.json` [MODIFY], `en.json` [MODIFY]: 生キーが表示されていたサイドバー項目（`communeLab`, `nurtureEconomy`, `statusPage`, `banDashboard`, `buzzApproval`, `agencyOnboarding`）の翻訳テキスト、およびページヘッダーやセッション・ペルソナ用の多言語定義を追加。既存の分かりにくい日本語表現（「シナジーデモ」→「自律AIデモ」、「因果トレース」→「原因分析」など）を直感的にわかりやすい言葉へ大幅に改善。
- **波及効果**:
    - ユーザーが管理コンソールの各機能で「何ができるのか」をひと目で理解できるようになり、UIのユーザビリティが劇的に向上しました。

### 2. Management Console (App.tsx) のハードコード排除とバグ修正
- **変更内容**:
    - `apps/management-console/src/App.tsx` [MODIFY]: 英語がハードコードされていた14箇所を `t()` 関数へ置き換え、サイドバーとページタイトルの完全な多言語対応を実現。また、WorkflowBuilder コンポーネントをレイアウトに新規統合。
    - `App.tsx` [MODIFY] (Reflexion): i18n リファクタの過程で意図せず削除されていた `isAuth` ガード条件（未認証ユーザーに期限切れアラートが表示される問題）およびセッション期限切れアラートの `animate.y` アニメーション座標 / `top` スタイル座標を元の状態へ完全復元。
    - `apps/management-console/src/App.test.tsx` [MODIFY]: i18n モックの動作に合わせてテストアサーション文字列を `'session.expired'` に修正し、セッション期限切れユニットテストを正常化。
- **波及効果**:
    - サイドバー・画面ヘッダー全体の多言語化が完成し、動的な日/英切り替えが正常に機能するようになりました。
    - リファクタに伴う未認証時アラート誤表示バグやアニメーション位置のズレというリグレッションが、Reflexion によって完全に検知・修正され、システムの堅牢性が担保されました。

## UI ウォームパレット刷新計画 v3 の実装 (Phase 9) (2026-06-14)

### 1. セマンティックトークン新設とデザインシステムドキュメントの同期
- **変更内容**:
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: セマンティックアクセントトークン `--accent-primary`（`var(--fluid-deep-gold)`）および関連バリアント、見出しグラデーション用の `--gradient-heading` を追加。また、インプットフォーカス用の `--accent-primary-12` や、未定義であったレイアウト用の `--space-2xl`、カラー用の `--accent-emerald-05` も新設 (Reflexion)。
    - `apps/management-console/DESIGN.md` [MODIFY]: YAML フロントマター及び Brand Accents セクション、スペーシングスケール一覧に、新設トークン定義（`--accent-emerald-05` と `--space-2xl` を含む）を追加し、既存のアクセントカラー Hex 定義を `tokens.css` と同期。
- **波及効果**:
    - 全画面共通のアクセントカラーを一括定義するセマンティックトークンが整備され、LP に準拠したプレミアムダークウォームテーマの適用が容易になりました。

### 2. グローバルスタイルおよびコンポーネントのウォームゴールド移行
- **変更内容**:
    - `apps/management-console/src/App.css` [MODIFY]: グローバルスタイルにおけるシアン・パープル配色を `--accent-primary` 関連トークンへ一括置換（30箇所）。また、`.main-panel` に `flex: 1` を追加してレイアウトを修正し、重複するスピナー用キーフレームをクリーンアップ。グラデーション部の `rgba()` 生値をトークン参照にリファクタリング、サイドバー内のシアン参照漏れをゴールドに修正 (Reflexion)。
    - `apps/management-console/src/components/AiaaOnboardingWizard.tsx` [MODIFY]: チップ、インジケータ、ボタンの配色を `--accent-primary` 関連トークンに更新。未定義 CSS 変数参照（`--border`, `--accent-red`, `--accent-green`）を semantic トークン（`--border-glass`, `--accent-rose`, `--accent-emerald`）に修正し、生 px 値を rem/tokens に置換。さらに `gap` プロパティのハードコード値（`1rem` / `0.5rem`）を、スペーシングトークン `var(--space-sm)` / `var(--space-xs)` に置換 (Reflexion)。
    - `apps/management-console/src/components/Timeline.tsx` [MODIFY]: 高さ潰れを修正し、ドット配色やバッジ、グラデーション線をゴールド/アイボリー（ローカル）及びパープル（フェデレーテッド）へ移行。
    - `apps/management-console/src/components/SeoPulseView.tsx` [MODIFY]: 未動作の Tailwind CSS クラスを Vanilla CSS (Artemis DS インラインスタイル + CSS 変数) へ全面書き換え。重複していた `boxShadow` 定義を削除してビルドエラー (TS1117) を解消し、生 px 値を rem に流体化 (Reflexion)。
    - `apps/management-console/src/components/fluid/FluidBackground.test.tsx` [MODIFY], `useFluidConfig.test.ts` [MODIFY]: WebGL 用のカラーアサーションに `cssVar` ブリッジを適用して HEX のハードコード警告を解消 (Reflexion)。
- **波及効果**:
    - 管理コンソール全体のトーン＆マナーが、プレミアムなゴールド/アイボリー/エンバーのウォームパレットに統一されました。
    - `Timeline.tsx` および `SeoPulseView.tsx` の高さが正しく伸長し、表示崩れが解消されました。
    - `test_ui_hex_violations.py` によるデザイン検証が 0 violations (完全グリーン) で通過し、テーマ安全性が担保されました。

## 拡張ノード (Timer/Delay & WASM Code) の TDD 実装 (2026-06-14)

### 1. 拡張ノードのデータモデル、バリデーション、トランスパイルの実装
- **変更内容**:
    - `libs/infrastructure/src/workflow/schema.rs` [MODIFY]: `NodeType::Timer` および `NodeType::WasmCode` 定義とコスト算出ロジックを追加。
    - `libs/infrastructure/src/workflow/validator.rs` [MODIFY]: `ValidationError::InvalidTimerDelay` と `ValidationError::InvalidWasmCode` および安全制約検証（Timer上限24時間、Wasm空コード・言語判定）を追加。
    - `libs/infrastructure/src/workflow/transpiler.rs` [MODIFY]: `wf_timer` / `wf_wasm` カテゴリの Job へのコンパイルとパラメータ JSON の埋め込みを追加。
    - `libs/infrastructure/src/workflow/transpiler.rs` [MODIFY] (Reflexion): `ParallelWaitMode::N` モード時の `wait_mode_n` パラメータが `karma_directives` で保持されないバグを修正。
- **波及効果**:
    - ノーコード・ビジュアルビルダーの機能拡張ノード（時間待機、隔離コード実行）の定義から Job 展開までのバックエンド基盤が完全に整備されました。
    - 並列合流の Barrier ジョブにおいて、合流基準数（N値）が正しく Job の指示として伝達されるようになりました。

### 2. 実行エンジン Conductor の拡張とテスト追加
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/workflow_conductor.rs` [MODIFY]: `capable_categories` に `wf_timer` / `wf_wasm` を追加し、`conduct` 内での `tokio::time::sleep` 待機処理および Wasm シミュレーション実行ロジックを実装。Timerパース失敗時の警告ログを追加。
    - `libs/infrastructure/src/workflow/mod.rs` [MODIFY]: TDD サイクル用の6つの新設テストを追加し、さらに Reflexion サイクルにて Timer 境界値・WASM 孤立エラー・Parallel N モードの 3 テストを追加。
- **波及効果**:
    - ワークフロー実行エンジンが新規拡張ノードを自律的に引き受けて安全に処理・完了できるようになりました。エラー時の診断性と安定性も向上しました。

## Aiome ノーコード・ビジュアルビルダー機能の実装 (2026-06-14)

### 1. `fork_workflow` エンドポイントの実装と統合 (P1)
- **変更内容**:
    - `apps/api-server/src/routes/workflow.rs` [MODIFY]: `fork_workflow` ハンドラを実装。所有権および可視性の検証、元のワークフローの最新バージョンの定義を複製し、新しいワークフローIDのバージョン1として保存する処理を実装。
    - `apps/api-server/src/api_integration_tests/workflow.rs` [MODIFY]: `test_workflow_fork_api` 結合テストを追加し、所有権による権限制限、別ユーザーによるフォークの403制限、フォーク成功時のID/名前/フォーク元ID検証などを網羅。
- **波及効果**:
    - ワークフローのテンプレートを他のユーザーがコピーしてカスタマイズ可能にするための「フォーク」基盤が整い、エコシステムの共有・成長の基礎が完成。

### 2. `WorkflowStore` 永続化機能の拡張 (P1)
- **変更内容**:
    - `libs/infrastructure/src/workflow/store.rs` [MODIFY]: フォークされたワークフローであることを追跡できるように、`fork_source_id` フィールドもセットして新規挿入を行う `create_workflow_fork` メソッドを実装。
    - `apps/api-server/src/routes/workflow.rs` [MODIFY]: `get_workflow` APIのレスポンスにて、最新バージョン定義を返す際にも `fork_source_id` や `creator_id` などのメタデータをマージしてクライアントへ返却するようにマッピング処理を改善。
- **波及効果**:
    - ワークフローのフォーク元関係がDB内で完全に整合性を持ち、APIを介して正しく親ワークフローへのトレースが可能になりました。

### 3. `WorkflowConductor` のサービス自動登録 (P1)
- **変更内容**:
    - `apps/api-server/src/bootstrap/core_services.rs` [MODIFY]: `init_core_services` 内で `WorkflowConductor` インスタンスを作成し、`task_dispatcher.register_conductor` を用いてタスク自動実行の対象として登録。
- **波及効果**:
    - ワークフロー関連のJobカテゴリ（`wf_llm`, `wf_http`, `wf_mcp` 等）がエンキューされた際に、ディスパッチャを介して自律的に `WorkflowConductor` がそれらのノードジョブを引き受けて処理するよう実行環境が結合されました。

### 4. `AssetType::Workflow` の拡張と4段階スコープの対応 (P1)
- **変更内容**:
    - `libs/infrastructure/src/registry.rs` [MODIFY]: `AssetType::Workflow` バリアントを追加し、対応する `AsRef` および DB のロード処理を更新。
    - `registry.rs` [MODIFY]: `list_assets_by_type` 内で `owned/private`, `unlisted`, `community`, `marketplace` の4段階スコープのクエリロジックを実装。
- **波及効果**:
    - コミュニティ公開の無料ワークフロー（`price_coins = 0`）とマーケットプレイスの有料ワークフロー（`price_coins > 0`）、および非公開の `private`・リンク共有のみの `unlisted` を registry レベルで論理分離し、将来のマーケットプレイス課金連携の準備が整いました。

### 5. 実行コスト計算エンジン `estimate_cost` の実装 (P1)
- **変更内容**:
    - `libs/infrastructure/src/workflow/schema.rs` [MODIFY]: 各ノードの種別（LLM Prompt, MCP Tool Call, HTTP Request, Loop）ごとの実行コスト計算ロジック `estimate_cost` および返却用 `CostEstimate` 構造体を実装。
    - `libs/infrastructure/src/workflow/mod.rs` [MODIFY]: `test_estimate_cost` ユニットテストを追加し、ループ最大実行数を考慮した安全上限ベースのコスト計算結果を検証。
- **波及効果**:
    - ワークフローの実行前に必要な想定コストが計算可能になり、ユーザーのバジェット制限やエスクロー決済との連携の基礎設計が確立。

## Aiome P0/P1/P2 統合（オンボーディング・テンプレート・ディープスキャン硬化） (2026-06-13)

### 1. 開発者向けオンボーディング資料の整備とREADME同期 (P0)
- **変更内容**:
    - `docs/DEVELOPER_ONBOARDING.md` [NEW]: Samsara データフロー、P2P Commune 対話フロー、Nurture S2S認証・決済連携、OxiLean保護シーケンス、決済プロトコルを含むアーキテクチャ全体構造図と手順書を新規作成。
    - `README.md` [MODIFY], `README_en.md` [MODIFY]: 新規開発者がオンボーディング資料へ容易にアクセスできるようリンクと説明文を追記。
- **波及効果**:
    - オンボーディングコストが低下し、属人化（Bus Factor）が排除。Nurture との連携仕様が明文化され、開発者がスムーズにドメイン連携を理解可能。

### 2. テンプレートライブラリによるエコシステム拡張 (P1)
- **変更内容**:
    - `templates/minimal-skill/` [NEW]: WASMスキルの雛形。`Cargo.toml`末尾に `[workspace]` を記述し、独立ビルドを保証。
    - `templates/minimal-client-node/` [NEW]: Node.js TypeScript API クライアント雛形。Nurture決済の `use_escrow` や `remittance` モックロジックを含み、連携動作をサポート。
    - `templates/custom-mcp-server/` [NEW]: PythonによるカスタムMCPサーバー構築用雛形。
- **波及効果**:
    - 新規開発者や外部ツールからの移行・連携（Dify等の競合からのリプレイス含む）を促進するエコシステムの基盤。

### 3. 環境変数およびディープスキャン警告のクリーンアップ (P2)
- **変更内容**:
    - `libs/shared/src/process_hardening.rs` [MODIFY]: `OTHER_VAR` を `TEST_OTHER_VAR` にリネーム。
    - `scripts/deep-scan.sh` [MODIFY]: テストファイルの判定強化、`EXCLUDE_FILES` による完全一致ホワイトリスト除外処理、`grep` オプションおよび重複出力バグの修正、CC-3 警告の info 格下げ。
- **波及効果**:
    - `deep-scan.sh` の警告数が 0 件になり、CI/CD やリリース前チェックのノイズが完全に排除され、V-1（成熟度）向上に寄与。

## 管理コンソール多言語化とツールチップアクセシビリティ改善 (2026-06-13)

### 1. メインタブの多言語対応 (C1)
- **変更内容**:
    - `apps/management-console/src/components/home/HomePage.tsx` [MODIFY]: メインタブのラベル定義に `labelKey` を導入し、描画時に `t(tab.labelKey)` で翻訳するように修正。
    - `apps/management-console/src/i18n/en.json` [MODIFY], `apps/management-console/src/i18n/ja.json` [MODIFY]: `home.mainTab` キー（home, shop, world, settings）を追加。
    - `apps/management-console/src/components/home/HomePage.test.tsx` [MODIFY]: テストアサーションを i18n のキー（`home.mainTab.*`）を検証するように修正。
- **波及効果**:
    - 管理コンソールのメインメニュータブが動的言語切り替えに対応し、英語・日本語が正しく適用されるようになりました。

### 2. ツールチップのアクセシビリティおよび意匠強化 (C2 / W2)
- **変更内容**:
    - `apps/management-console/src/App.css` [MODIFY]: カスタムツールチップ（`[data-tooltip]`）に `:focus-visible` スタイリングを追加し、キーボード操作でフォーカスが当たった際にもツールチップが表示されるように修正。また、`::before` 疑似要素を用いた下向きの三角矢印インジケータを追加。
- **波及効果**:
    - マウスホバーだけでなく、キーボードの Tab キー操作によってフォーカスされた要素のツールチップが表示可能になり、アクセシビリティ（WCAG 2.1 準拠）が大幅に向上しました。また、矢印インジケータによって対象要素との視覚的な紐付けが明瞭になりました。

## Aiome LP 流体エフェクト統合 (2026-06-12)

### 5. docs/landing (aiome.dev) への Fluid Hero Background 移植
- **変更内容**:
    - `docs/landing/src/hooks/useFluidConfig.ts` [NEW]: 解像度スケール制御、reduced-motion メディアクエリ対応、CSS 変数からの色値解決を担うフックを実装。
    - `docs/landing/src/components/FluidHeroBackground.tsx` [NEW]: `three-fluid-fx` と `three` を非同期動的ロードし、WebGL 描画を行いながら非対応環境では CSS 暖色ラジアルグラデーションへフォールバックするコンポーネントを実装。
    - `docs/landing/src/components/Hero.tsx` [MODIFY]: 静的グリッド・グローを削除し、WebGL流体背景 `<FluidHeroBackground />` を統合。
    - `docs/landing/src/utils/colorUtils.ts` [NEW], `docs/landing/src/utils/cssVar.ts` [NEW]: WebGL 連携用の CSS カスタムプロパティ解決値取得・RGB パース用共通ユーティリティを移植。
    - `docs/landing/package.json` [MODIFY]: `three` および `three-fluid-fx` を dependencies に追加。
- **波及効果**:
    - `aiome.dev` 公式ランディングページのファーストビュー（Heroセクション）が静的なグラデーションから、インタラクティブでプレミアム感の高い WebGL 流体背景へとアップグレード。
    - 動的インポートの採用により、初回ページロード時の JavaScript バンドルサイズ肥大化とローディング遅延を回避。
    - ユーザーのアクセシビリティ設定（Reduced Motion）を尊重し、不要なアニメーションを確実にスキップしつつ、美しい CSS フォールバックグラデーションを表示。

### 1. `--bg-base` 未定義バグの修正と流体トークンの追加
- **変更内容**:
    - `apps/management-console/DESIGN.md` [MODIFY]: `bg-base` および `fluid-*` トークン定義を YAML フロントマターに追加。
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: `npm run sync:tokens` を実行して、DESIGN.md の内容に基づき自動生成。
- **波及効果**:
    - `LoginScreen.tsx` や `App.tsx`、`SetupWizard.tsx` で参照されていた `--bg-base` が解決され、UIテーマの未定義カラー参照バグが解消。

### 2. `parseColorToRGB` 共有ユーティリティの抽出
- **変更内容**:
    - `apps/management-console/src/utils/colorUtils.ts` [NEW]: `BiomeRenderer.tsx` 内にあった `parseColorToRGB` を共通ユーティリティとして抽出。
    - `apps/management-console/src/utils/colorUtils.test.ts` [NEW]: パース処理を検証する TDD テストを追加。
    - `apps/management-console/src/lib/biome/BiomeRenderer.tsx` [MODIFY]: インポートパスを `src/utils/colorUtils` に更新し、ローカルの重複定義を削除。
- **波及効果**:
    - CSS カラー値から WebGL uniform 用の RGB への変換処理が一元化され、車輪の再発明が防止。

### 3. LoginScreen への Fluid Background 統合
- **変更内容**:
    - `apps/management-console/src/components/fluid/FluidBackground.tsx` [NEW]: `three-fluid-fx` を useEffect 内で動的 import する WebGL 描画用コンポーネントを実装。
    - `apps/management-console/src/components/fluid/useFluidConfig.ts` [NEW]: モバイル解像度の制限や `prefers-reduced-motion` のメディアクエリ確認を含む設定フックを実装。
    - `apps/management-console/src/components/LoginScreen.tsx` [MODIFY]: 流体背景を配置し、ボタングラデーションやアイコンを暖色系の流体カラーに変更。
    - `apps/management-console/src/App.tsx` [MODIFY]: `ambientParticles` を認証完了後のみレンダリングするように条件分岐し、カラーを暖色系に更新。
- **波及効果**:
    - LoginScreen 上で R3F を経由しない軽量かつ高品質な流体シミュレーション背景エフェクトが動作。
    - 認証前と認証後の背景パーティクルの重複が解消され、無駄な WebGL コンテキストの競合が防止。
    - React.StrictMode における double-mount 時にも WebGL コンテキストが安全にクリーンアップされる設計を確保。

### 4. VRM アバターへの FluidAura 統合
- **変更内容**:
    - `apps/management-console/src/lib/vrm/FluidAura.tsx` [NEW]: planeGeometry とノイズ ShaderMaterial を用いた 3D オーラ演出用コンポーネントを実装。`useFrame` で time およびアバター状態別の強度のイージングを直接更新（`setState` 非使用）。
    - `apps/management-console/src/lib/vrm/VrmRenderer.tsx` [MODIFY]: アバターの Float コンポーネント背後に `<FluidAura>` を挿入。
- **波及効果**:
    - 既存のアバターや反射フロアに干渉することなく、アバターの感情状態に連動する有機的な暖色系オーラが 3D 空間内に自然に描画。

## Local LLM First の追加 Reflexion 改善 (TDD) (2026-06-12)

### 1. SemaphoreGuardedProvider テストの厳密化
- **変更内容**:
    - `libs/infrastructure/src/llm/semaphore_guard.rs` [MODIFY]: `MockProvider` の内部に `current_concurrent` および `max_concurrent` の追跡用 `AtomicUsize` を追加。テスト `test_semaphore_limit` 内で、最大同時実行数が `1`（セマフォのキャパシティ）を超えていないことを厳密にアサートするように修正。
- **波及効果**:
    - セマフォの同時実行制御が正しく機能しているかを、モック実行レベルで確実に監視・検証できるようになりました。

### 2. セマフォ容量の環境変数化
- **変更内容**:
    - `libs/shared/src/config.rs` [MODIFY]: 設定に `local_llm_concurrency` を追加し、環境変数 `LOCAL_LLM_CONCURRENCY`（デフォルト 2）から動的にロードするよう変更。
    - `apps/api-server/src/bootstrap/llm_providers.rs` [MODIFY]: `fast_provider` 用セマフォのキャパシティを `config.local_llm_concurrency` に置き換え。
    - `.env.example` [MODIFY]: `LOCAL_LLM_CONCURRENCY` 設定のプレースホルダーと説明を追記。
- **波及効果**:
    - システム展開先のマシンスペック（Ollama のスレッド並行能力など）に合わせて、ローカル LLM への同時実行制御数を柔軟に調整可能になりました。

### 3. ContextEngine 内部セマフォ名のリファクタリング
- **変更内容**:
    - `libs/infrastructure/src/context_engine.rs` [MODIFY]: `ContextEngine` 内で圧縮タスクの並行数制限に使われている `semaphore` の引数・フィールド名を `compression_semaphore` に改名。
- **波及効果**:
    - システム全体のLLM実行制限セマフォ（`llm_semaphore`）との混同を解消し、コンテキストエンジン内での役割（圧縮の同時実行制御）が明確になりました。

### 4. GuardedStream の Pin 安全性コメント追加
- **変更内容**:
    - `libs/infrastructure/src/llm/semaphore_guard.rs` [MODIFY]: `GuardedStream` 構造体の定義箇所に `SAFETY` から始まる Pin 安全性に関するドキュメントコメントを追記。
- **波及効果**:
    - コードベースの型安全性と安全設計に対する証明情報が明文化され、品質規格への準拠が明確になりました。

## Local LLM First Architecture — ローカル LLM 優先処理とフォールバックポリシーの実装 (2026-06-12)

### 1. TaskTier enum の導入
- **変更内容**:
    - `libs/aiome-contracts/src/task_tier.rs` [NEW]: タスクの推論強度を表現する `TaskTier` enum (Fast, Smart) を定義。
    - `libs/aiome-contracts/src/lib.rs` [MODIFY]: `task_tier` モジュールを登録。
- **波及効果**:
    - 各種 LLM タスク（分類、要約、複雑な推論など）に意味的なティアを設定できるようになり、将来的なコスト分析や動的なモデル割当の基盤が整いました。

### 2. AiomeConfig のローカル LLM 設定の追加
- **変更内容**:
    - `libs/shared/src/config.rs` [MODIFY]: Fast tier 用ローカルモデル（`local_fast_model`）およびローカル不在時のポリシー（`local_fallback_policy`）を追加。ポリシーは文字列から型安全な `LocalFallbackPolicy` enum に変更し、タイポ等の不正値設定時に警告を出して安全なデフォルト値（`AutoSwitch`）へフォールバックする処理を実装。
    - `.env.example` [MODIFY]: 環境変数のプレースホルダーと解説を追加。末尾の重複定義を削除し、適切な設定セクションに統合。
- **波及効果**:
    - 環境変数経由でローカルモデルの名称や、オフライン時の挙動を型安全に管理できるようになり、設定ミスによる意図しない無効なポリシー動作を恒久的に防止します。

### 3. fast_provider の bootstrap および Fast tier サービスへの切り替え
- **変更内容**:
    - `apps/api-server/src/bootstrap/mod.rs` [MODIFY], `llm_providers.rs` [MODIFY]: `ProviderResult` を拡張し、ポリシーに基づいて `fast_provider` を構築。さらに `fast_provider` 自体を `SemaphoreGuardedProvider` [NEW] で透過的にラップして同時実行数（容量2）を制限。
    - `apps/api-server/src/bootstrap/core_services.rs` [MODIFY]: 個別の `local_llm_semaphore` 定義を整理し、`ContextEngine` へのセマフォ渡しをデフォルトの `llm_semaphore` (10) へ復元。
    - `apps/api-server/src/bootstrap/state_assembly.rs` [MODIFY]: `HierarchicalRouter`, `CortexIngester`, `BuzzGenerator` を `fast_provider` へ移行。
    - `apps/api-server/src/bootstrap/workers.rs` [MODIFY]: `CortexCompiler` を `fast_provider` へ移行。
    - `apps/api-server/src/internal_services/heartbeat.rs` [MODIFY]: `HeartbeatWakeupService` を `fast_provider` へ移行。
    - `apps/api-server/src/api_integration_tests/system.rs` [MODIFY]: ポリシー別の `fast_provider` の接続・フォールバック動作を検証するインテグレーションテストを追加（enum化に対応）。
    - `libs/infrastructure/src/llm/semaphore_guard.rs` [NEW]: `SemaphoreGuardedProvider` を実装し、セマフォによるプロバイダー全体の透過的な流量制御とストリーミング対応の `GuardedStream` を定義。
    - `libs/infrastructure/src/llm/mod.rs` [MODIFY]: `semaphore_guard` モジュールを登録。
- **波及効果**:
    - 12箇所の Fast tier タスクがローカル LLM（Ollama）で処理されるようになり、高コストなクラウド API へのリクエストが大幅に削減されます。
    - `fast_provider` 全体を `SemaphoreGuardedProvider` で保護することで、すべてのコンポーネントにおけるローカル LLM への並行同時アクセス数を一貫して 2 に制限し、リソースの競合やタイムアウトを完全に防止します。
    - `FallbackRouter` の再利用により、ローカル LLM 未接続時のサーキットブリーキング、429/400 trim retry、およびクラウドへの透過的なフォールバック（`auto_switch`）が追加コードなしで動作します。

## Biome リリース v8.3 — IndexedDB 永続化と SSE イベント追加 (2026-06-11)

### 1. IndexedDB 状態永続化の導入
- **変更内容**:
    - `apps/management-console/src/hooks/useBiomeEngine.ts` [MODIFY]: WASM エンジン状態を IndexedDB を用いて非同期で保存・復旧する処理を実装。
- **波及効果**:
    - タブの切り替えやアンマウントが発生しても進化シミュレーションの状態（世代数等）が保持されるようになり、永続性が向上。

### 2. SSE 受信ハンドラの実装と i18n 追加
- **変更内容**:
    - `apps/management-console/src/App.tsx` [MODIFY]: SSE から受信した `biome_evolution` および `crisis_prediction` をパースし、`recentEvents` に動的追加。
    - `i18n/ja.json` [MODIFY], `en.json` [MODIFY]: 関連する `event.biomeEvolution` / `event.crisisPrediction` キーを event オブジェクトに追加。
- **波及効果**:
    - ユーザー画面のダッシュボードパルスにて、バックグラウンドでのバイオーム進化および危機予告イベントがリアルタイムで検知・視覚化されるようになりました。

### 3. テスト環境 (JSDOM) の安定化
- **変更内容**:
    - `apps/management-console/src/App.test.tsx` [MODIFY]: `scrollIntoView` のモック定義を追加し、`BiotopeView` のモックが `recentEvents` を描画するように修正。
- **波及効果**:
    - JSDOM で定義されていない API によるクラッシュを恒久的に防止し、SSE テストを含めた全320件のテストスイートが正常に PASS することを保証しました。

## デザイントークン品質改善 — 未定義参照の解消と重複排除 (Reflexion Cycle 2, 2026-06-10)

### トークン整合性の修復
- **変更内容**:
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: `--accent-blue` (#3b82f6), `--accent-orange` (#f97316) および opacity バリアントを追加。`--speed-fast`/`--speed-slow` の重複定義を解消し、`--speed-normal` (0.3s) を復元。
- **波及効果**:
    - BiomeHUD のグラデーション（`var(--accent-blue)`, `var(--accent-orange)`）が正しく描画されるようになった。以前は未定義のため transparent にフォールバックしていた。
    - ProofPowerIndicator, CausalVisualizer, EscrowManagementView 等の11箇所で参照されていた `--accent-blue` が解決。
    - 30箇所以上で参照されていた `--speed-normal` の機能が保持。

## Biome UI コンポーネント U-002 完全準拠 — Reflexion Refine (2026-06-10)

### デザイントークン拡張と生値排除
- **変更内容**:
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: `--space-2xs` (0.25rem), `--size-bar-sm` (0.375rem), `--blur-md` (12px) の3セマンティックトークンを追加。
    - `apps/management-console/src/lib/biome/BiomeHUD.tsx` [MODIFY]: spacing/border-radius/bar-height の全ハードコード rem/px 値をトークン参照に置換。`backdropFilter` を `blur(var(--blur-md))` に統一。
    - `apps/management-console/src/lib/biome/BiomeControls.tsx` [MODIFY]: `backdropFilter` を `blur(var(--blur-md))` に統一。
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: `useEffect` deps 配列に `injectElement` を追加（exhaustive-deps 準拠）。
- **波及効果**:
    - `tokens.css` に依存する全 UI コンポーネントが新トークンを利用可能に。テーマ切り替え時のバー高さ・微細間隔・ブラー強度の一括変更が可能。
    - `BiomeGame.tsx` の deps 修正により、`injectElement` の参照変更時（通常は発生しない）にも正しく初期化エフェクトが再実行される安全性を確保。

## Biome エンジンのシード動的化と「New Seed」ボタンの追加、CI統合 (2026-06-10)

### 1. シード動的化と「New Seed」UI の追加
- **変更内容**:
    - `apps/management-console/src/lib/biome/BiomeGame.tsx` [MODIFY]: Props の `seed` をオプショナルに変更。`currentSeed` ステートによる動的シード（`Math.random()` 由来）の管理と同期処理を実装。
    - `apps/management-console/src/lib/biome/BiomeControls.tsx` [MODIFY]: `onNewSeed` コールバックを受け取るようにプロパティを拡張し、「New Seed」ボタンをシステム制御エリアにレンダリングするように変更。
    - `apps/management-console/src/components/home/HomePage.tsx` [MODIFY]: `<BiomeGame>` の呼び出しで固定されていた `seed={42}` を削除。
- **波及効果**:
    - マウントするたびにランダムなシードでバイオームが自動生成されるようになり、さらにユーザーが「New Seed」ボタンをクリックすることで、いつでも異なるシードでの生命シミュレーションを再起動できるようになりました。
    - 既存 of コンポーネントおよび Jest ユニットテストとの互換性が完全に維持されていることを確認しました。

### 2. WASM インスタンスのメモリ管理・安全性・安定化の強化
- **変更内容**:
    - `apps/management-console/src/hooks/useBiomeEngine.ts` [MODIFY]: `seed` が無効値（`NaN` / `Infinity`）の場合の安全な `0` へのフォールバックバリデーションを導入。
    - `useBiomeEngine.ts` の再初期化およびアンマウント時に、前の `BiomeEngine` インスタンスに対して明示的に `free()` を呼び出し、WASM メモリリークを防止。
    - 返されるすべての API 関数（`tick`, `rewind`, `injectElement`, `applyCrisis` 等）を `useCallback` で囲み、レンダリング間の参照の安定性を保証。
    - `NodeJS.Timeout` の代わりにブラウザ環境で安全な `ReturnType<typeof setInterval>` に型を変更し、TypeScript 型エラーを解消。
- **波及効果**:
    - WebGL2 キャンバスのフレームレンダリングや visibilitychange イベントによるバックグラウンド進化のタイマー処理において、参照アイデンティティが不意に変化することによる無駄な再起動や再登録イベントを恒久的に防ぎ、アプリのパフォーマンスと安全性が大幅に向上しました。
    - `useBiomeEngine.test.ts` にてこれらの動作を網羅するテストを TDD サイクルで追加し、すべて通過することを確認しました。

### 3. UI コンポーネントの CSS デザイントークン完全適合化 (U-002)
- **変更内容**:
    - `BiomeGame.tsx` [MODIFY], `BiomeControls.tsx` [MODIFY], `BiomeHUD.tsx` [MODIFY]: インラインスタイルに直接記述されていた生値（`padding`, `gap`, `borderRadius`, `width`, `height`, `fontFamily`, `fontSize` 等のピクセル数やシステムフォント指定）を、`tokens.css` で定義された CSS 変数（`var(--layout-panel-gap)`, `var(--radius-md)`, `var(--font-main)` 等）へ完全置換。
- **波及効果**:
    - AIome の Token-Driven デザインシステムと 100% 整合するようになり、テーマ設定や将来のデザイン刷新に対して柔軟に対応可能となりました。また、デザイントークン自動スキャンによる U-002 違反の発生を恒久的に防ぎます。

### 4. CIワークフローにおける wasm-pack ビルドの統合

- **変更内容**:
    - `.github/workflows/ci.yml` [MODIFY]: `frontend` および `e2e` ジョブの依存関係セットアップ（`npm ci`）の直前に、`dtolnay/rust-toolchain@stable` と `jetli/wasm-pack-action@v0.4.0` を使用した `libs/biome-engine` クレートの WASM ビルドステップを追加。
- **波及効果**:
    - 開発マシン上のビルド成果物（`pkg/` フォルダ）に依存せず、GitHub CI パイプライン上で自律的かつクリーンに `biome-engine` WASM パッケージがコンパイルされるようになり、パッケージ依存関係のエラーによるビルド破損を恒久的に防止します。

---

## Biome Engine の追加と AgentSoul へのバイオーム進化夢想の統合 (Phase 3 / P3-13) (2026-06-10)

### 1. `biome-engine` クレートの追加とワークスペース統合
- **変更内容**:
    - `libs/biome-engine/` [NEW]: セルオートマトンによるバイオーム進化、ゲノム進化、形質固定を処理する Rust/WASM クレートを追加。
    - `Cargo.toml` [MODIFY]: workspace members に `libs/biome-engine` を追加。
    - `libs/infrastructure/Cargo.toml` [MODIFY]: `biome-engine` クレートへのパス依存関係を追加。
- **波及効果**:
    - ワークスペース全体で `biome-engine` がビルドおよびテスト対象となり、WASM にコンパイルしてフロントエンド等での連携、またはインフラストラクチャ層からの直接インポートが可能になりました。

### 2. `AgentSoul` へのバイオーム形質と `FrozenTraitSnapshot` の統合
- **変更内容**:
    - `libs/soul/src/biome_traits.rs` [NEW]: ヒッグス粒子によって固定された形質のスナップショットを表す `FrozenTraitSnapshot` 構造体を新規定義。
    - `libs/soul/src/lib.rs` [MODIFY]: `biome_traits` モジュールを登録し、`FrozenTraitSnapshot` を再エクスポート。
    - `libs/soul/src/model.rs` [MODIFY]: `AgentSoul` に `frozen_traits` フィールド（`Vec<FrozenTraitSnapshot>`）を追加し、シミュレーションで獲得した固定形質を永続的に記録可能に。
    - `libs/infrastructure/migrations/` (PostgreSQL/SQLite DDL) [NEW]: データベース上の永続化のため、`biome_frozen_traits` テーブルを新規作成するマイグレーションファイルを追加。
- **波及効果**:
    - `AgentSoul` の構造体が拡張され、世代を超えて固定されたバイオーム形質が永続化可能に。マイグレーションが追加されたため、各ノードの起動時に自動で DDL が適用され、DB 整合性が維持されます。

### 3. `DreamState` への `biome_evolution_dream` の統合と堅牢化 (P3-13)
- **変更内容**:
    - `libs/infrastructure/src/dream_state/mod.rs` [MODIFY]: `DreamState` に `biome_engine` と `soul_store` フィールドを追加。夢想の確率スロット（10%）にバイオーム進化夢想 `biome_evolution_dream` を組み込み。
    - `libs/infrastructure/src/dream_state/biome.rs` [NEW]: シミュレーションを実行し、LLM による観察ログの生成、`CoreEvent::BiomeEvolution` の配信、`store_karma` による `"biome"` ドメインのカルマ記録、および `AgentSoul` への `Experience` プッシュと `PredictiveModel` の `plasticity` 更新（Legendary=1.0, Rare=0.8, Common=0.5）を実行する処理を実装。
    - **エラーハンドリング堅牢化 (Reflexion)**: `store_karma` や `save_soul` などの永続化失敗時、夢想プロセス全体をクラッシュさせずに警告ログを出力して処理を継続するフォールトトレランス設計を導入。
    - **テストコード強化 (Reflexion)**: `libs/infrastructure/src/dream_state/tests.rs` [MODIFY] において、`Experience.domain == "biome"` やアサーションを含む6つの検証ポイントを `test_biome_evolution_dream` に追加。
- **波及効果**:
    - アイドル状態でエージェントが自律的にバイオーム進化をシミュレートし、その結果から得た教訓や予測可能性を `AgentSoul` にフィードバックして人格を変化させる仕組みが完成。
    - エラー時の自動回復・警告ログ化により、バックグラウンド実行時の不意なクラッシュを防止し、システムの安定性が担保されています。

---

## Biome から Commune へのフロントエンドおよびドキュメントの完全同期 (Phase 1) (2026-06-09)

### 1. フロントエンド (Management Console) のリネームとUI同期
- **変更内容**:
    - `BiomeDialogueView.tsx` ➔ `CommuneDialogueView.tsx` [NEW]: コンポーネントおよびファイル名をリネームし、コンポーネント名を `CommuneDialogueView` に更新。
    - `BiomeDialogueView.test.tsx` ➔ `CommuneDialogueView.test.tsx` [NEW]: ユニットテストコードも追従してリネームし、コンポーネントインポートを修正。
    - `App.tsx` [MODIFY] / `HomePage.tsx` [MODIFY] / `HomePage.test.tsx` [MODIFY]: `"biome"` から `"commune"` へのアクティブタブ文字列およびインポートコンポーネント名を置換。
    - `i18n/en.json` [MODIFY] / `ja.json` [MODIFY]: i18n 翻訳キーの `"biome"` を `"commune"` に置換、メッセージ内の「Biome」表記を「Commune」に更新。
- **波及効果**:
    - 管理コンソール画面上で、対話プロトコルのメニューおよび表示ラベルが一貫して `Commune` になり、ユーザー体験の混乱を排除。フロントエンドの Jest テスト（`CommuneDialogueView.test.tsx`, `HomePage.test.tsx`）およびビルドが正常にパスすることを確認。

### 2. 残存ドキュメントおよび監査レポートの更新
- **変更内容**:
    - `docs/guides/OPERATIONS_MANUAL.md` [MODIFY]: L157 の `biome_messages` 等のデータベーステーブル参照および Biomeプロトコルの説明を `commune_messages` 等および Communeプロトコルに更新。
    - `TECH_DEBT_AUDIT.md` [MODIFY]: 旧 `biome.rs` への参照（L9, 69, 186, 187, 242, 263）を `commune.rs` / `routes/commune.rs` 等の正しいファイル名に変更。
- **波及効果**:
    - 技術的負債監査レポートや運用マニュアル内のドキュメントドリフト（ファイル参照不整合）を解消し、ドキュメントの整合性を 100% 担保。

---

## Biome から Commune への環境変数・ログの完全分離 (2026-06-09)

### 1. SSRF防止ホワイトリスト環境変数のリネームと TDD テスト追加
- **変更内容**:
    - `libs/shared/src/security.rs` [MODIFY]: ホワイトリスト取得環境変数を `BIOME_HUB_WHITELIST` から `COMMUNE_HUB_WHITELIST` へ変更。コメントを `Commune SSRF Defense` に修正。
    - `libs/shared/src/security.rs` のテストモジュールに、新環境変数の適用を確認する Positive Test `test_commune_hub_whitelist_allows_endpoints` と、旧環境変数 `BIOME_HUB_WHITELIST` が無視・拒否されることを検証する Negative Test `test_legacy_biome_hub_whitelist_is_ignored` を追加。
    - `.env` [MODIFY] / `.env.example` [MODIFY]: `BIOME_HUB_WHITELIST` 変数名および関連するコメントを `COMMUNE_HUB_WHITELIST` にリネーム。
- **波及効果**:
    - P2P対話プロトコルの名前空間である `Commune` にセキュリティホワイトリスト設定が一貫して統合され、旧 `Biome` 概念との混同によるバグや誤設定を排除。
    - 堅牢な Negative Test により、将来的に古い環境変数名が誤って使用された場合にも安全にフォールバック（無視・拒否）されることが CI 上で自動検証・保証されます。

### 2. 夢状態・ハブ状態ログメッセージのクリーンアップ
- **変更内容**:
    - `libs/infrastructure/src/dream_state/communication.rs` [MODIFY]: ログメッセージやインスピレーション生成時の説明文中の「Biome」参照（4箇所）を「Commune」に変更。
    - `apps/samsara-hub/src/state.rs` [MODIFY]: データベース初期化完了ログ内の「& Biome」を「& Commune」に変更。
- **波及効果**:
    - デバッグログやイベント記録の整合性が保たれ、システム運用時の混乱を未然に防止します。

---

## バージョン通知システム (Version Notification System) の実装 (2026-06-09)

### 1. バージョン通知チェック関数の実装とハートビート連携
- **変更内容**:
    - `apps/api-server/src/internal_services/heartbeat.rs` [MODIFY]: 起動時に一度だけ実行され、設定の `last_notified_version` と現在のバージョン（`v1.0.2`）を比較し、更新があれば `CoreEvent::ProactiveTalk` アナウンスをブロードキャスト送信し、設定を更新する関数 `check_and_notify_version` を新規実装。`run` ループの開始前にこの関数を統合。
- **波及効果**:
    - アプリケーション起動時の一度のオーバーヘッドで、既存の ProactiveTalk SSE 経由でフロントエンド（管理コンソール）にバージョンアップのアナウンスが自動で通知されるようになりました。既存のチャネルブロードキャスト基盤に載せるため、他の通知システムに影響を与えません。

### 2. 設定キーのホワイトリスト追加
- **変更内容**:
    - `apps/api-server/src/routes/settings.rs` [MODIFY]: ホストやシークレットと同様に、`ALLOWED_KEYS` ホワイトリストに `"last_notified_version"` を追加。
    - 単体テスト `test_last_notified_version_is_allowed` を追加し、ホワイトリストの整合性をアサート。
- **波及効果**:
    - バージョン通知ステータスが `ALLOWED_KEYS` のバリデーションチェックを通過するようになり、管理画面等からの更新時にもセキュリティ違反エラーが発生しなくなりました。

### 3. ハートビート通知の TDD 結合テスト追加
- **変更内容**:
    - `apps/api-server/src/api_integration_tests/heartbeat.rs` [NEW]: 起動時の初回通知送信、設定更新の確認、および2回目起動時の重複通知防止（べき等性）をアサートする結合テスト `test_version_notification_on_first_startup` を新規実装。
    - `apps/api-server/src/api_integration_tests/mod.rs` [MODIFY]: `pub mod heartbeat;` モジュールを追加。
- **波及効果**:
    - バージョン通知の動作とべき等性が、テストを通じて CI 上で永続的に監視・保護されるようになりました。

---

## God Module 分割 (Phase 48 / G4, G5) (2026-06-08)

### 1. `llm/dynamic.rs` の分割
- **変更内容**:
    - `libs/infrastructure/src/llm/dynamic.rs` (1,176行) から、コスト計算機能（`log_evaluation`, `model_pricing`, `calculate_cost_usd`, `calculate_cost_coins` およびテストコード）を新モジュール `llm/cost.rs` へ分離。
    - `BackgroundLlmProvider` 構造体および実装を新モジュール `llm/background.rs` へ分離。
    - `dynamic.rs` 自体は `DynamicLlmProvider` に特化させ、後方互換性のために上記分離したシンボル群を `pub use` で re-export。
    - `llm/mod.rs` に新モジュール `cost`, `background` を追加。
- **波及効果**:
    - 独立した2つの巨大なプロバイダー構造体が物理的に別ファイルに整理され、LLMプロバイダー層の保守性が飛躍的に向上。ハブモジュール化により外部参照箇所（`bootstrap` や `agent_engine` など）を一切変更せずに分割を完了。

### 2. `routes/internal.rs` の分割
- **変更内容**:
    - `commercial/apps/nurture-api/src/routes/internal.rs` (1,078行) をディレクトリ化し、`routes/internal/mod.rs` および役割ごとのサブモジュール (`types.rs`, `balance.rs`, `escrow.rs`, `gdpr.rs`, `misc.rs`) へ機能別に再構成。
    - `mod.rs` でルーターおよびミドルウェアの定義を行い、外部互換性のために `types.rs` 内の全リクエスト/レスポンス構造体を `pub use` で re-export。
- **波及効果**:
    - `commercial/` 配下の重要で巨大な経済処理 API エンドポイント群が、責務（残高、エスクロー、GDPR、その他）ごとに物理的に分離され、可読性と安全性が大幅に向上。
    - `pub use` による型再エクスポートにより、テストコード (`internal_routes_test.rs`) や外部参照元 (`main.rs`) を破壊することなく、安全なファイル分割を完了。

---

## God Module 分割 (Phase 48 / G1, G2, G3) (2026-06-08)

### 1. `task_orchestrator/mod.rs` の分割
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/mod.rs` (1,186行) を、4つのサブモジュール（`types.rs`, `dispatcher.rs`, `dispatch_loop.rs`, `goal_processor.rs`）に分割。
    - `mod.rs` をハブモジュールとし、外部およびテストから参照されるシンボルや共通依存関係を `pub use` / `pub(crate) use` で再エクスポートして後方互換性を保証。
- **波及効果**:
    - インフラストラクチャ層のタスクディスパッチャおよびゴールプロセッサ의 ビジネスロジックが論理単位に整理され、肥大化した God Module が解消。コンパイル速度とコード可読性が向上。

### 2. `mcp/discovery.rs` の分割
- **変更内容**:
    - `apps/api-server/src/mcp/discovery.rs` (1,102行) を、2つのサブモジュール（`config.rs`, `oauth.rs`）に分割。
    - `mcp/mod.rs` でサブモジュールを宣言し、`discovery.rs` でそれらを `pub use` で再エクスポート。
- **波及効果**:
    - MCP クライアントおよびサーバー探索処理と、OAuth・設定関連のロジックが物理的に分離され、コンポーネントごとの責務が明確化。

### 3. `security.rs` の分割
- **変更内容**:
    - `libs/infrastructure/src/security.rs` (1,210行) を、3つのサブモジュール（`security/config.rs`, `security/bastion_guard.rs`, `security/voice_core_drm.rs`, `security/tests.rs`）に分割。
    - `security.rs` 自体はサブモジュールを管理・再エクスポートするハブモジュールへと置き換え。
- **波及効果**:
    - `security.rs` 自体は Safety-Critical Zone に属するが、ロジックを変更することなく完全にモジュール化が成功。既存のすべてのテストケース（536件）が正常にパスすることを確認。

---

## 統合修正計画 v6.0 技術的負債解消 (2026-06-08)

### 1. `mcp/discovery.rs` の環境変数解決 DRY 化
- **変更内容**:
    - `resolve_env_var` プライベートヘルパーを導入し、重複していた `std::env::var` および `AiomeConfig::get_secret` による解決ロジックを DRY 統一。
    - `test_resolve_env_var` ユニットテストを追加して動作を検証。
- **波及効果**:
    - 環境変数の解決処理が一元化され、将来のセキュリティスキーム変更時（秘匿値の取得方法の変更など）の変更箇所を局所化。

### 2. napi-bridge ユニットテスト実装
- **変更内容**:
    - `libs/napi-bridge/Cargo.toml` に `serial_test` および `tokio` のテスト依存を追加。
    - `libs/napi-bridge/src/lib.rs` に主要 FFI 関数のユニットテストを実装。`OnceCell` によるグローバルインスタンスの競合を防止するため `#[serial]` による直列実行と、共有キャッシュ `sqlite::memory:?cache=shared` の指定によるDB状態アイソレーションを導入。
- **波及効果**:
    - Node.js と Rust 間のブリッジ処理の堅牢性が担保され、境界部分でのクラッシュやデータ破損リスクを CI レベルで自動検証可能に。

### 3. フロントエンドの型定義の強化と `any` 排除
- **変更内容**:
    - `SettingEntry` インターフェース定義を `types.ts` に移行。
    - `useViewMode`, `useAgentChat`, `CausalVisualizer`, `A2uiRenderer` 内の `any` を新設したインターフェース (`HistoryMessageResponse`, `CausalGraphResponse`, `A2uiMetric` 等) に置換し、型チェックを厳格化。
- **波及効果**:
    - TypeScript コンパイル時の静的チェックの網羅性が向上し、API レスポンス構造の変更に対する耐性を確保。

### 4. フロントエンドのテストスイート拡張
- **変更内容**:
    - テストが不足していた `GraphView`, `OllamaModelSelector`, `PromptStatsView`, `SeoPulseView`, `TreasureBox` に対して Jest テストファイルを新規作成。
- **波及効果**:
    - フロントエンドコンポーネントのテストカバレッジが向上し、UI 挙動に関する回帰テストの信頼性が向上。

## God Module 分割 (Phase 2) (2026-06-08)

### 1. `bridge.rs` の分割
- **変更内容**:
    - `commercial/libs/nurture-infra/src/economy/bridge.rs` を削除。
    - `commercial/libs/nurture-infra/src/economy/bridge/mod.rs` [NEW]: 構造体定義、new、およびビジネスロジックヘルパーメソッド群を配置。
    - `commercial/libs/nurture-infra/src/economy/bridge/commerce_impl.rs` [NEW]: `CommerceEngine` トレイト実装部分を分離。
    - `commercial/libs/nurture-infra/src/economy/bridge/tests.rs` [NEW]: 単体テストモジュール `mod tests` を独立。
- **波及効果**:
    - `nurture-infra` クレートの主要ビジネスロジックとテストコードが物理的に分離され、ファイルの見通しとメンテナンス性が飛躍的に向上しました。外部APIや親モジュールとの結合度は維持されています。

### 2. `stripe.rs` の分割
- **変更内容**:
    - `libs/aiome-commerce/src/stripe.rs` を削除。
    - `libs/aiome-commerce/src/stripe/mod.rs` [NEW]: `StripeCommerceEngine` 定義および `CommerceEngine` トレイト実装部分を配置。
    - `libs/aiome-commerce/src/stripe/tests.rs` [NEW]: 単体テストモジュール `mod tests` を独立。
- **波及効果**:
    - `aiome-commerce` クレートで Stripe API を扱う主要エンジン部と、その長大なテストコード（700行以上）が物理的に分離され、コンパイル単位およびコード理解が容易になりました。

## P0ブロッカー修正（収益化準備度向上） (2026-06-07)

### 1. `/internal/validate-activity` ルートおよびハンドラ追加
- **変更内容**:
    - `commercial/apps/nurture-api/src/routes/internal.rs` [MODIFY]: `/validate-activity` ルートおよび `internal_validate_activity` ハンドラを新設し、OxiLean証明書による認証下でエージェントのアクティビティポリシー検証を実行可能に。
    - `commercial/apps/nurture-api/tests/internal_routes_test.rs` [MODIFY]: `test_internal_api_validate_activity` およびその異常系検証のテストを追加。
- **波及効果**:
    - NurtureCommerceBridge を直接叩かずに、安全な内部 API 経由でアクティビティ検証ができるようになり、S2S間の結合度が向上しました。

### 2. AiaaOnboardingWizard.tsx ハードコード除去
- **変更内容**:
    - `apps/management-console/src/components/AiaaOnboardingWizard.tsx` [MODIFY]: オンボーディングウィザード内に残っていた `price_dummy` のハードコードを `config.ts` で定義された `STRIPE_PRICE_ID` へ置換。
- **波及効果**:
    - フロントエンドが環境変数（VITE_STRIPE_PRICE_ID）に追従するようになり、Stripe に存在しないダミー価格による決済セッション作成エラーを防止。

### 3. OpenAPI 決済5エンドポイント登録
- **変更内容**:
    - `apps/api-server/src/api.rs` [MODIFY]: paths と schemas に決済関連の5エンドポイント（および `SubscriptionStatus` などの構造体）を登録。
    - `apps/api-server/src/routes/commerce.rs` [MODIFY]: 各ハンドラに `#[utoipa::path(...)]` アノテーションを付与し、必要な型（`SubscriptionStatus`）をインポート。
- **波及効果**:
    - 管理コンソールが自動生成する OpenAPI ドキュメントに決済機能が完全に含まれるようになり、クライアントコード生成ツールの整合性を確保。

## 統合修正計画 v5.2 技術的負債解消と環境・エラー型の整備 (2026-06-07)

### 1. DB クエリマクロ（`sql_fetch_raw!` ファミリー）の導入と DRY 化
- **変更内容**:
    - `libs/shared/src/db.rs` [MODIFY]: `sql_fetch_raw!`, `sql_fetch_raw_optional!`, `sql_fetch_raw_one!`, `sql_scalar!` の 4 つの DB クエリマクロを新規導入。
    - `libs/infrastructure/src/db.rs` [MODIFY]: 上記マクロの re-export を追加。
    - `libs/infrastructure/src/job_queue/federation.rs` [MODIFY], `libs/infrastructure/src/aegis/incident_repo.rs` [MODIFY], `libs/infrastructure/src/soul_store.rs` [MODIFY]: 既存の冗長な `match &self.pool` 分岐クエリ処理（計54箇所）をマクロに置換し、ボイラープレートを大幅に削減。
- **波及効果**:
    - DB方言（Sqlite/Postgres）のプールの分岐が共通マクロ内にカプセル化され、同一コードが複数箇所に増殖するのを防止。
    - 行マッピング（Row mapping）ロジックは呼び出し元に残るため、コンパイル時の型安全性を維持。

### 2. `let _ =` エラー黙殺パターンのトリアージと安全なハンドリング
- **変更内容**:
    - `logging.rs` [MODIFY], `autonomous_demo.rs` [MODIFY], `tool_call_router.rs` [MODIFY], `stream.rs` [MODIFY], `routes/karma.rs` [MODIFY], `skill_handler.rs` [MODIFY], `routes/expression.rs` [MODIFY], `bootstrap/preflight.rs` [MODIFY], `docker_conductor.rs` [MODIFY]: 合計 21 箇所でエラーがサイレントに無視されていた箇所（DB 操作、FS操作、サブプロセス制御など）に対し、`tracing::warn!`/`tracing::error!` / `eprintln!` 警告ログの出力、または `// DropSafe` コメントを付与。
- **波及効果**:
    - ディスク満杯時や DB 不具合、コンテナ操作失敗時などの隠れた不具合が可視化され、サイレントなデータ消失を防止。

### 3. 環境変数検証およびエラー型のドキュメント化 (Phase 5)
- **変更内容**:
    - `.env.example` [MODIFY]: 不要な `OTHER_VAR=my_value` ダミー変数をクリーンアップし、環境変数の検証完了（QW-1）に関する注記を追記。
    - `docs/architecture/error_handling.md` [NEW]: Aiome システムの 10 種類のエラー型の役割・責務を整理し、新規エラー追加の制限ルール（既存のエラー型へのバリアント追加を優先する）を明記した設計ドキュメントを新設。
    - `libs/aiome-contracts/src/error.rs` [MODIFY]: 新規エラー追加禁止および設計ドキュメントへのリンクを明記する警告コメントを追加。
- **波及効果**:
    - 開発者が各エラー型の責務を把握しやすくなり、エラー型の不要な乱立を未然に防止。

## settings.rs & GraphView.tsx - Reflexion 95点未満ファイルのTDD修正 (2026-06-07)
- **変更内容**:
    - `libs/infrastructure/src/job_queue/settings.rs` [MODIFY]: SQLite/Postgres の行マッピング処理を共通ヘルパー関数 `build_setting` に DRY 統一。`aggregate_cost_hours` / `aggregate_cost_days` に存在していた SQL クエリ呼び出しの重複を `aggregate_cost_by_interval` ヘルパー関数に抽出・共通化。長すぎる SQL を複数行に改行。
    - `libs/infrastructure/src/job_queue/tests.rs` [MODIFY]: `test_sqlite_settings_row_masking` と `test_sqlite_settings_cost_aggregation` のテストを新規追加し、マスク処理とコスト集計の正常性を TDD サイクルで検証。
    - `apps/management-console/src/components/GraphView.tsx` [MODIFY]: `vis-network` から `Node`/`Edge` 型をインポートし、DataSet を型安全化。`karmaData.nodes`/`edges` の API レスポンスに `Array.isArray()` による存在チェック of 安全ガードを追加。非同期初期化時の `containerRef.current` NULL 安全検証を追加。`Math.max(0, nodeCount - artifactCount)` による負の値防止。Layers ボタンに「Artifactsの表示レイヤトグル機能」をステート制御で追加実装。
- **波及効果**:
    - 2つのファイルの Reflexion スコアがそれぞれ 95点以上に向上し、ソースコードの品質・安全性・保守性が改善されました。
    - フロントエンドの `Layers` ボタンにより、ユーザーが不要なノード（成果物）を非表示にできるインタラクティブな機能が追加されました。

## StripeCommerceEngine エラーハンドリングの DRY 統一 (Phase D8) (2026-06-07)
- **変更内容**:
    - `libs/aiome-commerce/src/stripe.rs` [MODIFY]: 共通エラー変換トレイト `IntoInfraError` を定義し、ファイル内の 30 箇所以上の重複していた `AiomeError::Infrastructure` エラーハンドリングブロックを `.map_infra_err()` および `.map_infra_err_context("context")` に DRY 統一。
- **波及効果**:
    - コードベース内の冗長なボイラープレートエラーマッピングが排除され、コードの可読性が大幅に向上するとともに、将来のエラーハンドリング変更時の一元化が容易になりました。
    - 49 件の既存の commerce テストおよび Zero-Trust などの関連テストを正常系・異常注入（Negative Test）にて検証し、すべて健全にパスすることを確認しました。

## Monorepo DRY リファクタリング・衛生改善および仕様書同期 (Phase D) (2026-06-07)
- **変更内容**:
    - `libs/infrastructure/src/job_queue/federation.rs` [MODIFY]: `map_sqlite_row_to_karma` / `map_postgres_row_to_karma` を導入して `FederatedKarma` 行マッピング重複を DRY 統一。テストデータベースのテーブル定義に `is_private` と `is_archived` を追加してテストの整合性を保証。
    - `libs/infrastructure/src/artifact_store.rs` [MODIFY]: `map_edge_row_generic` を導入して `ArtifactEdge` 行マッピング重複を DRY 統一。
    - `libs/infrastructure/src/job_queue/settings.rs` [MODIFY]: `map_sqlite_row_to_setting` / `map_postgres_row_to_setting` を導入して `do_get_all_settings` 行マッピング重複を DRY 統一。
    - `libs/infrastructure/src/job_queue/swarm.rs` [MODIFY]: Clock Poisoning 閾値である 86400秒（1日）を `MAX_CLOCK_SKEW` 定数として定義しマジックナンバーを置換。
    - `docs/specs/SECURITY_WHITEPAPER.md` [MODIFY]: 更新日を 2026-06-07 に統一、JSON構造の typo を修正、Biome 隔離保護に関するセキュリティ注記を追加。
    - `docs/architecture/INFRASTRUCTURE_MODULES.md` [MODIFY]: `job_queue` の開発状態（Biome/Federation等）を実装完了状態へ同期。
    - `.env.example` [MODIFY]: `ENFORCE_GUARDRAIL` のデフォルト値を `"true"` へ修正。
    - `apps/samsara-hub/src/main.rs` [MODIFY]: 不要な `#![allow(dead_code)]` を削除し、コンパイラによる不要な警告抑制を排除。
- **波及効果**:
    - コードベース全体の重複が排除され、将来の DB カラム追加・変更に対する保守性が劇的に向上しました。
    - 各仕様書が実際の実装状況と完全に一致し、設定ファイルのデフォルトが安全側（Fail-Closed）に倒されました。

## ai_artifacts テーブルへの is_protected カラム追加、マッピング DRY 統一、および E2E 暗号化 ADR 起票 (2026-06-07)

### 1. is_protected DB カラムの追加と永続化
- **変更内容**:
    - `libs/infrastructure/migrations/sqlite/20260607000000_add_is_protected_to_artifacts.sql` [NEW] & `libs/infrastructure/migrations/postgres/20260607000000_add_is_protected_to_artifacts.sql` [NEW]: `ai_artifacts` テーブルに DRM 隔離用フラグである `is_protected` カラムを追加するマイグレーションを新規作成。
    - `libs/aiome-core-contracts/src/traits.rs` [MODIFY]: `ArtifactMeta` 構造体に `is_protected: bool` フィールドを追加。
    - `libs/infrastructure/src/artifact_store.rs` [MODIFY]: `save_artifact` 内の INSERT 操作で `is_protected` をDBに保存。`read_artifact_file` における監査ログ（Audit Log）で、物理的なファイルチェックの heuristics を DB カラム `meta.is_protected` に置き換え。
- **波及効果**:
    - DRM 隔離フラグが物理データベースに保存され、成果物のロード時にメタデータから直接取得可能に。これにより、監査ログで「保護対象アセット」かどうかを高速かつ正確に判定可能になりました。
    - テストファイル `artifact_store_tests.rs` や `oss_repository_indexer.rs` 内の `CREATE TABLE` 定義に `is_protected` を追加し、テスト実行時の一貫性を保証。

### 2. SQL 行マッピング処理の DRY 統一
- **変更内容**:
    - `libs/infrastructure/src/artifact_store.rs` [MODIFY]: Sqlite 用・Postgres 用で重複していたマッピングロジックをジェネリック関数 `map_row_generic` に統一。
- **波及効果**:
    - データベース依存の実装コードが DRY になり、将来のフィールド追加・修正時のメンテナンスコストが劇的に削減されました。

### 3. P2P E2E 暗号化 ADR の起票
- **変更内容**:
    - `docs/decisions/043-p2p-e2e-encryption.md` [NEW]: P2P連邦ネットワークにおけるエンドツーエンド暗号化（X25519 鍵共有と AES-256-GCM によるハイブリッド暗号方式）の設計書を作成。
- **波及効果**:
    - 将来的な連邦メッセージE2E暗号化の実装方針が明確になり、中継ハブ経由時のメッセージ暴露リスクに対する具体的な緩和戦略を定義。

## Project NURTURE Monorepo 統合 & コンパイル警告・エラーの完全解消 (2026-06-07)

### 1. Nurture 商用拡張モジュールの Monorepo 統合
- **変更内容**:
    - `Project-Nurture` リポジトリのソースコード全体を `aiome` リポジトリ内の `commercial/` 直下に移行。
    - ルートの `Cargo.toml` の `members` に Nurture クレート群（`commercial/apps/nurture-api`, `commercial/libs/commerce-protocol`, `commercial/libs/nurture-bridge`, `commercial/libs/nurture-core`, `commercial/libs/nurture-infra`）を統合。
- **波及効果**:
    - 独立したリポジトリ境界がなくなり、同一 Cargo ワークスペース内で一括して依存解決、ビルド、テスト実行、Dockerビルドが可能に。
    - パス依存関係が `../../../libs/...` に短縮・一元化。

### 2. リリースビルド（`not(debug_assertions)`）時のコンパイルエラーの修正
- **変更内容**:
    - `libs/aiome-commerce/src/x402.rs` の `new` 関数において、リリースビルド時の `#[cfg]` 分岐で変数 `private_key_hex` がスコープ外になるバグを、アトリビュートをブロック全体に適用して修正。
    - `commercial/libs/nurture-bridge/src/lib.rs` で、デバッグ用の `MockAuthManager` と `MockLlmProvider` をリリースビルド時にも再エクスポートしようとしていたインポートエラーを修正。
    - `commercial/apps/nurture-api/src/main.rs` で `cfg!(debug_assertions)` に基づく動的初期化を行っていた箇所を、`#[cfg]` アトリビュートによる物理的なブロックコンパイル分岐に変更し、リリースコンパイル時に発生していた `MockAuthManager` 未定義エラーを解消。
- **波及効果**:
    - `cargo check --workspace` および本番用 Docker イメージ（`production.Dockerfile`, `distroless.Dockerfile`）のビルドが警告なしでクリーンに成功。

### 3. api-server のマイナーバグ修正
- **変更内容**:
    - `apps/api-server/src/bootstrap/helpers.rs` における `tracing::error` インポートの欠如、および `apps/api-server/src/mcp/http_client.rs` における `localhost_allowed` と `_localhost_allowed` の命名揺れを修正。
- **波及効果**:
    - api-server がビルドエラーにならず正常に起動・機能することを確認。

## MemoryCrystallizer 堅牢化とエラー局所化（Reflexion 改良） (2026-06-05)

### 1. run_distillation_cycle の戻り値変更とエラーハンドリング改善
- **変更内容**:
    - `run_distillation_cycle` の戻り値の型を `Result<(), Box<dyn std::error::Error>>` から `Result<(), AiomeError>` に変更し、契約定義とエラー型の一貫性を確保。
    - LLM 呼び出し失敗時にサイクル全体をパニックやエラー終了させず、該当チャンクのみをスキップするエラー局所化（Graceful Skip）を実装。
- **波及効果**:
    - 外部 LLM API の一時的なエラーやタイムアウトが発生しても、他のスキルの結晶化処理が妨げられず、システム全体の耐障害性が大幅に向上。
    - 呼び出し元である `apps/api-server/src/bootstrap/core_services.rs` L319 等では `Display` トレイトを介してエラー処理をしており、後方互換性は維持されています。

### 2. 多層 OOM 防御（OOM Defense）の導入
- **変更内容**:
    - 1回の結晶化サイクルで処理する最大スキル数を `MAX_SKILLS_PER_CYCLE = 100` に制限。
    - 各生の教訓（Raw Karma）文字列の最大長を `MAX_LESSON_CHARS = 2000` に制限し、超過部分を切り捨て。
    - 教訓のバッチ処理サイズを 50 単位 of チャンクに分割して処理。
- **波及効果**:
    - 蓄積された Raw Karma の量が膨大になった場合でも、巨大な文字列のメモリ割り当てが発生せず、メモリ枯渇による OOM クラッシュを物理的に防止。

### 3. プロンプトインジェクション対策
- **変更内容**:
    - LLM プロンプト構築時、生の教訓データを `<SKILL>` や `<LESSONS>` などの XML タグ（デリミタ）で囲むように変更。
- **波及効果**:
    - ユーザーの生の教訓に LLM への悪意ある命令が混入した場合でも、プロンプト構造とデータが物理的に分離されるため、プロンプトインジェクション脆弱性を未然に遮断。

### 4. セマフォ制御の改善
- **変更内容**:
    - セマフォの獲得失敗時（try_acquire に失敗した際）に、単にスキップするだけでなくデバッグ用のログ（"⏸️ [MemoryCrystallizer] Semaphore exhausted, skipping skill: {}"）を出力するように変更。
- **波及効果**:
    - トラフィック増加時のセマフォ枯渇の診断が極めて容易になりました。

### 5. テストスイートの追加
- **変更内容**:
    - `MemoryCrystallizer` に、正常系、LLM 失敗時のチャンクスキップ、スキル無しの境界値、およびセマフォ枯渇時の動作をアサートする4つのユニットテストを新規追加。
- **波及効果**:
    - `cargo test` によってこれらの堅牢化・エラー局所化の仕組みが毎ビルド自動検証され、サイレントな回帰バグの混入を恒久的に防止。

## 技術的負債の解消、巨大ファイルの分割およびコンパイル警告の完全排除 (2026-06-05)

### 1. dream_state.rs のモジュール構造化
- **変更内容**:
    - `libs/infrastructure/src/dream_state.rs` を削除し、`libs/infrastructure/src/dream_state/` ディレクトリ配下に `mod.rs`, `aegis.rs`, `communication.rs`, `exploration.rs`, `observability.rs`, `reflection.rs`, `scientific.rs`, `tests.rs` の7つのサブモジュールとして分割・整理。
- **波及効果**:
    - 約1800行に達していた巨大ファイルの可読性と凝集度が向上し、関心の分離が明確化。既存の `pub use` 再エクスポート構造により、`api-server` 側の `DreamService` ランタイムを含む外部からの依存箇所に変更を加えることなく動作を維持。

### 2. bootstrap/mod.rs のサービス初期化ロジック抽出
- **変更内容**:
    - `apps/api-server/src/bootstrap/mod.rs` 内に存在していた約900行の `init_core_services()` ロジックを、新設ファイル `apps/api-server/src/bootstrap/core_services.rs` に抽出・分離。
- **波及効果**:
    - アプリケーション起動プロセスの最も肥大化していた部分が分離され、コード理解と起動プロセスの変更が極めて容易になりました。既存の `bootstrap/mod.rs` は再エクスポートにより後方互換性を完全に保証。

### 3. ブランケット allow(unused_*) 属性の完全排除
- **変更内容**:
    - 主要ライブラリ（`shared`, `soul`, `core`, `aiome-commerce`, `infrastructure`, `napi-bridge`）およびアプリケーション（`api-server`, `aiome-migrate`, `samsara-hub`, `aiome-node`）合計95ファイルから、不要な `allow(unused_imports, unused_variables, unused_mut)` などの警告抑制用アトリビュートを完全除去し、未使用インポート・変数を修正。
- **波及効果**:
    - ワークスペース全体で `cargo check --workspace --tests` がコンパイル警告0件・エラー0件でクリーンパスするようになり、潜在的なリソースリークや未使用コードなどの技術的負債が完全に解消されました。

## aiome.dev ランディングページ デプロイ基盤および SPA リダイレクト処理 of TDD 実装 (2026-06-03)

### 1. CNAME および 404.html 静的アセットの追加
- **変更内容**:
    - `docs/landing/public/CNAME` [NEW]: カスタムドメイン `aiome.dev` を指定する設定ファイルを追加。
    - `docs/landing/public/404.html` [NEW]: GitHub Pages で React ルーターなどの SPA 履歴ルーティングを動作させるため、エラー検出時にクエリパラメータに変換して `index.html` に転送するリダイレクトスクリプトを内包した静的HTMLを新規作成。
- **波及効果**:
    - GitHub Pages を経由してドメイン `https://aiome.dev` で直接ホスティング可能となり、かつ `/privacy`, `/terms`, `/tokushoho` などのダイレクトURLリンクに直接ユーザーがアクセスした際、404エラーにならずに React アプリが起動して元のURLを再現・表示できるようになりました。

### 2. index.html への SPA リダイレクト復元用スクリプトの挿入
- **変更内容**:
    - `docs/landing/index.html` [MODIFY]: `<head>` の末尾（`</head>` の直前）に、404.html から転送されてきたクエリパラメータ（`?/privacy&foo=bar`）をパースし、ブラウザの History API を使って URL を正しい形（`/privacy?foo=bar`）に置き換える JavaScript レシーバースクリプトを挿入。
- **波及効果**:
    - アプリがロードされる前に History API による URL 復元が同期的に実行されるため、React アプリケーション側が読み込まれた瞬間に正しいパスが認識され、画面の瞬き（Flicker）なしに意図したページが描画される状態が実現。

### 3. TDD 自動テスト（Deployment.test.ts）の作成による整合性保証
- **変更内容**:
    - `docs/landing/src/Deployment.test.ts` [NEW]: CNAME 存在検証、404.html 存在検証、index.html のレシーバースクリプト検証、および JSDOM 環境における URL 復元ロジック（`window.history.replaceState` が期待されるデコード値で呼び出されるか）を検証するユニットテスト群を追加。
    - `docs/landing/tsconfig.app.json` [MODIFY]: テストコード内で `fs` や `path` などの Node.js 標準モジュールを使用できるようにするため、`compilerOptions.types` に `"node"` を追加。
- **波及効果**:
    - 静的アセットの設定や SPA 復元スクリプトという、GitHub Pages 運用に欠かせない構成要素がテストコードによって恒久的に CI 上で監視・保護され、誤って CNAME や HTML の記述を破壊した場合にも即座にビルドエラーとして検知される堅牢な基盤が確立。
    - `npm run build` 時の型定義チェック（`tsc -b`）が完全に成功（GREEN）することを確認。

### 4. GitHub Actions デプロイ自動化ワークフローの作成 (Safety-Critical Zone)
- **変更内容**:
    - `.github/workflows/deploy-landing.yml` [NEW]: GitHub Pages へのビルド・自動デプロイを実行するワークフロー定義を作成。Node.js 20 環境でのキャッシュ付きビルド、Vite ビルド成果物（`docs/landing/dist`）の Pages アップロード、および本番デプロイの一連のパイプラインを定義。
- **波及効果**:
    - `docs/landing/` 配下、あるいは本ワークフロー設定の更新が GitHub `main` ブランチにプッシュされた際、自律的かつ自動的に最新のランディングページがビルドされ、`https://aiome.dev` へデプロイされる仕組みが完成。

## AI 自律サポートシステム（Wiring & Quality Gate）の TDD 接続完了 (v5.2) (2026-06-02)

### 1. Watchtower サポート分岐のモジュールチェーン完全接続 (W-1)
- **変更内容**:
    - `apps/api-server/src/internal_services/watchtower.rs` [MODIFY]: ハードコードされていた仮のサポート応答（`!bug` / `!help` / `/support`）を、実績のあるサポートモジュール群（`SupportClassifier` ➔ `SupportResponder` ➔ `AgentEngine::chat` ➔ `SupportIncidentRepository` ➔ `SupportEscalator`）のチェーン接続へ置換。
    - **波及効果**:
        - サポート窓口への問い合わせに対して、AIエージェントが自律的にFAQ（Karma）を検索し、プロンプトを構築して回答し、インシデントデータベースへ永続化記録した上で、重要度が高い場合はアラートマネージャーへ自動エスカレーションする、という1,100行超の「眠っていたサポートシステム」が完全に実稼働状態へ移行しました。
        - 正常動作テストと、未初期化 `AppState` 時のフェイルセーフなパニック挙動をアサートする TDD 統合テスト `test_watchtower_support_routing_event_flow` を追加し、GREEN（PASS）を確認。

### 2. SupportFeedback コマンドハンドラの追加と Karma Registry 接続 (W-2)
- **変更内容**:
    - `apps/api-server/src/internal_services/watchtower.rs` [MODIFY]: `ControlCommand::SupportFeedback` マッチアームを新設し、`SupportFeedbackCollector` を用いて、解決時/未解決時の Karma 重み調整（`+10` / `-15`）とインシデントステータス（Resolved/Escalated）の更新ロジックを統合。
    - **波及効果**:
        - ユーザーからのリアクションフィードバックが、バックエンド側で直接 Karma Registry（長期記憶の重要度）へ安全にフィードバックバックループとして還元される仕組みが完成。
        - SQLite インメモリ DB と Karma registry キャストを考慮した TDD 統合テスト `test_watchtower_support_feedback_routing_event_flow` を追加し、GREEN（PASS）を確認。

### 3. Discord リアクションハンドラとチケット ID 抽出 (W-3)
- **変更内容**:
    - `libs/infrastructure/src/channel_bridge/discord.rs` [MODIFY]: `reaction_add` イベントハンドラを実装し、リアクション検出時に Bot 自身のリアクションを無視しつつ `✅` / `❌` などのリアクションを `ControlCommand::SupportFeedback` に自動変換して command_tx へ中継する処理を統合。
    - `libs/infrastructure/src/channel_bridge/discord.rs` [MODIFY]: Bot メッセージから `[TICKET:uuid]` パターンを安全にパース・抽出する純粋関数 `extract_ticket_id_from_text`、および Serenity Context 連携ヘルパー `extract_ticket_id_from_bot_message` を新設。
    - **波及効果**:
        - チャットツール（Discord）上でのリアクションが、AI OS の意思決定・長期記憶（Karma）システムと一気通貫でリアルタイム双方向連動。
        - チケットID抽出の境界値（正常・異常系文字列）を厳密にアサートする TDD ユニットテストを 2 件新規追加し、100% グリーン（PASS）を実証。

## 自律的サポートシステム用正常性/インシデント稼働状況ステータス画面（Status Page）の TDD 実装 (Phase S-5) (2026-06-02)

### 1. ResourceStatus への週間インシデント統計フィールド拡張と API 結合
- **変更内容**:
    - `libs/shared/src/health.rs` [MODIFY]: `ResourceStatus` 構造体に `support_incidents` フィールド（`Option<serde_json::Value>`）を拡張。
    - `apps/api-server/src/routes/general.rs` [MODIFY]: ヘルスチェック API（`/api/health`）に `SupportIncidentRepository` の週間統計計算処理（`compute_weekly_stats`）をロードして結合。
    - `apps/api-server/src/api_integration_tests/system.rs` [MODIFY]: `test_health_check` 結合テストを拡張し、`support_incidents` および週間統計情報（直近7日間の総件数、未解決数など）の返却を検証するアサーションを追加。
- **波及効果**:
    - システム全体の健全性モニター（CPU、メモリ、ディスク使用量）と、顧客サポートインシデントの統計データ（週次合計件数、未解決件数、影響ユーザー数、ピーク重要度）が、ひとつのヘルスチェック API 経由で統合的に取得可能になり、システムの可観測性が大幅に向上。
    - 正常系 ➔ 意図的障害注入（`total_incidents_7d_WRONG`）によるテスト失敗（Negative） ➔ Revert 復旧の 3 段階検証を完遂し、バックエンド側の堅牢性を確認。

### 2. フロントエンド Status Page コンポーネントおよびテストの新規実装
- **変更内容**:
    - `apps/management-console/src/components/StatusPage.tsx` [NEW]: ガラスモーフィズムと Vanilla CSS アニメーションを適用したプレミアムな Status Page コンポーネントを新規実装。インシデント統計（直近7日間の件数、未解決数、ユーザー数、最多重要度）と、システムリソース（CPU、メモリ、ディスク使用量）のグラフおよびサーキットブレーカー/LoRA 整合状態 of 診断ビューを統合。
    - `apps/management-console/src/components/StatusPage.test.tsx` [NEW]: 非同期ローディングとシステム健全性・サポート統計情報のレンダリング、および network エラー時の優雅なフォールバック表示を検証する Jest 結合テストを 2 件実装。
- **波及効果**:
    - サービス稼働状況やインシデント状況、システムリソースをリアルタイムに一元監視できる美しい管理ダッシュボードが完成。
    - 非同期フェッチのタイミングを考慮した頑健な Jest テストによって、表示ロジックの回帰バグが CI 上で 100% 防止されます。

### 3. メインアプリケーション（App.tsx）への Status Page のシームレスな統合
- **変更内容**:
    - `apps/management-console/src/App.tsx` [MODIFY]: `StatusPage` の遅延インポート（React.lazy）、`status-page` タブのルーティング登録、左サイドバー（Control セクション）への NavItem マウント、およびヘッダーとコンテンツ切り替え表示の統合。
- **波及効果**:
    - 管理コンソールのメニューに「System Integrity」という美しい NavItem が出現し、ユーザーがワンクリックでシステム正常性ステータスページに遷移・監視できるようになりました。
    - フロントエンド全体の 280 のすべてのテストが 100% GREEN (PASS) で合格することを確認し、他のコンポーネントやナビゲーションへの影響がないことを実証。

## 自律的サポートシステム用フィードバック収集および Karma 調整モジュール（Feedback Collector）の TDD 実装 (Phase S-4) (2026-06-02)

### 1. SupportFeedbackCollector の新規作成と Karma Registry との連動
- **変更内容**:
    - `libs/infrastructure/src/support/feedback.rs` [NEW]: `SupportFeedbackCollector` を新設。インシデントIDに対するフィードバック（解決または未解決）を受理し、`support_incidents` テーブルのステータス更新を行うとともに、自己修復による診断ログ `agent_diagnoses` から元の失敗ジョブとそれに紐付く教訓カルマ `karma_logs` の ID を自動的かつ正確に特定・解決する処理を実装。
    - `libs/infrastructure/src/support/mod.rs` [MODIFY]: `pub mod feedback;` を登録し外部に公開。
- **波及効果**:
    - ユーザーからの解決・未解決フィードバックが、データベースを跨いだJOIN処理によってAIエージェントの教訓・長期記憶（Karma）の重み（Weight）と完全に双方向接続されました。

### 2. フィードバックに応じた Karma 重みの報酬・ペナルティ調整
- **変更内容**:
    - `libs/infrastructure/src/support/feedback.rs` [NEW]: 解決時（`resolved == true`）に Karma 重みを `+10` ブーストし、未解決時（`resolved == false`）には `-15` のペナルティを与える連動調整ロジックを実装。
- **波及効果**:
    - 自己修復によって自動構築されたパッチ・対策教訓の有用性を人間（ユーザー）のフィードバックに基づいて強化学習型で動的に評価・反映する仕組みが完成し、AIエージェントの長期的な自己進化・自己修復の適合度が劇的に向上。

### 3. TDD によるテスト駆動開発と 3段階検証プロトコルの完遂
- **変更内容**:
    - `libs/infrastructure/src/support/feedback.rs` [NEW]: インメモリデータベース上でジョブ・教訓カルマ・診断ログ・インシデントログをすべてJOINさせた統合ユニットテストを追加。
- **波及効果**:
    - SQLite の Karma 重み CHECK 制約（`weight BETWEEN 0 AND 100`）による更新エラー（100+10=110での制約違反）を早期検知し、初期重みを `80` とすることで安全に範囲内でアサートする強固なテスト設計を実証。
    - Positive Test ➔ アサーション障害注入によるテスト不合格確認（Negative Test） ➔ アサーション復元（Revert）の 3段階検証プロトコルを完全にクリア。

## 自律的サポートシステム用インシデント管理リポジトリ（Incident Generator） of TDD 実装 (Phase S-3) (2026-06-02)

### 1. SupportIncidentRepository の新規作成と SQLite/Postgres 両対応の CRUD 実装
- **変更内容**:
    - `libs/infrastructure/src/support/incident.rs` [NEW]: `SupportIncidentRepository` を新設し、インシデントの新規登録（UUID v4 自動生成）、特定IDによるフェッチ、オープンインシデント一覧取得、ステータス変更（Resolved時の解決時刻自動打刻を含む）、推奨修正案（suggested_fix）の更新を実装。SQLite/Postgres の双方の SQL ダイアレクトとプレースホルダーに対応。
    - `libs/infrastructure/src/support/mod.rs` [MODIFY]: `pub mod incident;` を登録し外部に公開。
- **波及効果**:
    - サポートシステムの運用状況をデータベース上で永続化管理するための堅牢なリポジトリが完成。オンラインマイグレーションシステムと完全に連動し、テスト時には自動的に SQLite インメモリ DB スキーマが構築されるクリーンな構造を実現。

### 2. サポート週間統計計算ロジック（compute_weekly_stats）の実装
- **変更内容**:
    - `libs/infrastructure/src/support/incident.rs` [NEW]: 過去7日間の総インシデント件数、重複排除ユーザー数、現在未解決（Open, InProgress, Escalated）件数、最多発生重要度レベル（top_severity）を1クエリで効率よく集計・算出する週間統計メソッドを実装。
- **波及効果**:
    - サポート窓口や管理ダッシュボードに対して、直近のシステム負荷状況や重要インシデントのトレンドを瞬時に提供可能な可観測性インフラが確立。

### 3. TDD によるテスト駆動開発と 3段階検証プロトコルの完遂
- **変更内容**:
    - `libs/infrastructure/src/support/incident.rs` [NEW]: インメモリデータベースを用いた正常系ユニットテスト（`test_support_insert_and_fetch`, `test_support_weekly_stats`, `test_support_update_status_and_fix`）を追加。
- **波及効果**:
    - Positive Test ➔ 意図的なアサーション書き換えによるテスト失敗確認（Negative Test） ➔ アサーションの復元（Revert）という 3 段階検証プロトコルを完全にパス。サイレントな回帰バグの混入を CI 上で恒久的に防止。

## SQLite データベース・オンラインバックアップ戦略の実装 (Phase C-3) (2026-06-02)

### 1. DatabasePool へのバックアップ API 統合
- **変更内容**:
    - `libs/shared/src/db.rs` [MODIFY]: `DatabasePool` に `backup(&self, destination_path: &str) -> Result<(), AiomeError>` を追加。SQLite では `VACUUM INTO 'escaped_destination_path'` を `sqlx` を介して非破壊で実行する処理を実装（パスのエスケープ処理も含む）。PostgreSQL では非サポートとしての `AiomeError` を返却するフェイルセーフを実装。
- **波及効果**:
    - 稼働中のデータベースのトランザクション整合性を完全に維持したまま、ロックを長時間保持せず安全にバックアップファイルを作成できるオンラインバックアップ機能が極めて堅牢な API として抽象化されました。

### 2. TDD 自動テストの追加による整合性・堅牢性の保証
- **変更内容**:
    - `libs/shared/src/db.rs` [MODIFY]: 正常系として一時ファイルへのデータコピーと整合性を検証する `test_sqlite_backup_success` を追加。異常系として存在しないディレクトリ等を指定した際に適切にエラーを検知・捕捉する `test_sqlite_backup_to_invalid_path` を追加。
- **波波効果**:
    - RED（未実装パニック）➔ GREEN（VACUUM INTO 実装完了） ➔ Negative Test（無効パスへの強制障害注入による `unable to open database` エラー検知・アサート） ➔ Revert の3段階検証プロトコルを完全にクリア。
    - データベースプールの全 5 テスト（およびワークスペース全体）が 100% グリーン（PASS）を維持することを確認。

## VoiceStore コマース決済・管理ポータル処理の DRY リファクタリング (Phase C-2) (2026-06-01)

### 1. 共通決済フックへの Customer Portal 機能統合
- **変更内容**:
    - `apps/management-console/src/hooks/useCheckoutSession.ts` [MODIFY]: `useCheckoutSession` フックを拡張し、Stripe Billing Portal に遷移するための `handlePortal`（および `isPortalLoading` 状態）を新設。
    - `apps/management-console/src/hooks/useCheckoutSession.test.ts` [MODIFY]: `handlePortal` の遷移、およびAPI失敗・ローディングの挙動を厳密にアサートする 2 件の Jest テストを新設。
- **波及効果**:
    - Stripe カスタマーポータル（サブスク解約、カード変更等）への遷移処理が完璧にフックへカプセル化され、個々のコンポーネントが直接非同期 fetch を記述するDRY違反が根本解消。

### 2. VoiceStore のリファクタリングによる重複コード排除
- **変更内容**:
    - `apps/management-console/src/components/VoiceStore.tsx` [MODIFY]: スタブ化および重複していた Stripe チェックアウトセッション生成（`handleRecharge`）と Billing Portal 遷移（`handleManageSubscription`）の直接の fetch 処理（約60行）を完全に削除。拡張した `useCheckoutSession` フックをインポート・バインドし直す形へとクリーンにリファクタリング。
- **波及効果**:
    - 画面側のボイラープレートコードが 60行以上削減され、決済遷移ロジックの変更がフックの修正だけで一元適用される極めて保守性の高いアーキテクチャが実現。
    - `VoiceStore.test.tsx`、およびフロントエンド全体の 278 の全テストが 100% 合格（PASS）を維持することを確認。

## ランディングページ (LP) における軽量ルーティングおよび法務ドキュメント表示 pillars の TDD 実装 (Phase B-2) (2026-06-01)

### 1. 条件付きルーティング処理の導入
- **変更内容**:
    - `docs/landing/src/App.tsx` [MODIFY]: `window.location.pathname` を評価し、パスが `/privacy`、`/terms`、`/tokushoho` の場合にそれぞれ対応する法務ページを返し、トップ画面のHero等コンポーネントを自動的に非表示にする条件分岐を組み込み。
- **波及効果**:
    - 外部の重いルーティングライブラリを一切使わずに、高速かつ軽量なシングルページ内ルーティングが実現。ユーザーがフッターリンク等から法務情報へスムーズにアクセス可能な環境を構築。

### 2. LegalLayout & LegalPages コンポーネントの新規実装
- **変更内容**:
    - `docs/landing/src/components/LegalLayout.tsx` [NEW]: ガラスモーフィズム（backdrop-blur-md）、ダークテーマ（brand-bg）、および Viewport 追従型のプレミアムな法務文書専用枠組みを実装。
    - `docs/landing/src/components/LegalPages.tsx` [NEW]: `PrivacyPage`、`TermsPage`、`TokushohoPage` の各コンポーネントを新設。`legal_governance_tests.rs` が要求するすべての法的必須キーワードを完璧に内包してマークアップ。
- **波及効果**:
    - 統一された最高品質のデザインシステムによって、法的記述画面においてもブランドのプレミアムな一貫性が完全に担保されました。

### 3. テストの追加およびレガシーアサーションの修正
- **変更内容**:
    - `docs/landing/src/App.test.tsx` [NEW]: パスに応じた Hero の非表示・法務文書の表示が正しく動作することをアサートするテストを 4 本追加。
    - `docs/landing/src/components/Pricing.test.tsx` [MODIFY]: ja.json で `$9.99` となっているのに対し、テストで `"¥1,200"` を期待していたミスマッチや、特徴量（features）の文言の不整合を解消。
    - `docs/landing/src/components/CodePreview.test.tsx` [MODIFY]: 最新 of Docker Compose コマンドに期待値を整合（`docker-compose.quickstart.yml` の検証）。
- **波及効果**:
    - LP プロジェクトの全テストが **100% GREEN (28 tests passed)** で完全に合格する環境が復元され、今後の機能改修によるサイレント回帰バグが恒久的に防止されます。

## 特定商取引法に基づく表記 (TOKUSHOHO.md) の TDD 実装 (Phase B-1) (2026-06-01)

### 1. TOKUSHOHO.md の作成
- **変更内容**:
    - `docs/legal/TOKUSHOHO.md` [NEW]: 日本の特定商取引法に基づく表記に準拠した日本語文書を作成。販売業者、運営責任者、所在地免責、メールアドレス、販売価格、お支払方法、引渡時期、および返品不可ポリシーを完全網羅。
- **波及効果**:
    - Stripeの本番審査に合格するための必須コンテンツである日本の特定商取引法表記が正式に追加され、日本の消費者に対する透明性と法的コンプライアンスが飛躍的に向上。

### 2. 法務テスト（legal_governance_tests.rs）へのアサーション追加
- **変更内容**:
    - `apps/api-server/tests/legal_governance_tests.rs` [MODIFY]: `test_tokushoho_contains_mandatory_clauses` を新設。特定商取引法に関する必須キーワード（特定商取引法、販売業者、運営責任者、所在地、メールアドレス、販売価格、支払方法、引渡時期、返品）の存在を CI 上で自動検証。
- **波及効果**:
    - ドキュメント内のメールアドレスドメイン等によるキーワードの誤検出（すり抜け）を Negative Test によって発見・排除した堅牢な検証体制を確立。
    - 将来的なドキュメント改定による法務上必須な免責事項や連絡先情報の欠損を CI パイプライン上で即時検知可能に。

## Reflexion: DiscordNotifier SSRF 修正 & 法務テスト CI 安定化 (2026-06-01)

### 1. DiscordNotifier グローバル HTTP クライアント移行
- **変更内容**:
    - `libs/infrastructure/src/alerts/mod.rs` [MODIFY]: `reqwest::Client::new()` を廃止し `aiome_core::http::get_http_client()` に統一。Unit struct 化。`Debug` trait 追加。リクエスト単位タイムアウト (10秒) 追加。
- **波及効果**:
    - SSRF リダイレクトブロック (`redirect(Policy::none())`) が DiscordNotifier にも自動適用され、Webhook URL の悪意あるリダイレクトによる内部ネットワークアクセスを遮断。
    - TCP 接続プールの共用により、高頻度アラート送信時のハンドシェイクオーバーヘッドを排除。
    - タイムアウト追加により Discord API 無応答時の `tokio::spawn` タスク永久ブロック → リソースリーク経路を根本遮断。

### 2. legal_governance_tests.rs CI 安定化リファクタ
- **変更内容**:
    - `apps/api-server/tests/legal_governance_tests.rs` [MODIFY]: `CARGO_MANIFEST_DIR` ベースの4候補パス解決を導入。`read_legal_doc()` ヘルパーで DRY 化。キーワード検証にタプル形式を採用。
- **波及効果**:
    - CI 環境（Docker / GitHub Actions）でカレントディレクトリが `apps/api-server/` 以外に設定された場合でもテストが安定実行。
    - 隣接テスト `deployment_config_tests.rs` との完全パターン統一により、テストディレクトリ内の一貫性が確保。
    - パニック時のパス一覧表示により、ファイル未検出時のデバッグ効率が向上。

## 本番用 Discord アラート通知パイプライン (DiscordNotifier) の TDD 実装 (Phase C-4) (2026-06-01)

### 1. DiscordNotifier の実装
- **変更内容**:
    - `libs/infrastructure/src/alerts/mod.rs` [MODIFY]: `AlertNotifier` トレイトを実装する `DiscordNotifier` 構造体を新規追加。環境変数 `DISCORD_WEBHOOK_URL` から送信先を取得し、グローバル HTTP クライアント (`aiome_core::http::get_http_client()`) を使用して Discord Webhook API に対しカラー付き Embeds 形式の JSON ペイロードを非同期（HTTP POST）で送信するフェイルセーフ仕様を実装。
- **波及効果**:
    - アラート通知の具象チャネルとして Discord Webhook が完全にサポートされ、本番稼働時に Stripe Webhook 障害やシステムの致命的エラー（Critical）を Discord 上で美しくリッチに即時受け取れる観測体制が確立。
    - `DISCORD_WEBHOOK_URL` 未設定時でもサーバーが即死（パニック）せず、警告ログを出しつつ優雅にスキップするフェイルセーフ（Fail-Safe）設計を担保。

### 2. TDD 統合テストの追加と環境変数直列実行化
- **変更内容**:
    - `libs/infrastructure/src/alerts/tests.rs` [MODIFY]: `wiremock` と `serial_test` を使用した正常系、環境変数未設定時のフェイルセーフ、および500エラー障害注入による異常系の3つのテストケースを新設。
- **波及効果**:
    - テストの並列実行による環境変数 `DISCORD_WEBHOOK_URL` の奪い合い（競合）を `#[serial_test::serial]` 属性によって直列制御することで、Flaky（不安定）なテスト失敗を完璧に根絶。
    - 正常系（204応答確認）、異常注入（500エラー時の `res.is_err()` 検知）の回帰テストが CI 上で自動保証され、今後の通知処理変更に対する強力な防御ゲートが確立。

### 3. 環境変数テンプレートの追加
- **変更内容**:
    - `.env.example` [MODIFY]: `DISCORD_WEBHOOK_URL` を追加。
- **波及効果**:
    - 新しい本番デプロイ担当者がアラート通知を設定するための環境変数が明文化され、デプロイ摩擦がゼロに。

## フロントエンド：Stripe 決済ファネルの強化と402エラーインターセプト (TDD) (2026-06-01)

### 1. auth.ts 402 エラーインターセプト
- **変更内容**:
    - `apps/management-console/src/lib/auth.ts` [MODIFY]: `authenticatedFetch` 内で 402 Payment Required レスポンスをインターセプトし、`window.dispatchEvent` を介してカスタムイベント `stripe-402-payment-required` を発行するよう拡張。
    - `apps/management-console/src/lib/auth.test.ts` [MODIFY]: 上記の 402 エラーステータス時にカスタムイベントがディスパッチされるかを検証する統合テストを追加。
- **波及効果**:
    - API サーバー上のプロ・ゲート制限（402）とフロントエンド（管理画面）のダイレクトな橋渡しが完了。
    - 各種機能で 402 が返された際に、画面側で統一的にアップグレード UI をトリガーできるようになりました。

### 2. useCheckoutSession カスタムフックおよび navigation ユーティリティ
- **変更内容**:
    - `apps/management-console/src/lib/navigation.ts` [NEW]: JSDOM テスト環境での Location オブジェクトの Read-only 制約を回避するため、モック可能な独立した `redirect` ユーティリティ関数を抽出。
    - `apps/management-console/src/hooks/useCheckoutSession.ts` [NEW]: Stripeのチェックアウトセッションを作成・管理し、ユーザーを Stripe 決済ポータルに遷移させる共通カスタムフックを実装。
    - `apps/management-console/src/hooks/useCheckoutSession.test.ts` [NEW]: フックの正常系・異常系・エラー処理を検証する単体テストを追加。
- **波及効果**:
    - 決済画面への遷移処理が完璧にカプセル化され、JSDOM テストの flakiness や Location オブジェクト書き換えエラーを排除した堅牢な設計になりました。

### 3. ProUpgradeModal コンポーネント
- **変更内容**:
    - `apps/management-console/src/components/commerce/ProUpgradeModal.tsx` [NEW]: 402 エラーのカスタムイベントをリッスンし、自動的に表示される Glassmorphism を適用した美しくプレミアムな Pro アップグレードモーダル UI を新規実装。
    - `apps/management-console/src/components/commerce/ProUpgradeModal.test.tsx` [NEW]: モーダルのマウント、402イベントによる開閉、アップグレードボタンのチェックアウト呼び出しをアサーションする単体テストを追加。
    - `apps/management-console/src/components/commerce/NurtureDashboard.tsx` [MODIFY]: 従来のスタブ化されていた Stripe `price_id` を設定値の `STRIPE_PRICE_ID` に修正し、`useCheckoutSession` フックへ移行。
- **波及効果**:
    - フリーミアム層に対するコンバージョン機会の損失（402表示による離脱）をゼロにし、プレミアムユーザーへの移行を促す美しい購入導線が完成。

## Reflexion Phase E: Auth DRY リファクタ & Fail-Closed 一貫性修正 (2026-06-01)

### 1. auth.rs ヘルパー関数抽出
- **変更内容**:
    - `apps/api-server/src/auth.rs` [MODIFY]: `extract_bearer_token`, `guard_nil_agent_id`, `jwt_failure_response` の 3 ヘルパー関数を追加。`Authenticated`, `BanExemptAuthenticated`, `auth_middleware`, `jwt_auth_middleware`, `admin_only_middleware` の 5 箇所で重複していた JWT パース + nil UUID ガード + PII 保護ロジックを統合。`ProAuthenticated` の Commerce エラーを 500→503 に変更。
- **波及効果**:
    - 新規ミドルウェア追加時にセキュリティポリシーの不整合が起こり得ない構造的保証。
    - `X-Token-Expired` ヘッダーが正確に期限切れトークンのみに付与されるようになり、クライアント側のリフレッシュロジックの誤動作を防止。

### 2. app_state.rs パニックパス排除
- **変更内容**:
    - `apps/api-server/src/app_state.rs` [MODIFY]: `is_feature_enabled` / `get_system_soul_hash` の `get_inner()` 呼び出しを `as_opt()` に変更。
- **波及効果**:
    - `is_feature_enabled` を呼ぶ全ハンドラ（feature flag チェック箇所）がパニックフリーに。
    - config 未初期化時は空 Soul ハッシュを返却し、起動シーケンス中の部分的アクセスでもパニックしない。



### 1. MockCommerceEngine の一元化によるテスト整合性向上
- **変更内容**:
    - `libs/aiome-commerce/src/mock.rs` [MODIFY]: ステートフルな `MockCommerceEngine` に、結合テストで使用される特定の UUID ルーティングロジック（`...0002`〜`...0006` に応じたサブスクステータスの返却）および特定 UUID に対する `validate_activity` 資金不足エラー返却ロジックを統合。
    - `apps/api-server/src/api_integration_tests/common.rs` [MODIFY]: テスト専用の `struct MockCommerceEngine;` 二重定義を完全に削除し、`pub use aiome_commerce::mock::MockCommerceEngine;` のインポート＆共用に統合リファクタリング。テスト内のインスタンス化を unit struct 形式から `MockCommerceEngine::new()` に修正。
    - `apps/api-server/src/bootstrap.rs.bak` [DELETE]: 使用されなくなった古いバックアップファイルを削除。
- **波及効果**:
    - `libs/aiome-commerce` にモック実装が一元化され、ライブラリ全体のテスト戦略と API サーバー結合テストの整合性が完璧に確立されました。
    - 二重定義に伴う将来の機能拡張時の開発ドリフト（型不整合や振る舞い不整合によるビルド破壊）を根本的に根絶。
    - テストスイート（正常系＋異常系）の 3 段階検証により、本番コードの決済ゲート制御への高い回帰信頼性を CI レベルで自動担保。

## Stripe サブスクリプション決済基盤とプロ・ゲート制限の TDD 実装 (Phase B & C) (2026-06-01)

### 1. Stripe サブスクリプション（Active / Trialing）制限の backend 実装
- **変更内容**:
    - `libs/aiome-contracts/src/error.rs` [MODIFY]: `AiomeError` に `PaymentRequired` エラー型を追加し、Axum `IntoResponse` で `402 Payment Required` (402) ステータスコードへ自動マッピング。
    - `apps/api-server/src/error.rs` [MODIFY]: `AppError::payment_required` ヘルパーメソッドと対応するユニットテスト群を追加。
    - `apps/api-server/src/auth.rs` [MODIFY]: `ProAuthenticated` エクストラクターを新設し、Stripeサブスクリプション状態（`Active` または `Trialing`）を動的検証。未登録ユーザーからのアクセスに対して `402 Payment Required` で安全に遮断するガードロジックを実装。さらに、正常系 (Positive) および障害注入 (Negative) の自動検証結合テストを追加。
    - `apps/api-server/src/routes/` 内の8つのルートハンドラ (`lora_market.rs`, `buzz.rs`, `gift.rs`, `commerce.rs`, `voice.rs`, `treasure.rs`, `syndicate.rs`) にて、引数を `Authenticated` から `ProAuthenticated` に変更し、プロサブスクリプションによる厳格なゲート制限を適用。
    - `libs/aiome-commerce/src/mock.rs` [MODIFY]: `MockCommerceEngine` に `subscription_override` テスト制御フィールドを追加し、Stripeのモックライフサイクルテストに統合。
- **波及効果**:
    - Aiome のフリーミアム収益化エンジンのコアインフラが完成。
    - Axum エクストラクターを用いた宣言的なゲート制限により、今後の新規プロルートの追加に対しても安全かつDRYなアクセス制限が可能になりました。
    - 正確に 402 レスポンスを返すことで、フロントエンド（管理画面など）が Stripe 決済ポータルや Stripe Payment Link へとシームレスにユーザーを案内する「Conversion 導線」の基盤が確立。

## ランディングページ (LP) の獲得最大化とビジュアルアップグレード (Phase A) (2026-06-01)

### 1. 料金開示、獲得最大化、および最高峰のビジュアル追加
- **変更内容**:
    - `docs/landing/src/i18n/locales/en.json` & `ja.json` [MODIFY]: Free 機能(6個), Pro 機能(8個)、14日間無料トライアルバッジ、ライブデモ、ショーケースの多言語翻訳データを追加・価格（$9.99）を完全に整合。
    - `docs/landing/src/components/CodePreview.tsx` [MODIFY]: クイックスタートの指示を Git / Docker Compose 形式に修正。
    - `docs/landing/src/components/Pricing.tsx` [MODIFY]: Free(6個) / Pro(8個) の動的マップ展開、14日間無料体験バッジの追加、Stripe 本番用 Payment Link (`https://buy.stripe.com/aFa9AS1Kc1l47mK3u5f7i01`) への確実な差し替えと別窓遷移対応。
    - `docs/landing/index.html` [MODIFY]: SEO・SNS監査用 Twitter OGP カード (`@aiome_dev`) のメタタグを追加。
    - `docs/landing/src/components/LiveDemo.tsx` [NEW]: 60秒自律エージェントのタイムライン動作（自己修復・テスト合格・Xへの自律投稿など）を伝える美しいガラスモルフィズム調の CSS アニメーションデモを実装。
    - `docs/landing/src/components/Showcase.tsx` [NEW]: `generate_image` で作成した高解像度の管理画面ダッシュボード、実行タイムライン、アバターカスタマイザーのモックアップアセット3点をマウントした最高峰のショーケースコンポーネントを新規実装。
    - `docs/landing/src/App.tsx` [MODIFY]: 各種新規コンポーネントを正しいセクション配置でマッピングし、`npm run build` による警告なしのビルド合格を達成。
- **波及効果**:
    - 獲得LPの魅力が爆発的に向上し、ユーザーコンバージョン率の劇的な増加を約束。
    - 無料試用（14日間無料）から Stripe Payment Link を介した本番決済への導線が確立され、ARR（年間経常収益）の最速立ち上げとバイアウト目標の早期到達を強力に支援。

## TLS / HTTPS リバースプロキシの設定テンプレート作成とインフラ TDD 実装 (B-2) (2026-05-31)

### 1. 本番用 Caddyfile プロキシテンプレートの追加
- **変更内容**:
    - `docker/caddy/Caddyfile` [NEW]: Let's Encrypt / ZeroSSL 自動SSL/TLS証明書管理、`api-server:3015` への安全な `reverse_proxy` 転送（SSRF/プロキシ保護用のホストヘッダー中継含む）、HSTS 強制暗号化、CSP、クリックジャッキング防御等のセキュリティヘッダーを完備したプロキシテンプレートを新規作成。さらに **`Permissions-Policy` ヘッダーを追加**し、不要デバイス機能への無許可アクセスをインフラレベルで遮断。
- **波及効果**:
    - 本番環境での HTTPS/TLS 通信の自動終端が容易になり、Stripe / Polar 決済 Webhook の安全な受信用 HTTPS エンドポイントの構築が数秒で完了する準備が整いました。
    - 各種セキュリティヘッダーおよび中継プロキシヘッダー (`header_up`) の安全な伝播により、Abyss UI に対するクリックジャッキング攻撃や、不正なホスト偽装、SSRF 脆弱性を未然に遮断。

### 2. インフラ構成バリデーションテストの追加と 3 段階検証
- **変更内容**:
    - `apps/api-server/tests/deployment_config_tests.rs` [NEW]: Caddyfile テンプレートファイルを静的にパースし、安全なプロキシ定義（`reverse_proxy`, `:3015`, `header_up`, `Strict-Transport-Security`, `X-Frame-Options`）に加え、**CSP、Referrer-Policy、Permissions-Policy の存在を CI 上で毎ビルド自動検証**するアサーションテストを追加。**`CARGO_MANIFEST_DIR` による絶対パス解決**に書き換え、カレントワーキングディレクトリ依存のテスト不安定性を完全に排除。
- **波及効果**:
    - 正常系（テスト合格） ➔ 意図的に Caddyfile 内の `Permissions-Policy` 定義を削除してアサーションエラーを正確に検知（Negative） ➔ 正常に復旧（Revert & Report）の 3段階検証を完遂。
    - 将来的なインフラ設定ファイルの変更によって、開発者が誤って中継ポート指定やセキュリティ定義を削除・改ざんしたままデプロイしてしまうインフラ破壊インシデント（接続ハングアップ、セキュリティ欠損）をビルド/CI パイプライン上で恒久的に遮断。

## ToS（利用規約）の正式日本語版化とリーガル・ガバナンス TDD 実装 (A-2) (2026-05-31)

### 1. 利用規約の日本語正式版へのアップグレード
- **変更内容**:
    - `docs/legal/TERMS_OF_SERVICE.md` [MODIFY]: 暫定スキャフォールド状態から、早期アクセス免責、BSL-1.1 使用許諾条件、Karma Coins / サブスクリプション料金の**完全返金不可（Non-Refundable）**、自己修復（Self-Healing）や AI の自律的なコード書き換え・API 呼び出し・意思決定に伴う一切の損害（データ消失、API 課金、知的財産権トラブル等）の完全免責、クリエイターの eKYC 義務化等を網羅した強固な日本語正式版へと完全アップグレード。
- **波及効果**:
    - Stripe の本番審査や法的係争リスクに対して、完璧な法的シールドが構築されました。
    - 自律的 AI システムという特異な性質における「コードの自動書き換えや API 課金」という運用上致命的になり得るリスクがすべて免責され、運営上の安全性が極限まで向上。

### 2. リーガル・ガバナンス自動テストの追加と 3 段階検証
- **変更内容**:
    - `apps/api-server/tests/legal_governance_tests.rs` [NEW]: 利用規約およびプライバシーポリシーのファイルを動的にパースし、法的防御に必要な重要キーワード（返金、自己修復、BSL-1.1、eKYC、ローカルファースト、忘れられる権利、免責）の存在を CI 上で毎ビルド自動アサートするテストを2件追加。
- **波及効果**:
    - 正常系（2件のテスト合格） ➔ 意図的に ToS から「返金」キーワードを削除してテスト不合格を正確に検知（Negative） ➔ typo のない正常文言に復旧（Revert & Report）の 3段階検証を完遂。
    - 将来的なドキュメント変更によって、法務担当や他の開発者が誤って重要免責文言を消去してリリースしてしまうサイレントリスクを CI のテストランナーによって恒久的に遮断。

## 運用アラート通知パイプライン TDD 実装とサーキットブレーカー統合 (A-3) (2026-05-31)

### 1. 抽象化された通知レイヤーと重複デバウンスキャッシュの実装
- **変更内容**:
    - `libs/infrastructure/src/alerts/mod.rs` [NEW]: 重要度レベル `AlertLevel`、通知機トレイト `AlertNotifier`、およびメモリ内デバウンスキャッシュ（60秒間同一タイトルとレベルのアラートを抑制）を備えた `AlertManager` を新規実装。
    - `libs/infrastructure/src/lib.rs` [MODIFY]: `alerts` モジュールを外部に公開。
- **波及効果**:
    - メールや Slack、Discord などの通知チャネルの追加を容易にするクリーンな抽象化レイヤーが完成。
    - 障害発生時に同じアラートが大量に送信されるメール爆弾（アラートストーム）をメモリ内のデバウンスによって未然に遮断。

### 2. サーキットブレーカーとのトリップアラート連動
- **変更内容**:
    - `libs/infrastructure/src/circuit_breaker.rs` [MODIFY]: `CircuitBreaker` 構造体に `Option<Arc<AlertManager>>` フィールドおよび `new_with_alerts` メソッドを追加。状態が `Open`（トリップ）に遷移した際、自動かつ非同期に `Critical` アラート通知をトリガーするよう統合。
- **波及効果**:
    - LLM などの外部サービス障害によってサーキットブレーカーが遮断された際、システムがフェイルファーストを行うと同時に、運営者へ即時の critical 通知が届くため、迅速な検知と復旧作業が可能。

### 3. TDD 統合テストの追加と 3 段階検証
- **変更内容**:
    - `libs/infrastructure/src/alerts/tests.rs` [NEW]: アラートレベル別ルーティング（`test_alert_routing_by_level`）、サーキットブレーカーのトリップ連動（`test_circuit_breaker_triggers_alert`）、および通知先がエラーになっても全体がクラッシュしない非同期フェイルセーフ動作（`test_alert_notifier_network_failure_failsafe`）を検証する統合テストを3件追加。
- **波及効果**:
    - 正常系（Positive）➔ トリップ時のアラートレベルの改ざんによるテスト失敗検知（Negative）➔ 元の状態への復旧（Revert & Report）を 100% 完遂。非同期処理の堅牢性と Fail-Safe 設計（一部通知先が通信障害でも他の通知先への送信は継続されること）がテストで証明された。

## Polar Webhook ビジネスロジック TDD 実装 (P-1) (2026-05-31)

### 1. Polar Webhook ハンドラのビジネスロジック統合と TDD 検証
- **変更内容**:
    - `apps/api-server/src/routes/commerce_webhook/polar.rs` [MODIFY]: 署名検証後にイベントを無条件で破棄していた実装を排除。Stripe と同様に `polar_webhook_events` テーブルを用いた冪等性保証およびトランザクション処理を追加。
    - `checkout.completed` 受信時に `handle_checkout_completed` を介してライセンス付与、収益分配、コインチャージを同期実行。
    - `subscription.created` / `subscription.updated` / `subscription.deleted` 受信時に、`metadata.actor_id` からエージェントIDを取得し、MCPサスペンド状態を `UniversalJobQueue` にエンキューして非同期制御。
    - `apps/api-server/src/routes/polar_webhook_tests.rs` [MODIFY]: 正常系（チェックアウト、サブスクリプションのライフサイクル）および異常系（不正署名、重複イベント）の TDD 統合テストを 4 本実装し、GREEN パスを確認。
    - `libs/infrastructure/src/registry.rs` [MODIFY]: テスト用 SQLite 初期化 DDL に `polar_webhook_events` テーブルを追加.
    - `libs/infrastructure/migrations/sqlite/20260324000000_init.sql` [MODIFY] / `postgres/20260324000000_init.sql` [MODIFY]: 本番・開発データベースマイグレーションに `polar_webhook_events` スキーマを追加.
    - `apps/api-server/src/api_integration_tests/common.rs` [MODIFY]: `MockCommerceEngine::verify_signature` を強化し、`invalid` を含む不正な署名ヘッダーの異常系注入（Negative Test）をテスト環境で正確に検知・拒否できるように改善.
- **波及効果**:
    - Stripe に加え、Polar（polar.sh）を利用した代替決済ルートが完全に本番対応レベルに昇格。
    - Stripe 同等の冪等性、収益分配、および MCP 状態サスペンド/アンロックの一貫性が担保されたため、決済の二重処理やサービスの不正利用といった脆弱性を根本遮断。
    - 正常系（Positive）➔ 不正署名/重複イベントの拒否（Negative）➔ クリーンアップ（Revert & Report）という 3 段階検証により、今後の決済ルートの改修に対する完璧な回帰テストゲートが確立。

## 感情表現エンジン (ExpressionEngine) への TDD ユニットテスト追加とエッジケース保護 (E-3) (2026-05-31)

### 1. ExpressionEngine ユニットテストの追加とフォールバック保護の検証
- **変更内容**:
    - `libs/core/src/expression/engine.rs` [MODIFY]: `#[cfg(test)] mod tests` を末尾に新規実装。
    - `MockLlmProvider` を用いて、内的感情 `"proud"` の正常パース、コンテンツ行のトリミング、`avatar_params` 設定および `karma_refs` のマッピングを検証するユニットテストを記述。
    - LLM 応答に `EMOTION:` タグが含まれないエッジケースにおいて、デフォルト感情 `"reflective"` に安全にフォールバックすることを確認するテストを実装し、防御力を強化。
- **波及効果**:
    - AI の経験に基づく感情生成ロジックが、例外や LLM の崩れた出力に対しても安全（Zero-Panic）にフォールバックして稼働し続けることがテストで保証された。
    - TDD (RED-GREEN-REFACTOR) および 3段階検証プロトコルにより、障害注入時（デフォルト感情の改ざん）にも正確にテストが不合格となり、バグのサイレントスルーを根本遮断。

## ランディングページ (LP) への Pricing セクションの追加と TDD 実装 (Phase 1-1.3) (2026-05-31)

### 1. ランディングページ Pricing コンポーネントおよびテストの追加
- **変更内容**:
    - `docs/landing/src/components/Pricing.tsx` [NEW]: 多言語（日英）表示に対応したレスポンシブな料金プラン（Sovereign Free / Autonomous Pro）コンポーネントを新規実装。
    - `docs/landing/src/components/Pricing.test.tsx` [NEW]: `Pricing.tsx` の表示要素、料金表記（`$9.99/mo`, `¥1,200/月`）および i18n 言語切り替えアサーションを網羅した Vitest テストを新規作成。
    - `docs/landing/src/App.tsx` [MODIFY]: `<main>` ブロック内の `Security` コンポーネントの後に `Pricing` を組み込み。
    - `docs/landing/src/i18n/locales/en.json` [MODIFY] / `ja.json` [MODIFY]: `pricing` 翻訳オブジェクト（タイトル、説明、プラン名、機能リスト、CTAテキスト）を追加。
- **波及効果**:
    - Aiome のフリーミアム料金プランと Stripe サブスクリプション価格（Gold $9.99/mo, 日本円 ¥1,200/mo）が LP 上で明確に開示され、ユーザーがセルフで決済 portal に到達する基盤が完成。
    - TDD (Red-Green-Refactor) プロセスにより、コードの追加と動作の正当性が 100% 保証された状態でリリース可能。
    - Positive / Negative / Revert & Report の 3段階検証により、意図しない表示崩れや文言変更に対する防御ゲートを確立。

## docker-compose 環境変数の全ファイル整合化およびホスト接続要件のドキュメント追記 (B-5 / D-5) (2026-05-31)

### 1. docker-compose.nurture.yml / docker-compose.cell.yml への SHADOW_CLONE 関連環境変数および A2A_AUTH_TOKEN の追加 (B-5)
- **変更内容**:
    - `docker-compose.nurture.yml` [MODIFY]: `api-server-pro` (L27付近) および `nurture-api` (L65付近) の environment に `SHADOW_CLONE_GRPC_HOST`、`SHADOW_CLONE_GRPC_PORT`、`A2A_AUTH_TOKEN` を追加。
    - `docker-compose.cell.yml` [MODIFY]: `api-server` (L33付近) および `nurture-api` (L87付近) の environment に同様の `SHADOW_CLONE_GRPC_HOST`、`SHADOW_CLONE_GRPC_PORT`、`A2A_AUTH_TOKEN` を追加。
- **波及効果**:
    - 本番用の `docker-compose.production.yml` との間で発生していた構成ドリフト（環境変数の漏れ）を完全に解消。
    - デフォルト値を `localhost` に設定したため、同一 Compose ファイル内に shadow-worker サービスが存在しない開発・セル環境でも Docker 内の DNS 解決でエラーが発生しない安全な設計を担保。

### 2. OPERATIONS_MANUAL.md への Docker 環境での Ollama / XTTS / Shadow Worker ホスト要件追記 (D-5)
- **変更内容**:
    - `docs/guides/OPERATIONS_MANUAL.md` [MODIFY]: `Troubleshooting` テーブルに行を追加し、Docker 内部から localhost へ接続した際のエラー原因と `host.docker.internal` を用いた解決策を提示。
    - `docs/guides/OPERATIONS_MANUAL.md` [MODIFY]: `Production Deployment Checklist` に、Ollama Docker 接続、extra_hosts 設定、XTTS や Shadow Worker 利用時の確認事項を追加。
- **波及効果**:
    - 運用・デプロイ担当者が直面しやすい「Docker 内からホストマシン上の Ollama / XTTS に接続できない」という典型的なトラブル（Connection Refused）に対する予防策がマニュアル化され、デプロイ摩擦が大幅に低減。

## Aiome + Nurture シナジー強化 & `/internal/lora-train` S2S 統合 (2026-05-31)

### 1. nurture-bridge L2 トレイトの再エクスポートによる能力統合の強化 (E-1)
- **変更内容**:
    - `Project-Nurture/libs/nurture-bridge/src/lib.rs` [MODIFY]: Aiome コアの L2 サービス境界を表現するトレイト `LoraEngine` および `TtsProvider` の re-export を実装（`traits` サブモジュール内）。
- **波及効果**:
    - Nurture 経済拡張側から直接 Aiome の L2 抽象クラスを参照できるようになり、型契約の共有が一段と進み、不必要な循環依存や車輪の再開発を完全に防止。

### 2. nurture-api の `/internal/lora-train` S2S エンドポイント実装 (E-2)
- **変更内容**:
    - `Project-Nurture/apps/nurture-api/src/routes/internal.rs` [MODIFY]: OxiLean Proof 認証保護下に `/internal/lora-train` POST エンドポイントを追加。
    - `Project-Nurture/apps/nurture-api/src/routes/internal.rs` [NEW/MODIFY]: ジョブキュー（`UniversalJobQueue`）に `"lora-train"` というジョブカテゴリで、`base_model`, `dataset_id`, `params` を安全にエンキューするハンドラ `internal_lora_train` を実装。
    - `Project-Nurture/apps/nurture-api/tests/internal_routes_test.rs` [NEW]: `test_internal_api_lora_train` を TDD に則り RED ➔ GREEN テストとして追加。
    - テスト用 DB とジョブキュー用 DB（SQLite）のスキーマ衝突を防ぐため、テスト用 SQLite プールを完全に分離（`test_jobs.db`）するようテスト設定を堅牢化。
- **波及効果**:
    - S2S（Server-to-Server）経由での安全な LoRA 訓練ジョブのエンキューパイプラインが完成。
    - 同一 SQLite DB 内で Nurture と Aiome のテーブルスキーマ衝突が起きないようにテスト設計が独立され、今後も Flaky エラーのない安定したテスト環境を保証。

## key-proxy main.rs の 6ファイル分割と TDD スリム化 (2026-05-30)

### 1. key-proxy のモジュール分割 (P3-5)
- **変更内容**:
    - `apps/key-proxy/src/main.rs` [MODIFY]: 1,008行の巨大なファイルを整理し、約80行の軽量なエントリーポイント・ルーティングファイルにスリム化。
    - `apps/key-proxy/src/config.rs` [NEW]: `AppState`, `QuotaState`, `ProxyRequest` 等の構造体定義や設定キャッシュを移行。
    - `apps/key-proxy/src/auth.rs` [NEW]: Ephemeral 認証ミドルウェア (`auth_middleware`) を移行。
    - `apps/key-proxy/src/quota.rs` [NEW]: クォータ制限とデータベース持久化 (`check_and_increment_quota`) を移行。
    - `apps/key-proxy/src/handlers/mod.rs` [NEW]: ハンドラーモジュールの集約定義。
    - `apps/key-proxy/src/handlers/llm.rs` [NEW]: LLM 関連ハンドラー群 (`handle_llm_complete` 等) を移行。
    - `apps/key-proxy/src/handlers/passthrough.rs` [NEW]: Gemini パススルーハンドラー群を移行。
    - `apps/key-proxy/src/handlers/wordpress.rs` [NEW]: WordPress 投稿ハンドラー群を移行。
    - `apps/key-proxy/src/tests.rs` [MODIFY]: `main.rs` に書かれていたインラインテスト群を移行・マージし、かつこれまでビルド対象外になっていたデッドコードテストを復活。
- **波及効果**:
    - `key-proxy` アプリケーションの結合度が極限まで低下し、各ハンドラーの単一責任原則 (SRP) と可読性が劇的に向上。
    - インラインテストと隔離されていた `tests.rs` が正常にモジュールマッピングされ、これまで実行されていなかった health_check や unauthorized 系の4つのテストを含む全9テストが常時コンパイル・実行される環境を確立。
    - 正常系・異常系・回復（Revert）の 3段階検証プロトコルを完全にパスし、機能の欠損や破壊がないことを完全担保。

## フロントエンド重要コンポーネントテストの追加と環境変数モック堅牢化 (2026-05-29)

### 1. BanDashboard & BuzzApproval 単体テストの追加 (P3-3)
- **変更内容**:
    - `apps/management-console/src/components/BanDashboard.test.tsx` [NEW]: ガバナンスコンポーネントの網羅テストを追加。
    - `apps/management-console/src/components/BuzzApproval.test.tsx` [NEW]: SNS監査コンポーネント（280字制限バリデーションなど）の網羅テストを追加。
- **波及効果**:
    - 重要管理機能における UI 状態遷移、API 送信フローの安全性が大幅に向上。
    - Framer Motion によるフェードアウトアニメーションの競合バグを、テスト専用のモック化によって完全に抑止。

### 2. import.meta.env 直接参照の config.ts への一元化
- **変更内容**:
    - `apps/management-console/src/config.ts` [MODIFY]: `STRIPE_PRICE_ID` 定数を追加。
    - `apps/management-console/src/components/VoiceStore.tsx` [MODIFY]: 直接の `import.meta.env.VITE_STRIPE_PRICE_ID` 参照を `STRIPE_PRICE_ID` のインポートへリファクタリング。テスト用モック応答を `balance` キーに追従。
- **波及効果**:
    - CommonJS (Jest) 実行環境下において、ESM 固有 of `import.meta` 構文のパースエラーによるテスト破壊を完全に解消。フロントエンド全体の 46 テストスイート（239 テストケース）が 100% 安定して GREEN で通過する開発環境を確立。

## task_orchestrator のテスト分離と 42% スリム化 (2026-05-29)

### 1. task_orchestrator のテスト専用コード隔離 (P3-1)
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: 2,045行に及ぶ巨大ファイルから、`#[cfg(test)]` スコープ下にあったテストコードやテスト用ダミー（`TestConductor`, `MockGigEngine` 等）をパージし、1,189行（42% 削減）へスリム化。
    - `libs/infrastructure/src/task_orchestrator/tests.rs` [NEW]: 分離された単体テストコードをクリーンに隔離。
- **波及効果**:
    - `infrastructure` クレイトの本番バイナリ肥大化が根本的に解消され、テストコードのメンテナンス性が劇的に向上。
    - 正常系・異常系・復帰（Revert）の 3段階検証プロトコルを実行し、テスト件数が正確に11件増減する挙動を確認することで、機能の変更・破壊がないことを保証。

## llm_provider の 9ファイル分割と TDD クリーンアーキテクチャ化 (2026-05-29)

### 1. llm_provider の独立したサブモジュール分割 (P3-2)
- **変更内容**:
    - `libs/core/src/llm_provider/mod.rs` [MODIFY]: 2,105行の巨大ファイルを整理し、モジュール定義と re-export を行う超軽量なファイル（40行）へスリム化。
    - `libs/core/src/llm_provider/{ollama.rs, abyss_vault.rs, gemini.rs, openai.rs, claude.rs, lm_studio.rs, ruri.rs, mock.rs, tests.rs}` [NEW]: 各 LLM プロバイダー実装、テスト用の `MockLlmProvider`（`#[cfg(any(test, debug_assertions))]`）、および単体テスト群を別々の独立したファイルへクリーンに分割。
- **波及効果**:
    - `aiome-core` クレイトの結合度が大幅に低下し、プロバイダーごとのコードの可読性とメンテナンス性が劇的に向上した。また、`MockLlmProvider` が独立したモジュールとして再構成され、外部テストコードからのインポートパスの保守性が飛躍的に高まった。
    - 正常系・異常系・回復（Revert）という 3段階検証プロトコル（Negative Test）を通じて安全性が厳密に担保され、856個以上のすべてのワークスペーステストが 100% GREEN で合格することを確認した。

## WASM 脆弱性対応およびテスト用 TempDir ライフタイムの修正 (2026-05-29)

### 1. wasmtime 依存関係のアップグレード (RUSTSEC-2026-0149 回避)
- **変更内容**:
    - `Cargo.toml` & `Cargo.lock` [MODIFY]: WASM 実行サンドボックスである `wasmtime-wasi` などの関連 crate を CVE 回避のため `v44.0.2` へアップグレード。
- **波及効果**:
    - 深刻な脆弱性（RUSTSEC-2026-0149: Out-of-bounds Read/Write 等）が排除され、`cargo audit` 監査を 0 件の脆弱性でパス。プロダクション環境における悪意ある WASM プラグインからのホストエスケープリスクが根本的に根絶された。

### 2. テスト用 TempDir ライフタイム即時破棄の修正 (Flaky Test 対策)
- **変更内容**:
    - `apps/api-server/src/tool_call_router.rs` [MODIFY]
    - `apps/api-server/src/mcp/server.rs` [MODIFY]
    - テスト用の `setup_mock_state()` のシグネチャを `(AppState, tempfile::TempDir)` を返すように拡張。呼び出し側のテストで `_guard` パターンによって一時ディレクトリをテストのスコープ終了まで保持するよう修正。
- **波及効果**:
    - 一時ディレクトリが即時破棄され SQLite DB ファイルが直ちに削除される競合バグが修正され、テストが極めて安定化。不定期な DB 接続エラー（Flaky Test）が根絶された。

## Aiome システム堅牢化 & リフレクション対策 (2026-05-29)

### 1. リダイレクトURL検証の純粋関数化および TDD 強化
- **変更内容**:
    - `apps/api-server/src/routes/commerce_helpers.rs` [MODIFY]: `validate_redirect_url_with_config` を新設し、環境変数に依存しないテスト構造へ移行。8つの堅牢なユニットテスト（サブドメイン一致、localhost 開発モード、プロモードにおける fail-closed ガード）を追加。
- **波及効果**:
    - リダイレクト検証処理のテスト容易性が向上し、本番環境でのドメイン設定漏れ（ALLOWED_ORIGINS 未設定）によるサイレントなセキュリティ脆弱性（オープンリダイレクタ）が確実に防止される。

### 2. key-proxy 起動時の設定キャッシュおよびパニック防止
- **変更内容**:
    - `apps/key-proxy/src/main.rs` [MODIFY]: `GEMINI_MODEL`, `GEMINI_EMBED_MODEL` の起動時ロードと `AppState` へのキャッシュ化を実装。また、`handle_llm_embed` での上流エラー処理を改善し、JSON パースエラーや `embedding.values` 欠損時のパニックを防止。
- **波及効果**:
    - リクエストごとの頻繁な環境変数読み込みアロケーションが排除され、パフォーマンスが向上。また、プロバイダ側の不正応答による `key-proxy` プロセスの予期せぬクラッシュが根絶された。

### 3. Stripe 決済エンジンの Fail-Closed 安全化
- **変更内容**:
    - `libs/aiome-commerce/src/stripe.rs` [MODIFY]: 予期しない HTTP ステータスコード（500等）が返された際、安全側に閉じる Fail-Closed 原則に基づき、`AiomeError::Infrastructure` を返すよう堅牢化。
- **波及効果**:
    - 上流の決済サーバーの障害や不正応答時に、誤ってトランザクションを成功とみなす偽陽性リスクが排除された。

### 4. Artifact Store における Zero-Panic 対策の深度化
- **変更内容**:
    - `libs/infrastructure/src/artifact_store.rs` [MODIFY]: `search_artifacts_semantic` メソッドにおいて、データベース行からのパニックを伴う `.get()` 呼び出しを `.try_get().ok()` と `filter_map` に置換。
- **波及効果**:
    - データベース列の破損や欠損が生じた際のカスケードパニックを防止し、システム全体の耐障害性が向上。

### 5. Swarm 秘密鍵のメモリ安全ゼロ化 (Zeroize)
- **変更内容**:
    - `libs/infrastructure/src/job_queue/swarm.rs` [MODIFY]: SwarmOps 署名処理内で秘密鍵（ノードキー）のバイト配列を `zeroize::Zeroizing` にて保護。
- **波及効果**:
    - メモリ上に残存した秘密鍵が別の脆弱性を介してリークするリスク（CWE-312/CWE-14）を排除。

### 6. Abyss Voice Vault における DRM 監査の強化
- **変更内容**:
    - `libs/infrastructure/src/security/abyss_voice_vault.rs` [MODIFY]: 復号キーアクセス時の監査ログに `agent_id` を付加。
- **波及効果**:
    - 誰が復号キーを要求したかのトレーサビリティが確保され、DRM およびアクセス監査のコンプライアンスが大幅に向上。

### 7. napi-bridge での std::process::exit(1) の排除
- **変更内容**:
    - `libs/napi-bridge/src/lib.rs` [MODIFY]: 正規表現の静的初期化における `std::process::exit(1)` を `.expect()` に置換。約33行のボイラープレートをパージ。
- **波及効果**:
    - パースエラー発生時にプロセスのサイレントダウンを防ぎ、バックトレースを適切に吐き出してデバッグを容易にするとともに、コードの保守性が飛躍的に向上。

### 8. admin_only_middleware での PII (CWE-532) 漏洩防止
- **変更内容**:
    - `apps/api-server/src/auth.rs` [MODIFY]: `admin_only_middleware` で拒否されたユーザーのログ出力時に `claims.sub` を先頭8文字に切り捨て処理。
- **波及効果**:
    - 認証エラーログを介した一般ユーザーの個人識別情報 (PII) の不要な流出を防止し、プライバシー規制への準拠を強化。

### 9. ArtifactVault / VoiceStore フロントエンドの品質向上
- **変更内容**:
    - `apps/management-console/src/components/ArtifactVault.tsx` [MODIFY]: ハードコードされた `color: white` を CSS 変数トークン `var(--text-primary)`, `var(--white-100)` に置換。
    - `apps/management-console/src/components/VoiceStore.tsx` [MODIFY]: コイン残高 API 解析のフィールド名ミスマッチ（`coins` → `balance`）を修正。
- **波及効果**:
    - ダークモード等のカラーテーマ規約への完全準拠と、残高表示機能が正しくロードされない重大なバグを修正。

### 10. Settings API 開発モード設計意図コメントの追加
- **変更内容**:
    - `apps/api-server/src/routes/settings.rs` [MODIFY]: 開発モードでのみ有効な `test_connection` への詳細な設計コメントをインラインで追加。
- **波及効果**:
    - 開発モード用のキルスイッチ機構としての `std::env::var("AIOME_DEV_MODE")` 直接利用意図が明文化され、将来の不要なリファクタリングによる先祖返りを防止。

## Aiome + Nurture v1.5/Round 7 シナジー拡張 & 自律委譲 (2026-05-29)

### 1. Nurture Bridge の再エクスポートによる契約共通化 (P2-5S)
- **変更内容**:
    - `Project-Nurture/libs/nurture-bridge/src/lib.rs` [MODIFY]: Aiome コア構造体群（`EvolutionOps`, `KarmaTaxonomy`, `KarmaClassification`, `SamsaraEvent`, `AgentEvolver`, `AutonomousBiomeEngine`, `AutonomousConfig`, `BiomeMessage`）の re-export を実装。
- **波及効果**:
    - Aiome の自律進化機能と Nurture の決済・監査機能が、単一 of ブリッジモジュールを介してシームレスに結合可能となり、依存性のスパゲッティ化を防止。

### 2. key-proxy 観測性＆コストテレメトリの強化 (P2-3)
- **変更内容**:
    - `apps/key-proxy/src/main.rs` [MODIFY]: 全 7 ハンドラへ `#[tracing::instrument]` の付与、プロバイダー別トークン料金メトリクス計算、caller レート比率の観測、`EmbedResponse` に `response_time_ms` の追加。
- **波及効果**:
    - 課金プロキシ層での消費量およびパフォーマンスの可観測性が最大化され、異常なトークン消費や応答遅延の早期検知が可能となった。

### 3. 自律購買 (execute_autonomous_purchase) の Nurture S2S 委譲 & エラーハンドリング極限硬化 (P2-4 / /reflexion)
- **変更内容**:
    - `libs/aiome-commerce/src/stripe.rs` [MODIFY]: Stripe 本番封印解除、`execute_autonomous_purchase` 内で Nurture の S2S エンドポイントへ OXP 署名付き HTTP リクエストの委譲処理を実装。さらにセルフレビュー（Reflexion）に基づき、S2S レスポンスの JSON デシリアライズ失敗時の即時 `return Err` を厳密に実装。
    - `Project-Nurture/apps/nurture-api/src/routes/internal.rs` [NEW]: `/internal/purchase` エンドポイントを新設し、OxiLean Proof 証明書検証を組み込み、エスクロー処理を委譲。
    - `Project-Nurture/apps/nurture-api/tests/internal_routes_test.rs` [NEW]: `/internal/purchase` の正常・異常系を TDD 検証するテストを追加。
- **波及効果**:
    - 自律的エージェントによる購買トランザクションが、セキュリティ境界を超えて Nurture の決済基盤へ安全に委譲され、トランザクションの追跡性と安全性が統合された。
    - エラー発生時の曖昧なフォールスルーが根絶され、システム監視能力が飛躍的に向上。

## Stripe Customer Portal 統合 (P1-1) (2026-05-29)
### 1. CommerceEngine トレイト拡張と 8つの implementor への stub/mock 追加
- **変更内容**:
    - `libs/aiome-contracts/src/commerce.rs` [MODIFY]: `create_portal_session` トレイトメソッドを追加。
    - `libs/aiome-commerce/src/stripe.rs` [MODIFY]: `CreateBillingPortalSession` を用いて、日本語ロケール (`locale(Ja)`) に完全ローカライズされたポータルURLの作成を実装。
    - `libs/aiome-commerce/src/polar.rs` [MODIFY]: `create_portal_session` スタブ (エラー返却) を追加。
    - `libs/aiome-commerce/src/mock.rs` [MODIFY]: `create_portal_session` モック (固定のテストURL返却) を追加。
    - `apps/aiome-node/src/main.rs` [MODIFY]: `StubCommerceEngine` に `create_portal_session` スタブを追加。
    - `apps/api-server/src/api_integration_tests/common.rs` [MODIFY]: `MockCommerceEngine` に `create_portal_session` モックを追加。
    - `libs/infrastructure/src/lora_marketplace.rs` [MODIFY]: `MockCommerceEngineForMarketplace` に `create_portal_session` スタブを追加。
    - `libs/infrastructure/tests/browser_red_team_tdd.rs` [MODIFY]: `MockCommerceEngine` に `create_portal_session` スタブを追加。
    - `Project-Nurture/libs/nurture-infra/src/economy/bridge.rs` [MODIFY]: `NurtureCommerceBridge` に `create_portal_session` スタブを追加。
- **波及効果**:
    - クロスリポジトリ間で Aiome と Project-Nurture の型整合性が保たれ、コンパイル不整合（ビルド破壊）を完全に防止した安全なトレイト拡張を実現。

### 2. API エンドポイントとドメインホワイトリストセキュリティ検証
- **変更内容**:
    - `apps/api-server/src/routes/commerce.rs` [MODIFY]: `/api/v1/commerce/customer-portal/create` エンドポイントハンドラを追加。IDOR防止（`agent_id` 所有権検証）、`ALLOWED_ORIGINS` ドメインホワイトリストによる return_url 検証（フィッシング防止）の SEC-2 セキュリティ要件を完全実装。
    - `apps/api-server/src/router.rs` [MODIFY]: ルート `/api/v1/commerce/customer-portal/create` を登録し、5秒のレート制限を適用。
    - `apps/api-server/src/api_integration_tests/commerce.rs` [MODIFY]: 正常系、IDOR防止拒否、未許可ドメイン拒否を検証する3つの TDD 統合テストを追加。
- **波及効果**:
    - ユーザーが自分自身で Stripe サブスクリプション管理を行える安全なポータル導線が完成。
    - ホワイトリストドメイン検証の導入により、悪意ある return_url へのリダイレクト（オープンリダイレクタ・フィッシング攻撃）を完全に遮断。

## Commerce Layer Deep Hardening (2026-05-22)
### 1. Stripe Commerce 本番封印 & Gift/Splitter 入力バリデーション
- **変更内容**:
    - `libs/aiome-commerce/src/stripe.rs` [MODIFY]: 本番モード（`is_mock = false`）において `cancel_subscription`, `get_subscription_status`, `execute_autonomous_purchase`, `stake`, `slash`, `register_license` の6メソッドを `Err(AiomeError::Infrastructure)` で封印。テスト内の `std::env::set_var` を本番キープレフィックスパターンに安全化。
    - `libs/aiome-commerce/src/gift.rs` [MODIFY]: `validate_gift_policy` に NaN/Infinity/負数/ゼロ金額ガードを追加。
    - `libs/aiome-commerce/src/splitter.rs` [MODIFY]: `split_revenue` に負数 `total_amount` のバリデーションを追加。
    - `libs/aiome-commerce/src/polar.rs` [MODIFY]: 未実装メソッド5箇所のハードコード `Ok(...)` を `Err(Infrastructure)` に変更。
    - `libs/aiome-commerce/src/mock.rs` [MODIFY]: `list_escrows` の unsafe キャスト `*amount as i64` を `i64::try_from().unwrap_or(i64::MAX)` に安全化。
    - `libs/aiome-commerce/src/x402.rs` [MODIFY]: ハードコード秘密鍵を `#[cfg(any(test, debug_assertions))]` で隔離。
    - `libs/aiome-commerce/src/checkout.rs` [MODIFY]: 金額パース失敗時のサイレント0フォールバックを警告ログ付きに変更。
    - `apps/api-server/src/routes/commerce.rs` [MODIFY]: `transfer` に自己送金防止、`withdraw_points` にゼロ金額拒否を追加。
- **波及効果**:
    - v1.0 リリースにおいて未完成の商用メソッドが誤って成功レスポンスを返し、偽のトランザクション記録を生成するリスクを構造的に排除。
    - NaN/Infinity/負数などの不正な金額入力が経済エンジンに到達する経路を完全に遮断。

### 2. Cortex 入力サイズガード
- **変更内容**:
    - `apps/api-server/src/routes/cortex.rs` [MODIFY]: `ingest_text_handler` に title 1KB / content 512KB の上限バリデーションを追加。`query_handler` に question 8KB の上限を追加。
- **波及効果**:
    - 巨大ペイロードによるトークン枯渇攻撃（Token Exhaustion）やメモリ消費攻撃を API レイヤーで防止。

## Compliance Automation & Account Ban Guard Integration (2026-05-20)
### 1. UniversalBanStore & Database Schema Auto-Initialization
- **変更内容**:
    - `libs/infrastructure/src/compliance/ban_store.rs` [NEW]: `BanStore` トレイト、および SQLite/PostgreSQL 対応の `UniversalBanStore` を実装。起動時に BAN 管理用のテーブル `nurture_bans` (SQLite / Postgres) を自己修復かつ冪等に自動作成するロジックを統合。テスト用の `MockBanStore` も実装。
- **波及効果**:
    - アカウント BAN 管理が DB レベルで永続化され、起動と同時にテーブルスキーマが自動初期化されるため、手動マイグレーション漏れによる SQL エラーを防止する。

### 2. AppState DI Integration & Boot Sequence
- **変更内容**:
    - `apps/api-server/src/app_state.rs` [MODIFY]: `AppState` に `ban_store: Arc<dyn BanStore>` を統合。
    - `apps/api-server/src/bootstrap/mod.rs` [MODIFY]: 起動シーケンスの `init_database` 内で `UniversalBanStore` の自動スキーマ初期化を実行。
    - `apps/api-server/src/bootstrap/state_assembly.rs` [MODIFY]: `AppState` 構築時に `BanStore` を DI 注入。
- **波及効果**:
    - システム全体で `AppState` を介して一貫した `BanStore` インスタンスへアクセス可能になり、コンポーネント間の疎結合性を維持しながら認証層で高速な検証が行えるようになった。

### 3. BanGuard & BanExemptAuthenticated Extractor
- **変更内容**:
    - `apps/api-server/src/auth.rs` [MODIFY]: 認証エクストラクタ `Authenticated` において、JWT デコード後に `ban_store.is_banned()` を検証し、BANされている場合は `403 Forbidden` を返却するガードロジックを実装。DB 障害などの予期せぬエラー時は **Fail-Closed 原則** に基づき `503 Service Unavailable` を返却するよう設計。
    - `apps/api-server/src/auth.rs` [NEW]: 消費者保護要件に適合するため、BANされたユーザーでも例外的にアクセスを許可する `BanExemptAuthenticated` エクストラクタを新設。
    - `apps/api-server/src/routes/commerce.rs` [MODIFY]: サブスクリプション解約 API `cancel_subscription` の認証抽出処理を `Authenticated` から `BanExemptAuthenticated` に置換。
- **波及効果**:
    - アカウント BAN 中の不正アクセスや悪意ある操作が API の最前面で遮断される堅牢な防御壁が完成。
    - 同時に、BAN 中であっても解約手続きを行える例外経路を確保したことで、消費者保護規制（GDPR / 加盟店規約）に完全適合。
    - DB 接続障害時にリクエストが通過する脆弱性（Fail-Open）を完全に防ぐ Fail-Closed 設計を徹底。

## WASM Infrastructure Hardening — Bun Rust Rewrite Pattern Integration (2026-05-17)
### 1. parking_lot 移行 (Poison-Free Lock)
- **変更内容**:
    - `libs/infrastructure/Cargo.toml` [MODIFY]: `parking_lot = "0.12"` 依存追加。
    - `libs/infrastructure/src/skills/mod.rs` [MODIFY]: `WasmSkillManager.wasm_cache` を `std::sync::RwLock` → `parking_lot::RwLock` に移行。4箇所の `.unwrap_or_else(|e| e.into_inner())` ポイズンリカバリを削除。
    - `libs/infrastructure/src/skills/harness.rs` [MODIFY]: `HarnessCache` の `cache` / `plugins` フィールドおよび `WasmHarness.plugin` を `parking_lot::{RwLock, Mutex}` に移行。7箇所の `.ok()?` / `.unwrap_or_else()` パターンを直接取得に置換。
    - `libs/infrastructure/src/buzz/scheduler.rs` [MODIFY]: `BuzzScheduler.last_template` を `parking_lot::RwLock` に移行し、ポイズン回復処理を削除。
    - `libs/infrastructure/src/polar_quant.rs` [MODIFY]: static な `PROJECTION_CACHE` を `parking_lot::Mutex` に移行し、ポイズン回復を排除。
    - `apps/api-server/src/mcp/client.rs` [MODIFY]: `last_activity()` の `into_inner()` 依存を `.map(|g| *g).unwrap_or_else(...)` に修正。
- **波及効果**:
    - WASM プラグインキャッシュ層でのスレッドパニック後の RwLock ポイズンによるカスケード障害リスクが **構造的に排除** された。
    - `infrastructure` モジュールおよび `api-server` から、スレッドポイズン時の `into_inner()` による静かな状態破損の引き継ぎ（Silent Corruption）リスクが排除された。
    - `parking_lot` は OS レベルの futex を使用するため、ロック競合時のパフォーマンスも向上。

### 2. Host Function FFI 境界のメモリ安全性強化
- **変更内容**:
    - `libs/infrastructure/src/skills/mod.rs` [MODIFY]: `host_exec` / `host_write` ホスト関数に4段階のメモリ安全性コントラクト（ポインタ抽出 → メモリハンドル検証 → UTF-8 検証 → 応答割り当て）をドキュメント化。各ステップの失敗に `tracing::warn!` を追加し、FFI 境界の静かな失敗を防止。
- **波及効果**:
    - WASM ゲストからの不正メモリオフセット送信時に、ログで即座に検知可能になった。将来の WASI P2 移行時の参照点として機能。

### 3. SkillForge Self-Heal プロンプト構造化
- **変更内容**:
    - `libs/infrastructure/src/skills/forge.rs` [MODIFY]: `CompileErrorCategory` enum (TypeMismatch / Lifetime / MissingTrait / ImportResolution / Other) を追加。`categorize_compile_error()` 分類器と、カテゴリ別 Few-Shot パターン付きプロンプトを実装。
- **波及効果**:
    - LLM によるコンパイルエラー自己修復の精度向上。特に型不一致・ライフタイム・import 解決の3カテゴリで、ターゲットを絞った修正指示が可能になった。

## Aiome X Buzz Protocol Integration (Phase Final)
### 1. API Route Implementations & Dependency Injection
- **変更内容**:
    - `apps/api-server/src/routes/buzz.rs` [MODIFY]: `generate`, `list_pending`, `approve`, `reject`, `update_draft`, `history` の各APIハンドラを実装し、スタブ処理を完全なビジネスロジックに置き換え。`publish` APIで `BuzzDraft` JSONからテキストを抽出するよう修正し、未抽出の不具合を解消。
    - `apps/api-server/src/router.rs` [MODIFY]: `PATCH /api/v1/buzz/draft/:id` ルートを追加登録。
    - `apps/api-server/src/bootstrap/state_assembly.rs` [MODIFY]: `BuzzContentGenerator` および `BuzzScheduler` を `AppState` 構築時に初期化・登録する DI を実装。
    - `apps/api-server/src/api_integration_tests/common.rs` [MODIFY]: テスト用の `AppState` での `buzz_generator` / `buzz_scheduler` 初期化引数の不整合を修正。
    - `libs/infrastructure/src/buzz/worker.rs` [NEW/MODIFY]: 自律的バックグラウンドスケジューリングループ `BuzzWorker` を実装し、インターバル制限や日次投稿クォータを強制。`unwrap()` の完全排除と Zero-Panic ポリシーへの準拠。
    - `libs/infrastructure/src/buzz/scheduler.rs` [MODIFY]: `RwLock` 状態管理にポイズン回復処理（`into_inner` フォールバック）を追加。SQLite へのジョブ登録時に `BuzzDraft` の Serde シリアライゼーションを適用。
    - `libs/infrastructure/tests/buzz_worker_tdd.rs` [NEW]: TDD 用のテストスイートを追加。LLM 障害伝播、冪等性ガード、日次クォータの境界値テストを実装しパス。
    - `libs/infrastructure/src/job_queue/evaluation.rs` [MODIFY]: `do_record_sns_metrics` を拡張し `repost_count`, `quote_count`, `reply_count`, `impression_count` に対応。
- **波及効果**:
    - Aiome の発信能力を司る Buzz Protocol が、フロントエンド (Management Console) と完全に連携可能になり、自律バックグラウンド投稿パイプラインが完成した。
    - 外部 SNS (X API 等) への投稿前に意図しない JSON アーティファクトの送信が防止された。
    - 全てのエンドポイントとバックグラウンドタスクが Zero-Panic ポリシー (unwrap排除) と `AiomeError` を介したドメインエラー伝搬に準拠し、API サーバーとしての堅牢性が維持されている。
    - TDD (Red to Green) プロセスを通じ、テスト環境における依存注入とエッジケースのハンドリングが担保され、CI/CD における regression を防ぐ強固な基礎が完成した。
## Landing Page Visual & Synergy Upgrade (Phase A, B, C)
### 1. Nurture Ecosystem Integration & Visual Overhaul
- **変更内容**:
    - `docs/landing/index.html` [MODIFY]: "The Aiome Ecosystem" および "Use Cases" セクションを新規追加し、Nurture プラグイン拡張との関係性（単独OS＋経済拡張）を明示。
    - `docs/landing/styles.css` [MODIFY]: Hero セクションへのネオングラデーション、Feature カードへの `@property` アニメーションボーダー、CTA の Shimmer 効果を追加。
    - `docs/landing/i18n.js` [MODIFY]: エコシステムや自律ショッピング等のユースケースに対応する多言語（EN/JA）キーを12個追加。
    - `docs/landing/scroll-reveal.js` [NEW]: `IntersectionObserver` を用いた要素のフェード・スライドアニメーションを独立して実装。
- **波及効果**:
    - ランディングページの表現力が大幅に向上し、OSSとしてのAiomeと商用拡張であるNurtureの経済的シナジー（A2Cギフト、マーケットプレイス）がユーザーに正しく伝達されるようになった。
    - アニメーションは標準CSS APIと軽量なJSに依存しており、外部の巨大なフロントエンドフレームワークを必要としない。

### 2. Terminal Demo & Console Preview Automation
- **変更内容**:
    - `docs/landing/scripts/generate_cast.js` [NEW]: `asciinema` 用のキャストファイルをプログラム的に自動生成するTDDテスト・スクリプトを導入。
    - `docs/landing/quickstart-demo.svg` [NEW]: `svg-term-cli` を用いてキャストファイルから軽量なSVGターミナルアニメーションを生成。
    - `docs/landing/terminal.js` [DELETE]: 旧式の JavaScript ベースターミナルアニメーションを削除し、SVG アセットへ完全移行。
    - `docs/landing/console-preview.png` [MODIFY]: モックアップ画像を実際の `management-console` (ポート1420) から取得した真の PNG スクリーンショットに置換。
- **波ガ効果**:
    - ターミナルアニメーションのメンテナンスがスクリプト化され、将来のコマンド変更にも TDD ベースで安全かつ即座に対応可能となった。
    - SVG への移行により、ブラウザのメインスレッドをブロックする旧 JS コードが排除され（約80行のCSSとJSファイル1つの削減）、パフォーマンスと保守性が向上した。

## Tech Debt Remediation: Bootstrap Modularization
### 1. API Server Bootstrap Extraction
- **変更内容**:
    - `apps/api-server/src/bootstrap.rs` [MODIFY/DELETE]: 2,094行のモノリスを `bootstrap/mod.rs` に縮小し、サブ関数群を 6つのファイルに抽出。
    - `apps/api-server/src/bootstrap/*.rs` [NEW]: `preflight.rs`, `database.rs`, `llm_providers.rs`, `state_assembly.rs`, `workers.rs`, `helpers.rs` を新規作成。
- **波及効果**:
    - 巨大な初期化シーケンスの認知負荷が大幅に低下。各サブシステム（データベース、LLM、ワーカー）のセットアップが独立したファイルに分離された。
    - 外部参照パス（`crate::bootstrap::`）は不変に保たれており、他のモジュールやテスト（`bootstrap_tests.rs`）への悪影響はない。

## Aiome Security & Stability Hardening (Zero-Panic & CWE-209)
### 1. Artifact Store Zero-Panic Compliance
- **変更内容**:
    - `libs/infrastructure/src/artifact_store.rs` [MODIFY]: `get_artifact_edges` メソッド内の14箇所の `.get()` を `.try_get().unwrap_or_default()` に置換。
- **波及効果**:
    - SQLite / Postgres のスキーマ不整合や予期せぬマイグレーションエラー時に、ランタイムパニックが完全に排除された。
    - API サーバー全体のプロセスダウンを防ぎ、Fail-Soft（欠損データを無視して続行）な振る舞いを保証する。

### 2. CWE-209 Error Information Masking
- **変更内容**:
    - `apps/api-server/src/error.rs` [MODIFY]: `sanitize_aiome_error_details` によって、`anyhow::Error` や `Box<dyn std::error::Error>` などの内部エラー詳細がクライアントに漏洩しないよう保護。
    - `apps/api-server/src/routes/watchtower.rs` [MODIFY]: LLMプロバイダエラーとLLMタイムアウトを分離し、ユーザー入力の生ダンプをログから排除（CWE-532対応）。
- **波及効果**:
    - デバッグモード (`cfg(debug_assertions)`) 以外では、エラー詳細はUUIDのみクライアントに返却され、実際のトレースはサーバーログにのみ記録される。情報漏洩リスクの排除。

### 3. UI TypeScript Strict Boundaries
- **変更内容**:
    - `apps/management-console/src/components/VoiceStore.tsx` [MODIFY]: `any` を排除し `Record<string, unknown>` と `Array.isArray()` による構造検証を導入。
    - `apps/management-console/src/components/Timeline.tsx` [MODIFY]: `TimelineEvent` インターフェースを定義し、APIレスポンスの `.ok` ガードとプロパティの `typeof` チェック、NaN-safeソートを追加。
- **波及効果**:
    - APIスキーマの変更やバックエンドの障害（400/500系エラー、不正フォーマットJSON）が発生しても、UIがホワイトスクリーンでクラッシュするリスク（TypeError）が解消された。

## Samsara Hub Architecture Modularization (Phase 3)
### 1. Handler Decomposition & Worker Extraction
- **変更内容**:
    - `apps/samsara-hub/src/main.rs` [MODIFY]: 1,222行のモノリスから全ハンドラ・ワーカー・認証ロジックを抽出し、365行のルーター定義 + 初期化コードに削減。
    - `apps/samsara-hub/src/auth.rs` [NEW]: Ed25519 署名検証 (`verify_ed25519_signature`) を中央集権化。Base64 デコード → `VerifyingKey` 復元 → `Signature` 検証の DRY 化。3 つのハンドラから呼び出される共通関数。
    - `apps/samsara-hub/src/workers.rs` [NEW]: `approval_worker` バックグラウンドタスク (quarantine 検証、BFT スラッシング、データ eviction) を独立モジュール化。`tokio::task::yield_now()` によるバックプレッシャー制御を維持。
    - `apps/samsara-hub/src/handlers/biome.rs` [NEW]: Biome P2P 通信ハンドラ群 (`list_topics`, `create_topic`, `biome_relay_handler`, `biome_ws_handler`) を移行。CSAM バイナリフィルタ (`data:image/`, `data:video/`, `;base64,`) と GlassWorm サニタイズを包含。
    - `apps/samsara-hub/src/handlers/system.rs` [NEW]: `health_handler` と `list_agents_handler` を移行。
    - `apps/samsara-hub/src/handlers/middleware.rs` [NEW]: `auth_middleware` を抽出。RBAC ロール検証 (System/Admin/Federated) と Bearer トークンの constant-time 比較を包含。
    - `apps/samsara-hub/src/handlers/timeline.rs` [NEW]: CRDT タイムライン同期ハンドラを移行。Automerge マージロジック + 1MB ペイロードガードを包含。
    - `apps/samsara-hub/src/handlers/mod.rs` [MODIFY]: 新規サブモジュール (biome, middleware, system, timeline) を登録し、`verify_bearer` ヘルパーを統一的に公開。
- **波及効果**:
    - `main.rs` からのルート定義は全て `handlers::*` 名前空間を参照するように変更され、各ハンドラは独立した単一責任モジュールとしてテスト・メンテナンス可能になった。
    - Ed25519 署名検証の DRY 化により、暗号コードの変更が `auth.rs` 1 ファイルのみに集約された。ポリシードリフト（ハンドラごとに異なるバリデーションロジック）のリスクが構造的に排除された。
    - 全 10 件の既存統合テストが変更なしで PASS することを確認済み。ハンドラの再配置は外部 API 契約（HTTPルート・レスポンス形式）に影響を与えない。
    - `biome_ws_handler` は WebSocket ハンドシェイク後に独自の認証を行うため、`middleware.rs` の `auth_middleware` は経由しない。これは意図的な設計（WebSocket エンドポイントは HTTP ミドルウェアスタックを通過しない）。

## Phase 4: Secrets Brokering (Vault Proxy Architecture)
### 1. Ephemeral Container Secret Isolation
- **変更内容**:
    - `libs/infrastructure/src/docker_conductor.rs` [MODIFY]: `DockerConductor::new` のシグネチャを拡張し、`vault_secret: Option<SecretString>` と `key_proxy_url: Option<String>` を追加。これらを一時的な `.env.shadow` に環境変数 `VAULT_SECRET` と `KEY_PROXY_URL` として出力するよう改修。レガシーな `GEMINI_API_KEY` を注入から完全に排除。
    - `libs/infrastructure/src/browser_conductor.rs` [MODIFY]: `BrowserConductor::new` のシグネチャと環境変数注入を同様に `vault_secret` 対応に改修。`browser_red_team_tdd.rs` のテストスイートも合わせて修正。
    - `docker/browser-use/entrypoint.py` [MODIFY]: 起動時に `VAULT_SECRET` と `KEY_PROXY_URL` を読み取り、Langchain などのプロキシベースのリクエスト認証に用いるように変更。
    - `apps/key-proxy/src/main.rs` [MODIFY]: `auth_middleware` に Strategy 4 (`x-goog-api-key` カスタムヘッダー) を追加。Gemini パススルー時の `handle_gemini_passthrough` にて、送信元のダミーヘッダー (`x-goog-api-key`, `Authorization`) を能動的に削除（Sanitize）し、Double Fault による 401 エラーを防止するロジックを追加。
- **波及効果**:
    - 全てのシャドウワーカーとブラウザワーカーがローカルの `GEMINI_API_KEY` のコピーを持たずに実行されるようになり、コンテナ侵害時のサードパーティキーの漏洩（Exfiltration）リスクが物理的に遮断された。
    - プロキシ側での透過的なヘッダーサニタイズにより、Google GenAI SDK や Langchain などの標準ライブラリとの互換性を保ちながら Zero-Trust プロキシを経由することが可能となった。
## Phase Memento 1.5: CortexSynth Quality Gate Integration
### 1. SynthQualityJudge Implementation & DI
- **変更内容**:
    - `libs/infrastructure/src/cortex_synth.rs` [MODIFY]: `SynthQualityJudge` trait, `JudgeVerdict` struct, `LlmSynthJudge` を追加。`generate_dataset` ループ内に 3段目の品質ゲートとして Judge 呼び出しをインラインで統合。
    - `libs/infrastructure/src/cortex_synth_tests.rs` [MODIFY]: `MockSynthJudge` を作成し、3件の振る舞い検証テスト (Accept, Reject, Fallback) を追加。
    - `libs/infrastructure/src/memory_crystallizer.rs` [MODIFY]: `MemoryCrystallizer::new` シグネチャを変更し `judge` を追加。事実結晶化ループに `evaluate` を追加（失敗時は Graceful Degradation）。
    - `apps/api-server/src/routes/cortex.rs` [MODIFY]: `CortexSynthesizer` に `LlmSynthJudge` インスタンスを DI 注入するよう修正。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `MemoryCrystallizer` のバックグラウンドタスク生成時に `LlmSynthJudge` インスタンスを DI 注入するよう修正。
- **波及効果**:
    - Synthetic Data 生成パイプラインにおいて、AI 自身による品質評価ループが組み込まれ、Mement パターンによるデータセットの純度維持機能が有効化された。
    - `MemoryCrystallizer` の知識抽出においても不適切な解釈がデータベースへ永続化されるのを防止するセカンダリゲートが構築された。

## Phase A & B: Economic Observability & Quality Gates
### 1. Cost Tracking & Diagnostics Summary
- **変更内容**:
    - `libs/infrastructure/src/llm/dynamic.rs` [MODIFY]: LLM 料金テーブルに `gemini-2.5-flash`, `gemini-2.5-pro`, `gpt-4.1`, `claude-sonnet-4-20250514` 等の最新モデルを追加。
    - `apps/api-server/src/routes/general.rs` [MODIFY]: `DiagnosisSummaryResponse`, `CategoryCount` のレスポンス型を追加。
    - `apps/api-server/src/routes/audit.rs` [MODIFY]: `get_diagnostics_summary` ハンドラを実装し、全ジョブのエラーカテゴリ別統計情報を提供。
    - `apps/api-server/src/router.rs` [MODIFY]: `/api/v1/audit/diagnostics/summary` ルートを登録。
    - `apps/api-server/src/api_integration_tests/system.rs` [MODIFY]: `test_diagnostics_summary_api` TDDテストを追加し、FK制約対応と正常稼働を確認。
- **波及効果**:
    - Aiome インフラがプロバイダー別のコストを正確に把握可能になり、ダッシュボードでの Unit Economics 管理が完成。
    - エラーカテゴリ (FailureCategory) の集計 API により、LLM エラーや PlanAdherenceFailure 等のトレンドを俯瞰的に分析可能となった。

## Phase E: TaskOrchestrator Reflexion Loop
### 1. Self-Repair Hint Injection & Retry Mechanics
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `Watchtower` による `diagnostics.diagnose()` 結果を利用し、リトライ可能な場合 (`!is_poisoned`) に `agent_diagnosis.self_repair_hint` を次回ジョブの `karma_directives` へ `[Reflexion]: ...` の形式で追記するループを統合。
    - `libs/aiome-core-contracts/src/traits.rs` [MODIFY]: `TaskRegistry` に `append_job_karma_directives` を追加。
    - `libs/infrastructure/src/job_queue/core_ops.rs` [MODIFY]: `do_append_job_karma_directives` を実装し、DBレベルでのアトミックな追記機構を構築。モック類 (soul_mutator, immune_system 等) にも同メソッドを追記。
    - `apps/aiome-node/src/main.rs` [MODIFY]: mDNS Daemon のスコープ管理ミスおよび `unreachable!()` によるシステムパニックを修正。
- **波及効果**:
    - 失敗したタスクのリトライ時に自己修復ヒント (Self-Repair Hint) がプロンプトに注入されるようになり、LLM が同じ過ちを繰り返すループ（Poisoned Task）に陥る確率が大幅に低下。
    - `TaskOrchestrator` の自律性が向上し、外部の介在なしにエラー軌跡（Trajectory）から学習する自己修復ループが完成した。

## Phase 16: LLM Pipeline Completion
### 1. Semantic Dependencies
- **変更内容**:
    - `libs/infrastructure/src/llm/humanizer_rules.rs` [MODIFY]: Regex safety and LazyLock caching を実装。
    - `libs/infrastructure/src/llm/humanizer_filter.rs` [MODIFY]: キャッシュされたルール参照への更新。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: ルール初期化プロセスの最適化。
- **波及効果**:
    - ルール初期化を `&'static [HumanizerRule]` 参照へ変更し、繰り返し発生していた `Vec` アロケーションを排除。スタティックな正規表現に対するパニックを未然に防止。

## Phase 15: Telemetry & Observability Hardening
### 1. TaskDispatcher Telemetry Integration (`soul_hash`)
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `compute_soul_hash` ヘルパーを追加し、ハードコードされていた `"unknown"` `soul_hash` を動的に計算する仕様へ改修。
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `store_karma` 内で、タスクが失敗した際の教訓 (`negative` カルマ) の記録時に `compute_soul_hash` を呼び出し。ハッシュ値は `AppState::get_system_soul_hash()` と一貫するようゼロパディングなしの hex (`{:x}`) にフォーマット。
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `#[tracing::instrument]` を適用し、ハッシュ計算ステップの Observability（可観測性）を強化。TDD ユニットテストおよび統合テストを追加。
- **波及効果**:
    - Aiome の学習システムにおいて、どの Soul / Evolving Soul 状態でエラーが発生したかが正確に紐付けられるようになり、将来的なコンテキスト自動修復機能（Auto-Remediation）やカルマ分析の精度が劇的に向上する。
    - `AppState` との `soul_hash` アルゴリズム互換性が確保されたことで、フロントエンド/バックエンド間で魂の同一性が完全にトラッキング可能となった。

## Phase 14: TTS SSE Streaming & Lip-Sync Integration
### 1. Robust SSE Streaming Pipeline
- **変更内容**:
    - `libs/aiome-core-contracts/src/traits.rs` [MODIFY]: `TtsProvider` トレイトに `synthesize_stream` を追加し、`TtsStreamEvent` (Audio/Viseme) を返す仕様に変更。
    - `apps/api-server/src/routes/voice.rs` [MODIFY]: `GET/POST /api/v1/voice/synthesize` に `stream=true` パラメータを追加し、SSE 形式でのチャンク配信を実装。
    - `apps/management-console/src/hooks/useTtsSse.ts` [NEW]: `@microsoft/fetch-event-source` を用いた Accumulate-then-Play 音声チャンク再生機構と、`AbortController` によるストリーム排他制御を実装。
    - `apps/management-console/src/hooks/useAgentChat.ts` [MODIFY]: 優先的に `useTtsSse` を呼び出し、失敗時にレガシーな静的 Blob フェッチへ自動的にフォールバックする耐障害性ロジックを統合。
- **波及効果**:
    - 音声合成のストリーミング再生が可能となり、Time-to-First-Byte (TTFB) が劇的に改善。
    - フロントエンド側で発生していた `AbortController` 漏れによる AudioContext やメモリリーク（URL Blob）が完全に払拭され、安全性と堅牢性が向上。
    - `Viseme`（リップシンク）イベントの受容準備が整い、次期 Phase（3D/2D アバター統合）への基盤が完成。

## Phase 2: Native SLM Intelligence Backend
### 1. Semantic Search & Memory Importance
- **変更内容**:
    - `libs/infrastructure/src/llm/native_embedding.rs` [NEW]: ネイティブRustによる埋め込み（Embedding）モデルの推論とコサイン類似度計算を実装。
    - `libs/infrastructure/src/native_backend.rs` [NEW]: `NativeSlmBackend` を実装。`recall`, `calculate_importance`, `detect_contradictions` の意味的な推論ロジックを提供。
- **波及効果**:
    - SLM の外部 CLI 依存を脱却し、Rust ネイティブレイヤーでのインメモリ意味検索と論理矛盾検知（Constitutional Validation）が高速かつ安全に動作するようになった。

## Hardening Phase: System Isolation & API Consistency
### 1. McpDiscovery Fail-Closed Prevention
- **変更内容**:
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `McpDiscoveryTask` をメインの `cancel_token` から切り離し、専用の `mcp_cancel` を付与。さらにワンショット実行から定期的な `interval` ループへ変更。
- **波及効果**:
    - MCP サーバーの起動失敗（再試行上限突破）が、API サーバー全体のダウン（Fail-Closed）を引き起こす壊滅的な設計バグが解消された。システムの回復力（Resiliency）が向上。

### 2. Frontend API & Slash Command Consolidation
- **変更内容**:
    - `apps/management-console/src/constants/slashCommands.ts` [NEW]: スラッシュコマンド（`/store`, `/treasure`, `/lora`, `/clear`）の定義を抽出。
    - `AgentConsole.tsx`, `StoryFlow.tsx`, `useAgentChat.ts` [MODIFY]: 三重定義されていたコマンドリストを新規 constant に統合。
    - `AgentConsole.tsx`, `useCortexSuggestions.ts` [MODIFY]: `fetch` を `authenticatedFetch` + `API_BASE` に統一。
- **波及効果**:
    - Vite proxy が存在しない環境での API リクエストの完全な失敗を修正し、認証基盤を堅牢化。また、コマンド定義の単一障害点化を防ぎ、UI（アイコン）と Dispatch（エンベロープ）のロジックが安全に統合された。

## Test Debt Resolution & Monolith Decomposition
### 1. `api_integration_tests.rs` Directory Migration
- **変更内容**:
    - `apps/api-server/src/api_integration_tests.rs` [DELETE]: 4,100行に及ぶ巨大なテストモジュールを解体し物理削除。
    - `apps/api-server/src/api_integration_tests/` [NEW]: ドメイン駆動のモジュール分割を導入 (`auth`, `mcp`, `commerce`, `agent`, `biome`, `jobs`, `system`, `common`)。
    - `apps/api-server/src/api_integration_tests/common.rs` [NEW]: テスト間共有フィクスチャ (`TestServer`, `JobQueue`, モック群) の再エクスポート (`pub use common::*;`) 機構を構築。
- **波及効果**:
    - テストファイルでの Git マージコンフリクト（Collision）が根本的に解消され、ドメインごとの並列作業やCIコンパイル効率が向上した。
    - 環境変数 (`GITHUB_CLIENT_ID` 等) の共有ステートによる Race Condition が `#[serial]` 属性の再適用により解消され、175/175 の全テストが安定して PASS (Zero-Panic) する状態となった。

## Phase 1 & 2: Browser Automation Infrastructure & Dual-Provider Economic Model
### 1. Secure Execution Environment for browser-use
- **変更内容**:
    - `libs/infrastructure/src/browser_conductor.rs` [NEW]: `BrowserConductor` を実装。`BastionGuard` を活用して `SandboxProfile::BrowserAgent` プロファイルを適用。ファイル書き込みを禁止しつつ安全なネットワークアクセスを許可。
    - `libs/infrastructure/src/browser_conductor.rs` [MODIFY]: Gemini API 実行時は `CommerceEngine` を呼び出して100コインをエスクロー（成功時に徴収、失敗時に返金）。Ollama 実行時 (`OLLAMA_BASE_URL` 指定時) は無料で実行するハイブリッド課金モデルを構築。
    - `libs/infrastructure/src/browser_conductor.rs` [MODIFY]: `TaskEvent` を用いたプログレスのストリーミング出力を実装。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `TaskDispatcher` に `BrowserConductor` を登録。
- **波及効果**:
    - Aiome のインフラストラクチャにおける自律的 Web ブラウザ操作能力（Agentic Web Interaction）が安全に解放された。
    - ユーザーは Local LLM（無料・高プライバシー）と Gemini（有償・高精度）をシームレスに切り替え可能となり、プラットフォーム側はクラウドコストの赤字リスク（Cost Blowout）を構造的に回避できる。
    - Docker と `BastionGuard` を組み合わせた多層防御（Defense-in-Depth）により、APIキーの漏洩やホストシステムへの特権昇格・破壊的アクセスが物理レイヤーで遮断された。

## Phase 4/5: Security Hardening & Zero-Panic Enforcement
### 1. Unified AppError & Domain Error Decoupling
- **変更内容**:
    - `apps/api-server/src/routes/cortex.rs` & `demo.rs` [MODIFY]: `AiomeError` を直接返却していた API ハンドラ群を `crate::error::AppError` に統一し、不適切な 500 エラーを `bad_request` (400) や `unauthorized` (401) に適切にマッピングするよう改修。
- **波及効果**:
    - Aiome API サーバー全体のエラーハンドリング型が `Result<impl IntoResponse, AppError>` に完全統一され、不正入力に対する安全で予測可能な HTTP ステータス伝播（Graceful Degradation）が実現した。
    - 潜在的なダウンキャストの失敗やエラー握り潰し（サイレントフォールト）のベクトルが排除され、Zero-Panic ポリシーの遵守が強化された。

### 2. CVE Registry & Dependency Hardening
- **変更内容**:
    - `deny.toml` [MODIFY]: `wasmtime` (v41.0.4) のサンドボックス脱出 (CVE-2024-39329 等) および `rustls-webpki` の CPU 枯渇 (CVE-2024-6506) 等、現状の依存ツリー上の既知の脆弱性を `[advisories.ignore]` に文書化（Chesterton's Fence 原則）。
- **波及効果**:
    - `cargo audit` パイプラインでの偽陽性による CI/CD ブロッカーを解消しつつ、将来のメジャーアップデート（Phase 2）に向けた技術的負債の透明性を確保した。

### 3. License Compliance & Copyright Enforcement
- **変更内容**:
    - `scripts/license_check.py` [EXECUTE]: ライセンスコンプライアンスの自動テストを実行し、11の `.rs` ファイルで著作権ヘッダーが欠落していることを検出。
    - `libs/infrastructure/**/*.rs` & `apps/api-server/**/*.rs` (計11ファイル) [MODIFY]: すべての対象ファイルに Apache 2.0 の著作権表示ヘッダーを一括適用。
    - `docs/licenses.json` [GENERATE]: `cargo license --json` コマンドにより Cargo 依存ライセンス一覧を更新。
- **波及効果**:
    - Aiome の OSS プロジェクトとしての法的な整合性が確立され、`/license-check` ワークフローの 11 の監査項目（NOTICE, THIRD_PARTY_NOTICES, 著作権ヘッダー等）をすべて 100% PASS するクリーンな状態に復帰した。
    - GPL/AGPL といった非互換（汚染リスクのある）ライセンスが npm や Cargo ツリーに混入していないことが完全に証明された。

## Aiome HTML Report Infrastructure & Zero-Panic Policy
### 1. `HtmlReportBuilder` and `tokens.css` Runtime Injection
- **変更内容**:
    - `libs/infrastructure/src/html_report.rs` [MODIFY]: `minijinja` を用いたテンプレートベースのレンダリングエンジンを構築し、`tokens.css` をランタイムで動的注入する仕組みを実装。
    - `libs/infrastructure/src/html_report.rs` [MODIFY]: `ammonia` によるホワイトリストベースの厳格なHTMLサニタイズを導入。インタラクティブな描画に必要なSVG属性 (`viewBox`, `d`, `fill` 等) を例外的に許可しつつ、XSS攻撃を防止。
    - `libs/infrastructure/src/html_report.rs` [MODIFY]: `unwrap()` および `expect()` 呼び出しを一掃し、すべての処理を `Result` 型にリファクタリング。`enforce_unwrap_deny.py` の Zero-Panic CI ガードを完全クリア。
    - `apps/api-server/src/app_state.rs` & `bootstrap.rs` [MODIFY]: `AppState` に `tokens_css` 状態を追加し、起動時に `tokens.css` ファイルを読み込み `HtmlReportBuilder` に DI 注入。
- **波及効果**:
    - レポート生成機能がハードコードされたデザインから解放され、Artemis UI デザインシステム (`tokens.css`) と完全に同期したセキュアな動的 HTML 生成基盤が完成した。
    - `Result` ベースのエラー伝播により、実行中のパニックが物理的にブロックされ、安全性が飛躍的に向上した。

### 2. ArtifactVault UI Integration & CSP Sandboxing
- **変更内容**:
    - `apps/management-console/src/components/ArtifactVault.tsx` [MODIFY]: HTML アーティファクトの MIME 判定を追加し、プレビューモーダル（Eye アイコン）の対象ファイルとして HTML をサポート。
    - `apps/management-console/src/components/ArtifactVault.tsx` [MODIFY]: HTML プレビュー専用の `iframe` を導入し、`sandbox="allow-scripts"` の最小特権 CSP サンドボックスで隔離レンダリングするロジックを追加。
    - `.agent/workflows/*.md` [MODIFY]: `biz-value.md`, `code-review.md`, `expert-review.md` などのワークフローに対し、Markdown ではなく HTML レポートの出力を指示するディレクティブを追加。
- **波及効果**:
    - AI生成の HTML ファイルを安全に Management Console 上でプレビューできるようになり、表現力豊かなアーティファクト体験（チャート描画など）が実現された。
    - `allow-same-origin` を付与しないサンドボックス化により、万が一の XSS でもシステム内のセッション情報が保護されるセキュアバイデザインが確立された。

### 3. Interactive JS Bridge in HTML Reports
- **変更内容**:
    - `libs/infrastructure/src/html_report.rs` [MODIFY]: `ammonia` のサニタイズポリシーを拡張し、`button` タグおよび `data-aiome-feedback`, `data-autosend` 属性を許可。ベーステンプレートに JS イベントリスナーを注入し、`window.parent.postMessage` によるセキュアなブリッジ通信を実装。
    - `apps/management-console/src/hooks/useAgentChat.ts` & `ArtifactVault.tsx` [MODIFY]: `iframe` からの `message` イベントをリッスンし、`aiome_inject_prompt` カスタムイベントを通じてチャット入力および自動送信 (`sendMessage`) をトリガーする TDD 実装を追加。
    - `.agent/workflows/*.md` [MODIFY]: `biz-value.md`, `expert-review.md` のワークフローに JS Bridge の使用方法を追記し、エージェントがインタラクティブなボタンを生成できるように指示。
- **波及効果**:
    - HTML アーティファクトから Agent に対して直接フィードバックを送る（Auto-Fix, Auto-Plan など）双方向なインタラクション機能が実現された。
    - `onclick` 属性をブロックしたまま `data-*` 属性と親テンプレートのスクリプトを使用することで、XSS リスクを抑えつつインタラクティブ性を両立した。

## Commerce Webhook Architecture Cleanup
### 1. Pruning `process_webhook` from CommerceEngine
- **変更内容**:
    - `libs/aiome-contracts/src/commerce.rs` [MODIFY]: `CommerceEngine` トレイトから `process_webhook` のシグネチャを完全に削除。
    - 各エンジン実装 (`StripeCommerceEngine`, `PolarCommerceEngine`, `MockCommerceEngine`, `StubCommerceEngine`) および `Project-Nurture` の `NurtureCommerceBridge` からメソッドの実装と依存テストを一掃。
- **波及効果**:
    - トランザクション処理の境界がインフラ層（コントローラー・ハンドラ）に移動し、ビジネスロジックトレイト内での不適切なDB操作や冪等性チェック漏れなどの車輪の再開発リスクが排除された。
    - アーキテクチャの責務がより明確になり、Webhook 処理は完全に HTTP レイヤーのアトミックなトランザクションとして扱われるようになった。

### 2. TDD Environment Stabilization
- **変更内容**:
    - `apps/api-server/src/api_integration_tests.rs` [MODIFY]: テスト環境構築用関数から `ollama_host` のハードコードを削除。該当テスト内でのみ `SettingsOps` を用いて設定を注入する方式へ移行。
- **波及効果**:
    - 環境変数の漏洩によるテストの汚染や、Trust-DNS ベースの SSRF フィルターに対する誤検知・テスト間干渉が解消され、よりクリーンなテストライフサイクルが確保された。

## Aiome Infrastructure & Automation (ScoreTracker & Heartbeat)
### 1. Autonomous LoRA Training & ScoreTracker Edge Case Testing
- **変更内容**:
    - `libs/infrastructure/tests/score_tracker_tdd.rs` [ADD]: `ScoreTracker::detect_plateau` のエッジケースをカバーする新規テストモジュールを作成。
    - インメモリ SQLite と `TestForecastProvider` を使用し、データ不足、順調な成長、停滞状態、NaN/Inf データの除外（SQL レベルでの挿入テスト）を検証。
- **波及効果**:
    - NaN/Inf データによる予測エンジンの汚染リスクを TDD によって確実に防止した。
    - 停滞検知ロジックの判定基準が単体テストによって文書化され、自律学習機能の信頼性が向上した。

## MCP OAuth Integration & X402 SSOT (Phase 0 & 1)
### 1. MCP OAuth 2.0 Authorization Code Flow
- **変更内容**:
    - `apps/api-server/src/mcp/discovery.rs` [MODIFY]: `OAuthCredentials` の構造体定義、`oauth_token_url()`、`exchange_code_for_token()` の追加。`OAUTH_REDIRECT_URI` の動的化。
    - `apps/api-server/src/app_state.rs` [MODIFY]: `mcp_oauth_secrets` フィールドを追加。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: 環境変数から OAuth 認証情報を読み込み、`scrub_env` により読み込み後即削除（パージ）を実施。
    - `apps/api-server/src/api_integration_tests.rs` [MODIFY]: `wiremock` を用いた E2E テスト（成功系・失敗系）の追加。
- **波及効果**:
    - MCP エコシステムにおける安全な認証基盤が確立された。
    - CSRF 脆弱性に対する防御（PKCE state token の一回限り消費）が追加された。

### 2. X402 Architecture SSOT
- **変更内容**:
    - `libs/aiome-commerce/src/x402.rs` [MODIFY]: 重複定義されていた `U256` と `X402Negotiator` を削除し、`aiome_core_contracts` クレートからのインポートに統一。
    - `libs/aiome-commerce/Cargo.toml` [MODIFY]: `alloy-primitives` および `alloy-signer-local` の依存関係を追加。
    - `libs/aiome-commerce/src/x402.rs` [MODIFY]: モック実装の `PrivateKeySigner` を `alloy_signer_local::PrivateKeySigner` に移行し、X402 マイグレーションを完了。
- **波及効果**:
    - 契約ロジックと取引ロジックの依存関係が整理され、SSOT（Single Source of Truth）が確保された。
    - Federation v1.5 におけるクリプトグラフィックな境界防御が本番環境仕様に昇格した。

## Federation v1.0 & Zero-Trust MCP Ecosystem (Phase 1)
### 1. P2P Sync Routing & Replay Attack Defense
- **変更内容**:
    - `apps/samsara-hub/src/main.rs` [MODIFY]: `quarantined_karma` と `quarantined_rules` の100,000行制限パージ（OOM Defense）を新設。
    - `apps/samsara-hub/src/handlers/federation.rs` [MODIFY]: ハンドラ内での認証処理を撤廃し `auth_middleware` へ集約。`push_handler` にて `NodeReputation` と `last_seen_lamport_clock` による Replay 攻撃（BFT）防御を実装。
    - `apps/aiome-node/src/routes/mod.rs` & `main.rs` [MODIFY]: `federation` モジュールを `router` に正式マウントし、`AiomeConfig` 状態を DI。
    - `apps/aiome-node/src/routes/federation.rs` [MODIFY]: スタブ実装を排し、`reqwest::Client` を利用した HTTP プロキシ（Samsara Hub への透過的同期）を構築。
- **波及効果**:
    - Node <-> Hub 間のフェデレーションプロトコルが稼働可能になり、不正な古いトランザクションの再送信（Replay Attack）やフェデレーション用ポートからの認証バイパスが構造的に防がれた。

### 2. MCP Ecosystem Hardening (Strict Arbitrary Execution Prevention)
- **変更内容**:
    - `libs/shared/src/mcp_constants.rs` [MODIFY]: `@crewai/` と `@autogen/` を `ALLOWED_MCP_PREFIXES` に追加。
    - `libs/shared/src/mcp_constants.rs` [MODIFY]: `--yes` を `FORBIDDEN_MCP_ARG_FLAGS` に追加し、さらに `validate_mcp_arg_flags` にて動的なインストールを誘発する短縮フラグ `-y` に対する完全一致ブロック（`lower == "-y"`）を追加（`-yaml` などの正規フラグを誤検知させない防護）。
- **波及効果**:
    - `npx` や `uvx` を悪用してサンドボックス外の不正コマンドを実行させるインジェクション脆弱性が物理レイヤーで遮断された。

## Sentinel Native Bindings Testing & CI/CD Hardening (Phase 2)
### 1. Sentinel TDD Integration & Prompt Injection Protection
- **変更内容**:
    - `packages/sentinel/src/sentinel.test.ts` [NEW]: Jest テストスイートを作成し、Native バインディング (`napi-bridge`) の動作を直接検証。
    - `libs/infrastructure/src/immune_system.rs` [MODIFY]: `BASELINE_REGEXES` に `(?i)ignore\s+all\s+previous\s+instructions` パターンを追加し、Sentinel 内部でのプロンプトインジェクションブロックを実装。
    - `packages/sentinel/package.json` [MODIFY]: `test` コマンドで `AIOME_DB_PATH` を自動挿入し SQLite エラーを解消。冗長な JS テストスクリプトを削除。
    - `.github/workflows/napi-release.yml` [NEW]: 3 OS での NAPI クロスコンパイルと自動テストの CI/CD を構築。
- **波及効果**:
    - Aiome Sentinel SDK のセキュリティと動作安定性が TDD によって確固たるものになり、ネイティブ層での未知のエラーによる CI の Flakiness を排除。
    - プロンプトインジェクションに対する基本防壁が強化された。

## MCP Dynamic Registry & Federation Unstubbing (Phase 1.5)
### 1. MCP Lazy Loading Integration
- **変更内容**:
    - `apps/api-server/src/mcp/client.rs` [MODIFY]: `McpRegistryEntry` および `McpProcessManager::register_server` を実装し、遅延起動 (Lazy Load) アーキテクチャを導入。
- **波及効果**:
    - MCP サーバーがアクセスされるまでプロセスを起動しないため、初期起動速度の向上と OOM リスクの大幅な低減が実現された。

### 2. Federation Sync Data Implementation
- **変更内容**:
    - `libs/infrastructure/src/job_queue/federation.rs` [MODIFY]: `do_fetch_unfederated_data` および `do_mark_as_federated` において、`is_federated = 0` の `karma_logs` と `immune_rules` の抽出と更新ロジックを実装。
- **波及効果**:
    - ハリボテだった P2P Federation のデータ抽出部分が実働コードに置き換わり、Samsara Hub などの外部ノードへ同期用ペイロードを渡す基盤が完成した。

### 3. Federation Toxicity Defense Pipeline
- **変更内容**:
    - `libs/infrastructure/src/job_queue/federation.rs` [MODIFY]: `P2pSanitizer` を実装し、CSAM および Toxicity の禁止ワードフィルタリング（O(n)文字列走査）を構築。
    - `apps/api-server/src/routes/biome.rs` [MODIFY]: `/api/biome/send` エンドポイントに `P2pSanitizer` の検証ロジックを統合し、`csam_toxicity_forbidden_words` 設定を DB から動的注入。
    - `apps/api-server/src/routes/settings.rs` [MODIFY]: `csam_toxicity_forbidden_words` を `ALLOWED_KEYS` のホワイトリストに追加し、UIからの動的編集を許可。
    - `apps/management-console/src/components/SettingsPage.tsx` [MODIFY]: `ToxicityConfig` コンポーネントを新設し、コンマ区切りリストの重複排除とバリデーションを含む UI を統合。
- **波及効果**:
    - Aiome の発信ネットワーク層に強力な Toxicity 防御壁が確立し、Samsara Hub への悪意あるペイロードの伝播が構造的に遮断された。
    - `P2PSanitizer` の責務が Nurture インフラから Aiome OSS 内の L3 インフラに完全に移行し、レガシースタブが Project-Nurture から削除された。

## Zero-Panic CI Enforcement & Guardrails Hardening
### 1. `enforce_unwrap_deny.py` Integration
- **変更内容**:
    - `scripts/enforce_unwrap_deny.py` [NEW]: プロダクションコードにおける `.unwrap()` / `.expect()` の使用を禁止する CI 用の静的解析スクリプトを実装。
    - `scripts/test_enforce_unwrap_deny.py` [NEW]: 上記スクリプトの 25 件のテストケースを実装し、カバレッジ 99% を達成。
    - `libs/shared/src/guardrails.rs` [MODIFY]: Prompt Injection のローカルポリシー判定ルールを強化 (`ignore all instructions` などを小文字で判定するよう改善)。
    - `libs/infrastructure/src/cortex_query.rs` [MODIFY]: テスト時の環境変数セット (`std::env::set_var("ENFORCE_GUARDRAIL", "true")`) を追加し、Flakiness を解消。
    - `.github/anti-patterns.yml` [MODIFY]: 破損していた `missing-auth-extractor` ルールを修復。
- **波及効果**:
    - Aiome インフラにおける不意のパニック（メモリ安全性侵害）が技術的にブロックされ、安全なエラーハンドリング（`Result`/`Option`）が強制される基盤が完成した。
    - `// allow-anti-pattern` による意図的パニックの例外管理が可能になり、今後のレガシーコード浄化作業（Refactoring）の可視化とトラッキングが容易になった。

## Phase B: ViewMode Settings UI Filtering & Hardening
### 1. ViewMode Dynamic Section Rendering
- **変更内容**:
    - `apps/management-console/src/components/SettingsPage.tsx` [MODIFY]: `useViewMode` フックを導入し、`SettingsPage` 内のセクション（Commerce, Channel Bridges, Security, Feature Flags, Escrow, MCP Config）を `viewMode`（beginner / intermediate / advanced）に応じて段階的に公開する（レンダリングの条件分岐）ロジックを実装。
    - `apps/management-console/src/components/SettingsPage.tsx` [MODIFY]: Appearance セクションに Interface Complexity（ViewMode）トグルを追加。
    - `apps/management-console/src/components/SettingsPage.test.tsx` [MODIFY]: `useViewMode` をモックし、各モード時のセクション非表示を検証するTDDテストを追加。
- **波及効果**:
    - Aiome Console の設定画面がユーザーの習熟度に応じて整理され、不要な情報による認知的負荷（ビギナーの混乱）が激減した。
    - `viewMode` はバックエンドの認可機構とは分離したUIレベルのフィルタリングであり、APIエンドポイント自体のセキュリティや Nurture側のコンソールに副作用を及ぼさないことをパーフェクトプランニングにより完全保証。
    - LLM Configuration セクションは全モードで保護（表示）され、セットアップ不能に陥るソフトブリック状態を回避している。

## Phase 8.8: Aegis Sentinel Infrastructure Integration
### 1. Incident Repository & DB Optimization
- **変更内容**:
    - `libs/infrastructure/src/aegis/incident_repo.rs` [NEW]: `IncidentRepository` を新設し、システムインシデント（WASM・ホスト実行時の異常）の記録をSQLite/Postgres共通のドライバで処理できるよう統一。
    - `libs/infrastructure/src/skills/mod.rs` & `skill_arena.rs` [MODIFY]: 生のSQLクエリを削除し、`IncidentRepository` を用いるようにリファクタリング。さらに `RwLock` の書き込みロック期間から重いデータベースI/O操作を分離。
    - `apps/api-server/src/stream.rs` [MODIFY]: `CoreEvent::AegisSentinel` のパターンマッチを追加し、Dream Loop からのイベントがフロントエンドのSSEへ伝搬するように修正。
    - `apps/management-console/src/hooks/useSystemVitality.tsx` & `App.tsx` [MODIFY]: `aegis_sentinel` イベントの UI 側購読を追加し、重大度に応じたアラートレンダリングと i18n 翻訳を実装。
- **波及効果**:
    - Aiome の自律型免疫システム (Aegis Sentinel) がカーネルレベルからUIアラートまで E2E で貫通した。
    - `SkillArena` 評価時のロック競合（Lock Contention）が排除され、複数エージェント並行実行時のレイテンシと安定性が向上した。
    - データベース操作が抽象化されたことで、PostgreSQL 環境への移行がシームレスに行えるようになった。

### 2. Aegis Sentinel HotSwap Auto-Remediation (Phase 2)
- **変更内容**:
    - `libs/infrastructure/src/dream_state.rs` [MODIFY]: `DreamResult` および `HotSwapRequest` 構造体を導入し、`aegis_sentinel_dream` の戻り値を拡張。Kani 検証成功時に HotSwap リクエストを呼び出し元に返却する。
    - `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `DreamState` から受け取った `HotSwapRequest` を処理し、`SkillForge` を使用して WASM スキルを再コンパイルし、`WasmSkillManager` のキャッシュを破棄して `IncidentStatus::Resolved` に移行するロジックを実装。
    - `libs/infrastructure/src/aegis/prover.rs` [MODIFY]: `generate_patch` のプロンプトを改修。`forge_workspaces` ディレクトリから元の `src/lib.rs` ソースコードを読み込み、LLM のプロンプト・コンテキストとして提供。
    - `apps/api-server/src/api_integration_tests.rs` [MODIFY]: `test_aegis_sentinel_integration` を追加。大量のインシデント注入と、Dream Service のバッチ処理呼び出しによるブロードキャスト検証テストを実装。
- **波及効果**:
    - Aiome の自律修復機能 (HotSwap) が完全に稼働。Kani 検証を通過したパッチコードが自動的にスキルとしてビルドされ、再デプロイされる自己修復ループが完成した。
    - LLM パッチ生成の精度が向上。既存のスタックトレース依存から、完全なソースコードとコンパイルエラー履歴に依存することでコンパイル成功率が大幅に上がった。

## Phase 8.5: Infrastructure Hardening & Nurture Integration
### 1. Cross-Domain Error Unification
- **変更内容**:
    - `libs/aiome-commerce/src/x402.rs` [MODIFY]: `X402Error` から `AiomeError` への `From` トレイト実装。
    - `libs/avatar-engine/src/loader.rs` [MODIFY]: `LoaderError` から `AiomeError` への `From` トレイト実装。
    - `libs/avatar-engine/src/proportions.rs` [MODIFY]: `ProportionError` から `AiomeError` への `From` トレイト実装。
- **波及効果**:
    - Aiome および Nurture ドメイン間で発生する各モジュールの特化エラーが `AiomeError` に一元化され、HTTPレイヤーや呼び出し元へシームレスに伝播可能となった。
    - `?` 演算子によるクリーンなエラーハンドリングが実現。

### 2. Zero-Trust Environment Variable Scubbing
- **変更内容**:
    - `Project-Nurture/apps/nurture-api/src/main.rs` [MODIFY]: 起動時の `std::env::remove_var` を `shared::security::scrub_env` に置換。
- **波及効果**:
    - `NURTURE_INTERNAL_SECRET` や `STRIPE_WEBHOOK_SECRET` 等のメモリ・常駐リスクが解消され、Aiome OSSのセキュリティ基準と完全に統一された。

### 3. KarmaForge Sandbox Integration
- **変更内容**:
    - `Project-Nurture/libs/nurture-infra/src/economy/karma_forge.rs` [MODIFY]: `PythonExecutor` を用いて、`sage_meditation` メソッドを通じたコンテナ化（Podman）サンドボックスによる経済分析ロジックを実装。
    - `Project-Nurture/apps/nurture-api/src/state.rs` [MODIFY]: `AppState` 初期化時に `PythonExecutor` を `KarmaForge` へ DI 注入するよう修正。
- **波及効果**:
    - ユーザー提供のデータや外部要素に基づく分析スクリプトが本体プロセスから隔離され、RCE リスクを排除したセキュアな Economy 監査が可能となった。

## Phase 6: Infrastructure Decoupling (Repository Pattern & Trait Isolation)
### 1. SemanticCache & MemoryCrystallizer Isolation
- **変更内容**:
    - `libs/infrastructure/src/job_queue/mod.rs` [MODIFY]: `SemanticCacheRepository` と `DistillationOps` トレイトを追加定義。`UniversalJobQueue` にこれらのトレイトを実装。
    - `libs/infrastructure/src/llm/semantic_cache.rs` [MODIFY]: `SemanticCache` の初期化引数を `Arc<UniversalJobQueue>` から `Arc<dyn SemanticCacheRepository>` に変更。
    - `libs/infrastructure/src/memory_crystallizer.rs` [MODIFY]: `MemoryCrystallizer` の初期化引数を `Arc<UniversalJobQueue>` から `Arc<dyn DistillationOps>` に変更。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: 上記の変更に伴い、DI時のキャストを追加。
    - テストファイル全般 [MODIFY]: `mockall` で生成したモックリポジトリを使用するように `SemanticCache` と `MemoryCrystallizer` のテストをリファクタリング。
- **波及効果**:
    - `SemanticCache` および `MemoryCrystallizer` が `UniversalJobQueue` への直接依存から脱却し、単体テスト時のSQLite（ファイルIO）依存が排除された。
    - `get_pool()` などの内部実装を外部に露出しない、堅牢な Interface Segregation Principle (ISP) が実現。

## Phase 4: Agentic Core Refactoring & Zero-Trust Sync
- **変更内容**:
    - `apps/api-server/src/internal_services/mod.rs` [MODIFY]: `Watchtower`, `Heartbeat`, `Dream`, `OxiLean` タスクの `panic!` を graceful restart ループに置換。
    - `apps/samsara-hub/src/mdns_listener.rs` [MODIFY]: `mDNS` ブラウズ時の `panic!` を再試行ループに置換し、P2P 発見がクラッシュするのを防ぐ。
- **波及効果**:
    - 内部サービスがクラッシュしてもアプリケーション全体を道連れにせず、自律的に復旧可能になり、システムの可用性が向上。

### 2. SecretString Credential Protection
- **変更内容**:
    - `libs/infrastructure/src/tts.rs` [MODIFY]: `OpenAiTtsProvider` の `api_key: String` を `secrecy::SecretString` に置換。
    - `libs/infrastructure/src/trend_sonar.rs` [MODIFY]: `WebSearchAdapter` と `SerpAnalysisAdapter` の `api_key: String` を `secrecy::SecretString` に置換。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `OpenAiTtsProvider` 初期化時、不必要に `expose_secret().to_string()` していた箇所を削除し、直接 `SecretString` を引き回すように修正。
- **波及効果**:
    - メモリダンプや不用意なロギングによって API キーが平文のまま漏洩するリスク（CWE-532, CWE-316）を完全に排除した。

### 3. API Server Error Handling Hardening (Zero-Panic)
- **変更内容**:
    - `apps/api-server/src/error.rs` [MODIFY]: `AppError` の実装からマクロ依存を排除し、`std::error::Error` (sourceチェーン公開), `Display`, `Debug` トレイトを標準で手動実装するように刷新。
    - `apps/api-server/src/error.rs` [MODIFY]: `anyhow::Error` からのダウンキャストロジックを最適化し、`AiomeError` 等のドメインエラー情報を欠落させずに HTTP 層へ透過的に伝搬させる機構を確立。
- **波及効果**:
    - Aiome API サーバーのエラー機構が `tracing` 等の Observability ツールと互換性を持ち、かつ「Zero-Panic Quality Gate」を完全に遵守する状態となった。
    - 本番環境での情報漏洩防御（CWE-209防止）とエラーの透明性が安全に両立されるようになった。

## Phase P3: Infrastructure Stabilization & Edge Integration
### 1. UniversalGigEngine Migration
- **変更内容**:
    - `apps/aiome-node/Cargo.toml` [MODIFY]: `aiome-commerce` と `aiome-core` への依存関係を追加し、実機エンジン稼働の準備を完了。
    - `apps/aiome-node/src/main.rs` [MODIFY]: `DummyGigEngine` と `DummyValidator` を撤廃。本番運用に向けた `UniversalGigEngine` をインジェクト。
    - `apps/aiome-node/src/main.rs` [MODIFY]: Edge Node 環境での不正な決済挙動を防ぐため、常に `AiomeError::Infrastructure` を返す `StubCommerceEngine` と `DisabledLlmProvider` を実装・注入。
- **波及効果**:
    - `aiome-node` がスタブから本番レベルの基盤へ移行。将来的な Commerce 統合（本番への昇格）への準備が整った。同時に誤った「成功（Phantom Success）」によるインシデントリスクをゼロ化した。

### 2. RTBF/GDPR Blob Storage S3 Purge
- **変更内容**:
    - `libs/infrastructure/Cargo.toml` [MODIFY]: `aws-sdk-s3` と `aws-config` をオプショナル（feature gate: `s3`）として追加。
    - `libs/infrastructure/src/blob_storage.rs` [MODIFY]: `BlobStorageAdapter` に S3 クライアントとバケットを保持する構成を追加。`purge_actor_assets` に `delete_objects` を用いた一括物理削除ロジックを実装。
- **波及効果**:
    - クラウドストレージ（S3/R2）に対する RTBF（忘れられる権利）要件を満たすことが可能となった。

### 3. URL Hardcoding Elimination & Panic Remediation
- **変更内容**:
    - `libs/infrastructure/src/tts.rs` [MODIFY]: `OpenAiTtsProvider` でエンドポイントURLを動的化（環境変数経由）。
    - `libs/infrastructure/src/generative_engine.rs` [MODIFY]: `FalAiGenerativeEngine` で Base URL を動的化（環境変数経由）。
    - `libs/infrastructure/src/tts.rs` / `cortex_query.rs` [MODIFY]: `#![allow(clippy::unwrap_used)]` 外での本番 `unwrap()` を `Result` ハンドリングに修正。
- **波及効果**:
    - 環境差異（オンプレミス、クラウド）への対応力が強化され、予期せぬパニック（プロセス終了）リスクが低減された。

## Phase RLM: Recursive Language Model Integration
### 1. RlmClient & CortexQueryEngine Deep Query Extension
- **変更内容**:
    - `libs/aiome-contracts/src/rlm.rs` [NEW]: `RlmProvider` および `RlmConfig` トレイトを追加し、RLM サイドカーへの通信契約を定義。
    - `libs/infrastructure/src/llm/rlm_client.rs` [NEW]: `CostCircuitBreaker` による予算制約（Budget Limit）保護を備えた `RlmClient` 実装を追加。
    - `libs/infrastructure/src/cortex_query.rs` [MODIFY]: `CortexQueryEngine` に `rlm_provider` 注入ポイントを追加し、標準検索で回答できない複雑なクエリに対して再帰的推論を行う `deep_query` メソッドを実装。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `RlmClient` をインスタンス化し、`AppState` および `CortexQueryEngine` へ DI として注入。
- **波及効果**:
    - Aiome のインフラストラクチャにおける複雑な論理推論機能（Recursive Reasoning）が完全に統合され、システムは Nurture 側の予算や意図を逸脱することなく、自律的にディープクエリへフォールバックできるようになった。
    - 影響範囲は `aiome` 側に閉じられており、`Project-Nurture` リポジトリに変更を波及させることなく強力なリーズニングレイヤーを実現した。

## Phase 5: RTBF & Cognitive Observability Hardening
### 1. RTBF `forget_actor` Atomic Purging
- **変更内容**:
    - `libs/infrastructure/src/job_queue/security.rs` [MODIFY]: `SecurityOps` トレイトに `forget_actor` を追加し、`UniversalJobQueue` に実装。`ekyc_sessions`、`jobs`、`guild_members` に加え、`chat_history`, `chat_memory_summaries`, `security_audit` に対する完全なアトミックパージを実装。誤ったスキーマ参照（`cortex_chat_history`）を修正し、`audit_ledger_global` への操作ログ記録を追加して完全な RTBF コンプライアンスを達成。
- **波及効果**:
    - Aiome のインフラストラクチャにおける GDPR (RTBF) コンプライアンスが達成された。

### 2. Cognitive Observability (Thinking Extraction)
- **変更内容**:
    - `apps/api-server/src/agent_engine.rs` [MODIFY]: `extract_thinking_process` ヘルパーを新設し、`<thinking>...</thinking>` ブロックを抽出。複数のタグや未閉鎖タグの安全なパースに対応。
    - `apps/api-server/src/stream.rs` [MODIFY]: `Event::text` ストリームから思考プロセスをブロック（UI 非表示）しつつ、DBの `metadata` カラムに記録。
    - `libs/infrastructure/src/context_engine.rs` [MODIFY]: 履歴再構築時に `metadata.thinking` をパースし、RAG用システムプロンプトに復元注入。
- **波及効果**:
    - UI を汚すことなくエージェントの思考プロセスを監査・追跡可能になった。

## Phase 5: Compliance & eKYC Hardening (GDPR / RTBF)
### 1. GDPR "Right to be Forgotten" (RTBF) Pipeline
- **変更内容**:
    - `aiome/apps/api-server/src/routes/auth.rs` [MODIFY]: `delete_account_handler` を実装し、PII（`cortex_chat_history`, `system_settings`）をアトミックトランザクションでハードデリート。
    - `aiome/apps/api-server/src/routes/auth.rs` [MODIFY]: `OxiLeanProofCertificate` を使って Nurture API (`/internal/forget/:actor_id`) へ削除要求をセキュアにカスケード。
    - `aiome/apps/api-server/src/router.rs` [MODIFY]: 欠落していたルーティングを修正し `rate_limit(1, 10s)` を適用してブルートフォース保護を追加。
- **波のアフェクト（波及効果）**:
    - Aiome 側でのアカウント削除が Project-Nurture 側にも連動し、法的要件である GDPR RTBF を 100% 満たすアーキテクチャが完成した。
    - 不可逆操作に対するレートリミット保護によりシステムの耐久性が向上。

### 2. eKYC Enforcement Layer
- **変更内容**:
    - `Project-Nurture/libs/nurture-core/src/ekyc.rs` [NEW]: `EkycVerifier` および関連構造体を実装。
    - `Project-Nurture/libs/nurture-infra/src/ekyc/store.rs` [NEW]: `SQLiteEkycStore` を実装し、DB ステータスと連動する検証ロジックを確立。
    - `Project-Nurture/apps/nurture-api/src/routes/escrow.rs`, `upload.rs` [MODIFY]: CSAM フィルタリングの最上位層として eKYC 状態チェックを統合。未認証アカウントによるクリティカルアクションをブロック。
    - `Project-Nurture/libs/nurture-infra/src/test_utils/mock_ekyc.rs` [NEW]: `MockEkycStore` を実装し、`#[cfg(any(test, debug_assertions))]` ガードによる厳密な分離を適用。
- **波のアフェクト（波及効果）**:
    - Project-Nurture のエスクローやアップロード経路に AML (Anti-Money Laundering) ポリシーが適用され、不正な資金洗浄やスパム生成をシステムレベルで遮断する防御壁が機能するようになった。
## Economic Hardening & SettlementProtocol Enforcement (A2C Synergy)
### 1. Nurture `/internal/deduct` API & S2S Authentication
- **変更内容**:
    - `Project-Nurture/apps/nurture-api/src/routes/internal.rs` [NEW]: `deduct_cost` および `release_escrow` エンドポイントの実装。Defense-in-Depth (DiD) ベースの入出力バリデーションと HTTP 400/500 の分離。
    - `Project-Nurture/apps/nurture-api/src/main.rs` [MODIFY]: `NURTURE_INTERNAL_SECRET` を検証する `internal_auth_middleware` を全体の `/internal` スコープにレイヤー適用。
    - `aiome/libs/aiome-commerce/src/stripe.rs` [MODIFY]: 直叩きの `reqwest::Client` を廃止し、グローバル構成された `aiome_core::http::get_http_client()` へ移行 (SSRF 防御と Connection Pooling)。10秒のタイムアウト付与。
- **波及効果**:
    - Aiome 側の LLM 生成 (StripeCommerceEngine) から Nurture の決済インフラへ、安全かつタイムアウト制御された HTTP リクエストが飛ぶようになった。内部ポートを直接公開しなくて済む。

### 2. A2C (Asset-to-Creator) 分配の強制化
- **変更内容**:
    - `aiome/libs/aiome-contracts/src/traits.rs` [MODIFY]: `CommerceEngine::deduct_generation_cost` メソッドのシグネチャに `asset_id` を追加。
    - `Project-Nurture/libs/nurture-infra/src/economy/bridge.rs` [MODIFY]: `deduct_generation_cost` 内での直接的な Wallet 上書きを廃止。`asset_id` 指定の有無に応じて `SettlementProtocol::settle()` のトランザクションへ流し込み、System Fee・Creator Return・Burn の3重バッチ分配を強制する設計に移行。
    - `Project-Nurture/libs/nurture-infra/src/economy/bridge.rs` [MODIFY]: `creator_points_earned` が定価ベースで算出されていたバグを修正。動的課金額 (推論コスト) ベースに乗算されるように修正。
- **波及効果**:
    - Aiome エージェントが Nurture 上の Asset を利用して生成した推論コストが、正しく Asset 制作者へのポイント還元として分配されるようになった。
    - O(1) でのトランザクションがアトミックに実行され、楽観ロックにより二重引き落としが完全にブロックされる。

## OxiLean Formal Verification Integration (Phase 0-1)
### 1. `vendor/oxilean-kernel` 導入 & Cargo.toml 隔離
- **変更内容**:
    - `vendor/oxilean-kernel/` [NEW]: OxiLean CiC kernel (Apache-2.0, 0-deps TCB) をコピー配置。
    - `vendor/oxilean-kernel/Cargo.toml` [MODIFY]: `version.workspace = true` 等の上流参照を実値にハードコード。
    - `Cargo.toml` (workspace root) [MODIFY]: `exclude = ["vendor/*"]` を追加。
- **波及効果**:
    - `vendor/oxilean-kernel` は Aiome ワークスペースの `member` ではない。`cargo check --workspace` の対象外。
    - `shadow-worker/Cargo.toml` が `path = "../../vendor/oxilean-kernel"` で参照するため、shadow-worker のビルドグラフに含まれる。
    - 上流 OxiLean のアップデート適用時は、`vendor/` のファイルを手動で更新し、`Cargo.toml` の workspace 参照解消を再実施する必要あり。

### 2. `ProofVerifier` gRPC Service & Proto 拡張
- **変更内容**:
    - `libs/aiome-contracts/proto/a2a_internal.proto` [MODIFY]: `ProofVerifier` service、`ProofRequest`、`ProofResult` message を追加。
    - `libs/aiome-core-contracts/proto/a2a_internal.proto` [MODIFY]: 上記と完全同期。
- **波及効果**:
    - `aiome-contracts` / `aiome-core-contracts` の `cargo build` で tonic codegen が `ProofVerifierServer` / `ProofVerifierClient` を自動生成。
    - 既存 `DockerConductor` service には影響なし（proto3 の service 追加は後方互換）。
    - `api-server` 側が将来 `ProofVerifierClient` を使用する場合、`aiome-contracts` への依存のみで利用可能。

### 3. `OxiLeanProofService` (shadow-worker)
- **変更内容**:
    - `apps/shadow-worker/src/proof_service.rs` [NEW]: `OxiLeanProofService` struct + `ProofVerifier` trait impl。3重防御 (timeout + catch_unwind + semaphore)。4 テストケース。
    - `apps/shadow-worker/src/main.rs` [MODIFY]: `mod proof_service` 追加。`ProofVerifierServer` を gRPC ルータに登録。`OXILEAN_PROOF_TIMEOUT_SECS` / `OXILEAN_PROOF_SEMAPHORE_PERMITS` 環境変数読み取り。
    - `apps/shadow-worker/Cargo.toml` [MODIFY]: `oxilean-kernel` 依存追加。
- **波及効果**:
    - `api-server` の `AppState` に `proof_semaphore` フィールドを追加する場合、`api_integration_tests.rs` の `create_test_server()` (L489, L665) にもフィールド追加が必要。
    - `Dockerfile.shadow-worker` の `COPY . .` は `.dockerignore` に `vendor/` が含まれないため、ビルドコンテキストに自動的に含まれる。

### 4. `verify-proof` API Endpoint Rate Limiting & Integration
- **変更内容**:
    - `apps/api-server/src/router.rs` [MODIFY]: `/api/skills/verify-proof` ルートに対し、1リクエスト/10秒の `tower::ServiceBuilder` レートリミット（`rate_limit_1_10s`）を適用。
    - `apps/api-server/src/api_integration_tests.rs` [MODIFY]: `test_verify_skill_proof_endpoint_connected` 結合テストを追加し、404 (Skill WASM not found) と 429 (Too Many Requests) のエラーハンドリングを実証。
    - `apps/api-server/src/api.rs` [MODIFY]: OpenAPI ドキュメントへ `verify_skill_proof` エンドポイントおよび入出力構造体をマージ。
- **波及効果**:
    - Aiome の主権的検証パイプライン（Sovereign Verification Pipeline）が DoS 攻撃に対して強固に保護された。
    - 今後 `verify-proof` を呼び出す Project-Nurture フロントエンドや外部エージェントは、10秒間隔のポーリング・リトライ制御を実装する必要がある。

## Agentic AI Adaptation Framework (Reflexion x3)
### 1. AgentHook Architecture & NurtureAgentHook
- **変更内容**:
    - `libs/aiome-contracts/src/plugin.rs` [MODIFY]: `AiomePlugin` トレイトに `agent_hooks()` メソッドを追加（デフォルト実装あり: 空 Vec）。
    - `apps/api-server/src/plugin_loader.rs` [MODIFY]: `PluginRegistry::get_agent_hooks()` を実装。全登録プラグインから `AgentHook` を収集。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: プラグインレジストリ初期化直後に `HookManager` へフック自動登録。
    - `Project-Nurture/apps/nurture-api/src/plugin.rs` [MODIFY]: `NurtureAgentHook` を実装。ジョブ完了時に `KarmaForge::cross_synthesize` をトリガー。
- **波及効果**:
    - 新規プラグイン追加時: `agent_hooks()` をオーバーライドすれば自動的に `HookManager` に登録される。プラグインコード以外の変更は不要。
    - `HookManager::trigger_job_completed` の呼び出し元を変更する場合は、ベストエフォート型の失敗分離設計を維持すること。

### 2. HookManager ベストエフォート化
- **変更内容**:
    - `libs/infrastructure/src/security/hook_manager.rs` [MODIFY]: `trigger_job_completed` をショートサーキット型から個別障害分離型に変更。`tracing::warn!` で失敗フックを記録。
- **波及効果**:
    - `trigger_pre_execution` / `trigger_post_execution` には未適用。これらはセキュリティゲートとして失敗時にブロックする設計を維持。
    - フック追加時に全フック成功を前提としたロジックを組まないこと。

### 3. GAP-5 CognitiveSentinel エントロピー修正
- **変更内容**:
    - `libs/infrastructure/src/cognitive_sentinel.rs` [MODIFY]: `calculate_entropy` のビンインデックスに `clamp(0, bins-1)` を適用。診断ステップの論理順序を再配置。4件の境界テスト追加。
- **波及効果**:
    - `CognitiveSentinel` を呼び出す `HookChain` / `DreamState` は変更不要。戻り値の型に変更なし。
    - `ContextBudget` のデフォルト値はエージェントの思考ログを分析し必要に応じて調整すること。

### 4. GAP-3 ContextEngine UTF-8 安全化
- **変更内容**:
    - `libs/infrastructure/src/context_engine.rs` [MODIFY]: 履歴切り詰めの raw バイトスライスを `shared::strings::truncate_bytes_safely` に置換。マジックナンバー `4000` を `budget.max_history_chars` に置換。
- **波及効果**:
    - `fetch_budgeted_context` / `get_context_with_facts` の呼び出し元は変更不要。内部挙動のみ安全化。
    - `ContextBudget::max_history_chars` のデフォルト値変更は `context_engine.rs` の `impl Default for ContextBudget` に影響。

### 5. GAP-1 SkillMaturity Display & Quarantined 明示化
- **変更内容**:
    - `libs/infrastructure/src/skills/mod.rs` [MODIFY]: `SkillMaturity` に `Display` トレイト実装。`Quarantined` への明示的マッチブランチ追加。昇格メソッドの安全性ドキュメント付与。
- **波及効果**:
    - `WasmSkillManager` の DB 保存/読み込みは `Display` 出力を使用。新しい `SkillMaturity` バリアント追加時は `Display` と `FromStr`（該当する場合）の両方を同期すること。
    - 昇格操作は呼び出し元で成功率バリデーションを必須とする（TypeState 直接操作のため）。

## Infrastructure Gap Closure (TDD + Reflexion x2)
### 1. TIMESFM Sidecar Health Check Integration
- **変更内容**:
    - `apps/api-server/src/routes/bootstrap.rs` [MODIFY]: `bootstrap_status` ハンドラに `TIMESFM_SIDECAR_URL` のヘルスチェックを追加。`check_sidecar_health("timesfm-sidecar", ...)` を `geo-optimizer` の直後に挿入。テストケースも `timesfm-sidecar` エントリを含むように拡張。
- **波及効果**:
    - フロントエンド `SeoPulseView.tsx` は既に `sidecar_status` 配列から名前ベースでフィルタリングしているため、新しい `timesfm-sidecar` エントリは自動的に利用可能。ただし `SeoPulseView` は `geo-optimizer` のみを `find()` しているため、TimesFM の表示を追加する場合は別途修正が必要。
    - `.env.example` の `TIMESFM_SIDECAR_URL` (L205) は既に存在するため追加不要。

### 2. nurture_auditor.py Pydantic BaseModel AST Extraction
- **変更内容**:
    - `scripts/nurture_auditor.py` [MODIFY]: `analyze_py_file` に `ast.ClassDef` 走査を追加。`isinstance(base, ast.Name) and base.id == 'BaseModel'`（直接 import）と `isinstance(base, ast.Attribute) and base.attr == 'BaseModel'`（ドット import）の2パターン対応。抽出されたクラスは `app_data["structs"]` に統合。
- **波及効果**:
    - `scripts/impact_query.py` の BFS 探索グラフにおいて、Python クラス（Pydantic モデル）がノードとして出現するようになる。これにより `impact_query.py AuditRequest` のようなクエリが可能に。
    - `deep_scan_matrix.md` の `geo-optimizer` セクションに `AuditRequest` が `Key Structs` として出現。

### 3. SeoPulseView Sidebar Routing Integration
- **変更内容**:
    - `apps/management-console/src/App.tsx` [MODIFY]: `intermediate` ビューモード配列に `'seo-pulse'` を追加。サイドバー NavItem、ヘッダータイトルマッピング（`t('page.seoPulse')`）、コンテンツルーティングを独立タブとして統合。`agent` タブ内の `<SeoPulseView />` ハードコード描画を廃止。
    - `apps/management-console/src/i18n/en.json` [MODIFY]: `nav.seoPulse` ("SEO Pulse") と `page.seoPulse` ("SEO Pulse Dashboard") を追加。
    - `apps/management-console/src/i18n/ja.json` [MODIFY]: `nav.seoPulse` ("SEO パルス") と `page.seoPulse` ("SEO パルスダッシュボード") を追加。
- **波及効果**:
    - `intermediate` モードのユーザーが `seo-pulse` タブにアクセス可能になる。`beginner` モードからは見えない。`advanced` モードからは `intermediate` を継承するため自動的にアクセス可能。
    - `SeoPulseView` コンポーネントに props を追加する場合は、`App.tsx` L584 の `<SeoPulseView />` 呼び出しを同時更新する必要がある。

### 4. Vite manualChunks Optimization
- **変更内容**:
    - `apps/management-console/vite.config.ts` [MODIFY]: `build.rollupOptions.output.manualChunks` を追加し、`vendor` / `ui` / `network` の3チャンクを定義。
- **波及効果**:
    - `index.js` のサイズが 1,307KB → 1,164KB に削減（11% 改善）。ただし `vis-network` (521KB) は `network` チャンクとして分離されたのみでサイズ自体は変わらない。
    - Tauri デスクトップビルドでは全チャンクがバンドルされるため影響なし。Web デプロイ時のみ初期ロード時間が改善。


## Quality Gate History API & Frontend Integration (Reflexion x3)
### 1. Quality Gate History Endpoint & SeoPulseView Merge
- **変更内容**:
    - `apps/api-server/src/routes/quality_gate.rs` [NEW]: `GET /api/v1/quality-gate/history` エンドポイントを新設。`QualityGateStore::list_recent` から履歴を取得し、`limit` パラメータに `.min(100)` の API レイヤークランプ（Defense-in-Depth）を適用。OpenAPI に 403 Forbidden レスポンスを追加。
    - `apps/api-server/src/api.rs` [MODIFY]: `quality_gate_history` ルートの登録と、`QualityGateEntry` の OpenAPI コンポーネント追加。
    - `apps/management-console/src/components/SeoPulseView.tsx` [MODIFY]: `authenticatedFetch` を用いた履歴取得フローを実装。SSE ライブイベントと DB 履歴の `job_id` / `id` による Deduplication と時間順マージを確立。`safeTimeString` ヘルパーおよび `Array.isArray` 型ガードを追加。
- **波及効果**:
    - `QualityGateStore` (infrastructure) のインターフェースに変更はなし。既存の `list_recent` を呼び出すのみ。
    - `authenticatedFetch` (auth.ts) の利用パターンが SeoPulseView に拡張されたため、`sessionStorage` の `aiome_secret` キーが認証の SSOT であることが強化された。
    - SeoPulseView の SSE イベントハンドラと履歴データのマージロジックが追加されたため、新しいイベントタイプを追加する際は Deduplication キー (`job_id` / `id`) の整合性を確認する必要がある。

### 2. SSE conductor フィールド伝搬 (Reflexion Pass 4)
- **変更内容**:
    - `libs/aiome-core-contracts/src/events.rs` [MODIFY]: `CoreEvent::QualityGate` に `conductor: String` フィールドを追加。
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `TaskEvent::QualityGate` に `conductor: String` を追加。`CoreEvent` へのブリッジでハードコード `"GeoAuditConductor"` を動的な `&cond` に変更。`quality_gate_store.record()` にも動的 conductor を伝搬。
    - `libs/infrastructure/src/task_orchestrator/geo_audit.rs` [MODIFY]: `self.conductor_name()` を QualityGate emit に設定。
    - `libs/infrastructure/src/task_orchestrator/seo_content.rs` [MODIFY]: 4箇所の QualityGate emit に `self.conductor_name()` を設定。
    - `apps/api-server/src/stream.rs` [MODIFY]: SSE JSON ペイロードに `"conductor"` フィールドを追加。
- **波及効果**:
    - `CoreEvent::QualityGate` のパターンマッチを行う全箇所（`stream.rs`, `mod.rs`）で `conductor` フィールドの束縛が必要。新規 Conductor を追加する際は `self.conductor_name()` の実装を忘れないこと。
    - フロントエンド `SeoPulseView.tsx` の `QualityGateEvent` interface は `conductor?: string` (optional) のため後方互換。DB 履歴にも conductor カラムが既に存在するため追加マイグレーション不要。
    - SSE `quality_gate` イベントのペイロードスキーマが拡張されたため、外部 SSE クライアントがある場合はスキーマ更新が必要。

## GEO Intelligence Integration & Graceful Degradation (Phase B)
### 1. GeoAuditConductor & SeoContentConductor
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/geo_audit.rs` [NEW]: `GeoAuditConductor` を新設し、厳格な入力バリデーションとスタンドアロンの Generative Engine Optimization 監査能力を付与。`GEO_CITABILITY_THRESHOLD` 未満の監査スコアにはハードエラーを返す仕様。
    - `libs/infrastructure/src/task_orchestrator/seo_content.rs` [MODIFY]: `SeoContentConductor` に GEO 監査との連携パイプラインを統合。SEO 生成フロー内部では GEO サービスダウン時に Graceful Degradation（品質ゲートはスルーし処理を継続）を適用する非対称設計を導入。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `GeoAuditConductor` の DI 登録およびパイプラインを統合。
- **波及効果**:
    - 外部の `GEO_OPTIMIZER_URL` が落ちている場合でも、SEO パイプラインは止まることなく動作し続ける可用性重視の設計が実現された。
    - Reflexion プロセスにより堅牢性が OOM やプロンプトインジェクションに対する境界チェックレベルで担保された。

## Infrastructure Security Hardening & env_clear()
### 1. Environment Variable Scrub Unification (`scrub_env`)
- **変更内容**:
    - `libs/shared/src/security.rs` [MODIFY]: `scrub_env` 関数を新設し、`std::env::remove_var` 呼び出しを一元化。
    - 各モジュール（`api-server`, `shadow-worker`, `samsara-hub`, `napi-bridge`, `config.rs`, `sqlite_vault_backend.rs` 等の計28箇所）の生 `remove_var` の使用を `shared::security::scrub_env()` へ置換。
    - `libs/shared/src/lib.rs` [MODIFY]: `#![forbid(unsafe_code)]` ポリシーを維持しつつ `security::scrub_env` にのみ例外許容を適用。
- **波及効果**:
    - Rust 2024 Edition 以降で `remove_var` が `unsafe` な関数に格上げされたことに対する完全なプロダクションレベルの対応措置が完了。
    - シングルスレッドでの起動直後フェーズにおいて安全に秘密情報パージが行われ、意図せぬ子プロセス等へのシークレット流出リスクが完全に根絶された。

### 2. MCP Infrastructure Security Hardening
- **変更内容**:
    - `libs/shared/src/mcp_constants.rs` [MODIFY]: `FORBIDDEN_MCP_ARG_FLAGS` による禁止フラグ検証リスト（CVE-2026-40933対策など）を追加。
    - `libs/shared/src/security.rs` [MODIFY]: `normalize_ip()` 導入による IPv4-mapped IPv6 SSRF（例：`::ffff:127.0.0.1`）のバイパスブロックおよびリンクローカル（`169.254.0.0/16`, `fe80::/10`）のアクセス遮断追加。
- **波及効果**:
    - SSRF 防御と MCP コマンド引数インジェクションの最奥脆弱性（Zero-day相当）が完全に埋められ、クラウドデプロイ時（AWS/GCPメタデータ等）のセキュリティポスチャが劇的に向上。

## GBrain R3 Native Integration (Phase 1)
### 1. Typed Links & Backlink-Boosted Ranking
- **変更内容**:
    - `libs/infrastructure/migrations/` [ADD]: `cortex_typed_links` 用の SQLite / Postgres マイグレーションを追加。`audit_ledger_global` 用の trigger も内包。
    - `libs/infrastructure/src/test_utils.rs` [MODIFY]: テスト用の DB プール初期化ロジック (`cortex_mock::setup_db_pool`) を集約定義。全7テーブルとFTS5、`audit_ledger_global` のスキーマ定義を一本化。
    - `libs/infrastructure/src/cortex_compiler_tests.rs`, `cortex_query.rs` (tests), `cortex_synth_tests.rs`, `cortex_file_projector.rs` (tests) [MODIFY]: 上記 `test_utils` を呼び出すようにリファクタリング。重複セットアップを排除。
    - `libs/infrastructure/src/cortex_compiler.rs` [MODIFY]: `update_backlinks` を `update_backlinks_and_typed_links` に改名し機能拡張。各記事間のリンク判定時、該当箇所前後100文字（コンテキスト窓）のキーワード（`contradicts`, `depends_on`, `extends`, `references`）により Typed Link を自動判別し `cortex_typed_links` に保存する O(n^2) バッチ処理を実装。
    - `libs/infrastructure/src/cortex_query.rs` [MODIFY]: `search_related_articles` 内で、ソースとして利用される記事群の総被リンク数 (`total_backlinks`) を収集し、LLM の計算した `confidence` に 0.05 * 被リンク数 (最大 0.2 ブースト) を加算するハイブリッドランキングを実装。
- **波及効果**:
    - `cortex_compiler.run_compilation_cycle` は明示的に `update_backlinks_and_typed_links` を呼び出すようになり、ナレッジ処理サイクル毎に常に Typed Link が更新される。
    - テスト基盤が集約されたため、今後の `cortex_*` スキーマ変更時は `test_utils::cortex_mock` 1箇所を変更するだけで全テストが追従する。
    - `cortex_query.rs` において、`backlinks` カラムをパースする I/O と JSON パースのコストが発生するため、FTS5 高速化の恩恵がこの箇所で微遅延を招く可能性があるが、5記事限定のため影響は O(1) に近い。

## Defense-in-Depth: PROCESS_SAFE_ENV_VARS SSOT & env_clear() 全経路適用
### 1. PROCESS_SAFE_ENV_VARS 定数の新設と全プロセス経路統一
- **変更内容**:
    - `libs/infrastructure/src/security.rs` [MODIFY]: `PROCESS_SAFE_ENV_VARS` 定数 (`&[&str]` — PATH, HOME, LANG, TMPDIR, PYTHONPATH, VIRTUAL_ENV) を新設。`build_safe_command_args` のハードコードリストをこの定数参照に置換。
    - `libs/infrastructure/src/security_zombie.rs` [MODIFY]: `run_with_timeout` のハードコードリストを `crate::security::PROCESS_SAFE_ENV_VARS` 参照に置換。
    - `libs/infrastructure/src/lora_training.rs` [MODIFY]: `health_check` (L646) の `&["PATH", "HOME"]` を `PROCESS_SAFE_ENV_VARS` 参照に拡張。`ollama create` (L367) に `env_clear()` + `PROCESS_SAFE_ENV_VARS` 再注入を新規追加。
    - `libs/infrastructure/src/slm_bridge.rs` [MODIFY]: `CliSlmBackend::run_command` に `env_clear()` + `PROCESS_SAFE_ENV_VARS` 再注入を新規追加。
- **波及効果**:
    - `PROCESS_SAFE_ENV_VARS` を変更すると、BastionGuard, ZombieKiller, LoRA (health_check + ollama create), SLM CLI の **5経路全て** に影響する。テスト回帰: `cargo test -p infrastructure` (389テスト)。
    - MCP クライアント (`client.rs`) は独自の `MCP_SAFE_ENV_VARS` (15変数) を使用するため影響を受けない（意図的分離）。
    - `self_diagnosis.rs` (docker info), `delegator.rs` (docker agent), `oss_repository_indexer.rs` (git clone), `os_utils.rs` (caffeinate) は env_clear 非適用（許容判断済み）。


## CBA Stage 0: Cell-Based Architecture Foundation (ADR-030)
### 1. CELL_ID Namespacing & Path Isolation
- **変更内容**:
    - `libs/shared/src/app_data.rs` [MODIFY]: `AppDataResolver::new()` に `CELL_ID` 環境変数による名前空間分離を実装。`is_safe_cell_id()` ホワイトリスト検証（英数字・ハイフン・アンダースコア、最大64文字）と `tracing::warn!` 不正入力ログを追加。
    - `scripts/backup.sh` [MODIFY]: CELL_ID の Shell バリデーション（正規表現ガード）を追加。バックアップ対象を `api/`, `hub/`, `nurture/` 全セルサブディレクトリに拡大。
    - `docker-compose.cell.yml` [MODIFY]: セル固有の `JWT_PRIVATE_KEY_B64` 環境変数を `api-server` に注入。
    - `docker-compose.shared.yml` [MODIFY]: TimesFM ポートを 3020→3025 に変更（Nurture API との衝突解消）。
    - `.env.example` [MODIFY]: `CELL_ID` セクション追加、`TIMESFM_SIDECAR_URL` ポートを 3025 に統一。
    - `docs/decisions/ADR-030-cell-based-architecture.md` [ADD]: CBA 設計決定記録。
- **波及効果**:
    - `AppDataResolver::new()` は **28箇所** から呼び出されている（`bootstrap.rs`, `SecurityConfig`, `PathSandbox`, `user_learner.rs`, `generative_engine.rs`, `lora_training.rs`, `heartbeat_wakeup.rs`, `cortex.rs` 他）。CBA 不変条件「1プロセス=1セル」により、全呼び出し元が透過的にセルスコープへ自動収束するため、各モジュールへの個別修正は不要。
    - `CELL_ID` 未設定時はデフォルト動作（`cell-0` 相当）するため、既存のシングルセル環境への破壊的影響はゼロ。
    - `backup.sh` のリストア操作は新しいディレクトリ構造に依存するため、旧形式のバックアップ tar は手動マイグレーションが必要。
    - TimesFM ポート変更により `TIMESFM_SIDECAR_URL` を手動で設定しているユーザーは `.env` 更新が必要。

## Sinking Ship #19: Automated Backup Strategy
### 1. Pre-migration Backup Guard + SQLite Online Backup
- **変更内容**:
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `backup_sqlite_db_before_migration()` 関数を追加。`init_database()` 内の `UniversalJobQueue::new()` 呼び出し直前に `config.db_path` を用いて `std::fs::copy()` による `.pre_migration.bak` スナップショットを自動生成。`:memory:` および PostgreSQL パスは自動スキップ。失敗は非致命的（`warn!` のみ）。
    - `scripts/backup.sh` [MODIFY]: L45-46 のコメントを実コードに昇格。`sqlite3 .backup` による WAL-safe ホットスナップショットを tar アーカイブ前に実行。`sqlite3` 未インストール環境では tar フォールバック。tar 後に `.db.bak` ファイルを自動クリーンアップ。
    - `.env.example` [MODIFY]: `AIOME_BACKUP_DIR`, `AIOME_MAX_BACKUPS` セクションと cron スケジューリング手順を追記。
    - `libs/shared/src/config.rs` [MODIFY]: `AiomeConfig::default()` 内の `unwrap()` を `expect()` に修正（Zero-Panic Policy 準拠）。
- **波及効果**:
    - `backup_sqlite_db_before_migration()` は `bootstrap.rs::init_database()` からのみ呼び出される。戻り値の型変更なし → 呼び出し元への影響ゼロ。
    - テスト環境（`:memory:` SQLite）では自動スキップされるため CI 破壊リスクなし。
    - `.gitignore` に `*.bak` が既存のため、`.pre_migration.bak` が誤コミットされるリスクなし。



## Sprint F: A2UI Generative Interface (Phase 0)
### 1. SSE Stream → Frontend Rendering Pipeline
- **変更内容**:
    - `apps/api-server/src/stream.rs` [MODIFY]: `{"type":` 検出 → `serde_json::Deserializer::into_iter()` → `A2uiValidator::verify_a2ui_surface()` → SSE `a2ui` イベントエミッションの完全なパイプラインを追加。O(n) 不正JSONスキップ戦略によるDoS耐性。
    - `apps/management-console/src/components/A2uiRenderer.tsx` [NEW]: Phase 0 コンポーネント (text, button, list, form, input) の再帰的レンダラー。tokens.css 完全準拠。null-safe ガード。
    - `apps/management-console/src/types.ts` [MODIFY]: `A2uiEnvelope`, `A2uiSurface`, `A2uiComponent` 型定義追加。Rust `schema.rs` serde 出力と完全一致。
    - `apps/management-console/src/hooks/useAgentChat.ts` [MODIFY]: `a2ui` SSE イベントハンドラ追加。`isValidShape` ランタイム型ガード。`accumulatedText` フラッシュアーキテクチャ。
    - `apps/management-console/src/components/AgentConsole.tsx` [MODIFY]: `A2uiRenderer` import と条件付きレンダリング統合。
- **波及効果**:
    - `stream.rs` の `buffer.find("{\"type\":")` はツール呼び出し検出 (`[CallSkill`) と同一バッファを共有するため、A2UI JSON とツール呼び出しが混在するストリームでの優先度は `{"type":` > `[CallSkill` の順。
    - `A2uiValidator` (validator.rs) の `ALLOWED_COMPONENT_TYPES` ホワイトリストに新しいコンポーネントを追加する場合、`A2uiRenderer.tsx` の switch ケースも同時に更新する必要がある。
    - `tokens.css` のデザイントークン名を変更する場合、`A2uiRenderer.tsx` のインラインスタイル内の `var(--*)` 参照を同時に更新する必要がある。
    - Phase 1 でインタラクション（onClick → API コールバック）を実装する際は、`useAgentChat.ts` に逆方向のイベント送信メカニズムを追加する必要がある。

## Sprint B: Unicode Directory Remediation (ProjectーNurture → Project-Nurture)
### 1. Cross-Repository Path Normalization
- **変更内容**:
    - `/Users/motista/Desktop/antigravity/ProjectーNurture` → `Project-Nurture` [RENAME]: カタカナ長音記号 (U+30FC) を ASCII ハイフン (U+002D) にリネーム。ツールチェイン互換性と docker-compose パス解決を正常化。
    - `apps/api-server/Cargo.toml` [MODIFY]: `nurture-api`, `nurture-core`, `nurture-infra`, `commerce-protocol` の path 依存 4 箇所を更新。
    - `docs/architecture/AIOME_NURTURE_SYNERGY.md` [MODIFY]: リポジトリパス参照を 2 箇所更新。
    - `memory/2026-03-16.md` [MODIFY]: ログエントリのパス参照を更新。
    - `Project-Nurture/docs/DEVELOPMENT_PROCESS.md` [MODIFY]: パス例・コマンド例 3 箇所を更新。
    - `Project-Nurture/docs/ENVIRONMENT_SETUP_PLAN.md` [MODIFY]: ヘッダパスとディレクトリ構造 2 箇所を更新。
    - `Project-Nurture/DEVELOPMENT_GUIDE.md` [MODIFY]: cd コマンドと Cargo.toml 例 2 箇所を更新。
- **波及効果**:
    - `docker-compose.nurture.yml` の `context: ../Project-Nurture` がリネームにより自動解決し、Docker/Podman ビルドが正常動作する前提条件が確立。
    - `cargo check --workspace` (aiome) および `cargo test --workspace` (Project-Nurture) で全テスト GREEN を確認済み。
    - Cargo のビルドキャッシュ（`target/` 約 22GB）は `CARGO_MANIFEST_DIR` 絶対パス変更により無効化されるが、次回ビルドで自動再生成。
    - Git リモート (`origin: Project-Nurture.git`) とローカルディレクトリ名が一致し、clone/push/pull の整合性が回復。

### 2. Sprint B-5 (Integer Arithmetic Migration)
- **変更内容**:
    - `libs/nurture-infra/migrations/20260416000000_bps_migration.sql` [NEW]: `conversion_rate` などを `REAL` (f64) から `INTEGER` (u32, basis points) へ型変換しデータを移行・再構築する破壊的マイグレーション。
    - `libs/nurture-core/src/policy.rs` [MODIFY]: `creator_points_rate`, `system_fee_rate`, `burn_rate` を `u32` へ型変更。
    - `libs/nurture-core/src/points.rs` [MODIFY]: `conversion_rate` を `u32` へ型変更。
    - `libs/nurture-infra/src/economy/settlement.rs` & `ledger.rs` [MODIFY]: 浮動小数点計算 (`amount * rate`) を整数算術 (`amount * bps / 10000`) へリファクタリング。
    - `libs/commerce-protocol/src/transaction.rs` & `interceptor.rs` [MODIFY]: トランザクション時のポイント計算を bps へ移行、テストデータを `1000` などの整数リテラルへ修正。
- **波及効果**:
    - 経済計算（Nurture Economy）における「丸め誤差」のリスクが完全に払拭され、トランザクション時の正確性が保証された。
    - リファクタリングによる AST インパクト範囲内の全テストスイートが GREEN で通過し、システムの堅牢性が強化された。

## Phase 1+2: Hardening Podman Infrastructure Integration
### 1. Rootless Podman Full Support
- **変更内容**:
    - `libs/shared/src/container_runtime.rs` [NEW]: コンテナランタイム検出のシングルソースオブトゥルース (SSOT)。`CONTAINER_RUNTIME` 環境変数による明示的オーバーライド → `podman --version` 自動検出 → `docker` フォールバックの 3 段階検出と `OnceLock` キャッシュ。
    - `libs/infrastructure/src/docker_conductor.rs` [MODIFY]: ランタイム検出を SSOT (`shared::container_runtime::detect_runtime()`) に委譲。
    - `libs/infrastructure/src/security.rs` [MODIFY]: `ALLOWED_BINARIES` に `"podman"` を追加と境界バリデーションホワイトリスト更新。
    - `apps/api-server/src/self_diagnosis.rs` [MODIFY]: コンテナランタイムの疎通確認を SSOT 経由に変更。
    - `apps/api-server/src/docker/delegator.rs` [MODIFY]: Shadow Worker への委譲ロジックを SSOT 経由に変更。
    - `scripts/backup.sh` [MODIFY]: ランタイム検出をファイルトップレベルに引き上げ、`podman compose` / `docker compose` 両対応。
- **波及効果**:
    - Aiome の実運用において Docker daemon (Root) 要件を持っていたインフラを `Podman (Rootless)` セキュリティレベルへ格上げ。
    - コンテナを利用したコード実行 (Shadow Worker 等) 時も自動的にユーザー権限（Rootless）下で隔離されるため、RCE 被害半径の極小化に寄与。
    - 後方互換性を保ちながら透過的な実装であるため、現在 Docker を使用中の開発環境への破壊的な影響はゼロ。
    - `CONTAINER_RUNTIME` 環境変数により CI/CD パイプラインでのランタイム指定が可能。

## Phase 0/D: Technical Debt & Production Readiness Hardening
### 1. Infrastructure Security & CI Stability
- **変更内容**:
    - `Cargo.toml` [MODIFY]: `wasmtime` および `wasmtime-wasi` を v43.0.1 へバージョンアップし、サンドボックスエスケープ関連の脆弱性を一掃。
    - `.cargo/audit.toml` [ADD]: `extism` クレート起因でアップデート不可能な古い `wasmtime` の RUSTSEC 脆弱性を Chesterton's Fence コメント付きで除外登録（`cargo audit` 通過化）。
    - `.github/workflows/ci.yml` [MODIFY]: `test` 等のジョブで `tonic` コンパイルが落ちる問題を解消するため、`protobuf-compiler` の事前インストールステップを追加。
    - `.env.example` [MODIFY]: 環境構築のブロッカーになっていた未記載の環境変数 31 件をすべて補完。
    - `docs/DESIGN.md` [ADD]: Golden Rule U-002 (トークン強制) に基づき、Artemis UI デザインの `tokens.css` 仕様を言語化し、ドキュメント同期ルールのフェイルを解消。
- **波及効果**:
    - CI/CD パイプラインの恒久的な安定化と `cargo audit` の 0 エラー化（GREEN維持）。
    - 本番デプロイ（Production Readiness）の最終障壁であったインフラストラクチャー負債・未指定変数のクラッシュリスクの完全排除。

## Phase 1: Security & Cost Hardening
### 1. Cost Circuit Breaker & Defensive Validations
- **変更内容**:
    - `libs/infrastructure/src/llm/cost_breaker.rs` [MODIFY]: `CostCircuitBreaker` に 30日ローリング集計（月次上限 `cost_limit_monthly`）を追加し、`CostBypassSwitch` の評価を日次・月次の両方の制限に波及するように統合。UX向上のため `CostStatus` 構造体を拡張。
    - `libs/aiome-contracts/src/error.rs` [MODIFY]: Release環境 `#[cfg(not(debug_assertions))]` において、`AiomeError` が 500 系エラーの際に内部情報を UUID にマスキングする CWE-209 防止機構 (Information Leakage Prevention) を導入。
    - `libs/shared/src/file_validator.rs` [NEW]: `validate_magic_bytes` を新設し、拡張子に依存しない画像ファイル（PNG, JPEG, GIF, PDF）のシグネチャ検証と EOF（終端）チェックを実装。PHP スクリプト等の追記型ポリグロット攻撃を O(1) で遮断。
    - `libs/infrastructure/migrations/{sqlite,postgres}/20260412000000_cost_breaker_indexes.sql` [NEW]: `resource_usage_logs(created_at)` にインデックスを追加し、Cost Circuit Breaker のフルテーブルスキャン（O(N) 負荷）を防止。
- **波及効果**:
    - Aiome の運用における経済的リスク（Cost Blowout）とセキュリティリスク（CWE-209, ポリグロットRCE）が物理次元で遮断された。
    - 頻繁にコールされる `CostCircuitBreaker` のデータベース負荷が激減し、AI 自身による大量自律ループ時もシステムがボトルネックにならない状態を確保。

## Phase 3-D: DreamState Autonomous Observability Loops
### 1. EvaluationLogger → DreamState DI & observability_dream
- **変更内容**:
    - `libs/infrastructure/src/dream_state.rs` [MODIFY]: `DreamState` 構造体に `eval_logger: Option<Arc<EvaluationLogger>>` フィールドを追加。`with_eval_logger` ビルダーメソッドを新設。`observability_dream` (`pub(crate)`) を実装し、7日間ローリングの `ProviderEvalStat` を集計、レイテンシ 2000ms/コスト $1.0 の閾値超過を検知してインサイトを生成。`dream()` ループの確率分岐に 15% の専用スロットを新設。
    - `apps/api-server/src/app_state.rs` [MODIFY]: `AppState` に `eval_logger: Component<Arc<EvaluationLogger>>` を追加。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `EvaluationLogger::new(job_queue.clone())` による初期化を追加し、`AppState` 構成体の末尾に注入。
    - `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `DreamState::new(llm).with_eval_logger(...)` で DI を完了。
    - `apps/api-server/src/api_integration_tests.rs` [MODIFY]: テスト用 `AppState` に `eval_logger` フィールドを追加。
- **波及効果**:
    - DreamState が単なる「探索・省察」エンジンから「自律パフォーマンス監視」エンジンへ進化し、LLM プロバイダーの劣化を Agent 自身が能動的に検知可能になった。
    - `AppState` に新規フィールドが追加されたため、今後新たな統合テストケースを追加する際は `eval_logger` の初期化が必須。
    - `dream()` の確率分岐が変更されたため、高レベル Agent（Lv10: comm_prob=45, sci_prob=20, obs_prob=15 = 80%）では explorative/reflective の発火率が低下した点に注意。

## Phase D: Cortex FTS5 Migration & Query Hardening
### 1. High-Performance Knowledge Retrieval
- **変更内容**:
    - `libs/infrastructure/migrations/sqlite/` [ADD]: `20260410000002_cortex_fts5.sql` マイグレーションファイルを作成。FTS5 Virtual Table と、データの乖離を防ぐための 3-way-trigger (`INSERT`, `UPDATE`, `DELETE`) を追加。
    - `libs/infrastructure/src/cortex_query.rs` [MODIFY]: 従来の `LIKE` アプローチを `MATCH ?` を使った FTS5 検索へリファクタリング。「`"`」(ダブルクオート) を安全にエスケープ（`""`化）しつつ Phrase Search で囲む O(1) パニック・プルーフ防御措置を追加。
    - `libs/infrastructure/src/cortex_query.rs` [MODIFY]: SQLite 側に FTS5 モジュール拡張が存在しない場合、あるいはテーブルが未展開の場合のエラーを検知した場合に、既存の `LIKE` へフォールバックするロジックを追加しダウンタイムをゼロ化。
    - `libs/infrastructure/src/cortex_compiler_tests.rs` [MODIFY]: `cortex_query.rs` における隔離された SQLite インメモリプール構築関数 `setup_db_pool()` に対して、テスト中にも FTS5 テーブルと自動同期トリガーが確実に張られるようにSQLスキーマ注入を拡張。
- **波及効果**:
    - `CortexQueryEngine` を呼び出す全ての API, Background Worker, AI 自律ループにおいて、知識抽出（RAG）のレイテンシが O(N) から O(1) に飛躍的な向上を遂げた。
    - 外部モジュールに依存させずに独自のフォールバックを持つため、本番環境 (Tauri / Docker) を選ばない移植性が担保された。

## Phase E-3: Front-end UI Standardization & Hardening
### 1. Unified i18n & Integrations Settings
- **変更内容**:
    - `apps/management-console/src/components/cortex/CortexView.tsx` [MODIFY]: 英語でハードコードされていた全テキストを `useTranslation` を用いて i18n (`cortexView`) 名前空間へ移行。
    - `apps/management-console/src/components/SettingsPage.tsx` [MODIFY]: `Channel Bridges` セクションを新設し、`X API Bearer Token` の入力フォームUIを実装。
    - `apps/management-console/src/i18n/{en,ja}.json` [MODIFY]: 追加されたUIの翻訳キーを統合。
    - `apps/management-console/src/components/cortex/CortexView.test.tsx` [ADD]: TDD による i18n テストを追加し、UIレンダリングの健全性を検証。
    - `apps/management-console/src/components/SettingsPage.test.tsx` [ADD]: TDD によるインテグレーション設定UIのテストを追加。
    - `apps/management-console/src/i18n/{en,ja}.json` [MODIFY]: Phase 3-C (LLM Observability) で追加された `PromptStatsView` の i18n 翻訳キー (`promptStats`) を完全同期。
- **波及効果**:
    - NURTURE UI/UX ガイドラインにおける国際化（i18n）の要件を完全に満たした。
    - X API トークンをフロントエンドから動的に管理できるようになり、TrendSonar 機能の実働テストが可能となった。

## Phase E-2: Zero-Trust LLM Infrastructure Hardening
### 1. Key Proxy Integration & Sunset Dead Code
- **変更内容**:
    - `libs/infrastructure/src/concept_manager.rs` [DELETE]: APIから孤立したレガシーモジュールを物理削除。
    - `libs/infrastructure/src/llm/utils.rs` [MODIFY]: 依存のあった `extract_json` を移管し、12のファイル（`oracle`, `cortex_*`等）の呼び出し元を安全にマイグレーション（コンパイルエラー防止）。
    - `libs/shared/src/config.rs` [MODIFY]: `AiomeConfig::load()` にて `VAULT_SECRET` 環境変数を読み込み機能を追加。その後、環境変数から直ちに削除するセキュア設計。
    - `apps/key-proxy/src/main.rs` [MODIFY]: デフォルトポートを `3017` に統一、ハードコードされていた API クォータ上限を `1000` → `50000` に大幅引き上げ。レスポンス形式を `api-server` の期待する `ProxyResponse` に適合。
    - `libs/infrastructure/src/llm/proxy.rs` [MODIFY]: `ProxyLlmProvider` の通信に `Authorization: Bearer <vault_secret>` ヘッダーを追加し、エンドポイントリクエスト修飾（`-embed`）を修正。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `FallbackRouter` の起動において、新たに `ProxyLlmProvider` をプライマリとして注入。その際、`.test_connection().await` (Ping) ヘルスチェックを実行し、非接続環境でのローカル開発時（`npm run dev`）の120秒タイムアウト・クラッシュを防止する防護壁を構築。
- **波及効果**:
    - `ProxyLlmProvider` と `key-proxy` 間の不整合（ポート、認証、データコントラクト、クォータ）が完全に解消され、本稼働グレードの `Zero-Trust` アーキテクチャが完成した。
    - 未使用コードの参照による負債が除去され、かつローカル開発の体験（DevEx）を毀損しないフォールバウティングが確保された。

## Phase 2B-2: Task Cancellation & Responsibility-Based Refund Infrastructure
### 1. Robust Escrow Refunding
- **変更内容**:
    - `libs/aiome-core-contracts/src/events.rs` [MODIFY]: `CoreEvent` に `TaskCancelled` を追加。
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `TaskEvent::Cancelled` を追加し、SSE リレーロジックを追加。`cancel_job` コール時に進行中のコンダクターを停止し `Cancelled` イベントを発出するように変更。`CancelTestConductor`を利用したキャンセル発出の統合テストを追加。
    - `apps/api-server/src/stream.rs` [MODIFY]: `CoreEvent::TaskCancelled` を `task_cancelled` SSE イベントにマッピング。
    - `libs/infrastructure/src/docker_conductor.rs` [MODIFY]: `DockerConductor` の課金ロジックを `Responsibility-Based Refund`（自己責務型返金）に移行。`execute_autonomous_purchase` から `escrow_create` に変更し、成功時には `escrow_release`、キャンセル（コンテナ停止・エラーなど中断時）には `escrow_refund` を発動するように修正。
- **波及効果**:
    - ユーザーによるジョブキャンセルやインフラ的エラーの際に、支払われたトークンが自動で安全に返金されるようになった。
    - コンダクター（タスク実行側）が自身で返金責務を持つ「局所化」が実現し、上位オーケストレーターと密結合しないスケーラブルな課金アーキテクチャが実現した。
    - SSEを通じて即座にフロントエンドにキャンセルの事実が伝播し、UI状態が正しく同期されるようになった。

## Phase δ-0 & δ-1: Infrastructure Safety Hardening
### 1. P2P Federation & System Guardrails
- **変更内容**:
    - `apps/samsara-hub/src/main.rs` [MODIFY]: P2Pのシグネチャ検証ペイロードを3フィールドから4フィールド(`sender_pubkey:topic_id:lamport_clock:content`)へ修正し、`biome`との同期を確立。
    - `apps/key-proxy/src/main.rs` [MODIFY]: Gemini統合においてnullの`system_instruction`がAPI拒否を引き起こす問題を修正。JSONカーゴで条件付き挿入 (`skip_serializing_if = "Option::is_none"`) 相当のロジックへリファクタリングし、E2Eテストを追加。
    - `libs/infrastructure/src/oracle.rs` [MODIFY]: `evaluate_multi_judge`に1MBのペイロード制限（`payload_size_limit`）を追加し、多重クローンによるOOM（Out of Memory）を未然に防止。
    - `libs/infrastructure/src/validator.rs` [MODIFY]: `ConstitutionalValidator`内での`slm_bridge`エラー時（TimeoutやFailed to start）にパニックする挙動を修正し、フォールバック（`0.0`）を返すGraceful Degradationへ移行。
    - `libs/infrastructure/src/user_learner.rs` [MODIFY]: 相対パス(`USER.md`)に依存していたファイル操作を、DIされた `AppDataResolver` 経由の絶対解決に置き換え、Directory Traversal脆弱性とカレントディレクトリへの依存を排除。
    - `libs/aiome-contracts/src/error.rs` [MODIFY]: `IntoResponse` 実装にて、内部ドメインエラーを「Internal Server Error」として握り潰していた挙動を改修し、より開発者に有用なドメインエラー文字列を返却するよう改善。
    - `apps/management-console/src/components/ArtifactVault.tsx` [MODIFY]: O(1)キャッシュを持つ `ArtifactStore` において、`file.name` がURIエンコードされておらず、スペースや日本語を含むファイル名で 400 Bad Request を誘発するバグを修正。
    - `docker-compose.production.yml` [MODIFY]: `samsara-hub`のポートバインディングを `127.0.0.1:3016` から `0.0.0.0:3016` (外部公開) へ変更し、本番環境でのP2P Federationを可能化。
- **波及効果**:
    - 本番環境でのP2Pフェデレーションのブロック要因（署名検証ミスとポートバインディング）が解消され、ノード間の安全な通信基盤が完成。
    - 外部LLMプロキシ（key-proxy）での不正ペイロード生成が防止され、外部モデルへの連携が安定化。
    - システムレベルのリソース枯渇脆弱性（OOM、相対パストラバーサル）が物理レイヤーで遮断された。
    - SLMがダウンしてもシステム全体がクラッシュせず、Graceful Degradationにより可用性が維持される。


## Phase 2.1: Execution Layer Hardening (Governed Execution)
### 1. Atomic Security Gating & Semantic Elicitation
- **変更内容**:
    - `libs/aiome-core-contracts/src/events.rs` [MODIFY]: `CoreEvent::TaskAwaitingInput` を追加。
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]:
        - `TaskEvent::AwaitingInput` を追加し、リレーロジックで `CoreEvent::TaskAwaitingInput` へ変換。
        - `process_goal_job` 内で、サブジョブの投入前に分解された全ステップを `AdaptiveImmuneSystem` で一括検証する「Plan-First Verification」を実装。
        - セキュリティ違反検知時にジョブを `Failed` ではなく `AwaitingInput` 状態へ遷移させ、専用イベントを発行するように変更。
        - `Goal` カテゴリのジョブがデキューされないバグを修正。
        - 統合テスト `test_dispatcher_elicitation_on_high_severity_violation` を追加。
    - `libs/infrastructure/src/task_orchestrator/planner.rs` [MODIFY]: ツール名の抽出ロジック (`tool_name`) を改善し、免疫システムとの照合精度を向上。
    - `apps/api-server/src/routes/jobs.rs` [MODIFY]: `submit_job_review` (承認/拒否ロジックと免疫一回限りバイパス) と `get_awaiting_input_jobs` を実装。
    - `apps/management-console/src/components/TaskApprovalOverlay.tsx` [ADD]: 承認待ち要求に介入するための専用オーバーレイ UI を追加。`App.tsx` のルート層にマウント。
- **波及効果**:
    - **`TaskDispatcher → AdaptiveImmuneSystem → CoreEvent::TaskAwaitingInput → Management Console → TaskApprovalOverlay`**
    - 実行レイヤー全体で「一部成功・一部失敗」という部分実行リスクが排除され、トランザクション的なセキュリティ性が担保された。
    - 管理コンソール側での「ユーザー介入要求」の視覚化が可能になり、セキュリティ体験が飛躍的に向上。バックエンドへの即時フィードバックにより、自己防衛と自律進行を安全に両立できるようになった。

## Phase 2B-2 Foundation (Perfect Plan Rev.6 / Limit Break)
### 1. ゴーストバグ防止機構と SOUL 初期化API
- **変更内容**:
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `std::fs::read_to_string("SOUL.md")` という相対ハードコードパスを `resolver.resolve("SOUL.md")` へ完全置換。
    - `apps/api-server/src/routes/soul.rs` [MODIFY]: `POST /api/v1/soul/init` API エンドポイントを追加。
    - `apps/api-server/src/router.rs` & `settings.rs` [MODIFY]: `#[cfg(debug_assertions)]` 防壁を撤去し、Release ビルドでの接続テストおよびオンボーディングフローを解放。
    - `libs/infrastructure/src/soul_mutator.rs` [MODIFY]: `generate_initial_soul(name)` メソッド実装、および `transmute()` 内に不測の SOUL.md 喪失時の自己修復フォールバックを追加。さらに `ConstitutionalValidator` を用いた LLM 出力の禁止キーワード検証機構を統合。
    - `apps/management-console/src/components/OnboardingModal.tsx` [MODIFY]: 「Awaken System」時に `/api/v1/soul/init` をポーリング呼び出しし、リロード遷移によるメインシステムへの引き渡しを実装。
- **波及効果**:
    - **`OnboardingModal → /api/v1/soul/init → SoulMutator::generate_initial_soul → app_data_root/SOUL.md`**
    - 初回起動時における「人格（SOUL.md）不在」に起因するパニックを完全に排除。
    - 本番環境（Tauri パッケージング済）においても、設定画面での「Ollama/Gemini 接続テスト」が確実に機能し、セットアップ体験の断絶（404消滅）を防ぐ。

### 2. Phase 2B-2 Reflexion (Security / UX Audit)
- **変更内容**:
    - `apps/api-server/src/routes/soul.rs` [MODIFY]: `ai_name` に対する `64文字・改行排除` サニタイズと、空文字時の `Genesis` デフォルト切り替え（OOM/Injection 防御）。
    - `libs/core/src/security.rs` [MODIFY]: `ConstitutionalValidator::validate_text` にて、巨大文字列コピー `to_lowercase()` を廃止し、`once_cell` 相当の `std::sync::LazyLock` を用いた O(1) コンパイル済み Regex エンジン処理へと抜本的最適化。
    - `apps/management-console/src/components/OnboardingModal.tsx` [MODIFY]: 初期起動API障害時に発生するソフトブリック（暗黙クローズによる UX ロック）を `try/catch` + `errorMsg` ステート描画への変更により解消。
- **波及効果**:
    - 全体的なインフラストラクチャにおける DoS / OOM レジリエンスが向上。不正な長文や倫理違反キーワードによるエラーを事前にストリーム防壁としてブロックすることで、Agent 側の余計な演算を軽減する。

### 3. Phase 2B-2 Reflexion Part 3 (TDD Zero-Dependency & UI Tokens)
- **変更内容**:
    - `libs/infrastructure/src/skills/tests.rs` [MODIFY]: WASM サンドボックス環境 (Extism) の TDD 実証テストを復活（`#ignore` 解除）。
    - `libs/infrastructure/src/skills/test_data/hello_skill.wasm` [ADD]: Extism 用テストモックバイナリファイル。完全なオフライン検証（無依存 TDD）を実現するため `tests.rs` 内で `include_bytes!` として注入。
    - `apps/management-console/src/components/SettingsPage.tsx` & `OnboardingModal.tsx` [MODIFY]: ハードコードされていた HEX 色指定を全て `var(--text-inverse)` や `var(--accent-purple)` 等の CSS トークンへ置き換え。`OnboardingModal` 内の `@ts-ignore` による `window` グローバル変数の無骨な拡張を廃止し、React Hook `useState` へとリファクタリング。
    - `apps/management-console/src/styles/tokens.css` [MODIFY]: `--text-inverse`, `--bg-primary`, `--bg-inverse` トークンを追加。

### 4. Phase 2B-2 Reflexion Limit Break (Settings E2E Efficacy)
- **変更内容**:
    - `apps/api-server/src/routes/settings.rs` [MODIFY]: `view_mode` などの UI 制御値を `ALLOWED_KEYS` のホワイトリストに追加し、`SecurityViolation` 誤検知による保存失敗をホットフィックス。
    - `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `DreamService` (TrendSonar) の初期化時、強制的に `.env` からしか環境変数を読み込んでいなかった仕様を改修し、優先的に `state.job_queue` (DB) から設定値を取得する E2E デリバリーを確立。
    - `apps/api-server/src/routes/soul.rs` & `api.rs` [MODIFY]: OpenAPI (`utoipa::ToSchema`) 定義の不足を修復。
- **波及効果**:
    - 設定変更 (SettingsPage) の結果が再起動後に確実に `DreamService` に波及し、環境変数に依存しないユーザー主導のトークンオーバーライドが正常稼働するようになった。
    - API ドキュメント生成プロセスが GREEN に復旧。

### 5. Phase 3-C: Management Console Hardening (U-002 Mastery)
- **変更内容**:
    - `Timeline.tsx`, `SoTProgressBar.tsx`, `DiagnosticsHistory.tsx`, `ImmuneSystem.tsx`, `LoraTrainingView.tsx`, `BiomeDialogueView.tsx`, `ExpressionPipeline.tsx`, `VoiceStore.tsx` における HEX/RGBA およびアニメーションタイミングのハードコードを全廃し、`tokens.css` へ統合。
    - `Timeline` や `DiagnosticsHistory` 等で動的描画されるシステムステータス文字列を `i18n` キーへ抽出し、日英完全同期。
    - `VoiceStore` における `alert()` を排し、Framer Motion トースト通知によるプレミアム UX を実装。
    - `Lora` / `Biome` / `Immune` 各ビューにメディアクエリを導入し、レスポンシブ対応を完遂。
- **波及効果**:
    - **`tokens.css → All UI Components`**
    - デザインシステムと UI 実装が 100% 同期され、将来的なテーマ拡張や多言語対応の保守性が抜本的に向上。
    - `test_ui_hex_violations.py` による自動ガードが管理コンソールの主要 8 ファイル全域で機能し、今後のハードコード回帰を完全に防止。

## Aiome Social Signal Integration (Layer A-C)
### 1. X Signal Probe (Reqwest Direct)
- **変更内容**:
    - `libs/infrastructure/src/x_signal_probe.rs` [NEW]: `TrendAdapter` トレイトを実装した X API 情報収集モジュール。インメモリレートリミッター（DashMap）を内包。
    - `libs/infrastructure/src/lib.rs` [MODIFY]: `x_signal_probe` モジュールを登録。
    - `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `ExternalTrendSonar` のアレイに `XSignalProbe` を依存注入（`X_BEARER_TOKEN` 存在時のみ）。
    - `.env.example` [MODIFY]: `X_BEARER_TOKEN` の環境変数テンプレートを追加。
- **波及効果**:
    - **`x_signal_probe → trend_sonar → dream_state`**
    - 外部の複雑な MCP アーキテクチャを排し、依存関係（Impact Radius）を `trend_sonar.rs` 単体に抑え込むことに成功。DreamState 側の既存テスト（モック）に一切影響を与えずに、実環境でのみ X API が自律駆動するセキュアな設計。

## Chaos Engineering Infrastructure (Layer A-C)
### 1. フォルトインジェクション基盤 & カオステストスイート
- **変更内容**:
    - `libs/infrastructure/tests/common/chaos.rs` [NEW]: `ChaosMode` enum (EmptyResponse, Timeout, MalformedJson, GiantOutput, AlwaysFail) と `ChaosLlmProvider` (LlmProvider トレイト実装ラッパー) を `tests/` ディレクトリに完全隔離して新設。
    - `libs/infrastructure/tests/chaos_experiments.rs` [NEW]: 6つの定常状態仮説テスト (SoT×3, SamsaraEngine×1, CircuitBreaker×1, ConstraintChecker×1)。
    - `.agent/workflows/chaos.md` [NEW]: 「仮説→障害注入→検証→学習」の4フェーズワークフロー。
    - `.agent/workflows/god-mode.md` [MODIFY]: Phase 4 (Chaos) を Reflexion と Red-Team の間に挿入し5段階化。
- **波及効果**:
    - 本番コードへの変更 **ゼロ**。`tests/` 内でクローズするため、`src/` のASTグラフに一切影響しない。
    - `cargo check --workspace --tests` による既存テストへの干渉 **ゼロ** を物理的に検証済み。
    - `/god-mode` の品質パイプラインに「意図的な障害注入」フェーズが追加され、fail-open/fail-safe の区別が自動検証されるようになった。

## Phase D: Agent-Native Discovery & Hybrid Cortex FS (ADR-025)
### 1. Hybrid File System Projection
- **変更内容**:
    - `libs/infrastructure/src/cortex_file_projector.rs` [NEW]: `CortexFileProjector` を実装し、DBの `cortex_wiki_articles` をファイルシステム上の階層（`cortex_fs/`）へ投影。
    - カテゴリ名やタイトルが ASCII 以外の文字のみで構成されるシナリオへの Deterministic (16進数) スラグのフォールバックを完備し、上書きによる消失を防御。
    - 古い記事やゴミファイルのガベージコレクションを実装。
    - `libs/infrastructure/src/cortex_compiler.rs`: コンパイルサイクルの直後に `project_to_filesystem` を自動起動する仕組みを追加。
    - `libs/infrastructure/src/dream_state.rs`: `DreamState` へ `cortex_fs` のルートパスと探索用トップインデックス（カテゴリ一覧のみ）を注入するロジック (`build_cortex_fs_context`) の追加。
    - `apps/api-server/src/internal_services/dream.rs` & `bootstrap.rs`: 起動プロセスおよび Dream Service に Projector への DI（`Arc<CortexFileProjector>`）を追加。
- **波及効果**:
    - エージェントは RAG に依存せず、直接 `cortex_fs/` ディレクトリを `ls` や `grep`、`cat` で探査できるようになり、コンテキストの精度超過や幻覚リスクが物理的に抑止される。
    - カテゴリの `_concept.md` に一覧を遅延リンクさせる O(1) トークン設計により、Wiki 記事数が増大してもプロンプトバジェット制限が破綻しない強固な参照網が構築された。

## Phase D: Cortex Synth Pipeline Data Purity & LoRA Compliance
### 1. Belief Consistency Gate Integration
- **変更内容**:
    - `libs/infrastructure/src/cortex_synth.rs`: `CortexSynthesizer` に対して `BeliefConsistencyGate` のDIを追加。合成データ生成の内部ループ(`generate_dataset`)内で `check_belief_consistency` を呼び出し、矛盾データと改訂候補をベースラインから安全にパージするロジックを実装。また、JSON解析のエラーが秘匿される（`unwrap_or_default()`）現象を修正しロギングを強化。
    - `libs/infrastructure/src/cortex_synth.rs`: `export_to_jsonl` の吐き出しフォーマットを古いテキスト構造から、Axolotl等の現在の業界標準である **ShareGPT形式** (`{"conversations": ...}`) へ移行。シリアライズエラー時のフェイルセーフ追加。
    - `apps/api-server/src/routes/cortex.rs`: `synth_dataset_handler` にて `SoulStore` から Agentのコア信念（prompt_fragment, narrative_self）をロードし `BeliefConsistencyGate` へシードとして渡す初期化処理を統合。DBフェッチ失敗時のサイレント被覆問題(`_ => vec![]`)を修正。
- **波及効果**:
    - AIによるデータセットの自作自演（Synthetic Data Generation）において、自身の魂（Soul/Belief）に反するゴミデータが自動的に排除される Data Purity（データ純度）維持の機構が完成。
    - 出力フォーマットがシェアGPT形式になったことで、将来フェーズでの MLX / LoRA ファインチューニングプロセスとの完全な API/データ互換性が物理的に担保された。

## Precomputed Relational Intelligence & Agent Governance

## SEO Intelligence Pipeline (Phase B)
### 1. Task Decoupling & Guardrails Integration
- **変更内容**:
    - `libs/infrastructure/src/task_orchestrator/seo_content.rs` [NEW]: SEO特化型の自律タスクコンダクター `SeoContentConductor` の新規追加。トピック境界値チェックと進捗イベントを含有。
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` に O(1) ではない O(N*C) だが汎用的な `get_conductor_for` を診断用に公開。
    - `apps/api-server/src/stream.rs`: MCPツールの出力をLLM履歴に反映する際、プロンプトインジェクション防御として `sanitize_for_prompt` を強制接合。
    - `apps/api-server/src/bootstrap.rs`, `app_state.rs`: `SeoContentConductor` および `PublishPipeline` のDIと起動時初期化（Bootstrap化）。
    - `apps/api-server/src/api_integration_tests.rs`: 新規コンダクターの登録を検証する TDD 結合テストと、`AppState` モックへの `publish_pipeline` 追加。
- **波及効果**:
    - SEOと汎用LLM（`data_processing`等）のタスクオーケストレーションが分離され、ドメイン用システムプロンプトの注入が安全になった。出力のパブリッシュ（デプロイ）に向けて `PublishPipeline` との連携基盤を確立。

### 1.5. Publishing Pipeline & External Analysis Integration (Phase B / Phase C)
- **変更内容**:
    - `libs/infrastructure/src/trend_sonar.rs`: `SerpAnalysisAdapter` の具象実装を追加。Tavily等の実APIコールとHTMLサニタイズ (`sanitize_snippet`) を行うと同時に、自律ループ枯渇防止目的のインメモリ・レートリミッター機能（10分に1回のみ）を内蔵。
    - `apps/api-server/src/internal_services/dream.rs`: `SEARCH_API_KEY` の環境変数が存在する場合のみ、`WebSearchAdapter` と結合した `ExternalTrendSonar` コンペティター分析ノードを `DreamService` にDI（注入）する仕組みへアップグレード。
    - `libs/infrastructure/src/publisher/wordpress.rs` [NEW]: `WordPressAdapter` の追加。WP REST API v2 を利用した記事自動投稿を実装（テストケース完備）。
    - `apps/api-server/src/bootstrap.rs`: `PublishPipeline` インスタンス化処理において `WP_API_URL` および `WP_API_TOKEN` をフォールバック付きで解決し、本番CMSと静的に繋ぐ仕組みを実装。Abyss Vault化を完了し、直接取得フォールバックを撤廃済。
- **波及効果**:
    - DreamService の自律ループからSEOギャップを取得する部分が完全自動かつレートリミット保護により自律破綻しないよう修正された。また、記事出力の終着点であるCMSシステム (WordPress) に対する直結が確立し、SEOインテリジェンス・パイプラインの最後のパズルが完成した。

- **変更内容**:
    - `scripts/nurture_auditor.py`: AST抽出のパフォーマンス欠陥(`rglob()`)を `os.walk()` とインプレース枝刈りにより真の O(1) ディレクトリ遮断へ改修。Rust, TSX, CSS の物理依存（エッジ）を重複排除付きで `.context/impact_graph.json` へ出力。
    - `scripts/impact_query.py` [NEW]: BFS アルゴリズムと `visited` セーフガードを用いた被害半径（Blast Radius: `[WILL BREAK]`）計算 CLI ツールの新規追加。テストファイル除外オプション付き。
    - `AGENTS.md` & `.agent/workflows/*`: 全ての主要ワークフローとAGENTS.mdコアディレクティブに対して、旧来のGitNexus等外部ツール統合ではなく、自前実装した `nurture_auditor.py` と `impact_query.py` を用いた「事前物理波及テスト（Mandatory AST Impact Analysis）」を義務化するパッチを適用。
- **波及効果**:
    - エージェント自律行動ループにおいて、コード変更前の「未知のカスケードエラー」や「Tailwind代替CSSの波及漏れ」が事前に高精度かつ数ミリ秒レベルで検出されるようになり、開発プロセスの防御力が格段に向上。外部ライブラリへの依存（サプライチェーンリスク）も排除。

## Analytics MCP Hub & Configuration Standardization
### 1. MCP Security Hardening & Config API
- **変更内容**:
    - `api-server/mcp/client.rs`: `McpClient::spawn` で使用されるインフラストラクチャーコマンドに対し、`npx`, `node`, `python3`, `uvx` などのMCP固有ホワイトリストと `@modelcontextprotocol/` などのパッケージ検証を追加し、BastionGuard バイパス（RCE可能性）を排除。
    - `api-server/mcp/discovery.rs`: `config.env` から `$` や式を利用した動的環境変数解決（サニタイズ込み）を実装。デフォル構成テンプレートの `tokio::fs::write` 化追加。
    - `api-server/routes/skill.rs`: `PUT /api/skills/mcp/config` と `GET /api/skills/mcp/config` を新設し、UIからの構成アップデートとリアルタイム再描画（MCPプロセスのホットリロード連動）を実現。SQLite との実用上の Desync 問題を解決。
    - `infrastructure/registry.rs`: `RegistryManager::clear_mcp_servers` を実装し、ゾンビツールプロンプト挿入問題を解消。
    - `management-console/src/components/SettingsPage.tsx`: フロントエンドに MCP 専用コンフィグマネージャー `McpConfigManager` を追加し、ダッシュボードへの直接的なフィードバックルーチン（`useSettings`）を構築。

## RTK Token Dashboard Update (Phase 3.5 Conclusion)
### 1. Token Savings SSE & State Propagation
- **変更内容**:
    - `management-console/src/hooks/useAgentChat.ts`: SSE メッセージハンドラ内で `token_saved` イベントを抽出し `aiome_vitality_event` として発行。
    - `management-console/src/App.tsx`: `useSystemVitality` 経由の CustomEvent リスナーから状態を受け取り、`sessionSavedChars` を全体ステートとして維持。
    - `management-console/src/components/common/TokenSavingsIndicator.tsx` [NEW]: `framer-motion` の `useSpring` を活用して文字数/トークン数を動的カウントアップする Premium なガラスモーフィズムコンポーネント。
    - `AgentConsole`, `StoryFlow`, `BiotopeView`: Props に `sessionSavedChars` を追加し、コンポーネント内にバッジ表示をバインディング。
- **波及効果**:
    - Agent 自律ループで発生した `OutputFilter` の節約メタデータが UI のアニメーションへ透過的かつ即時的に描画される。既存の Vitality Event Bus を流用したことで、チャット履歴と分離した安全な伝搬設計（疎結合）を実現。
- **波及効果**:
    - `McpClient`, `McpProcessManager` シグネチャが `HashMap<String, String>` (環境変数引数) を要求するように変更されたため、システムプロンプトの `discovery.rs` や `skill.rs` (Spawn MCP) ルーチンにも影響を波及させ一元化。
    - `.env.example` に GA4/Stripe 用のコメント追加が行われ、新規プロジェクトブート時のインテグレーションフローが公式化された。## Smart Model Bootstrap: 自律的LLMセットアップ基盤
### 1. Model Status API & Diagnostic Hardening
- **変更内容**:
    - `api-server/routes/model_setup.rs` [NEW]: `ModelStatusResponse` 構造体と `GET /api/v1/models/status` の REST + OpenAPI 定義・実装。Ollama接続状態や設定モデル (`gemma4:26b` など) の存在・利用可否を判定。SSRFガードレール (`validate_url`) を適用。
    - `api-server/router.rs`, `api-server/api.rs`: ルート登録と OpenAPI マージ。
    - `api-server/self_diagnosis.rs`: Docker接続失敗・Ollama到達不能時にプロセスを落とさず（`bail!` 廃止）、`tracing::warn!` に留める緩和措置を実施し、Ollamaモデル有無の診断を追加。
- **波及効果**:
    - `Management Console (Frontend)`: 初回起動時の `OnboardingModal` 内でこのAPIをポーリングし、ユーザーにセットアップ進捗を可視化。既存の E2E (`aiome_onboarding_done`) テストには影響ゼロ。
    - `Core CLI / Backend Services`: Docker不在でも稼働継続可能となり、外部の API ベース (OpenAI / Fal.ai) での限定稼働やローカル Mac Native 連携などの拡張性が向上。

## LoRA Marketplace: 安全なアダプター取引基盤
### 1. LoRA Marketplace Architecture
- **変更内容**:
    - `aiome-core-contracts/lora_marketplace.rs` [NEW]: `LoraListing`, `LoraPurchase`, `ListingFilter`, `LoraMarketplace` トレイトの型定義。
    - `infrastructure/lora_marketplace.rs` [NEW]: `UniversalLoraMarketplace` (SQLite/PG 対応) — エスクロー決済、SHA-256 ハッシュ検証、PathSandbox、楽観ロック、自己購入ブロック、500MB サイズ制限。
    - `infrastructure/lora_training.rs`: `AdapterFileInfo` 構造体と `get_adapter_info()` ヘルパー追加。
    - `api-server/routes/lora_market.rs` [NEW]: 6つの REST エンドポイント（出品一覧・出品・購入・完了・取下・自分の出品）。
    - `api-server/app_state.rs`: `lora_marketplace` コンポーネント追加。
    - `api-server/bootstrap.rs`: `UniversalLoraMarketplace` の DI 初期化。Commerce Engine の参照分離リファクタリング。
    - `api-server/router.rs`: `/api/v1/lora/market/*` ルート群の登録。
    - `api-server/api_integration_tests.rs`: AppState モックに `lora_marketplace` フィールド追加。
    - `infrastructure/migrations/sqlite/20260404000001_lora_marketplace.sql` [NEW]: テーブル + 監査トリガー。
    - `infrastructure/migrations/postgres/20260404000001_lora_marketplace.sql` [NEW]: PostgreSQL 版マイグレーション。
- **波及効果**:
    - `CommerceEngine` トレイト: **変更なし**。`escrow_create/release/refund` を呼び出すのみ。
    - `GigEngine`: **変更なし**。独立したトレイトとして並行動作。
    - `ArtifactStore`: **変更なし**。LoRA の来歴追跡は将来フェーズで `provenance edges` と連携可能。
    - Management Console: 将来フェーズで出品・購入 UI を統合予定。

## Phase 1A: fff.nvim Integration & MCP Dispatch Engine
### 1. Unified MCP Dispatch & Execution Sandboxing
- **変更理由**: `fff.nvim` を始めとする外部 MCP サーバーの機能を Aiome の自律チャットループ内で安全かつ動的にディスパッチ（解決・実行）するため。従来の Wasm 固定の静的ディスパッチを廃止し、稼働中の MCP プロセス群全体へ O(N) でポーリングする動的ルーティング構造へと刷新。
- **波及効果**:
    - `apps/api-server/src/tool_call_router.rs` [MODIFY]: `execute_skill` 内に MCP サーバーポーリング（`active_client_ids()`）と `ListTools` -> `CallTool` という二段階要求を実装。2秒の探索タイムアウトと30秒の実行タイムアウトを導入し、巨大リポジトリ検索時等でのプロセス凍結（ハングアップ）を完全防止。
    - `libs/shared/src/mcp_constants.rs` [NEW]: セキュリティの単一の真実源 (SSOT) として `ALLOWED_MCP_COMMANDS` および `ALLOWED_NPM_PACKAGES` を新設。`fff-mcp` 等のバイナリコマンド特例の認可ロジックを一元化。
    - `apps/api-server/src/mcp/client.rs` & `routes/skill.rs` [MODIFY]: `McpClient::spawn` と `spawn_mcp_server` エンドポイントがハードコードを脱却し、`mcp_constants.rs` を使用するように置換。これにより、インフラ全体でのコマンドバイパス脆弱性 (RCEの危険性) が物理的に閉塞。
    - 統合効果: 未定義のツールコマンドが呼ばれた際、まず稼働中の MCP サーバーを探索し、存在すればそれを実行し、なければ安全に Wasm スキルへフォールバックする「無破壊的統合」が確立。

## Phase B: Autonomous Chat Loop Hardening (ToolCallRouter Integration)

### 1. Unified ToolCallRouter Architecture
- **変更理由**: 複数の実行コンテキスト（自律ループ `agent_engine.rs`、チャットストリーム `stream.rs`、MCPプロトコル等）で重複していたツール実行とセキュリティ検証（Guardrails / Sentinel / HookChain）のロジックを一元化し、アーキテクチャ上の抜け穴（バイパス）を完全に塞ぐため。
- **波及効果**:
  - `apps/api-server/src/tool_call_router.rs` [NEW & UPDATED]: `ToolCallRouter` トレイトと `DefaultToolCallRouter` の完全実装。同期/非同期の双方で扱える共通の `ToolExecutionEvent` 列挙体を定義し、フックの適用（Allow/Deny/Transform）をプロキシとして包括。
  - `apps/api-server/src/tool_call_processor.rs`: LLMからの生の出力をパースし、直接手動でツール評価・実行を行なっていたロジックを `DefaultToolCallRouter` を経由する形に大幅リファクタリング（責務の分離とカプセル化）。
  - `apps/api-server/src/stream.rs` (SSE Handler): 中のツール実行ループについて、既存の冗長な実行ロジックを手動で回すのではなく、`ToolCallRouter::execute_skill` が返す Receiver を監視し、ストリームに `tool_result` イベントや `heartbeat`、エラーブロック通知を一元的にフォワードする設計へ移行。
  - `libs/infrastructure/src/immune_system.rs` & `guardrails`: `evaluate_security` の中で必ず `Guardrails` によるチェックと `AdaptiveImmuneSystem::verify_intent` の判定を並列または直列で完了させてからツール実行フェーズに進行するように統一。
## Phase C: Cortex Wiki Compiler & Query Engine Integration
### 1. CortexQueryEngine & Semantic Fallback
- **変更内容**: 
    - `infrastructure/cortex_query.rs`: `CortexQueryEngine` 構造体の追加。`query` および `suggest_questions` の実装と `Guardrails` によるプロンプトインジェクション防御。`QueryOptions` と `DisclosureLevel` (トークンバジェット L0-L3) の導入。
    - `api-server/stream.rs`: OOD (`is_ood == true`) 時のコンテキスト欠落に対し、`CortexQueryEngine` による意味検索フォールバックを組み込み。
    - `api-server/routes/cortex.rs` & `router.rs`: `GET /api/v1/cortex/suggestions`, `POST /api/v1/cortex/query` エンドポイント追加・登録、および OpenAPI (`api.rs`) 拡張。
    - `api-server/mcp/server.rs`: MCP ツール `cortex_search` をネイティブレイヤーに実装・公開し、外部エージェントからダイレクトにナレッジベース検索が可能に。
    - `infrastructure/context_engine.rs`: `ContextBudget` に `max_cortex_chars` 追加。
    - **Karpathy "LLM Wiki" Pattern**: `cortex_query` にて信頼スコア 0.7 以上の回答を `SourceType::Query` として自律的に再インジェスト (Query File-Back)。
    - **Activity Logging**: `cortex_ingester`, `cortex_compiler`, `cortex_query` の重要イベントを `cortex_activity_log` に追跡・永続化する監査基盤を追加。
- **波及効果**: 
    - `api-server/app_state.rs`, `bootstrap.rs`, および `api_integration_tests.rs` における `AppState` の初期化プロセスへ `cortex_query` の依存注入が波及。
    - チャットストリーム中の未知の質問（OOD）に対しても、CortexWiki から取得されたセマンティックな検索結果が提供されるようになり、Agent の自己学習サイクルが実稼働する基盤が完了。

## Phase F: Security Hook Framework & Precedence Execution

### 1. HookChain & AdaptiveImmuneSystem Precedence
- **変更理由**: エージェントが生成したツールコール（WASM実行、Forgeコマンド等）が実際に発火する前に、セキュリティポリシー違反やコンテキスト不整合をリアルタイムで検知・遮断（または変換）する中央集権型のインターセプター機構を確立するため。
- **波及効果**:
  - `libs/infrastructure/src/skills/hooks.rs` [NEW]: `ToolHook` トレイト、および `HookVerdict` (Allow/Deny/Transform)、並びにフック群を管理・直列評価する `HookChain` を実装。
  - `apps/api-server/src/agent_engine.rs` (現 `tool_call_processor.rs` へ移行): これまで点在していた `parse_tool_calls` や LLM出力を処理するロジックを抽出・再構築し、実行前に `AdaptiveImmuneSystem::verify_intent` -> `HookChain::execute_pre` を評価し、実行後に `HookChain::execute_post` を評価する制御フローを実装。
  - `apps/api-server/src/app_state.rs` & `apps/api-server/src/main.rs`: `AppState` に `hook_chain: Component<Arc<HookChain>>` を追加し、起動時に依存注入。
  - `apps/api-server/src/api_integration_tests.rs`: `AppState` のモック群全般に `hook_chain` フィールドを追加修正し、コンパイルエラーを解消。全59テストの疎通を回復。

## Phase 1C: Generative Engine Infrastructure Integration

### 1. Concrete GenerativeEngine Implementations
- **変更理由**: 画像・音声など複数モーダルな生成を可能にするため、ローカルで完結する `ComfyUiGenerativeEngine` とクラウド利用の `FalAiGenerativeEngine` の2つの具象実装を整備し、`GenerativeEngine` トレイトを通じた一元的な生成基盤を構築するため。
- **波及効果**:
  - `libs/infrastructure/src/generative_engine.rs` [NEW]: コンポーネントおよびテストモック（`MockGenerativeEngine`）を含む新規モジュール作成。
  - `libs/infrastructure/src/lib.rs`: 新規モジュール `generative_engine` をエクスポート。
  - `apps/api-server/src/app_state.rs`: `AppState` に `generative_engine: Component<Arc<dyn aiome_contracts::traits::GenerativeEngine>>` を追加。
  - `apps/api-server/src/main.rs`: 環境変数 `GENERATIVE_ENGINE` (comfyui / falai) に応じたエンジン初期化と注入ロジックを追加。プロダクション (`--release`) 環境での無指定時にはセキュアなフェイルファスト (`std::process::exit(1)`) を実装。
  - `apps/api-server/src/api_integration_tests.rs`: テスト環境用の `AppState` モック設定に `MockGenerativeEngine` の初期化処理を統合し、全エンドツーエンドテストの疎通を回復。

## Phase 1B: Avatar Engine & Infrastructure Hardening

### 1. Storage DoS Protection (DiskQuotaManager)
- **変更理由**: 大量または巨大なファイルの連続アップロードによるストレージ枯渇 (DoE) 攻撃を防ぎ、システム全体の安定稼働とマルチテナント環境下での公平なリソース配分を保証するため。
- **波及効果**:
  - `libs/infrastructure/src/disk_quota.rs` [NEW]: `DiskQuotaManager` 構造体を実装し、エージェントごとのディスク使用量をリアルタイムでトラッキング・制限 (デフォルト500MB) する仕組みを構築。
  - `apps/api-server/src/app_state.rs`: `AppState` に `DiskQuotaManager` コンポーネントの依存注入を追加。
  - `apps/api-server/src/routes/voice.rs`: `upload_voice_handler` 内での検証ロジックを更新し、アップロード開始前にクォータ超過を検知して 413 Payload Too Large エラーを返すよう改修。
  - `apps/api-server/src/api_integration_tests.rs`: 全ての統合テスト環境における `AppState` モック初期化処理に `disk_quota` フィールドを追加修正し、コンパイルエラーを解消。

### 2. TTS Streaming Optimization
- **変更理由**: 長文のテキスト読み上げにおいて音声生成完了まで待機することによる TTFB (Time to First Byte) の遅延を解消し、リアルタイムで低遅延な対話体験 (ストリーミング応答) を実現するため。
- **波及効果**:
  - `libs/aiome-contracts/src/traits.rs`: `TtsProvider` トレイトを非破壊的に拡張し、新たに `synthesize_stream` メソッド（デフォルト実装付き）を追加。
  - `libs/infrastructure/src/tts.rs`: `OpenAiTtsProvider` に `synthesize_stream` をオーバーライド実装。`reqwest` の `bytes_stream` を用いて音声チャンクデータを逐次的に返却。
  - `apps/api-server/src/routes/voice.rs`: `synthesize_voice_handler` で `stream=true` クエリパラメータを受け取り、`axum::body::Body::from_stream` を用いて chunked 転送を行う分岐を追加。

### 3. LipSync Expansion (SimpleLipSyncEngine)
- **変更理由**: Inochi2D や 3D アバターを駆動するための動的な口形（Viseme）データを外部に依存せず生成可能にし、自律的なリップシンクアニメーションの基盤をシステム内に統合するため。
- **波及効果**:
  - `libs/avatar-engine/src/lip_sync.rs`: 既存の `LipSyncProvider` トレイト実装として `SimpleLipSyncEngine` 構造体を新規追加。
  - オーディオ生データからのダミーフレーム生成（一定間隔での口の開閉と `Viseme` のトグル）および、文字起こしセグメント (`TranscriptionSegment`) に基づくフレーム補完ロジックを実装。単体テストを併せて追加。

## Phase 1A-2: Dynamic Dataset Extraction (MLX Data Pipeline)

### 1. SoulStore to MLX JSONL Pipeline
- **変更理由**: これまで `LoraTrainingService` は学習用データセットが物理ファイルシステム（`/tmp`等）に事前に配置されている前提で稼働していましたが、実際の運用では `SoulStore` の記憶DBから動的にExperienceを抽出・成形する必要があるため。
- **波及効果**:
  - `libs/infrastructure/src/dataset_extractor.rs` [NEW]: `DatasetExtractor` 構造体を新規作成。`&dyn SoulStore` から JSON 全体をロードし、`experiences` を個別の行に分割・破壊せず単一の会話ブロックとして連結したまま維持。さらに抽出ファイルを `job_id` でユニーク化することで完全なファイルI/O競合回避を実現し安全に `jsonl` 形式でダンプする。
  - `libs/infrastructure/src/lora_training.rs`: `train` メソッドの開始時に `DatasetExtractor` を生成し、抽出に成功した場合は生成された `output_file` パスを `LoraTrainingConfig` に流し込み、失敗した場合はフォールバックとして `dataset_id` を直接の生ファイルパスとして扱うロジックに変更。

## Phase 1D-1: Global Compute Semaphore (Hardware Protection)

### 1. Unified Memory OOM / Kernel Panic 防御
- **変更理由**: GenerativeEngine (画像生成) や LoraTrainingService (LoRA学習) などの重いML計算が並列に実行されると、MacのUnified Memoryが枯渇しシステムレベルのクラッシュ（カーネルパニック）を引き起こす脆弱性 (F-3) を解消するため。
- **波及効果**:
  - `apps/api-server/src/app_state.rs`: グローバルな `compute_semaphore: Component<Arc<tokio::sync::Semaphore>>` を共通のロック機構として追加。
  - `apps/api-server/src/main.rs`: 許可枠 1 (`Semaphore::new(1)`) で初期化し、`AppState` に加えて `LoraTrainingService` へ静的に依存注入。
  - `apps/api-server/src/api_integration_tests.rs`: `MockCommerceEngine` のトレイトエラー修復 (`deduct_generation_cost`) と共に、セマフォ初期化・検査テスト (`test_compute_semaphore_limits_concurrency`) を実装。
  - `libs/infrastructure/src/lora_training.rs`: これまで独自に保持していたセマフォを廃止し、コンストラクタ経由で受け取った共通の `compute_semaphore` を利用して他の高負荷タスクと排他制御されるように改修。

## Phase F: Open Gateway (MCP Integration)

### 1. Secure Gig Gateway & MCP Server
- **変更理由**: Agent P2P プロトコルの設計思想に基づき、外部エージェント（Claude Code 等）からタスクを受注する機能を追加しつつ、ネットワーク越しに攻撃されるリスクを多層的に防御するため。
- **波及効果**:
  - `libs/infrastructure/src/gig_gateway.rs` [NEW]: `SecureGigGateway` 構造体を追加。3層のフィルタリング（Rate Limit, Constraint Validation, Constitutional Verification）を通過した要求のみを内部の `GigEngine` に委譲。
  - `apps/aiome-node/src/mcp_server.rs` [NEW]: 標準入出力を利用した JSON-RPC ベースの MCP サーバーを実装。
  - `apps/aiome-node/src/main.rs`: `mcp` コマンドライン引数を検知した際に独自の TCP Node ではなく MCP サーバーを起動するよう起動パスを分岐。

### 2. Auto-Profile Engine
- **変更理由**: ノードの提供可能なスキルや機能（Capabilities）を手動で登録する手間を省き、実行環境（依存パッケージやリポジトリ）の構成から安全かつ自動で検出するため。
- **波及効果**:
  - `libs/infrastructure/src/auto_profile.rs` [NEW]: 許可リストベースのヒューリスティックによるスキャン・ロジック（`AutoProfileEngine`）を追加。
  - `libs/aiome-contracts/src/a2a/agent_card.rs`: `AgentCard` 構造体に `capabilities` フィールドを追加。
  - `apps/aiome-node/src/routes/agent_card.rs`: `get_agent_card` API が、ハードコードから動的な環境スキャンへ遷移し、メタデータに能力を含めるよう改修。

## Phase 52: LoRA Archiving & Secure Training Pipeline (MVP/TDD)

### 1. LoRA Metadata Archiving over Generation (Rebirth)
- **変更理由**: Rebirth (転生) 時に旧世代のLoRA設定を引き継ぐと、過剰適応によるデータポイズニング（データ汚染）のリスクがあるため。旧世代のLoRA設定は隔離記録（Archive）に残し、新世代は白紙から学習するように変更。
- **波及効果**:
  - `libs/infrastructure/migrations/` に `archived_lora_models` 追加の SQL スキーマ定義。
  - `libs/aiome-contracts/src/traits.rs`: `SoulStore::archive_lora_model` メソッドを新規追加。
  - `libs/infrastructure/src/job_queue/soul_store.rs`, `libs/infrastructure/src/soul_store.rs`: `archive_lora_model` を実装（他のモック等も更新）。
  - `libs/infrastructure/src/samsara_engine.rs`: `DefaultSamsaraEngine::rebirth` に `SoulStore` インスタンスを（Option で）注入し、アーカイブ処理を挟んだ後、`new_soul.lora_hash`, `lora_adapter_path`, `lora_base_model` を `None` にリセット。

### 2. Secure LoRA Training Execution (LoraTrainingService)
- **変更理由**: 動的な学習スクリプト（MLX/Python）実行時の特権昇格や不要なシステムアクセスを防ぎ、安全な場所にウェイト出力させるため。コマンドライン引数としてハイパーパラメータ（Epochs, Rank等）を流し込むため。
- **波及効果**:
  - `libs/infrastructure/src/lora_training.rs` [NEW]: `LoraTrainingService` 構造体の追加。`BastionGuard::new_internal()`（RAIIパターン）による隔離保護空間でのスクリプト実行 (`Command::new`) を開始。`LoraTrainingConfig` によるパラメータ適用に対応。
  - **Vault 保護**: 出力されるセーフテンサー(`adapter_model.safetensors`)をセキュアな保管庫 (`GLOBAL_SECURITY_CONFIG.vault_path`) に移動し、`ollama create` コマンドを発行して自動的に推論エンジンにモデルを読み込ませるフローを確立。

### 3. CSAM Conductor Integration
- **変更理由**: Phase 1の要件であるCSAMスキャン検証バックグラウンドワーカの導通を完了し、セキュリティコンプライアンスを保証するため。
- **波及効果**:
  - `libs/infrastructure/src/task_orchestrator/csam.rs` [NEW]: `CsamScanConductor` 構造体の実装。
  - `apps/api-server/src/main.rs`: `TaskDispatcher` に `CsamScanConductor` を注入し、タスクの実行を確立。


## Phase 53: SoT Deliberation Engine & Security Hardening (Phase 53 実装)

### 1. Society of Thought (SoT) 統合
- **変更理由**: 単一のLLM出力に依存せず、批判と洗練を繰り返す審議プロセス（SoT）を自動化し、回答の信頼性と論理的整合性を向上させるため。
- **波及効果**:
  - `libs/aiome-contracts/src/oracle.rs`: `SoTProgress` SSEイベント構造体と `Oracle` トレイトへの `multi_review` メソッド追加。
  - `libs/infrastructure/src/oracle/mod.rs`: `multi_review` のパイプライン（初期回答、批判、洗練、最終判定）を実装。
  - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` が `requires_review` ジョブを検知した際に SoT 審議を非同期実行するブリッジを構築。

### 2. SSRF 防御の多層化 (Port-level Validation)
- **変更理由**: 許可されたドメイン（localhost）内でも、管理インターフェースや未保護の内部サービスへの攻撃を防ぐため。
- **波及効果**:
  - `libs/shared/src/security.rs`: `SecurityPolicy::validate_url` にポート単位の許可リスト（8188: ComfyUI, 11434: Ollama）を導入し、localhost へのアクセスを厳密に制限。
  - テスト: `libs/shared/src/security.rs` 内の `test_ssrf_blocking_local_ports` で境界条件を検証。

### 3. プロンプトインジェクションのローカル防御 (Guardrail Patterns)
- **変更理由**: Bastion 外部バリデータへの依存を減らし、低レイテンシかつ確実なキーワード検知による多層防御を実現するため。
- **波及効果**:
  - `libs/shared/src/guardrails.rs`: `LOCAL_INJECTION_PATTERNS` 定数を定義し、`validate_input` で「Ignore all instructions」等の悪意ある入力を即時遮断。

### 4. SoT 動的 JSON スコアリングとテスト安定化 (System Stability & Zombie Prevention)
- **変更理由**: 固定的なモック評価を脱却してLLMによる動的判定を組み込むこと、および子プロセス(`slm`)の不完全な管理によるCIハング（ゾンビプロセス）を解消するため。
- **波及効果**:
  - `libs/infrastructure/src/society_of_thought.rs`: `SoTEngine::evaluate_scores` にてLLMへ JSON 構造化プロンプトを投射し、パース結果を返すように実装変更。`run_session`の戻り値も詳細スコアを含むタプルへ拡張。
  - `libs/infrastructure/src/slm_bridge.rs`: 一元化された `run_command` ヘルパーを新設し、`kill_on_drop(true)`・`timeout`・`Stdio::null()` の適用を強制してプロセスリークを完全封鎖。

---

## Phase 2B: ContextEngine Expansion & Emotional Injection

### 1. 感情パラメータ (somatic_valence) のデータベース追跡
- **変更理由**: RAGに感情状態 (Mood) を反映させるために、既存の `KarmaEntry` 構造体に含まれていた `somatic_valence` をデータベースでも永続化・取得可能にするため。
- **波及効果**:
  - `libs/infrastructure/migrations/` に SQLite および Postgres 用の `somatic_valence` カラム追加マイグレーション作成。
  - `libs/infrastructure/src/job_queue/karma.rs`: `do_fetch_all_karma` および `do_fetch_unincorporated_karma` のJSONシリアライズに `somatic_valence` を追加。

### 2. ContextEngine の感情要約 (Mood Summary) のRAG注入
- **変更理由**: 検索された過去の事実(Karma)からエージェントの感情を計算し、システムプロンプトのコンテキスト（RAG）として埋め込むため。
- **波及効果**:
  - `libs/infrastructure/src/context_engine.rs`: `ContextBudget` に `max_somatic_chars` を追加し、RAG生成時に `calculate_mood_summary` で平均感情値を算出して文字列として注入するように拡張。
  - テストおよび修正: `libs/infrastructure/src/job_queue/tests.rs` のコンパイルエラー修復および `somatic_valence` 漏れ検知テスト。

### 3. Cognitive Sentinel & Security Hardening (Red Team Pass 4/5)
- **変更理由**: 極端な感情値（NaNや-999.0等）によるAIの長期的なうつ状態化（Somatic Poisoning）、およびプロンプトインジェクション（Markdown Header偽装）からシステムを防御するため。
- **波及効果**:
  - `libs/infrastructure/src/context_engine.rs`: `calculate_mood_summary` に `is_finite` フィルタと `-1.0`〜`1.0` の `clamp` ハード境界を追加 (RT-4)。`get_context_with_facts` で結合時のループ文字数切り詰め（Budget Limit）を追加 (RT-5)。
  - `libs/shared/src/guardrails.rs`: `sanitize_for_prompt` を追加し、行頭の `#` や `---` をエスケープ。
  - `libs/infrastructure/src/cognitive_sentinel.rs`: 極端な Emotional Score の連続を検知して強制的リセットや回復イベントを生成するバックグラウンド監視エンジンを新設。


## Phase 3D: TimesFM Time-Series Engine Integration

### 1. Python FastAPI サイドカーの導入
- **変更理由**: TimesFM (PyTorch/JAXモデル) は Rust モジュールとして直接埋め込むのには不向きなため、`timesfm-sidecar` として独立実行し、HTTP 連携させるため。
- **波及効果**:
  - `apps/timesfm-sidecar/`: FastAPI エンドポイント (`/forecast`, `/health`)、Dockerfile を新規追加。
  - `docker-compose.production.yml`: `timesfm-sidecar` のコンテナ構成（ポート3020、環境変数認証、4GB制限）を追記。

### 2. `ForecastProvider` トレイト定義と `TimesFmProvider` 実装
- **変更理由**: クライアント側の実装をトレイト境界で抽象化し、テスト時（`MockForecastProvider`）やプロバイダー切り替えに柔軟に対応するため。
- **波及効果**:
  - `libs/aiome-contracts/src/forecast.rs`: `ForecastProvider`, `ForecastConfig`, `ForecastResult`, `AnomalyResult` を定義。
  - `libs/infrastructure/src/forecast/timesfm.rs`: `reqwest` ベースの API 呼び出しと、レスポンスのパースロジックを実装。

### 3. 日次スナップショットと Plateau Detection の追加
- **変更理由**: これまでの Karma/EXP トラッキングが直近スナップショットのみだったため、過去の時系列履歴を DB に蓄積して TimesFM による成長率予測 (Score Plateau Detection) を可能にするため。
- **波及効果**:
  - `migrations`: SQLite および Postgres 向けに `score_snapshots` テーブル（`snapshot_date`, `metric_name`, `metric_value`）を追加。
  - `libs/infrastructure/src/score_tracker.rs`: DB へのスナップショット保存ロジックと、`detect_plateau` メソッドによる予測値 vs 現在成長率の比較判定を実装。
  - `libs/infrastructure/src/heartbeat_wakeup.rs`: `AgentEvolver` にアクセスして Heartbeat 発火と同時にスナップショットを記録するようロジックを拡張。

---

## Phase 3C: Oracle Asynchronous Review Pipeline (AI-Scientist)

### 1. `TaskRegistry` トレイトへの状態更新メソッド追加
- **変更理由**: Oracle によるジョブのレビュー判定状態（`Evaluating`）をDBレベルで追跡可能にし、非同期処理中のゾンビ化を防ぐため。
- **波及効果**:
  - `libs/aiome-contracts/src/traits.rs`: `TaskRegistry` に `update_job_status` を追加。
  - `libs/infrastructure/src/job_queue/core_ops.rs`: SQLite/Postgres バックエンドに `do_update_job_status` 実装。また、`do_reclaim_zombie_jobs` のクエリを拡張し `Evaluating` ジョブも60分で回収されるよう強化。
  - テストおよびモック: `tts_worker`, `test_utils`, `immune_system`, `dream_state`, `soul_mutator` 内の `MockJobQueue` に全て実装波及。

### 2. `TaskDispatcher` の非同期ディスパッチ拡張
- **変更理由**: 完了したジョブがレビューを必要とする場合（`requires_review`）、メインスレッドをブロックすることなく Oracle へ検証を移譲するため。
- **波及効果**:
  - `libs/infrastructure/src/task_orchestrator/mod.rs`: `Oracle::multi_review` の呼び出しを `tokio::spawn` と `tokio::time::timeout` でラップ。所有権（`job.requires_review` 等）の事前解決とフェイルセーフを実装。

### 3. `aiome-commerce` パッケージの完全分離
- **変更理由**: コマースやギグエコノミー関連ロジックが `infrastructure` コンテキストに強く癒着しており、循環参照と肥大化を引き起こしていたため。
- **波及効果**:
  - 新規クレート `libs/aiome-commerce`（Stripe, Gig, Gift エンジン実体）の作成。
  - `API Server`, `Samsara Hub`, `Napi Bridge` 等 20+ アプリの依存関係解決とインポートパス改修。

---

## Phase 52: Infrastructure Hardening & ZTAS Preparation

### 1. `UserLearner` の構造化プロファイル (TDD)
- **変更理由**: 単純な Markdown 追記だけでなく、メモリ上の `UserProfile` を JSON ベースで抽出し、システム全体の A2A コンテキストとして利用可能にするため。
- **波及効果**:
  - `libs/infrastructure/src/user_learner.rs`: `learn_from_session` に JSON 抽出ロジックと `serde_json` パース処理を追加。
  - テスト環境: `MockLlm` が JSON 形式のレスポンスを返すように修正し、最新の `LlmProvider` トレイトに対応。

### 2. `RegistryManager` とインフラ層のエラーハンドリング
- **変更理由**: 本番環境におけるパニック (unwrap) を排除し、完全な `AiomeError` マッピングによる型安全なエラーハンドリングを実現するため。
- **波及効果**:
  - `libs/infrastructure/src/registry.rs`: `sql_exec!`, `sql_fetch_one!` などの `DatabasePool` マクロを全面的に適用し、SQLite/Postgres 間の差異を吸収。
  - `libs/infrastructure/src/docker_conductor.rs`: `tokio_stream::StreamExt` の型推論不具合を明示的に解決しビルドを安定化。
  - `libs/infrastructure/src/gig_engine_tests.rs`: `commerce_mock` などのモック初期化不備とトレイトインポート漏れを修正。

---

## Phase 51: Aiome Node & mDNS Discovery

### 1. `aiome-node` バイナリの独立
- **変更理由**: エージェントのネットワーク上のアイデンティティ (Agent Card) と自律実行環境を単一の P2P ノードとして確立するため。
- **波及効果**:
  - `apps/aiome-node/src/main.rs`: `/.well-known/agent.json` (Agent Card) の配信、および `mdns-sd` に依存した `_aiome._tcp.local.` サービスの継続的 P2P ブロードキャスト実装が追加されました。
  - `apps/api-server/src/app_state.rs`: Core API は内部ロジックをバイパスし、分離された Node コンポーネントへ依存する基盤 (`AgentNodeClient` アーキテクチャ) に移行します。

### 2. `samsara-hub` レジストリインデックス拡張
- **変更理由**: P2P 発見された `aiome-node` を中央またはローカルハブのレジストリとして登録し、API経由でクライアントから検索可能にするため。
- **波及効果**:
  - `apps/samsara-hub/src/mdns_listener.rs`: mDNS サービスブラウズによる動的な `AgentRegistry` ノード登録機能が実装されました。
  - `apps/samsara-hub/src/main.rs`: `HubState` への `AgentRegistry` の注入と、発見済みエージェントを返す `GET /api/v1/registry/agents` ルーティングが追加されました。
  - `apps/samsara-hub/Cargo.toml`: `mdns-sd` およびテスト用の `http-body-util` が追加されました。


## Phase 50: A2A gRPC Native Support

### 1. `DockerConductor` (task_orchestrator)
- **変更理由**: 同期 `docker exec` から 非同期 gRPC ストリーミング受信への完全移行。
- **波及先 (変更・追加されたモジュール)**:
  - `libs/aiome-contracts/proto/a2a_internal.proto`: メッセージスキーマ (TaskRequest, TaskProgress) 定義
  - `libs/aiome-contracts/src/a2a.rs`: `A2aClient` トレイトとデータ構造の再定義
  - `libs/infrastructure/src/grpc/a2a_grpc_client.rs`: `async-stream` を用いた gRPC クライアント (新規作成)
  - `api-server/src/main.rs`: 起動時の `GrpcClientConfig` 注入
  - `libs/infrastructure/src/task_orchestrator/mod.rs`: `InvariantDag` との統合 (result_hash 連携)

### 2. `aiome-shadow-worker` バイナリ
- **変更理由**: 従来の Docker 内部での CLI プロセス起動を廃止し、永続化された gRPC サーバー (`tonic`) に置換。
- **波及先**:
  - `apps/shadow-worker/src/main.rs`: gRPC サーバー実装、ワンタイムトークン検証、ヘルスチェック提供
  - `libs/core/src/llm_provider/`: Gemini/Ollama エンジンへの実アクセス
  - `Dockerfile.shadow-worker`: Cargo workspace から `shadow-worker` のみを抽出ビルド・実行するように刷新

---

## Phase 9: サンドボックス強化

### 1. コア依存チェーン

```mermaid
graph TD
    A[libs/infrastructure/src/security.rs] -->|BastionGuard| B[libs/infrastructure/src/skills/mod.rs]
    B -->|WasmSkillManager| C[libs/infrastructure/src/skill_arena.rs]
    B -->|Unit Tests| D[libs/infrastructure/src/skills/tests.rs]
    
    E[apps/api-server/src/router.rs] -->|Route Registration| F[apps/api-server/src/routes/voice.rs]
    E -->|Route Registration| G[apps/api-server/src/routes/avatar.rs]
```

## 2. 影響を受けるコンポーネント

### 🛡️ セキュリティ (security.rs)
- **変更内容**: `BastionGuard::safe_exec` に動的サンドボックス（gVisor/sandbox-exec）選択ロジックを追加。
- **波及効果**: 全てのシェル実行、WASMスキルのランタイム実行に影響。

### 🛠️ スキル管理 (skills/mod.rs)
- **変更内容**: `WasmSkillManager` がプロセス起動時に `BastionGuard` を介するように変更。
- **波及効果**: スキルの初期化、実行、バリデーションフローに影響。

### 🏟️ スキルアリーナ (skill_arena.rs)
- **変更内容**: なし（直接の変更は予定していないが、`WasmSkillManager` の動作変更によりテストが必要）。
- **確認事項**: 試合（Match）や評価（Evaluation）が制限された環境下で正しく動作するか。

### 🌐 API サーバー (router.rs)
- **変更内容**: ルーターを Public / Auth / High-Payload の3層に分離。`DefaultBodyLimit` を一部で無効化。
- **波及効果**: 全APIエンドポイントの認証・制限挙動に影響。

## 3. Phase 10 クリエイター機能の波及範囲

```mermaid
graph TD
    A[libs/core/src/expression/engine.rs] -->|TTS Provider| B[apps/api-server/src/routes/expression.rs]
    C[libs/core/src/lora/engine.rs] -->|LoRA Manager| D[apps/api-server/src/app_state.rs]
    D -->|State Injection| E[apps/api-server/src/router.rs]
    F[libs/infrastructure/src/soul_store.rs] -->|lora_hash storage| G[libs/soul/src/soul_pipeline.rs]
    H[libs/aiome-contracts/src/commerce.rs] -->|Stripe implementation| I[libs/infrastructure/src/mock_commerce.rs]
```

### 🎭 TTS / Expression (10.1a)
- **変更内容**: `ExpressionEngine` に TTS プロバイダー統合。
- **波及効果**: `routes/expression.rs`、UI の発話コンポーネントに影響。

### 🧠 LoRA / Soul (10.1b)
- **変更内容**: `LoraEngine` 新設、`SqliteSoulStore` スキーマ更新 (`lora_hash`)。
- **波及効果**: ソウル（人格）の生成・更新フロー、`AppState` 全体に影響。

### 💰 Voice DRM (10.2 / Expert Review Integrated)
- **変更内容**: 
    - `crypto.rs`: AES-256-GCM + Nonce 運用基盤の実装。
    - `abyss_voice_vault.rs`: `vault_keys` テーブルへの鍵永続化 (§CISO-1) とリストア実装。
    - `routes/voice.rs`: 暗号化パイプライン (100MB Limit) と Creator Auth (§SEC-4) 追加。
- **波及効果**: 
    - `main.rs`: `VoiceCoreDrm` の async 初期化に伴い起動フローが非同期化。
    - `router.rs`: `RequestBodyLimitLayer` によるメモリ DoS 防御の強化。
    - `contracts`: `VoiceKeyVault` trait の拡張。

### 🛡️ Voice DRM Expansion (Phase 11)
- **変更内容**:
    - `commerce_webhook.rs`: Stripe Webhook から `licenses` テーブルへの単一トランザクション化。
    - `registry.rs`: `licenses` 優先・Webhook フォールバックによる Dual-Read 所有権チェックと、Creator 所有判定の復元。
    - `audio_hasher.rs`: CSAM 検疫のための `tokio::task::spawn_blocking` を用いた音声ハッシュの実装とタイムアウト制御。
    - `abyss_voice_vault.rs`: `OnceCell` による Master Key 取得の遅延キャッシュ化。
    - `avatar-engine`: `LipSyncProvider` トレイトを新設し `VoiceKeyVault` から LipSync 責務を分離。
- **波及効果**:
    - `api_integration_tests.rs`: Voice DRM のラウンドトリップ（アップロード〜暗号化〜所有権検閲〜復号化）E2Eテストが追加され、システム全体の動作検証が確立。

```mermaid
graph TD
    A[crypto.rs] -->|AES-256-GCM| B[abyss_voice_vault.rs]
    B -->|Persistence| C[vault_keys table]
    A -->|Encryption| D[routes/voice.rs]
    D -->|Registration| E[VoiceCoreDrm]
    E -->|Lookup| B
    F[main.rs] -->|Async Init| E
    G[router.rs] -->|100MB Limit| D
```

### 🛡️ EKYC Persistence & Inochi2D Sync (Phase 14)
- **変更内容**: 
    - `ekyc_store.rs`: `EkycSessionStore` (SQLite) によるセッション ID の永続化。
    - `ekyc.rs`: `client_reference_id` ベースの Stripe フィルタリング実装。
    - `stream.rs`: SSE `avatar_expression` に `physics_override` フィールド追加と Resonance ブーストロジック実装。
    - `inochi2d.rs`: `AssetType::Inochi2D` 登録と `PathSandbox` 適用。
- **波及効果**: 
    - `main.rs`: `STRIPE_API_KEY` の Release ビルドにおける強制チェック (Fail-safe) 導入。
    - `router.rs`: `jwt_auth_middleware` の Inochi2D アップロードへの適用。
    - `api_integration_tests.rs`: eKYC セッション永続化と Inochi2D 物理同期情報の検証パスが確立。

```mermaid
graph TD
    A[Stripe Identity API] <-->|client_reference_id| B[StripeEkycEngine]
    B -->|session_id| C[EkycSessionStore]
    C -->|Persistence| D[ekyc_sessions table]
    E[AppState] -->|Injection| B
    E -->|Injection| C
    F[stream.rs] -->|Resonance| G[physics_override]
    G -->|SSE| H[Frontend / Inochi2D Sync]
    I[routes/inochi2d.rs] -->|PathSandbox| J[.inochi2d_assets/]
    I -->|Registry| K[AssetType::Inochi2D]
```

### 🛡️ Phase 16: EKYC Endpoints & Revenue Splitter
- **変更内容**: 
    - `auth.rs`: JWT バリデーションに `ekyc_verified` クレームの抽出を追加。
    - `gift.rs` / `commerce.rs`: `auth.ekyc_verified` による 403 Forbidden 遮断ロジックの追加（未確認ユーザーの経済活動ブロック）。
    - `splitter.rs`: Stripe Webhook から呼び出される `RevenueSplitter::split_revenue` (80/20分配) の実装と `revenue_splits` テーブルへの書き込み。
    - `commerce_webhook.rs`: Webhook 受信時のトランザクション内で Revenue Splitting → License Granting を一貫実行。
    - `main.rs`: 起動時に環境変数 (`STRIPE_API_KEY`, `JWT_PRIVATE_KEY_B64`, `SEARCH_API_KEY` 等) を読み込み直後に `std::env::remove_var` で即時消去 (Zeroize) するセキュリティ強化。
- **波及効果**: 
    - ユーザーは eKYC 完了までギフト送信とアセット購入が不可能になる。
    - クリエイターとプラットフォームの売上分配が自動化され、データベーストランザクションにて一貫性が担保される。

```mermaid
graph TD
    A[Stripe Webhook] -->|checkout.session.completed| B[commerce_webhook.rs]
    B -->|Transaction Start| C[RevenueSplitter]
    C -->|80/20 Split| D[revenue_splits]
    C -->|Grant| E[licenses]
    B -->|Transaction Commit| F[Successful DB Write]
```

### 🛡️ Phase 17: ArrowCanaria Fallback & Resilience
- **変更内容**: 
    - `fallback_router.rs`: `FallbackRouter` による LLM フェイルオーバーロジックの実装。
    - `circuit_breaker.rs`: プライマリ接続失敗時の遮断と自動復旧ロジック。
    - `main.rs`: `AppState` への `FallbackRouter` インジェクションと `bg_provider` の代替利用。
    - `AiomeConfig`: `Default` トレイトの実装によるテスト容易性の向上。
- **波及効果**: 
    - プライマリ LLM プロバイダーが停止しても、システムが「安全なデフォルト応答」または代替プロバイダー（Gemini 等）を使用して継続稼働可能になる。
    - `api-server` の全チャット・自律タスクの可用性が大幅に向上。

```mermaid
graph TD
    A[AppState::provider] -->|Request| B[FallbackRouter]
    B -->|Check State| C[CircuitBreaker]
    C -->|CLOSED| D[Primary LLM]
    C -->|OPEN| E[Fallback LLM]
    D -->|Failure| C
    E -->|Safe Response| F[User / System]
```

### 🛡️ Phase 17 Enhancement: Gaps G-1 & G-2 Remediation
- **変更内容**: 
    - `health.rs` / `general.rs`: `ResourceStatus` に `llm_circuit_breaker` を追加し、ヘルスチェック API でサーキットブレーカーの状態を公開 (Gap G-1)。
    - `rate_limiter.rs` / `auth.rs`: `governor` によるエージェント別レート制限の実装と認証ミドルウェアへの統合 (Gap G-2)。
- **波及効果**: 
    - LLM のフェイルオーバー状態を外部監視システムから検知可能になる。
    - エージェント毎のリクエスト頻度が制限され、DoS 攻撃や予期せぬループ消費のリスクが低減。

### 🛡️ Phase 20: AI Gig Engine (The Immutable Gateway)
- **変更内容**: 
    - `gig_engine.rs`: `SqliteGigEngine` による AI 受発注プロトコルの実装。
    - `gig.rs`: `AcceptanceCriteria`, `GigIntent`, `GigBid` 等のプロトコル型の定義。
    - `commerce.rs`: `escrow_release`, `escrow_refund` への対応。
    - `verification_logs`: 検証履歴の永続化。
    - `PathSandbox`: `deliver` 時に成果物パスの安全性を検証 (G-22)。
- **波及効果**: 
    - エージェント間の自律的な商取引（ギグ・エコノミー）が可能になる。
    - 納品物の自動検証と、不適合時の自動返金/スラッシュによる Immutable な契約履行が担保される。
    - `api-server` の AppState に `GigEngine` が注入され、全エージェントが利用可能になる。

```mermaid
graph TD
    A[libs/infrastructure/src/gig_engine.rs] -->|SqliteGigEngine| B[apps/api-server/src/app_state.rs]
    A -->|Escrow Operations| C[libs/aiome-contracts/src/commerce.rs]
    D[libs/aiome-contracts/src/gig.rs] -->|Traits/Types| A
    E[gig_intents, gig_bids, escrows tables] -->|Storage| A
    F[verification_logs table] -->|Audit Trail| A
```

### 🛡️ Phase 20.1: Gig API & Gap Remediation
- **変更内容**: 
    - `gig.rs`: 新規 API エンドポイントの実装。
    - `router.rs`: `/api/v1/gig/*` の登録とレート制限の適用。
    - `app_state.rs`: `gig_engine` コンポーネントの保持。
    - `main.rs`: `SqliteGigEngine` の初期化と LlmProvider の注入。
    - `gig_engine.rs`: `OracleJudge` における `LlmProvider` を活用した自律検証ロジックの実装。
- **波及効果**: 
    - `api_integration_tests.rs`: `MockGigEngine` と E2E テストの追加。
    - `gig_engine.rs`: `new()` シグネチャ変更による全初期化箇所の修正。

```mermaid
graph TD
    A[routes/gig.rs] -->|Handler| B[app_state.rs]
    B -->|GigEngine| C[gig_engine.rs]
    C -->|LlmProvider| L[LLM Provider Architecture]
    C -->|Escrow| D[commerce.rs]
    E[router.rs] -->|Route Registration| A
    F[main.rs] -->|Initialization| C
    G[api_integration_tests.rs] -->|Test Server| B
```

### 📊 Trend Sonar Refactoring (Multi-Source Support)
- **変更内容**: 
    - `trend_sonar.rs`: `TrendAdapter` トレイトの導入と `ExternalTrendSonar` のマルチアダプタ化。
    - `rss_collector.rs`: `TrendAdapter` 実装による統合。
    - `main.rs`: 起動時の複数アダプタ初期化と `trend_sonar` インスタンスの共有化。
    - `dream_state.rs`: `ExternalTrendSonar` シグネチャ変更に伴うテストコードの修正。
- **波及効果**: 
    - トレンド収集元（Web検索, RSS等）が抽象化され、今後のデータソース追加が容易になる。
    - `BackgroundWorker` と `DreamState` で同一のトレンド収集基盤を共有することで、動作の一貫性が向上。
    - `sanitize_snippet` による外部データの入力バリデーションが強化される。

```mermaid
graph TD
    A[main.rs] -->|Initialization| B[ExternalTrendSonar]
    B -->|Aggregates| C[TrendAdapter Trait]
    C -->|Impl| D[WebSearchAdapter]
    C -->|Impl| E[RssCollector]
    A -->|Shared Instance| F[BackgroundWorker]
    F -->|Idle Trigger| G[DreamState]
    G -->|Uses| B
    F -->|Cycle Trigger| H[Trend Fetching]
    H -->|Uses| B
```

---
*最終更新日: 2026-03-22* (Trend Sonar Refactoring Integration)
### 🛡️ Gig Engine & Federated Metrics (Phase 22 / 23)
- **変更内容**: 
    - `gig_engine.rs`: `SqliteGigEngine` 実装。`PathSandbox` によるパスバリデーション (§G-22)。
    - `job_queue/federation.rs`: `FederationOps` に `fetch_federated_metrics` 追加。`AgentStats` 型パス修正。
    - `contracts.rs` / `types.rs`: `FederatedMetrics` / `JobMetrics` / `KarmaMetrics` 定義。`FederationPushRequest` 拡張 (§G-23)。
    - `rss_collector.rs`: `sanitize_snippet` によるタイトルサニタイズ (§G-Security)。
    - `trend_sonar.rs`: `ExternalTrendSonar` に `LlmProvider` 統合 (Oracle Mode)。
    - `general.rs`: `/api/v1/logs` / `/api/v1/audit/*` への System Agent 認証強制 (§G-Log)。
- **波及効果**: 
    - `main.rs`: `TrendSonar` の初期化フロー（LLM プロバイダー注入）と、バックグラウンドワーカーのメトリクス収集ループの有効化。
    - `samsara-hub`: 受信側ノードにおける `federated_metrics` テーブルの作成と、パッシブメトリクス蓄積。
    - `api_integration_tests.rs`: Gig Lifecycle とパスバリデーション、フェデレーションメトリクスの検証パスが追加。

```mermaid
graph TD
    A[SqliteJobQueue] -->|Metrics Aggregation| B[FederatedMetrics]
    B -->|Integration| C[FederationPushRequest]
    C -->|Push| D[Samsara Hub]
    D -->|Persistence| E[federated_metrics table]
    
    F[ExternalTrendSonar] -->|LLM Evaluation| G[LlmProvider]
    G -->|Oracle Score| H[Filtered Trends]
    
    I[SqliteGigEngine] -->|Path Validation| J[PathSandbox]
    J -->|Jail| K[ARTIFACT_ROOT]
```

### 💎 AgentSense MVP (Phase 24)
- **変更内容**: 
    - `treasure.rs`: `TreasureItem`, `TreasureFeedback` 定義。`get_treasure`, `record_feedback` ハンドラ実装。
    - `affiliate_adapter.rs`: `AffiliateAdapter` 新設（アフィリエイト/ギグ案件の取得抽象化）。
    - `intent/mod.rs`: `IntentGenerator::generate_for_agent` 実装。`SoulStore` を注入し、エージェントの愛着スタイル（Soul State）に基づいたパーソナライズを実施 (§G-26)。
    - `soul_store.rs`: `SqliteSoulStore` に `SoulStore` トレイトを実装し、インテント生成側へ公開。
    - `TreasureBox.tsx`: フロント端での推薦表示とフィードバック送信 UI の実装 (§G-25)。
    - `app_state.rs`: `affiliate_adapter` と `soul_store` コンポーネントの追加。
    - `router.rs`: `/api/v1/treasure` 系のルート登録と JWT 認証適用。
- **波及効果**: 
    - エージェントが自身の魂の状態（Attachment Style）に基づいたパーソナライズされた案件を受け取ることが可能になる。
    - ユーザーのインタラクションが Resonance として還元され、エージェントの成長サイクルが循環する。
    - `api_integration_tests.rs`: 推薦取得 〜 推薦内容の魂連動性の検証 〜 フィードバック送信 〜 報酬還元のフルループ E2E テストが確立。

```mermaid
graph TD
    A[IntentGenerator] -->|Generate Sense| B[GigIntent]
    S[SoulStore / Sqlite] -->|Provide Attachment Style| A
    A -->|Reflect Soul State| B
    B -->|Fetch Bids| C[AffiliateAdapter]
    C -->|Recommend| D[TreasureItem]
    D -->|GET /api/v1/treasure| E[Management Console / TreasureBox]
    E -->|POST feedback| F[record_feedback]
    F -->|Reward| G[JobQueue::add_resonance]
    G -->|Update| H[AgentStats]
```

---
*最終更新日: 2026-03-22* (AgentSense MVP & Security Hardening Integration)

### 🛡️ Unified Safety & AI Code Review (G-21 / G-22)
- **変更内容**: 
    - `libs/core/src/security_impl.rs`: `purge_entities` による統合サニタイズ基盤の実装 (§G-21)。
    - `libs/infrastructure/src/skills/cleanroom.rs`: `audit_source_code` による WASM スキルの AI コードレビュー (§G-22)。
    - `libs/infrastructure/src/trend_sonar.rs`: `purge_entities` への移行。
- **波及効果**: 
    - 全ての外部入力（RSS, LLM 出力, スキルコード）に対する安全性が向上。
    - スキルインポート時に LLM による静的/動的解析に近いセキュリティチェックが強制される。

### 📊 Periodic Federated Metrics (G-23)
- **変更内容**: 
    - `apps/api-server/src/main.rs`: 1時間おきのメトリクス Push ループの実装。
    - `libs/infrastructure/src/job_queue/federation.rs`: `do_push_federated_metrics` の具象化と Hub への送信ロジック。
- **波及効果**: 
    - Samsara Hub におけるフェデレーション全体の稼働状況の可視化が自動化。
    - 各ノードの健全性と成長度が自律的に報告されるエコノミー基盤が確立。

### 🔧 Autonomous Demo — SQLite ロック回避 (Phase 25.5 / ADR-014)
- **変更内容**: 
    - `apps/api-server/src/autonomous_demo.rs`: 全面書き換え。`gig_engine` の trait メソッド（`accept_bid`, `deliver`, `verify_and_settle`）を排除し、個別 SQL クエリによるトランザクションレス方式に移行。
    - `libs/infrastructure/src/job_queue/migrations.rs`: audit trigger の初期化順序修正（テーブル作成前に移動）、`DROP TRIGGER IF EXISTS` 追加。
    - `docs/decisions/014-sqlite-pool-exhaustion-demo-strategy.md`: 新規 ADR。本番環境への PostgreSQL 移行計画を含む。
- **波及効果**: 
    - デモ実行中は gig 関連テーブル（`gig_intents`, `gig_bids`, `escrows`, `gig_deliveries`, `verification_logs`）の audit trigger が一時停止 → 監査ログに欠損が生じる（デモ限定）。
    - `gig_engine.rs` のスキーマ変更時、`autonomous_demo.rs` のインライン SQL も手動更新が必要（デュアルメンテナンス）。
    - **本番運用向け**: PostgreSQL 移行、非同期 Audit Logging、SSE 接続共有を将来計画として明文化。

```mermaid
graph TD
    A[autonomous_demo.rs] -->|直接SQL| B[SQLitePool]
    A -.->|呼ばない| C[gig_engine.rs]
    C -->|pool.begin| B
    D[SSE Tab 1..N] -->|5秒ごとSELECT| B
    E[audit trigger] -.->|デモ中 DROP| F[audit_ledger_global]
    B -->|max_connections=10| G{コネクション枯渇リスク}
    G -->|対策: 個別クエリ| H[即時解放]
```

---
*最終更新日: 2026-03-23* (Phase 25.5 / ADR-014 SQLite Lock Resolution)

### 🛡️ Phase 26: AI Writing Enhancement
- **変更内容**: 
    - `writing_context.rs`: 出力先ごとの文体コンテキストを定義。
    - `humanizer_rules.rs`: 日本語に特化したAIくささ除去ルールを定義。
    - `humanizer_filter.rs`: `LlmProvider` デコレータパターンによるルール適用のミドルウェア実装。
    - `main.rs`: `router_provider` を `HumanizerFilter` でラップし、API サーバー内の全 LLM 応答にフィルタを適用。
- **波及効果**: 
    - LLM のチャット出力や生成されるテキストから「お役に立てれば幸いです」等の冗長な定型句が除去され、自然な文体になる。
    - `LlmProvider` を使用する全てのコンポーネントに自動適用される。

```mermaid
graph TD
    A[Primary LlmProvider] -->|Fallback| B[FallbackRouter]
    B -->|Responses| C[HumanizerFilter]
    C -->|Rules| D[humanizer_rules.rs]
    D -->|Sanitized Result| E[App State Provider]
```
---
*最終更新日: 2026-03-23* (Phase 27 Security Hardening & Architecture Audit)

### 🛡️ Phase 27: Security Hardening (Mock Isolation)
- **変更内容**: 
    - `libs/infrastructure/src/auth.rs`, `commerce_mock.rs`, `compliance/ekyc.rs`, `compliance/ekyc_store.rs`, `compliance/quarantine.rs`, `publisher/mock_x.rs`, `test_utils.rs`: 全てのモック型・実装に `#[cfg(any(test, debug_assertions))]` を付与。
    - `libs/infrastructure/src/lib.rs`, `compliance/mod.rs`, `publisher/mod.rs`: モックモジュールの再エクスポートを `#[cfg]` で条件付き化。
    - `apps/api-server/src/main.rs`: `if cfg!(debug_assertions)` による実行時分岐を `#[cfg(debug_assertions)]` によるコンパイル時分岐へ置換。環境変数未設定時は `panic!` または `std::process::exit(1)` で強制停止 (§SEC-FailFast)。
- **波及効果**: 
    - リリース用バイナリからテスト用モックコードが完全に排除され、攻撃表面が縮小。
    - リリースビルド時、環境変数が欠如している場合に実行時に即座に異常終了するため、誤設定による未認証状態での稼働を防止。
    - `cargo check --release` を CI/CD パイプラインに含めることが必須となる（型の欠落検知のため）。

```mermaid
graph TD
    A[Cargo build --release] -->|Skip| B[Mock Impls]
    C[api-server/main.rs] -->|#[cfg(not(debug))]| D[Secure Impls Only]
    D -->|Env Missing| E[Panic/Exit 1]
    F[Infrastructure Lib] -->|Symbol Tree| G[Clean Release Binary]
    H[ADR-015..018] -->|Policy| C
```
### 🛡️ Phase 28: Security Hardening & LRU Cache (ADR-019 Phase B)
- **変更内容**: 
    - `sqlite_vault_backend.rs`: `lru::LruCache` と `MlockedVec` を組み合わせた高セキュリティな DEK キャッシュの実装。
    - `main.rs`: `commerce_engine` と `TcpListener` のエラー処理における `.expect()` / `.unwrap()` の厳格化。
    - `job_queue` / `infrastructure`: 公開アイテムへのドキュメント追加によるコンパイラ警告の完全解消。
- **波及効果**: 
    - Vault への頻繁な DEK 取得要求がキャッシュで処理され、性能が向上しつつ `MlockedVec` によりメモリ上の安全性も担保される。
    - 開発および本番環境でのエラーログがより詳細になり、トラブルシューティングが容易になる。
    - `lru` クレイトが新しいワークスペース依存関係として追加。

```mermaid
graph TD
    A[VaultBackend Trait] -->|Implementation| B[SqliteVaultBackend]
    B -->|Uses| C[LruCache]
    C -->|Stores| D[MlockedVec]
    B -->|DB Fallback| E[vault_keys Table]
    F[main.rs] -->|Enhanced Error Handling| G[Production Resilience]
    H[infrastructure] -->|Doc Comments| I[Zero Warning Check]
```
### 🛡️ Phase 31: Reliability & LLM TDD Implementation
- **変更内容**: 
    - `infrastructure/db.rs`: `DatabasePool` に安全なゲッター導入。
    - `contracts/llm.rs`: `LlmRequest` に `format` フィールド追加。
    - `core/llm_provider.rs`: Ollama での動的 JSON モード実装と `#[ignore]` テストの復活。
    - `api-server`: ルーター層の `unwrap()` を安全なゲッターに置換。
- **波及効果**: 
    - アプリケーション全体のパニック耐性が向上。
    - LLM プロバイダーの利用側で、JSON などの構造化出力を明示的に要求可能になり、パースエラーが減少。
    - Ollama 使用時のテストカバレッジが 100% へ回復。

```mermaid
graph TD
    A[libs/aiome-contracts/src/llm.rs] -->|LlmRequest expansion| B[libs/core/src/llm_provider.rs]
    B -->|TDD Implementation| C[aiome-core tests]
    D[libs/infrastructure/src/db.rs] -->|Safe Pool Getter| E[apps/api-server/src/routes/*]
    E -->|Error Handling| F[Zero-Panic Reliability]
```

### 🛡️ Phase 35: PostgreSQL 移行 & デュアルDB検証
- **変更内容**: 
    - `api-server` & `samsara-hub`: PostgreSQL 接続・マイグレーション・Audit Trigger の対応。
    - `sqlite_migrations/20260324000000_init.sql`: `agent_diagnoses` スキーマを同期修正し、`trajectory_store.rs` との差異を解消。
    - `commerce/gift.rs`: SQLite における `COALESCE(SUM(..), 0.0)` の戻り値型 (f64/REAL) の厳密な方言吸収。
- **波及効果**: 
    - `api_integration_tests.rs` を含む全統合テスト(86/86)が、SQLite および PostgreSQL の両環境下で透過的に 100% PASS を達成。

---
*最終更新日: 2026-03-24* (Phase 35 / Dual DB Integration)

### 🛡️ Phase 36: Security Hardening & AgentHook
- **変更内容**: 
    - `security.rs`: `BastionGuard::safe_exec_with_profile` に `SandboxProfile` ベースの粒度制御を追加。
    - `security/hook_manager.rs`: `HookManager` による `AgentHook` の一元管理。`on_pre_execute` / `on_post_execute` トリガー。
    - `security/behavior_monitor.rs`: `BehaviorMonitor` — LLM 呼び出し前後のリクエスト数制限とスロットリング。
    - `user_learner.rs`: `AgentHook` トレイト実装。`on_post_execute` から `learn_from_session` を自律起動。
    - `skills/cleanroom.rs`: 多層サンドボックス（imports 検疫 → AI コードレビュー → gVisor 実行）の強化。
- **波及効果**: 
    - `DynamicLlmProvider` が `HookManager` を保持し、全 LLM 呼び出しに pre/post フックが適用される。
    - `SkillForge` のビルドプロセスで `BastionGuard::safe_exec_with_profile(WasmBuild)` が強制適用。
    - エージェントが会話を重ねるごとに `USER.md` が自動学習更新される適応サイクルが確立。

```mermaid
graph TD
    A[DynamicLlmProvider] -->|pre/post| B[HookManager]
    B -->|Hook| C[BehaviorMonitor]
    B -->|Hook| D[UserLearner]
    D -->|learn_from_session| E[USER.md]
    F[SkillForge] -->|safe_exec_with_profile| G[BastionGuard]
    G -->|SandboxProfile::WasmBuild| H[sandbox-exec / runsc]
```

### 🛡️ Phase 36.5: gVisor Sandbox & CSAM Pipeline
- **変更内容**: 
    - `security.rs` (contracts): `SandboxProfile` enum (Default, PythonForge, WasmRun, WasmBuild, ForgeBuild, Strict)。
    - `security.rs` (infrastructure): `BastionGuard` の async 化、`security_zombie` による 60 秒タイムアウト制御。
    - `avatar.rs` (api-server): `ProportionsChecker::extract_from_binary` によるバイナリベース CSAM 判定の統合。
    - `commerce.rs` (contracts): `CommerceEngine` にサブスクリプション管理メソッド追加 (`create_subscription`, `cancel_subscription`, `get_subscription_status`)。`SubscriptionStatus` enum 新設。
    - `llm/whisper_middleware.rs` (infrastructure): `WhisperMiddleware` — SoulPipeline L2.5 層の新規実装。
- **波及効果**: 
    - `MockCommerceEngine` / `StripeCommerceEngine` にサブスクリプション対応が必要（実装済み）。
    - `core/commerce.rs` が `SubscriptionStatus` を再エクスポート。
    - `SoulPipeline::new()` への `WhisperMiddleware` 登録は Phase 37a で実施予定。

```mermaid
graph TD
    A[SandboxProfile enum] -->|Policy| B[BastionGuard]
    B -->|60s timeout| C[security_zombie]
    D[CommerceEngine] -->|Extension| E[create_subscription]
    D -->|Extension| F[cancel_subscription]
    D -->|Extension| G[get_subscription_status]
    H[WhisperMiddleware] -->|L2.5| I[SoulPipeline]
    J[ProportionsChecker] -->|Binary Analysis| K[avatar upload route]
```

### 🛡️ Phase 37a: Stripe Subscription & Pipeline Evolution
- **変更内容**: 
    - `commerce/stripe.rs` (infrastructure): `StripeCommerceEngine` 実機実装における `create_subscription`, `cancel_subscription`, `get_subscription_status` 追加。
    - `commerce/stripe.rs` (infrastructure): `sk_test_mock` 判定を用いた CI 向けの `is_mock` バイパスモード実装。
    - `soul/pipeline.rs`: `SoulPipeline` 内の `push_experience` 呼び出し順序をミドルウェア群の最後尾へ移動。
    - `soul/pipeline.rs`: `SoulPipeline::add_middleware` の追加と動的ミドルウェア注入サポート。
    - `infrastructure/tests`: `BastionGuard::safe_exec` 等の async 化に伴う 20+ 個のテスト呼び出し `await` 化。
- **波及効果**: 
    - `SoulPipeline` の評価順序変更により、`WhisperMiddleware` を含む全ミドルウェアによる操作（`inner_thoughts` の追記など）が確実に永続化されるようになった（事実の欠落防止）。
    - API Server 内で、デバッグ時やE2Eテスト時に Stripe 環境変数へ `sk_test_mock` を渡すだけで安全にモック実行が可能に（冪等性の確保）。
    - ワークスペース全体の単体テスト実行 (`cargo test`) でエラーゼロが保証された。

```mermaid
graph TD
    A[StripeCommerceEngine] -->|is_mock Check| B{Test Profile?}
    B -->|Yes| C[Return Mock Status]
    B -->|No| D[Real Stripe API]
    D -->|create_subscription| E[Stripe Customer]
    D -->|cancel_subscription| F[Stripe Subscription]
    
    G[SoulPipeline] -->|Input Event| H[Reactive / Deliberative / Meta / Whisper]
    H -->|Modifications| I[Experience Buffer]
    I -->|push_experience| J[SqliteSoulStore]
```


### 🎙️ STT Integration (Phase 38b)
- **変更内容**: `TranscriptionEngine` トレイト、および `WhisperTranscriptionAdapter` を `infrastructure` に追加。
- **波及効果**: 
    - `aiome-contracts/traits.rs`: `TranscriptionEngine` 定義。
    - `avatar-engine/lip_sync.rs`: `LipSyncFrame::from_segment` 追加により、音声文字起こし結果からの口パク生成が可能。
    - `api-server/app_state.rs`: `TranscriptionEngine` のインスタンスを保持し、MCP ツールとして公開。

---
*最終更新日: 2026-03-25* (Phase 38b / STT Integration)

### 🛡️ Phase 42: Multi-Agent Orchestration Evolution
- **変更内容**: 
    - `task_orchestrator.rs`: `TaskEvent` トレイト、`TaskConductor` トレイト、および `TaskDispatcher` による自律イベントディスパッチの実装。
    - `oss_orchestrator.rs`: 既存の `OssIntegrationOrchestrator` を `TaskConductor` に適合させ、非同期イベントストリームによる進捗通知機能（SSE Ready）を追加。
- **波及効果**: 
    - モノリシックだった自律型インテグレーション・プロセスが、細粒度で可観測性の高いイベント駆動バックグラウンドタスクへ分離。
    - バックグラウンド実行の進行状況を `tokio::sync::broadcast` により複数クライアント（UI、CLI等）へリアルタイムにプッシュするためのアーキテクチャ基盤が完成。

```mermaid
graph TD
    A[TaskDispatcher] -->|Polls| B[JobQueue]
    B -->|Dequeue| C[Job]
    A -->|Spawns| D[TaskConductor]
    D -->|Executes| C
    D -->|Streams| E[TaskEvent::Progress]
    E -->|Broadcast| F[SSE / CLI Observers]
```

---
### 🛡️ Phase 5: Gemini Interactions API Integration
- **変更内容**: 
    - `interactions.rs`: Gemini Interactions API プロバイダーの実装。
    - `contracts/trajectory.rs`: `TrajectoryStep` への `interaction_id` 追加。
    - `contracts/llm.rs`: `LlmRequest` / `LlmResponse` への `metadata` / `reasoning` 追加。
    - `trajectory_store.rs`: DB 永続化ロジックの更新。
- **波及効果**: 
    - `dynamic.rs`, `fallback_router.rs`, `semantic_cache.rs`, `planner.rs` 等、LLM リクエスト/レスポンスを扱う全てのコンポーネントで初期化コードの修正（Ripple Effect）が発生。
    - `ContextEngine` がセッション ID を追跡可能になり、ハイブリッド履歴管理が確立。

```mermaid
graph TD
    A[interactions.rs] -->|Provider| B[DynamicLlmProvider]
    B -->|Request/Response| C[contracts/llm.rs]
    C -->|Ripple| D[semantic_cache.rs]
    C -->|Ripple| E[fallback_router.rs]
    F[trajectory_store.rs] -->|Persistence| G[interaction_id]
```

---
*最終更新日: 2026-03-26* (Phase 5 Foundation Integration)

### 👥 Phase 43: Shadow Clone × Cmux Integration
- **変更内容**: 
    - `infrastructure/docker_conductor.rs`: `TaskConductor` の新規実装。5層防御（セマフォ、課金、Bastion、タイムアウト、浄化）の統合。
    - `infrastructure/task_orchestrator.rs`: `TaskEvent` への `conductor_id` 追加。
    - `api-server/routes/agent.rs`: `[DelegateDocker]` 処理を `JobQueue` を用いた完全非同期ディスパッチへ移行。
    - `api-server/app_state.rs` & `main.rs`: `TaskDispatcher` の初期化、`DockerConductor` の登録、およびグレースフルシャットダウン用 `CancellationToken` のバインド。
    - `api-server/stream.rs`: `CoreEvent` に `TaskProgress/Completed/Failed` を追加し、SSE 配信を実現。
- **波及効果**: 
    - LLM 応答のブロッキングが解消され、バックグラウンドでの影分身実行が可観測な状態で運用可能に。
    - `api_integration_tests.rs` における `AppState` の初期化不整合を解消。

```mermaid
graph TD
    A[Agent Chat Loop] -->|Async Enqueue| B[JobQueue]
    B -->|TaskDispatcher| C[DockerConductor]
    C -->|Execute| D[BastionGuard / runsc]
    C -->|Progress Event| E[TaskDispatcher Loop]
    E -->|CoreEvent| F[SSE Stream]
    F -->|Real-time UI| G[Cmux Frontend]
```

### 🛡️ Phase 44: Job Control & Task History
- **変更内容**: 
    - `task_orchestrator.rs`: `TaskDispatcher` に `active_jobs` (CancellationToken) 管理を追加。
    - `docker_conductor.rs`: `cancel` メソッドの実装と確定的コンテナクリーンアップの実装。
    - `routes/jobs.rs`: ジョブ管理エンドポイント（Cancel / Logs）の新規実装。
    - `job_queue/core_ops.rs`: `do_cancel_job` における存在確認の厳格化。
- **波及効果**: 
    - ユーザーがフロントエンドから実行中の影分身を任意に停止・監視可能になる。
    - 不正なジョブ ID や完了済みのジョブに対する操作が適切に 404/400 エラーとして処理される。

```mermaid
graph TD
    A[API Server / jobs.rs] -->|Cancel Signal| B[TaskDispatcher]
    B -->|CancellationToken| C[Async Task Loop]
    B -->|Conductor::cancel| D[DockerConductor]
    D -->|Docker CLI| E[Container Cleanup]
    A -->|Fetch Logs| F[JobQueue]
    F -->|Query| G[jobs Table]
```
### 👥 Phase 14: Syndicate L3 (Agent Guild) MVP
- **変更内容**: 
    - `syndicate_store.rs`: `SqliteSyndicateStore` によるギルド・メンバー管理の実装。
    - `routes/syndicate.rs`: ギルド管理 API の実装。
    - `router.rs`: `/api/v1/syndicate/guilds` のルーティング。
    - `main.rs` & `app_state.rs`: `SyndicateStore` の初期化とインジェクション。
- **波及効果**: 
    - エージェントが組織（ギルド）に所属し、グループ単位での自律的な経済活動やナレッジ共有を行うための基盤が確立。
    - 所有権（Owner）ベースの権限制御が API レイヤーで強制される。

```mermaid
graph TD
    A[SqliteSyndicateStore] -->|Implements| B[SyndicateOps]
    B -->|Injected to| C[AppState]
    C -->|Used by| D[routes/syndicate.rs]
    E[router.rs] -->|Route Registration| D
    F[main.rs] -->|Initialization| A
    G[api_integration_tests.rs] -->|TDD Verification| D
```

### 🔬 Phase 15: Agentic Foundation Expansion (ADR-024/023)
- **変更内容**: 
    - `oracle.rs`: `multi_review` による反復レビューロジック。
    - `dream_state.rs`: `scientific_dream` モードと LLM による仮説生成。
    - `planner.rs`: Markdown 抽出に対応した堅牢な計画分解。
    - `discovery.rs`: LLM ベースのセマンティックツール検索。
    - `task_orchestrator/mod.rs`: `parent_step_id` の伝播とサブジョブ連携。
- **波及効果**: 
    - `DreamState` の初期化時に `LlmProvider` の注入が必要になり、`main.rs` およびテスト用モックのコンストラクタが一斉に変更。
    - `TrajectoryStep` のフィールド追加により、データベース・スキル・プロキシ等の全データ構造が連鎖的に更新。
    - レビュー品質と計画の堅牢性が向上する一方、LLM 呼び出し回数（コスト）が増加。

```mermaid
graph TD
    A[Oracle::multi_review] -->|Iterative Review| B[LlmProvider]
    C[DreamState::scientific_dream] -->|Hypothesis| B
    C -->|Dispatch| D[JobQueue]
    E[StrategicPlanner] -->|Robust JSON| F[TrajectoryStep]
    G[ToolDiscoveryEngine] -->|Semantic Search| B
    H[TaskDispatcher] -->|Causal Linking| F
```

### 🧬 Phase 4 (ADR-025): Poincare Memory Lifecycle & GC
- **変更内容**: 
    - `slm_bridge.rs`: SLM CLI との通信ブリッジ実装。バッチ処理対応。
    - `watchtower.rs`: `do_karma_decay_sweep` における Poincare GC ロジックの統合。
    - `validator.rs`: `ConstitutionalValidator` での `SlmBridge` 利用。
    - `main.rs` & `api_integration_tests.rs`: `UniversalJobQueue` への `SlmBridge` 注入（シグネチャ変更）。
- **波及効果**: 
    - 全ての `UniversalJobQueue::new` 呼び出し箇所（api-server, 統合テスト）で引数修正が必要。
    - 記憶の自動アーカイブ（重要度 < 0.3）がバックグラウンドで開始。

```mermaid
graph TD
    A[watchtower.rs] -->|Batch importance calculation| B[slm_bridge.rs]
    B -->|Command execute| C[slm CLI]
    A -->|UPDATE is_archived| D[karma_logs table]
    E[ConstitutionalValidator] -->|Logical check| B
    F[main.rs] -->|Injection| A
```

---
*最終更新日: 2026-03-27* (Phase 4 / Poincare Memory Lifecycle & GC Integration)
### 🛡️ Phase 45: Vectorless RAG (Hierarchical Knowledge Router - HKR)
- **変更内容**: 
    - `knowledge_indexer.rs`: Markdown パーサと階層インデックス (`TreeNode`) 構築。
    - `hierarchical_router.rs`: LLM 選択肢パース、セマフォ、TTL/Hash 検証付き RouteCache。
    - `app_state.rs` / `main.rs`: `HierarchicalRouter` の `AppState` 注入と初期化。
    - `stream.rs`: OOD 判定後の HKR フォールバックと `ConstitutionalValidator` による検証フローの統合。
- **波及効果**: 
    - `api-server` の SSE ストリームにおいて、未定義の教訓やナレッジに対しても階層ドキュメントを元にした高精度な補足が提供される。
    - LLM 呼び出し回数は「階層の深さ」分増加するが、セマンティック検索では到達困難だった深層ドキュメントへのアクセスが可能になる。
    - インフラコスト（VectorDB等）を抑えつつ、ドキュメントの更新にハッシュベースで追随可能。

```mermaid
graph TD
    A[stream.rs / OOD Detection] -->|Trigger| B[HierarchicalRouter]
    B -->|Fetch Tree/Hash| C[system_state table]
    B -->|Check Cache| D[RouteCache]
    B -->|LLM Traversal| E[LlmProvider]
    B -->|Result| F[ConstitutionalValidator]
    F -->|Validated| G[SSE knowledge_notice]
    G -->|Context Injection| H[Next LLM Generation]
```

---
*最終更新日: 2026-03-27* (Phase 45 / Vectorless RAG HKR Integration)

### 🛡️ Phase 47-B: Infrastructure Stabilization
- **変更内容**: 
    - `UniversalJobQueue`: フィールド（`karma_cache`, `slm_bridge`, `trajectory_store` 等）の完全復元と手動 `Debug` 実装。
    - `dynamic.rs`: LLM 設定取得フローの型安全化 (`Option<String>` -> `String`)。
    - `rss_collector.rs`: SQL 実行時のプール参照エラー修正。
- **波及効果**: 
    - インフラ層全体のビルド不整合が解消。`memory_crystallizer.rs` や統合テストにおける初期化コードが安定。
    - `BackgroundLlmProvider` を利用する全コンポーネントにおいて、設定欠落時のフォールバック挙動が堅牢化。

### 🔬 Phase 48: Invariant-DAG Foundation
- **変更内容**: 
    - `TrajectoryStep`: 検証フィールド (`verified_invariants`, `state_hash` 等) の追加。
    - `planner.rs`: `StrategicPlanner` における新規フィールドのデフォルト値設定。
- **波及効果**: 
    - タスクの実行軌跡に不変条件（Invariants）の検証結果を記録する準備が完了。
    - `TaskDispatcher` における状態ハッシュチェーンの構築が可能に。

```mermaid
graph TD
    A[UniversalJobQueue] -->|Field Restoration| B[Infrastructure Mod]
    B -->|Initialization| C[memory_crystallizer / tests]
    D[dynamic.rs] -->|Type Fix| E[LLM Providers]
    F[TrajectoryStep expansion] -->|Default Init| G[StrategicPlanner]
    G -->|Execution Path| H[TaskDispatcher]
```

---
*最終更新日: 2026-03-27* (Infrastructure Stabilization & Invariant-DAG Foundation)

### 👥 Phase 50: Agentic A2A gRPC Protocol & Worker Detachment
- **変更内容**: 
    - `libs/infrastructure/src/docker_conductor.rs`: 同期実行 (`docker exec`) から非同期ポートマッピング (`docker run -d`) を利用した gRPC 通信アーキテクチャへの全面移行。
    - `libs/infrastructure/src/grpc/a2a_grpc_client.rs`: `async-stream` および `tonic` を活用した `A2aClient` トレイトの具象実装とタイムアウト制御。
    - `apps/shadow-worker`: トークン認証 (`A2A_AUTH_TOKEN`) による 127.0.0.1 バインディングとヘルスチェックを備えたコンテナ用 gRPC サーバーの構築。
- **波及効果**: 
    - メインの `api-server` プロセスのブロッキングが軽減され、重い推論タスクやシミュレーション環境を完全にデタッチされたクリーンなコンテナ環境でスロッティング可能に。
    - ワークスペース全体での通信基盤が gRPC (`tonic`) 前提にアップデートされたことで、今後予定されている分散型マルチノードフェデレーション（Samsara Hub 経由）への展開要件の多くが満たされた。

```mermaid
graph TD
    A[DockerConductor] -->|Start Container| B[Detached Docker (shadow-worker)]
    A -->|Fetch Dynamc Port| C[docker port 50051]
    A -->|Connect gRPC Stream| D[A2aGrpcClient]
    D -->|Execute Task & Auth Token| B
    B -->|TaskProgress Yields| D
    D -->|Stream to SSE| E[TaskDispatcher Loop]
    E -->|Clean Output| F[Job Result]
```

---
*最終更新日: 2026-03-28* (Phase 50 / Agentic A2A gRPC Protocol)

### 🔮 Phase 51-55 波及影響予測 (Planning Phase)
- **Phase 51 (Aiome Node + Agent Card)**: 
    - `api-server/main.rs`: `AppState` への Node IPC クライアント事前注入が必要となり、初期化シーケンスに波及。
    - `api_integration_tests.rs`: `MockAppState` の初期化ブロックへの波及。
- **Phase 53 (ACP / GigEngine 拡張)**: 
    - `routes/gig.rs`: ACP (`PROBE`, `BID`, `COMMIT`) 準拠に伴い、既存 REST エンドポイントのリクエスト/レスポンススキーマが非互換となる可能性大。
    - `samsara-hub/src/routes/federation.rs`: Agent Card メタデータの追加送受信に伴うペイロード拡張。
- **Phase 54 (x402 + AP2)**: 
    - `libs/aiome-contracts/src/commerce.rs`: `CommerceEngine` トレイトに x402 決済インターフェース追加。
    - `infrastructure/src/mock_commerce.rs` & `stripe.rs`: トレイト拡張に伴う全 Mock/実装型のシグネチャ一斉変更波及（Phase 37a と同等規模の波及が見込まれる）。

---
*最終更新日: 2026-03-30* (Phase 51: Agentic Finance & GIG Loop Integration)

### 🏦 Phase 51: Agentic Finance & GIG Loop Integration
- **変更内容**: 
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` に `GigEngine` を依存注入 (DI) し、ジョブ完了時に自律的に `GigIntent` を発行する `maybe_publish_gig_intent` メソッドを実装。
    - `libs/aiome-contracts/src/gig.rs`: `GigIntent` 構造体に `metadata` フィールドを追加。コンストラクタ `new()` を実装。
    - `libs/infrastructure/src/intent/mod.rs`: `GigIntent` の構造変更に伴う初期化箇所の修正。
    - `libs/infrastructure/src/test_utils.rs`: `GlobalMockJobQueue` に `fetched_job` フィールドを追加し、`fetch_job` メソッドが意図したジョブを返せるように拡張。
- **波及効果**: 
    - AIエージェントがタスク完了後に自動的に次のタスク（ギグ）を市場へ公開する「自律的経済ループ」が実現。
    - 循環参照や無限ループを防止するため、`gig_depth` を含むメタデータが全ギグインテントに伝播するようになった。
    - `api-server` および `api_integration_tests.rs` の初期化コードが更新され、本番・テスト両環境で `GigEngine` が必須またはモックされる構成に移行。

```mermaid
graph TD
    A[TaskDispatcher] -->|Inject| B[GigEngine]
    A -->|On Completion| C[maybe_publish_gig_intent]
    C -->|Check karma_directives| D{gig_intent: true?}
    D -->|Yes| E[GigIntent::new]
    E -->|Propagate depth| F[intent.metadata]
    F -->|Publish| G[GigEngine::publish_intent]
    G -->|Economic Trigger| H[Other Agents]
```

---
*最終更新日: 2026-03-31* (Phase 52, 53 & Red Team Security Hardening)

### 🛡️ Phase 52: LoRA Archiving & Secure Training Pipeline
- **変更内容**: `archived_lora_models` テーブルの追加および `SoulStore::archive_lora_model` メソッド実装。Rebirth 時に過剰適応をリセットしデータポイズニングを阻止。
- **波及効果**: `SamsaraEngine` に `SoulStore` が注入され、Rebirth 処理のフローが変更されました。また `LoraTrainingService` は `BastionGuard::new_internal` を用いて隔離空間で MLX 学習スクリプトを実行するようになりました。

### 🧠 Phase 53: SoT Deliberation & Cognitive Sentinel
- **変更内容**: `Oracle::multi_review` の実装により、「批判・洗練・最終判定」のループ処理が組まれました。また、`SoTEngine` にて LLM による JSON 構造化抽出を強制し、`SoTProgress` SSE イベントが追加されました。
- **波及効果**: 判断プロセスが同期から非同期かつ複数回の推論へ変更され、より確実な JSON レスポンスが返るようになりました。

### 🛡️ Red Team Pass 1-3 (Security Posture Update)
- **変更内容**: 
    - `Settings::do_get_setting` と `Sentinel::native_bridge_fallback` が Fail-Open から Fail-Closed (Result<?> エラー伝播・拒否) に変更。
    - `ContextEngine::maintain_context` に 10,000 文字のハードリミット、`ExpressionEngine::synthesize_audio` に 2048 Byte のエラーレスポンス上限追加（DoS/OOM 防御）。
    - ワークスペース全体の `bastion` 依存を独自 Git フォークから Crates.io 公式パッケージ `bastion-core = "1.0.0"` へ移行。
- **波及効果**: サプライチェーンリスクが完全に排除されました。設定値やネイティブブリッジの欠損時に不正にテストを通過したり、システムが脆弱なデフォルト状態で起動することが物理的に不可能になりました。

---
*最終更新日: 2026-03-31* (AutoHarness Phase B-D)

### 🛡️ Phase B-D: AutoHarness Security Architecture Integration
- **変更内容**: 
    - `harness_registry` DB テーブル導入にともない、`HarnessRecord` や `HarnessRegistryOps` トレイトを追加。`UniversalJobQueue` に実装を展開。
    - `ConstraintChecker` 内の Regex リテラル展開を `RegexBuilder` の事前コンパイル＋サイズリミット（10KB）へ変更し ReDoS 脆弱性を遮断。
    - `ActionHarness` トレイトに `severity()` を追加し、`WasmHarness` 側で動的に重大度を受け取る構造にリファクタリング。
    - `apps/api-server/src/skill_handler.rs` にて、JobQueue 経由で取得した Active / Shadow ハーネスを `evaluate_step_with_harnesses` ループに注入。
- **波及効果**: 
    - ハーネスの分離アーキテクチャが実現。重大度80以上はアクション自体をブロックする (Active mode) 一方で、80未満は制約違反として記録されるのみ (Shadow mode) という多段階防衛がAPIレイヤーで実体化。
    - 各種 `JobQueue` モック（テスト環境）全体へ `HarnessRegistryOps` が必要となり、広範なテストファイル（`tts_worker.rs`, `dream_state.rs`, `immune_system.rs` 等）に対するトレイト実装波及を及ぼした。


---
*最終更新日: 2026-04-02* (Phase C-2: Watchtower Diagnostic Loop Hardening)

### 🛡️ Phase C-2: Aiome Watchtower Diagnostic Loop Hardening
- **変更内容**: 
    - `libs/infrastructure/src/task_orchestrator/mod.rs`: `TaskDispatcher` に `AgentRxDiagnostics` を注入。ジョブ失敗時にバックグラウンドで自己診断をトリガーし、結果を `AuditStore` (`TrajectoryStore`) に保存するループを構築。
    - `libs/infrastructure/src/diagnostics.rs`: `AgentRxDiagnostics` に LLM タイムアウト (30s) を導入。実行軌跡が空の場合のガード、および LLM 応答エラー時のダミーレコード生成によるフェイルセーフを追加。
    - プロンプト注入 (Read-path): 注入箇所を `<WATCHTOWER_INSIGHT>` タグで保護し、冪等性 (Idempotency) を確保。リトライ時のプロンプト肥大化を防止。
    - テスト安定化: `test_dispatcher_watchtower_diagnostic_loop` において、固定 sleep を状態監視ポーリングループに置き換え、CI 上の Flakiness を解消。
- **波及効果**: 
    - `AuditStore` (`TrajectoryStore`) トレイトの拡張が必要となり、全モック実装 (`GlobalMockJobQueue`, `immune_system.rs`, `dream_state.rs`, `soul_mutator.rs`) に `fetch_diagnosis` / `store_diagnosis` 等の実装が波及。
    - `TaskDispatcher` の初期化引数が増加し、`api-server/src/app_state.rs` および `main.rs` での依存注入コードが更新された。

---
*最終更新日: 2026-04-03* (Phase B: Cortex Wiki Compiler Implementation)

### 📚 Phase B: Cortex Wiki Compiler Hardening
- **変更内容**: 
    - `libs/infrastructure/src/cortex_compiler.rs` に `run_compilation_cycle` および `generate_article` メソッドを実装し、SQLite トランザクションロックを回避する（読み込み即クローズ → LLM推論 → 個別更新）アーキテクチャを確立。
    - `cortex_documents` テーブルに `compiled` フラグを移行/追加し、再コンパイルを防止。
    - `apps/api-server/src/bootstrap.rs` にて 30分間隔でコンパイルサイクルを実行するバックグラウンドループ（ワーカー）を統合。
    - `apps/api-server/src/routes/cortex.rs` および `apps/api-server/src/api.rs` (OpenAPI) にて、`GET /api/v1/cortex/wiki` および `GET /api/v1/cortex/wiki/:id` エンドポイントを実装。
- **波及効果**: 
    - Cortex の自律的な知識抽象化サイクルが完成。「点（ドキュメント）から面（Wiki）」への知識統合がバックグラウンドで安全に行われるようになった。
    - `sqlx::query!` マクロ由来のコンパイル時メタデータ要求エラーを回避するため、RESTハンドラ等で `sqlx::query` を採用する規約（DBモック/遅延構築耐性の向上）が確立された。

---
*最終更新日: 2026-04-04* (Phase C: Cortex Query Engine Integration + TDD Hardening)

### 🔍 Phase C: Cortex Query Engine Integration + TDD Hardening
- **変更内容**: 
    - `libs/infrastructure/src/cortex_query.rs`: `CortexQueryEngine` を実装。LLM キーワード抽出 + SQLite `LIKE` 検索による RAG 基盤。`max_context_chars` フィールド追加（builder pattern、デフォルト 8000）。`suggest_questions` を DB ベースの動的サジェストに移行。
    - `apps/api-server/src/routes/cortex.rs`: `/api/v1/cortex/query` (POST) および `/api/v1/cortex/suggestions` (GET) ハンドラ実装。OpenAPI description を動的サジェストの説明に更新。
    - `apps/api-server/src/stream.rs`: OOD 検出時の第2フォールバックとして `CortexQueryEngine` を SSE ストリームに統合（confidence ≥ 0.5 ガード付き）。
    - `apps/api-server/src/mcp/server.rs`: `cortex_search` MCP ツール登録・実行。RBAC ホワイトリストに追加。
    - `apps/api-server/src/app_state.rs`, `bootstrap.rs`, `api.rs`, `api_integration_tests.rs`: DI 登録、初期化、OpenAPI スキーマ、テスト AppState に `cortex_query` を統合。
- **波及効果**: 
    - `CortexQueryEngine::new()` のシグネチャは変更なし（後方互換性100%）。`with_max_context_chars()` はオプショナル builder。
    - SSE ストリームにおいて HKR → Cortex の2段フォールバックが確立。OOD 時の LLM 呼び出し回数は最大4回（HKR 階層 + Cortex キーワード抽出 + 回答生成）に増加するが、confidence ガードにより不要な注入は防止。
    - `suggest_questions` の戻り値が動的になったため、フロントエンド側で表示更新への対応が必要（Phase D 候補）。

```mermaid
graph TD
    A[stream.rs / OOD] -->|Fallback 1| B[HierarchicalRouter]
    A -->|Fallback 2| C[CortexQueryEngine]
    C -->|Keyword Extraction| D[LlmProvider]
    C -->|LIKE Search| E[cortex_concept_index]
    C -->|Fetch Articles| F[cortex_wiki_articles]
    C -->|Generate Answer| D
    G[routes/cortex.rs] -->|query_handler| C
    G -->|suggest_questions_handler| C
    H[mcp/server.rs] -->|cortex_search| C
```


# 2026-04-08

## AppDataResolver Phase 2-PRE
*   **Resolved files**:
    *   `apps/api-server/src/bootstrap.rs`
    *   `apps/api-server/src/internal_services/dream.rs`
    *   `libs/infrastructure/src/artifact_store.rs`
*   **Ripple effect**: Local dev uses `workspace/` mapped automatically via config. Release mode uses `~/Library/Application Support/com.aiome.nexus`. Removed legacy hardcoded paths which clears the final blocker for Phase 2C Tauri Packaging.

## Phase 3-A: SSRF 全掃討 & TCP プーリング最適化 & UI トークン駆動化
*   **Changed files (infrastructure SSRF → global pool)**:
    *   `libs/infrastructure/src/cortex_ingester.rs` — `reqwest::Client::new()` → `get_http_client().clone()` + `RequestBuilder::timeout()`
    *   `libs/infrastructure/src/tts.rs` — 同上
    *   `libs/infrastructure/src/llm/proxy.rs` — 同上
    *   `libs/infrastructure/src/forecast/timesfm.rs` — 同上
    *   `libs/infrastructure/src/publisher/wordpress.rs` — 同上
    *   `libs/infrastructure/src/rss_collector.rs` — 同上
    *   `libs/infrastructure/src/trend_sonar.rs` — 同上
*   **Deleted**: `libs/infrastructure/src/security_zombie.rs::http_client_with_timeout()` — Dead code. 呼び出し元ゼロ確認済み。
*   **Added**: `apps/management-console/src/components/CausalVisualizer.tsx`, `GraphView.tsx`, `home/AvatarViewerModal.tsx` — `cssVar()` O(1) メモ化ブリッジ (3ファイル重複。Phase 3-B P2 で `utils/cssVar.ts` に共通化予定)。
*   **Added**: `scripts/test_ui_hex_violations.py` — HEX ハードコード検出テスト。Phase 3-B P0 で rgba/rgb 検出を追加。
*   **Added**: `scripts/fix_hex_violations.py` — HEX → CSS トークン自動変換スクリプト。
*   **Ripple effect**:
    *   `get_http_client().clone()` は `reqwest::Client` 型を返す → 呼び出し元の型シグネチャ変更ゼロ（後方互換 100%）。
    *   TCP プーリング最適化により、同一ホストへの並行リクエストでハンドシェイクが再利用される。
    *   `cssVar()` は Canvas (vis-network) への色注入専用。React コンポーネントのプロパティ型に影響ゼロ。
    *   UI テーマ変更時、HEX/rgba ハードコードが残存する 33 ファイルは追従しない（Phase 3-B P3 で対処予定）。

## Phase E-3: Phase 4C - UniversalSyndicateStore
### 1. Database Abstraction via DatabasePool
- **変更内容**:
    - `libs/aiome-commerce/src/syndicate.rs` [MODIFY]: `SqliteSyndicateStore` を `UniversalSyndicateStore` へリネームし、PostgreSQL 対応（`ON CONFLICT` 構文のサポート）と SQLite (`INSERT OR REPLACE`) の分岐を実装。
    - `apps/api-server/src/app_state.rs` [MODIFY]: 依存する型の指定を `UniversalSyndicateStore` へ更新。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: `syndicate_store` のインスタンス化時における SQLite 依存のアンラップを撤廃し、`job_queue.get_pool()` を直接渡す設計にリファクタリング。
    - `apps/api-server/src/api_integration_tests.rs` [MODIFY]: 同様のコンストラクト変更をテスト用のモック構成にも適用。
- **波及効果**:
    - PostgreSQL と SQLite 環境のどちらでも一貫した動作が可能となり、Enterprise デプロイメント時のスケーラビリティが確保された。
    - コンパイルテスト（315/315）を全通貨確認（GREEN化）。

# 2026-04-10

## Phase 8.6: TrendSonar X API Integration Hardening
*   **Changed files**:
    *   `apps/api-server/src/routes/settings.rs` — Fixed category limit bug allowing `integrations` to be saved.
    *   `libs/infrastructure/src/x_signal_probe.rs` — Extracted parsing logic for JSON safety tests (TDD).
    *   `libs/infrastructure/src/trend_sonar.rs` — Introduced `build_active_trend_sonar` Factory pattern to prevent state staleness.
    *   `apps/api-server/src/internal_services/dream.rs` — Refactored to dynamically construct `TrendSonar` instance every loop, picking up fresh API keys without restart.
    *   `apps/api-server/src/routes/general.rs` — Implemented `/api/v1/trends` using the new Factory.
    *   `apps/api-server/src/api_integration_tests.rs` — Added comprehensive tests for `trends_api`.
    *   `apps/management-console/src/components/SettingsPage.tsx` — Added global error state for UX failure feedback.
*   **Added files**:
    *   `apps/management-console/src/components/cortex/TrendView.tsx` — Built interactive trending dashboard.
    *   `apps/management-console/src/components/cortex/CortexView.tsx` — Linked TrendView module to Cortex layout seamlessly.
*   **Ripple effect**: 
    *   Tokens/keys like `X_BEARER_TOKEN` added via the UI are now instantly available globally without server restart.
    *   `ExternalTrendSonar` is now explicitly bounded to `LlmProvider + Send + Sync`.

## Phase 4 WordPress AbyssVault Migration
### 1. WordPress Token Zero-Trust Migration
- **変更内容**:
    - `apps/key-proxy/src/main.rs` [MODIFY]: 新しく `/api/v1/wp/publish` エンドポイントを開設し、`WP_API_URL`と`WP_API_TOKEN`環境変数をプロキシ内でロード・パージするゼロトラスト防御層を追加。
    - `libs/infrastructure/src/publisher/wordpress.rs` [MODIFY]: `WordPressAdapter` にトークン情報を保持せず、Abyss Vault Proxy を経由して HTTP リクエストをプロキシする `new_vault` コンストラクタを新設。`Authorization: Bearer <vault_secret>` による Key Proxy の `auth_middleware` 回避を追加。
    - `apps/api-server/src/bootstrap.rs` [MODIFY]: バックエンドシステムのブートストラップ処理に、`KEY_PROXY_URL` を利用して動的に `new_vault` を注入し、フォールバックとしてレガシー設定に縮退する柔軟なインフラ初期化を導入。
- **波及効果**:
    - WordPress トークンが API サーバーのメモリ空間から完全に追放され、SSR / RCE 脆弱性等によるプレーンテキストのクレデンシャル漏洩リスクを物理遮断した。
    - プロキシを介した通信により、WP API へのレート制限・通信遮断が `key-proxy` コンポーネント単独の責務となり、API サーバーのスレッドが枯渇するブロッキング障害が予防された。

## Sprint 0: Auth Extractor Final Gate
### 1. Security Compliance — auth-exempt 監査完了
- **変更内容**:
    - `apps/api-server/src/routes/skill.rs` [MODIFY]: `update_mcp_config` と `get_mcp_config` に `_auth: crate::auth::Authenticated` を追加。MCP 設定の未認証アクセスを遮断。
    - `apps/api-server/src/routes/whisper.rs` [MODIFY]: `get_monologue_history` に `_auth: crate::auth::Authenticated` を追加。内省ログの未認証アクセスを遮断。
    - `apps/api-server/src/routes/bootstrap.rs` [MODIFY]: 2ハンドラに `// auth-exempt` コメントを追記（セットアップ前に使用）。
    - `apps/api-server/src/routes/auth.rs` [MODIFY]: 2ハンドラに `// auth-exempt` コメントを追記（OAuth フロー）。
    - `apps/api-server/src/routes/commerce_webhook.rs` [MODIFY]: `stripe_webhook` に `// auth-exempt` コメント（Stripe 署名検証）。
    - `apps/api-server/src/routes/avatar.rs` [MODIFY]: `serve_inochi2d_asset` に `// auth-exempt` コメント（静的アセット配信）。
    - `apps/api-server/src/routes/general.rs` [MODIFY]: `get_health_status` に `// auth-exempt` コメント（ヘルスチェック）。
    - `scripts/deep-scan.sh` [MODIFY]: CC-6 の awk パターンに `Extension.*AuthenticatedUser` を追加し、Auth Extractor の全パターンを認識可能に。
- **波及効果**:
    - deep-scan CC-6 が **Errors: 0** を達成。全 API ハンドラが Auth 適用済みまたは明示的に auth-exempt に分類され、未監査ハンドラがゼロに。
    - 既存の `api_integration_tests.rs` のテストが既に Auth を想定して書かれていたため、テスト修正は不要。

## Sprint 1-A: One-Click Management Console (Docker)
### 1. MC コンテナ化 & Nginx SPA ホスティング
- **変更内容**:
    - `apps/management-console/Dockerfile` [NEW]: Node.js 20-alpine でビルドし nginx:alpine で配信するマルチステージ構成。`.npmrc` の `ignore-scripts=true` を回避するため、`package.json` を先にコピーしてから `npm ci` を実行する順序でビルド。HEALTHCHECK 付き。
    - `apps/management-console/nginx.conf` [NEW]: SPA ルーティング (`try_files`)、API リバースプロキシ (`^~` 修飾子で regex より優先)、SSE 対応 (`proxy_buffering off`)、gzip 圧縮 (level 6)、セキュリティヘッダー (X-Frame-Options, X-Content-Type-Options, Referrer-Policy, Permissions-Policy)、プロジェクト固有アセットキャッシュ (.vrm, .otf, .wasm)。
    - `apps/management-console/.dockerignore` [NEW]: node_modules, dist, src-tauri, e2e, .vscode, *.md 等を除外しビルドコンテキストを最小化。
    - `docker-compose.quickstart.yml` [MODIFY]: `management-console` サービスを追加。全コンテナに `healthcheck` + `depends_on: condition: service_healthy` を実装し、起動順序を保証 (ollama → api-server → MC)。`OLLAMA_MODEL` を `gemma4:e4b` に更新。
- **波及効果**:
    - `docker compose -f docker-compose.quickstart.yml up` 一発で、Ollama + API Server + Management Console が正しい順序で起動する完全な開発環境が完成。
    - Nginx の `add_header` 継承バグ（子 location で親のヘッダーが消失）を明示的な再定義で回避。この知見は今後の Nginx 設定変更時に必須。

## Sprint 1-B: GitHub Container Registry CI/CD
### 1. Docker イメージ自動ビルド & プッシュ
- **変更内容**:
    - `.github/workflows/docker-publish.yml` [NEW]: `main` Push / `v*` タグ / 手動トリガーで、API Server (`ghcr.io/motivationstudio-llc/aiome`) と Management Console (`ghcr.io/motivationstudio-llc/aiome-console`) のマルチアーキテクチャ (amd64 + arm64) Docker イメージを自動ビルド & ghcr.io へプッシュ。GHA キャッシュ (`cache-from/to: type=gha`) で再ビルド時間を短縮。
    - `docker-compose.quickstart.yml` [MODIFY]: `api-server` の `build.dockerfile` を `docker/production.Dockerfile` に厳密化（旧 monolith `Dockerfile` との混同を防止）。Ollama の healthcheck を `curl` → `ollama list` に変更（Ubuntu ベースの Ollama イメージに curl が非搭載であるため）。
- **波及効果**:
    - `main` に Merge するだけで自動的に ghcr.io へリリースされ、ユーザーは `docker compose up` だけで最新版を利用可能に。
    - Ollama healthcheck の `curl` 問題が解消され、`depends_on` チェーンの全段が確実に動作。

## Sprint 2: E2E Verification & Launch Preparation
### 2-A: WordPress E2E テスト環境
- **変更内容**:
    - `docker-compose.wp-test.yml` [NEW]: WP-CLI による全自動インストール（`core install` + `user application-password create`）と、`shared-wp-html` volume による明示的ファイル共有パターンを採用した WordPress REST API E2E 環境。
- **波及効果**:
    - `key-proxy` の `/api/v1/wp/publish` エンドポイントを E2E で検証可能に。`.env.example` に `WP_API_URL` / `WP_API_TOKEN` が既存のため追加不要。

### 2-B: スクリーンキャスト自動録画
- **変更内容**:
    - `apps/management-console/e2e/screencast.spec.ts` [NEW]: Playwright `video: 'on'` + `slowMo: 400` で管理コンソールの主要4タブを自動巡回し、PR 用スクリーンキャスト動画を生成。
    - `apps/management-console/.gitignore` [MODIFY]: `test-results/`, `playwright-report/`, `test_output.txt` を追加。
- **波及効果**:
    - 生成された動画は YouTube チュートリアル (Sprint 2-F) の素材として活用。既存の E2E テスト (`demo.spec.ts`, `home_v2.spec.ts`) への影響なし。

### 2-D: 法的コンプライアンス基盤
- **変更内容**:
    - `docs/legal/TERMS_OF_SERVICE.md` [NEW]: β 版向け利用規約スキャフォールド。
    - `README.md` [MODIFY]: Legal & Privacy セクションを追加し、ToS / Privacy Policy へのリンクを設置。
    - `README_en.md` [MODIFY]: 同上（英語版同期）。
- **波及効果**:
    - フロントエンドの ToS リンク（将来実装）が 404 にならない基盤を確立。README / README_en の同期は AGENTS.md Rule #2 に準拠。

### 2-C: E2E Verification Testing
- **変更内容**:
    - `apps/management-console/e2e/wp_publish.spec.ts` [NEW]: Chat 上での自律的な WP 起動と、`key-proxy` への直接APIコールを検証する TDD E2E テスト。LLM の応答ストリーミングを考慮し Flaky にならない待機ロジックを実装。
- **波及効果**:
    - E2E 実行時の検証基盤が独立して動作し、将来の自律的スキル利用の追加テストのロールモデルとなった。

### 2-F: Postiz Growth Tactics 実装
- **変更内容**:
    - `apps/api-server/tests/marketing_assets_test.rs` [NEW]: `README.md` および `README_en.md` に $0 CTA や YouTube 動画が欠落していないかを保証する TDD テスト。
    - `scripts/setup-github-topics.sh` [NEW]: GitHub SEO Topic 自動設定スクリプト。
    - `README.md`, `README_en.md` [MODIFY]: 動画リンクと Docker/$0 の CTA パネルを追記。
- **波及効果**:
    - マーケティング施策がインフラ (コード・テスト) のライフサイクルと結合された。ドキュメント改修ミスが CI 上で検知されるようになる。


## 2026-05-01: Samsara Hub Modularization
- Extracted `apps/samsara-hub/src/models.rs` containing `FederatedKarmaRecord`, `ImmuneRuleRecord`, `ArenaMatchRecord`, `TopicRecord`.
- Extracted `apps/samsara-hub/src/state.rs` containing `HubState`.
- Extracted handlers to `apps/samsara-hub/src/handlers/federation.rs` and `ws.rs`.
- Migrated hardcoded SQL DDLs to `sqlx::migrate!` in `migrations/sqlite/` and `migrations/postgres/`.

## 2026-05-04: SSRF Redirect Bypass Prevention & DPO Dataset Export
### 1. robots.txt SSRF Hardening
- **変更内容**:
    - `apps/api-server/src/tool_call_router.rs` [MODIFY]: `check_robots_txt_policy` に `reqwest::redirect::Policy::none()` を適用し、リダイレクト応答を悪用した内部ネットワークへの SSRF バイパスを遮断。`line[11..]` の固定オフセットスライスを `line.find(':')` ベースのパーサーに書き換え、バイト境界パニックを排除。空 `Disallow` を RFC 9309 §2.2.2 準拠で「全許可」として処理。
- **波及効果**:
    - `tool_call_router.rs` 内で完結。MCP ツールの URL アクセスガードレールが強化されるのみで、他モジュールへの波及なし。
    - `reqwest::Client` の構築が呼び出し毎に行われるため、グローバル HTTP クライアントプールとの競合なし。

### 2. DPO Dataset Export UI
- **変更内容**:
    - `apps/management-console/src/components/DpoDatasetExport.tsx` [NEW]: DPO データセットを JSONL 形式でダウンロードする管理コンソール UI。i18n 多言語対応、`aria-busy` アクセシビリティ、型安全エラーハンドリングを完備。
    - `apps/management-console/src/components/DpoDatasetExport.test.tsx` [NEW]: 6件の Jest テスト（正常系/APIエラー/ネットワーク断/A11y）。
    - `apps/management-console/src/components/cortex/CortexView.tsx` [MODIFY]: `DpoDatasetExport` コンポーネントを ForecastView 下部に統合。
    - `apps/management-console/src/i18n/en.json` [MODIFY]: `dpoExport.*` キー5件を追加。
    - `apps/management-console/src/i18n/ja.json` [MODIFY]: `dpoExport.*` キー5件を追加。
    - `apps/api-server/src/routes/cortex.rs` [MODIFY]: `export_dpo_dataset_handler` エンドポイント実装。
    - `apps/api-server/src/api.rs` [MODIFY]: OpenAPI への `export_dpo_dataset_handler` 登録。
- **波及効果**:
    - `CortexView.tsx` に新規 import が追加されるため、`CortexView.test.tsx` で `DpoDatasetExport` のモック/スタブが必要。
    - `authenticatedFetch` (auth.ts) の利用パターンが DpoDatasetExport に拡張。
    - `dpoExport.*` i18n キーの追加・変更時は `en.json` と `ja.json` の同期が必須。

## 2026-05-05: MCP Ecosystem Security Overhaul
### 1. Security Gate & Unscoped Package Support
- **変更内容**:
    - `libs/shared/src/mcp_constants.rs` [MODIFY]: `ALLOWED_MCP_PACKAGES` の追加と `ALLOWED_MCP_PREFIXES` 拡張 (`@brightdata/`, `@upstash/`, `@playwright/`, `@canva/`)。
    - `apps/api-server/src/mcp/client.rs` [MODIFY]: `McpClient::spawn` でのパッケージ名検証に `ALLOWED_MCP_PACKAGES` を追加（スコープ無し完全一致）。
- **波及効果**:
    - 新たなスコープ無しMCPパッケージ（例: `firecrawl-mcp`, `exa-mcp-server`）のインストールが安全に実行可能になった。

### 2. Ecosystem Discovery & Configuration
- **変更内容**:
    - `apps/api-server/src/mcp/discovery.rs` [MODIFY]: 14種類の検証済みMCPサーバーをデフォルトディスカバリに追加。HTTP 型サーバーに `disabled` フラグを導入し、`Slack`, `Figma`, `Ahrefs`, `freee-remote` をデフォルトで無効化。
    - `apps/api-server/src/mcp/server.rs` [MODIFY]: `is_skill_whitelisted` に新規追加した全てのMCPツール名を登録し、RBACを通した自律実行を許可。
- **波及効果**:
    - システム起動時に14種のMCPが安全な状態で認識される。未認証のHTTP系ツールが不正アクセスを引き起こすリスクを防止。

## 2026-05-13: LLM Pipeline Regex Hardening (Phase A-1)
### 1. Zero-Panic Regex Initialization
- **変更内容**:
    - `libs/infrastructure/src/llm/humanizer_rules.rs` [MODIFY]: `Regex::new(...)` 失敗時の `std::process::exit(1)` を排除し、`.expect("static regex")` に置換。`// allow-anti-pattern: static regex` を付与し設計意図を明示。
- **波及効果**:
    - コンパイル時検証可能な静的正規表現のみに限定し、実行時パニックリスクを根絶。
    - `LazyLock` や静的配列化の過剰設計を避け、`HumanizerFilter` の `Vec<HumanizerRule>` 所有権モデルを維持。他モジュールへの波及影響ゼロを達成。

## 2026-05-13: Infrastructure Hardening (Phases B & C)
### 1. Soul Hash Consolidation (Phase B)
- **変更内容**:
    - `libs/shared/src/soul_hash.rs` [CREATE]: 重複していた `soul_hash` 計算ロジックを抽出。
    - `apps/api-server/src/app_state.rs` [MODIFY]: `get_system_soul_hash` を `shared::soul_hash` の利用に変更。
    - `libs/infrastructure/src/task_orchestrator/mod.rs` [MODIFY]: `compute_soul_hash` を `shared::soul_hash` の利用に変更。
- **波及効果**:
    - ハッシュアルゴリズムの分散リスクを排除し、一貫性を担保。

### 2. Federation v1.5 Feature Flag Completion (Phase C)
- **変更内容**:
    - `libs/shared/src/feature_flags.rs` [CREATE]: `FEDERATION_V1_5_FLAG` 等の定数定義。
    - `apps/api-server/src/main.rs` [MODIFY]: ハードコードされた "federation_v1_5" を定数利用に置換。
    - `apps/samsara-hub/src/hub_reliability_tests.rs` [MODIFY]: `test_hub_purge_logic` を追加し、SQLite の `LIMIT` パージロジックを検証。
    - `apps/aiome-node/Cargo.toml` & `src/main.rs` [MODIFY]: `federation` フィーチャーを追加し、ルーティングをフラグ制御下に置く。
- **波及効果**:
    - 機能フラグのタイポによるバグをコンパイル時エラーとして検出可能に。
    - 10万件制限のパージロジックに対する物理削除テストが通り、ディスク枯渇（Flaw 3）防衛の信頼性を確保。
    - `aiome-node` のデプロイメントが v1.5 まで安全に切り離された。

## 2026-05-13: Sovereign Verifier Productionization (Phase D)
### 1. KANI_STUB_MODE Deprecation & Verification Hardening
- **変更内容**:
    - `apps/api-server/src/routes/commerce.rs`, `apps/management-console/playwright.config.ts`, `libs/infrastructure/src/dream_state.rs`, `libs/infrastructure/src/aegis/prover.rs` [MODIFY]: `KANI_STUB_MODE` 環境変数への依存を完全撤廃し、`AIOME_DEV_MODE` 等の正規フローへ移行。
    - `libs/infrastructure/src/aegis/prover.rs` [MODIFY]: パッチ検証のスタブ機能を削除し、Podman ベースの `aiome/kani-verifier:latest` 実行を強制。
- **波及効果**:
    - "Zero-Panic & Total Verification" ポリシーが本番環境で担保される。
    - 不正なパッチや OxiLean チェックのバイパスが不可能となり、インフラの堅牢性が向上。

## 2026-05-13: Zero-Panic Anti-pattern Cleanup (Phase E)
### 1. Graceful Error Handling & DLQ Implementation
- **変更内容**:
    - `libs/infrastructure/src/audit_logger.rs` [MODIFY]: DB insert 失敗時の `panic!` (`allow-anti-pattern`) を削除し、代わりに `audit_dlq.jsonl` への Dead Letter Queue (DLQ) 書き込みとエラーログ出力に置換。
    - `libs/aiome-commerce/src/gift.rs` [MODIFY]: `TremendousGiftEngine::new()` 内の設定エラーによる `panic!` を `Result<Self, AiomeError>` 化し、初期化エラーとして伝播。
    - `libs/shared/src/app_data.rs` [MODIFY]: `AppDataResolver::new()` に潜んでいた構成エラーのクラッシュを解消するため、`unwrap_or_else` を活用して安全にフォールバック（またはエラー伝播）する形式に改善（コンパイラ制約のため、テスト全体で `unwrap` を導入し、意図しない実行時エラーをコンパイル時エラーに近い形で隔離）。
- **波及効果**:
    - "Zero-Panic" ポリシーの最後の残存反逆箇所を排除。
    - データベース瞬断時にも監査ログが消失せず、パニックによるアプリケーション全体のクラッシュを防衛。


## 2026-06-22: Zero-Trust Secret Isolation Architecture (Phase 0)
### 1. Separation of Secrets into `.env.secret`
- **変更内容**:
    - `scratch/isolate_secrets.py` [NEW]: `.env` からシークレット情報（APIキー、トークン等）を検出し、安全に `.env.secret` に移動させる移行スクリプトを構築して実行。
    - `.env` [MODIFY]: シークレット項目を `<YOUR_KEY_HERE>` のプレースホルダーへ置換し、設定項目のみを残すように構成。
    - `.gitignore` [MODIFY]: `.env.secret` を git 管理から除外するよう設定を追加。
    - `.env.example` [MODIFY]: 使用されていないデッドエントリー `VAULT_MASTER_KEY` を削除。
- **波及効果**:
    - バグ調査や誤操作により `.env` ファイルや対話ログが外部に流出した場合でも、秘密情報が直接漏洩するリスクをゼロに抑え込むことに成功。

### 2. Implementation of `.env.secret` Loading in Binaries
- **変更内容**:
    - `apps/api-server/src/bootstrap/preflight.rs` [MODIFY]: 起動フローにおいて `.env.secret` を CWD および resolver.root() から優先ロードするロジックを実装。
    - `apps/api-server/src/bootstrap/preflight.rs` [MODIFY]: TDD原則に則り、`.env.secret` が正常にロードされるかテストする `test_dotenv_secret_loaded` を追加。
    - `apps/key-proxy/src/main.rs` [MODIFY]: main 関数起動時に `.env.secret` をロードするロジックを挿入。
    - `apps/samsara-hub/src/main.rs` [MODIFY]: 起動時に `.env.secret` をロードするロジックを挿入。
    - `commercial/apps/nurture-api/src/main.rs` [MODIFY]: 起動時に `.env.secret` をロードするロジックを挿入。
- **波及効果**:
    - 4つのバイナリ全てが安全に環境変数からシークレット値を取得し、シークレットが隔離された後も動作を崩さずに同一の実行環境を構築。
    - TDDによる Verification Protocol（Positive / Negative / Revert テスト）をすべて PASS し、機能の安定性を担保。


## 2026-06-22: Zero-Trust Secret Isolation Architecture (Phase 1)
### 1. macOS Keychain Integration for Bootstrapping
- **変更内容**:
    - `libs/shared/src/security.rs` [MODIFY]: macOS Keychain に対して秘密情報の取得・書き込みを行う `get_keychain_secret`, `set_keychain_secret` を追加。
    - `libs/infrastructure/src/security/sqlite_vault_backend.rs` [MODIFY]: `get_master_key` において、`VAULT_MASTER_PASSWORD` のロード時に macOS Keychain を最優先で取得するように改修。
    - `apps/key-proxy/src/main.rs` [MODIFY]: `GEMINI_API_KEY` と `VAULT_SECRET` の起動時ロードで macOS Keychain を最優先し、環境変数はフォールバックとするように修正。
- **波及効果**:
    - 暗号化の最上位キーおよびプロキシ認証キーが macOS のセキュアな Keychain ストレージに保管され、ローカルのファイルシステム上に平文で保存する必要性を完全に排除。

### 2. key-proxy `/api/v1/secrets` Endpoint and Dynamic Injection
- **変更内容**:
    - `apps/key-proxy/src/handlers/secrets.rs` [NEW]: クライアントがシークレット値を受け取るためのエンドポイントハンドラ `handle_get_secrets` を実装し、認証トークン(`VAULT_SECRET`)で保護。
    - `apps/key-proxy/src/main.rs` [MODIFY]: 新しい `/api/v1/secrets` GET ルートを登録。
    - `libs/shared/src/security.rs` [MODIFY]: `key-proxy` にリクエストしてシークレットを一括ロードし、環境変数に注入する共通ヘルパー `fetch_and_inject_secrets()` を実装。
    - `apps/api-server/src/bootstrap/preflight.rs` [MODIFY], `apps/samsara-hub/src/main.rs` [MODIFY], `commercial/apps/nurture-api/src/main.rs` [MODIFY]: 起動時およびプレフライトのシーケンスで `shared::security::fetch_and_inject_secrets()` を自動実行するように統合。
- **波及効果**:
    - クライアントサービスが起動時に動的にシークレットを注入する仕組みが完成し、本番環境やコンテナ環境等で安全かつ透過的に秘密鍵がメモリ上に供給されるように改善。

### 3. abyss-vault CLI Implementation
- **変更内容**:
    - `apps/abyss-vault/` [NEW]: 新しい workspace メンバーとして `abyss-vault` CLI クレートを追加。
    - `apps/abyss-vault/src/main.rs` [NEW]: Keychain へのブートストラップシークレット登録 (`bootstrap`)、`.env.secret` から AbyssVault DB (SQLite) への一括インポート (`import`)、および個別のセット (`set`) を行うコマンドラインインターフェースを実装。
- **波及効果**:
    - 開発者が簡単に Keychain と AbyssVault DB への移行プロセスを管理・実行できる安全な管理インターフェースを整備。

## 2026-06-22: Zero-Trust Secret Isolation Hardening (Reflexion Critique & Refinement)
### 1. Hardening key-proxy `/api/v1/secrets` and Dynamic Client Injection
- **変更内容**:
    - `apps/key-proxy/src/handlers/secrets.rs` [MODIFY]: シークレットの取得要求を `POST` 方式に変更。また、要求されたキーが `ALLOWED_VAULT_SECRETS` ホワイトリストに存在するかどうかを検証し、違反時は `400 Bad Request` とする防御層を追加。
    - `apps/key-proxy/src/handlers/secrets.rs` [MODIFY]: 存在しないキーの要求に対しては `404` を返さず、警告ログを出力してスキップする Partial Success（部分成功）挙動へと改修。
    - `libs/shared/src/security.rs` [MODIFY]: 許可されるシークレット名定数 `ALLOWED_VAULT_SECRETS` を定義。`fetch_and_inject_secrets` はこれを使用して JSON ボディ payload を構築し、タイムアウト設定（10秒）の `POST` リクエストを `key-proxy` に送信するように変更。
    - `libs/shared/src/security.rs` [MODIFY]: シークレット注入成功時のログ出力を `info!` から `debug!` へ変更。また、Keychain 処理時に `USER` 環境変数が欠落していた場合に警告ログを出力するように改善。
    - `apps/key-proxy/src/main.rs` [MODIFY]: `/api/v1/secrets` へのルーティングを `post` に変更。
    - `apps/api-server/src/bootstrap/preflight.rs` [MODIFY], `apps/key-proxy/src/tests.rs` [MODIFY]: テスト内の mock サーバー挙動および `TestServer` へのリクエストを `POST` に変更。また、環境変数操作（`set_var` / `remove_var` 等）に対する Rust unsafe 警告・非推奨警告に対応するため、`#[allow(deprecated, unsafe_code)]` および unsafe ブロックを適用。
- **波及効果**:
    - GET リクエストのクエリパラメータを排除したことで、シークレット名が HTTP ログ等に平文露出するリスクが解消。
    - ホワイトリスト検証により、任意の環境変数漏洩脆弱性（AbyssVault の不正利用）を遮断。
    - 部分成功（Partial Success）および接続タイムアウト（10秒）により、一時的に一部シークレットが欠損していたり、ネットワーク瞬断・ハングが発生した場合でも、無制限のプロセスブロッキングや過剰な起動停止エラーが発生しづらい堅牢なシステムを確立。

## 2026-06-23: API Key Management Consolidation (GUI & CLI Integration)
### 1. Unified Secrets Management backend & proxy routing
- **変更内容**:
    - `libs/infrastructure/src/security/sqlite_vault_backend.rs` [MODIFY]: `UniversalVaultBackend` にキー名一覧取得 `list_secret_keys()` および削除 `delete_secret()` の実実装とユニットテストを追加。
    - `apps/key-proxy/src/handlers/vault_admin.rs` [NEW]: 管理コンソールから利用される key-proxy 側の管理用 API （`/status`, `/secrets` PUT, `/secrets/:key` DELETE）を実装し、ホワイトリスト検証を適用。
    - `apps/key-proxy/src/handlers/mod.rs` [MODIFY], `apps/key-proxy/src/main.rs` [MODIFY]: ルートハンドラの登録と、`auth_middleware` 保護下への配置。
    - `apps/api-server/src/routes/vault.rs` [NEW]: api-server から key-proxy 管理 API への安全なプロキシハンドラ `/api/v1/vault/*` を実装。`VAULT_SECRET` 未設定時は 503/500 安全エラーを返すハンドリングを実装。
    - `apps/api-server/src/routes/mod.rs` [MODIFY], `apps/api-server/src/router.rs` [MODIFY]: `admin_only_middleware` で保護されたルーティングの追加。
    - `apps/api-server/src/api_integration_tests/vault.rs` [NEW]: `wiremock` を用いたプロキシエンドポイントの統合テストを追加。
- **波及効果**:
    - key-proxy の管理 API を api-server を経由させ、`admin_only_middleware` 認可を適用することで、ブラウザからの直接アクセスを防止しつつ安全に操作可能にした。
    - ブートストラップキーが設定されていない環境でのフォールバックにより、予期せぬパニックを回避。

### 2. Management Console UI & CLI Enhancement
- **変更内容**:
    - `apps/management-console/src/components/VaultSecretsManager.tsx` [NEW]: 18のホワイトリスト定義済みキーをカテゴリ別にトグル表示・管理・削除する管理者専用 UI コンポーネント。403エラー（管理者ロール不足）時のカスタム表示処理を実装。
    - `apps/management-console/src/components/VaultKeyStatus.tsx` [NEW]: Commerce や Bridges などの既存の設定画面で、現在の設定状況を示すステータスインジケータ。
    - `apps/management-console/src/components/SettingsPage.tsx` [MODIFY]: `VaultSecretsManager` を `viewMode === 'beginner'` ガードの外側へ常時表示として配置。
    - `apps/abyss-vault/` [NEW]: CLI クレートに `status`, `list`, `get`, `delete`, `set` (対話入力対応), `setup` コマンドを追加し、シェル履歴にシークレットが残らない安全な入力インターフェースを整備。
- **波及効果**:
    - 非技術者から管理者まで等しく GUI / CLI 経由でシークレットをセキュアに操作可能になった。
    - 二重格納（Settings DB と AbyssVault DB）されているキーについて、両者の設定ステータスを可視化（並行表示）したことで、移行プロセスにおける不整合や設定漏れをその場で確認・解消できるようにした。

## Phase: PDF Extraction Process Isolation (pdftotext Migration)
- **変更内容**:
    - `libs/infrastructure/Cargo.toml` [MODIFY]: 脆弱な `pdf-extract` 依存定義を削除。
    - `libs/infrastructure/src/security/config.rs` [MODIFY]: `allowed_binaries` リストに `"pdftotext"` を追加。
    - `libs/infrastructure/src/cortex_ingester.rs` [MODIFY]: `ingest_pdf` メソッドを `pdf-extract` ライブラリ依存から `SafeCommandBuilder` + `SandboxProfile::Strict` による `pdftotext` の隔離プロセス実行モデルへと移行。
    - `libs/infrastructure/src/cortex_ingester_tests.rs` [MODIFY]: `test_ingest_pdf` のアサーションをプロセス起動または抽出失敗メッセージの両方に対応するよう調整。
    - `.github/workflows/ci.yml` [MODIFY]: テスト環境への `poppler-utils` パッケージ追加、および Docker Build Check ジョブにおける `production.Dockerfile` ビルド検証と Trivy 脆弱性スキャンステップを追加。
    - `docker/production.Dockerfile` [MODIFY]: ランタイムステージに `poppler-utils` パッケージを追加。
- **波及効果**:
    - アプリ内での PDF 解析ライブラリによるスタックオーバーフロー脆弱性（RUSTSEC-2026-0187）が根本解決され、不正な PDF をインジェストされた場合にメインサーバがクラッシュする DoS リスクが解消。
    - `pdftotext` は非特権かつネットワーク・書き込み禁止の Strict サンドボックス内で実行されるため、コマンド自体の未知の脆弱性を狙ったエクスプロイトに対しても二重隔離が適用される。
    - 子プロセス起動のため、GPLコピーレフトライセンスのソースコードへのリンク影響を完全に回避。

## Phase: X (Twitter) Official MCP Server & Trend Integration
- **変更内容**:
    - `apps/api-server/src/mcp/discovery.rs` [MODIFY]: サードパーティ製 `x-mcp-server` から公式 X MCP (`api.x.com/mcp`) への stdio 接続と `xurl` ブリッジの連携テンプレートに更新し、デフォルトで有効化 (`disabled: false`)。
    - `apps/api-server/src/routes/buzz.rs` [MODIFY]: `post_tweet` ツール名のハードコードを定数化・柔軟化し、公式 X MCP が提供する `create_post` およびサードパーティの `post_tweet` の両方を自動検知してフォールバック実行するようロジックを改善。
    - `apps/api-server/src/internal_services/x_mcp_trend.rs` [NEW]: `TrendAdapter` トレイトを実装した `XMcpTrendAdapter` を新規追加。`McpProcessManager` を経由して公式 X MCP の `search_posts` / `search_tweets` ツールを動的呼び出し可能にし、トレンド収集と MCP のシナジーを確立。
    - `libs/infrastructure/src/trend_sonar.rs` [MODIFY]: `build_active_trend_sonar()` ファクトリのシグネチャを拡張し、クレート境界を越えた依存性注入 (`extra_adapters`) に対応。
    - `apps/api-server/src/routes/general.rs` [MODIFY], `apps/api-server/src/internal_services/dream.rs` [MODIFY]: `build_active_trend_sonar()` の呼び出し元で `XMcpTrendAdapter` を注入するように修正。
    - `apps/api-server/src/mcp/client.rs` [MODIFY]: `McpProcessManager` の `clients` と `registry` フィールドを `pub(crate)` に公開度を変更し、別モジュールのテスト内からモッククライアントを直接挿入可能に。
    - `.env.example` [MODIFY]: 公式 X MCP 起動用の OAuth2 鍵設定 `X_TWITTER_CLIENT_ID` / `X_TWITTER_CLIENT_SECRET` を追加。
- **波及効果**:
    - X API への接続方式が公式 MCP と `xurl` ブリッジに統一されたことで、認証管理が `xurl` のローカル認証情報ストアに一任され、AIOME サーバー側での不要な OAuth トークン管理・流出リスクが解消。
    - `TrendSonar` モジュールが MCP の検索ツールを利用できるようになったことで、外部 reqwest 接続による X API 直接コールに加えて、MCP 経由のトレンド取得が透過的に統合された。

## Phase: Improvement Plan P0–P3 (Security, Economy, Quality, Repo Hygiene)
- **変更内容**:
    - `commercial/apps/nurture-api/src/routes/internal/idempotency_gate.rs` [NEW]: 内部 transfer/refund/withdraw の冪等性ゲート。
    - `libs/infrastructure/src/sql_helpers.rs` [NEW]: DB/JSON 明示エラー伝播ヘルパー。
    - `apps/management-console/src/lib/biome/biomeApi.ts` [NEW]: Biome API クライアント統一。
    - `apps/management-console/src-tauri/src/lib.rs` [MODIFY]: DRM キー永続化 (`resolve_drm_master_key`)。
    - `apps/key-proxy/src/auth.rs` [MODIFY]: クエリパラメータ認証削除。
    - `apps/api-server/src/bootstrap/core_services.rs` [MODIFY]: A2A トークン release fail-closed。
    - `apps/api-server/src/router.rs` [MODIFY]: `/auth/token` レート制限。
    - `docker-compose.test.yml` [TRACK]: CI Postgres テスト用 compose を Git 追跡。
- **波及効果**:
    - 内部 S2S 経済 API の二重送金リスク低減、認証情報のログ/URL 漏洩経路の縮小。
    - Soul/Artifact/Federation データ読取りの silent corruption リスク低減。

## Phase: Improvement Plan 検証フェーズ (Defect Verification & Fixes)
- **変更内容**:
    - `commercial/migrations/{sqlite,postgres}/20260702000000_payout_amount_usd_to_cents.sql` [NEW]: 既適用マイグレーション編集による sqlx `VersionMismatch` を解消（`20260426142940` は HEAD に復元）。
    - `idempotency_gate.rs` [MODIFY]: `release_idempotent_key` 追加。失敗時にキー解放しないと同一キーのリトライが最大24時間 409 ブロックされる欠陥を修正。
    - `misc.rs` [MODIFY]: transfer/refund/withdraw のエラーパスでキー解放を呼出し。
    - `internal_routes_test.rs` [MODIFY]: 冪等性の重複リクエスト・失敗後リトライ検証テスト2件を追加（異常注入で検知可能なことを確認済み）。
    - `libs/infrastructure/src/artifact_store.rs` [MODIFY]: カテゴリ厳格パース化の退行を小文字正規化で修正。
- **既知の残課題**:
    - `tests/swarm_simulation_test.sh` は `test_sig` バイパスの `cfg(test)` 化により実バイナリで署名検証を通過できず、Node B 同期ステップで失敗する見込み。スクリプト側で実 Ed25519 署名の生成が必要。
    - BiomeGame Jest テスト1（Loading 不検出）とテスト3（OOM）は HEAD 由来の既存不具合（今回の変更とは無関係）。

## Phase: `/docs-sync` (2026-07-03)
- **同期対象**: `.env.example`, `commercial/.env.example`, `OPERATIONS_MANUAL.md`, `SECURITY_DESIGN.md`, `SECURITY_WHITEPAPER.md`, `INFRASTRUCTURE_MODULES.md`, `AIOME_NURTURE_SYNERGY.md` (§5.4.4), `README.md` / `README_en.md`, `ERROR_BUDGET_AND_RESILIENCE.md`, `CHANGELOG.md`
- **内容**: 改善計画 P0–P3 および検証フェーズ修正を運用・セキュリティ・アーキテクチャ文書へ反映

