// The host's side of the iframe handshake: frame the product, and hand it one
// end of a `MessageChannel` once it asks.

/** Options of {@link createIframeHost}. */
export interface IframeHostOptions {
  /** URL of the product page. */
  iframeUrl: string;
  /** Element the iframe is appended to. */
  container: HTMLElement;
}

/** A framed product. */
export interface IframeHost {
  iframe: HTMLIFrameElement;
  /** The host's end of the product's channel. */
  port: MessagePort;
  dispose(): void;
}

/**
 * Frame a product and transfer one end of a `MessageChannel` to it when it
 * sends `truapi-ready`; the other end is the returned `port`. Frames then
 * travel over the channel only.
 */
export function createIframeHost(options: IframeHostOptions): IframeHost {
  const channel = new MessageChannel();
  let productPort: MessagePort | undefined = channel.port2;

  const iframe = document.createElement("iframe");
  iframe.setAttribute("sandbox", "allow-forms allow-same-origin allow-scripts");
  iframe.src = options.iframeUrl;
  const origin = new URL(options.iframeUrl, window.location.href).origin;

  const onMessage = (event: MessageEvent) => {
    if (event.source !== iframe.contentWindow || event.origin !== origin)
      return;
    if (event.data?.type !== "truapi-ready" || productPort === undefined)
      return;
    iframe.contentWindow?.postMessage({ type: "truapi-init" }, origin, [
      productPort,
    ]);
    productPort = undefined;
  };
  window.addEventListener("message", onMessage);
  options.container.appendChild(iframe);

  return {
    iframe,
    port: channel.port1,
    dispose() {
      window.removeEventListener("message", onMessage);
      channel.port1.close();
      iframe.remove();
    },
  };
}
