//! Top-level instrumentation API.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_coverage_types::{FileCoverage, UnhandledPragma};
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_transformer::{
    DecoratorOptions, JsxOptions, TransformOptions, Transformer, TypeScriptOptions,
};
use oxc_traverse::traverse_mut;

use crate::coverage_builder::{CoverageMaps, build_file_coverage, build_function_identity_map};
use crate::pragma::PragmaMap;
use crate::transform::{
    CoverageState, CoverageTransform, PreambleInputs, TransformInit, djb31_hex,
    generate_cov_fn_name, generate_preamble_source,
};

/// Options for the `instrument` function.
#[derive(Debug, Clone)]
pub struct InstrumentOptions {
    /// Name of the global coverage variable (default: `"__coverage__"`).
    pub coverage_variable: String,
    /// Whether to generate a source map for the instrumented output.
    pub source_map: bool,
    /// Input source map JSON string from a prior transformation (e.g., TypeScript -> JS).
    /// When provided, this is stored on the `FileCoverage` as `inputSourceMap` so
    /// downstream tools (nyc, istanbul-reports) can chain back to the original source.
    pub input_source_map: Option<String>,
    /// When true AND [`InstrumentOptions::input_source_map`] is set, compose the
    /// input source map into the coverage map during instrumentation instead of
    /// embedding it for downstream composition. Has no effect when
    /// `input_source_map` is `None`.
    ///
    /// See [Composing an input source map](crate#composing-an-input-source-map)
    /// for what this changes about the emitted map and counters.
    ///
    /// Defaults to `false`.
    pub compose_input_source_map: bool,
    /// When true, adds truthy-value tracking (`bT`) for logical expression operands.
    /// This enables nyc-style logic coverage that tracks not just which branch was
    /// taken, but whether each operand evaluated to a truthy value.
    pub report_logic: bool,
    /// When true (the default), receiver-safe optional-chaining (`?.`) links are
    /// tracked as `optional-chain` branches: their operands are wrapped in a
    /// `cov_fn_oc` helper that records whether the value was nullish.
    /// Receiver-bound optional calls such as `object.method?.()` stay native so
    /// instrumentation cannot detach the method from its `this` value.
    ///
    /// Set to false to leave optional chains native: no `cov_fn_oc` helper is
    /// emitted and no `optional-chain` branch is registered. This matches
    /// `istanbul-lib-instrument` byte-for-byte on `?.` and avoids the
    /// per-operand helper-call overhead in optional-chain-dense hot paths.
    /// Statement, function, and other branch coverage are unaffected.
    ///
    /// Defaults to `true`.
    pub track_optional_chain: bool,
    /// Class method names to exclude from coverage instrumentation.
    /// Matches Istanbul's `ignoreClassMethods` behavior for class methods and
    /// named function expressions with a matching id.
    pub ignore_class_methods: Vec<String>,
    /// When true, run `oxc_transformer`'s TypeScript-strip pass on the parsed
    /// AST before coverage instrumentation. Set this when passing raw
    /// TypeScript source that has not been pre-transformed by Babel / tsc /
    /// esbuild.
    ///
    /// See [TypeScript and decorators](crate#typescript-and-decorators) for the
    /// positions the stripped output reports and what happens to raw TypeScript
    /// when this is left off.
    ///
    /// Defaults to `false`, for callers that supply JavaScript already
    /// transformed by Babel / tsc / esbuild.
    pub strip_typescript: bool,
    /// How decorator syntax is handled by the strip pass. See
    /// [`DecoratorMode`] for the variants and their semantics. Has no effect
    /// unless `strip_typescript` is also true.
    ///
    /// Defaults to [`DecoratorMode::PassThrough`]: decorator syntax flows
    /// through verbatim and a downstream tool is responsible for lowering it.
    pub decorator_mode: DecoratorMode,
    /// Whether the source is compiled under [`strictNullChecks`]. Only
    /// consulted when `decorator_mode` is
    /// [`DecoratorMode::ExperimentalWithMetadata`], where it decides how a
    /// nullable union is written into the emitted `design:type` metadata.
    ///
    /// See [TypeScript and decorators](crate#typescript-and-decorators) for the
    /// consumers that read that metadata and why a mismatch is silent.
    ///
    /// Defaults to `true`, matching both the oxc default and `tsc` under
    /// `strict`.
    ///
    /// [`strictNullChecks`]: https://www.typescriptlang.org/tsconfig#strictNullChecks
    pub strict_null_checks: bool,
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
    /// Defaults to `false`, which keeps the serialized map to the fields
    /// Istanbul consumers expect.
    pub function_identity_overlay: bool,
    /// When true, give a name to a function/arrow expression that is a direct
    /// argument of a call or `new` expression and has no other inferable name,
    /// derived from the callee: `arr.map(x => x)` gives `"map"`.
    ///
    /// See [Naming callback arguments](crate#naming-callback-arguments) for how
    /// this interacts with binding-inferred names and why it is opt-in.
    ///
    /// Defaults to `false`.
    pub name_callback_arguments: bool,
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
    /// Decorator syntax flows through verbatim. A downstream tool (Babel,
    /// tsc, esbuild) is expected to lower it.
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
            strict_null_checks: true,
            function_identity_overlay: false,
            name_callback_arguments: false,
        }
    }
}

