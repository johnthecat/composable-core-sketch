use core::time::Duration;
use std::sync::Arc;

use host_extension::wallet::Wallet;

use crate::{MockWallet, PhoneDecision};

/// The simulated phone: hands out its wallet, and lets an app act as the user
/// holding the phone.
#[derive(uniffi::Object)]
pub struct MockPhone {
	wallet: Arc<MockWallet>,
}

#[uniffi::export]
impl MockPhone {
	/// A phone with no accounts that approves each sign request after
	/// `sign_delay_ms` milliseconds.
	#[uniffi::constructor]
	pub fn new(sign_delay_ms: u32) -> Arc<Self> {
		Arc::new(Self {
			wallet: Arc::new(MockWallet::new(Duration::from_millis(sign_delay_ms.into()))),
		})
	}

	/// The phone's wallet, for the app to pass to the runtime or wrap.
	pub fn wallet(&self) -> Arc<dyn Wallet> {
		self.wallet.clone()
	}

	/// Adds an account named `name`; account subscriptions see the new list.
	pub fn add_account(&self, name: String) {
		self.wallet.add_account(&name);
	}

	/// Sets how the phone answers sign requests, including those still pending.
	pub fn set_decision(&self, decision: PhoneDecision) {
		self.wallet.set_decision(decision);
	}

	/// Sets how long the phone takes to answer sign requests made from now on.
	pub fn set_sign_delay(&self, sign_delay_ms: u32) {
		self.wallet.set_sign_delay(Duration::from_millis(sign_delay_ms.into()));
	}
}
