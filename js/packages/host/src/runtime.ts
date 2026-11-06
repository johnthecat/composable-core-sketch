import { Vector } from "scale-ts";

import initCore, {
  HostRuntime as CoreHostRuntime,
  type InitInput,
} from "../wasm/sketch_core.js";
import { mapSettled, subscriptionIterable } from "./adapter-support.js";
import { Account, Signature, type Wallet } from "./generated/extensions.js";
import {
  decodeWalletError,
  rawWallet,
} from "./generated/extensions-adapter.js";

/** What {@link createHostRuntime} builds the runtime from. */
export interface HostRuntimeOptions {
  /** The wallet the core signs with: a provider, or host code around one. */
  wallet: Wallet;
  /**
   * The core's WASM binary. Defaults to the file beside the core's JS glue,
   * which a bundler such as Vite serves.
   */
  coreWasm?: InitInput;
}

/**
 * The product-facing runtime. A declared failure rejects with a
 * `WalletErrorException`.
 */
export interface HostRuntime {
  /** The accounts a product may sign with. */
  accounts(): Promise<Account[]>;
  /**
   * Signs `message` with the account `publicKey`. The core checks that the
   * wallet holds the account, then asks it to sign the message wrapped in
   * `<Bytes>…</Bytes>`.
   */
  signMessage(publicKey: Uint8Array, message: string): Promise<Signature>;
  /** The account list, first as it is and then after each change. */
  subscribeAccounts(): AsyncIterable<Account[]>;
}

/**
 * Loads the core's WASM binary and builds a host runtime that signs with
 * `options.wallet`.
 */
export async function createHostRuntime(
  options: HostRuntimeOptions,
): Promise<HostRuntime> {
  await initCore(
    options.coreWasm === undefined
      ? undefined
      : { module_or_path: options.coreWasm },
  );
  const core = new CoreHostRuntime(rawWallet(options.wallet));
  const accounts = Vector(Account);
  return {
    accounts: () =>
      mapSettled(
        () => core.accounts(),
        (bytes) => accounts.dec(bytes),
        decodeWalletError,
      ),
    signMessage: (publicKey, message) =>
      mapSettled(
        () => core.signMessage(publicKey, message),
        (bytes) => Signature.dec(bytes),
        decodeWalletError,
      ),
    subscribeAccounts: () =>
      subscriptionIterable<Account[]>((onItem, onError) =>
        core.subscribeAccounts(
          (item) => onItem(accounts.dec(item)),
          (reason) => onError(decodeWalletError(reason)),
        ),
      ),
  };
}
