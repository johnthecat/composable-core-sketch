// What every web host shares: framing a product, and connecting it to the
// runtime its WASM binary builds.

export {
  connectProductPort,
  type HostRuntime,
  type ProductRuntime,
} from "./runtime.js";
export {
  createIframeHost,
  type IframeHost,
  type IframeHostOptions,
} from "./iframe-host.js";
