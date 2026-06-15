//! Top-level instrumentation API.

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_transformer::{
    DecoratorOptions, JsxOptions, TransformOptions, Transformer, TypeScriptOptions,
};
use oxc_traverse::traverse_mut;

use std::collections::BTreeMap;

use crate::coverage_builder::{CoverageMaps, build_file_coverage, build_function_identity_map};
use crate::pragma::PragmaMap;
use crate::transform::{
    CoverageState, CoverageTransform, PreambleInputs, TransformInit, djb31_hex,
    generate_cov_fn_name, generate_preamble_source,
};
use oxc_coverage_types::{FileCoverage, UnhandledPragma};

/// Options for the `instrument` function.
#[derive(Debug, Clone)]
pub struct InstrumentOptions {
    /// Name of the global coverage variable (default: `"__coverage__"`).
    pub coverage_variable: String,
    /// Whether to generate a source map for the instrumented output.
    pub source_map: bool,
    /// Input source map JSON string from a prior transformation (e.g., TypeScript → JS).
    /// When provided, this is stored on the `FileCoverage` as `inputSourceMap` so
    /// downstream tools (nyc, istanbul-reports) can chain back to the original source.
    pub input_source_map: Option<String>,
    /// When true AND [`InstrumentOptions::input_source_map`] is set, compose the
    /// input source map into the coverage map during instrumentation instead of
    /// embedding it for downstream composition.
    ///
    /// The resulting `FileCoverage` (and the `coverageData` literal baked into
    /// the instrumented code's preamble, hence the runtime coverage variable)
    /// carries original-source positions, is re-keyed by the original source
    /// `path`, and has no `inputSourceMap` field. A subsequent
    /// [`crate::remap_coverage`] / `remapCoverageMap` on the result is a no-op.
    ///
    /// This trades the per-collection remap round-trip (instrument, then walk
    /// every entry through its embedded map at report time) for a one-time
    /// composition at instrument time. Useful for E2E collectors (Playwright et
    /// al.) that dump `window.__coverage__` directly and want original-source
    /// positions without a normalization pass.
    ///
    /// In eager mode a coverage point whose positions do not remap through the
    /// input source map is NOT instrumented at all: it gets no `statementMap` /
    /// `fnMap` / `branchMap` entry AND no counter in the emitted code. The
    /// runtime `__coverage__` object and the emitted counters therefore always
    /// agree (no dangling `++cov.b[id][...]` against a pruned slot). Composition
    /// itself is then a pure remap of the surviving positions to original
    /// coordinates, so it never emits past-EOF entries (e.g. compiler
    /// boilerplate in a Vue SFC chunk that has no mapping back to the `.vue` is
    /// simply never instrumented). If the input map is unusable (declares no
    /// source, fails to parse), the gate is off and the embedded
    /// `inputSourceMap` is left in place so the lazy remap path still works. Has
    /// no effect when `input_source_map` is `None`.
    ///
    /// Defaults to false.
    pub compose_input_source_map: bool,
    /// When true, adds truthy-value tracking (`bT`) for logical expression operands.
    /// This enables nyc-style logic coverage that tracks not just which branch was
    /// taken, but whether each operand evaluated to a truthy value.
    pub report_logic: bool,
    /// When true (the default), each optional-chaining (`?.`) link is tracked as
    /// an `optional-chain` branch: its operand is wrapped in a `cov_fn_oc`
    /// helper call that records whether the value was nullish. This is more
    /// complete than `istanbul-lib-instrument`, which does not track `?.` as a
    /// branch.
    ///
    /// Set to false to leave optional chains native: no `cov_fn_oc` helper is
    /// emitted and no `optional-chain` branch is registered. This matches
    /// `istanbul-lib-instrument` byte-for-byte on `?.` and avoids the
    /// per-operand helper-call overhead in optional-chain-dense hot paths
    /// (issue #108). Statement, function, and other branch coverage are
    /// unaffected.
    ///
    /// Defaults to true so existing behavior is unchanged.
    pub track_optional_chain: bool,
    /// Class method names to exclude from coverage instrumentation.
    /// Matches Istanbul's `ignoreClassMethods` behavior for class methods and
    /// named function expressions with a matching id.
    pub ignore_class_methods: Vec<String>,
    /// When true, run `oxc_transformer`'s TypeScript-strip pass on the parsed
    /// AST before coverage instrumentation. Set this when passing raw
    /// TypeScript source that has not been pre-transformed by Babel /
    /// tsc / esbuild.
    ///
    /// Output: instrumented JavaScript whose `statementMap` / `branchMap`
    /// positions reference the original TypeScript byte offsets (surviving
    /// AST nodes retain their `Span` through the strip pass).
    ///
    /// Defaults to false so existing Vitest / nyc callers that supply
    /// already-transformed JavaScript are unaffected. **If false and you
    /// pass raw TypeScript, the output will contain TypeScript syntax and
    /// will not be executable as JavaScript** (no error is returned).
    ///
    /// Decorator handling: by default, decorator syntax (Stage 3 and legacy
    /// `experimentalDecorators` alike) flows through unchanged. NestJS /
    /// Angular / TypeORM users who need `@Injectable()` / `@Controller()`
    /// classes lowered into `_decorate(...)` calls (with or
    /// without `design:type` / `design:paramtypes` metadata) should set
    /// [`InstrumentOptions::decorator_mode`] to
    /// [`DecoratorMode::Experimental`] or
    /// [`DecoratorMode::ExperimentalWithMetadata`].
    ///
    /// JSX is preserved verbatim on `.tsx` files (the codegen pass emits
    /// it unchanged).
    pub strip_typescript: bool,
    /// How decorator syntax is handled by the strip pass. See
    /// [`DecoratorMode`] for the variants and their semantics. Has no effect
    /// unless `strip_typescript` is also true.
    ///
    /// Defaults to [`DecoratorMode::PassThrough`]: decorator syntax flows
    /// through verbatim and a downstream tool is responsible for lowering it.
    pub decorator_mode: DecoratorMode,
    /// When true, attach an optional `x_fallow_functionMap` overlay to the
    /// resulting `FileCoverage`. The overlay carries a stable
    /// `fallow:fn:<hex>` identity per function, keyed by the same ids as
    /// `fnMap`, derived from `(path, name, decl span, loc span)`. Standard
    /// Istanbul consumers ignore the `x_`-prefixed field; downstream
    /// code-quality tools (Fallow et al.) use it to join AST inventories,
    /// runtime coverage, and source-mapped positions across runs without
    /// reconstructing identity from `(path, name, line, column)` after the
    /// fact.
    ///
    /// Defaults to false. The default JSON output stays byte-identical to
    /// what Istanbul consumers expect.
    pub function_identity_overlay: bool,
}

