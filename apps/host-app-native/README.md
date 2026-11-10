# host-app-native

A native host's own crate, as the RFC's "iOS and Android hosts" section describes. It links the core and the providers
the host uses into one library, and declares the runtime constructor once. It builds as a static library for the [Swift
app](../host-app-swift/README.md) and a dynamic one for the [Kotlin app](../host-app-kotlin/README.md).

Each crate it links keeps its own UniFFI namespace, so the library also carries the providers' exports (`MockPhone`,
`log_notifier`) and the stand-in product's frame builders ([product-native](../product-native/README.md)). Why it reads
the same as [host-app-web-wasm](../host-app-web/host-app-web-wasm/README.md): see "One host crate per host" in
[docs/departures-from-the-rfc.md](../../docs/departures-from-the-rfc.md).

## Look at

1. [`src/lib.rs`](src/lib.rs): the same constructor for Swift and Kotlin.
2. [`scripts/native-bindings.sh`](../../scripts/native-bindings.sh): the library build and its bindings for one app.
