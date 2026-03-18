# ADR 007: Preserve Intent Warning Suppression

## Status
Accepted

## Context
During the workspace cleanup phase (Phase B), we encountered approximately 400+ warnings related to `missing_documentation` and `unused_code`. 
A pure cleanup (removing unused code) often leads to:
1. Loss of developer intent (code left for future features or debugging).
2. Constant re-importing effort when features are resumed.
3. Merge conflicts with other branches.

## Decision
We will achieve a "Zero Warning" state NOT by removing code, but by **safely suppressing** warnings while preserving the code:
1. **Unused Imports/Variables/Dead Code**: Suppressed at the crate root level using `#![allow(unused_imports, unused_variables, dead_code, unused_mut)]` in `lib.rs` or `main.rs`.
2. **Specific Variables**: Prefixed with `_` if local suppression is preferred.
3. **Missing Docs**: All public structures MUST have documentation (`///`). If the logic is self-explanatory or generated, `#[allow(missing_docs)]` can be used at the module level ONLY after basic descriptions are provided.

## Consequences
- **Pros**: Zero compiler warnings in CI/CD. No loss of code infrastructure. High development velocity.
- **Cons**: Crate roots become slightly cluttered with `allow` attributes. Dead code is not physically removed, slightly increasing binary size (mitigated by LTO).
