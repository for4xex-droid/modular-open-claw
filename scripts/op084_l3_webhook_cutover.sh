#!/usr/bin/env bash
# OP-084 L3-4: Live Webhook を app.aiome.dev に寄せ、必須 7 イベントを有効化。
# 使い方（秘密はシェル履歴に残さないよう注意）:
#   export STRIPE_API_KEY='sk_live_…'   # Dashboard → Developers → API keys（チャット禁止）
#   ./scripts/op084_l3_webhook_cutover.sh
# 既定の Stripe CLI ログイン鍵が rk_live（制限付き）の場合、このスクリプトは必須。
set -euo pipefail

WEBHOOK_ID="${STRIPE_WEBHOOK_ENDPOINT_ID:-we_1TlXDCBcUTwo5TwLaihGahjO}"
LEGACY_WEBHOOK_ID="${STRIPE_LEGACY_WEBHOOK_ENDPOINT_ID:-we_1TlVbZBcUTwo5TwLm59jPH3k}"
URL="${STRIPE_WEBHOOK_URL:-https://app.aiome.dev/api/v1/commerce/webhook}"

EVENTS=(
  checkout.session.completed
  invoice.paid
  invoice.payment_failed
  customer.subscription.deleted
  customer.subscription.updated
  charge.dispute.created
  checkout.session.expired
)

if [[ -z "${STRIPE_API_KEY:-}" ]]; then
  echo "ERROR: STRIPE_API_KEY (sk_live_…) を export してください。CLI の rk_live では webhook update 権限がありません。" >&2
  exit 1
fi
if [[ "${STRIPE_API_KEY}" != sk_live_* ]]; then
  echo "ERROR: STRIPE_API_KEY は sk_live_ で始まる必要があります（現在のプレフィックスは秘密のため表示しません）。" >&2
  exit 1
fi

if ! command -v stripe >/dev/null 2>&1; then
  echo "ERROR: stripe CLI が必要です。" >&2
  exit 1
fi

cmd=(stripe webhook_endpoints update "$WEBHOOK_ID" --live --confirm --api-key="$STRIPE_API_KEY"
  --url="$URL"
  --description="Aiome Pro Live → app.aiome.dev (OP-084 L3)")
for e in "${EVENTS[@]}"; do
  cmd+=(--enabled-events="$e")
done

echo "Updating Live webhook $WEBHOOK_ID → $URL (7 events)…"
"${cmd[@]}" >/tmp/op084_wh_update.json

python3 - <<'PY'
import json
from pathlib import Path
o = json.loads(Path("/tmp/op084_wh_update.json").read_text())
if o.get("error"):
    raise SystemExit(f"Stripe error: {o['error']}")
ev = sorted(o.get("enabled_events") or [])
print(f"OK id={o.get('id')} status={o.get('status')} livemode={o.get('livemode')}")
print(f"url={o.get('url')}")
print(f"events={len(ev)}")
for e in ev:
    print(f"  - {e}")
if len(ev) != 7:
    raise SystemExit("ERROR: expected 7 enabled events")
PY

echo "Disabling legacy incomplete webhook $LEGACY_WEBHOOK_ID (best-effort)…"
if stripe webhook_endpoints update "$LEGACY_WEBHOOK_ID" --live --confirm --api-key="$STRIPE_API_KEY" \
  -d disabled=true >/tmp/op084_wh_disable.json 2>/tmp/op084_wh_disable.err; then
  python3 - <<'PY'
import json
from pathlib import Path
o = json.loads(Path("/tmp/op084_wh_disable.json").read_text())
print(f"legacy status={o.get('status')} id={o.get('id')}")
PY
else
  echo "WARN: legacy disable failed (続行可). 詳細は /tmp/op084_wh_disable.err"
fi

echo
echo "Next (Human / 本番ホスト):"
echo "  1) Dashboard で当該 endpoint の Signing secret (whsec_) を確認 → Abyss Vault の STRIPE_WEBHOOK_SECRET"
echo "  2) Vault に STRIPE_API_KEY=sk_live_… を格納"
echo "  3) ホスト: STRIPE_TEST_MODE=false"
echo "     STRIPE_PRICE_SUBSCRIPTION_MONTHLY=price_1TpXFpBcUTwo5TwLmK9SQbKL"
echo "  4) api-server 再起動: env のみなら restart 可。"
echo "     イメージ更新が必要なら distroless rebuild +"
echo "     up -d --force-recreate --no-deps --no-build api-server"
echo "     （restart だけではイメージは変わらない）"
echo "  5) unset STRIPE_API_KEY"
