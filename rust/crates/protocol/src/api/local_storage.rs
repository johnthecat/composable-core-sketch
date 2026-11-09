//! The [`LocalStorage`] service.

use crate::{
	CallContext, CallError,
	versioned::local_storage::{
		HostLocalStorageReadError, HostLocalStorageReadRequest, HostLocalStorageReadResponse,
		HostLocalStorageWriteError, HostLocalStorageWriteRequest, HostLocalStorageWriteResponse,
	},
	wire, wire_trait,
};

/// Key/value storage scoped to the calling product.
#[wire_trait(id = 7)]
#[async_trait::async_trait]
pub trait LocalStorage: Send + Sync {
	/// Read a value by key.
	#[wire(id = 0)]
	async fn read(
		&self,
		cx: &CallContext,
		request: HostLocalStorageReadRequest,
	) -> Result<HostLocalStorageReadResponse, CallError<HostLocalStorageReadError>>;

	/// Write a value to a key.
	#[wire(id = 1)]
	async fn write(
		&self,
		cx: &CallContext,
		request: HostLocalStorageWriteRequest,
	) -> Result<HostLocalStorageWriteResponse, CallError<HostLocalStorageWriteError>>;
}
