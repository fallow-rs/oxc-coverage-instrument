// Verifies the Vitest istanbul coverage output points at the original .ts
// source, not at an intermediate transpiled JS file. The vitest.config.ts in
// this example wires `coverage.instrumenter` to `createOxcInstrumenter`, which
// auto-detects raw TypeScript when the filename ends in .ts / .tsx and no
// inputSourceMap is present.
//
// Asserts:
//   1. coverage-final.json contains a key whose path ends with math.ts
//      (NOT math.js, i.e., the TS source is the wire-format key)
//   2. statementMap entries point at line numbers that exist in the .ts source
//   3. compute() function counter and at least one branch counter are populated
//
// Exit codes: 0 on success, non-zero on failure with a descriptive message.

import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const coveragePath = resolve(repoRoot, 'coverage', 'coverage-final.json');

if (!existsSync(coveragePath)) {
    console.error(`FAIL: coverage-final.json not found at ${coveragePath}`);
    console.error('Run `npm run coverage` first.');
    process.exit(1);
}

const coverage = JSON.parse(readFileSync(coveragePath, 'utf8'));
const keys = Object.keys(coverage);

const mathKey = keys.find((k) => k.endsWith('math.ts'));
if (!mathKey) {
    console.error(
        `FAIL: no coverage entry for math.ts. Got keys:\n${keys.map((k) => `  - ${k}`).join('\n')}`,
    );
    console.error('Expected at least one key ending in `math.ts`, not `math.js` or a transpiled path.');
    process.exit(1);
}

const fc = coverage[mathKey];
if (!fc.statementMap || Object.keys(fc.statementMap).length === 0) {
    console.error(`FAIL: math.ts statementMap is empty: ${JSON.stringify(fc.statementMap)}`);
    process.exit(1);
}

const tsSourcePath = resolve(repoRoot, 'src', 'math.ts');
const tsSource = readFileSync(tsSourcePath, 'utf8');
const tsLines = tsSource.split('\n').length;

for (const [id, loc] of Object.entries(fc.statementMap)) {
    if (loc.start.line < 1 || loc.start.line > tsLines) {
        console.error(
            `FAIL: statementMap[${id}].start.line = ${loc.start.line} is out of range for math.ts (${tsLines} lines)`,
        );
        process.exit(1);
    }
}

const fnNames = Object.values(fc.fnMap).map((f) => f.name);
if (!fnNames.includes('compute')) {
    console.error(`FAIL: compute() function not found in fnMap. Got names: ${JSON.stringify(fnNames)}`);
    process.exit(1);
}

if (Object.keys(fc.branchMap).length === 0) {
    console.error('FAIL: branchMap is empty; expected at least one branch counter for the if-else in compute().');
    process.exit(1);
}

const computeHits = Object.entries(fc.fnMap)
    .filter(([, f]) => f.name === 'compute')
    .map(([id]) => fc.f[id]);
if (!computeHits.some((h) => h > 0)) {
    console.error(`FAIL: compute() function counter is zero. Tests must have invoked it. Got: ${JSON.stringify(computeHits)}`);
    process.exit(1);
}

console.log(`OK: ${basename(mathKey)} coverage points at .ts source lines.`);
console.log(`  statements: ${Object.keys(fc.statementMap).length}`);
console.log(`  functions:  ${Object.keys(fc.fnMap).length}`);
console.log(`  branches:   ${Object.keys(fc.branchMap).length}`);
console.log(`  compute() hits: ${computeHits[0]}`);
