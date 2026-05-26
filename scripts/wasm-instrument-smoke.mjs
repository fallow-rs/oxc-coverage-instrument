#!/usr/bin/env node

import { strict as assert } from 'node:assert';
import { instrument } from '../crates/oxc-coverage-instrument-napi/index.js';

const filename = 'wasm-smoke-fixture.js';
const source = Array.from({ length: 100 }, (_, index) => {
  const n = index + 1;
  return `export function f${n}(value) { return value === ${n} ? ${n} : ${n + 1}; }`;
}).join('\n');

const result = instrument(source, filename, { sourceMap: true });
const fileCoverage = JSON.parse(result.coverageMap);

assert.ok(result.code.includes('__coverage__'), 'expected instrumented coverage preamble');
assert.ok(typeof result.sourceMap === 'string', 'expected source map output');
assert.equal(fileCoverage.path, filename, `expected FileCoverage path ${filename}`);
assert.ok(Object.keys(fileCoverage.statementMap).length >= 100, 'expected statement coverage for the 100-line fixture');
assert.ok(Object.keys(fileCoverage.fnMap).length >= 100, 'expected function coverage for the 100-line fixture');

console.log(
  `wasm instrument smoke OK: ${Object.keys(fileCoverage.statementMap).length} statements, ${Object.keys(fileCoverage.fnMap).length} functions`,
);
