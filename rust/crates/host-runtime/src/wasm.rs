//! The wasm-bindgen surface: [`WasmHostRuntime`], which a host crate's
//! [`crate::constructor`] returns, and the product runtimes it hands out. JS
//! sees them as `HostRuntime` and `ProductRuntime`. The types they take,
//! `ProductContext` and `FrameSink`, are the interface crate's, generated into
//! the same TypeScript module as the extension points.

use std::sync::Arc;

use host_extension::{
	from_js,
	product::{ForeignFrameSink, JsFrameSink, ProductContext},
};
use wasm_bindgen::prelude::*;

use crate::{HostRuntime, ProductRuntime};

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
	/// A runtime serving `product`, which sends its frames to `sink`.
	#[wasm_bindgen(js_name = productRuntime)]
	pub fn product_runtime(
		&self,
		#[wasm_bindgen(unchecked_param_type = "HostExtension.ProductContext")] product: JsValue,
		sink: JsFrameSink,
	) -> Result<WasmProductRuntime, JsError> {
		let product =
			from_js::<ProductContext, String>(product).map_err(|reason| JsError::new(&reason))?;
		let sink = ForeignFrameSink::new(sink).map_err(|reason| JsError::new(&reason))?;
		Ok(WasmProductRuntime {
			runtime: Arc::clone(&self.runtime).product_runtime(product, Arc::new(sink)),
		})
	}
}

/// One product's connection, as a web host drives it. `free()` ends it, and
/// drops every call still in flight.
#[wasm_bindgen(js_name = ProductRuntime)]
pub struct WasmProductRuntime {
	runtime: Arc<ProductRuntime>,
}

#[wasm_bindgen(js_class = ProductRuntime)]
impl WasmProductRuntime {
	/// Start serving one frame from the product. Responses and subscription
	/// items go to the sink.
	#[wasm_bindgen(js_name = receiveFrame)]
	pub fn receive_frame(&self, frame: Vec<u8>) -> Result<(), JsError> {
		self.runtime
			.receive_frame(frame)
			.map_err(|error| JsError::new(&error.to_string()))
	}
}
