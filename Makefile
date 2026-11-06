# Run `make help` for the targets.

.DEFAULT_GOAL := help
.PHONY: help setup codegen check-generated wasm host build dev fmt fmt-check clippy test typecheck check clean

# The dated nightly for rustfmt and rustdoc JSON; see nightly-toolchain.
NIGHTLY_TOOLCHAIN ?= $(or $(SKETCH_NIGHTLY_TOOLCHAIN),$(shell head -n1 nightly-toolchain))
# A bun on PATH, otherwise the one the root devDependencies install.
BUN ?= $(shell command -v bun 2>/dev/null || echo $(CURDIR)/node_modules/.bin/bun)
BIN := $(CURDIR)/node_modules/.bin

HOST_PKG := js/packages/host
WEB_APP := apps/web
# Committed outputs of sketch-codegen.
GENERATED := \
	rust/crates/sketch-wallet-extension/src/wasm.rs \
	$(HOST_PKG)/src/generated/extensions.ts \
	$(HOST_PKG)/src/generated/extensions-adapter.ts
PRETTIER_FILES := "js/packages/*/src/**/*.ts" "apps/web/src/**/*.ts" "apps/web/*.ts" "scripts/**/*.mjs" "!**/generated/**"

help: ## Show this help.
	@awk 'BEGIN { FS = ":.*##"; printf "Usage: make <target>\n\nTargets:\n" } \
	      /^[a-zA-Z0-9_-]+:.*?##/ { printf "  %-16s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

setup: ## Install the JS dependencies and the pinned nightly.
	npm install
	rustup toolchain install "$(NIGHTLY_TOOLCHAIN)" --component rustfmt
	rustup target add wasm32-unknown-unknown

codegen: ## Regenerate the web side of the extension points from rustdoc JSON.
	./scripts/codegen.sh

check-generated: ## Fail when the committed generated files differ from a fresh codegen run.
	@tmp=$$(mktemp -d) && \
	for file in $(GENERATED); do mkdir -p "$$tmp/$$(dirname $$file)" && cp "$$file" "$$tmp/$$file"; done && \
	./scripts/codegen.sh > "$$tmp/codegen.log" 2>&1 || { cat "$$tmp/codegen.log"; exit 1; }; \
	for file in $(GENERATED); do \
		diff -u "$$tmp/$$file" "$$file" || { echo "$$file was stale; it is regenerated now, commit it."; exit 1; }; \
	done; \
	echo "Generated files are current."

wasm: ## Build the core and the mock wallet provider as two WASM binaries.
	node scripts/build-wasm.mjs

host: wasm ## Build @sketch/host (needs the core's WASM glue for its types).
	cd $(HOST_PKG) && $(BIN)/tsc -b

build: host ## Build everything, including the production web app.
	cd $(WEB_APP) && $(BIN)/tsc -p tsconfig.json && $(BIN)/vite build

dev: host ## Serve the web app with Vite on http://localhost:5181.
	cd $(WEB_APP) && $(BIN)/vite --port 5181

fmt: ## Format Rust (pinned nightly) and TypeScript.
	cargo +$(NIGHTLY_TOOLCHAIN) fmt --all
	$(BIN)/prettier --write $(PRETTIER_FILES)

fmt-check: ## Check formatting without changing files.
	cargo +$(NIGHTLY_TOOLCHAIN) fmt --all -- --check
	$(BIN)/prettier --check $(PRETTIER_FILES)

# Each WASM crate is linted on its own, as wasm-pack builds it: linting them
# together would unify the interface crate's `export` feature into the core.
clippy: ## Clippy with -D warnings: native workspace, then each WASM binary on wasm32.
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p sketch-core --target wasm32-unknown-unknown -- -D warnings
	cargo clippy -p sketch-wallet-mock --target wasm32-unknown-unknown -- -D warnings

typecheck: host ## Typecheck the TypeScript, tests included.
	cd $(HOST_PKG) && $(BIN)/tsc -p tsconfig.test.json
	cd $(WEB_APP) && $(BIN)/tsc -p tsconfig.json

test: host ## Rust tests, then the bun tests (the web app's run both WASM binaries).
	cargo test --workspace
	cd $(HOST_PKG) && $(BUN) test
	cd $(WEB_APP) && $(BUN) test

check: fmt-check clippy check-generated typecheck test ## Everything CI would run.

clean: ## Remove build outputs.
	cargo clean
	rm -rf $(HOST_PKG)/dist $(HOST_PKG)/wasm js/packages/wallet-mock/wasm $(WEB_APP)/dist
	find . -name '*.tsbuildinfo' -not -path './node_modules/*' -delete
