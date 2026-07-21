#!/usr/bin/env bash
# Benchmark oxc-coverage-instrument against istanbul-lib-instrument,
# babel-plugin-istanbul, and swc-plugin-coverage-instrument on real-world
# JavaScript libraries.
#
# Usage:
#   ./scripts/benchmark-comparison.sh            # run full benchmark
#   ./scripts/benchmark-comparison.sh --quick     # react + lodash only
#   ./scripts/benchmark-comparison.sh --self-test # deterministic harness checks
#
# Prerequisites (installed automatically on first run):
#   - cargo build --release of the CLI
#   - Node.js 18+
#   - npm packages: istanbul-lib-instrument, @babel/core,
#     babel-plugin-istanbul, @swc/core, swc-plugin-coverage-instrument
#
# Note on fairness:
#   - "oxc (native)" = CLI binary, includes ~3ms process startup overhead
#   - "oxc (napi)"   = Node.js N-API binding, apples-to-apples with other
#                       Node.js tools (same process, no startup cost)
#   - "babel-plugin" = babel-plugin-istanbul via @babel/core
#   - "swc (wasm)"   = swc-plugin-coverage-instrument, Rust compiled to WASM
#                       running inside SWC's WASM sandbox, so not a native Rust
#                       comparison (includes WASM and serialisation overhead)
#   - "istanbul-lib" = istanbul-lib-instrument standalone (parse + instrument)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BENCH_DIR="${ROOT_DIR}/.bench-tmp"
OXC="${ROOT_DIR}/target/release/oxc-coverage-instrument"
NAPI_DIR="${ROOT_DIR}/crates/oxc_coverage_instrument_napi"
LIB_DIR="${ROOT_DIR}/crates/oxc_coverage_instrument"
RUNS=5
QUICK=false
SELF_TEST=false
STATISTIC_LABEL="median"

case "${1:-}" in
  --quick)
    QUICK=true
    RUNS=3
    ;;
  --self-test) SELF_TEST=true ;;
  "") ;;
  *) echo "unknown argument: $1" >&2; exit 2 ;;
esac

# ---------- helpers ----------

BENCH_FILES=()

setup_files() {
  local args=(--print-paths)
  if [[ "$QUICK" == true ]]; then
    args+=(--limit=2)
  fi
  local paths
  paths="$(node "$SCRIPT_DIR/prepare-real-world-corpus.mjs" "${args[@]}")"
  while IFS= read -r path; do
    if [[ -n "$path" ]]; then
      BENCH_FILES+=("$path")
    fi
  done <<< "$paths"
}

setup_npm() {
  local dir="$1"; shift
  mkdir -p "$dir"
  if [[ ! -d "${dir}/node_modules" ]]; then
    echo "  installing $* in ${dir}..." >&2
    (cd "$dir" && npm init -y --silent >/dev/null 2>&1 && npm install --silent "$@" >/dev/null 2>&1)
  fi
}

build_oxc() {
  echo "  building oxc-coverage-instrument CLI (release)..." >&2
  cargo build --release -p oxc_coverage_instrument_cli \
    --manifest-path "$ROOT_DIR/Cargo.toml" 2>&1 | tail -1
}

build_napi() {
  local node_bin=""
  local candidate
  for candidate in "${NAPI_DIR}"/coverage-instrument.*.node; do
    if [[ -f "$candidate" ]]; then
      node_bin="$candidate"
      break
    fi
  done
  local newest_src
  newest_src=$(find "$LIB_DIR/src" "$NAPI_DIR/src" "$NAPI_DIR/Cargo.toml" "$ROOT_DIR/Cargo.toml" \
    -type f -print0 2>/dev/null | xargs -0 ls -t 2>/dev/null | head -1)
  if [[ -z "$node_bin" ]] || [[ -n "$newest_src" && "$newest_src" -nt "$node_bin" ]]; then
    echo "  building napi bindings (release)..." >&2
    (cd "$NAPI_DIR" && cargo clean -p oxc_coverage_instrument_napi >/dev/null 2>&1; \
                      npm run build 2>&1 | tail -1)
  fi
}

