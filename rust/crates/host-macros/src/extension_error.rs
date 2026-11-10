//! `#[extension_error]`: an extension point's error, with the catch-all variant
//! every error has and the conversions into it.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Item, ItemEnum, parse_quote};

use crate::extension_type::{ValueKind, add_attributes};

/// Expands `#[extension_error]` on an enum; it takes no arguments.
pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	crate::no_arguments(&args, "extension_error")?;
	let mut item = syn::parse2::<ItemEnum>(item)?;
	if item.variants.iter().any(|variant| variant.ident == "Unknown") {
		return Err(syn::Error::new_spanned(
			&item.ident,
			"#[extension_error] adds `Unknown { reason }` itself; declare only the other failures",
		));
	}
	item.variants.push(parse_quote! {
		/// A failure the trait does not declare.
		#[error("{reason}")]
		Unknown {
			/// Human-readable reason.
			reason: String,
		}
	});
	let name = item.ident.clone();
	let declared = add_attributes(Item::Enum(item), ValueKind::Error)?;
	Ok(quote! {
		#declared

		/// What a failure the trait does not declare becomes. The web glue calls
		/// it with a rejection's message.
		impl ::core::convert::From<String> for #name {
			fn from(reason: String) -> Self {
				Self::Unknown { reason }
			}
		}

		/// What an undeclared exception thrown by a Swift or Kotlin
		/// implementation becomes.
		#[cfg(not(target_arch = "wasm32"))]
		impl ::core::convert::From<::uniffi::UnexpectedUniFFICallbackError> for #name {
			fn from(error: ::uniffi::UnexpectedUniFFICallbackError) -> Self {
				error.reason.into()
			}
		}
	})
}
