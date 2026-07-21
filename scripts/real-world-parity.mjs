#!/usr/bin/env node
// Real-world regression check: compare oxc-coverage-instrument vs
// istanbul-lib-instrument coverage-map counts across the benchmark JS
// libraries cached under `.bench-tmp/files/` by `scripts/benchmark-comparison.sh`.
//
// Any statement or function count divergence fails the run. Branch counts
// are allowed to exceed istanbul's (documented `??=`/`||=`/`&&=` superset)
// but never fall below; that would be a regression. The lodash artifact is
// also executed before and after instrumentation with an observable API probe.
//
// Usage:
//   # populate .bench-tmp/files first, e.g. via ./scripts/benchmark-comparison.sh
//   node scripts/real-world-parity.mjs

import { createInstrumenter } from 'istanbul-lib-instrument';
import { readFileSync, readdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import vm from 'node:vm';
import { createOxcInstrumenter } from '../crates/oxc_coverage_instrument_napi/vitest.js';

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

const counts = (cov) => ({
  s: Object.keys(cov.statementMap).length,
  f: Object.keys(cov.fnMap).length,
  b: Object.keys(cov.branchMap).length,
});

const executeLodashProbe = (code) => {
  const module = { exports: {} };
  const sandbox = { module, exports: module.exports };
  vm.runInNewContext(code, sandbox, { filename: 'lodash.js' });
  const lodash = module.exports;
  if (typeof lodash.chunk !== 'function') {
    throw new Error('lodash execution probe did not expose chunk()');
  }
  return JSON.stringify(lodash.chunk([1, 2, 3], 2));
};

let diverged = 0;
let executedRealWorldFile = false;
for (const name of files) {
  const src = readFileSync(join(dir, name), 'utf8');
  const sizeKB = (src.length / 1024).toFixed(1);

  const istanbulInst = createInstrumenter({ esModules: false, produceSourceMap: false });
  istanbulInst.instrumentSync(src, name);
  const istanbul = counts(istanbulInst.lastFileCoverage());

  const oxcInst = createOxcInstrumenter({ coverageVariable: '__coverage__' });
  const instrumented = oxcInst.instrumentSync(src, name);
  const oxc = counts(oxcInst.lastFileCoverage());

  if (name === 'lodash.js') {
    const originalResult = executeLodashProbe(src);
    const instrumentedResult = executeLodashProbe(instrumented);
    if (instrumentedResult !== originalResult) {
      throw new Error(
        `lodash execution probe diverged: original=${originalResult} instrumented=${instrumentedResult}`,
      );
    }
    executedRealWorldFile = true;
  }

  const sOk = oxc.s === istanbul.s;
  const fOk = oxc.f === istanbul.f;
  const bOk = oxc.b >= istanbul.b;
  const ok = sOk && fOk && bOk;
  const tag = ok ? '[OK]  ' : '[DIFF]';
  console.log(
    `${tag} ${name.padEnd(24)} ${sizeKB.padStart(6)} KB  ` +
      `istanbul s=${istanbul.s} f=${istanbul.f} b=${istanbul.b}  ` +
      `oxc s=${oxc.s} f=${oxc.f} b=${oxc.b}`,
  );
  if (!ok) {
    diverged++;
    if (!sOk) console.log(`       statements differ: istanbul=${istanbul.s} oxc=${oxc.s}`);
    if (!fOk) console.log(`       functions differ:  istanbul=${istanbul.f} oxc=${oxc.f}`);
    if (!bOk) {
      console.log(
        `       branches regress:  istanbul=${istanbul.b} oxc=${oxc.b} (oxc should be >= istanbul)`,
      );
    }
  }
}

if (!executedRealWorldFile) {
  console.error('error: lodash.js is required for the real-world execution probe');
  process.exit(2);
}

console.log(`\n${diverged === 0 ? 'PASS' : 'FAIL'}: ${diverged} of ${files.length} files diverged`);
process.exit(diverged === 0 ? 0 : 1);
