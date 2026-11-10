//! The [`Signing`] service and its payloads.

use parity_scale_codec::{Decode, Encode};

use crate::{Bytes, Bytes32, Bytes64, CallError, wire, wire_trait};

/// Request to sign raw bytes with one of the product's accounts.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostSignRawRequest {
	/// Public key of the account to sign with.
	pub public_key: Bytes32,
	/// Bytes to sign, passed to the wallet as given.
	pub payload: Bytes,
}

/// A raw-bytes signature.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostSignRawResponse {
	/// 64 signature bytes.
	pub signature: Bytes64,
}

/// Why raw bytes were not signed.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub enum HostSignRawError {
	/// The user declined on the device that holds the keys.
	Rejected,
	/// No account with the requested public key.
	UnknownAccount,
	/// Any other failure.
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// Signing with the product's accounts.
#[wire_trait(id = 13)]
#[async_trait::async_trait]
pub trait Signing: Send + Sync {
	/// This call exists because it crosses two extension points in turn,
	/// `UserConfirmation` and then `Wallet`. A host sees the call wait on the
	/// user or fail in a provider, and the core can drop it while it waits.
	///
	/// Sign raw bytes. The user confirms each call; a refusal fails it as
	/// `Denied`.
	#[wire(id = 0)]
	async fn sign_raw(
		&self,
		request: HostSignRawRequest,
	) -> Result<HostSignRawResponse, CallError<HostSignRawError>>;
}
