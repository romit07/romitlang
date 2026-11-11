#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ "${SKIP_SITE_BUILD:-}" != "1" ]; then
  "$ROOT/scripts/build_site.sh"
fi
BOOK="$ROOT/docs/book"
PORT="${PORT:-9877}"
python3 -m http.server "$PORT" -d "$BOOK" &
PID=$!
trap 'kill $PID 2>/dev/null || true' EXIT
sleep 1
for path in / /introduction.html /install.html /playground/ /why.html; do
  code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT$path")
  test "$code" = 200 || { echo "missing $path ($code)"; exit 1; }
done
node "$ROOT/scripts/verify_wasm.mjs" "$BOOK/playground/pkg/romit_wasm_bg.wasm" "$BOOK/playground/pkg/romit_wasm.js"
echo "site verify ok"
