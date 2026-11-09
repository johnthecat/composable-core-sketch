// The demo's compositions end to end: the app's real WASM binary, holding the
// core and both providers, and the product's generated client on the far end
// of a MessageChannel. Needs `make wasm` and the package builds.

import { beforeAll, describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";

import {
  createClient,
  createMessagePortProvider,
  createTransport,
  type TrUApiClient,
} from "@sketch/protocol-client";
import { connectProductPort, type Notifier, type Wallet } from "@sketch/host";

import init, {
  hostRuntime,
  logNotifier,
  MockPhone,
  PhoneDecision,
} from "../wasm/host_app_web_wasm.js";
import { memoryStorage, pageConfirmation } from "./host-extensions.js";
import { trackRequests, type WalletRequest } from "./tracking-wallet.js";
import { TypeScriptWallet } from "./ts-wallet.js";

beforeAll(async () => {
  await init({
    module_or_path: await readFile(
      new URL("../wasm/host_app_web_wasm_bg.wasm", import.meta.url),
    ),
  });
});

interface Composition {
  client: TrUApiClient;
  /** A client of another product served by the same runtime. */
  connect(productId: string): TrUApiClient;
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
  const inner = logNotifier();
  const notifier: Notifier = {
    async notify(text) {
      notices.push(text);
      await inner.notify(text);
    },
  };
  const runtime = hostRuntime(
    memoryStorage(),
    pageConfirmation(
      () => "approve",
      async () => true,
    ),
    wallets[options.wallet],
    options.notifier ? notifier : undefined,
  );
  const connect = (productId: string) => {
    const channel = new MessageChannel();
    connectProductPort(runtime, { productId }, channel.port1);
    return createClient(
      createTransport(createMessagePortProvider(channel.port2)),
    );
  };
  const [alice] = await phone.wallet().accounts();
  return {
    client: connect("test-product"),
    connect,
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

const sign = (composition: Composition, publicKey = composition.alice) =>
  composition.client.signing.signRaw({
    publicKey,
    payload: new TextEncoder().encode("hi"),
  });

describe("a product talking to the core through frames", () => {
  test("a confirmed signature: the wrapper sees the account check and the wrapped message", async () => {
    const composition = await compose({ wallet: "tracked" });

    const result = await sign(composition);

    expect(result.isOk() && result.value.signature.length).toBe(64);
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
        // 17 bytes: the core wraps "hi" in `<Bytes>…</Bytes>`.
        summary: expect.stringMatching(/^sign 17 bytes with /),
        outcome: expect.stringMatching(/^signature /),
      },
    ]);
  });

  test("a request declined on the phone reaches the product as Rejected", async () => {
    const composition = await compose({ wallet: "tracked" });
    composition.phone.setDecision(PhoneDecision.Decline);

    const result = await sign(composition);

    expect(result.isErr() && result.error).toEqual({
      tag: "Domain",
      value: { tag: "V1", value: { tag: "Rejected", value: undefined } },
    });
    expect(composition.requests().at(-1)?.status).toBe("rejected");
  });

  test("a wallet written in TypeScript serves the core like a provider", async () => {
    const composition = await compose({ wallet: "typescript" });

    const accounts = await composition.client.account.getAccounts();
    const carol = accounts.isOk() ? accounts.value.accounts[0] : undefined;
    const signed = carol && (await sign(composition, carol.publicKey));

    expect({ name: carol?.name, signed: signed?.isOk() }).toEqual({
      name: "carol",
      signed: true,
    });
  });

  test("a notice passes through the host's wrapper to the log provider", async () => {
    const composition = await compose({ wallet: "none", notifier: true });

    const shown = await composition.client.notifications.pushNotification({
      text: "saved",
    });
    const empty = await composition.client.notifications.pushNotification({
      text: " ",
    });

    expect({
      shown: shown.isOk(),
      empty: empty.isErr() && empty.error.tag,
      notices: composition.notices,
    }).toEqual({
      shown: true,
      empty: "Domain",
      notices: ["saved", " "],
    });
  });

  /** Follow the product's account subscription, keeping each list's names. */
  const follow = (composition: Composition) => {
    const lists: string[][] = [];
    composition.client.account.accountsSubscribe().subscribe({
      next: (item) => lists.push(item.accounts.map((account) => account.name)),
    });
    return lists;
  };

  // The wallet pushes each change to the listener the tracking wrapper gave it,
  // which records it and passes it to the core's.
  test("an account added on the phone reaches the product's subscription through the tracking wallet", async () => {
    const composition = await compose({ wallet: "tracked" });
    const followed = follow(composition);
    await Bun.sleep(20);

    composition.phone.addAccount("bob");
    await Bun.sleep(20);

    expect(followed).toEqual([["alice"], ["alice", "bob"]]);
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
    await Bun.sleep(20);

    await composition.typeScriptWallet.addAccount("erin");
    await Bun.sleep(20);

    expect(followed).toEqual([["carol"], ["carol", "erin"]]);
  });

  test("an extension point object missing a method is refused when the runtime is built", () => {
    const incomplete = {
      accounts: async () => [],
      sign: async () => ({ bytes: new Uint8Array(64) }),
    } as unknown as Wallet;

    expect(() =>
      hostRuntime(
        memoryStorage(),
        pageConfirmation(
          () => "approve",
          async () => true,
        ),
        incomplete,
      ),
    ).toThrow("the Wallet object has no method setAccountsListener");
  });

  test("storage goes through the host's ProductStorage, scoped to the product", async () => {
    const composition = await compose({ wallet: "none" });
    const other = composition.connect("other-product");
    const value = new TextEncoder().encode("a note");

    const written = await composition.client.localStorage.write({
      key: "note",
      value,
    });
    const own = await composition.client.localStorage.read({ key: "note" });
    const others = await other.localStorage.read({ key: "note" });

    expect({
      written: written.isOk(),
      own: own.isOk() && own.value.value,
      others: others.isOk() && others.value.value,
    }).toEqual({ written: true, own: value, others: undefined });
  });
});
