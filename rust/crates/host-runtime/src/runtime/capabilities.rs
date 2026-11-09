//! The protocol services, served over the extension points.

use futures::{StreamExt, future, stream};
use host_extension::{ProductStorageError, SignRequest, UserConfirmationReview, WalletError};
use protocol::{
	Bytes, CallContext, CallError, Subscription,
	api::{Account, LocalStorage, Notifications, Signing},
	latest,
	versioned::{
		account::{
			HostAccountAccountsSubscribeError, HostAccountAccountsSubscribeItem,
			HostAccountAccountsSubscribeRequest, HostAccountGetAccountsError,
			HostAccountGetAccountsRequest, HostAccountGetAccountsResponse,
		},
		local_storage::{
			HostLocalStorageReadError, HostLocalStorageReadRequest, HostLocalStorageReadResponse,
			HostLocalStorageWriteError, HostLocalStorageWriteRequest,
			HostLocalStorageWriteResponse,
		},
		notifications::{
			HostPushNotificationError, HostPushNotificationRequest, HostPushNotificationResponse,
		},
		signing::{HostSignRawError, HostSignRawRequest, HostSignRawResponse},
	},
};

use super::{ProductRuntimeHost, versioning::version};

/// Wraps a message the core asks the wallet to sign, so that a signed message
/// can never pass for a signed transaction.
const MESSAGE_PREFIX: &[u8] = b"<Bytes>";
/// Closes [`MESSAGE_PREFIX`].
const MESSAGE_SUFFIX: &[u8] = b"</Bytes>";

/// A wallet failure, as the account methods report it.
fn account_error(error: WalletError) -> CallError<latest::GenericError> {
	CallError::Domain(error.to_string().into())
}

#[async_trait::async_trait]
impl Account for ProductRuntimeHost {
	async fn get_accounts(
		&self,
		_cx: &CallContext,
		request: HostAccountGetAccountsRequest,
	) -> Result<HostAccountGetAccountsResponse, CallError<HostAccountGetAccountsError>> {
		version(request)
			.exec(|()| async move {
				let accounts = self.wallet()?.accounts().await.map_err(account_error)?;
				Ok(latest::HostAccountGetAccountsResponse { accounts })
			})
			.await
	}

	async fn accounts_subscribe(
		&self,
		_cx: &CallContext,
		request: HostAccountAccountsSubscribeRequest,
	) -> Subscription<HostAccountAccountsSubscribeItem, CallError<HostAccountAccountsSubscribeError>>
	{
		version(request)
			.exec(|()| async move {
				let wallet = match self.wallet() {
					Ok(wallet) => wallet,
					Err(error) => return Subscription::interrupted(error),
				};
				// Follow the pushes before reading, so no change falls between.
				let changes = self.services.accounts.subscribe();
				let current = match wallet.accounts().await {
					Ok(current) => current,
					Err(error) => return Subscription::interrupted(account_error(error)),
				};
				let mut last = None;
				Subscription::new(stream::once(future::ready(current)).chain(changes).filter_map(
					move |accounts| {
						let changed = last.as_ref() != Some(&accounts);
						last = Some(accounts.clone());
						future::ready(
							changed.then_some(Ok(latest::HostAccountAccountsSubscribeItem {
								accounts,
							})),
						)
					},
				))
			})
			.await
	}
}

#[async_trait::async_trait]
impl Signing for ProductRuntimeHost {
	async fn sign_raw(
		&self,
		_cx: &CallContext,
		request: HostSignRawRequest,
	) -> Result<HostSignRawResponse, CallError<HostSignRawError>> {
		version(request)
			.exec(|latest::HostSignRawRequest { public_key, payload }| async move {
				let wallet = self.wallet()?;
				let accounts = wallet.accounts().await.map_err(sign_error)?;
				if !accounts.iter().any(|account| account.public_key == public_key) {
					return Err(CallError::Domain(latest::HostSignRawError::UnknownAccount));
				}
				let review = UserConfirmationReview::SignRaw {
					product_id: self.product.product_id.clone(),
					public_key,
					payload: Bytes(payload.clone()),
				};
				let confirmed =
					self.services
						.user_confirmation
						.confirm_user_action(review)
						.await
						.map_err(|error| CallError::HostFailure { reason: error.to_string() })?;
				if !confirmed {
					return Err(CallError::Denied);
				}
				let payload = Bytes([MESSAGE_PREFIX, &payload, MESSAGE_SUFFIX].concat());
				let signature =
					wallet.sign(SignRequest { public_key, payload }).await.map_err(sign_error)?;
				Ok(latest::HostSignRawResponse { signature: signature.bytes })
			})
			.await
	}
}

/// A wallet failure, as `sign_raw` reports it.
fn sign_error(error: WalletError) -> CallError<latest::HostSignRawError> {
	CallError::Domain(match error {
		WalletError::Rejected => latest::HostSignRawError::Rejected,
		WalletError::UnknownAccount => latest::HostSignRawError::UnknownAccount,
		WalletError::Unknown { reason } => latest::HostSignRawError::Unknown { reason },
	})
}

#[async_trait::async_trait]
impl LocalStorage for ProductRuntimeHost {
	async fn read(
		&self,
		_cx: &CallContext,
		request: HostLocalStorageReadRequest,
	) -> Result<HostLocalStorageReadResponse, CallError<HostLocalStorageReadError>> {
		version(request)
			.exec(|latest::HostLocalStorageReadRequest { key }| async move {
				let value = self
					.services
					.product_storage
					.read(self.storage_key(&key))
					.await
					.map_err(storage_error)?;
				Ok(latest::HostLocalStorageReadResponse { value: value.map(|bytes| bytes.0) })
			})
			.await
	}

	async fn write(
		&self,
		_cx: &CallContext,
		request: HostLocalStorageWriteRequest,
	) -> Result<HostLocalStorageWriteResponse, CallError<HostLocalStorageWriteError>> {
		version(request)
			.exec(|latest::HostLocalStorageWriteRequest { key, value }| async move {
				self.services
					.product_storage
					.write(self.storage_key(&key), Bytes(value))
					.await
					.map_err(storage_error)
			})
			.await
	}
}

/// A platform storage failure, as the storage methods report it.
fn storage_error(error: ProductStorageError) -> CallError<latest::HostLocalStorageError> {
	CallError::Domain(match error {
		ProductStorageError::Full => latest::HostLocalStorageError::Full,
		ProductStorageError::Unknown { reason } => {
			latest::HostLocalStorageError::Unknown { reason }
		},
	})
}

#[async_trait::async_trait]
impl Notifications for ProductRuntimeHost {
	async fn push_notification(
		&self,
		_cx: &CallContext,
		request: HostPushNotificationRequest,
	) -> Result<HostPushNotificationResponse, CallError<HostPushNotificationError>> {
		version(request)
			.exec(|latest::HostPushNotificationRequest { text }| async move {
				self.notifier()?.notify(text).await.map_err(|error| {
					CallError::Domain(latest::HostPushNotificationError::Unknown {
						reason: error.to_string(),
					})
				})
			})
			.await
	}
}
