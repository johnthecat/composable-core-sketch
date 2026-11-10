#!/usr/bin/env node
// Builds the web host app's one WASM binary from its host crate,
// `apps/host-app-web/host-app-web-wasm`, with `wasm-pack build --target web`:
// the core and both providers in one binary, written to
// `apps/host-app-web/wasm/`. It also holds the stand-in product.
//
// The check after the build holds the generated `.d.ts` to the shape the app's
// `tsc` relies on: it imports the interface crate's TypeScript module exactly
// once, as `HostExtension`, and declares no `Wasm<Trait>` class. The app's
// `tsc` then holds every signature to the interfaces in that module.
//
// SKETCH_WASM_PROFILE=dev builds an unoptimised binary; the default is release.

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

const crate = "apps/host-app-web/host-app-web-wasm";
const outName = "host_app_web_wasm";
const out = resolve(repoRoot, "apps/host-app-web/wasm");

process.stdout.write(`wasm-pack build ${crate} --${profile} -> ${out}\n`);
try {
  await execFileAsync(
    "wasm-pack",
    [
      "build",
      "--target",
      "web",
      "--out-dir",
      out,
      "--out-name",
      outName,
      profile === "dev" ? "--dev" : "--release",
      resolve(repoRoot, crate),
    ],
    { cwd: repoRoot },
  );
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
// cover the output directory.
await rm(resolve(out, ".gitignore"), { force: true });

const glue = await readFile(resolve(out, `${outName}.d.ts`), "utf8");
// Two imports would mean two crates declared `typescript_module!`, which the
// one `HostExtension` namespace cannot serve.
const imports = glue.match(/^import type \* as HostExtension from "[^"]+";$/gm);
const leaked = [...glue.matchAll(/\bclass (Wasm\w+)\b/g)].map(
  ([, name]) => name,
);
if (imports?.length !== 1 || leaked.length > 0) {
  throw new Error(
    `${outName}.d.ts: ${imports?.length ?? 0} HostExtension imports, expected 1; leaked ${
      leaked.join(", ") || "nothing"
    }`,
  );
}
const { size } = await stat(resolve(out, `${outName}_bg.wasm`));
process.stdout.write(
  `  ${outName}_bg.wasm ${(size / 1024).toFixed(1)} KiB, typed extension points\n`,
);
