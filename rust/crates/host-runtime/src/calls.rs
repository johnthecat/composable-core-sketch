//! The calls in flight on one product connection: requests awaiting their
//! response, and running subscriptions. Each runs on the spawner until it
//! finishes or the product stops it. Dropping the connection ends it too.

use core::pin::pin;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use futures::{
	StreamExt,
	channel::oneshot,
	future::{BoxFuture, select},
};
use host_extension::{Bytes, product::FrameSink};
use parity_scale_codec::Encode;
use protocol::{
	dispatch::{MethodIds, SubscriptionOutput, SubscriptionStream},
	frame::{
		MESSAGE_TYPE_INTERRUPT, MESSAGE_TYPE_RESPONSE, ProtocolMessage, encode_clean_interrupt,
	},
};

/// Runs a call.
pub type Spawner = Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>;

/// The platform's spawner: `spawn_local` on the web, and natively the one
/// executor thread every host runtime shares, as truapi's bridge executor.
/// Nothing that runs there may block it; see the interface crate's threading
/// contract. Calls run in the order they are spawned or woken.
pub fn platform_spawner() -> Spawner {
	#[cfg(target_arch = "wasm32")]
	return Arc::new(|call: BoxFuture<'static, ()>| wasm_bindgen_futures::spawn_local(call));
	#[cfg(not(target_arch = "wasm32"))]
	{
		static EXECUTOR: std::sync::OnceLock<futures::executor::ThreadPool> =
			std::sync::OnceLock::new();
		let executor = EXECUTOR.get_or_init(|| {
			futures::executor::ThreadPool::builder()
				.pool_size(1)
				.name_prefix("host-runtime-")
				.create()
				.expect("the host runtime's executor thread starts")
		});
		Arc::new(move |call: BoxFuture<'static, ()>| executor.spawn_ok(call))
	}
}

/// The calls in flight on one product connection. It owns each call's stop
/// sender, and dropping a sender ends its call, so dropping `LiveCalls` drops
/// every call, which cancels a Swift or Kotlin implementation it awaits.
pub struct LiveCalls {
	/// The stop sender of each call, by request id.
	stops: Mutex<HashMap<String, oneshot::Sender<()>>>,
	spawner: Spawner,
}

impl LiveCalls {
	/// No calls yet, each to run on `spawner`.
	pub fn new(spawner: Spawner) -> Self {
		Self { stops: Mutex::default(), spawner }
	}

	/// Run the request `request_id`, then send its encoded response to `sink`.
	pub fn request(
		&self,
		request_id: String,
		ids: MethodIds,
		call: BoxFuture<'static, Vec<u8>>,
		sink: Arc<dyn FrameSink>,
	) {
		let reply = Reply { request_id: request_id.clone(), ids, sink };
		self.run(request_id, async move {
			let response = call.await;
			reply.send(MESSAGE_TYPE_RESPONSE, response);
		});
	}

	/// Start the subscription `request_id`, then pump its items into frames for
	/// `sink` until it ends. A stream that runs dry ends with a clean interrupt.
	pub fn subscription(
		&self,
		request_id: String,
		ids: MethodIds,
		start: BoxFuture<'static, SubscriptionStream>,
		sink: Arc<dyn FrameSink>,
	) {
		let reply = Reply { request_id: request_id.clone(), ids, sink };
		self.run(request_id, async move {
			let mut stream = start.await;
			loop {
				match stream.next().await {
					Some(SubscriptionOutput::Item(value)) => {
						reply.send(MESSAGE_TYPE_RESPONSE, value)
					},
					Some(SubscriptionOutput::Interrupt(value)) => {
						return reply.send(MESSAGE_TYPE_INTERRUPT, value);
					},
					None => return reply.send(MESSAGE_TYPE_INTERRUPT, encode_clean_interrupt()),
				}
			}
		});
	}

	/// Stop the call a product started with `request_id`.
	pub fn stop(&self, request_id: &str) {
		lock(&self.stops).remove(request_id);
	}

	/// Run `call` on the spawner until it finishes or its stop sender is
	/// dropped.
	fn run(&self, request_id: String, call: impl Future<Output = ()> + Send + 'static) {
		let (stop_sender, stopped) = oneshot::channel();
		{
			let mut stops = lock(&self.stops);
			// A finished call dropped its receiver, so its sender is cancelled.
			stops.retain(|_, stop| !stop.is_canceled());
			stops.insert(request_id, stop_sender);
		}
		(self.spawner)(Box::pin(async move {
			select(stopped, pin!(call)).await;
		}));
	}
}

/// Where one call's frames go.
struct Reply {
	request_id: String,
	ids: MethodIds,
	sink: Arc<dyn FrameSink>,
}

impl Reply {
	fn send(&self, message_type: u8, value: Vec<u8>) {
		let frame = ProtocolMessage {
			request_id: self.request_id.clone(),
			ids: self.ids,
			message_type,
			value,
		};
		self.sink.emit_frame(Bytes(frame.encode()));
	}
}

/// Locks `mutex`, ignoring poison: no lock in the core is held across code
/// that can leave its data half-written.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
	mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
