//! The web host app's own crate: it links the core and the providers the app
//! uses into the app's one WASM binary, and declares the runtime constructor
//! once, as the native host crate does.
//!
//! The binary also carries the providers' exports (`MockPhone`,
//! `logNotifier`), each handing its provider to TypeScript as a typed object.
//!
//! Why it reads the same as the native host crate: see the README's "Where to look".

use std::sync::Arc;

use host_runtime::{
	HostRuntime,
	host::extension::{Notifier, ProductStorage, UserConfirmation, Wallet},
};
// Links the providers' exports into the binary.
use host_provider_notifier_log as _;
use host_provider_wallet_mock as _;

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

/// Build the host runtime over the extension points the app passes in: its
/// own storage and confirmation, and the providers or its code around them.
/// Without a wallet or a notifier, the product calls that need one are
/// `Unsupported`.
#[host_runtime::constructor]
pub fn host_runtime(
	product_storage: Arc<dyn ProductStorage>,
	user_confirmation: Arc<dyn UserConfirmation>,
	wallet: Option<Arc<dyn Wallet>>,
	notifier: Option<Arc<dyn Notifier>>,
) -> Arc<HostRuntime> {
	Arc::new(HostRuntime::new(
		product_storage,
		user_confirmation,
		wallet,
		notifier,
		host_runtime::platform_spawner(),
	))
}
