// Runs the demo's composition end to end: the core and the mock wallet
// provider as their two real WASM binaries, with the tracking wrapper between
// them. Needs `make wasm` and the @sketch/host build.

import { beforeAll, describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";

import {
  createHostRuntime,
  typedWallet,
  WalletErrorException,
  type HostRuntime,
} from "@sketch/host";
import initWalletProvider, { MockPhone } from "@sketch/wallet-mock";

import {
  shortHex,
  trackRequests,
  type WalletRequest,
} from "./tracking-wallet.js";

const coreWasm = await readFile(
  new URL("../wasm/sketch_core_bg.wasm", import.meta.resolve("@sketch/host")),
);
const providerWasm = await readFile(
  new URL(
    "./sketch_wallet_mock_bg.wasm",
    import.meta.resolve("@sketch/wallet-mock"),
  ),
);

beforeAll(async () => {
  await initWalletProvider({ module_or_path: providerWasm });
});

interface Demo {
  phone: MockPhone;
  runtime: HostRuntime;
  alice: Uint8Array;
  /** Requests after setup, oldest first, without ids and timestamps. */
  log(): Pick<WalletRequest, "method" | "summary" | "status" | "outcome">[];
}

async function demo(signDelayMs = 0): Promise<Demo> {
  const phone = new MockPhone(signDelayMs);
  phone.addAccount("alice");
  let requests: readonly WalletRequest[] = [];
  const wallet = trackRequests(typedWallet(phone.wallet()), (latest) => {
    requests = latest;
  });
  const runtime = await createHostRuntime({ wallet, coreWasm });
  const [alice] = await runtime.accounts();
  if (alice === undefined) throw new Error("the phone holds no account");
  const setupRequests = requests.length;
  return {
    phone,
    runtime,
    alice: alice.publicKey,
    log: () =>
      requests
        .slice(0, requests.length - setupRequests)
        .reverse()
        .map(({ method, summary, status, outcome }) => ({
          method,
          summary,
          status,
          outcome,
        })),
  };
}

async function rejection(promise: Promise<unknown>): Promise<unknown> {
  return promise.then(
    () => {
      throw new Error("expected a rejection");
    },
    (reason: unknown) => reason,
  );
}

describe("host code around the provider sees each request the core sends", () => {
  test("a signed message: the core checks the account, then asks to sign", async () => {
    const { runtime, alice, log } = await demo();

    const signature = await runtime.signMessage(alice, "hi");

    expect(signature.bytes.length).toBe(64);
    expect(log()).toEqual([
      {
        method: "accounts",
        summary: "list accounts",
        status: "answered",
        outcome: "1 account",
      },
      {
        method: "sign",
        // 17 bytes: the core wraps "hi" in `<Bytes>…</Bytes>`.
        summary: `sign 17 bytes with ${shortHex(alice)}`,
        status: "answered",
        outcome: `signature ${shortHex(signature.bytes)}`,
      },
    ]);
  });

  test("a request still waiting for the phone shows as pending", async () => {
    const { runtime, alice, log } = await demo(50);

    const signed = runtime.signMessage(alice, "hi");
    await Bun.sleep(10);
    const whilePending = log().at(-1);
    await signed;

    expect(whilePending).toMatchObject({ method: "sign", status: "pending" });
  });

  test("a request declined on the phone reaches the product as Rejected", async () => {
    const { phone, runtime, alice, log } = await demo();
    phone.setDecision("decline");

    const reason = await rejection(runtime.signMessage(alice, "hi"));

    expect((reason as WalletErrorException).error).toEqual({ tag: "Rejected" });
    expect(log().at(-1)).toMatchObject({
      method: "sign",
      status: "rejected",
      outcome: "declined on the phone",
    });
  });

  test("an undeclared failure crosses both binaries as Unknown", async () => {
    const { phone, runtime, alice, log } = await demo();
    phone.setDecision("unreachable");

    const reason = await rejection(runtime.signMessage(alice, "hi"));

    expect((reason as WalletErrorException).error).toEqual({
      tag: "Unknown",
      value: { reason: "the phone did not answer" },
    });
    expect(log().at(-1)?.status).toBe("failed");
  });

  test("an account the wallet does not hold never reaches sign", async () => {
    const { runtime, log } = await demo();

    const reason = await rejection(
      runtime.signMessage(new Uint8Array(32).fill(0xee), "hi"),
    );

    expect((reason as WalletErrorException).error).toEqual({
      tag: "UnknownAccount",
    });
    expect(log().map((request) => request.method)).toEqual(["accounts"]);
  });

  test("an account added on the phone reaches a product subscription", async () => {
    const { phone, runtime, log } = await demo();
    const updates = runtime.subscribeAccounts()[Symbol.asyncIterator]();

    const first = await updates.next();
    phone.addAccount("bob");
    const second = await updates.next();
    const subscription = log().at(-1);
    await updates.return?.();

    expect(
      [first.value, second.value].map((accounts) => accounts.length),
    ).toEqual([1, 2]);
    expect(subscription).toMatchObject({
      method: "subscribeAccounts",
      status: "live",
      outcome: "update 2: 2 accounts",
    });
  });
});
