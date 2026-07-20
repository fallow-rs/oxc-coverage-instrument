#!/usr/bin/env node
// Runtime compatibility checks copied from upstream istanbul-lib-instrument
// YAML specs. These cases cover statement-child containers and ignore hints,
// the area that has repeatedly produced drift.
//
// Source:
// https://github.com/istanbuljs/istanbuljs/tree/28ffdbc314596bdcb3007e85d30a62372602b262/packages/istanbul-lib-instrument/test/specs
// Upstream package license: BSD-3-Clause.

import assert from 'node:assert/strict';
import vm from 'node:vm';
import { instrument } from '../crates/oxc_coverage_instrument_napi/index.js';

const cases = [
  {
    sourceFile: 'with.yaml',
    name: 'with statement - no blocks',
    code: 'with (Math) output = abs(args[0]);\n',
    tests: [
      {
        args: [-1],
        out: 1,
        statements: { 0: 1, 1: 1 },
      },
    ],
  },
  {
    sourceFile: 'while.yaml',
    name: 'simple while',
    code: 'var x = args[0], i=0;\nwhile (i < x) i++;\noutput = i;\n',
    tests: [
      {
        name: 'covers loop once',
        args: [1],
        out: 1,
        statements: { 0: 1, 1: 1, 2: 1, 3: 1, 4: 1 },
      },
      {
        name: 'covers loop multiple times',
        args: [10],
        out: 10,
        statements: { 0: 1, 1: 1, 2: 1, 3: 10, 4: 1 },
      },
    ],
  },
  {
    sourceFile: 'for.yaml',
    name: 'simple for',
    code: 'var x = args[0], i, j = -10;\nfor (i =0; i < x; i++) j = i;\noutput = j;\n',
    tests: [
      {
        name: 'covers loop exactly once',
        args: [10],
        out: 9,
        statements: { 0: 1, 1: 1, 2: 1, 3: 10, 4: 1 },
      },
    ],
  },
  {
    sourceFile: 'do-while.yaml',
    name: 'simple do-while',
    code: 'var x = args[0], i=0;\ndo { i++; } while (i < x);\noutput = i;\n',
    tests: [
      {
        name: 'correct line coverage',
        args: [10],
        out: 10,
        statements: { 0: 1, 1: 1, 2: 1, 3: 10, 4: 1 },
      },
      {
        name: 'single entry into while',
        args: [-1],
        out: 1,
        statements: { 0: 1, 1: 1, 2: 1, 3: 1, 4: 1 },
      },
    ],
  },
  {
    sourceFile: 'statement-hints.yaml',
    name: 'ignore before case',
    code: 'switch (args[0]) {\n/* istanbul ignore next */\ncase "1": output = 2; break;\ndefault: output = 1;\n}\n',
    tests: [
      {
        args: ['2'],
        out: 1,
        branches: { 0: [1] },
        statements: { 0: 1, 1: 1 },
      },
    ],
  },
  {
    sourceFile: 'statement-hints.yaml',
    name: 'ignore class methods - es6',
    options: { ignoreClassMethods: ['dummy'] },
    code: 'class TestClass {\n    dummy(i) {return i;}\n    nonIgnored(i) {return i;}\n}\nvar testClass = new TestClass();\ntestClass.nonIgnored();\noutput = testClass.dummy(args[0]);\n',
    tests: [
      {
        name: 'ignores only specified es6 methods',
        args: [10],
        out: 10,
        functions: { 0: 1 },
        statements: { 0: 1, 1: 1, 2: 1, 3: 1 },
      },
    ],
  },
  {
    sourceFile: 'statement-hints.yaml',
    name: 'ignore class methods - es5',
    options: { ignoreClassMethods: ['testMethod'] },
    code: 'function TestClass() {}\nTestClass.prototype.testMethod = function testMethod(i) {\n    return i;\n};\nTestClass.prototype.goodMethod = function goodMethod(i) {return i;};\nvar testClass = new TestClass();\ntestClass.goodMethod();\noutput = testClass.testMethod(args[0]);\n',
    tests: [
      {
        name: 'ignores only specified es5 methods',
        args: [10],
        out: 10,
        functions: { 0: 1, 1: 1 },
        statements: { 0: 1, 1: 1, 2: 1, 3: 1, 4: 1, 5: 1 },
      },
    ],
  },
  {
    sourceFile: 'switch.yaml',
    name: 'switch with ignore next (issue 64)',
    code: `'use strict'\n\nconst test = foo => {\n  switch (foo) {\n    // the bug discussed in #64, seems to only\n    // crop up when a function is invoked before\n    // a single line comment, hence Date.now().\n    case 'ok':\n      return Date.now()\n\n    /* istanbul ignore next */\n    default:\n      throw new Error('nope')\n  }\n}\n\ntest('ok')\noutput = 'unknown'\n`,
    tests: [
      {
        name: '100% coverage',
        args: [],
        out: 'unknown',
        functions: { 0: 1 },
        branches: { 0: [1] },
        statements: { 0: 1, 1: 1, 2: 1, 3: 1, 4: 1 },
      },
    ],
  },
];

const numericKeys = (obj) => Object.keys(obj ?? {}).sort((a, b) => Number(a) - Number(b));

const compareCounts = (actual, expected, label, context) => {
  const expectedCounts = expected ?? {};
  assert.deepEqual(numericKeys(actual), numericKeys(expectedCounts), `${context}: ${label} keys`);
  for (const key of numericKeys(expectedCounts)) {
    // Values come from a vm context, so normalize arrays out of that realm.
    const value = JSON.parse(JSON.stringify(actual[key]));
    assert.deepEqual(value, expectedCounts[key], `${context}: ${label}[${key}]`);
  }
};

const runCase = (spec) => {
  const filename = `upstream/${spec.sourceFile}:${spec.name}.js`;
  const result = instrument(spec.code, filename, spec.options);
  for (const test of spec.tests) {
    const context = `${spec.sourceFile} / ${spec.name} / ${test.name ?? 'unnamed'}`;
    const sandbox = { args: test.args ?? [], output: undefined, console };
    sandbox.globalThis = sandbox;
    vm.runInNewContext(`${result.code}\n`, sandbox, { filename });

    assert.deepEqual(sandbox.output, test.out, `${context}: output`);
    const coverage = sandbox.__coverage__?.[filename];
    assert(coverage, `${context}: coverage object`);
    compareCounts(coverage.s, test.statements, 'statements', context);
    compareCounts(coverage.f, test.functions, 'functions', context);
    compareCounts(coverage.b, test.branches, 'branches', context);
  }
};

for (const spec of cases) {
  runCase(spec);
  console.log(`[OK] ${spec.sourceFile}: ${spec.name}`);
}

console.log(`PASS: ${cases.length} upstream istanbul-lib-instrument spec cases passed.`);
