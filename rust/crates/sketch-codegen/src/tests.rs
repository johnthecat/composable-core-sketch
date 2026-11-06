//! Snapshots of what the emitters produce for the `Wallet` extension point.
//!
//! Rerun with `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1` after a deliberate change to an
//! emitter, and review the diff of `src/tests/*.snap`. The committed generated
//! files are these outputs after rustfmt and prettier; `make check-generated`
//! keeps them current.

use std::{env, fs, path::Path};

use crate::{
	model::{
		ExtensionTrait, Extensions, Field, Method, MethodKind, Param, TypeDef, TypeDefKind,
		TypeRef, Variant,
	},
	rust, ts,
};

fn field(name: &str, type_ref: TypeRef) -> Field {
	Field { name: name.into(), docs: Some(format!("The {name}.")), type_ref }
}

fn wallet() -> Extensions {
	let error = || "WalletError".to_string();
	let accounts = || TypeRef::Vec(Box::new(TypeRef::Named("Account".into())));
	Extensions {
		crate_name: "sketch_wallet_extension".into(),
		traits: vec![ExtensionTrait {
			name: "Wallet".into(),
			docs: Some("Holds the user's accounts.".into()),
			methods: vec![
				Method {
					name: "accounts".into(),
					docs: Some("Lists the accounts.".into()),
					params: Vec::new(),
					kind: MethodKind::Async { ok: accounts(), error: error() },
				},
				Method {
					name: "sign".into(),
					docs: Some("Signs a payload.".into()),
					params: vec![Param {
						name: "request".into(),
						type_ref: TypeRef::Named("SignRequest".into()),
					}],
					kind: MethodKind::Async {
						ok: TypeRef::Named("Signature".into()),
						error: error(),
					},
				},
				Method {
					name: "subscribe_accounts".into(),
					docs: Some("The account list.\n\nAn error ends the stream.".into()),
					params: Vec::new(),
					kind: MethodKind::Stream { item: accounts(), error: error() },
				},
			],
		}],
		types: vec![
			TypeDef {
				name: "Account".into(),
				docs: Some("An account.".into()),
				kind: TypeDefKind::Struct(vec![
					field("public_key", TypeRef::FixedBytes(32)),
					field("name", TypeRef::String),
				]),
			},
			TypeDef {
				name: "SignRequest".into(),
				docs: None,
				kind: TypeDefKind::Struct(vec![
					field("public_key", TypeRef::FixedBytes(32)),
					field("payload", TypeRef::Bytes),
				]),
			},
			TypeDef {
				name: "Signature".into(),
				docs: None,
				kind: TypeDefKind::Struct(vec![field("bytes", TypeRef::FixedBytes(64))]),
			},
			TypeDef {
				name: "WalletError".into(),
				docs: Some("Why the wallet did not answer.".into()),
				kind: TypeDefKind::Enum(vec![
					Variant { name: "Rejected".into(), docs: None, fields: Vec::new() },
					Variant {
						name: "Unknown".into(),
						docs: Some("A failure the trait does not declare.".into()),
						fields: vec![field("reason", TypeRef::String)],
					},
				]),
			},
		],
	}
}

fn assert_snapshot(name: &str, actual: &str) {
	let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests").join(name);
	if env::var_os("SKETCH_CODEGEN_WRITE_SNAPSHOTS").is_some() {
		fs::write(&path, actual).unwrap();
		return;
	}
	let expected = fs::read_to_string(&path)
		.unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
	assert_eq!(actual, expected, "{name} differs from its snapshot");
}

#[test]
fn typescript_interfaces_match_the_snapshot() {
	assert_snapshot("wallet.extensions.ts.snap", &ts::interfaces(&wallet()));
}

#[test]
fn typescript_adapters_match_the_snapshot() {
	assert_snapshot("wallet.extensions-adapter.ts.snap", &ts::adapters(&wallet()));
}

#[test]
fn rust_wasm_bridge_matches_the_snapshot() {
	assert_snapshot("wallet.wasm.rs.snap", &rust::wasm_bridge(&wallet()));
}
