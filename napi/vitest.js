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

/**
 * Creates an instrumenter that implements the istanbul-lib-instrument
 * Instrumenter interface, backed by oxc-coverage-instrument.
 *
 * When used with Vitest's `coverage.instrumenter` option, the factory receives
 * `InstrumenterOptions` with `coverageVariable` and `ignoreClassMethods`.
 * These are forwarded to the native instrumenter automatically.
 *
 * @param {object} [options]
 * @param {string} [options.coverageVariable] - Global variable for coverage data.
 *   Vitest passes its internal `__VITEST_COVERAGE__`; defaults to `__coverage__`.
 * @param {string[]} [options.ignoreClassMethods] - Class methods to skip.
 * @param {boolean} [options.reportLogic] - Enable truthy-value tracking (bT).
 * @returns {{ instrumentSync, lastSourceMap, lastFileCoverage }}
 */
function createOxcInstrumenter(options) {
  options = options || {};
  const coverageVariable = options.coverageVariable || '__coverage__';
  const ignoreClassMethods = options.ignoreClassMethods || [];
  const reportLogic = options.reportLogic || false;

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
      const result = instrument(code, filename, {
        coverageVariable,
        sourceMap: true,
        inputSourceMap: inputSourceMap ? JSON.stringify(inputSourceMap) : undefined,
        reportLogic,
        ignoreClassMethods,
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
