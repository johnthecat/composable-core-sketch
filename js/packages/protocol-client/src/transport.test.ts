import { describe, expect, test } from "bun:test";

import { createClient } from "./generated/client.js";
import * as T from "./generated/types.js";
import * as S from "./scale.js";
import {
  MESSAGE_TYPE_INTERRUPT,
  MESSAGE_TYPE_RESPONSE,
  createTransport,
  decodeWireMessage,
  encodeWireMessage,
  type ProtocolMessage,
  type WireProvider,
} from "./transport.js";

describe("wire frames", () => {
  // The same bytes as the host runtime's `frame` test: both sides must agree
  // on the layout or no frame crosses.
  test("a frame is its request id, address, leg and raw payload", () => {
    const message: ProtocolMessage = {
      requestId: "p:1",
      traitId: 13,
      methodId: 0,
      messageType: 1,
      value: new Uint8Array([0xaa, 0xbb]),
    };
    const encoded = encodeWireMessage(message);

    expect(encoded).toEqual(
      new Uint8Array([0x0c, 0x70, 0x3a, 0x31, 13, 0, 1, 0xaa, 0xbb]),
    );
    expect(decodeWireMessage(encoded)).toEqual(message);
  });
});

/** A host stand-in: hands each product frame to `answer`, which replies. */
function loopback(
  answer: (
    frame: ProtocolMessage,
    reply: (frame: ProtocolMessage) => void,
  ) => void,
): WireProvider {
  const listeners = new Set<(frame: Uint8Array) => void>();
  const reply = (frame: ProtocolMessage) => {
    for (const listener of listeners) listener(encodeWireMessage(frame));
  };
  return {
    postMessage: (bytes) => answer(decodeWireMessage(bytes), reply),
    subscribe(callback) {
      listeners.add(callback);
      return () => listeners.delete(callback);
    },
  };
}

describe("the generated client over a transport", () => {
  test("a domain error arrives as the versioned error the method declares", async () => {
    const client = createClient(
      createTransport(
        loopback((frame, reply) => {
          const error = S.Result(
            T.VersionedHostSignRawResponse,
            S.CallError(T.VersionedHostSignRawError),
          ).enc({
            success: false,
            value: {
              tag: "Domain",
              value: { tag: "V1", value: { tag: "Rejected" } },
            },
          });
          reply({ ...frame, messageType: MESSAGE_TYPE_RESPONSE, value: error });
        }),
      ),
    );

    const result = await client.signing.signRaw({
      publicKey: new Uint8Array(32),
      payload: new Uint8Array(),
    });

    expect(result.isErr() && result.error).toEqual({
      tag: "Domain",
      value: { tag: "V1", value: { tag: "Rejected", value: undefined } },
    });
  });

  test("a subscription delivers items, then completes on a clean interrupt", () => {
    const client = createClient(
      createTransport(
        loopback((frame, reply) => {
          const item = T.VersionedHostAccountAccountsSubscribeItem.enc({
            tag: "V1",
            value: {
              accounts: [{ publicKey: new Uint8Array(32), name: "alice" }],
            },
          });
          reply({ ...frame, messageType: MESSAGE_TYPE_RESPONSE, value: item });
          reply({
            ...frame,
            messageType: MESSAGE_TYPE_INTERRUPT,
            value: new Uint8Array([0]),
          });
        }),
      ),
    );

    const names: string[][] = [];
    let completed = false;
    client.account.accountsSubscribe().subscribe({
      next: (item) => names.push(item.accounts.map((account) => account.name)),
      complete: () => {
        completed = true;
      },
    });

    expect({ names, completed }).toEqual({
      names: [["alice"]],
      completed: true,
    });
  });
});
