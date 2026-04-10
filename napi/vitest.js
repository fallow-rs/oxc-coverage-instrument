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

  let _lastSourceMap = null;
  let _lastFileCoverage = null;

  return {
    instrumentSync(code, filename, inputSourceMap) {
      const result = instrument(code, filename, {
        coverageVariable,
        sourceMap: true,
        inputSourceMap: inputSourceMap ? JSON.stringify(inputSourceMap) : undefined,
        reportLogic,
        ignoreClassMethods,
      });

      _lastFileCoverage = JSON.parse(result.coverageMap);
      _lastSourceMap = result.sourceMap ? JSON.parse(result.sourceMap) : null;

      return result.code;
    },

    lastSourceMap() {
      return _lastSourceMap;
    },

    lastFileCoverage() {
      return _lastFileCoverage;
    },
  };
}

module.exports = { createOxcInstrumenter };
