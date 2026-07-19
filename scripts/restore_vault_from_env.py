#!/usr/bin/env python3
"""Restore key-proxy vault from host .env via PUT /api/v1/admin/secrets (no values printed).

Auth uses the container's existing VAULT_SECRET env (never passed on docker argv).
The Bearer header is written via printf into a temp file and passed with curl -H @file
so shell metacharacters in the secret do not break `sh -c` quoting.
Secret values are sent only on curl stdin (--data-binary @-).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

KEYS = [
    "STRIPE_API_KEY",
    "STRIPE_WEBHOOK_SECRET",
    "JWT_PRIVATE_KEY_B64",
    "API_SERVER_SECRET",
    "FEDERATION_SECRET",
    "FAL_KEY",
    "GEMINI_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "TTS_OPENAI_API_KEY",
    "SEARCH_API_KEY",
    "TIMESFM_AUTH_TOKEN",
    "TREMENDOUS_API_KEY",
    "X_BEARER_TOKEN",
    "DISCORD_TOKEN",
    "TELEGRAM_TOKEN",
    "POLAR_API_KEY",
    "POLAR_WEBHOOK_SECRET",
]

# Build Authorization header from container env without embedding it in shell words.
# mktemp + EXIT trap avoids leaving Bearer material in /tmp on failure.
_AUTH_HDR = (
    'h=$(mktemp) || exit 1; '
    'trap \'rm -f "$h"\' EXIT; '
    'printf \'%s\' "Authorization: Bearer ${VAULT_SECRET}" > "$h"; '
)
_CURL_PUT = (
    _AUTH_HDR
    + 'body=/tmp/vault_store_body; '
    'code=$(curl -sS -o "$body" -w "%{http_code}" '
    "-X PUT http://127.0.0.1:9999/api/v1/admin/secrets "
    '-H @"$h" '
    '-H "Content-Type: application/json" '
    "--data-binary @-); "
    # Never leave response bodies (may include error context) on disk.
    'rm -f "$body"; '
    'printf %s "$code"'
)
_CURL_STATUS = (
    _AUTH_HDR + 'curl -sS http://127.0.0.1:9999/api/v1/admin/status -H @"$h"'
)


def load_env(path: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in path.read_text().splitlines():
        s = line.strip()
        if not s or s.startswith("#") or "=" not in s:
            continue
        k, v = s.split("=", 1)
        out[k.strip()] = v.strip().strip("\"'")
    return out


def main() -> int:
    root = Path(os.environ.get("AIOME_ROOT", "/app/aiome"))
    env_path = root / ".env"
    if not env_path.is_file():
        print(f"ERROR: missing {env_path}", file=sys.stderr)
        return 1

    env = load_env(env_path)
    # Host .env must have values to restore; auth itself uses container VAULT_SECRET.
    if not env.get("VAULT_SECRET"):
        print(
            "ERROR: VAULT_SECRET missing in host .env "
            "(required as presence check; auth uses container env)",
            file=sys.stderr,
        )
        return 1

    pre = subprocess.run(
        [
            "docker",
            "exec",
            "aiome-key-proxy-1",
            "sh",
            "-c",
            'test -n "${VAULT_SECRET:-}"',
        ],
        capture_output=True,
    )
    if pre.returncode != 0:
        print(
            "ERROR: VAULT_SECRET empty or missing inside key-proxy container",
            file=sys.stderr,
        )
        return 2

    present = [k for k in KEYS if env.get(k)]
    print(f"will_store_count={len(present)}")
    print("will_store=", ",".join(present))
    if "STRIPE_API_KEY" not in present:
        print(
            "ERROR: STRIPE_API_KEY not in .env — cannot auto-restore commerce",
            file=sys.stderr,
        )
        return 3

    ok = fail = 0
    for k in present:
        body = json.dumps({"key": k, "value": env[k]})
        proc = subprocess.run(
            ["docker", "exec", "-i", "aiome-key-proxy-1", "sh", "-c", _CURL_PUT],
            input=body.encode(),
            capture_output=True,
        )
        code = proc.stdout.decode().strip()
        if code in ("200", "201", "204"):
            print(f"OK {k} http={code}")
            ok += 1
        else:
            err = proc.stderr.decode()[:120]
            print(f"FAIL {k} http={code!r} stderr={err!r}")
            fail += 1

    status = subprocess.run(
        ["docker", "exec", "aiome-key-proxy-1", "sh", "-c", _CURL_STATUS],
        capture_output=True,
        text=True,
    )
    if status.returncode == 0 and status.stdout:
        try:
            data = json.loads(status.stdout)
            print(
                f"vault_status configured={data.get('configured')}/{data.get('total')}"
            )
            set_keys = [s["key"] for s in data.get("secrets", []) if s.get("is_set")]
            print("vault_set_keys=", ",".join(set_keys))
            print("stripe_set=", "STRIPE_API_KEY" in set_keys)
        except json.JSONDecodeError:
            print("vault_status_raw_len", len(status.stdout))

    print(f"done ok={ok} fail={fail}")
    return 0 if fail == 0 and ok > 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
