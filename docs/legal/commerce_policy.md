# Aiome Commerce & Voice Store Policy

**Effective Date:** 2026-03-20
**Version:** 1.0.0

## 1. Introduction
This document outlines the commercial terms, digital rights management (DRM), and revenue distribution policies for the Aiome Voice Store and Creator Registry. These policies govern all transactions involving AI creator assets, specifically XTTS voice models.

## 2. Karma Coins (KC)
All transactions within the Aiome Voice Store are conducted using "Karma Coins" (KC), a virtual platform currency.
- Karma Coins are non-refundable and hold no real-world cash value outside the Aiome ecosystem.
- USD/Fiat to KC conversion is managed via the Stripe Commerce Engine.

## 3. Creator Revenue Share
Aiome operates on a fair-revenue model to support AI creators:
- **Creators:** Receive 80% of the Karma Coins spent on their registered assets.
- **Platform (MotivationStudio LLC):** Retains a 20% platform fee for infrastructure, security, and payment processing.
- Revenue distribution occurs asynchronously via the internal ledger system.

## 4. Digital Rights Management (DRM)
To protect creator intellectual property, Aiome implements strict cryptographic controls:
- **Abyss Security Proxy:** Voice asset encryption keys are securely stored and managed by the Abyss Voice Vault.
- Keys are intrinsically tied to the purchaser's verifiable identity.
- Secondary distribution, extraction, or reproduction of purchased voice models is strictly prohibited and technically enforced via memory-safe (`zeroize`) key handling.

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
