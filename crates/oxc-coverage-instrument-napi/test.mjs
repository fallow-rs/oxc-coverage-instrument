// Quick test for the napi bindings
import {
  instrument,
  remapCoverageMap,
  remapCoverageMapWithLoader,
  v8ToIstanbul,
  v8ToIstanbulWithLoader,
} from './index.js';
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
  const noElseSynthetic = JSON.parse(noElse.coverageMap).branchMap['0'].locations[1];
  assert.ok(
    Number.isInteger(noElseSynthetic.start.line),
    'synthetic else arm must carry a real start line, not an empty placeholder',
  );
  assert.deepEqual(
    noElseSynthetic.start,
    noElseSynthetic.end,
    'synthetic else arm is anchored as a zero-width location',
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
  // Istanbul keeps the branch entry when only one arm is ignored: the
  // surviving arm still gets a location and a counter.
  assert.equal(Object.keys(ternaryMap.branchMap).length, 1, 'surviving arm should keep its branch entry');
  assert.equal(ternaryMap.branchMap['0'].locations.length, 1, 'ignored arm must be dropped from locations');
  assert.equal(ternaryMap.b['0'].length, 1, 'branch hit array must match locations length');

  const fullyIgnoredTernary = instrument(
    'function f(x) { return x ? /* v8 ignore next */ 1 : /* v8 ignore next */ 2; }',
    'fully-ignored-ternary.js',
  );
  assert.equal(Object.keys(JSON.parse(fullyIgnoredTernary.coverageMap).branchMap).length, 0, 'empty branches should be pruned');

  const logicalLeaf = instrument(
    'function f(a, b) { return a && /* v8 ignore next */ b; }',
    'logical-leaf.js',
  );
  const logicalMap = JSON.parse(logicalLeaf.coverageMap);
  assert.equal(
    Object.keys(logicalMap.branchMap).length,
    1,
    'logical branch entry must survive when one operand is ignored',
  );
  assert.equal(logicalMap.branchMap['0'].locations.length, 1, 'ignored leaf must be dropped from locations');
  assert.equal(logicalMap.b['0'].length, 1, 'branch hit array must match locations length');

  // Pragma scoped to a single arm: the branch entry survives with one
  // remaining arm. Pragma attached to a JSX attribute or child wraps the
  // whole subtree and the branch is dropped entirely.
  for (const [label, source, path, expectedBranches, expectedArms] of [
    ['nullish rhs', 'function f(x) { return x ?? /* v8 ignore next -- @preserve */ [] }', 'issue-27-nullish.js', 1, 1],
    ['or rhs', 'function f(x) { return x || /* v8 ignore next -- @preserve */ true }', 'issue-27-or.js', 1, 1],
    ['and rhs', 'function f(x) { return x && /* v8 ignore next -- @preserve */ true }', 'issue-27-and.js', 1, 1],
    ['conditional rhs', 'function f(x) { return x ? 1 : /* v8 ignore next -- @preserve */ 2 }', 'issue-27-cond.js', 1, 1],
    [
      'jsx attribute',
      `function f(pass) {
        return <Tag
          /* v8 ignore next -- @preserve */
          text={pass ? 'Pass' : 'Fail'}
        />
      }`,
      'issue-28.tsx',
      0,
      0,
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
      0,
      0,
    ],
  ]) {
    const map = JSON.parse(instrument(source, path).coverageMap);
    assert.equal(
      Object.keys(map.branchMap).length,
      expectedBranches,
      `${label}: unexpected branch entry count`,
    );
    if (expectedBranches > 0) {
      assert.equal(
        map.branchMap['0'].locations.length,
        expectedArms,
        `${label}: surviving arm count mismatch`,
      );
    }
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

// Test 17: remapCoverageMap rewrites coverage paths and positions through inputSourceMap
{
  const originalTs = 'const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n';
  const intermediateJs = 'const x = 1;\nconst y = 2;\nconst z = 3;\n';
  const inputSourceMap = {
    version: 3,
    sources: ['src/app.ts'],
    sourcesContent: [originalTs],
    mappings: 'AAAA;AACA;AACA',
    names: [],
  };
  const result = instrument(intermediateJs, 'intermediate.js', {
    inputSourceMap: JSON.stringify(inputSourceMap),
  });

  const coverageMap = { 'intermediate.js': JSON.parse(result.coverageMap) };
  const remapped = JSON.parse(remapCoverageMap(JSON.stringify(coverageMap)));

  assert(remapped['src/app.ts'], 'remapped map should be keyed by the resolved original path');
  assert(!remapped['intermediate.js'], 'remapped map should not retain the intermediate key');
  assert.equal(remapped['src/app.ts'].path, 'src/app.ts', 'path on the file coverage should match');
  assert(!remapped['src/app.ts'].inputSourceMap, 'inputSourceMap should be cleared after remap');
  const lines = Object.values(remapped['src/app.ts'].statementMap)
    .map((loc) => loc.start.line)
    .sort();
  assert.deepEqual(lines, [1, 2, 3], 'statement lines should map back to original.ts lines');
  console.log('  [PASS] remapCoverageMap rewrites coverage through inputSourceMap');
}

// Test 18: v8ToIstanbul converts V8 byte-range coverage into Istanbul FileCoverage
{
  const source = 'const x = 1;\nconst y = 2;\nconst z = 3;\n';
  const functions = [
    {
      functionName: '',
      ranges: [{ startOffset: 0, endOffset: source.length, count: 1 }],
      isBlockCoverage: false,
    },
  ];
  const fc = JSON.parse(v8ToIstanbul(source, 'app.js', JSON.stringify(functions)));
  const counts = Object.values(fc.s);
  assert(counts.length > 0, 'statementMap should populate');
  assert(counts.every((c) => c === 1), `all statements covered, got: ${JSON.stringify(counts)}`);
  console.log('  [PASS] v8ToIstanbul produces Istanbul FileCoverage from V8 ranges');
}

// Test 19: remapCoverageMapWithLoader supplies maps for entries without inputSourceMap
{
  const intermediateJs = 'const x = 1;\nconst y = 2;\nconst z = 3;\n';
  const inputSourceMap = JSON.stringify({
    version: 3,
    sources: ['src/app.ts'],
    sourcesContent: ['const x: number = 1;\nconst y: number = 2;\nconst z: number = 3;\n'],
    mappings: 'AAAA;AACA;AACA',
    names: [],
  });
  // Instrument WITHOUT inputSourceMap so the coverage has none embedded.
  const result = instrument(intermediateJs, 'intermediate.js');
  const coverageMap = { 'intermediate.js': JSON.parse(result.coverageMap) };

  // Loader-supplied path: the dictionary key matches the FileCoverage path.
  const remapped = JSON.parse(
    remapCoverageMapWithLoader(JSON.stringify(coverageMap), { 'intermediate.js': inputSourceMap }),
  );
  assert(remapped['src/app.ts'], 'loader-supplied map should remap the entry');
  assert(!remapped['intermediate.js'], 'intermediate key should be replaced');

  // Empty dictionary: passthrough.
  const passthrough = JSON.parse(remapCoverageMapWithLoader(JSON.stringify(coverageMap), {}));
  assert(passthrough['intermediate.js'], 'no loader entry -> passthrough');
  console.log('  [PASS] remapCoverageMapWithLoader supplies external maps');
}

// Test 20: v8ToIstanbulWithLoader resolves external //# sourceMappingURL references
{
  const map = JSON.stringify({
    version: 3,
    sources: ['src/app.ts'],
    sourcesContent: ['const x: number = 1;\n'],
    mappings: 'AAAA',
    names: [],
  });
  const source = 'const x = 1;\n//# sourceMappingURL=app.js.map\n';
  const functions = [
    {
      functionName: '',
      ranges: [{ startOffset: 0, endOffset: source.length, count: 1 }],
      isBlockCoverage: false,
    },
  ];

  const fc = JSON.parse(
    v8ToIstanbulWithLoader(source, 'app.js', JSON.stringify(functions), { 'app.js.map': map }),
  );
  assert(fc.inputSourceMap, 'loader entry should be attached as inputSourceMap');
  assert.equal(fc.inputSourceMap.sources[0], 'src/app.ts');

  // No matching loader entry -> map left unset.
  const fc2 = JSON.parse(v8ToIstanbulWithLoader(source, 'app.js', JSON.stringify(functions), {}));
  assert(!fc2.inputSourceMap, 'unknown URL -> inputSourceMap unset');
  console.log('  [PASS] v8ToIstanbulWithLoader resolves external map URLs');
}

// Test 21: v8ToIstanbul resolves if-arm[0] through the collected then-block span
{
  const source = 'function f(x) {\n  if (x) {\n    a();\n  } else {\n    b();\n  }\n}\n';
  const moduleEnd = source.length;
  const thenStart = source.indexOf('if (x) {') + 7;
  const thenEnd = source.indexOf('} else') + 1;
  const elseStart = source.indexOf('else {') + 5;
  const elseEnd = source.lastIndexOf('\n  }') + 4;

  // Function ran 5 times; predicate truthy 3 of those; else taken 2 of those.
  const functions = [
    {
      functionName: 'f',
      ranges: [
        { startOffset: 0, endOffset: moduleEnd, count: 5 },
        { startOffset: thenStart, endOffset: thenEnd, count: 3 },
        { startOffset: elseStart, endOffset: elseEnd, count: 2 },
      ],
      isBlockCoverage: true,
    },
  ];
  const fc = JSON.parse(v8ToIstanbul(source, 'ifelse.js', JSON.stringify(functions)));
  const [ifId] = Object.entries(fc.branchMap).find(([, entry]) => entry.type === 'if');
  assert.deepEqual(
    fc.b[ifId],
    [3, 2],
    'arm[0] now reflects then-block count; arm[1] reflects else-block count',
  );
  console.log('  [PASS] v8ToIstanbul resolves if-arm[0] through collected body span');
}

// Test: stripTypescript option strips TS annotations and produces executable JS
{
  const result = instrument(
    'const x: number = 1;\nconsole.log(x);\n',
    'app.ts',
    { stripTypescript: true, sourceMap: true },
  );
  assert(!result.code.includes(': number'), 'TS annotation must be stripped');
  assert(result.code.includes('const x ='), 'output must contain executable JS');
  const cov = JSON.parse(result.coverageMap);
  assert(cov.path === 'app.ts', 'coverage map path should be app.ts');
  assert(Object.keys(cov.statementMap).length >= 2, 'statementMap should be populated');
  console.log('  [PASS] stripTypescript option strips TS and produces executable JS');
}

// Test: stripTypescript defaults to false (preserves backward compatibility)
{
  const result = instrument('const x: number = 1;\n', 'app.ts');
  assert(result.code.includes(': number'), 'without stripTypescript the TS annotation must remain');
  console.log('  [PASS] stripTypescript defaults to false');
}

// Test: createOxcInstrumenter auto-detects .ts as raw TypeScript when no inputSourceMap
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter();
  const code = inst.instrumentSync('const x: number = 1;\nconsole.log(x);\n', 'app.ts');
  assert(!code.includes(': number'), 'auto-detect must strip TS on .ts without inputSourceMap');
  assert(code.includes('const x ='), 'auto-detect must emit executable JS');
  console.log('  [PASS] vitest adapter auto-detects .ts without inputSourceMap');
}

// Test: createOxcInstrumenter auto-detects .tsx and preserves JSX
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter();
  const code = inst.instrumentSync(
    'const el: JSX.Element = <div>hi</div>;\nconsole.log(el);\n',
    'app.tsx',
  );
  assert(!code.includes(': JSX.Element'), 'auto-detect must strip TS on .tsx');
  assert(code.includes('<div>'), 'auto-detect must preserve JSX on .tsx');
  console.log('  [PASS] vitest adapter auto-detects .tsx and preserves JSX');
}

