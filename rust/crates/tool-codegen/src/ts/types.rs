//! TypeScript types for Rust structs and enums, named as in Rust.

use heck::ToLowerCamelCase;
use indoc::formatdoc;

use crate::model::{EnumVariant, Field, TypeDef, TypeDefKind, TypeRef, VariantShape};

/// TypeScript type of a value.
pub fn ts_type(type_ref: &TypeRef) -> String {
	match type_ref {
		TypeRef::Unit => "undefined".into(),
		TypeRef::Bool => "boolean".into(),
		TypeRef::U8 | TypeRef::U16 | TypeRef::U32 => "number".into(),
		TypeRef::U64 => "bigint".into(),
		TypeRef::String => "string".into(),
		TypeRef::Bytes => "Uint8Array".into(),
		TypeRef::Vec(element) => match **element {
			TypeRef::Option(_) => format!("({})[]", ts_type(element)),
			_ => format!("{}[]", ts_type(element)),
		},
		TypeRef::Option(inner) => format!("{} | undefined", ts_type(inner)),
		TypeRef::Named(name) => name.rust_name().to_string(),
	}
}

/// The type of one struct or enum: a struct is an interface with camelCase
/// fields, an enum a union of `{ tag, value }` objects.
pub fn type_declaration(def: &TypeDef) -> String {
	let name = def.name.rust_name();
	let docs = jsdoc(def.docs.as_deref(), "");
	match &def.kind {
		TypeDefKind::Struct(fields) => formatdoc!(
			"
			{docs}export interface {name} {{
			{fields}}}
			",
			fields = interface_fields(fields),
		),
		TypeDefKind::Enum(variants) => {
			let members: String = variants.iter().map(union_member).collect();
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

fn union_member(variant: &EnumVariant) -> String {
	let value = match &variant.shape {
		VariantShape::Unit => "value?: undefined".to_string(),
		VariantShape::Named(fields) => format!("value: {}", inline_object(fields)),
	};
	format!(
		"{}  | {{ tag: \"{}\"; {value} }}\n",
		jsdoc(variant.docs.as_deref(), "  "),
		variant.name
	)
}

fn interface_fields(fields: &[Field]) -> String {
	fields
		.iter()
		.map(|field| {
			format!(
				"{}  {}: {};\n",
				jsdoc(field.docs.as_deref(), "  "),
				field.name.to_lower_camel_case(),
				ts_type(&field.type_ref)
			)
		})
		.collect()
}

fn inline_object(fields: &[Field]) -> String {
	let fields = fields
		.iter()
		.map(|field| format!("{}: {}", field.name.to_lower_camel_case(), ts_type(&field.type_ref)))
		.collect::<Vec<_>>()
		.join("; ");
	format!("{{ {fields} }}")
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

#[cfg(test)]
mod tests;
