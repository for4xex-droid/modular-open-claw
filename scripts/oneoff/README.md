# scripts/oneoff — 適用済みワンショット証跡

本番ホスト等へ **一度だけ適用した** 改変スクリプト・レポートの保管場所です。

| ファイル | 用途 | 状態 |
|---|---|---|
| `prod_enable_caddy_aiome_dev.py` | 本番 compose への Caddy / DOMAIN_NAME 配線 | **適用済・再実行禁止** |
| `prod_set_domain_app_aiome_dev.py` | 本番 `.env` の DOMAIN_NAME 設定 | **適用済・再実行禁止** |
| `patch_prod_api_distroless.py` | 本番 api-server を distroless に切替 | **適用済・再実行禁止** |
| `biz_value_report.html` | 2026-07-10 ビジネスバリュー・レポート | アーカイブ |

- パスは本番ホスト (`/app/aiome/...`) にハードコードされています。ローカルで実行しないでください。
- 恒久手順の正本は [`docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md`](../../docs/guides/HUMAN_PUBLIC_BETA_RUNBOOK.md) および `docker/distroless.Dockerfile` です。
