#!/usr/bin/env bash
# Build the aiome/kani-verifier podman image locally

set -eo pipefail

echo "Building aiome/kani-verifier:latest..."
podman build -t aiome/kani-verifier:latest "$(dirname "$0")"
echo "Done."
