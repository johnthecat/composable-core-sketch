//! Interface crate of the core: everything a host supplies to the core is an
//! extension point declared here, one public module per extension point, such
//! as [`wallet`], which holds its trait, its error, the types it names and,
//! on `wasm32`, its web glue. The core re-exports this crate as
//! `host_runtime::host::extension`. A provider depends on this crate, never on
//! the core.
//!
//! Required by every host: [`ProductStorage`](storage::ProductStorage) and
//! [`UserConfirmation`](confirmation::UserConfirmation), and for each product it frames a
//! [`ProductContext`](product::ProductContext) and a [`FrameSink`](product::FrameSink).
//! Optional: [`Wallet`](wallet::Wallet) and [`Notifier`](notifier::Notifier); without one, the
//! product calls that need it fail as `Unsupported`.
//!
//! # Declaring an extension point
//!
//! - The trait carries `#[extension_point]`. Its methods are calls, `async fn(&self, owned values)
//!   -> Result<T, <Trait>Error>`, and, for state the core follows, one `fn
//!   set_<value>_listener(&self, Arc<dyn L>)`.
//! - `<Trait>Error` carries `#[extension_error]` and lists only the failures the trait declares;
//!   the attribute adds `Unknown { reason }` for every other failure.
//! - A record or enum the trait names carries `#[extension_type]`. Values are plain data, never
//!   live objects. Bytes are the protocol's `Bytes`, `Bytes32` or `Bytes64`; the attributes refuse
//!   `Vec<u8>` and `[u8; N]`, which would reach JS as an array of numbers.
//! - No call stays pending to deliver changes. State the core follows has a plain read, such as
//!   [`Wallet::accounts`](wallet::Wallet::accounts), and its changes are pushed to a listener, such
//!   as [`AccountsListener`](wallet::AccountsListener), which the core hands over once with
//!   [`Wallet::set_accounts_listener`](wallet::Wallet::set_accounts_listener). A listener carries
//!   `#[extension_listener]`; its methods are notifications, which return nothing at once. On the
//!   web a JS failure in a notification is logged, because the caller has nowhere to receive it.
//! - The TypeScript package the interfaces are generated into is named once, by
//!   `typescript_module!` below.
//!
//! # Threading
//!
//! Natively the core runs every product call on one executor thread that all
//! host runtimes share, and calls each extension point from there; on the web
//! everything runs on the page's one thread. Blocking that thread stalls every
//! product, not just the call being served. So:
//!
//! - An async method may keep its future pending as long as it needs, but it suspends rather than
//!   blocks the thread that polls it. Swift and Kotlin implementations bridged through UniFFI
//!   suspend naturally; the rule chiefly binds Rust implementations, which only await.
//! - A sync method, a listener's notification, `set_<value>_listener` or `FrameSink::emit_frame`,
//!   returns promptly: it hands its value on, and never waits for the user or for the core.
//! - An implementation never holds a lock across an `.await`, and never waits on the core from
//!   inside a call the core made.
//!
//! # Cancellation
//!
//! When the core drops a call, a Swift or Kotlin implementation's task is
//! cancelled. A JS promise cannot be cancelled, so a JS implementation runs
//! on and its result is discarded, as is the result of a Rust implementation
//! that a Swift, Kotlin or JS wrapper awaits.
//!
//! The attributes add the UniFFI export for Swift and Kotlin and the web glue
//! (`Js<Trait>`, `Foreign<Trait>`, `Wasm<Trait>`, built over the helpers in the
//! `wasm` module); `tool-codegen` writes the TypeScript interfaces from this
//! crate's rustdoc JSON.

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

host_macros::typescript_module!("@sketch/host-extension");

pub mod confirmation;
pub mod notifier;
pub mod product;
pub mod storage;
pub mod wallet;
#[cfg(target_arch = "wasm32")]
mod wasm;

/// The byte types extension point values carry, re-exported so a provider needs
/// only this crate.
pub use protocol::{Bytes, Bytes32, Bytes64};
/// A typed JS value as a Rust value, for the core's own web surface, which
/// takes a [`ProductContext`](product::ProductContext) as JS hands it over.
#[cfg(target_arch = "wasm32")]
pub use wasm::from_js;
