use indoc::indoc;

use super::*;
use crate::model::{EnumVariant, TypeDef};

fn name(path: &str) -> TypeName {
	TypeName::new(path.split("::").map(str::to_string).collect())
}

#[test]
fn an_envelope_is_prefixed_and_each_variant_shape_has_its_codec() {
	let def = TypeDef {
		name: name("protocol::versioned::signing::HostSignRawError"),
		docs: None,
		kind: TypeDefKind::Enum(vec![
			EnumVariant { name: "Plain".into(), docs: None, shape: VariantShape::Unit },
			EnumVariant {
				name: "V1".into(),
				docs: None,
				shape: VariantShape::Newtype(TypeRef::Named(name(
					"protocol::v01::HostSignRawError",
				))),
			},
			EnumVariant {
				name: "Other".into(),
				docs: None,
				shape: VariantShape::Named(vec![Field {
					name: "public_key".into(),
					docs: None,
					type_ref: TypeRef::FixedBytes(32),
				}]),
			},
		]),
	};

	assert_eq!(
		type_definition(&def, &mut Imports::default()),
		indoc! {r#"
			export type VersionedHostSignRawError =
			  | { tag: "Plain"; value?: undefined }
			  | { tag: "V1"; value: HostSignRawError }
			  | { tag: "Other"; value: { publicKey: Uint8Array } };
			export const VersionedHostSignRawError: S.Codec<VersionedHostSignRawError> = S.Enum({ Plain: S._void, V1: HostSignRawError, Other: S.Struct({ publicKey: S.Bytes(32) }) }) as unknown as S.Codec<VersionedHostSignRawError>;
		"#},
	);
}

#[test]
fn a_list_of_optional_values_keeps_its_parentheses() {
	let list = TypeRef::Vec(Box::new(TypeRef::Option(Box::new(TypeRef::U64))));

	assert_eq!(
		(ts_type(&list, &mut Imports::default()), codec(&list, &mut Imports::default())),
		("(bigint | undefined)[]".to_string(), "S.Vector(S.Option(S.u64))".to_string()),
	);
}
