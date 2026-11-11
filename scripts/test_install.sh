#!/bin/sh
# Exercise install.sh against a locally built release tarball.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing command: %s\n' "$1" >&2
    exit 1
  fi
}

need_cmd cargo
need_cmd curl
need_cmd sha256sum
need_cmd tar

TARGET="$(rustc -vV | sed -n 's/^host: //p')"
DETECTED="$(ROMIT_PRINT_TARGET=1 sh "${ROOT}/install.sh")"
if [ "$DETECTED" != "$TARGET" ]; then
  printf 'detect_target=%s host=%s\n' "$DETECTED" "$TARGET" >&2
  exit 1
fi

VERSION="0.0.1-test"
ARCHIVE="romit-${VERSION}-${TARGET}.tar.gz"
STAGING="${ROOT}/.install-test-dist"
INSTALL_ROOT="${ROOT}/.install-test-home"
FAKE_HOME="${INSTALL_ROOT}/home"
BIN_DIR="${FAKE_HOME}/.romit/bin"

rm -rf "$STAGING" "$INSTALL_ROOT"
mkdir -p "${STAGING}/romit-${VERSION}-${TARGET}" "$BIN_DIR"

cargo build --release -p romit
cp "target/release/romit" "${STAGING}/romit-${VERSION}-${TARGET}/romit"
cp LICENSE README.md "${STAGING}/romit-${VERSION}-${TARGET}/"
tar -czf "${STAGING}/${ARCHIVE}" -C "$STAGING" "romit-${VERSION}-${TARGET}"
(
  cd "$STAGING"
  sha256sum "${ARCHIVE}" >SHA256SUMS
)

case "$(uname -s)" in
  Darwin)
    if command -v python3 >/dev/null 2>&1; then
      SERVER_CMD="python3 -m http.server"
    else
      SERVER_CMD="python -m SimpleHTTPServer"
    fi
    ;;
  *)
    SERVER_CMD="python3 -m http.server"
    ;;
esac

PORT="${ROMIT_TEST_PORT:-8765}"
$SERVER_CMD "$PORT" --directory "$STAGING" >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT INT HUP TERM
sleep 1

ROMIT_VERSION="$VERSION" \
ROMIT_BASE_URL="http://127.0.0.1:${PORT}" \
ROMIT_INSTALL_DIR="$BIN_DIR" \
HOME="$FAKE_HOME" \
  sh "${ROOT}/install.sh"

if ! "$BIN_DIR/romit" --version >/dev/null; then
  printf 'installed binary failed --version\n' >&2
  exit 1
fi

printf 'install.sh integration test passed\n'
