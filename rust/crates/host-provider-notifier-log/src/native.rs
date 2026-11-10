use std::sync::Arc;

use host_extension::notifier::Notifier;

use crate::LogNotifier;

/// The log notifier, writing to standard output, for the app to pass to the
/// runtime or wrap.
#[uniffi::export]
pub fn log_notifier() -> Arc<dyn Notifier> {
	Arc::new(LogNotifier::new(|line| println!("{line}")))
}
