# Composable Core sketch

A small implementation of the [Composable Core RFC](https://github.com/paritytech/trinity-user-agents/pull/1958) on
truapi's architecture.

## What the sketch shows

- A product reaches the core only through protocol frames, sent by a client generated from the protocol crate.
- Everything a host supplies to the core is declared in one interface crate: the extension points `ProductStorage`
  and `UserConfirmation`, which every host passes, `Wallet` and `Notifier`, which a host may leave out, and the
  `ProductContext` and `FrameSink` of each product it frames.
- A provider crate, host code around a provider, or host code alone implements each extension point. Host code that
  wraps the wallet provider sees every request the core sends it, with no status API in the core.
- Each host has one host crate that links the core and its providers. The same constructor is exported through UniFFI
  to Swift and Kotlin and through wasm-bindgen to the web, where it takes typed extension point objects.

## Tour

Read each list in order; each line names a file and what to look at in it.

**A product call becomes a frame.**

1. `apps/product/src/main.ts`: the product calls `client.signing.signRaw(...)` and the other methods.
2. `js/packages/protocol-client/src/generated/client.ts` (after `make codegen`): each method encodes its request and
   decodes its result.
3. `js/packages/protocol-client/src/transport.ts`: frames, request ids, and subscriptions as observables.
4. `rust/crates/protocol/src/frame.rs`: the frame layout both sides share.

**The protocol is one trait per service.**

1. `rust/crates/protocol/src/api.rs`: the services and their trait ids.
2. `rust/crates/protocol/src/api/signing.rs`: a method, its wire id, its payload types, and why it exists.
3. `rust/crates/host-macros/src/method_table.rs`: the method table `#[wire_trait]` writes from the trait.
4. `rust/crates/tool-codegen/src/ts/client.rs`: the TypeScript client written from rustdoc JSON.

**The core serves the protocol over extension points.**

1. `rust/crates/host-runtime/src/runtime.rs`: `HostExtensions`, `HostRuntime`, and one `ProductRuntime` per product.
2. `rust/crates/host-runtime/src/dispatcher.rs`: a frame routed to its method by the wire pair.
3. `rust/crates/host-runtime/src/calls.rs`: the calls in flight, which a product runtime drops with itself.
4. `rust/crates/host-runtime/src/runtime/capabilities.rs`: each method, served over the extension points.
5. `rust/crates/host-runtime/src/runtime/bus.rs`: the wallet's account changes fanned out to every subscription.

**The interface crate declares what a host supplies.**

1. `rust/crates/host-extension/src/wallet.rs`: an extension point, its error, and the listener it pushes changes to.
2. `rust/crates/host-extension/src/notifier.rs`: the RFC's example.
3. `rust/crates/host-extension/src/storage.rs` and `confirmation.rs`: required extension points with no provider.
4. `rust/crates/host-extension/src/product.rs`: what a host passes for each product it frames.
5. `rust/crates/host-macros/src/extension_point.rs` and `web_glue.rs`: the UniFFI export and the web glue the
   attribute writes.

**Providers implement extension points.**

1. `rust/crates/host-provider-wallet-mock/src/wallet.rs`: a wallet on a simulated phone.
2. `rust/crates/host-provider-wallet-mock/src/native.rs` and `wasm.rs`: how it reaches Swift, Kotlin and JS.
3. `rust/crates/host-provider-notifier-log/src/notifier.rs`: a second provider, for the notifier.

**Each host links the core and its providers in one host crate.**

1. `apps/host-app-web/host-app-web-wasm/src/lib.rs`: the web host's constructor, `hostRuntime`.
2. `rust/crates/host-macros/src/constructor.rs`: what `#[host_runtime::constructor]` exports on each platform.
3. `apps/host-app-native/src/lib.rs`: the same constructor for Swift and Kotlin.

**Host code composes the runtime.**

1. `apps/host-app-web/src/main.ts`: the runtime built from the page's choices.
2. `apps/host-app-web/src/tracking-wallet.ts`: host code around the wallet provider.
3. `apps/host-app-web/src/ts-wallet.ts`: a wallet written only in TypeScript.
4. `apps/host-app-web/src/host-extensions.ts`: storage and confirmation on the page.
5. `js/packages/host/src/runtime.ts`: a product's port connected to its product runtime.
6. `apps/host-app-swift/Sources/HostAppSwift/main.swift` and `apps/host-app-kotlin/src/main/kotlin/Main.kt`: the
   same composition natively, driven by the stand-in product in `apps/product-native/src/lib.rs`.

## Run it

Install the prerequisites listed in [docs/development.md](docs/development.md), then:

```sh
make setup
make dev      # the web host app on http://localhost:5181, framing the product from :5182
make swift    # the Swift app
make kotlin   # the Kotlin app
make check    # formatting, lints, typechecks and tests
```

The web host app lets you pick the wallet (the mock provider wrapped by the tracking wallet, the provider as it is, a
TypeScript wallet, or none), the notifier (the log provider wrapped to show each notice, or none), how the user
confirmation answers, and how and when the simulated phone answers sign requests. It adds accounts, which the
product's subscription sees, and reloads the product, which drops its calls in flight. A missing wallet or notifier
makes the product calls that need it fail as `Unsupported`. Append `?product=<url>` to frame another product.

## More

- [docs/departures-from-the-rfc.md](docs/departures-from-the-rfc.md): where the sketch departs from the RFC, and
  what it follows from truapi or leaves out.
- [docs/checklists.md](docs/checklists.md): the files to touch to add a method, an extension point, a provider or a
  protocol service.
- [docs/development.md](docs/development.md): prerequisites, the Make targets, and what each test covers.
