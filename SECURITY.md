# Security Policy

## Supported Versions

Currently, Aiome provides security updates for the latest major version.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Incident Response & Formal Verification
Aiome's core agentic loop and sandbox boundaries have been mathematically modeled using TLA+ and model-based tested via Rust's TypeState pattern. However, as an ever-evolving foundation for Autonomous AI Agents, unexpected vulnerabilities — particularly regarding prompt injection and sandbox escapes — may arise.

If you uncover a vector that allows an Agent to bypass the `AdaptiveImmuneSystem`, `Watchtower`, or break out of its `gVisor`/WASM constraints, please consider it a high-priority severity. 

## Commerce Integrations (Stripe)
For commerce and gig features, Aiome uses the `StripeCommerceEngine`. To ensure absolute security in production environments, the engine enforces a strict "Fail-Closed" policy.
If the `AIOME_DEV_MODE` environment variable is not explicitly set to `true` (or when running in Release mode), using test webhook secrets (e.g., `whsec_test`) will actively block the webhook verification process. Ensure `AIOME_DEV_MODE="false"` is set or omitted in your production deployments to prevent test-bypass vulnerabilities.

## Data Protection & Incident Recovery
Aiome employs an automatic Pre-migration Guard and supports WAL-safe SQLite Online Backups to protect against data loss from database corruption or failed migrations. If an agent goes rogue or a systemic failure occurs, operators can securely revert to the most recent snapshot without data inconsistency. Please refer to the Operations Manual for backup scheduling.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them to us via email at **security@motivationstudio.dev**.
Please include the following information in your report:
- Type of issue (e.g. prompt injection, buffer overflow, sandbox escape)
- Instructions to reproduce the vulnerability (including logs or code if possible)
- The version of Aiome you were using
- Potential impact

We will endeavor to respond to your report within 48 hours. If the vulnerability is confirmed, we will work with you to patch it, prepare a CVE if applicable, and credit you in the subsequent release patch notes.

Thank you for helping keep the Aiome ecosystem secure and structurally sound!
