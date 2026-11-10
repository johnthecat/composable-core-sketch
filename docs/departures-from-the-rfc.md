# Where the sketch departs from the RFC

1. **Everything the host supplies is declared in the interface crate.** Replaces the RFC's `HostCallbacks`: storage and
   confirmation are extension points like the wallet and the notifier, required in `HostExtensions` while the other
   two are `Option`. The product's `ProductContext` and `FrameSink` are an `#[extension_type]` and an
   `#[extension_listener]` there too, so the web host gets their TypeScript from codegen like the rest. Each extension
   point is a public module of the interface crate, such as `host_extension::wallet`, holding its trait, its error,
   the types it names and, on wasm32, its web glue.
2. **One host crate per host, one binary.** Replaces separate provider WASM binaries and the interface crate's
   `export` Cargo feature: each host crate links the core and its providers, so a web host ships one WASM binary, and a
   provider hands itself to JS with `Wasm<Trait>::into_js`. The sketch's two host crates,
   `apps/host-app-web/host-app-web-wasm` and `apps/host-app-native`, read the same only because its hosts link the
   same two providers; a real host links its own set. The native one also links the stand-in product, whose exports
   the Swift and Kotlin apps call. The trade-off: a web host that builds its own combination
   of providers needs a Rust toolchain. Prebuilt bundles, one per standard host, would spare the others that.
3. **Attributes, not generated Rust.** Replaces `truapi-codegen` writing `src/wasm.rs`: `#[extension_point]`,
   `#[extension_listener]`, `#[extension_error]` and `#[extension_type]` write the UniFFI export and the web glue, and
   `#[wire_trait]` writes the protocol's method tables. `tool-codegen` writes TypeScript only. The macros live in one
   crate, `host-macros`, as truapi's live in `truapi-macros`.
4. **The host crate's constructor, not `createHostRuntime`.** Replaces `@parity/truapi-host/js`'s `createHostRuntime`:
   `#[host_runtime::constructor]` exports the host crate's `fn host_runtime(...)` through UniFFI natively and as
   `hostRuntime` on the web. It keeps one parameter per extension point for Swift, Kotlin and TS, and its body builds
   `HostExtensions` for `HostRuntime::new`. Each `Js<Trait>` parameter is typed `HostExtension.<Trait>`; the
   interface crate names its TypeScript package once, `typescript_module!("@sketch/host-extension")`, which adds
   `import type * as HostExtension from "@sketch/host-extension";` to the binary's `.d.ts`. The constructor throws,
   naming the method, when an object lacks one. `scripts/build-wasm.mjs` checks that the `.d.ts` has that one import
   and no `Wasm<Trait>` class; the app's `tsc` checks every signature against `@sketch/host-extension`.
