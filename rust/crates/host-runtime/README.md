# host-runtime

The core. It serves the product protocol to each product connection over the extension points the host supplies. It
depends on the interface crate and never on a provider: a host crate links it with the providers it picks.

## Look at

1. [`src/runtime.rs`](src/runtime.rs): `HostExtensions` and `HostRuntime`. It hands out one `ProductRuntime` per
   product.
2. [`src/dispatcher.rs`](src/dispatcher.rs): a frame routed to its method by the wire pair.
3. [`src/calls.rs`](src/calls.rs): the calls in flight, which a product runtime drops with itself. Natively they run on
   one executor thread that every host runtime shares, so a Swift or Kotlin extension point is called off the app's
   threads.
4. [`src/runtime/capabilities.rs`](src/runtime/capabilities.rs): each method, served over the extension points.
5. [`src/runtime/bus.rs`](src/runtime/bus.rs): the wallet's account changes fanned out to every subscription; see
   [pushed changes](../host-extension/README.md#pushed-changes).
6. [`src/wasm.rs`](src/wasm.rs): the runtime as a web host holds it.

## Tests

[`src/runtime/tests.rs`](src/runtime/tests.rs) drives the core by frames and checks that:

- a confirmed signature reaches the wallet with the product's payload;
- a refused confirmation is `Denied` before the wallet signs;
- an unknown account fails before the user is asked;
- no wallet is `Unsupported`;
- a notifier refusal reaches the product;
- dropping the product runtime drops a call still in flight;
- a subscription sends the current list and each pushed change, and nothing after it stops.

## Next

- [host-extension](../host-extension/README.md), which declares what the core calls.
