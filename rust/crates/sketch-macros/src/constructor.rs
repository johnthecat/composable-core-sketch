//! `#[constructor]`: one declaration of a host crate's runtime constructor,
//! exported through UniFFI natively and wasm-bindgen on the web.
//!
//! The web export needs no TypeScript module of its own: each parameter is a
//! `Js<Trait>`, whose `.d.ts` name `#[extension_point]` declares.

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat};

use crate::type_path::{dyn_trait_name, single_argument};

/// Expands `#[constructor]` on a host crate's constructor function; it takes
/// no arguments.
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
	if !args.is_empty() {
		return syn::Error::new_spanned(args, "#[constructor] takes no arguments")
			.to_compile_error();
	}
	let function = match syn::parse2::<ItemFn>(item) {
		Ok(function) => function,
		Err(error) => return error.to_compile_error(),
	};
	match web_export(&function) {
		Ok(web) => quote! {
			#[cfg_attr(not(target_arch = "wasm32"), ::uniffi::export)]
			#function

			#web
		},
		Err(error) => error.to_compile_error(),
	}
}

/// The wasm-bindgen function that wraps each JS object in its `Foreign<Trait>`
/// and calls `function`. It throws, naming the method, when an object lacks
/// one.
fn web_export(function: &ItemFn) -> syn::Result<TokenStream> {
	let name = &function.sig.ident;
	let web_name = format_ident!("{name}_web");
	let js_name = name.to_string().to_lower_camel_case();
	let extension = quote!(::host_runtime::host::extension);
	let mut params = Vec::new();
	let mut args = Vec::new();
	for input in &function.sig.inputs {
		let FnArg::Typed(typed) = input else {
			return Err(syn::Error::new_spanned(input, "a constructor takes no `self`"));
		};
		let Pat::Ident(param) = typed.pat.as_ref() else {
			return Err(syn::Error::new_spanned(&typed.pat, "name each parameter"));
		};
		let param = &param.ident;
		let (optional, trait_name) = match single_argument(&typed.ty, "Option") {
			Some(inner) => (true, dyn_trait_name(inner)),
			None => (false, dyn_trait_name(&typed.ty)),
		};
		let Some(trait_name) = trait_name else {
			return Err(syn::Error::new_spanned(
				&typed.ty,
				"a constructor parameter is `Arc<dyn Trait>` or `Option<Arc<dyn Trait>>`",
			));
		};
		let js_type = format_ident!("Js{trait_name}");
		let foreign = format_ident!("Foreign{trait_name}");
		if optional {
			params.push(quote!(#param: ::core::option::Option<#extension::#js_type>));
			args.push(quote! {
				#param
					.map(#extension::#foreign::new)
					.transpose()
					.map_err(|reason| ::wasm_bindgen::JsError::new(&reason))?
					.map(|foreign| {
						::std::sync::Arc::new(foreign) as ::std::sync::Arc<dyn #extension::#trait_name>
					})
			});
		} else {
			params.push(quote!(#param: #extension::#js_type));
			args.push(quote! {
				::std::sync::Arc::new(
					#extension::#foreign::new(#param)
						.map_err(|reason| ::wasm_bindgen::JsError::new(&reason))?,
				)
			});
		}
	}
	let doc = format!("[`{name}`] for web hosts, each extension point a typed JS object.");
	Ok(quote! {
		#[cfg(target_arch = "wasm32")]
		#[doc = #doc]
		#[::wasm_bindgen::prelude::wasm_bindgen(js_name = #js_name)]
		pub fn #web_name(
			#(#params),*
		) -> ::core::result::Result<::host_runtime::WasmHostRuntime, ::wasm_bindgen::JsError> {
			::core::result::Result::Ok(::host_runtime::WasmHostRuntime::new(#name(#(#args),*)))
		}
	})
}
