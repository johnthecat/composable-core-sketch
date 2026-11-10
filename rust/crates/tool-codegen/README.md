# tool-codegen

Generates the TypeScript interfaces of [@sketch/host-extension](../../../js/packages/host-extension/README.md) from the
interface crate's rustdoc JSON, with the protocol types an extension point names. `make codegen` runs it through
[`scripts/codegen.sh`](../../../scripts/codegen.sh). It writes no Rust.

## Look at

1. [`src/ts/extensions.rs`](src/ts/extensions.rs): the extension point interfaces and their exception classes.
2. [`src/extension.rs`](src/extension.rs): how it reads an extension point from rustdoc JSON.

## Tests

A snapshot of `extensions.ts` for the RFC's `Notifier` and a listener naming a protocol type
([`src/ts/extensions/tests/`](src/ts/extensions/tests/); rewrite with `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1`), and unit
tests of the rustdoc reader (async methods, wire tags, newtypes, public modules) and the TS type emission.
