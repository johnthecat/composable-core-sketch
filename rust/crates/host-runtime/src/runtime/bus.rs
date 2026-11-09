//! Fans the values an extension point pushes out to every product subscription
//! following them, as truapi's `NativeEventBus` does for the theme.

use std::sync::{Mutex, MutexGuard, PoisonError};

use futures::channel::mpsc;
use host_extension::{AccountsListener, ProductAccount};

/// One sender per live product subscription following a `T`.
pub struct Bus<T> {
	subscribers: Mutex<Vec<mpsc::UnboundedSender<T>>>,
}

// Written by hand: a derive would ask `T` to be `Default`.
impl<T> Default for Bus<T> {
	fn default() -> Self {
		Self { subscribers: Mutex::new(Vec::new()) }
	}
}

impl<T: Clone> Bus<T> {
	/// Every value published from now on. Dropping the receiver ends the
	/// subscription; the next publish forgets its sender.
	pub fn subscribe(&self) -> mpsc::UnboundedReceiver<T> {
		let (sender, receiver) = mpsc::unbounded();
		lock(&self.subscribers).push(sender);
		receiver
	}

	/// Send `value` to every live subscription.
	pub fn publish(&self, value: T) {
		lock(&self.subscribers).retain(|sender| sender.unbounded_send(value.clone()).is_ok());
	}
}

/// The core's listener for the wallet's accounts.
impl AccountsListener for Bus<Vec<ProductAccount>> {
	fn accounts_changed(&self, accounts: Vec<ProductAccount>) {
		self.publish(accounts);
	}
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
	mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
