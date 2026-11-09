//! The wasm-bindgen surface: [`WasmHostRuntime`], which a host crate's
//! [`crate::constructor`] returns, and the product runtimes it hands out. JS
//! sees them as `HostRuntime` and `ProductRuntime`, and the types they take
//! are declared once, in `@sketch/host`'s `runtime.ts`.

use std::sync::Arc;

use js_sys::{Function, Reflect, Uint8Array};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, prelude::*};

use crate::{FrameSink, HostRuntime, ProductContext, ProductRuntime};

/// A JS `FrameSink`: sends a product runtime's frames to its `emitFrame`.
struct JsFrameSink {
	emit_frame: SendWrapper<Function>,
}

impl FrameSink for JsFrameSink {
	fn emit_frame(&self, frame: Vec<u8>) {
		let frame = Uint8Array::from(frame.as_slice());
		if let Err(error) = self.emit_frame.call1(&JsValue::NULL, &frame) {
			web_sys::console::error_1(&error);
		}
	}
}

/// The host runtime, as a web host holds it.
#[wasm_bindgen(js_name = HostRuntime)]
pub struct WasmHostRuntime {
	runtime: Arc<HostRuntime>,
}

impl WasmHostRuntime {
	/// Hands `runtime` to JS.
	pub fn new(runtime: Arc<HostRuntime>) -> Self {
		Self { runtime }
	}
}

#[wasm_bindgen(js_class = HostRuntime)]
impl WasmHostRuntime {
	/// A runtime serving `product`, which sends its frames to
	/// `sink.emitFrame`.
	#[wasm_bindgen(js_name = productRuntime)]
	pub fn product_runtime(
		&self,
		#[wasm_bindgen(unchecked_param_type = "HostExtension.Product")] product: JsValue,
		#[wasm_bindgen(unchecked_param_type = "HostExtension.FrameSink")] sink: JsValue,
	) -> Result<WasmProductRuntime, JsValue> {
		let product_id = Reflect::get(&product, &JsValue::from_str("productId"))?
			.as_string()
			.ok_or_else(|| JsValue::from_str("product.productId must be a string"))?;
		let emit_frame = Reflect::get(&sink, &JsValue::from_str("emitFrame"))?
			.dyn_into::<Function>()
			.map_err(|_| JsValue::from_str("sink.emitFrame must be a function"))?;
		let sink = Arc::new(JsFrameSink { emit_frame: SendWrapper::new(emit_frame) });
		Ok(WasmProductRuntime {
			runtime: self.runtime.product_runtime(ProductContext { product_id }, sink),
		})
	}
}

/// One product's connection, as a web host drives it. `free()` ends it.
#[wasm_bindgen(js_name = ProductRuntime)]
pub struct WasmProductRuntime {
	runtime: Arc<ProductRuntime>,
}

#[wasm_bindgen(js_class = ProductRuntime)]
impl WasmProductRuntime {
	/// Serve one frame from the product. Responses and subscription items go
	/// to `emitFrame`.
	#[wasm_bindgen(js_name = receiveFrame)]
	pub async fn receive_frame(&self, frame: Vec<u8>) -> Result<(), JsValue> {
		self.runtime
			.receive_frame(frame)
			.await
			.map_err(|error| JsValue::from_str(&error.to_string()))
	}
}
