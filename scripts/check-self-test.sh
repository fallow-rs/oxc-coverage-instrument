#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHECK="$ROOT/scripts/check.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
  echo "check self-test failed: $1" >&2
  exit 1
}

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "$expected" "$file"; then
    echo "expected output to contain: $expected" >&2
    sed -n '1,120p' "$file" >&2
    fail "unexpected output"
  fi
}

assert_equal() {
  local actual="$1"
  local expected="$2"
  local context="$3"
  if [ "$actual" != "$expected" ]; then
    echo "expected $context:" >&2
    printf '%s\n' "$expected" >&2
    echo "actual $context:" >&2
    printf '%s\n' "$actual" >&2
    fail "$context differs"
  fi
}

assert_file_equal() {
  local file="$1"
  local expected="$2"
  local context="$3"
  if [ ! -f "$file" ]; then
    fail "$context is missing"
  fi
  assert_equal "$(<"$file")" "$expected" "$context"
}

write_fake_surface_fixture() {
  local napi_dir="$1"
  local bin_dir="$2"
  local file

  mkdir -p \
    "$napi_dir/scripts" \
    "$napi_dir/node_modules/@napi-rs/cli/dist" \
    "$napi_dir/npm/wasm32-wasi" \
    "$napi_dir/npm/wasm32-wasi-singlethreaded" \
    "$bin_dir"
  cp "$ROOT/crates/oxc-coverage-instrument-napi/package.json" "$napi_dir/package.json"
  cp "$ROOT/crates/oxc-coverage-instrument-napi/npm/wasm32-wasi/package.json" \
    "$napi_dir/npm/wasm32-wasi/package.json"
  cp "$ROOT/crates/oxc-coverage-instrument-napi/npm/wasm32-wasi-singlethreaded/package.json" \
    "$napi_dir/npm/wasm32-wasi-singlethreaded/package.json"

  while IFS= read -r file; do
    printf 'original-root:%s\n' "$file" >"$napi_dir/$file"
  done < <(node -e 'for (const file of require(process.argv[1]).files) console.log(file)' "$napi_dir/package.json")
  printf 'original-root:%s\n' 'coverage-instrument.wasm32-wasi.wasm' \
    >"$napi_dir/coverage-instrument.wasm32-wasi.wasm"
  while IFS= read -r file; do
    printf 'original-threaded:%s\n' "$file" >"$napi_dir/npm/wasm32-wasi/$file"
    printf 'original-single:%s\n' "$file" >"$napi_dir/npm/wasm32-wasi-singlethreaded/$file"
  done < <(node -e 'for (const file of require(process.argv[1]).files) console.log(file)' \
    "$napi_dir/npm/wasm32-wasi/package.json")

  printf '%s\n' \
    "import { writeFileSync } from 'node:fs';" \
    "writeFileSync(new URL('../browser.js', import.meta.url), 'selector\\n');" \
    >"$napi_dir/scripts/patch-browser-loader.mjs"
  for file in cli.js index.js index.cjs; do
    printf 'original-cli:%s\n' "$file" >"$napi_dir/node_modules/@napi-rs/cli/dist/$file"
  done
  printf '%s\n' \
    "import { writeFileSync } from 'node:fs';" \
    "writeFileSync(new URL('../node_modules/@napi-rs/cli/dist/cli.js', import.meta.url), 'patched-cli\\n');" \
    >"$napi_dir/scripts/patch-napi-wasi-link-dir.mjs"
  : >"$napi_dir/scripts/patch-wasi-browser-shim.mjs"
  printf '%s\n' \
    "import { copyFileSync, readFileSync } from 'node:fs';" \
    "const root = new URL('../', import.meta.url);" \
    "const target = new URL('../npm/wasm32-wasi-singlethreaded/', import.meta.url);" \
    "const manifest = JSON.parse(readFileSync(new URL('package.json', target), 'utf8'));" \
    "for (const file of manifest.files) copyFileSync(new URL(file, root), new URL(file, target));" \
    "if (process.env.FAIL_PREPARE_PACKAGE_SURFACE === '1') process.exit(9);" \
    >"$napi_dir/scripts/prepare-wasi-singlethreaded-package.mjs"
  printf '%s\n' \
    "import { readFileSync } from 'node:fs';" \
    "const target = new URL('../npm/wasm32-wasi-singlethreaded/', import.meta.url);" \
    "const manifest = JSON.parse(readFileSync(new URL('package.json', target), 'utf8'));" \
    "for (const file of manifest.files) {" \
    "  if (!readFileSync(new URL(file, target), 'utf8').startsWith('single:')) process.exit(8);" \
    "}" \
    >"$napi_dir/scripts/validate-wasi-singlethreaded-package.mjs"

  ln -s "$(command -v node)" "$bin_dir/node"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$bin_dir/npm"
  printf '%s\n' '#!/bin/sh' 'exit 0' >"$bin_dir/cargo"
  # Keep fake-tool variables literal so they expand only when the fake runs.
  # shellcheck disable=SC2016
  printf '%s\n' '#!/bin/sh' \
    'if [ "$1" = "target" ]; then printf "%s\n" wasm32-wasip1-threads wasm32-wasip1; fi' \
    >"$bin_dir/rustup"
  # shellcheck disable=SC2016
  printf '%s\n' '#!/bin/sh' \
    'kind=threaded' \
    'for arg in "$@"; do if [ "$arg" = "wasm32-wasip1" ]; then kind=single; fi; done' \
    'for file in $(node -e '\''for (const file of require(process.argv[1]).files) console.log(file)'\'' "$FAKE_NAPI_DIR/npm/wasm32-wasi/package.json"); do' \
    '  printf "%s:%s\n" "$kind" "$file" > "$FAKE_NAPI_DIR/$file"' \
    'done' \
    'printf "%s:index.js\n" "$kind" > "$FAKE_NAPI_DIR/index.js"' \
    'printf "%s:index.d.ts\n" "$kind" > "$FAKE_NAPI_DIR/index.d.ts"' \
    'printf "%s:browser.js\n" "$kind" > "$FAKE_NAPI_DIR/browser.js"' \
    >"$bin_dir/npx"
  chmod +x "$bin_dir/npm" "$bin_dir/cargo" "$bin_dir/rustup" "$bin_dir/npx"
}