/// Consuming builder over [`InstrumentOptions::default`], one method per field.
///
/// Struct-literal construction stays available, but it breaks whenever a new
/// option is added; chaining off `default()` does not.
///
/// ```
/// use oxc_coverage_instrument::InstrumentOptions;
///
/// let options = InstrumentOptions::default()
///     .with_coverage_variable("__my_coverage__".to_string())
///     .with_report_logic(true);
///
/// assert_eq!(options.coverage_variable, "__my_coverage__");
/// assert!(options.report_logic);
/// ```
impl InstrumentOptions {
    /// Set [`InstrumentOptions::coverage_variable`].
    #[must_use]
    pub fn with_coverage_variable(mut self, name: String) -> Self {
        self.coverage_variable = name;
        self
    }

    /// Set [`InstrumentOptions::source_map`].
    #[must_use]
    pub fn with_source_map(mut self, source_map: bool) -> Self {
        self.source_map = source_map;
        self
    }

    /// Set [`InstrumentOptions::input_source_map`].
    #[must_use]
    pub fn with_input_source_map(mut self, input_source_map: Option<String>) -> Self {
        self.input_source_map = input_source_map;
        self
    }

    /// Set [`InstrumentOptions::compose_input_source_map`].
    #[must_use]
    pub fn with_compose_input_source_map(mut self, compose: bool) -> Self {
        self.compose_input_source_map = compose;
        self
    }

    /// Set [`InstrumentOptions::report_logic`].
    #[must_use]
    pub fn with_report_logic(mut self, report_logic: bool) -> Self {
        self.report_logic = report_logic;
        self
    }

    /// Set [`InstrumentOptions::track_optional_chain`].
    #[must_use]
    pub fn with_track_optional_chain(mut self, track: bool) -> Self {
        self.track_optional_chain = track;
        self
    }

    /// Set [`InstrumentOptions::ignore_class_methods`].
    #[must_use]
    pub fn with_ignore_class_methods(mut self, names: Vec<String>) -> Self {
        self.ignore_class_methods = names;
        self
    }

    /// Set [`InstrumentOptions::strip_typescript`].
    #[must_use]
    pub fn with_strip_typescript(mut self, strip: bool) -> Self {
        self.strip_typescript = strip;
        self
    }

    /// Set [`InstrumentOptions::decorator_mode`].
    #[must_use]
    pub fn with_decorator_mode(mut self, mode: DecoratorMode) -> Self {
        self.decorator_mode = mode;
        self
    }

