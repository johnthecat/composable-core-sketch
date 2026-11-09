//! UniFFI's binding generator at the workspace's UniFFI version, so generating
//! the Swift and Kotlin bindings needs no globally installed copy.

fn main() {
	uniffi::uniffi_bindgen_main();
}
