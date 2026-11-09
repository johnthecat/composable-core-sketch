# Composable Core sketch

A small implementation of the [Composable Core RFC](https://github.com/paritytech/trinity-user-agents/pull/1958) on
truapi's architecture. A product talks to the core only through protocol frames. Everything the host supplies to the
core is an extension point, a trait in one interface crate: `ProductStorage` and `UserConfirmation`, which every host
passes, and `Wallet` and `Notifier`, which a host may leave out. A provider crate, host code around a provider, or
host code alone implements each one. Host code wraps the wallet provider and sees every request the core sends it,
with no status API in the core.

Each host has one host crate that links the core and the providers it uses and declares the runtime constructor. The
same declaration is exported through UniFFI for the Swift and Kotlin apps and through wasm-bindgen for the web app,
whose one WASM binary takes and returns typed extension point objects. The trade-off: a web host that builds its own
combination of providers needs a Rust toolchain. Prebuilt bundles, one per standard host, would spare the others that.

## Where to look

Folder and package share one name, prefixed by the RFC role.

| RFC role                                | Folder                                                                                                  |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Protocol: the product's API to the host | `rust/crates/protocol`; the product client `js/packages/protocol-client`                                |
| Core                                    | `rust/crates/host-runtime`                                                                              |
| Interface crate                         | `rust/crates/host-extension`: `wallet.rs`, `notifier.rs`, `storage.rs`, `confirmation.rs`               |
| Macros, for the protocol and the glue   | `rust/crates/sketch-macros`                                                                             |
| Providers                               | `rust/crates/host-provider-wallet-mock`, `rust/crates/host-provider-notifier-log`                       |
| Shared web host code                    | `js/packages/host` (`@sketch/host`): the generated extension point interfaces, the iframe host         |
| Web host app                            | `apps/host-app-web`; its host crate `apps/host-app-web/host-app-web-wasm`                               |
| Native host crate                       | `apps/host-app-native`                                                                                  |
| Swift and Kotlin apps                   | `apps/host-app-swift`, `apps/host-app-kotlin`                                                           |
| Product                                 | `apps/product`: the page the host frames, on its own origin                                             |
| Tools                                   | `rust/crates/tool-codegen`, `rust/crates/tool-uniffi-bindgen`, `scripts/`                               |

Every app is under `apps/`, every Rust crate the apps do not own under `rust/crates/`. The web host crate sits inside
the app it builds, because only that app imports its output, `apps/host-app-web/wasm/`; the Swift and Kotlin apps
share `apps/host-app-native`. The two host crates read the same only because the sketch's hosts link the same two
providers; a real host links its own set, so there is one host crate per host.

## Follow one call

`signRaw` on the web, from the product to the wallet provider and back:

1. `apps/product/src/main.ts` calls `client.signing.signRaw(...)`.
2. `js/packages/protocol-client/src/generated/client.ts` encodes the versioned request, and `src/transport.ts` sends
   it as a frame, `[requestId][trait 13][method 0][message type][payload]`, on the `MessagePort` that `src/sandbox.ts`
   received from the host.
3. `js/packages/host/src/runtime.ts` (`connectProductPort`) hands the frame to `ProductRuntime.receiveFrame` in
   `rust/crates/host-runtime/src/wasm.rs`, and `src/host_core.rs` decodes it.
4. `src/dispatcher.rs` looks the wire pair up in the method table that `#[wire_trait]` writes on
   `rust/crates/protocol/src/api/signing.rs` (`sketch-macros/src/method_table.rs`), and the table calls
   `Signing::sign_raw` in `host-runtime/src/runtime/capabilities.rs`. It checks the account with `Wallet::accounts`,
   asks `UserConfirmation`, declared in `rust/crates/host-extension/src/confirmation.rs`, then calls `Wallet::sign`,
   declared in `src/wallet.rs`.
5. On the web the core holds each of those as `Foreign<Trait>`, written by `#[extension_point]`
   (`sketch-macros/src/web_glue.rs`) over `host-extension/src/wasm.rs`. `hostRuntime` in
   `apps/host-app-web/host-app-web-wasm/src/lib.rs`, expanded by `sketch-macros/src/constructor.rs`, wrapped them when
   `apps/host-app-web/src/main.ts` built the runtime. The confirmation is `pageConfirmation` in
   `apps/host-app-web/src/host-extensions.ts`.
6. `ForeignWallet` calls the JS wallet, `apps/host-app-web/src/tracking-wallet.ts`: host code that records the
   request as pending on the phone, then answered or declined.
7. The tracking wallet calls `phone.wallet()`, a `WasmWallet` from the same binary
   (`rust/crates/host-provider-wallet-mock/src/wasm.rs`), which calls `MockWallet::sign` in `src/wallet.rs`.

The signature returns the same way, and the core sends the response frame to the product through `emitFrame`.

## Layout

```
rust/crates/
  protocol/                    api.rs (services!), one trait per service in api/ with wire ids; payloads in v01.rs;
                               envelopes in versioned/; `latest`; dispatch.rs (the method table types); bytes.rs
  host-runtime/                frame.rs, dispatcher.rs, subscription.rs, host_core.rs (HostRuntime, ProductRuntime,
                               FrameSink), runtime/ (services, versioning, bus.rs), wasm.rs
  host-extension/              wallet.rs, notifier.rs, storage.rs, confirmation.rs; wasm.rs (what the web glue
                               calls) and wasm/tests.rs (run under Node)
  sketch-macros/               lib.rs (entry points); one module per macro; type_path.rs; web_glue.rs
  host-provider-wallet-mock/   wallet.rs; wasm.rs (MockPhone); native.rs
  host-provider-notifier-log/  notifier.rs; wasm.rs (logNotifier); native.rs
  tool-codegen/                rustdoc JSON -> the product client and the extension point interfaces (TypeScript)
  tool-uniffi-bindgen/         UniFFI's binding generator at the workspace's version
js/packages/
  protocol-client/             transport.ts, scale.ts, sandbox.ts; generated client.ts, types.ts, wire-table.ts
  host/                        runtime.ts (Product, FrameSink, connectProductPort), iframe-host.ts; generated
                               extensions.ts
apps/
  product/                     index.html, src/main.ts
  host-app-web/                host-app-web-wasm/ (the host crate); wasm/ (its build output); index.html,
                               src/main.ts, host-extensions.ts, tracking-wallet.ts, ts-wallet.ts,
                               composition.test.ts
  host-app-native/             the native host crate; product_frames.rs (the demos' frame builders)
  host-app-swift/              SwiftPM package, Sources/HostAppSwift/main.swift
  host-app-kotlin/             Gradle project, src/main/kotlin/Main.kt
scripts/                       codegen.sh, build-wasm.mjs, native-bindings.sh
```

## What the web host app offers

`apps/host-app-web` serves on <http://localhost:5181> and frames the product from <http://localhost:5182>; append
`?product=<url>` to frame another. It lets you pick:

- the wallet: the mock provider wrapped by the tracking wallet, the provider as it is, a wallet written only in
  TypeScript (`ts-wallet.ts`), or none;
- the notifier: the log provider wrapped to show each notice on the page, as in the RFC's example, or none;
- how the user confirmation answers: ask on the page, always approve, always deny;
- how the simulated phone answers sign requests, and when; and it adds accounts, which the product's subscription
  sees. The tracking table shows each change the wallet pushes. The product can stop following accounts and follow
  again.

A missing wallet or notifier makes the product calls that need it fail as `CallError::Unsupported`.

## Where the sketch departs from the RFC

1. **Everything the host supplies is an extension point.** Replaces the RFC's `HostCallbacks`: storage and
   confirmation are traits in the interface crate like the wallet and the notifier, required parameters of the
   constructor, while the other two are `Option`.
2. **One host crate per host, one binary.** Replaces separate provider WASM binaries and the interface crate's
   `export` Cargo feature: each host crate links the core and its providers, so a web host ships one WASM binary, and a
   provider hands itself to JS with `Wasm<Trait>::into_js`.
3. **Attributes, not generated Rust.** Replaces `truapi-codegen` writing `src/wasm.rs`: `#[extension_point]`,
   `#[extension_listener]`, `#[extension_error]` and `#[extension_type]` write the UniFFI export and the web glue, and
   `#[wire_trait]` writes the protocol's method tables. `tool-codegen` writes TypeScript only. The macros live in one
   crate, `sketch-macros`, as truapi's live in `truapi-macros`.
4. **The host crate's constructor, not `createHostRuntime`.** Replaces `@parity/truapi-host/js`'s `createHostRuntime`:
   `#[host_runtime::constructor]` exports the host crate's `fn host_runtime(...)` through UniFFI natively and as
   `hostRuntime` on the web. Its body passes the extension points and `host_runtime::platform_spawner()`, where
   subscriptions run, to `HostRuntime::new`. Each `Js<Trait>` parameter is typed `HostExtension.<Trait>`; the interface crate names its
   TypeScript package once, `typescript_module!("@sketch/host")`, which adds
   `import type * as HostExtension from "@sketch/host";` to the binary's `.d.ts`. The constructor throws, naming the
   method, when an object lacks one. `scripts/build-wasm.mjs` checks that the `.d.ts` has that one import and no
   `Wasm<Trait>` class; the app's `tsc` checks every signature against `@sketch/host`.
5. **Calls go from the core to an extension point; changes are pushed into the core.** A call is
   `async fn(&self, owned values) -> Result<T, <Trait>Error>`. No call stays pending to deliver changes (see Pushed
   changes).
6. **Dropping a call follows truapi's rule.** The core dropping a call cancels a Swift or Kotlin implementation's
   task. A Rust provider behind a Swift wrapper finishes its call, and the core discards the result; so does a JS
   implementation, since a promise cannot be cancelled.
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

### Pushed changes

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

### Native notes

- **The core calls foreign code from its own threads.** Natively each product subscription runs on a thread of its
  own, so a Swift or Kotlin extension point is called off the app's threads.
- **The native apps play the product with frame builders.** A native truapi host runs products in web views with the
  TS client. The Swift and Kotlin apps have none, so they build request frames with
  `apps/host-app-native/src/product_frames.rs` from the protocol crate's own types (`sign_raw_frame`, ...) and print
  received frames as text (`describe_frame`). The bytes are what the TS client sends; it is not a product SDK.

## What follows truapi, and what is left out

Followed: the protocol with wire-id doc tags, version modules, `versioned_type!` envelopes and `latest`; the frame
layout and leg bytes; a `Dispatcher` and `SubscriptionManager` over per-method tables; `ProductRuntimeHost` serving
each service through `version(request).exec(...)`; `productRuntime(product, { emitFrame })` and `receiveFrame`; the
generated TS client over `TrUApiTransport` with neverthrow; the iframe `truapi-ready`/`truapi-init` handshake;
rustdoc JSON on the pinned nightly; generated files ignored by git. Trait ids match truapi's: Account 2,
LocalStorage 7, Notifications 8, Signing 13.

Left out, since none changes how an extension point attaches:

- a second protocol version, so no hand-written `IntoLatest`/`FromLatest` and no `UnsupportedVersion` refusal;
- the `Cancel` leg, host-initiated subscriptions, the handshake, request timeouts, the wire schema hash, the debug tap;
- running the core in a Web Worker, and reconnecting an iframe that navigates.

The mock and TypeScript wallets' keys and signatures are SHA-2 hashes standing in for real ones.

## Pipeline

Prerequisites: Rust stable with the `wasm32-unknown-unknown` target; the dated nightly in `nightly-toolchain` with
rustfmt; `wasm-pack`; Node 20 or newer; bun, on `PATH` or from the root `devDependencies`. For the native apps:
Xcode's command line tools (Swift 5.9 or newer), and a JDK with Gradle (`brew install gradle`). Make is the only
command layer; the npm packages declare no scripts.

| Target           | What it does                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------------- |
| `make setup`     | `npm install`, the pinned nightly with rustfmt, the wasm32 target                                                |
| `make codegen`   | `scripts/codegen.sh`: rustdoc JSON of `protocol` and `host-extension`, one `tool-codegen` run, prettier          |
| `make wasm`      | `scripts/build-wasm.mjs`: `wasm-pack build --target web` of `host-app-web-wasm` into `apps/host-app-web/wasm/`   |
| `make packages`  | `codegen`, then `tsc -b` for `@sketch/protocol-client` and `@sketch/host`                                        |
| `make build`     | `packages` and `wasm`, then typecheck and `vite build` for `apps/product` and `apps/host-app-web`                |
| `make dev`       | `packages` and `wasm`, then Vite for the host app on :5181 and the product on :5182                              |
| `make preview`   | `build`, then the production builds on the same ports                                                            |
| `make fmt`       | `cargo +<nightly> fmt --all` and prettier; `make fmt-check` checks only                                          |
| `make clippy`    | clippy with `-D warnings` on the native workspace, then on wasm32 for the crates in `WASM_CRATES`                |
| `make typecheck` | `packages` and `wasm`, then `tsc` over both packages and both apps, tests included                               |
| `make test`      | `packages` and `wasm`, then `cargo test --workspace`, `wasm-pack test --node rust/crates/host-extension`, and `bun test` in `protocol-client`, `host` and `host-app-web` |
| `make check`     | `codegen`, `fmt-check`, `clippy`, `typecheck`, `test`                                                            |
| `make swift`     | `scripts/native-bindings.sh swift` (builds `host-app-native`, generates bindings), `swift run HostAppSwift`      |
| `make kotlin`    | `scripts/native-bindings.sh kotlin`, `gradle run`                                                                |
| `make native`    | both native apps                                                                                                 |
| `make clean`     | removes build and codegen outputs                                                                                |

Nothing `make codegen` writes is committed. The native apps need no codegen: their bindings come from UniFFI.

## Tests

- `tool-codegen`: a snapshot of `extensions.ts` for the RFC's `Notifier` and a listener naming a protocol type
  (`src/tests/*.snap`; rewrite with `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1`), and unit tests of the rustdoc reader and the
  TS type emission.
- `host-runtime`: the core driven by frames. A confirmed signature wraps the message in `<Bytes>…</Bytes>`; a refused
  confirmation is `Denied` before the wallet signs; an unknown account fails before the user is asked; no wallet is
  `Unsupported`; a notifier refusal reaches the product; a subscription sends the current list and each pushed
  change, and nothing after it stops. The frame layout matches the bytes the TS client's test checks.
- `host-extension`: an undeclared failure becomes `Unknown`, and a declared one keeps its message. Under Node, each
  byte type round-trips through the web glue's conversion as a `Uint8Array`, a value that does not convert is an
  error, and only a rejection named for the trait is declared.
- `host-provider-wallet-mock`: an added account is pushed to the last listener given.
- `@sketch/host`: a rejection from the WASM glue is accepted by its generated exception class.
- `@sketch/protocol-client`: the frame bytes, and the generated client over a loopback transport.
- `apps/host-app-web/src/composition.test.ts`: the glue, through the app's real WASM binary with the product's generated
  client on the far end of a `MessageChannel`: a signature through the tracking wallet, a declined request, the
  TypeScript wallet, the notifier wrapper, an account pushed through the tracking wallet and through the TypeScript
  wallet, storage scoped to each product, and an extension point object missing a method.
- The Swift and Kotlin apps check two things as they run, and exit with a failure if either fails: that the product
  received the current account list and then the pushed change, and that a stopped subscription receives no item
  when another account is added.

## Checklists

Each step is one file. "Verify" is the command that proves the change on every platform.

### Add a method to an extension point

By hand:

1. `rust/crates/host-extension/src/<point>.rs`: the method, `async fn(&self, owned values) -> Result<T, <Trait>Error>`.
2. The same file: any new record or enum it names, with `#[extension_type]`; any new failure, as a `<Trait>Error`
   variant.
3. `rust/crates/host-runtime/src/runtime/capabilities.rs`: the core's call.
4. Every implementation: the providers (`rust/crates/host-provider-*/src/<point>.rs`), host code
   (`apps/host-app-web/src/*.ts`, `apps/host-app-swift/Sources/HostAppSwift/main.swift`,
   `apps/host-app-kotlin/src/main/kotlin/Main.kt`) and test doubles (`rust/crates/host-runtime/src/host_core/tests.rs`).

Generated: the UniFFI export and web glue (by the attribute), the TypeScript interface (`make codegen`), and the Swift
and Kotlin bindings (`make swift`, `make kotlin`).

Verify: `make check && make swift && make kotlin`.

### Add an extension point

By hand:

1. `rust/crates/host-extension/src/<point>.rs`: the trait with `#[extension_point]`, `<Trait>Error` with
   `#[extension_error]`, and its records and enums with `#[extension_type]`.
2. For a value the core follows, the same file: a `<Value>Listener` with `#[extension_listener]`, and
   `fn set_<value>_listener(&self, listener: Arc<dyn <Value>Listener>)` on the trait.
3. `rust/crates/host-extension/src/lib.rs`: `mod <point>;` and `pub use <point>::*;`, which also brings its web glue
   to the crate root.
4. `rust/crates/host-runtime/src/runtime.rs`: its field in `RuntimeServices`, `Option` if optional, plus a
   `Bus<<Value>>` for a followed value.
5. `rust/crates/host-runtime/src/runtime/bus.rs`, for a followed value: `impl <Value>Listener for Bus<<Value>>`.
6. `rust/crates/host-runtime/src/host_core.rs`: a parameter of `HostRuntime::new`, and for a followed value the
   `set_<value>_listener` call.
7. `rust/crates/host-runtime/src/runtime/capabilities.rs`: the core's calls; an optional one answers `Unsupported`
   when absent.
8. `apps/host-app-native/src/lib.rs` and `apps/host-app-web/host-app-web-wasm/src/lib.rs`: one constructor
   parameter, `Arc<dyn Trait>` or `Option<Arc<dyn Trait>>`, passed on to `HostRuntime::new`.
9. The hosts that pass it: `apps/host-app-web/src/main.ts` and `composition.test.ts`, the Swift and Kotlin apps, and
   `rust/crates/host-runtime/src/host_core/tests.rs`.

Generated: as for a method, plus `hostRuntime`'s new parameter, typed `HostExtension.<Trait>`.

Verify: `make check && make swift && make kotlin`.

### Add a provider

By hand:

1. `rust/crates/host-provider-<name>/Cargo.toml`: `host-extension`, plus `uniffi` natively and `wasm-bindgen` on
   wasm32.
2. `src/lib.rs`: `uniffi::setup_scaffolding!()` natively, and the modules below.
3. `src/<point>.rs`: the trait's implementation.
4. `src/native.rs`: `#[uniffi::export] fn <name>() -> Arc<dyn Trait>`.
5. `src/wasm.rs`: `#[wasm_bindgen] fn <name>() -> Js<Trait>`, through `Wasm<Trait>::into_js`.
6. `Cargo.toml` at the root: the workspace dependency.
7. Each host crate that links it: its `Cargo.toml` dependency and `use host_provider_<name> as _;` in `src/lib.rs`.
8. `Makefile`: `-p host-provider-<name>` in `WASM_CRATES`.
9. Host code: hand it to the runtime, or wrap it first.

Generated: its UniFFI bindings (`make swift`, `make kotlin`) and its exports in the web host's `.d.ts` (`make wasm`).

Verify: `make check && make swift && make kotlin`.

### Add a protocol method or service

By hand, for a method:

1. `rust/crates/protocol/src/v01.rs`: its request, response and error payloads.
2. `rust/crates/protocol/src/versioned/<service>.rs`: their envelopes, in `versioned_type!`.
3. `rust/crates/protocol/src/lib.rs`: the `latest` names the core uses.
4. `rust/crates/protocol/src/api/<service>.rs`: the method, with the next free `#[wire(id = N)]`.
5. `rust/crates/host-runtime/src/runtime/capabilities.rs`: its implementation, inside `version(request).exec(...)`.
6. `apps/product/src/main.ts`: the product's call.

For a new service, also:

1. `rust/crates/protocol/src/api/<service>.rs`: the trait with `#[wire_trait(id = N)]`, the id from truapi's table.
2. `rust/crates/protocol/src/api.rs`: `<service>::<Trait>` in `services!`, and its row in the id table.
3. `rust/crates/protocol/src/versioned.rs`: `pub mod <service>;`.
4. `rust/crates/host-runtime/src/runtime/capabilities.rs`: `impl <Trait> for ProductRuntimeHost`.

Generated: the wire pair constants and method table (by `#[wire_trait]`), and the TypeScript client (`make codegen`).

Verify: `make check`.
