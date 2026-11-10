//! `#[wire]` and `#[wire_trait]`: the wire ids of protocol methods and traits.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	Ident, ItemTrait, LitInt, Token, TraitItemFn,
	parse::{Parse, ParseStream},
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

/// Checks a trait method's `id = N` and leaves the method as it is.
pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	syn::parse2::<WireId>(args)?;
	let method = syn::parse2::<TraitItemFn>(item)
		.map_err(|error| syn::Error::new(error.span(), "#[wire] applies to a trait method"))?;
	Ok(quote!(#method))
}

/// The trait, followed by its method table.
pub fn expand_trait(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	let WireId(id) = syn::parse2(args)?;
	let item_trait = syn::parse2::<ItemTrait>(item)
		.map_err(|error| syn::Error::new(error.span(), "#[wire_trait] applies to a trait"))?;
	let table = crate::method_table::method_table(&item_trait, id)?;
	Ok(quote!(#item_trait #table))
}
