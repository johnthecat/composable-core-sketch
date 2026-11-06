// The host package: `createHostRuntime` loads the core and connects it to the
// host's wallet. The `Wallet` interface and its adapters are generated from the
// interface crate by sketch-codegen.

export {
  createHostRuntime,
  type HostRuntime,
  type HostRuntimeOptions,
} from "./runtime.js";
export {
  Account,
  SignRequest,
  Signature,
  WalletError,
  WalletErrorException,
  type Wallet,
} from "./generated/extensions.js";
export {
  rawWallet,
  typedWallet,
  type RawWallet,
} from "./generated/extensions-adapter.js";
