#!/usr/bin/env node
// Byte-for-byte diff between oxc-coverage-instrument and istanbul-lib-instrument
// across the shared conformance fixtures.
//
// Asserts that the non-divergent parts of the coverage map match exactly:
//   - statementMap (all spans)
//   - fnMap (line, decl, loc; excluding documented name and method-decl-end divergences)
//   - branchMap (excluding intentional logical-assignment superset)
//   - counter arrays s, f, b
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

const istanbul = createInstrumenter({ esModules: true, produceSourceMap: false });
const oxc = createOxcInstrumenter({ coverageVariable: '__coverage__' });

// istanbul does not instrument `??=` / `||=` / `&&=`; we do (documented superset).
// Drop those oxc branch entries from the comparison so the rest of the shape can
// be asserted byte-for-byte. The branch-type check in the conformance-suite
// Rust tests continues to assert the wider structural match.
const isLogicalAssignmentBranch = (branch, source) => {
  if (branch.type !== 'binary-expr') return false;
  // A logical-assignment's loc starts at the assignment target and ends after
  // the right-hand side, so match the source slice literally for any of the
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

// Intentional divergences filtered from the diff, so the remaining shape can be
// asserted byte-for-byte. Each is documented in the README under "Differences
// from istanbul-lib-instrument":
//
//   - fn-name inference: oxc uses the JS-runtime inferred name (`f` for
//     `const f = function() {}`, `bar` for `class C { bar() {} }`);
//     istanbul emits `(anonymous_N)`. Filtered by dropping `name` below.
//
//   - method decl end span: oxc uses the full method key span (`bar`),
//     while istanbul truncates method decls to the first character (`b`).
//     Filtered by dropping only `fnMap[*].decl.end` for method syntax.
//
// Normalize both maps into a canonical shape before diffing. Istanbul adds
// `hash` and `_coverageSchema` fields which oxc doesn't emit, and its
// top-level ordering may differ. We compare only the fields that both
// instrumenters are contracted to populate, and zero out fields covered
// by intentional-divergence filters.
const isMethodDecl = (fn, source) => {
  const offset = lineColToOffset(source, fn.decl.start.line, fn.decl.start.column);
  if (source.slice(offset, offset + 'function'.length) === 'function') {
    return false;
  }
  const prefix = source.slice(Math.max(0, offset - 16), offset);
  return !/function\s*$/.test(prefix);
};

const normalizeFn = (fn, source) => ({
  line: fn.line,
  decl: {
    start: fn.decl.start,
    // Class/object method declarations intentionally use the full key span in
    // oxc, while istanbul truncates to the key's first character. Keep the
    // start pinned and filter only that known end-position divergence.
    end: isMethodDecl(fn, source) ? '<method-decl-end-filtered>' : fn.decl.end,
  },
  loc: fn.loc,
});

// The synthetic else arm of an `if` with no `else` clause anchors as a
// zero-width Location in oxc (the consequent's end), while istanbul emits
// an empty `{ start: {}, end: {} }` placeholder. The empty form crashes
// downstream `istanbul-reports` (`Cannot read properties of undefined`),
// so oxc intentionally diverges here. Collapse both shapes to a single
// sentinel before diffing so this divergence does not flood the report.
const isSyntheticBranchLocation = (loc) => {
  if (!loc || typeof loc !== 'object') return false;
  const start = loc.start;
  const end = loc.end;
  if (!start || !end) return false;
  // istanbul-lib-instrument records its placeholder as a Position object
  // whose `line`/`column` keys exist but are `undefined`. oxc anchors the
  // same slot as a real zero-width Location at the consequent's end.
  const istanbulEmpty =
    typeof start.line !== 'number' && typeof end.line !== 'number';
  const oxcZeroWidth =
    typeof start.line === 'number' &&
    typeof end.line === 'number' &&
    start.line === end.line &&
    start.column === end.column;
  return istanbulEmpty || oxcZeroWidth;
};

const normalizeBranch = (br) => ({
  type: br.type,
  line: br.line,
  loc: br.loc,
  locations: br.locations.map((loc) =>
    isSyntheticBranchLocation(loc) ? '<synthetic-arm-location-filtered>' : loc
  ),
});

const normalize = (cov, source) => ({
  statementMap: cov.statementMap,
  fnMap: Object.fromEntries(
    Object.entries(cov.fnMap).map(([id, f]) => [
      id,
      // `name` is dropped: inferred names are a documented divergence.
      normalizeFn(f, source),
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

const fixtures = readdirSync(fixturesDir).filter((f) => f.endsWith('.js')).sort();
let totalDiffs = 0;
let fixturesWithDiffs = 0;

for (const file of fixtures) {
  const source = readFileSync(join(fixturesDir, file), 'utf8');

  istanbul.instrumentSync(source, file);
  const iCov = normalize(istanbul.lastFileCoverage(), source);

  oxc.instrumentSync(source, file);
  const oCov = normalize(dropLogicalAssignment(oxc.lastFileCoverage(), source), source);

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
  console.log(`PASS: ${fixtures.length} fixtures match istanbul-lib-instrument after documented filters.`);
  process.exit(0);
} else {
  console.log(`FAIL: ${fixturesWithDiffs}/${fixtures.length} fixtures diverge (${totalDiffs} leaf diffs).`);
  console.log('If the divergence is intentional, add a targeted filter in scripts/istanbul-diff.mjs');
  console.log('and document it in README § "Differences from istanbul-lib-instrument".');
  process.exit(1);
}
