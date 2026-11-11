#!/bin/sh
# Romit installer — POSIX sh
# SPDX-License-Identifier: MIT
set -eu

ROMIT_REPO="${ROMIT_REPO:-romit07/romitlang}"
ROMIT_INSTALL_DIR="${ROMIT_INSTALL_DIR:-$HOME/.romit/bin}"
ROMIT_MODIFY_PATH="${ROMIT_MODIFY_PATH:-}"

err() {
  printf 'romit install: %s\n' "$*" >&2
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "required command not found: $1"
    exit 1
  fi
}

detect_target() {
  os="$(uname -s 2>/dev/null || true)"
  arch="$(uname -m 2>/dev/null || true)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64 | amd64) echo "x86_64-unknown-linux-gnu" ;;
        aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
        *)
          err "unsupported Linux architecture: $arch"
          exit 1
          ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64) echo "x86_64-apple-darwin" ;;
        arm64 | aarch64) echo "aarch64-apple-darwin" ;;
        *)
          err "unsupported macOS architecture: $arch"
          exit 1
          ;;
      esac
      ;;
    *)
      err "unsupported operating system: $os (use install.ps1 on Windows)"
      exit 1
      ;;
  esac
}

normalize_version() {
  v="$1"
  case "$v" in
    v*) echo "${v#v}" ;;
    *) echo "$v" ;;
  esac
}

fetch_latest_version() {
  need_cmd curl
  tag=$(curl -fsSL "https://api.github.com/repos/${ROMIT_REPO}/releases/latest" |
    grep '"tag_name"' |
    head -n 1 |
    sed 's/.*"tag_name": *"//;s/".*//')
  normalize_version "$tag"
}

download_file() {
  url="$1"
  dest="$2"
  if [ -n "${ROMIT_BASE_URL:-}" ]; then
    base="${ROMIT_BASE_URL%/}"
    case "$url" in
      http://* | https://* | file://*) curl -fsSL "$url" -o "$dest" ;;
      *)
        # Relative filename against ROMIT_BASE_URL
        curl -fsSL "${base}/${url}" -o "$dest"
        ;;
    esac
  else
    curl -fsSL "$url" -o "$dest"
  fi
}

verify_checksum() {
  archive="$1"
  sums="$2"
  need_cmd sha256sum
  base="${archive##*/}"
  line=$(grep " ${base}\$" "$sums" || grep " ${base}" "$sums" | head -n 1 || true)
  if [ -z "$line" ]; then
    err "no SHA256 entry for ${base} in checksum file"
    exit 1
  fi
  expected="${line%% *}"
  actual=$(sha256sum "$archive" | awk '{print $1}')
  if [ "$expected" != "$actual" ]; then
    err "checksum mismatch for ${base}"
    err "expected: $expected"
    err "actual:   $actual"
    exit 1
  fi
}

path_snippet() {
  # shellcheck disable=SC2016
  printf 'export PATH="%s:$PATH"' "$ROMIT_INSTALL_DIR"
}

print_path_help() {
  snippet=$(path_snippet)
  err ""
  err "Add Romit to your PATH:"
  err ""
  err "  bash / zsh — add to ~/.bashrc or ~/.zshrc:"
  err "    $snippet"
  err ""
  err "  fish — add to ~/.config/fish/config.fish:"
  err "    fish_add_path $ROMIT_INSTALL_DIR"
  err ""
}

maybe_append_path() {
  snippet=$(path_snippet)
  if [ "$ROMIT_MODIFY_PATH" = "1" ]; then
    for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
      if [ -f "$rc" ] && ! grep -Fq "$ROMIT_INSTALL_DIR" "$rc" 2>/dev/null; then
        printf '\n# romit\n%s\n' "$snippet" >>"$rc"
        err "appended PATH to $rc"
      fi
    done
    fish_config="$HOME/.config/fish/config.fish"
    if [ -f "$fish_config" ] && ! grep -Fq "$ROMIT_INSTALL_DIR" "$fish_config" 2>/dev/null; then
      mkdir -p "$(dirname "$fish_config")"
      printf '\n# romit\nfish_add_path %s\n' "$ROMIT_INSTALL_DIR" >>"$fish_config"
      err "appended fish_add_path to $fish_config"
    fi
  else
    print_path_help
    err "Re-run with ROMIT_MODIFY_PATH=1 to append PATH lines automatically."
  fi
}

