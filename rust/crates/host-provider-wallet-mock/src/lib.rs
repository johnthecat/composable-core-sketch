//! A [`Wallet`](host_extension::wallet::Wallet) provider that simulates a wallet on a paired
//! phone. It holds its accounts in memory and answers each sign request after a
//! delay. It declines a request, or never answers, when told to.
//!
//! `README.md` describes its exports and its place in the demo.

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

#[cfg(not(target_arch = "wasm32"))]
mod native;
mod wallet;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use wallet::{MockWallet, PhoneDecision};
