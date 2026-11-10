// The host page: builds the core over the extension points it implements and
// the providers it picks, and drives it with the stand-in product.

import "./style.css";

import type {
  Notifier,
  UserConfirmationReview,
  Wallet,
} from "@sketch/host-extension";

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
  type HostRuntime,
} from "../wasm/host_app_web_wasm.js";
import {
  memoryStorage,
  pageConfirmation,
  showingNotifier,
  type ConfirmationMode,
} from "./host-extensions.js";
import {
  shortHex,
  trackRequests,
  type TrackingWallet,
  type WalletRequest,
} from "./tracking-wallet.js";
import { StandInProduct, type Frame } from "./stand-in-product.js";
import { TypeScriptWallet } from "./ts-wallet.js";

type WalletChoice = "tracked" | "provider" | "typescript" | "none";
type NotifierChoice = "wrapped" | "none";

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (found === null) throw new Error(`missing #${id}`);
  return found as T;
}

function checked<T extends string>(name: string): T {
  const input = document.querySelector<HTMLInputElement>(
    `input[name="${name}"]:checked`,
  );
  if (input === null) throw new Error(`no ${name} is selected`);
  return input.value as T;
}

const bootStatus = element<HTMLParagraphElement>("boot-status");
const frameRows = element<HTMLTableSectionElement>("frames");
const productText = element<HTMLInputElement>("product-text");
const followButton = element<HTMLButtonElement>("product-follow");
const requestRows = element<HTMLTableSectionElement>("requests");
const notice = element<HTMLParagraphElement>("notice");

async function main(): Promise<void> {
  // One WASM binary holds the core and the providers this app links.
  await init();
  const phone = new MockPhone(
    Number(element<HTMLInputElement>("sign-delay").value),
  );
  phone.addAccount("Alice");
  phone.addAccount("Bob");
  const typeScriptWallet = new TypeScriptWallet();
  await typeScriptWallet.addAccount("Carol");
  const storage = memoryStorage();
  const confirmation = pageConfirmation(
    () => checked<ConfirmationMode>("confirmation"),
    askUser,
  );

  // Each composition gets its own tracking wallet. One from a dropped runtime
  // may still settle a call; only the current one draws.
  let composition = 0;
  let tracking: TrackingWallet | undefined;
  let currentWallet: Wallet | undefined;
  const pickWallet = (choice: WalletChoice): Wallet | undefined => {
    const current = composition;
    tracking = undefined;
    switch (choice) {
      case "tracked":
        tracking = trackRequests(phone.wallet(), (requests) => {
          if (current === composition) renderRequests(requests);
        });
        return tracking;
      case "provider":
        return phone.wallet();
      case "typescript":
        return typeScriptWallet;
      case "none":
        return undefined;
    }
  };
  const notifier = (choice: NotifierChoice): Notifier | undefined =>
    choice === "none"
      ? undefined
      : showingNotifier(logNotifier(), (text) => {
          notice.textContent = `Notice: ${text}`;
        });

  // Connecting the product gives it a product runtime; disconnecting frees
  // it, which drops the calls it still has in flight.
  let runtime: HostRuntime | undefined;
  let product: StandInProduct | undefined;
  let stopFollowing: (() => void) | undefined;
  const labels = new Map<string, string>();
  const logFrame = (frame: Frame) => {
    const label = labels.get(frame.requestId) ?? "accountsSubscribe";
    if (frame.leg !== FrameLeg.Item) labels.delete(frame.requestId);
    const row = document.createElement("tr");
    row.append(
      ...[frame.requestId, label, FrameLeg[frame.leg], frame.outcome].map(cell),
    );
    frameRows.prepend(row);
    while (frameRows.rows.length > 50) frameRows.lastElementChild?.remove();
  };
  const connectProduct = (current: HostRuntime) => {
    // Request ids restart with each connection.
    labels.clear();
    product = new StandInProduct(current, "sketch-product", logFrame);
  };
  const disconnectProduct = () => {
    stopFollowing = undefined;
    followButton.textContent = "Follow accounts";
    product?.disconnect();
    product = undefined;
    tracking?.callsDropped();
  };
  const compose = () => {
    disconnectProduct();
    runtime?.free();
    composition += 1;
    renderRequests([]);
    notice.textContent = "";
    const walletChoice = checked<WalletChoice>("wallet");
    const notifierChoice = checked<NotifierChoice>("notifier");
    currentWallet = pickWallet(walletChoice);
    runtime = hostRuntime(
      storage,
      confirmation,
      currentWallet,
      notifier(notifierChoice),
    );
    connectProduct(runtime);
    bootStatus.textContent = `Composed: wallet ${walletChoice}, notifier ${notifierChoice}.`;
  };
  element("reload-product").addEventListener("click", () => {
    if (runtime === undefined) return;
    disconnectProduct();
    connectProduct(runtime);
  });

  // The product's buttons. Each sends the frame a product would send.
  const send = (label: string, build: (requestId: string) => Uint8Array) => {
    void product?.call((requestId) => {
      labels.set(requestId, label);
      return build(requestId);
    });
  };
  const text = () => new TextEncoder().encode(productText.value);
  element("product-sign").addEventListener("click", async () => {
    // The stand-in signs with the wallet's first account; without a wallet
    // the core answers Unsupported whatever the key.
    const [first] = (await currentWallet?.accounts()) ?? [];
    const publicKey = first?.publicKey ?? new Uint8Array(32);
    send("signRaw", (requestId) => signRawFrame(requestId, publicKey, text()));
  });
  element("product-accounts").addEventListener("click", () =>
    send("getAccounts", getAccountsFrame),
  );
  followButton.addEventListener("click", () => {
    if (product === undefined) return;
    if (stopFollowing === undefined) {
      stopFollowing = product.followAccounts();
      followButton.textContent = "Stop following accounts";
      return;
    }
    stopFollowing();
    stopFollowing = undefined;
    followButton.textContent = "Follow accounts";
    // The core serves frames in order, so once this call is answered it has
    // served the stop: no item of the subscription can follow.
    send("getAccounts, after the stop", getAccountsFrame);
  });
  element("product-write").addEventListener("click", () =>
    send("localStorage.write", (requestId) =>
      storageWriteFrame(requestId, "note", text()),
    ),
  );
  element("product-read").addEventListener("click", () =>
    send("localStorage.read", (requestId) =>
      storageReadFrame(requestId, "note"),
    ),
  );
  element("product-notify").addEventListener("click", () =>
    send("pushNotification", (requestId) =>
      pushNotificationFrame(requestId, productText.value),
    ),
  );

  for (const input of document.querySelectorAll<HTMLInputElement>(
    'input[name="wallet"], input[name="notifier"]',
  )) {
    input.addEventListener("change", compose);
  }
  for (const input of document.querySelectorAll<HTMLInputElement>(
    'input[name="decision"]',
  )) {
    input.addEventListener("change", () =>
      phone.setDecision(
        PhoneDecision[input.value as keyof typeof PhoneDecision],
      ),
    );
  }
  element<HTMLInputElement>("sign-delay").addEventListener(
    "change",
    (event) => {
      phone.setSignDelay(
        Math.max(0, Number((event.target as HTMLInputElement).value) || 0),
      );
    },
  );
  element<HTMLFormElement>("add-account-form").addEventListener(
    "submit",
    (event) => {
      event.preventDefault();
      const name = element<HTMLInputElement>("account-name");
      if (name.value.trim() === "") return;
      if (checked<WalletChoice>("wallet") === "typescript") {
        void typeScriptWallet.addAccount(name.value.trim());
      } else {
        phone.addAccount(name.value.trim());
      }
      name.value = "";
    },
  );

  compose();
}

