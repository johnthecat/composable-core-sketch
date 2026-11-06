# Composable Core sketch

A small, standalone implementation of the
[Composable Core RFC](https://github.com/paritytech/trinity-user-agents/pull/1958) on the web. It uses the same pipeline
as the truapi repository: a trait in an interface crate, rustdoc JSON on a pinned nightly, a codegen binary that emits
the TypeScript interface and the Rust wasm-bindgen wrappers, wasm-pack builds driven from a node script, and SCALE bytes
across every WASM boundary.

Web only: there is no UniFFI, Swift or Kotlin side.

## What it demonstrates

The extension point is `Wallet`, close to the RFC's account holder case: list accounts, sign a request (which waits for
the user on a paired phone), and subscribe to the account list.

- The core (`sketch-core`) depends on the interface crate only and takes the wallet as `Arc<dyn Wallet>`. Its
  product-facing `HostRuntime` checks that the wallet holds the account before asking it to sign, wraps the message in
  `<Bytes>…</Bytes>`, and drops account updates that repeat the previous list.
- The provider (`sketch-wallet-mock`) depends on the interface crate only and simulates a phone: it answers each sign
  request after a delay, and approves, declines or never answers it.
- The core and the provider are two separate WASM binaries. The web app loads both, wraps the provider in host code,
  and passes the wrapper to the core.
