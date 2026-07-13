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
   * Track each optional chaining (`?.`) link as a branch. Defaults to true.
   * Set to false to leave optional chains native with no `_oc` helper or
   * `optional-chain` branch, matching `istanbul-lib-instrument` and avoiding
   * per-operand helper calls in `?.`-dense hot paths.
   */
  trackOptionalChainBranches?: boolean;
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
  /**
   * Lower legacy `experimentalDecorators` syntax into `_decorate(...)` calls.
   * Defaults to false and only applies when the TypeScript strip pass runs.
   * Requires `@oxc-project/runtime` at execution. This option is not detected
   * from `tsconfig.json` and must be enabled explicitly.
   */
  experimentalDecorators?: boolean;
  /**
   * Emit `_decorateMetadata` type metadata alongside decorated members.
   * Required for NestJS dependency injection and TypeORM column inference.
   * Setting this to true auto-promotes `experimentalDecorators` to true at
   * this adapter layer, matching `tsconfig.json` semantics.
   */
  emitDecoratorMetadata?: boolean;
}

/** A JSON primitive accepted in an input source map. */
export type JsonPrimitive = string | number | boolean | null;

/** A JSON-compatible value accepted in an input source map. */
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];

/** A JSON-compatible object accepted as an input source map. */
export interface JsonObject {
  [key: string]: JsonValue;
}

/** A source map returned by the instrumenter. */
export interface OxcSourceMap {
  [key: string]: JsonValue | undefined;
  version: number;
  sources: string[];
  names: string[];
  mappings: string;
  file?: string;
  sourceRoot?: string;
  sourcesContent?: Array<string | null>;
}

/** A position in an Istanbul coverage location. */
export interface IstanbulPosition {
  line?: number;
  column?: number;
}

/** A source range in Istanbul coverage data. */
export interface IstanbulLocation {
  start: IstanbulPosition;
  end: IstanbulPosition;
}

/** Function metadata in Istanbul coverage data. */
export interface IstanbulFunction {
  name: string;
  line: number;
  decl: IstanbulLocation;
  loc: IstanbulLocation;
}

/** Branch metadata in Istanbul coverage data. */
export interface IstanbulBranch {
  loc: IstanbulLocation;
  line: number;
  type: string;
  locations: IstanbulLocation[];
}

/** Coverage data for one instrumented file. */
export interface IstanbulFileCoverage {
  path: string;
  statementMap: Record<string, IstanbulLocation>;
  fnMap: Record<string, IstanbulFunction>;
  branchMap: Record<string, IstanbulBranch>;
  s: Record<string, number>;
  f: Record<string, number>;
  b: Record<string, number[]>;
  bT?: Record<string, number[]>;
  inputSourceMap?: JsonValue;
  x_fallow_functionMap?: Record<string, JsonObject>;
}

/** Instrumenter returned by `createOxcInstrumenter`. */
export interface OxcInstrumenter {
  instrumentSync(code: string, filename: string, inputSourceMap?: JsonObject): string;
  lastSourceMap(): OxcSourceMap | null;
  lastFileCoverage(): IstanbulFileCoverage | null;
  /** Property alias for compatibility with vite-plugin-istanbul. */
  readonly fileCoverage: IstanbulFileCoverage | null;
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
export declare function createOxcInstrumenter(options?: OxcInstrumenterOptions): OxcInstrumenter;
