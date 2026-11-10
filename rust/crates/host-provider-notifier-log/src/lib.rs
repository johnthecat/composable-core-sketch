//! A [`Notifier`](host_extension::notifier::Notifier) provider that writes each notice to the
//! log.
//!
//! `README.md` describes its exports and its place in the demo.

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

#[cfg(not(target_arch = "wasm32"))]
mod native;
mod notifier;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use notifier::LogNotifier;
