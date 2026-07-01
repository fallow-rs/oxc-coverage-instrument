//! Tests for the opt-in `name_callback_arguments` option, which names an
//! otherwise-anonymous function/arrow that is a direct call/`new` argument
//! from its callee (`arr.map(cb)` -> `"map"`). `istanbul-lib-instrument`
//! leaves these `(anonymous_N)`, so the default output is unchanged; only the
//! flag surfaces the callee-derived names.

use oxc_coverage_instrument::{InstrumentOptions, instrument};

fn names(source: &str, name_callbacks: bool) -> Vec<String> {
    let options =
        InstrumentOptions { name_callback_arguments: name_callbacks, ..Default::default() };
    let result = instrument(source, "t.js", &options).unwrap();
    // fn_map is a BTreeMap keyed by the sequential id string ("0", "1", ...);
    // BTreeMap orders lexicographically, which is good enough for these small
    // single- or double-function fixtures.
    result.coverage_map.fn_map.values().map(|entry| entry.name.clone()).collect()
}

#[test]
fn default_leaves_callback_arguments_anonymous() {
    // Off by default: byte-identical to istanbul for a plain callback.
    let got = names("arr.map((x) => x + 1);", false);
    assert_eq!(got.len(), 1);
    assert!(
        got[0].starts_with("(anonymous_"),
        "default output must stay anonymous, got {}",
        got[0]
    );
}

#[test]
fn member_callee_names_the_callback() {
    assert_eq!(names("arr.map((x) => x + 1);", true), vec!["map"]);
}

#[test]
fn plain_identifier_callee_names_the_callback() {
    assert_eq!(names("useMemo(() => compute());", true), vec!["useMemo"]);
}

#[test]
fn function_expression_argument_is_named_too() {
    // Not just arrows: an anonymous `function` expression as an argument.
    assert_eq!(names("run(function () { return 1; });", true), vec!["run"]);
}

#[test]
fn new_expression_callee_names_the_callback() {
    assert_eq!(names("new Promise((resolve) => resolve(1));", true), vec!["Promise"]);
}

#[test]
fn later_argument_after_a_string_literal_is_named_from_callee() {
    // The classic event-handler / route-handler shape: the function is the
    // second argument, after a string. It is named from the callee, not the
    // string (the argument-position ancestor cannot see sibling arguments).
    assert_eq!(
        names("el.addEventListener(\"click\", () => handle());", true),
        vec!["addEventListener"]
    );
}

#[test]
fn computed_string_key_callee_is_named() {
    assert_eq!(names("obj[\"handler\"](() => run());", true), vec!["handler"]);
}

#[test]
fn binding_name_is_unaffected_by_the_flag() {
    // The callee fallback runs only after the binding name (checked first), so
    // a bound arrow keeps its declarator name with the flag on. Its body being
    // a call (`run()`) does not pull "run": the arrow is not `run`'s argument.
    assert_eq!(names("const handler = () => run();", true), vec!["handler"]);
}

#[test]
fn named_function_expression_keeps_its_own_id() {
    // `func.id` beats the callee: an explicitly named function expression
    // reports its declared name, not the callee.
    assert_eq!(names("run(function inner() { return 1; });", true), vec!["inner"]);
}

#[test]
fn iife_callee_stays_anonymous() {
    // The function is the *callee*, not an argument, so it is not a callback.
    let got = names("(function () { return 1; })();", true);
    assert_eq!(got.len(), 1);
    assert!(got[0].starts_with("(anonymous_"), "IIFE must stay anonymous, got {}", got[0]);
}

#[test]
fn computed_non_string_callee_stays_anonymous() {
    // `arr[i](cb)` has no static callee name to borrow.
    let got = names("handlers[index](() => run());", true);
    assert_eq!(got.len(), 1);
    assert!(
        got[0].starts_with("(anonymous_"),
        "computed non-string callee must stay anonymous, got {}",
        got[0]
    );
}

#[test]
fn nested_callbacks_each_take_their_own_callee() {
    // `outer(() => inner(() => {}))`: the inner arrow pulls "inner", the outer
    // arrow pulls "outer". Each function reads its own immediate call ancestor,
    // so nesting resolves correctly.
    let mut got = names("outer(() => inner(() => 1));", true);
    got.sort();
    assert_eq!(got, vec!["inner", "outer"]);
}

#[test]
fn repeated_callee_names_are_stable_and_repeat() {
    // Two `.map` callbacks both surface as "map"; they are kept distinct by
    // their line/column in the fnMap, and the name never renumbers (unlike
    // `(anonymous_N)`), which matters for downstream identity.
    let got = names("a.map((x) => x);\nb.map((y) => y);", true);
    assert_eq!(got, vec!["map", "map"]);
}

#[test]
fn parenthesized_identifier_callee_names_the_callback() {
    // Oxc keeps `ParenthesizedExpression` as a real node; `(foo)(cb)` unwraps
    // the paren to name the callback "foo".
    assert_eq!(names("(foo)(() => run());", true), vec!["foo"]);
}

#[test]
fn parenthesized_member_callee_names_the_callback() {
    // The paren wraps a member access: `(a.b)(cb)` -> "b".
    assert_eq!(names("(a.b)(() => run());", true), vec!["b"]);
}

#[test]
fn parenthesized_argument_is_named_from_callee() {
    // The callback itself is parenthesized in the arguments position:
    // `foo((function () {}))`. The paren ancestor is skipped to reach the call.
    assert_eq!(names("foo((function () { return 1; }));", true), vec!["foo"]);
}

#[test]
fn iife_with_parenthesized_callee_stays_anonymous() {
    // `(() => 1)()` is an IIFE: the arrow is the (parenthesized) callee, not an
    // argument, so skipping the paren lands on the callee position and it stays
    // anonymous. Guards that paren-unwrapping does not misname an IIFE.
    let got = names("(() => 1)();", true);
    assert_eq!(got.len(), 1);
    assert!(got[0].starts_with("(anonymous_"), "IIFE must stay anonymous, got {}", got[0]);
}