# Precise timing via Python (sub-ms accuracy, includes process startup for CLI).
# `check=True` keeps a failed subprocess from becoming an impossibly fast sample.
measure_command_median() {
  local runs="$1"
  shift
  python3 - "$runs" "$@" <<'PY'
import statistics
import subprocess
import sys
import time

runs = int(sys.argv[1])
command = sys.argv[2:]
samples = []
for _ in range(runs):
    start = time.perf_counter()
    subprocess.run(
        command,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    samples.append((time.perf_counter() - start) * 1000)
print(f"{statistics.median(samples):.1f}")
PY
}

median_values() {
  python3 - "$@" <<'PY'
import statistics
import sys

values = [float(value) for value in sys.argv[1:]]
print(f"{statistics.median(values):.1f}")
PY
}

time_oxc() {
  measure_command_median "$RUNS" "$OXC" "$1"
}

cpu_model() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    sysctl -n machdep.cpu.brand_string
  elif [[ -r /proc/cpuinfo ]]; then
    awk -F ': ' '/^model name/ { print $2; exit }' /proc/cpuinfo
  else
    uname -p
  fi
}

file_sha256() {
  node -e \
    'const { createHash } = require("node:crypto"); const { readFileSync } = require("node:fs"); console.log(createHash("sha256").update(readFileSync(process.argv[1])).digest("hex"));' \
    "$1"
}

corpus_versions() {
  # JavaScript template variables must expand inside Node, not in this shell.
  # shellcheck disable=SC2016
  node -e \
    'const manifest = require(process.argv[1]); console.log(manifest.projects.map(({ name, version }) => `${name}@${version}`).join(", "));' \
    "$SCRIPT_DIR/real-world-corpus.json"
}

benchmark_heading() {
  echo "Running benchmarks ($RUNS runs each, $STATISTIC_LABEL)..."
}

environment_metadata() {
  local repository_state="clean"
  if [[ -n "$(git -C "$ROOT_DIR" status --porcelain --untracked-files=no)" ]]; then
    repository_state="dirty"
  fi
  echo "## Reproducibility"
  echo ""
  echo "- Repository commit: $(git -C "$ROOT_DIR" rev-parse HEAD)"
  echo "- Repository state: $repository_state"
  echo "- Machine: $(uname -s) $(uname -m)"
  echo "- CPU: $(cpu_model)"
  echo "- Node.js: $(node --version)"
  echo "- Corpus manifest SHA-256: $(file_sha256 "$SCRIPT_DIR/real-world-corpus.json")"
  echo "- Corpus versions: $(corpus_versions)"
}

benchmark_self_test() {
  local temp
  temp="$(mktemp -d)"
  trap 'rm -rf "$temp"' RETURN

  [[ "$(median_values 3 1 2)" == "2.0" ]]
  [[ "$(median_values 4 1 3 2)" == "2.5" ]]
  [[ "$(median_values 2 2 2 2)" == "2.0" ]]
  [[ "$(median_values 1.25 1.75)" == "1.5" ]]

  if measure_command_median 1 /usr/bin/false >/dev/null 2>&1; then
    echo "benchmark self-test: failed subprocess became a timing sample" >&2
    return 1
  fi

  mkdir -p "$temp/bin"
  # Keep fake-command variables literal so they expand only when the fake runs.
  # shellcheck disable=SC2016
  printf '%s\n' \
    '#!/bin/sh' \
    'printf "%s\n" "$*" >>"$BENCH_SELF_BUILD_LOG"' \
    >"$temp/bin/cargo"
  chmod +x "$temp/bin/cargo"
  BENCH_SELF_BUILD_LOG="$temp/build.log" PATH="$temp/bin:$PATH" build_oxc >/dev/null
  BENCH_SELF_BUILD_LOG="$temp/build.log" PATH="$temp/bin:$PATH" build_oxc >/dev/null
  [[ "$(wc -l <"$temp/build.log" | tr -d ' ')" == "2" ]]
  grep -Fq -- \
    "build --release -p oxc_coverage_instrument_cli --manifest-path $ROOT_DIR/Cargo.toml" \
    "$temp/build.log"
  benchmark_heading >"$temp/heading.log"
  grep -Fq -- "($RUNS runs each, median)" "$temp/heading.log"
  environment_metadata >"$temp/metadata.log"
  for field in \
    "Repository commit" \
    "Repository state" \
    "Machine" \
    "CPU" \
    "Node.js" \
    "Corpus manifest SHA-256" \
    "Corpus versions"; do
    grep -Fq -- "- $field:" "$temp/metadata.log"
  done

  local BENCH_DIR="$temp/generated"
  mkdir -p "$BENCH_DIR"
  write_napi_bench
  [[ -f "$BENCH_DIR/napi-bench.cjs" ]]
  node --check "$BENCH_DIR/napi-bench.cjs"
  echo "benchmark self-test passed"
}