    /// Set [`InstrumentOptions::strict_null_checks`].
    #[must_use]
    pub fn with_strict_null_checks(mut self, strict_null_checks: bool) -> Self {
        self.strict_null_checks = strict_null_checks;
        self
    }

    /// Set [`InstrumentOptions::function_identity_overlay`].
    #[must_use]
    pub fn with_function_identity_overlay(mut self, overlay: bool) -> Self {
        self.function_identity_overlay = overlay;
        self
    }

    /// Set [`InstrumentOptions::name_callback_arguments`].
    #[must_use]
    pub fn with_name_callback_arguments(mut self, name_callback_arguments: bool) -> Self {
        self.name_callback_arguments = name_callback_arguments;
        self
    }
}

/// Result of instrumenting a source file.
///
/// Marked `#[non_exhaustive]` so a future output can be added without a
/// breaking change; construct it only through [`instrument`].
#[derive(Debug)]
#[non_exhaustive]
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
fn is_valid_js_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Serialize a `FileCoverage` to JSON.
///
/// `FileCoverage` is composed of `BTreeMap`, `Vec`, `String` and primitive
/// numbers, all with first-party serde implementations that cannot fail at
/// runtime, so the serialization is infallible.
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
/// - [`InstrumentError::InvalidCoverageVariable`] if
///   [`InstrumentOptions::coverage_variable`] is not a valid JavaScript
///   identifier
/// - [`InstrumentError::ParseError`] if `source` cannot be parsed
/// - [`InstrumentError::TransformError`] if
///   [`InstrumentOptions::strip_typescript`] is set and the strip pass reports
///   an error
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
    if pragmas.ignore_file && !options.strip_typescript {
        return Ok(empty_coverage_result(filename, source, unhandled_pragmas));
    }

    let scoping = prepare_scoping(PrepareScopingInput {
        allocator: &allocator,
        filename,
        program: &mut parsed.program,
        options,
    })?;
    if pragmas.ignore_file {
        let (code, _) = emit_code(EmitInputs {
            program: &parsed.program,
            scoping,
            source,
            filename,
            preamble: "",
            options,
        });
        return Ok(empty_coverage_result(filename, &code, unhandled_pragmas));
    }
    let cov_fn_name = generate_cov_fn_name(filename);

    let (transform, scoping) = run_coverage_transform(CoverageTransformRun {
        allocator: &allocator,
        program: &mut parsed.program,
        scoping,
        pragmas,
        source,
        cov_fn_name: &cov_fn_name,
        options,
    });

    let needs_optional_chain_helper = transform.used_optional_chain_helper;
    let coverage_map = finalize_coverage_map(filename, transform, options);

    let (coverage_json, preamble) = build_instrument_preamble(
        &coverage_map,
        options,
        &cov_fn_name,
        needs_optional_chain_helper,
    );

    let (code, raw_source_map) = emit_code(EmitInputs {
        program: &parsed.program,
        scoping,
        source,
        filename,
        preamble: &preamble,
        options,
    });
    let source_map = raw_source_map
        .as_deref()
        .map(|sm| finalize_source_map(sm, &preamble, options.input_source_map.as_deref()));

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

struct PrepareScopingInput<'arena, 'a> {
    allocator: &'arena Allocator,
    filename: &'a str,
    program: &'a mut Program<'arena>,
    options: &'a InstrumentOptions,
}

fn prepare_scoping(input: PrepareScopingInput<'_, '_>) -> Result<Scoping, InstrumentError> {
    let PrepareScopingInput { allocator, filename, program, options } = input;
    // `with_enum_eval` pre-computes TypeScript enum member values into `Scoping`.
    // Since oxc 0.140 the transformer asserts on it when lowering an `enum`, so
    // this is required on the scoping handed to `strip_typescript_pass` below.
    // The V8-collect path builds its own scoping for the traverse pass only, and
    // never reaches the transformer, so it does not pay for enum evaluation.
    let scoping =
        SemanticBuilder::new().with_enum_eval(true).build(program).semantic.into_scoping();
    if !options.strip_typescript {
        return Ok(scoping);
    }
    strip_typescript_pass(StripTypescriptInput {
        allocator,
        filename,
        program,
        scoping,
        decorator_mode: options.decorator_mode,
        strict_null_checks: options.strict_null_checks,
    })
}

