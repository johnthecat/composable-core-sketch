//! The [`Signing`] service.

use crate::{
	CallContext, CallError,
	versioned::signing::{HostSignRawError, HostSignRawRequest, HostSignRawResponse},
	wire, wire_trait,
};

/// Signing with the product's accounts.
#[wire_trait(id = 13)]
#[async_trait::async_trait]
pub trait Signing: Send + Sync {
	/// Sign raw bytes. The user confirms each call; a refusal fails it as
	/// `Denied`.
	#[wire(id = 0)]
	async fn sign_raw(
		&self,
		cx: &CallContext,
		request: HostSignRawRequest,
	) -> Result<HostSignRawResponse, CallError<HostSignRawError>>;
}