assert_prepared_surface() {
  local napi_dir="$1"
  local file

  assert_file_equal "$napi_dir/index.js" "threaded:index.js" "threaded root index.js"
  assert_file_equal "$napi_dir/index.d.ts" "threaded:index.d.ts" "threaded root index.d.ts"
  assert_file_equal "$napi_dir/browser.js" "selector" "root browser selector"
  while IFS= read -r file; do
    assert_file_equal "$napi_dir/npm/wasm32-wasi/$file" "threaded:$file" "threaded package $file"
    assert_file_equal "$napi_dir/npm/wasm32-wasi-singlethreaded/$file" "single:$file" \
      "single-threaded package $file"
  done < <(node -e 'for (const file of require(process.argv[1]).files) console.log(file)' \
    "$napi_dir/npm/wasm32-wasi/package.json")
}

if "$CHECK" unknown-profile >"$TMP/unknown.log" 2>&1; then
  fail "unknown profile succeeded"
fi
assert_contains "$TMP/unknown.log" "Unknown profile: unknown-profile"
assert_contains "$TMP/unknown.log" "Usage:"

mkdir -p "$TMP/empty-bin"
if PATH="$TMP/empty-bin" /bin/bash "$CHECK" typos >"$TMP/missing.log" 2>&1; then
  fail "direct profile succeeded without its prerequisite"
fi
assert_contains "$TMP/missing.log" "Required tool 'typos' is not installed."

mkdir -p "$TMP/old-node-bin"
printf '%s\n' '#!/bin/sh' 'echo 20' >"$TMP/old-node-bin/node"
chmod +x "$TMP/old-node-bin/node"
if PATH="$TMP/old-node-bin:/usr/bin:/bin" "$CHECK" inspector-smoke >"$TMP/old-node.log" 2>&1; then
  fail "inspector profile accepted an unsupported Node.js major"
fi
assert_contains "$TMP/old-node.log" "Node.js 22 is required for inspector-smoke."

mkdir -p "$TMP/old-python-bin"
printf '%s\n' '#!/bin/sh' 'echo 3.10' 'exit 1' >"$TMP/old-python-bin/python3"
chmod +x "$TMP/old-python-bin/python3"
if PATH="$TMP/old-python-bin:/usr/bin:/bin" "$CHECK" version-sync >"$TMP/old-python.log" 2>&1; then
  fail "version-sync accepted unsupported Python"
fi
assert_contains "$TMP/old-python.log" "Python 3.11 or newer with tomllib is required for version-sync. Found 3.10."

mkdir -p "$TMP/fake-bin"
# Keep fake-tool variables literal so they expand only when the fake runs.
# shellcheck disable=SC2016
printf '%s\n' '#!/bin/sh' 'printf "%s|%s\n" "$PWD" "$*" > "$FAKE_CARGO_LOG"' >"$TMP/fake-bin/cargo"
chmod +x "$TMP/fake-bin/cargo"
(
  cd "$TMP"
  FAKE_CARGO_LOG="$TMP/fmt-cargo.log" PATH="$TMP/fake-bin:/usr/bin:/bin" "$CHECK" fmt >"$TMP/fmt.log" 2>&1
)
assert_equal "$(<"$TMP/fmt-cargo.log")" "$ROOT|fmt --all --check" "external-cwd fmt invocation"

