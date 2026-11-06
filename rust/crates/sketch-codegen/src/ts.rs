//! Emits the TypeScript side of the extension points.
//!
//! - [`interfaces`]: `extensions.ts`, with the typed interface of each trait, the types its methods
//!   name with their `scale-ts` codecs, and an exception class per declared error type.
//! - [`adapters`]: `extensions-adapter.ts`, with the raw interface of each trait as it crosses the
//!   WASM boundary, and the two adapters between it and the typed interface.

use std::{collections::BTreeSet, fmt::Write};

use indoc::{formatdoc, writedoc};

use crate::model::{
	Extensions, Field, Method, MethodKind, TypeDef, TypeDefKind, TypeRef, camel_case,
};

/// Module the hand-written stream and promise helpers live in.
const SUPPORT_MODULE: &str = "../adapter-support.js";
/// Module [`interfaces`] is written to, as [`adapters`] imports it.
const INTERFACES_MODULE: &str = "./extensions.js";

/// `extensions.ts`.
pub fn interfaces(extensions: &Extensions) -> String {
	let mut codec_imports = BTreeSet::new();
	let mut body = String::new();
	for type_def in &extensions.types {
		body.push_str(&type_definition(type_def, &mut codec_imports));
	}
	for error in extensions.error_types() {
		writedoc!(
			body,
			r#"

			/**
			 * Rejection carrying a {{@link {error}}}, a failure the extension point
			 * declares. Any other rejection is a failure it does not declare.
			 */
			export class {error}Exception extends Error {{
			  readonly error: {error};

			  constructor(error: {error}) {{
			    super(`{error}.${{error.tag}}`);
			    this.name = "{error}Exception";
			    this.error = error;
			  }}
			}}
			"#
		)
		.expect("writing to a String cannot fail");
	}
	for extension in &extensions.traits {
		let methods: String = extension
			.methods
			.iter()
			.map(|method| {
				let params = typed_params(method);
				let returns = match &method.kind {
					MethodKind::Async { ok, .. } => format!("Promise<{}>", ts_type(ok)),
					MethodKind::Stream { item, .. } => format!("AsyncIterable<{}>", ts_type(item)),
				};
				format!(
					"{}  {}({params}): {returns};\n",
					jsdoc(method.docs.as_deref(), "  "),
					camel_case(&method.name)
				)
			})
			.collect::<Vec<_>>()
			.join("\n");
		writedoc!(
			body,
			"

			{docs}export interface {name} {{
			{methods}}}
			",
			docs = jsdoc(extension.docs.as_deref(), ""),
			name = extension.name,
		)
		.expect("writing to a String cannot fail");
	}

	codec_imports.insert("type Codec");
	formatdoc!(
		r#"
		{header}

		import {{ {imports} }} from "scale-ts";
		{body}"#,
		header = header(extensions),
		imports = codec_imports.into_iter().collect::<Vec<_>>().join(", "),
	)
}

