//! Interface crate of the core: everything a host supplies to the core is an
//! extension point declared here, one module per extension point, re-exported
//! from the crate root. The core re-exports this crate as
//! `host_runtime::host::extension`. A provider depends on this crate, never on
//! the core.
//!
//! Required by every host: [`ProductStorage`] and [`UserConfirmation`].
//! Optional: [`Wallet`] and [`Notifier`]; without one, the product calls that
//! need it fail as `Unsupported`.
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
//!   [`Wallet::accounts`], and its changes are pushed to a listener, such as [`AccountsListener`],
//!   which the core hands over once with [`Wallet::set_accounts_listener`]. A listener carries
//!   `#[extension_listener]`; its methods are notifications, which return nothing at once. On the
//!   web a JS failure in a notification is logged, because the caller has nowhere to receive it.
//! - The TypeScript package the interfaces are generated into is named once, by
//!   `typescript_module!` below.
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

sketch_macros::typescript_module!("@sketch/host");

mod confirmation;
mod notifier;
mod storage;
mod wallet;
#[cfg(target_arch = "wasm32")]
mod wasm;

// Each module's public items, the web glue's `Foreign<Trait>`, `Js<Trait>` and
// `Wasm<Trait>` included on wasm32, at the crate root.
pub use confirmation::*;
pub use notifier::*;
/// The byte types extension point values carry, re-exported so a provider needs
/// only this crate.
pub use protocol::{Bytes, Bytes32, Bytes64};
pub use storage::*;
pub use wallet::*;
