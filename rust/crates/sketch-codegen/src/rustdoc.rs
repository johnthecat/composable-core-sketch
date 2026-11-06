//! Reads the extension points out of an interface crate's rustdoc JSON.
//!
//! Every public trait at the crate root is an extension point. Its methods are
//! either `async_trait` methods returning `Result<T, E>`, which rustdoc shows as a
//! boxed `Future`, or plain methods returning `BoxStream<'static, Result<T, E>>`.
//! The structs and enums they name, directly or through other types, are
//! extracted with them.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::model::{
	ExtensionTrait, Extensions, Field, Method, MethodKind, Param, TypeDef, TypeDefKind, TypeRef,
	Variant,
};

/// Oldest rustdoc JSON `format_version` the walkers below were written against.
const MIN_FORMAT_VERSION: u32 = 57;

/// The parts of a rustdoc JSON document the extraction reads.
#[derive(Deserialize)]
pub struct Crate {
	format_version: u32,
	root: Value,
	index: HashMap<String, Item>,
}

#[derive(Deserialize)]
struct Item {
	crate_id: u32,
	name: Option<String>,
	docs: Option<String>,
	inner: Value,
}

/// Parses rustdoc JSON, rejecting a format older than the extraction knows.
pub fn parse(json: &str) -> Result<Crate> {
	let krate: Crate = serde_json::from_str(json).context("parsing rustdoc JSON")?;
	if krate.format_version < MIN_FORMAT_VERSION {
		bail!(
			"rustdoc JSON format_version {} is older than {MIN_FORMAT_VERSION}; regenerate it \
			 with the nightly in `nightly-toolchain`",
			krate.format_version
		);
	}
	Ok(krate)
}

impl Crate {
	/// Extracts every extension point trait and the types they name.
	pub fn extensions(&self) -> Result<Extensions> {
		let root = self.item(&id_string(&self.root)?)?;
		let crate_name = root.name.clone().context("crate root has no name")?;
		let item_ids = root
			.inner
			.pointer("/module/items")
			.and_then(Value::as_array)
			.context("crate root is not a module")?;

		let mut traits = Vec::new();
		for item_id in item_ids {
			let item = self.item(&id_string(item_id)?)?;
			if let Some(body) = item.inner.get("trait") {
				traits.push(self.extension_trait(item, body)?);
			}
		}

		let mut types = Vec::new();
		for method in traits.iter().flat_map(|extension| &extension.methods) {
			for param in &method.params {
				self.collect_types(&param.type_ref, &mut types)?;
			}
			let (value, error) = match &method.kind {
				MethodKind::Async { ok, error } => (ok, error),
				MethodKind::Stream { item, error } => (item, error),
			};
			self.collect_types(value, &mut types)?;
			self.collect_types(&TypeRef::Named(error.clone()), &mut types)?;
		}
		Ok(Extensions { crate_name, traits, types })
	}

	fn item(&self, id: &str) -> Result<&Item> {
		self.index.get(id).with_context(|| format!("rustdoc item {id} is missing"))
	}

	fn local_type(&self, name: &str) -> Option<&Item> {
		self.index
			.values()
			.find(|item| item.crate_id == 0 && item.name.as_deref() == Some(name) && is_type(item))
	}

	fn extension_trait(&self, item: &Item, body: &Value) -> Result<ExtensionTrait> {
		let name = item.name.clone().context("trait has no name")?;
		let method_ids =
			body.get("items").and_then(Value::as_array).context("trait has no items")?;
		let mut methods = Vec::new();
		for method_id in method_ids {
			let method = self.item(&id_string(method_id)?)?;
			let method_name = method.name.clone().context("trait method has no name")?;
			let sig = method
				.inner
				.pointer("/function/sig")
				.with_context(|| format!("{name}::{method_name} is not a function"))?;
			methods.push(
				self.method(method, method_name.clone(), sig)
					.with_context(|| format!("in {name}::{method_name}"))?,
			);
		}
		Ok(ExtensionTrait { name, docs: item.docs.clone(), methods })
	}

