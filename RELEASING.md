# Release Procedure (RELEASING.md)

This document outlines the standard operating procedure for releasing a new version of the Aiome and Project-Nurture infrastructure.

## 1. Pre-Flight Checks

Before initiating a release, ensure the following checks have passed:
- [ ] All P0 (Release Blocker) issues are resolved.
- [ ] The `main` branch is passing all CI checks (`cargo test`, `cargo audit`, `cargo deny`).
- [ ] No `#[cfg(debug_assertions)]` bypasses exist in production-critical paths.
- [ ] The `CHANGELOG.md` is updated and reflects the new version number under `[Unreleased]` → `[vX.Y.Z]`.
- [ ] `.env.example` is synchronized with any new or changed environment variables.
- [ ] `ARCHITECTURE.md` is regenerated and committed (`make arch`).

## 2. Security and Vulnerability Audit

- Run the dependency and keyword scan (nurture_auditor.py is deprecated):
  ```bash
  # grep-based scan for unsafe, unwrap, or panics in production code
  grep -rn "unsafe\|unwrap\|panic!" libs/ apps/ --include="*.rs" | grep -v test | head -20
  ```
- Run the cargo audit explicitly to ensure no new vulnerabilities:
  ```bash
  cargo audit
  ```
- Verify `audit.toml` is the sole SSOT for ignored advisories (no `--ignore` flags elsewhere).

## 3. Version Bumping

1. Update the version number in all relevant `Cargo.toml` files.
2. Update the `README.md` and `README_en.md` if any configuration options have changed.
3. If new environment variables were introduced, ensure `.env.example` is updated.
4. Move `[Unreleased]` entries in `CHANGELOG.md` to the new version section.

## 4. Building the Release

Create a clean release build to verify everything compiles with optimizations enabled:
```bash
RUSTFLAGS="-D warnings" cargo build --release --workspace
```

## 5. Tagging and Deployment

1. Create a **signed** tag:
   ```bash
   git tag -s vX.Y.Z -m "Release vX.Y.Z"
   ```
2. Push the tag to trigger the CD pipeline (if configured):
   ```bash
   git push origin vX.Y.Z
   ```
3. If deploying manually via Docker Compose:
   ```bash
   docker-compose -f docker-compose.production.yml pull
   docker-compose -f docker-compose.production.yml up -d
   ```
4. Wait for health checks to stabilize:
   ```bash
   # Verify all services are healthy (timeout: 60s)
   timeout 60 bash -c 'until curl -sf http://localhost:3015/api/v1/health; do sleep 2; done'
   ```

## 6. Post-Release Verification

- Check the production logs for any immediate panics or loop states.
- Verify the global error handlers (CWE-209 protection) are active by triggering a known 5xx error and confirming no internal stack traces or details are leaked.
- Verify the Webhook endpoints are responding correctly and returning generic 500 errors on infrastructure failures.
- Confirm `ALLOWED_ORIGINS` is explicitly set for samsara-hub (check for the warning log if not).

## 7. Rollback Plan

If the release fails in production:
1. Identify the failing commit or tag.
2. Revert the deployment to the previous stable tag:
   ```bash
   git checkout <previous_stable_tag>
   docker-compose -f docker-compose.production.yml up -d
   ```
3. Document the failure in a Post-Mortem under `docs/decisions/`.

