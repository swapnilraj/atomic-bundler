# Makefile for Atomic Bundler

.PHONY: help build test fmt lint clean dev run docker-build docker-run install-deps check-deps

# Default target
help: ## Show this help message
	@echo "Atomic Bundler - Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Development commands
build: ## Build all crates
	cargo build --workspace

build-release: ## Build all crates in release mode
	cargo build --workspace --release

test: ## Run all tests
	cargo test --workspace

test-verbose: ## Run tests with verbose output
	cargo test --workspace -- --nocapture

fmt: ## Format code using rustfmt
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

lint: ## Run clippy lints
	cargo clippy --workspace --all-targets --all-features -- -D warnings

lint-fix: ## Fix clippy issues automatically where possible
	cargo clippy --workspace --all-targets --all-features --fix -- -D warnings

clean: ## Clean build artifacts
	cargo clean

check: ## Check code without building
	cargo check --workspace

# Development environment
dev: fmt lint test ## Run development checks (format, lint, test)

install-deps: ## Install development dependencies
	rustup component add rustfmt clippy
	cargo install cargo-watch cargo-audit cargo-outdated

watch: ## Watch for changes and rebuild
	cargo watch -x "build --workspace"

watch-test: ## Watch for changes and run tests
	cargo watch -x "test --workspace"

audit: ## Run security audit
	cargo audit

outdated: ## Check for outdated dependencies
	cargo outdated

# Running the application
run: ## Run the middleware binary
	cargo run --bin middleware

run-release: ## Run the middleware binary in release mode
	cargo run --bin middleware --release

# Configuration
config-example: ## Copy example config
	cp config.example.yaml config.yaml

# Docker commands
docker-build: ## Build Docker image
	docker build -t atomic-bundler:latest .

docker-run: ## Run Docker container
	docker run -p 8080:8080 -v $(PWD)/config.yaml:/app/config.yaml atomic-bundler:latest

docker-dev: ## Build and run Docker container for development
	docker build -t atomic-bundler:dev .
	docker run -p 8080:8080 -v $(PWD)/config.yaml:/app/config.yaml atomic-bundler:dev

# Database commands
db-setup: ## Set up database (SQLite)
	mkdir -p data
	sqlite3 data/atomic_bundler.db < sql/schema.sql

db-reset: ## Reset database
	rm -f data/atomic_bundler.db
	$(MAKE) db-setup

# CI/CD helpers
ci-build: build-release ## CI build step
ci-test: test ## CI test step  
ci-lint: fmt-check lint ## CI lint step
ci-audit: audit ## CI security audit step

# Documentation
docs: ## Generate documentation
	cargo doc --workspace --no-deps --open

docs-build: ## Build documentation
	cargo doc --workspace --no-deps

# Benchmarks (placeholder for future)
bench: ## Run benchmarks
	@echo "Benchmarks not yet implemented"

# Release preparation
pre-commit: fmt lint test audit ## Run all pre-commit checks

# Environment setup
.env: ## Create .env file from example
	cp .env.example .env
	@echo "Please edit .env file with your configuration"

# Check system dependencies
check-deps: ## Check if required system dependencies are installed
	@command -v cargo >/dev/null 2>&1 || { echo "cargo is required but not installed. Install Rust first."; exit 1; }
	@command -v sqlite3 >/dev/null 2>&1 || { echo "sqlite3 is required but not installed."; exit 1; }
	@command -v docker >/dev/null 2>&1 || { echo "docker is recommended but not installed."; }
	@echo "All required dependencies are available!"
