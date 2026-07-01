/**
 * Options for creating an Oxc-based coverage instrumenter.
 *
 * When used with Vitest's `coverage.instrumenter` option, the factory receives
 * `InstrumenterOptions` with `coverageVariable` and `ignoreClassMethods`.
 */
export interface OxcInstrumenterOptions {
  /** Global variable name for coverage data (Vitest passes `__VITEST_COVERAGE__`). */
  coverageVariable?: string;
  /** Class method names to exclude from coverage instrumentation. */
  ignoreClassMethods?: string[];
  /** When true, adds truthy-value tracking (bT) for logical expressions. */
  reportLogic?: boolean;
  /**
   * Attach the optional `x_fallow_functionMap` extension to the last file
   * coverage object. Defaults to false.
   */
  functionIdentityOverlay?: boolean;
  /**
   * Name an otherwise-anonymous function/arrow that is a direct call/`new`
   * argument from the callee (`arr.map(cb)` -> `"map"`), instead of the
   * `(anonymous_N)` fallback. Binding names still take precedence. Defaults to
   * false (byte-identical to Istanbul).
   */
  nameCallbackArguments?: boolean;
  /**
   * Run the TypeScript-strip pass before instrumentation. When omitted
   * (default), the adapter auto-detects: it strips when the filename matches
   * `/\.([mc]ts|tsx?)$/i` AND no `inputSourceMap` was supplied (i.e., the
   * source has not already been transformed by Vite / Babel / tsc). Set to
   * `false` to disable auto-detect under toolchains that pre-transform
   * TypeScript but do not produce an `inputSourceMap`. Set to `true` to force
   * strip. Non-boolean values throw `TypeError`.
   */
  stripTypescript?: boolean;
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
  instrumentSync(code: string, filename: string, inputSourceMap?: any): string;
  lastSourceMap(): any;
  lastFileCoverage(): any;
  /** Property alias for compatibility with vite-plugin-istanbul. */
  readonly fileCoverage: any;
};
