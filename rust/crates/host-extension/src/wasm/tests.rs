use js_sys::{Reflect, Uint8Array};
use protocol::{Bytes, Bytes32, Bytes64};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

use super::*;
use crate::wallet::WalletError;

// The generated TypeScript declares every byte value a `Uint8Array`. The
// byte types serialize themselves that way, and the attributes refuse any
// other byte representation, so one round trip per byte type covers every
// extension type.

/// `value` through the glue's conversion and back, with the bytes JS saw.
fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> (Vec<u8>, T) {
	let js = to_js::<T, String>(value).expect("converts to JS");
	let bytes = js.clone().dyn_into::<Uint8Array>().expect("a Uint8Array").to_vec();
	(bytes, from_js::<T, String>(js).expect("converts back"))
}

#[wasm_bindgen_test]
fn bytes32_crosses_as_a_uint8_array() {
	assert_eq!(round_trip(&Bytes32([1; 32])), (vec![1; 32], Bytes32([1; 32])));
}

#[wasm_bindgen_test]
fn bytes64_crosses_as_a_uint8_array() {
	assert_eq!(round_trip(&Bytes64([2; 64])), (vec![2; 64], Bytes64([2; 64])));
}

#[wasm_bindgen_test]
fn bytes_cross_as_a_uint8_array_and_their_absence_as_undefined() {
	let absent = to_js::<Option<Bytes>, String>(&None).expect("converts to JS");

	assert_eq!(
		(round_trip(&Bytes(vec![3, 4])), absent.is_undefined()),
		((vec![3, 4], Bytes(vec![3, 4])), true),
	);
}

#[wasm_bindgen_test]
fn a_value_that_does_not_convert_is_an_error_not_a_stand_in_value() {
	let converted = to_js::<u64, String>(&u64::MAX);

	assert!(
		converted
			.as_ref()
			.is_err_and(|reason| reason.starts_with("a value did not convert")),
		"{converted:?}",
	);
}

#[wasm_bindgen_test]
fn only_an_error_named_for_the_trait_is_declared() {
	let carrying_rejected = |name: &str| {
		let thrown = js_sys::Error::new("declined");
		thrown.set_name(name);
		let error = to_js::<_, String>(&WalletError::Rejected).expect("converts to JS");
		Reflect::set(&thrown, &JsValue::from_str("error"), &error).expect("sets the property");
		JsValue::from(thrown)
	};

	assert_eq!(
		(
			rejection::<WalletError>(carrying_rejected("WalletException"), "WalletException"),
			rejection::<WalletError>(carrying_rejected("TypeError"), "WalletException"),
		),
		(WalletError::Rejected, WalletError::Unknown { reason: "declined".into() }),
	);
}
