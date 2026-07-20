#!/usr/bin/env node
// Smoke suite for eager `composeInputSourceMap` (issues #105 / #106).
//
// Exercises every coverage-point kind (statement, function, branch: if / ternary
// / switch / logical / default-arg) sitting on a source-map-UNMAPPED line, plus
// positive controls on mapped lines. For each case it:
//   1. instruments with composeInputSourceMap: true and a partial input map,
//   2. asserts the emitted code has NO dangling counter (every `cov.s/f/b[id]`
//      referenced in the code exists in the baked coverage map) - the #106 bug,
//   3. asserts no surviving entry lands past the end of the original file - #105,
//   4. EXECUTES the instrumented code and asserts it does not throw and the
//      program logic still produces the right value.
//
// Run: node scripts/compose-eager-smoke.mjs   (exit 0 = all pass)
import { instrument, remapCoverageMap } from '../crates/oxc_coverage_instrument_napi/index.js';

let failures = 0;
const check = (name, cond, detail = '') => {
  if (cond) {
    console.log(`  [PASS] ${name}`);
  } else {
    failures++;
    console.log(`  [FAIL] ${name}${detail ? ` - ${detail}` : ''}`);
  }
};

// Build a v3 source map where generated lines 1..mappedLines map to original
// lines 1..mappedLines, and every later generated line is unmapped.
const partialMap = (mappedLines, totalLines, sourcesContent) => {
  const parts = [];
  for (let i = 1; i <= totalLines; i++) {
    parts.push(i === 1 ? 'AAAA' : i <= mappedLines ? 'AACA' : '');
  }
  return JSON.stringify({
    version: 3,
    sources: ['original.ts'],
    names: [],
    mappings: parts.join(';'),
    sourcesContent: [sourcesContent],
  });
};

const danglingRefs = (code, map) => {
  const out = {};
  for (const kind of ['s', 'f', 'b']) {
    const refs = [...new Set([...code.matchAll(new RegExp(`\\.${kind}\\[(\\d+)\\]`, 'g'))].map((m) => m[1]))];
    out[kind] = refs.filter((id) => !(id in map[{ s: 's', f: 'f', b: 'b' }[kind]]));
  }
  return out;
};

const runOnce = (name, { generated, mappedLines, originalLines, expectVar, expectVal, expectNoBranches }) => {
  const totalLines = generated.split('\n').length;
  const srcContent = generated.split('\n').slice(0, originalLines).join('\n') + '\n';
  const inputSourceMap = partialMap(mappedLines, totalLines, srcContent);
  const r = instrument(generated, 'mod.ts', {
    coverageVariable: '__c',
    inputSourceMap,
    composeInputSourceMap: true,
  });
  const map = JSON.parse(r.coverageMap);

  const d = danglingRefs(r.code, map);
  check(`${name}: no dangling counters`, d.s.length + d.f.length + d.b.length === 0, JSON.stringify(d));

  const pastEof = Object.values(map.statementMap).filter((l) => l.start.line > originalLines).length;
  check(`${name}: no statement past EOF`, pastEof === 0, `${pastEof} past line ${originalLines}`);

  if (expectNoBranches) {
    check(`${name}: unmapped branch not instrumented`, Object.keys(map.branchMap).length === 0);
  }

  const g = {};
  let threw = null;
  try {
    new Function('globalThis', r.code)(g);
  } catch (e) {
    threw = e;
  }
  check(`${name}: runs without throwing`, threw === null, threw && `${threw.constructor.name}: ${threw.message}`);
  if (expectVar) {
    check(`${name}: program logic correct`, g[expectVar] === expectVal, `${expectVar}=${g && g[expectVar]} (want ${expectVal})`);
  }
  return { r, map, g };
};

console.log('Eager composeInputSourceMap smoke (issues #105 / #106)\n');

// 1. Ternary (branch) on an unmapped line - the canonical #106 crash.
runOnce('ternary on unmapped line', {
  generated: ['const a = 1;', 'const b = 2;', "const w = a > 0 ? 'p' : 'n';", 'globalThis.r = a + b + w.length;', ''].join('\n'),
  mappedLines: 2,
  originalLines: 2,
  expectVar: 'r',
  expectVal: 1 + 2 + 1,
  expectNoBranches: true,
});

