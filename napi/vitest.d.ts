/**
 * Creates an instrumenter that implements the istanbul-lib-instrument
 * Instrumenter interface, backed by oxc-coverage-instrument.
 *
 * Use with Vitest's `coverage.instrumenter` option (requires vitest with
 * custom instrumenter support):
 *
 * ```ts
 * import { defineConfig } from 'vitest/config'
 * import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest'
 *
 * export default defineConfig({
 *   test: {
 *     coverage: {
 *       provider: 'istanbul',
 *       instrumenter: () => createOxcInstrumenter(),
 *     }
 *   }
 * })
 * ```
 */
export declare function createOxcInstrumenter(options?: {
  /** Name of the global coverage variable (default: "__coverage__"). */
  coverageVariable?: string
  /** Class method names to exclude from function coverage. */
  ignoreClassMethods?: string[]
  /** When true, adds truthy-value tracking (bT) for logical expressions. */
  reportLogic?: boolean
}): {
  instrumentSync(code: string, filename: string, inputSourceMap?: any): string
  lastSourceMap(): any
  lastFileCoverage(): any
}
