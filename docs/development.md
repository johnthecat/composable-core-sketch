# Development

## Prerequisites

Rust stable with the `wasm32-unknown-unknown` target; the dated nightly in `nightly-toolchain` with rustfmt;
`wasm-pack`; Node 20 or newer; bun, on `PATH` or from the root `devDependencies`. For the native apps: Xcode's
command line tools (Swift 5.9 or newer), and a JDK 21 or newer on `PATH` or in `JAVA_HOME`; the Kotlin app's Gradle
wrapper fetches Gradle itself. Make is the only command layer; the npm packages declare no scripts.
`NIGHTLY_TOOLCHAIN` overrides the nightly in `nightly-toolchain`, for the Makefile and the scripts it runs.

## Pipeline

| Target           | What it does                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------------- |
| `make setup`     | `npm install`, the pinned nightly with rustfmt, the wasm32 target                                                |
| `make codegen`   | `scripts/codegen.sh`: rustdoc JSON of `protocol` and `host-extension`, one `tool-codegen` run, prettier          |
| `make wasm`      | `scripts/build-wasm.mjs`: `wasm-pack build --target web` of `host-app-web-wasm` into `apps/host-app-web/wasm/`   |
| `make packages`  | `codegen`, then `tsc -b` for `@sketch/protocol-client`, `@sketch/host-extension` and `@sketch/host`              |
| `make build`     | `packages` and `wasm`, then typecheck and `vite build` for `apps/product` and `apps/host-app-web`                |
| `make dev`       | `packages` and `wasm`, then Vite for the host app on :5181 and the product on :5182                              |
| `make preview`   | `build`, then the production builds on the same ports                                                            |
| `make fmt`       | `cargo +<nightly> fmt --all` and prettier; `make fmt-check` checks only                                          |
| `make clippy`    | clippy with `-D warnings` on the native workspace, then on wasm32 for the crates in `WASM_CRATES`                |
| `make typecheck` | `packages` and `wasm`, then `tsc` over the packages and both apps, tests included                                |
| `make test`      | `packages` and `wasm`, then `cargo test --workspace`, `wasm-pack test --node rust/crates/host-extension`, and `bun test` in `protocol-client`, `host-extension` and `host-app-web` |
| `make check`     | `codegen`, `fmt-check`, `clippy`, `typecheck`, `test`                                                            |
| `make swift`     | `scripts/native-bindings.sh swift` (builds `host-app-native`, generates bindings), `swift run HostAppSwift`      |
| `make kotlin`    | `scripts/native-bindings.sh kotlin`, `./gradlew run`                                                             |
| `make native`    | both native apps                                                                                                 |
| `make clean`     | removes build and codegen outputs                                                                                |

Nothing `make codegen` writes is committed. The native apps need no codegen: their bindings come from UniFFI.

## Tests

- `tool-codegen`: a snapshot of `extensions.ts` for the RFC's `Notifier` and a listener naming a protocol type
  (`src/ts/extensions/tests/*.snap`; rewrite with `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1`), and unit tests of the rustdoc
  reader (async methods, wire tags, newtypes, public modules) and the TS type emission.
- `protocol`: the frame layout, which matches the bytes the TS client's test checks.
- `host-runtime`: the core driven by frames. A confirmed signature reaches the wallet with the product's payload; a
  refused confirmation is `Denied` before the wallet signs; an unknown account fails before the user is asked; no
  wallet is `Unsupported`; a notifier refusal reaches the product; dropping the product runtime drops a call still in
  flight; a subscription sends the current list and each pushed change, and nothing after it stops.
- `host-extension`: an undeclared failure becomes `Unknown`, and a declared one keeps its message. Under Node, each
  byte type round-trips through the web glue's conversion as a `Uint8Array`, a value that does not convert is an
  error, and only a rejection named for the trait is declared.
- `host-provider-notifier-log`: a notice becomes one log line, and an empty one is refused.
- `host-provider-wallet-mock`: the phone answers each request as it was last told, and never answers when
  unreachable; a signature binds the payload to the account; signing with an account it does not hold fails; an added
  account is pushed to the last listener given.
- `@sketch/host-extension`: a rejection from the WASM glue is accepted by its generated exception class.
- `@sketch/protocol-client`: the frame bytes, and the generated client over a loopback transport.
- `apps/host-app-web/src/composition.test.ts`: the glue, through the app's real WASM binary with the product's generated
  client on the far end of a `MessageChannel`: a signature through the tracking wallet, a declined request, the
  TypeScript wallet, the notifier wrapper, an account pushed through the tracking wallet and through the TypeScript
  wallet, a stopped subscription that receives no frame, storage scoped to each product, and an extension point
  object missing a method.
- The Swift and Kotlin apps check two things as they run, and exit with a failure if either fails: that the product
  received the current account list and then the pushed change, and that a stopped subscription receives no item
  when another account is added.

The stop checks wait on no timer. The host serves a product's frames in order, so the answer to a call made after
the stop proves the stop was served (natively, `receive_frame` serves it before it returns). An item for a later push
would then reach the product before the answer to a call made after that push, because the core runs calls in the
order they are woken. So each check fails when stopping is broken, and passes only because no item can be sent.
