//! The core's side of each product connection: [`ProductRuntimeHost`] serves
//! the protocol methods over the extension points.

use std::sync::Arc;

use host_extension::{Notifier, ProductAccount, ProductStorage, UserConfirmation, Wallet};
use protocol::CallError;

use crate::{host_core::ProductContext, subscription::Spawner};

mod bus;
mod capabilities;
mod versioning;

pub use bus::Bus;

/// What every product connection of one host runtime shares.
pub struct RuntimeServices {
	/// Where products' values persist.
	pub product_storage: Arc<dyn ProductStorage>,
	/// Asks the user before the core goes on.
	pub user_confirmation: Arc<dyn UserConfirmation>,
	/// The wallet provider, or the host code around one. `None` when the host
	/// attached none.
	pub wallet: Option<Arc<dyn Wallet>>,
	/// The notifier provider, or the host code around one.
	pub notifier: Option<Arc<dyn Notifier>>,
	/// Where the wallet pushes account changes, for every product
	/// subscription to follow.
	pub accounts: Arc<Bus<Vec<ProductAccount>>>,
	/// Runs subscription pumps.
	pub spawner: Spawner,
}

/// Serves the protocol to one product.
pub struct ProductRuntimeHost {
	services: Arc<RuntimeServices>,
	product: ProductContext,
}

impl ProductRuntimeHost {
	/// The connection of `product` over `services`.
	pub fn new(services: Arc<RuntimeServices>, product: ProductContext) -> Self {
		Self { services, product }
	}

	/// The wallet, or `Unsupported` when the host attached none.
	fn wallet<D>(&self) -> Result<&Arc<dyn Wallet>, CallError<D>> {
		self.services.wallet.as_ref().ok_or(CallError::Unsupported)
	}

	/// The notifier, or `Unsupported` when the host attached none.
	fn notifier<D>(&self) -> Result<&Arc<dyn Notifier>, CallError<D>> {
		self.services.notifier.as_ref().ok_or(CallError::Unsupported)
	}

	/// `key` in the product's own storage namespace.
	fn storage_key(&self, key: &str) -> String {
		format!("{}/{key}", self.product.product_id)
	}
}
