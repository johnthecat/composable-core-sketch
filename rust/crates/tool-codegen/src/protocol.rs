//! The product protocol: its traits and methods with their wire ids, read from
//! the protocol crate's rustdoc JSON.
//!
//! A protocol trait carries `#[wire_trait(id = N)]` and each of its methods
//! `#[wire(id = N)]`, which rustdoc keeps as `@wire_trait_id=N` and
//! `@wire_id=N` doc tags. Each method takes at most one request, and returns
//! either `Result<Response, CallError<Error>>` or
//! `Subscription<Item, CallError<Error>>`.

use anyhow::{Context, Result, bail};
use heck::ToLowerCamelCase;
use serde_json::Value;

use crate::{
	model::TypeRef,
	rustdoc::{
		Crate, World, async_trait_output, clean_docs, params, path_leaf, tag_value, type_args,
	},
};

/// A protocol service.
pub struct ApiTrait {
	/// Trait name, such as `Signing`.
	pub name: String,
	/// Wire id of the trait.
	pub id: u8,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// Methods in declaration order.
	pub methods: Vec<ApiMethod>,
}

impl ApiTrait {
	/// Name of its namespace on the TypeScript client, such as `localStorage`.
	pub fn client_name(&self) -> String {
		self.name.to_lower_camel_case()
	}
}

/// A protocol method.
pub struct ApiMethod {
	/// Rust name, in snake case.
	pub name: String,
	/// Wire id within its trait.
	pub id: u8,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// The request, or `None` for a method that takes none.
	pub request: Option<TypeRef>,
	/// Request/response or subscription, with what each leg carries.
	pub kind: ApiKind,
}

/// The two exchange shapes of a protocol method.
pub enum ApiKind {
	/// One request, one response.
	Request {
		/// The response.
		response: TypeRef,
		/// The method's own error.
		error: TypeRef,
	},
	/// One start, then items until an interrupt.
	Subscription {
		/// Each item.
		item: TypeRef,
		/// The method's own error.
		error: TypeRef,
	},
}

impl ApiMethod {
	/// Every type the method names.
	pub fn type_refs(&self) -> Vec<&TypeRef> {
		let (value, error) = match &self.kind {
			ApiKind::Request { response, error } => (response, error),
			ApiKind::Subscription { item, error } => (item, error),
		};
		self.request.iter().chain([value, error]).collect()
	}
}

/// Every protocol trait of `krate`, sorted by wire id.
pub fn extract(world: &World, krate: &Crate) -> Result<Vec<ApiTrait>> {
	let mut traits = Vec::new();
	for (_, item) in krate.items_in_module("api", "trait") {
		let Some(id) = tag_value(item.docs.as_deref(), "wire_trait_id") else {
			continue;
		};
		let name = item.name.clone().context("trait has no name")?;
		let mut methods = Vec::new();
		for method in krate.trait_items(item)? {
			let method_name = method.name.clone().context("method has no name")?;
			methods.push(
				api_method(
					world,
					krate,
					method_name.clone(),
					method.docs.as_deref(),
					&method.inner,
				)
				.with_context(|| format!("in {name}::{method_name}"))?,
			);
		}
		methods.sort_by_key(|method| method.id);
		traits.push(ApiTrait { name, id, docs: clean_docs(item.docs.as_deref()), methods });
	}
	traits.sort_by_key(|api| api.id);
	if let Some(pair) = traits.windows(2).find(|pair| pair[0].id == pair[1].id) {
		bail!("{} and {} share wire id {}", pair[0].name, pair[1].name, pair[0].id);
	}
	Ok(traits)
}

fn api_method(
	world: &World,
	krate: &Crate,
	name: String,
	docs: Option<&str>,
	inner: &Value,
) -> Result<ApiMethod> {
	let id = tag_value(docs, "wire_id").context("method has no #[wire(id = N)]")?;
	let sig = inner.pointer("/function/sig").context("not a function")?;
	let request = match params(sig)?.as_slice() {
		[] => None,
		[(_, request)] => Some(world.type_ref(krate, request)?),
		_ => bail!("a protocol method takes at most one request"),
	};
	let output = sig
		.get("output")
		.and_then(async_trait_output)
		.context("a protocol method is an async_trait method")?;
	let mut args = type_args(output);
	let (Some(value), Some(call_error)) = (args.next(), args.next()) else {
		bail!("a protocol method returns Result or Subscription with two type arguments");
	};
	if path_leaf(call_error) != Some("CallError") {
		bail!("a protocol method fails with CallError<Error>");
	}
	let error = world.type_ref(krate, type_args(call_error).next().context("CallError<?>")?)?;
	let value = world.type_ref(krate, value)?;
	let kind = match path_leaf(output) {
		Some("Result") => ApiKind::Request { response: value, error },
		Some("Subscription") => ApiKind::Subscription { item: value, error },
		_ => bail!("a protocol method returns Result or Subscription"),
	};
	Ok(ApiMethod { name, id, docs: clean_docs(docs), request, kind })
}
