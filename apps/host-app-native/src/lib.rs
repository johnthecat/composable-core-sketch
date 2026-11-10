//! A native host's own crate, as the RFC's "iOS and Android hosts" section
//! describes: it links the core and the providers the host uses into one
//! library, and declares the runtime constructor once.
//!
//! Built as a static library for the Swift app and a dynamic one for the
//! Kotlin app. Each crate it links keeps its own UniFFI namespace, so the
//! library also carries the providers' exports (`MockPhone`, `log_notifier`)
//! and the stand-in product's frame builders (`product-native`).
//!
//! Why it reads the same as the web host crate: see "One host crate per host" in
//! `docs/departures-from-the-rfc.md`.

use std::sync::Arc;

use host_runtime::{
	HostExtensions, HostRuntime,
	host::extension::{confirmation, notifier, storage, wallet},
};
// Links the providers' exports, and the stand-in product's, into the library.
use host_provider_notifier_log as _;
use host_provider_wallet_mock as _;
use product_native as _;

uniffi::setup_scaffolding!();

/// Build the host runtime over the extension points the app passes in: its
/// own storage and confirmation, and the providers or its code around them.
/// Without a wallet or a notifier, the product calls that need one are
/// `Unsupported`.
#[host_runtime::constructor]
pub fn host_runtime(
	product_storage: Arc<dyn storage::ProductStorage>,
	user_confirmation: Arc<dyn confirmation::UserConfirmation>,
	wallet: Option<Arc<dyn wallet::Wallet>>,
	notifier: Option<Arc<dyn notifier::Notifier>>,
) -> Arc<HostRuntime> {
	HostRuntime::new(HostExtensions { product_storage, user_confirmation, wallet, notifier })
}
