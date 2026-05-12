# Aiome Quickstart Guide 🚀

Aiome (Autonomous AI Operating System) is designed to be fully local, private, and extensible. This guide will help you get the entire system up and running with just a few commands using Docker.

## 🛠️ Prerequisites

- **Docker & Docker Compose**: Make sure Docker is installed and running on your system.
- **System Requirements**: 
  - 8GB RAM minimum (16GB recommended for running larger LLM models)
  - 20GB free disk space
- **macOS Users**: We highly recommend installing [Ollama natively](https://ollama.com/download/mac) to leverage Apple Silicon (M1/M2/M3) GPU acceleration, rather than running it inside Docker.

---

## 🏃 Getting Started

### 1. Clone the Repository

First, clone the Aiome repository to your local machine:

```bash
git clone https://github.com/motivationstudio-llc/aiome.git
cd aiome
```

### 2. Start the System

We provide a streamlined Docker Compose file designed for quick, zero-configuration local setups.

```bash
docker compose -f docker-compose.quickstart.yml up -d
```

> **Note**: The first time you run this command, Docker will download the necessary images (API Server, Management Console, and optionally Ollama). This may take a few minutes depending on your internet connection.

### 3. Access the Dashboard

Once the containers are running, open your browser and navigate to the Aiome Management Console:

👉 **[http://localhost:1420](http://localhost:1420)**

You will be greeted by the beginner-friendly setup wizard which will guide you through:
1. Naming your AI Agent
2. Selecting your preferred LLM Engine (Local Ollama, LM Studio, or Cloud APIs)
3. Setting your experience level

---

## 🍎 macOS Optimization

If you are on a Mac and want to use hardware acceleration (Metal):

1. Install [Ollama for macOS](https://ollama.com/download/mac).
2. Open `docker-compose.quickstart.yml` in your editor.
3. Comment out or delete the entire `ollama:` block.
4. Under the `api-server` section, update the environment variable to point to your host machine:
   ```yaml
   OLLAMA_HOST: "http://host.docker.internal:11434"
   ```
5. Run `docker compose -f docker-compose.quickstart.yml up -d`.

---

## 🛑 Stopping the System

To shut down the Aiome ecosystem gracefully:

```bash
docker compose -f docker-compose.quickstart.yml down
```

This will stop and remove the containers, but all your AI's memories, configurations, and ledger data are safely preserved in your local Docker volumes (`aiome_data`).

---

## 📚 What's Next?

- Head over to the **Settings** tab in the app to switch your View Mode from `Beginner` to `Advanced` to unlock full developer capabilities (MCP integrations, Federation, LoRA training).
- Check out the [Architecture Documentation](../architecture/ARCHITECTURE.md) to understand the internals of the Samsara Hub.
