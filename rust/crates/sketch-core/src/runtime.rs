use std::sync::Arc;

use futures::{StreamExt, future, stream::BoxStream};

use crate::host::extension::{Account, SignRequest, Signature, Wallet, WalletError};

/// Wraps a message the core asks the wallet to sign, so that a signed message can
/// never be mistaken for a signed transaction.
const MESSAGE_PREFIX: &[u8] = b"<Bytes>";
/// Closes [`MESSAGE_PREFIX`].
const MESSAGE_SUFFIX: &[u8] = b"</Bytes>";

/// Serves products' account and signing calls over the host's wallet.
///
/// The runtime holds the wallet as `Arc<dyn Wallet>`, so it stays non-generic
/// whichever provider, or host code around one, the host passes in.
pub struct HostRuntime {
	wallet: Arc<dyn Wallet>,
}

impl HostRuntime {
	/// A runtime that signs with `wallet`.
	pub fn new(wallet: Arc<dyn Wallet>) -> Self {
		Self { wallet }
	}

	/// The accounts a product may sign with.
	pub async fn accounts(&self) -> Result<Vec<Account>, WalletError> {
		self.wallet.accounts().await
	}

	/// Signs `message` with the account `public_key`.
	///
	/// The core first checks that the wallet holds the account, and fails with
	/// [`WalletError::UnknownAccount`] without asking for a signature when it does
	/// not. It then asks the wallet to sign the message wrapped in
	/// `<Bytes>…</Bytes>`.
	pub async fn sign_message(
		&self,
		public_key: [u8; 32],
		message: &str,
	) -> Result<Signature, WalletError> {
		let accounts = self.wallet.accounts().await?;
		if !accounts.iter().any(|account| account.public_key == public_key) {
			return Err(WalletError::UnknownAccount);
		}
		let payload = [MESSAGE_PREFIX, message.as_bytes(), MESSAGE_SUFFIX].concat();
		self.wallet.sign(SignRequest { public_key, payload }).await
	}

	/// The account list, first as it is and then after each change. An update
	/// that repeats the list it follows is dropped.
	pub fn subscribe_accounts(&self) -> BoxStream<'static, Result<Vec<Account>, WalletError>> {
		let mut last: Option<Vec<Account>> = None;
		self.wallet
			.subscribe_accounts()
			.filter(move |item| {
				let repeated =
					matches!((item, &last), (Ok(accounts), Some(previous)) if accounts == previous);
				if let Ok(accounts) = item {
					last = Some(accounts.clone());
				}
				future::ready(!repeated)
			})
			.boxed()
	}
}

#[cfg(test)]
mod tests;
