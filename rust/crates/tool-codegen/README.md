# tool-codegen

Generates the TypeScript side of the sketch from rustdoc JSON. `make codegen` runs it through
[`scripts/codegen.sh`](../../../scripts/codegen.sh). It writes the client of
[@sketch/protocol-client](../../../js/packages/protocol-client/README.md) from the protocol crate, and the interfaces of
[@sketch/host-extension](../../../js/packages/host-extension/README.md) from the interface crate.

## Look at

1. [`src/ts/client.rs`](src/ts/client.rs): the TypeScript client written from rustdoc JSON.
2. [`src/ts/extensions.rs`](src/ts/extensions.rs): the extension point interfaces.

## Tests

A snapshot of `extensions.ts` for the RFC's `Notifier` and a listener naming a protocol type
([`src/ts/extensions/tests/`](src/ts/extensions/tests/); rewrite with `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1`), and unit
tests of the rustdoc reader (async methods, wire tags, newtypes, public modules) and the TS type emission.
