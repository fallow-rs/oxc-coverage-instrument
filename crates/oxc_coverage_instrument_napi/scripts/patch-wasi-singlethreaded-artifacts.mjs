#!/usr/bin/env node
// Post-build patch for the generated single-threaded WASI artifacts.
//
// napi-rs 3.9 emits a separate single-threaded flavor for wasm32-wasip1, with
// `wasip1` file names and a `binding-wasm32-wasip1` package reference. This
// repo publishes that flavor as `binding-wasm32-wasi-singlethreaded` with the
// `wasi` file names, so the script renames the files and the references first.
// Older napi-rs releases emitted worker and shared-memory shims for
// wasm32-wasip1 and could mark one runtime global immutable. The remaining
// patches remove those markers when they occur, and they do nothing otherwise.

import { existsSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(__dirname, '..');
const targetDir = process.argv[2] ? resolve(packageRoot, process.argv[2]) : packageRoot;

const threadedPackage = '@oxc-coverage-instrument/binding-wasm32-wasi';
const singlePackage = '@oxc-coverage-instrument/binding-wasm32-wasi-singlethreaded';
// Match the threaded package name only when no suffix follows it, so a second
// run does not rename the single-threaded package again.
const threadedPackagePattern = /@oxc-coverage-instrument\/binding-wasm32-wasi(?![-\w])/g;

const wasip1Renames = [
  ['coverage-instrument.wasm32-wasip1.wasm', 'coverage-instrument.wasm32-wasi.wasm'],
  ['coverage-instrument.wasm32-wasip1.debug.wasm', 'coverage-instrument.wasm32-wasi.debug.wasm'],
  ['coverage-instrument.wasip1.cjs', 'coverage-instrument.wasi.cjs'],
  ['coverage-instrument.wasip1-browser.js', 'coverage-instrument.wasi-browser.js'],
];

export function renameWasip1References(source) {
  return source
    .replaceAll('coverage-instrument.wasm32-wasip1.', 'coverage-instrument.wasm32-wasi.')
    .replaceAll('@oxc-coverage-instrument/binding-wasm32-wasip1', threadedPackage);
}

// Move the napi-rs 3.9 `wasip1` outputs to the file names that this repo
// publishes. Returns the number of files that moved.
export function normalizeWasip1Outputs(dir) {
  let moved = 0;
  for (const [from, to] of wasip1Renames) {
    const source = join(dir, from);
    if (!existsSync(source)) {
      continue;
    }
    const target = join(dir, to);
    if (from.endsWith('.wasm')) {
      renameSync(source, target);
    } else {
      writeFileSync(target, renameWasip1References(readFileSync(source, 'utf8')));
      rmSync(source);
    }
    moved += 1;
  }
  return moved;
}

export function patchSingleThreadedWasm(buffer) {
  if (WebAssembly.validate(buffer)) {
    return { buffer, patched: false };
  }

  const bytes = Buffer.from(buffer);
  let offset = 8;
  let importedGlobals = 0;
  const candidates = [];

  const readU32 = () => {
    let result = 0;
    let shift = 0;
    while (true) {
      const byte = bytes[offset++];
      result |= (byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) {
        return result >>> 0;
      }
      shift += 7;
    }
  };

  const skipString = () => {
    const length = readU32();
    offset += length;
  };

  const skipLimits = () => {
    const flags = readU32();
    readU32();
    if (flags & 1) readU32();
    if (flags & 2) readU32();
  };

  const skipInitExpr = () => {
    while (offset < bytes.length) {
      const opcode = bytes[offset++];
      if (opcode === 0x0b) return;
      if (opcode === 0x23 || opcode === 0x41 || opcode === 0x42) {
        readU32();
      } else if (opcode === 0x43) {
        offset += 4;
      } else if (opcode === 0x44) {
        offset += 8;
      } else {
        throw new Error(`unsupported wasm global init opcode 0x${opcode.toString(16)}`);
      }
    }
  };

  while (offset < bytes.length) {
    const sectionId = bytes[offset++];
    const sectionLength = readU32();
    const sectionEnd = offset + sectionLength;

    if (sectionId === 2) {
      const count = readU32();
      for (let index = 0; index < count; index += 1) {
        skipString();
        skipString();
        const kind = bytes[offset++];
        if (kind === 0) {
          readU32();
        } else if (kind === 1) {
          offset += 1;
          skipLimits();
        } else if (kind === 2) {
          skipLimits();
        } else if (kind === 3) {
          offset += 2;
          importedGlobals += 1;
        } else {
          throw new Error(`unsupported wasm import kind ${kind}`);
        }
      }
    } else if (sectionId === 6) {
      const count = readU32();
      for (let localIndex = 0; localIndex < count; localIndex += 1) {
        const valtype = bytes[offset++];
        const mutOffset = offset;
        const mutability = bytes[offset++];
        const globalIndex = importedGlobals + localIndex;
        if (valtype === 0x7f && mutability === 0) {
          candidates.push({ globalIndex, mutOffset });
        }
        skipInitExpr();
      }
    }

    offset = sectionEnd;
  }

  for (const candidate of candidates) {
    const patched = Buffer.from(bytes);
    patched[candidate.mutOffset] = 1;
    if (WebAssembly.validate(patched)) {
      return { buffer: patched, patched: true, globalIndex: candidate.globalIndex };
    }
  }

  throw new Error('failed to patch single-threaded wasm: no immutable i32 global made it valid');
}

export function patchSingleThreadedShim(source) {
  const out = source
    .replace(threadedPackagePattern, singlePackage)
    .replace(
      /\n\n\/\/ oxc-coverage-instrument: SharedArrayBuffer guard \(issue #89\)\nif \(typeof SharedArrayBuffer === 'undefined'\) \{\n  throw new Error\(\n    'oxc-coverage-instrument: the browser WASM binding requires SharedArrayBuffer\. ' \+\n      'Enable Cross-Origin-Opener-Policy: same-origin and ' \+\n      'Cross-Origin-Embedder-Policy: require-corp on your host page so the ' \+\n      'browser is cross-origin isolated\. See ' \+\n      'https:\/\/github\.com\/fallow-rs\/oxc-coverage-instrument#runtime-matrix',\n  \);\n\}\n/,
      '\n',
    )
    .replaceAll(
      'Cannot find coverage-instrument.wasm32-wasi.wasm file, and @oxc-coverage-instrument/binding-wasm32-wasi package is not installed.',
      `Cannot find coverage-instrument.wasm32-wasi.wasm file, and ${singlePackage} package is not installed.`,
    )
    .replace(
      /const __sharedMemory = new WebAssembly\.Memory\(\{\s*initial: 4000,\s*maximum: 65536,\s*shared: true,\s*\}\)/,
      'const __sharedMemory = new WebAssembly.Memory({\n  initial: 4000,\n  maximum: 65536,\n})',
    )
    .replace(
      'memory: __sharedMemory,\n    }',
      'memory: __sharedMemory,\n      __tls_align: 1,\n      __tls_size: 0,\n      __wasm_init_tls: () => {},\n    }',
    )
    .replace(/asyncWorkPoolSize: 4,\n\s*/g, 'asyncWorkPoolSize: 0,\n  ')
    .replace(/asyncWorkPoolSize: \(function\(\) \{[\s\S]*?\n  \}\)\(\),\n\s*/g, 'asyncWorkPoolSize: 0,\n  ')
    .replace(/reuseWorker: true,\n\s*/g, '')
    .replace(/\n  onCreateWorker\(\) \{[\s\S]*?\n  \},\n  overwriteImports/, '\n  overwriteImports');

  if (out.includes('shared: true')) {
    throw new Error('failed to remove shared memory from single-threaded shim');
  }
  if (out.includes('reuseWorker: true') || out.includes('onCreateWorker()')) {
    throw new Error('failed to remove worker-pool setup from single-threaded shim');
  }
  return out;
}

const wasmModuleImport = "import __wasmModule from './coverage-instrument.wasm32-wasi.wasm'\n";
const wasmFileFromModule = `const __wasmFile =
  __wasmModule instanceof WebAssembly.Module
    ? __wasmModule
    : typeof __wasmModule === 'string'
      ? await fetch(__wasmModule).then((res) => res.arrayBuffer())
      : __wasmModule`;

export function patchSingleThreadedBrowserShim(source) {
  let out = patchSingleThreadedShim(source);

  // napi-rs 3.9 shape: fetch the wasm URL, check the response, then read it.
  // Workers cannot fetch a relative module URL, so import the wasm module and
  // let the bundler provide it.
  const fetchResponseBlock =
    /const __wasmUrl = new URL\('\.\/coverage-instrument\.wasm32-wasi\.wasm', import\.meta\.url\)\.href\nconst __wasmResponse = await globalThis\.fetch\(__wasmUrl\)\n[\s\S]*?\nconst __wasmFile = await __wasmResponse\.arrayBuffer\(\)/;
  if (!out.includes(wasmModuleImport) && fetchResponseBlock.test(out)) {
    out = out
      .replace(/} from '@napi-rs\/wasm-runtime'\n/, `} from '@napi-rs/wasm-runtime'\n${wasmModuleImport}`)
      .replace(fetchResponseBlock, wasmFileFromModule);
  }

  if (out.includes('from \'@napi-rs/wasm-runtime\'') && out.includes('await fetch(__wasmUrl)')) {
    out = out
      .replace(
        /} from '@napi-rs\/wasm-runtime'\n/,
        `} from '@napi-rs/wasm-runtime'\n${wasmModuleImport}`,
      )
      .replace(
        /const __wasmUrl = new URL\('\.\/coverage-instrument\.wasm32-wasi\.wasm', import\.meta\.url\)\.href\n/,
        '',
      )
      .replace(
        /const __wasmFile = await fetch\(__wasmUrl\)\.then\(\(res\) => res\.arrayBuffer\(\)\)/,
        wasmFileFromModule,
      );
  }

  if (!out.includes(wasmModuleImport)) {
    throw new Error('failed to replace the wasm fetch with a wasm module import in the browser shim');
  }
  return out;
}

