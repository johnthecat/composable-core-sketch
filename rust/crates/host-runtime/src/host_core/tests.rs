//! The core driven the way a host drives it: product frames in, frames out.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

use futures::{
	executor::{LocalPool, block_on},
	future::BoxFuture,
	task::LocalSpawnExt,
};
use host_extension::{
	AccountsListener, Bytes, Bytes32, Bytes64, Notifier, NotifierError, ProductAccount,
	ProductStorage, ProductStorageError, SignRequest, Signature, UserConfirmation,
	UserConfirmationError, UserConfirmationReview, Wallet, WalletError,
};
use parity_scale_codec::{Decode, DecodeAll, Encode};
use protocol::{
	CallError,
	api::{
		account::ACCOUNT_ACCOUNTS_SUBSCRIBE, notifications::NOTIFICATIONS_PUSH_NOTIFICATION,
		signing::SIGNING_SIGN_RAW,
	},
	v01,
	versioned::{
		account::{HostAccountAccountsSubscribeItem, HostAccountAccountsSubscribeRequest},
		notifications::{HostPushNotificationError, HostPushNotificationRequest},
		signing::{HostSignRawError, HostSignRawRequest, HostSignRawResponse},
	},
};

use super::*;
use crate::frame::{MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_STOP};

const ALICE: Bytes32 = Bytes32([1; 32]);

/// A host whose user answers every confirmation with `confirms`.
struct TestHost {
	confirms: bool,
	reviews: Mutex<Vec<UserConfirmationReview>>,
	storage: Mutex<HashMap<String, Bytes>>,
}

#[async_trait::async_trait]
impl ProductStorage for TestHost {
	async fn read(&self, key: String) -> Result<Option<Bytes>, ProductStorageError> {
		Ok(self.storage.lock().unwrap().get(&key).cloned())
	}

	async fn write(&self, key: String, value: Bytes) -> Result<(), ProductStorageError> {
		self.storage.lock().unwrap().insert(key, value);
		Ok(())
	}
}

#[async_trait::async_trait]
impl UserConfirmation for TestHost {
	async fn confirm_user_action(
		&self,
		review: UserConfirmationReview,
	) -> Result<bool, UserConfirmationError> {
		self.reviews.lock().unwrap().push(review);
		Ok(self.confirms)
	}
}

/// A wallet holding Alice that records what it is asked to sign, and through
/// which a test pushes account lists to the core.
#[derive(Default)]
struct RecordingWallet {
	signed: Mutex<Vec<SignRequest>>,
	listener: Mutex<Option<Arc<dyn AccountsListener>>>,
}

impl RecordingWallet {
	fn push_accounts(&self, names: &[&str]) {
		let accounts = names
			.iter()
			.map(|name| ProductAccount { public_key: ALICE, name: name.to_string() })
			.collect();
		let listener = self.listener.lock().unwrap().clone();
		listener.expect("the core attached its listener").accounts_changed(accounts);
	}
}

#[async_trait::async_trait]
impl Wallet for RecordingWallet {
	async fn accounts(&self) -> Result<Vec<ProductAccount>, WalletError> {
		Ok(vec![ProductAccount { public_key: ALICE, name: "alice".into() }])
	}

	async fn sign(&self, request: SignRequest) -> Result<Signature, WalletError> {
		self.signed.lock().unwrap().push(request);
		Ok(Signature { bytes: Bytes64([9; 64]) })
	}

	fn set_accounts_listener(&self, listener: Arc<dyn AccountsListener>) {
		*self.listener.lock().unwrap() = Some(listener);
	}
}

/// A notifier that refuses every notice.
struct RefusingNotifier;

#[async_trait::async_trait]
impl Notifier for RefusingNotifier {
	async fn notify(&self, _text: String) -> Result<(), NotifierError> {
		Err(NotifierError::Rejected { reason: "muted".into() })
	}
}

/// The frames a product runtime sent.
#[derive(Default)]
struct RecordingSink(Mutex<Vec<ProtocolMessage>>);

impl FrameSink for RecordingSink {
	fn emit_frame(&self, frame: Vec<u8>) {
		self.0
			.lock()
			.unwrap()
			.push(ProtocolMessage::decode(&mut frame.as_slice()).unwrap());
	}
}

impl RecordingSink {
	fn take(&self) -> Vec<ProtocolMessage> {
		std::mem::take(&mut *self.0.lock().unwrap())
	}
}

/// Queues subscription pumps for the test to run on its own thread.
fn queueing_spawner() -> (Spawner, Arc<Mutex<Vec<BoxFuture<'static, ()>>>>) {
	let queue = Arc::new(Mutex::new(Vec::new()));
	let spawner: Spawner = {
		let queue = queue.clone();
		Arc::new(move |pump| queue.lock().unwrap().push(pump))
	};
	(spawner, queue)
}

struct Fixture {
	runtime: Arc<ProductRuntime>,
	sink: Arc<RecordingSink>,
	host: Arc<TestHost>,
	wallet: Arc<RecordingWallet>,
	pumps: Arc<Mutex<Vec<BoxFuture<'static, ()>>>>,
}

fn fixture(confirms: bool, with_wallet: bool) -> Fixture {
	let host =
		Arc::new(TestHost { confirms, reviews: Mutex::default(), storage: Mutex::default() });
	let wallet = Arc::new(RecordingWallet::default());
	let (spawner, pumps) = queueing_spawner();
	let runtime = HostRuntime::new(
		host.clone(),
		host.clone(),
		with_wallet.then(|| wallet.clone() as Arc<dyn Wallet>),
		Some(Arc::new(RefusingNotifier)),
		spawner,
	);
	let sink = Arc::new(RecordingSink::default());
	let runtime =
		runtime.product_runtime(ProductContext { product_id: "demo".into() }, sink.clone());
	Fixture { runtime, sink, host, wallet, pumps }
}

