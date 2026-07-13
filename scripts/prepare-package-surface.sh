#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "${BASH_SOURCE[0]%/*}/.." && pwd)"
NAPI_DIR="$ROOT/crates/oxc-coverage-instrument-napi"
THREADED_PACKAGE="$NAPI_DIR/npm/wasm32-wasi"
TMP="$(mktemp -d)"
THREAD_ROOT_READY=0

die() {
  echo "prepare-package-surface: $1" >&2
  exit 1
}

require_tool() {
  local tool="$1"
  local hint="$2"
  if ! command -v "$tool" >/dev/null 2>&1; then
    die "Required tool '$tool' is not installed. $hint"
  fi
}

package_files() {
  local manifest="$1"
  node -e '
    const manifest = require(process.argv[1]);
    for (const file of manifest.files) console.log(file);
  ' "$manifest"
}

copy_package_files() {
  local source_dir="$1"
  local target_dir="$2"
  local manifest="$3"
  local file
  mkdir -p "$target_dir"
  while IFS= read -r file; do
    if [ ! -f "$source_dir/$file" ]; then
      die "WASI build did not produce '$file' in ${source_dir#"$ROOT"/}."
    fi
    cp "$source_dir/$file" "$target_dir/$file"
  done < <(package_files "$manifest")
}

restore_threaded_root() {
  if [ "$THREAD_ROOT_READY" != "1" ]; then
    return
  fi
  copy_package_files "$TMP/threaded" "$NAPI_DIR" "$THREADED_PACKAGE/package.json"
  cp "$TMP/browser.js" "$NAPI_DIR/browser.js"
  (cd "$NAPI_DIR" && node scripts/patch-browser-loader.mjs)
  THREAD_ROOT_READY=0
}

cleanup() {
  restore_threaded_root
  rm -rf "$TMP"
}
trap cleanup EXIT

require_tool node "Install Node.js 22."
require_tool npm "Install npm with Node.js."
require_tool npx "Install npm with Node.js."
require_tool cargo "Install Rust using rustup."
require_tool rustup "Install Rust using rustup."

if [ ! -d "$NAPI_DIR/node_modules/@napi-rs/cli" ]; then
  die "N-API dependencies are missing. Run 'npm --prefix crates/oxc-coverage-instrument-napi install'."
fi

installed_targets="$(rustup target list --installed)"
for target in wasm32-wasip1-threads wasm32-wasip1; do
  if ! grep -Fxq "$target" <<<"$installed_targets"; then
    die "Rust target '$target' is missing. Run 'rustup target add $target'."
  fi
done

echo "[prepare:package-surface] build threaded wasm32-wasip1-threads artifacts"
(
  cd "$NAPI_DIR"
  node scripts/patch-napi-wasi-link-dir.mjs
  rm -f coverage-instrument.wasm32-wasi*.wasm
  npx napi build --release --platform --target wasm32-wasip1-threads
  node scripts/patch-wasi-browser-shim.mjs
)
copy_package_files "$NAPI_DIR" "$TMP/threaded" "$THREADED_PACKAGE/package.json"
cp "$NAPI_DIR/browser.js" "$TMP/browser.js"
THREAD_ROOT_READY=1

echo "[prepare:package-surface] build single-threaded wasm32-wasip1 artifacts"
(
  cd "$NAPI_DIR"
  rm -f coverage-instrument.wasm32-wasi*.wasm
  NAPI_RS_WASI_LINK_DIR=wasm32-wasip1 npx napi build --release --platform --target wasm32-wasip1
  node scripts/prepare-wasi-singlethreaded-package.mjs .
  node scripts/validate-wasi-singlethreaded-package.mjs npm/wasm32-wasi-singlethreaded
)

copy_package_files "$TMP/threaded" "$THREADED_PACKAGE" "$THREADED_PACKAGE/package.json"
restore_threaded_root

echo "[prepare:package-surface] package artifacts are ready"
