//! Versioned envelopes of [`Notifications`](crate::api::Notifications) methods.

use crate::v01;

sketch_macros::versioned_type! {
	pub enum HostPushNotificationRequest { V1 => v01::HostPushNotificationRequest }
	pub enum HostPushNotificationResponse { V1 }
	pub enum HostPushNotificationError { V1 => v01::HostPushNotificationError }
}
