use core::time::Duration;
use std::sync::{Mutex, MutexGuard, PoisonError};

use futures::{StreamExt, channel::mpsc};
use futures_timer::Delay;
use sha2::{Digest, Sha256, Sha512};
use sketch_wallet_extension::{Account, BoxStream, SignRequest, Signature, Wallet, WalletError};

/// How the simulated phone answers a sign request once its delay has passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
/// Keys and signatures are fakes derived with SHA-2: they identify an account and
/// bind a payload to it for the demo, and are not cryptographic keys or
/// signatures.
pub struct MockWallet {
	state: Mutex<State>,
}

struct State {
	accounts: Vec<Account>,
	decision: PhoneDecision,
	sign_delay: Duration,
	subscribers: Vec<mpsc::UnboundedSender<Result<Vec<Account>, WalletError>>>,
}

impl MockWallet {
	/// A wallet with no accounts that approves each sign request after
	/// `sign_delay`.
	pub fn new(sign_delay: Duration) -> Self {
		Self {
			state: Mutex::new(State {
				accounts: Vec::new(),
				decision: PhoneDecision::Approve,
				sign_delay,
				subscribers: Vec::new(),
			}),
		}
	}

	/// Adds an account named `name` and sends the new list to every subscriber.
	pub fn add_account(&self, name: &str) -> Account {
		let mut state = self.state();
		let index = u32::try_from(state.accounts.len()).unwrap_or(u32::MAX);
		let account = Account { public_key: mock_public_key(name, index), name: name.into() };
		state.accounts.push(account.clone());
		let accounts = state.accounts.clone();
		state
			.subscribers
			.retain(|subscriber| subscriber.unbounded_send(Ok(accounts.clone())).is_ok());
		account
	}

	/// Sets how the phone answers sign requests from now on, including those
	/// still waiting for their delay.
	pub fn set_decision(&self, decision: PhoneDecision) {
		self.state().decision = decision;
	}

	/// Sets how long the phone takes to answer a sign request made from now on.
	pub fn set_sign_delay(&self, sign_delay: Duration) {
		self.state().sign_delay = sign_delay;
	}

	fn state(&self) -> MutexGuard<'_, State> {
		self.state.lock().unwrap_or_else(PoisonError::into_inner)
	}
}

#[async_trait::async_trait]
impl Wallet for MockWallet {
	async fn accounts(&self) -> Result<Vec<Account>, WalletError> {
		Ok(self.state().accounts.clone())
	}

	async fn sign(&self, request: SignRequest) -> Result<Signature, WalletError> {
		let sign_delay = self.state().sign_delay;
		Delay::new(sign_delay).await;

		let state = self.state();
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

	fn subscribe_accounts(&self) -> BoxStream<'static, Result<Vec<Account>, WalletError>> {
		let (sender, receiver) = mpsc::unbounded();
		let mut state = self.state();
		let _ = sender.unbounded_send(Ok(state.accounts.clone()));
		state.subscribers.push(sender);
		receiver.boxed()
	}
}

/// Fake public key: a hash of the account's name and position.
fn mock_public_key(name: &str, index: u32) -> [u8; 32] {
	Sha256::new()
		.chain_update(b"sketch mock account")
		.chain_update(name)
		.chain_update(index.to_le_bytes())
		.finalize()
		.into()
}

/// Fake signature: a hash of the public key and the payload.
fn mock_signature(request: &SignRequest) -> Signature {
	let bytes = Sha512::new()
		.chain_update(b"sketch mock signature")
		.chain_update(request.public_key)
		.chain_update(&request.payload)
		.finalize()
		.into();
	Signature { bytes }
}

#[cfg(test)]
mod tests;
