# Aiome Tauri Desktop Application Development Rules

Tauri v2 デスクトップアプリとしての Aiome 開発において、AIエージェントおよび開発者が遵守すべき追加の絶対的ルールです。

## ルール定義

### T-001: Tauri Shell Isolation [CRITICAL]
Tauri クレート（management-console）は「薄いシェル（ラッパー）」に徹すること。
`libs/` や `apps/api-server/` のコアロジックを `src-tauri/` に直接インポート（依存関係追加など）してはならない。
* **例外**: [AppDataResolver](file:///Users/motista/Desktop/antigravity/aiome/libs/shared/src/app_data.rs#L13) のみ、データパス解決の目的で Tauri クレートから参照することを許可する。

### T-002: Sidecar Lifecycle Contract [CRITICAL]
サイドカープロセス（`api-server`, `key-proxy`, `nurture-api`）の起動・停止は `tauri-plugin-shell` の Command API 経由で行い、以下の環境変数を必ず明示的に渡すこと：
* `AIOME_DATA_DIR`: データディレクトリパス（Tauri の `app_data_dir` を解決したパス）
* `CELL_ID`: セル識別子（デフォルト: `"desktop-0"`）
* `KEY_PROXY_URL`: `http://localhost:3017`（Docker 内のホスト名から localhost へ上書き）
* `NURTURE_API_URL`: `http://localhost:3020` （ローカル経済エンジン有効時、api-server へ注入）
* `NURTURE_INTERNAL_SECRET`: セッションごとの一時トークン（api-server および nurture-api へ注入）
* `DATABASE_URL`: `sqlite:${AIOME_DATA_DIR}/nurture.db` （nurture-api へ注入）

起動順序: `nurture-api` → `api-server` → `key-proxy`
停止順序: `api-server` → `key-proxy` → `nurture-api`（起動の逆順）

また、Tauri アプリの終了イベント（`RunEvent::ExitRequested`）検知時には、起動したすべてのサイドカーに SIGTERM を送信してプロセスをクリーンアップし、ゾンビプロセスの残留を防止すること。

### T-003: CSP Synchronization [HIGH]
フロントエンドが新しいエンドポイントや外部サービスと通信するロジックを追加・変更する場合、`tauri.conf.json` の `connect-src` などの CSP 設定を同期更新すること。
全外部通信は原則としてローカルの `api-server` 経由のプロキシを使用することを推奨する。

### T-004: Feature Flag Guard [HIGH]
Tauri 専用の機能（トレイメニュー操作、サイドカー自動起動など）を追加するコードは、Cargo の feature flag（`sidecar-auto` など）で条件付きコンパイル（`#[cfg(feature = "sidecar-auto")]`）を適用すること。
これにより、Tauri 未インストール環境や通常の CI 実行時における `cargo test --workspace` が破損することを防ぐ。

### T-005: Platform Test Guard [MEDIUM]
macOS の `muda`（メニューやトレイライブラリ）の制約として、これらはメインスレッドからしか呼び出せない。テストコードから呼び出す場合は macOS をガードするパターン（`#[cfg(not(target_os = "macos"))]`）を適用するか、スレッド安全なテスト方法を選択すること。

### T-006: No Mock Sidecars in Prod [CRITICAL]
リリースビルドおよび製品パッケージ生成前には、安全ガードレールスクリプト (`python3 scripts/desktop_sidecar_manager.py --check-all`) を実行し、全サイドカーバイナリ（`api-server`, `key-proxy`, `nurture-api`, `obscura`）が本物の実行可能バイナリであることを物理判定（マジックバイト・最小サイズ検証）により強制確認しなければならない。ダミープレースホルダーが混入した状態でのビルドは厳禁とする。
ローカル開発および常用CIでは、Rustコアバイナリのみを検証する `--check-core` の使用を許可する。

### T-007: Git Binary Lockdown [HIGH]
ローカルでビルドした巨大な実バイナリや、自動生成されるプレースホルダーが Git リポジトリに誤ってコミットされないよう、`.gitignore` で `/apps/management-console/src-tauri/binaries/` ディレクトリを完全に遮断し、Git の追跡から排除し続けなければならない。既存の誤って追跡されたファイルは `git rm --cached` で解除すること。

