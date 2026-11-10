# @sketch/product

The product: a page the host frames from its own origin. It reaches the core only through
[@sketch/protocol-client](../../js/packages/protocol-client/README.md).

## Look at

1. [`src/main.ts`](src/main.ts): the product calls `client.signing.signRaw(...)` and the other methods.

## Next

- [@sketch/protocol-client](../../js/packages/protocol-client/README.md), where a call becomes a frame.