/// How `strip_typescript` handles decorator syntax.
///
/// The Rust API uses a single enum so invalid combinations (e.g. "emit
/// metadata without lowering decorators") are unrepresentable; the
/// upstream `oxc_transformer` decorator pass is gated on legacy-mode being
/// on, and metadata emission is only meaningful when lowering is active.
///
/// The napi surface keeps the two-optional-boolean shape
/// (`experimentalDecorators` + `emitDecoratorMetadata`) familiar from
/// `tsconfig.json` and reconstructs this enum on the adapter side,
/// returning a JS `Error` for the invalid combination instead of silently
/// promoting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecoratorMode {
    /// Decorator syntax flows through verbatim. Matches v0.6.x behavior.
    /// A downstream tool (Babel, tsc, esbuild) is expected to lower it.
    #[default]
    PassThrough,
    /// Lower TypeScript `experimentalDecorators` syntax (the
    /// `@Injectable()` / `@Controller()` style used by NestJS, Angular,
    /// class-validator, TypeORM) into runtime `_decorate(...)` calls. No
    /// `design:type` / `design:paramtypes` / `design:returntype` metadata
    /// is emitted. Mirrors `experimentalDecorators: true` +
    /// `emitDecoratorMetadata: false` in `tsconfig.json`.
    ///
    /// The transformer emits ES module imports from
    /// `@oxc-project/runtime/helpers/*` at the top of the file; consumers
    /// must install `@oxc-project/runtime` (or provide an equivalent
    /// shim). See the README for details and troubleshooting.
    Experimental,
    /// Lower experimental decorators AND emit TypeScript-style decorator
    /// metadata (`design:type`, `design:paramtypes`, `design:returntype`)
    /// as `_decorateMetadata(...)` calls alongside each decorated class /
    /// method / property / accessor. Required for NestJS dependency
    /// injection, TypeORM column type inference, and class-validator's
    /// metadata-driven validation. Mirrors `experimentalDecorators: true`
    /// + `emitDecoratorMetadata: true` in `tsconfig.json`.
    ///
    /// Same `@oxc-project/runtime` requirement as [`Self::Experimental`].
    ExperimentalWithMetadata,
}

