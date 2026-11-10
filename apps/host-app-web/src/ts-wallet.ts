// A wallet written entirely in TypeScript: the host implements the extension
// point itself, with no Rust provider. Keys and signatures are SHA-2 hashes
// standing in for real ones, not cryptographic keys or signatures.

import {
  WalletException,
  type AccountsListener,
  type ProductAccount,
  type SignRequest,
  type Signature,
  type Wallet,
} from "@sketch/host-extension";

async function sha(algorithm: "SHA-256" | "SHA-512", ...parts: Uint8Array[]) {
  const joined = new Uint8Array(
    parts.reduce((length, part) => length + part.length, 0),
  );
  let offset = 0;
  for (const part of parts) {
    joined.set(part, offset);
    offset += part.length;
  }
  return new Uint8Array(await crypto.subtle.digest(algorithm, joined));
}

/** An in-memory wallet that signs at once. */
export class TypeScriptWallet implements Wallet {
  #accounts: ProductAccount[] = [];
  #listener: AccountsListener | undefined;

  /** Add an account named `name`, and push the new list to the listener. */
  async addAccount(name: string): Promise<void> {
    const publicKey = await sha(
      "SHA-256",
      new TextEncoder().encode(`ts wallet ${name}`),
    );
    this.#accounts = [...this.#accounts, { publicKey, name }];
    this.#listener?.accountsChanged(this.#accounts);
  }

  async accounts(): Promise<ProductAccount[]> {
    return this.#accounts;
  }

  async sign(request: SignRequest): Promise<Signature> {
    const known = this.#accounts.some((account) =>
      account.publicKey.every(
        (byte, index) => byte === request.publicKey[index],
      ),
    );
    if (!known) throw new WalletException({ tag: "UnknownAccount" });
    return { bytes: await sha("SHA-512", request.publicKey, request.payload) };
  }

  setAccountsListener(listener: AccountsListener): void {
    this.#listener = listener;
  }
}
