//! The web glue `#[extension_point]` and `#[extension_listener]` write: the
//! trait's JS object type, the trait over such an object, and a Rust
//! implementation handed to JS.

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, Ident, Pat, TraitItemFn, Type};

use crate::{
	TYPESCRIPT_MODULE_DECLARATION, TYPESCRIPT_NAMESPACE,
	type_path::{dyn_trait_name, reject_raw_bytes},
};

/// One method of the trait, as the glue needs it.
pub struct Method<'a> {
	/// Rust name.
	pub name: &'a Ident,
	/// The JS method name, in camel case.
	pub js_name: String,
	/// Its doc attributes, copied onto the exported JS method.
	pub docs: Vec<&'a syn::Attribute>,
	/// Arguments after `&self`.
	pub params: Vec<Param<'a>>,
	/// What it returns.
	pub shape: Shape<'a>,
}

/// What a method returns.
pub enum Shape<'a> {
	/// `async fn … -> Result<ok, error>`: a call the core awaits.
	Call {
		/// The value on success.
		ok: &'a Type,
		/// The trait's error, `<Trait>Error`.
		error: &'a Type,
		/// The name a declared `error` rejects with on the web:
		/// `<Trait>Exception`.
		exception: String,
	},
	/// `fn …` returning nothing, which returns at once: handing over a
	/// listener, or a listener's own method.
	Notification,
}

/// One argument after `&self`.
pub struct Param<'a> {
	/// Its name.
	pub name: &'a Ident,
	/// Its type.
	pub ty: &'a Type,
	/// `X` when the argument is `Arc<dyn X>`, another trait of this crate.
	pub interface: Option<Ident>,
}

impl<'a> Method<'a> {
	/// `method` as the glue needs it, given its `shape`. Every argument is
	/// owned, and a value argument holds no raw bytes; `shape_error` describes
	/// the shapes the macro accepts.
	pub fn new(
		method: &'a TraitItemFn,
		shape: Shape<'a>,
		shape_error: impl Fn() -> syn::Error,
	) -> syn::Result<Self> {
		let mut params = Vec::new();
		for input in &method.sig.inputs {
			let FnArg::Typed(typed) = input else { continue };
			let Pat::Ident(name) = typed.pat.as_ref() else { return Err(shape_error()) };
			if matches!(typed.ty.as_ref(), Type::Reference(_)) {
				return Err(shape_error());
			}
			let interface = dyn_trait_name(&typed.ty);
			if interface.is_none() {
				reject_raw_bytes(&typed.ty)?;
			}
			params.push(Param { name: &name.ident, ty: typed.ty.as_ref(), interface });
		}
		Ok(Self {
			name: &method.sig.ident,
			js_name: method.sig.ident.to_string().to_lower_camel_case(),
			docs: method.attrs.iter().filter(|attr| attr.path().is_ident("doc")).collect(),
			params,
			shape,
		})
	}
}

