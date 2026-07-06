<div align="right">
  <a href="README.md">日本語</a> | <strong>English</strong>
</div>

<p align="center">
  <img src="docs/assets/logo/Aiomeロゴ（横長120×500）.png" alt="Aiome Logo" width="400">
</p>

<h1 align="center">Aiome</h1>
<p align="center">
  <strong>The sovereign OS for autonomous AI — own it, govern it, let it earn.</strong><br><br>
  <a href="https://aiome.dev"><strong>aiome.dev (Official Website)</strong></a><br><br>
  Put an autonomous AI team for SEO, social media, and research on your own machine.<br>
  Your data never leaves, every action is auditable, and results show up as numbers.
</p>

<p align="center">
  <img src="https://github.com/motivationstudio-llc/aiome/workflows/CI/badge.svg" alt="CI Status">
  <img src="https://img.shields.io/badge/License-BUSL_1.1-blue.svg" alt="License: BUSL-1.1">
  <img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/TLA%2B-Verified-0052cc.svg" alt="TLA+ Verified">
  <a href="https://github.com/motivationstudio-llc/aiome"><img src="https://img.shields.io/badge/Built%20by-Agents-blueviolet" alt="Built by Agents"></a>
  <a href="https://aiome.dev"><img src="https://img.shields.io/badge/Website-aiome.dev-00f2ff.svg" alt="Website"></a>
</p>

---

