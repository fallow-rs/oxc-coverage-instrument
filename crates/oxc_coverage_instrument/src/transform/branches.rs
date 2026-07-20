//! Branch instrumentation for `if` arms, ternary arms, logical assignment
//! and optional-chain links.

use std::mem;

use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN, Span};
use oxc_traverse::TraverseCtx;

use crate::pragma::IgnoreType;

use super::counters::{
    CounterKind, CounterType, PendingInsertion, build_counter_stmt, dummy_expr, index_literal,
    inject_branch_counter_into_statement, prepend_counter,
};
use super::coverage_map::is_synthetic_span;
use super::ignore::{enclosing_destructure_property_pragma, is_ignored_case};
use super::{CoverageState, CoverageTransform};

pub(super) struct ElseBranchInput<'arena, 'a> {
    pub(super) stmt: &'a mut IfStatement<'arena>,
    pub(super) branch_id: usize,
    pub(super) synthetic_anchor: u32,
}

pub(super) struct ConditionalArmInput<'arena, 'a> {
    pub(super) branch_id: usize,
    pub(super) arm: &'a mut Expression<'arena>,
    pub(super) ignored: bool,
}

pub(super) struct OptionalChainLinkInput<'arena, 'a> {
    pub(super) object: &'a mut Expression<'arena>,
    pub(super) link_span: Span,
}

