// The demo's compositions end to end: the app's real WASM binary, holding the
// core and both providers, driven by the stand-in product's frames. Needs
// `make wasm` and the package builds.

import { beforeAll, describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";

import type { Wallet } from "@sketch/host-extension";

import init, {
  FrameLeg,
  getAccountsFrame,
  hostRuntime,
  logNotifier,
  MockPhone,
  PhoneDecision,
  pushNotificationFrame,
  signRawFrame,
  storageReadFrame,
  storageWriteFrame,
} from "../wasm/host_app_web_wasm.js";
import {
  memoryStorage,
  pageConfirmation,
  showingNotifier,
} from "./host-extensions.js";
import { StandInProduct, type Frame } from "./stand-in-product.js";
import { trackRequests, type WalletRequest } from "./tracking-wallet.js";
import { TypeScriptWallet } from "./ts-wallet.js";

/** A confirmation the user always approves. */
const approveAll = pageConfirmation(
  () => "approve",
  async () => true,
);

beforeAll(async () => {
  await init({
    module_or_path: await readFile(
      new URL("../wasm/host_app_web_wasm_bg.wasm", import.meta.url),
    ),
  });
});

interface Composition {
  product: StandInProduct;
  /** Every frame the core sent to `product`, in order. */
  frames: Frame[];
  /** Another product served by the same runtime. */
  connect(productId: string): StandInProduct;
  phone: MockPhone;
  typeScriptWallet: TypeScriptWallet;
  alice: Uint8Array;
  /** The requests the tracking wrapper saw, oldest first. */
  requests(): Pick<
    WalletRequest,
    "method" | "status" | "summary" | "outcome"
  >[];
  notices: string[];
}

async function compose(options: {
  wallet: "tracked" | "typescript" | "none";
  notifier?: boolean;
}): Promise<Composition> {
  const phone = new MockPhone(0);
  phone.addAccount("alice");
  let requests: readonly WalletRequest[] = [];
  const typeScriptWallet = new TypeScriptWallet();
  await typeScriptWallet.addAccount("carol");
  const wallets: Record<typeof options.wallet, Wallet | undefined> = {
    tracked: trackRequests(phone.wallet(), (latest) => {
      requests = latest;
    }),
    typescript: typeScriptWallet,
    none: undefined,
  };
  const notices: string[] = [];
  const runtime = hostRuntime(
    memoryStorage(),
    approveAll,
    wallets[options.wallet],
    options.notifier
      ? showingNotifier(logNotifier(), (text) => notices.push(text))
      : undefined,
  );
  const [alice] = await phone.wallet().accounts();
  const frames: Frame[] = [];
  return {
    product: new StandInProduct(runtime, "test-product", (frame) =>
      frames.push(frame),
    ),
    frames,
    connect: (productId) => new StandInProduct(runtime, productId),
    phone,
    typeScriptWallet,
    alice: alice?.publicKey ?? new Uint8Array(32),
    requests: () =>
      [...requests].reverse().map(({ method, status, summary, outcome }) => ({
        method,
        status,
        summary,
        outcome,
      })),
    notices,
  };
}

const message = new TextEncoder().encode("hi");

const sign = async (composition: Composition, publicKey = composition.alice) =>
  (
    await composition.product.call((requestId) =>
      signRawFrame(requestId, publicKey, message),
    )
  ).outcome;

/** The account names in a frame the stand-in read back as text. */
const names = (frame: Frame) =>
  [...frame.outcome.matchAll(/name: "([^"]*)"/g)].map((match) => match[1]);

describe("the stand-in product talking to the core through frames", () => {
  test("a confirmed signature: the wrapper sees the account check and the product's message", async () => {
    const composition = await compose({ wallet: "tracked" });

    const outcome = await sign(composition);

    expect(outcome).toStartWith("Ok(HostSignRawResponse");
    expect(composition.requests()).toEqual([
      {
        method: "accounts",
        status: "answered",
        summary: "list accounts",
        outcome: "1 account",
      },
      {
        method: "sign",
        status: "answered",
        // "hi", as the product sent it.
        summary: expect.stringMatching(/^sign 2 bytes with /),
        outcome: expect.stringMatching(/^signature /),
      },
    ]);
  });

  test("a request declined on the phone reaches the product as Rejected", async () => {
    const composition = await compose({ wallet: "tracked" });
    composition.phone.setDecision(PhoneDecision.Decline);

    const outcome = await sign(composition);

    expect(outcome).toBe("Err(Domain(Rejected))");
    expect(composition.requests().at(-1)?.status).toBe("rejected");
  });

  test("a wallet written in TypeScript serves the core like a provider", async () => {
    const composition = await compose({ wallet: "typescript" });

    const accounts = await composition.product.call(getAccountsFrame);
    const [carol] = await composition.typeScriptWallet.accounts();
    const signed = await sign(composition, carol?.publicKey);

    expect({ names: names(accounts), signed }).toEqual({
      names: ["carol"],
      signed: expect.stringMatching(/^Ok\(HostSignRawResponse/),
    });
  });

  test("a notice passes through the host's wrapper to the log provider", async () => {
    const composition = await compose({ wallet: "none", notifier: true });

    const shown = await composition.product.call((requestId) =>
      pushNotificationFrame(requestId, "saved"),
    );
    const empty = await composition.product.call((requestId) =>
      pushNotificationFrame(requestId, " "),
    );

    expect({
      shown: shown.outcome,
      empty: empty.outcome,
      notices: composition.notices,
    }).toEqual({
      shown: "Ok(())",
      empty: expect.stringMatching(/^Err\(Domain\(/),
      notices: ["saved", " "],
    });
  });

  /**
   * Follow the product's account subscription: each list's names, a promise
   * for the next item, and the stop.
   */
  const follow = (composition: Composition) => {
    const lists: string[][] = [];
    let arrived = () => {};
    const stop = composition.product.followAccounts((item) => {
      lists.push(names(item));
      arrived();
    });
    const next = () => new Promise<void>((resolve) => (arrived = resolve));
    return { lists, next, stop };
  };

  // No timing: the core serves the product's frames in order, so the answer to
  // a call made after the stop proves the stop was served; and an item the core
  // sent for the next push would reach the product before the answer to a call
  // made after that push.
  test("a stopped subscription receives nothing more", async () => {
    const composition = await compose({ wallet: "tracked" });
    const { lists, next, stop } = follow(composition);
    await next();

    stop();
    await composition.product.call(getAccountsFrame);
    composition.phone.addAccount("bob");
    await composition.product.call(getAccountsFrame);
    const items = composition.frames.filter(
      (frame) => frame.leg === FrameLeg.Item,
    );

    expect({ lists, items: items.length }).toEqual({
      lists: [["alice"]],
      items: 1,
    });
  });

  // The wallet pushes each change to the listener the tracking wrapper gave it,
  // which records it and passes it to the core's.
  test("an account added on the phone reaches the product's subscription through the tracking wallet", async () => {
    const composition = await compose({ wallet: "tracked" });
    const followed = follow(composition);
    await followed.next();

    const pushed = followed.next();
    composition.phone.addAccount("bob");
    await pushed;

    expect(followed.lists).toEqual([["alice"], ["alice", "bob"]]);
    expect(
      composition
        .requests()
        .filter((request) => request.method === "accountsChanged"),
    ).toEqual([
      {
        method: "accountsChanged",
        status: "pushed",
        summary: "the wallet pushed a change",
        outcome: "2 accounts",
      },
    ]);
  });

  test("an account added to a wallet written in TypeScript reaches the product's subscription", async () => {
    const composition = await compose({ wallet: "typescript" });
    const followed = follow(composition);
    await followed.next();

    const pushed = followed.next();
    await composition.typeScriptWallet.addAccount("erin");
    await pushed;

    expect(followed.lists).toEqual([["carol"], ["carol", "erin"]]);
  });

  test("an extension point object missing a method is refused when the runtime is built", () => {
    const incomplete = {
      accounts: async () => [],
      sign: async () => ({ bytes: new Uint8Array(64) }),
    } as unknown as Wallet;

    expect(() => hostRuntime(memoryStorage(), approveAll, incomplete)).toThrow(
      "the Wallet object has no method setAccountsListener",
    );
  });

  test("storage goes through the host's ProductStorage, scoped to the product", async () => {
    const composition = await compose({ wallet: "none" });
    const other = composition.connect("other-product");

    const written = await composition.product.call((requestId) =>
      storageWriteFrame(requestId, "note", new TextEncoder().encode("a note")),
    );
    const own = await composition.product.call((requestId) =>
      storageReadFrame(requestId, "note"),
    );
    const others = await other.call((requestId) =>
      storageReadFrame(requestId, "note"),
    );

    expect({
      written: written.outcome,
      own: own.outcome,
      others: others.outcome,
    }).toEqual({
      written: "Ok(())",
      // "a note", as bytes.
      own: "Ok(HostLocalStorageReadResponse { value: Some(Bytes([97, 32, 110, 111, 116, 101])) })",
      others: "Ok(HostLocalStorageReadResponse { value: None })",
    });
  });
});
