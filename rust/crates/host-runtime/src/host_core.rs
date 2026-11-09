//! The runtime a host builds once, and the per-product runtimes it hands out.

use std::sync::Arc;

use host_extension::{Notifier, ProductStorage, UserConfirmation, Wallet};
use parity_scale_codec::Decode;

use crate::{
	dispatcher::Dispatcher,
	frame::ProtocolMessage,
	runtime::{Bus, ProductRuntimeHost, RuntimeServices},
	subscription::Spawner,
};

/// The product a connection serves.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Record))]
pub struct ProductContext {
	/// Identifies the product, and scopes its storage.
	pub product_id: String,
}

/// Where a product runtime sends the frames it produces.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export(foreign))]
pub trait FrameSink: Send + Sync {
	/// Deliver one encoded frame to the product.
	fn emit_frame(&self, frame: Vec<u8>);
}

/// Why a product runtime refused a frame.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Error))]
pub enum ProductRuntimeError {
	/// The bytes are not a frame.
	#[error("invalid frame: {reason}")]
	InvalidFrame {
		/// Why they did not decode.
		reason: String,
	},
}

/// The host runtime: built once from the extension points the host supplies,
/// then shared by every product connection.
///
/// Each extension point is an `Arc<dyn Trait>`, so the runtime stays
/// non-generic whichever provider, or host code around one, the host passes in.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Object))]
pub struct HostRuntime {
	services: Arc<RuntimeServices>,
}

impl HostRuntime {
	/// A runtime over the host's extension points. `wallet` and `notifier` are
	/// optional: without one, the product calls that need it fail as
	/// `Unsupported`. The runtime hands the wallet its accounts listener once,
	/// here. Subscriptions run on `spawner`: a host crate passes
	/// [`platform_spawner`](crate::platform_spawner), and the tests their own
	/// executor.
	pub fn new(
		product_storage: Arc<dyn ProductStorage>,
		user_confirmation: Arc<dyn UserConfirmation>,
		wallet: Option<Arc<dyn Wallet>>,
		notifier: Option<Arc<dyn Notifier>>,
		spawner: Spawner,
	) -> Self {
		let accounts = Arc::new(Bus::default());
		if let Some(wallet) = &wallet {
			wallet.set_accounts_listener(accounts.clone());
		}
		Self {
			services: Arc::new(RuntimeServices {
				product_storage,
				user_confirmation,
				wallet,
				notifier,
				accounts,
				spawner,
			}),
		}
	}
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
impl HostRuntime {
	/// A runtime serving `product`, which sends its frames to `sink`.
	pub fn product_runtime(
		&self,
		product: ProductContext,
		sink: Arc<dyn FrameSink>,
	) -> Arc<ProductRuntime> {
		let dispatcher = Dispatcher::new(
			protocol::api::methods(),
			Arc::new(ProductRuntimeHost::new(self.services.clone(), product)),
			self.services.spawner.clone(),
		);
		Arc::new(ProductRuntime { dispatcher, sink })
	}
}

/// One product's connection: takes its frames and answers through its sink.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Object))]
pub struct ProductRuntime {
	dispatcher: Dispatcher<ProductRuntimeHost>,
	sink: Arc<dyn FrameSink>,
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
impl ProductRuntime {
	/// Serve one frame from the product. Responses and subscription items go to
	/// the sink, possibly after this returns.
	pub async fn receive_frame(&self, frame: Vec<u8>) -> Result<(), ProductRuntimeError> {
		let message = ProtocolMessage::decode(&mut frame.as_slice())
			.map_err(|error| ProductRuntimeError::InvalidFrame { reason: error.to_string() })?;
		self.dispatcher.dispatch(message, &self.sink).await;
		Ok(())
	}
}

#[cfg(test)]
mod tests;
