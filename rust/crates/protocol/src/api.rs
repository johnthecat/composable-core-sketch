//! The services a host serves to products, one trait per service.
//!
//! Each trait's wire id, set by `#[wire_trait(id = N)]`, and each method's, set
//! by `#[wire(id = N)]`, are permanent once released. The ids follow truapi's
//! table for the same services.
//!
//! | Id | Trait |
//! |----|-------|
//! | 2 | `Account` |
//! | 7 | `LocalStorage` |
//! | 8 | `Notifications` |
//! | 13 | `Signing` |
//!
//! `services!` names each service once:
//!
//! - its module;
//! - its trait;
//! - its place in [`methods`] and in [`ProductApi`].

/// Declares every service from one list of `module::Trait`.
///
/// It declares:
///
/// - the modules;
/// - the traits re-exported here;
/// - [`methods`] joining their method tables;
/// - [`ProductApi`] requiring them all.
macro_rules! services {
	($($module:ident :: $service:ident),+ $(,)?) => {
		$(
			pub mod $module;
			pub use $module::$service;
		)+

		/// The method table of every service, for a dispatcher to serve
		/// `host` with.
		pub fn methods<H: ProductApi + 'static>() -> Vec<crate::dispatch::Method<H>> {
			[$($module::methods()),+].into_iter().flatten().collect()
		}

		/// Every service a host serves to products.
		pub trait ProductApi: $($service +)+ Send + Sync {}

		impl<T> ProductApi for T where T: $($service +)+ Send + Sync {}
	};
}

services! {
	account::Account,
	local_storage::LocalStorage,
	notifications::Notifications,
	signing::Signing,
}
