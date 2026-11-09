//! TypeScript types and `scale-ts` codecs for Rust structs and enums.
//!
//! Every generated file imports the codec primitives as `S` and names a struct
//! or enum by its TypeScript name; the type and its codec share that name. A
//! file that names types it does not define records them in [`Imports`].

use std::collections::BTreeSet;

use heck::ToLowerCamelCase;
use indoc::formatdoc;

use crate::model::{EnumVariant, Field, TypeDef, TypeDefKind, TypeName, TypeRef, VariantShape};

/// The structs and enums a file names, so it can import those defined elsewhere.
#[derive(Default)]
pub struct Imports {
	named: BTreeSet<String>,
	paths: Vec<TypeName>,
}

impl Imports {
	/// Record that the file names `name`.
	pub fn record(&mut self, name: &TypeName) {
		if self.named.insert(name.ts_name()) {
			self.paths.push(name.clone());
		}
	}

	/// TypeScript names of the recorded types that `is_external` says another
	/// file defines, sorted.
	pub fn external(&self, is_external: impl Fn(&TypeName) -> bool) -> Vec<String> {
		let mut names: Vec<_> = self
			.paths
			.iter()
			.filter(|name| is_external(name))
			.map(TypeName::ts_name)
			.collect();
		names.sort();
		names
	}
}

/// TypeScript type of a value.
pub fn ts_type(type_ref: &TypeRef, imports: &mut Imports) -> String {
	match type_ref {
		TypeRef::Unit => "undefined".into(),
		TypeRef::Bool => "boolean".into(),
		TypeRef::U8 | TypeRef::U16 | TypeRef::U32 => "number".into(),
		TypeRef::U64 => "bigint".into(),
		TypeRef::String => "string".into(),
		TypeRef::Bytes | TypeRef::FixedBytes(_) => "Uint8Array".into(),
		TypeRef::Vec(element) => match **element {
			TypeRef::Option(_) => format!("({})[]", ts_type(element, imports)),
			_ => format!("{}[]", ts_type(element, imports)),
		},
		TypeRef::Option(inner) => format!("{} | undefined", ts_type(inner, imports)),
		TypeRef::Named(name) => {
			imports.record(name);
			name.ts_name()
		},
	}
}

/// `scale-ts` codec expression of a value.
pub fn codec(type_ref: &TypeRef, imports: &mut Imports) -> String {
	match type_ref {
		TypeRef::Unit => "S._void".into(),
		TypeRef::Bool => "S.bool".into(),
		TypeRef::U8 => "S.u8".into(),
		TypeRef::U16 => "S.u16".into(),
		TypeRef::U32 => "S.u32".into(),
		TypeRef::U64 => "S.u64".into(),
		TypeRef::String => "S.str".into(),
		TypeRef::Bytes => "S.Bytes()".into(),
		TypeRef::FixedBytes(len) => format!("S.Bytes({len})"),
		TypeRef::Vec(element) => format!("S.Vector({})", codec(element, imports)),
		TypeRef::Option(inner) => format!("S.Option({})", codec(inner, imports)),
		TypeRef::Named(name) => {
			imports.record(name);
			name.ts_name()
		},
	}
}

/// The type and the codec of one struct or enum.
pub fn type_definition(def: &TypeDef, imports: &mut Imports) -> String {
	let declaration = type_declaration(def, imports);
	let name = def.name.ts_name();
	let codec = match &def.kind {
		TypeDefKind::Struct(fields) => struct_codec(fields, imports),
		TypeDefKind::Enum(variants) => {
			let codecs = variants
				.iter()
				.map(|variant| format!("{}: {}", variant.name, variant_codec(variant, imports)))
				.collect::<Vec<_>>()
				.join(", ");
			format!("S.Enum({{ {codecs} }}) as unknown as S.Codec<{name}>")
		},
	};
	format!(
		"{declaration}export const {name}: S.Codec<{name}> = {codec};
"
	)
}

/// The type of one struct or enum, without a codec: a struct is an interface
/// with camelCase fields, an enum a union of `{ tag, value }` objects.
pub fn type_declaration(def: &TypeDef, imports: &mut Imports) -> String {
	let name = def.name.ts_name();
	let docs = jsdoc(def.docs.as_deref(), "");
	match &def.kind {
		TypeDefKind::Struct(fields) => formatdoc!(
			"
			{docs}export interface {name} {{
			{fields}}}
			",
			fields = interface_fields(fields, imports),
		),
		TypeDefKind::Enum(variants) => {
			let members: String =
				variants.iter().map(|variant| union_member(variant, imports)).collect();
			formatdoc!(
				"
				{docs}export type {name} =
				{members};
				",
				members = members.trim_end(),
			)
		},
	}
}

fn union_member(variant: &EnumVariant, imports: &mut Imports) -> String {
	let value = match &variant.shape {
		VariantShape::Unit => "value?: undefined".to_string(),
		VariantShape::Newtype(inner) => format!("value: {}", ts_type(inner, imports)),
		VariantShape::Named(fields) => format!("value: {}", inline_object(fields, imports)),
	};
	format!(
		"{}  | {{ tag: \"{}\"; {value} }}\n",
		jsdoc(variant.docs.as_deref(), "  "),
		variant.name
	)
}

fn variant_codec(variant: &EnumVariant, imports: &mut Imports) -> String {
	match &variant.shape {
		VariantShape::Unit => "S._void".into(),
		VariantShape::Newtype(inner) => codec(inner, imports),
		VariantShape::Named(fields) => struct_codec(fields, imports),
	}
}

fn interface_fields(fields: &[Field], imports: &mut Imports) -> String {
	fields
		.iter()
		.map(|field| {
			format!(
				"{}  {}: {};\n",
				jsdoc(field.docs.as_deref(), "  "),
				field.name.to_lower_camel_case(),
				ts_type(&field.type_ref, imports)
			)
		})
		.collect()
}

fn inline_object(fields: &[Field], imports: &mut Imports) -> String {
	let fields = fields
		.iter()
		.map(|field| {
			format!("{}: {}", field.name.to_lower_camel_case(), ts_type(&field.type_ref, imports))
		})
		.collect::<Vec<_>>()
		.join("; ");
	format!("{{ {fields} }}")
}

fn struct_codec(fields: &[Field], imports: &mut Imports) -> String {
	let fields = fields
		.iter()
		.map(|field| {
			format!("{}: {}", field.name.to_lower_camel_case(), codec(&field.type_ref, imports))
		})
		.collect::<Vec<_>>()
		.join(", ");
	format!("S.Struct({{ {fields} }})")
}

/// A JSDoc block for `docs`, each line prefixed with `indent`; empty without
/// docs.
pub fn jsdoc(docs: Option<&str>, indent: &str) -> String {
	let Some(docs) = docs.map(str::trim).filter(|docs| !docs.is_empty()) else {
		return String::new();
	};
	let lines: String =
		docs.lines()
			.map(|line| {
				if line.is_empty() {
					format!("{indent} *\n")
				} else {
					format!("{indent} * {line}\n")
				}
			})
			.collect();
	format!("{indent}/**\n{lines}{indent} */\n")
}

/// The `// Generated …` line every generated file starts with.
pub fn header(source: &str) -> String {
	format!("// Generated by tool-codegen from {source}. Do not edit.")
}

#[cfg(test)]
mod tests;
