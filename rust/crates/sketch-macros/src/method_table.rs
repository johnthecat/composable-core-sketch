//! The method table `#[wire_trait]` writes next to a protocol trait: one
//! `<TRAIT>_<METHOD>` wire pair constant per method, and a `methods`
//! function returning each method's handler for the dispatcher.

use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemTrait, LitInt, ReturnType, TraitItem, TraitItemFn, Type};

use crate::type_path::last_segment;

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
		let request = request_type(method)?;
		let (shape, value, error) = output_types(method)?;
		entries.push(match shape.as_str() {
			"Result" => quote! {
				crate::dispatch::Method {
					ids: #constant,
					handler: crate::dispatch::Handler::Request(
						|host: ::std::sync::Arc<H>, cx: crate::CallContext, bytes: ::std::vec::Vec<u8>|
						 -> ::futures::future::BoxFuture<'static, ::std::vec::Vec<u8>> {
							::std::boxed::Box::pin(async move {
								match <#request as ::parity_scale_codec::DecodeAll>::decode_all(&mut &bytes[..]) {
									Ok(request) => ::parity_scale_codec::Encode::encode(&host.#name(&cx, request).await),
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
						|host: ::std::sync::Arc<H>, cx: crate::CallContext, bytes: ::std::vec::Vec<u8>|
						 -> ::futures::future::BoxFuture<
							'static,
							::core::result::Result<crate::dispatch::SubscriptionStream, ::std::vec::Vec<u8>>,
						> {
							::std::boxed::Box::pin(async move {
								match <#request as ::parity_scale_codec::DecodeAll>::decode_all(&mut &bytes[..]) {
									Ok(request) => Ok(crate::dispatch::subscription_stream(host.#name(&cx, request).await)),
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

/// The id in a method's `#[wire(id = N)]`.
fn wire_id(method: &TraitItemFn) -> syn::Result<LitInt> {
	let attr = method.attrs.iter().find(|attr| attr.path().is_ident("wire")).ok_or_else(|| {
		syn::Error::new_spanned(&method.sig, "a protocol method needs #[wire(id = N)]")
	})?;
	let mut id = None;
	attr.parse_nested_meta(|meta| {
		id = Some(meta.value()?.parse::<LitInt>()?);
		Ok(())
	})?;
	id.ok_or_else(|| syn::Error::new_spanned(attr, "expected `id = N`"))
}

/// The type of the request, the parameter after `cx`.
fn request_type(method: &TraitItemFn) -> syn::Result<&Type> {
	match method.sig.inputs.iter().collect::<Vec<_>>().as_slice() {
		[FnArg::Receiver(_), FnArg::Typed(_), FnArg::Typed(request)] => Ok(&request.ty),
		_ => Err(syn::Error::new_spanned(
			&method.sig,
			"a protocol method takes `&self`, `cx: &CallContext` and one request",
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
