# host-provider-wallet-mock

A `Wallet` provider that simulates a wallet on a paired phone. It depends on the interface crate only, never on the
core. A host crate that links it carries both of its exports into its library.

## Look at

1. [`src/wallet.rs`](src/wallet.rs): a wallet on a simulated phone.
2. [`src/native.rs`](src/native.rs) and [`src/wasm.rs`](src/wasm.rs): how it reaches the native apps and the web. Each
   exports a `MockPhone`, which hands the provider out as a `Wallet` trait object for UniFFI and as a typed `Wallet`
   object for JS.

## Tests

[`src/wallet/tests.rs`](src/wallet/tests.rs): the phone answers each request as it was last told, and never answers when
unreachable. A signature binds the payload to the account. Signing with an account it does not hold fails. An added
account is pushed to the last listener given.

## Next

- [@sketch/host-app-web](../../../apps/host-app-web/README.md), whose tracking wallet wraps it.
