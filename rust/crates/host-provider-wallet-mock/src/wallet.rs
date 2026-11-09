use core::time::Duration;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use futures_timer::Delay;
use host_extension::{
	AccountsListener, Bytes32, Bytes64, ProductAccount, SignRequest, Signature, Wallet, WalletError,
};
use sha2::{Digest, Sha256, Sha512};

/// How the simulated phone answers a sign request once its delay has passed.
/// Swift and Kotlin get it through UniFFI, and TS as the enum `PhoneDecision`.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Enum))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub enum PhoneDecision {
	/// The user approves, and the wallet signs.
	Approve,
	/// The user declines: [`WalletError::Rejected`].
	Decline,
	/// The phone never answers: an undeclared [`WalletError::Unknown`].
	Unreachable,
}

/// In-memory wallet standing in for the wallet on a paired phone.
///
/// Keys and signatures are fakes derived with SHA-2: they identify an account
/// and bind a payload to it for the demo, and are not cryptographic keys or
/// signatures.
pub struct MockWallet {
	state: Arc<Mutex<State>>,
}

struct State {
	accounts: Vec<ProductAccount>,
	decision: PhoneDecision,
	sign_delay: Duration,
	/// Where the list is pushed when it changes: the last listener given.
	listener: Option<Arc<dyn AccountsListener>>,
}

impl MockWallet {
	/// A wallet with no accounts that approves each sign request after
	/// `sign_delay`.
	pub fn new(sign_delay: Duration) -> Self {
		Self {
			state: Arc::new(Mutex::new(State {
				accounts: Vec::new(),
				decision: PhoneDecision::Approve,
				sign_delay,
				listener: None,
			})),
		}
	}

	/// Adds an account named `name` and pushes the new list to the listener.
	pub fn add_account(&self, name: &str) -> ProductAccount {
		let (account, accounts, listener) = {
			let mut state = lock(&self.state);
			let index = u32::try_from(state.accounts.len()).unwrap_or(u32::MAX);
			let account =
				ProductAccount { public_key: mock_public_key(name, index), name: name.into() };
			state.accounts.push(account.clone());
			(account, state.accounts.clone(), state.listener.clone())
		};
		// Outside the lock: the listener may be host code that calls back in.
		if let Some(listener) = listener {
			listener.accounts_changed(accounts);
		}
		account
	}

	/// Sets how the phone answers sign requests from now on, including those
	/// still waiting for their delay.
	pub fn set_decision(&self, decision: PhoneDecision) {
		lock(&self.state).decision = decision;
	}

	/// Sets how long the phone takes to answer a sign request made from now on.
	pub fn set_sign_delay(&self, sign_delay: Duration) {
		lock(&self.state).sign_delay = sign_delay;
	}
}

#[async_trait::async_trait]
impl Wallet for MockWallet {
	async fn accounts(&self) -> Result<Vec<ProductAccount>, WalletError> {
		Ok(lock(&self.state).accounts.clone())
	}

	async fn sign(&self, request: SignRequest) -> Result<Signature, WalletError> {
		let sign_delay = lock(&self.state).sign_delay;
		Delay::new(sign_delay).await;

		let state = lock(&self.state);
		match state.decision {
			PhoneDecision::Approve => {},
			PhoneDecision::Decline => return Err(WalletError::Rejected),
			PhoneDecision::Unreachable => {
				return Err(WalletError::from("the phone did not answer".to_string()));
			},
		}
		if !state.accounts.iter().any(|account| account.public_key == request.public_key) {
			return Err(WalletError::UnknownAccount);
		}
		Ok(mock_signature(&request))
	}

	fn set_accounts_listener(&self, listener: Arc<dyn AccountsListener>) {
		lock(&self.state).listener = Some(listener);
	}
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
	state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Fake public key: a hash of the account's name and position.
fn mock_public_key(name: &str, index: u32) -> Bytes32 {
	Bytes32(
		Sha256::new()
			.chain_update(b"sketch mock account")
			.chain_update(name)
			.chain_update(index.to_le_bytes())
			.finalize()
			.into(),
	)
}

/// Fake signature: a hash of the public key and the payload.
fn mock_signature(request: &SignRequest) -> Signature {
	let bytes = Sha512::new()
		.chain_update(b"sketch mock signature")
		.chain_update(request.public_key.0)
		.chain_update(&request.payload.0)
		.finalize()
		.into();
	Signature { bytes: Bytes64(bytes) }
}

#[cfg(test)]
mod tests;
