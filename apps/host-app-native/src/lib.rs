//! A native host's own crate, as the RFC's "iOS and Android hosts" section
//! describes: it links the core and the providers the host uses into one
//! library, and declares the runtime constructor once.
//!
//! Built as a static library for the Swift app and a dynamic one for the
//! Kotlin app. Each crate it links keeps its own UniFFI namespace, so the
//! library also carries the providers' exports (`MockPhone`, `log_notifier`),
//! and this crate's own product frame builders for the demos.
//!
//! Why it reads the same as the web host crate: see the README's "Where to look".

use std::sync::Arc;

use host_runtime::{
	HostRuntime,
	host::extension::{Notifier, ProductStorage, UserConfirmation, Wallet},
};
// Links the providers' exports into the library.
use host_provider_notifier_log as _;
use host_provider_wallet_mock as _;

uniffi::setup_scaffolding!();

mod product_frames;

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
