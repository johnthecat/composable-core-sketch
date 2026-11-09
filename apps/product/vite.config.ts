import { defineConfig } from "vite";

// The product is served from its own origin, as a real product is. The host
// app frames it from http://localhost:5182/ in dev and in preview.
export default defineConfig({
  build: { target: "es2022" },
  server: { port: 5182, strictPort: true },
  preview: { port: 5182, strictPort: true },
});
