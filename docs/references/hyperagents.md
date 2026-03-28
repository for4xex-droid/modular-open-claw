# Hyperagents: A New Way to Auto-Research

**Source**: [X (Twitter) - AT (@neural_avb)](https://x.com/neural_avb/status/2037526862964969583)
**Date Added**: 2026-03-28

## Overview
Hyperagents is an auto-research framework developed by Meta AI and university researchers. It aims to create self-improving AI systems based on the concept of "Darwin-Gödel Machines."

## Key Concepts
- **Recursive Tree Exploration**: Maintains an archive of states/actions with parent-child relationships, structured as a tree (DAG).
- **Task Agent vs. Meta Agent**:
  - **Task Agent**: Executes specific research tasks.
  - **Meta Agent**: Evaluates the Task Agent's performance, analyzes failures (diagnostics), and re-designs the system's own improvement mechanisms.

## Relevance to Aiome
This architectural model perfectly aligns with Aiome's trajectory and self-improvement goals:
- **Trajectory Pipeline**: Aiome's `TrajectoryStore` and `TrajectoryStep` with `state_hash` / `parent_state_hash` (introduced in Phase 48) provide the exact DAG archive structure described.
- **Diagnostics**: Aiome's `AgentRxDiagnostics`, `fetch_diagnosis`, and `store_diagnosis` already implement the foundational "Meta Agent" layer.
- **Self-Improvement**: ADR-023 (AI-Scientist Loop) can adopt Hyperagents' theoretical approach to strengthen the recursive evaluation cycle.
