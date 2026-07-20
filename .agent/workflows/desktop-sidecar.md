---
description: Tauri サイドカーバイナリのライフサイクル管理とダミー混入防止の物理検証
---

# /desktop-sidecar - Tauri デスクトップサイドカー管理・検証ワークフロー

Tauri デスクトップアプリケーションに必要なサイドカーバイナリのライフサイクル管理、および製品ビルド時におけるダミー（プレースホルダー）混入を防ぐ安全検証を実施します。

## 概要（OP-088 P3）

**公式 Desktop は設定不要（既定 InProcess）**。同梱サイドカーは `api-server` + `key-proxy`（+ 任意で `obscura`）のみ。  
`nurture-api` は公式パッケージに含めない。Local escape は開発用 `--with-nurture-sidecar` + `NURTURE_MODE=local`。

本ワークフローは `scripts/desktop_sidecar_manager.py` を運用します：
1. **プレースホルダー自動生成**: 公式セット（+ obscura）。`--with-nurture-sidecar` で nurture-api も生成可。
2. **本物のバイナリ物理検証**: マジックバイト + 最小サイズ（100KB）。
3. **2段階検証**: `--check-core`（公式2本）/ `--check-all`（+ obscura、かつ実 nurture-api 混入禁止）。

---

## npm ラップ（management-console）

```bash
npm run sidecar:placeholders   # 公式プレースホルダー
npm run sidecar:build          # 公式ビルド
npm run sidecar:build:local    # + nurture-api（dev）
npm run sidecar:check          # --check-core --forbid-nurture-sidecar（CI 同等）
npm run sidecar:check:release  # --check-all
```

---

## 実行コマンド一覧

### 1. 開発環境のセットアップ（プレースホルダー生成）
```bash
python3 scripts/desktop_sidecar_manager.py --setup-placeholders
# Local 用に nurture-api プレースホルダーも必要なら:
python3 scripts/desktop_sidecar_manager.py --setup-placeholders --with-nurture-sidecar
```

### 2. 公式サイドカービルド
```bash
python3 scripts/desktop_sidecar_manager.py --build
```
* `api-server` は `--features nurture`（desktop / no AWS）。
* `nurture-api` はビルドしない。旧成果物があれば削除する。

### 3. Local escape 用 nurture-api ビルド
```bash
python3 scripts/desktop_sidecar_manager.py --build --with-nurture-sidecar
export NURTURE_MODE=local
```
* **公式 `tauri.conf.json` には nurture-api が無い**（意図的）。Local で Tauri から spawn するには、開発中のみ次を一時追加する:
  * `bundle.externalBin` ← `"binaries/nurture-api"`
  * `capabilities/default.json` の `shell:allow-execute.allow` ← `{ "name": "nurture-api" }`
* 公式パッケージへこの変更をコミットしないこと（P3 / Q3）。

### 4. ステージ1検証（ローカル開発・常時CI）
```bash
python3 scripts/desktop_sidecar_manager.py --check-core
```
* 対象: `api-server`, `key-proxy` のみ。

### 5. ステージ2検証（リリースビルド）
```bash
python3 scripts/desktop_sidecar_manager.py --check-all
```
* 対象: 公式2本 + `obscura`。
* **実バイナリの `nurture-api` が binaries/ にあれば Fail-Closed**（公式同梱回帰防止）。

---

## 安全ガードレール運用規約

1. **本番パッケージのビルド前**に `--check-all` を実行し、失敗時はパッケージを出さない。
2. **Git ロックダウン**: `apps/management-console/src-tauri/binaries/` は `.gitignore` で遮断を維持。
3. **CI 配線（済）**: `.github/workflows/ci.yml` ジョブ `desktop-sidecar`
   * `python3 scripts/test_desktop_sidecar_manager.py`
   * `python3 scripts/desktop_sidecar_manager.py --build`
   * `python3 scripts/desktop_sidecar_manager.py --check-core --forbid-nurture-sidecar`
   * `--check-all`（obscura 必須）はリリース手元/専用パイプライン向け。常用 CI では obscura 無しでもゲート可能。
