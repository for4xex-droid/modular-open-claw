# Aiome Quickstart Guide 🚀

Aiome (Autonomous AI Operating System) is designed to be fully local, private, and extensible. This guide will help you get the entire system up and running with Docker Compose **local builds** (no GHCR login required).

## 🛠️ Prerequisites

- **Docker & Docker Compose**: Make sure Docker is installed and running on your system.
- **System Requirements**: 
  - 8GB RAM minimum (16GB recommended for running larger LLM models)
  - 20GB free disk space
- **macOS Users**: We highly recommend installing [Ollama natively](https://ollama.com/download/mac) to leverage Apple Silicon (M1/M2/M3) GPU acceleration, rather than running it inside Docker.

---

## 🏃 Getting Started

### 1. Clone the Repository

```bash
git clone https://github.com/motivationstudio-llc/aiome.git
cd aiome
```

### 2. Start the System

```bash
docker compose -f docker-compose.quickstart.yml up -d --build
```

> **Note**: The first run **builds** the API Server and Management Console from source (often **10+ minutes** cold). Later starts are much faster.  
> After pulling code changes that affect the API binary (Mock / IntentFirewall / etc.), **always** use `up -d --build` — reusing an old image without rebuild can crash at startup.  
> If your Compose is older and still tries to pull private GHCR tags, use:  
> `docker compose -f docker-compose.quickstart.yml up -d --build --pull never`

### 3. Access the Dashboard

👉 **[http://localhost:1420](http://localhost:1420)**

You will be greeted by the beginner-friendly setup wizard which will guide you through:
1. Naming your AI Agent
2. Selecting your preferred LLM Engine (Local Ollama, LM Studio, or Cloud APIs)
3. Setting your experience level

---

## 🍎 macOS Optimization (Pattern B — native Ollama)

If you are on a Mac and want hardware acceleration (Metal), use the **override compose file** (no need to edit the main file):

```bash
# Host: Ollama.app running, model pulled once
ollama pull gemma4:26b

# Stop container Ollama if it was running (port 11434)
docker stop aiome-ollama 2>/dev/null || true

# Start API + MC only, pointing at host Ollama
docker compose -f docker-compose.quickstart.yml \
  -f docker-compose.quickstart.native-ollama.yml \
  up -d --build api-server management-console
```

Or use the helper script:

```bash
./scripts/local_llm_setup.sh pattern-b-check
./scripts/local_llm_setup.sh pattern-b-up
```

### Pattern A (default — Docker Ollama, lighter `gemma4:e4b`)

```bash
docker compose -f docker-compose.quickstart.yml up -d --build
./scripts/local_llm_setup.sh pattern-a   # pull gemma4:e4b into the container
```

See also: `./scripts/local_llm_setup.sh status`

---

## 🛑 Stopping the System

```bash
docker compose -f docker-compose.quickstart.yml down
```

This will stop and remove the containers, but all your AI's memories, configurations, and ledger data are safely preserved in your local Docker volumes (`aiome_data`).

---

## 📚 What's Next?

- Head over to the **Settings** tab in the app to switch your View Mode from `Beginner` to `Advanced` to unlock full developer capabilities (MCP integrations, Federation, LoRA training).
- Check out the [Architecture Documentation](../architecture/ARCHITECTURE.md) to understand the internals of the Samsara Hub.
- **Release verification (Human)**: Before Public Beta, complete the checklist in [Quick Start Verification](QUICK_START_VERIFICATION.md) (warm 5-minute end-to-end run).
