#!/bin/bash
# Aiome - GitHub Topics Setup Script (Postiz Growth Playbook P5)
# This script sets the SEO-optimized topics for the Aiome repository.

set -euo pipefail

if ! command -v gh &> /dev/null; then
    echo "❌ Error: GitHub CLI ('gh') is not installed."
    echo "Please install it from https://cli.github.com/ and run 'gh auth login' first."
    exit 1
fi

if ! gh auth status &> /dev/null; then
    echo "❌ Error: Not authenticated with GitHub CLI."
    echo "Please run 'gh auth login' before executing this script."
    exit 1
fi

REPO="motivationstudio-llc/aiome"
TOPICS=(
  "ai"
  "agent"
  "rust"
  "autonomous"
  "desktop-app"
  "llm"
  "self-hosted"
  "tla-plus"
  "docker"
  "podman"
  "ai-agents"
)

echo "Setting GitHub Topics for $REPO..."
for topic in "${TOPICS[@]}"; do
  gh repo edit "$REPO" --add-topic "$topic"
done
echo "✅ GitHub Topics updated successfully!"
