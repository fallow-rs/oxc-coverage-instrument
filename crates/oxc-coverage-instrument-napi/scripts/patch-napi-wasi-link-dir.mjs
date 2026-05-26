#!/usr/bin/env node
// napi-rs 3.6.x accepts `--target wasm32-wasip1`, but its build helper still
// hardcodes a threaded emnapi link directory for every WASI target. That makes
// the supposedly single-threaded artifact import shared memory and fail in
// Workers. Patch the installed CLI so CI can choose the emnapi link directory
// with `NAPI_RS_WASI_LINK_DIR=wasm32-wasip1`.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const cliRoot = join(__dirname, '..', 'node_modules', '@napi-rs', 'cli', 'dist');
const files = ['cli.js', 'index.js', 'index.cjs'];
const threadedLinkDirs = new Set(['wasm32-wasi-threads', 'wasm32-wasip1-threads']);

function patchSource(source) {
  let patched = false;
  const lines = source.split('\n').map((line) => {
    if (!line.includes('const emnapi =') || !line.includes('lib')) {
      return line;
    }
    for (const linkDir of threadedLinkDirs) {
      const replacement = `process.env.NAPI_RS_WASI_LINK_DIR || "${linkDir}"`;
      const quoted = `"${linkDir}"`;
      if (line.includes(quoted)) {
        patched = true;
        return line.replace(quoted, replacement);
      }
      const singleQuoted = `'${linkDir}'`;
      if (line.includes(singleQuoted)) {
        patched = true;
        return line.replace(singleQuoted, replacement);
      }
    }
    return line;
  });
  return { source: lines.join('\n'), patched };
}

for (const file of files) {
  const path = join(cliRoot, file);
  if (!existsSync(path)) {
    console.error(`[patch-napi-wasi-link-dir] missing ${path}; run npm install first.`);
    process.exit(1);
  }
  const original = readFileSync(path, 'utf8');
  if (original.includes('process.env.NAPI_RS_WASI_LINK_DIR')) {
    console.log(`[patch-napi-wasi-link-dir] ${file} already patched.`);
    continue;
  }
  const { source, patched } = patchSource(original);
  if (!patched) {
    console.error(`[patch-napi-wasi-link-dir] ${file} has unexpected napi-rs shape.`);
    process.exit(1);
  }
  writeFileSync(path, source);
  console.log(`[patch-napi-wasi-link-dir] patched ${file}.`);
}
