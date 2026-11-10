//! Reads rustdoc JSON: the crates tool-codegen loads, and the type shapes in them.
//!
//! Each input is one crate documented with `--no-deps`. A type another loaded
//! crate defines appears only by path in `paths`; [`World`] follows that path
//! into the crate that defines it, so a type the interface crate names from the
//! protocol crate resolves to the protocol crate's definition.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;

use crate::model::{EnumVariant, Field, TypeDef, TypeDefKind, TypeName, TypeRef, VariantShape};

/// Oldest rustdoc JSON `format_version` the walkers below were written against.
const MIN_FORMAT_VERSION: u32 = 57;

/// One crate's rustdoc JSON.
#[derive(Deserialize)]
pub struct Crate {
	format_version: u32,
	root: Value,
	index: HashMap<String, Item>,
	paths: HashMap<String, ItemPath>,
}

/// One documented item.
#[derive(Deserialize)]
pub struct Item {
	/// Name in source.
	pub name: Option<String>,
	/// Rustdoc comment, with any codegen tags still in it.
	pub docs: Option<String>,
	/// Kind-dependent body.
	pub inner: Value,
}

/// Where an item lives: its crate and full path.
#[derive(Deserialize)]
pub struct ItemPath {
	crate_id: u32,
	/// Full path, starting with the crate name.
	pub path: Vec<String>,
}

/// Parses rustdoc JSON, rejecting a format older than the walkers know.
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
	/// The crate's name, as paths start with it.
	pub fn name(&self) -> Result<&str> {
		self.item(&id_string(&self.root)?)?
			.name
			.as_deref()
			.context("crate root has no name")
	}

	/// The item with id `id`.
	pub fn item(&self, id: &str) -> Result<&Item> {
		self.index.get(id).with_context(|| format!("rustdoc item {id} is missing"))
	}

	/// The items the crate exports, with their ids: those of the root and of
	/// its public modules, following `pub use` of local items.
	pub fn root_items(&self) -> Result<Vec<(String, &Item)>> {
		let mut items = Vec::new();
		self.module_items(self.item(&id_string(&self.root)?)?, &mut items)?;
		Ok(items)
	}

	/// The items `module` exports, appended to `items`.
	fn module_items<'a>(
		&'a self,
		module: &Item,
		items: &mut Vec<(String, &'a Item)>,
	) -> Result<()> {
		let ids = module
			.inner
			.pointer("/module/items")
			.and_then(Value::as_array)
			.context("not a module")?;
		for id in ids {
			let id = id_string(id)?;
			let item = self.item(&id)?;
			if item.inner.get("module").is_some() {
				self.module_items(item, items)?;
				continue;
			}
			let Some(import) = item.inner.get("use") else {
				items.push((id, item));
				continue;
			};
			if import.get("is_glob").and_then(Value::as_bool) == Some(true) {
				bail!("glob re-exports are not supported; export each item, or a public module");
			}
			let Some(target_id) = import.get("id").filter(|id| !id.is_null()) else { continue };
			let target_id = id_string(target_id)?;
			if let Some(target) = self.index.get(&target_id) {
				items.push((target_id, target));
			}
		}
		Ok(())
	}

	/// The items of a trait, in declaration order.
	pub fn trait_items(&self, item: &Item) -> Result<Vec<&Item>> {
		item.inner
			.pointer("/trait/items")
			.and_then(Value::as_array)
			.context("not a trait")?
			.iter()
			.map(|id| self.item(&id_string(id)?))
			.collect()
	}

	/// The path of the local item `id`.
	pub fn local_path(&self, id: &str) -> Option<&ItemPath> {
		self.paths.get(id).filter(|path| path.crate_id == 0)
	}

	fn id_of_path(&self, path: &[String]) -> Option<&str> {
		self.paths
			.iter()
			.find(|(_, item)| item.crate_id == 0 && item.path == path)
			.map(|(id, _)| id.as_str())
	}
}

/// Every loaded crate, so a path one names resolves in the crate defining it.
pub struct World<'a> {
	crates: Vec<&'a Crate>,
}

