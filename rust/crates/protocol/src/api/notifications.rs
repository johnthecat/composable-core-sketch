//! The [`Notifications`] service.

use crate::{
	CallContext, CallError,
	versioned::notifications::{
		HostPushNotificationError, HostPushNotificationRequest, HostPushNotificationResponse,
	},
	wire, wire_trait,
};

/// Notices shown to the user.
#[wire_trait(id = 8)]
#[async_trait::async_trait]
pub trait Notifications: Send + Sync {
	/// Show a notice to the user.
	#[wire(id = 0)]
	async fn push_notification(
		&self,
		cx: &CallContext,
		request: HostPushNotificationRequest,
	) -> Result<HostPushNotificationResponse, CallError<HostPushNotificationError>>;
}
