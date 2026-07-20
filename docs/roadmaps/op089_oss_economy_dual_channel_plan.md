# OP-089 — OSS / Economy 二系統 Desktop チャネル（v1.0）

- **ステータス**: Implemented（2026-07-21）
- **由来**: OP-088 Q5 / P5-d（OP-088 クローズ条件外）
- **OPEN**: **OP-089**
- **許可**: Human「OP-089 を実装しろ」

## 0. 目的

公式 Desktop（**Economy** = Nurture InProcess 同梱）と、OSS 向け軽量チャネル（**commercial/ 非リンク**）を配布物として分離し、ライセンス境界と Fail-Closed 検査を固定する。

## 1. 非目標

- OP-088 Ship / P5-a〜c の再実装
- フル Tauri OS×arch リリース列車の新設
- `productName` / bundle id の二重化
- Tauri spawn 本線・認証・決済の変更

## 2. チャネル定義

| | **Economy（公式製品・既定）** | **OSS（軽量）** |
|---|---|---|
| api-server | `--features nurture` | feature なし |
| key-proxy | 同梱 | 同梱 |
| nurture-api sidecar | 禁止 | 禁止 |
| commercial/ リンク | あり（NurturePlugin） | なし |
| アセット名規約 | `AiomeOS-Economy-<ver>-<triple>` | `AiomeOS-OSS-<ver>-<triple>` |
| ライセンス文言 | Aiome BSL + Nurture (`commercial/LICENSE`) | Aiome BSL +「本ビルドに Nurture 商用エンジンは含まれない」 |

## 3. 実装成果物

| 項目 | 正本 |
|------|------|
| ビルド/検査 | `scripts/desktop_sidecar_manager.py --channel {economy\|oss}` |
| Fail-Closed | `--verify-channel-link`（cargo tree: Economy=has nurture-api / OSS=lacks） |
| npm | `sidecar:build:economy` / `sidecar:build:oss` / `sidecar:check:channels` |
| CI | `desktop-sidecar` ジョブに両チャネル link 検査 |
| 運用文書 | [`docs/guides/DESKTOP_CHANNELS.md`](../guides/DESKTOP_CHANNELS.md) |

## 4. DoD

- [x] Economy: nurture-api が依存ツリーに存在する
- [x] OSS: nurture-api が依存ツリーに存在しない
- [x] 両チャネルで nurture-api **sidecar 実体**禁止（既存 forbid）
- [x] OSS + `--with-nurture-sidecar` は拒否
- [x] Positive + Negative テスト
- [x] OPEN / CHANGELOG / RIPPLE / README 同期

## 5. OP-088 との関係

P5-d 委譲先。OP-088 はクローズ可能。
