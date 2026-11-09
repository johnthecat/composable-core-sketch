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
pub fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
	if !args.is_empty() {
		return syn::Error::new_spanned(args, "#[extension_type] takes no arguments")
			.to_compile_error();
	}
	match syn::parse2::<Item>(item) {
		Ok(Item::Struct(item)) => add_attributes(Item::Struct(item), ValueKind::Record),
		Ok(Item::Enum(item)) => add_attributes(Item::Enum(item), ValueKind::Enum),
		Ok(other) => {
			syn::Error::new_spanned(other, "#[extension_type] applies to a struct or enum")
				.to_compile_error()
		},
		Err(error) => error.to_compile_error(),
	}
}

/// Expands `#[protocol_record]` on a struct; it takes no arguments.
pub fn expand_record(args: TokenStream, item: TokenStream) -> TokenStream {
	if !args.is_empty() {
		return syn::Error::new_spanned(args, "#[protocol_record] takes no arguments")
			.to_compile_error();
	}
	match syn::parse2::<Item>(item) {
		Ok(Item::Struct(item)) => add_attributes(Item::Struct(item), ValueKind::Record),
		Ok(other) => syn::Error::new_spanned(other, "#[protocol_record] applies to a struct")
			.to_compile_error(),
		Err(error) => error.to_compile_error(),
	}
}

/// `item` with the derives of a `kind` value, or an error naming a field
/// that holds `Vec<u8>` or `[u8; N]`. Byte fields use the protocol's byte
/// types, which serialize themselves as bytes.
pub fn add_attributes(mut item: Item, kind: ValueKind) -> TokenStream {
	let fields: Vec<&Fields> = match &item {
		Item::Struct(item) => vec![&item.fields],
		Item::Enum(item) => item.variants.iter().map(|variant| &variant.fields).collect(),
		_ => Vec::new(),
	};
	let raw_bytes = fields.into_iter().flatten().try_for_each(|field| reject_raw_bytes(&field.ty));
	if let Err(error) = raw_bytes {
		return error.to_compile_error();
	}
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
	if matches!(kind, ValueKind::Error) {
		attributes.insert(0, parse_quote!(#[derive(Debug, ::thiserror::Error)]));
	}
	match &mut item {
		Item::Struct(item) => item.attrs.extend(attributes),
		Item::Enum(item) => item.attrs.extend(attributes),
		_ => {},
	}
	quote!(#item)
}
