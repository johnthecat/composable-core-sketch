// Connecting a framed product to a host runtime. `Product` and `FrameSink`
// are what a host's WASM binary takes: its `.d.ts` names them from here. The
// runtime itself comes from that binary; `HostRuntime` and `ProductRuntime`
// are the part of its classes this module calls.

/** The product a runtime serves. */
export interface Product {
  productId: string;
}

/** Where a product runtime sends the frames it produces, as natively. */
export interface FrameSink {
  /** One encoded frame for the product. */
  emitFrame(frame: Uint8Array): void;
}

/** One product's connection to the core. */
export interface ProductRuntime {
  /** Serve one frame from the product. */
  receiveFrame(frame: Uint8Array): Promise<void>;
  /** Release the WASM object, which ends the connection. */
  free(): void;
}

/** The runtime a host builds once and shares across products. */
export interface HostRuntime {
  /** A runtime serving `product`, which sends its frames to `sink`. */
  productRuntime(product: Product, sink: FrameSink): ProductRuntime;
}

/**
 * Serve the product on the far end of `port`: its frames go to a new product
 * runtime, and the runtime's frames go back. Returns a function that
 * disconnects it.
 */
export function connectProductPort(
  runtime: HostRuntime,
  product: Product,
  port: MessagePort,
): () => void {
  const productRuntime = runtime.productRuntime(product, {
    emitFrame: (frame) => port.postMessage(frame),
  });
  port.onmessage = (event: MessageEvent) => {
    if (event.data instanceof Uint8Array) {
      productRuntime.receiveFrame(event.data).catch((error: unknown) => {
        console.error("the core refused a product frame", error);
      });
    }
  };
  port.start();
  return () => {
    port.onmessage = null;
    port.close();
    productRuntime.free();
  };
}