- The wrapper (`apps/web/src/tracking-wallet.ts`) implements `Wallet` by forwarding to the provider. It therefore sees
  every request the core sends, and the page shows each one as pending on the phone, answered, declined or failed. The
  core gained no status callback for this ([#1552](https://github.com/paritytech/trinity-user-agents/issues/1552)).

## How the code maps to the RFC

| RFC section                | Here                                                                                                                                                                                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Interface crate            | `rust/crates/sketch-wallet-extension`: the `Wallet` trait, its types, and `WalletError` with `From<String>` for failures the trait does not declare. The core re-exports it as `sketch_core::host::extension`.                                    |
| Provider crate             | `rust/crates/sketch-wallet-mock`: `src/lib.rs` holds only the module setup, `src/wallet.rs` the implementation, `src/wasm.rs` the export, whose `MockPhone.wallet()` returns the generated `WasmWallet`. Enables the interface crate's `export`. |
| Generated web interface    | `js/packages/host/src/generated/extensions.ts` (the TS `Wallet` interface, types and `scale-ts` codecs) and `rust/crates/sketch-wallet-extension/src/wasm.rs` (`ForeignWallet`, and `WasmWallet` behind `export`).                             |
| JS host                    | `@sketch/host` (`js/packages/host`): `createHostRuntime({ wallet })` loads the core's WASM binary. `apps/web/src/main.ts` loads the provider, wraps it and passes it in.                                                                        |
| Core takes `Arc<dyn Trait>` | `rust/crates/sketch-core/src/runtime.rs` (`HostRuntime`), exported to JS by `src/wasm.rs` as a non-generic wasm-bindgen class.                                                                                                                  |

`WasmWallet` is re-exported only under `#[cfg(all(target_arch = "wasm32", feature = "export"))]`. `make wasm` checks
that the core's binary leaves the class out and the provider's carries it.

### How a call crosses the boundaries

Each trait has a raw form, `RawWallet` in `extensions-adapter.ts`, which is what crosses between JS and WASM:

- arguments and results are SCALE-encoded `Uint8Array`s;
- a failure the trait declares rejects with the encoded `WalletError`; in typed TypeScript it is a
  `WalletErrorException`;
- any other rejection or exception is a failure the trait does not declare, and becomes `WalletError::from(reason)`
  in Rust, that is `WalletError::Unknown`;
- a subscription takes `onItem` and `onError` and returns a function that ends it; an error ends it.

`ForeignWallet` calls a `RawWallet` (holding its methods as `js_sys::Function`s behind `send_wrapper`) and
`WasmWallet` implements one. The generated `rawWallet()` and `typedWallet()` convert between `RawWallet` and the typed
`Wallet`, so host code wraps a provider in plain TypeScript:

```ts
await initWalletProvider();
const phone = new MockPhone(1500);
const wallet = trackRequests(typedWallet(phone.wallet()), renderRequests);
const runtime = await createHostRuntime({ wallet });
```

A host that does not wrap the provider could pass `phone.wallet()` to the core as it is; this sketch's
`createHostRuntime` takes a typed `Wallet` only.

## Pipeline

Prerequisites:

- Rust stable with the `wasm32-unknown-unknown` target, and the dated nightly in `nightly-toolchain` with `rustfmt`
  (`make setup` installs both).
- `wasm-pack` (`cargo install wasm-pack`).
- Node 20 or newer and npm.
- Bun. A `bun` on `PATH` is used when present; otherwise the one the root `devDependencies` install.

| Target                 | What it does                                                                                                                                                                                                                       |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `make setup`           | `npm install`, the pinned nightly with `rustfmt`, and the wasm32 target.                                                                                                                                                           |
| `make codegen`         | `scripts/codegen.sh`: `cargo +<nightly> doc -p sketch-wallet-extension` with `RUSTDOCFLAGS="-Z unstable-options --output-format json"`, `cargo run -p sketch-codegen` on `target/doc/sketch_wallet_extension.json`, then rustfmt on the generated Rust and prettier on the generated TypeScript. |
| `make check-generated` | Runs the codegen and fails when a committed generated file changed.                                                                                                                                                                |
| `make wasm`            | `scripts/build-wasm.mjs`: `wasm-pack build --target web` for the core into `js/packages/host/wasm/` and for the provider into `js/packages/wallet-mock/wasm/`. `SKETCH_WASM_PROFILE=dev` skips optimisation.                       |
| `make host`            | `make wasm`, then `tsc -b` for `@sketch/host`.                                                                                                                                                                                     |
| `make build`           | `make host`, then typechecks the web app and runs `vite build` into `apps/web/dist/`.                                                                                                                                             |
| `make dev`             | `make host`, then serves the web app with Vite on <http://localhost:5181>.                                                                                                                                                        |
| `make fmt`             | `cargo +<nightly> fmt --all` and prettier.                                                                                                                                                                                         |
| `make clippy`          | `cargo clippy -D warnings` on the native workspace, then on wasm32 for the core and the provider, each on its own so the `export` feature resolves as it does in their binaries.                                                   |
| `make typecheck`       | `tsc` over both TypeScript packages, tests included.                                                                                                                                                                              |
| `make test`            | `cargo test --workspace`, then `bun test` in `@sketch/host` and in the web app.                                                                                                                                                   |
| `make check`           | `fmt-check`, `clippy`, `check-generated`, `typecheck` and `test`.                                                                                                                                                                  |

The tests that encode why things matter:

- `sketch-codegen` snapshots its three outputs for a `Wallet` model (`src/tests/*.snap`; rewrite with
  `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1`).
- The interface crate and `@sketch/host` assert the same SCALE vectors, so a value encodes to the same bytes on both
  sides.
- `apps/web/src/tracking-wallet.test.ts` loads both real WASM binaries in bun and checks what the host wrapper sees:
  the core's account check, a request pending on the phone, a declined and an unanswered request, an unknown account
  that never reaches `sign`, and an account added on the phone reaching a product subscription.

### Generated files are committed

`src/wasm.rs` of the interface crate and `js/packages/host/src/generated/*.ts` are committed, so a reviewer can read
them without running the pipeline, and Rust builds need no nightly. truapi ignores both kinds of generated output
instead. `make check-generated` keeps the committed copies current. The hand-written helpers they call are
`rust/crates/sketch-wallet-extension/src/wasm/support.rs` and `js/packages/host/src/adapter-support.ts`.

## Layout

```
rust/crates/
  sketch-wallet-extension/  interface crate: the Wallet trait and its types; src/wasm.rs is generated,
                            src/wasm/support.rs holds the helpers it calls; `export` feature for providers
  sketch-wallet-mock/       provider crate: in-memory wallet simulating a phone; its own WASM binary
  sketch-core/              core: HostRuntime over Arc<dyn Wallet>; its own WASM binary
  sketch-codegen/           rustdoc JSON -> TypeScript interface and adapters, and the interface crate's wasm.rs
js/packages/
  host/                     @sketch/host: createHostRuntime, the generated Wallet interface and adapters;
                            the core's wasm-pack output goes to wasm/
  wallet-mock/              @sketch/wallet-mock: the provider's wasm-pack output, in wasm/
apps/web/                   Vite app: loads both binaries, wraps the provider, shows every request
scripts/
  codegen.sh                the codegen pipeline
  build-wasm.mjs            builds both WASM binaries
nightly-toolchain           the dated nightly for rustfmt and rustdoc JSON
```

The mock wallet's keys and signatures are SHA-2 hashes standing in for real ones. They are not cryptographic keys or
signatures.
