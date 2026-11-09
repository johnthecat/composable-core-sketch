use super::*;

// `#[extension_error]` writes the catch-all variant and the conversion into
// it; a declared variant keeps its own message.

#[test]
fn an_undeclared_failure_becomes_the_added_unknown_variant() {
	let error = WalletError::from("phone unreachable".to_string());
	let message = error.to_string();

	assert_eq!(
		(error, message),
		(WalletError::Unknown { reason: "phone unreachable".into() }, "phone unreachable".into()),
	);
}

#[test]
fn a_declared_failure_keeps_its_message() {
	assert_eq!(WalletError::Rejected.to_string(), "the user declined the request");
}
