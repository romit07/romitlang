#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const wasmPath = process.argv[2];
const jsPath = process.argv[3];
if (!wasmPath || !jsPath) {
  console.error("usage: verify_wasm.mjs <wasm> <js>");
  process.exit(2);
}
const mod = await import(pathToFileURL(jsPath).href);
const bytes = readFileSync(wasmPath);
await mod.default(bytes);
const version = mod.romit_version();
if (version !== "0.0.2") {
  console.error("unexpected version", version);
  process.exit(1);
}
const out = JSON.parse(mod.run_program('say "Hello, world!"\n'));
if (!out.ok || !out.stdout.includes("Hello, world!")) {
  console.error("wasm run failed", out);
  process.exit(1);
}
console.log("wasm hello ok");
