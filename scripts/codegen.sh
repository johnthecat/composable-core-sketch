#!/usr/bin/env bash
# Regenerate the web side of the extension points from the interface crate.
#
# Pipeline:
#   1. cargo +<pinned nightly> doc -p sketch-wallet-extension --no-deps
#      -> target/doc/sketch_wallet_extension.json
#   2. cargo run -p sketch-codegen -- --input target/doc/sketch_wallet_extension.json
#                                     --ts-output js/packages/host/src/generated
#                                     --rust-output rust/crates/sketch-wallet-extension/src
#      -> extensions.ts, extensions-adapter.ts and the interface crate's wasm.rs
#   3. rustfmt (pinned nightly) on wasm.rs, prettier on the TypeScript
#
# rustdoc documents the host target, where the interface crate's `wasm` module
# is compiled out, so a stale or missing wasm.rs never blocks step 1.
#
# Run from anywhere; paths resolve from the repository root.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# The dated nightly from nightly-toolchain; SKETCH_NIGHTLY_TOOLCHAIN overrides it.
NIGHTLY_TOOLCHAIN="${SKETCH_NIGHTLY_TOOLCHAIN:-$(head -n1 "$ROOT/nightly-toolchain" | tr -d '[:space:]')}"

# Homebrew LLVM on DYLD_LIBRARY_PATH makes rustc and rustdoc load a mismatched
# libLLVM.
unset DYLD_LIBRARY_PATH

RUSTDOCFLAGS="${RUSTDOCFLAGS:-} -D warnings -Z unstable-options --output-format json" \
  cargo +"$NIGHTLY_TOOLCHAIN" doc -p sketch-wallet-extension --no-deps

cargo run -p sketch-codegen -- \
  --input target/doc/sketch_wallet_extension.json \
  --ts-output js/packages/host/src/generated \
  --rust-output rust/crates/sketch-wallet-extension/src

rustfmt +"$NIGHTLY_TOOLCHAIN" --edition 2024 \
  rust/crates/sketch-wallet-extension/src/wasm.rs

# --no: use the installed prettier, never resolve or fetch one.
npm exec --no -- prettier --write "js/packages/host/src/generated/**/*.ts"
