#!/usr/bin/env node
// Eager compose vs deferred remap over real @babel/core-emitted source maps.
//
// For every benchmark library, transform it with babel (sourceMaps on, once
// plain and once compact) and instrument the output twice: eagerly composed
// (`composeInputSourceMap: true`) and plain-then-remapped through
// `remapCoverageMap` with `dropUnmapped`. The statement, function and branch
// maps must be byte-equal: real maps exercise dense, precise segments where
// the fold must stay inert, and compact output shifts positions off the
// segment starts, where `getMapping` widening makes spans collapse.
//
// `@babel/core` is resolved through istanbul-lib-instrument, which declares it.
//
// Usage:
//   # populate .bench-tmp/files first, e.g. via ./scripts/benchmark-comparison.sh
//   node scripts/compose-real-map-parity.mjs
import { existsSync, readFileSync, readdirSync } from 'fs';
import { createRequire } from 'module';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const require = createRequire(import.meta.url);
const babel = createRequire(require.resolve('istanbul-lib-instrument/package.json'))('@babel/core');
const { instrument, remapCoverageMap } = require('../crates/oxc_coverage_instrument_napi/index.js');

const __dirname = dirname(fileURLToPath(import.meta.url));
const dir = join(__dirname, '..', '.bench-tmp', 'files');

if (!existsSync(dir)) {
  console.error(`error: ${dir} does not exist`);
  console.error('populate it by running ./scripts/benchmark-comparison.sh first');
  process.exit(2);
}

const files = readdirSync(dir).filter((f) => f.endsWith('.js')).sort();
if (files.length === 0) {
  console.error(`error: no .js files under ${dir}`);
  process.exit(2);
}

let failures = 0;
for (const file of files) {
  const source = readFileSync(join(dir, file), 'utf8');
  for (const compact of [false, true]) {
    const out = babel.transformSync(source, {
      filename: file,
      sourceMaps: true,
      configFile: false,
      babelrc: false,
      compact,
    });
    const ism = JSON.stringify(out.map);
    const eager = JSON.parse(
      instrument(out.code, file, { coverageVariable: '__c', inputSourceMap: ism, composeInputSourceMap: true })
        .coverageMap,
    );
    const plain = JSON.parse(
      instrument(out.code, file, { coverageVariable: '__c', inputSourceMap: ism }).coverageMap,
    );
    const lazyOut = JSON.parse(remapCoverageMap(JSON.stringify({ [file]: plain }), { dropUnmapped: true }));
    const lazy = lazyOut[eager.path];
    if (!lazy) {
      failures++;
      console.log(`${file} compact=${compact}: lazy remap missing path ${eager.path} (got ${Object.keys(lazyOut)})`);
      continue;
    }
    for (const dim of ['statementMap', 'fnMap', 'branchMap']) {
      const ok = JSON.stringify(eager[dim]) === JSON.stringify(lazy[dim]);
      if (!ok) failures++;
      console.log(
        `${file} compact=${compact} ${dim}: ${ok ? 'EQ' : `DIFF eager=${Object.keys(eager[dim]).length} lazy=${Object.keys(lazy[dim]).length}`}`,
      );
    }
  }
}

console.log(failures ? `\nFAIL: ${failures} dimension(s) diverged` : '\nPASS: eager compose matches the deferred remap');
process.exit(failures ? 1 : 0);
