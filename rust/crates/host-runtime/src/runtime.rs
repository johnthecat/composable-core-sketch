//! The runtime a host builds once, and the per-product runtimes it hands out.
//! Each product runtime serves the protocol through a `ProductRuntimeHost`
//! over the host runtime's extension points.

use std::sync::Arc;

use host_extension::{
	confirmation::UserConfirmation,
	notifier::Notifier,
	product::{FrameSink, ProductContext},
	storage::ProductStorage,
	wallet::{ProductAccount, Wallet},
};
use parity_scale_codec::Decode;
use protocol::{CallError, frame::ProtocolMessage};

use crate::{
	calls::{Spawner, platform_spawner},
	dispatcher::Dispatcher,
};

mod bus;
mod capabilities;

use bus::Bus;

/// The extension points a host supplies, each an `Arc<dyn Trait>` so the
/// runtime stays non-generic whichever provider, or host code around one, the
/// host passes in.
pub struct HostExtensions {
	/// Where products' values persist.
	pub product_storage: Arc<dyn ProductStorage>,
	/// Asks the user before the core goes on.
	pub user_confirmation: Arc<dyn UserConfirmation>,
	/// The wallet; without one, the product calls that need it fail as
	/// `Unsupported`.
	pub wallet: Option<Arc<dyn Wallet>>,
	/// The notifier; optional like the wallet.
	pub notifier: Option<Arc<dyn Notifier>>,
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

/// The host runtime: built once from the host's extension points, then shared
/// by every product connection.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Object))]
pub struct HostRuntime {
	extensions: HostExtensions,
	/// Where the wallet pushes account changes, for every product
	/// subscription to follow.
	accounts: Arc<Bus<Vec<ProductAccount>>>,
	/// Runs product calls.
	spawner: Spawner,
}

impl HostRuntime {
	/// A runtime over `extensions`. It hands the wallet its accounts listener
	/// once, here, and runs product calls on the platform's spawner.
	pub fn new(extensions: HostExtensions) -> Arc<Self> {
		Self::with_spawner(extensions, platform_spawner())
	}

	/// [`Self::new`], with product calls run on `spawner`; the tests run them
	/// on their own executor.
	fn with_spawner(extensions: HostExtensions, spawner: Spawner) -> Arc<Self> {
		let accounts = Arc::new(Bus::default());
		if let Some(wallet) = &extensions.wallet {
			wallet.set_accounts_listener(accounts.clone());
		}
		Arc::new(Self { extensions, accounts, spawner })
	}
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
impl HostRuntime {
	/// A runtime serving `product`, which sends its frames to `sink`.
	pub fn product_runtime(
		self: Arc<Self>,
		product: ProductContext,
		sink: Arc<dyn FrameSink>,
	) -> Arc<ProductRuntime> {
		let spawner = self.spawner.clone();
		let host = Arc::new(ProductRuntimeHost { runtime: self, product });
		let dispatcher = Dispatcher::new(protocol::api::methods(), host, spawner, sink);
		Arc::new(ProductRuntime { dispatcher })
	}
}

/// One product's connection: takes its frames and answers through its sink.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Object))]
pub struct ProductRuntime {
	dispatcher: Dispatcher<ProductRuntimeHost>,
}

#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
impl ProductRuntime {
	/// Start serving one frame from the product. Responses and subscription
	/// items go to the sink later; dropping the runtime drops every call still
	/// in flight.
	pub fn receive_frame(&self, frame: Vec<u8>) -> Result<(), ProductRuntimeError> {
		let message = ProtocolMessage::decode(&mut frame.as_slice())
			.map_err(|error| ProductRuntimeError::InvalidFrame { reason: error.to_string() })?;
		self.dispatcher.dispatch(message);
		Ok(())
	}
}

/// Serves the protocol to one product.
struct ProductRuntimeHost {
	runtime: Arc<HostRuntime>,
	product: ProductContext,
}

impl ProductRuntimeHost {
	/// The wallet, or `Unsupported` when the host attached none.
	fn wallet<D>(&self) -> Result<&dyn Wallet, CallError<D>> {
		self.runtime.extensions.wallet.as_deref().ok_or(CallError::Unsupported)
	}

	/// The notifier, or `Unsupported` when the host attached none.
	fn notifier<D>(&self) -> Result<&dyn Notifier, CallError<D>> {
		self.runtime.extensions.notifier.as_deref().ok_or(CallError::Unsupported)
	}

	/// `key` in the product's own storage namespace.
	fn storage_key(&self, key: &str) -> String {
		format!("{}/{key}", self.product.product_id)
	}
}

#[cfg(test)]
mod tests;