// 2. Function declaration on an unmapped line - fn counter must drop.
runOnce('function on unmapped line', {
  generated: ['const a = 1;', 'function dbl(x) { return x * 2; }', 'globalThis.r = a + dbl(20);', ''].join('\n'),
  mappedLines: 1,
  originalLines: 1,
  expectVar: 'r',
  expectVal: 1 + 40,
});

// 3. Switch on an unmapped line.
runOnce('switch on unmapped line', {
  generated: [
    'const a = 5;',
    'let out = 0;',
    'switch (a) { case 5: out = 50; break; default: out = -1; }',
    'globalThis.r = out;',
    '',
  ].join('\n'),
  mappedLines: 2,
  originalLines: 2,
  expectVar: 'r',
  expectVal: 50,
});

// 4. Logical expression on an unmapped line.
runOnce('logical && on unmapped line', {
  generated: ['const a = 1;', 'const ok = a > 0 && a < 10;', 'globalThis.r = ok ? 1 : 0;', ''].join('\n'),
  mappedLines: 1,
  originalLines: 1,
  expectVar: 'r',
  expectVal: 1,
});

// 5. Default-arg (branch) on an unmapped line.
runOnce('default-arg on unmapped line', {
  generated: ['const base = 7;', 'function f(x = 3) { return x; }', 'globalThis.r = base + f();', ''].join('\n'),
  mappedLines: 1,
  originalLines: 1,
  expectVar: 'r',
  expectVal: 7 + 3,
});

// 6. Vue-SFC-like: mapped script head + unmapped compiler-boilerplate tail
//    mixing statements, a function and a branch.
runOnce('mapped head + unmapped boilerplate tail', {
  generated: [
    'const count = 0;', // line 1 mapped (real script)
    'const label = "x";', // line 2 mapped (real script)
    'function _hmr() { return count; }', // line 3 unmapped boilerplate (fn)
    'const _flag = count > 0 ? 1 : 0;', // line 4 unmapped boilerplate (branch)
    '_hmr();', // line 5 unmapped boilerplate (stmt)
    'globalThis.r = count + label.length + _flag;', // line 6 unmapped
    '',
  ].join('\n'),
  mappedLines: 2,
  originalLines: 2,
  expectVar: 'r',
  expectVal: 0 + 1 + 0,
});

// 7. Positive control: fully-mapped file - branch/fn/stmt all kept, runs,
//    and counts are correct (both ternary arms exercised).
{
  const generated = "const pick = (x) => (x > 0 ? 'pos' : 'neg');\nglobalThis.r = pick(1) + pick(-1);\n";
  const { map, g, r } = runOnce('fully-mapped file (control)', {
    generated,
    mappedLines: 2,
    originalLines: 2,
    expectVar: 'r',
    expectVal: 'posneg',
  });
  check('control: branch kept with 2 arms', map.branchMap['0'] && map.branchMap['0'].locations.length === 2);
  const fc = g.__c['original.ts'];
  check('control: both arms counted once', fc && JSON.stringify(fc.b['0']) === JSON.stringify([1, 1]), JSON.stringify(fc && fc.b));

  // 8. eager == lazy(dropUnmapped) on a fully-mapped file (paths agree).
  const lazyPlain = JSON.parse(
    instrument(generated, 'mod.ts', { coverageVariable: '__c', inputSourceMap: partialMap(2, 2, generated) }).coverageMap,
  );
  const lazy = JSON.parse(remapCoverageMap(JSON.stringify({ 'mod.ts': lazyPlain }), { dropUnmapped: true }))['original.ts'];
  const eagerMap = JSON.parse(r.coverageMap);
  check(
    'control: eager statementMap == lazy(dropUnmapped)',
    JSON.stringify(eagerMap.statementMap) === JSON.stringify(lazy.statementMap),
  );
}

// ---------------------------------------------------------------------------
// Heavy sweep: diverse constructs x EVERY mapping cutoff (0..N lines mapped).
// At each cutoff the instrumented eager-composed output must (a) have no
// dangling counters, (b) place no surviving entry past the mapped original
// line count, (c) run without throwing, and (d) produce the SAME observable
// `globalThis.__r` as the uninstrumented source (instrumentation must never
// change behavior). Sweeping the cutoff across every line stresses the exact
// mapped/unmapped boundary where #105 / #106 live.
// ---------------------------------------------------------------------------
console.log('\nHeavy cutoff sweep (every mapping boundary x diverse constructs)\n');

