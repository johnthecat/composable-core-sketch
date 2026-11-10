use futures::executor::block_on;

use host_extension::Bytes;

use super::*;

fn sign_request(account: &ProductAccount) -> SignRequest {
	SignRequest { public_key: account.public_key, payload: Bytes(b"hello".to_vec()) }
}

#[test]
fn the_phone_answers_each_request_as_it_was_last_told() {
	let wallet = MockWallet::new(Duration::ZERO);
	let alice = wallet.add_account("alice");

	let approved = block_on(wallet.sign(sign_request(&alice)));
	wallet.set_decision(PhoneDecision::Decline);
	let declined = block_on(wallet.sign(sign_request(&alice)));
	wallet.set_decision(PhoneDecision::Unreachable);
	let mut unreachable = Box::pin(wallet.sign(sign_request(&alice)));
	let unanswered = block_on(futures::future::poll_immediate(&mut unreachable)).is_none();

	assert_eq!(
		(approved.map(|_| ()), declined.map(|_| ()), unanswered),
		(Ok(()), Err(WalletError::Rejected), true),
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
	let stranger = SignRequest { public_key: Bytes32([7; 32]), payload: Bytes(Vec::new()) };

	assert_eq!(block_on(wallet.sign(stranger)), Err(WalletError::UnknownAccount));
}

/// Records every list pushed to it.
#[derive(Default)]
struct RecordingListener(std::sync::Mutex<Vec<Vec<String>>>);

impl AccountsListener for RecordingListener {
	fn accounts_changed(&self, accounts: Vec<ProductAccount>) {
		self.0
			.lock()
			.unwrap()
			.push(accounts.into_iter().map(|account| account.name).collect());
	}
}

#[test]
fn an_added_account_is_pushed_to_the_last_listener_given() {
	let wallet = MockWallet::new(Duration::ZERO);
	let replaced = Arc::new(RecordingListener::default());
	let current = Arc::new(RecordingListener::default());
	wallet.add_account("alice");

	wallet.set_accounts_listener(replaced.clone());
	wallet.set_accounts_listener(current.clone());
	wallet.add_account("bob");

	assert_eq!(
		(replaced.0.lock().unwrap().clone(), current.0.lock().unwrap().clone()),
		(vec![], vec![vec!["alice".to_string(), "bob".to_string()]]),
	);
}
