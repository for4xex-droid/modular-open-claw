# Aiome Incident Response Runbook

本ドキュメントは、Aiome プロダクション環境において、致命的なセキュリティ違反や異常なコスト急上昇が発生した際の標準的な対応手順 (Runbook) を定義するものです。

## 1. 緊急対応フロー (Triage & Containment)

### 🚨 1.1 コスト急増・リミット到達アラート
**検知:** `CostCircuitBreaker` のログまたは UI アラート (`Budget Exhausted`) が連続発生。
**即時対応:**
1. **停止判断:** 異常なトランザクションが特定のユーザー・エージェントから来ているか確認する。
   ```bash
   sqlite3 data/aiome.db "SELECT agent_id, SUM(estimated_cost_usd) FROM resource_usage_logs WHERE created_at > datetime('now', '-1 hour') GROUP BY agent_id ORDER BY SUM(estimated_cost_usd) DESC LIMIT 5;"
   ```
2. **システム全体のAPI通信遮断 (Kill Switch):**
   万が一、インフラ全体のAPIキー流出が疑われる場合、ダミーキーに差し替えるか、プロセスを即時停止する。
   ```bash
   systemctl stop aiome-api-server
   ```
3. **ベンダー側での制限:** 
   Stripe または対象の LLM プロバイダー (Anthropic / Gemini 等) の管理画面にログインし、一時的に当該キーの Rate Limit を 0 にするかキーを Revoke する。

### 🚨 1.2 CWE-209 関連・情報漏洩疑い
**検知:** 外部からの異常なエントリポイントアクセスや、WAF での Error Message 関連のブロック警告。
**即時対応:**
1. アプリケーションログにて `Internal Server Error [Error ID: xxxx]` が異常発生していないかをチェック。
2. もし特定のパスへのアクセスでエラーが多発している場合、ロードバランサー (Nginx / HAProxy) レベルで該当の URI パスをブロックする。

### 🚨 1.3 Magic Bytes 偽装ファイルアップロード検知
**検知:** `AiomeError::SecurityViolation` にて "Magic bytes mismatch" がログに記録されている。
**即時対応:**
1. 当該ファイルをアップロードした `agent_id` および `IPアドレス` を特定。
2. 被疑 IP アドレスを WAF または iptables にて直ちにブロック。
3. アップロード先ディレクトリ (`/tmp/aiome_sandbox` 等) を隔離し、アンチウイルス/YARAスキャンを実施。

## 2. 復旧と事後分析 (Recovery & Retrospective)

1. **パッチ適用:**
   影響範囲を特定後、修正プログラムを作成し、`/perfect-plan` および `/tdd` サイクルを回し、CI を通過したホットフィックスをデプロイする。
2. **サービスの再開:**
   環境変数や設定ファイル (`settings` テーブル) からコストバイパスフラグ (`cost_bypass_amount`) を適切に調整・クリアし、再起動する。
3. **Post-mortem の作成:**
   「なぜ発生したか (Root Cause)」「どうすれば防げたか」を `memory/` 配下に記録し、次回スプリントにてシステム防御を強化する。

## 3. 重要連絡先
- Security Team: `security@motivationstudio.local`
- Infra Team: `infra@motivationstudio.local`