impl DecoratorMode {
    /// Whether legacy decorator lowering should be enabled on the upstream
    /// transformer (i.e. `DecoratorOptions::legacy`).
    #[must_use]
    pub const fn legacy(self) -> bool {
        matches!(self, Self::Experimental | Self::ExperimentalWithMetadata)
    }

    /// Whether `design:type` / `design:paramtypes` / `design:returntype`
    /// metadata should be emitted alongside lowered decorators.
    #[must_use]
    pub const fn emit_metadata(self) -> bool {
        matches!(self, Self::ExperimentalWithMetadata)
    }
}

impl Default for InstrumentOptions {
    fn default() -> Self {
        Self {
            coverage_variable: "__coverage__".to_string(),
            source_map: false,
            input_source_map: None,
            compose_input_source_map: false,
            report_logic: false,
            track_optional_chain: true,
            ignore_class_methods: Vec::new(),
            strip_typescript: false,
            decorator_mode: DecoratorMode::PassThrough,
            function_identity_overlay: false,
        }
    }
}

/// Result of instrumenting a source file.
#[derive(Debug)]
pub struct InstrumentResult {
    /// The instrumented source code with coverage counters injected.
    pub code: String,
    /// Istanbul-compatible coverage map for this file.
    pub coverage_map: FileCoverage,
    /// Pre-serialized JSON of `coverage_map`. Produced once internally for the
    /// preamble's `coverageData` literal and the hash guard, then exposed here
    /// so language bindings (napi-rs, etc.) and downstream JSON sinks can avoid
    /// a second serialization of the same `BTreeMap` tree.
    pub coverage_map_json: String,
    /// Output source map JSON string (only present if `InstrumentOptions::source_map` is true).
    pub source_map: Option<String>,
    /// Unhandled pragma comments found during instrumentation.
    /// Contains `/* istanbul ignore ... */` and `/* v8 ignore ... */` comments
    /// that were not processed. Callers should decide whether to warn or error.
    pub unhandled_pragmas: Vec<UnhandledPragma>,
}

/// Check whether a string is a valid JavaScript identifier (ASCII subset).
///
/// Returns `true` if the string is non-empty, starts with `[a-zA-Z_$]`,
/// and all remaining characters are `[a-zA-Z0-9_$]`.
fn is_valid_js_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Serialize a `FileCoverage` to JSON.
///
/// `FileCoverage` is composed of `BTreeMap`, `Vec`, `String`, and primitive
/// numbers, all with first-party serde implementations that cannot fail at
/// runtime. The .expect call documents this rather than threading a
/// never-produced error variant through the call chain.
fn serialize_coverage_map(coverage_map: &FileCoverage) -> String {
    serde_json::to_string(coverage_map).expect("FileCoverage serializes to JSON infallibly")
}

/// Instrument a JavaScript/TypeScript source file for coverage collection.
///
/// Parses the source with `oxc_parser`, collects statement/function/branch
/// locations via AST traversal, injects coverage counter expressions into
/// the AST, and emits the instrumented code via `oxc_codegen`.
///
/// # Errors
///
/// Returns an error if the source cannot be parsed.
///
/// # Example
///
/// ```
/// use oxc_coverage_instrument::{instrument, InstrumentOptions};
///
/// let source = "function add(a, b) { return a + b; }";
/// let result = instrument(source, "add.js", &InstrumentOptions::default()).unwrap();
///
/// // coverage_map contains fnMap, statementMap, branchMap
/// assert_eq!(result.coverage_map.fn_map.len(), 1);
/// assert_eq!(result.coverage_map.fn_map["0"].name, "add");
/// ```
pub fn instrument(
    source: &str,
    filename: &str,
    options: &InstrumentOptions,
) -> Result<InstrumentResult, InstrumentError> {
    validate_coverage_variable(options)?;
    let allocator = Allocator::default();
    let mut parsed = parse_program(&allocator, source, filename)?;

    let (pragmas, unhandled_pragmas) = PragmaMap::from_program(&parsed.program, source);
    if pragmas.ignore_file {
        return Ok(empty_coverage_result(filename, source, unhandled_pragmas));
    }

    let scoping = prepare_scoping(&allocator, filename, &mut parsed.program, options)?;
    let cov_fn_name = generate_cov_fn_name(filename);

    let (transform, scoping) = run_coverage_transform(
        &allocator,
        &mut parsed.program,
        scoping,
        pragmas,
        source,
        &cov_fn_name,
        options,
    );

    let coverage_map = finalize_coverage_map(filename, transform, options);

    let (coverage_json, preamble) = build_instrument_preamble(&coverage_map, options, &cov_fn_name);

    let (code, source_map) =
        emit_instrument_output(&parsed.program, scoping, source, filename, &preamble, options);

    Ok(InstrumentResult {
        code,
        coverage_map,
        coverage_map_json: coverage_json,
        source_map,
        unhandled_pragmas,
    })
}

