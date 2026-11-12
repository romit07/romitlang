#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BOOK="$ROOT/docs/book"
mkdir -p "$BOOK"
cp "$ROOT/install.sh" "$ROOT/install.ps1" "$ROOT/docs/src/"
(cd "$ROOT/docs" && mdbook build)
"$ROOT/scripts/build_wasm.sh" "$ROOT/site/playground/pkg"
mkdir -p "$ROOT/site/playground/examples"
cp "$ROOT"/examples/beginner/*.rmt "$ROOT/site/playground/examples/"
cp "$ROOT"/examples/apps/*.rmt "$ROOT/site/playground/examples/"
cp -a "$ROOT/site/." "$BOOK/"
test -f "$BOOK/index.html"
test -f "$BOOK/playground/index.html"
test -f "$BOOK/playground/pkg/romit_wasm_bg.wasm"
