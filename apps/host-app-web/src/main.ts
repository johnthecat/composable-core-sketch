// The host page: builds the core over the extension points it implements and
// the providers it picks, and frames the product.

import "./style.css";

import {
  connectProductPort,
  createIframeHost,
  type IframeHost,
  type Notifier,
  type UserConfirmationReview,
  type Wallet,
} from "@sketch/host";

import init, {
  hostRuntime,
  logNotifier,
  MockPhone,
  PhoneDecision,
} from "../wasm/host_app_web_wasm.js";
import {
  memoryStorage,
  pageConfirmation,
  type ConfirmationMode,
} from "./host-extensions.js";
import {
  shortHex,
  trackRequests,
  type WalletRequest,
} from "./tracking-wallet.js";
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

/**
 * The product app, on its own origin. `?product=<url>` frames another one,
 * the way a host loads whichever product the user opens.
 */
const productUrl =
  new URLSearchParams(window.location.search).get("product") ??
  "http://localhost:5182/";

const bootStatus = element<HTMLParagraphElement>("boot-status");
const productFrame = element<HTMLDivElement>("product-frame");
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
  const wallet = (choice: WalletChoice): Wallet | undefined => {
    const current = composition;
    switch (choice) {
      case "tracked":
        return trackRequests(phone.wallet(), (requests) => {
          if (current === composition) renderRequests(requests);
        });
      case "provider":
        return phone.wallet();
      case "typescript":
        return typeScriptWallet;
      case "none":
        return undefined;
    }
  };
  // Host code around the log provider, as in the RFC's example: it shows each
  // notice on the page, then lets the provider log it.
  const notifier = (choice: NotifierChoice): Notifier | undefined => {
    if (choice === "none") return undefined;
    const inner = logNotifier();
    return {
      async notify(text) {
        notice.textContent = `Notice: ${text}`;
        await inner.notify(text);
      },
    };
  };

  let framed: { host: IframeHost; disconnect: () => void } | undefined;
  const compose = () => {
    composition += 1;
    framed?.disconnect();
    framed?.host.dispose();
    renderRequests([]);
    notice.textContent = "";
    const runtime = hostRuntime(
      storage,
      confirmation,
      wallet(checked<WalletChoice>("wallet")),
      notifier(checked<NotifierChoice>("notifier")),
    );
    let disconnect = () => {};
    const host = createIframeHost({
      iframeUrl: productUrl,
      container: productFrame,
      onPort: (port) => {
        disconnect = connectProductPort(
          runtime,
          { productId: "sketch-product" },
          port,
        );
      },
    });
    framed = { host, disconnect };
    bootStatus.textContent = `Composed: wallet ${checked("wallet")}, notifier ${checked("notifier")}.`;
    bootStatus.dataset.composition = `${checked("wallet")}/${checked("notifier")}`;
  };

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