struct CoverageTransformRun<'src, 'arena, 'a> {
    allocator: &'arena Allocator,
    program: &'a mut Program<'arena>,
    scoping: Scoping,
    pragmas: PragmaMap,
    source: &'src str,
    cov_fn_name: &'src str,
    options: &'a InstrumentOptions,
}

fn run_coverage_transform<'src, 'arena>(
    input: CoverageTransformRun<'src, 'arena, '_>,
) -> (CoverageTransform<'src, 'arena>, Scoping) {
    let CoverageTransformRun { allocator, program, scoping, pragmas, source, cov_fn_name, options } =
        input;
    let mut transform = CoverageTransform::new(TransformInit {
        allocator,
        source,
        cov_fn_name,
        report_logic: options.report_logic,
        track_optional_chain: options.track_optional_chain,
        ignore_class_methods: options.ignore_class_methods.clone(),
        name_callback_arguments: options.name_callback_arguments,
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
    // Whether the transform wrapped any optional-chain link, so the preamble
    // declares the `cov_fn_oc` helper.
    needs_optional_chain_helper: bool,
) -> (String, String) {
    // Serialize once and reuse it for both the hash guard and the coverageData
    // literal.
    let coverage_json = serialize_coverage_map(coverage_map);
    let coverage_hash = djb31_hex(&coverage_json);
    let preamble = generate_preamble_source(&PreambleInputs {
        coverage: coverage_map,
        coverage_json: &coverage_json,
        coverage_hash: &coverage_hash,
        coverage_var: &options.coverage_variable,
        cov_fn_name,
        report_logic: options.report_logic,
        needs_optional_chain_helper,
    });
    (coverage_json, preamble)
}

struct StripTypescriptInput<'arena, 'a> {
    allocator: &'arena Allocator,
    filename: &'a str,
    program: &'a mut Program<'arena>,
    scoping: Scoping,
    decorator_mode: DecoratorMode,
    strict_null_checks: bool,
}

/// Run `oxc_transformer`'s TypeScript-strip pass on the parsed program in
/// place. Returns the updated `Scoping` produced by the transformer (the
/// semantic state changes as type-only nodes are removed). Surviving nodes
/// retain their original `Span` values, so positions still refer to the
/// original TypeScript source offsets.
///
/// # Errors
///
/// Returns [`InstrumentError::TransformError`] if the transformer reports a
/// diagnostic at error severity.
fn strip_typescript_pass(input: StripTypescriptInput<'_, '_>) -> Result<Scoping, InstrumentError> {
    let StripTypescriptInput {
        allocator,
        filename,
        program,
        scoping,
        decorator_mode,
        strict_null_checks,
    } = input;
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
    // `InstrumentOptions::decorator_mode`, and control how nullable unions are
    // written into that metadata via `InstrumentOptions::strict_null_checks`.
    let options = TransformOptions {
        typescript: TypeScriptOptions::default(),
        jsx: JsxOptions::disable(),
        decorator: DecoratorOptions {
            legacy: decorator_mode.legacy(),
            emit_decorator_metadata: decorator_mode.emit_metadata(),
            strict_null_checks,
        },
        ..TransformOptions::default()
    };
    let transformer = Transformer::new(allocator, Path::new(filename), &options);
    let ret = transformer.build_with_scoping(scoping, program);
    // `diagnostics` carries warnings as well as errors, so gate on error
    // severity: a warning-only transform is not a failure.
    if ret.diagnostics.has_errors() {
        return Err(InstrumentError::TransformError(
            ret.diagnostics.errors().map(|e| format!("{e}")).collect::<Vec<_>>(),
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
    // As in `strip_typescript_pass`: gate on error severity, not on any diagnostic.
    if parsed.diagnostics.has_errors() {
        Err(InstrumentError::ParseError(
            parsed.diagnostics.errors().map(|e| format!("{e}")).collect::<Vec<_>>().join("; "),
        ))
    } else {
        Ok(parsed)
    }
}

/// A `FileCoverage` with every map empty, for a source suppressed by
/// `/* istanbul ignore file */`.
fn empty_coverage(filename: &str) -> FileCoverage {
    build_file_coverage(CoverageMaps {
        path: filename.to_string(),
        statement_locs: Vec::new(),
        fn_entries: Vec::new(),
        branch_entries: Vec::new(),
        logical_branch_ids: Vec::new(),
    })
}

fn empty_coverage_result(
    filename: &str,
    source: &str,
    unhandled_pragmas: Vec<UnhandledPragma>,
) -> InstrumentResult {
    let coverage_map = empty_coverage(filename);
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
    let overlay_conflict = transform.eager_function_overlay_conflict;
    let mut coverage_map =
        build_coverage_map(filename, transform, options.input_source_map.as_deref());
    if options.function_identity_overlay {
        // Dropped before composition, so the composed result is overlay-free
        // the way the deferred `merge_functions` conflict rule leaves it.
        coverage_map.x_fallow_function_map = (!overlay_conflict)
            .then(|| build_function_identity_map(&coverage_map.path, &coverage_map.fn_map));
    }

    // Fold the embedded `inputSourceMap` in before the map is serialized into
    // the preamble's `coverageData` literal, so the runtime `__coverage__`
    // ships original-source positions and a later `remap_coverage` is a no-op.
    // Ordered after the function-identity overlay so the overlay's ids stay
    // derived from pre-remap positions, which is what instrument-then-remap
    // produces (the remap pipeline does not rewrite the overlay). An unusable
    // input map yields `None` here, leaving the embedded map in place for the
    // lazy remap path.
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
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) struct V8CollectResult {
    /// Location maps with all hit counts still zero.
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
/// UTF-16 ranges.
///
/// # Errors
///
/// Returns [`InstrumentError::ParseError`] if `source` cannot be parsed.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
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
        // V8-collect builds the location maps that V8 UTF-16 ranges intersect
        // against, so optional-chain branches stay tracked (the default); this
        // path emits no runtime helper, only the maps.
        track_optional_chain: true,
        ignore_class_methods: Vec::new(),
        // V8-collect builds only the position maps that V8 UTF-16 ranges
        // intersect against; the fnMap names never reach a consumer here, so
        // it stays Istanbul-exact (no callback naming) with no options to wire.
        name_callback_arguments: false,
        // V8-collect never composes an input source map; gate is a no-op.
        eager_remapper: None,
    });
    let state = CoverageState { pragmas };
    let _scoping = traverse_mut(&mut transform, &allocator, &mut parsed.program, scoping, state);

    // Built before `branch_map` moves into `build_file_coverage`, which drops
    // branches with no locations but assigns ids from the pre-filter
    // `enumerate` index. The keys here use that same index, so they line up
    // with the surviving entries.
    let arm_body_byte_spans = collect_arm_body_byte_spans(&transform);

    let coverage_map = build_coverage_map(filename, transform, None);
    Ok(V8CollectResult { coverage_map, arm_body_byte_spans })
}

fn empty_v8_collect_result(filename: &str) -> V8CollectResult {
    V8CollectResult { coverage_map: empty_coverage(filename), arm_body_byte_spans: BTreeMap::new() }
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
        // The output map has exactly one source, the instrumented file, so the
        // input map is returned for any source name; `remap` drops sources it
        // cannot load. Cloned per call because `remap` may invoke the loader
        // more than once for a single source name.
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
