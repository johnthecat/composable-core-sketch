use serde_json::json;

use super::*;

#[test]
fn async_trait_methods_are_read_through_their_boxed_future() {
	let output = json!({ "resolved_path": { "path": "::core::pin::Pin", "args": { "angle_bracketed": { "args": [
		{ "type": { "resolved_path": { "path": "Box", "args": { "angle_bracketed": { "args": [
			{ "type": { "dyn_trait": { "traits": [
				{ "trait": { "path": "::core::future::Future", "args": { "angle_bracketed": {
					"args": [],
					"constraints": [{ "name": "Output", "binding": { "equality": { "type": { "primitive": "u8" } } } }]
				} } } },
				{ "trait": { "path": "::core::marker::Send", "args": null } }
			] } } }
		] } } } } }
	] } } } });

	assert_eq!(async_trait_output(&output), Some(&json!({ "primitive": "u8" })));
	assert_eq!(async_trait_output(&json!({ "primitive": "u8" })), None);
}

#[test]
fn wire_tags_are_read_from_docs_and_kept_out_of_them() {
	let docs = " Sign raw bytes.\n@wire_id=7";

	assert_eq!(
		(tag_value(Some(docs), "wire_id"), clean_docs(Some(docs))),
		(Some(7), Some("Sign raw bytes.".to_string())),
	);
}

#[test]
fn a_newtype_resolves_to_its_field_and_a_record_stays_named() {
	let krate = parse(
		&json!({
			"format_version": MIN_FORMAT_VERSION,
			"root": 0,
			"index": {
				"0": { "name": "protocol", "docs": null, "inner": { "module": {} } },
				"1": { "name": "Bytes32", "docs": null,
					"inner": { "struct": { "kind": { "tuple": [2] } } } },
				"2": { "name": "0", "docs": null, "inner": { "struct_field":
					{ "array": { "type": { "primitive": "u8" }, "len": "32" } } } },
				"3": { "name": "Account", "docs": null,
					"inner": { "struct": { "kind": { "plain": { "fields": [] } } } } }
			},
			"paths": {
				"1": { "crate_id": 0, "path": ["protocol", "Bytes32"], "kind": "struct" },
				"3": { "crate_id": 0, "path": ["protocol", "Account"], "kind": "struct" }
			}
		})
		.to_string(),
	)
	.unwrap();
	let world = World::new(vec![&krate]);
	let named = |id: u32| json!({ "resolved_path": { "path": "x", "id": id, "args": null } });

	assert_eq!(
		(world.type_ref(&krate, &named(1)).unwrap(), world.type_ref(&krate, &named(3)).unwrap()),
		(
			TypeRef::FixedBytes(32),
			TypeRef::Named(TypeName::new(vec!["protocol".into(), "Account".into()]))
		),
	);
}

#[test]
fn a_public_module_exports_its_items() {
	let krate = parse(
		&json!({
			"format_version": MIN_FORMAT_VERSION,
			"root": 0,
			"index": {
				"0": { "name": "host_extension", "docs": null, "inner": { "module": { "items": [1, 3] } } },
				"1": { "name": "wallet", "docs": null, "inner": { "module": { "items": [2] } } },
				"2": { "name": "Wallet", "docs": null, "inner": { "trait": { "items": [] } } },
				"3": { "name": "Notifier", "docs": null, "inner": { "trait": { "items": [] } } }
			},
			"paths": {}
		})
		.to_string(),
	)
	.unwrap();

	let names: Vec<_> = krate
		.root_items()
		.unwrap()
		.into_iter()
		.map(|(_, item)| item.name.clone())
		.collect();

	assert_eq!(names, [Some("Wallet".to_string()), Some("Notifier".to_string())]);
}
