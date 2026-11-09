//! The versioned envelope of protocol errors, which a host answers on the
//! reserved address for a frame no method serves.

use crate::v01;

sketch_macros::versioned_type! {
	pub enum ProtocolError { V1 => v01::ProtocolError }
}
