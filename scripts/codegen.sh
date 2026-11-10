#!/usr/bin/env bash
# Generate the TypeScript the sketch does not write by hand. Nothing it writes
# is committed, and no Rust is generated: `#[wire_trait]` writes the method
# tables and `#[extension_point]` the web glue.
#
# Pipeline:
#   1. cargo +<pinned nightly> doc -p protocol -p host-extension --no-deps
#      -> target/doc/{protocol,host_extension}.json
#   2. cargo run -p tool-codegen
#      -> the extension point interfaces (@sketch/host-extension), with the
#         protocol types they name
#   3. prettier on the generated TypeScript

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# The dated nightly; the Makefile exports NIGHTLY_TOOLCHAIN, and run on its own
# the script reads nightly-toolchain.
NIGHTLY_TOOLCHAIN="${NIGHTLY_TOOLCHAIN:-$(head -n1 "$ROOT/nightly-toolchain" | tr -d '[:space:]')}"

RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -D warnings -Z unstable-options --output-format json" \
  cargo +"$NIGHTLY_TOOLCHAIN" doc -p protocol -p host-extension --no-deps

cargo run -p tool-codegen -- \
  --protocol-input target/doc/protocol.json \
  --extension-input target/doc/host_extension.json \
  --output js/packages/host-extension/src/generated

# --no: use the installed prettier, never resolve or fetch one.
npm exec --no -- prettier --write "js/packages/host-extension/src/generated/**/*.ts"
