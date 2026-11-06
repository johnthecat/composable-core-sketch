use parity_scale_codec::{Decode, Encode};

use super::*;

// The generated TypeScript codecs must produce the same bytes, or a value
// changes on its way across the WASM boundary. `js/packages/host/src/
// extensions.test.ts` asserts these vectors on the TypeScript side.

#[test]
fn sign_request_encodes_as_the_typescript_vector() {
	let request = SignRequest { public_key: [1; 32], payload: b"hi".to_vec() };

	let expected = [[1; 32].as_slice(), &[0x08, b'h', b'i']].concat();
	assert_eq!(request.encode(), expected);
}

#[test]
fn account_list_encodes_as_the_typescript_vector() {
	let accounts = vec![Account { public_key: [2; 32], name: "bob".into() }];

	let expected = [&[0x04], [2; 32].as_slice(), &[0x0c, b'b', b'o', b'b']].concat();
	assert_eq!(accounts.encode(), expected);
}

#[test]
fn wallet_error_variants_keep_their_indices() {
	let errors = [
		WalletError::Rejected,
		WalletError::UnknownAccount,
		WalletError::Unknown { reason: "x".into() },
	];

	let encoded: Vec<Vec<u8>> = errors.iter().map(Encode::encode).collect();
	assert_eq!(encoded, [vec![0x00], vec![0x01], vec![0x02, 0x04, b'x']]);
}

#[test]
fn an_undeclared_failure_becomes_unknown_and_survives_a_round_trip() {
	let error = WalletError::from("phone unreachable".to_string());

	let decoded = WalletError::decode(&mut error.encode().as_slice()).unwrap();
	assert_eq!(decoded, WalletError::Unknown { reason: "phone unreachable".into() });
}
