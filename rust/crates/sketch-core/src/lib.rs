//! The core. It owns the host runtime and reaches the wallet only through the
//! [`Wallet`](host::extension::Wallet) extension point, so it depends on the
//! interface crate and never on a provider.
//!
//! [`HostRuntime`] is the product-facing logic over the wallet. On the web the
//! crate builds to its own WASM binary, whose `HostRuntime` class takes the
//! host's wallet as a JS object and wraps it in `ForeignWallet`.

pub mod host {
	//! What the core declares for hosts to implement or attach.

	pub mod extension {
		//! The extension points: traits a provider or the host implements, and the
		//! types their methods name.

		pub use sketch_wallet_extension::*;
	}
}

mod runtime;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use runtime::HostRuntime;