fn validate_coverage_variable(options: &InstrumentOptions) -> Result<(), InstrumentError> {
    if is_valid_js_identifier(&options.coverage_variable) {
        return Ok(());
    }
    Err(InstrumentError::InvalidCoverageVariable(options.coverage_variable.clone()))
}

fn emit_instrument_output(
    program: &Program<'_>,
    scoping: Scoping,
    source: &str,
    filename: &str,
    preamble: &str,
    options: &InstrumentOptions,
) -> (String, Option<String>) {
    let (code, raw_source_map) =
        emit_code(EmitInputs { program, scoping, source, filename, preamble, options });
    let source_map = finalize_emitted_source_map(raw_source_map.as_ref(), preamble, options);
    (code, source_map)
}

fn prepare_scoping<'a>(
    allocator: &'a Allocator,
    filename: &str,
    program: &mut Program<'a>,
    options: &InstrumentOptions,
) -> Result<Scoping, InstrumentError> {
    let scoping = SemanticBuilder::new().build(program).semantic.into_scoping();
    if !options.strip_typescript {
        return Ok(scoping);
    }
    strip_typescript_pass(allocator, filename, program, scoping, options.decorator_mode)
}

fn run_coverage_transform<'src, 'arena>(
    allocator: &'arena Allocator,
    program: &mut Program<'arena>,
    scoping: Scoping,
    pragmas: PragmaMap,
    source: &'src str,
    cov_fn_name: &'src str,
    options: &InstrumentOptions,
) -> (CoverageTransform<'src, 'arena>, Scoping) {
    let mut transform = CoverageTransform::new(TransformInit {
        allocator,
        source,
        cov_fn_name,
        report_logic: options.report_logic,
        track_optional_chain: options.track_optional_chain,
        ignore_class_methods: options.ignore_class_methods.clone(),
        eager_remapper: eager_remapper(options),
    });
    let state = CoverageState { pragmas };
    let scoping = traverse_mut(&mut transform, allocator, program, scoping, state);
    (transform, scoping)
}

fn build_instrument_preamble(
    coverage_map: &FileCoverage,
    options: &InstrumentOptions,
    cov_fn_name: &str,
) -> (String, String) {
    // Serialize once and reuse it for both the hash guard and the coverageData
    // literal. The hash is computed over the same JSON embedded in the preamble.
    let coverage_json = serialize_coverage_map(coverage_map);
    let coverage_hash = djb31_hex(&coverage_json);
    let preamble = generate_preamble_source(&PreambleInputs {
        coverage: coverage_map,
        coverage_json: &coverage_json,
        coverage_hash: &coverage_hash,
        coverage_var: &options.coverage_variable,
        cov_fn_name,
        report_logic: options.report_logic,
    });
    (coverage_json, preamble)
}

fn finalize_emitted_source_map(
    raw_source_map: Option<&String>,
    preamble: &str,
    options: &InstrumentOptions,
) -> Option<String> {
    raw_source_map
        .as_ref()
        .map(|sm| finalize_source_map(sm, preamble, options.input_source_map.as_deref()))
}

