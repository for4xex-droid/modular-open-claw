.PHONY: all preflight check fmt clippy test audit pattern docs arch

all: preflight

preflight: fmt clippy audit pattern test docs arch
	@echo "🚀 Preflight Check: All Systems GREEN. Ready for Takeoff."

check:
	@echo "🔍 Checking compilation..."
	cargo check --workspace --all-targets

fmt:
	@echo "🎨 Checking formatting..."
	cargo fmt --all -- --check

clippy:
	@echo "📎 Running strict clippy..."
	cargo clippy --workspace --all-targets -- -D warnings

test:
	@echo "🧪 Running full test suite..."
	./scripts/test_all.sh

audit:
	@echo "🔒 Checking dependencies for vulnerabilities..."
	cargo audit

pattern:
	@echo "🛑 Enforcing Anti-Patterns (Strict Mode)..."
	./scripts/pattern-enforcer.sh

docs:
	@echo "📚 Verifying documentation synchronization..."
	./scripts/docs-sync-check.sh

arch:
	@echo "🏛️ Verifying architecture documentation..."
	python3 scripts/generate_architecture.py
	git diff --exit-code ARCHITECTURE.md || (echo "🚨 ARCHITECTURE.md is out of sync. Please commit the newly generated changes." && exit 1)
