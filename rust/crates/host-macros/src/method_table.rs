//! The method table `#[wire_trait]` writes next to a protocol trait: one
//! `<TRAIT>_<METHOD>` wire pair constant per method, and a `methods`
//! function returning each method's handler for the dispatcher.

use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemTrait, ReturnType, TraitItem, TraitItemFn, Type};

use crate::{type_path::last_segment, wire::WireId};

/// The table of `item_trait`, whose wire id is `trait_id`.
pub fn method_table(item_trait: &ItemTrait, trait_id: u8) -> syn::Result<TokenStream> {
	let trait_name = &item_trait.ident;
	let prefix = trait_name.to_string().to_shouty_snake_case();
	let mut constants = Vec::new();
	let mut entries = Vec::new();
	for item in &item_trait.items {
		let TraitItem::Fn(method) = item else { continue };
		let method_id = wire_id(method)?;
		let name = &method.sig.ident;
		let constant = format_ident!("{prefix}_{}", name.to_string().to_shouty_snake_case());
		let doc = format!("Wire pair of `{trait_name}::{name}`.");
		constants.push(quote! {
			#[doc = #doc]
			pub const #constant: crate::dispatch::MethodIds =
				crate::dispatch::MethodIds { trait_id: #trait_id, method_id: #method_id };
		});
		let Request { decoded, binding, arguments } = request(method)?;
		let (shape, value, error) = output_types(method)?;
		entries.push(match shape.as_str() {
			"Result" => quote! {
				crate::dispatch::Method {
					ids: #constant,
					handler: crate::dispatch::Handler::Request(
						|host: ::std::sync::Arc<H>, bytes: ::std::vec::Vec<u8>|
						 -> ::futures::future::BoxFuture<'static, ::std::vec::Vec<u8>> {
							::std::boxed::Box::pin(async move {
								match <#decoded as ::parity_scale_codec::DecodeAll>::decode_all(&mut &bytes[..]) {
									Ok(#binding) => ::parity_scale_codec::Encode::encode(&host.#name(#arguments).await),
									Err(error) => {
										let malformed: ::core::result::Result<#value, crate::CallError<#error>> =
											Err(crate::CallError::MalformedFrame { reason: error.to_string() });
										::parity_scale_codec::Encode::encode(&malformed)
									},
								}
							})
						},
					),
				}
			},
			_ => quote! {
				crate::dispatch::Method {
					ids: #constant,
					handler: crate::dispatch::Handler::Subscription(
						|host: ::std::sync::Arc<H>, bytes: ::std::vec::Vec<u8>|
						 -> ::futures::future::BoxFuture<
							'static,
							::core::result::Result<crate::dispatch::SubscriptionStream, ::std::vec::Vec<u8>>,
						> {
							::std::boxed::Box::pin(async move {
								match <#decoded as ::parity_scale_codec::DecodeAll>::decode_all(&mut &bytes[..]) {
									Ok(#binding) => Ok(crate::dispatch::subscription_stream(host.#name(#arguments).await)),
									Err(error) => {
										let malformed: crate::CallError<#error> =
											crate::CallError::MalformedFrame { reason: error.to_string() };
										Err(crate::dispatch::subscription_interrupt(malformed))
									},
								}
							})
						},
					),
				}
			},
		});
	}
	let doc =
		format!("The method table of [`{trait_name}`], for a dispatcher to serve `host` with.");
	Ok(quote! {
		#(#constants)*

		#[doc = #doc]
		pub fn methods<H: #trait_name + 'static>() -> ::std::vec::Vec<crate::dispatch::Method<H>> {
			::std::vec![#(#entries),*]
		}
	})
}

/// The id in a method's `#[wire(id = N)]`, parsed as `#[wire]` parses it.
fn wire_id(method: &TraitItemFn) -> syn::Result<u8> {
	let attr = method.attrs.iter().find(|attr| attr.path().is_ident("wire")).ok_or_else(|| {
		syn::Error::new_spanned(&method.sig, "a protocol method needs #[wire(id = N)]")
	})?;
	attr.parse_args::<WireId>().map(|WireId(id)| id)
}

/// How a handler decodes a method's request and passes it on.
struct Request {
	/// The type the payload decodes to: the request, or `()` when there is none.
	decoded: TokenStream,
	/// The pattern the decoded value binds to.
	binding: TokenStream,
	/// The arguments the trait method is called with.
	arguments: TokenStream,
}

/// The request of `method`: its one parameter after `&self`, if it has one.
fn request(method: &TraitItemFn) -> syn::Result<Request> {
	match method.sig.inputs.iter().collect::<Vec<_>>().as_slice() {
		[FnArg::Receiver(_)] => {
			Ok(Request { decoded: quote!(()), binding: quote!(()), arguments: quote!() })
		},
		[FnArg::Receiver(_), FnArg::Typed(request)] => {
			let ty = &request.ty;
			Ok(Request {
				decoded: quote!(#ty),
				binding: quote!(request),
				arguments: quote!(request),
			})
		},
		_ => Err(syn::Error::new_spanned(
			&method.sig,
			"a protocol method takes `&self` and at most one request",
		)),
	}
}

/// `Result` or `Subscription`, its value type, and its `CallError`'s domain type.
fn output_types(method: &TraitItemFn) -> syn::Result<(String, &Type, &Type)> {
	let error = || {
		syn::Error::new_spanned(
			&method.sig.output,
			"a protocol method returns Result<_, CallError<_>> or Subscription<_, CallError<_>>",
		)
	};
	let ReturnType::Type(_, output) = &method.sig.output else { return Err(error()) };
	let (shape, args) = last_segment(output).ok_or_else(error)?;
	let [value, call_error] = args.as_slice() else { return Err(error()) };
	let (call_error_name, domain) = last_segment(call_error).ok_or_else(error)?;
	let shape = shape.to_string();
	match (shape.as_str(), call_error_name.to_string().as_str(), domain.as_slice()) {
		("Result" | "Subscription", "CallError", [domain]) => Ok((shape, value, domain)),
		_ => Err(error()),
	}
}
