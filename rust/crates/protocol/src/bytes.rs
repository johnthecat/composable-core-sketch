//! Byte values that say how they cross each boundary themselves.
//!
//! SCALE encodes each as its contents. UniFFI lowers each as bytes (`Data` in
//! Swift, `ByteArray` in Kotlin). On the web, serde serializes each with
//! `serialize_bytes`, which `serde-wasm-bindgen` turns into a `Uint8Array`.

use parity_scale_codec::{Decode, Encode};

/// A 32-byte value, such as a public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub struct Bytes32(pub [u8; 32]);

/// A 64-byte value, such as a signature.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Bytes64(pub [u8; 64]);

/// A byte string of any length.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Bytes(pub Vec<u8>);

#[cfg(not(target_arch = "wasm32"))]
uniffi::custom_type!(Bytes32, Vec<u8>, {
	lower: |bytes| bytes.0.to_vec(),
	try_lift: |bytes| Ok(Bytes32(bytes.as_slice().try_into()?)),
});

#[cfg(not(target_arch = "wasm32"))]
uniffi::custom_type!(Bytes64, Vec<u8>, {
	lower: |bytes| bytes.0.to_vec(),
	try_lift: |bytes| Ok(Bytes64(bytes.as_slice().try_into()?)),
});

#[cfg(not(target_arch = "wasm32"))]
uniffi::custom_type!(Bytes, Vec<u8>, {
	lower: |bytes| bytes.0,
	try_lift: |bytes| Ok(Bytes(bytes)),
});

#[cfg(target_arch = "wasm32")]
mod web;