	fn method(&self, item: &Item, name: String, sig: &Value) -> Result<Method> {
		let inputs =
			sig.get("inputs").and_then(Value::as_array).context("signature has no inputs")?;
		let mut params = Vec::new();
		for input in inputs {
			let (Some(param_name), Some(ty)) = (input.get(0).and_then(Value::as_str), input.get(1))
			else {
				bail!("malformed input {input}");
			};
			if param_name == "self" {
				continue;
			}
			params.push(Param { name: param_name.to_string(), type_ref: self.type_ref(ty)? });
		}

		let output = sig
			.get("output")
			.filter(|output| !output.is_null())
			.context("method returns nothing")?;
		let kind = if let Some(future_output) = async_trait_output(output) {
			let (ok, error) = self.result(future_output)?;
			MethodKind::Async { ok, error }
		} else if path_leaf(output) == Some("BoxStream") {
			let item = type_args(output).next().context("BoxStream has no item type")?;
			let (item, error) = self.result(item)?;
			MethodKind::Stream { item, error }
		} else {
			bail!("expected an async_trait method or a BoxStream return, found {output}");
		};
		Ok(Method { name, docs: item.docs.clone(), params, kind })
	}

	/// Splits `Result<T, E>` into `T` and the name of `E`, which must be a local type.
	fn result(&self, ty: &Value) -> Result<(TypeRef, String)> {
		if path_leaf(ty) != Some("Result") {
			bail!("expected Result<T, E>, found {ty}");
		}
		let mut args = type_args(ty);
		let (Some(ok), Some(error)) = (args.next(), args.next()) else {
			bail!("Result needs two type arguments");
		};
		let TypeRef::Named(error) = self.type_ref(error)? else {
			bail!("the error type must be a struct or enum of the interface crate");
		};
		Ok((self.type_ref(ok)?, error))
	}

	fn type_ref(&self, ty: &Value) -> Result<TypeRef> {
		if let Some(primitive) = ty.get("primitive").and_then(Value::as_str) {
			return Ok(match primitive {
				"bool" => TypeRef::Bool,
				"u8" => TypeRef::U8,
				"u16" => TypeRef::U16,
				"u32" => TypeRef::U32,
				"u64" => TypeRef::U64,
				other => bail!("unsupported primitive {other}"),
			});
		}
		if ty.get("tuple").and_then(Value::as_array).is_some_and(Vec::is_empty) {
			return Ok(TypeRef::Unit);
		}
		if let Some(array) = ty.get("array") {
			let element = array.get("type").context("array has no element type")?;
			if self.type_ref(element)? != TypeRef::U8 {
				bail!("only byte arrays are supported");
			}
			let len = array.get("len").and_then(Value::as_str).context("array has no length")?;
			return Ok(TypeRef::FixedBytes(len.parse().context("array length is not a number")?));
		}
		if ty.get("borrowed_ref").is_some() {
			bail!("parameters and fields must be owned");
		}
		let leaf = path_leaf(ty).with_context(|| format!("unsupported type {ty}"))?;
		let mut args = type_args(ty);
		Ok(match leaf {
			"String" => TypeRef::String,
			"Vec" => match self.type_ref(args.next().context("Vec has no element type")?)? {
				TypeRef::U8 => TypeRef::Bytes,
				element => TypeRef::Vec(Box::new(element)),
			},
			"Option" => TypeRef::Option(Box::new(
				self.type_ref(args.next().context("Option has no type")?)?,
			)),
			name if self.local_type(name).is_some() => TypeRef::Named(name.to_string()),
			other => bail!("unsupported type {other}: not a struct or enum of the interface crate"),
		})
	}

