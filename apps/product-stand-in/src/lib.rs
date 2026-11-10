//! A stand-in product for every demo host: it encodes a product's frames with
//! the protocol crate's own types and reads frames back as text. Each function
//! is exported through UniFFI natively and through wasm-bindgen on the web.
//!
//! `README.md` describes its role in the demo.

use std::fmt::Debug;

use parity_scale_codec::{Decode, DecodeAll, Encode};
use protocol::{
	Bytes, Bytes32, CallError,
	api::{
		account::{
			ACCOUNT_ACCOUNTS_SUBSCRIBE, ACCOUNT_GET_ACCOUNTS, GenericError,
			HostAccountAccountsSubscribeItem, HostAccountGetAccountsResponse,
		},
		local_storage::{
			HostLocalStorageError, HostLocalStorageReadRequest, HostLocalStorageReadResponse,
			HostLocalStorageWriteRequest, LOCAL_STORAGE_READ, LOCAL_STORAGE_WRITE,
		},
		notifications::{
			HostPushNotificationError, HostPushNotificationRequest, NOTIFICATIONS_PUSH_NOTIFICATION,
		},
		signing::{HostSignRawError, HostSignRawRequest, HostSignRawResponse, SIGNING_SIGN_RAW},
	},
	dispatch::MethodIds,
	frame::{
		MESSAGE_TYPE_INTERRUPT, MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_RESPONSE, MESSAGE_TYPE_STOP,
		ProtocolMessage,
	},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
uniffi::setup_scaffolding!();

/// Which leg of a method's exchange a frame from the host carries.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Enum))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen, derive(Clone))]
pub enum FrameLeg {
	/// A request's response.
	Response,
	/// One subscription item.
	Item,
	/// A subscription's end.
	End,
	/// Not a frame, or a leg the product does not expect.
	Unknown,
}

/// A frame the host sent to the product, as text.
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Record))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
pub struct ReceivedFrame {
	/// The request it answers.
	#[cfg_attr(target_arch = "wasm32", wasm_bindgen(readonly, js_name = requestId))]
	pub request_id: String,
	/// Which leg it carries.
	#[cfg_attr(target_arch = "wasm32", wasm_bindgen(readonly))]
	pub leg: FrameLeg,
	/// The decoded payload.
	#[cfg_attr(target_arch = "wasm32", wasm_bindgen(readonly))]
	pub outcome: String,
}

/// Why the stand-in could not build a frame.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(not(target_arch = "wasm32"), derive(uniffi::Error))]
pub enum FrameError {
	/// The public key given is not 32 bytes.
	#[error("a public key is 32 bytes")]
	PublicKeyLength,
}

#[cfg(target_arch = "wasm32")]
impl From<FrameError> for JsValue {
	fn from(error: FrameError) -> Self {
		JsError::new(&error.to_string()).into()
	}
}

fn frame(request_id: String, ids: MethodIds, message_type: u8, value: Vec<u8>) -> Vec<u8> {
	ProtocolMessage { request_id, ids, message_type, value }.encode()
}

/// `account.getAccounts()`.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = getAccountsFrame))]
pub fn get_accounts_frame(request_id: String) -> Vec<u8> {
	frame(request_id, ACCOUNT_GET_ACCOUNTS, MESSAGE_TYPE_REQUEST, Vec::new())
}

/// `account.accountsSubscribe()`.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = accountsSubscribeFrame))]
pub fn accounts_subscribe_frame(request_id: String) -> Vec<u8> {
	frame(request_id, ACCOUNT_ACCOUNTS_SUBSCRIBE, MESSAGE_TYPE_REQUEST, Vec::new())
}

/// Stops the account subscription started with `request_id`.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = accountsUnsubscribeFrame))]
pub fn accounts_unsubscribe_frame(request_id: String) -> Vec<u8> {
	frame(request_id, ACCOUNT_ACCOUNTS_SUBSCRIBE, MESSAGE_TYPE_STOP, Vec::new())
}