// Test: createOxcInstrumenter does NOT auto-strip when inputSourceMap is provided
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter();
  // Pass raw TS source WITH an inputSourceMap. In real Vite/Vitest usage the
  // source would already be transformed JS by this point, but feeding raw TS
  // is the cleanest way to OBSERVE whether the strip pass ran. Auto-detect
  // must treat the inputSourceMap presence as "already transformed upstream"
  // and skip the strip; the TS annotation in the output proves it.
  const fakeMap = { version: 3, sources: ['orig.ts'], mappings: '', names: [] };
  const code = inst.instrumentSync('const x: number = 1;\n', 'app.ts', fakeMap);
  assert(
    code.includes(': number'),
    'with inputSourceMap the strip pass must not run, TS annotation must survive',
  );
  console.log('  [PASS] vitest adapter does not strip when inputSourceMap is present');
}

// Test: createOxcInstrumenter does NOT auto-strip .js files
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter();
  const code = inst.instrumentSync('const x = 1;\nconsole.log(x);\n', 'app.js');
  assert(code.includes('const x ='), '.js must pass through as executable JS');
  console.log('  [PASS] vitest adapter does not auto-strip .js files');
}

// Test: explicit stripTypescript: false overrides auto-detect on .ts
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter({ stripTypescript: false });
  const code = inst.instrumentSync('const x: number = 1;\n', 'app.ts');
  assert(
    code.includes(': number'),
    'explicit stripTypescript: false must keep TS annotations on .ts',
  );
  console.log('  [PASS] vitest adapter honors explicit stripTypescript: false on .ts');
}

