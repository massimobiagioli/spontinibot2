# Spontini Bot 2 — Makefile
# Container-first operator entry point.
# Every target runs inside the containers. The host needs only Docker + Compose + make.
# Spec: docs/STACK.md §7.3
#
# Usage:
#   make              # -> help (default)
#   make up           # start the full stack
#   make verify       # full pre-completion gate
#   make shell SERVICE=ingest   # shell inside a different service

.DEFAULT_GOAL := help

# --- Config ---------------------------------------------------------------
SERVICE          ?= backend
COMPOSE          := docker compose
WORKSPACE_CRATES := -p backend -p ingest-core -p ingest -p ingest-cli -p kb-store

# --- Help (self-documenting) ---------------------------------------------
.PHONY: help
## help: show this help
help:
	@awk 'BEGIN { \
		printf "Spontini Bot 2 — operator targets\n\n"; \
		printf "Usage:\n  make \033[36m<target>\033[0m [SERVICE=<svc>]\n\nTargets:\n"; \
	} \
	/^## / { \
		sub(/^## /, ""); \
		split($$0, a, ":"); \
		name = a[1]; \
		sub(/^[^:]*:/, "", $$0); \
		sub(/^ /, "", $$0); \
		printf "  \033[36m%-16s\033[0m %s\n", name, $$0; \
	}' $(MAKEFILE_LIST)

# --- Lifecycle ------------------------------------------------------------
.PHONY: build
## build: build all container images
build:
	$(COMPOSE) build

.PHONY: up
## up: start the full stack in the background
up:
	$(COMPOSE) up -d

.PHONY: down
## down: stop the stack (preserves volumes)
down:
	$(COMPOSE) down

.PHONY: logs
## logs: tail logs from every service
logs:
	$(COMPOSE) logs -f

.PHONY: ps
## ps: show running services and ports
ps:
	$(COMPOSE) ps

.PHONY: shell
## shell: open a bash shell inside a service (SERVICE=ingest to switch)
shell:
	$(COMPOSE) run --rm $(SERVICE) bash

# --- Testing --------------------------------------------------------------
.PHONY: test
## test: run every test suite, in containers
test: test-backend test-frontend

.PHONY: test-backend
## test-backend: cargo test (workspace) inside the backend container
test-backend:
	$(COMPOSE) run --rm backend cargo test $(WORKSPACE_CRATES)

.PHONY: test-frontend
## test-frontend: npm run test inside the frontend and admin-ui containers
test-frontend:
	$(COMPOSE) run --rm frontend npm run test
	$(COMPOSE) run --rm admin-ui npm run test

.PHONY: bdd
## bdd: run Gherkin scenarios end-to-end against the running stack
bdd:
	$(COMPOSE) run --rm backend cargo test --test bdd $(WORKSPACE_CRATES)

# --- Lint / format / check ------------------------------------------------
.PHONY: lint
## lint: clippy (Rust) + eslint (frontend) inside containers
lint:
	$(COMPOSE) run --rm backend cargo clippy $(WORKSPACE_CRATES) -- -D warnings
	$(COMPOSE) run --rm frontend npm run lint
	$(COMPOSE) run --rm admin-ui npm run lint

.PHONY: fmt
## fmt: format the whole codebase (write mode)
fmt:
	$(COMPOSE) run --rm backend cargo fmt
	$(COMPOSE) run --rm frontend npm run format
	$(COMPOSE) run --rm admin-ui npm run format

.PHONY: fmt-check
## fmt-check: verify formatting without writing
fmt-check:
	$(COMPOSE) run --rm backend cargo fmt --check
	$(COMPOSE) run --rm frontend npm run format -- --check
	$(COMPOSE) run --rm admin-ui npm run format -- --check

.PHONY: check
## check: cargo check (workspace) — fast compile gate
check:
	$(COMPOSE) run --rm backend cargo check $(WORKSPACE_CRATES)

# --- Coverage gate --------------------------------------------------------
.PHONY: coverage
## coverage: run tarpaulin inside the backend container, enforce 100% line / 80% branch
coverage:
	$(COMPOSE) run --rm backend cargo tarpaulin $(WORKSPACE_CRATES) \
		--workspace --exclude-files '**/main.rs' '**/tests/**' \
		--line 100 --branch 80 --out Html --out Lcov

# --- Data / ingest --------------------------------------------------------
.PHONY: migrate
## migrate: run libSQL migrations inside the backend container
migrate:
	$(COMPOSE) run --rm backend cargo run -p backend -- migrate

.PHONY: ingest-run
## ingest-run: trigger an immediate ingest run via /admin/api/ingest/run
ingest-run:
	@curl -sS -X POST http://localhost:8080/admin/api/ingest/run

# --- Docker config --------------------------------------------------------
.PHONY: compose-config
## compose-config: validate the docker-compose.yml
compose-config:
	$(COMPOSE) config -q

# --- Pre-completion gate --------------------------------------------------
.PHONY: verify
## verify: pre-completion gate (build + test + lint + fmt-check + coverage + compose config)
verify: build test lint fmt-check coverage compose-config
	@echo "verify: all gates passed"

# --- Cleanup --------------------------------------------------------------
.PHONY: clean
## clean: remove build artifacts inside containers (set CLEAN_VOLUMES=1 to also drop volumes — destructive)
clean:
	$(COMPOSE) run --rm backend cargo clean
	$(COMPOSE) run --rm frontend rm -rf dist node_modules/.vite
	$(COMPOSE) run --rm admin-ui rm -rf dist node_modules/.vite
	@if [ "$(CLEAN_VOLUMES)" = "1" ]; then \
		echo "CLEAN_VOLUMES=1 — dropping Docker volumes (destructive)"; \
		$(COMPOSE) down -v; \
	else \
		echo "Volumes preserved. To drop them: make clean CLEAN_VOLUMES=1"; \
	fi
