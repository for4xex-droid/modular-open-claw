#!/usr/bin/env python3
"""Surgical patch: prod api-server -> distroless without postgres/Caddy swap.

⚠️ APPLIED / ARCHIVE ONLY (2026-07-12, NT-1)
- 適用先は本番ホストの絶対パス `/app/aiome/docker-compose.production.yml` にハードコードされている。
- 再実行禁止。証跡として `scripts/oneoff/` に隔離。恒久イメージは docker/distroless.Dockerfile。
"""
from pathlib import Path
import re
import sys

p = Path("/app/aiome/docker-compose.production.yml")
text = p.read_text()
start = text.index("  api-server:\n")
rest = text[start + 1 :]
m = re.search(r"\n  [a-z0-9-]+:\n", rest)
if not m:
    sys.exit("cannot find end of api-server block")
end = start + 1 + m.start()
block = text[start:end]

if "dockerfile: docker/distroless.Dockerfile" in block:
    print("already patched")
    sys.exit(0)

if "dockerfile: docker/production.Dockerfile" not in block:
    sys.exit("unexpected dockerfile in api-server block")

block = block.replace(
    "dockerfile: docker/production.Dockerfile",
    "dockerfile: docker/distroless.Dockerfile",
    1,
)
block = block.replace('user: "1001:1001"', 'user: "65532:65532"', 1)

hc_old = (
    "    healthcheck:\n"
    '      test: ["CMD", "curl", "-f", "http://localhost:3015/api/health"]\n'
    "      interval: 30s\n"
    "      timeout: 10s\n"
    "      retries: 3\n"
)
hc_new = (
    "    # Distroless has no curl/shell — reachability via external check / Caddy.\n"
    "    healthcheck:\n"
    "      disable: true\n"
)
if hc_old not in block:
    sys.exit("healthcheck block not found")
block = block.replace(hc_old, hc_new, 1)

if "3015:3015" not in block:
    sys.exit("refusing to lose 3015 port mapping")
if "AIOME_DEV_MODE" not in block:
    sys.exit("AIOME_DEV_MODE missing — keep soft-launch until HTTPS")

p.write_text(text[:start] + block + text[end:])
print("OK: api-server -> distroless + uid 65532 + healthcheck.disable")
for line in block.splitlines():
    if any(
        k in line
        for k in (
            "dockerfile",
            "user:",
            "healthcheck",
            "disable",
            "ports",
            "3015",
            "AIOME_DEV_MODE",
        )
    ):
        print(line)
