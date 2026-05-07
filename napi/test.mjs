// Quick test for the napi bindings
import { instrument } from './index.js';
import { strict as assert } from 'node:assert';

console.log('Testing oxc-coverage-instrument napi bindings...\n');

function runInstrumented(result, filename, callExpression) {
  const sharedGlobal = {};
  new Function('globalThis', `${result.code}\n${callExpression}`)(sharedGlobal);
  return sharedGlobal.__coverage__[filename];
}

// Test 1: Basic instrumentation
{
  const result = instrument('function add(a, b) { return a + b; }', 'test.js');
  assert(result.code.includes('cov_'), 'Code should contain coverage counter');
  const coverageMap = JSON.parse(result.coverageMap);
  assert.equal(coverageMap.path, 'test.js');
  assert.equal(Object.keys(coverageMap.fnMap).length, 1);
  assert.equal(coverageMap.fnMap['0'].name, 'add');
  console.log('  [PASS] Basic instrumentation');
}

// Test 2: With options
{
  const result = instrument('const x = 1;', 'test.js', {
    coverageVariable: '__custom_cov__',
  });
  assert(result.code.includes('__custom_cov__'), 'Should use custom coverage variable');
  console.log('  [PASS] Custom coverage variable');
}

// Test 3: Source map
{
  const result = instrument('function f() { return 1; }', 'test.js', {
    sourceMap: true,
  });
  assert(result.sourceMap, 'Should have source map');
  const sm = JSON.parse(result.sourceMap);
  assert.equal(sm.version, 3);
  console.log('  [PASS] Source map generation');
}

// Test 4: TypeScript
{
  const result = instrument(
    'function add(a: number, b: number): number { return a + b; }',
    'test.ts',
  );
  const coverageMap = JSON.parse(result.coverageMap);
  assert.equal(coverageMap.fnMap['0'].name, 'add');
  console.log('  [PASS] TypeScript support');
}

// Test 5: Error handling
{
  try {
    instrument('function {{{', 'bad.js');
    assert.fail('Should have thrown');
  } catch (e) {
    assert(e.message.includes('parse error'), `Expected parse error, got: ${e.message}`);
    console.log('  [PASS] Parse error handling');
  }
}

// Test 6: Performance
{
  const source = 'function f(x) { if (x > 0) { return x; } else { return -x; } }\n'.repeat(100);
  const start = performance.now();
  const iterations = 1000;
  for (let i = 0; i < iterations; i++) {
    instrument(source, 'perf.js');
  }
  const elapsed = performance.now() - start;
  const avgMs = elapsed / iterations;
  const throughput = (source.length / 1024 / 1024) / (avgMs / 1000);
  console.log(`  [PASS] Performance: ${avgMs.toFixed(3)}ms avg, ${throughput.toFixed(1)} MiB/s`);
}

// Test 7: Istanbul format compliance
{
  const result = instrument('function f() { if (true) { return 1; } else { return 0; } }', 'test.js');
  const cm = JSON.parse(result.coverageMap);
  assert(cm.statementMap, 'Must have statementMap');
  assert(cm.fnMap, 'Must have fnMap');
  assert(cm.branchMap, 'Must have branchMap');
  assert(cm.s, 'Must have s');
  assert(cm.f, 'Must have f');
  assert(cm.b, 'Must have b');
  assert.equal(cm.branchMap['0'].type, 'if');
  assert.equal(cm.branchMap['0'].locations.length, 2);
  console.log('  [PASS] Istanbul format compliance');
}

// Test 8: Default-arg branches increment at runtime
{
  const result = instrument('function f(x = 1) { return x; }\nconst obj = {};\nconst { y = 2 } = obj;\nf();', 'default-arg.js');
  const context = { globalThis: {} };
  const runner = new Function('globalThis', `${result.code}\nreturn globalThis.__coverage__;`);
  const coverage = runner(context.globalThis);
  assert.equal(coverage['default-arg.js'].b['0'][0], 1, 'Default parameter should hit branch counter');
  assert.equal(coverage['default-arg.js'].b['1'][0], 1, 'Destructuring default should hit branch counter');
  console.log('  [PASS] Default-arg runtime branch counters');
}