FAKE_CARGO_LOG="$TMP/pre-push-cargo.log" PATH="$TMP/fake-bin:/usr/bin:/bin" "$CHECK" pre-push >"$TMP/pre-push.log" 2>&1
assert_contains "$TMP/pre-push.log" "typos is not installed; skipping optional pre-push check"

fake_napi="$TMP/fake-napi"
fake_surface_bin="$TMP/fake-surface-bin"
fake_prepare="$TMP/prepare-package-surface.sh"
write_fake_surface_fixture "$fake_napi" "$fake_surface_bin"
sed "s#^NAPI_DIR=.*#NAPI_DIR=\"$fake_napi\"#" "$ROOT/scripts/prepare-package-surface.sh" \
  >"$fake_prepare"
chmod +x "$fake_prepare"
cp -R "$fake_napi" "$TMP/fake-napi-prerequisite"
rm -rf "$TMP/fake-napi-prerequisite/node_modules"
sed "s#^NAPI_DIR=.*#NAPI_DIR=\"$TMP/fake-napi-prerequisite\"#" \
  "$ROOT/scripts/prepare-package-surface.sh" >"$TMP/prepare-package-prerequisite.sh"
chmod +x "$TMP/prepare-package-prerequisite.sh"
cp -R "$TMP/fake-napi-prerequisite" "$TMP/fake-napi-prerequisite-before"
if FAKE_NAPI_DIR="$TMP/fake-napi-prerequisite" PATH="$fake_surface_bin:/usr/bin:/bin" \
  "$TMP/prepare-package-prerequisite.sh" >"$TMP/prepare-prerequisite.log" 2>&1; then
  fail "package preparation succeeded without N-API dependencies"
fi
if ! diff -qr "$TMP/fake-napi-prerequisite-before" "$TMP/fake-napi-prerequisite" \
  >"$TMP/prepare-prerequisite.diff"; then
  sed -n '1,120p' "$TMP/prepare-prerequisite.diff" >&2
  fail "package preparation changed destinations before prerequisites passed"
fi
cp -R "$fake_napi" "$TMP/fake-napi-before"
if FAKE_NAPI_DIR="$fake_napi" FAIL_PREPARE_PACKAGE_SURFACE=1 \
  PATH="$fake_surface_bin:/usr/bin:/bin" "$fake_prepare" >"$TMP/prepare-failure.log" 2>&1; then
  fail "package preparation failure injection succeeded"
fi
if ! diff -qr "$TMP/fake-napi-before" "$fake_napi" >"$TMP/prepare-restore.diff"; then
  sed -n '1,120p' "$TMP/prepare-restore.diff" >&2
  fail "package preparation did not restore destinations after failure"
fi
FAKE_NAPI_DIR="$fake_napi" PATH="$fake_surface_bin:/usr/bin:/bin" \
  "$fake_prepare" >"$TMP/prepare-success.log" 2>&1
assert_prepared_surface "$fake_napi"

(
  cd "$TMP"
  "$CHECK" --list >"$TMP/list.log"
)

expected_profiles=(
  fmt
  clippy
  rust-check
  rust-test-fast
  rust-test
  doc-test
  rust-doc
  typos
  version-sync
  napi-test
  wasi-shim-test
  browser-loader-test
  istanbul-diff
  prepare-package-surface
  package-surface
  audit
  shear
  inspector-smoke
  real-world-output
  istanbul-upstream
  vitest-typecheck
  vitest-coverage
  vitest-verify
  actionlint
  zizmor
  commitlint
  self-test
  rust
  pre-push
  all-local
)
expected_list="$(printf '%s\n' "${expected_profiles[@]}")"
actual_list="$(awk '{print $1}' "$TMP/list.log")"
assert_equal "$actual_list" "$expected_list" "profile inventory"

sed -n '/^run_all_local() {$/,/^}$/p' "$CHECK" >"$TMP/all-local-body.log"
assert_contains "$TMP/all-local-body.log" "run_self_test"

while read -r documented_profile; do
  if ! grep -Eq "^${documented_profile}[[:space:]]" "$TMP/list.log"; then
    fail "CONTRIBUTING.md references unknown check profile '$documented_profile'"
  fi
done < <(sed -n 's#.*\./scripts/check\.sh \([a-z][a-z0-9-]*\).*#\1#p' "$ROOT/CONTRIBUTING.md")

echo "check self-test: PASS"
