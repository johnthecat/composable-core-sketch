//! `#[extension_listener]`: a trait an extension point pushes changes to, as
//! UniFFI exports it, and its web glue.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemTrait, ReturnType, TraitItem};

use crate::web_glue::{Method, Shape, web_glue};

/// Expands `#[extension_listener]` on `item`, which takes no arguments.
pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	crate::no_arguments(&args, "extension_listener")?;
	let item = syn::parse2::<ItemTrait>(item)?;
	let methods = item.items.iter().map(method).collect::<syn::Result<Vec<_>>>()?;
	let glue = web_glue(&item.ident, &methods);
	Ok(quote! {
		#[cfg_attr(not(target_arch = "wasm32"), ::uniffi::export(rust, foreign))]
		#item

		#glue
	})
}

/// A notification: `fn(&self, owned values)` returning nothing.
fn method(item: &TraitItem) -> syn::Result<Method<'_>> {
	let TraitItem::Fn(method) = item else {
		return Err(syn::Error::new_spanned(item, "a listener declares methods only"));
	};
	let shape_error = || {
		syn::Error::new_spanned(
			&method.sig,
			"a listener method is a notification, `fn(&self, owned values)` returning nothing",
		)
	};
	if method.sig.asyncness.is_some() || !matches!(method.sig.output, ReturnType::Default) {
		return Err(shape_error());
	}
	let notification = Method::new(method, Shape::Notification, shape_error)?;
	if let Some(param) = notification.params.iter().find(|param| param.interface.is_some()) {
		return Err(syn::Error::new_spanned(param.ty, "a listener method takes values only"));
	}
	Ok(notification)
}
