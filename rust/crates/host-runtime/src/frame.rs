//! Wire frames.
//!
//! Every message between a product and the host is one frame:
//!
//! ```text
//!   [requestId: SCALE str][trait: u8][method: u8][message_type: u8][payload bytes...]
//! ```
//!
//! The `(trait, method)` pair addresses a method through the method tables
//! `#[wire_trait]` writes. `message_type` names which leg of the method's
//! exchange the frame carries, so the dispatcher reads it without decoding the
//! payload. The payload is that leg's own versioned envelope, SCALE-encoded and
//! inlined without a length prefix.

use parity_scale_codec::{Decode, Encode, Error as CodecError, Input, Output};
use protocol::{CallError, dispatch::MethodIds};

/// Reserved trait id of method-independent protocol errors. No protocol trait
/// may take it.
pub const PROTOCOL_ERROR_TRAIT_ID: u8 = 255;
/// Reserved method id of method-independent protocol errors.
pub const PROTOCOL_ERROR_METHOD_ID: u8 = 255;

/// Product to host: a request, or a subscription's start.
pub const MESSAGE_TYPE_REQUEST: u8 = 0;
/// Host to product: a response, or one subscription item.
pub const MESSAGE_TYPE_RESPONSE: u8 = 1;
/// Host to product: a subscription's end, `Result<(), Interrupt>`.
pub const MESSAGE_TYPE_INTERRUPT: u8 = 2;
/// Product to host: stop a subscription. No payload.
pub const MESSAGE_TYPE_STOP: u8 = 3;

/// One wire frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMessage {
	/// Correlates a request with its response, and a subscription's frames.
	pub request_id: String,
	/// Address, leg and payload.
	pub payload: Payload,
}

/// What a frame carries after its request id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
	/// Trait id, the first byte of the wire pair.
	pub trait_id: u8,
	/// Method id within the trait.
	pub method_id: u8,
	/// Which leg of the exchange this is; see the `MESSAGE_TYPE_*` constants.
	pub message_type: u8,
	/// The leg's SCALE-encoded versioned envelope.
	pub value: Vec<u8>,
}

impl ProtocolMessage {
	/// The frame `request_id` sends on the leg `message_type` of the method
	/// `ids`, carrying `value`.
	pub fn new(request_id: String, ids: MethodIds, message_type: u8, value: Vec<u8>) -> Self {
		Self {
			request_id,
			payload: Payload {
				trait_id: ids.trait_id,
				method_id: ids.method_id,
				message_type,
				value,
			},
		}
	}
}

/// `Ok(())`, a subscription's natural end. One byte whatever the method's
/// interrupt type.
pub fn encode_clean_interrupt() -> Vec<u8> {
	Ok::<(), CallError<()>>(()).encode()
}

impl Encode for ProtocolMessage {
	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		self.request_id.encode_to(dest);
		self.payload.trait_id.encode_to(dest);
		self.payload.method_id.encode_to(dest);
		self.payload.message_type.encode_to(dest);
		dest.write(&self.payload.value);
	}
}

// The payload has no length prefix, so decoding reads to the end of a
// slice-shaped input.
impl Decode for ProtocolMessage {
	fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
		let request_id = String::decode(input)?;
		let trait_id = u8::decode(input)?;
		let method_id = u8::decode(input)?;
		let message_type = u8::decode(input)?;
		let remaining = input
			.remaining_len()?
			.ok_or_else(|| CodecError::from("a frame must be decoded from a slice"))?;
		let mut value = vec![0; remaining];
		input.read(&mut value)?;
		Ok(Self { request_id, payload: Payload { trait_id, method_id, message_type, value } })
	}
}

#[cfg(test)]
mod tests;
