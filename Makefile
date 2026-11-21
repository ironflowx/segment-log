# Shortcuts for the commands in CLAUDE.md and CI. `make check` is what CI runs on each platform;
# `make` lists the targets: it prints each target's `## ` comment.
# foundation's Makefile has the same shape and shared targets; keep the two alike.
.DEFAULT_GOAL := help
.PHONY: help check fmt lint test doc clean release release-dry-run build musl

help: ## Show the available targets and how to use them
	@echo 'Usage: make [target] [VERSION=x.y.z]'
	@echo
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z_-]+:.*## / {printf "  %-16s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

check: lint test ## Run lint and test, as CI does

fmt: ## Format all crates in place
	cargo fmt --all

lint: ## Check formatting and run clippy with warnings as errors
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings

test: ## Run the workspace tests
	cargo test --workspace

doc: ## Build the API docs and open them
	cargo doc --workspace --no-deps --open

clean: ## Remove build output
	cargo clean

release: ## Cut a release: make release VERSION=0.1.0
	scripts/release.sh $(VERSION)

release-dry-run: ## Rehearse a release without publishing: make release-dry-run VERSION=0.1.0
	scripts/release.sh $(VERSION) --dry-run

build: ## Build release binaries
	cargo build --release

musl: ## Build the guest server binary for Linux x86_64 (musl)
	rustup target add x86_64-unknown-linux-musl
	cargo build --release -p segment-log-server --target x86_64-unknown-linux-musl
