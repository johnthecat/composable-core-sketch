//! A [`Wallet`](host_extension::Wallet) provider that simulates a wallet on a paired
//! phone. It holds its accounts in memory, answers each sign request after a
//! delay, and declines it or never answers when told to.
//!
//! The crate depends on the interface crate only, never on the core.
//! `src/wallet.rs` implements the trait, and each export sits in its own file:
//! `src/wasm.rs` for the web, whose `MockPhone` class hands the provider to JS
//! as a typed `Wallet` object, and `src/native.rs` for UniFFI, whose
//! `MockPhone` hands it out as a `Wallet` trait object. A host crate that
//! links this crate carries both exports into its library.

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

#[cfg(not(target_arch = "wasm32"))]
mod native;
mod wallet;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use wallet::{MockWallet, PhoneDecision};
