#!/usr/bin/env python3
"""Set DOMAIN_NAME on prod host .env (NT-1 one-shot).

⚠️ APPLIED / ARCHIVE ONLY (2026-07-12)
- 適用先は本番ホストの絶対パス `/app/aiome/.env` にハードコードされている。
- 再実行禁止。証跡として `scripts/oneoff/` に隔離。
"""
from pathlib import Path
import re

env_path = Path("/app/aiome/.env")
env = env_path.read_text()


def upsert(key: str, value: str) -> None:
    global env
    pat = re.compile(rf"^{re.escape(key)}=.*$", re.M)
    line = f"{key}={value}"
    if pat.search(env):
        env = pat.sub(line, env, count=1)
    else:
        if not env.endswith("\n"):
            env += "\n"
        env += line + "\n"


upsert("DOMAIN_NAME", "app.aiome.dev")

m = re.search(r"^ALLOWED_ORIGINS=(.*)$", env, re.M)
parts: list[str] = []
if m:
    cur = m.group(1).strip().strip('"').strip("'")
    parts = [p.strip() for p in cur.split(",") if p.strip()]

for w in (
    "https://app.aiome.dev",
    "http://localhost:3015",
    "http://localhost:1420",
):
    if w not in parts:
        parts.append(w)

upsert("ALLOWED_ORIGINS", ",".join(parts))
env_path.write_text(env)
print("OK DOMAIN_NAME=app.aiome.dev")
print("ALLOWED_ORIGINS hosts:")
for p in parts:
    print(" -", p.split("://")[-1] if "://" in p else p)
