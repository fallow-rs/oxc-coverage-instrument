#!/usr/bin/env node
// Generate Istanbul reference coverage maps for conformance testing.
//
// Run: node tests/conformance/generate-reference.mjs
//
// This instruments each fixture with istanbul-lib-instrument and writes
// the coverage map to tests/conformance/reference/<fixture>.json.
// The Rust conformance test reads these JSON files to compare.

import { createInstrumenter } from 'istanbul-lib-instrument';
import { readdirSync, readFileSync, writeFileSync, mkdirSync } from 'fs';
import { join, basename, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, 'fixtures');
const referenceDir = join(__dirname, 'reference');

mkdirSync(referenceDir, { recursive: true });

const fixtures = readdirSync(fixturesDir)
  .filter(f => f.endsWith('.js'))
  .sort();

console.log(`Generating Istanbul reference data for ${fixtures.length} fixtures...\n`);

let passed = 0;
let failed = 0;

for (const filename of fixtures) {
  const source = readFileSync(join(fixturesDir, filename), 'utf-8');
  const name = basename(filename, '.js');

  try {
    const instrumenter = createInstrumenter({
      compact: false,
      esModules: false,
      coverageVariable: '__coverage__',
    });
    instrumenter.instrumentSync(source, filename);
    const coverage = instrumenter.lastFileCoverage();

    // Extract the fields we want to compare
    const reference = {
      path: coverage.path,
      statements: Object.keys(coverage.statementMap).length,
      functions: Object.keys(coverage.fnMap).length,
      branches: Object.keys(coverage.branchMap).length,
      statementMap: coverage.statementMap,
      fnMap: Object.fromEntries(
        Object.entries(coverage.fnMap).map(([id, fn]) => [
          id,
          {
            name: fn.name,
            line: fn.line,
            decl: fn.decl,
            loc: fn.loc,
          },
        ])
      ),
      branchMap: Object.fromEntries(
        Object.entries(coverage.branchMap).map(([id, br]) => [
          id,
          {
            type: br.type,
            line: br.line,
            loc: br.loc,
            locations: br.locations,
            locationCount: br.locations.length,
          },
        ])
      ),
      s: coverage.s,
      f: coverage.f,
      b: coverage.b,
    };

    const outPath = join(referenceDir, `${name}.json`);
    writeFileSync(outPath, JSON.stringify(reference, null, 2));

    console.log(`  [OK] ${filename}: ${reference.statements}s ${reference.functions}f ${reference.branches}b`);
    passed++;
  } catch (err) {
    console.log(`  [FAIL] ${filename}: ${err.message}`);
    failed++;
  }
}

console.log(`\nDone: ${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
