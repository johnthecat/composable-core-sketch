// The product client: the generated service clients over a byte-level
// transport, and the iframe handshake that connects a product to its host.

export * from "./generated/client.js";
export * from "./generated/types.js";
export * as scale from "./scale.js";
export type { CallErrorValue } from "./scale.js";
export * as wire from "./generated/wire-table.js";
export {
  createMessagePortProvider,
  createTransport,
  decodeWireMessage,
  encodeWireMessage,
  MESSAGE_TYPE_RESPONSE,
  SubscriptionError,
  type MethodIds,
  type ObservableLike,
  type Observer,
  type ProtocolMessage,
  type Subscription,
  type TrUApiTransport,
  type WireProvider,
} from "./transport.js";
export { connectIframeHost } from "./sandbox.js";
