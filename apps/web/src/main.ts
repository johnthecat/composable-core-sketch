import "./style.css";

import {
  createHostRuntime,
  typedWallet,
  WalletErrorException,
  type Account,
} from "@sketch/host";
import initWalletProvider, { MockPhone } from "@sketch/wallet-mock";

import {
  shortHex,
  toHex,
  trackRequests,
  type WalletRequest,
} from "./tracking-wallet.js";

type PhoneDecision = "approve" | "decline" | "unreachable";

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (found === null) throw new Error(`missing #${id}`);
  return found as T;
}

const bootStatus = element<HTMLParagraphElement>("boot-status");
const accountList = element<HTMLUListElement>("accounts");
const accountSelect = element<HTMLSelectElement>("sign-account");
const signForm = element<HTMLFormElement>("sign-form");
const signMessage = element<HTMLInputElement>("sign-message");
const signUnknown = element<HTMLButtonElement>("sign-unknown");
const signResult = element<HTMLParagraphElement>("sign-result");
const signDelay = element<HTMLInputElement>("sign-delay");
const addAccountForm = element<HTMLFormElement>("add-account-form");
const accountName = element<HTMLInputElement>("account-name");
const requestRows = element<HTMLTableSectionElement>("requests");

async function main(): Promise<void> {
  // The provider is its own WASM binary. The host loads it, takes its wallet
  // and wraps it before the core ever sees it.
  await initWalletProvider();
  const phone = new MockPhone(Number(signDelay.value));
  phone.addAccount("Alice");
  phone.addAccount("Bob");
  const wallet = trackRequests(typedWallet(phone.wallet()), renderRequests);

  // `createHostRuntime` loads the core's WASM binary and passes it the wallet.
  const runtime = await createHostRuntime({ wallet });
  bootStatus.textContent =
    "Both WASM binaries loaded: the core from @sketch/host, the wallet from @sketch/wallet-mock.";

  wirePhone(phone);
  void followAccounts(runtime.subscribeAccounts());

  const sign = async (publicKey: Uint8Array) => {
    showResult("pending", "Waiting for the wallet…");
    try {
      const signature = await runtime.signMessage(publicKey, signMessage.value);
      showResult("ok", `Signed: 0x${toHex(signature.bytes)}`);
    } catch (reason) {
      showResult("error", describeError(reason));
    }
  };
  signForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const selected = accountSelect.selectedOptions[0];
    if (selected !== undefined) void sign(hexToBytes(selected.value));
  });
  signUnknown.addEventListener("click", () => {
    void sign(new Uint8Array(32).fill(0xee));
  });
}

function wirePhone(phone: MockPhone): void {
  for (const input of document.querySelectorAll<HTMLInputElement>(
    'input[name="decision"]',
  )) {
    input.addEventListener("change", () => {
      if (input.checked) phone.setDecision(input.value as PhoneDecision);
    });
  }
  signDelay.addEventListener("change", () => {
    phone.setSignDelay(Math.max(0, Number(signDelay.value) || 0));
  });
  addAccountForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const name = accountName.value.trim();
    if (name === "") return;
    phone.addAccount(name);
    accountName.value = "";
  });
}

async function followAccounts(
  updates: AsyncIterable<Account[]>,
): Promise<void> {
  try {
    for await (const accounts of updates) renderAccounts(accounts);
  } catch (reason) {
    accountList.replaceChildren(
      listItem(`Account subscription ended: ${describeError(reason)}`),
    );
  }
}

function renderAccounts(accounts: Account[]): void {
  accountList.replaceChildren(
    ...accounts.map((account) =>
      listItem(`${account.name}  ${shortHex(account.publicKey)}`),
    ),
  );
  const selected = accountSelect.value;
  accountSelect.replaceChildren(
    ...accounts.map((account) => {
      const option = document.createElement("option");
      option.value = toHex(account.publicKey);
      option.textContent = account.name;
      return option;
    }),
  );
  if (selected !== "") accountSelect.value = selected;
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

function describeError(reason: unknown): string {
  if (reason instanceof WalletErrorException) {
    const { error } = reason;
    switch (error.tag) {
      case "Rejected":
        return "The user declined the request.";
      case "UnknownAccount":
        return "The wallet holds no such account; the core did not ask it to sign.";
      case "Unknown":
        return `Failed: ${error.value.reason}`;
    }
  }
  return reason instanceof Error ? reason.message : String(reason);
}

function showResult(kind: "pending" | "ok" | "error", text: string): void {
  signResult.className = `result result-${kind}`;
  signResult.textContent = text;
}

function listItem(text: string): HTMLLIElement {
  const item = document.createElement("li");
  item.textContent = text;
  return item;
}

function cell(content: string | Node): HTMLTableCellElement {
  const td = document.createElement("td");
  td.append(content);
  return td;
}

function hexToBytes(hex: string): Uint8Array {
  return new Uint8Array(
    Array.from({ length: hex.length / 2 }, (_, index) =>
      parseInt(hex.slice(index * 2, index * 2 + 2), 16),
    ),
  );
}

main().catch((reason: unknown) => {
  bootStatus.textContent = `Failed to start: ${describeError(reason)}`;
  bootStatus.className = "result result-error";
  console.error(reason);
});
