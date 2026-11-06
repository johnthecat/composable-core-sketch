//! A [`Wallet`](sketch_wallet_extension::Wallet) provider that simulates a wallet
//! on a paired phone. It holds its accounts in memory, answers each sign request
//! after a delay, and declines or fails it when told to.
//!
//! The crate depends on the interface crate only, never on the core. On the web it
//! builds to its own WASM binary, whose `MockPhone` class hands out the provider
//! as the generated `WasmWallet`.

mod wallet;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use wallet::{MockWallet, PhoneDecision};
