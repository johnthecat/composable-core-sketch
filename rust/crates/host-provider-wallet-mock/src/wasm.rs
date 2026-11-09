use core::time::Duration;
use std::sync::Arc;

use host_extension::{JsWallet, WasmWallet};
use wasm_bindgen::prelude::*;

use crate::{MockWallet, PhoneDecision};

/// The simulated phone: hands out its wallet, and lets the demo act as the user
/// holding the phone.
#[wasm_bindgen(js_name = MockPhone)]
pub struct WasmMockPhone {
	wallet: Arc<MockWallet>,
}

#[wasm_bindgen(js_class = MockPhone)]
impl WasmMockPhone {
	/// A phone with no accounts that approves each sign request after
	/// `signDelayMs` milliseconds.
	#[wasm_bindgen(constructor)]
	pub fn new(sign_delay_ms: u32) -> Self {
		Self { wallet: Arc::new(MockWallet::new(Duration::from_millis(sign_delay_ms.into()))) }
	}

	/// The phone's wallet, a typed `Wallet` object for web host code to pass to
	/// the runtime or wrap.
	pub fn wallet(&self) -> JsWallet {
		WasmWallet::into_js(self.wallet.clone())
	}

	/// Adds an account named `name`; account subscriptions see the new list.
	#[wasm_bindgen(js_name = addAccount)]
	pub fn add_account(&self, name: &str) {
		self.wallet.add_account(name);
	}

	/// Sets how the phone answers sign requests, including those still pending.
	#[wasm_bindgen(js_name = setDecision)]
	pub fn set_decision(&self, decision: PhoneDecision) {
		self.wallet.set_decision(decision);
	}

	/// Sets how long the phone takes to answer sign requests made from now on.
	#[wasm_bindgen(js_name = setSignDelay)]
	pub fn set_sign_delay(&self, sign_delay_ms: u32) {
		self.wallet.set_sign_delay(Duration::from_millis(sign_delay_ms.into()));
	}
}
