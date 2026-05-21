// Vitest Istanbul instrumenter adapter for oxc-coverage-instrument.
//
// Implements the istanbul-lib-instrument Instrumenter interface so it can be
// used as a drop-in replacement in @vitest/coverage-istanbul.
//
// Usage in vitest.config.ts:
//
//   import { defineConfig } from 'vitest/config'
//   import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest'
//
//   export default defineConfig({
//     test: {
//       coverage: {
//         provider: 'istanbul',
//         instrumenter: (options) => createOxcInstrumenter(options),
//       }
//     }
//   })

const { instrument } = require('./index.js');

// Filename pattern matching exactly the TypeScript extensions Node recognises:
// .ts, .tsx, .mts, .cts. The narrower form `/\.([mc]ts|tsx?)$/i` is preferred
// over `/\.[mc]?tsx?$/i` because the latter would also match `.mtsx` and
// `.ctsx`, which are not real TypeScript extensions. Used for auto-detect;
// explicit user opt-in/out via the `stripTypescript` option always wins.
const TS_EXTENSION_REGEX = /\.([mc]ts|tsx?)$/i;

/**
 * Creates an instrumenter that implements the istanbul-lib-instrument
 * Instrumenter interface, backed by oxc-coverage-instrument.
 *
 * When used with Vitest's `coverage.instrumenter` option, the factory receives
 * `InstrumenterOptions` with `coverageVariable` and `ignoreClassMethods`.
 * These are forwarded to the native instrumenter automatically.
 *
 * @param {object} [options]
 * @param {string} [options.coverageVariable] Global variable for coverage data.
 *   Vitest passes its internal `__VITEST_COVERAGE__`; defaults to `__coverage__`.
 * @param {string[]} [options.ignoreClassMethods] Class method names to skip.
 * @param {boolean} [options.reportLogic] Enable truthy-value tracking (bT).
 * @param {boolean} [options.stripTypescript] Run the TypeScript-strip pass
 *   before instrumentation. When omitted (default), the adapter auto-detects:
 *   it strips when the filename matches `/\.([mc]ts|tsx?)$/i` AND no
 *   `inputSourceMap` was supplied (i.e., the source has not already been
 *   transformed by Vite / Babel / tsc). Set to `false` to disable auto-detect
 *   under toolchains that pre-transform TypeScript but do not produce an
 *   `inputSourceMap` (`@vitejs/plugin-react-swc` in some configurations, Bun's
 *   native TS runner, Node 23+ with `--experimental-strip-types`). Set to
 *   `true` to force strip regardless of filename or `inputSourceMap` presence.
 *   Non-boolean values throw `TypeError`: the `?: boolean` type contract is
 *   strict because callers passing a string like `'auto'` (a prior design
 *   discussion shape) would otherwise be silently coerced to `Boolean('auto')`
 *   which is `true`, force-stripping every file.
 * @param {boolean} [options.experimentalDecorators] Lower legacy
 *   `experimentalDecorators` syntax into `_decorate(...)` calls (matches the
 *   `experimentalDecorators` flag in `tsconfig.json`). Defaults to false; pass
 *   true for NestJS / Angular / class-validator / TypeORM. Only active when
 *   the strip pass runs. Requires `@oxc-project/runtime` at execution; see the
 *   README's "Legacy decorators" section. No auto-detect from `tsconfig.json`
 *   in this version; set explicitly when you need it.
 * @param {boolean} [options.emitDecoratorMetadata] Emit
 *   `_decorateMetadata("design:type" / "design:paramtypes" / "design:returntype", ...)`
 *   alongside each decorated member (matches `emitDecoratorMetadata` in
 *   `tsconfig.json`). Required for NestJS DI and TypeORM column type inference.
 *   Setting this to true auto-promotes `experimentalDecorators` to true at
 *   this layer, matching tsconfig.json semantics. The bare `instrument()`
 *   napi entry point rejects the same combination with an Error; the vitest
 *   adapter preserves the tsconfig-style ergonomics for the vitest UX.
 * @returns {{ instrumentSync, lastSourceMap, lastFileCoverage }}
 */
