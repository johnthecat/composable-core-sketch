//! The product client, `@sketch/protocol-client`: `types.ts` with every protocol type
//! and its codec, `wire-table.ts` with each method's wire pair, and
//! `client.ts` with one class per service over the byte-level transport.

use std::fmt::Write;

use heck::{ToLowerCamelCase, ToShoutySnakeCase};
use indoc::{formatdoc, writedoc};

use super::types::{Imports, header, jsdoc, ts_type, type_definition};
use crate::{
	model::{EnumVariant, TypeDef, TypeDefKind, TypeRef, VariantShape},
	protocol::{ApiKind, ApiMethod, ApiTrait},
};

const SOURCE: &str = "the `protocol` crate";

/// `types.ts`: every type the protocol names.
pub fn types(defs: &[TypeDef]) -> String {
	let mut imports = Imports::default();
	let body: String = defs
		.iter()
		.map(|def| format!("\n{}", type_definition(def, &mut imports)))
		.collect();
	format!("{}\n\nimport * as S from \"../scale.js\";\n{body}", header(SOURCE))
}

/// `wire-table.ts`: the wire pair and exchange shape of every method.
pub fn wire_table(traits: &[ApiTrait]) -> String {
	let mut body = String::new();
	for api in traits {
		for method in &api.methods {
			writedoc!(
				body,
				"

				/** Wire pair of `{client}.{method}`. */
				export const {constant}: MethodIds = {{ trait: {trait_id}, method: {method_id} }};
				",
				client = api.client_name(),
				method = method.name.to_lower_camel_case(),
				constant = wire_constant(api, method),
				trait_id = api.id,
				method_id = method.id,
			)
			.expect("writing to a String cannot fail");
		}
	}
	format!("{}\n\nimport type {{ MethodIds }} from \"../transport.js\";\n{body}", header(SOURCE))
}

/// `SIGNING_SIGN_RAW` for `Signing::sign_raw`, the name both wire tables use.
pub fn wire_constant(api: &ApiTrait, method: &ApiMethod) -> String {
	format!("{}_{}", api.name.to_shouty_snake_case(), method.name.to_shouty_snake_case())
}

/// `client.ts`: one class per service and `createClient` binding them to a
/// transport.
pub fn client(traits: &[ApiTrait], defs: &[TypeDef]) -> String {
	let mut classes = String::new();
	for api in traits {
		let methods = api
			.methods
			.iter()
			.map(|method| client_method(api, method, defs))
			.collect::<Vec<_>>()
			.join("\n");
		writedoc!(
			classes,
			"

			{docs}export class {name}Client {{
			  readonly #transport: TrUApiTransport;

			  constructor(transport: TrUApiTransport) {{
			    this.#transport = transport;
			  }}

			{methods}}}
			",
			docs = jsdoc(api.docs.as_deref(), ""),
			name = api.name,
		)
		.expect("writing to a String cannot fail");
	}
	let fields: String = traits
		.iter()
		.map(|api| format!("  readonly {}: {}Client;\n", api.client_name(), api.name))
		.collect();
	let constructors: String = traits
		.iter()
		.map(|api| format!("    {}: new {}Client(transport),\n", api.client_name(), api.name))
		.collect();
	formatdoc!(
		r#"
		{header}

		import type {{ ResultAsync }} from "neverthrow";

		import * as S from "../scale.js";
		import {{
		  createObservable,
		  interruptDecoder,
		  type ObservableLike,
		  type TrUApiTransport,
		}} from "../transport.js";
		import * as T from "./types.js";
		import * as W from "./wire-table.js";
		{classes}
		/** Every service, bound to one transport. */
		export interface TrUApiClient {{
		{fields}}}

		/** Bind each service to `transport`. */
		export function createClient(transport: TrUApiTransport): TrUApiClient {{
		  return {{
		{constructors}  }};
		}}
		"#,
		header = header(SOURCE),
	)
}

