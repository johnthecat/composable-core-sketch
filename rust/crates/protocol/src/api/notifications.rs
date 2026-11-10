//! The [`Notifications`] service and its payloads.

use parity_scale_codec::{Decode, Encode};

use crate::{CallError, wire, wire_trait};

/// A notice a product asks the host to show the user.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub struct HostPushNotificationRequest {
	/// Text of the notice.
	pub text: String,
}

/// Why a notice was not shown.
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
pub enum HostPushNotificationError {
	/// Any failure.
	Unknown {
		/// Human-readable reason.
		reason: String,
	},
}

/// Notices shown to the user.
#[wire_trait(id = 8)]
#[async_trait::async_trait]
pub trait Notifications: Send + Sync {
	/// This call exists because it is the RFC's own example, the `Notifier`
	/// extension point, and it links a second provider crate,
	/// `host-provider-notifier-log`.
	///
	/// Show a notice to the user.
	#[wire(id = 0)]
	async fn push_notification(
		&self,
		request: HostPushNotificationRequest,
	) -> Result<(), CallError<HostPushNotificationError>>;
}