impl<'arena> CoverageTransform<'_, 'arena> {
    /// Register the `if` branch and inject the counter for each arm the
    /// pragmas leave in place, synthesizing a missing `else` where needed.
    pub(super) fn instrument_if_branches(
        &mut self,
        stmt: &mut IfStatement<'arena>,
        ctx: &mut TraverseCtx<'arena, CoverageState>,
    ) {
        if self.in_ignored_subtree() {
            self.ignored_if_arm_push_counts.push(0);
            return;
        }
        let pragma = ctx.state.pragmas.get(stmt.span.start);
        self.record_ignored_if_arm(stmt, pragma);

        // istanbul-lib-instrument's `coverIfBranches` passes `n.loc`, the whole
        // `IfStatement` span, as the consequent location rather than the
        // narrower consequent block:
        // `insertBranchCounter(path.get('consequent'), branch, n.loc)` in
        // `istanbul-lib-instrument/src/visitor.js`. Reporters highlight that
        // range, so it is reproduced here. The consequent body span goes into
        // the side-table instead, because `v8_to_istanbul` has to resolve
        // arm[0] against V8's `BlockStatement` range and V8 emits no range
        // matching the whole-IfStatement convention.
        let consequent_span = stmt.span;
        let consequent_body_span = stmt.consequent.span();
        // Skipping the umbrella skips every counter under this branch, but the
        // traversal still recurses; the pragma-arm bookkeeping above and
        // `pop_ignored_if_arms`'s pop stay balanced either way.
        let Some(branch_id) = self.add_branch("if", stmt.span) else {
            return;
        };
        let cov_fn = self.cov_fn_name;

        if pragma != Some(IgnoreType::If)
            && let Some(path_idx) =
                self.add_branch_path_with_body(branch_id, consequent_span, consequent_body_span)
        {
            inject_branch_counter_into_statement(
                &mut stmt.consequent,
                CounterKind::branch(cov_fn, branch_id, path_idx),
                ctx,
            );
        }
        if pragma != Some(IgnoreType::Else) {
            self.inject_else_branch_counter(
                ElseBranchInput { stmt, branch_id, synthetic_anchor: consequent_body_span.end },
                ctx,
            );
        }
    }

    /// Drop the ignored-arm spans pushed by the `if` being left.
    pub(super) fn pop_ignored_if_arms(&mut self) {
        if let Some(count) = self.ignored_if_arm_push_counts.pop() {
            for _ in 0..count {
                self.ignored_if_arm_spans.pop();
            }
        }
    }

    /// Register the `cond-expr` branch for a ternary and inject a counter into
    /// each arm the pragmas leave in place.
    pub(super) fn instrument_conditional_branches(
        &mut self,
        expr: &mut ConditionalExpression<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        if self.in_ignored_subtree()
            || ctx.state.pragmas.get(expr.span.start) == Some(IgnoreType::Next)
            || is_synthetic_span(expr.span)
        {
            return;
        }
        let ignore_consequent =
            ctx.state.pragmas.get(expr.consequent.span().start) == Some(IgnoreType::Next);
        let ignore_alternate =
            ctx.state.pragmas.get(expr.alternate.span().start) == Some(IgnoreType::Next);
        if ignore_consequent && ignore_alternate {
            return;
        }

        let Some(branch_id) = self.add_branch("cond-expr", expr.span) else {
            return;
        };

        // istanbul drops only the pragma'd arm's location from the branch map,
        // so the entry survives with the one remaining arm still counted.
        self.inject_conditional_arm_counter(
            ConditionalArmInput {
                branch_id,
                arm: &mut expr.consequent,
                ignored: ignore_consequent,
            },
            ctx,
        );
        self.inject_conditional_arm_counter(
            ConditionalArmInput { branch_id, arm: &mut expr.alternate, ignored: ignore_alternate },
            ctx,
        );
    }

    /// Register the `switch` branch and prepend a counter to each case body
    /// the pragmas leave in place.
    pub(super) fn instrument_switch_cases(
        &mut self,
        stmt: &mut SwitchStatement<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        if self.in_ignored_subtree() {
            return;
        }
        let Some(branch_id) = self.add_branch("switch", stmt.span) else {
            return;
        };

        let cov_fn = self.cov_fn_name;
        for case in &mut stmt.cases {
            if is_ignored_case(case, &ctx.state.pragmas) {
                continue;
            }
            let Some(path_idx) = self.add_branch_path(branch_id, case.span) else {
                continue;
            };
            let branch_stmt =
                build_counter_stmt(CounterKind::branch(cov_fn, branch_id, path_idx), ctx);
            case.consequent.insert(0, branch_stmt);
        }
    }

    /// Register the `default-arg` branch for a parameter default and carry the
    /// parameter name into a function-valued default.
    pub(super) fn instrument_parameter_default(
        &mut self,
        param: &mut FormalParameter<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        if self.in_ignored_subtree() {
            return;
        }
        if ctx.state.pragmas.get(param.span.start) == Some(IgnoreType::Next) {
            return;
        }
        // Istanbul gives `function f(x = 1) {}` a `default-arg` branch with one
        // location, the default expression.
        if let Some(init) = &mut param.initializer {
            // `function f(cb = () => 1)` -> `fnMap[N].name = "cb"`: the inner
            // arrow or function is the direct initializer of the parameter
            // binding, so it inherits the parameter's name.
            if matches!(
                **init,
                Expression::FunctionExpression(_)
                    | Expression::ArrowFunctionExpression(_)
                    | Expression::ClassExpression(_)
            ) && let Some(id) = param.pattern.get_binding_identifier()
            {
                self.pending_name = Some(id.name.to_string());
            }
            let init_span = init.span();
            let Some(branch_id) = self.add_branch("default-arg", param.span) else {
                return;
            };
            if self.add_branch_path(branch_id, init_span).is_some() {
                prepend_counter(init, CounterKind::branch(self.cov_fn_name, branch_id, 0), ctx);
            }
        }
    }

    /// Register the `default-arg` branch for a destructuring default and carry
    /// the binding name into a function-valued default.
    pub(super) fn instrument_destructuring_default(
        &mut self,
        pattern: &mut AssignmentPattern<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        if self.in_ignored_subtree() {
            return;
        }
        // `/* istanbul ignore next */` can sit at the pattern itself (shorthand
        // object property, array element) or one level up at the enclosing
        // `BindingProperty`. Either binding suppresses the `default-arg`
        // branch on this default value.
        if ctx.state.pragmas.get(pattern.span.start) == Some(IgnoreType::Next)
            || enclosing_destructure_property_pragma(ctx)
        {
            return;
        }
        // Carry the binding name into any inner function or arrow on the right
        // of the default, so `function f(cb = () => 1)` and the destructuring
        // equivalents surface as `fnMap[N].name = "cb"`.
        if matches!(
            pattern.right,
            Expression::FunctionExpression(_)
                | Expression::ArrowFunctionExpression(_)
                | Expression::ClassExpression(_)
        ) && let Some(id) = pattern.left.get_binding_identifier()
        {
            self.pending_name = Some(id.name.to_string());
        }
        // Istanbul types destructuring defaults (`const { x = 1 } = obj`) as
        // `default-arg` too.
        let right_span = pattern.right.span();
        let Some(branch_id) = self.add_branch("default-arg", pattern.span) else {
            return;
        };
        if self.add_branch_path(branch_id, right_span).is_some() {
            prepend_counter(
                &mut pattern.right,
                CounterKind::branch(self.cov_fn_name, branch_id, 0),
                ctx,
            );
        }
    }

    pub(super) fn inject_conditional_arm_counter(
        &mut self,
        input: ConditionalArmInput<'arena, '_>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let ConditionalArmInput { branch_id, arm, ignored } = input;
        if ignored {
            return;
        }
        let Some(path_idx) = self.add_branch_path(branch_id, arm.span()) else {
            return;
        };
        prepend_counter(arm, CounterKind::branch(self.cov_fn_name, branch_id, path_idx), ctx);
    }

    pub(super) fn try_instrument_logical_assignment(
        &mut self,
        expr: &mut AssignmentExpression<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        if !is_logical_assignment_operator(expr.operator) {
            return;
        }
        let left_span = expr.left.span();
        let right_span = expr.right.span();
        let Some(branch_id) = self.add_branch("binary-expr", expr.span) else {
            return;
        };
        self.add_branch_path_location(
            branch_id,
            self.span_to_location(left_span),
            (left_span.start, left_span.end),
        );
        self.add_branch_path_location(
            branch_id,
            self.span_to_location(right_span),
            (right_span.start, right_span.end),
        );
        self.pending_insertions.push(PendingInsertion {
            target_start: expr.span.start,
            counter_id: branch_id,
            counter_type: CounterType::BranchLeft,
        });
        prepend_counter(&mut expr.right, CounterKind::branch(self.cov_fn_name, branch_id, 1), ctx);
    }

    /// Record which arm spans of an `if` are pragma-ignored, so statements
    /// nested inside an ignored arm register no counters of their own.
    pub(super) fn record_ignored_if_arm(
        &mut self,
        stmt: &IfStatement<'arena>,
        pragma: Option<IgnoreType>,
    ) {
        let mut ignored_arm_count = 0_usize;
        if pragma == Some(IgnoreType::If) {
            self.ignored_if_arm_spans.push(stmt.consequent.span());
            ignored_arm_count += 1;
        } else if pragma == Some(IgnoreType::Else)
            && let Some(alt) = &stmt.alternate
        {
            self.ignored_if_arm_spans.push(alt.span());
            ignored_arm_count += 1;
        }
        self.ignored_if_arm_push_counts.push(ignored_arm_count);
    }

    /// Synthesize a missing else-arm block where needed and inject its branch
    /// counter, as istanbul-lib-instrument's `coverIfBranches` does.
    ///
    /// `synthetic_anchor` is the offset reported as the synthetic else arm's
    /// location when the `IfStatement` has no `else` clause. Anchoring on the
    /// consequent's end keeps `branchMap[N].locations[1]` a real `Location`, so
    /// consumers reading `start.line` off it do not trip over a placeholder.
    pub(super) fn inject_else_branch_counter(
        &mut self,
        input: ElseBranchInput<'arena, '_>,
        ctx: &mut TraverseCtx<'arena, CoverageState>,
    ) {
        let ElseBranchInput { stmt, branch_id, synthetic_anchor } = input;
        // Resolved before any AST mutation: a real else uses its own span,
        // while a missing or already zero-width else anchors at the
        // consequent's end.
        let arm_span = match &stmt.alternate {
            Some(alt) if !is_synthetic_span(alt.span()) => alt.span(),
            _ => Span::new(synthetic_anchor, synthetic_anchor),
        };
        // V8 alternate ranges begin where the consequent ends and include the
        // `else` transition. Keep Istanbul's narrower alternate location for
        // reporters while retaining the V8-visible span for count matching.
        let v8_body_span = Span::new(synthetic_anchor, arm_span.end);
        // Resolved before the empty block is synthesized, so an else arm the
        // eager gate rejects leaves no spurious `else {}` in the output.
        let Some(path_idx) = self.add_branch_path_with_body(branch_id, arm_span, v8_body_span)
        else {
            return;
        };
        if stmt.alternate.is_none() {
            let scope_id =
                ctx.create_child_scope_of_current(oxc_syntax::scope::ScopeFlags::empty());
            stmt.alternate = Some(Statement::new_block_statement_with_scope_id(
                SPAN,
                ArenaVec::new_in(ctx),
                scope_id,
                ctx,
            ));
        }
        if let Some(alt) = &mut stmt.alternate {
            inject_branch_counter_into_statement(
                alt,
                CounterKind::branch(self.cov_fn_name, branch_id, path_idx),
                ctx,
            );
        }
    }

    /// Wrap an optional-chain link's `object`/`callee` with the
    /// `cov_fn_oc(...)` helper so each `?.` site records whether the
    /// observed value was nullish (arm 0) or continued (arm 1). The
    /// branch entry is typed `optional-chain` with two locations: a
    /// zero-width slot anchored at the link's start, plus the link's
    /// full span. Both run at the same source position; the convention
    /// keeps the JSON-shape consistent with two-arm branch types and
    /// lets reporters render either arm without divergent special cases.
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "takes `&mut` so the three traverse hooks can pass their own `ctx` through unchanged"
    )]
    pub(super) fn wrap_optional_chain_link(
        &mut self,
        input: OptionalChainLinkInput<'arena, '_>,
        ctx: &mut TraverseCtx<'arena, CoverageState>,
    ) {
        let OptionalChainLinkInput { object, link_span } = input;
        // The eager gate applies at the whole-branch level only: the
        // `cov_fn_oc` helper references fixed arm indices 0 and 1, so either
        // both arms are registered or the link is left unwrapped.
        let Some(branch_id) = self.add_branch("optional-chain", link_span) else {
            return;
        };
        let anchor = Span::new(link_span.start, link_span.start);
        let anchor_loc = self.span_to_location(anchor);
        self.add_branch_path_location(branch_id, anchor_loc, (anchor.start, anchor.end));
        let link_loc = self.span_to_location(link_span);
        self.add_branch_path_location(branch_id, link_loc, (link_span.start, link_span.end));

        // `cov_fn_oc(<original>, <branch_id>)` observes the value, increments
        // `b[id][0]` or `b[id][1]` on nullishness, and returns the value
        // unchanged so native `?.` semantics still fire. The three dispatch
        // points all gate on `track_optional_chain`, which is what sets the name.
        let oc_name = self
            .cov_fn_oc_name
            .expect("wrap_optional_chain_link runs only when track_optional_chain is on");
        let callee = Expression::new_identifier(SPAN, oc_name, ctx);
        let original = mem::replace(object, dummy_expr(ctx));
        let mut args = ArenaVec::new_in(ctx);
        args.push(Argument::from(original));
        args.push(Argument::from(index_literal(ctx, branch_id)));
        *object = Expression::new_call_expression(
            SPAN,
            callee,
            None::<TSTypeParameterInstantiation>,
            args,
            false,
            ctx,
        );
    }
}

fn is_logical_assignment_operator(operator: oxc_syntax::operator::AssignmentOperator) -> bool {
    use oxc_syntax::operator::AssignmentOperator;

    matches!(
        operator,
        AssignmentOperator::LogicalOr
            | AssignmentOperator::LogicalAnd
            | AssignmentOperator::LogicalNullish
    )
}
