use std::sync::Arc;

use futures::{StreamExt, future};
use js_sys::{Function, Promise, Uint8Array};
use parity_scale_codec::Encode;
use wasm_bindgen::prelude::*;

use crate::{
	HostRuntime,
	host::extension::{ForeignWallet, WalletError},
};

/// The host runtime, as the web host builds it. Results resolve to SCALE bytes,
/// and failures reject with an encoded `WalletError`.
#[wasm_bindgen(js_name = HostRuntime)]
pub struct WasmHostRuntime {
	runtime: Arc<HostRuntime>,
}

#[wasm_bindgen(js_class = HostRuntime)]
impl WasmHostRuntime {
	/// A runtime over `wallet`, a JS object implementing the raw `Wallet`
	/// interface: a provider's `WasmWallet`, or host code around one.
	#[wasm_bindgen(constructor)]
	pub fn new(
		#[wasm_bindgen(unchecked_param_type = "object")] wallet: JsValue,
	) -> Result<Self, JsValue> {
		let wallet = ForeignWallet::new(&wallet)?;
		Ok(Self { runtime: Arc::new(HostRuntime::new(Arc::new(wallet))) })
	}

	/// The accounts a product may sign with, as an encoded `Vec<Account>`.
	#[wasm_bindgen(unchecked_return_type = "Promise<Uint8Array>")]
	pub fn accounts(&self) -> Promise {
		let runtime = Arc::clone(&self.runtime);
		settle(async move { runtime.accounts().await })
	}

	/// Signs `message` with the account `publicKey`, resolving to an encoded
	/// `Signature`.
	#[wasm_bindgen(js_name = signMessage, unchecked_return_type = "Promise<Uint8Array>")]
	pub fn sign_message(&self, public_key: Vec<u8>, message: String) -> Promise {
		let runtime = Arc::clone(&self.runtime);
		settle(async move {
			let public_key = <[u8; 32]>::try_from(public_key)
				.map_err(|_| WalletError::from("a public key has 32 bytes".to_string()))?;
			runtime.sign_message(public_key, &message).await
		})
	}

	/// Passes each account list, encoded, to `onItem`. An error goes to `onError`,
	/// encoded, and ends the subscription. Returns a function that ends it.
	#[wasm_bindgen(js_name = subscribeAccounts, unchecked_return_type = "() => void")]
	pub fn subscribe_accounts(
		&self,
		#[wasm_bindgen(unchecked_param_type = "(item: Uint8Array) => void")] on_item: Function,
		#[wasm_bindgen(unchecked_param_type = "(error: Uint8Array) => void")] on_error: Function,
	) -> Function {
		let mut accounts = self.runtime.subscribe_accounts();
		let (pump, handle) = future::abortable(async move {
			while let Some(item) = accounts.next().await {
				match item {
					Ok(accounts) => {
						if on_item.call1(&JsValue::NULL, &encode(&accounts)).is_err() {
							return;
						}
					},
					Err(error) => {
						let _ = on_error.call1(&JsValue::NULL, &encode(&error));
						return;
					},
				}
			}
		});
		wasm_bindgen_futures::spawn_local(async move {
			let _ = pump.await;
		});
		Closure::<dyn Fn()>::new(move || handle.abort())
			.into_js_value()
			.unchecked_into()
	}
}

fn encode<T: Encode>(value: &T) -> JsValue {
	Uint8Array::from(value.encode().as_slice()).into()
}

fn settle<T: Encode>(result: impl Future<Output = Result<T, WalletError>> + 'static) -> Promise {
	wasm_bindgen_futures::future_to_promise(async move {
		result.await.map(|value| encode(&value)).map_err(|error| encode(&error))
	})
}
