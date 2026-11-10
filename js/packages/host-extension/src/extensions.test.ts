import { describe, expect, test } from "bun:test";

import { ProductStorageException, WalletException } from "./index.js";

// The WASM glue cannot construct these classes: a Rust provider's declared
// error rejects as a plain `Error` carrying the class name and the error.
// Host code still catches it with `instanceof`.
describe("declared error exceptions", () => {
  test("accept a rejection from the WASM glue by name", () => {
    const rejection = Object.assign(new Error("the user declined"), {
      name: "WalletException",
      error: { tag: "Rejected" },
    });

    expect(rejection instanceof WalletException).toBe(true);
    expect(rejection instanceof ProductStorageException).toBe(false);
  });

  test("reject an error that carries no declared error", () => {
    const thrown = new Error("network down");
    thrown.name = "WalletException";

    expect(thrown instanceof WalletException).toBe(false);
  });

  test("accept their own instances", () => {
    const thrown = new ProductStorageException({ tag: "Full" });

    expect(thrown instanceof ProductStorageException).toBe(true);
    expect(thrown.message).toBe("ProductStorageError.Full");
  });
});
