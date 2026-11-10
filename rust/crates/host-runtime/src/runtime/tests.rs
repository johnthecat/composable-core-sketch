//! The core driven the way a host drives it: product frames in, frames out.

use std::{
	collections::HashMap,
	sync::{
		Arc, Mutex,
		atomic::{AtomicUsize, Ordering},
	},
};

use futures::{channel::mpsc, executor::LocalPool, future::BoxFuture, task::LocalSpawnExt};
use host_extension::{
	Bytes, Bytes32, Bytes64,
	confirmation::{UserConfirmation, UserConfirmationError, UserConfirmationReview},
	notifier::{Notifier, NotifierError},
	product::{FrameSink, ProductContext},
	storage::{ProductStorage, ProductStorageError},
	wallet::{AccountsListener, ProductAccount, SignRequest, Signature, Wallet, WalletError},
};
use parity_scale_codec::{Decode, DecodeAll, Encode};
use protocol::{
	CallError,
	api::{
		account::{ACCOUNT_ACCOUNTS_SUBSCRIBE, HostAccountAccountsSubscribeItem},
		notifications::{
			HostPushNotificationError, HostPushNotificationRequest, NOTIFICATIONS_PUSH_NOTIFICATION,
		},
		signing::{HostSignRawError, HostSignRawRequest, HostSignRawResponse, SIGNING_SIGN_RAW},
	},
	dispatch::MethodIds,
	frame::{MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_STOP, ProtocolMessage},
};

use super::*;

const ALICE: Bytes32 = Bytes32([1; 32]);

/// A host whose user answers every confirmation with `confirms`, or never
/// answers when it is `None`.
struct TestHost {
	confirms: Option<bool>,
	reviews: Mutex<Vec<UserConfirmationReview>>,
	/// Confirmations dropped before the user answered.
	dropped: AtomicUsize,
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

/// Counts a confirmation dropped while it waited.
struct DropCounter<'a>(&'a AtomicUsize);

impl Drop for DropCounter<'_> {
	fn drop(&mut self) {
		self.0.fetch_add(1, Ordering::SeqCst);
	}
}

#[async_trait::async_trait]
impl UserConfirmation for TestHost {
	async fn confirm_user_action(
		&self,
		review: UserConfirmationReview,
	) -> Result<bool, UserConfirmationError> {
		self.reviews.lock().unwrap().push(review);
		match self.confirms {
			Some(confirms) => Ok(confirms),
			None => {
				let _waiting = DropCounter(&self.dropped);
				futures::future::pending().await
			},
		}
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
	fn emit_frame(&self, frame: Bytes) {
		self.0
			.lock()
			.unwrap()
			.push(ProtocolMessage::decode(&mut frame.0.as_slice()).unwrap());
	}
}

impl RecordingSink {
	fn take(&self) -> Vec<ProtocolMessage> {
		std::mem::take(&mut *self.0.lock().unwrap())
	}
}

struct Fixture {
	runtime: Arc<ProductRuntime>,
	sink: Arc<RecordingSink>,
	host: Arc<TestHost>,
	wallet: Arc<RecordingWallet>,
	/// The calls the core spawned, which [`Self::run`] runs on [`Self::pool`].
	spawned: mpsc::UnboundedReceiver<BoxFuture<'static, ()>>,
	pool: LocalPool,
}

impl Fixture {
	/// Send the product's frame, then run every call until it waits.
	fn send(&mut self, ids: MethodIds, message_type: u8, value: Vec<u8>) {
		let frame = ProtocolMessage { request_id: "p:1".into(), ids, message_type, value };
		self.runtime.receive_frame(frame.encode()).unwrap();
		self.run();
	}

