# DEPRECATED for Quick Start / GHCR.
# This context (`apps/management-console`) cannot resolve:
#   "biome-engine": "file:../../libs/biome-engine/pkg"
# Use repository-root builds instead:
#   docker build -f docker/console.Dockerfile .
#   docker compose -f docker-compose.quickstart.yml build management-console

FROM alpine:3.20
RUN printf '%s\n' \
  "ERROR: Do not build this Dockerfile with context apps/management-console." \
  "Use: docker build -f docker/console.Dockerfile <repo-root>" \
  >&2 && exit 1
