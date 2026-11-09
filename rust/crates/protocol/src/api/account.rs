//! The [`Account`] service.

use crate::{
	CallContext, CallError, Subscription,
	versioned::account::{
		HostAccountAccountsSubscribeError, HostAccountAccountsSubscribeItem,
		HostAccountAccountsSubscribeRequest, HostAccountGetAccountsError,
		HostAccountGetAccountsRequest, HostAccountGetAccountsResponse,
	},
	wire, wire_trait,
};

/// The accounts a product may sign with.
#[wire_trait(id = 2)]
#[async_trait::async_trait]
pub trait Account: Send + Sync {
	/// List the accounts.
	#[wire(id = 0)]
	async fn get_accounts(
		&self,
		cx: &CallContext,
		request: HostAccountGetAccountsRequest,
	) -> Result<HostAccountGetAccountsResponse, CallError<HostAccountGetAccountsError>>;

	/// Emit the account list, then the list after each change.
	#[wire(id = 1)]
	async fn accounts_subscribe(
		&self,
		cx: &CallContext,
		request: HostAccountAccountsSubscribeRequest,
	) -> Subscription<HostAccountAccountsSubscribeItem, CallError<HostAccountAccountsSubscribeError>>;
}
