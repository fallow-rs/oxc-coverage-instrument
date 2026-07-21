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
//   node scripts/prepare-real-world-corpus.mjs
//   node scripts/compose-real-map-parity.mjs
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { loadVerifiedCorpus } from './prepare-real-world-corpus.mjs';

const require = createRequire(import.meta.url);
const babel = createRequire(require.resolve('istanbul-lib-instrument/package.json'))('@babel/core');
const { instrument, remapCoverageMap } = require('../crates/oxc_coverage_instrument_napi/index.js');

let projects;
try {
  projects = loadVerifiedCorpus();
} catch (error) {
  console.error(`error: ${error.message}`);
  process.exit(2);
}

const firstMismatch = (eager, lazy) => {
  const keys = [...new Set([...Object.keys(eager), ...Object.keys(lazy)])].sort();
  return keys.find((key) => JSON.stringify(eager[key]) !== JSON.stringify(lazy[key])) ?? 'none';
};

let failures = 0;
for (const project of projects) {
  const { filename: file, name, version, path } = project;
  const label = `${name}@${version}`;
  const source = readFileSync(path, 'utf8');
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
      console.log(
        `${label} compact=${compact}: lazy remap missing path ${eager.path} ` +
          `(got ${Object.keys(lazyOut)[0] ?? 'none'})`,
      );
      continue;
    }
    for (const dim of ['statementMap', 'fnMap', 'branchMap']) {
      const ok = JSON.stringify(eager[dim]) === JSON.stringify(lazy[dim]);
      if (!ok) failures++;
      const detail = ok
        ? 'EQ'
        : `DIFF eager=${Object.keys(eager[dim]).length} ` +
          `lazy=${Object.keys(lazy[dim]).length} first=${firstMismatch(eager[dim], lazy[dim])}`;
      console.log(
        `${label} compact=${compact} ${dim}: ${detail}`,
      );
    }
  }
}

console.log(failures ? `\nFAIL: ${failures} dimension(s) diverged` : '\nPASS: eager compose matches the deferred remap');
process.exit(failures ? 1 : 0);