impl Param<'_> {
	/// An expression turning the Rust argument into a `JsValue`, or failing
	/// with an `error`.
	fn encode_expression(&self, error: &TokenStream) -> TokenStream {
		let name = self.name;
		match &self.interface {
			Some(interface) => {
				let wasm = format_ident!("Wasm{interface}");
				quote!(::wasm_bindgen::JsValue::from(crate::#wasm::into_js(#name)))
			},
			None => quote!(crate::wasm::to_js::<_, #error>(&#name)?),
		}
	}

	/// A statement rebinding the `JsValue` argument as its Rust type, or
	/// failing with what `reason_into_error`, a function of a `String`,
	/// returns.
	fn decode_statement(&self, reason_into_error: &TokenStream) -> TokenStream {
		let Self { name, ty, interface } = self;
		match interface {
			Some(interface) => {
				let foreign = format_ident!("Foreign{interface}");
				quote! {
					let #name: #ty = ::std::sync::Arc::new(
						crate::#foreign::new(::wasm_bindgen::JsCast::unchecked_into(#name))
							.map_err(#reason_into_error)?,
					);
				}
			},
			None => quote! {
				let #name = crate::wasm::from_js::<#ty, ::std::string::String>(#name)
					.map_err(#reason_into_error)?;
			},
		}
	}
}

/// The web glue of the trait `name`: `Js<Trait>`, `Foreign<Trait>` and
/// `Wasm<Trait>`, all compiled for `wasm32` only.
pub fn web_glue(name: &Ident, methods: &[Method]) -> TokenStream {
	let js_type = format_ident!("Js{name}");
	let module_declaration = format_ident!("{TYPESCRIPT_MODULE_DECLARATION}");
	let foreign = format_ident!("Foreign{name}");
	let wasm = format_ident!("Wasm{name}");
	let typescript_name = name.to_string();
	let typescript_type = format!("{TYPESCRIPT_NAMESPACE}.{name}");
	let js_type_doc =
		format!("A JS object implementing `{name}`: host code, or a Rust value handed to JS.");
	let foreign_doc = format!("[`{name}`] over a JS object implementing it, for Rust callers.");
	let new_doc = format!(
		"Wraps `object`, or names the first method of `{name}` it lacks, so a host learns at \
		 startup and not on the first call."
	);
	let wasm_doc = format!("A Rust [`{name}`] as a JS object implementing it.");
	let into_js_doc = format!("Hand `inner` to JS as an object implementing `{name}`.");
	let js_names = methods.iter().map(|method| &method.js_name);

	let foreign_methods = methods.iter().map(|method| {
		let Method { name, js_name, params, shape, .. } = method;
		let param_list = params.iter().map(|Param { name, ty, .. }| quote!(#name: #ty));
		match shape {
			Shape::Call { ok, error, exception } => {
				let error = quote!(#error);
				let args = params.iter().map(|param| param.encode_expression(&error));
				quote! {
					async fn #name(&self, #(#param_list),*) -> ::core::result::Result<#ok, #error> {
						let args = ::std::vec![#(#args),*];
						let value = self.0.call::<#error>(#js_name, args, #exception).await?;
						crate::wasm::from_js::<#ok, #error>(value)
					}
				}
			},
			Shape::Notification => {
				let string = quote!(::std::string::String);
				let args = params.iter().map(|param| param.encode_expression(&string));
				quote! {
					fn #name(&self, #(#param_list),*) {
						self.0.send(#js_name, move || ::core::result::Result::Ok(::std::vec![#(#args),*]));
					}
				}
			},
		}
	});

	let exported_methods = methods.iter().map(|method| {
		let Method { name, js_name, docs, params, shape } = method;
		let param_list =
			params.iter().map(|Param { name, .. }| quote!(#name: ::wasm_bindgen::JsValue));
		let args = params.iter().map(|Param { name, .. }| name);
		match shape {
			Shape::Call { ok, error, exception } => {
				let decode_params =
					params.iter().map(|param| param.decode_statement(&quote!(<#error>::from)));
				quote! {
					#(#docs)*
					#[wasm_bindgen(js_name = #js_name)]
					pub fn #name(&self, #(#param_list),*) -> ::js_sys::Promise {
						let inner = ::std::sync::Arc::clone(&self.0);
						crate::wasm::export(
							async move {
								#(#decode_params)*
								let value: #ok = inner.#name(#(#args),*).await?;
								crate::wasm::to_js::<_, #error>(&value)
							},
							#exception,
						)
					}
				}
			},
			Shape::Notification => {
				let into_js_error =
					quote!(|reason: ::std::string::String| ::wasm_bindgen::JsError::new(&reason));
				let decode_params =
					params.iter().map(|param| param.decode_statement(&into_js_error));
				quote! {
					#(#docs)*
					#[wasm_bindgen(js_name = #js_name)]
					pub fn #name(
						&self,
						#(#param_list),*
					) -> ::core::result::Result<(), ::wasm_bindgen::JsError> {
						#(#decode_params)*
						self.0.#name(#(#args),*);
						::core::result::Result::Ok(())
					}
				}
			},
		}
	});

	quote! {
		#[cfg(target_arch = "wasm32")]
		#[::wasm_bindgen::prelude::wasm_bindgen]
		extern "C" {
			#[doc = #js_type_doc]
			#[wasm_bindgen(typescript_type = #typescript_type)]
			pub type #js_type;
		}

		// Fails to build, naming the constant, in a crate that has not
		// declared its TypeScript module with `typescript_module!`.
		#[cfg(target_arch = "wasm32")]
		const _: &str = crate::#module_declaration;

		#[cfg(target_arch = "wasm32")]
		#[doc = #foreign_doc]
		pub struct #foreign(crate::wasm::JsObject);

		#[cfg(target_arch = "wasm32")]
		impl #foreign {
			#[doc = #new_doc]
			pub fn new(object: #js_type) -> ::core::result::Result<Self, ::std::string::String> {
				crate::wasm::JsObject::new(object.into(), #typescript_name, &[#(#js_names),*])
					.map(Self)
			}
		}

		#[cfg(target_arch = "wasm32")]
		#[::async_trait::async_trait]
		impl #name for #foreign {
			#(#foreign_methods)*
		}

		#[cfg(target_arch = "wasm32")]
		#[doc = #wasm_doc]
		#[::wasm_bindgen::prelude::wasm_bindgen(skip_typescript)]
		pub struct #wasm(::std::sync::Arc<dyn #name>);

		#[cfg(target_arch = "wasm32")]
		impl #wasm {
			#[doc = #into_js_doc]
			pub fn into_js(inner: ::std::sync::Arc<dyn #name>) -> #js_type {
				::wasm_bindgen::JsCast::unchecked_into(::wasm_bindgen::JsValue::from(Self(inner)))
			}
		}

		#[cfg(target_arch = "wasm32")]
		#[::wasm_bindgen::prelude::wasm_bindgen]
		impl #wasm {
			#(#exported_methods)*
		}
	}
}
