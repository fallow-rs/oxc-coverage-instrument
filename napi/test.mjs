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
  assert.equal(Object.keys(ternaryMap.branchMap).length, 1, 'non-ignored ternary arm should still be tracked');
  assert.equal(ternaryMap.branchMap['0'].locations.length, 1, 'ignored ternary arm should be pruned from branch paths');

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
    JSON.parse(logicalLeaf.coverageMap).branchMap['0'].locations.length,
    1,
    'ignored logical leaf should be pruned from branch paths',
  );

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

  console.log('  [PASS] Issue regression runtime parity');
}

console.log('\nAll tests passed!');
