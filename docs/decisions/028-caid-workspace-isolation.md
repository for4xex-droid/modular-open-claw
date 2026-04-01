# ADR 028: CAID (Centralized Asynchronous Isolated Delegation) Architecture

Date: 2026-03-31

## Status

Accepted

## Context

Aiome's Autonomous Pipeline consists of multiple task conductors (e.g. `OssIntegrationOrchestrator`, `DockerConductor`) working in a distributed manner via gRPC (`aiome-shadow-worker`).
One major problem emerged as the system scaled: **Resource Contention and Poisoning during Autonomous Execution**.

- When the Agent generated code, it directly modified the host workspace.
- A failed compilation or logic error committed directly to the main workspace caused the IDE and subsequent tasks to fail, halting the entire progression (Cascading Failure).
- Testing locally in parallel led to DB locks and Port binding collisions (e.g. Web Server binding 3000 continuously).
- Over-provisioning full Docker sandboxes for simple text/JSON generation jobs was inefficient.

We evaluated adopting libgit2 via `git2` crate to manage isolated branches, but found its memory overhead and ABI compatibility challenging for an orchestrator that also cross-compiles to WASM.

## Decision

We introduce **CAID (Centralized Asynchronous Isolated Delegation)**, an architecture that isolates hazardous tasks via OS-native `git worktree` dynamically, specifically designed for:

1. **Git Worktree Isolation (`WorkspaceManager::create_worktree`)**: TaskDispatcher isolates tasks such as `CodeGeneration` by creating a dynamic `git worktree` at `.worktrees/<job_id>`. This executes purely via `std::process::Command` ("git") to remain dependency-light.
2. **Mandatory Pruning & Cleanup (`WorkspaceManager::cleanup_worktree`)**: Regardless of Success or Failure, the worktree is completely dropped post-execution (Ghost Town Prevention).
3. **Port Conflict Resolution (Test-Gated Integration)**: Worktrees pass `AIOME_WORKTREE_ID` environment variable to their Conductor/Docker layer. `Oracle::run_integration_tests` enforces a pre-validation testing phase mapped using dynamic port offsets resolving by `AIOME_WORKTREE_ID`.
4. **Soft Isolation Fallback**: In environments where `.git` is absent, the system gracefully degrades to a "Soft Isolation Mode" avoiding hard errors.
5. **Opt-in Isolation**: Lightweight categorization allows tasks to run directly in main memory, skipping worktrees if they do not manipulate the local Rust codebase (e.g., General planning tasks, LLM summary tasks).
6. **DAG Execution Pre-requisites**: `TrajectoryStep` adds `depends_on: Vec<u32>` to support parallelized merging logic, resolving DAG nodes appropriately.

## Consequences

### Positive
- **No Cascading Overwriting**: The primary `.git` workspace is strictly treated as read-only / target mode until `merge_worktree` passes the final Oracle Consensus layer.
- **Port-level Parallelism**: Independent agents can run independent `cargo test` logic on independent UI logic without `database is locked` issues.
- **Traceability**: All failed experiments die with their worktree but the `Job` log contains context on why it failed, drastically speeding up causal learning.

### Negative
- Slower initial boot-time per heavy job as `git worktree add` must clone index cache.
- Hard disks with slow random I/O might experience throttling issues during heavy `cargo build` within new worktrees. This requires eventual incremental compilation caching (`sccache`) across worktrees.
