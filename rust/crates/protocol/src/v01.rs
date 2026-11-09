//! Version 1 payloads.

use parity_scale_codec::{Decode, Encode};

use crate::{Bytes32, Bytes64};

/// An account a product may sign with. The `Wallet` extension point carries
/// it too, so it is a `#[protocol_record]`.
#[sketch_macros::protocol_record]
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ProductAccount {
	/// 32-byte public key that identifies the account.
	pub public_key: Bytes32,
	/// Name the user gave the account.
	pub name: String,
}

/// A failure described only by its reason.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, thiserror::Error)]
#[error("{reason}")]
pub struct GenericError {
	/// Human-readable reason.
	pub reason: String,
}

impl From<String> for GenericError {
	fn from(reason: String) -> Self {
		Self { reason }
	}
}

/// The accounts a product may sign with.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostAccountGetAccountsResponse {
	/// Every account, in the wallet's order.
	pub accounts: Vec<ProductAccount>,
}

/// The account list after a change.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostAccountAccountsSubscribeItem {
	/// Every account, in the wallet's order.
	pub accounts: Vec<ProductAccount>,
}

/// Request to sign raw bytes with one of the product's accounts.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostSignRawRequest {
	/// Public key of the account to sign with.
	pub public_key: Bytes32,
	/// Bytes to sign. The host wraps them in `<Bytes>…</Bytes>` first.
	pub payload: Vec<u8>,
}

/// A raw-bytes signature.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostSignRawResponse {
	/// 64 signature bytes.
	pub signature: Bytes64,
}

/// Why raw bytes were not signed.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, thiserror::Error)]
pub enum HostSignRawError {
	/// The user declined on the device that holds the keys.
	#[error("the user declined the request")]
	Rejected,
	/// No account with the requested public key.
	#[error("no account with this public key")]
	UnknownAccount,
	/// Any other failure.
	#[error("{reason}")]
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// Request to read a value from the product's storage.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostLocalStorageReadRequest {
	/// Storage key to read.
	pub key: String,
}

/// A value read from the product's storage.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostLocalStorageReadResponse {
	/// Stored value, if present.
	pub value: Option<Vec<u8>>,
}

/// Request to write a value into the product's storage.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostLocalStorageWriteRequest {
	/// Storage key to write.
	pub key: String,
	/// Value to store at the key.
	pub value: Vec<u8>,
}

/// Why a storage operation failed.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, thiserror::Error)]
pub enum HostLocalStorageError {
	/// The storage quota is exhausted.
	#[error("storage quota exhausted")]
	Full,
	/// Any other failure.
	#[error("{reason}")]
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// A notice a product asks the host to show the user.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct HostPushNotificationRequest {
	/// Text of the notice.
	pub text: String,
}

/// Why a notice was not shown.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, thiserror::Error)]
pub enum HostPushNotificationError {
	/// Any failure.
	#[error("{reason}")]
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// A frame the host could not serve.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum ProtocolError {
	/// The host serves no method at this address.
	UnsupportedMessage {
		/// Trait id of the frame.
		trait_id: u8,
		/// Method id of the frame.
		method_id: u8,
	},
}
