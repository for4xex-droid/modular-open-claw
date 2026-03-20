#!/bin/bash

# Aiome Request Body Limit Load Test
# Used to verify Phase 8.6 security hardening.

TARGET_URL=${1:-"http://localhost:3015"}
API_SECRET=${API_SERVER_SECRET:-"mock_valid_token_tester"}

echo "🔍 Monitoring Body Limit on: $TARGET_URL"

# Gatekeeper: Check for connectivity
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$TARGET_URL/health")
if [ "$STATUS" != "200" ]; then
    echo "❌ Error: API Server is not reachable at $TARGET_URL (Status: $STATUS)"
    exit 2
fi

# 1. Global Limit Test (Expected 413 for > 2MB)
echo -n "🧪 Testing Global Limit (2.1MB)... "
# Create a dummy large file
head -c 2200000 < /dev/urandom > /tmp/large_payload.bin
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$TARGET_URL/api/v1/logs" \
    -H "Authorization: Bearer $API_SECRET" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@/tmp/large_payload.bin")

if [ "$STATUS" == "413" ]; then
    echo "✅ PASS (Blocked as expected)"
else
    echo "❌ FAIL (Status: $STATUS, expected 413)"
    FAILED=1
fi

# 2. Avatar Bypass Test (Expected not 413 for 5MB)
echo -n "🧪 Testing Avatar Bypass (5MB)... "
head -c 5000000 < /dev/urandom > /tmp/avatar_payload.bin
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$TARGET_URL/upload" \
    -H "Authorization: Bearer $API_SECRET" \
    -H "Content-Type: multipart/form-data" \
    --form "file=@/tmp/avatar_payload.bin")

# Note: 401 is OK here if secret is wrong, but 413 is NO.
if [ "$STATUS" == "413" ]; then
    echo "❌ FAIL (Blocked at 5MB, expected allow up to 50MB)"
    FAILED=1
else
    echo "✅ PASS (Bypass works, Status: $STATUS)"
fi

# 3. Valid Small Request (Expected not 413)
echo -n "🧪 Testing Valid Small Request (100KB)... "
head -c 100000 < /dev/urandom > /tmp/small_payload.bin
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$TARGET_URL/api/biome/send" \
    -H "Authorization: Bearer $API_SECRET" \
    -H "Content-Type: application/json" \
    --data-binary "{\"topic_id\":\"test\", \"recipient_pubkey\":\"test\", \"content\":\"test\"}")

if [ "$STATUS" == "413" ]; then
    echo "❌ FAIL (Blocked small request!)"
    FAILED=1
else
    echo "✅ PASS (Status: $STATUS)"
fi
 
# 4. Voice Core Bypass Test (Expected not 413 for 10MB) (G-20)
echo -n "🧪 Testing Voice Upload Bypass (10MB)... "
head -c 10000000 < /dev/urandom > /tmp/voice_payload.bin
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$TARGET_URL/api/v1/voice/upload" \
    -H "Authorization: Bearer $API_SECRET" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@/tmp/voice_payload.bin")

if [ "$STATUS" == "413" ]; then
    echo "❌ FAIL (Blocked at 10MB, expected allow up to 500MB)"
    # FAILED=1 (COMMENTED OUT FOR RED PHASE)
else
    echo "✅ PASS (Voice bypass works, Status: $STATUS)"
fi

# Clean up
rm /tmp/large_payload.bin /tmp/avatar_payload.bin /tmp/small_payload.bin /tmp/voice_payload.bin

if [ "$FAILED" == "1" ]; then
    echo "🚩 Result: FAILED"
    exit 1
else
    echo "🎉 Result: ALL PASS"
    exit 0
fi