// Test 9: Same path with changed shape refreshes stale coverage data
{
  const first = instrument('function f() { return 1; }\nf();', 'same.js');
  const second = instrument('function f() { if (true) { return 1; } return 0; }\nf();', 'same.js');
  const sharedGlobal = {};
  new Function('globalThis', first.code)(sharedGlobal);
  new Function('globalThis', second.code)(sharedGlobal);
  assert.ok(sharedGlobal.__coverage__['same.js'].b['0'], 'Updated instrumentation should refresh branch data for the same path');
  console.log('  [PASS] Stale coverage refresh by hash');
}

// Test 10: Issue regressions that require runtime counter execution
{
  const noElse = instrument(
    `function f(x) {
      if (!x.roles) return x
      return x.roles.map(r => r)
    }`,
    'issue-19.js',
  );
  const noElseCoverage = runInstrumented(
    noElse,
    'issue-19.js',
    "eval('f')({ roles: ['a'] }); eval('f')({});",
  );
  assert.deepEqual(noElseCoverage.b['0'], [1, 1], 'if without else should hit both branch counters');
  assert.deepEqual(
    JSON.parse(noElse.coverageMap).branchMap['0'].locations[1],
    { start: {}, end: {} },
    'if without else should use Istanbul-style unknown alternate location',
  );

  const objectMethod = instrument(
    `const obj = {
      /* v8 ignore next -- @preserve */
      method(x) {
        const y = x.foo
        if (y) {
          y.bar = 1
        }
      },
    }`,
    'issue-20.js',
  );
  const objectMethodMap = JSON.parse(objectMethod.coverageMap);
  assert.equal(Object.keys(objectMethodMap.fnMap).length, 0, 'ignored object method should not add fnMap entries');
  assert.equal(Object.keys(objectMethodMap.branchMap).length, 0, 'ignored object method should not add branchMap entries');

  const ternary = instrument(
    `function f(x) {
      return {
        ...x,
        ...(x.set
          ? { a: 1 }
          : /* v8 ignore next -- @preserve */
            {}),
      }
    }`,
    'issue-21.js',
  );
  const ternaryMap = JSON.parse(ternary.coverageMap);
  assert.equal(Object.keys(ternaryMap.fnMap).length, 1, 'enclosing function should still be tracked');
  assert.equal(Object.keys(ternaryMap.branchMap).length, 0, 'one-path ternary branch stubs should be pruned');
  assert.equal(Object.keys(ternaryMap.b).length, 0, 'branch hit arrays should match pruned branchMap');

  const fullyIgnoredTernary = instrument(
    'function f(x) { return x ? /* v8 ignore next */ 1 : /* v8 ignore next */ 2; }',
    'fully-ignored-ternary.js',
  );
  assert.equal(Object.keys(JSON.parse(fullyIgnoredTernary.coverageMap).branchMap).length, 0, 'empty branches should be pruned');

  const logicalLeaf = instrument(
    'function f(a, b) { return a && /* v8 ignore next */ b; }',
    'logical-leaf.js',
  );
  assert.equal(
    Object.keys(JSON.parse(logicalLeaf.coverageMap).branchMap).length,
    0,
    'one-path logical branch stubs should be pruned',
  );

  for (const [label, source, path] of [
    ['nullish rhs', 'function f(x) { return x ?? /* v8 ignore next -- @preserve */ [] }', 'issue-27-nullish.js'],
    ['or rhs', 'function f(x) { return x || /* v8 ignore next -- @preserve */ true }', 'issue-27-or.js'],
    ['and rhs', 'function f(x) { return x && /* v8 ignore next -- @preserve */ true }', 'issue-27-and.js'],
    ['conditional rhs', 'function f(x) { return x ? 1 : /* v8 ignore next -- @preserve */ 2 }', 'issue-27-cond.js'],
    [
      'jsx attribute',
      `function f(pass) {
        return <Tag
          /* v8 ignore next -- @preserve */
          text={pass ? 'Pass' : 'Fail'}
        />
      }`,
      'issue-28.tsx',
    ],
    [
      'jsx child',
      `function f(x) {
        return <div>
          {/* v8 ignore next -- @preserve */}
          {x ? <a/> : <b/>}
        </div>
      }`,
      'issue-29.tsx',
    ],
  ]) {
    const map = JSON.parse(instrument(source, path).coverageMap);
    assert.equal(Object.keys(map.branchMap).length, 0, `${label} should not leave branch entries`);
  }

  const cachedSetup = instrument('function f() { return 1; }\nf();', 'issue-34.js');
  const covName = cachedSetup.code.match(/var (cov_[a-f0-9]+)/)?.[1];
  assert(covName, 'instrumented code should declare a coverage binding');
  assert(cachedSetup.code.includes('return actualCoverage; })();'), 'coverage setup should be invoked once');
  assert(!cachedSetup.code.includes(`${covName}().`), 'counter sites should use cached coverage data');
  const issue34Coverage = runInstrumented(cachedSetup, 'issue-34.js', "eval('f')();");
  assert.equal(issue34Coverage.f['0'], 2, 'cached coverage object should still record runtime function hits');
  assert.equal(issue34Coverage.s['0'], 2, 'cached coverage object should still record runtime statement hits');

  const classMethod = instrument(
    `class C {
      /* istanbul ignore next */
      render(x) {
        if (x) { return 1; }
        return 2;
      }

      update() { return 3; }
    }`,
    'class-method-ignore.js',
  );
  const classMethodMap = JSON.parse(classMethod.coverageMap);
  assert.equal(
    Object.keys(classMethodMap.fnMap).length,
    1,
    'ignored class method should not add a fnMap entry',
  );
  assert.equal(
    Object.keys(classMethodMap.branchMap).length,
    0,
    'ignored class method body should not add branches',
  );
  assert.equal(
    Object.keys(classMethodMap.statementMap).length,
    1,
    'ignored class method body should not add statements',
  );

  const ignoreClassMethods = instrument(
    `function TestClass() {}
    TestClass.prototype.testMethod = function testMethod(i) { return i; };
    TestClass.prototype.goodMethod = function goodMethod(i) { return i; };
    var testClass = new TestClass();
    testClass.goodMethod();
    testClass.testMethod(1);`,
    'ignore-class-methods.js',
    { ignoreClassMethods: ['testMethod'] },
  );
  const ignoreClassMethodsMap = JSON.parse(ignoreClassMethods.coverageMap);
  assert.deepEqual(
    Object.values(ignoreClassMethodsMap.fnMap).map((entry) => entry.name),
    ['TestClass', 'goodMethod'],
    'ignoreClassMethods should skip matching named function expressions',
  );
  assert.deepEqual(
    Object.values(ignoreClassMethodsMap.statementMap).map((loc) => loc.start.line),
    [2, 3, 3, 4, 5, 6],
    'ignoreClassMethods should skip matching function expression bodies',
  );

  const ignoreIf = instrument(
    `function f(x) {
      /* istanbul ignore if */
      if (x) return 1;
      return 2;
    }`,
    'ignore-if.js',
  );
  const ignoreIfMap = JSON.parse(ignoreIf.coverageMap);
  assert.equal(ignoreIfMap.branchMap['0'].locations.length, 1, 'ignore if should keep only the alternate path');
  assert.equal(
    Object.keys(ignoreIfMap.statementMap).length,
    2,
    'ignore if should skip statement counters in the ignored arm',
  );

  const ignoredSwitchCase = instrument(
    `function f(item) {
      switch (item.type) {
        case 'html': return 'a'
        /* istanbul ignore next */
        default: return 'b'
      }
    }`,
    'ignored-switch-case.js',
  );
  const ignoredSwitchCaseMap = JSON.parse(ignoredSwitchCase.coverageMap);
  assert.equal(
    ignoredSwitchCaseMap.branchMap['0'].locations.length,
    1,
    'ignored switch case should be pruned from branch paths',
  );
  assert.deepEqual(
    Object.values(ignoredSwitchCaseMap.statementMap).map((loc) => loc.start.line),
    [2, 3],
    'ignored switch case statements should be pruned',
  );

  console.log('  [PASS] Issue regression runtime parity');
}

