#!/bin/bash
# Aiome Automated Integration Test Suite
# This script handles the lifecycle of the Key-Proxy for integration testing.

set -e

echo "🛡️  Starting Integration Test Lifecycle..."

# 1. Clean up any existing proxy on port 9999
if lsof -Pi :9999 -sTCP:LISTEN -t >/dev/null ; then
    echo "⚠️  Port 9999 is busy. Attempting to kill existing process..."
    lsof -ti:9999 | xargs kill -9
fi

# 2. Start Key-Proxy in background with dummy key
echo "🚀 Building Key-Proxy (Abyss Vault)..."
cargo build --bin key-proxy
echo "🚀 Starting Key-Proxy in background..."
KEY_PROXY_PORT=9999 GEMINI_API_KEY=test_dummy_key VAULT_SECRET=test_master_key_123_test_master_key_123 cargo run --bin key-proxy > /tmp/key_proxy_test.log 2>&1 &
PROXY_PID=$!

# 3. Wait for proxy to be ready
echo "⏳ Waiting for proxy to bind to port 9999..."
MAX_RETRIES=30
COUNT=0
while ! lsof -Pi :9999 -sTCP:LISTEN -t >/dev/null; do
    sleep 1
    COUNT=$((COUNT+1))
    if [ $COUNT -ge $MAX_RETRIES ]; then
        echo "❌ Timeout waiting for Key-Proxy to start. Check /tmp/key_proxy_test.log"
        kill $PROXY_PID || true
        exit 1
    fi
done
echo "✅ Key-Proxy is ready (PID: $PROXY_PID)."

# 4. Run Unit & Doc Tests (Workspace-wide)
echo "🧪 Running unit and documentation tests..."
cargo test --workspace --lib --bins --no-fail-fast

echo "🧪 Running OxiLean Kernel tests..."
cargo test --manifest-path vendor/oxilean-kernel/Cargo.toml

# 5. Run Integration Tests
echo "🧪 Running Zero-Trust integration tests..."
export KEY_PROXY_URL="http://127.0.0.1:9999"
if cargo test -p infrastructure --test zero_trust; then
    echo "✅ Integration tests PASSED."
    RESULT=0
else
    echo "❌ Integration tests FAILED."
    RESULT=1
fi

# 5. Cleanup
echo "🧹 Cleaning up Key-Proxy..."
kill $PROXY_PID || true
wait $PROXY_PID 2>/dev/null || true

exit $RESULT