/// Run `oxc_transformer`'s TypeScript-strip pass on the parsed program in
/// place. Returns the updated `Scoping` produced by the transformer (the
/// semantic state may change as type-only nodes are removed). Surviving nodes
/// retain their original `Span` values, so positions still refer to the
/// original TypeScript source offsets.
fn strip_typescript_pass<'a>(
    allocator: &'a Allocator,
    filename: &str,
    program: &mut Program<'a>,
    scoping: Scoping,
    decorator_mode: DecoratorMode,
) -> Result<Scoping, InstrumentError> {
    // `JsxOptions::default()` calls `JsxOptions::enable()`, which would
    // rewrite `<div>` to `React.createElement` / `_jsx` on `.tsx` input.
    // Strip-pass only removes type syntax; JSX must round-trip unchanged
    // so codegen can emit it verbatim. Pin the JSX pass off explicitly.
    // `typescript` is also listed explicitly so a future change to
    // `TransformOptions::default()` cannot silently alter the strip pass.
    //
    // `decorator` defaults to `legacy: false, emit_decorator_metadata: false`,
    // which makes the decorator pass a no-op (syntax flows through verbatim).
    // Callers can opt into legacy lowering and metadata emission via
    // `InstrumentOptions::decorator_mode`.
    let options = TransformOptions {
        typescript: TypeScriptOptions::default(),
        jsx: JsxOptions::disable(),
        decorator: DecoratorOptions {
            legacy: decorator_mode.legacy(),
            emit_decorator_metadata: decorator_mode.emit_metadata(),
        },
        ..TransformOptions::default()
    };
    let transformer = Transformer::new(allocator, Path::new(filename), &options);
    let ret = transformer.build_with_scoping(scoping, program);
    if !ret.errors.is_empty() {
        return Err(InstrumentError::TransformError(
            ret.errors.iter().map(|e| format!("{e}")).collect::<Vec<_>>(),
        ));
    }
    Ok(ret.scoping)
}

fn parse_program<'a>(
    allocator: &'a Allocator,
    source: &'a str,
    filename: &str,
) -> Result<ParserReturn<'a>, InstrumentError> {
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let parsed = Parser::new(allocator, source, source_type).parse();
    if parsed.errors.is_empty() {
        Ok(parsed)
    } else {
        Err(InstrumentError::ParseError(
            parsed.errors.iter().map(|e| format!("{e}")).collect::<Vec<_>>().join("; "),
        ))
    }
}

fn empty_coverage_result(
    filename: &str,
    source: &str,
    unhandled_pragmas: Vec<UnhandledPragma>,
) -> InstrumentResult {
    let coverage_map = build_file_coverage(CoverageMaps {
        path: filename.to_string(),
        statement_locs: Vec::new(),
        fn_entries: Vec::new(),
        branch_entries: Vec::new(),
        logical_branch_ids: Vec::new(),
    });
    let coverage_map_json = serialize_coverage_map(&coverage_map);
    InstrumentResult {
        code: source.to_string(),
        coverage_map,
        coverage_map_json,
        source_map: None,
        unhandled_pragmas,
    }
}

fn build_coverage_map(
    filename: &str,
    transform: CoverageTransform<'_, '_>,
    input_source_map: Option<&str>,
) -> FileCoverage {
    let mut coverage_map = build_file_coverage(CoverageMaps {
        path: filename.to_string(),
        statement_locs: transform.statement_map,
        fn_entries: transform.fn_map,
        branch_entries: transform.branch_map,
        logical_branch_ids: transform.logical_branch_ids,
    });
    if let Some(input_sm) = input_source_map {
        coverage_map.input_source_map = serde_json::from_str(input_sm).ok();
    }
    coverage_map
}

fn eager_remapper(
    options: &InstrumentOptions,
) -> Option<oxc_coverage_source_maps::PositionRemapper> {
    if !options.compose_input_source_map {
        return None;
    }

    options
        .input_source_map
        .as_deref()
        .and_then(oxc_coverage_source_maps::PositionRemapper::from_json)
}

