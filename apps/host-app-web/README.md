# @sketch/host-app-web

The web host app. It builds its WASM binary from [host-app-web-wasm](host-app-web-wasm/README.md) and implements storage
and confirmation on the page. Then it picks the providers and frames the [product](../product/README.md).

## Look at

1. [`src/main.ts`](src/main.ts): the runtime built from the page's choices.
2. [`src/tracking-wallet.ts`](src/tracking-wallet.ts): host code around the wallet provider.
3. [`src/ts-wallet.ts`](src/ts-wallet.ts): a wallet written only in TypeScript.
4. [`src/host-extensions.ts`](src/host-extensions.ts): storage and confirmation on the page.

## The page

`make dev` serves it on http://localhost:5181. The page lets you pick:

- the wallet, one of:
  - the mock provider wrapped by the tracking wallet;
  - the provider as it is;
  - a TypeScript wallet;
  - none;
- the notifier (the log provider wrapped to show each notice, or none);
- how the user confirmation answers;
- how and when the simulated phone answers sign requests.

The app adds accounts, which the product's subscription sees, and reloads the product, which drops its calls in flight.
A missing wallet or notifier makes the product calls that need it fail as `Unsupported`. Append `?product=<url>` to
frame another product.

## Tests

[`src/composition.test.ts`](src/composition.test.ts) checks the glue through the app's real WASM binary, with the
product's generated client on the far end of a `MessageChannel`:

- a signature through the tracking wallet;
- a declined request;
- the TypeScript wallet;
- the notifier wrapper;
- an account pushed through the tracking wallet and through the TypeScript wallet;
- a stopped subscription that receives no frame;
- storage scoped to each product;
- an extension point object missing a method.

## Next

- [@sketch/host](../../js/packages/host/README.md), which frames the product and connects it.
