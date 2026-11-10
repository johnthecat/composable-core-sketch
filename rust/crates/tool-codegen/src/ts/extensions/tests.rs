//! A snapshot of `extensions.ts` for the RFC's `Notifier`, and a `Clock`
//! that pushes a protocol type to the `TimeListener` it is given.
//!
//! Rerun with `SKETCH_CODEGEN_WRITE_SNAPSHOTS=1` after a deliberate change to
//! the emitter, and review the diff of `src/ts/extensions/tests/*.snap`.

use std::{env, fs, path::Path};

use crate::{
	extension::{ExtensionMethod, ExtensionTrait, Param, Returns},
	model::{EnumVariant, Field, TypeDef, TypeDefKind, TypeName, TypeRef, VariantShape},
	ts::extensions::extensions,
};

fn name(path: &str) -> TypeName {
	TypeName::new(path.split("::").map(str::to_string).collect())
}

fn notifier_error() -> TypeDef {
	TypeDef {
		name: name("host_extension::notifier::NotifierError"),
		docs: Some("Why a notice was not shown.".into()),
		kind: TypeDefKind::Enum(vec![
			EnumVariant { name: "Muted".into(), docs: None, shape: VariantShape::Unit },
			EnumVariant {
				name: "Unknown".into(),
				docs: None,
				shape: VariantShape::Named(vec![Field {
					name: "reason".into(),
					docs: None,
					type_ref: TypeRef::String,
				}]),
			},
		]),
	}
}

fn traits() -> Vec<ExtensionTrait> {
	let error = notifier_error().name;
	let time = TypeRef::Named(name("protocol::api::Time"));
	vec![
		ExtensionTrait {
			name: "Notifier".into(),
			docs: Some("Shows a notice to the user.".into()),
			methods: vec![ExtensionMethod {
				name: "notify".into(),
				docs: Some("Shows `text`.".into()),
				params: vec![("text".into(), Param::Value(TypeRef::String))],
				returns: Returns::Call { ok: TypeRef::Unit, error },
			}],
		},
		ExtensionTrait {
			name: "Clock".into(),
			docs: None,
			methods: vec![ExtensionMethod {
				name: "set_time_listener".into(),
				docs: None,
				params: vec![("listener".into(), Param::Interface("TimeListener".into()))],
				returns: Returns::Notification,
			}],
		},
		ExtensionTrait {
			name: "TimeListener".into(),
			docs: None,
			methods: vec![ExtensionMethod {
				name: "time_changed".into(),
				docs: None,
				params: vec![("time".into(), Param::Value(time))],
				returns: Returns::Notification,
			}],
		},
	]
}

fn assert_snapshot(file: &str, actual: &str) {
	let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ts/extensions/tests").join(file);
	if env::var_os("SKETCH_CODEGEN_WRITE_SNAPSHOTS").is_some() {
		fs::write(&path, actual).unwrap();
		return;
	}
	let expected = fs::read_to_string(&path)
		.unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
	assert_eq!(actual, expected, "{file} differs from its snapshot");
}

/// A protocol type the listener names, which the file imports.
fn time() -> TypeDef {
	TypeDef {
		name: name("protocol::api::Time"),
		docs: None,
		kind: TypeDefKind::Struct(vec![Field {
			name: "millis".into(),
			docs: None,
			type_ref: TypeRef::U64,
		}]),
	}
}

#[test]
fn extension_points_emit_typed_interfaces_and_import_protocol_types() {
	let types = [notifier_error(), time()];

	assert_snapshot("extensions.ts.snap", &extensions("host_extension", &traits(), &types));
}
