#!/usr/bin/env python3
"""Host hotfix: drop Postgres AIOME_DB_PATH/depends for api-server; keep A2A_AUTH + curl healthcheck."""
from __future__ import annotations

import re
import sys
from pathlib import Path


def main() -> int:
    p = Path(sys.argv[1] if len(sys.argv) > 1 else "docker-compose.production.yml")
    t = p.read_text()

    if "- A2A_AUTH_TOKEN=${A2A_AUTH_TOKEN}\n" not in t.split("api-server:")[1].split("nurture-api:")[
        0
    ]:
        t = t.replace(
            "      - A2A_NODE_TOKEN=${A2A_AUTH_TOKEN}\n",
            "      - A2A_AUTH_TOKEN=${A2A_AUTH_TOKEN}\n"
            "      - A2A_NODE_TOKEN=${A2A_AUTH_TOKEN}\n",
            1,
        )
        print("ADDED_A2A_AUTH")

    t2, n = re.subn(r"\n      - AIOME_DB_PATH=postgres://[^\n]+\n", "\n", t, count=1)
    print(f"REMOVED_DB_PATH={n}")
    t = t2

    old_hc = 'test: ["CMD", "nc", "-z", "localhost", "9999"]'
    new_hc = 'test: ["CMD", "curl", "-f", "http://127.0.0.1:9999/api/v1/health"]'
    if old_hc in t:
        t = t.replace(old_hc, new_hc)
        print("FIXED_HEALTHCHECK")
    elif "127.0.0.1:9999/api/v1/health" in t:
        print("HEALTHCHECK_ALREADY_CURL")

    idx = t.find("  api-server:")
    idx2 = t.find("\n  nurture-api:")
    if idx >= 0 and idx2 > idx:
        block = t[idx:idx2]
        block2, n2 = re.subn(
            r"\n      postgres:\n        condition: service_healthy", "", block, count=1
        )
        print(f"REMOVED_API_PG_DEPENDS={n2}")
        t = t[:idx] + block2 + t[idx2:]

    p.write_text(t)
    print("WROTE", p)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