function patchFile(path, patcher) {
  if (!existsSync(path)) {
    throw new Error(`missing required file: ${path}`);
  }
  const original = readFileSync(path);
  const next = patcher(original);
  writeFileSync(path, next);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const moved = normalizeWasip1Outputs(targetDir);
  if (moved > 0) {
    console.log(`[patch-wasi-singlethreaded-artifacts] renamed ${moved} wasip1 output file(s).`);
  }

  const wasmPath = join(targetDir, 'coverage-instrument.wasm32-wasi.wasm');
  patchFile(wasmPath, (buffer) => {
    const result = patchSingleThreadedWasm(buffer);
    if (result.patched) {
      console.log(`[patch-wasi-singlethreaded-artifacts] made wasm global #${result.globalIndex} mutable.`);
    } else {
      console.log('[patch-wasi-singlethreaded-artifacts] wasm already validates.');
    }
    return result.buffer;
  });

  patchFile(join(targetDir, 'coverage-instrument.wasi.cjs'), (buffer) =>
    patchSingleThreadedShim(buffer.toString('utf8')),
  );
  patchFile(join(targetDir, 'coverage-instrument.wasi-browser.js'), (buffer) =>
    patchSingleThreadedBrowserShim(buffer.toString('utf8')),
  );

  console.log(`[patch-wasi-singlethreaded-artifacts] patched ${targetDir}.`);
}