// Test: explicit stripTypescript: true overrides the auto-detect skip on .ts
// when inputSourceMap is present. Auto-detect would NOT strip (inputSourceMap
// signals "already transformed upstream"), but explicit true forces the strip
// pass anyway. Observable via the TS annotation disappearing from the output.
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter({ stripTypescript: true });
  const fakeMap = { version: 3, sources: ['orig.ts'], mappings: '', names: [] };
  const code = inst.instrumentSync('const x: number = 1;\n', 'app.ts', fakeMap);
  assert(
    !code.includes(': number'),
    'explicit stripTypescript: true must force strip even when inputSourceMap is present',
  );
  console.log('  [PASS] vitest adapter honors explicit stripTypescript: true override');
}

// Test: non-boolean stripTypescript throws TypeError instead of silently
// coercing. Catches the 'auto' string case (a prior tri-state design shape)
// that Boolean coercion would turn into force-strip.
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  let caught = null;
  try {
    createOxcInstrumenter({ stripTypescript: 'auto' });
  } catch (e) {
    caught = e;
  }
  assert(caught instanceof TypeError, `expected TypeError, got ${caught}`);
  assert(
    caught.message.includes('stripTypescript'),
    `error message must name the offending option, got: ${caught.message}`,
  );
  console.log('  [PASS] vitest adapter rejects non-boolean stripTypescript with TypeError');
}

