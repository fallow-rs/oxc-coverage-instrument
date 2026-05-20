//! Tests for `InstrumentOptions::strip_typescript`. Verifies that raw
//! TypeScript source can be instrumented in one shot: the output is valid
//! JavaScript (type annotations removed), and statementMap / branchMap
//! positions still refer to the original TypeScript source offsets.

use oxc_coverage_instrument::{InstrumentError, InstrumentOptions, instrument};

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
fn ts_direct_default_off_preserves_existing_behavior() {
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
        experimental_decorators: true,
        emit_decorator_metadata: true,
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

#[test]
fn ts_direct_decorator_metadata_statement_counters_land_on_real_lines() {
    // The decorator lowering pass produces synthetic helper-call expressions.
    // Statement and function counters must only anchor on lines that exist
    // in the original TypeScript source (1..=8 for `NESTJS_SAMPLE`); they
    // must never land at synthetic line 0 or beyond the original source end.
    // Branch counters from emit_decorator_metadata typeof guards are tracked
    // separately under #81.
    let opts = InstrumentOptions {
        strip_typescript: true,
        experimental_decorators: true,
        emit_decorator_metadata: true,
        ..InstrumentOptions::default()
    };
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    let source_line_count = NESTJS_SAMPLE.matches('\n').count() as u32;

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
        experimental_decorators: true,
        emit_decorator_metadata: false,
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
    // Default behavior (both new flags false): decorator syntax flows
    // through verbatim, no helpers imported. Preserves v0.6.1 behavior.
    let opts = InstrumentOptions { strip_typescript: true, ..InstrumentOptions::default() };
    assert!(!opts.experimental_decorators);
    assert!(!opts.emit_decorator_metadata);
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
fn ts_direct_emit_decorator_metadata_implicitly_promotes_experimental() {
    // emit_decorator_metadata=true with experimental_decorators=false must
    // silently promote experimental_decorators (upstream's decorator pass
    // is gated on legacy mode). Output must contain metadata calls.
    let opts = InstrumentOptions {
        strip_typescript: true,
        experimental_decorators: false,
        emit_decorator_metadata: true,
        ..InstrumentOptions::default()
    };
    let result = instrument(NESTJS_SAMPLE, "foo.service.ts", &opts).expect("instrument");
    assert!(
        result.code.contains("_decorate("),
        "implicit legacy promotion must produce _decorate call, got: {}",
        result.code
    );
    assert!(
        result.code.contains("_decorateMetadata("),
        "metadata calls expected with implicit promotion, got: {}",
        result.code
    );
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
