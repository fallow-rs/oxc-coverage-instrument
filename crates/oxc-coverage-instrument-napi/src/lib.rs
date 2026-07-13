//! Node.js bindings for oxc-coverage-instrument.
//!
//! Exposes the `instrument` function to JavaScript via napi-rs.

// napi-derive generates code that triggers needless_pass_by_value
#![expect(clippy::needless_pass_by_value, reason = "napi function signatures require owned types")]
// napi-derive lowers TS `Record<string, string>` to `HashMap<String, String>` with the default
// hasher; we can't add a generic hasher parameter because napi-derive does not propagate
// generics into the JS-facing function signature.
#![expect(
    clippy::implicit_hasher,
    reason = "napi-derive cannot lower a generic BuildHasher parameter into the napi function signature"
)]

use std::collections::HashMap;

use napi_derive::napi;
use oxc_coverage_instrument::{DecoratorMode, RemapOptions as CoreRemapOptions};

/// Build a friendlier `napi::Error` for the case where the caller passed a
/// single `FileCoverage` JSON to a function that expects an Istanbul-shape
/// `CoverageMap` (`{[path]: FileCoverage}`). The raw serde_json error in
/// this case is "invalid type: string \"...\", expected struct FileCoverage"
/// which is technically correct but unhelpful: the user reads it as "the
/// FileCoverage shape is wrong" when the actual problem is "you handed me
/// the wrong outer container". Caught during the v0.7.2 smoke test.
fn invalid_coverage_json_error<E: std::fmt::Display>(coverage_json: &str, err: E) -> napi::Error {
    let hint = if looks_like_single_file_coverage(coverage_json) {
        " (hint: input parses as a single FileCoverage; remapCoverageMap \
         expects an Istanbul CoverageMap shape `{[path]: FileCoverage}`. \
         Wrap with `{ [fc.path]: fc }` before calling.)"
    } else {
        ""
    };
    napi::Error::new(napi::Status::InvalidArg, format!("invalid coverage JSON: {err}{hint}"))
}

