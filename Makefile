.PHONY: help build build-release check test test-watch run run-release clean format lint clippy clippy-fix \
	docker-build docker-up docker-down docker-logs docker-shell \
	db-migrate db-reset db-seed validate-env setup dev \
	docs docs-open docs-dev docs-build docs-preview install-deps check-deps

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
DOCKER_COMPOSE := docker-compose
RUST_LOG ?= mindia=info,tower_http=info
PORT ?= 3000

# Colors for output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
BLUE := \033[0;34m
NC := \033[0m # No Color

##@ Help

help: ## Display this help message
	@echo "$(BLUE)Mindia - Media Management Service$(NC)"
	@echo ""
	@echo "$(GREEN)Available commands:$(NC)"
	@awk 'BEGIN {FS = ":.*##"; printf ""} /^[a-zA-Z_-]+:.*?##/ { printf "  $(YELLOW)%-20s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(GREEN)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Building

build: ## Build the project in debug mode
	@echo "$(BLUE)Building project...$(NC)"
	$(CARGO) build -p mindia-api

build-release: ## Build the project in release mode
	@echo "$(BLUE)Building project in release mode...$(NC)"
	$(CARGO) build --release -p mindia-api

check: ## Check the project without building
	@echo "$(BLUE)Checking project...$(NC)"
	$(CARGO) check

##@ Testing

test: ## Run all tests
	@echo "$(BLUE)Running tests...$(NC)"
	$(CARGO) test

# Feature profiles: minimal | standard | full (see mindia-api/Cargo.toml)
test-minimal: ## Run mindia-api tests with minimal feature set
	$(CARGO) test -p mindia-api --no-default-features --features minimal

test-standard: ## Run mindia-api tests with standard feature set (no plugins)
	$(CARGO) test -p mindia-api --no-default-features --features standard

test-full: ## Run mindia-api tests with full feature set
	$(CARGO) test -p mindia-api --no-default-features --features full

test-watch: ## Run tests in watch mode
	@echo "$(BLUE)Running tests in watch mode...$(NC)"
	$(CARGO) watch -x test

test-single: ## Run a single test (usage: make test-single TEST=test_name)
	@echo "$(BLUE)Running test: $(TEST)$(NC)"
	$(CARGO) test $(TEST)

##@ Running

run: ## Run the application in debug mode
	@echo "$(BLUE)Running application...$(NC)"
	set -a; [ -f .env ] && . ./.env; set +a; \
	RUST_LOG=$(RUST_LOG) PORT=$(PORT) $(CARGO) run -p mindia-api

run-release: ## Run the application in release mode
	@echo "$(BLUE)Running application in release mode...$(NC)"
	set -a; [ -f .env ] && . ./.env; set +a; \
	RUST_LOG=$(RUST_LOG) PORT=$(PORT) $(CARGO) run --release -p mindia-api

##@ Code Quality

format: ## Format the codebase
	@echo "$(BLUE)Formatting code...$(NC)"
	$(CARGO) fmt

format-check: ## Check if code is formatted
	@echo "$(BLUE)Checking code formatting...$(NC)"
	$(CARGO) fmt -- --check

lint: ## Run clippy linter
	@echo "$(BLUE)Running clippy...$(NC)"
	$(CARGO) clippy -- -D warnings

clippy: lint ## Alias for lint

clippy-fix: ## Run clippy and apply automatic fixes
	@echo "$(BLUE)Running clippy with auto-fix...$(NC)"
	$(CARGO) clippy --fix --allow-dirty --allow-staged

##@ Database

db-migrate: ## Run database migrations
	@echo "$(BLUE)Running database migrations...$(NC)"
	$(CARGO) sqlx migrate run

db-migrate-revert: ## Revert the last database migration
	@echo "$(BLUE)Reverting last migration...$(NC)"
	$(CARGO) sqlx migrate revert

db-reset: ## Reset the database (WARNING: destroys all data)
	@echo "$(YELLOW)WARNING: This will destroy all database data!$(NC)"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "$(BLUE)Resetting database...$(NC)"; \
		$(CARGO) sqlx database drop -y || true; \
		$(CARGO) sqlx database create; \
		$(CARGO) sqlx migrate run; \
	fi

db-seed: ## Seed the database with sample data
	@echo "$(YELLOW)No seed script available$(NC)"
	@echo "Create scripts/seed.sh to implement database seeding"

##@ Docker

docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(NC)"
	$(DOCKER_COMPOSE) build

docker-up: ## Start Docker containers
	@echo "$(BLUE)Starting Docker containers...$(NC)"
	$(DOCKER_COMPOSE) up -d

docker-down: ## Stop Docker containers
	@echo "$(BLUE)Stopping Docker containers...$(NC)"
	$(DOCKER_COMPOSE) down

docker-logs: ## View Docker container logs
	@echo "$(BLUE)Viewing Docker logs...$(NC)"
	$(DOCKER_COMPOSE) logs -f

docker-shell: ## Open a shell in the app container
	@echo "$(BLUE)Opening shell in app container...$(NC)"
	$(DOCKER_COMPOSE) exec app /bin/sh || $(DOCKER_COMPOSE) exec app /bin/bash

docker-restart: ## Restart Docker containers
	@echo "$(BLUE)Restarting Docker containers...$(NC)"
	$(DOCKER_COMPOSE) restart

docker-clean: ## Remove Docker containers and volumes
	@echo "$(YELLOW)WARNING: This will remove containers and volumes!$(NC)"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "$(BLUE)Cleaning Docker resources...$(NC)"; \
		$(DOCKER_COMPOSE) down -v; \
	fi

##@ Development

setup: install-deps validate-env ## Set up development environment
	@echo "$(GREEN)✓ Development environment setup complete!$(NC)"

install-deps: ## Install development dependencies
	@echo "$(BLUE)Installing dependencies...$(NC)"
	@if ! command -v cargo-watch &> /dev/null; then \
		echo "$(YELLOW)Installing cargo-watch...$(NC)"; \
		$(CARGO) install cargo-watch; \
	fi
	@if ! command -v sqlx-cli &> /dev/null; then \
		echo "$(YELLOW)Installing sqlx-cli...$(NC)"; \
		$(CARGO) install sqlx-cli --no-default-features --features postgres; \
	fi
	@echo "$(GREEN)✓ Dependencies installed$(NC)"

check-deps: ## Check if required dependencies are installed
	@echo "$(BLUE)Checking dependencies...$(NC)"
	@missing=0; \
	for cmd in cargo rustc docker docker-compose; do \
		if ! command -v $$cmd &> /dev/null; then \
			echo "$(RED)✗ $$cmd is not installed$(NC)"; \
			missing=1; \
		else \
			echo "$(GREEN)✓ $$cmd is installed$(NC)"; \
		fi; \
	done; \
	if [ $$missing -eq 1 ]; then \
		echo "$(RED)Some dependencies are missing. Run 'make install-deps' to install optional tools.$(NC)"; \
		exit 1; \
	fi

validate-env: ## Validate environment variables
	@echo "$(BLUE)Validating environment...$(NC)"
	@if [ ! -f .env ]; then \
		echo "$(RED)✗ .env file not found$(NC)"; \
		echo "  Copy .env.example to .env and configure it"; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ .env file exists$(NC)"

dev: ## Start development environment (docker + app)
	@echo "$(BLUE)Starting development environment...$(NC)"
	@$(MAKE) docker-up
	@sleep 3
	@echo "$(GREEN)✓ Docker containers started$(NC)"
	@echo "$(BLUE)Starting application...$(NC)"
	@$(MAKE) run

##@ Cleanup

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean

clean-all: clean ## Clean everything including Docker volumes
	@echo "$(BLUE)Cleaning everything...$(NC)"
	@$(MAKE) docker-clean

##@ Documentation

docs: ## Generate Rust documentation
	@echo "$(BLUE)Generating documentation...$(NC)"
	$(CARGO) doc --no-deps

docs-open: ## Generate and open Rust documentation in browser
	@echo "$(BLUE)Generating documentation...$(NC)"
	$(CARGO) doc --no-deps --open

docs-dev: ## Start VitePress docs dev server
	npm run docs:dev

docs-build: ## Build VitePress docs for production
	npm run docs:build

docs-preview: ## Preview VitePress docs build
	npm run docs:preview

##@ Utilities

prepare-sqlx: ## Prepare sqlx offline mode
	@echo "$(BLUE)Preparing sqlx for offline mode...$(NC)"
	@./scripts/prepare-sqlx.sh

generate-encryption-key: ## Generate a new encryption key for ENCRYPTION_KEY
	@./scripts/generate-encryption-key.sh

bump-version: ## Bump version (usage: make bump-version VERSION=0.2.0)
	@if [ -z "$(VERSION)" ]; then \
		echo "$(RED)Error: VERSION not specified$(NC)"; \
		echo "Usage: make bump-version VERSION=0.2.0"; \
		exit 1; \
	fi
	@./scripts/bump-version.sh $(VERSION)

clear-media: ## Clear media and tasks from database (development only)
	@echo "$(YELLOW)⚠️  WARNING: This will delete all media and tasks!$(NC)"
	@./scripts/clear-media-tasks.sh

deploy: ## Deploy to Fly.io
	@echo "$(BLUE)Deploying to Fly.io...$(NC)"
	@./scripts/deploy-to-fly.sh

version: ## Show version information
	@echo "$(BLUE)Version Information:$(NC)"
	@echo "  Rust: $$(rustc --version)"
	@echo "  Cargo: $$($(CARGO) --version)"
	@echo "  Project: $$(grep '^version' Cargo.toml | cut -d'"' -f2)"

##@ CI/CD

ci: format-check lint test ## Run CI checks (format, lint, test)
	@echo "$(GREEN)✓ All CI checks passed!$(NC)"

ci-full: ci build-release ## Run full CI pipeline
	@echo "$(GREEN)✓ Full CI pipeline completed!$(NC)"

