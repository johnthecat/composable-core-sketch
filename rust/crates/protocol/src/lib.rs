//! The product protocol: the API a host serves to products, the payloads its
//! methods carry, and the versioned envelopes those payloads travel in.
//!
//! [`api`] declares one trait per service, each method tagged with its wire id.
//! Payloads live in per-version modules ([`v01`]); [`versioned`] wraps them in
//! the envelopes that cross the wire, and [`latest`] names the newest payload of
//! each. `#[wire_trait]` writes each trait's method table ([`dispatch`]), which
//! the host runtime serves; `tool-codegen` reads this crate's rustdoc JSON to
//! generate the TypeScript product client.
//!
//! This crate holds no runtime, so an interface crate may name its types in
//! extension point signatures without depending on the core.

use core::{
	pin::Pin,
	task::{Context, Poll},
};

use futures::Stream;
use parity_scale_codec::{Decode, Encode};

pub mod api;
mod bytes;
pub mod dispatch;
pub mod v01;
pub mod versioned;

pub use bytes::{Bytes, Bytes32, Bytes64};
pub use sketch_macros::{wire, wire_trait};

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

/// Newest payload types, unwrapped from their versioned envelopes. Code outside
/// this crate names payloads from here rather than from a version module.
pub mod latest {
	pub use crate::v01::{
		GenericError, HostLocalStorageError, HostLocalStorageReadResponse, HostSignRawError,
		ProductAccount,
	};
	use crate::versioned::{self, Versioned};

	/// Latest payload type of a versioned envelope.
	pub type LatestOf<T> = <T as Versioned>::Latest;

	/// The accounts a product may sign with.
	pub type HostAccountGetAccountsResponse =
		LatestOf<versioned::account::HostAccountGetAccountsResponse>;
	/// The account list after a change.
	pub type HostAccountAccountsSubscribeItem =
		LatestOf<versioned::account::HostAccountAccountsSubscribeItem>;
	/// Raw-bytes signing request.
	pub type HostSignRawRequest = LatestOf<versioned::signing::HostSignRawRequest>;
	/// Raw-bytes signing result.
	pub type HostSignRawResponse = LatestOf<versioned::signing::HostSignRawResponse>;
	/// Product storage read request.
	pub type HostLocalStorageReadRequest =
		LatestOf<versioned::local_storage::HostLocalStorageReadRequest>;
	/// Product storage write request.
	pub type HostLocalStorageWriteRequest =
		LatestOf<versioned::local_storage::HostLocalStorageWriteRequest>;
	/// Notice a product asks the host to show.
	pub type HostPushNotificationRequest =
		LatestOf<versioned::notifications::HostPushNotificationRequest>;
	/// Why a notice was not shown.
	pub type HostPushNotificationError =
		LatestOf<versioned::notifications::HostPushNotificationError>;
}

/// How a call failed: with the method's own error, or for a reason any method
/// can fail with.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum CallError<D> {
	/// The method's own error.
	Domain(D),
	/// The user or the host refused the call.
	Denied,
	/// The host has nothing that serves the call, such as an extension point
	/// it attached no provider for.
	Unsupported,
	/// The request did not decode.
	MalformedFrame {
		/// Why it did not decode.
		reason: String,
	},
	/// The host failed while serving the call.
	HostFailure {
		/// Human-readable reason.
		reason: String,
	},
}

impl<D> CallError<D> {
	/// Maps the method's own error, keeping every other variant.
	pub fn map_domain<T>(self, map: impl FnOnce(D) -> T) -> CallError<T> {
		match self {
			Self::Domain(domain) => CallError::Domain(map(domain)),
			Self::Denied => CallError::Denied,
			Self::Unsupported => CallError::Unsupported,
			Self::MalformedFrame { reason } => CallError::MalformedFrame { reason },
			Self::HostFailure { reason } => CallError::HostFailure { reason },
		}
	}
}

/// Per-call context the dispatcher hands every method.
pub struct CallContext {
	/// The request id of the frame that started the call.
	pub request_id: String,
}

/// What a subscription method returns: a stream of items that ends with its
/// interrupt value, `Err`, or by running dry.
pub struct Subscription<Item, Interrupt> {
	inner: Pin<Box<dyn Stream<Item = Result<Item, Interrupt>> + Send>>,
}

impl<Item, Interrupt> Stream for Subscription<Item, Interrupt> {
	type Item = Result<Item, Interrupt>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.inner.as_mut().poll_next(cx)
	}
}

impl<Item, Interrupt> Subscription<Item, Interrupt> {
	/// Wraps a stream of items.
	pub fn new<S>(stream: S) -> Self
	where
		S: Stream<Item = Result<Item, Interrupt>> + Send + 'static,
	{
		Self { inner: Box::pin(stream) }
	}

	/// A subscription that ends at once with `interrupt`.
	pub fn interrupted(interrupt: Interrupt) -> Self
	where
		Item: Send + 'static,
		Interrupt: Send + 'static,
	{
		Self::new(futures::stream::once(core::future::ready(Err(interrupt))))
	}
}