/** Show `review` on the page until the user approves or denies it. */
function askUser(review: UserConfirmationReview): Promise<boolean> {
  const panel = element<HTMLDivElement>("confirmation");
  const { productId, publicKey, payload } = review.value;
  element("confirmation-text").textContent =
    `${productId} asks to sign "${new TextDecoder().decode(payload)}" with ${shortHex(publicKey)}.`;
  panel.hidden = false;
  return new Promise((resolve) => {
    const answer = (approved: boolean) => () => {
      panel.hidden = true;
      resolve(approved);
    };
    element("confirm-approve").onclick = answer(true);
    element("confirm-deny").onclick = answer(false);
  });
}

function renderRequests(requests: readonly WalletRequest[]): void {
  requestRows.replaceChildren(
    ...requests.map((request) => {
      const row = document.createElement("tr");
      const status = document.createElement("span");
      status.className = `status status-${request.status}`;
      status.textContent =
        request.status === "pending" && request.method === "sign"
          ? "waiting for the phone"
          : request.status;
      const time =
        request.settledAt === undefined
          ? ""
          : `${request.settledAt - request.startedAt} ms`;
      row.append(
        cell(String(request.id)),
        cell(request.method),
        cell(request.summary),
        cell(status),
        cell(request.outcome ?? ""),
        cell(time),
      );
      return row;
    }),
  );
}

function cell(content: string | Node): HTMLTableCellElement {
  const td = document.createElement("td");
  td.append(content);
  return td;
}

main().catch((reason: unknown) => {
  bootStatus.textContent = `Failed to start: ${String(reason)}`;
  console.error(reason);
});
