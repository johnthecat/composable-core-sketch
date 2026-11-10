//! Proc macros of the sketch, in one crate as truapi has one `truapi-macros`.
//!
//! The protocol crate uses:
//!
//! - [`macro@wire_trait`] and [`macro@wire`]: wire ids, and each trait's method table;
//! - [`macro@protocol_record`]: a payload record an extension point also carries.
//!
//! The interface crate, `host-extension`, uses:
//!
//! - [`macro@extension_point`] on a trait the core calls;
//! - [`macro@extension_listener`] on a trait an extension point pushes changes to;
//! - [`macro@extension_error`] on an extension point's error;
//! - [`macro@extension_type`] on a record or enum the traits name;
//! - [`typescript_module!`] once, at its root.
//!
//! A host crate uses [`macro@constructor`], re-exported as `host_runtime::constructor`.
//!
//! Each macro's implementation lives in its own module, and one module reads types out of
//! signatures for all of them. The generated code names `::uniffi`, `::async_trait`, `::serde`,
//! `::thiserror`, `::wasm_bindgen`, `::js_sys`, `::parity_scale_codec` and `::futures`, which the
//! crate using a macro depends on. The protocol macros name `crate::dispatch`, and the extension
//! macros the interface crate's `crate::wasm` helpers, so each is for use inside its own crate.

mod constructor;
mod extension_error;
mod extension_listener;
mod extension_point;
mod extension_type;
mod method_table;
mod type_path;
mod typescript_module;
mod web_glue;
mod wire;

use proc_macro::TokenStream;

/// Mark a protocol trait method with its wire id within the trait.
///
/// ```ignore
/// #[wire(id = 0)]
/// async fn sign_raw(...) -> ...;
/// ```
///
/// One id addresses every frame of the method; which leg a frame carries is
/// its `message_type` byte. Expands to the method plus a hidden `@wire_id=N` doc
/// tag that `tool-codegen` reads from rustdoc JSON.
#[proc_macro_attribute]
pub fn wire(args: TokenStream, item: TokenStream) -> TokenStream {
	wire::expand(args, item)
}

/// Mark a protocol trait with its wire id, the first byte of the
/// `(trait, method)` pair every frame of its methods carries.
///
/// ```ignore
/// #[wire_trait(id = 13)]
/// pub trait Signing: Send + Sync { ... }
/// ```
///
/// Next to the trait it writes a `<TRAIT>_<METHOD>` wire pair constant per
/// method and a `methods` function: the method table a dispatcher serves,
/// decoding each request, calling the trait and encoding the result. So a
/// module holds one service. Each method must carry `#[wire(id = N)]`, take
/// `&self` and at most one request, and return `Result<_, CallError<_>>` or
/// `Subscription<_, CallError<_>>`. A method without a request takes an empty
/// payload.
#[proc_macro_attribute]
pub fn wire_trait(args: TokenStream, item: TokenStream) -> TokenStream {
	wire::expand_trait(args, item)
}

/// Mark a protocol payload record that an extension point also carries. It
/// adds `uniffi::Record` natively and, on `wasm32`, the serde shape the web
/// glue converts with: the same attributes [`macro@extension_type`] adds to a
/// record, so the protocol copies none by hand.
#[proc_macro_attribute]
pub fn protocol_record(args: TokenStream, item: TokenStream) -> TokenStream {
	extension_type::expand_record(args.into(), item.into()).into()
}

