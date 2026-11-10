//! A stand-in product for the Swift and Kotlin demos.
//!
//! Native hosts run products in web views, which speak the protocol through the
//! TypeScript client. The command-line demos have no web view, so they build a
//! product's frames here instead: encoded by the protocol crate's own types, so
//! the bytes are exactly what the client sends, and read back as text. This is
//! a test helper, not a product SDK. The native host crate links it like a
//! provider, so its exports reach the apps from the same library.

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
		Payload, ProtocolMessage,
	},
};

uniffi::setup_scaffolding!();

/// Which leg of a method's exchange a frame from the host carries.
#[derive(uniffi::Enum)]
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
#[derive(uniffi::Record)]
pub struct ReceivedFrame {
	/// The request it answers.
	pub request_id: String,
	/// Which leg it carries.
	pub leg: FrameLeg,
	/// The decoded payload.
	pub outcome: String,
}

fn frame(request_id: String, ids: MethodIds, message_type: u8, value: Vec<u8>) -> Vec<u8> {
	ProtocolMessage::new(request_id, ids, message_type, value).encode()
}

/// `account.getAccounts()`.
#[uniffi::export]
pub fn get_accounts_frame(request_id: String) -> Vec<u8> {
	frame(request_id, ACCOUNT_GET_ACCOUNTS, MESSAGE_TYPE_REQUEST, Vec::new())
}

/// `account.accountsSubscribe()`.
#[uniffi::export]
pub fn accounts_subscribe_frame(request_id: String) -> Vec<u8> {
	frame(request_id, ACCOUNT_ACCOUNTS_SUBSCRIBE, MESSAGE_TYPE_REQUEST, Vec::new())
}

/// Stops the account subscription started with `request_id`.
#[uniffi::export]
pub fn accounts_unsubscribe_frame(request_id: String) -> Vec<u8> {
	frame(request_id, ACCOUNT_ACCOUNTS_SUBSCRIBE, MESSAGE_TYPE_STOP, Vec::new())
}

/// `signing.signRaw({ publicKey, payload })`.
#[uniffi::export]
pub fn sign_raw_frame(request_id: String, public_key: Bytes32, payload: Vec<u8>) -> Vec<u8> {
	let request = HostSignRawRequest { public_key, payload: Bytes(payload) };
	frame(request_id, SIGNING_SIGN_RAW, MESSAGE_TYPE_REQUEST, request.encode())
}

/// `localStorage.write({ key, value })`.
#[uniffi::export]
pub fn storage_write_frame(request_id: String, key: String, value: Vec<u8>) -> Vec<u8> {
	let request = HostLocalStorageWriteRequest { key, value: Bytes(value) };
	frame(request_id, LOCAL_STORAGE_WRITE, MESSAGE_TYPE_REQUEST, request.encode())
}

/// `localStorage.read({ key })`.
#[uniffi::export]
pub fn storage_read_frame(request_id: String, key: String) -> Vec<u8> {
	let request = HostLocalStorageReadRequest { key };
	frame(request_id, LOCAL_STORAGE_READ, MESSAGE_TYPE_REQUEST, request.encode())
}

/// `notifications.pushNotification({ text })`.
#[uniffi::export]
pub fn push_notification_frame(request_id: String, text: String) -> Vec<u8> {
	let request = HostPushNotificationRequest { text };
	frame(request_id, NOTIFICATIONS_PUSH_NOTIFICATION, MESSAGE_TYPE_REQUEST, request.encode())
}

/// Reads a frame the host sent, as text.
#[uniffi::export]
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
	let Payload { trait_id, method_id, message_type, value } = message.payload;
	let ids = MethodIds { trait_id, method_id };
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
		_ => format!("unknown method ({trait_id}, {method_id})"),
	};
	ReceivedFrame { request_id: message.request_id, leg, outcome }
}

fn describe<T: Decode + Debug>(payload: &[u8]) -> String {
	match T::decode_all(&mut &payload[..]) {
		Ok(value) => format!("{value:?}"),
		Err(error) => format!("undecodable: {error}"),
	}
}