	/// Run every call the core spawned until it waits.
	fn run(&mut self) {
		while let Ok(call) = self.spawned.try_recv() {
			self.pool.spawner().spawn_local(call).unwrap();
		}
		self.pool.run_until_stalled();
	}
}

fn fixture(confirms: Option<bool>, with_wallet: bool) -> Fixture {
	let host = Arc::new(TestHost {
		confirms,
		reviews: Mutex::default(),
		dropped: AtomicUsize::default(),
		storage: Mutex::default(),
	});
	let wallet = Arc::new(RecordingWallet::default());
	let (spawn, spawned) = mpsc::unbounded();
	let spawner: Spawner = Arc::new(move |call| spawn.unbounded_send(call).unwrap());
	let runtime = HostRuntime::with_spawner(
		HostExtensions {
			product_storage: host.clone(),
			user_confirmation: host.clone(),
			wallet: with_wallet.then(|| wallet.clone() as Arc<dyn Wallet>),
			notifier: Some(Arc::new(RefusingNotifier)),
		},
		spawner,
	);
	let sink = Arc::new(RecordingSink::default());
	let runtime =
		runtime.product_runtime(ProductContext { product_id: "demo".into() }, sink.clone());
	Fixture { runtime, sink, host, wallet, spawned, pool: LocalPool::new() }
}

fn sign_raw(
	fixture: &mut Fixture,
	public_key: Bytes32,
) -> Result<HostSignRawResponse, CallError<HostSignRawError>> {
	let request = HostSignRawRequest { public_key, payload: Bytes(b"hi".to_vec()) };
	fixture.send(SIGNING_SIGN_RAW, MESSAGE_TYPE_REQUEST, request.encode());
	let [response] = fixture.sink.take().try_into().unwrap();
	Decode::decode(&mut response.value.as_slice()).unwrap()
}

#[test]
fn a_confirmed_request_reaches_the_wallet_with_the_product_payload() {
	let mut fixture = fixture(Some(true), true);

	let response = sign_raw(&mut fixture, ALICE);

	assert_eq!(
		(response, fixture.wallet.signed.lock().unwrap().clone()),
		(
			Ok(HostSignRawResponse { signature: Bytes64([9; 64]) }),
			vec![SignRequest { public_key: ALICE, payload: Bytes(b"hi".to_vec()) }],
		),
	);
}

#[test]
fn a_refused_confirmation_fails_as_denied_before_the_wallet_signs() {
	let mut fixture = fixture(Some(false), true);

	let response = sign_raw(&mut fixture, ALICE);

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
	let mut fixture = fixture(Some(true), true);

	let response = sign_raw(&mut fixture, Bytes32([2; 32]));

	assert_eq!(
		(response, fixture.host.reviews.lock().unwrap().len()),
		(Err(CallError::Domain(HostSignRawError::UnknownAccount)), 0),
	);
}

#[test]
fn without_a_wallet_signing_is_unsupported() {
	let mut fixture = fixture(Some(true), false);

	assert_eq!(sign_raw(&mut fixture, ALICE), Err(CallError::Unsupported));
}

#[test]
fn a_notifier_refusal_reaches_the_product_as_its_domain_error() {
	let mut fixture = fixture(Some(true), true);
	let request = HostPushNotificationRequest { text: "hi".into() };

	fixture.send(NOTIFICATIONS_PUSH_NOTIFICATION, MESSAGE_TYPE_REQUEST, request.encode());
	let [response] = fixture.sink.take().try_into().unwrap();

	assert_eq!(
		Result::<(), CallError<HostPushNotificationError>>::decode_all(
			&mut response.value.as_slice()
		),
		Ok(Err(CallError::Domain(HostPushNotificationError::Unknown {
			reason: "refused: muted".into()
		}))),
	);
}

// A product that disconnects has its calls dropped, which drops what the core
// awaits: here a confirmation the user never answers.
#[test]
fn dropping_the_product_runtime_drops_a_call_still_in_flight() {
	let mut fixture = fixture(None, true);
	let request = HostSignRawRequest { public_key: ALICE, payload: Bytes(b"hi".to_vec()) };
	fixture.send(SIGNING_SIGN_RAW, MESSAGE_TYPE_REQUEST, request.encode());
	let while_connected = fixture.host.dropped.load(Ordering::SeqCst);

	let Fixture { runtime, sink, host, mut pool, .. } = fixture;
	drop(runtime);
	pool.run_until_stalled();

	assert_eq!(
		(while_connected, host.dropped.load(Ordering::SeqCst), sink.take().len()),
		(0, 1, 0)
	);
}

/// The account lengths in the subscription items `sink` received.
fn account_items(sink: &RecordingSink) -> Vec<usize> {
	sink.take()
		.into_iter()
		.map(|message| {
			let item: HostAccountAccountsSubscribeItem =
				Decode::decode(&mut message.value.as_slice()).unwrap();
			item.accounts.len()
		})
		.collect()
}

#[test]
fn a_subscription_sends_the_current_list_then_each_pushed_change_until_it_stops() {
	let mut fixture = fixture(Some(true), true);
	fixture.send(ACCOUNT_ACCOUNTS_SUBSCRIBE, MESSAGE_TYPE_REQUEST, Vec::new());

	fixture.wallet.push_accounts(&["alice"]);
	fixture.wallet.push_accounts(&["alice", "bob"]);
	fixture.run();
	let while_subscribed = account_items(&fixture.sink);
	fixture.send(ACCOUNT_ACCOUNTS_SUBSCRIBE, MESSAGE_TYPE_STOP, Vec::new());
	fixture.wallet.push_accounts(&["alice", "bob", "carol"]);
	fixture.run();

	assert_eq!((while_subscribed, account_items(&fixture.sink)), (vec![1, 2], vec![]));
}
