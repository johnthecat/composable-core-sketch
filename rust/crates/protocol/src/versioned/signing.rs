//! Versioned envelopes of [`Signing`](crate::api::Signing) methods.

use crate::v01;

sketch_macros::versioned_type! {
	pub enum HostSignRawRequest { V1 => v01::HostSignRawRequest }
	pub enum HostSignRawResponse { V1 => v01::HostSignRawResponse }
	pub enum HostSignRawError { V1 => v01::HostSignRawError }
}
