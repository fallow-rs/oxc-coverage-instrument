#!/usr/bin/env node
// Compare oxc-coverage-instrument output against istanbul-lib-instrument
// Run: node scripts/compare-istanbul.mjs

import { createInstrumenter } from 'istanbul-lib-instrument';
import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const fixturesDir = join(root, 'tests', 'fixtures');

// Simple JS fixtures for structural comparison
const simpleFixtures = [
  { name: 'simple_function', source: 'function add(a, b) { return a + b; }' },
  { name: 'arrow_expression', source: 'const double = (x) => x * 2;' },
  { name: 'arrow_block', source: 'const add = (a, b) => { return a + b; };' },
  { name: 'if_else', source: 'function f(x) { if (x > 0) { return 1; } else { return -1; } }' },
  { name: 'ternary', source: 'function f(x) { return x > 0 ? 1 : -1; }' },
  { name: 'switch', source: 'function f(x) { switch(x) { case 1: return "one"; case 2: return "two"; default: return "other"; } }' },
  { name: 'logical_and_or', source: 'function f(a, b) { return a && b || false; }' },
  { name: 'nullish_coalescing', source: 'function f(a, b) { return a ?? b; }' },
  { name: 'for_loop', source: 'function f(arr) { for (let i = 0; i < arr.length; i++) { console.log(arr[i]); } }' },
  { name: 'for_of', source: 'function f(arr) { for (const item of arr) { console.log(item); } }' },
  { name: 'while_loop', source: 'function f() { let i = 0; while (i < 10) { i++; } return i; }' },
  { name: 'do_while', source: 'function f() { let i = 0; do { i++; } while (i < 10); return i; }' },
  { name: 'class_methods', source: 'class Calc { add(a, b) { return a + b; } sub(a, b) { return a - b; } }' },
  { name: 'nested_if', source: 'function f(a, b) { if (a) { if (b) { return 1; } else { return 2; } } else { return 3; } }' },
  { name: 'multiple_functions', source: 'function a() { return 1; }\nfunction b() { return 2; }\nconst c = () => 3;\nconst d = function() { return 4; };' },
];

function instrumentWithIstanbul(source, filename) {
  const instrumenter = createInstrumenter({
    compact: false,
    esModules: false,
    coverageVariable: '__coverage__',
  });
  instrumenter.instrumentSync(source, filename);
  return instrumenter.lastFileCoverage();
}

console.log('=== Istanbul-lib-instrument Reference Output ===\n');

const details = [];

for (const { name, source } of simpleFixtures) {
  try {
    const cm = instrumentWithIstanbul(source, `${name}.js`);
    const detail = {
      name,
      statements: Object.keys(cm.statementMap).length,
      functions: Object.keys(cm.fnMap).length,
      branches: Object.keys(cm.branchMap).length,
      branchTypes: Object.values(cm.branchMap).map(b => b.type).sort(),
      functionNames: Object.values(cm.fnMap).map(f => f.name).sort(),
      hasSchema: '_coverageSchema' in cm,
      hasHash: 'hash' in cm,
      hasInputSourceMap: 'inputSourceMap' in cm,
      schemaValue: cm._coverageSchema ?? null,
    };
    details.push(detail);

    console.log(`--- ${name} ---`);
    console.log(`  Stmts:   ${detail.statements}`);
    console.log(`  Fns:     ${detail.functions} [${detail.functionNames.join(', ')}]`);
    console.log(`  Branches:${detail.branches} [${detail.branchTypes.join(', ')}]`);
    console.log(`  Schema:  ${detail.hasSchema} (${detail.schemaValue})`);
    console.log(`  Hash:    ${detail.hasHash}`);
    console.log(`  InpSM:   ${detail.hasInputSourceMap}`);
    console.log();
  } catch (err) {
    console.log(`--- ${name} --- ERROR: ${err.message}\n`);
  }
}

// Write reference output for Rust-side comparison
writeFileSync('/tmp/istanbul-reference-output.json', JSON.stringify(details, null, 2));
console.log('Reference output: /tmp/istanbul-reference-output.json');

// Full coverage maps
const fullMaps = {};
for (const { name, source } of simpleFixtures) {
  try {
    fullMaps[name] = instrumentWithIstanbul(source, `${name}.js`);
  } catch {}
}
writeFileSync('/tmp/istanbul-full-coverage-maps.json', JSON.stringify(fullMaps, null, 2));
console.log('Full maps: /tmp/istanbul-full-coverage-maps.json\n');

// Performance
console.log('=== Performance: Istanbul-lib-instrument ===\n');
const perfSource = readFileSync(join(fixturesDir, 'medium-app.js'), 'utf-8');
const iterations = 1000;
const start = performance.now();
for (let i = 0; i < iterations; i++) {
  instrumentWithIstanbul(perfSource, 'medium-app.js');
}
const elapsed = performance.now() - start;
const avgMs = elapsed / iterations;
const throughputMiBs = (perfSource.length / 1024 / 1024) / (avgMs / 1000);
console.log(`File: medium-app.js (${(perfSource.length / 1024).toFixed(1)} KB)`);
console.log(`${iterations} iterations in ${elapsed.toFixed(0)}ms`);
console.log(`Average: ${avgMs.toFixed(3)}ms per file`);
console.log(`Throughput: ${throughputMiBs.toFixed(1)} MiB/s`);
