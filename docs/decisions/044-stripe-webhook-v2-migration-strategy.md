# ADR-044: Stripe Webhook v2 移行戦略と後方互換性の維持

**Status**: Accepted  
**Date**: 2026-06-23  
**Deciders**: motivationstudio

## Context

Stripe API は v2 へのアップデートに伴い、大規模オブジェクトのメタデータを送信しない「thin events（薄いイベント）」と複数の署名シークレットの利用を導入している。
既存のシステムは v1 のフルペイロード（snapshot events）を前提に設計されており、これをすべて書き直すのはリスクが高い。
また、移行期間中に v1 と v2 双方の Webhook をダウンタイムなしに受信し分ける必要がある。

## Decision

1. **複数シークレット対応の署名検証**:
   `STRIPE_WEBHOOK_SECRET` をカンマ区切りで複数指定可能にし、`verify_signature` 内でループして検証する。署名は一致するもののデシリアライズスキーマの不整合によるパースエラー（`BadParse`）が発生した場合は、署名検証自体は「成功」と判定して終了する。
2. **thin event の透過的自動解決**:
   Webhook 受信ハンドラ (`stripe.rs`) のパース直後に `v2.core.event` を自動検知する。
   該当する場合は `stripe_api_key` を利用して `related_object.url` から REST API でフルデータを自動フェッチ（SSRF 防御として `https://api.stripe.com` プレフィックスを固定結合）。
   取得データを v1 互換の JSON ペイロードへ透過的に書き換える。

## Consequences

- **Good**: 既存の v1 ベースの全決済・ライセンス付与ハンドラを1行も書き換えることなく、透過的に v2 Webhook の受信に対応。
- **Good**: v1 と v2 の両シークレットをカンマ区切りで登録できるため、DNSやStripe側設定の変更時にダウンタイムゼロで移行可能。
- **Bad**: v2 thin event の受信時に追加の HTTP ラウンドトリップ（Stripe API への GET）が発生する（最大10秒のタイムアウト付き）。
