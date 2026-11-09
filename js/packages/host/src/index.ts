// What every web host shares: the extension point interfaces its WASM binary
// takes, generated from the interface crate by tool-codegen, and the code that
// frames a product and connects it to the runtime.

export {
  connectProductPort,
  type FrameSink,
  type HostRuntime,
  type Product,
  type ProductRuntime,
} from "./runtime.js";
export {
  createIframeHost,
  type IframeHost,
  type IframeHostOptions,
} from "./iframe-host.js";
export * from "./generated/extensions.js";