// Test: TS_EXTENSION_REGEX rejects non-TypeScript extensions that look similar.
// The narrow form /\.([mc]ts|tsx?)$/i must not match .ts.bak (where .bak is
// the actual extension) or .mtsx (not a real Node / TS extension).
// Strategy: feed TS source to a non-matching filename. Auto-detect must skip
// the strip; the parser then rejects the TS syntax because the filename
// extension (.bak / .mtsx) means SourceType::from_path falls back to JS.
{
  const { createOxcInstrumenter } = await import('./vitest.js');
  const inst = createOxcInstrumenter();
  // .ts.bak: actual extension is .bak; strip must not auto-engage.
  let caughtBak = null;
  try {
    inst.instrumentSync('const x: number = 1;\n', 'app.ts.bak');
  } catch (e) {
    caughtBak = e;
  }
  assert(
    caughtBak !== null && /parse error/.test(caughtBak.message),
    'app.ts.bak must not auto-strip; parser must reject TS syntax',
  );
  // .mtsx: not a real extension (only .mts / .cts / .ts / .tsx are valid).
  let caughtMtsx = null;
  try {
    inst.instrumentSync('const x: number = 1;\n', 'app.mtsx');
  } catch (e) {
    caughtMtsx = e;
  }
  assert(
    caughtMtsx !== null && /parse error/.test(caughtMtsx.message),
    'app.mtsx must not auto-strip; .mtsx is not a real TypeScript extension',
  );
  console.log(
    '  [PASS] vitest adapter regex rejects .ts.bak and .mtsx (non-TS extensions)',
  );
}

console.log('\nAll tests passed!');
