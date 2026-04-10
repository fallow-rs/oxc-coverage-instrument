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
//         instrumenter: createOxcInstrumenter(),
//       }
//     }
//   })

const { instrument } = require('./index.js');

/**
 * Creates an instrumenter that implements the istanbul-lib-instrument
 * Instrumenter interface, backed by oxc-coverage-instrument.
 *
 * @param {object} [options]
 * @param {string} [options.coverageVariable='__coverage__']
 * @param {string[]} [options.ignoreClassMethods=[]]
 * @param {boolean} [options.reportLogic=false]
 * @returns {Instrumenter}
 */
function createOxcInstrumenter(options = {}) {
  const coverageVariable = options.coverageVariable || '__coverage__';
  const ignoreClassMethods = options.ignoreClassMethods || [];
  const reportLogic = options.reportLogic || false;

  let _lastSourceMap = null;
  let _lastFileCoverage = null;

  return {
    /**
     * Instrument source code synchronously.
     * @param {string} code - Source code to instrument.
     * @param {string} filename - File path for coverage map.
     * @param {object} [inputSourceMap] - Input source map from prior transform.
     * @returns {string} Instrumented source code.
     */
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

    /**
     * Get the source map of the last instrumented file.
     * @returns {object|null}
     */
    lastSourceMap() {
      return _lastSourceMap;
    },

    /**
     * Get the coverage object of the last instrumented file.
     * @returns {object}
     */
    lastFileCoverage() {
      return _lastFileCoverage;
    },
  };
}

module.exports = { createOxcInstrumenter };
