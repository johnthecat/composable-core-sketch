//! Routes product frames to the methods that serve them.
//!
//! A [`Dispatcher`] holds the protocol's method table (`protocol::api::methods`,
//! written by `#[wire_trait]`), keyed on each method's `(trait, method)` wire
//! pair, and the host that serves it.

use std::{collections::HashMap, sync::Arc};

use parity_scale_codec::Encode;
use protocol::{
	CallContext,
	dispatch::{Handler, Method, MethodIds},
	v01,
	versioned::protocol_error::ProtocolError,
};

use crate::{
	FrameSink,
	frame::{
		MESSAGE_TYPE_INTERRUPT, MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_RESPONSE, MESSAGE_TYPE_STOP,
		PROTOCOL_ERROR_METHOD_ID, PROTOCOL_ERROR_TRAIT_ID, ProtocolMessage,
	},
	subscription::{Spawner, SubscriptionManager},
};

/// Routes one product connection's frames to `H`.
pub struct Dispatcher<H> {
	methods: HashMap<(u8, u8), Handler<H>>,
	host: Arc<H>,
	live: SubscriptionManager,
}

impl<H: Send + Sync + 'static> Dispatcher<H> {
	/// Serves every method of `methods` with `host`, running subscriptions on
	/// `spawner`.
	pub fn new(methods: Vec<Method<H>>, host: Arc<H>, spawner: Spawner) -> Self {
		let methods = methods
			.into_iter()
			.map(|Method { ids, handler }| ((ids.trait_id, ids.method_id), handler))
			.collect();
		Self { methods, host, live: SubscriptionManager::new(spawner) }
	}

	/// Serve one frame, sending what it produces to `sink`.
	pub async fn dispatch(&self, message: ProtocolMessage, sink: &Arc<dyn FrameSink>) {
		let ProtocolMessage { request_id, payload } = message;
		let ids = MethodIds { trait_id: payload.trait_id, method_id: payload.method_id };
		let reply = |message_type, value| {
			ProtocolMessage::new(request_id.clone(), ids, message_type, value).encode()
		};
		let cx = CallContext { request_id: request_id.clone() };
		match (self.methods.get(&(ids.trait_id, ids.method_id)), payload.message_type) {
			(Some(Handler::Request(handler)), MESSAGE_TYPE_REQUEST) => {
				let response = handler(self.host.clone(), cx, payload.value).await;
				sink.emit_frame(reply(MESSAGE_TYPE_RESPONSE, response));
			},
			(Some(Handler::Subscription(handler)), MESSAGE_TYPE_REQUEST) => {
				match handler(self.host.clone(), cx, payload.value).await {
					Ok(stream) => self.live.activate(request_id, ids, stream, sink.clone()),
					Err(interrupt) => sink.emit_frame(reply(MESSAGE_TYPE_INTERRUPT, interrupt)),
				}
			},
			(Some(Handler::Subscription(_)), MESSAGE_TYPE_STOP) => self.live.stop(&request_id),
			(Some(_), _) => {},
			(None, _) => {
				let error = ProtocolError::from(v01::ProtocolError::UnsupportedMessage {
					trait_id: ids.trait_id,
					method_id: ids.method_id,
				});
				let address = MethodIds {
					trait_id: PROTOCOL_ERROR_TRAIT_ID,
					method_id: PROTOCOL_ERROR_METHOD_ID,
				};
				let frame = ProtocolMessage::new(
					request_id,
					address,
					MESSAGE_TYPE_RESPONSE,
					error.encode(),
				);
				sink.emit_frame(frame.encode());
			},
		}
	}
}
