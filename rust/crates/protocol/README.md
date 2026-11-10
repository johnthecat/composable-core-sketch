# protocol

The product protocol that products speak to the core: one trait per service with the payloads its methods carry, and the
frame layout both sides share. It holds no runtime, so the interface crate names its types in extension point signatures
without depending on the core.

## Look at

1. [`src/api.rs`](src/api.rs): the services and their trait ids.
2. [`src/api/signing.rs`](src/api/signing.rs): a method with its wire id and payload types. Its doc says why it exists.
3. [`src/frame.rs`](src/frame.rs): the frame layout both sides share.
4. [`src/dispatch.rs`](src/dispatch.rs): the method table a dispatcher serves, which `#[wire_trait]` fills from each
   trait.
5. [`src/bytes.rs`](src/bytes.rs): the byte types. Each says how it crosses each boundary itself. Extension point values
   hold bytes only as these: the attributes refuse `Vec<u8>` and `[u8; N]`, and `tool-codegen` reads each newtype as its
   field.

## Tests

[`src/frame/tests.rs`](src/frame/tests.rs) checks the frame layout against the bytes the TypeScript client's test checks
too.

## Next

- [host-macros](../host-macros/README.md), for what `#[wire_trait]` writes.
- [host-runtime](../host-runtime/README.md), which serves the protocol.
- [@sketch/protocol-client](../../../js/packages/protocol-client/README.md), the client generated from it.
