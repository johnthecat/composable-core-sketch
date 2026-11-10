//! The product client, `@sketch/protocol-client`:
//!
//! - `types.ts` with every protocol type and its codec;
//! - `wire-table.ts` with each method's wire pair;
//! - `client.ts` with one class per service over the byte-level transport.

use std::fmt::Write;

use heck::{ToLowerCamelCase, ToShoutySnakeCase};
use indoc::{formatdoc, writedoc};

use super::types::{codec, header, jsdoc, ts_type, type_definition};
use crate::{
	model::{TypeDef, TypeRef},
	protocol::{ApiKind, ApiMethod, ApiTrait},
};

const SOURCE: &str = "the `protocol` crate";

/// `types.ts`: every type the protocol names.
pub fn types(defs: &[TypeDef]) -> String {
	let body: String = defs.iter().map(|def| format!("\n{}", type_definition(def))).collect();
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
pub fn client(traits: &[ApiTrait]) -> String {
	let mut classes = String::new();
	for api in traits {
		let methods = api
			.methods
			.iter()
			.map(|method| client_method(api, method))
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

fn client_method(api: &ApiTrait, method: &ApiMethod) -> String {
	let name = method.name.to_lower_camel_case();
	let docs = jsdoc(method.docs.as_deref(), "  ");
	let (param, payload) = match &method.request {
		Some(request) => (
			format!("request: {}", qualified(request)),
			format!("{}.enc(request)", qualified_codec(request)),
		),
		None => (String::new(), "new Uint8Array()".to_string()),
	};
	let ids = format!("W.{}", wire_constant(api, method));
	match &method.kind {
		ApiKind::Request { response, error } => {
			let ok = qualified(response);
			let err = format!("S.CallErrorValue<{}>", qualified(error));
			formatdoc!(
				"
				{docs}  {name}({param}): ResultAsync<{ok}, {err}> {{
				    return this.#transport.request<{ok}, {err}>({{
				      ids: {ids},
				      payload: {payload},
				      decodeResponse: (payload) =>
				        S.Result({response}, S.CallError({error})).dec(payload),
				    }});
				  }}
				",
				response = qualified_codec(response),
				error = qualified_codec(error),
			)
		},
		ApiKind::Subscription { item, error } => {
			let item_type = qualified(item);
			let err = format!("S.CallErrorValue<{}>", qualified(error));
			formatdoc!(
				"
				{docs}  {name}({param}): ObservableLike<{item_type}, {err}> {{
				    return createObservable<{item_type}, {err}>({{
				      transport: this.#transport,
				      ids: {ids},
				      payload: {payload},
				      decodeItem: (payload) => {item}.dec(payload),
				      decodeInterrupt: interruptDecoder(S.CallError({error})),
				    }});
				  }}
				",
				item = qualified_codec(item),
				error = qualified_codec(error),
			)
		},
	}
}

/// A type as `client.ts` names it, with structs and enums under `T.`.
fn qualified(type_ref: &TypeRef) -> String {
	match type_ref {
		TypeRef::Named(name) => format!("T.{}", name.rust_name()),
		TypeRef::Vec(element) => format!("{}[]", qualified(element)),
		TypeRef::Option(inner) => format!("{} | undefined", qualified(inner)),
		other => ts_type(other),
	}
}

/// The codec of a type as `client.ts` names it, with structs and enums under
/// `T.`.
fn qualified_codec(type_ref: &TypeRef) -> String {
	match type_ref {
		TypeRef::Named(name) => format!("T.{}", name.rust_name()),
		TypeRef::Vec(element) => format!("S.Vector({})", qualified_codec(element)),
		TypeRef::Option(inner) => format!("S.Option({})", qualified_codec(inner)),
		other => codec(other),
	}
}
