#!/usr/bin/env node
// Byte-for-byte diff between oxc-coverage-instrument and istanbul-lib-instrument
// across the shared conformance fixtures.
//
// Asserts that the coverage map matches exactly under the Istanbul profile:
// statementMap, fnMap, branchMap, and the s/f/b counter arrays.
//
// Exits non-zero on any diff. Runs in CI on every PR so span-level regressions
// that count-only tests miss fail fast.
//
// Usage: node scripts/istanbul-diff.mjs

import { createInstrumenter } from 'istanbul-lib-instrument';
import { readdirSync, readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createOxcInstrumenter } from '../crates/oxc_coverage_instrument_napi/vitest.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, '..', 'crates', 'oxc_coverage_instrument', 'tests', 'conformance', 'fixtures');

const istanbulJs = createInstrumenter({ esModules: true, produceSourceMap: false });
const istanbulTs = createInstrumenter({
  esModules: true,
  produceSourceMap: false,
  parserPlugins: ['typescript'],
});
const oxc = createOxcInstrumenter({ coverageVariable: '__coverage__', compat: 'istanbul' });

// Normalize both maps into a canonical shape before diffing. Istanbul adds
// `hash` and `_coverageSchema` fields which oxc doesn't emit, and its
// top-level ordering may differ. We compare only the fields that both
// instrumenters are contracted to populate.
const normalizeFn = (fn) => ({
  name: fn.name,
  line: fn.line,
  decl: fn.decl,
  loc: fn.loc,
});

const normalizeBranch = (br) => ({
  type: br.type,
  line: br.line,
  loc: br.loc,
  locations: br.locations,
});

const normalize = (cov) => ({
  statementMap: cov.statementMap,
  fnMap: Object.fromEntries(
    Object.entries(cov.fnMap).map(([id, f]) => [
      id,
      normalizeFn(f),
    ])
  ),
  branchMap: Object.fromEntries(
    Object.entries(cov.branchMap).map(([id, br]) => [id, normalizeBranch(br)])
  ),
  s: cov.s,
  f: cov.f,
  b: cov.b,
});

const diffKeys = (a, b, path = '') => {
  const diffs = [];
  if (JSON.stringify(a) === JSON.stringify(b)) return diffs;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) {
    diffs.push({ path, istanbul: a, oxc: b });
    return diffs;
  }
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  for (const k of keys) {
    diffs.push(...diffKeys(a[k], b[k], path ? `${path}.${k}` : k));
  }
  return diffs;
};

const fixtures = readdirSync(fixturesDir).filter((file) => /\.(?:js|ts)$/.test(file)).sort();
const cases = fixtures.map((file) => ({
  file,
  source: readFileSync(join(fixturesDir, file), 'utf8'),
}));
cases.push(
  {
    file: 'profile-logical-and-optional.js',
    source: 'let value; value ??= fallback; const nested = object?.property;',
  },
  {
    file: 'profile-names-methods-and-synthetic-else.js',
    source: 'export default function () {} const object = { execute() {} }; if (value) work();',
  },
);
let totalDiffs = 0;
let fixturesWithDiffs = 0;

for (const { file, source } of cases) {
  const istanbul = file.endsWith('.ts') ? istanbulTs : istanbulJs;
  istanbul.instrumentSync(source, file);
  const iCov = normalize(istanbul.lastFileCoverage());

  oxc.instrumentSync(source, file);
  const oCov = normalize(oxc.lastFileCoverage());

  const diffs = diffKeys(iCov, oCov);
  if (diffs.length === 0) {
    console.log(`[OK]   ${file}`);
    continue;
  }
  fixturesWithDiffs++;
  totalDiffs += diffs.length;
  console.log(`[DIFF] ${file}: ${diffs.length} leaf diff(s):`);
  for (const d of diffs.slice(0, 5)) {
    console.log(`  ${d.path}: istanbul=${JSON.stringify(d.istanbul)} oxc=${JSON.stringify(d.oxc)}`);
  }
  if (diffs.length > 5) console.log(`  … and ${diffs.length - 5} more`);
}

console.log('');
if (fixturesWithDiffs === 0) {
  console.log(`PASS: ${cases.length} cases match istanbul-lib-instrument without filters.`);
  process.exit(0);
} else {
  console.log(`FAIL: ${fixturesWithDiffs}/${cases.length} cases diverge (${totalDiffs} leaf diffs).`);
  console.log('The Istanbul profile must stay byte-identical on all compared fields.');
  process.exit(1);
}
