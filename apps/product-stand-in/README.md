# product-stand-in

The product every demo host drives the core with. The RFC leaves products as they are: a product reaches the core only
through protocol frames. So instead of a real product, this crate builds the request frames a product sends, from the
protocol crate's own types, and reads the frames the core sends back as text (`describe_frame`). The bytes are what a
product sends. It is a demo helper, not a product SDK.

Each function is exported through UniFFI for the Swift and Kotlin apps and through wasm-bindgen for the web. Each host
crate links it like a provider, so the same builders reach every app from its one library or binary.

## Look at

1. [`src/lib.rs`](src/lib.rs): one frame builder per protocol method, and `describe_frame`.

## Next

- [@sketch/host-app-web](../host-app-web/README.md), whose page and composition test send these frames.
- [host-app-swift](../host-app-swift/README.md) and [host-app-kotlin](../host-app-kotlin/README.md), which send the same
  frames.