	/// Appends the type `type_ref` names, after the types it names in turn.
	fn collect_types(&self, type_ref: &TypeRef, types: &mut Vec<TypeDef>) -> Result<()> {
		let name = match type_ref {
			TypeRef::Named(name) => name,
			TypeRef::Vec(element) | TypeRef::Option(element) => {
				return self.collect_types(element, types);
			},
			_ => return Ok(()),
		};
		if types.iter().any(|known| &known.name == name) {
			return Ok(());
		}
		let item = self.local_type(name).with_context(|| format!("type {name} is missing"))?;
		let kind = if let Some(body) = item.inner.get("struct") {
			let field_ids = body
				.pointer("/kind/plain/fields")
				.and_then(Value::as_array)
				.with_context(|| format!("struct {name} must have named fields"))?;
			TypeDefKind::Struct(self.fields(field_ids)?)
		} else {
			let variant_ids = item
				.inner
				.pointer("/enum/variants")
				.and_then(Value::as_array)
				.with_context(|| format!("{name} is neither a struct nor an enum"))?;
			let mut variants = Vec::new();
			for variant_id in variant_ids {
				let variant = self.item(&id_string(variant_id)?)?;
				let kind = variant.inner.pointer("/variant/kind").context("variant has no kind")?;
				let fields = if kind.as_str() == Some("plain") {
					Vec::new()
				} else if let Some(field_ids) =
					kind.pointer("/struct/fields").and_then(Value::as_array)
				{
					self.fields(field_ids)?
				} else {
					bail!("variants of {name} must be unit or have named fields");
				};
				variants.push(Variant {
					name: variant.name.clone().context("variant has no name")?,
					docs: variant.docs.clone(),
					fields,
				});
			}
			TypeDefKind::Enum(variants)
		};
		let fields: Vec<&Field> = match &kind {
			TypeDefKind::Struct(fields) => fields.iter().collect(),
			TypeDefKind::Enum(variants) => {
				variants.iter().flat_map(|variant| &variant.fields).collect()
			},
		};
		for field in fields {
			self.collect_types(&field.type_ref, types)?;
		}
		types.push(TypeDef { name: name.clone(), docs: item.docs.clone(), kind });
		Ok(())
	}

	fn fields(&self, field_ids: &[Value]) -> Result<Vec<Field>> {
		field_ids
			.iter()
			.map(|field_id| {
				let field = self.item(&id_string(field_id)?)?;
				let name = field.name.clone().context("field has no name")?;
				let ty = field
					.inner
					.get("struct_field")
					.with_context(|| format!("{name} is not a field"))?;
				let type_ref = self.type_ref(ty).with_context(|| format!("in field {name}"))?;
				Ok(Field { name, docs: field.docs.clone(), type_ref })
			})
			.collect()
	}
}

fn is_type(item: &Item) -> bool {
	item.inner.get("struct").is_some() || item.inner.get("enum").is_some()
}

fn id_string(id: &Value) -> Result<String> {
	match id {
		Value::Number(number) => Ok(number.to_string()),
		Value::String(id) => Ok(id.clone()),
		other => bail!("expected a rustdoc id, found {other}"),
	}
}

/// Last segment of a `resolved_path` type's path.
fn path_leaf(ty: &Value) -> Option<&str> {
	let path = ty.pointer("/resolved_path/path")?.as_str()?;
	path.rsplit("::").next()
}

/// Type arguments of a `resolved_path` type, skipping lifetimes.
fn type_args(ty: &Value) -> impl Iterator<Item = &Value> {
	ty.pointer("/resolved_path/args/angle_bracketed/args")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|arg| arg.get("type"))
}

/// `T` in the `Pin<Box<dyn Future<Output = T> + Send>>` `async_trait` returns.
fn async_trait_output(output: &Value) -> Option<&Value> {
	if path_leaf(output) != Some("Pin") {
		return None;
	}
	let boxed = type_args(output).next()?;
	if path_leaf(boxed) != Some("Box") {
		return None;
	}
	let bounds = type_args(boxed).next()?.pointer("/dyn_trait/traits")?.as_array()?;
	bounds.iter().find_map(|bound| {
		let future = bound.get("trait")?;
		if !future.get("path")?.as_str()?.ends_with("Future") {
			return None;
		}
		future
			.pointer("/args/angle_bracketed/constraints")?
			.as_array()?
			.iter()
			.find(|constraint| constraint.get("name").and_then(Value::as_str) == Some("Output"))?
			.pointer("/binding/equality/type")
	})
}
