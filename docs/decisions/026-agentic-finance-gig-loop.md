# ADR-026: Agentic Finance & GIG Loop Integration

## Status
Proposed (2026-03-30)

## Context
Aiome aims to create an autonomous AI economy where agents can hire other agents to solve sub-tasks. Previously, the `GigEngine` was standalone, and `TaskDispatcher` handled task execution without awareness of the economic layer. To close the loop (Sense -> Plan -> Act -> Hire -> Evolve), the `TaskDispatcher` must be able to autonomously publish GIG intents when a job's completion results in further work requirements.

## Decision
We will integrate the `GigEngine` directly into the `TaskDispatcher` via Dependency Injection (DI).

### 1. GigEngine Injection
`TaskDispatcher` will accept an `Option<Arc<dyn GigEngine>>` in its constructor. This allows for:
- Production use with `UniversalGigEngine`.
- Testing with `MockGigEngine`.
- Optional disabling of economic features in minimal environments.

### 2. Autonomous Triggering
Upon successful job completion, the `TaskDispatcher` will inspect the `karma_directives` (JSON) for a `gig_intent: true` flag. If present, it will:
- Extract `description` and `budget` from the job's `output_artifacts`.
- Create a new `GigIntent` via a factory method.
- Populate `metadata` with causal information (parent job ID).

### 3. Safety Guardrails (Recursion Control)
To prevent infinite self-hiring loops (which could deplete budgets/resources), we introduce a `gig_depth` field in the intent metadata.
- The depth is incremented on each hop.
- A hard limit of **3** is enforced at the dispatcher level.
- Intents exceeding this depth will be logged and skipped.

### 4. Event Notification
A new `TaskEvent::GigPublished` variant will be added to the orchestration stream to provide real-time updates to the Management Console.

## Consequences
- **Positive**: Enables complex, multi-hop autonomous task decomposition across the AI workforce.
- **Positive**: Provides a clear audit trail of economic causality via `metadata`.
- **Negative**: Increases the complexity of `TaskDispatcher`.
- **Negative**: Adds a dependency from the infrastructure orchestration layer to the commerce/contracts layer.

## Alternatives Considered
- **Separate Bridge Worker**: A background task polling for completed jobs to publish GIGs. *Rejected due to increased latency and complexity in managing transactional consistency.*
- **LLM-only Control**: Relying solely on LLM to call a "publish_gig" tool. *Rejected because autonomous loops should be a first-class citizen of the OS kernel (Dispatcher).*