// Test 11: No-block loop bodies increment their own statement counters
{
  const cases = [
    {
      name: 'while',
      filename: 'loop-while.js',
      source: 'function f() { let i = 0; while (i < 3) i++; return i; }',
      call: "eval('f')();",
    },
    {
      name: 'for',
      filename: 'loop-for.js',
      source: 'function f() { let total = 0; for (let i = 0; i < 3; i++) total++; return total; }',
      call: "eval('f')();",
    },
    {
      name: 'for-of',
      filename: 'loop-for-of.js',
      source: 'function f(items) { let total = 0; for (const x of items) total += x; return total; }',
      call: "eval('f')([1, 2, 3]);",
    },
    {
      name: 'for-in',
      filename: 'loop-for-in.js',
      source: 'function f(obj) { let total = 0; for (const k in obj) total++; return total; }',
      call: "eval('f')({ a: 1, b: 2, c: 3 });",
    },
    {
      name: 'do-while',
      filename: 'loop-do-while.js',
      source: 'function f() { let i = 0; do i++; while (i < 3); return i; }',
      call: "eval('f')();",
    },
  ];

  for (const item of cases) {
    const result = instrument(item.source, item.filename);
    const map = JSON.parse(result.coverageMap);
    const emittedStatementCounters = result.code.match(/\+\+cov_[a-f0-9]+\.s\[\d+\]/g) ?? [];
    assert.equal(
      emittedStatementCounters.length,
      Object.keys(map.statementMap).length,
      `${item.name} should emit every statementMap counter`,
    );

    const coverage = runInstrumented(result, item.filename, item.call);
    assert(
      Object.entries(coverage.s).every(([, hits]) => hits > 0),
      `${item.name} should hit every statement counter when the body runs`,
    );
  }

  console.log('  [PASS] No-block loop body statement counters');
}