impl<'a> World<'a> {
	/// The world of `crates`.
	pub fn new(crates: Vec<&'a Crate>) -> Self {
		Self { crates }
	}

	/// Resolve a type as `krate` writes it.
	pub fn type_ref(&self, krate: &Crate, ty: &Value) -> Result<TypeRef> {
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
			if self.type_ref(krate, element)? != TypeRef::U8 {
				bail!("only byte arrays are supported");
			}
			return Ok(TypeRef::Bytes);
		}
		if ty.get("borrowed_ref").is_some() {
			bail!("arguments and fields must be owned");
		}
		let leaf = path_leaf(ty).with_context(|| format!("unsupported type {ty}"))?;
		let mut args = type_args(ty);
		match leaf {
			"String" => return Ok(TypeRef::String),
			"Vec" => {
				return Ok(
					match self.type_ref(krate, args.next().context("Vec has no type")?)? {
						TypeRef::U8 => TypeRef::Bytes,
						element => TypeRef::Vec(Box::new(element)),
					},
				);
			},
			"Option" => {
				let inner = self.type_ref(krate, args.next().context("Option has no type")?)?;
				return Ok(TypeRef::Option(Box::new(inner)));
			},
			_ => {},
		}
		let id = ty.pointer("/resolved_path/id").context("type has no id")?;
		let (defining, defining_id) = self.definition(krate, &id_string(id)?)?;
		let item = defining.item(defining_id)?;
		if let Some(target) = item.inner.pointer("/type_alias/type") {
			return self.type_ref(defining, target);
		}
		// A newtype, such as `Bytes32([u8; 32])`, crosses as its one field:
		// SCALE encodes it so, and the byte newtypes serialize so.
		if let Some([field_id]) = item
			.inner
			.pointer("/struct/kind/tuple")
			.and_then(Value::as_array)
			.map(Vec::as_slice)
		{
			let field = defining.item(&id_string(field_id)?)?;
			let ty = field.inner.get("struct_field").context("newtype field has no type")?;
			return self.type_ref(defining, ty);
		}
		let path = defining.local_path(defining_id).context("type has no path")?;
		Ok(TypeRef::Named(TypeName::new(path.path.clone())))
	}

	/// The crate that defines the item `krate` calls `id`, and its id there.
	fn definition(&self, krate: &Crate, id: &str) -> Result<(&'a Crate, &'a str)> {
		let path = krate.paths.get(id).with_context(|| format!("item {id} has no path"))?;
		self.locate(&path.path)
	}

	/// The crate that defines the item at `path`, and its id there.
	fn locate(&self, path: &[String]) -> Result<(&'a Crate, &'a str)> {
		let crate_name = path.first().context("empty path")?;
		for candidate in &self.crates {
			if candidate.name()? == crate_name &&
				let Some(found) = candidate.id_of_path(path)
			{
				return Ok((candidate, found));
			}
		}
		bail!("{} is defined in a crate tool-codegen did not load", path.join("::"))
	}

	/// The definition of `name`.
	pub fn type_def(&self, name: &TypeName) -> Result<TypeDef> {
		let (krate, id) = self.locate(&name.path)?;
		let item = krate.item(id)?;
		let kind = if let Some(body) = item.inner.get("struct") {
			let field_ids = body
				.pointer("/kind/plain/fields")
				.and_then(Value::as_array)
				.with_context(|| format!("struct {name} must have named fields"))?;
			TypeDefKind::Struct(self.fields(krate, field_ids)?)
		} else if let Some(variant_ids) =
			item.inner.pointer("/enum/variants").and_then(Value::as_array)
		{
			let mut variants = Vec::new();
			for variant_id in variant_ids {
				let variant = krate.item(&id_string(variant_id)?)?;
				let variant_name = variant.name.clone().context("variant has no name")?;
				let kind = variant.inner.pointer("/variant/kind").context("variant has no kind")?;
				let shape = if kind.as_str() == Some("plain") {
					VariantShape::Unit
				} else if let Some(ids) = kind.pointer("/struct/fields").and_then(Value::as_array) {
					VariantShape::Named(self.fields(krate, ids)?)
				} else {
					bail!("variant {name}::{variant_name} must be unit or have named fields");
				};
				variants.push(EnumVariant {
					name: variant_name,
					docs: clean_docs(variant.docs.as_deref()),
					shape,
				});
			}
			TypeDefKind::Enum(variants)
		} else {
			bail!("{name} is neither a struct nor an enum");
		};
		Ok(TypeDef { name: name.clone(), docs: clean_docs(item.docs.as_deref()), kind })
	}

	fn fields(&self, krate: &Crate, ids: &[Value]) -> Result<Vec<Field>> {
		ids.iter()
			.map(|id| {
				let field = krate.item(&id_string(id)?)?;
				let name = field.name.clone().context("field has no name")?;
				let ty = field.inner.get("struct_field").context("field has no type")?;
				let type_ref =
					self.type_ref(krate, ty).with_context(|| format!("in field {name}"))?;
				Ok(Field { name, docs: clean_docs(field.docs.as_deref()), type_ref })
			})
			.collect()
	}

	/// Every type `roots` name, directly or through other types, each after
	/// the types it names.
	pub fn reachable_types<'r>(
		&self,
		roots: impl IntoIterator<Item = &'r TypeRef>,
	) -> Result<Vec<TypeDef>> {
		let mut defs = Vec::new();
		for root in roots {
			self.collect(root, &mut defs)?;
		}
		Ok(defs)
	}

	fn collect(&self, type_ref: &TypeRef, defs: &mut Vec<TypeDef>) -> Result<()> {
		let name = match type_ref {
			TypeRef::Named(name) => name,
			TypeRef::Vec(inner) | TypeRef::Option(inner) => return self.collect(inner, defs),
			_ => return Ok(()),
		};
		if defs.iter().any(|known| &known.name == name) {
			return Ok(());
		}
		let def = self.type_def(name)?;
		for inner in def.type_refs() {
			self.collect(inner, defs)?;
		}
		defs.push(def);
		Ok(())
	}
}

