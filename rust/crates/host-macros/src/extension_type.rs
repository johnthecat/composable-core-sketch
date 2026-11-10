//! `#[extension_type]` and `#[protocol_record]`: the derives a record or enum
//! needs to cross an extension point on every platform.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Item, parse_quote};

use crate::type_path::reject_raw_bytes;

/// What a value type is, which picks its UniFFI derive.
pub enum ValueKind {
	/// A struct: `uniffi::Record`.
	Record,
	/// An enum: `uniffi::Enum`.
	Enum,
	/// An error enum: `uniffi::Error`, and `thiserror::Error` with the `Debug`
	/// it requires.
	Error,
}

/// Expands `#[extension_type]` on a struct or enum; it takes no arguments.
pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	crate::no_arguments(&args, "extension_type")?;
	match syn::parse2::<Item>(item)? {
		item @ Item::Struct(_) => add_attributes(item, ValueKind::Record),
		item @ Item::Enum(_) => add_attributes(item, ValueKind::Enum),
		other => {
			Err(syn::Error::new_spanned(other, "#[extension_type] applies to a struct or enum"))
		},
	}
}

/// Expands `#[protocol_record]` on a struct; it takes no arguments.
pub fn expand_record(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	crate::no_arguments(&args, "protocol_record")?;
	match syn::parse2::<Item>(item)? {
		item @ Item::Struct(_) => add_attributes(item, ValueKind::Record),
		other => Err(syn::Error::new_spanned(other, "#[protocol_record] applies to a struct")),
	}
}

/// `item` with the derives of a `kind` value, or an error naming a field
/// that holds `Vec<u8>` or `[u8; N]`. Byte fields use the protocol's byte
/// types, which serialize themselves as bytes.
pub fn add_attributes(mut item: Item, kind: ValueKind) -> syn::Result<TokenStream> {
	let fields: Vec<&Fields> = match &item {
		Item::Struct(item) => vec![&item.fields],
		Item::Enum(item) => item.variants.iter().map(|variant| &variant.fields).collect(),
		_ => Vec::new(),
	};
	fields.into_iter().flatten().try_for_each(|field| reject_raw_bytes(&field.ty))?;
	let uniffi_derive = match kind {
		ValueKind::Record => quote!(::uniffi::Record),
		ValueKind::Enum => quote!(::uniffi::Enum),
		ValueKind::Error => quote!(::uniffi::Error),
	};
	let serde_shape = match kind {
		ValueKind::Record => quote!(rename_all = "camelCase"),
		ValueKind::Enum | ValueKind::Error => {
			quote!(tag = "tag", content = "value", rename_all_fields = "camelCase")
		},
	};
	let mut attributes: Vec<syn::Attribute> = vec![
		parse_quote!(#[cfg_attr(not(target_arch = "wasm32"), derive(#uniffi_derive))]),
		parse_quote!(#[cfg_attr(
			target_arch = "wasm32",
			derive(::serde::Serialize, ::serde::Deserialize),
			serde(#serde_shape)
		)]),
	];
	match kind {
		ValueKind::Error => {
			attributes.insert(0, parse_quote!(#[derive(Debug, ::thiserror::Error)]));
		},
		// A hidden doc tag, so tool-codegen emits the type even when no trait
		// names it.
		ValueKind::Record | ValueKind::Enum => {
			attributes.push(parse_quote!(#[doc = "@extension_type"]));
		},
	}
	match &mut item {
		Item::Struct(item) => item.attrs.extend(attributes),
		Item::Enum(item) => item.attrs.extend(attributes),
		_ => {},
	}
	Ok(quote!(#item))
}
