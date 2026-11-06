//! Hand-written helpers the generated wrappers in the parent module share.
//!
//! Values cross the WASM boundary as SCALE bytes in a `Uint8Array`. A declared
//! failure crosses as the encoded error type; any other rejection or exception is
//! a failure the trait does not declare, and becomes the error's `From<String>`.

use core::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use futures::{
	channel::mpsc,
	stream::{self, BoxStream, Stream, StreamExt},
};
use js_sys::{Array, Function, Promise, Reflect, Uint8Array};
use parity_scale_codec::{Decode, Encode};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;

/// SCALE-encodes `value` into a `Uint8Array`.
pub fn encode<T: Encode>(value: &T) -> JsValue {
	Uint8Array::from(value.encode().as_slice()).into()
}

/// Decodes `bytes` as `T`; a failure is an undeclared `E`.
pub fn decode<T: Decode, E: From<String>>(bytes: &Uint8Array) -> Result<T, E> {
	T::decode(&mut bytes.to_vec().as_slice())
		.map_err(|error| E::from(format!("argument did not decode: {error}")))
}

/// One method of a JS object, called with that object as `this`.
pub struct JsMethod(SendWrapper<(JsValue, Function)>);

impl JsMethod {
	/// Looks up the method `name` on `object`, failing when it is not a function.
	pub fn lookup(object: &JsValue, name: &str) -> Result<Self, JsValue> {
		let function = Reflect::get(object, &JsValue::from_str(name))?
			.dyn_into::<Function>()
			.map_err(|_| JsValue::from_str(&format!("{name} must be a function")))?;
		Ok(Self(SendWrapper::new((object.clone(), function))))
	}

	fn apply(&self, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
		let (this, function) = &*self.0;
		function.apply(this, &args.into_iter().collect::<Array>())
	}

	/// Calls the method and decodes the value its promise resolves to.
	pub fn call<T: Decode, E: Decode + From<String>>(
		&self,
		args: Vec<JsValue>,
	) -> impl Future<Output = Result<T, E>> + Send {
		let returned = self.apply(args);
		SendWrapper::new(async move {
			let promise = Promise::resolve(&returned.map_err(rejection::<E>)?);
			let resolved = JsFuture::from(promise).await.map_err(rejection::<E>)?;
			decode_value(resolved)
		})
	}

	/// Calls a subscribing method, which takes `onItem` and `onError` after `args`
	/// and returns a function that ends the subscription. Dropping the stream calls
	/// it.
	pub fn subscribe<T, E>(&self, mut args: Vec<JsValue>) -> BoxStream<'static, Result<T, E>>
	where
		T: Decode + Send + 'static,
		E: Decode + From<String> + Send + 'static,
	{
		let (sender, receiver) = mpsc::unbounded();
		let item_sender = sender.clone();
		let on_item = Closure::<dyn Fn(JsValue)>::new(move |value| {
			let item = decode_value(value);
			let failed = item.is_err();
			let _ = item_sender.unbounded_send(item);
			if failed {
				item_sender.close_channel();
			}
		});
		let on_error = Closure::<dyn Fn(JsValue)>::new(move |reason| {
			let _ = sender.unbounded_send(Err(rejection(reason)));
			sender.close_channel();
		});
		args.push(on_item.as_ref().clone());
		args.push(on_error.as_ref().clone());

		let dispose = match self.apply(args).map(JsCast::dyn_into::<Function>) {
			Ok(Ok(dispose)) => dispose,
			Ok(Err(_)) => {
				let error = E::from("a subscription must return a dispose function".to_string());
				return stream::once(async { Err(error) }).boxed();
			},
			Err(reason) => {
				let error = rejection(reason);
				return stream::once(async { Err(error) }).boxed();
			},
		};
		JsSubscription {
			receiver,
			_callbacks: SendWrapper::new((on_item, on_error)),
			dispose: SendWrapper::new(dispose),
		}
		.boxed()
	}
}

type Callbacks = (Closure<dyn Fn(JsValue)>, Closure<dyn Fn(JsValue)>);

struct JsSubscription<T> {
	receiver: mpsc::UnboundedReceiver<T>,
	_callbacks: SendWrapper<Callbacks>,
	dispose: SendWrapper<Function>,
}

impl<T> Stream for JsSubscription<T> {
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
		Pin::new(&mut self.receiver).poll_next(cx)
	}
}

impl<T> Drop for JsSubscription<T> {
	fn drop(&mut self) {
		let _ = self.dispose.call0(&JsValue::NULL);
	}
}

fn decode_value<T: Decode, E: From<String>>(value: JsValue) -> Result<T, E> {
	let bytes = value
		.dyn_into::<Uint8Array>()
		.map_err(|_| E::from("expected a Uint8Array".to_string()))?;
	decode(&bytes)
}

/// The error a rejection or exception carries: a declared failure when it is the
/// encoded error type, otherwise an undeclared one built from its text.
fn rejection<E: Decode + From<String>>(reason: JsValue) -> E {
	match reason.dyn_into::<Uint8Array>() {
		Ok(bytes) => E::decode(&mut bytes.to_vec().as_slice())
			.unwrap_or_else(|error| E::from(format!("rejection did not decode: {error}"))),
		Err(reason) => E::from(js_to_string(reason)),
	}
}

fn js_to_string(value: JsValue) -> String {
	value
		.as_string()
		.or_else(|| value.dyn_ref::<js_sys::Error>().map(|error| error.message().into()))
		.unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(feature = "export")]
pub use self::export::{export_async, export_stream};

#[cfg(feature = "export")]
mod export {
	use core::future::Future;

	use futures::{StreamExt, future, stream::BoxStream};
	use js_sys::{Function, Promise};
	use parity_scale_codec::Encode;
	use wasm_bindgen::{JsCast, JsValue, closure::Closure};

	use super::encode;

	/// Runs `future` and returns a promise that resolves with its encoded value or
	/// rejects with its encoded error.
	pub fn export_async<T: Encode, E: Encode>(
		future: impl Future<Output = Result<T, E>> + 'static,
	) -> Promise {
		wasm_bindgen_futures::future_to_promise(async move {
			future.await.map(|value| encode(&value)).map_err(|error| encode(&error))
		})
	}

	/// Opens a stream with `open` and passes each encoded item to `on_item`. An
	/// error goes to `on_error`, encoded, and ends the subscription, as does a
	/// callback that throws. Returns a function that ends it.
	pub fn export_stream<T: Encode + 'static, E: Encode + 'static>(
		open: impl FnOnce() -> Result<BoxStream<'static, Result<T, E>>, E>,
		on_item: Function,
		on_error: Function,
	) -> Function {
		let opened = open();
		let (pump, handle) = future::abortable(async move {
			let mut stream = match opened {
				Ok(stream) => stream,
				Err(error) => {
					let _ = on_error.call1(&JsValue::NULL, &encode(&error));
					return;
				},
			};
			while let Some(item) = stream.next().await {
				let delivered = match item {
					Ok(value) => on_item.call1(&JsValue::NULL, &encode(&value)).is_ok(),
					Err(error) => {
						let _ = on_error.call1(&JsValue::NULL, &encode(&error));
						false
					},
				};
				if !delivered {
					break;
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
