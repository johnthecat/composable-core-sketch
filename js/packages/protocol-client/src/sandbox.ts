// The product's side of the iframe handshake: ask the parent host for a
// `MessagePort`, then speak the protocol over it.

import { createClient, type TrUApiClient } from "./generated/client.js";
import { createMessagePortProvider, createTransport } from "./transport.js";

/** How often the product repeats `truapi-ready` until the host answers. */
const READY_INTERVAL_MS = 100;

/**
 * Connect to the host page this product is framed in. The product repeats
 * `truapi-ready` to its parent until the host answers with `truapi-init`
 * carrying a `MessagePort`, which then carries every frame.
 */
export function connectIframeHost(): Promise<TrUApiClient> {
  return new Promise((resolve) => {
    const onMessage = (event: MessageEvent) => {
      if (event.source !== window.parent) return;
      if (event.data?.type !== "truapi-init") return;
      const [port] = event.ports;
      if (!port) return;
      window.removeEventListener("message", onMessage);
      window.clearInterval(interval);
      resolve(createClient(createTransport(createMessagePortProvider(port))));
    };
    window.addEventListener("message", onMessage);
    const postReady = () =>
      window.parent.postMessage({ type: "truapi-ready" }, "*");
    const interval = window.setInterval(postReady, READY_INTERVAL_MS);
    postReady();
  });
}
