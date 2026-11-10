//! The method table a dispatcher serves: each protocol method's wire pair and
//! a handler that decodes its request and calls the trait. The handler encodes
//! what the trait returns.
//!
//! `#[wire_trait]` writes one table per trait (`signing::methods`, ...) from the
//! trait's own signatures, and [`crate::api::methods`] joins them. The host
//! runtime routes frames by the wire pair; nothing else describes a method.

use std::sync::Arc;

use futures::{
	StreamExt,
	future::BoxFuture,
	stream::{BoxStream, Stream},
};
use parity_scale_codec::{Decode, Encode};

/// Wire pair of one protocol method, encoded as its two bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct MethodIds {
	/// Trait id.
	pub trait_id: u8,
	/// Method id within the trait.
	pub method_id: u8,
}

/// One frame a subscription produces.
pub enum SubscriptionOutput {
	/// An encoded item.
	Item(Vec<u8>),
	/// An encoded `Err(interrupt)`, which ends the subscription.
	Interrupt(Vec<u8>),
}

/// The frames of one running subscription.
pub type SubscriptionStream = BoxStream<'static, SubscriptionOutput>;

/// Serves a request: the host and the encoded request, to the encoded
/// `Result<Response, CallError<Error>>`.
pub type RequestHandler<H> = fn(Arc<H>, Vec<u8>) -> BoxFuture<'static, Vec<u8>>;

/// Starts a subscription: its frames, which a request that does not decode
/// ends at once.
pub type SubscriptionHandler<H> = fn(Arc<H>, Vec<u8>) -> BoxFuture<'static, SubscriptionStream>;

/// How a method is served.
pub enum Handler<H> {
	/// One request, one response.
	Request(RequestHandler<H>),
	/// One start, then items until an interrupt.
	Subscription(SubscriptionHandler<H>),
}

/// One protocol method: its wire pair and its handler.
pub struct Method<H> {
	/// Wire pair.
	pub ids: MethodIds,
	/// How the method is served.
	pub handler: Handler<H>,
}

/// Encode a typed subscription as the frames it produces.
pub fn subscription_stream<Item, Interrupt, S>(stream: S) -> SubscriptionStream
where
	Item: Encode + 'static,
	Interrupt: Encode + 'static,
	S: Stream<Item = Result<Item, Interrupt>> + Send + 'static,
{
	stream
		.map(|item| match item {
			Ok(item) => SubscriptionOutput::Item(item.encode()),
			Err(interrupt) => SubscriptionOutput::Interrupt(Err::<(), _>(interrupt).encode()),
		})
		.boxed()
}