fn sign_raw(
	fixture: &Fixture,
	public_key: Bytes32,
) -> Result<HostSignRawResponse, CallError<HostSignRawError>> {
	let request =
		HostSignRawRequest::from(v01::HostSignRawRequest { public_key, payload: b"hi".to_vec() });
	block_on(
		fixture.runtime.receive_frame(
			ProtocolMessage::new(
				"p:1".into(),
				SIGNING_SIGN_RAW,
				MESSAGE_TYPE_REQUEST,
				request.encode(),
			)
			.encode(),
		),
	)
	.unwrap();
	let [response] = fixture.sink.take().try_into().unwrap();
	Decode::decode(&mut response.payload.value.as_slice()).unwrap()
}

#[test]
fn a_confirmed_message_is_signed_wrapped_so_it_cannot_pass_for_a_transaction() {
	let fixture = fixture(true, true);

	let response = sign_raw(&fixture, ALICE);

	assert_eq!(
		(response, fixture.wallet.signed.lock().unwrap().clone()),
		(
			Ok(HostSignRawResponse::V1(v01::HostSignRawResponse { signature: Bytes64([9; 64]) })),
			vec![SignRequest { public_key: ALICE, payload: Bytes(b"<Bytes>hi</Bytes>".to_vec()) }],
		),
	);
}

#[test]
fn a_refused_confirmation_fails_as_denied_before_the_wallet_signs() {
	let fixture = fixture(false, true);

	let response = sign_raw(&fixture, ALICE);

	assert_eq!(
		(
			response,
			fixture.host.reviews.lock().unwrap().len(),
			fixture.wallet.signed.lock().unwrap().len()
		),
		(Err(CallError::Denied), 1, 0),
	);
}

#[test]
fn an_unknown_account_fails_before_the_user_is_asked() {
	let fixture = fixture(true, true);

	let response = sign_raw(&fixture, Bytes32([2; 32]));

	assert_eq!(
		(response, fixture.host.reviews.lock().unwrap().len()),
		(Err(CallError::Domain(HostSignRawError::V1(v01::HostSignRawError::UnknownAccount))), 0),
	);
}

#[test]
fn without_a_wallet_signing_is_unsupported() {
	let fixture = fixture(true, false);

	assert_eq!(sign_raw(&fixture, ALICE), Err(CallError::Unsupported));
}

#[test]
fn a_notifier_refusal_reaches_the_product_as_its_domain_error() {
	let fixture = fixture(true, true);
	let request =
		HostPushNotificationRequest::from(v01::HostPushNotificationRequest { text: "hi".into() });

	block_on(
		fixture.runtime.receive_frame(
			ProtocolMessage::new(
				"p:1".into(),
				NOTIFICATIONS_PUSH_NOTIFICATION,
				MESSAGE_TYPE_REQUEST,
				request.encode(),
			)
			.encode(),
		),
	)
	.unwrap();
	let [response] = fixture.sink.take().try_into().unwrap();

	assert_eq!(
		Result::<(), CallError<HostPushNotificationError>>::decode_all(
			&mut response.payload.value.as_slice()
		),
		Ok(Err(CallError::Domain(HostPushNotificationError::V1(
			v01::HostPushNotificationError::Unknown { reason: "refused: muted".into() }
		)))),
	);
}

/// The account lengths in the subscription items `sink` received.
fn account_items(sink: &RecordingSink) -> Vec<usize> {
	sink.take()
		.into_iter()
		.map(|message| {
			let HostAccountAccountsSubscribeItem::V1(item) =
				Decode::decode(&mut message.payload.value.as_slice()).unwrap();
			item.accounts.len()
		})
		.collect()
}

#[test]
fn a_subscription_sends_the_current_list_then_each_pushed_change_until_it_stops() {
	let fixture = fixture(true, true);
	let start = HostAccountAccountsSubscribeRequest::V1.encode();
	block_on(
		fixture.runtime.receive_frame(
			ProtocolMessage::new(
				"p:1".into(),
				ACCOUNT_ACCOUNTS_SUBSCRIBE,
				MESSAGE_TYPE_REQUEST,
				start,
			)
			.encode(),
		),
	)
	.unwrap();
	let mut pool = LocalPool::new();
	for pump in fixture.pumps.lock().unwrap().drain(..) {
		pool.spawner().spawn_local(pump).unwrap();
	}

	pool.run_until_stalled();
	fixture.wallet.push_accounts(&["alice"]);
	fixture.wallet.push_accounts(&["alice", "bob"]);
	pool.run_until_stalled();
	let while_subscribed = account_items(&fixture.sink);
	block_on(
		fixture.runtime.receive_frame(
			ProtocolMessage::new(
				"p:1".into(),
				ACCOUNT_ACCOUNTS_SUBSCRIBE,
				MESSAGE_TYPE_STOP,
				Vec::new(),
			)
			.encode(),
		),
	)
	.unwrap();
	pool.run_until_stalled();
	fixture.wallet.push_accounts(&["alice", "bob", "carol"]);
	pool.run_until_stalled();

	assert_eq!((while_subscribed, account_items(&fixture.sink)), (vec![1, 2], vec![]));
}
