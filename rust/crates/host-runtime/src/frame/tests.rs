use super::*;

// The TypeScript client encodes the same layout; `js/packages/protocol-client/src/
// transport.test.ts` asserts these bytes on its side.
#[test]
fn a_frame_is_its_request_id_address_leg_and_raw_payload() {
	let message = ProtocolMessage {
		request_id: "p:1".into(),
		payload: Payload { trait_id: 13, method_id: 0, message_type: 1, value: vec![0xaa, 0xbb] },
	};

	let encoded = message.encode();

	assert_eq!(encoded, [&[0x0c][..], b"p:1", &[13, 0, 1, 0xaa, 0xbb]].concat());
	assert_eq!(ProtocolMessage::decode(&mut encoded.as_slice()).unwrap(), message);
}
