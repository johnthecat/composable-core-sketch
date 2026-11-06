// Hand-written helpers the generated adapters in `generated/` call.

/**
 * Runs `call` and maps how it settles: a value through `onValue`, a rejection or
 * exception through `onReason`.
 */
export async function mapSettled<T, U>(
  call: () => Promise<T>,
  onValue: (value: T) => U,
  onReason: (reason: unknown) => unknown,
): Promise<U> {
  let value: T;
  try {
    value = await call();
  } catch (reason) {
    throw onReason(reason);
  }
  return onValue(value);
}

/**
 * Delivers each item of `iterable` to `onItem` until it ends, throws, or the
 * returned function is called. A throw goes to `onError`.
 */
export function pumpIterable<T>(
  iterable: AsyncIterable<T>,
  onItem: (item: T) => void,
  onError: (reason: unknown) => void,
): () => void {
  const iterator = iterable[Symbol.asyncIterator]();
  let disposed = false;
  void (async () => {
    try {
      for (;;) {
        const result = await iterator.next();
        if (disposed || result.done) return;
        onItem(result.value);
      }
    } catch (reason) {
      if (!disposed) onError(reason);
    }
  })();
  return () => {
    if (disposed) return;
    disposed = true;
    void iterator.return?.();
  };
}

/**
 * Turns a callback subscription into an async iterable. Iterating starts the
 * subscription and leaving the loop ends it. An error ends the iteration by
 * throwing.
 */
export function subscriptionIterable<T>(
  subscribe: (
    onItem: (item: T) => void,
    onError: (reason: unknown) => void,
  ) => () => void,
): AsyncIterable<T> {
  return {
    [Symbol.asyncIterator](): AsyncIterator<T> {
      const items: T[] = [];
      let failure: { reason: unknown } | undefined;
      let finished = false;
      let wake: (() => void) | undefined;
      const notify = () => {
        wake?.();
        wake = undefined;
      };
      const unsubscribe = subscribe(
        (item) => {
          items.push(item);
          notify();
        },
        (reason) => {
          failure = { reason };
          notify();
        },
      );
      const finish = () => {
        if (finished) return;
        finished = true;
        unsubscribe();
      };
      return {
        async next(): Promise<IteratorResult<T>> {
          while (!finished && items.length === 0 && failure === undefined) {
            await new Promise<void>((resolve) => {
              wake = resolve;
            });
          }
          if (items.length > 0) {
            const [item] = items.splice(0, 1) as [T];
            return { done: false, value: item };
          }
          finish();
          if (failure !== undefined) {
            const { reason } = failure;
            failure = undefined;
            throw reason;
          }
          return { done: true, value: undefined };
        },
        async return(): Promise<IteratorResult<T>> {
          finish();
          notify();
          return { done: true, value: undefined };
        },
      };
    },
  };
}
