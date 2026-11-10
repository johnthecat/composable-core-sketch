# Composable Core sketch

A small implementation of the [Composable Core RFC](https://github.com/paritytech/trinity-user-agents/pull/1958) on
truapi's architecture.

## What the sketch shows

- The RFC leaves products as they are: a product reaches the core only through protocol frames. So every host here
  drives the core with the same stand-in product, which builds those frames from the protocol crate's own types.
- Everything a host supplies to the core is declared in one interface crate:
  - the extension points `ProductStorage` and `UserConfirmation`, which every host passes;
  - `Wallet` and `Notifier`, which a host may leave out;
  - the `ProductContext` and `FrameSink` of each product it frames.
- A provider crate, host code around a provider, or host code alone implements each extension point. Host code that
  wraps the wallet provider sees every request the core sends it, with no status API in the core.
- Each host has one host crate that links the core and its providers. The same constructor is exported through UniFFI
  to Swift and Kotlin and through wasm-bindgen to the web, where it takes typed extension point objects.

## Packages

Open them in this order. Each README says what to look at in the package and what its tests prove.

**Protocol**

- [`protocol`](rust/crates/protocol/README.md): the services products call, and the frame layout both sides share.

**Core**

- [`host-runtime`](rust/crates/host-runtime/README.md): the core, serving the protocol over the extension points.

**Interface crate**

- [`host-extension`](rust/crates/host-extension/README.md): every extension point a host supplies.
- [`@sketch/host-extension`](js/packages/host-extension/README.md): its TypeScript interfaces, generated.

**Macros**

- [`host-macros`](rust/crates/host-macros/README.md): the proc macros that write the extension point glue and the
  protocol's method tables.

**Providers**

- [`host-provider-wallet-mock`](rust/crates/host-provider-wallet-mock/README.md): a wallet on a simulated phone.
- [`host-provider-notifier-log`](rust/crates/host-provider-notifier-log/README.md): a notifier that writes to the log.

**Host crates and apps**

- [`host-app-web-wasm`](apps/host-app-web/host-app-web-wasm/README.md): the web host's crate, linking the core and its
  providers into one WASM binary.
- [`@sketch/host-app-web`](apps/host-app-web/README.md): the web host app.
- [`host-app-native`](apps/host-app-native/README.md): the native host's crate, linking the core and its providers into
  one library.
- [`host-app-swift`](apps/host-app-swift/README.md): the Swift host app.
- [`host-app-kotlin`](apps/host-app-kotlin/README.md): the Kotlin host app.

**Product**

- [`product-stand-in`](apps/product-stand-in/README.md): the stand-in product every host drives the core with.

**Tools**

- [`tool-codegen`](rust/crates/tool-codegen/README.md): the TypeScript generator.
- [`tool-uniffi-bindgen`](rust/crates/tool-uniffi-bindgen/README.md): the pinned UniFFI binding generator.

## Run it

Install the prerequisites listed in [docs/development.md](docs/development.md), then:

```sh
make setup
make dev      # the web host app on http://localhost:5181
make swift    # the Swift app
make kotlin   # the Kotlin app
make check    # formatting and lints, then typechecks and tests
```

[`@sketch/host-app-web`](apps/host-app-web/README.md) describes what the web host page lets you choose.

## More

- [docs/departures-from-the-rfc.md](docs/departures-from-the-rfc.md): where the sketch departs from the RFC, and
  what it follows from truapi or leaves out.
- [docs/checklists.md](docs/checklists.md): the files to touch to add one of these:
  - a method;
  - an extension point;
  - a provider;
  - a protocol service.
- [docs/development.md](docs/development.md):
  - prerequisites;
  - the Make targets;
  - why the stop checks need no timer.
