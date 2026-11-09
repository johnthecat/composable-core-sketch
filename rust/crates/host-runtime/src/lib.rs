//! The core: serves the product protocol to each product connection over the
//! extension points the host supplies.
//!
//! A host builds one [`HostRuntime`] from its extension points, each an
//! `Arc<dyn Trait>`: storage and user confirmation always, a wallet and a
//! notifier when it has them. Then one [`ProductRuntime`] per product takes the
//! product's frames and answers through a [`FrameSink`]. The core depends on
//! the interface crate, re-exported as [`host::extension`], and never on a
//! provider.
//!
//! A host crate declares its constructor once with [`constructor`], which
//! exports it through UniFFI natively and wasm-bindgen on the web, where
//! `WasmHostRuntime` wraps the runtime.

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

pub mod host {
	//! What the core declares for hosts to implement or attach.

	pub mod extension {
		//! The extension points: traits a provider or the host implements, and
		//! the types they name.

		pub use host_extension::*;
	}
}

mod dispatcher;
mod frame;
mod host_core;
mod runtime;
mod subscription;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use frame::{
	MESSAGE_TYPE_INTERRUPT, MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_RESPONSE, MESSAGE_TYPE_STOP,
	Payload, ProtocolMessage,
};
pub use host_core::{FrameSink, HostRuntime, ProductContext, ProductRuntime, ProductRuntimeError};
/// Export a host crate's runtime constructor; see [`sketch_macros::constructor`].
pub use sketch_macros::constructor;
pub use subscription::{Spawner, platform_spawner};
#[cfg(target_arch = "wasm32")]
pub use wasm::{WasmHostRuntime, WasmProductRuntime};
