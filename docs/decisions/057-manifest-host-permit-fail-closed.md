# ADR-057: PermissionManifest Host Permit (Fail-Closed Empty Domains)

**Status**: Accepted  
**Date**: 2026-07-31  
**Deciders**: motivationstudio  
**Related**: OP-096 / OP-097 / OP-098, [`autonomous_egress_defense_plan.md`](../roadmaps/autonomous_egress_defense_plan.md) v1.3, [`manifest_host_drift_plan.md`](../roadmaps/manifest_host_drift_plan.md) v1.3+S0, Code Mode `aiome.fetch`, BastionGuard `check_network`

## Context

`BastionGuard::check_network` treated an empty `allowed_domains` list as “skip domain filter → allow” while Code Mode `aiome.fetch` already denied empty lists. Matching used `url.contains(domain)`, which is weaker and inconsistent with Code Mode’s host / suffix / `*` rules. The sole production caller of `check_network` passes a **bare domain** from the Manifest list (WASM `with_allowed_host` registration), not always a full URL.

## Decision

1. **Canonical algorithm**: `aiome_contracts::security::host_permitted(host, allowed_domains)` — empty list → deny; trim host/entries; reject control chars / internal whitespace on host; lowercase normalize host and allow entries; strip a single trailing `.` (FQDN); ignore empty or leading/trailing-`.` junk entries; `*` → allow; exact host after normalization; subdomain suffix only when the allow entry **contains `.`** (blocks bare-TLD suffix like `com` → `evil.com`). Base semantics from Code Mode; suffix/junk/normalize harden is intentional.
2. **Bastion** uses `host_permitted` after resolving `target` to a host (`network_target_host`: HTTP(S)/WS(S) URL host, else bare trimmed string). Empty host → deny. `system_internal` bypass unchanged and must not be expanded casually.
3. **Code Mode** calls the same function (no duplicated loop).
4. **WASM `with_allowed_host`**: skip `*` **after trim** (and empty entries) via `wasm_hosts_for_extism` so `"* "` / `" * "` cannot bypass the skip; register only trimmed enumerated hosts that pass Bastion `check_network`.
5. **Do not** unify with `commerce_helpers` redirect validation, tool_call_router SSRF, workflow `assert_resolved_url_safe`, or seatbelt domain filtering. **Exception (OP-097)**: `constraint_checker` DomainBlocked may **delegate** to `host_permitted` (no local reimplementation). Do not invent host checks on `execute_wasm_skill` (live path already covered by Bastion/WASM).
6. **Do not** rename `RuntimeJail::check_network`.
7. **Do not** rewrite `ConstitutionalValidator` domain substring axioms (`libs/core/src/security.rs`) as `host_permitted` — different contract (manifest entry denial, not host permit).

## Consequences

- Bastion empty-domain Fail-Open is closed (Breaking for any future URL callers; WASM empty-list registration behavior unchanged — zero hosts registered either way).
- Single policy surface for Manifest host checks reduces drift.
- Residual: subprocess seatbelt still uses boolean `allow_network` only — **accepted** after OP-098 Spike（[`manifest_host_drift_plan.md`](../roadmaps/manifest_host_drift_plan.md) §8）。Do not map Manifest `allowed_domains` into seatbelt (DNS/hostname filtering is unreliable / false comfort).
- **Follow-up**: OP-097 ✅（`constraint_checker` → `host_permitted`）。OP-098 ✅ Residual（no implementation OP）。
- **OP-097 clarification**: `constraint_checker` の `DomainBlocked` は旧来の exact-host 一致より広い。`host_permitted` 委任により subdomain suffix（allow entry に `.` を含む場合）および `*` ワイルドカードが有効。旧 exact-only を前提にした呼び出し側は挙動差に注意すること。
- **Normalization (OP-099 review fix)**: host / allow entry は lowercase 正規化、FQDN 末尾ドット除去、制御文字・内部空白は deny。Case-sensitive exact match は廃止。

## Out of scope

- Host firewall (LuLu / Little Snitch; OP-095 H1 optional)
- Fitness regex gates on `allow_network` + empty domains
- Commerce / auth / Vault / Tauri shell changes
- Unifying commerce redirect / router SSRF / workflow URL checks with `host_permitted`
- Renaming `RuntimeJail::check_network`
