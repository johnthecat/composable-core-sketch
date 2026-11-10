# host-app-web-wasm

The web host app's own crate. It links the core and the providers the app uses into the app's one WASM binary, and
declares the runtime constructor once, as [host-app-native](../../host-app-native/README.md) does. The binary also
carries the providers' exports (`MockPhone`, `logNotifier`), each handing its provider to TypeScript as a typed object.
Why the two host crates read the same: see "One host crate per host" in
[docs/departures-from-the-rfc.md](../../../docs/departures-from-the-rfc.md).

## Look at

1. [`src/lib.rs`](src/lib.rs): the web host's constructor, `hostRuntime`.
2. [`scripts/build-wasm.mjs`](../../../scripts/build-wasm.mjs): the build, and the check that holds the binary's `.d.ts`
   to the shape the app's `tsc` relies on.

## Next

- [host-macros](../../../rust/crates/host-macros/README.md), for what `#[host_runtime::constructor]` exports.
