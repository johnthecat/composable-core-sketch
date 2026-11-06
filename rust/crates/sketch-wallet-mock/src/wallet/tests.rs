use futures::executor::block_on;

use super::*;

fn sign_request(account: &Account) -> SignRequest {
	SignRequest { public_key: account.public_key, payload: b"hello".to_vec() }
}

#[test]
fn the_phone_answers_each_request_as_it_was_last_told() {
	let wallet = MockWallet::new(Duration::ZERO);
	let alice = wallet.add_account("alice");

	let approved = block_on(wallet.sign(sign_request(&alice)));
	wallet.set_decision(PhoneDecision::Decline);
	let declined = block_on(wallet.sign(sign_request(&alice)));
	wallet.set_decision(PhoneDecision::Unreachable);
	let unreachable = block_on(wallet.sign(sign_request(&alice)));

	assert_eq!(
		[approved.map(|_| ()), declined.map(|_| ()), unreachable.map(|_| ())],
		[
			Ok(()),
			Err(WalletError::Rejected),
			Err(WalletError::Unknown { reason: "the phone did not answer".into() })
		],
	);
}

#[test]
fn a_signature_binds_the_payload_to_the_account() {
	let wallet = MockWallet::new(Duration::ZERO);
	let alice = wallet.add_account("alice");
	let bob = wallet.add_account("bob");

	let by_alice = block_on(wallet.sign(sign_request(&alice))).unwrap();
	let by_bob = block_on(wallet.sign(sign_request(&bob))).unwrap();
	let again_by_alice = block_on(wallet.sign(sign_request(&alice))).unwrap();

	assert_ne!(by_alice, by_bob);
	assert_eq!(by_alice, again_by_alice);
}

#[test]
fn signing_with_an_account_the_wallet_does_not_hold_fails() {
	let wallet = MockWallet::new(Duration::ZERO);
	let stranger = SignRequest { public_key: [7; 32], payload: Vec::new() };

	assert_eq!(block_on(wallet.sign(stranger)), Err(WalletError::UnknownAccount));
}

#[test]
fn a_subscriber_sees_the_current_list_then_each_added_account() {
	let wallet = MockWallet::new(Duration::ZERO);
	let alice = wallet.add_account("alice");
	let mut accounts = wallet.subscribe_accounts();

	let bob = wallet.add_account("bob");

	assert_eq!(block_on(accounts.next()), Some(Ok(vec![alice.clone()])));
	assert_eq!(block_on(accounts.next()), Some(Ok(vec![alice, bob])));
}