// Test 12: Other no-block statement-child containers increment body counters
{
  const cases = [
    {
      name: 'with',
      filename: 'with-no-block.js',
      source: 'function f(obj) { with (obj) x++; return obj.x; }',
      call: "eval('f')({ x: 0 });",
    },
    {
      name: 'label',
      filename: 'label-no-block.js',
      source: 'function f() { let n = 0; label: n++; return n; }',
      call: "eval('f')();",
    },
    {
      name: 'loop-label',
      filename: 'loop-label-no-block.js',
      source: 'function f() { let n = 0; while (n < 3) label: n++; return n; }',
      call: "eval('f')();",
    },
    {
      name: 'label-loop',
      filename: 'label-loop.js',
      source: 'function f() { let n = 0; label: while (n < 3) { n++; continue label; } return n; }',
      call: "eval('f')();",
    },
  ];

  for (const item of cases) {
    const result = instrument(item.source, item.filename);
    const map = JSON.parse(result.coverageMap);
    const emittedStatementCounters = result.code.match(/\+\+cov_[a-f0-9]+\.s\[\d+\]/g) ?? [];
    assert.equal(
      emittedStatementCounters.length,
      Object.keys(map.statementMap).length,
      `${item.name} should emit every statementMap counter`,
    );

    const coverage = runInstrumented(result, item.filename, item.call);
    assert(
      Object.entries(coverage.s).every(([, hits]) => hits > 0),
      `${item.name} should hit every statement counter when the body runs`,
    );
  }

  console.log('  [PASS] No-block statement-child body counters');
}

