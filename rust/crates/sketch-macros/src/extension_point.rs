//! `#[extension_point]`: a trait the core calls, as UniFFI exports it, and its
//! web glue.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemTrait, ReturnType, TraitItem};

use crate::{
	type_path::{dyn_trait_name, last_segment, reject_raw_bytes, result_types},
	web_glue::{Method, Shape, web_glue},
};

/// Expands `#[extension_point]` on `item`, which takes no arguments.
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
	if !args.is_empty() {
		return syn::Error::new_spanned(args, "#[extension_point] takes no arguments")
			.to_compile_error();
	}
	let item = match syn::parse2::<ItemTrait>(item) {
		Ok(item) => item,
		Err(error) => return error.to_compile_error(),
	};
	let error_name = format!("{}Error", item.ident);
	let exception = format!("{}Exception", item.ident);
	let methods = item
		.items
		.iter()
		.map(|method| self::method(method, &error_name, &exception))
		.collect::<syn::Result<Vec<_>>>();
	let methods = match methods {
		Ok(methods) => methods,
		Err(error) => return error.to_compile_error(),
	};
	let glue = web_glue(&item.ident, &methods);
	quote! {
		#[cfg_attr(not(target_arch = "wasm32"), ::uniffi::export(rust, foreign))]
		#[::async_trait::async_trait]
		#item

		#glue
	}
}

/// A call, `async fn … -> Result<T, <Trait>Error>`, or a listener handed over
/// with `fn set_<value>_listener(&self, listener: Arc<dyn L>)`.
fn method<'a>(item: &'a TraitItem, error_name: &str, exception: &str) -> syn::Result<Method<'a>> {
	let TraitItem::Fn(method) = item else {
		return Err(syn::Error::new_spanned(item, "an extension point declares methods only"));
	};
	let shape_error = || {
		syn::Error::new_spanned(
			&method.sig,
			format!(
				"an extension point method is a call, `async fn(&self, owned values) -> \
				 Result<T, {error_name}>`, or hands over a listener, \
				 `fn set_<value>_listener(&self, listener: Arc<dyn Listener>)`"
			),
		)
	};
	let shape = if method.sig.asyncness.is_some() {
		let (ok, error) = result_types(&method.sig.output).ok_or_else(shape_error)?;
		if !last_segment(error).is_some_and(|(name, _)| name == error_name) {
			return Err(syn::Error::new_spanned(
				error,
				format!("an extension point's error is its own `{error_name}`"),
			));
		}
		reject_raw_bytes(ok)?;
		Shape::Call { ok, error, exception: exception.to_string() }
	} else {
		let name = method.sig.ident.to_string();
		let listener = match method.sig.inputs.iter().nth(1) {
			Some(FnArg::Typed(typed)) => dyn_trait_name(&typed.ty),
			_ => None,
		};
		let hands_over_a_listener = name.starts_with("set_") &&
			name.ends_with("_listener") &&
			method.sig.inputs.len() == 2 &&
			listener.is_some() &&
			matches!(method.sig.output, ReturnType::Default);
		if !hands_over_a_listener {
			return Err(shape_error());
		}
		Shape::Notification
	};
	Method::new(method, shape, shape_error)
}
