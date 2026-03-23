# ADR-018: Watchtower App (External I/O Layer)

## Context
The "Watchtower" is the bridge between the Aiome internal operating system and external social/content platforms (X, Discord, YouTube, etc.). Current implementations are fragmented or mock-heavy.

## Decision
1.  **Independent I/O Lifecycle**: The Watchtower should be an independent, advanced app (`apps/watchtower`) that is responsible for all external ingestion and publication, decouple from core OS logic.
2.  **Platform Adapters**: Rather than platform-specific code in the OS, we will use a `WatchtowerAdapter` pattern in `libs/shared/src/watchtower.rs`.
3.  **Encapsulated Credentials**: All external API keys and secrets MUST be stored and managed by the `KeyProxy` (or Abyss Vault) rather than in the general OS memory or environment.
4.  **B-2 Publish Pipeline**: The `PublishPipeline` orchestrator must handle retry, rate limiting, and multi-platform concurrent publication.
5.  **Observability & Monitoring**: Watchtower must provide detailed metrics on cross-platform performance (reach, engagement, karma-generation) via Prometheus/Grafana integration.

## Status
Proposed (Phase 27 Target)

## Consequences
- **Loose Coupling**: AI OS evolves separately from complex, ever-changing social platform APIs.
- **Robustness**: If X API breaks, only the Watchtower needs update; the internal agent's "Intent" generation remains intact.
- **Scalability**: New platforms can be added as simple adapters in the Watchtower without re-architecture.
- **Security**: Centralized external-facing credentials significantly reduce the risk of accidental exposure.