fn client_method(api: &ApiTrait, method: &ApiMethod, defs: &[TypeDef]) -> String {
	let mut imports = Imports::default();
	let name = method.name.to_lower_camel_case();
	let docs = jsdoc(method.docs.as_deref(), "  ");
	let request_envelope = envelope_name(&method.request);
	let (param, request_value) = match latest_payload(&method.request, defs) {
		Some(payload) => (format!("request: {}", qualified(&payload, &mut imports)), "request"),
		None => (String::new(), "undefined"),
	};
	let encode_request = format!(
		"T.{request_envelope}.enc({{ tag: \"V{version}\", value: {request_value} }})",
		version = latest_version(&method.request, defs),
	);
	let ids = format!("W.{}", wire_constant(api, method));
	match &method.kind {
		ApiKind::Request { response, error } => {
			let ok = latest_payload(response, defs)
				.map(|payload| qualified(&payload, &mut imports))
				.unwrap_or_else(|| "undefined".into());
			let err = format!("S.CallErrorValue<T.{}>", envelope_name(error));
			formatdoc!(
				"
				{docs}  {name}({param}): ResultAsync<{ok}, {err}> {{
				    return this.#transport.request<{ok}, {err}>({{
				      ids: {ids},
				      payload: {encode_request},
				      decodeResponse: (payload) => {{
				        const result = S.Result(T.{response}, S.CallError(T.{error})).dec(payload);
				        return result.success ? {{ success: true, value: result.value.value }} : result;
				      }},
				    }});
				  }}
				",
				response = envelope_name(response),
				error = envelope_name(error),
			)
		},
		ApiKind::Subscription { item, error } => {
			let item_type = latest_payload(item, defs)
				.map(|payload| qualified(&payload, &mut imports))
				.unwrap_or_else(|| "undefined".into());
			let err = format!("S.CallErrorValue<T.{}>", envelope_name(error));
			formatdoc!(
				"
				{docs}  {name}({param}): ObservableLike<{item_type}, {err}> {{
				    return createObservable<{item_type}, {err}>({{
				      transport: this.#transport,
				      ids: {ids},
				      payload: {encode_request},
				      decodeItem: (payload) => T.{item}.dec(payload).value,
				      decodeInterrupt: interruptDecoder(S.CallError(T.{error})),
				    }});
				  }}
				",
				item = envelope_name(item),
				error = envelope_name(error),
			)
		},
	}
}

/// TypeScript name of a versioned envelope.
fn envelope_name(type_ref: &TypeRef) -> String {
	match type_ref {
		TypeRef::Named(name) => name.ts_name(),
		other => panic!("a protocol method names versioned envelopes, found {other:?}"),
	}
}

/// The newest variant of an envelope.
fn latest_variant<'d>(type_ref: &TypeRef, defs: &'d [TypeDef]) -> &'d EnumVariant {
	let TypeRef::Named(name) = type_ref else {
		panic!("a protocol method names versioned envelopes, found {type_ref:?}");
	};
	let def = defs
		.iter()
		.find(|def| &def.name == name)
		.expect("envelopes are reachable types");
	let TypeDefKind::Enum(variants) = &def.kind else {
		panic!("the versioned envelope {name} is not an enum");
	};
	variants.last().expect("an envelope has a variant")
}

fn latest_version(type_ref: &TypeRef, defs: &[TypeDef]) -> usize {
	let TypeRef::Named(name) = type_ref else { return 1 };
	defs.iter()
		.find(|def| &def.name == name)
		.and_then(|def| match &def.kind {
			TypeDefKind::Enum(variants) => Some(variants.len()),
			TypeDefKind::Struct(_) => None,
		})
		.unwrap_or(1)
}

/// The payload of an envelope's newest variant, or `None` when it carries none.
fn latest_payload(type_ref: &TypeRef, defs: &[TypeDef]) -> Option<TypeRef> {
	match &latest_variant(type_ref, defs).shape {
		VariantShape::Newtype(payload) => Some(payload.clone()),
		VariantShape::Unit => None,
		VariantShape::Named(_) => panic!("a versioned envelope variant carries one payload"),
	}
}

/// A type as `client.ts` names it, with structs and enums under `T.`.
fn qualified(type_ref: &TypeRef, imports: &mut Imports) -> String {
	match type_ref {
		TypeRef::Named(name) => format!("T.{}", name.ts_name()),
		TypeRef::Vec(element) => format!("{}[]", qualified(element, imports)),
		TypeRef::Option(inner) => format!("{} | undefined", qualified(inner, imports)),
		other => ts_type(other, imports),
	}
}
