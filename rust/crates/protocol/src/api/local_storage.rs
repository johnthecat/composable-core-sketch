//! The [`LocalStorage`] service and its payloads.

use parity_scale_codec::{Decode, Encode};

use crate::{Bytes, CallError, wire, wire_trait};

/// Request to read a value from the product's storage.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostLocalStorageReadRequest {
	/// Storage key to read.
	pub key: String,
}

/// A value read from the product's storage.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostLocalStorageReadResponse {
	/// Stored value, if present.
	pub value: Option<Bytes>,
}

/// Request to write a value into the product's storage.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostLocalStorageWriteRequest {
	/// Storage key to write.
	pub key: String,
	/// Value to store at the key.
	pub value: Bytes,
}

/// Why a storage operation failed.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub enum HostLocalStorageError {
	/// The storage quota is exhausted.
	Full,
	/// Any other failure.
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// Key/value storage scoped to the calling product.
#[wire_trait(id = 7)]
#[async_trait::async_trait]
pub trait LocalStorage: Send + Sync {
	/// This call exists because `ProductStorage` is a required extension
	/// point with no provider, so every host supplies its own, and the core
	/// scopes each key to the calling product.
	///
	/// Read a value by key.
	#[wire(id = 0)]
	async fn read(
		&self,
		request: HostLocalStorageReadRequest,
	) -> Result<HostLocalStorageReadResponse, CallError<HostLocalStorageError>>;

	/// This call exists because a read needs something written first, through
	/// the same `ProductStorage` and the same per-product keys.
	///
	/// Write a value to a key.
	#[wire(id = 1)]
	async fn write(
		&self,
		request: HostLocalStorageWriteRequest,
	) -> Result<(), CallError<HostLocalStorageError>>;
}