/// Declare an extension point: a trait the core calls and a provider, or host
/// code, implements.
///
/// A method is one of two shapes:
///
/// - a call, `async fn(&self, owned values) -> Result<T, <Trait>Error>`, where the error is the
///   trait's own [`macro@extension_error`];
/// - a notification handing over a listener, `fn set_<value>_listener(&self, listener: Arc<dyn
///   L>)`, where `L` is an [`macro@extension_listener`].
///
/// Values never hold `Vec<u8>` or `[u8; N]`; bytes are the protocol's `Bytes`, `Bytes32` or
/// `Bytes64`. The crate names the TypeScript module of its interfaces once, with
/// [`typescript_module!`]; without it the crate fails to build for `wasm32`, naming the missing
/// `HOST_EXTENSION_TYPESCRIPT_MODULE`. The attribute adds:
///
/// - `uniffi::export(rust, foreign)` on native targets, so Swift and Kotlin can implement the trait
///   and call a Rust implementation;
/// - `async_trait`;
/// - on `wasm32`: `Js<Trait>`, the JS object type, typed as `HostExtension.<Trait>` in the
///   generated `.d.ts`; `Foreign<Trait>`, which implements the trait over such an object for the
///   core and refuses an object missing a method; and `Wasm<Trait>::into_js`, which hands a Rust
///   implementation to JS as one. Values cross as typed JS values. A declared error crosses as an
///   `Error` named `<Trait>Exception` whose `error` property holds it.
#[proc_macro_attribute]
pub fn extension_point(args: TokenStream, item: TokenStream) -> TokenStream {
	extension_point::expand(args.into(), item.into()).into()
}

/// Declare a listener: a trait an extension point pushes changes to, which the
/// core implements, and so does host code around a provider that wants to see
/// them.
///
/// Every method is a notification, `fn(&self, owned values)` returning
/// nothing, which returns at once. The attribute adds what
/// [`macro@extension_point`] adds. On the web a notification calls the JS
/// method without waiting, and a JS failure in it is logged on the console,
/// because the caller has nowhere to receive it.
#[proc_macro_attribute]
pub fn extension_listener(args: TokenStream, item: TokenStream) -> TokenStream {
	extension_listener::expand(args.into(), item.into()).into()
}

/// Declare an extension point's error, named `<Trait>Error`. Write only the
/// failures the trait declares; the attribute adds the `Unknown { reason }`
/// variant for every other failure, `From<String>` and, natively,
/// `From<uniffi::UnexpectedUniFFICallbackError>` into it, and the derives an
/// error needs on each platform, `Debug` and `thiserror::Error` included.
/// Each declared variant carries its own `#[error("…")]` message.
#[proc_macro_attribute]
pub fn extension_error(args: TokenStream, item: TokenStream) -> TokenStream {
	extension_error::expand(args.into(), item.into()).into()
}

/// Declare a record or enum an extension point names. The attribute adds the
/// UniFFI derive natively, and the serde derives the web glue converts it with:
/// camelCase fields and enums as `{ tag, value }`. It refuses a field holding
/// `Vec<u8>` or `[u8; N]`. Other derives are the type's own.
#[proc_macro_attribute]
pub fn extension_type(args: TokenStream, item: TokenStream) -> TokenStream {
	extension_type::expand(args.into(), item.into()).into()
}

/// Name the TypeScript module an interface crate's extension points are
/// declared in, once, at the crate root: `typescript_module!("@sketch/host-extension");`.
///
/// On `wasm32` it adds one line to the `.d.ts` of every WASM binary linking
/// the crate, `import type * as HostExtension from "<module>";`, through
/// which each `Js<Trait>` is typed as `HostExtension.<Trait>`. Natively it
/// adds nothing. Every [`macro@extension_point`] and
/// [`macro@extension_listener`] names the constant it declares, so the crate
/// cannot leave it out.
#[proc_macro]
pub fn typescript_module(input: TokenStream) -> TokenStream {
	typescript_module::expand(input.into()).into()
}

/// Export a host crate's runtime constructor: through UniFFI natively, and
/// through wasm-bindgen on the web, where each `Arc<dyn module::Trait>` parameter
/// becomes the typed JS object `HostExtension.Trait` and each
/// `Option<Arc<dyn Trait>>` an optional one. The function returns
/// `Arc<host_runtime::HostRuntime>`. The web constructor throws, naming the
/// method, when an object lacks one.
#[proc_macro_attribute]
pub fn constructor(args: TokenStream, item: TokenStream) -> TokenStream {
	constructor::expand(args.into(), item.into()).into()
}

/// The namespace the `.d.ts` imports an interface crate's TypeScript module
/// as.
const TYPESCRIPT_NAMESPACE: &str = "HostExtension";

/// The constant `typescript_module!` declares at the crate root, which every
/// extension point and listener names.
const TYPESCRIPT_MODULE_DECLARATION: &str = "HOST_EXTENSION_TYPESCRIPT_MODULE";
