.PHONY: help build check test test-integration test-all fmt clippy audit deny docs wheel ci clean

help:  ## Show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build:  ## Build all crates (dev profile)
	cargo build --workspace

check: fmt clippy  ## Fast pre-commit check (fmt + clippy)

fmt:  ## Verify rustfmt formatting
	cargo fmt --all -- --check

clippy:  ## Run clippy with all warnings as errors
	cargo clippy --workspace --all-targets -- -D warnings

test:  ## Run unit tests (no network)
	cargo test --workspace

test-integration: build  ## Run integration tests (calls real APIs, needs network)
	cargo test -p facto-run -- --ignored

test-all: build  ## Run all tests including network-dependent ones
	cargo test --workspace -- --include-ignored

audit:  ## Scan dependencies for known vulnerabilities (cargo-audit required)
	cargo audit

deny:  ## Run cargo-deny checks (licenses, advisories, bans, sources)
	cargo deny check

docs:  ## Build rustdocs with broken-link detection
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

wheel:  ## Build PyPI wheel locally via maturin
	cd facto-run && uvx maturin build --release --out dist

ci: fmt clippy test docs audit deny  ## Reproduce the full CI pipeline locally

clean:  ## Remove build artifacts
	cargo clean
	rm -rf facto-run/dist
