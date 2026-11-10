# Run `make help` for the targets.

.DEFAULT_GOAL := help
.PHONY: help setup codegen wasm packages build dev preview fmt fmt-check clippy typecheck test check swift kotlin native clean

# The dated nightly for rustfmt and rustdoc JSON, from nightly-toolchain unless
# set; exported, so scripts/codegen.sh uses the same one.
NIGHTLY_TOOLCHAIN ?= $(shell head -n1 nightly-toolchain)
export NIGHTLY_TOOLCHAIN
# A bun on PATH, otherwise the one the root devDependencies install.
BUN ?= $(shell command -v bun 2>/dev/null || echo $(CURDIR)/node_modules/.bin/bun)
BIN := $(CURDIR)/node_modules/.bin
CLIENT_PKG := js/packages/protocol-client
EXTENSION_PKG := js/packages/host-extension
HOST_PKG := js/packages/host
HOST_APP := apps/host-app-web
PRODUCT_APP := apps/product
PRETTIER_FILES := "js/packages/*/src/**/*.ts" "apps/*/src/**/*.ts" "apps/*/*.ts" "scripts/**/*.mjs" "!**/generated/**"

help: ## Show this help.
	@awk 'BEGIN { FS = ":.*##"; printf "Usage: make <target>\n\nTargets:\n" } \
	      /^[a-zA-Z0-9_-]+:.*?##/ { printf "  %-12s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

setup: ## Install the JS dependencies, the pinned nightly and the wasm32 target.
	npm install
	rustup toolchain install "$(NIGHTLY_TOOLCHAIN)" --component rustfmt
	rustup target add wasm32-unknown-unknown

codegen: ## Generate the product client and the extension point interfaces (TypeScript only).
	./scripts/codegen.sh

wasm: ## Build the web host app's WASM binary: the core and the providers it links.
	node scripts/build-wasm.mjs

packages: codegen ## Build @sketch/protocol-client, @sketch/host-extension and @sketch/host.
	cd $(CLIENT_PKG) && $(BIN)/tsc -b
	cd $(EXTENSION_PKG) && $(BIN)/tsc -b
	cd $(HOST_PKG) && $(BIN)/tsc -b

build: packages wasm ## Build everything, including both web apps for production.
	cd $(PRODUCT_APP) && $(BIN)/tsc -p tsconfig.json && $(BIN)/vite build
	cd $(HOST_APP) && $(BIN)/tsc -p tsconfig.json && $(BIN)/vite build

# The host app frames the product from the product app's own origin, so both
# servers run: the host on http://localhost:5181, the product on :5182.
dev: packages wasm ## Serve the web host app (:5181) and the product (:5182) with Vite.
	trap 'kill 0' EXIT; \
	(cd $(PRODUCT_APP) && $(BIN)/vite) & \
	(cd $(HOST_APP) && $(BIN)/vite) & \
	wait

preview: build ## Serve the production builds of both apps on the same ports.
	trap 'kill 0' EXIT; \
	(cd $(PRODUCT_APP) && $(BIN)/vite preview) & \
	(cd $(HOST_APP) && $(BIN)/vite preview) & \
	wait

fmt: ## Format Rust (pinned nightly) and TypeScript.
	cargo +$(NIGHTLY_TOOLCHAIN) fmt --all
	$(BIN)/prettier --write $(PRETTIER_FILES)

fmt-check: ## Check formatting without changing files.
	cargo +$(NIGHTLY_TOOLCHAIN) fmt --all -- --check
	$(BIN)/prettier --check $(PRETTIER_FILES)

# The wasm32 pass lints the web glue that `#[extension_point]` and
# `#[host_runtime::constructor]` write, which native builds compile out. Each
# provider is listed here; see "Add a provider" in `docs/checklists.md`.
WASM_CRATES := -p protocol -p host-extension -p host-runtime -p host-provider-wallet-mock -p host-provider-notifier-log -p host-app-web-wasm

clippy: ## Clippy with -D warnings: the native workspace, then the crates in the WASM binary.
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy $(WASM_CRATES) --target wasm32-unknown-unknown -- -D warnings

typecheck: packages wasm ## Typecheck the TypeScript, tests included.
	cd $(CLIENT_PKG) && $(BIN)/tsc -p tsconfig.test.json
	cd $(EXTENSION_PKG) && $(BIN)/tsc -p tsconfig.test.json
	cd $(PRODUCT_APP) && $(BIN)/tsc -p tsconfig.json
	cd $(HOST_APP) && $(BIN)/tsc -p tsconfig.test.json

test: packages wasm ## Rust tests (natively, and the web glue's under Node), then the bun tests.
	cargo test --workspace
	wasm-pack test --node rust/crates/host-extension
	cd $(CLIENT_PKG) && $(BUN) test
	cd $(EXTENSION_PKG) && $(BUN) test
	cd $(HOST_APP) && $(BUN) test

check: codegen fmt-check clippy typecheck test ## Everything CI would run.

swift: ## Build the host library, generate Swift bindings, run the Swift app.
	./scripts/native-bindings.sh swift
	cd apps/host-app-swift && swift run HostAppSwift

kotlin: ## Build the host library, generate Kotlin bindings, run the Kotlin app.
	./scripts/native-bindings.sh kotlin
	cd apps/host-app-kotlin && ./gradlew --console=plain -q run

native: swift kotlin ## Run both native apps.

clean: ## Remove build and codegen outputs.
	cargo clean
	rm -rf js/packages/*/dist $(HOST_APP)/wasm apps/*/dist
	rm -rf $(CLIENT_PKG)/src/generated $(EXTENSION_PKG)/src/generated
	rm -rf apps/host-app-swift/.build apps/host-app-swift/Sources/HostAppNative apps/host-app-swift/Sources/HostAppNativeFFI
	rm -rf apps/host-app-kotlin/build apps/host-app-kotlin/.gradle apps/host-app-kotlin/.kotlin
	find . -name '*.tsbuildinfo' -not -path './node_modules/*' -delete