/// Rustdoc comment without codegen tags (lines starting with `@`).
pub fn clean_docs(docs: Option<&str>) -> Option<String> {
	let cleaned = docs?
		.lines()
		.filter(|line| !line.trim_start().starts_with('@'))
		.map(|line| line.strip_prefix(' ').unwrap_or(line))
		.collect::<Vec<_>>()
		.join("\n");
	let cleaned = cleaned.trim().to_string();
	(!cleaned.is_empty()).then_some(cleaned)
}

/// Whether `docs` carry the codegen tag `@{tag}`.
pub fn has_tag(docs: Option<&str>, tag: &str) -> bool {
	docs.is_some_and(|docs| docs.lines().any(|line| line.trim() == format!("@{tag}")))
}

/// The string form of a rustdoc id, which is a number in item bodies and a
/// string as a map key.
pub fn id_string(id: &Value) -> Result<String> {
	match id {
		Value::Number(number) => Ok(number.to_string()),
		Value::String(id) => Ok(id.clone()),
		other => bail!("expected a rustdoc id, found {other}"),
	}
}

/// Last segment of a `resolved_path` type's path.
pub fn path_leaf(ty: &Value) -> Option<&str> {
	ty.pointer("/resolved_path/path")?.as_str()?.rsplit("::").next()
}

/// Type arguments of a `resolved_path` type, skipping lifetimes.
pub fn type_args(ty: &Value) -> impl Iterator<Item = &Value> {
	ty.pointer("/resolved_path/args/angle_bracketed/args")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(|arg| arg.get("type"))
}

/// `T` in the `Pin<Box<dyn Future<Output = T> + Send>>` `async_trait` returns.
pub fn async_trait_output(output: &Value) -> Option<&Value> {
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

/// The parameters of a function signature after `&self`.
pub fn params(sig: &Value) -> Result<Vec<(&str, &Value)>> {
	sig.get("inputs")
		.and_then(Value::as_array)
		.context("signature has no inputs")?
		.iter()
		.filter_map(|input| Some((input.get(0)?.as_str()?, input.get(1)?)))
		.filter(|(name, _)| *name != "self")
		.map(Ok)
		.collect()
}

#[cfg(test)]
mod tests;
