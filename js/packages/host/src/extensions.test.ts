import { describe, expect, test } from "bun:test";
import { Vector } from "scale-ts";

import {
  Account,
  SignRequest,
  WalletError,
  WalletErrorException,
  type Wallet,
} from "./generated/extensions.js";
import { rawWallet, typedWallet } from "./generated/extensions-adapter.js";

const bytes = (...parts: (number | number[] | string)[]): Uint8Array =>
  new Uint8Array(
    parts.flatMap((part) =>
      typeof part === "string"
        ? [...new TextEncoder().encode(part)]
        : Array.isArray(part)
          ? part
          : [part],
    ),
  );

// The same vectors as the interface crate's Rust tests: a value must encode to
// the same bytes on both sides of the WASM boundary.
describe("codecs agree with the Rust encoding", () => {
  test("SignRequest", () => {
    const request = {
      publicKey: new Uint8Array(32).fill(1),
      payload: bytes("hi"),
    };
    expect(SignRequest.enc(request)).toEqual(
      bytes(Array(32).fill(1), 0x08, "hi"),
    );
  });

  test("Vec<Account>", () => {
    const accounts = [{ publicKey: new Uint8Array(32).fill(2), name: "bob" }];
    expect(Vector(Account).enc(accounts)).toEqual(
      bytes(0x04, Array(32).fill(2), 0x0c, "bob"),
    );
  });

  test("WalletError variants keep their indices", () => {
    const errors: WalletError[] = [
      { tag: "Rejected" },
      { tag: "UnknownAccount" },
      { tag: "Unknown", value: { reason: "x" } },
    ];
    expect(errors.map((error) => WalletError.enc(error))).toEqual([
      bytes(0x00),
      bytes(0x01),
      bytes(0x02, 0x04, "x"),
    ]);
  });
});

const alice: Account = { publicKey: new Uint8Array(32).fill(1), name: "alice" };

/** Goes through the raw interface and back, as a wallet does between WASM binaries. */
const acrossTheBoundary = (wallet: Wallet): Wallet =>
  typedWallet(rawWallet(wallet));

describe("a wallet adapted to the raw interface and back", () => {
  test("resolves with the values the wallet resolved with", async () => {
    const wallet = acrossTheBoundary({
      accounts: async () => [alice],
      sign: async (request) => ({
        bytes: new Uint8Array(64).fill(request.payload.length),
      }),
      subscribeAccounts: async function* () {},
    });

    expect(await wallet.accounts()).toEqual([alice]);
    expect(
      await wallet.sign({ publicKey: alice.publicKey, payload: bytes("abc") }),
    ).toEqual({ bytes: new Uint8Array(64).fill(3) });
  });

  test("keeps a declared failure, and passes any other one through", async () => {
    const declined = new WalletErrorException({ tag: "Rejected" });
    const crashed = new Error("phone crashed");
    const wallet = acrossTheBoundary({
      accounts: async () => {
        throw crashed;
      },
      sign: async () => {
        throw declined;
      },
      subscribeAccounts: async function* () {},
    });

    const signed: unknown = await wallet
      .sign({ publicKey: alice.publicKey, payload: bytes() })
      .catch((reason: unknown) => reason);
    expect(signed).toBeInstanceOf(WalletErrorException);
    expect((signed as WalletErrorException).error).toEqual(declined.error);
    await expect(wallet.accounts()).rejects.toBe(crashed);
  });

  test("delivers each subscription item, then the error that ends it", async () => {
    const wallet = acrossTheBoundary({
      accounts: async () => [],
      sign: async () => ({ bytes: new Uint8Array(64) }),
      subscribeAccounts: async function* () {
        yield [];
        yield [alice];
        throw new WalletErrorException({
          tag: "Unknown",
          value: { reason: "gone" },
        });
      },
    });

    const received: Account[][] = [];
    let ended: unknown;
    try {
      for await (const accounts of wallet.subscribeAccounts()) {
        received.push(accounts);
      }
    } catch (reason) {
      ended = reason;
    }

    expect(received).toEqual([[], [alice]]);
    expect(ended).toBeInstanceOf(WalletErrorException);
    expect((ended as WalletErrorException).error).toEqual({
      tag: "Unknown",
      value: { reason: "gone" },
    });
  });
});
