//! The extension points of the interface crate, read from its rustdoc JSON.
//!
//! Every public trait at the crate root is an extension point, or a listener
//! one takes. A call is an `async_trait` method, which rustdoc shows returning
//! a boxed `Future` of `Result<T, E>`, whose error `E` has the
//! `Unknown { reason }` variant `#[extension_error]` adds. A notification is
//! a plain method returning nothing. Every argument is owned: a value, or
//! `Arc<dyn Trait>` for another trait of the crate.

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use crate::{
	model::{TypeDefKind, TypeName, TypeRef, VariantShape},
	rustdoc::{
		Crate, Item, World, async_trait_output, clean_docs, has_tag, params, path_leaf, type_args,
	},
};

/// An extension point.
pub struct ExtensionTrait {
	/// Trait name, such as `Wallet`.
	pub name: String,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// Methods in declaration order.
	pub methods: Vec<ExtensionMethod>,
}

/// A method of an [`ExtensionTrait`].
pub struct ExtensionMethod {
	/// Rust name, in snake case.
	pub name: String,
	/// Rustdoc comment.
	pub docs: Option<String>,
	/// Arguments after `&self`.
	pub params: Vec<(String, Param)>,
	/// What it returns.
	pub returns: Returns,
}

/// An argument of an [`ExtensionMethod`].
pub enum Param {
	/// A value.
	Value(TypeRef),
	/// `Arc<dyn Trait>`: another trait of the crate, by name.
	Interface(String),
}

/// What an [`ExtensionMethod`] returns.
pub enum Returns {
	/// `async fn … -> Result<ok, error>`: a call the caller awaits.
	Call {
		/// Value on success.
		ok: TypeRef,
		/// The declared error type.
		error: TypeName,
	},
	/// `fn …` returning nothing, which returns at once: a notification.
	Notification,
}

impl ExtensionMethod {
	/// Every value type the method names, its error included.
	pub fn type_refs(&self) -> Vec<TypeRef> {
		let params = self.params.iter().filter_map(|(_, param)| match param {
			Param::Value(type_ref) => Some(type_ref.clone()),
			Param::Interface(_) => None,
		});
		let returns = match &self.returns {
			Returns::Call { ok, error } => vec![ok.clone(), TypeRef::Named(error.clone())],
			Returns::Notification => Vec::new(),
		};
		params.chain(returns).collect()
	}
}

/// Every `#[extension_type]` the crate root of `krate` exports, which the
/// attribute tags `@extension_type=1`, including one no trait names, such as
/// the `ProductContext` the host hands the core.
pub fn root_types(krate: &Crate) -> Result<Vec<TypeRef>> {
	let mut types = Vec::new();
	for (id, item) in krate.root_items()? {
		if has_tag(item.docs.as_deref(), "extension_type") &&
			let Some(path) = krate.local_path(&id)
		{
			types.push(TypeRef::Named(TypeName::new(path.path.clone())));
		}
	}
	Ok(types)
}

/// The extension points the crate root of `krate` exports.
pub fn extension_traits(world: &World, krate: &Crate) -> Result<Vec<ExtensionTrait>> {
	let mut traits = Vec::new();
	for (_, item) in krate.root_items()? {
		if item.inner.get("trait").is_some() {
			traits.push(extension_trait(world, krate, item)?);
		}
	}
	ensure!(!traits.is_empty(), "the crate root exports no extension point trait");
	Ok(traits)
}

fn extension_trait(world: &World, krate: &Crate, item: &Item) -> Result<ExtensionTrait> {
	let name = item.name.clone().context("trait has no name")?;
	let mut methods = Vec::new();
	for method in krate.trait_items(item)? {
		let method_name = method.name.clone().context("method has no name")?;
		let method = extension_method(world, krate, method_name.clone(), method)
			.with_context(|| format!("in {name}::{method_name}"))?;
		if let Returns::Call { error, .. } = &method.returns {
			ensure!(
				error.rust_name() == format!("{name}Error"),
				"{name}::{method_name} fails with {error}; an extension point's error is {name}Error"
			);
		}
		methods.push(method);
	}
	Ok(ExtensionTrait { name, docs: clean_docs(item.docs.as_deref()), methods })
}

fn extension_method(
	world: &World,
	krate: &Crate,
	name: String,
	item: &Item,
) -> Result<ExtensionMethod> {
	let sig = item.inner.pointer("/function/sig").context("not a function")?;
	let params = params(sig)?
		.into_iter()
		.map(|(param, ty)| {
			let param_type = match interface_name(ty) {
				Some(name) => Param::Interface(name.to_string()),
				None => Param::Value(world.type_ref(krate, ty)?),
			};
			Ok((param.to_string(), param_type))
		})
		.collect::<Result<Vec<_>>>()?;
	let docs = clean_docs(item.docs.as_deref());
	let output = sig.get("output").filter(|output| !output.is_null());
	let Some(output) = output else {
		return Ok(ExtensionMethod { name, docs, params, returns: Returns::Notification });
	};
	let result = async_trait_output(output).context(
		"an extension point method is async, or returns nothing; declare the trait with \
		 #[extension_point]",
	)?;
	if path_leaf(result) != Some("Result") {
		bail!("an extension point method returns Result<T, E>");
	}
	let mut args = type_args(result);
	let (Some(ok), Some(error)) = (args.next(), args.next()) else {
		bail!("Result needs two type arguments");
	};
	let TypeRef::Named(error) = world.type_ref(krate, error)? else {
		bail!("the error type must be an enum");
	};
	check_catch_all(world, &error)?;
	Ok(ExtensionMethod {
		name,
		docs,
		params,
		returns: Returns::Call { ok: world.type_ref(krate, ok)?, error },
	})
}

/// `Trait` when `ty` is `Arc<dyn Trait>`.
fn interface_name(ty: &Value) -> Option<&str> {
	if path_leaf(ty) != Some("Arc") {
		return None;
	}
	let bounds = type_args(ty).next()?.pointer("/dyn_trait/traits")?.as_array()?;
	bounds.iter().find_map(|bound| path_leaf_of_trait(bound.get("trait")?))
}

/// The last segment of a trait bound's path.
fn path_leaf_of_trait(bound: &Value) -> Option<&str> {
	bound.get("path")?.as_str()?.rsplit("::").next()
}

/// The TypeScript side reads undeclared failures as `Unknown { reason }`, the
/// variant `#[extension_error]` adds, so every error must have it.
fn check_catch_all(world: &World, error: &TypeName) -> Result<()> {
	let def = world.type_def(error)?;
	let TypeDefKind::Enum(variants) = &def.kind else {
		bail!("the error {error} must be an enum");
	};
	let has_catch_all = variants.iter().any(|variant| {
		variant.name == "Unknown" &&
			matches!(&variant.shape, VariantShape::Named(fields)
				if fields.len() == 1 && fields[0].name == "reason" && fields[0].type_ref == TypeRef::String)
	});
	ensure!(has_catch_all, "the error {error} needs an `Unknown {{ reason: String }}` variant");
	Ok(())
}