/// Cheap, platform-agnostic "this JSON looks like a single `FileCoverage`,
/// not a `CoverageMap`" heuristic for [`invalid_coverage_json_error`]'s hint.
///
/// The original implementation (PR #67, v0.7.2) was
/// `serde_json::from_str::<FileCoverage>(coverage_json).is_ok()`, but that
/// returned `Err` under the `wasm32-wasi` napi binding even when the same
/// JSON round-tripped through `FileCoverage` cleanly on the native binding.
/// The hint silently disappeared on wasi and the regression test failed in
/// CI without the underlying reproducer being easy to obtain locally.
///
/// Switching to a shape check on the parsed `serde_json::Value` (top-level
/// object with a string `path` and the three canonical Istanbul map keys)
/// removes the dependency on full `FileCoverage` deserialization, which is
/// where the platform divergence lives. The false-positive rate is
/// effectively zero for realistic CoverageMap inputs because every
/// CoverageMap key is a file-path string and the outer container's children
/// (per-file `FileCoverage` objects) only get reached after the outer parse
/// has already succeeded.
fn looks_like_single_file_coverage(coverage_json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(coverage_json) else {
        return false;
    };
    let Some(obj) = value.as_object() else {
        return false;
    };
    obj.get("path").is_some_and(serde_json::Value::is_string)
        && obj.contains_key("statementMap")
        && obj.contains_key("fnMap")
        && obj.contains_key("branchMap")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        InstrumentOptions, RemapOptions, core_remap_options_from, decorator_mode_from_flags,
        invalid_coverage_json_error, looks_like_single_file_coverage, native_instrument_options,
        remap_coverage_map, remap_coverage_map_with_loader,
    };
    use oxc_coverage_instrument::DecoratorMode;

    const SINGLE_FC: &str =
        r#"{"path":"app.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}"#;
    const COVERAGE_MAP: &str = r#"{"app.js":{"path":"app.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;

    fn empty_options() -> InstrumentOptions {
        InstrumentOptions {
            coverage_variable: None,
            source_map: None,
            input_source_map: None,
            compose_input_source_map: None,
            report_logic: None,
            track_optional_chain_branches: None,
            ignore_class_methods: None,
            strip_typescript: None,
            experimental_decorators: None,
            emit_decorator_metadata: None,
            function_identity_overlay: None,
            name_callback_arguments: None,
        }
    }

    fn one_line_coverage_map_json() -> String {
        r#"{
            "intermediate.js": {
                "path": "intermediate.js",
                "statementMap": {
                    "0": {
                        "start": {"line": 1, "column": 0},
                        "end": {"line": 1, "column": 1}
                    }
                },
                "fnMap": {},
                "branchMap": {},
                "s": {"0": 1},
                "f": {},
                "b": {}
            }
        }"#
        .to_string()
    }

    fn one_line_source_map_json() -> String {
        r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["x"],"mappings":"AAAA","names":[]}"#
            .to_string()
    }

    #[test]
    fn detects_single_file_coverage() {
        assert!(looks_like_single_file_coverage(SINGLE_FC));
    }

    #[test]
    fn rejects_coverage_map_outer_container() {
        // CoverageMap's keys are path strings; the outer container's value
        // for `"app.js"` is a `FileCoverage`, not a string, so the `path`
        // string check fails on the outer object.
        assert!(!looks_like_single_file_coverage(COVERAGE_MAP));
    }

    #[test]
    fn rejects_non_object_root() {
        assert!(!looks_like_single_file_coverage("[]"));
        assert!(!looks_like_single_file_coverage("\"hello\""));
        assert!(!looks_like_single_file_coverage("42"));
        assert!(!looks_like_single_file_coverage("null"));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(!looks_like_single_file_coverage("{ not json"));
        assert!(!looks_like_single_file_coverage(""));
    }

    #[test]
    fn rejects_object_missing_required_keys() {
        // Has path but no Istanbul map keys: ambiguous, treat as not-FileCoverage.
        assert!(!looks_like_single_file_coverage(r#"{"path":"app.js"}"#));
        // Has statementMap but no path: not a FileCoverage.
        assert!(!looks_like_single_file_coverage(
            r#"{"statementMap":{},"fnMap":{},"branchMap":{}}"#
        ));
        // Path is not a string.
        assert!(!looks_like_single_file_coverage(
            r#"{"path":42,"statementMap":{},"fnMap":{},"branchMap":{}}"#,
        ));
    }

    #[test]
    fn invalid_coverage_json_error_adds_single_file_hint() {
        let err = invalid_coverage_json_error(SINGLE_FC, "outer container mismatch");
        let message = err.to_string();
        assert!(message.contains("invalid coverage JSON: outer container mismatch"));
        assert!(message.contains("input parses as a single FileCoverage"));
        assert!(message.contains("expects an Istanbul CoverageMap shape"));
    }

    #[test]
    fn invalid_coverage_json_error_omits_hint_for_coverage_map_shape() {
        let err = invalid_coverage_json_error(COVERAGE_MAP, "bad inner entry");
        let message = err.to_string();
        assert!(message.contains("invalid coverage JSON: bad inner entry"));
        assert!(!message.contains("input parses as a single FileCoverage"));
    }

    #[test]
    fn remap_coverage_map_reports_single_file_hint() {
        let err = remap_coverage_map(SINGLE_FC.to_string(), None)
            .expect_err("single FileCoverage is not a CoverageMap");
        let message = err.to_string();
        assert!(message.contains("input parses as a single FileCoverage"));
        assert!(message.contains("Wrap with `{ [fc.path]: fc }` before calling"));
    }

    #[test]
    fn remap_coverage_map_with_loader_uses_json_string_maps() {
        let mut source_maps = HashMap::new();
        source_maps.insert("intermediate.js".to_string(), one_line_source_map_json());

        let remapped = remap_coverage_map_with_loader(
            one_line_coverage_map_json(),
            source_maps,
            Some(RemapOptions { drop_unmapped: Some(false) }),
        )
        .expect("loader-backed remap succeeds");
        let value: serde_json::Value =
            serde_json::from_str(&remapped).expect("remapped coverage is valid JSON");

        assert!(value.get("src/app.ts").is_some(), "entry is rekeyed by source path");
        assert!(
            value["src/app.ts"].get("inputSourceMap").is_none(),
            "consumed source map is not serialized back",
        );
    }

    #[test]
    fn decorator_flags_map_to_core_modes() {
        assert_eq!(
            decorator_mode_from_flags(false, false).expect("pass-through mode"),
            DecoratorMode::PassThrough,
        );
        assert_eq!(
            decorator_mode_from_flags(true, false).expect("experimental mode"),
            DecoratorMode::Experimental,
        );
        assert_eq!(
            decorator_mode_from_flags(true, true).expect("metadata mode"),
            DecoratorMode::ExperimentalWithMetadata,
        );
    }

    #[test]
    fn decorator_metadata_requires_experimental_decorators() {
        let err = decorator_mode_from_flags(false, true).expect_err("metadata alone is invalid");
        assert_eq!(
            err.to_string(),
            "InvalidArg, emitDecoratorMetadata: true requires experimentalDecorators: true",
        );
    }

    #[test]
    fn remap_options_default_to_keep_unmapped() {
        assert!(!core_remap_options_from(None).drop_unmapped);
        assert!(!core_remap_options_from(Some(RemapOptions { drop_unmapped: None })).drop_unmapped);
        assert!(
            !core_remap_options_from(Some(RemapOptions { drop_unmapped: Some(false) }))
                .drop_unmapped
        );
        assert!(
            core_remap_options_from(Some(RemapOptions { drop_unmapped: Some(true) })).drop_unmapped
        );
    }

    #[test]
    fn native_options_fill_core_defaults() {
        let opts = native_instrument_options(None).expect("default options");
        assert_eq!(opts.coverage_variable, "__coverage__");
        assert!(!opts.source_map);
        assert!(!opts.compose_input_source_map);
        assert!(!opts.report_logic);
        assert!(opts.track_optional_chain);
        assert!(opts.ignore_class_methods.is_empty());
        assert!(!opts.strip_typescript);
        assert_eq!(opts.decorator_mode, DecoratorMode::PassThrough);
        assert!(!opts.function_identity_overlay);
        assert!(!opts.name_callback_arguments);
    }

    #[test]
    fn native_options_map_js_flags_to_core_options() {
        let mut input = empty_options();
        input.coverage_variable = Some("__cov".to_string());
        input.source_map = Some(true);
        input.input_source_map = Some(r#"{"version":3}"#.to_string());
        input.compose_input_source_map = Some(true);
        input.report_logic = Some(true);
        input.track_optional_chain_branches = Some(false);
        input.ignore_class_methods = Some(vec!["render".to_string(), "toJSON".to_string()]);
        input.strip_typescript = Some(true);
        input.experimental_decorators = Some(true);
        input.emit_decorator_metadata = Some(true);
        input.function_identity_overlay = Some(true);
        input.name_callback_arguments = Some(true);

        let opts = native_instrument_options(Some(input)).expect("mapped options");
        assert_eq!(opts.coverage_variable, "__cov");
        assert!(opts.source_map);
        assert_eq!(opts.input_source_map.as_deref(), Some(r#"{"version":3}"#));
        assert!(opts.compose_input_source_map);
        assert!(opts.report_logic);
        assert!(!opts.track_optional_chain);
        assert_eq!(opts.ignore_class_methods, ["render", "toJSON"]);
        assert!(opts.strip_typescript);
        assert_eq!(opts.decorator_mode, DecoratorMode::ExperimentalWithMetadata);
        assert!(opts.function_identity_overlay);
        assert!(opts.name_callback_arguments);
    }

    #[test]
    fn native_options_reject_metadata_without_experimental_decorators() {
        let mut input = empty_options();
        input.emit_decorator_metadata = Some(true);

        let err = native_instrument_options(Some(input)).expect_err("invalid decorator flags");
        assert_eq!(
            err.to_string(),
            "InvalidArg, emitDecoratorMetadata: true requires experimentalDecorators: true",
        );
    }
}

/// Reconstruct the typed [`DecoratorMode`] enum from the two-optional-boolean
/// shape exposed on the napi `InstrumentOptions`. The pair `(experimental =
/// false, metadata = true)` is rejected: the upstream `oxc_transformer`
/// decorator pass is gated on legacy mode, so emitting metadata without also
/// lowering decorators is nonsensical. Returning a JS `Error` here surfaces
/// the misconfiguration loudly instead of silently promoting it on the Rust
/// side.
fn decorator_mode_from_flags(experimental: bool, metadata: bool) -> napi::Result<DecoratorMode> {
    match (experimental, metadata) {
        (false, false) => Ok(DecoratorMode::PassThrough),
        (true, false) => Ok(DecoratorMode::Experimental),
        (true, true) => Ok(DecoratorMode::ExperimentalWithMetadata),
        (false, true) => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "emitDecoratorMetadata: true requires experimentalDecorators: true",
        )),
    }
}

/// Options for the instrument function.
#[napi(object)]
pub struct InstrumentOptions {
    /// Name of the global coverage variable (default: "__coverage__").
    pub coverage_variable: Option<String>,
    /// Whether to generate a source map for the instrumented output.
    pub source_map: Option<bool>,
    /// Input source map JSON string from a prior transformation.
    pub input_source_map: Option<String>,
    /// When true AND `inputSourceMap` is set, compose the input source map into
    /// the coverage map during instrumentation. The returned `coverageMap` (and
    /// the runtime `__coverage__` baked into the instrumented `code`) carries
    /// original-source positions, is keyed by the original source path, and has
    /// no `inputSourceMap` embedded; `remapCoverageMap` on the result is a
    /// no-op. Useful for E2E collectors (Playwright et al.) that dump
    /// `window.__coverage__` directly and want original-source positions without
    /// a downstream remap round-trip.
    ///
    /// In eager mode a coverage point whose positions have no mapping in
    /// `inputSourceMap` is NOT instrumented at all: it gets no `statementMap` /
    /// `fnMap` / `branchMap` entry AND no counter in the emitted `code`, so the
    /// runtime `__coverage__` object and the emitted counters always agree (no
    /// dangling counter against a pruned slot). Composition itself is then a
    /// pure remap of the surviving positions, so it never emits past-EOF entries
    /// (e.g. compiler boilerplate in a Vue SFC chunk with no mapping back to the
    /// `.vue` is simply never instrumented). If `inputSourceMap` is unusable, the
    /// gate is off and the embedded map is retained for the lazy path. Has no
    /// effect when `inputSourceMap` is unset. Default false.
    pub compose_input_source_map: Option<bool>,
    /// When true, adds truthy-value tracking (bT) for logical expression operands.
    pub report_logic: Option<bool>,
    /// Track each optional-chaining (`?.`) link as a branch (default: true).
    /// When false, optional chains are left native: no `_oc` helper is emitted
    /// and no `optional-chain` branches are registered. Matches
    /// `istanbul-lib-instrument` (which does not track `?.` as a branch) and
    /// avoids the per-operand helper-call overhead in optional-chain-dense hot
    /// paths. Statement and other branch coverage are unaffected.
    pub track_optional_chain_branches: Option<bool>,
    /// Class method names to exclude from coverage instrumentation.
    pub ignore_class_methods: Option<Vec<String>>,
    /// When true, run the TypeScript-strip pass before instrumentation.
    /// Set this when passing raw TypeScript source that has not been
    /// pre-transformed by Babel / tsc / esbuild. Defaults to false, which
    /// preserves backward compatibility with existing Vitest / nyc callers
    /// that supply already-transformed JavaScript. If false and you pass
    /// raw TypeScript, the output will contain TypeScript syntax and will
    /// not be executable as JavaScript (no error is returned).
    pub strip_typescript: Option<bool>,
    /// When true, lower TypeScript `experimentalDecorators` syntax
    /// (`@Injectable()` / `@Controller()` style used by NestJS, Angular,
    /// class-validator, TypeORM) into runtime `_decorate(...)`
    /// calls. Mirrors the `experimentalDecorators` flag in `tsconfig.json`.
    ///
    /// The instrumented output references imports from `@oxc-project/runtime` at
    /// runtime; install `@oxc-project/runtime` (or provide an equivalent shim).
    /// See the README for details and troubleshooting.
    ///
    /// Has no effect unless `stripTypescript` is also true. Defaults to false.
    pub experimental_decorators: Option<bool>,
    /// When true, emit TypeScript-style decorator metadata
    /// (`design:type`, `design:paramtypes`, `design:returntype`) alongside
    /// each decorated class / method / property. Required for NestJS
    /// dependency injection, TypeORM column type inference, and
    /// class-validator's metadata-driven validation. Mirrors the
    /// `emitDecoratorMetadata` flag in `tsconfig.json`.
    ///
    /// Requires `experimentalDecorators: true`; passing
    /// `emitDecoratorMetadata: true` with `experimentalDecorators: false`
    /// (or omitted) is invalid and throws a JS `Error` instead of being
    /// silently promoted. The instrumented output requires
    /// `@oxc-project/runtime` at execution; see the README.
    ///
    /// Has no effect unless `stripTypescript` is also true. Defaults to false.
    pub emit_decorator_metadata: Option<bool>,
    /// When true, attach an optional `x_fallow_functionMap` overlay to the
    /// returned coverage map. The overlay carries a stable
    /// `fallow:fn:<hex>` identity per function, keyed by the same ids as
    /// `fnMap`, derived from `(path, name, decl span, loc span)`. Standard
    /// Istanbul consumers ignore the `x_`-prefixed field; downstream code
    /// quality tools (Fallow et al.) use it as a long-lived join key.
    ///
    /// Defaults to false. The default JSON output stays byte-identical to
    /// what Istanbul consumers expect.
    pub function_identity_overlay: Option<bool>,
    /// When true, name an otherwise-anonymous function/arrow that is a direct
    /// argument of a call or `new` expression from its callee: `arr.map(cb)`
    /// -> `"map"`, `el.addEventListener("click", () => {})` ->
    /// `"addEventListener"`, `new Promise((res) => {})` -> `"Promise"`.
    /// `istanbul-lib-instrument` leaves these `(anonymous_N)`, so this is an
    /// opt-in enhancement. Names inferred from a binding (variable declarator,
    /// property key, assignment target, default value) still win; this only
    /// replaces the `(anonymous_N)` fallback with a callee-derived name that is
    /// also stable across rebuilds (the `(anonymous_N)` counter renumbers).
    /// Only the callee is used, never a sibling string argument.
    ///
    /// Defaults to false. The default JSON output stays byte-identical to what
    /// Istanbul consumers expect.
    pub name_callback_arguments: Option<bool>,
}

/// Options for `remapCoverageMap` and `remapCoverageMapWithLoader`.
///
/// Defaults to the legacy keep-generated-position behaviour, so callers that
/// omit this argument see no change.
#[napi(object)]
pub struct RemapOptions {
    /// When `true`, statement / function / branch entries whose positions
    /// cannot be looked up in the source map are pruned, along with their
    /// matching `s` / `f` / `b` / `bT` hit-count slots.
    ///
    /// Drop semantics mirror `istanbul-lib-source-maps`'s `transformer.js`:
    /// statements drop when start or end fails to remap; functions drop when
    /// any of `decl` / `loc` start or end fails (a matching
    /// `x_fallow_functionMap` overlay entry drops with the function); branch
    /// arms drop per arm, and the whole branch drops when no arms survive or
    /// when the umbrella `loc` start/end fails to remap.
    ///
    /// Defaults to `false`.
    pub drop_unmapped: Option<bool>,
}

fn core_remap_options_from(options: Option<RemapOptions>) -> CoreRemapOptions {
    CoreRemapOptions { drop_unmapped: options.and_then(|o| o.drop_unmapped).unwrap_or(false) }
}

/// A coverage pragma comment that was found but not handled.
#[napi(object)]
pub struct UnhandledPragma {
    /// The full comment text.
    pub comment: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based column.
    pub column: u32,
}

/// Result of instrumenting a source file.
#[napi(object)]
pub struct InstrumentResult {
    /// The instrumented source code with coverage counters injected.
    pub code: String,
    /// Istanbul-compatible coverage map as a JSON string.
    /// Parse with `JSON.parse()` to get the coverage object.
    pub coverage_map: String,
    /// Output source map JSON string (only present if source_map option is true).
    pub source_map: Option<String>,
    /// Unhandled pragma comments found during instrumentation.
    pub unhandled_pragmas: Vec<UnhandledPragma>,
}

/// Instrument a JavaScript/TypeScript source file for coverage collection.
///
/// Parses the source with Oxc, injects Istanbul-compatible coverage counters
/// via AST mutation, and returns the instrumented code with a coverage map.
#[napi]
pub fn instrument(
    source: String,
    filename: String,
    options: Option<InstrumentOptions>,
) -> napi::Result<InstrumentResult> {
    let opts = native_instrument_options(options)?;
    let result = oxc_coverage_instrument::instrument(&source, &filename, &opts)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    let unhandled_pragmas = result
        .unhandled_pragmas
        .into_iter()
        .map(|p| UnhandledPragma { comment: p.comment, line: p.line, column: p.column })
        .collect();

    Ok(InstrumentResult {
        code: result.code,
        coverage_map: result.coverage_map_json,
        source_map: result.source_map,
        unhandled_pragmas,
    })
}

fn native_instrument_options(
    options: Option<InstrumentOptions>,
) -> napi::Result<oxc_coverage_instrument::InstrumentOptions> {
    let Some(o) = options else {
        return Ok(oxc_coverage_instrument::InstrumentOptions::default());
    };

    let experimental = o.experimental_decorators.unwrap_or(false);
    let metadata = o.emit_decorator_metadata.unwrap_or(false);
    let decorator_mode = decorator_mode_from_flags(experimental, metadata)?;
    Ok(oxc_coverage_instrument::InstrumentOptions {
        coverage_variable: o.coverage_variable.unwrap_or_else(|| "__coverage__".to_string()),
        source_map: o.source_map.unwrap_or(false),
        input_source_map: o.input_source_map,
        compose_input_source_map: o.compose_input_source_map.unwrap_or(false),
        report_logic: o.report_logic.unwrap_or(false),
        track_optional_chain: o.track_optional_chain_branches.unwrap_or(true),
        ignore_class_methods: o.ignore_class_methods.unwrap_or_default(),
        strip_typescript: o.strip_typescript.unwrap_or(false),
        decorator_mode,
        function_identity_overlay: o.function_identity_overlay.unwrap_or(false),
        name_callback_arguments: o.name_callback_arguments.unwrap_or(false),
    })
}

/// Remap a coverage-final.json-shaped JSON string through each entry's
/// embedded `inputSourceMap` (typically attached during instrumentation).
///
/// Entries without an `inputSourceMap` are returned unchanged under their
/// original key. Entries with one are walked through the map and re-keyed by
/// the original source path (with `sourceRoot` joined per
/// `istanbul-lib-source-maps` semantics). Returns the remapped JSON.
/// A generated file that maps to several original sources fans out to one
/// entry per source. Entries from several chunks that resolve to the same path
/// merge by Istanbul location identity and sum their counters.
///
/// Every returned entry satisfies the Istanbul merge invariant
/// `keys(s) ⊆ keys(statementMap)` (and the same for `f`/`fnMap`,
/// `b`/`bT`/`branchMap`): an orphan counter, an `s`/`f`/`b` key with no
/// matching location-map entry, is dropped rather than passed through. Such an
/// orphan crashes `istanbul-lib-coverage`'s `CoverageMap.merge` (and therefore
/// `nyc report`) with "Cannot destructure property 'start' of 'undefined'"; it
/// can reach the input when an upstream instrumenter incremented a counter
/// whose map slot was later pruned (issue #107). Dropping it is a no-op on
/// already-consistent coverage.
///
/// Equivalent to `createSourceMapStore().transformCoverage(coverageMap)` in
/// the Vitest istanbul reporter path. For nyc's disk-read flow (Mode A
/// fallback) use [`remap_coverage_map_with_loader`] and supply a
/// `Record<string, string>` of preloaded maps keyed by FileCoverage path.
///
/// Pass `{ dropUnmapped: true }` to align with `istanbul-lib-source-maps`'s
/// `transformer.js` and drop entries whose positions cannot be looked up in
/// the source map (instead of silently keeping their generated-output
/// coordinates). Ambiguous unmapped entries in a multi-source map are omitted
/// in either mode because they have no safe original owner. See
/// [`RemapOptions`] for the full per-kind drop semantics.
#[napi]
pub fn remap_coverage_map(
    coverage_json: String,
    options: Option<RemapOptions>,
) -> napi::Result<String> {
    let parsed = oxc_coverage_instrument::parse_coverage_map(&coverage_json)
        .map_err(|e| invalid_coverage_json_error(&coverage_json, e))?;
    let core_options = core_remap_options_from(options);
    let remapped = oxc_coverage_instrument::remap_coverage_map_with_options(&parsed, core_options);
    serde_json::to_string(&remapped)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}

/// Like [`remap_coverage_map`], but with a preloaded map dictionary used as
/// the Mode A disk-read fallback. Entries whose path is a key in
/// `source_maps` and which carry no embedded `inputSourceMap` use the
/// dictionary's value as the source map JSON. Each `source_maps` value must
/// be a valid source map JSON string; entries that fail to parse silently
/// pass through.
///
/// The dictionary form matches the practical Jest/nyc/istanbul JS workflow:
/// the caller has already read maps from disk (or another source) before
/// calling the converter. The richer Rust-side
/// [`oxc_coverage_instrument::SourceMapStore`] (Mode B continuous remap)
/// stays Rust-only until a Jest provider integration targets it directly.
#[napi]
pub fn remap_coverage_map_with_loader(
    coverage_json: String,
    source_maps: HashMap<String, String>,
    options: Option<RemapOptions>,
) -> napi::Result<String> {
    let parsed = oxc_coverage_instrument::parse_coverage_map(&coverage_json)
        .map_err(|e| invalid_coverage_json_error(&coverage_json, e))?;
    let core_options = core_remap_options_from(options);
    let mut store = oxc_coverage_instrument::SourceMapStore::new();
    for (path, coverage) in &parsed {
        if coverage.input_source_map.is_none()
            && let Some(json) = source_maps.get(path)
        {
            store.add_map_json(path.clone(), json);
        }
    }
    let remapped = store.transform_coverage_map_with_options(&parsed, core_options);
    serde_json::to_string(&remapped)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}

/// Convert V8 byte-range coverage into Istanbul `FileCoverage` JSON.
///
/// `v8FunctionsJson` is the JSON array shape that the V8 inspector emits
/// under `Profiler.takePreciseCoverage().result[].functions` (the same shape
/// Node's `--experimental-coverage` and `@vitest/coverage-v8` consume).
///
/// `wrapperLength` accounts for Node's CJS module wrapper prefix and defaults
/// to 0 (correct for ESM).
///
/// Returns a JSON object compatible with Istanbul's `FileCoverage`. Statement,
/// function, and branch counts are populated from the V8 ranges. Branch arm
/// counts resolve correctly for if-else (arm\[0\] via the collected
/// consequent-body byte span, arm\[1\] via the alternate-body span) and switch
/// cases with `{ ... }` bodies. Branch arms with no matching V8 range
/// (ternary consequent/alternate, logical-expr right-hand operands, and
/// `default-arg` expressions) report `0`; this is honest under-reporting,
/// not over-reporting, so CI coverage thresholds will not silently pass on
/// un-instrumented arms.
///
/// When the source ends with a `//# sourceMappingURL=data:application/json;base64,...`
/// (or percent-encoded) trailer, the embedded map is decoded and attached
/// to the result as `inputSourceMap`. For external `//# sourceMappingURL=foo.js.map`
/// references, use [`v8_to_istanbul_with_loader`] and pass a dictionary of
/// URL -> map JSON entries.
///
/// If the returned object has `inputSourceMap` set, chain `remapCoverageMap`
/// next to resolve coverage positions back to the original source; otherwise
/// the inline map will ride along and downstream JS reporters that also call
/// into `istanbul-lib-source-maps` may double-remap.
#[napi]
pub fn v8_to_istanbul(
    source: String,
    filename: String,
    v8_functions_json: String,
    wrapper_length: Option<u32>,
) -> napi::Result<String> {
    let functions: Vec<oxc_coverage_instrument::V8FunctionCoverage> =
        serde_json::from_str(&v8_functions_json).map_err(|e| {
            napi::Error::new(napi::Status::InvalidArg, format!("invalid V8 functions JSON: {e}"))
        })?;
    let result = oxc_coverage_instrument::v8_to_istanbul(
        &source,
        &filename,
        &functions,
        wrapper_length.unwrap_or(0),
    )
    .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}

/// Like [`v8_to_istanbul`], but accepts a preloaded `external_source_maps`
/// dictionary used to resolve external `//# sourceMappingURL=` references.
/// The dictionary key is the URL as it appears in the source's trailing
/// comment (e.g. `foo.js.map`); the value is the map's JSON content. If the
/// source has an inline data-URL map the dictionary is not consulted.
/// Entries whose value fails to parse silently leave `inputSourceMap` unset.
#[napi]
pub fn v8_to_istanbul_with_loader(
    source: String,
    filename: String,
    v8_functions_json: String,
    external_source_maps: HashMap<String, String>,
    wrapper_length: Option<u32>,
) -> napi::Result<String> {
    let functions: Vec<oxc_coverage_instrument::V8FunctionCoverage> =
        serde_json::from_str(&v8_functions_json).map_err(|e| {
            napi::Error::new(napi::Status::InvalidArg, format!("invalid V8 functions JSON: {e}"))
        })?;
    let result = oxc_coverage_instrument::v8_to_istanbul_with_loader(
        &source,
        &filename,
        &functions,
        wrapper_length.unwrap_or(0),
        |url| external_source_maps.get(url).cloned(),
    )
    .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
}
