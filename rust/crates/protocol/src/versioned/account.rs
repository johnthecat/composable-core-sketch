//! Versioned envelopes of [`Account`](crate::api::Account) methods.

use crate::v01;

sketch_macros::versioned_type! {
	pub enum HostAccountGetAccountsRequest { V1 }
	pub enum HostAccountGetAccountsResponse { V1 => v01::HostAccountGetAccountsResponse }
	pub enum HostAccountGetAccountsError { V1 => v01::GenericError }
	pub enum HostAccountAccountsSubscribeRequest { V1 }
	pub enum HostAccountAccountsSubscribeItem { V1 => v01::HostAccountAccountsSubscribeItem }
	pub enum HostAccountAccountsSubscribeError { V1 => v01::GenericError }
}
