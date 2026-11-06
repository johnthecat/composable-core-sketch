import { defineConfig } from "vite";

// Both WASM binaries reach the page through their packages' wasm-pack glue,
// which names each `.wasm` file with `new URL(…, import.meta.url)`; Vite serves
// and emits them from that.
export default defineConfig({
  build: { target: "es2022" },
});
