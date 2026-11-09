//! Product subscriptions: the streams the dispatcher pumps into frames.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use futures::{
	StreamExt,
	channel::oneshot,
	future::{BoxFuture, Either, select},
};
use parity_scale_codec::Encode;
use protocol::dispatch::{MethodIds, SubscriptionOutput, SubscriptionStream};

use crate::{
	FrameSink,
	frame::{
		MESSAGE_TYPE_INTERRUPT, MESSAGE_TYPE_RESPONSE, ProtocolMessage, encode_clean_interrupt,
	},
};

/// Runs a subscription's pump.
pub type Spawner = Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>;

/// The platform's spawner: `spawn_local` on the web, a thread per pump
/// natively.
pub fn platform_spawner() -> Spawner {
	#[cfg(target_arch = "wasm32")]
	return Arc::new(|pump: BoxFuture<'static, ()>| wasm_bindgen_futures::spawn_local(pump));
	#[cfg(not(target_arch = "wasm32"))]
	Arc::new(|pump: BoxFuture<'static, ()>| {
		std::thread::spawn(move || futures::executor::block_on(pump));
	})
}

/// The live subscriptions of one product connection, by request id.
pub struct SubscriptionManager {
	active: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
	spawner: Spawner,
}

impl SubscriptionManager {
	/// A manager that runs each subscription's pump on `spawner`.
	pub fn new(spawner: Spawner) -> Self {
		Self { active: Arc::default(), spawner }
	}

	/// Pump `stream` into frames for `sink` until it ends or the product
	/// stops it. A stream that runs dry ends with a clean interrupt.
	pub fn activate(
		&self,
		request_id: String,
		ids: MethodIds,
		mut stream: SubscriptionStream,
		sink: Arc<dyn FrameSink>,
	) {
		let (stop_sender, mut stop) = oneshot::channel();
		lock(&self.active).insert(request_id.clone(), stop_sender);
		let active = self.active.clone();
		let finished = request_id.clone();
		let send = move |message_type, value| {
			sink.emit_frame(
				ProtocolMessage::new(request_id.clone(), ids, message_type, value).encode(),
			);
		};
		(self.spawner)(Box::pin(async move {
			loop {
				match select(stop, stream.next()).await {
					Either::Left(_) => return,
					Either::Right((item, still_running)) => {
						stop = still_running;
						match item {
							Some(SubscriptionOutput::Item(value)) => {
								send(MESSAGE_TYPE_RESPONSE, value)
							},
							Some(SubscriptionOutput::Interrupt(value)) => {
								send(MESSAGE_TYPE_INTERRUPT, value);
								break;
							},
							None => {
								send(MESSAGE_TYPE_INTERRUPT, encode_clean_interrupt());
								break;
							},
						}
					},
				}
			}
			lock(&active).remove(&finished);
		}));
	}

	/// Stop the subscription a product started with `request_id`.
	pub fn stop(&self, request_id: &str) {
		if let Some(stop) = lock(&self.active).remove(request_id) {
			let _ = stop.send(());
		}
	}
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
	mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
