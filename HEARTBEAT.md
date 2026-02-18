# HEARTBEAT.md

## 🔄 定期メンテナンス・ヘルスチェック

1. **Factory Health Monitor**:
   - `GET http://localhost:3015/api/health` を叩き、リソース使用状況を確認。
   - `memory_usage_mb > 500` または `cpu_usage_percent > 80.0` の場合は警告を通知。
2. **Lex AI Compliance Check**:
   - `libs/infrastructure` 内の新機能が `AgentAct` トレイトを適切に実装しているか確認。
3. **Git Sync Verification**:
   - `git status` を確認し、未プッシュの重要な変更がないかチェック。

## 💓 Heartbeat Status
- `HEARTBEAT_OK` を返す前に、上記の項目を巡回すること。
