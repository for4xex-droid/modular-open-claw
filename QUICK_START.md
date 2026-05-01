# Aiome Quickstart Guide

This guide will get you up and running with the Aiome platform locally using Docker. 
This is the **MVP v1.0 Quickstart** configuration, designed to launch the minimal core of the Autonomous AI Operating System.

## Prerequisites

- **Docker** and **Docker Compose** installed.
- At least 8GB of free RAM.
- (Optional but Recommended) A local installation of [Ollama](https://ollama.com/) if you are on macOS to leverage Metal GPU acceleration.

## 🚀 1. Launch the Stack

Clone the repository and run the Docker Compose quickstart configuration:

```bash
cd aiome
docker-compose -f docker-compose.quickstart.yml up -d
```

This will launch:
1. **Aiome API Server** (Port 3015)
2. **Aiome Management Console** (Port 1420)
3. **Ollama** (Port 11434, inside Docker - unless configured otherwise)

## 🔐 2. Authentication

Open the Management Console in your browser:
**[http://localhost:1420](http://localhost:1420)**

You will be presented with a login screen asking for a "Secret Key".
By default, the Quickstart environment uses the following secret:

```
quickstart_secret_change_in_production
```

> **Note:** Behind the scenes, the Management Console automatically exchanges this secret for a secure, short-lived JWT token to communicate with the backend, adhering to the Aiome v1.0 security architecture.

## ⚠️ Docker Environment Constraints

Please be aware of the following limitations when running in this Quickstart environment compared to a full production deployment:

1. **Nurture API (Commerce/DRM) is Disabled:**
   - The Quickstart environment does not connect to the global Project Nurture ecosystem.
   - Features such as Agentic Commerce, Skill Vault (DRM), and AI-to-Consumer (A2C) monetization are **disabled**.

2. **Simplified Identity (eKYC) Flow:**
   - EKYC functionality is available in a "mock" mode unless a valid `STRIPE_API_KEY` is provided in the `docker-compose.quickstart.yml` file.

3. **Authentication Mechanism:**
   - Quickstart uses the `API_SERVER_SECRET` fallback to generate local admin JWT tokens.
   - In a true Production environment (`docker-compose.production.yml`), authentication relies strictly on `JWT_PRIVATE_KEY_B64` ED25519 signatures, and `API_SERVER_SECRET` is NOT used for user login.

4. **Security Isolation:**
   - The Quickstart configuration omits critical container security options (like `IPC_LOCK`, read-only `rootfs`) to maximize compatibility and startup speed. **Do not use this compose file for public-facing deployments.**

## 🛑 3. Shutting Down

To safely spin down the environment without losing data (data is stored in Docker volumes):

```bash
docker-compose -f docker-compose.quickstart.yml down
```

To wipe all data and start fresh:

```bash
docker-compose -f docker-compose.quickstart.yml down -v
```