do_uninstall() {
  if [ -f "${ROMIT_INSTALL_DIR}/romit" ]; then
    rm -f "${ROMIT_INSTALL_DIR}/romit"
    err "removed ${ROMIT_INSTALL_DIR}/romit"
  else
    err "no romit binary at ${ROMIT_INSTALL_DIR}/romit"
  fi
  if [ -d "$HOME/.romit/bin" ] && [ -z "$(ls -A "$HOME/.romit/bin" 2>/dev/null || true)" ]; then
    rmdir "$HOME/.romit/bin" 2>/dev/null || true
  fi
}

do_install() {
  need_cmd curl
  need_cmd tar
  target=$(detect_target)
  if [ -n "${ROMIT_VERSION:-}" ]; then
    version=$(normalize_version "$ROMIT_VERSION")
  else
    version=$(fetch_latest_version)
    if [ -z "$version" ]; then
      err "could not determine latest release version"
      err "set ROMIT_VERSION or ROMIT_BASE_URL for local testing"
      exit 1
    fi
  fi

  archive="romit-${version}-${target}.tar.gz"
  if [ -n "${ROMIT_BASE_URL:-}" ]; then
    base="${ROMIT_BASE_URL%/}"
    archive_url="${base}/${archive}"
    sums_url="${base}/SHA256SUMS"
  else
    archive_url="https://github.com/${ROMIT_REPO}/releases/download/v${version}/${archive}"
    sums_url="https://github.com/${ROMIT_REPO}/releases/download/v${version}/SHA256SUMS"
  fi

  tmpdir="${TMPDIR:-/tmp}/romit-install.$$"
  mkdir -p "$tmpdir"
  trap 'rm -rf "$tmpdir"' EXIT INT HUP TERM

  archive_path="${tmpdir}/${archive}"
  sums_path="${tmpdir}/SHA256SUMS"

  err "installing romit ${version} for ${target}"
  download_file "$archive_url" "$archive_path"
  download_file "$sums_url" "$sums_path"
  verify_checksum "$archive_path" "$sums_path"

  extract_dir="${tmpdir}/extract"
  mkdir -p "$extract_dir"
  tar -xzf "$archive_path" -C "$extract_dir"

  if [ -f "${extract_dir}/romit" ]; then
    bin_src="${extract_dir}/romit"
  else
    bin_src=$(find "$extract_dir" -name romit -type f | head -n 1)
    if [ -z "$bin_src" ]; then
      err "archive did not contain a romit binary"
      exit 1
    fi
  fi

  mkdir -p "$ROMIT_INSTALL_DIR"
  cp "$bin_src" "${ROMIT_INSTALL_DIR}/romit"
  chmod +x "${ROMIT_INSTALL_DIR}/romit"

  err "installed to ${ROMIT_INSTALL_DIR}/romit"
  "${ROMIT_INSTALL_DIR}/romit" --version
  maybe_append_path
}

usage() {
  err "usage: install.sh [--uninstall]"
  err "environment: ROMIT_VERSION ROMIT_BASE_URL ROMIT_INSTALL_DIR ROMIT_MODIFY_PATH ROMIT_REPO"
}

main() {
  if [ "${ROMIT_PRINT_TARGET:-}" = "1" ]; then
    detect_target
    exit 0
  fi
  case "${1:-}" in
    --uninstall | uninstall)
      do_uninstall
      ;;
    -h | --help)
      usage
      ;;
    "")
      do_install
      ;;
    *)
      err "unknown option: $1"
      usage
      exit 1
      ;;
  esac
}

main "$@"
