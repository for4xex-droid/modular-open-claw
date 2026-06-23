export default {
  async fetch(request, env, ctx) {
    // WebhookのPOSTリクエストのみを中継
    if (request.method !== "POST") {
      return new Response("Method Not Allowed", { status: 405 });
    }

    // Stripeの署名ヘッダーの存在を確認
    const stripeSignature = request.headers.get("stripe-signature");
    if (!stripeSignature) {
      return new Response("Missing stripe-signature header", { status: 400 });
    }

    // 署名検証のためにボディをバイナリ（生データ）のまま抽出
    const body = await request.arrayBuffer();

    try {
      // 設定されたFORWARD_URL（本番APIサーバー）へそのまま転送
      const response = await fetch(env.FORWARD_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "stripe-signature": stripeSignature,
        },
        body: body,
      });

      const responseBody = await response.text();
      return new Response(responseBody, {
        status: response.status,
        headers: { "Content-Type": "application/json" },
      });
    } catch (err) {
      return new Response(`Forwarding failed: ${err.message}`, { status: 500 });
    }
  }
};
