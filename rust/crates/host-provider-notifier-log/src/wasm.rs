use std::sync::Arc;

use host_extension::notifier::{JsNotifier, WasmNotifier};
use wasm_bindgen::prelude::*;

use crate::LogNotifier;

/// The log notifier, writing to the browser console, for web host code to pass
/// to the runtime or wrap.
#[wasm_bindgen(js_name = logNotifier)]
pub fn log_notifier() -> JsNotifier {
	WasmNotifier::into_js(Arc::new(LogNotifier::new(|line| {
		web_sys::console::info_1(&JsValue::from_str(line));
	})))
}