npm_package_version() {
  node -e 'console.log(require(process.argv[1]).version)' "$1"
}

# ---------- benchmark scripts ----------

write_istanbul_bench() {
  cat > "${BENCH_DIR}/istanbul/bench.js" << 'EOF'
const { createInstrumenter } = require('istanbul-lib-instrument');
const fs = require('fs');
const path = require('path');
const file = process.argv[2];
const runs = parseInt(process.argv[3] || '5', 10);
const code = fs.readFileSync(file, 'utf8');
const instrumenter = createInstrumenter({ compact: false });
try { instrumenter.instrumentSync(code.slice(0, 500), 'warmup.js'); } catch {}
const times = [];
for (let i = 0; i < runs; i++) {
  const start = performance.now();
  instrumenter.instrumentSync(code, file);
  times.push(performance.now() - start);
}
times.sort((a, b) => a - b);
process.stdout.write(times[Math.floor(times.length / 2)].toFixed(1));
EOF
}

write_babel_bench() {
  cat > "${BENCH_DIR}/babel/bench.js" << 'EOF'
const babel = require('@babel/core');
const fs = require('fs');
const path = require('path');
const file = process.argv[2];
const runs = parseInt(process.argv[3] || '5', 10);
const code = fs.readFileSync(file, 'utf8');
babel.transformSync('const x=1;', { filename: 'w.js', plugins: ['babel-plugin-istanbul'], babelrc: false, configFile: false });
const times = [];
for (let i = 0; i < runs; i++) {
  const start = performance.now();
  babel.transformSync(code, { filename: file, plugins: ['babel-plugin-istanbul'], babelrc: false, configFile: false });
  times.push(performance.now() - start);
}
times.sort((a, b) => a - b);
process.stdout.write(times[Math.floor(times.length / 2)].toFixed(1));
EOF
}

write_swc_bench() {
  cat > "${BENCH_DIR}/swc/bench.js" << 'EOF'
const swc = require('@swc/core');
const fs = require('fs');
const path = require('path');
const file = process.argv[2];
const runs = parseInt(process.argv[3] || '5', 10);
const code = fs.readFileSync(file, 'utf8');
const pluginDir = path.dirname(require.resolve('swc-plugin-coverage-instrument'));
const wasm = fs.readdirSync(pluginDir).filter(f => f.endsWith('.wasm'));
const pluginPath = wasm.length ? path.join(pluginDir, wasm[0]) : require.resolve('swc-plugin-coverage-instrument');
try { swc.transformSync('const x=1;', { filename: 'w.js', jsc: { experimental: { plugins: [[pluginPath, {}]] } } }); } catch {}
const times = [];
for (let i = 0; i < runs; i++) {
  const start = performance.now();
  try {
    swc.transformSync(code, { filename: file, jsc: { parser: { syntax: 'ecmascript' }, experimental: { plugins: [[pluginPath, {}]] } } });
  } catch { break; }
  times.push(performance.now() - start);
}
if (times.length > 0) {
  times.sort((a, b) => a - b);
  process.stdout.write(times[Math.floor(times.length / 2)].toFixed(1));
} else {
  process.stdout.write('ERR');
}
EOF
}

write_napi_bench() {
  cat > "${BENCH_DIR}/napi-bench.cjs" << EOF
const { instrument } = require('${NAPI_DIR}');
const fs = require('fs');
const path = require('path');
const file = process.argv[2];
const runs = parseInt(process.argv[3] || '5', 10);
const code = fs.readFileSync(file, 'utf8');
instrument('const x=1;', 'warmup.js');
const times = [];
for (let i = 0; i < runs; i++) {
  const start = performance.now();
  instrument(code, file);
  times.push(performance.now() - start);
}
times.sort((a, b) => a - b);
process.stdout.write(times[Math.floor(times.length / 2)].toFixed(1));
EOF
}

