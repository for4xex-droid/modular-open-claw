# ADR-042: Auto RIPPLE_MAP Architecture & Cortex Intelligence Integration

## Status
Accepted

## Context
Aiome has integrated economic observability and complex intelligence structures (Graphify-derived context). As the system scales, changes to core infrastructure often cascade across authentication, telemetry, frontend UI, and database schemas. 
Our Perfect Planning framework highlighted that manual impact analysis (e.g., tracking `cortex_query` dependencies) is error-prone. The `.context/RIPPLE_MAP.md` is our source of truth for semantic dependencies, but its manual maintenance leads to staleness and "Zero-Panic" policy violations. Furthermore, as we integrate Cortex Confidence Tags and God Node detections, we need a way to track the blast radius of changing central intelligence components.

## Decision
We will implement an **Auto RIPPLE_MAP Architecture**:
1. **Dynamic AST Analysis**: Utilize the existing AST tools (`nurture_auditor.py` and `impact_query.py`) to automatically update `.context/RIPPLE_MAP.md` during CI or via a dedicated `/docs-sync` workflow, tracing `use` statements and module bounds.
2. **Cortex Native Integration**: Map semantic nodes from the `cortex_concept_index` directly into the Auto RIPPLE_MAP schema. This enriches static code analysis with Graphify's "God Node" domain intelligence, ensuring that highly connected concepts are treated as critical infrastructure.
3. **Evidence Quality Enforcement**: Architecture modifications affecting central God Nodes must pass stricter review thresholds. The RIPPLE_MAP will cross-reference `confidence` tags to map code boundaries, rejecting automated changes that threaten the stability of "extracted" (confidence >= 0.8) core modules.

## Consequences
- **Positive**: Eliminates human error in dependency tracking; guarantees "Zero-Panic" compliance by proactively catching hidden downstream dependencies before runtime.
- **Negative**: Increases CI/workflow execution time due to deep semantic and AST processing; requires maintaining the AST tools to handle evolving Rust module structures.
