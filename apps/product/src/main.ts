// The product: framed by the host page, it reaches the core only through the
// generated client, whose calls become protocol frames on a MessagePort.

import "./style.css";

import {
  connectIframeHost,
  type CallErrorValue,
  type ProductAccount,
  type TrUApiClient,
} from "@sketch/protocol-client";
import type { ResultAsync } from "neverthrow";

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (found === null) throw new Error(`missing #${id}`);
  return found as T;
}

const connection = element<HTMLParagraphElement>("connection");
const accountList = element<HTMLUListElement>("accounts");
const accountSelect = element<HTMLSelectElement>("sign-account");
const message = element<HTMLInputElement>("sign-message");
const note = element<HTMLInputElement>("note");
const results = element<HTMLOListElement>("results");

/** Show one call's outcome; the style colours a line by its `outcome`. */
function report(method: string, outcome: string, detail = ""): void {
  const item = document.createElement("li");
  item.dataset.outcome = outcome;
  item.textContent = `${method}: ${outcome}${detail === "" ? "" : ` (${detail})`}`;
  results.prepend(item);
}

/** `Domain:Rejected`, `Denied`, `Unsupported`, ... */
function describeError(error: CallErrorValue<unknown>): [string, string] {
  if (error.tag !== "Domain") {
    const reason =
      error.value && "reason" in error.value ? error.value.reason : "";
    return [error.tag, reason];
  }
  const payload = error.value;
  if (typeof payload === "object" && payload !== null && "tag" in payload) {
    const tagged = payload as { tag: string; value?: { reason?: string } };
    return [`Domain:${tagged.tag}`, tagged.value?.reason ?? ""];
  }
  const generic = payload as { reason?: string };
  return ["Domain", generic.reason ?? ""];
}

async function call<T, D>(
  method: string,
  result: ResultAsync<T, CallErrorValue<D>>,
  describe: (value: T) => string,
): Promise<void> {
  const outcome = await result;
  if (outcome.isOk()) {
    report(method, "ok", describe(outcome.value));
  } else {
    const [tag, detail] = describeError(outcome.error);
    report(method, tag, detail);
  }
}

function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}

function renderAccounts(accounts: ProductAccount[]): void {
  accountList.replaceChildren(
    ...accounts.map((account) => {
      const item = document.createElement("li");
      item.textContent = `${account.name} 0x${hex(account.publicKey).slice(0, 8)}…`;
      return item;
    }),
  );
  const selected = accountSelect.value;
  accountSelect.replaceChildren(
    ...accounts.map((account) => {
      const option = document.createElement("option");
      option.value = hex(account.publicKey);
      option.textContent = account.name;
      return option;
    }),
  );
  if (selected !== "") accountSelect.value = selected;
}

function fromHex(value: string): Uint8Array {
  return new Uint8Array(
    Array.from({ length: value.length / 2 }, (_, index) =>
      parseInt(value.slice(index * 2, index * 2 + 2), 16),
    ),
  );
}

function wire(client: TrUApiClient): void {
  const follow = () =>
    client.account.accountsSubscribe().subscribe({
      next: (item) => {
        renderAccounts(item.accounts);
        report("accountsSubscribe", "item", `${item.accounts.length} accounts`);
      },
      error: (error) => {
        report("accountsSubscribe", ...describeError(error.reason));
      },
    });
  let following: { unsubscribe(): void } | undefined = follow();
  const followButton = element<HTMLButtonElement>("follow-accounts");
  followButton.addEventListener("click", () => {
    if (following === undefined) {
      following = follow();
      followButton.textContent = "Stop following accounts";
    } else {
      following.unsubscribe();
      following = undefined;
      followButton.textContent = "Follow accounts";
      // The host serves frames in order, so once a later call is answered it
      // has served the stop: no item of this subscription can follow.
      void client.account.getAccounts().then(() => {
        report("accountsSubscribe", "stopped", "the host served the stop");
      });
    }
  });

  const sign = (publicKey: Uint8Array) =>
    call(
      "signRaw",
      client.signing.signRaw({
        publicKey,
        payload: new TextEncoder().encode(message.value),
      }),
      (response) => `0x${hex(response.signature).slice(0, 16)}…`,
    );
  element<HTMLFormElement>("sign-form").addEventListener("submit", (event) => {
    event.preventDefault();
    if (accountSelect.value !== "") void sign(fromHex(accountSelect.value));
  });
  element("sign-unknown").addEventListener("click", () => {
    void sign(new Uint8Array(32).fill(0xee));
  });
  element("save-note").addEventListener("click", () => {
    void call(
      "write",
      client.localStorage.write({
        key: "note",
        value: new TextEncoder().encode(note.value),
      }),
      () => "saved",
    );
  });
  element("load-note").addEventListener("click", () => {
    void call("read", client.localStorage.read({ key: "note" }), (response) =>
      response.value === undefined
        ? "nothing stored"
        : new TextDecoder().decode(response.value),
    );
  });
  element("notify").addEventListener("click", () => {
    void call(
      "pushNotification",
      client.notifications.pushNotification({ text: note.value }),
      () => "shown",
    );
  });
}

connectIframeHost().then((client) => {
  connection.textContent = "Connected to the host.";
  wire(client);
});
