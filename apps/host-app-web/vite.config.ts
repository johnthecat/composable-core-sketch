import { defineConfig } from "vite";

// The web host app. Its WASM binary, built from `host-app-web-wasm` into
// `wasm/`, reaches the page through the wasm-pack glue, which names the `.wasm`
// file with `new URL(…, import.meta.url)`; Vite serves and emits it from that.
export default defineConfig({
  build: { target: "es2022" },
  server: { port: 5181, strictPort: true },
  preview: { port: 5181, strictPort: true },
});