const baseline = (code) => {
  const g = {};
  new Function('globalThis', code)(g);
  return g.__r;
};

const samples = {
  'nested functions + closure': `
const make = (n) => { const add = (x) => x + n; return add; };
const f = make(10);
globalThis.__r = f(5) + make(1)(1);
`,
  'class with methods + branch': `
class Box { constructor(v) { this.v = v; } big() { return this.v > 5 ? 'B' : 's'; } }
const b = new Box(9);
globalThis.__r = b.big() + new Box(1).big();
`,
  'for / while / for-of loops': `
let s = 0;
for (let i = 0; i < 3; i++) { s += i; }
let w = 0; while (w < 2) { w++; }
let p = 1; for (const x of [2, 3]) { p *= x; }
globalThis.__r = s + w + p;
`,
  'try / catch / finally': `
let out = '';
try { throw new Error('x'); } catch (e) { out += 'c'; } finally { out += 'f'; }
globalThis.__r = out;
`,
  'optional chaining + nullish': `
const o = { a: { b: 2 } };
const m = o?.a?.b ?? -1;
const n = o?.z?.b ?? -9;
globalThis.__r = m + n;
`,
  'switch + logical + ternary': `
const pick = (x) => { switch (x) { case 1: return 'a'; default: return 'b'; } };
const g = (1 > 0) && (2 > 1);
const t = g ? pick(1) : pick(9);
globalThis.__r = t + pick(9);
`,
  'logical assignment + default args': `
function f(a, b = 3) { let c; c ||= a + b; return c; }
let z = 0; z ??= 7;
globalThis.__r = f(1) + f(1, 1) + z;
`,
  'async + await (sync-resolved)': `
const run = async () => { const v = await Promise.resolve(4); return v * 2; };
globalThis.__p = run();
globalThis.__r = 'started';
`,
  'generator function': `
function* gen() { yield 1; yield 2; }
let t = 0; for (const v of gen()) { t += v; }
globalThis.__r = t;
`,
};

let sweepCases = 0;
for (const [name, raw] of Object.entries(samples)) {
  const code = raw.trimStart();
  const totalLines = code.split('\n').length;
  let want;
  try {
    want = baseline(code);
  } catch (e) {
    check(`sweep ${name}: baseline runs`, false, `${e.constructor.name}: ${e.message}`);
    continue;
  }
  let allOk = true;
  let detail = '';
  for (let cutoff = 0; cutoff <= totalLines; cutoff++) {
    const srcContent = code.split('\n').slice(0, Math.max(cutoff, 1)).join('\n') + '\n';
    const inputSourceMap = partialMap(cutoff, totalLines, srcContent);
    const r = instrument(code, 'mod.ts', { coverageVariable: '__c', inputSourceMap, composeInputSourceMap: true });
    const map = JSON.parse(r.coverageMap);
    const d = danglingRefs(r.code, map);
    if (d.s.length + d.f.length + d.b.length > 0) { allOk = false; detail = `cutoff ${cutoff} dangling ${JSON.stringify(d)}`; break; }
    const pastEof = Object.values(map.statementMap).filter((l) => l.start.line > cutoff).length;
    if (cutoff > 0 && pastEof > 0) { allOk = false; detail = `cutoff ${cutoff}: ${pastEof} past EOF`; break; }
    const g = {};
    try {
      new Function('globalThis', r.code)(g);
    } catch (e) {
      allOk = false; detail = `cutoff ${cutoff} threw ${e.constructor.name}: ${e.message}`; break;
    }
    if (g.__r !== want) { allOk = false; detail = `cutoff ${cutoff}: result ${g.__r} != baseline ${want}`; break; }
    sweepCases++;
  }
  check(`sweep ${name} (${totalLines + 1} cutoffs)`, allOk, detail);
}
console.log(`  (swept ${sweepCases} instrument-and-execute cases)`);

console.log(`\n${failures === 0 ? 'SMOKE: PASS' : `SMOKE: FAIL (${failures})`}`);
process.exit(failures === 0 ? 0 : 1);