fn finalize_coverage_map(
    filename: &str,
    transform: CoverageTransform<'_, '_>,
    options: &InstrumentOptions,
) -> FileCoverage {
    let mut coverage_map =
        build_coverage_map(filename, transform, options.input_source_map.as_deref());
    if options.function_identity_overlay {
        coverage_map.x_fallow_function_map =
            Some(build_function_identity_map(&coverage_map.path, &coverage_map.fn_map));
    }

    // Eager composition (issue #100): when requested, fold the embedded
    // `inputSourceMap` into the coverage map now, before the map is serialized
    // into the preamble's `coverageData` literal. The runtime `__coverage__`
    // then ships original-source positions/path and `remap_coverage` on the
    // result is a no-op. Run AFTER the function-identity overlay attaches so
    // the overlay's ids stay derived from the pre-remap positions, keeping the
    // eager path bit-for-bit equal to instrument-then-remap (the remap pipeline
    // intentionally does not rewrite the overlay). When the input map is
    // unusable, the remap returns `None` and we leave the embedded map in place
    // so the lazy remap path remains available.
    //
    // Drop-at-the-AST-level (issue #106): the transform above was given the same
    // `inputSourceMap`-backed position-remap predicate, so unmappable statement /
    // function / branch points were never instrumented (no map entry, no
    // counter). The instrumented code and coverage data are therefore derived
    // from the same decision and consistent by construction.
    if options.compose_input_source_map
        && options.input_source_map.is_some()
        && let Some(composed) = oxc_coverage_source_maps::remap_coverage(&coverage_map)
    {
        return composed;
    }

    coverage_map
}

/// Output of [`collect_for_v8_to_istanbul`]: the Istanbul `FileCoverage`
/// (statement / function / branch maps) plus a side-table of body byte spans
/// keyed by surviving branch id. Both are needed by `v8_to_istanbul` to
/// resolve V8 hit counts; the body byte spans solve the if-arm 0 case where
/// the istanbul-reported location (the whole `IfStatement`) does not match
/// any V8 block range.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal type intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) struct V8CollectResult {
    pub(crate) coverage_map: FileCoverage,
    /// `arm_body_byte_spans["<branch_id>"][<arm_idx>]` is the `(start, end)`
    /// byte range of the arm body when known, or `(0, 0)` when the body span
    /// is synthetic / unknown (e.g. a synthesized else-arm).
    pub(crate) arm_body_byte_spans: BTreeMap<String, Vec<(u32, u32)>>,
}

/// Parse, scan pragmas, traverse the AST, and build the `FileCoverage` map
/// without performing codegen, preamble emission, or coverage-map JSON
/// serialization. This is the visit-only path `v8_to_istanbul` uses; the
/// codegen + preamble + hash work that `instrument()` performs is dead work
/// when the caller only needs the location maps to intersect against V8
/// byte ranges.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal function intentionally; the explicit pub(crate) documents that this is not part of the public API even though the parent module is already private"
)]
pub(crate) fn collect_for_v8_to_istanbul(
    source: &str,
    filename: &str,
) -> Result<V8CollectResult, InstrumentError> {
    let allocator = Allocator::default();
    let mut parsed = parse_program(&allocator, source, filename)?;

    let (pragmas, _unhandled_pragmas) = PragmaMap::from_program(&parsed.program, source);
    if pragmas.ignore_file {
        return Ok(empty_v8_collect_result(filename));
    }

    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();
    let cov_fn_name = generate_cov_fn_name(filename);

    let mut transform = CoverageTransform::new(TransformInit {
        allocator: &allocator,
        source,
        cov_fn_name: &cov_fn_name,
        report_logic: false,
        // V8-collect builds the location maps that V8 byte ranges intersect
        // against, so optional-chain branches stay tracked (the default); this
        // path emits no runtime helper, only the maps.
        track_optional_chain: true,
        ignore_class_methods: Vec::new(),
        // V8-collect never composes an input source map; gate is a no-op.
        eager_remapper: None,
    });
    let state = CoverageState { pragmas };
    let _scoping = traverse_mut(&mut transform, &allocator, &mut parsed.program, scoping, state);

    // Build the body-byte-span side-table BEFORE moving `branch_map` into
    // `from_maps`. The id assignment in `from_maps` filters out branches with
    // empty `locations` and preserves the original sequential id via
    // `enumerate` (filter runs AFTER), so the keys here use the same
    // pre-filter index and only retain surviving entries.
    let arm_body_byte_spans = collect_arm_body_byte_spans(&transform);

    let coverage_map = build_coverage_map(filename, transform, None);
    Ok(V8CollectResult { coverage_map, arm_body_byte_spans })
}

fn empty_v8_collect_result(filename: &str) -> V8CollectResult {
    let coverage_map = build_file_coverage(CoverageMaps {
        path: filename.to_string(),
        statement_locs: Vec::new(),
        fn_entries: Vec::new(),
        branch_entries: Vec::new(),
        logical_branch_ids: Vec::new(),
    });
    V8CollectResult { coverage_map, arm_body_byte_spans: BTreeMap::new() }
}

