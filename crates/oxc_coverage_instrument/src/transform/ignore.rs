//! Predicates deciding whether a node is suppressed by an istanbul ignore
//! pragma, by an enclosing ignored subtree, or by `ignoreClassMethods`.

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxc_traverse::TraverseCtx;

use crate::pragma::{IgnoreType, PragmaMap};

use super::{CoverageState, CoverageTransform};

impl CoverageTransform<'_, '_> {
    pub(super) fn in_ignored_subtree(&self) -> bool {
        self.ignored_fn_stack.iter().any(|&ignored| ignored)
            || self.ignored_stmt_stack.iter().any(|&ignored| ignored)
            || self.ignored_prop_stack.iter().any(|&ignored| ignored)
            || self.ignored_switch_case_stack.iter().any(|&ignored| ignored)
    }

    pub(super) fn is_in_ignored_if_arm(&self, span: Span) -> bool {
        self.ignored_if_arm_spans
            .iter()
            .any(|ignored| ignored.start <= span.start && span.end <= ignored.end)
    }

    /// Open an ignore frame for a class member, object property or JSX node.
    /// An ignored frame consumes `skip_next`: the pragma binds to this node and
    /// must not reach the next one.
    pub(super) fn push_prop_ignore_frame(&mut self, ignored: bool) {
        self.ignored_prop_stack.push(ignored);
        if ignored {
            self.skip_next = false;
        }
    }
}

/// Walk to the enclosing `BindingProperty` of an `AssignmentPattern` and
/// return true if it carries an `ignore next` pragma at its start. Handles
/// the common shape `function f({ /* istanbul ignore next */ key: x = 1 })`,
/// where the pragma anchors on the property's key, not on the
/// AssignmentPattern itself.
pub(super) fn enclosing_destructure_property_pragma(ctx: &TraverseCtx<'_, CoverageState>) -> bool {
    use oxc_traverse::Ancestor;
    for a in ctx.ancestors() {
        match a {
            Ancestor::AssignmentPatternLeft(_) | Ancestor::AssignmentPatternRight(_) => {}
            Ancestor::BindingPropertyValue(prop) => {
                return ctx.state.pragmas.get(prop.span().start) == Some(IgnoreType::Next);
            }
            Ancestor::BindingPropertyKey(prop) => {
                return ctx.state.pragmas.get(prop.span().start) == Some(IgnoreType::Next);
            }
            _ => return false,
        }
    }
    false
}

pub(super) fn is_ignored_case(case: &SwitchCase, pragmas: &PragmaMap) -> bool {
    pragmas.get(case.span.start) == Some(IgnoreType::Next)
        || case
            .consequent
            .first()
            .is_some_and(|stmt| pragmas.get(stmt.span().start) == Some(IgnoreType::Next))
}

pub(super) fn jsx_attribute_ignored(
    attr: &JSXAttribute,
    pragmas: &PragmaMap,
    skip_next: bool,
) -> bool {
    pragmas.get(attr.span.start) == Some(IgnoreType::Next)
        || pragmas.get(attr.name.span().start) == Some(IgnoreType::Next)
        || skip_next
}

pub(super) fn jsx_spread_attribute_ignored(
    attr: &JSXSpreadAttribute,
    pragmas: &PragmaMap,
    skip_next: bool,
) -> bool {
    pragmas.get(attr.span.start) == Some(IgnoreType::Next)
        || pragmas.get(attr.argument.span().start) == Some(IgnoreType::Next)
        || skip_next
}

pub(super) fn jsx_child_ignored(child: &JSXChild, pragmas: &PragmaMap, skip_next: bool) -> bool {
    pragmas.get(child.span().start) == Some(IgnoreType::Next)
        || match child {
            JSXChild::ExpressionContainer(container) => {
                pragmas.get(container.expression.span().start) == Some(IgnoreType::Next)
            }
            JSXChild::Spread(spread) => {
                pragmas.get(spread.expression.span().start) == Some(IgnoreType::Next)
            }
            _ => false,
        }
        || skip_next
}

#[derive(Clone, Copy)]
pub(super) struct MethodPragmaInput<'a> {
    pub(super) method: &'a MethodDefinition<'a>,
    pub(super) key_span: Span,
    pub(super) skip_next: bool,
}

pub(super) fn method_ignored_by_pragma(
    input: MethodPragmaInput<'_>,
    ctx: &TraverseCtx<'_, CoverageState>,
) -> bool {
    let MethodPragmaInput { method, key_span, skip_next } = input;
    !matches!(method.key, PropertyKey::PrivateIdentifier(_))
        && (ctx.state.pragmas.get(method.span.start) == Some(IgnoreType::Next)
            || ctx.state.pragmas.get(key_span.start) == Some(IgnoreType::Next)
            || skip_next)
}

pub(super) fn mark_ignored_declarator_fn(decl: &VariableDeclarator<'_>, skip_next: &mut bool) {
    if matches!(
        decl.init,
        Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
    ) {
        *skip_next = true;
    }
}
