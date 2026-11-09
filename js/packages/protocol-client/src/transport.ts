// The byte-level transport the generated client calls: wire frames, request
// correlation, and subscriptions as observables.

import { ResultAsync, err, ok, type Result } from "neverthrow";
import { Result as ResultCodec, _void, str, type Codec } from "scale-ts";

/** Wire pair of one protocol method. */
export interface MethodIds {
  /** Trait id, the first byte of the pair. */
  trait: number;
  /** Method id within the trait. */
  method: number;
}

/** Product to host: a request, or a subscription's start. */
export const MESSAGE_TYPE_REQUEST = 0;
/** Host to product: a response, or one subscription item. */
export const MESSAGE_TYPE_RESPONSE = 1;
/** Host to product: a subscription's end. */
export const MESSAGE_TYPE_INTERRUPT = 2;
/** Product to host: stop a subscription. */
export const MESSAGE_TYPE_STOP = 3;
/** Trait id of method-independent protocol errors. */
export const PROTOCOL_ERROR_TRAIT_ID = 255;

/**
 * One wire frame: `[requestId: SCALE str][trait][method][message_type][payload]`.
 * The payload is the leg's versioned envelope, without a length prefix.
 */
export interface ProtocolMessage {
  requestId: string;
  traitId: number;
  methodId: number;
  messageType: number;
  value: Uint8Array;
}

/** Encode a frame. */
export function encodeWireMessage(message: ProtocolMessage): Uint8Array {
  const head = str.enc(message.requestId);
  const frame = new Uint8Array(head.length + 3 + message.value.length);
  frame.set(head);
  frame.set(
    [message.traitId, message.methodId, message.messageType],
    head.length,
  );
  frame.set(message.value, head.length + 3);
  return frame;
}

/** Decode a frame. */
export function decodeWireMessage(frame: Uint8Array): ProtocolMessage {
  const requestId = str.dec(frame);
  const offset = str.enc(requestId).length;
  const [traitId, methodId, messageType] = frame.subarray(offset, offset + 3);
  if (
    traitId === undefined ||
    methodId === undefined ||
    messageType === undefined
  ) {
    throw new Error("wire frame too short");
  }
  return {
    requestId,
    traitId,
    methodId,
    messageType,
    value: frame.slice(offset + 3),
  };
}

/** The pipe frames travel through, such as a `MessagePort`. */
export interface WireProvider {
  /** Send one frame to the peer. */
  postMessage(frame: Uint8Array): void;
  /** Receive each frame from the peer; returns a function that stops. */
  subscribe(callback: (frame: Uint8Array) => void): () => void;
}

/** A provider over a `MessagePort`. */
export function createMessagePortProvider(port: MessagePort): WireProvider {
  const listeners = new Set<(frame: Uint8Array) => void>();
  port.onmessage = (event: MessageEvent) => {
    if (event.data instanceof Uint8Array) {
      for (const listener of [...listeners]) listener(event.data);
    }
  };
  port.start();
  return {
    postMessage: (frame) => port.postMessage(frame),
    subscribe(callback) {
      listeners.add(callback);
      return () => listeners.delete(callback);
    },
  };
}

/** The host serves no method at the address a call used. */
export class UnsupportedMessageError extends Error {
  constructor(ids: MethodIds) {
    super(`the host serves no method at (${ids.trait}, ${ids.method})`);
    this.name = "UnsupportedMessageError";
  }
}

/** Options of {@link TrUApiTransport.request}. */
export interface RequestParams<Ok, Err> {
  ids: MethodIds;
  /** The encoded request envelope. */
  payload: Uint8Array;
  /** Decode the response leg into its outcome. */
  decodeResponse(payload: Uint8Array):
    | { success: true; value: Ok }
    | {
        success: false;
        value: Err;
      };
}

/** Options of {@link TrUApiTransport.subscribeRaw}. */
export interface SubscribeRawParams {
  ids: MethodIds;
  /** The encoded start envelope. */
  payload: Uint8Array;
  /** Each item leg's payload. */
  onReceive(payload: Uint8Array): void;
  /** The interrupt leg's payload, which ends the subscription. */
  onInterrupt(payload: Uint8Array): void;
  /** The subscription could not run. */
  onClose(error: Error): void;
}

/** A running subscription. */
export interface Subscription {
  unsubscribe(): void;
}

/** What the generated client calls. */
export interface TrUApiTransport {
  /** Send a request and settle with its decoded outcome. */
  request<Ok, Err>(params: RequestParams<Ok, Err>): ResultAsync<Ok, Err>;
  /** Start a subscription. */
  subscribeRaw(params: SubscribeRawParams): Subscription;
}