fn collect_arm_body_byte_spans(
    transform: &CoverageTransform<'_, '_>,
) -> BTreeMap<String, Vec<(u32, u32)>> {
    let mut spans: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    for (idx, body_spans) in transform.branch_arm_body_byte_spans.iter().enumerate() {
        let surviving =
            transform.branch_map.get(idx).is_some_and(|entry| !entry.locations.is_empty());
        if surviving {
            spans.insert(idx.to_string(), body_spans.clone());
        }
    }
    spans
}

struct EmitInputs<'a, 'arena> {
    program: &'a Program<'arena>,
    scoping: Scoping,
    source: &'a str,
    filename: &'a str,
    preamble: &'a str,
    options: &'a InstrumentOptions,
}

fn emit_code(inputs: EmitInputs<'_, '_>) -> (String, Option<String>) {
    let EmitInputs { program, scoping, source, filename, preamble, options } = inputs;
    let codegen_options = CodegenOptions {
        source_map_path: if options.source_map { Some(PathBuf::from(filename)) } else { None },
        ..CodegenOptions::default()
    };
    let codegen_ret = Codegen::new()
        .with_options(codegen_options)
        .with_source_text(source)
        .with_scoping(Some(scoping))
        .build(program);
    let code = format!("{preamble}{}", codegen_ret.code);
    (code, codegen_ret.map.map(|sm| sm.to_json_string()))
}

/// Offset the codegen source map by the preamble line count and, if an input
/// source map was provided, compose the result with it so the final map chains
/// all the way back to the original source (e.g., TypeScript).
///
/// Composition is delegated to `srcmap-remapping`, which mirrors the semantics
/// of `@ampproject/remapping` (the prior art `istanbul-lib-source-maps` and
/// most JS bundlers also follow). Line offsetting uses `srcmap-remapping`'s
/// `ConcatBuilder`.
fn finalize_source_map(
    output_json: &str,
    preamble: &str,
    input_source_map: Option<&str>,
) -> String {
    let preamble_lines =
        u32::try_from(preamble.chars().filter(|&c| c == '\n').count()).unwrap_or(u32::MAX);

    // Bridge the codegen source map to srcmap_sourcemap via JSON. Both crates
    // emit the standard source map v3 format, so the round-trip is lossless.
    // Bail out to the raw serialization if the parse ever fails.
    let Ok(output_sm) = srcmap_sourcemap::SourceMap::from_json(output_json) else {
        return output_json.to_string();
    };

    let offset_sm = if preamble_lines > 0 {
        let mut builder = srcmap_remapping::ConcatBuilder::new(None);
        builder.add_map(&output_sm, preamble_lines);
        builder.build()
    } else {
        output_sm
    };

    if let Some(input_sm_json) = input_source_map
        && let Ok(input_sm) = srcmap_sourcemap::SourceMap::from_json(input_sm_json)
    {
        // The output map has exactly one source (the instrumented file).
        // Return the input map for any source name; remap drops sources it
        // can't load. Clone per call since `remap` may invoke the loader
        // more than once per unique source name.
        let composed = srcmap_remapping::remap(&offset_sm, |_name: &str| Some(input_sm.clone()));
        return composed.to_json();
    }

    offset_sm.to_json()
}

/// Error type for instrumentation failures.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InstrumentError {
    /// The source could not be parsed.
    ParseError(String),
    /// The coverage variable name is not a valid JavaScript identifier.
    InvalidCoverageVariable(String),
    /// Coverage data serialization failed. Reserved for future use: the current
    /// `FileCoverage` shape only contains types whose serde implementations are
    /// infallible, so `instrument()` does not currently construct this variant.
    SerializationError(String),
    /// The TypeScript strip pass produced diagnostics. Only emitted when
    /// `InstrumentOptions::strip_typescript` is enabled. The vector
    /// contains one entry per transformer diagnostic so callers can
    /// surface them individually instead of string-scraping a joined
    /// message.
    TransformError(Vec<String>),
}

impl std::fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
            Self::TransformError(msgs) => write!(f, "transform error: {}", msgs.join("; ")),
            Self::InvalidCoverageVariable(name) => {
                write!(
                    f,
                    "invalid coverage variable: {name:?} is not a valid JavaScript identifier"
                )
            }
        }
    }
}

impl std::error::Error for InstrumentError {}
