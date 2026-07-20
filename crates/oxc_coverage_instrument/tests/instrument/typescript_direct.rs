//! Tests for `InstrumentOptions::strip_typescript`. Verifies that raw
//! TypeScript source can be instrumented in one shot: the output is valid
//! JavaScript (type annotations removed), and statementMap / branchMap
//! positions still refer to the original TypeScript source offsets.

use oxc_coverage_instrument::{DecoratorMode, InstrumentError, InstrumentOptions, instrument};

use crate::common::assert_reparses;

fn ts_opts() -> InstrumentOptions {
    InstrumentOptions { source_map: true, strip_typescript: true, ..InstrumentOptions::default() }
}

#[test]
fn ts_direct_output_is_valid_js() {
    let src = "const x: number = 1;\nconsole.log(x);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert!(
        !result.code.contains(": number"),
        "type annotation must be stripped, got: {}",
        result.code
    );
    assert!(
        !result.code.contains("interface "),
        "interface declarations must be stripped, got: {}",
        result.code
    );
    assert!(
        result.code.contains("const x ="),
        "executable JS variable declaration expected, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_ignore_file_pragmas_still_strip_typescript() {
    for tool in ["istanbul", "v8", "c8"] {
        let src = format!(
            "/* {tool} ignore file */\ninterface Hidden {{ value: number }}\nexport const add = (a: number, b: number): number => a + b;"
        );
        let result = instrument(&src, "app.ts", &ts_opts()).expect("instrument");

        assert!(!result.code.contains("interface Hidden"), "{tool}: {code}", code = result.code);
        assert!(!result.code.contains(": number"), "{tool}: {code}", code = result.code);
        assert!(!result.code.contains("cov_"), "{tool}: ignored file must have no preamble");
        assert_reparses(&result.code, "app.js");
        assert!(result.coverage_map.statement_map.is_empty(), "{tool}");
        assert!(result.coverage_map.fn_map.is_empty(), "{tool}");
        assert!(result.coverage_map.branch_map.is_empty(), "{tool}");
        assert!(result.source_map.is_none(), "{tool}: preserve ignored-file source-map contract");
    }
}

#[test]
fn ts_direct_statement_map_positions_are_ts_positions() {
    let src = "const x: number = 1;\nconst y: string = \"hi\";\nconsole.log(x, y);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    let map = &result.coverage_map.statement_map;
    assert_eq!(map.len(), 3, "expected 3 statements, got {}: {map:#?}", map.len());

    let s0 = &map["0"];
    assert_eq!(s0.start.line, 1, "statement 0 should be on line 1, got {s0:?}");
    let s1 = &map["1"];
    assert_eq!(s1.start.line, 2, "statement 1 should be on line 2, got {s1:?}");
    let s2 = &map["2"];
    assert_eq!(s2.start.line, 3, "statement 2 should be on line 3, got {s2:?}");
}

#[test]
fn ts_direct_source_map_sources_and_content() {
    let src = "const x: number = 1;\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    let sm: serde_json::Value =
        serde_json::from_str(result.source_map.as_deref().expect("source map")).expect("json");
    assert_eq!(sm["sources"][0], "app.ts");
    let content = sm["sourcesContent"][0].as_str().expect("sourcesContent[0]");
    assert!(
        content.contains(": number"),
        "sourcesContent must retain TS annotations, got: {content}"
    );
}

#[test]
fn ts_direct_type_only_nodes_produce_no_counters() {
    let src = "interface Foo { a: number; }\ntype Bar = string;\ndeclare const baz: number;\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert_eq!(
        result.coverage_map.statement_map.len(),
        0,
        "type-only nodes must not produce statement counters: {:#?}",
        result.coverage_map.statement_map
    );
    assert_eq!(result.coverage_map.fn_map.len(), 0);
    assert_eq!(result.coverage_map.branch_map.len(), 0);
}

#[test]
fn ts_direct_interface_is_skipped_in_position_map() {
    let src = "const x: number = 1;\nconst y: string = \"hi\";\ninterface Foo { a: number }\nconsole.log(x, y);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    let lines: Vec<u32> =
        result.coverage_map.statement_map.values().map(|loc| loc.start.line).collect();
    assert!(!lines.contains(&3), "interface on line 3 must not appear in statement_map: {lines:?}");
    assert!(lines.contains(&1));
    assert!(lines.contains(&2));
    assert!(lines.contains(&4));
}