// Test 13: reportLogic adds bT (truthy-value tracking) for logical operands
{
  const source = 'function f(a, b) { return a && b; }';

  const off = instrument(source, 'logic.js');
  const cmOff = JSON.parse(off.coverageMap);
  const binaryIds = Object.entries(cmOff.branchMap)
    .filter(([, entry]) => entry.type === 'binary-expr')
    .map(([id]) => id);
  assert.equal(binaryIds.length, 1, 'Should record exactly one binary-expr branch for `a && b`');
  assert.equal(cmOff.bT, undefined, 'bT must be omitted when reportLogic is off');

  const on = instrument(source, 'logic.js', { reportLogic: true });
  const coverage = runInstrumented(
    on,
    'logic.js',
    'globalThis.f = f;\nf(0, 0); f(1, 0); f(1, 1);',
  );
  const [id] = binaryIds;

  // Three calls: f(0,0), f(1,0), f(1,1)
  // b[0] (a evaluated) = 3; b[1] (b evaluated when a truthy) = 2
  assert.deepEqual(coverage.b[id], [3, 2], 'b counts should reflect short-circuit semantics');
  // bT[0] (a truthy) = 2 (calls 2,3); bT[1] (b truthy) = 1 (call 3)
  assert.deepEqual(coverage.bT[id], [2, 1], 'bT counts should reflect truthy outcomes per operand');
  console.log('  [PASS] reportLogic tracks truthy hits at runtime');
}

// Test 14: inputSourceMap is composed into the coverage map
{
  const inputSourceMap = {
    version: 3,
    sources: ['original.ts'],
    names: ['x'],
    mappings: 'AAAA,EAAA',
    sourcesContent: ['const x: number = 1;\n'],
    file: 'pre-transform.js',
  };

  const result = instrument('const x = 1;', 'after-transform.js', {
    inputSourceMap: JSON.stringify(inputSourceMap),
  });
  const cm = JSON.parse(result.coverageMap);
  assert(cm.inputSourceMap, 'inputSourceMap must be attached to the coverage map');
  assert.equal(cm.inputSourceMap.version, 3);
  assert.deepEqual(cm.inputSourceMap.sources, inputSourceMap.sources);
  assert.equal(cm.inputSourceMap.mappings, inputSourceMap.mappings, 'mappings should pass through verbatim');
  assert.deepEqual(cm.inputSourceMap.names, inputSourceMap.names);
  assert.deepEqual(cm.inputSourceMap.sourcesContent, inputSourceMap.sourcesContent);
  console.log('  [PASS] inputSourceMap composed into coverage map');
}

// Test 15: ignoreClassMethods drops fnMap entries for matching methods
{
  const source = `
    class C {
      keep() { return 1; }
      drop() { return 2; }
    }
  `;

  const baseline = instrument(source, 'class.js');
  const baselineNames = Object.values(JSON.parse(baseline.coverageMap).fnMap).map((f) => f.name);
  assert(baselineNames.includes('keep') && baselineNames.includes('drop'), 'Baseline should include both methods');

  const filtered = instrument(source, 'class.js', { ignoreClassMethods: ['drop'] });
  const filteredNames = Object.values(JSON.parse(filtered.coverageMap).fnMap).map((f) => f.name);
  assert(filteredNames.includes('keep'), 'Non-ignored method should still be tracked');
  assert(!filteredNames.includes('drop'), 'Ignored class method should be omitted from fnMap');
  console.log('  [PASS] ignoreClassMethods drops fnMap entries');
}

// Test 16: unhandledPragmas surfaces unrecognized istanbul/v8 directives
{
  const source = '/* istanbul ignore bogus */ const x = 1;';
  const result = instrument(source, 'pragma.js');
  assert(Array.isArray(result.unhandledPragmas), 'unhandledPragmas must be an array');
  assert.equal(result.unhandledPragmas.length, 1, 'Should report exactly one unhandled pragma');
  const [pragma] = result.unhandledPragmas;
  assert(pragma.comment.includes('istanbul ignore bogus'), 'Comment text should include the directive');
  assert.equal(pragma.line, 1, 'Line should be 1-based');
  assert.equal(pragma.column, 0, 'Column should be 0-based');

  const clean = instrument('const x = 1;', 'clean.js');
  assert.equal(clean.unhandledPragmas.length, 0, 'Clean source should have no unhandled pragmas');
  console.log('  [PASS] unhandledPragmas surfaces unknown directives');
}

console.log('\nAll tests passed!');
