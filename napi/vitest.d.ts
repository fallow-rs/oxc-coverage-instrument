/**
 * Options for creating an Oxc-based coverage instrumenter.
 *
 * When used with Vitest's `coverage.instrumenter` option, the factory receives
 * `InstrumenterOptions` with `coverageVariable` and `ignoreClassMethods`.
 */
export interface OxcInstrumenterOptions {
  /** Global variable name for coverage data (Vitest passes `__VITEST_COVERAGE__`). */
  coverageVariable?: string
  /** Class method names to exclude from function coverage. */
  ignoreClassMethods?: string[]
  /** When true, adds truthy-value tracking (bT) for logical expressions. */
  reportLogic?: boolean
}

/**
 * Creates an instrumenter that implements the istanbul-lib-instrument
 * Instrumenter interface, backed by oxc-coverage-instrument.
 *
 * @example
 * ```ts
 * import { defineConfig } from 'vitest/config'
 * import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest'
 *
 * export default defineConfig({
 *   test: {
 *     coverage: {
 *       provider: 'istanbul',
 *       instrumenter: (options) => createOxcInstrumenter(options),
 *     }
 *   }
 * })
 * ```
 */
export declare function createOxcInstrumenter(options?: OxcInstrumenterOptions): {
  instrumentSync(code: string, filename: string, inputSourceMap?: any): string
  lastSourceMap(): any
  lastFileCoverage(): any
}