function createOxcInstrumenter(options) {
  options = options || {};
  const coverageVariable = options.coverageVariable || '__coverage__';
  const ignoreClassMethods = options.ignoreClassMethods || [];
  const reportLogic = options.reportLogic || false;
  const stripTypescriptOverride = options.stripTypescript;
  if (stripTypescriptOverride !== undefined && typeof stripTypescriptOverride !== 'boolean') {
    throw new TypeError(
      `oxc-coverage-instrument: createOxcInstrumenter({ stripTypescript }) must be a boolean or undefined, got ${typeof stripTypescriptOverride}`,
    );
  }
  const experimentalDecoratorsRaw = options.experimentalDecorators || false;
  if (typeof experimentalDecoratorsRaw !== 'boolean') {
    throw new TypeError(
      `oxc-coverage-instrument: createOxcInstrumenter({ experimentalDecorators }) must be a boolean or undefined, got ${typeof options.experimentalDecorators}`,
    );
  }
  const emitDecoratorMetadata = options.emitDecoratorMetadata || false;
  if (typeof emitDecoratorMetadata !== 'boolean') {
    throw new TypeError(
      `oxc-coverage-instrument: createOxcInstrumenter({ emitDecoratorMetadata }) must be a boolean or undefined, got ${typeof options.emitDecoratorMetadata}`,
    );
  }
  // Auto-promote at the vitest layer: setting emitDecoratorMetadata alone is a
  // tsconfig.json idiom (TypeScript itself silently enables experimentalDecorators
  // in that case). The bare napi `instrument()` rejects the same combination so
  // the underlying Rust DecoratorMode enum stays unrepresentable-invalid; this
  // adapter is the place to mirror tsconfig ergonomics for vitest users.
  const experimentalDecorators = experimentalDecoratorsRaw || emitDecoratorMetadata;

  // Raw JSON strings from the last instrument call — parsed lazily on first access.
  let _lastCoverageMapJson = null;
  let _lastSourceMapJson = null;
  // Parsed objects — null until actually read via lastFileCoverage()/lastSourceMap().
  let _lastFileCoverage = null;
  let _lastSourceMap = null;
  // Dirty flags — set to true after each instrumentSync to invalidate cached parses.
  let _coverageDirty = false;
  let _sourceMapDirty = false;

  function ensureCoverageParsed() {
    if (_coverageDirty && _lastCoverageMapJson !== null) {
      _lastFileCoverage = JSON.parse(_lastCoverageMapJson);
      _coverageDirty = false;
    }
  }

  function ensureSourceMapParsed() {
    if (_sourceMapDirty && _lastSourceMapJson !== null) {
      _lastSourceMap = JSON.parse(_lastSourceMapJson);
      _sourceMapDirty = false;
    }
  }

  return {
    instrumentSync(code, filename, inputSourceMap) {
      const stripTypescript =
        stripTypescriptOverride === undefined
          ? TS_EXTENSION_REGEX.test(filename) && !inputSourceMap
          : stripTypescriptOverride;
      const result = instrument(code, filename, {
        coverageVariable,
        sourceMap: true,
        inputSourceMap: inputSourceMap ? JSON.stringify(inputSourceMap) : undefined,
        reportLogic,
        ignoreClassMethods,
        stripTypescript,
        experimentalDecorators,
        emitDecoratorMetadata,
      });

      // Store raw JSON — defer parsing until actually needed.
      _lastCoverageMapJson = result.coverageMap;
      _lastSourceMapJson = result.sourceMap || null;
      _coverageDirty = true;
      _sourceMapDirty = true;

      // result.code is a Buffer (zero-copy from Rust); convert to string for
      // istanbul-lib-instrument compatibility (instrumentSync must return string).
      return result.code.toString();
    },

    lastSourceMap() {
      ensureSourceMapParsed();
      return _lastSourceMap;
    },

    lastFileCoverage() {
      ensureCoverageParsed();
      return _lastFileCoverage;
    },

    // Property alias used by vite-plugin-istanbul (vs lastFileCoverage() method used by Vitest)
    get fileCoverage() {
      ensureCoverageParsed();
      return _lastFileCoverage;
    },
  };
}

module.exports = { createOxcInstrumenter };