#[test]
fn ts_direct_typescript_function_gets_counter() {
    let src = "function add(a: number, b: number): number {\n  return a + b;\n}\nadd(1, 2);\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert!(!result.code.contains(": number"), "param annotations stripped");
    assert_eq!(result.coverage_map.fn_map.len(), 1, "function counter expected");
    let f = &result.coverage_map.fn_map["0"];
    assert_eq!(f.name, "add");
    assert_eq!(f.decl.start.line, 1);
}

#[test]
fn ts_direct_parse_error_returns_parse_error() {
    let src = "const x: = ;\n";
    let err = instrument(src, "app.ts", &ts_opts()).expect_err("parse must fail");
    assert!(matches!(err, InstrumentError::ParseError(_)), "expected ParseError, got {err:?}");
}

#[test]
fn ts_direct_off_leaves_type_annotations_in_output() {
    let src = "const x: number = 1;\n";
    let opts = InstrumentOptions::default();
    assert!(!opts.strip_typescript);
    let result = instrument(src, "app.ts", &opts).expect("instrument");
    assert!(
        result.code.contains(": number"),
        "with strip_typescript=false the TS annotation must remain, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_tsx_jsx_is_preserved_in_output() {
    let src = "const greet = (name: string) => <div>Hello {name}</div>;\nconsole.log(greet(\"world\"));\n";
    let result = instrument(src, "app.tsx", &ts_opts()).expect("instrument");
    assert!(
        result.code.contains("<div>"),
        "JSX must survive strip_typescript on .tsx, got: {}",
        result.code
    );
    assert!(!result.code.contains(": string"), "TS annotation must be stripped on .tsx");
    assert!(
        !result.code.contains("React.createElement"),
        "JSX must not be transformed to call form, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_js_file_passes_through_unchanged() {
    let src = "const x = 1;\nconsole.log(x);\n";
    let result = instrument(src, "app.js", &ts_opts()).expect("instrument");
    assert!(result.code.contains("const x ="));
    assert_eq!(result.coverage_map.statement_map.len(), 2);
}

// Sample NestJS-style source exercising every decorator position
// emit_decorator_metadata cares about: class-level decorator,
// constructor parameter injection, method decorator, parameter decorator,
// and a typed return.
const NESTJS_SAMPLE: &str = "import { Injectable, Body } from '@nestjs/common';\n\
@Injectable()\n\
export class FooService {\n  \
  constructor(private readonly bar: BarRepo) {}\n  \
  method(@Body() dto: CreateDto): string {\n    \
    return dto.name;\n  \
  }\n\
}\n";

#[test]
fn ts_direct_decorator_metadata_emits_helper_imports() {
    // With both flags on, oxc_transformer lowers @Injectable() into a
    // `_decorate(...)` call and emits `_decorateMetadata("design:type", ...)`,
    // `_decorateMetadata("design:paramtypes", ...)`, and
    // `_decorateMetadata("design:returntype", ...)` for every decorated
    // member that carries TypeScript type annotations. The helpers are
    // imported from `@oxc-project/runtime`.
    let opts = InstrumentOptions {
        strip_typescript: true,
        decorator_mode: DecoratorMode::ExperimentalWithMetadata,
        ..InstrumentOptions::default()
    };
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    assert!(
        result.code.contains("import _decorate from \"@oxc-project/runtime/helpers/decorate\""),
        "expected _decorate import, got: {}",
        result.code
    );
    assert!(
        result.code.contains(
            "import _decorateMetadata from \"@oxc-project/runtime/helpers/decorateMetadata\""
        ),
        "expected _decorateMetadata import, got: {}",
        result.code
    );
    assert!(
        result.code.contains("_decorateMetadata(\"design:paramtypes\""),
        "expected design:paramtypes metadata call, got: {}",
        result.code
    );
    assert!(
        result.code.contains("_decorateMetadata(\"design:type\""),
        "expected design:type metadata call, got: {}",
        result.code
    );
    assert!(
        result.code.contains("_decorateMetadata(\"design:returntype\""),
        "expected design:returntype metadata call, got: {}",
        result.code
    );
}

/// A nullable union is the only thing `strict_null_checks` changes, and it
/// changes it silently: the emitted code runs either way, but NestJS DI,
/// TypeORM column inference, and class-validator all read `design:type` and
/// would see a different constructor. Pin both directions so the default
/// cannot drift with an upstream change.
const NULLABLE_PROPERTY_SAMPLE: &str = "function D() { return function (_t: any, _k: any) {}; }\n\
export class C {\n\
  @D() foo: string | null = null;\n\
}\n";

#[test]
fn ts_direct_strict_null_checks_widens_nullable_design_type() {
    let opts = InstrumentOptions {
        strip_typescript: true,
        decorator_mode: DecoratorMode::ExperimentalWithMetadata,
        strict_null_checks: true,
        ..InstrumentOptions::default()
    };
    let result = instrument(NULLABLE_PROPERTY_SAMPLE, "c.ts", &opts).expect("instrument");
    assert!(
        result.code.contains("_decorateMetadata(\"design:type\", Object)"),
        "under strictNullChecks `string | null` should widen to Object, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_without_strict_null_checks_elides_null_from_design_type() {
    let opts = InstrumentOptions {
        strip_typescript: true,
        decorator_mode: DecoratorMode::ExperimentalWithMetadata,
        strict_null_checks: false,
        ..InstrumentOptions::default()
    };
    let result = instrument(NULLABLE_PROPERTY_SAMPLE, "c.ts", &opts).expect("instrument");
    assert!(
        result.code.contains("_decorateMetadata(\"design:type\", String)"),
        "without strictNullChecks `null` should be elided, leaving String, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_strict_null_checks_defaults_to_true() {
    assert!(
        InstrumentOptions::default().strict_null_checks,
        "default must match tsc under `strict`, which NestJS and TypeORM users compile with"
    );
}

#[test]
fn ts_direct_decorator_metadata_statement_counters_land_on_real_lines() {
    // The decorator lowering pass produces synthetic helper-call expressions.
    // Statement and function counters must only anchor on lines that exist
    // in the original TypeScript source (1..=8 for `NESTJS_SAMPLE`); they
    // must never land at synthetic line 0 or beyond the original source end.
    // The typeof guards `emit_decorator_metadata` inserts carry a synthetic
    // span and register no branch, so they cannot appear here.
    let opts = InstrumentOptions {
        strip_typescript: true,
        decorator_mode: DecoratorMode::ExperimentalWithMetadata,
        ..InstrumentOptions::default()
    };
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    let source_line_count = u32::try_from(NESTJS_SAMPLE.matches('\n').count()).unwrap();

    for (key, loc) in &result.coverage_map.statement_map {
        assert!(
            loc.start.line >= 1 && loc.start.line <= source_line_count,
            "statement {key} anchors on out-of-range line {}: {loc:?}",
            loc.start.line
        );
    }
    for (key, f) in &result.coverage_map.fn_map {
        assert!(
            f.decl.start.line >= 1 && f.decl.start.line <= source_line_count,
            "fn {key} declaration anchors on out-of-range line {}: {f:?}",
            f.decl.start.line
        );
    }
    // Sanity: at least the class body and the method's return statement.
    assert!(
        !result.coverage_map.statement_map.is_empty(),
        "expected at least one statement counter on the class body"
    );
    // Sanity: constructor + method.
    assert_eq!(
        result.coverage_map.fn_map.len(),
        2,
        "expected constructor + method fn counters, got: {:#?}",
        result.coverage_map.fn_map
    );
}

#[test]
fn ts_direct_experimental_decorators_only_no_metadata() {
    // class-validator and decorator-using libraries that DON'T need runtime
    // type metadata should be able to lower decorators without paying the
    // design:type / design:paramtypes / design:returntype emission cost.
    let opts = InstrumentOptions {
        strip_typescript: true,
        decorator_mode: DecoratorMode::Experimental,
        ..InstrumentOptions::default()
    };
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    assert!(
        result.code.contains("_decorate("),
        "expected _decorate call (lowered decorator), got: {}",
        result.code
    );
    assert!(
        !result.code.contains("_decorateMetadata("),
        "no metadata emission expected when emit_decorator_metadata=false, got: {}",
        result.code
    );
}

#[test]
fn ts_direct_decorators_default_pass_through() {
    // The default mode emits the decorator syntax verbatim and imports no
    // helpers.
    let opts = InstrumentOptions { strip_typescript: true, ..InstrumentOptions::default() };
    assert_eq!(opts.decorator_mode, DecoratorMode::PassThrough);
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    assert!(
        result.code.contains("@Injectable()"),
        "decorator syntax must survive verbatim by default, got: {}",
        result.code
    );
    assert!(
        !result.code.contains("@oxc-project/runtime"),
        "no helper import expected by default, got: {}",
        result.code
    );
    assert!(
        !result.code.contains("_decorate("),
        "no lowering expected by default, got: {}",
        result.code
    );
}

#[test]
fn decorator_mode_legacy_and_emit_metadata_match_variant() {
    // The Rust API uses an enum so the invalid combination "emit metadata
    // without lowering decorators" is unrepresentable. The upstream
    // transformer flags must follow the enum variant exactly.
    assert!(!DecoratorMode::PassThrough.legacy());
    assert!(!DecoratorMode::PassThrough.emit_metadata());
    assert!(DecoratorMode::Experimental.legacy());
    assert!(!DecoratorMode::Experimental.emit_metadata());
    assert!(DecoratorMode::ExperimentalWithMetadata.legacy());
    assert!(DecoratorMode::ExperimentalWithMetadata.emit_metadata());
}

#[test]
fn ts_direct_decorator_metadata_does_not_emit_synthetic_branches() {
    // Issue #81: oxc_transformer's legacy decorator pass synthesizes
    // `typeof X === "function" ? X : Object` guards inside the
    // `_decorateMetadata("design:paramtypes", [...])` calls. Those
    // ConditionalExpression and BinaryExpression nodes carry zero-width
    // spans at byte 0, so naive branch instrumentation registers them at
    // L1:C0 and inflates the branch denominator for any NestJS / TypeORM
    // user who enables metadata. The branch instrumenter must filter
    // these synthetic nodes out the same way it already filters the
    // synthesized else-arm of a transformed enum IIFE.
    let opts = InstrumentOptions {
        strip_typescript: true,
        decorator_mode: DecoratorMode::ExperimentalWithMetadata,
        ..InstrumentOptions::default()
    };
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    for (key, entry) in &result.coverage_map.branch_map {
        let degenerate = (entry.loc.start.line, entry.loc.start.column)
            == (entry.loc.end.line, entry.loc.end.column)
            && entry.loc.start.line == 1
            && entry.loc.start.column == 0;
        assert!(
            !degenerate,
            "branch {key} ({}) anchors at L1:C0 with a zero-width span; \
             this is a synthetic typeof guard from the decorator metadata pass \
             and must be filtered out: {entry:?}",
            entry.branch_type
        );
        for (idx, loc) in entry.locations.iter().enumerate() {
            let degenerate = (loc.start.line, loc.start.column) == (loc.end.line, loc.end.column)
                && loc.start.line == 1
                && loc.start.column == 0;
            assert!(
                !degenerate,
                "branch {key} ({}) location {idx} anchors at L1:C0 (synthetic): {loc:?}",
                entry.branch_type
            );
        }
    }
}

#[test]
fn ts_direct_enum_counter_spans_original_source() {
    // TypeScript `enum` declarations are converted to an IIFE by the
    // transformer. Verify the resulting statement counters point at the
    // original source byte offsets, not synthetic (0, 0) spans.
    let src = "enum Color { Red, Green, Blue }\nconst c: Color = Color.Red;\n";
    let result = instrument(src, "app.ts", &ts_opts()).expect("instrument");
    assert!(result.coverage_map.statement_map.len() >= 4);

    let enum_stmt = &result.coverage_map.statement_map["0"];
    assert_eq!(enum_stmt.start.line, 1, "enum statement should anchor at line 1");
    assert!(
        enum_stmt.end.column > enum_stmt.start.column,
        "enum statement span must be non-degenerate: {enum_stmt:?}"
    );

    let member_lines: Vec<u32> =
        result.coverage_map.statement_map.values().map(|loc| loc.start.line).collect();
    assert!(
        member_lines.iter().all(|&l| l > 0),
        "no statement counter should land on synthetic line 0: {member_lines:?}"
    );
}
