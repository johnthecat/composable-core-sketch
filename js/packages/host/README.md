# @sketch/host

What every web host shares: framing a product, and connecting it to the runtime its WASM binary builds. It does not link
the core; the runtime comes from the host's own binary.

## Look at

1. [`src/iframe-host.ts`](src/iframe-host.ts): the host's side of the iframe handshake.
2. [`src/runtime.ts`](src/runtime.ts): a product's port connected to its product runtime.

## Next

- [@sketch/host-app-web](../../../apps/host-app-web/README.md), a host built on it.
