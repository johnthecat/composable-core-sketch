# Checklists

Each step is one file. "Verify" is the command that proves the change on every platform.

## Add a method to an extension point

By hand:

1. `rust/crates/host-extension/src/<point>.rs`: the method, `async fn(&self, owned values) -> Result<T, <Trait>Error>`.
2. The same file: any new record or enum it names, with `#[extension_type]`; any new failure, as a `<Trait>Error`
   variant.
3. `rust/crates/host-runtime/src/runtime/capabilities.rs`: the core's call.
4. Every implementation:
   - the providers (`rust/crates/host-provider-*/src/<point>.rs`);
   - the web host code (`apps/host-app-web/src/*.ts`);
   - the Swift host code (`apps/host-app-swift/Sources/HostAppSwift/main.swift`);
   - the Kotlin host code (`apps/host-app-kotlin/src/main/kotlin/Main.kt`);
   - test doubles (`rust/crates/host-runtime/src/runtime/tests.rs`).

Generated:

- the UniFFI export and web glue (by the attribute);
- the TypeScript interface (`make codegen`);
- the Swift and Kotlin bindings (`make swift`, `make kotlin`).

Verify: `make check && make swift && make kotlin`.

## Add an extension point

By hand:

1. `rust/crates/host-extension/src/<point>.rs`:
   - the trait with `#[extension_point]`;
   - `<Trait>Error` with `#[extension_error]`;
   - its records and enums with `#[extension_type]`.
2. For a value the core follows, the same file: a `<Value>Listener` with `#[extension_listener]`, and
   `fn set_<value>_listener(&self, listener: Arc<dyn <Value>Listener>)` on the trait.
3. `rust/crates/host-extension/src/lib.rs`: `pub mod <point>;`.
4. `rust/crates/host-runtime/src/runtime.rs`: its field in `HostExtensions`, `Option` if optional. For a followed
   value, also a `Bus<<Value>>` field on `HostRuntime` and the `set_<value>_listener` call in `with_spawner`.
5. For a followed value, `rust/crates/host-runtime/src/runtime/bus.rs`: `impl <Value>Listener for Bus<<Value>>`.
6. `rust/crates/host-runtime/src/runtime/capabilities.rs`: the core's calls; an optional one answers `Unsupported`
   when absent.
7. `apps/host-app-native/src/lib.rs` and `apps/host-app-web/host-app-web-wasm/src/lib.rs`: one constructor
   parameter, `Arc<dyn <point>::Trait>` or `Option<Arc<dyn <point>::Trait>>`, named through its module so the
   constructor finds its web glue there, and its field in the `HostExtensions` it builds.
8. The hosts that pass it:
   - `apps/host-app-web/src/main.ts` and `composition.test.ts`;
   - the Swift and Kotlin apps;
   - `rust/crates/host-runtime/src/runtime/tests.rs`.

Generated: as for a method, plus `hostRuntime`'s new parameter, typed `HostExtension.<Trait>`.

Verify: `make check && make swift && make kotlin`.

## Add a provider

By hand:

1. `rust/crates/host-provider-<name>/Cargo.toml`:
   - `host-extension`;
   - `uniffi`, natively;
   - `wasm-bindgen`, on wasm32.
2. `src/lib.rs`: `uniffi::setup_scaffolding!()` natively, and the modules below.
3. `src/<point>.rs`: the trait's implementation.
4. `src/native.rs`: what hands it to Swift and Kotlin, a `#[uniffi::export] fn <name>() -> Arc<dyn Trait>` (the
   notifier's `log_notifier`), or an exported object whose method returns one, when the provider also needs controls
   (the wallet's `MockPhone`).
5. `src/wasm.rs`: the same for JS, a `#[wasm_bindgen] fn <name>() -> Js<Trait>` or an exported class, through
   `Wasm<Trait>::into_js`.
6. `Cargo.toml` at the root: the workspace dependency.
7. Each host crate that links it: its `Cargo.toml` dependency and `use host_provider_<name> as _;` in `src/lib.rs`.
8. `Makefile`: `-p host-provider-<name>` in `WASM_CRATES`.
9. Host code: hand it to the runtime, or wrap it first.

Generated: its UniFFI bindings (`make swift`, `make kotlin`) and its exports in the web host's `.d.ts` (`make wasm`).

Verify: `make check && make swift && make kotlin`.

## Add a protocol method or service

By hand, for a method:

1. `rust/crates/protocol/src/api/<service>.rs`: its request, response and error types, deriving `Encode` and
   `Decode`, and the method with the next free `#[wire(id = N)]`, taking `&self` and at most one request.
2. `rust/crates/host-runtime/src/runtime/capabilities.rs`: its implementation.
3. `apps/product/src/main.ts`: the product's call.

For a new service, also:

1. `rust/crates/protocol/src/api/<service>.rs`: the trait with `#[wire_trait(id = N)]`, the id from truapi's table.
2. `rust/crates/protocol/src/api.rs`: `<service>::<Trait>` in `services!`, and its row in the id table.
3. `rust/crates/host-runtime/src/runtime/capabilities.rs`: `impl <Trait> for ProductRuntimeHost`.

Generated: the wire pair constants and method table (by `#[wire_trait]`), and the TypeScript client (`make codegen`).

Verify: `make check`.
