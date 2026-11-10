//! Wire ids of protocol traits and methods.
//!
//! The ids are emitted as hidden doc tags, so they survive into rustdoc JSON for
//! `tool-codegen`. Rust rejects unknown helper attributes on trait methods; doc tags
//! carry the metadata without one.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
	Ident, ItemTrait, LitInt, Token, TraitItemFn,
	parse::{Parse, ParseStream},
	parse_macro_input,
};

/// The `id = N` argument both attributes take.
pub struct WireId(pub u8);

impl Parse for WireId {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let key: Ident = input.parse()?;
		if key != "id" {
			return Err(syn::Error::new(key.span(), "expected `id = N`"));
		}
		input.parse::<Token![=]>()?;
		let lit: LitInt = input.parse()?;
		let id = lit.base10_parse().map_err(|err| {
			syn::Error::new(lit.span(), format!("a wire id must fit in a u8: {err}"))
		})?;
		if !input.is_empty() {
			return Err(input.error("expected a single `id = N` argument"));
		}
		Ok(Self(id))
	}
}

/// Re-emit a trait method with its hidden `@wire_id=N` tag.
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
	let WireId(id) = parse_macro_input!(args as WireId);
	let tag = format!("@wire_id={id}");
	match syn::parse::<TraitItemFn>(item) {
		Ok(mut method) => {
			method.attrs.push(syn::parse_quote!(#[doc = #tag]));
			quote!(#method).into()
		},
		Err(_) => syn::Error::new(
			proc_macro2::Span::call_site(),
			"#[wire] can only be applied to trait methods",
		)
		.to_compile_error()
		.into(),
	}
}

/// Re-emit a trait with its hidden `@wire_trait_id=N` tag, followed by its method
/// table.
pub fn expand_trait(args: TokenStream, item: TokenStream) -> TokenStream {
	let WireId(id) = parse_macro_input!(args as WireId);
	let tag = format!("@wire_trait_id={id}");
	match syn::parse::<ItemTrait>(item) {
		Ok(mut item_trait) => {
			let table = match crate::method_table::method_table(&item_trait, id) {
				Ok(table) => table,
				Err(error) => return error.to_compile_error().into(),
			};
			item_trait.attrs.push(syn::parse_quote!(#[doc = #tag]));
			quote!(#item_trait #table).into()
		},
		Err(_) => syn::Error::new(
			proc_macro2::Span::call_site(),
			"#[wire_trait] can only be applied to traits",
		)
		.to_compile_error()
		.into(),
	}
}
