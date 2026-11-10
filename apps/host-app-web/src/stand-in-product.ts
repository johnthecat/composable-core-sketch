// The stand-in product: request frames that `product-stand-in` builds in the
// app's WASM binary, sent straight into a product runtime. The core serves
// them as it serves any product's frames; the Swift and Kotlin apps send the
// same ones.

import {
  accountsSubscribeFrame,
  accountsUnsubscribeFrame,
  describeFrame,
  FrameLeg,
  type HostRuntime,
  type ProductRuntime,
} from "../wasm/host_app_web_wasm.js";

/** A frame the core sent to the product, read back as text. */
export interface Frame {
  readonly requestId: string;
  readonly leg: FrameLeg;
  readonly outcome: string;
}

/** One product connected to a host runtime. */
export class StandInProduct {
  readonly #runtime: ProductRuntime;
  readonly #answers = new Map<string, (frame: Frame) => void>();
  readonly #items = new Map<string, (frame: Frame) => void>();
  #nextRequest = 0;

  /**
   * Connects the product `productId` to `host`. `onFrame` sees every frame
   * the core sends it, in order.
   */
  constructor(
    host: HostRuntime,
    productId: string,
    onFrame: (frame: Frame) => void = () => {},
  ) {
    this.#runtime = host.productRuntime(
      { productId },
      { emitFrame: (bytes) => this.#receive(bytes, onFrame) },
    );
  }

  /** Sends the request `build` makes, and resolves with its answer. */
  call(build: (requestId: string) => Uint8Array): Promise<Frame> {
    const requestId = this.#requestId();
    return new Promise((resolve) => {
      this.#answers.set(requestId, resolve);
      this.#runtime.receiveFrame(build(requestId));
    });
  }

  /**
   * Follows the account list: each item goes to `onItem` as well as to
   * `onFrame`. Returns a function that sends the stop.
   */
  followAccounts(onItem: (frame: Frame) => void = () => {}): () => void {
    const requestId = this.#requestId();
    this.#items.set(requestId, onItem);
    this.#runtime.receiveFrame(accountsSubscribeFrame(requestId));
    return () => {
      this.#items.delete(requestId);
      this.#runtime.receiveFrame(accountsUnsubscribeFrame(requestId));
    };
  }

  /** Disconnects: the core drops the calls still in flight. */
  disconnect(): void {
    this.#runtime.free();
  }

  #requestId(): string {
    this.#nextRequest += 1;
    return `p:${this.#nextRequest}`;
  }

  #receive(bytes: Uint8Array, onFrame: (frame: Frame) => void): void {
    const received = describeFrame(bytes);
    const frame: Frame = {
      requestId: received.requestId,
      leg: received.leg,
      outcome: received.outcome,
    };
    received.free();
    onFrame(frame);
    if (frame.leg === FrameLeg.Item) {
      this.#items.get(frame.requestId)?.(frame);
      return;
    }
    this.#items.delete(frame.requestId);
    this.#answers.get(frame.requestId)?.(frame);
    this.#answers.delete(frame.requestId);
  }
}
