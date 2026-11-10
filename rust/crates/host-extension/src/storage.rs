//! The `ProductStorage` extension point: where products' values persist.

use host_macros::{extension_error, extension_point};
use protocol::Bytes;

/// Why a value was not read or written.
#[extension_error]
pub enum ProductStorageError {
	/// The storage quota is exhausted.
	#[error("storage quota exhausted")]
	Full,
}

/// Key/value persistence for products. Keys arrive namespaced by product.
#[extension_point]
pub trait ProductStorage: Send + Sync {
	/// Read the value at `key`.
	async fn read(&self, key: String) -> Result<Option<Bytes>, ProductStorageError>;

	/// Write `value` at `key`.
	async fn write(&self, key: String, value: Bytes) -> Result<(), ProductStorageError>;
}
