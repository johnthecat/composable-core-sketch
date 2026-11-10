//! The product protocol: the API a host serves to products, and the payloads
//! its methods carry.
//!
//! [`api`] declares one trait per service, each method tagged with its wire id.
//! The request, response and error types a trait carries sit next to it.
//! `#[wire_trait]` writes each trait's method table ([`dispatch`]), which the
//! host runtime serves; `tool-codegen` reads this crate's rustdoc JSON to
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
pub mod frame;

pub use bytes::{Bytes, Bytes32, Bytes64};
pub use host_macros::{wire, wire_trait};

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

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
