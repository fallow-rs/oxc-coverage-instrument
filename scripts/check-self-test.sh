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
printf '%s\n' '#!/bin/sh' 'exit 0' >"$TMP/fake-bin/cargo"
chmod +x "$TMP/fake-bin/cargo"
PATH="$TMP/fake-bin:/usr/bin:/bin" "$CHECK" pre-push >"$TMP/pre-push.log" 2>&1
assert_contains "$TMP/pre-push.log" "typos is not installed; skipping optional pre-push check"

(
  cd "$TMP"
  "$CHECK" --list >"$TMP/list.log"
)
assert_contains "$TMP/list.log" "fmt"
assert_contains "$TMP/list.log" "rust-check"
assert_contains "$TMP/list.log" "rust-test-fast"
assert_contains "$TMP/list.log" "wasi-shim-test"
assert_contains "$TMP/list.log" "browser-loader-test"
assert_contains "$TMP/list.log" "commitlint"
assert_contains "$TMP/list.log" "all-local"

echo "check self-test: PASS"
