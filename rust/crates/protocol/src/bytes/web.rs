//! The serde impls: every byte value serializes with `serialize_bytes`, and
//! deserializes from bytes, or a sequence of them, of the right length.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use serde_bytes::ByteBuf;

use super::{Bytes, Bytes32, Bytes64};

impl Serialize for Bytes32 {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_bytes(&self.0)
	}
}

impl<'de> Deserialize<'de> for Bytes32 {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		fixed(deserializer).map(Self)
	}
}

impl Serialize for Bytes64 {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_bytes(&self.0)
	}
}

impl<'de> Deserialize<'de> for Bytes64 {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		fixed(deserializer).map(Self)
	}
}

impl Serialize for Bytes {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_bytes(&self.0)
	}
}

impl<'de> Deserialize<'de> for Bytes {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		ByteBuf::deserialize(deserializer).map(|bytes| Self(bytes.into_vec()))
	}
}

fn fixed<'de, D: Deserializer<'de>, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error> {
	let bytes = ByteBuf::deserialize(deserializer)?;
	<[u8; N]>::try_from(bytes.as_slice())
		.map_err(|_| D::Error::invalid_length(bytes.len(), &format!("{N} bytes").as_str()))
}
