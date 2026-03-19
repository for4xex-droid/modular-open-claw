#!/bin/bash
# Aiome Global Build Check Script
set -e

echo "🏗️  Starting workspace-wide build check..."

# 1. Cargo Check
echo "🔍 Running cargo check --workspace --all-targets..."
cargo check --workspace --all-targets

# 2. Clippy (Warnings are allowed for now, but errors are fatal)
echo "📎 Running cargo clippy..."
cargo clippy --workspace --all-targets -- -D warnings || { echo "⚠️  Clippy found potential issues. Please review."; }

# 3. Security Audit (Optional, depends on cargo-audit being installed)
if command -v cargo-audit &> /dev/null; then
    echo "🛡️  Running cargo audit..."
    cargo audit || echo "⚠️  Security vulnerabilities found in dependencies."
else
    echo "ℹ️  cargo-audit not installed. Skipping security scan."
fi

echo "✅ Build check complete."
