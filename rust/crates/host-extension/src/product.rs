//! What a host hands the core for each product it frames: the product's
//! context, and the sink the core sends the product's frames to.

use host_macros::{extension_listener, extension_type};
use protocol::Bytes;

/// The product a connection serves.
#[extension_type]
pub struct ProductContext {
	/// Identifies the product, and scopes its storage.
	pub product_id: String,
}

/// Where a product runtime sends the frames it produces: the host's end of the
/// product's channel.
#[extension_listener]
pub trait FrameSink: Send + Sync {
	/// Deliver one encoded frame to the product. Returns at once.
	fn emit_frame(&self, frame: Bytes);
}
