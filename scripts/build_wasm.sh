#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/site/playground/pkg}"
rustup target add wasm32-unknown-unknown
cargo build -p romit_wasm --release --target wasm32-unknown-unknown
mkdir -p "$OUT"
wasm-bindgen "$ROOT/target/wasm32-unknown-unknown/release/romit_wasm.wasm" \
  --out-dir "$OUT" \
  --target web \
  --no-typescript
