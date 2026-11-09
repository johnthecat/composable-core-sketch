// SCALE codec primitives the generated client and host code use: `scale-ts`,
// plus the codec of the protocol's `CallError<D>`.

import { Enum, Struct, _void, str, type Codec } from "scale-ts";

export type { Codec, ResultPayload } from "scale-ts";
export {
  Bytes,
  Enum,
  Option,
  Result,
  Struct,
  Vector,
  _void,
  bool,
  str,
  u16,
  u32,
  u64,
  u8,
} from "scale-ts";

/** Rust's `CallError<D>`: the method's own error, or one any method can fail with. */
export type CallErrorValue<D> =
  | { tag: "Domain"; value: D }
  | { tag: "Denied"; value?: undefined }
  | { tag: "Unsupported"; value?: undefined }
  | { tag: "MalformedFrame"; value: { reason: string } }
  | { tag: "HostFailure"; value: { reason: string } };

/** SCALE codec of `CallError<D>`, in the Rust enum's variant order. */
export function CallError<D>(domain: Codec<D>): Codec<CallErrorValue<D>> {
  return Enum({
    Domain: domain,
    Denied: _void,
    Unsupported: _void,
    MalformedFrame: Struct({ reason: str }),
    HostFailure: Struct({ reason: str }),
  }) as unknown as Codec<CallErrorValue<D>>;
}
