# Desktop 配布チャネル（OP-089）

正本計画: [`docs/roadmaps/op089_oss_economy_dual_channel_plan.md`](../roadmaps/op089_oss_economy_dual_channel_plan.md)

## チャネル

| チャネル | 用途 | api-server |
|----------|------|------------|
| **economy**（既定・公式製品） | Nurture InProcess 同梱の製品体験 | `--features nurture` |
| **oss** | 軽量 / commercial 非リンク | feature なし |

両チャネルとも公式 sidecar は `api-server` + `key-proxy` のみ。`nurture-api` プロセスは同梱禁止。

## ビルド

```bash
# Economy（既定）
npm run sidecar:build:economy
# または
python3 scripts/desktop_sidecar_manager.py --build --channel economy

# OSS
npm run sidecar:build:oss

# リンク検査のみ（フルビルド無し）
npm run sidecar:check:channels
```

成果物ディレクトリに `channel-manifest.json` が書かれます。

## リリースアセット命名（規約）

| チャネル | プレフィックス |
|----------|----------------|
| Economy | `AiomeOS-Economy-<version>-<triple>` |
| OSS | `AiomeOS-OSS-<version>-<triple>` |

`productName` / bundle id は共通（二重化しない）。チャネルはファイル名と manifest で区別する。

## ライセンス文言

- **Economy**: Aiome（ルート `LICENSE` / BSL 1.1）に加え、Nurture 商用エンジン（`commercial/LICENSE`）が同梱される。
- **OSS**: Aiome BSL 1.1。**本ビルドに Nurture 商用エンジン（`commercial/`）は含まれない。** Economy 機能が必要な場合は公式 Economy チャネルを利用すること。

## Local escape

`nurture-api` sidecar が必要な開発のみ:

```bash
npm run sidecar:build:local
# = --channel economy --with-nurture-sidecar
```

OSS チャネルとの併用は Fail-Closed で拒否される。
