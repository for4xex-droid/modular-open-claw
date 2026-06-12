# Aiome & Nurture API Client (Node.js / TypeScript)

本テンプレートは、Node.js / TypeScript を使用して Aiome API Server の WebSocket に接続し、エージェント自律活動のログをフックして Project-Nurture エコノミーシステム（決済・エスクロー）と remittance 連携を行うためのデモボイラープレートです。

## セットアップ & 起動方法

1. 依存関係のインストール:
   ```bash
   npm install
   ```
2. 設定環境変数を付与して起動:
   ```bash
   export AIOME_WS_URL="ws://localhost:1420/ws"
   export NURTURE_API_URL="http://localhost:8080"
   export API_SERVER_SECRET="my_super_secret_key_123456" # gitleaks:allow
   npm start
   ```

## Nurture S2S 証明書認証のモック方法

実環境の Nurture 決済API（`/internal/deduct`）は、セキュリティのため `require_oxp_certificate` ミドルウェアによって保護されており、リクエストヘッダーに有効な OxiLeanProofCertificate が要求されます。

開発・テスト環境でこの制限を模ックするため、本コードではヘッダー `x-oxilean-proof-certificate` に Base64 エンコードした以下の JSON を自動挿入しています。

```json
{
  "signature": "mock_signature_eddsa_oxilean_assertion_999",
  "oxp_score": 950,
  "timestamp": 1781273307
}
```

* **oxp_score**: `900` 以上である必要があります。
* **timestamp**: サーバー受信時刻から `300秒（5分）` 以内の鮮度を持つ必要があります。

開発中のモック決済では、上記仕様に基づいて証明書ヘッダーを組み立てて通信テストを行ってください。
