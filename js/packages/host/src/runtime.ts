// Connecting a framed product to a host runtime. The runtime comes from the
// host's own WASM binary; `HostRuntime` and `ProductRuntime` are the part of
// its classes this module calls, over the interface crate's `ProductContext`
// and `FrameSink`.

import type { FrameSink, ProductContext } from "@sketch/host-extension";

/** One product's connection to the core. */
export interface ProductRuntime {
  /** Start serving one frame from the product; throws on a malformed one. */
  receiveFrame(frame: Uint8Array): void;
  /** Release the WASM object, which ends the connection and drops its calls. */
  free(): void;
}

/** The runtime a host builds once and shares across products. */
export interface HostRuntime {
  /** A runtime serving `product`, which sends its frames to `sink`. */
  productRuntime(product: ProductContext, sink: FrameSink): ProductRuntime;
  /** Release the WASM object. */
  free(): void;
}

/**
 * Serve the product on the far end of `port`: its frames go to a new product
 * runtime, and the runtime's frames go back. Returns a function that
 * disconnects it.
 */
export function connectProductPort(
  runtime: HostRuntime,
  product: ProductContext,
  port: MessagePort,
): () => void {
  const productRuntime = runtime.productRuntime(product, {
    emitFrame: (frame) => port.postMessage(frame),
  });
  port.onmessage = (event: MessageEvent) => {
    if (!(event.data instanceof Uint8Array)) return;
    try {
      productRuntime.receiveFrame(event.data);
    } catch (error) {
      console.error("the core refused a product frame", error);
    }
  };
  port.start();
  return () => {
    port.onmessage = null;
    port.close();
    productRuntime.free();
  };
}