# ---------- main ----------

if [[ "$SELF_TEST" == true ]]; then
  benchmark_self_test
  exit 0
fi

echo "Setting up..." >&2
setup_files
build_oxc
build_napi
setup_npm "${BENCH_DIR}/istanbul" istanbul-lib-instrument
setup_npm "${BENCH_DIR}/babel" @babel/core babel-plugin-istanbul
setup_npm "${BENCH_DIR}/swc" @swc/core swc-plugin-coverage-instrument
write_istanbul_bench
write_babel_bench
write_swc_bench
write_napi_bench

echo "" >&2
benchmark_heading >&2
echo ""

environment_metadata
echo "- oxc npm package: $(npm_package_version "$NAPI_DIR/package.json")"
echo "- istanbul-lib-instrument: $(npm_package_version "${BENCH_DIR}/istanbul/node_modules/istanbul-lib-instrument/package.json")"
echo "- @babel/core: $(npm_package_version "${BENCH_DIR}/babel/node_modules/@babel/core/package.json")"
echo "- babel-plugin-istanbul: $(npm_package_version "${BENCH_DIR}/babel/node_modules/babel-plugin-istanbul/package.json")"
echo "- @swc/core: $(npm_package_version "${BENCH_DIR}/swc/node_modules/@swc/core/package.json")"
echo "- swc-plugin-coverage-instrument: $(npm_package_version "${BENCH_DIR}/swc/node_modules/swc-plugin-coverage-instrument/package.json")"
echo ""

# ---------- Table 1: Node.js tools (apples-to-apples) ----------

echo "## Node.js API comparison (all running in the same Node.js process)"
echo ""
printf "| %-25s | %8s | %10s | %12s | %10s | %12s |\n" \
  "File" "Size" "oxc (napi)" "babel-plugin" "swc (wasm)" "istanbul-lib"
printf "|%-27s|%10s|%12s|%14s|%12s|%14s|\n" \
  "$(printf -- '-%.0s' {1..27})" "$(printf -- '-%.0s' {1..10})" \
  "$(printf -- '-%.0s' {1..12})" "$(printf -- '-%.0s' {1..14})" \
  "$(printf -- '-%.0s' {1..12})" "$(printf -- '-%.0s' {1..14})"

for filepath in "${BENCH_FILES[@]}"; do
  name=$(basename "$filepath")
  size_bytes=$(wc -c < "$filepath" | tr -d ' ')
  if (( size_bytes > 1048576 )); then
    size="$(echo "scale=1; $size_bytes / 1048576" | bc) MB"
  else
    size="$(echo "scale=0; $size_bytes / 1024" | bc) KB"
  fi

  t_napi=$(node "${BENCH_DIR}/napi-bench.cjs" "$filepath" "$RUNS")
  t_babel=$(cd "${BENCH_DIR}/babel" && node bench.js "$filepath" "$RUNS" 2>/dev/null)
  t_swc=$(cd "${BENCH_DIR}/swc" && node bench.js "$filepath" "$RUNS")
  t_istanbul=$(cd "${BENCH_DIR}/istanbul" && node bench.js "$filepath" "$RUNS")

  printf "| %-25s | %8s | %8s ms | %10s ms | %8s ms | %10s ms |\n" \
    "$name" "$size" "$t_napi" "$t_babel" "$t_swc" "$t_istanbul"
done

echo ""
echo "## Native CLI (includes ~3ms process startup)"
echo ""
printf "| %-25s | %8s | %12s |\n" "File" "Size" "oxc (native)"
printf "|%-27s|%10s|%14s|\n" \
  "$(printf -- '-%.0s' {1..27})" "$(printf -- '-%.0s' {1..10})" "$(printf -- '-%.0s' {1..14})"

for filepath in "${BENCH_FILES[@]}"; do
  name=$(basename "$filepath")
  size_bytes=$(wc -c < "$filepath" | tr -d ' ')
  if (( size_bytes > 1048576 )); then
    size="$(echo "scale=1; $size_bytes / 1048576" | bc) MB"
  else
    size="$(echo "scale=0; $size_bytes / 1024" | bc) KB"
  fi

  t_oxc=$(time_oxc "$filepath")
  printf "| %-25s | %8s | %10s ms |\n" "$name" "$size" "$t_oxc"
done
