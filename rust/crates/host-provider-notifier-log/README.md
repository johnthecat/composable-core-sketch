# host-provider-notifier-log

A `Notifier` provider that writes each notice to the log: the provider crate of the RFC's example, and the demo's second
provider. It depends on the interface crate only.

## Look at

1. [`src/notifier.rs`](src/notifier.rs): the provider, which writes each notice as one log line.
2. [`src/native.rs`](src/native.rs) and [`src/wasm.rs`](src/wasm.rs): `log_notifier` returns it as a trait object for
   UniFFI, and `logNotifier` hands it to JS as a typed `Notifier` object.

## Tests

[`src/notifier/tests.rs`](src/notifier/tests.rs): a notice becomes one log line, and an empty one is refused.

## Next

- [@sketch/host-app-web](../../../apps/host-app-web/README.md), which wraps it to show each notice on the page.
