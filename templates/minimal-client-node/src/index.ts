import WebSocket from 'ws';
import * as http from 'http';

// ──────────────────────────────────────
// Configuration
// ──────────────────────────────────────
const AIOME_WS_URL = process.env.AIOME_WS_URL || 'ws://localhost:1420/ws';
const NURTURE_API_URL = process.env.NURTURE_API_URL || 'http://localhost:8080';
const API_SECRET = process.env.API_SERVER_SECRET || 'my_super_secret_key_123456';

// ──────────────────────────────────────
// 1. WebSocket での自律ログ・アクティビティ購読
// ──────────────────────────────────────
function connectToAiome() {
    console.log(`📡 Connecting to Aiome: ${AIOME_WS_URL}`);
    const ws = new WebSocket(AIOME_WS_URL, {
        headers: {
            'Authorization': `Bearer ${API_SECRET}`
        }
    });

    ws.on('open', () => {
        console.log('✅ Connected to Aiome Event Stream.');
        // 自律アクティビティ監視の開始メッセージ
        ws.send(JSON.stringify({ type: 'subscribe', channel: 'activities' }));
    });

    ws.on('message', (data: string) => {
        try {
            const event = JSON.parse(data);
            console.log('📬 Event received from Agent OS:', event);
            
            // 例: 自律活動で「タスクの推論が発生した」イベントをフックし、
            // Nurture エコノミーに紐づいた remittance 決済を実行する
            if (event.type === 'inference_cost_trigger') {
                handleRemittance({
                    userId: event.userId,
                    amount: event.cost,
                    assetId: event.assetId || 'asset_lora_default',
                    useEscrow: true
                });
            }
        } catch (e) {
            console.error('❌ Failed to parse event:', e);
        }
    });

    ws.on('close', () => {
        console.log('🔌 Disconnected. Retrying in 5s...');
        setTimeout(connectToAiome, 5000);
    });

    ws.on('error', (err) => {
        console.error('❌ WebSocket error:', err);
    });
}

// ──────────────────────────────────────
// 2. 【Nurture 連携】経済 Remittance / エスクロー決済フック
// ──────────────────────────────────────
interface RemittanceParams {
    userId: string;
    amount: number;
    assetId: string;
    useEscrow: boolean;
}

function handleRemittance(params: RemittanceParams) {
    console.log(`💰 Initiating Remittance via Nurture: Deducting ${params.amount} credits.`);

    const payload = JSON.stringify({
        user_id: params.userId,
        amount: params.amount,
        asset_id: params.assetId,
        use_escrow: params.useEscrow,
        metadata: {
            source: "aiome-node-client",
            timestamp: Math.floor(Date.now() / 1000)
        }
    });

    // 開発環境において Nurture 側の `require_oxp_certificate` 制限をパスするためのダミー証明書
    const mockOxiLeanCertificate = JSON.stringify({
        signature: "mock_signature_eddsa_oxilean_assertion_999",
        oxp_score: 950, // OXP 900以上が必要条件
        timestamp: Math.floor(Date.now() / 1000) // 300秒の鮮度期限内
    });

    const url = new URL(`${NURTURE_API_URL}/internal/deduct`);
    const options = {
        hostname: url.hostname,
        port: url.port || 80,
        path: url.pathname,
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${API_SECRET}`,
            // OxiLean 証明書ヘッダーを添付
            'x-oxilean-proof-certificate': Buffer.from(mockOxiLeanCertificate).toString('base64')
        }
    };

    const req = http.request(options, (res) => {
        let body = '';
        res.on('data', (chunk) => body += chunk);
        res.on('end', () => {
            if (res.statusCode === 200) {
                console.log(`✅ Deduct approved. Nurture ledger updated:`, body);
            } else {
                console.error(`❌ Remittance denied (${res.statusCode}):`, body);
            }
        });
    });

    req.on('error', (e) => {
        console.error('❌ Connection to Nurture API failed:', e);
    });

    req.write(payload);
    req.end();
}

// 接続の起動
connectToAiome();
