#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { basename } from 'node:path';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';

const DEFAULT_MAX_BROTLI_BYTES = 2 * 1024 * 1024;

let maxBrotliBytes = DEFAULT_MAX_BROTLI_BYTES;
const files = [];

for (const arg of process.argv.slice(2)) {
  if (arg.startsWith('--max-brotli-bytes=')) {
    maxBrotliBytes = Number.parseInt(arg.slice('--max-brotli-bytes='.length), 10);
  } else {
    files.push(arg);
  }
}

if (!Number.isSafeInteger(maxBrotliBytes) || maxBrotliBytes <= 0) {
  throw new Error(`invalid --max-brotli-bytes value: ${maxBrotliBytes}`);
}

if (files.length === 0) {
  throw new Error('usage: node scripts/wasm-size-check.mjs [--max-brotli-bytes=N] <file.wasm> [...]');
}

const mb = (bytes) => `${(bytes / 1024 / 1024).toFixed(2)} MB`;
let failed = false;

for (const file of files) {
  const bytes = readFileSync(file);
  const gzip = gzipSync(bytes, { level: 9 });
  const brotli = brotliCompressSync(bytes, {
    params: {
      [constants.BROTLI_PARAM_QUALITY]: 11,
    },
  });

  console.log(`${basename(file)}`);
  console.log(`  raw      ${bytes.byteLength.toString().padStart(10)}  ${mb(bytes.byteLength)}`);
  console.log(`  gzip -9  ${gzip.byteLength.toString().padStart(10)}  ${mb(gzip.byteLength)}`);
  console.log(`  brotli   ${brotli.byteLength.toString().padStart(10)}  ${mb(brotli.byteLength)}`);

  if (brotli.byteLength > maxBrotliBytes) {
    console.error(
      `::error file=${file}::Brotli-compressed wasm size ${brotli.byteLength} exceeds ceiling ${maxBrotliBytes} (${mb(maxBrotliBytes)}).`,
    );
    failed = true;
  }
}

if (failed) {
  process.exit(1);
}
