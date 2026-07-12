#!/usr/bin/env bash
# Aiome local LLM: Pattern A (Docker Ollama) / Pattern B (native Ollama) helpers + hygiene.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_A=(-f "$ROOT/docker-compose.quickstart.yml")
COMPOSE_B=(-f "$ROOT/docker-compose.quickstart.yml" -f "$ROOT/docker-compose.quickstart.native-ollama.yml")

# Models kept on native Ollama (Pattern B + Cursor MCP — not Aiome runtime but user tooling)
KEEP_NATIVE_OLLAMA=(
  "gemma4:26b"
  "deep-dive-local:latest"
)

# Pattern A target in Docker Ollama
DOCKER_OLLAMA_MODEL="${DOCKER_OLLAMA_MODEL:-gemma4:e4b}"

usage() {
  cat <<'EOF'
Usage: scripts/local_llm_setup.sh <command>

Commands:
  status          Show native + Docker Ollama models and quickstart env
  pattern-a       Pull gemma4:e4b into aiome-ollama (Pattern A)
  pattern-a-up    Start full quickstart stack (Pattern A, default compose)
  pattern-b-up    Start api-server + MC with native Ollama (Pattern B)
  pattern-b-check Verify host Ollama has gemma4:26b
  hygiene-dry-run List removable native Ollama models + HF cache (Aiome-unused)
  hygiene-apply   Remove listed native Ollama models + optional HF junk (see script)

Pattern A (quickstart default):
  docker compose -f docker-compose.quickstart.yml up -d --build

Pattern B (macOS Metal, existing gemma4:26b on host):
  docker stop aiome-ollama 2>/dev/null || true
  scripts/local_llm_setup.sh pattern-b-up
EOF
}

require_ollama_cli() {
  command -v ollama >/dev/null || { echo "ERROR: ollama CLI not found"; exit 1; }
}

require_docker() {
  command -v docker >/dev/null || { echo "ERROR: docker not found"; exit 1; }
}

is_kept_model() {
  local name="$1"
  for k in "${KEEP_NATIVE_OLLAMA[@]}"; do
    [[ "$name" == "$k" ]] && return 0
  done
  return 1
}

cmd_status() {
  require_docker
  echo "=== Pattern A (Docker Ollama) ==="
  if docker ps --format '{{.Names}}' | grep -qx 'aiome-ollama'; then
    docker exec aiome-ollama ollama list 2>/dev/null || echo "(ollama list failed)"
  else
    echo "aiome-ollama not running"
  fi
  if docker ps --format '{{.Names}}' | grep -qx 'aiome-api-server'; then
    docker inspect aiome-api-server --format '{{range .Config.Env}}{{println .}}{{end}}' \
      | grep -E '^OLLAMA_|^BG_LLM_|^LLM_MODEL=' || true
  fi
  echo
  echo "=== Pattern B (native Ollama) ==="
  require_ollama_cli
  ollama list 2>/dev/null || echo "(native ollama unreachable)"
  echo
  echo "=== Compose files ==="
  echo "  A: docker-compose.quickstart.yml"
  echo "  B: + docker-compose.quickstart.native-ollama.yml"
}

cmd_pattern_a() {
  require_docker
  if ! docker ps --format '{{.Names}}' | grep -qx 'aiome-ollama'; then
    echo "Starting aiome-ollama..."
    docker compose "${COMPOSE_A[@]}" up -d ollama
    sleep 3
  fi
  echo "Pulling ${DOCKER_OLLAMA_MODEL} into aiome-ollama..."
  docker exec aiome-ollama ollama pull "${DOCKER_OLLAMA_MODEL}"
  docker exec aiome-ollama ollama list
}

cmd_pattern_a_up() {
  require_docker
  docker compose "${COMPOSE_A[@]}" up -d --build
}

cmd_pattern_b_check() {
  require_ollama_cli
  if ! curl -sf "http://127.0.0.1:11434/api/tags" >/dev/null 2>&1; then
    echo "ERROR: native Ollama not reachable at http://127.0.0.1:11434"
    echo "Start Ollama.app or: ollama serve"
    exit 1
  fi
  if ollama list | tail -n +2 | awk '{print $1}' | grep -qx 'gemma4:26b'; then
    echo "OK: gemma4:26b present on native Ollama"
  else
    echo "WARN: gemma4:26b not found — run: ollama pull gemma4:26b"
    exit 1
  fi
}

cmd_pattern_b_up() {
  require_docker
  cmd_pattern_b_check
  echo "Stopping container Ollama (avoid port 11434 clash)..."
  docker stop aiome-ollama 2>/dev/null || true
  docker compose "${COMPOSE_B[@]}" up -d --build --force-recreate api-server management-console
  echo "Pattern B up. API uses host Ollama at host.docker.internal:11434"
}

list_removable_ollama() {
  require_ollama_cli
  ollama list | tail -n +2 | awk '{print $1}' | while read -r name; do
    [[ -z "$name" ]] && continue
    if is_kept_model "$name"; then
      echo "KEEP  $name"
    else
      echo "REMOVE $name"
    fi
  done
}

cmd_hygiene_dry_run() {
  echo "=== Native Ollama ==="
  list_removable_ollama || true
  echo
  echo "=== HuggingFace cache (Aiome embedding keeps ruri-v3-310m) ==="
  HF="${HOME}/.cache/huggingface/hub"
  if [[ -d "$HF" ]]; then
    for d in "$HF"/models--*; do
      [[ -d "$d" ]] || continue
      base=$(basename "$d")
      if [[ "$base" == *ruri-v3-310m* ]]; then
        echo "KEEP  $d"
      else
        echo "REMOVE $d ($(du -sh "$d" 2>/dev/null | cut -f1))"
      fi
    done
  fi
}

cmd_hygiene_apply() {
  require_ollama_cli
  echo "Removing unused native Ollama models (keeping: ${KEEP_NATIVE_OLLAMA[*]})..."
  ollama list | tail -n +2 | awk '{print $1}' | while read -r name; do
    [[ -z "$name" ]] && continue
    if is_kept_model "$name"; then
      echo "KEEP  $name"
    else
      echo "RM    $name"
      ollama rm "$name" || echo "WARN: failed to rm $name"
    fi
  done
  HF="${HOME}/.cache/huggingface/hub"
  if [[ -d "$HF" ]]; then
    echo "Pruning non-ruri HuggingFace hub caches..."
    for d in "$HF"/models--*; do
      [[ -d "$d" ]] || continue
      base=$(basename "$d")
      if [[ "$base" == *ruri-v3-310m* ]]; then
        echo "KEEP  $d"
      else
        echo "RM    $d"
        rm -rf "$d"
      fi
    done
  fi
  echo "Done. Run: ollama list"
}

main() {
  local cmd="${1:-status}"
  case "$cmd" in
    status) cmd_status ;;
    pattern-a) cmd_pattern_a ;;
    pattern-a-up) cmd_pattern_a_up ;;
    pattern-b-up) cmd_pattern_b_up ;;
    pattern-b-check) cmd_pattern_b_check ;;
    hygiene-dry-run) cmd_hygiene_dry_run ;;
    hygiene-apply) cmd_hygiene_apply ;;
    -h|--help|help) usage ;;
    *) echo "Unknown command: $cmd"; usage; exit 1 ;;
  esac
}

main "$@"
