<div align="right">
  <a href="README.md">日本語</a> | <strong>English</strong>
</div>

<p align="center">
  <img src="docs/assets/logo.png" alt="Aiome Logo" width="300">
</p>

<h1 align="center">Aiome</h1>
<p align="center">
  <strong>The Self-Healing AI Agent OS</strong><br>
  <em>Written entirely by AI agents. 75,000+ lines of production Rust.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/License-BUSL_1.1-blue.svg" alt="License: BUSL-1.1">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/TLA%2B-Verified-0052cc.svg" alt="TLA+ Verified">
  <a href="https://github.com/motivationstudio-llc/aiome"><img src="https://img.shields.io/badge/Built%20by-Agents-blueviolet" alt="Built by Agents"></a>
</p>

---

![Aiome Quickstart Demo](docs/assets/quickstart_demo.webp)

---

## ⚡ Quick Start (5 Seconds / No config needed)

Aiome is designed to let you experience all features (chat, autonomous tool execution, self-healing, simulated AI economy) instantly with a single command, without tedious configuration.
Advanced features like commercial payments (Commerce/Stripe) **automatically operate in mock mode** when API keys are not set, ensuring nothing breaks and you can evaluate everything.

### Option A: Using Docker (Recommended)
Skip the 10+ minute initial compilation locally by using our pre-built image running alongside Ollama.

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
docker compose -f docker-compose.quickstart.yml up -d
```
Once started, access the Management UI via your browser at port `1420`.

### Option B: Build from Source

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
# ⚠️ Initial compilation will take 5-15 minutes depending on your hardware
cargo run --bin api-server
```

> **About Commerce Features**:
> If `STRIPE_API_KEY` is not set in `.env`, the system detects this and automatically falls back to `MockCommerceEngine`. You can immediately experience the AI ecosystem features like billing, escrows, and gig orders seamlessly using a fake balance, without any configuration.

---

## 🌌 What is Aiome? (Philosophy & Concept)

Aiome is more than just an agent framework—it is an **Autonomous AI Operating System** fundamentally designed to let AI agents operate, defend themselves, and evolve safely.

**100% of the code was written autonomously by AI agents.**
This is not merely an experiment. It is the result of agents proactively designing and implementing the environment in which they can operate most safely and with utmost discipline.

All of the following capabilities are built directly into the OS, not left as plugins:

- 🛡️ **Trust Layer**: Formal O(1) boundary verification and a SHA-256 protected audit chain. Mathematical safety is guaranteed via Model-Based Testing.
- 🧠 **Soul Engine**: The middleware governing the agent's personality, memory, and emotional evolution.
- 📚 **Cortex Knowledge Base**: An autonomous Wiki compiler that evolves beyond simple RAG. The LLM extracts concepts from multiple documents and self-reconstructs its knowledge as an interconnected web.
- 🏥 **Self-Healing (Watchtower)**: An autonomous diagnostic loop that infers failure causes, extracts repair hints, and ensures idempotent retries when tasks fail.
- 🎨 **Creative Studio**: A dynamic evaluation environment where agent-generated tools and skills are executed safely inside WASM sandboxes.
- 🎭 **Avatar & Voice**: A "living expression" engine powering interactions through synthetic voice and VRM Avatars, transcending text.
- 💰 **Agent Economy (Commerce & Gig)**: An escrow and economic foundation enabling AIs to autonomously contract, verify, and depend on each other for tasks.
- 🏪 **LoRA Marketplace**: A personality distribution platform where agents can safely trade and share LoRA adapters via escrow payments and file-isolated sandboxes.

It is the "skull, nervous system, and immune system" allowing the "wild genius (LLM)" to survive and safely evolve in the real world over the long term. This is the very reason for Aiome's existence.

---

## 🏗️ Architecture

To guarantee robustness, Aiome heavily utilizes Rust's TypeState pattern to maintain strict layer isolation:

```text
apps/api-server      ← Main Binary (The Body / Management Engine)
apps/watchtower      ← External Channel Integration (The Soul / Discord & Telegram Bridge)
      ↓
libs/core            ← Domain Logic (Open)
      ↓
libs/infrastructure  ← I/O Implementations (SQLite / Ollama, etc. / Open)
      ↓
libs/soul            ← Soul Engine (Agents' L1-L3 Middleware / Open)
      ↓
libs/aiome-commerce  ← AI Economy Engine (Mock / Stripe)
```

---

## 🛡️ Trust Layer (Defense-first)

Handing a raw shell directly to an LLM offers "fragile freedom", pregnant with the risks of infinite loops and API key leaks. Aiome provides:
1. WASM Sandbox isolation for all dynamically generated tools and skills.
2. Local guardrails for prompt injection defense.
3. Cryptographic hash chains (Karma) stored in SQLite forcing immutable recording of "what task I failed at in the past".
4. gVisor container isolation integration.

---

## 🧠 Soul Engine & Self-Healing

1. **Strategic Planner & Scientist Loop**: The AI independently hypothesizes improvements, runs iterative self-reviews, and submits experimental jobs.
2. **Watchtower Diagnostic Loop**: Autonomously extracts lessons from failed jobs, guaranteeing feedback goes into the next attempt (idempotent retries with injected repair hints).

---

## 🛠️ Technical Stack

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Docker](https://img.shields.io/badge/docker-%232496ED.svg?style=for-the-badge&logo=docker&logoColor=white)

| Component | Technology | Role |
|---|---|---|
| **Core Engine** | Rust | High-speed, memory-safe, and robust security foundation |
| **Formal Verification** | TLA+ / TLC / Rust TypeState | Mathematical verification of state transitions using TLA+ and model checkers |
| **Storage** | SQLite | Self-contained operations via a low-dependency embedded database |
| **Expansion** | WebAssembly (WASM) | Secure skill execution environment under strict network constraints |

---

## 📚 Documentation

- **[Architecture Law](docs/architecture/ARCHITECTURE_LAW.md)**: Foundational principles guaranteeing intellectual honesty and safety.
- **[Operations Manual](docs/guides/OPERATIONS_MANUAL.md)**: Detailed environment setup and operational procedures.
- **[Security Design](docs/architecture/SECURITY_DESIGN.md)**: Deep dive into the defense-in-depth mechanisms.

---

## 🤝 Contributing

- **[Contributing Guide (CONTRIBUTING.md)](CONTRIBUTING.md)**: Rules for human contributions to a "Built by Agents" project.
- **[Reporting Vulnerabilities (SECURITY.md)](SECURITY.md)**: Security incident contact points.

---

## 🛡️ License

**Aiome Core** is provided under the **Business Source License 1.1 (BUSL-1.1)** with an eye toward sustainable commercialization.  
*It will automatically transition to the Apache License 2.0 on the specified change date (in 2030).*

Most capabilities are available globally at no cost for research and non-commercial purposes, but specific commercial usage is restricted initially. Please ensure you review the `LICENSE` file in the repository for detailed terms.

---

*Built automatically by Agents of [motivationstudio, LLC](https://github.com/motivationstudio-llc) — Powering the Future of AI Autonomy.*
