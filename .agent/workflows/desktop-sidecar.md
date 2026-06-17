# /desktop-sidecar - Tauri デスクトップサイドカー管理・検証ワークフロー

Tauri デスクトップアプリケーションに必要なサイドカーバイナリのライフサイクル管理、および製品ビルド時におけるダミー（プレースホルダー）混入を防ぐ安全検証を実施します。

## 概要

本ワークフローは、以下の3つの安全規約に基づき自動検証スクリプト `scripts/desktop_sidecar_manager.py` を運用します：
1. **プレースホルダー自動生成**: ローカル開発初期や環境構築時にダミーファイルを即座に生成し、ビルドエラーを回避する。
2. **本物のバイナリ物理検証**: ファイルヘッダのマジックバイトおよび最小ファイルサイズ（100KB以上）による物理判定で、ダミー混入をシャットアウトする。
3. **2段階検証ポリシー**: ローカル開発およびCIでは最低限必要なRustコアサイドカーのみをチェックし、本番リリースビルド前にはフォールバック含む全サイドカーの完全性を強制検証する。

---

## 実行コマンド一覧

### 1. 開発環境のセットアップ（プレースホルダー生成）
Tauri のビルドエラーを避けるためにダミーファイル（Windowsはバッチ、その他はシェルスクリプト）を作成します。
```bash
python3 scripts/desktop_sidecar_manager.py --setup-placeholders
```

### 2. ローカルサイドカーバイナリのビルド
Rust ワークスペースから実バイナリをビルドし、自動的に `apps/management-console/src-tauri/binaries/` へ配置します。
* `nurture-api` は AWS SDK 排除フラグ（`--no-default-features --features desktop`）でビルドされます。
* `obscura` は PATH 上に実バイナリが存在すれば配置され、なければプレースホルダーのまま維持されます。
```bash
python3 scripts/desktop_sidecar_manager.py --build
```

### 3. ステージ1検証：コアバイナリ検証（ローカル開発・常時CI）
Rust製コアサイドカー3つ（`api-server`, `key-proxy`, `nurture-api`）が実バイナリであることを検証します。
```bash
python3 scripts/desktop_sidecar_manager.py --check-core
```
* **動作**: 3つのうちいずれかがダミーもしくは見つからない場合、終了ステータス `1` で異常終了します（Fail-Closed）。

### 4. ステージ2検証：全バイナリ検証（リリースビルドCI）
コア3つに加えて `obscura` を含む全4つのサイドカーが実バイナリであることを完全検証します。
```bash
python3 scripts/desktop_sidecar_manager.py --check-all
```
* **動作**: `obscura` を含め、いずれか1つでもダミーもしくは欠損がある場合、終了ステータス `1` で異常終了します。

---

## 安全ガードレール運用規約

1. **本番パッケージのビルド前検証の義務化**
   * 製品リリースビルドを実行する CI パイプライン、あるいは手動ビルドの直前には、必ず `python3 scripts/desktop_sidecar_manager.py --check-all` を実行しなければならない。
   * これが失敗した場合は、絶対にビルドパッケージを出力してはならない。

2. **Git 履歴汚染の防止（ロックダウン）**
   * 実バイナリおよび自動生成されたプレースホルダーは、絶対に Git リポジトリにコミットしてはならない。
   * `.gitignore` に `/apps/management-console/src-tauri/binaries/` ルールが正しく設定されていることを常時維持すること。
