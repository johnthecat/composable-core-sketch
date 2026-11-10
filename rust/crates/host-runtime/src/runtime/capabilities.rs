//! The protocol services, served over the extension points.

use futures::{StreamExt, future, stream};
use host_extension::{
	confirmation::UserConfirmationReview,
	storage::ProductStorageError,
	wallet::{SignRequest, WalletError},
};
use protocol::{
	CallError, Subscription,
	api::{
		Account, LocalStorage, Notifications, Signing,
		account::{GenericError, HostAccountAccountsSubscribeItem, HostAccountGetAccountsResponse},
		local_storage::{
			HostLocalStorageError, HostLocalStorageReadRequest, HostLocalStorageReadResponse,
			HostLocalStorageWriteRequest,
		},
		notifications::{HostPushNotificationError, HostPushNotificationRequest},
		signing::{HostSignRawError, HostSignRawRequest, HostSignRawResponse},
	},
};

use super::ProductRuntimeHost;

/// A wallet failure, as the account methods report it.
fn account_error(error: WalletError) -> CallError<GenericError> {
	CallError::Domain(GenericError { reason: error.to_string() })
}

#[async_trait::async_trait]
impl Account for ProductRuntimeHost {
	async fn get_accounts(
		&self,
	) -> Result<HostAccountGetAccountsResponse, CallError<GenericError>> {
		let accounts = self.wallet()?.accounts().await.map_err(account_error)?;
		Ok(HostAccountGetAccountsResponse { accounts })
	}

	async fn accounts_subscribe(
		&self,
	) -> Subscription<HostAccountAccountsSubscribeItem, CallError<GenericError>> {
		let wallet = match self.wallet() {
			Ok(wallet) => wallet,
			Err(error) => return Subscription::interrupted(error),
		};
		// Follow the pushes before reading, so no change falls between.
		let changes = self.runtime.accounts.subscribe();
		let current = match wallet.accounts().await {
			Ok(current) => current,
			Err(error) => return Subscription::interrupted(account_error(error)),
		};
		let mut last = None;
		Subscription::new(stream::once(future::ready(current)).chain(changes).filter_map(
			move |accounts| {
				if last.as_ref() == Some(&accounts) {
					return future::ready(None);
				}
				last = Some(accounts.clone());
				future::ready(Some(Ok(HostAccountAccountsSubscribeItem { accounts })))
			},
		))
	}
}

#[async_trait::async_trait]
impl Signing for ProductRuntimeHost {
	async fn sign_raw(
		&self,
		HostSignRawRequest { public_key, payload }: HostSignRawRequest,
	) -> Result<HostSignRawResponse, CallError<HostSignRawError>> {
		let wallet = self.wallet()?;
		let accounts = wallet.accounts().await.map_err(sign_error)?;
		if !accounts.iter().any(|account| account.public_key == public_key) {
			return Err(CallError::Domain(HostSignRawError::UnknownAccount));
		}
		let review = UserConfirmationReview::SignRaw {
			product_id: self.product.product_id.clone(),
			public_key,
			payload: payload.clone(),
		};
		let confirmed = self
			.runtime
			.extensions
			.user_confirmation
			.confirm_user_action(review)
			.await
			.map_err(|error| CallError::HostFailure { reason: error.to_string() })?;
		if !confirmed {
			return Err(CallError::Denied);
		}
		let signature =
			wallet.sign(SignRequest { public_key, payload }).await.map_err(sign_error)?;
		Ok(HostSignRawResponse { signature: signature.bytes })
	}
}

/// A wallet failure, as `sign_raw` reports it.
fn sign_error(error: WalletError) -> CallError<HostSignRawError> {
	CallError::Domain(match error {
		WalletError::Rejected => HostSignRawError::Rejected,
		WalletError::UnknownAccount => HostSignRawError::UnknownAccount,
		WalletError::Unknown { reason } => HostSignRawError::Unknown { reason },
	})
}

#[async_trait::async_trait]
impl LocalStorage for ProductRuntimeHost {
	async fn read(
		&self,
		HostLocalStorageReadRequest { key }: HostLocalStorageReadRequest,
	) -> Result<HostLocalStorageReadResponse, CallError<HostLocalStorageError>> {
		let value = self
			.runtime
			.extensions
			.product_storage
			.read(self.storage_key(&key))
			.await
			.map_err(storage_error)?;
		Ok(HostLocalStorageReadResponse { value })
	}

	async fn write(
		&self,
		HostLocalStorageWriteRequest { key, value }: HostLocalStorageWriteRequest,
	) -> Result<(), CallError<HostLocalStorageError>> {
		self.runtime
			.extensions
			.product_storage
			.write(self.storage_key(&key), value)
			.await
			.map_err(storage_error)
	}
}

/// A platform storage failure, as the storage methods report it.
fn storage_error(error: ProductStorageError) -> CallError<HostLocalStorageError> {
	CallError::Domain(match error {
		ProductStorageError::Full => HostLocalStorageError::Full,
		ProductStorageError::Unknown { reason } => HostLocalStorageError::Unknown { reason },
	})
}

#[async_trait::async_trait]
impl Notifications for ProductRuntimeHost {
	async fn push_notification(
		&self,
		HostPushNotificationRequest { text }: HostPushNotificationRequest,
	) -> Result<(), CallError<HostPushNotificationError>> {
		self.notifier()?.notify(text).await.map_err(|error| {
			CallError::Domain(HostPushNotificationError::Unknown { reason: error.to_string() })
		})
	}
}