/** A transport over `provider`. */
export function createTransport(provider: WireProvider): TrUApiTransport {
  let counter = 0;
  const nextId = () => `p:${++counter}`;
  const requests = new Map<
    string,
    { ids: MethodIds; settle(frame: ProtocolMessage): void }
  >();
  const subscriptions = new Map<string, SubscribeRawParams>();

  provider.subscribe((bytes) => {
    const frame = decodeWireMessage(bytes);
    if (frame.traitId === PROTOCOL_ERROR_TRAIT_ID) {
      const request = requests.get(frame.requestId);
      const subscription = subscriptions.get(frame.requestId);
      requests.delete(frame.requestId);
      subscriptions.delete(frame.requestId);
      if (request) request.settle(frame);
      subscription?.onClose(new UnsupportedMessageError(subscription.ids));
      return;
    }
    const request = requests.get(frame.requestId);
    if (request && frame.messageType === MESSAGE_TYPE_RESPONSE) {
      requests.delete(frame.requestId);
      request.settle(frame);
      return;
    }
    const subscription = subscriptions.get(frame.requestId);
    if (!subscription) return;
    if (frame.messageType === MESSAGE_TYPE_RESPONSE) {
      subscription.onReceive(frame.value);
    } else if (frame.messageType === MESSAGE_TYPE_INTERRUPT) {
      subscriptions.delete(frame.requestId);
      subscription.onInterrupt(frame.value);
    }
  });

  const send = (
    requestId: string,
    ids: MethodIds,
    messageType: number,
    value: Uint8Array,
  ) =>
    provider.postMessage(
      encodeWireMessage({
        requestId,
        traitId: ids.trait,
        methodId: ids.method,
        messageType,
        value,
      }),
    );

  return {
    request<Ok, Err>(params: RequestParams<Ok, Err>): ResultAsync<Ok, Err> {
      const requestId = nextId();
      const outcome = new Promise<Result<Ok, Err>>((resolve, reject) => {
        requests.set(requestId, {
          ids: params.ids,
          settle(frame) {
            if (frame.traitId === PROTOCOL_ERROR_TRAIT_ID) {
              reject(new UnsupportedMessageError(params.ids));
              return;
            }
            try {
              const decoded = params.decodeResponse(frame.value);
              resolve(decoded.success ? ok(decoded.value) : err(decoded.value));
            } catch (error) {
              reject(error);
            }
          },
        });
      });
      send(requestId, params.ids, MESSAGE_TYPE_REQUEST, params.payload);
      return new ResultAsync(outcome);
    },

    subscribeRaw(params) {
      const requestId = nextId();
      subscriptions.set(requestId, params);
      send(requestId, params.ids, MESSAGE_TYPE_REQUEST, params.payload);
      return {
        unsubscribe() {
          if (!subscriptions.delete(requestId)) return;
          send(requestId, params.ids, MESSAGE_TYPE_STOP, new Uint8Array());
        },
      };
    },
  };
}

/** Why a subscription ended early: its interrupt value, or a transport failure. */
export class SubscriptionError<Reason> extends Error {
  readonly reason: Reason | undefined;

  constructor(message: string, reason?: Reason) {
    super(message);
    this.name = "SubscriptionError";
    this.reason = reason;
  }
}

/** Receives a subscription's items and its end. */
export interface Observer<Item, Reason> {
  next?(item: Item): void;
  error?(error: SubscriptionError<Reason>): void;
  complete?(): void;
}

/** A subscription a product starts by subscribing an observer. */
export interface ObservableLike<Item, Reason> {
  subscribe(observer?: Observer<Item, Reason>): Subscription;
}

/**
 * The decoder of an interrupt leg, `Result<(), Reason>`: `undefined` for a
 * clean end, the reason otherwise.
 */
export function interruptDecoder<Reason>(
  reason: Codec<Reason>,
): (payload: Uint8Array) => Reason | undefined {
  const codec = ResultCodec(_void, reason);
  return (payload) => {
    const decoded = codec.dec(payload);
    return decoded.success ? undefined : decoded.value;
  };
}

/** A subscription method's result: starts the subscription per observer. */
export function createObservable<Item, Reason>(options: {
  transport: TrUApiTransport;
  ids: MethodIds;
  payload: Uint8Array;
  decodeItem(payload: Uint8Array): Item;
  decodeInterrupt(payload: Uint8Array): Reason | undefined;
}): ObservableLike<Item, Reason> {
  return {
    subscribe(observer = {}) {
      return options.transport.subscribeRaw({
        ids: options.ids,
        payload: options.payload,
        onReceive: (payload) => observer.next?.(options.decodeItem(payload)),
        onInterrupt: (payload) => {
          const reason = options.decodeInterrupt(payload);
          if (reason === undefined) observer.complete?.();
          else
            observer.error?.(
              new SubscriptionError("subscription interrupted", reason),
            );
        },
        onClose: (error) =>
          observer.error?.(new SubscriptionError<Reason>(error.message)),
      });
    },
  };
}
