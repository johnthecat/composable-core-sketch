use std::sync::Mutex;

use futures::{executor::block_on, stream};

use super::*;

/// Wallet with fixed answers that records every sign request it receives.
struct RecordingWallet {
	accounts: Vec<Account>,
	updates: Vec<Vec<Account>>,
	signed: Mutex<Vec<SignRequest>>,
}

impl RecordingWallet {
	fn holding(accounts: Vec<Account>) -> Arc<Self> {
		Arc::new(Self { accounts, updates: Vec::new(), signed: Mutex::new(Vec::new()) })
	}

	fn signed(&self) -> Vec<SignRequest> {
		self.signed.lock().unwrap().clone()
	}
}

#[async_trait::async_trait]
impl Wallet for RecordingWallet {
	async fn accounts(&self) -> Result<Vec<Account>, WalletError> {
		Ok(self.accounts.clone())
	}

	async fn sign(&self, request: SignRequest) -> Result<Signature, WalletError> {
		self.signed.lock().unwrap().push(request);
		Ok(Signature { bytes: [9; 64] })
	}

	fn subscribe_accounts(&self) -> BoxStream<'static, Result<Vec<Account>, WalletError>> {
		stream::iter(self.updates.clone().into_iter().map(Ok)).boxed()
	}
}

fn account(name: &str, key: u8) -> Account {
	Account { public_key: [key; 32], name: name.into() }
}

#[test]
fn a_message_is_signed_wrapped_so_it_cannot_pass_for_a_transaction() {
	let wallet = RecordingWallet::holding(vec![account("alice", 1)]);
	let runtime = HostRuntime::new(wallet.clone());

	block_on(runtime.sign_message([1; 32], "hi")).unwrap();

	assert_eq!(
		wallet.signed(),
		[SignRequest { public_key: [1; 32], payload: b"<Bytes>hi</Bytes>".to_vec() }],
	);
}

#[test]
fn an_unknown_account_fails_before_the_wallet_is_asked_to_sign() {
	let wallet = RecordingWallet::holding(vec![account("alice", 1)]);
	let runtime = HostRuntime::new(wallet.clone());

	let result = block_on(runtime.sign_message([2; 32], "hi"));

	assert_eq!((result, wallet.signed()), (Err(WalletError::UnknownAccount), Vec::new()));
}

#[test]
fn an_account_update_that_repeats_the_list_is_dropped() {
	let alice = account("alice", 1);
	let bob = account("bob", 2);
	let wallet = Arc::new(RecordingWallet {
		accounts: Vec::new(),
		updates: vec![vec![alice.clone()], vec![alice.clone()], vec![alice.clone(), bob.clone()]],
		signed: Mutex::new(Vec::new()),
	});
	let runtime = HostRuntime::new(wallet);

	let updates: Vec<_> = block_on(runtime.subscribe_accounts().collect());

	assert_eq!(updates, [Ok(vec![alice.clone()]), Ok(vec![alice, bob])]);
}
