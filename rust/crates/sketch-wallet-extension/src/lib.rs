//! The `Wallet` extension point: the trait through which the core lists accounts
//! and requests signatures, and the types its methods name.
//!
//! The core re-exports this crate as `sketch_core::host::extension`. A provider
//! crate depends on this crate, never on the core, and implements [`Wallet`].
//!
//! On the web, the `wasm` module that `sketch-codegen` generates holds two
//! wrappers. `ForeignWallet` wraps a JS object implementing the raw `Wallet`
//! interface as a [`Wallet`], for the core. `WasmWallet` exports a Rust provider
//! to JS with that same interface; it sits behind the `export` feature, which only
//! providers enable.

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::ForeignWallet;
#[cfg(all(target_arch = "wasm32", feature = "export"))]
pub use wasm::WasmWallet;

pub use futures::stream::BoxStream;
use parity_scale_codec::{Decode, Encode};

/// An account the wallet holds.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Account {
	/// 32-byte public key that identifies the account.
	pub public_key: [u8; 32],
	/// Name the user gave the account.
	pub name: String,
}

/// A request to sign `payload` with the account `public_key`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct SignRequest {
	/// Public key of the account to sign with.
	pub public_key: [u8; 32],
	/// Bytes to sign.
	pub payload: Vec<u8>,
}

/// A signature the wallet produced.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Signature {
	/// 64 signature bytes.
	pub bytes: [u8; 64],
}

/// Why the wallet did not answer a request.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, thiserror::Error)]
pub enum WalletError {
	/// The user declined the request.
	#[error("the user declined the request")]
	Rejected,
	/// The wallet holds no account with the requested public key.
	#[error("the wallet holds no account with this public key")]
	UnknownAccount,
	/// A failure the trait does not declare.
	#[error("{reason}")]
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// What a failure the trait does not declare becomes. The generated web wrappers
/// call it with a rejected promise's reason.
impl From<String> for WalletError {
	fn from(reason: String) -> Self {
		Self::Unknown { reason }
	}
}

/// Holds the user's accounts and signs with them, possibly on a paired phone.
#[async_trait::async_trait]
pub trait Wallet: Send + Sync {
	/// Lists the accounts the wallet holds.
	async fn accounts(&self) -> Result<Vec<Account>, WalletError>;

	/// Signs `request.payload` with `request.public_key`, resolving once the user
	/// approves.
	async fn sign(&self, request: SignRequest) -> Result<Signature, WalletError>;

	/// The account list, first as it is and then after each change. An error ends
	/// the stream.
	fn subscribe_accounts(&self) -> BoxStream<'static, Result<Vec<Account>, WalletError>>;
}

#[cfg(test)]
mod tests;
