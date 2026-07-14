# Aiome Commerce & Voice Store Policy

**Effective Date:** 2026-03-20
**Version:** 1.0.0
**Status:** **FROZEN / SUPERSEDED for public billing** — 有償 KC・マーケット開放まで本文書は**効力を持たない**。

> **正本の優先順位（2026-07-15）**  
> 公開課金・KC の法的位置づけは次を正とする。本ファイルの「Fiat→KC」「チャージ」記述は**歴史的ドラフト**であり、現行実装・公開表示と矛盾する。  
> - 公開: [`TERMS_OF_SERVICE.md`](TERMS_OF_SERVICE.md) / [`TOKUSHOHO.md`](TOKUSHOHO.md) / [`CANCELLATION_POLICY.md`](CANCELLATION_POLICY.md)  
> - 社内: [`KC_LEGAL_POSITION.md`](KC_LEGAL_POSITION.md)（KC = 無償・非販売・非換金）  
> - 必須チェック: [`COMPLIANCE_CHECKLIST.md`](COMPLIANCE_CHECKLIST.md)

## 1. Introduction
This document outlines the commercial terms, digital rights management (DRM), and revenue distribution policies for the Aiome Voice Store and Creator Registry. These policies govern all transactions involving AI creator assets, specifically XTTS voice models.

## 2. Karma Coins (KC)
All transactions within the Aiome Voice Store are conducted using "Karma Coins" (KC), a virtual platform currency.
- Karma Coins are non-refundable and hold no real-world cash value outside the Aiome ecosystem.
- ~~USD/Fiat to KC conversion is managed via the Stripe Commerce Engine.~~ **（凍結）現行は Fiat→KC チャージ経路なし。Stripe は Pro サブスクのみ。詳細は `KC_LEGAL_POSITION.md`。**

## 3. Creator Revenue Share
Aiome operates on a fair-revenue model to support AI creators:
- **Creators:** Receive 85% of the Karma Coins spent on their registered assets.
- **Platform (MotivationStudio LLC):** Retains a 15% platform fee for infrastructure, security, and payment processing.
- Revenue distribution occurs asynchronously via the internal ledger system.

## 4. Digital Rights Management (DRM)
To protect creator intellectual property, Aiome implements layered controls:
- **PathSandbox Isolation (Current):** Protected assets are stored in a dedicated vault directory, isolated from standard artifact storage via `PathSandbox` traversal prevention.
- **Abyss Security Proxy (Planned):** Full encryption-at-rest with cryptographic key binding to purchaser identity.
  - Keys will be tied to the purchaser's verifiable identity via eKYC.
  - Memory-safe (`zeroize`) key handling for runtime protection.
- Secondary distribution, extraction, or reproduction of purchased voice models is strictly prohibited.

## 5. Idempotency and Webhook Integrity
All financial transactions initiated via external payment gateways (e.g., Stripe) are guaranteed to be processed exactly once:
- The system employs cryptographic signature verification (`verify_signature`) on all incoming webhooks.
- Idempotency is enforced at the database level (`stripe_webhook_events` table) to prevent double-crediting or duplicate purchases.

## 6. Prohibited Assets
Aiome enforces zero-tolerance for the following within the Registry:
- Deepfakes or unauthorized voice clones of real individuals without explicit, verifiable consent (enforced via eKYC).
- Content violating the CSAM policy or promoting illegal acts.
- Assets failing the Hash/Anomaly screening process.

## 7. Modifications
MotivationStudio LLC reserves the right to modify this policy. Changes will be communicated via the Samsara Hub and require re-consent by active creators.
