#!/usr/bin/env python3
"""Add Caddy + DOMAIN_NAME wiring to prod compose without postgres swap.

⚠️ APPLIED / ARCHIVE ONLY (2026-07-12, NT-1 本番ホスト向け one-shot)
- 適用先は本番ホストの絶対パス `/app/aiome/...` にハードコードされている。
- 再実行禁止。証跡として `scripts/oneoff/` に隔離。恒久手順は HUMAN_PUBLIC_BETA_RUNBOOK.md を正とする。
"""
from pathlib import Path
import re
import sys

compose = Path("/app/aiome/docker-compose.production.yml")
text = compose.read_text()

CADDY = '''
  # Caddy Reverse Proxy (TLS for DOMAIN_NAME)
  caddy:
    image: caddy:2-alpine
    environment:
      - DOMAIN_NAME=${DOMAIN_NAME:-localhost}
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./docker/caddy/Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - api-server
      - samsara-hub
    restart: always

'''

if re.search(r"^  caddy:\s*$", text, re.M):
    print("caddy service already present")
else:
    # insert after "services:" line
    if not text.startswith("services:"):
        # find services:
        m = re.search(r"^services:\s*$", text, re.M)
        if not m:
            sys.exit("services: not found")
        idx = m.end()
        text = text[:idx] + "\n" + CADDY + text[idx:]
    else:
        text = text.replace("services:\n", "services:\n" + CADDY, 1)
    print("inserted caddy service")

# volumes section
if "caddy_data:" not in text:
    if re.search(r"^volumes:\s*$", text, re.M):
        text = re.sub(
            r"^volumes:\s*$",
            "volumes:\n  caddy_data:\n  caddy_config:",
            text,
            count=1,
            flags=re.M,
        )
        print("added caddy volumes under existing volumes:")
    else:
        text += "\nvolumes:\n  caddy_data:\n  caddy_config:\n"
        print("appended volumes section")

compose.write_text(text)

# .env updates (no secret dump)
env_path = Path("/app/aiome/.env")
env = env_path.read_text()
changed = False

def upsert(key: str, value: str) -> None:
    global env, changed
    pat = re.compile(rf"^{re.escape(key)}=.*$", re.M)
    line = f"{key}={value}"
    if pat.search(env):
        env = pat.sub(line, env, count=1)
    else:
        env += ("\n" if not env.endswith("\n") else "") + line + "\n"
    changed = True

upsert("DOMAIN_NAME", "aiome.dev")
# Keep localhost for tunnel soft-launch; add HTTPS origin
if "https://aiome.dev" not in env:
    m = re.search(r"^ALLOWED_ORIGINS=(.*)$", env, re.M)
    if m:
        cur = m.group(1).strip().strip('"').strip("'")
        parts = [p for p in cur.split(",") if p]
        if "https://aiome.dev" not in parts:
            parts.append("https://aiome.dev")
        if "http://localhost:3015" not in parts:
            parts.append("http://localhost:3015")
        upsert("ALLOWED_ORIGINS", ",".join(parts))
    else:
        upsert(
            "ALLOWED_ORIGINS",
            "https://aiome.dev,http://localhost:3015,http://localhost:1420",
        )

if changed:
    env_path.write_text(env)
    print("updated .env DOMAIN_NAME / ALLOWED_ORIGINS")
else:
    print(".env unchanged")

print("OK")
