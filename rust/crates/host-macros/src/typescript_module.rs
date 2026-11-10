//! `typescript_module!`: where an interface crate's TypeScript interfaces live,
//! named once for every `#[extension_point]` in the crate.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::LitStr;

use crate::{TYPESCRIPT_MODULE_DECLARATION, TYPESCRIPT_NAMESPACE};

/// Expands `typescript_module!("<module>")`.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
	let module = syn::parse2::<LitStr>(input)?;
	let declaration = format_ident!("{TYPESCRIPT_MODULE_DECLARATION}");
	let import = format!("import type * as {TYPESCRIPT_NAMESPACE} from \"{}\";", module.value());
	// wasm-bindgen consumes the custom section's constant, so the module is
	// also kept in a plain one that `#[extension_point]` can name.
	Ok(quote! {
		#[cfg(target_arch = "wasm32")]
		#[::wasm_bindgen::prelude::wasm_bindgen(typescript_custom_section)]
		const _: &str = #import;

		/// The TypeScript module every `Js<Trait>` of this crate is typed from.
		#[cfg(target_arch = "wasm32")]
		const #declaration: &str = #module;
	})
}
