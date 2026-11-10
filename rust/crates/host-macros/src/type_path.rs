//! Reading types out of signatures, for every macro here.

use quote::format_ident;
use syn::{GenericArgument, Ident, Path, PathArguments, ReturnType, Type, TypeParamBound};

/// The last path segment of `ty`, and its type arguments.
pub fn last_segment(ty: &Type) -> Option<(&Ident, Vec<&Type>)> {
	let Type::Path(path) = ty else { return None };
	let segment = path.path.segments.last()?;
	let args = match &segment.arguments {
		PathArguments::AngleBracketed(args) => args
			.args
			.iter()
			.filter_map(|arg| match arg {
				GenericArgument::Type(ty) => Some(ty),
				_ => None,
			})
			.collect(),
		_ => Vec::new(),
	};
	Some((&segment.ident, args))
}

/// `T` in `name<T>`.
pub fn single_argument<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
	match last_segment(ty)? {
		(ident, args) if ident == name => args.first().copied(),
		_ => None,
	}
}

/// `T` and `E` of a function returning `Result<T, E>`.
pub fn result_types(output: &ReturnType) -> Option<(&Type, &Type)> {
	let ReturnType::Type(_, output) = output else { return None };
	match last_segment(output)? {
		(ident, args) if ident == "Result" => match args.as_slice() {
			[ok, error] => Some((ok, error)),
			_ => None,
		},
		_ => None,
	}
}

/// The path of `X` in `Arc<dyn X>`, as written.
pub fn dyn_trait_path(ty: &Type) -> Option<Path> {
	let Type::TraitObject(object) = single_argument(ty, "Arc")? else { return None };
	object.bounds.iter().find_map(|bound| match bound {
		TypeParamBound::Trait(bound) => Some(bound.path.clone()),
		_ => None,
	})
}

/// `path` with its last segment prefixed, such as `wallet::JsWallet` for
/// `wallet::Wallet`: the web glue sits next to the trait it serves, so the
/// path that reaches a trait reaches its glue.
pub fn glue_path(path: &Path, prefix: &str) -> Path {
	let mut glue = path.clone();
	if let Some(last) = glue.segments.last_mut() {
		last.ident = format_ident!("{prefix}{}", last.ident);
	}
	glue
}

/// A compile error when `ty` holds bytes as `Vec<u8>` or `[u8; N]`, which reach
/// JS as an array of numbers. Extension point values use the protocol's byte
/// types, which reach it as a `Uint8Array`.
pub fn reject_raw_bytes(ty: &Type) -> syn::Result<()> {
	let raw = match ty {
		Type::Array(array) => is_u8(&array.elem),
		_ => single_argument(ty, "Vec").is_some_and(is_u8),
	};
	if raw {
		return Err(syn::Error::new_spanned(
			ty,
			"bytes cross as `protocol::Bytes`, `Bytes32` or `Bytes64`; `Vec<u8>` and `[u8; N]` \
			 would reach JS as an array of numbers",
		));
	}
	match ty {
		Type::Array(array) => reject_raw_bytes(&array.elem),
		_ => last_segment(ty)
			.map_or(Ok(()), |(_, args)| args.into_iter().try_for_each(reject_raw_bytes)),
	}
}

fn is_u8(ty: &Type) -> bool {
	matches!(ty, Type::Path(path) if path.path.is_ident("u8"))
}