/// `extensions-adapter.ts`.
pub fn adapters(extensions: &Extensions) -> String {
	let mut codec_imports = BTreeSet::new();
	let mut named_imports = BTreeSet::new();
	let mut body = String::new();

	for error in extensions.error_types() {
		named_imports.insert(error.to_string());
		named_imports.insert(format!("{error}Exception"));
		writedoc!(
			body,
			"

			/** Encodes a {{@link {error}Exception}} as the raw interfaces reject with it; passes any other reason through. */
			export function encode{error}(reason: unknown): unknown {{
			  return reason instanceof {error}Exception ? {error}.enc(reason.error) : reason;
			}}

			/** Decodes a raw rejection carrying an encoded {{@link {error}}}; passes any other reason through. */
			export function decode{error}(reason: unknown): unknown {{
			  return reason instanceof Uint8Array ? new {error}Exception({error}.dec(reason)) : reason;
			}}
			"
		)
		.expect("writing to a String cannot fail");
	}

	let mut support_imports = BTreeSet::new();
	for extension in &extensions.traits {
		let name = &extension.name;
		named_imports.insert(format!("type {name}"));
		let mut raw_methods = Vec::new();
		let mut to_raw = Vec::new();
		let mut to_typed = Vec::new();
		for method in &extension.methods {
			let ts_name = camel_case(&method.name);
			let error = method.error();
			let raw_params: Vec<String> = method
				.params
				.iter()
				.map(|param| format!("{}: Uint8Array", camel_case(&param.name)))
				.collect();
			let arg_names: Vec<String> =
				method.params.iter().map(|param| camel_case(&param.name)).collect();
			let decoded_args: Vec<String> = method
				.params
				.iter()
				.map(|param| {
					let codec = codec(&param.type_ref, &mut codec_imports, &mut named_imports);
					format!("{codec}.dec({})", camel_case(&param.name))
				})
				.collect();
			let encoded_args: Vec<String> = method
				.params
				.iter()
				.map(|param| {
					let codec = codec(&param.type_ref, &mut codec_imports, &mut named_imports);
					format!("{codec}.enc({})", camel_case(&param.name))
				})
				.collect();
			let (arg_names, decoded_args, encoded_args) =
				(arg_names.join(", "), decoded_args.join(", "), encoded_args.join(", "));
			match &method.kind {
				MethodKind::Async { ok, .. } => {
					support_imports.insert("mapSettled");
					let codec = codec(ok, &mut codec_imports, &mut named_imports);
					raw_methods.push(format!(
						"{}  {ts_name}({}): Promise<Uint8Array>;",
						jsdoc(method.docs.as_deref(), "  "),
						raw_params.join(", ")
					));
					to_raw.push(format!(
						"    {ts_name}: ({arg_names}) => mapSettled(() => typed.{ts_name}({decoded_args}), (value) => {codec}.enc(value), encode{error}),"
					));
					to_typed.push(format!(
						"    {ts_name}: ({arg_names}) => mapSettled(() => raw.{ts_name}({encoded_args}), (bytes) => {codec}.dec(bytes), decode{error}),"
					));
				},
				MethodKind::Stream { item, .. } => {
					support_imports.insert("pumpIterable");
					support_imports.insert("subscriptionIterable");
					let codec = codec(item, &mut codec_imports, &mut named_imports);
					let callbacks =
						"onItem: (item: Uint8Array) => void, onError: (error: unknown) => void";
					let raw_params =
						raw_params.into_iter().chain([callbacks.to_string()]).collect::<Vec<_>>();
					raw_methods.push(format!(
						"{}  {ts_name}({}): () => void;",
						jsdoc(method.docs.as_deref(), "  "),
						raw_params.join(", ")
					));
					let raw_args = method
						.params
						.iter()
						.map(|param| camel_case(&param.name))
						.chain(["onItem".to_string(), "onError".to_string()])
						.collect::<Vec<_>>()
						.join(", ");
					let encoded_args = if encoded_args.is_empty() {
						encoded_args
					} else {
						format!("{encoded_args}, ")
					};
					to_raw.push(format!(
						"    {ts_name}: ({raw_args}) => pumpIterable(typed.{ts_name}({decoded_args}), (item) => onItem({codec}.enc(item)), (reason) => onError(encode{error}(reason))),"
					));
					to_typed.push(format!(
						"    {ts_name}: ({arg_names}) => subscriptionIterable<{}>((onItem, onError) => raw.{ts_name}({encoded_args}(item) => onItem({codec}.dec(item)), (reason) => onError(decode{error}(reason)))),",
						ts_type(item)
					));
				},
			}
		}
		writedoc!(
			body,
			"

			/**
			 * {{@link {name}}} as it crosses the WASM boundary. Arguments and results are
			 * SCALE bytes, and a declared failure rejects with the encoded error. The
			 * core's `Foreign{name}` calls it, and a provider's `Wasm{name}` implements it.
			 */
			export interface Raw{name} {{
			{raw_methods}
			}}

			/** Adapts a typed {{@link {name}}} to {{@link Raw{name}}}, for passing to the core. */
			export function raw{name}(typed: {name}): Raw{name} {{
			  return {{
			{to_raw}
			  }};
			}}

			/** Adapts a {{@link Raw{name}}}, such as a provider's `Wasm{name}`, to a typed {{@link {name}}}. */
			export function typed{name}(raw: Raw{name}): {name} {{
			  return {{
			{to_typed}
			  }};
			}}
			",
			raw_methods = raw_methods.join("\n\n"),
			to_raw = to_raw.join("\n"),
			to_typed = to_typed.join("\n"),
		)
		.expect("writing to a String cannot fail");
	}

	let mut imports = String::new();
	if !codec_imports.is_empty() {
		let codec_imports = codec_imports.into_iter().collect::<Vec<_>>().join(", ");
		writeln!(imports, r#"import {{ {codec_imports} }} from "scale-ts";"#)
			.expect("writing to a String cannot fail");
	}
	let support_imports = support_imports.into_iter().collect::<Vec<_>>().join(", ");
	let named_imports = named_imports.into_iter().collect::<Vec<_>>().join(", ");
	writedoc!(
		imports,
		r#"
		import {{ {support_imports} }} from "{SUPPORT_MODULE}";
		import {{ {named_imports} }} from "{INTERFACES_MODULE}";
		"#
	)
	.expect("writing to a String cannot fail");

	format!("{}\n\n{imports}{body}", header(extensions))
}

fn header(extensions: &Extensions) -> String {
	format!(
		"// Generated by sketch-codegen from the `{}` crate. Do not edit.",
		extensions.crate_name
	)
}

fn type_definition(type_def: &TypeDef, codec_imports: &mut BTreeSet<&'static str>) -> String {
	let name = &type_def.name;
	let docs = jsdoc(type_def.docs.as_deref(), "");
	let mut named = BTreeSet::new();
	match &type_def.kind {
		TypeDefKind::Struct(fields) => {
			codec_imports.insert("Struct");
			formatdoc!(
				"

				{docs}export interface {name} {{
				{fields}}}
				export const {name}: Codec<{name}> = {codec};
				",
				fields = interface_fields(fields, "  "),
				codec = struct_codec(fields, codec_imports, &mut named),
			)
		},
		TypeDefKind::Enum(variants) => {
			codec_imports.insert("Enum");
			let members: String = variants
				.iter()
				.map(|variant| {
					let value = if variant.fields.is_empty() {
						"value?: undefined".to_string()
					} else {
						format!("value: {}", inline_object(&variant.fields))
					};
					format!(
						"{}  | {{ tag: \"{}\"; {value} }}\n",
						jsdoc(variant.docs.as_deref(), "  "),
						variant.name
					)
				})
				.collect();
			let codecs = variants
				.iter()
				.map(|variant| {
					let codec = if variant.fields.is_empty() {
						codec_imports.insert("_void");
						"_void".to_string()
					} else {
						struct_codec(&variant.fields, codec_imports, &mut named)
					};
					format!("{}: {codec}", variant.name)
				})
				.collect::<Vec<_>>()
				.join(", ");
			formatdoc!(
				"

				{docs}export type {name} =
				{members};
				export const {name}: Codec<{name}> = Enum({{ {codecs} }}) as Codec<{name}>;
				",
				members = members.trim_end(),
			)
		},
	}
}

fn interface_fields(fields: &[Field], indent: &str) -> String {
	fields
		.iter()
		.map(|field| {
			format!(
				"{}{indent}{}: {};\n",
				jsdoc(field.docs.as_deref(), indent),
				camel_case(&field.name),
				ts_type(&field.type_ref)
			)
		})
		.collect()
}

fn inline_object(fields: &[Field]) -> String {
	let fields = fields
		.iter()
		.map(|field| format!("{}: {}", camel_case(&field.name), ts_type(&field.type_ref)))
		.collect::<Vec<_>>()
		.join("; ");
	format!("{{ {fields} }}")
}

fn struct_codec(
	fields: &[Field],
	codec_imports: &mut BTreeSet<&'static str>,
	named: &mut BTreeSet<String>,
) -> String {
	codec_imports.insert("Struct");
	let fields = fields
		.iter()
		.map(|field| {
			format!("{}: {}", camel_case(&field.name), codec(&field.type_ref, codec_imports, named))
		})
		.collect::<Vec<_>>()
		.join(", ");
	format!("Struct({{ {fields} }})")
}

/// The parameters of a typed interface method.
fn typed_params(method: &Method) -> String {
	method
		.params
		.iter()
		.map(|param| format!("{}: {}", camel_case(&param.name), ts_type(&param.type_ref)))
		.collect::<Vec<_>>()
		.join(", ")
}

/// TypeScript type of a value.
fn ts_type(type_ref: &TypeRef) -> String {
	match type_ref {
		TypeRef::Unit => "void".into(),
		TypeRef::Bool => "boolean".into(),
		TypeRef::U8 | TypeRef::U16 | TypeRef::U32 => "number".into(),
		TypeRef::U64 => "bigint".into(),
		TypeRef::String => "string".into(),
		TypeRef::Bytes | TypeRef::FixedBytes(_) => "Uint8Array".into(),
		TypeRef::Vec(element) => match **element {
			TypeRef::Option(_) => format!("({})[]", ts_type(element)),
			_ => format!("{}[]", ts_type(element)),
		},
		TypeRef::Option(inner) => format!("{} | undefined", ts_type(inner)),
		TypeRef::Named(name) => name.clone(),
	}
}

/// `scale-ts` codec expression of a value, recording what it imports.
fn codec(
	type_ref: &TypeRef,
	codec_imports: &mut BTreeSet<&'static str>,
	named: &mut BTreeSet<String>,
) -> String {
	let (import, expression) = match type_ref {
		TypeRef::Unit => ("_void", "_void".to_string()),
		TypeRef::Bool => ("bool", "bool".to_string()),
		TypeRef::U8 => ("u8", "u8".to_string()),
		TypeRef::U16 => ("u16", "u16".to_string()),
		TypeRef::U32 => ("u32", "u32".to_string()),
		TypeRef::U64 => ("u64", "u64".to_string()),
		TypeRef::String => ("str", "str".to_string()),
		TypeRef::Bytes => ("Bytes", "Bytes()".to_string()),
		TypeRef::FixedBytes(len) => ("Bytes", format!("Bytes({len})")),
		TypeRef::Vec(element) => {
			("Vector", format!("Vector({})", codec(element, codec_imports, named)))
		},
		TypeRef::Option(inner) => {
			("Option", format!("Option({})", codec(inner, codec_imports, named)))
		},
		TypeRef::Named(name) => {
			named.insert(name.clone());
			return name.clone();
		},
	};
	codec_imports.insert(import);
	expression
}

/// A JSDoc block for `docs`, each line prefixed with `indent`; empty without docs.
fn jsdoc(docs: Option<&str>, indent: &str) -> String {
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
