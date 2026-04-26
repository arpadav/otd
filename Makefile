.DEFAULT_GOAL := build
.PHONY: frontend backend build serve bundle docker clean

# --------------------------------------------------
# mode: debug (default) or release
# --------------------------------------------------
MODE ?= debug

ifeq ($(MODE),release)
	CARGO_FLAGS := --release
else
	CARGO_FLAGS :=
endif

# --------------------------------------------------
# frontend: build sveltekit spa
# --------------------------------------------------
frontend:
	cd frontend && npm i --no-audit --no-fund && npm run build

# --------------------------------------------------
# backend: build rust binary (depends on frontend)
# --------------------------------------------------
backend: frontend
	cargo build $(CARGO_FLAGS)

# --------------------------------------------------
# builds both frontend and backend
# --------------------------------------------------
build: frontend backend

# --------------------------------------------------
# serve: dev mode - run vite and cargo concurrently
# --------------------------------------------------
serve:
	@echo "Starting dev servers..."
	@bash -c '\
	trap "pkill -P $$$$ 2>/dev/null; kill -- -$$$$ 2>/dev/null" INT TERM EXIT; \
	cd frontend && npm run dev & \
	cargo run -- --admin-host 0.0.0.0 & \
	wait'

# --------------------------------------------------
# bundle: create distributable tarball
# --------------------------------------------------
bundle:
	bash scripts/bundle.sh

# --------------------------------------------------
# docker: build docker image
# --------------------------------------------------
docker:
	bash scripts/docker-build.sh

# --------------------------------------------------
# clean: remove build artifacts
# --------------------------------------------------
clean:
	cargo clean
	rm -rf frontend/build frontend/.svelte-kit frontend/node_modules

reserve: clean build serve
