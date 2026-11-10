//! What the web glue `#[extension_point]` writes is built on: calling a JS
//! object's async method, and handing a Rust future to JS as a promise.
//!
//! Values cross as typed JS values through serde:
//!
//! - records with camelCase fields;
//! - enums as `{ tag, value }`;
//! - the protocol's byte types as `Uint8Array`.
//!
//! A value that does not convert fails the call with the error's
//! `Unknown` variant. A declared error crosses as an `Error` named
//! `<Trait>Exception` whose `error` property holds it; any other rejection is a
//! failure the trait does not declare, and becomes the error's `From<String>`.
//!
//! A method that returns nothing, such as a listener's, is a notification: the
//! JS method is called and returns at once, and a failure is reported on the
//! console, since the caller has nowhere to receive it.

use js_sys::{Array, Function, Promise, Reflect};
use send_wrapper::SendWrapper;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// A JS object implementing an extension point.
pub struct JsObject(SendWrapper<JsValue>);

impl JsObject {
	/// Wraps `object`, an implementation of the extension point `trait_name`,
	/// or names the first of `methods` it lacks.
	pub fn new(object: JsValue, trait_name: &str, methods: &[&str]) -> Result<Self, String> {
		for method in methods {
			let found = Reflect::get(&object, &JsValue::from_str(method))
				.is_ok_and(|value| value.is_function());
			if !found {
				return Err(format!("the {trait_name} object has no method {method}"));
			}
		}
		Ok(Self(SendWrapper::new(object)))
	}

	/// Calls the method `name` with `args` and waits for its promise. A
	/// rejection becomes `E`; it is declared when it is an `Error` named
	/// `exception` carrying `error`. Dropping the returned future leaves the
	/// JS call running; its result is discarded.
	pub fn call<E: DeserializeOwned + From<String>>(
		&self,
		name: &'static str,
		args: Vec<JsValue>,
		exception: &'static str,
	) -> impl Future<Output = Result<JsValue, E>> + Send {
		let returned = self.apply(name, args);
		SendWrapper::new(async move {
			let promise =
				Promise::resolve(&returned.map_err(|reason| rejection::<E>(reason, exception))?);
			JsFuture::from(promise)
				.await
				.map_err(|reason| rejection::<E>(reason, exception))
		})
	}

	/// Calls the method `name` with the arguments `args` builds, without
	/// waiting. A failure, to build the arguments or in the call, is reported on
	/// the console. `args` is a closure so that the glue's `?` on each
	/// argument's conversion has a `Result` to return from in a method that
	/// returns nothing.
	pub fn send(&self, name: &'static str, args: impl FnOnce() -> Result<Vec<JsValue>, String>) {
		let outcome = args()
			.map_err(|reason| JsValue::from_str(&reason))
			.and_then(|args| self.apply(name, args));
		if let Err(reason) = outcome {
			web_sys::console::error_2(&JsValue::from_str(&format!("{name} failed:")), &reason);
		}
	}

	fn apply(&self, name: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
		let method = Reflect::get(&self.0, &JsValue::from_str(name))?
			.dyn_into::<Function>()
			.map_err(|_| JsValue::from_str(&format!("{name} is not a function")))?;
		method.apply(&self.0, &args.into_iter().collect::<Array>())
	}
}

/// `value` as a typed JS value, or the reason it did not convert as `E`.
pub fn to_js<T: Serialize, E: From<String>>(value: &T) -> Result<JsValue, E> {
	serde_wasm_bindgen::to_value(value)
		.map_err(|error| E::from(format!("a value did not convert: {error}")))
}

/// A typed JS value as `T`; a value of the wrong shape is an undeclared `E`.
pub fn from_js<T: DeserializeOwned, E: From<String>>(value: JsValue) -> Result<T, E> {
	serde_wasm_bindgen::from_value(value)
		.map_err(|error| E::from(format!("a value did not convert: {error}")))
}

/// A promise of what `future` returns. A declared error rejects as an `Error`
/// named `exception` whose `error` property holds the error.
pub fn export<E: Serialize + core::fmt::Display>(
	future: impl Future<Output = Result<JsValue, E>> + 'static,
	exception: &'static str,
) -> Promise {
	wasm_bindgen_futures::future_to_promise(async move {
		future.await.map_err(|error| declared(&error, exception))
	})
}

/// `error` as the `Error` a declared error rejects with. If `error` itself does
/// not convert, the rejection is an undeclared one carrying the reason.
fn declared<E: Serialize + core::fmt::Display>(error: &E, exception: &str) -> JsValue {
	let thrown = js_sys::Error::new(&error.to_string());
	match to_js::<E, String>(error) {
		Ok(value) => {
			thrown.set_name(exception);
			let _ = Reflect::set(&thrown, &JsValue::from_str("error"), &value);
		},
		Err(reason) => thrown.set_message(&format!("{error} ({reason})")),
	}
	thrown.into()
}

/// The error a rejection carries: the declared error when it is an `Error`
/// named `exception` whose `error` property is an `E`, or else an undeclared
/// one built from its message.
fn rejection<E: DeserializeOwned + From<String>>(reason: JsValue, exception: &str) -> E {
	let named = Reflect::get(&reason, &JsValue::from_str("name"))
		.is_ok_and(|name| name.as_string().as_deref() == Some(exception));
	if named &&
		let Ok(declared) = Reflect::get(&reason, &JsValue::from_str("error")) &&
		let Ok(error) = serde_wasm_bindgen::from_value(declared)
	{
		return error;
	}
	E::from(
		reason
			.dyn_ref::<js_sys::Error>()
			.map(|error| String::from(error.message()))
			.or_else(|| reason.as_string())
			.unwrap_or_else(|| format!("{reason:?}")),
	)
}

#[cfg(test)]
mod tests;
