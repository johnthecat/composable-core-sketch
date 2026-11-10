# Development

## Prerequisites

- Rust stable with the `wasm32-unknown-unknown` target;
- the dated nightly in `nightly-toolchain` with rustfmt;
- `wasm-pack`;
- Node 20 or newer;
- bun, on `PATH` or from the root `devDependencies`.

For the native apps: Xcode's command line tools (Swift 5.9 or newer), and a JDK 21 or newer on `PATH` or in
`JAVA_HOME`; the Kotlin app's Gradle wrapper fetches Gradle itself. Make is the only command layer; the npm packages
declare no scripts. `NIGHTLY_TOOLCHAIN` overrides the nightly in `nightly-toolchain`, for the Makefile and the
scripts it runs.

## Pipeline

| Target           | What it does                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------------- |
| `make setup`     | `npm install`, the pinned nightly with rustfmt, the wasm32 target                                                |
| `make codegen`   | `scripts/codegen.sh`: rustdoc JSON of `protocol` and `host-extension`, one `tool-codegen` run, prettier          |
| `make wasm`      | `scripts/build-wasm.mjs`: `wasm-pack build --target web` of `host-app-web-wasm` into `apps/host-app-web/wasm/`   |
| `make packages`  | `codegen`, then `tsc -b` for `@sketch/host-extension`                                                          |
| `make build`     | `packages` and `wasm`, then typecheck and `vite build` for `apps/host-app-web`                                 |
| `make dev`       | `packages` and `wasm`, then Vite for the host app on :5181                                                       |
| `make preview`   | `build`, then the production build on the same port                                                            |
| `make fmt`       | `cargo +<nightly> fmt --all` and prettier; `make fmt-check` checks only                                          |
| `make clippy`    | clippy with `-D warnings` on the native workspace, then on wasm32 for the crates in `WASM_CRATES`                |
| `make typecheck` | `packages` and `wasm`, then `tsc` over the package and the app, tests included                                  |
| `make test`      | `packages` and `wasm`, then `cargo test --workspace`, `wasm-pack test --node rust/crates/host-extension`, and `bun test` in `host-extension` and `host-app-web` |
| `make check`     | `codegen`, `fmt-check`, `clippy`, `typecheck`, `test`                                                            |
| `make swift`     | `scripts/native-bindings.sh swift` (builds `host-app-native`, generates bindings), `swift run HostAppSwift`      |
| `make kotlin`    | `scripts/native-bindings.sh kotlin`, `./gradlew run`                                                             |
| `make native`    | both native apps                                                                                                 |
| `make clean`     | removes build and codegen outputs                                                                                |

Nothing `make codegen` writes is committed. The native apps need no codegen: their bindings come from UniFFI.

## Tests

Each package README says what its tests prove. The stop checks run in the web host app's composition test and in
both native apps.

The stop checks wait on no timer. The host serves a product's frames in order, so the answer to a call made after
the stop proves the stop was served (natively, `receive_frame` serves it before it returns). An item for a later push
would then reach the product before the answer to a call made after that push, because the core runs calls in the
order they are woken. So each check fails when stopping is broken, and passes only because no item can be sent.
