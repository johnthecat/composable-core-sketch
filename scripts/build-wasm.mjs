#!/usr/bin/env node
// Builds the core and the mock wallet provider as two separate WASM binaries
// with `wasm-pack build --target web`. The web host loads both and connects
// them at runtime.
//
//   sketch-core         -> js/packages/host/wasm/         (@sketch/host)
//   sketch-wallet-mock  -> js/packages/wallet-mock/wasm/  (@sketch/wallet-mock)
//
// Each crate is built on its own, so Cargo resolves features per binary: only
// the provider turns on the interface crate's `export` feature. The check at the
// end holds that, by looking for the `WasmWallet` class in each binary's glue.
//
// SKETCH_WASM_PROFILE=dev builds unoptimised binaries; the default is release.

import { execFile } from "node:child_process";
import { readFile, rm, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const profile = process.env.SKETCH_WASM_PROFILE ?? "release";
if (profile !== "release" && profile !== "dev") {
  throw new Error(
    `Unsupported SKETCH_WASM_PROFILE=${profile}; expected release or dev`,
  );
}

const binaries = [
  {
    crate: "sketch-core",
    outName: "sketch_core",
    outDir: "js/packages/host/wasm",
    exportsWasmWallet: false,
  },
  {
    crate: "sketch-wallet-mock",
    outName: "sketch_wallet_mock",
    outDir: "js/packages/wallet-mock/wasm",
    exportsWasmWallet: true,
  },
];

function formatBytes(bytes) {
  return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KiB`;
}

async function build({ crate, outName, outDir, exportsWasmWallet }) {
  const out = resolve(repoRoot, outDir);
  const args = [
    "build",
    "--target",
    "web",
    "--out-dir",
    out,
    "--out-name",
    outName,
    profile === "dev" ? "--dev" : "--release",
    resolve(repoRoot, "rust/crates", crate),
  ];
  process.stdout.write(`wasm-pack build ${crate} --${profile} -> ${outDir}\n`);
  try {
    await execFileAsync("wasm-pack", args, { cwd: repoRoot });
  } catch (error) {
    if (error?.code === "ENOENT") {
      console.error(
        "wasm-pack is required. Install it with `cargo install wasm-pack`.",
      );
      process.exit(1);
    }
    process.stderr.write(error.stderr ?? "");
    throw error;
  }
  // wasm-pack writes a nested `.gitignore: *`; the repository's own ignores
  // cover the output directories.
  await rm(resolve(out, ".gitignore"), { force: true });

  const glue = await readFile(resolve(out, `${outName}.d.ts`), "utf8");
  const hasWasmWallet = /\bclass WasmWallet\b/.test(glue);
  if (hasWasmWallet !== exportsWasmWallet) {
    throw new Error(
      `${crate}: expected the WasmWallet class to be ${
        exportsWasmWallet ? "exported" : "left out"
      }, but it is ${hasWasmWallet ? "exported" : "left out"}`,
    );
  }
  const { size } = await stat(resolve(out, `${outName}_bg.wasm`));
  process.stdout.write(
    `  ${outName}_bg.wasm ${formatBytes(size)}, WasmWallet ${
      hasWasmWallet ? "exported" : "left out"
    }\n`,
  );
}

for (const binary of binaries) {
  await build(binary);
}
