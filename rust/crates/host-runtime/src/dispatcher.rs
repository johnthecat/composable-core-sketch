//! Routes product frames to the methods that serve them.
//!
//! A [`Dispatcher`] holds:
//!
//! - the protocol's method table (`protocol::api::methods`, written by `#[wire_trait]`), keyed on
//!   each method's `(trait, method)` wire pair;
//! - the host that serves it;
//! - the product's frame sink.
//!
//! Every call runs under [`LiveCalls`]. A frame no method serves is dropped.

use std::{collections::HashMap, sync::Arc};

use host_extension::product::FrameSink;
use protocol::{
	dispatch::{Handler, Method, MethodIds},
	frame::{MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_STOP, ProtocolMessage},
};

use crate::calls::{LiveCalls, Spawner};

/// Routes one product connection's frames to `H`.
pub struct Dispatcher<H> {
	methods: HashMap<MethodIds, Handler<H>>,
	host: Arc<H>,
	calls: LiveCalls,
	sink: Arc<dyn FrameSink>,
}

impl<H: Send + Sync + 'static> Dispatcher<H> {
	/// Serves every method of `methods` with `host`, running calls on
	/// `spawner` and sending what they produce to `sink`.
	pub fn new(
		methods: Vec<Method<H>>,
		host: Arc<H>,
		spawner: Spawner,
		sink: Arc<dyn FrameSink>,
	) -> Self {
		let methods = methods.into_iter().map(|Method { ids, handler }| (ids, handler)).collect();
		Self { methods, host, calls: LiveCalls::new(spawner), sink }
	}

	/// Start serving one frame.
	pub fn dispatch(&self, message: ProtocolMessage) {
		let ProtocolMessage { request_id, ids, message_type, value } = message;
		match (self.methods.get(&ids), message_type) {
			(Some(Handler::Request(handler)), MESSAGE_TYPE_REQUEST) => {
				let call = handler(self.host.clone(), value);
				self.calls.request(request_id, ids, call, self.sink.clone());
			},
			(Some(Handler::Subscription(handler)), MESSAGE_TYPE_REQUEST) => {
				let start = handler(self.host.clone(), value);
				self.calls.subscription(request_id, ids, start, self.sink.clone());
			},
			(Some(Handler::Subscription(_)), MESSAGE_TYPE_STOP) => self.calls.stop(&request_id),
			_ => {},
		}
	}
}
