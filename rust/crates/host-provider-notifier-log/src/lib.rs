//! A [`Notifier`](host_extension::notifier::Notifier) provider that writes each notice to the
//! log, the provider crate of the RFC's example.
//!
//! The crate depends on the interface crate only. `src/notifier.rs` implements
//! the trait; `src/wasm.rs` hands the provider to JS as a typed `Notifier`
//! object, and `src/native.rs` returns it as a trait object for UniFFI.

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

#[cfg(not(target_arch = "wasm32"))]
mod native;
mod notifier;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use notifier::LogNotifier;
