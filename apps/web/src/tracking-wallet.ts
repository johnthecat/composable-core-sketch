// Host code around the wallet provider. It implements `Wallet` by forwarding to
// the provider, so it sees every call the core makes through the extension
// point and records how each one stands. The core has no status API for this.

import {
  WalletErrorException,
  type Account,
  type SignRequest,
  type Signature,
  type Wallet,
} from "@sketch/host";

/** Where a request from the core to the wallet stands. */
export type RequestStatus =
  "pending" | "answered" | "rejected" | "failed" | "live" | "closed";

/** One call the core made through the `Wallet` extension point. */
export interface WalletRequest {
  readonly id: number;
  readonly method: keyof Wallet;
  /** What the core asked for. */
  readonly summary: string;
  readonly status: RequestStatus;
  /** What came back, or why it failed. */
  readonly outcome?: string;
  readonly startedAt: number;
  readonly settledAt?: number;
}

/**
 * Wraps `inner` in a `Wallet` that reports every request the core sends
 * through it. `onChange` receives the full list after each change, newest
 * first.
 */
export function trackRequests(
  inner: Wallet,
  onChange: (requests: readonly WalletRequest[]) => void,
): Wallet {
  let requests: WalletRequest[] = [];
  let nextId = 1;

  const start = (
    method: keyof Wallet,
    summary: string,
    status: RequestStatus,
  ) => {
    const id = nextId++;
    requests = [
      { id, method, summary, status, startedAt: Date.now() },
      ...requests,
    ];
    onChange(requests);
    return id;
  };
  const update = (id: number, change: Partial<WalletRequest>) => {
    requests = requests.map((request) =>
      request.id === id ? { ...request, ...change } : request,
    );
    onChange(requests);
  };
  const settle = (id: number, status: RequestStatus, outcome: string) =>
    update(id, { status, outcome, settledAt: Date.now() });
  const fail = (id: number, reason: unknown) => {
    const [status, outcome] = describeFailure(reason);
    settle(id, status, outcome);
  };

  async function track<T>(
    method: keyof Wallet,
    summary: string,
    call: () => Promise<T>,
    describe: (value: T) => string,
  ): Promise<T> {
    const id = start(method, summary, "pending");
    try {
      const value = await call();
      settle(id, "answered", describe(value));
      return value;
    } catch (reason) {
      fail(id, reason);
      throw reason;
    }
  }

  return {
    accounts: () =>
      track("accounts", "list accounts", () => inner.accounts(), countAccounts),
    sign: (request: SignRequest) =>
      track(
        "sign",
        `sign ${request.payload.length} bytes with ${shortHex(request.publicKey)}`,
        () => inner.sign(request),
        (signature: Signature) => `signature ${shortHex(signature.bytes)}`,
      ),
    subscribeAccounts: () => {
      const updates = inner.subscribeAccounts();
      return {
        async *[Symbol.asyncIterator]() {
          const id = start(
            "subscribeAccounts",
            "follow the account list",
            "live",
          );
          let count = 0;
          let ended = false;
          try {
            for await (const accounts of updates) {
              count += 1;
              update(id, {
                outcome: `update ${count}: ${countAccounts(accounts)}`,
              });
              yield accounts;
            }
          } catch (reason) {
            ended = true;
            fail(id, reason);
            throw reason;
          } finally {
            if (!ended) settle(id, "closed", `${count} updates`);
          }
        },
      };
    },
  };
}

function describeFailure(reason: unknown): [RequestStatus, string] {
  if (reason instanceof WalletErrorException) {
    const { error } = reason;
    if (error.tag === "Rejected") return ["rejected", "declined on the phone"];
    if (error.tag === "Unknown") return ["failed", error.value.reason];
    return ["failed", error.tag];
  }
  return ["failed", reason instanceof Error ? reason.message : String(reason)];
}

function countAccounts(accounts: Account[]): string {
  return accounts.length === 1 ? "1 account" : `${accounts.length} accounts`;
}

/** `0x` and the first and last bytes of `bytes` in hex. */
export function shortHex(bytes: Uint8Array): string {
  const hex = toHex(bytes);
  return hex.length <= 12
    ? `0x${hex}`
    : `0x${hex.slice(0, 6)}…${hex.slice(-4)}`;
}

/** `bytes` in lowercase hex, without a prefix. */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}
