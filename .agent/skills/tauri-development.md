---
name: tauri-development
description: src-tauri/・tauri.conf.json・サイドカー（api-server, key-proxy, nurture-api）・CSP・トレイ/メニューに触れるときに必読の Tauri v2 固有ルール（T-001〜T-007）。通常の React UI 変更のみでは不要。
---

# Aiome Tauri Desktop Application Development Rules

Tauri v2 デスクトップアプリとしての Aiome 開発において、AIエージェントおよび開発者が遵守すべき追加の絶対的ルールです。

## ルール定義

### T-001: Tauri Shell Isolation [CRITICAL]
Tauri クレート（management-console）は「薄いシェル（ラッパー）」に徹すること。
`libs/` や `apps/api-server/` のコアロジックを `src-tauri/` に直接インポート（依存関係追加など）してはならない。
* **例外**: [AppDataResolver](file:///Users/motista/Desktop/antigravity/aiome/libs/shared/src/app_data.rs#L13) のみ、データパス解決の目的で Tauri クレートから参照することを許可する。

### T-002: Sidecar Lifecycle Contract [CRITICAL]
公式 Desktop の同梱サイドカーは `api-server` + `key-proxy`（+ 任意 `obscura`）のみ。**`nurture-api` は公式パッケージに含めない**（OP-088 P3 / 既定 InProcess）。

起動・停止は `tauri-plugin-shell` の Command API 経由で行い、以下を明示注入すること：
* `AIOME_DATA_DIR`: データディレクトリパス（Tauri の `app_data_dir` を解決したパス）
* `CELL_ID`: セル識別子（デフォルト: `"desktop-0"`）
* `KEY_PROXY_URL`: `http://localhost:3017`
* **InProcess（既定）**: api-server へ `NURTURE_IN_PROCESS=true` + `NURTURE_INTERNAL_SECRET` + `NURTURE_DRM_MASTER_KEY` + `NURTURE_API_URL=http://127.0.0.1:3015`（自己 HTTP）。nurture-api は **非 spawn**
* **Local（`NURTURE_MODE=local`・開発）**: nurture-api を spawn する場合のみ `NURTURE_API_URL=http://localhost:3020`、`DATABASE_URL=sqlite:${AIOME_DATA_DIR}/nurture.db`、双方へ `NURTURE_INTERNAL_SECRET`。公式パッケージには sidecar が無いため失敗しうる

起動順序（Local 時）: `nurture-api` → `api-server` → `key-proxy`  
起動順序（InProcess）: `api-server` → `key-proxy`  
停止: 起動の逆順。`RunEvent::ExitRequested` で SIGTERM クリーンアップ必須。

### T-003: CSP Synchronization [HIGH]
フロントエンドが新しいエンドポイントや外部サービスと通信するロジックを追加・変更する場合、`tauri.conf.json` の `connect-src` などの CSP 設定を同期更新すること。
全外部通信は原則としてローカルの `api-server` 経由のプロキシを使用することを推奨する。
**OP-088 P3**: 公式 CSP から `localhost:3020` / `127.0.0.1:3020` を除去する（nurture-api 非同梱）。`externalBin` 変更時も本ルールで CSP / capabilities を同期すること。

### T-004: Feature Flag Guard [HIGH]
Tauri 専用の機能（トレイメニュー操作、サイドカー自動起動など）を追加するコードは、Cargo の feature flag（`sidecar-auto` など）で条件付きコンパイル（`#[cfg(feature = "sidecar-auto")]`）を適用すること。
これにより、Tauri 未インストール環境や通常の CI 実行時における `cargo test --workspace` が破損することを防ぐ。

### T-005: Platform Test Guard [MEDIUM]
macOS の `muda`（メニューやトレイライブラリ）の制約として、これらはメインスレッドからしか呼び出せない。テストコードから呼び出す場合は macOS をガードするパターン（`#[cfg(not(target_os = "macos"))]`）を適用するか、スレッド安全なテスト方法を選択すること。

### T-006: No Mock Sidecars in Prod [CRITICAL]
リリースビルドおよび製品パッケージ生成前には `python3 scripts/desktop_sidecar_manager.py --check-all` を実行し、公式サイドカー（`api-server`, `key-proxy`, `obscura`）が本物であること、および **実バイナリの `nurture-api` が binaries/ に混入していないこと** を強制確認しなければならない。
ローカル開発および常用CIでは `--check-core`（`api-server` + `key-proxy`）を許可する。
Local escape 用の nurture-api は `--build --with-nurture-sidecar` のみ（公式 `externalBin` 外）。

### T-007: Git Binary Lockdown [HIGH]
ローカルでビルドした巨大な実バイナリや、自動生成されるプレースホルダーが Git リポジトリに誤ってコミットされないよう、`.gitignore` で `/apps/management-console/src-tauri/binaries/` ディレクトリを完全に遮断し、Git の追跡から排除し続けなければならない。既存の誤って追跡されたファイルは `git rm --cached` で解除すること。

