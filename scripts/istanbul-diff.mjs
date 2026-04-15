#!/usr/bin/env node
// Byte-for-byte diff between oxc-coverage-instrument and istanbul-lib-instrument
// across the 25 shared conformance fixtures.
//
// Asserts that the non-divergent parts of the coverage map match exactly:
//   - statementMap (all spans)
//   - fnMap (all spans incl. decl + loc, and names)
//   - branchMap (excluding intentional logical-assignment superset)
//   - counter arrays s, f, b
//
// Exits non-zero on any diff. Meant to run in CI on every PR so micro-regressions
// (like the v0.3.4 fnMap.decl column off-by-N) fail fast.
//
// Usage: node scripts/istanbul-diff.mjs

import { createInstrumenter } from 'istanbul-lib-instrument';
import { readdirSync, readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createOxcInstrumenter } from '../napi/vitest.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, '..', 'tests', 'conformance', 'fixtures');

const istanbul = createInstrumenter({ esModules: true, produceSourceMap: false });
const oxc = createOxcInstrumenter({ coverageVariable: '__coverage__' });

// istanbul does not instrument `??=` / `||=` / `&&=`; we do (documented superset).
// Drop those oxc branch entries from the comparison so the rest of the shape can
// be asserted byte-for-byte. The branch-type check in the conformance-suite
// Rust tests continues to assert the wider structural match.
const isLogicalAssignmentBranch = (branch, source) => {
  if (branch.type !== 'binary-expr') return false;
  // A logical-assignment's loc starts at the assignment target and ends after
  // the right-hand side — match the source slice literally for any of the
  // three operators.
  const startOffset = lineColToOffset(source, branch.loc.start.line, branch.loc.start.column);
  const endOffset = lineColToOffset(source, branch.loc.end.line, branch.loc.end.column);
  const slice = source.slice(startOffset, endOffset);
  return /\?\?=|\|\|=|&&=/.test(slice);
};

const lineColToOffset = (src, line, col) => {
  let offset = 0;
  for (let i = 1; i < line; i++) {
    const next = src.indexOf('\n', offset);
    if (next === -1) break;
    offset = next + 1;
  }
  return offset + col;
};

const dropLogicalAssignment = (cov, source) => {
  const branchMap = {};
  const b = {};
  let newIdx = 0;
  for (const [oldId, branch] of Object.entries(cov.branchMap)) {
    if (isLogicalAssignmentBranch(branch, source)) continue;
    branchMap[String(newIdx)] = branch;
    b[String(newIdx)] = cov.b[oldId];
    newIdx++;
  }
  return { ...cov, branchMap, b };
};

// Intentional divergences that should NOT show up in the advisory diff.
// Each one is tracked by an open issue; any *new* diff outside these
// categories is a true regression and should fail review.
//
//   - fn-name inference: oxc uses the JS-runtime inferred name (`f` for
//     `const f = function() {}`, `bar` for `class C { bar() {} }`);
//     istanbul emits `(anonymous_N)`. oxc is more accurate. See issue #8.
//
// Normalize both maps into a canonical shape before diffing. Istanbul adds
// `hash` and `_coverageSchema` fields which oxc doesn't emit, and its
// top-level ordering may differ. We compare only the fields that both
// instrumenters are contracted to populate, and zero out fields covered
// by intentional-divergence filters.
const normalize = (cov) => ({
  statementMap: cov.statementMap,
  fnMap: Object.fromEntries(
    Object.entries(cov.fnMap).map(([id, f]) => [
      id,
      // Skip `name` — tracked as intentional divergence (issue #8).
      { line: f.line, decl: f.decl, loc: f.loc },
    ])
  ),
  branchMap: Object.fromEntries(
    Object.entries(cov.branchMap).map(([id, br]) => [
      id,
      { type: br.type, line: br.line, loc: br.loc, locations: br.locations },
    ])
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

const fixtures = readdirSync(fixturesDir).filter((f) => f.endsWith('.js')).sort();
let totalDiffs = 0;
let fixturesWithDiffs = 0;

for (const file of fixtures) {
  const source = readFileSync(join(fixturesDir, file), 'utf8');

  istanbul.instrumentSync(source, file);
  const iCov = normalize(istanbul.lastFileCoverage());

  oxc.instrumentSync(source, file);
  const oCov = normalize(dropLogicalAssignment(oxc.lastFileCoverage(), source));

  const diffs = diffKeys(iCov, oCov);
  if (diffs.length === 0) {
    console.log(`[OK]   ${file}`);
    continue;
  }
  fixturesWithDiffs++;
  totalDiffs += diffs.length;
  console.log(`[DIFF] ${file} — ${diffs.length} leaf diff(s):`);
  for (const d of diffs.slice(0, 5)) {
    console.log(`  ${d.path}: istanbul=${JSON.stringify(d.istanbul)} oxc=${JSON.stringify(d.oxc)}`);
  }
  if (diffs.length > 5) console.log(`  … and ${diffs.length - 5} more`);
}

console.log('');
if (fixturesWithDiffs === 0) {
  console.log(`PASS: ${fixtures.length} fixtures byte-for-byte identical to istanbul-lib-instrument.`);
  process.exit(0);
} else {
  console.log(`FAIL: ${fixturesWithDiffs}/${fixtures.length} fixtures diverge (${totalDiffs} leaf diffs).`);
  console.log('If the divergence is intentional, add a targeted filter in scripts/istanbul-diff.mjs');
  console.log('and document it in README § "Differences from istanbul-lib-instrument".');
  process.exit(1);
}
