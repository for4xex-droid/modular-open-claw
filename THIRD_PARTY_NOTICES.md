# Third-Party Notices

This file lists third-party projects that Aiome incorporates, references, or is inspired by.
It is provided in compliance with open-source license requirements and as a matter of good practice.

---

## Integrated via Docker / CLI (Runtime Dependencies)

### AutoResearchClaw
- **URL**: https://github.com/aiming-lab/AutoResearchClaw
- **License**: Apache License 2.0
- **Usage**: Autonomous research pipeline, integrated as a Docker-based service via `ResearchBridge`.
- **Copyright**: Copyright 2026 aiming-lab contributors

---

## Architecture & Design References

### MetaClaw
- **URL**: https://github.com/aiming-lab/MetaClaw
- **License**: Apache License 2.0
- **Usage**: Cross-run knowledge transfer concepts informed the Karma learning integration design.
- **Copyright**: Copyright 2026 aiming-lab contributors

### Inochi2D
- **URL**: https://github.com/Inochi2D
- **License**: BSD 2-Clause License
- **Usage**: 2D animation architecture concepts referenced for VRM/avatar rendering design.
- **Copyright**: Copyright Inochi2D Project contributors

---

## Datasets

### AItuber-Personas-Japan
- **URL**: https://huggingface.co/datasets/DataPilot/AItuber-Personas-Japan
- **License**: ODC-BY (Open Data Commons Attribution License)
- **Usage**: AItuber persona seed data (6-axis parameters + streaming theme lists) used to enrich `AnamnesisProfile` initialization and `DreamState` topic generation.
- **Copyright**: Copyright 2026 DataPilot contributors
- **Generated with**: Kimi K2.5 (Moonshot AI) via SDG-LOOM pipeline

---

## Research Papers

### Trojan's Whisper (Liu et al., 2026)
- **Citation**: Liu, F., Chen, Z., Lan, T., et al. "Trojan's Whisper: Stealthy Manipulation of OpenClaw through Injected Bootstrapped Guidance." arXiv:2603.19974.
- **License**: CC BY 4.0
- **Usage**: Security architecture design (BehaviorMonitor, Cleanroom multi-layer defense) informed by the countermeasures proposed in this paper.

---

## Rust Crate Dependencies

Aiome depends on numerous open-source Rust crates distributed via [crates.io](https://crates.io).
These dependencies and their licenses are managed by Cargo.

To generate a complete list of dependency licenses, run:

```bash
cargo install cargo-license
cargo license --json > docs/licenses.json
```

Key dependencies include (but are not limited to):

| Crate | License |
|-------|---------|
| tokio | MIT |
| axum | MIT |
| sqlx | MIT OR Apache-2.0 |
| serde | MIT OR Apache-2.0 |
| wasmtime | Apache-2.0 |
| reqwest | MIT OR Apache-2.0 |
| tracing | MIT |
| secrecy | MIT OR Apache-2.0 |

---

## npm Dependencies (Frontend)

### three-fluid-fx
- **URL**: https://github.com/artcodev/three-fluid-fx
- **License**: MIT
- **Usage**: Fluid simulation overlay effect for the LoginScreen and FluidAura effect for the VRM avatar.
- **Copyright**: (c) 2026 Artem Korenevych

---

## npm / Node.js Dependencies

Frontend dependencies for the Management Console (Tauri + React) are managed by npm.
Licenses can be audited with:

```bash
npx license-checker --summary
```

---

*Last updated: 2026-06-12*
