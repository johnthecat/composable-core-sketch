# host-extension

The interface crate: everything a host supplies to the core is declared here, one module per extension point. The core,
every provider and the host code depend on it. It depends on the protocol crate, never on the core.

## Look at

1. [`src/wallet.rs`](src/wallet.rs): an extension point and its error, then the listener it pushes changes to.
2. [`src/notifier.rs`](src/notifier.rs): the RFC's example.
3. [`src/storage.rs`](src/storage.rs) and [`src/confirmation.rs`](src/confirmation.rs): required extension points with
   no provider.
4. [`src/product.rs`](src/product.rs): what a host passes for each product it frames.
5. [`src/lib.rs`](src/lib.rs): how to declare an extension point. It also states the threading and cancellation
   contract.
6. [`src/wasm.rs`](src/wasm.rs): what the web glue is built on.

## Pushed changes

As truapi does for the theme (`current_theme()` plus `notify_theme_changed` into its `NativeEventBus`), the wallet has a
plain read, `accounts()`, and pushes each change to an `AccountsListener` (`accounts_changed(accounts)`), declared with
`#[extension_listener]`. The core hands its listener to the wallet once, through `Wallet::set_accounts_listener`, when
`HostRuntime::new` attaches the wallet. Its listener, a `Bus<T>` in
[`host-runtime/src/runtime/bus.rs`](../host-runtime/src/runtime/bus.rs), fans each change out to every product
subscription, which starts with the current list. Ending a product subscription drops only the core's receiver; nothing
stays pending in the wallet.

A wrapper that wants to see the changes gives the inner wallet its own listener and forwards to the core's. The tracking
wallets in TypeScript, Swift and Kotlin do this. The listener is set through a trait method, not at provider
construction, because the core does not exist yet when a provider is built, and because a wrapper then needs no setup of
its own: it wraps the listener it is given.

## Tests

[`src/wallet/tests.rs`](src/wallet/tests.rs) checks that an undeclared failure becomes `Unknown`, and that a declared
one keeps its message. Under Node, [`src/wasm/tests.rs`](src/wasm/tests.rs) checks the web glue's conversions:

- each byte type round-trips as a `Uint8Array`;
- a value that does not convert is an error;
- only a rejection named for the trait is a declared error.

## Next

- [host-provider-wallet-mock](../host-provider-wallet-mock/README.md), a provider of this crate's `Wallet`.
- [@sketch/host-extension](../../../js/packages/host-extension/README.md), its TypeScript side.