/// `signing.signRaw({ publicKey, payload })`. Fails when `public_key` is not
/// 32 bytes.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = signRawFrame))]
pub fn sign_raw_frame(
	request_id: String,
	public_key: Vec<u8>,
	payload: Vec<u8>,
) -> Result<Vec<u8>, FrameError> {
	let public_key = Bytes32(public_key.try_into().map_err(|_| FrameError::PublicKeyLength)?);
	let request = HostSignRawRequest { public_key, payload: Bytes(payload) };
	Ok(frame(request_id, SIGNING_SIGN_RAW, MESSAGE_TYPE_REQUEST, request.encode()))
}

/// `localStorage.write({ key, value })`.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = storageWriteFrame))]
pub fn storage_write_frame(request_id: String, key: String, value: Vec<u8>) -> Vec<u8> {
	let request = HostLocalStorageWriteRequest { key, value: Bytes(value) };
	frame(request_id, LOCAL_STORAGE_WRITE, MESSAGE_TYPE_REQUEST, request.encode())
}

/// `localStorage.read({ key })`.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = storageReadFrame))]
pub fn storage_read_frame(request_id: String, key: String) -> Vec<u8> {
	let request = HostLocalStorageReadRequest { key };
	frame(request_id, LOCAL_STORAGE_READ, MESSAGE_TYPE_REQUEST, request.encode())
}

/// `notifications.pushNotification({ text })`.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = pushNotificationFrame))]
pub fn push_notification_frame(request_id: String, text: String) -> Vec<u8> {
	let request = HostPushNotificationRequest { text };
	frame(request_id, NOTIFICATIONS_PUSH_NOTIFICATION, MESSAGE_TYPE_REQUEST, request.encode())
}

/// Reads a frame the host sent, as text.
#[cfg_attr(not(target_arch = "wasm32"), uniffi::export)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = describeFrame))]
pub fn describe_frame(frame: Vec<u8>) -> ReceivedFrame {
	let message = match ProtocolMessage::decode(&mut frame.as_slice()) {
		Ok(message) => message,
		Err(error) => {
			return ReceivedFrame {
				request_id: String::new(),
				leg: FrameLeg::Unknown,
				outcome: error.to_string(),
			};
		},
	};
	let ProtocolMessage { request_id, ids, message_type, value } = message;
	let leg = match message_type {
		MESSAGE_TYPE_RESPONSE if ids == ACCOUNT_ACCOUNTS_SUBSCRIBE => FrameLeg::Item,
		MESSAGE_TYPE_RESPONSE => FrameLeg::Response,
		MESSAGE_TYPE_INTERRUPT => FrameLeg::End,
		_ => FrameLeg::Unknown,
	};
	let outcome = match (ids, &leg) {
		(ACCOUNT_GET_ACCOUNTS, _) => {
			describe::<Result<HostAccountGetAccountsResponse, CallError<GenericError>>>(&value)
		},
		(ACCOUNT_ACCOUNTS_SUBSCRIBE, FrameLeg::Item) => {
			describe::<HostAccountAccountsSubscribeItem>(&value)
		},
		(ACCOUNT_ACCOUNTS_SUBSCRIBE, _) => describe::<Result<(), CallError<GenericError>>>(&value),
		(SIGNING_SIGN_RAW, _) => {
			describe::<Result<HostSignRawResponse, CallError<HostSignRawError>>>(&value)
		},
		(LOCAL_STORAGE_WRITE, _) => {
			describe::<Result<(), CallError<HostLocalStorageError>>>(&value)
		},
		(LOCAL_STORAGE_READ, _) => describe::<
			Result<HostLocalStorageReadResponse, CallError<HostLocalStorageError>>,
		>(&value),
		(NOTIFICATIONS_PUSH_NOTIFICATION, _) => {
			describe::<Result<(), CallError<HostPushNotificationError>>>(&value)
		},
		_ => format!("unknown method ({}, {})", ids.trait_id, ids.method_id),
	};
	ReceivedFrame { request_id, leg, outcome }
}

fn describe<T: Decode + Debug>(payload: &[u8]) -> String {
	match T::decode_all(&mut &payload[..]) {
		Ok(value) => format!("{value:?}"),
		Err(error) => format!("undecodable: {error}"),
	}
}