[![Aiome Quickstart Demo](docs/assets/quickstart_demo.webp)](#)
*(Coming Soon)*

---

## ✨ What Aiome Promises (Three Pillars)

| Pillar | Promise | Proof |
|---|---|---|
| 🏠 **Sovereign — You own it** | Fully self-hosted, **$0/month**. Agent memory and logs never leave your machine | One-command Docker setup in 5 minutes, MCP support with no lock-in, BSL 1.1 (auto-transitions to Apache 2.0 in 2030) |
| 🛡️ **Governed — You govern it** | Run autonomously without runaway risk | 26-screen management console (audit logs, approval queue, root-cause analysis, LLM stats) + three-layer defense: Trust Layer / Cell isolation / WASM sandbox |
| 💰 **Earning — You let it earn** | AI works; results are visible | Four official Playbooks (**SEO operations / SNS operations / competitive research / support triage**) for day-one workflows. The Nurture economy lets AI transact and returns value to you |

After completing the setup wizard, pick one Playbook and your operational workflow starts immediately.

---

## ⚡ Quick Start (5 Seconds / No config needed)

> [!TIP]
> **$0 / month 💸**
> Self-host Aiome on your own machine using Docker / Podman and get an advanced AI Agent OS for **$0 per month** with unlimited usage. All features are included by default.

Aiome is designed to let you experience all features (chat, autonomous tool execution, self-healing, simulated AI economy) instantly with a single command, without tedious configuration.
Advanced features like commercial payments (Commerce/Stripe) **automatically operate in mock mode** when API keys are not set, ensuring nothing breaks and you can evaluate everything.

> **📖 Detailed Specifications and Constraints:**
> Please ensure you read [QUICK_START.md](QUICK_START.md) regarding authentication flows and feature limitations within the Docker Quickstart environment.

### Option A: Using Docker / Podman (Recommended)
Skip the 10+ minute initial compilation locally by using our pre-built image running alongside Ollama.

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
# Standard Quickstart (Mocked payments)
docker compose -f docker-compose.quickstart.yml up -d

# Or, start in Commercial Mode (Nurture Engine / Live Stripe payments enabled)
docker compose -f docker-compose.commercial.yml up -d
```
Once started, access the Management UI via your browser at port `1420`.

> **💡 Note for Podman Users**:
> In Podman rootless environments, `host.docker.internal` might not resolve by default. If you cannot connect to your local Ollama instance, please change `OLLAMA_HOST` in `docker-compose.quickstart.yml` to `http://host.containers.internal:11434`, or run Ollama within the container runtime (as configured in the default compose file).

### Option B: Build from Source

> [!IMPORTANT]
> **Production Security**:
> A strong `API_SERVER_SECRET` environment variable (minimum 16 characters) is strictly required to start the server. If it is not set or uses an insecure default, the process will panic (terminate) immediately to protect your environment. Release builds also require `A2A_NODE_TOKEN`. See the [Operations Manual](docs/guides/OPERATIONS_MANUAL.md) for details.

```bash
git clone https://github.com/motivationstudio-llc/aiome
cd aiome
# ⚠️ Initial compilation will take 5-15 minutes depending on your hardware
API_SERVER_SECRET="my_super_secret_key_123456" cargo run --bin api-server # gitleaks:allow
```

> **About Commerce Features**:
> If `STRIPE_API_KEY` is not set in `.env`, the system detects this and automatically falls back to `MockCommerceEngine`. You can immediately experience the AI ecosystem features like billing, escrows, and gig orders seamlessly using a fake balance, without any configuration.
> 
> **About Official X (Twitter) Integration**:
> By setting `X_TWITTER_CLIENT_ID` and `X_TWITTER_CLIENT_SECRET` in `.env`, you can enable automated social media management such as tweet posting and search functionality via the official X MCP server.

---

## 🌌 What is Aiome? (Philosophy & Concept)

Aiome is a self-hosted OS where AI agents live and work safely.
It is more than just an agent framework—an **Autonomous AI Operating System** designed to delegate real work to agents while auditing, approving, and analyzing every action they take.

**100% of the code was written autonomously by AI agents.**
This is not merely an experiment. It is the result of agents proactively designing and implementing the environment in which they can operate most safely and with utmost discipline.

## 💰 Aiome × Nurture — Autonomous AI Economy

**Aiome is the body (OS); Nurture is the heart (economy engine).**
Nurture is the commercial engine that runs on Aiome (`commercial/`, BSL 1.1), giving AI economic agency—AI buys, sells, and gives back to you—together forming a sovereign autonomous AI economy you can own.

```mermaid
graph LR
    subgraph OSS["Aiome — OS layer (OSS)"]
        OS["Agent execution · 3-layer defense · audit"] --> CT["Contract layer<br/>CommerceEngine / AiomePlugin / JobQueue"]
    end
    subgraph COM["Nurture — economy layer (commercial/)"]
        EC["Dual currency · Merkle ledger · marketplace"] --> NB["nurture-bridge (sole integration point)"]
    end
    NB -- "implements traits (one-way dependency)" --> CT
```

### Why the economy can't be bolted on

Other products bolt the economy on as a plugin. In Aiome, the economy interface (`CommerceEngine`) is defined in the OSS contract layer from day one, and the commercial Nurture engine implements it with a **one-way dependency**. That means the mock economy runs fully on the OSS build, coin conservation is verified by TLA+ (`CoinsConserved`), and every transaction is chained into a Merkle audit log.

**Deep Dive**: [Integration design](docs/architecture/AIOME_NURTURE_SYNERGY.md) · [Economy TLA+ spec](commercial/specs/NurtureEconomyProtocol.tla) · [ADR-011 Bridge isolation](commercial/docs/decisions/011-nurture-bridge-isolation.md)

| Transaction Model | Description |
|---|---|
| 🏪 AI Goes Shopping (B2A) | Agents autonomously discover and purchase digital assets like LoRA personalities, VRM avatars, and voice models under CSAM triple-layer defense and on-memory DRM. |
| 🤝 Agents Trade Skills (A2A) | WASM skills and knowledge are autonomously exchanged between agents via escrow contracts and the Karma reputation registry to expand capabilities. |
| 🎁 AI Gives Back (A2C) | The OS senses user dedication and triggers surprise gifts or real-world rewards (via Tremendous API, etc.) based on Karma score and care streaks (Easter Egg strategy). |

When Stripe API keys are not configured in `.env`, the system automatically falls back to `MockCommerceEngine`. You can immediately experience all economic features (billing, skill trading, and gifts) using fake balances, completely free.

For detailed technical specifications, transaction protocols, and sequence diagrams, please refer to [AIOME_NURTURE_SYNERGY.md](docs/architecture/AIOME_NURTURE_SYNERGY.md). For external-facing details, see also [commercial/README.md](commercial/README.md).

---

All of the following capabilities are built directly into the OS, not left as plugins:

- 🛡️ **Trust Layer**: Formal O(1) boundary verification and a SHA-256 protected audit chain. Includes a "Governed Execution" layer that mandates user intervention for high-risk tool calls, and introduces a new Formal Verification/Theorem Proving layer powered by OxiLean.
- 🦠 **Cell-Based Architecture (CBA)**: Enforces physical process-level cell isolation, strict `CELL_ID` validation, and SQLite database path limits during preflight. Halts immediately if unauthorized path access is detected, ensuring robust protection against path traversal, shell injection, and cross-cell data contamination.
- 🕸️ **GEO Intelligence**: Generative Engine Optimization (GEO) audit engine. Built with an asymmetric Graceful Degradation design to autonomously maintain SEO and publishing pipelines even during infrastructure outages.
- 🔐 **Zero-Trust Hardening**: Rust 2024 Edition compliant centralized secret purging via `scrub_env`, fortified by comprehensive SSRF defense mechanisms that strictly block IPv4-mapped IPv6 and Link-local addresses.
- 🔒 **P2P Federation E2E Encryption**: Integrates an end-to-end encryption protocol using X25519 Diffie-Hellman key sharing (with 0-RTT/1-RTT Montgomery key conversion from Ed25519 identity keys), HKDF-SHA256 key derivation, and ChaCha20-Poly1305 (AEAD) encryption to secure P2P traffic against eavesdropping or message tampering at Samsara Hub.
- 🧠 **Soul Engine**: The middleware governing the agent's personality, memory, and emotional evolution.
- ⚖️ **Governed Execution Layer**: Atomic security gating for multi-step plans and a semantic elicitation workflow for human-in-the-loop safety.
- 📚 **Cortex Knowledge Base**: An autonomous Wiki compiler that evolves beyond simple RAG. Featuring Progressive Disclosure and Query File-Back, the LLM extracts concepts from multiple documents, self-reconstructs its knowledge as an interconnected web, and compounds knowledge through self-querying.
- 🏥 **Self-Healing (Watchtower)**: An autonomous diagnostic loop that infers failure causes, extracts repair hints, and ensures idempotent retries when tasks fail. Fortified with a "Verify-to-Iterate Loop" that intercepts Oracle validation rejections (Reject/Revise) and feeds structural feedback back to the planner for self-repair.
- 💾 **Crash Recovery & Backup**: Automated WAL-safe online snapshots using `sqlite3 .backup` coupled with a Pre-migration Guard. A bulletproof data protection architecture ensuring rapid recovery from unpredictable failures or migration corruptions.
- 🎨 **Creative Studio**: A dynamic evaluation environment where agent-generated tools and skills are executed safely inside WASM sandboxes.
- 🎭 **Avatar & Voice**: A "living expression" engine powering interactions through synthetic voice and VRM Avatars, transcending text.
- 💰 **Agent Economy (Commerce & Gig)**: An escrow and economic foundation enabling AIs to autonomously contract, verify, and depend on each other for tasks. Equipped with a self-responsibility "Resilient Escrow Refund" architecture that ensures instantaneous fund release upon task failure or UI rejection.
- 🏪 **LoRA Marketplace**: A personality distribution platform where agents can safely trade and share LoRA adapters via escrow payments and file-isolated sandboxes.
- 📣 **Buzz Protocol (Autonomous SNS Worker)**: An autonomous background worker that drafts and schedules social media content based on trends, LLM generation, and daily quotas. Includes a built-in Approval/Rejection interface for human-in-the-loop safety before publishing.
- 🛡️ **Autonomous Support System**: A fully integrated system with external channels (e.g., Discord) for automated incident classification, response generation, escalation, and feedback loops. By automatically embedding ticket IDs (`[TICKET:uuid]`) in responses and detecting user reactions (using `OnceLock` for Bot ID caching and `LazyLock` for regex caching to minimize API calls), it dynamically updates weights in the Karma Registry (agents' long-term memory weights) for real-time self-evolution.
- 📡 **TrendSonar Integration**: Real-time ingestion of external trend signals (e.g., X API, SERP). Employs `FuturesUnordered` for highly concurrent fetching and features robust autonomous handling of `429 Retry-After` responses, completely preventing thread stalls and protecting API quotas. The dynamic factory pattern allows zero-downtime, instant reconfiguration when API keys change via the Management Console.
- 🔌 **Dynamic MCP Federation**: Full support for the Model Context Protocol (MCP). Instantly mount officially provided MCP packages such as **"Automated GitHub Issue triage", "Bidirectional Notion Knowledge Base sync", and "Real-time Web Search"**. Includes seamless integration via the GUI dashboard and strict security validation to prevent path traversal and malicious URL schemes.
- 🎨 **Premium Management Console**: 100% Token-driven UI system. Centralized via `tokens.css` to eliminate all hardcoded HEX/RGBA color values. A bulletproof management panel, including a real-time semantic elicitation flow (AwaitingInput Overlay).

It is the "skull, nervous system, and immune system" allowing the "wild genius (LLM)" to survive and safely evolve in the real world over the long term. This is the very reason for Aiome's existence.

---

## 🏗️ Architecture

To guarantee robustness, Aiome heavily utilizes Rust's TypeState pattern to maintain strict layer isolation. Furthermore, the commercial payment and economic sidecar engine is unified under the `commercial/` directories.

```text
apps/api-server      ← Main Binary + Watchtower (Body / Soul / Discord Bridge)
apps/samsara-hub     ← P2P Federation (Hub / CRDT Sync)
apps/management-console ← Premium Management Console (Vite + React / 100+ Components)
apps/key-proxy       ← Key Proxy (AbyssVault / WordPress Integration)
      ↓
commercial/apps/nurture-api ← Nurture Commercial Economy Engine (BUSL-1.1)
commercial/libs/*           ← Commercial payment protocols, bridge, and infra
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

## 🛡️ Proof of Safety — Trust Layer (Defense-first)

This is the evidence behind the "governed" promise. Core logic is formally verified with TLA+ and implemented in 146,000+ lines of zero-panic Rust (3,500+ automated tests).

Handing a raw shell directly to an LLM offers "fragile freedom", pregnant with the risks of infinite loops and API key leaks. Aiome provides:
1. WASM Sandbox isolation for all dynamically generated tools and skills.
2. Local guardrails for prompt injection defense.
3. Cryptographic hash chains (Karma) stored in SQLite forcing immutable recording of "what task I failed at in the past".
4. gVisor container isolation integration.
5. **GlassWorm Shield**: Comprehensive deployment of an ultra-fast sanitizer to prevent stealth attacks and LLM poisoning using invisible Unicode characters.
6. **Impact Analysis Protocol**: A structured `grep_search`-based dependency tracing protocol and semantic dependency map (`RIPPLE_MAP.md`) natively built-in, preventing unknown cascade errors during autonomous code refactoring by agents.
7. **Automated Chaos Engineering**: Natively built fault-injection framework running steady-state resilience tests (e.g., simulating LLM timeouts and malformed responses) to mathematically guarantee graceful degradation against unpredictable AI failures.
8. **Cell-Based Architecture (CBA)**: Physical path isolation governed by the 1-process=1-cell invariant. Implements multi-layered defense against path traversal and shell injection via `AppDataResolver` and shell guards.
9. **GDPR/RTBF & Content Compliance**: Guarantees complete physical data purge (via `forget_actor`) across up to 7 tables in a single atomic transaction, coupled with secure downstream deletion propagation (Zero-Trust Sync). Additionally, integrates an automated safety filter to detect and filter harmful content.
10. **Aegis Sentinel**: Actively monitors WASM boundaries, autonomously generating LLM patches, verifying them with Kani, and executing real-time code HotSwaps to heal the system without downtime.
11. **Adaptive Immune System**: An active defense system that detects input threat patterns and prevents learning rule drift before execution (forming a multi-layered immune structure with the post-incident **Aegis Sentinel**).
12. **Multi-Context Sanitization**: Context-aware output sanitization (`SqlQuery` comment and double-quote removal, recursive `FilePath` traversal prevention, and `OnceLock`-driven panic-free `HttpHeader` CRLF stripping) to prevent injection and traversal bypasses.
13. **Sidecar Physical Validation**: Restricts the release packaging of mock sidecars (like `api-server` shells) via build/CI-time verification (`desktop_sidecar_manager.py`) of magic bytes and minimum file sizes (100KB).

---

## 🧠 Soul Engine & Self-Healing

1. **Strategic Planner & Scientist Loop**: The AI independently hypothesizes improvements, runs iterative self-reviews, and submits experimental jobs.
2. **Watchtower Diagnostic Loop**: Autonomously extracts lessons from failed jobs, guaranteeing feedback goes into the next attempt (idempotent retries with injected repair hints).
3. **Intelligence Layer (DreamState)**: A fully autonomous architecture where the AI autonomously generates experimental jobs or self-reflections during idle time, and self-discovers solutions via external MCP signals (ToolDiscovery) for unknown tasks.
4. **Arena Battle**: An evaluation and verification environment where autonomous agents compete via skills or decision models to self-select the optimal strategy.
5. **Society of Thought**: A multi-agent consensus engine where multiple decision-making agents discuss through prompts to reach an agreement.
6. **Memory Crystallizer**: A memory organization system that distills key decisions and lessons from accumulated short-term experiences, crystallizing and compressing them into long-term memory (such as `MEMORY.md`). Features multi-layered OOM defense (via limits on processing skills, batch partitioning, and char length caps), XML-delimited prompt injection mitigation, and localized failure recovery to ensure resilience against LLM API disruptions.
7. **TimesFM Forecast**: Integrates Google's TimesFM (Time-Series Foundation Model) to enable accurate forecasting of trends and asset demands within the autonomous economy.

---

## 🛠️ Technical Stack

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)
![Docker](https://img.shields.io/badge/docker-%232496ED.svg?style=for-the-badge&logo=docker&logoColor=white)
![Podman](https://img.shields.io/badge/podman-%23892CA0.svg?style=for-the-badge&logo=podman&logoColor=white)

| Component | Technology | Role |
|---|---|---|
| **Core Engine** | Rust | High-speed, memory-safe, and robust security foundation |
| **Formal Verification** | TLA+ / TLC / Rust TypeState | Mathematical verification of state transitions using TLA+ and model checkers |
| **Storage** | SQLite | Self-contained operations via a low-dependency embedded database |
| **Expansion** | WebAssembly / OxiLean | Secure skill execution environment and formal verification under strict network constraints |

---

## 📚 Documentation

- **[Developer Onboarding](docs/DEVELOPER_ONBOARDING.md)**: Guidelines for local environment setup, crate architecture, system data flows, and Nurture S2S integration.
- **[Architecture Law](docs/architecture/ARCHITECTURE_LAW.md)**: Foundational principles guaranteeing intellectual honesty and safety.
- **[Operations Manual](docs/guides/OPERATIONS_MANUAL.md)**: Detailed environment setup and operational procedures.
- **[Quick Start Verification Checklist](docs/guides/QUICK_START_VERIFICATION.md)**: G1 pre-release human walkthrough (5-minute end-to-end).
- **[Stripe Production Setup](docs/operations/stripe-production-setup.md)**: Production Price ID and env deployment (R2-1).
- **[Security Design](docs/architecture/SECURITY_DESIGN.md)**: Deep dive into the defense-in-depth mechanisms.

---

## ❓ FAQ

<details>
<summary><strong>Can autonomous agents run away?</strong></summary>

Aiome is designed with governance at its core. Dangerous operations wait for human approval in the approval queue, every action is recorded in audit logs, and agents can only run inside Cell (isolated process) and WASM sandboxes. Core logic is formally verified with TLA+.
</details>

<details>
<summary><strong>Where does my data go?</strong></summary>

Nowhere else. Aiome is fully self-hosted—agent memory, files, and logs all stay on your machine. You choose and connect the LLM you use.
</details>

<details>
<summary><strong>Will costs spiral out of control?</strong></summary>

The OS itself is self-hosted at $0/month. LLM usage is visualized in real time on the console's LLM stats screen, and economic features can be experienced in Mock mode without spending real money.
</details>

<details>
<summary><strong>Is there vendor lock-in?</strong></summary>

No. MCP (Model Context Protocol) support lets you connect external tools freely, and the license is BSL 1.1—automatic transition to Apache 2.0 in April 2030 is guaranteed in the license text.
</details>

---

## 🤝 Contributing

- **[Contributing Guide (CONTRIBUTING.md)](CONTRIBUTING.md)**: Rules for human contributions to a "Built by Agents" project.
- **[Reporting Vulnerabilities (SECURITY.md)](SECURITY.md)**: Security incident contact points.

---

## ⚖️ Legal & Privacy

With the public beta release of the product, we have established the following legal documents. Please review them before use.

- **[Terms of Service](docs/legal/TERMS_OF_SERVICE.md)**
- **[Privacy Policy](docs/legal/PRIVACY_POLICY.md)**

---

## 🛡️ License & Commercial Fees

Both **Aiome Core** and **Nurture Commercial Engine** are unified under the **Business Source License 1.1 (BUSL-1.1)** to ensure sustainable open-source monetization.  
*It will automatically transition to the Apache License 2.0 on the specified change date (April 1, 2030).*

### Pricing & Fees

| Tier | Audience | Price |
|---|---|---|
| **Free (Sovereign)** | Personal / research self-hosting | **$0/month** — full OS features including Mock economy |
| **Pro (Autonomous)** | Real economy unlocked | **$19.99/month** (14-day free trial) |
| **Agency (B2B)** | Multi-tenant operations | Coming Soon |

Purchase Pro via the [official landing page (aiome.dev/#pricing)](https://aiome.dev/#pricing) Stripe Payment Link. External copy SSOT: [`docs/marketing/MESSAGING.md`](docs/marketing/MESSAGING.md). Stripe setup: [`docs/operations/stripe-setup.md`](docs/operations/stripe-setup.md).

In-app commercial transactions (Gig fulfillment payments, asset purchases, etc.) carry a **15%** platform fee; **85%** goes to creators (same as the [Terms of Service](docs/legal/TERMS_OF_SERVICE.md)).

Please ensure you review the `LICENSE` and `commercial/LICENSE` files in the repository for detailed terms. The economy features do not guarantee any income.

---

*Built automatically by Agents of [motivationstudio, LLC](https://github.com/motivationstudio-llc) — Powering the Future of AI Autonomy.*
