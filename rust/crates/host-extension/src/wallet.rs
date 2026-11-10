//! The `Wallet` extension point: the accounts the core signs with.
//!
//! The host, this device or a paired phone holds them.

use std::sync::Arc;

use host_macros::{extension_error, extension_listener, extension_point, extension_type};
pub use protocol::api::account::ProductAccount;
use protocol::{Bytes, Bytes32, Bytes64};

/// A request to sign `payload` with the account `public_key`.
#[extension_type]
#[derive(Debug, Clone, PartialEq)]
pub struct SignRequest {
	/// Public key of the account to sign with.
	pub public_key: Bytes32,
	/// Bytes to sign.
	pub payload: Bytes,
}

/// A signature the wallet produced.
#[extension_type]
#[derive(Debug, PartialEq)]
pub struct Signature {
	/// 64 signature bytes.
	pub bytes: Bytes64,
}

/// Why the wallet did not answer a request.
#[extension_error]
#[derive(PartialEq)]
pub enum WalletError {
	/// The user declined the request.
	#[error("the user declined the request")]
	Rejected,
	/// The wallet holds no account with the requested public key.
	#[error("the wallet holds no account with this public key")]
	UnknownAccount,
}

/// Holds the user's accounts and signs with them, possibly on a paired phone.
#[extension_point]
pub trait Wallet: Send + Sync {
	/// Lists the accounts the wallet holds.
	async fn accounts(&self) -> Result<Vec<ProductAccount>, WalletError>;

	/// Signs `request.payload` with `request.public_key`, resolving once the
	/// user approves.
	async fn sign(&self, request: SignRequest) -> Result<Signature, WalletError>;

	/// Pushes every later change of the account list to `listener`. The core
	/// calls it once, when it attaches the wallet; a wallet keeps the last
	/// listener it was given.
	fn set_accounts_listener(&self, listener: Arc<dyn AccountsListener>);
}

/// Where a wallet pushes changes of its account list. The core implements it,
/// and so does host code around a wallet that wants to see the changes.
#[extension_listener]
pub trait AccountsListener: Send + Sync {
	/// The account list changed to `accounts`. Returns at once.
	fn accounts_changed(&self, accounts: Vec<ProductAccount>);
}

#[cfg(test)]
mod tests;
