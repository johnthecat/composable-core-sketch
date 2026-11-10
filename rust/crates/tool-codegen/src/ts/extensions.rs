//! `extensions.ts` in `@sketch/host-extension`:
//!
//! - the TypeScript interface of each extension point;
//! - the types it names;
//! - one exception class per declared error.
//!
//! The types are the shapes the web glue of `#[extension_point]` converts
//! Rust values to and from:
//!
//! - camelCase fields;
//! - bytes as `Uint8Array`;
//! - enums as `{ tag, value }`.
//!
//! Types from the protocol come from `@sketch/protocol-client`.
//! A call returns a `Promise`; a notification, such as a listener's, returns
//! `void` at once.

use std::fmt::Write;

use heck::ToLowerCamelCase;
use indoc::writedoc;

use super::types::{header, jsdoc, ts_type, type_declaration};
use crate::{
	extension::{ExtensionMethod, ExtensionTrait, Param, Returns},
	model::{TypeDef, TypeName, TypeRef},
};

/// `extensions.ts` for the extension points of `crate_name`, with `types`, every
/// type they name. A type another crate defines is imported from the product
/// client.
pub fn extensions(crate_name: &str, traits: &[ExtensionTrait], types: &[TypeDef]) -> String {
	let mut body = String::new();
	for def in types.iter().filter(|def| def.name.crate_name() == crate_name) {
		write!(body, "\n{}", type_declaration(def)).expect("writing to a String cannot fail");
	}
	for (error, extension) in errors(traits) {
		let error = error.rust_name();
		writedoc!(
			body,
			r#"

			/**
			 * A failure that {{@link {error}}} declares. The web glue rejects with an
			 * `Error` of the same name whose `error` holds it, which `instanceof`
			 * also accepts. Any other rejection arrives as the `Unknown` variant.
			 */
			export class {extension}Exception extends Error {{
			  readonly error: {error};

			  constructor(error: {error}) {{
			    super(`{error}.${{error.tag}}`);
			    this.name = "{extension}Exception";
			    this.error = error;
			  }}

			  static [Symbol.hasInstance](value: unknown): boolean {{
			    return value instanceof Error && value.name === "{extension}Exception" && "error" in value;
			  }}
			}}
			"#
		)
		.expect("writing to a String cannot fail");
	}
	for extension in traits {
		let methods = extension.methods.iter().map(signature).collect::<Vec<_>>().join("\n");
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
	let mut external: Vec<String> = types
		.iter()
		.filter(|def| def.name.crate_name() != crate_name)
		.map(|def| def.name.rust_name().to_string())
		.collect();
	external.sort();
	let import = if external.is_empty() {
		String::new()
	} else {
		format!("\nimport type {{ {} }} from \"@sketch/protocol-client\";\n", external.join(", "))
	};
	format!("{}\n{import}{body}", header("the `host-extension` crate"))
}

/// Each extension point's error, `<Trait>Error`, with the trait's name, which
/// names its exception `<Trait>Exception`.
fn errors(traits: &[ExtensionTrait]) -> Vec<(&TypeName, &str)> {
	traits
		.iter()
		.filter_map(|extension| {
			extension.methods.iter().find_map(|method| match &method.returns {
				Returns::Call { error, .. } => Some((error, extension.name.as_str())),
				Returns::Notification => None,
			})
		})
		.collect()
}

fn signature(method: &ExtensionMethod) -> String {
	let params = method
		.params
		.iter()
		.map(|(name, param)| {
			let ts = match param {
				Param::Value(type_ref) => ts_type(type_ref),
				Param::Interface(name) => name.clone(),
			};
			format!("{}: {ts}", name.to_lower_camel_case())
		})
		.collect::<Vec<_>>()
		.join(", ");
	let returns = match &method.returns {
		Returns::Call { ok: TypeRef::Unit, .. } => "Promise<void>".to_string(),
		Returns::Call { ok, .. } => format!("Promise<{}>", ts_type(ok)),
		Returns::Notification => "void".to_string(),
	};
	format!(
		"{}  {}({params}): {returns};\n",
		jsdoc(method.docs.as_deref(), "  "),
		method.name.to_lower_camel_case()
	)
}

#[cfg(test)]
mod tests;
