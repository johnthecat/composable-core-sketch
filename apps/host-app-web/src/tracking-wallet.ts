// Host code around the wallet provider. It implements `Wallet` by forwarding
// to the provider, so it sees every call the core makes through the extension
// point and records how each one stands. It gives the provider its own
// accounts listener, which records each change the provider pushes and passes
// it on to the core's. The core has no status API for this. When the host
// disconnects the product, the core drops the product's calls, and the host
// marks the ones still pending as dropped.

import {
  WalletException,
  type AccountsListener,
  type ProductAccount,
  type SignRequest,
  type Signature,
  type Wallet,
  type WalletError,
} from "@sketch/host-extension";

/** Where a request between the core and the wallet stands. */
export type RequestStatus =
  "pending" | "answered" | "rejected" | "failed" | "pushed" | "dropped";

/** One call the core made to the wallet, or one change the wallet pushed. */
export interface WalletRequest {
  readonly id: number;
  readonly method: "accounts" | "sign" | "accountsChanged";
  /** What the core asked for, or what the wallet pushed. */
  readonly summary: string;
  readonly status: RequestStatus;
  /** What came back, or why it failed. */
  readonly outcome?: string;
  readonly startedAt: number;
  readonly settledAt?: number;
}

/** A `Wallet` that reports what passes through it. */
export interface TrackingWallet extends Wallet {
  /** The host disconnected the product, so the core dropped its pending calls. */
  callsDropped(): void;
}

/**
 * Wraps `inner` in a `Wallet` that reports every request the core sends
 * through it and every change `inner` pushes. `onChange` receives the full
 * list after each change, newest first.
 */
export function trackRequests(
  inner: Wallet,
  onChange: (requests: readonly WalletRequest[]) => void,
): TrackingWallet {
  let requests: WalletRequest[] = [];
  let nextId = 1;

  const add = (request: Omit<WalletRequest, "id" | "startedAt">) => {
    const id = nextId++;
    requests = [{ id, startedAt: Date.now(), ...request }, ...requests];
    onChange(requests);
    return id;
  };
  const settle = (id: number, status: RequestStatus, outcome: string) => {
    requests = requests.map((request) =>
      request.id === id && request.status === "pending"
        ? { ...request, status, outcome, settledAt: Date.now() }
        : request,
    );
    onChange(requests);
  };
  const fail = (id: number, error: WalletError) =>
    error.tag === "Rejected"
      ? settle(id, "rejected", "declined on the phone")
      : settle(
          id,
          "failed",
          error.tag === "Unknown" ? error.value.reason : error.tag,
        );

  async function track<T>(
    method: "accounts" | "sign",
    summary: string,
    call: () => Promise<T>,
    describe: (value: T) => string,
  ): Promise<T> {
    const id = add({ method, summary, status: "pending" });
    try {
      const value = await call();
      settle(id, "answered", describe(value));
      return value;
    } catch (reason) {
      if (reason instanceof WalletException) fail(id, reason.error);
      else settle(id, "failed", String(reason));
      throw reason;
    }
  }

  return {
    callsDropped() {
      for (const request of requests) {
        if (request.status === "pending") {
          settle(request.id, "dropped", "the product disconnected");
        }
      }
    },
    accounts: () =>
      track("accounts", "list accounts", () => inner.accounts(), countAccounts),
    sign: (request: SignRequest) =>
      track(
        "sign",
        `sign ${request.payload.length} bytes with ${shortHex(request.publicKey)}`,
        () => inner.sign(request),
        (signature: Signature) => `signature ${shortHex(signature.bytes)}`,
      ),
    setAccountsListener: (listener: AccountsListener) =>
      inner.setAccountsListener({
        accountsChanged(accounts: ProductAccount[]) {
          add({
            method: "accountsChanged",
            summary: "the wallet pushed a change",
            status: "pushed",
            outcome: countAccounts(accounts),
          });
          listener.accountsChanged(accounts);
        },
      }),
  };
}

function countAccounts(accounts: ProductAccount[]): string {
  return accounts.length === 1 ? "1 account" : `${accounts.length} accounts`;
}

/** `0x` and the first and last bytes of `bytes` in hex. */
export function shortHex(bytes: Uint8Array): string {
  const hex = Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  return hex.length <= 12
    ? `0x${hex}`
    : `0x${hex.slice(0, 6)}…${hex.slice(-4)}`;
}
