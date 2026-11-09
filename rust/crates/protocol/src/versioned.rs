//! Versioned envelopes of the protocol's payloads.
//!
//! A versioned envelope is a SCALE enum whose variants (`V1`, `V2`, ...) are
//! successive versions of one message, newest last. The host turns a received
//! envelope into its [`Versioned::Latest`] payload with [`IntoLatest`], serves
//! it in latest terms, then answers in the caller's version with [`FromLatest`].
//! `sketch_macros::versioned_type!` declares them, one per leg of a method's
//! exchange.

/// A versioned message envelope.
///
/// The first encoded byte is `version() - 1`, and every version from 1 to
/// [`LATEST`](Self::LATEST) exists.
pub trait Versioned: Sized {
	/// The newest version's payload.
	type Latest;

	/// Version number of the newest variant.
	const LATEST: u8;

	/// Version number of the variant held.
	fn version(&self) -> u8;
}

/// Upgrade a received envelope to its latest payload.
pub trait IntoLatest: Versioned {
	/// Convert whatever version is held into the latest payload.
	fn into_latest(self) -> Self::Latest;
}

/// Downgrade a latest payload to the version a peer at `target` speaks.
pub trait FromLatest: Versioned {
	/// Build the envelope for protocol version `target`.
	fn from_latest(latest: Self::Latest, target: u8) -> Self;
}

pub mod account;
pub mod local_storage;
pub mod notifications;
pub mod protocol_error;
pub mod signing;