5. **Calls go from the core to an extension point; changes are pushed into the core.** A call is
   `async fn(&self, owned values) -> Result<T, <Trait>Error>`. No call stays pending to deliver changes (see
   [Pushed changes](#pushed-changes)).
6. **Dropping a call follows truapi's rule.** A product runtime owns its calls in flight, so freeing it drops them.
   The core dropping a call cancels a Swift or Kotlin implementation's task. A Rust provider behind a Swift wrapper
   finishes its call, and the core discards the result; so does a JS implementation, since a promise cannot be
   cancelled.
7. **Errors and their names.** `<Trait>Error` lists only declared failures, and `#[extension_point]` refuses another
   name. `#[extension_error]` adds `Unknown { reason }`, `From<String>`, and natively
   `From<UnexpectedUniFFICallbackError>`. On the web a declared error rejects as an `Error` named `<Trait>Exception`
   whose `error` property holds it, which the generated class of that name accepts with `instanceof`; any other
   rejection becomes `Unknown`. Kotlin keeps UniFFI's `<Trait>Exception`. An error is a type of its trait's crate,
   because UniFFI's Kotlin bindings cannot lower another crate's error from a foreign trait's async method.
8. **One UniFFI namespace per crate.** Library mode generates bindings per crate from one library. Swift compiles them
   into one module, `HostAppNative`; Kotlin gets a package per crate.
9. **The interface crate depends on the protocol crate, never on the core.** `Wallet::accounts` returns the protocol's
   `ProductAccount` as it is. A protocol record an extension point carries is marked `#[protocol_record]`, which adds
   the same UniFFI and serde attributes `#[extension_type]` adds, so the protocol copies none by hand.
10. **Byte values say how they cross.** The protocol's `Bytes`, `Bytes32` and `Bytes64` are newtypes that SCALE
    encodes as their contents, UniFFI lowers as bytes, and serde serializes with `serialize_bytes`, which reaches JS as
    a `Uint8Array`. The attributes refuse `Vec<u8>` and `[u8; N]` in extension point values, and `tool-codegen` reads a
    newtype as its field.

## Pushed changes

As truapi does for the theme (`current_theme()` plus `notify_theme_changed` into its `NativeEventBus`), the wallet has a
plain read, `accounts()`, and pushes each change to an `AccountsListener` (`accounts_changed(accounts)`), declared
with `#[extension_listener]`. The core hands its listener to the wallet once, through `Wallet::set_accounts_listener`,
when `HostRuntime::new` attaches the wallet. Its listener, a `Bus<T>` in `host-runtime/src/runtime/bus.rs`, fans each
change out to every product subscription, which starts with the current list. Ending a product subscription drops
only the core's receiver; nothing stays pending in the wallet.

A wrapper that wants to see the changes gives the inner wallet its own listener and forwards to the core's, as the
tracking wallets in TypeScript, Swift and Kotlin do. A listener method returns nothing and returns at once; on the web
a JS failure in one is logged, because the caller has nowhere to receive it. The listener is set through a trait
method, not at provider construction, because the core does not exist yet when a provider is built, and because a
wrapper then needs no setup of its own: it wraps the listener it is given.

## Native notes

- **One executor thread.** Natively every product call of every host runtime runs on one shared executor thread, as
  truapi's calls run on its bridge executor, so a Swift or Kotlin extension point is called off the app's threads.
  Nothing that runs there may block it; the interface crate's docs state the threading contract. Calls run in the
  order they are spawned or woken.
- **The native apps play the product with a stand-in.** A native truapi host runs products in web views with the TS
  client. The Swift and Kotlin apps have none, so they build request frames with `apps/product-native` from the
  protocol crate's own types (`sign_raw_frame`, ...), read received frames as text (`describe_frame`), and wait for
  each response before the next step. The bytes are what the TS client sends; it is not a product SDK.

## What follows truapi, and what is left out

Followed: the protocol traits with wire-id doc tags; the frame layout and leg bytes; a `Dispatcher` over per-method
tables; `ProductRuntimeHost` serving each service; `productRuntime(product, { emitFrame })` and `receiveFrame`; the
generated TS client over `TrUApiTransport` with neverthrow; the iframe `truapi-ready`/`truapi-init` handshake; rustdoc
JSON on the pinned nightly; generated files ignored by git. Trait ids match truapi's: Account 2, LocalStorage 7,
Notifications 8, Signing 13.

Left out, since none changes how an extension point attaches:

- protocol versioning: truapi carries each payload in a versioned envelope and serves it through
  `version(request).exec(...)`. Here each method names its request, response and error types directly, so a frame's
  payload is that type's SCALE encoding, and a method without a request sends an empty payload;
- the protocol error truapi answers on its reserved address for a frame no method serves; the sketch drops such a
  frame;
- the `<Bytes>…</Bytes>` wrapping truapi's signing host puts around a raw message; the wallet signs the payload as
  the product sent it;
- the `Cancel` leg, host-initiated subscriptions, the handshake, request timeouts, the wire schema hash, the debug tap;
- running the core in a Web Worker, and reconnecting an iframe that navigates.

The mock and TypeScript wallets' keys and signatures are SHA-2 hashes standing in for real ones.
