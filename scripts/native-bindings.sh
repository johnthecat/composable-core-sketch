#!/usr/bin/env bash
# Build the host library from `apps/host-app-native` and generate its
# UniFFI bindings for one app. None of the output is committed.
#
#   native-bindings.sh swift   -> apps/host-app-swift/Sources/HostAppNative/
#                                 (every crate's Swift in one module) and
#                                 Sources/HostAppNativeFFI/ (headers, one module map)
#   native-bindings.sh kotlin  -> apps/host-app-kotlin/build/generated/uniffi/
#                                 (one package per crate)
#
# Library mode reads the UniFFI metadata of every crate the library links, so
# one run covers:
#   - the core;
#   - the interface crate;
#   - the protocol;
#   - both providers.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

case "$(uname -s)" in
  Darwin) LIBRARY=target/debug/libhost_app_native.dylib ;;
  *) LIBRARY=target/debug/libhost_app_native.so ;;
esac

cargo build -p host-app-native

case "${1:-}" in
  swift)
    OUT=target/uniffi-swift
    rm -rf "$OUT"
    cargo run -q -p tool-uniffi-bindgen -- generate --library "$LIBRARY" --language swift --out-dir "$OUT"
    SWIFT=apps/host-app-swift/Sources
    rm -rf "$SWIFT/HostAppNative" "$SWIFT/HostAppNativeFFI"
    mkdir -p "$SWIFT/HostAppNative" "$SWIFT/HostAppNativeFFI"
    cp "$OUT"/*.swift "$SWIFT/HostAppNative/"
    cp "$OUT"/*.h "$SWIFT/HostAppNativeFFI/"
    # SwiftPM wants a module named after the target; the bindings import one
    # module per crate, all declared in the same map.
    {
      echo 'module HostAppNativeFFI { export * }'
      cat "$OUT"/*.modulemap
    } > "$SWIFT/HostAppNativeFFI/module.modulemap"
    ;;
  kotlin)
    OUT=apps/host-app-kotlin/build/generated/uniffi
    rm -rf "$OUT"
    cargo run -q -p tool-uniffi-bindgen -- generate --library "$LIBRARY" --language kotlin --no-format --out-dir "$OUT"
    ;;
  *)
    echo "usage: $0 swift|kotlin" >&2
    exit 2
    ;;
esac
