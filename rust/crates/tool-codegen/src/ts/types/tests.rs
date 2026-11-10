use indoc::indoc;

use super::*;
use crate::model::{EnumVariant, TypeDef, TypeName};

fn name(path: &str) -> TypeName {
	TypeName::new(path.split("::").map(str::to_string).collect())
}

#[test]
fn each_variant_shape_has_its_union_member() {
	let def = TypeDef {
		name: name("protocol::api::signing::HostSignRawError"),
		docs: None,
		kind: TypeDefKind::Enum(vec![
			EnumVariant { name: "Rejected".into(), docs: None, shape: VariantShape::Unit },
			EnumVariant {
				name: "Other".into(),
				docs: None,
				shape: VariantShape::Named(vec![
					Field { name: "public_key".into(), docs: None, type_ref: TypeRef::Bytes },
					Field {
						name: "account".into(),
						docs: None,
						type_ref: TypeRef::Named(name("protocol::api::account::ProductAccount")),
					},
				]),
			},
		]),
	};

	assert_eq!(
		type_declaration(&def),
		indoc! {r#"
			export type HostSignRawError =
			  | { tag: "Rejected"; value?: undefined }
			  | { tag: "Other"; value: { publicKey: Uint8Array; account: ProductAccount } };
		"#},
	);
}

#[test]
fn a_list_of_optional_values_keeps_its_parentheses() {
	let list = TypeRef::Vec(Box::new(TypeRef::Option(Box::new(TypeRef::U64))));

	assert_eq!(ts_type(&list), "(bigint | undefined)[]");
}
