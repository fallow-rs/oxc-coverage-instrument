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
  rust
  pre-push
  all-local
)
expected_list="$(printf '%s\n' "${expected_profiles[@]}")"
actual_list="$(awk '{print $1}' "$TMP/list.log")"
assert_equal "$actual_list" "$expected_list" "profile inventory"

while read -r documented_profile; do
  if ! grep -Eq "^${documented_profile}[[:space:]]" "$TMP/list.log"; then
    fail "CONTRIBUTING.md references unknown check profile '$documented_profile'"
  fi
done < <(sed -n 's#.*\./scripts/check\.sh \([a-z][a-z0-9-]*\).*#\1#p' "$ROOT/CONTRIBUTING.md")

echo "check self-test: PASS"
