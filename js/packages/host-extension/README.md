# @sketch/host-extension

The TypeScript side of the interface crate, [host-extension](../../../rust/crates/host-extension/README.md):
[tool-codegen](../../../rust/crates/tool-codegen/README.md) generates it from that crate's rustdoc JSON. Host code
implements its interfaces, and a host's WASM binary imports it as `HostExtension` to type its constructor's parameters.

## Look at

1. `src/generated/extensions.ts` (after `make codegen`): one interface per extension point, and one exception class per
   declared error.
2. [`src/index.ts`](src/index.ts): what the package exports.

## Tests

[`src/extensions.test.ts`](src/extensions.test.ts): a rejection from the WASM glue is accepted by its generated
exception class.

## Next

- [@sketch/host-app-web](../../../apps/host-app-web/README.md), whose host code implements these interfaces.
