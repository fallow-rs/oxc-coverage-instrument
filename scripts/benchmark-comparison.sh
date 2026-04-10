#!/usr/bin/env bash
# Benchmark oxc-coverage-instrument against istanbul-lib-instrument,
# babel-plugin-istanbul, and swc-plugin-coverage-instrument on real-world
# JavaScript libraries.
#
# Usage:
#   ./scripts/benchmark-comparison.sh            # run full benchmark
#   ./scripts/benchmark-comparison.sh --quick     # react + lodash only
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
#                       running inside SWC's WASM sandbox — not a native Rust
#                       comparison (includes WASM + serialisation overhead)
#   - "istanbul-lib" = istanbul-lib-instrument standalone (parse + instrument)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BENCH_DIR="${ROOT_DIR}/.bench-tmp"
FILES_DIR="${BENCH_DIR}/files"
OXC="${ROOT_DIR}/target/release/oxc-coverage-instrument"
NAPI_DIR="${ROOT_DIR}/napi"
RUNS=5
QUICK=false

if [[ "${1:-}" == "--quick" ]]; then
  QUICK=true
  RUNS=3
fi

# ---------- helpers ----------

setup_files() {
  mkdir -p "$FILES_DIR"
  local urls=(
    "https://cdn.jsdelivr.net/npm/react@18.2.0/umd/react.development.js|react.development.js"
    "https://cdn.jsdelivr.net/npm/lodash@4.17.21/lodash.js|lodash.js"
  )
  if [[ "$QUICK" == false ]]; then
    urls+=(
      "https://cdn.jsdelivr.net/npm/vue@3.3.4/dist/vue.global.js|vue.global.js"
      "https://cdn.jsdelivr.net/npm/d3@7.8.5/dist/d3.js|d3.js"
      "https://cdn.jsdelivr.net/npm/three@0.155.0/build/three.js|three.js"
    )
  fi
  for entry in "${urls[@]}"; do
    local url="${entry%%|*}"
    local name="${entry##*|}"
    if [[ ! -f "${FILES_DIR}/${name}" ]]; then
      echo "  downloading ${name}..." >&2
      curl -sL "$url" -o "${FILES_DIR}/${name}"
    fi
  done
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
  if [[ ! -f "$OXC" ]] || [[ "$ROOT_DIR/src/transform.rs" -nt "$OXC" ]]; then
    echo "  building oxc-coverage-instrument CLI (release)..." >&2
    cargo build --release -p oxc-coverage-instrument-cli --manifest-path "$ROOT_DIR/Cargo.toml" 2>&1 | tail -1
  fi
}

build_napi() {
  if [[ ! -f "${NAPI_DIR}/coverage-instrument.darwin-arm64.node" ]] && \
     [[ ! -f "${NAPI_DIR}/coverage-instrument.darwin-x64.node" ]] && \
     [[ ! -f "${NAPI_DIR}/coverage-instrument.linux-x64-gnu.node" ]]; then
    echo "  building napi bindings (release)..." >&2
    (cd "$NAPI_DIR" && npm run build 2>&1 | tail -1)
  fi
}

# Precise timing via python (sub-ms accuracy, includes process startup for CLI)
time_oxc() {
  local file="$1"
  python3 -c "
import subprocess, time
best = 1e9
for _ in range($RUNS):
    start = time.perf_counter()
    subprocess.run(['$OXC', '$file'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    elapsed = (time.perf_counter() - start) * 1000
    if elapsed < best: best = elapsed
print(f'{best:.1f}')
"
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
  cat > "${BENCH_DIR}/napi-bench.js" << EOF
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
echo "Running benchmarks ($RUNS runs each, median)..." >&2
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

for filepath in "$FILES_DIR"/*.js; do
  name=$(basename "$filepath")
  size_bytes=$(wc -c < "$filepath" | tr -d ' ')
  if (( size_bytes > 1048576 )); then
    size="$(echo "scale=1; $size_bytes / 1048576" | bc) MB"
  else
    size="$(echo "scale=0; $size_bytes / 1024" | bc) KB"
  fi

  t_napi=$(node "${BENCH_DIR}/napi-bench.js" "$filepath" "$RUNS")
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

for filepath in "$FILES_DIR"/*.js; do
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
