#!/usr/bin/env -S just --justfile

set shell := ["bash", "-cu"]

_default:
  @just --list -u

alias r := ready
alias c := check
alias t := test

# When ready, run the same checks as CI
ready:
  git diff --exit-code --quiet
  just fmt
  just check
  just test
  just lint
  just doc
  git status

# Format all files
fmt:
  cargo fmt --all

# Compile the workspace
check:
  ./scripts/check.sh rust-check

# Run all the tests
test:
  ./scripts/check.sh rust-test

# Lint the whole workspace
lint:
  ./scripts/check.sh clippy

# Build the documentation
doc:
  ./scripts/check.sh rust-doc

# Compare emitted instrumentation against Istanbul byte-for-byte
istanbul-diff:
  ./scripts/check.sh istanbul-diff

# Replay the upstream Istanbul spec subset
istanbul-upstream:
  ./scripts/check.sh istanbul-upstream
