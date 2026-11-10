//! The [`Account`] service and its payloads.

use parity_scale_codec::{Decode, Encode};

use crate::{Bytes32, CallError, Subscription, wire, wire_trait};

/// An account a product may sign with. The `Wallet` extension point carries
/// it too, so it is a `#[protocol_record]`.
#[host_macros::protocol_record]
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ProductAccount {
	/// 32-byte public key that identifies the account.
	pub public_key: Bytes32,
	/// Name the user gave the account.
	pub name: String,
}

/// The accounts a product may sign with.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostAccountGetAccountsResponse {
	/// Every account, in the wallet's order.
	pub accounts: Vec<ProductAccount>,
}

/// The account list after a change.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostAccountAccountsSubscribeItem {
	/// Every account, in the wallet's order.
	pub accounts: Vec<ProductAccount>,
}

/// A failure described only by its reason.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct GenericError {
	/// Human-readable reason.
	pub reason: String,
}

/// The accounts a product may sign with.
#[wire_trait(id = 2)]
#[async_trait::async_trait]
pub trait Account: Send + Sync {
	/// This call exists because a product needs a plain read beside the
	/// subscription, and because the host answers frames in order, the
	/// product's stop checks call it to learn that a stop was served.
	///
	/// List the accounts.
	#[wire(id = 0)]
	async fn get_accounts(&self)
	-> Result<HostAccountGetAccountsResponse, CallError<GenericError>>;

	/// This call exists because it is the protocol's one subscription: the
	/// core turns what the wallet pushes to its `AccountsListener` into items
	/// until the product stops it.
	///
	/// Emit the account list, then the list after each change.
	#[wire(id = 1)]
	async fn accounts_subscribe(
		&self,
	) -> Subscription<HostAccountAccountsSubscribeItem, CallError<GenericError>>;
}
