//! `fnMap` entry registration for functions, arrows, methods and class
//! members, plus the name inheritance and class-field counter hoisting that
//! keep `Function.name` intact.

use std::collections::BTreeMap;
use std::mem;

use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN, Span};
use oxc_traverse::TraverseCtx;

use crate::pragma::IgnoreType;

use super::counters::{
    ClassFieldHoist, CounterKind, build_class_field_counter, build_counter_stmt, dummy_expr,
    prepend_counter,
};
use super::coverage_map::is_synthetic_span;
use super::ignore::{MethodPragmaInput, method_ignored_by_pragma};
use super::names::{callback_argument_name, method_label, property_key_to_name};
use super::{CoverageState, CoverageTransform};

impl<'arena> CoverageTransform<'_, 'arena> {
    /// Register the `fnMap` entry for a function and queue its counter for the
    /// body hook, or record that its subtree is suppressed.
    pub(super) fn register_function_entry(
        &mut self,
        func: &Function<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let has_pragma = ctx.state.pragmas.get(func.span.start) == Some(IgnoreType::Next);
        let ignored_named_function_expression = func.r#type == FunctionType::FunctionExpression
            && func
                .id
                .as_ref()
                .is_some_and(|id| self.ignore_class_methods.contains(&id.name.to_string()));
        // Istanbul cascades pragma and `ignoreClassMethods` skips into the body.
        let pragma_skip = has_pragma
            || self.skip_next
            || self.in_ignored_subtree()
            || ignored_named_function_expression;
        let fn_counter_only_skip = self.skip_fn_counter_only;
        self.skip_next = false;
        self.skip_fn_counter_only = false;
        self.ignored_fn_stack.push(pragma_skip);
        if pragma_skip {
            self.pending_name = None;
            return;
        }
        if fn_counter_only_skip {
            if func.body.is_some() {
                self.pending_fn_counters.push(None);
            }
            self.pending_name = None;
            return;
        }

        let name = self.resolve_function_name(func, ctx);
        // istanbul-lib-instrument points `decl` at the identifier itself:
        // `function foo()` gives the `foo` identifier span, a class method
        // `bar() {}` gives the key span (recorded by `register_method_definition`
        // before this hook runs), and an anonymous `function ()` gives a
        // one-character marker where the name would have been.
        let decl_span = if let Some(id) = &func.id {
            id.span
        } else if let Some(span) = self.pending_method_decl.take() {
            span
        } else {
            Span::new(func.span.start, func.span.start + 1)
        };
        if let Some(body) = &func.body {
            // Pushed for every function with a body so the stack stays balanced
            // with the body hook's pop; `None` emits no counter.
            let fn_id = self.add_function(name, decl_span, body.span);
            self.pending_fn_counters.push(fn_id);
        }
    }

    /// Prepend the function counter to a body, if the function was registered.
    pub(super) fn insert_function_counter(
        &mut self,
        body: &mut FunctionBody<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        if self.in_ignored_subtree() {
            return;
        }
        // Popped unconditionally to stay balanced with `register_function_entry`;
        // only a registered function (`Some`) gets a counter.
        if let Some(Some(fn_id)) = self.pending_fn_counters.pop() {
            let cov_fn = self.cov_fn_name;
            let counter = build_counter_stmt(CounterKind::func(cov_fn, fn_id), ctx);
            body.statements.insert(0, counter);
        }
    }

    /// Register the `fnMap` entry for an arrow, or record that its subtree is
    /// suppressed.
    pub(super) fn register_arrow_entry(
        &mut self,
        arrow: &ArrowFunctionExpression<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let pragma_skip = ctx.state.pragmas.get(arrow.span.start) == Some(IgnoreType::Next)
            || self.skip_next
            || self.in_ignored_subtree();
        // Only pragma-driven skips suppress body statements.
        self.ignored_fn_stack.push(pragma_skip);
        if pragma_skip {
            self.skip_next = false;
            self.pending_name = None;
            return;
        }

        let name = self
            .pending_name
            .take()
            .or_else(|| self.name_callback_arguments.then(|| callback_argument_name(ctx)).flatten())
            .unwrap_or_else(|| format!("(anonymous_{})", self.fn_map.len()));
        let fn_id = self.add_function(
            name,
            Span::new(arrow.span.start, arrow.span.start + 1),
            arrow.body.span,
        );

        // Mutating the body here would invalidate the scope ids traverse is
        // holding, so the body hook inserts the counter and
        // `convert_arrow_expression_body` converts an expression body to a
        // block. Pushed unconditionally to stay balanced with the body hook's
        // pop; `None` emits no counter.
        self.pending_fn_counters.push(fn_id);
    }

    /// Give an expression-bodied arrow a block body ending in a `return`, so
    /// the counter inserted into its body has a statement slot to live in.
    pub(super) fn convert_arrow_expression_body(
        &mut self,
        arrow: &mut ArrowFunctionExpression<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        // Converting the expression body to a block body has to happen after
        // the body has been traversed; doing it in the enter hook would
        // invalidate the scope ids traverse is holding.
        if arrow.expression && !arrow.body.statements.is_empty() {
            if let Some(Statement::ExpressionStatement(expr_stmt)) =
                arrow.body.statements.last_mut()
            {
                let dummy = dummy_expr(ctx);
                let expr = mem::replace(&mut expr_stmt.expression, dummy);
                let last_idx = arrow.body.statements.len() - 1;
                arrow.body.statements[last_idx] =
                    Statement::new_return_statement(SPAN, Some(expr), ctx);
            }
            arrow.expression = false;
        }
        self.ignored_fn_stack.pop();
    }

    /// Name an anonymous default export `"default"`, per istanbul convention.
    pub(super) fn name_anonymous_default_export(
        &mut self,
        decl: &ExportDefaultDeclaration<'arena>,
    ) {
        if self.in_ignored_subtree() {
            return;
        }
        // Named function exports (`export default function foo`) keep their
        // declared identifier. Class exports do not need handling here
        // because their constructor (if any) receives its name via
        // `register_method_definition`, and a class without a constructor
        // produces no `fnMap` entry at all.
        let anonymous = match &decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(func) => func.id.is_none(),
            ExportDefaultDeclarationKind::ArrowFunctionExpression(_) => true,
            _ => false,
        };
        if anonymous {
            self.pending_name = Some("default".to_string());
        }
    }

    /// Carry a method's name and `decl` span into its inner `Function`, or
    /// record that the method is suppressed.
    pub(super) fn register_method_definition(
        &mut self,
        method: &MethodDefinition<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let parent_ignored = self.in_ignored_subtree();
        let key_span = method.key.span();
        if method_ignored_by_pragma(
            MethodPragmaInput { method, key_span, skip_next: self.skip_next },
            ctx,
        ) {
            self.ignored_prop_stack.push(true);
            self.skip_next = false;
            return;
        }
        self.ignored_prop_stack.push(false);
        if parent_ignored {
            return;
        }
        if matches!(method.key, PropertyKey::PrivateIdentifier(_)) {
            // istanbul-lib-instrument instruments private method bodies for
            // statement coverage but does not surface them in `fnMap`.
            self.skip_fn_counter_only = true;
            return;
        }
        let Some(name) = property_key_to_name(&method.key) else { return };
        if self.ignore_class_methods.contains(&name) {
            if let Some(ignored) = self.ignored_prop_stack.last_mut() {
                *ignored = true;
            }
            return;
        }
        self.pending_name = Some(method_label(method.kind, name));
        // `decl` for a method is the key's span (`bar` in `class C { bar(x) {} }`),
        // the same rule istanbul applies to named function declarations.
        self.pending_method_decl = Some(key_span);
    }

    /// Attach the statement counter for a class property initializer, hoisting
    /// it to a sibling field when the initializer needs its inferred name.
    pub(super) fn instrument_property_definition(
        &mut self,
        prop: &mut PropertyDefinition<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let parent_ignored = self.in_ignored_subtree();
        let has_ignore_next =
            ctx.state.pragmas.get(prop.span.start) == Some(IgnoreType::Next) || self.skip_next;
        self.ignored_prop_stack.push(has_ignore_next);
        if has_ignore_next {
            self.skip_next = false;
            return;
        }
        if parent_ignored {
            return;
        }

        // Istanbul gives each class property initializer (`class Foo { x = expr }`)
        // a statement counter, but a `PropertyDefinition` is a class element
        // rather than a `Statement`, so the statement hook never sees it. The
        // initializer is wrapped in place: `x = (++cov.s[N], expr)`.
        let Some(value) = prop.value.as_mut() else { return };
        let span = value.span();
        if is_synthetic_span(span) {
            return;
        }

        let is_named_initializer = matches!(
            value,
            Expression::FunctionExpression(_)
                | Expression::ArrowFunctionExpression(_)
                | Expression::ClassExpression(_)
        );
        if is_named_initializer && !self.pending_class_field_hoists.is_empty() {
            // `class Foo { field = function () {} }`: the counter goes into a
            // synthetic sibling field so the initializer stays a bare
            // function/class expression and NamedEvaluation can still bind
            // `Function.name`.
            self.try_hoist_named_property_initializer(prop, span);
            return;
        }

        if let Some(stmt_id) = self.add_statement(span) {
            prepend_counter(value, CounterKind::stmt(self.cov_fn_name, stmt_id), ctx);
        }
    }

    /// Carry an object property's key name into its function-valued value, or
    /// record that the property is suppressed.
    pub(super) fn register_object_property(
        &mut self,
        prop: &ObjectProperty<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let is_method_like =
            prop.method || matches!(prop.kind, PropertyKind::Get | PropertyKind::Set);
        let key_has_ignore_next = ctx.state.pragmas.get(prop.span.start) == Some(IgnoreType::Next)
            || ctx.state.pragmas.get(prop.key.span().start) == Some(IgnoreType::Next)
            || self.skip_next;
        let is_function_valued = matches!(
            prop.value,
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
        );
        // On a method-like property the pragma suppresses the method, on a
        // plain data property it suppresses the value. A function-valued data
        // property is left to the inner function/arrow hook, which consumes
        // `skip_next` itself.
        let has_ignore_next = key_has_ignore_next && (is_method_like || !is_function_valued);
        self.push_prop_ignore_frame(has_ignore_next);

        // Carry the property's key name into the inner function or arrow so
        // `fnMap[N].name` reads as the source does instead of
        // `(anonymous_N)`. Covers shorthand methods, accessors and
        // function-valued properties.
        if !has_ignore_next && !self.in_ignored_subtree() {
            let inherits_name = is_method_like || is_function_valued;
            if inherits_name && let Some(base) = property_key_to_name(&prop.key) {
                let label = match prop.kind {
                    PropertyKind::Get => format!("get {base}"),
                    PropertyKind::Set => format!("set {base}"),
                    PropertyKind::Init => base,
                };
                self.pending_name = Some(label);
            }
        }
    }

    /// Emit the hoisted class-field counters as synthetic sibling fields.
    pub(super) fn insert_class_field_counters(
        &mut self,
        body: &mut ClassBody<'arena>,
        ctx: &TraverseCtx<'arena, CoverageState>,
    ) {
        let Some(hoists) = self.pending_class_field_hoists.pop() else { return };
        if hoists.is_empty() {
            return;
        }
        let cov_fn = self.cov_fn_name;
        // Each synthetic counter field is inserted immediately before the
        // `PropertyDefinition` it belongs to, so a static counter lands next to
        // a static field and an instance counter next to an instance field and
        // runtime evaluation order is unchanged.
        let mut by_target: BTreeMap<u32, &ClassFieldHoist> = BTreeMap::new();
        for hoist in &hoists {
            by_target.insert(hoist.target_start, hoist);
        }
        let original = mem::replace(&mut body.body, ArenaVec::new_in(ctx));
        for element in original {
            if let ClassElement::PropertyDefinition(prop) = &element
                && let Some(hoist) = by_target.get(&prop.span.start)
            {
                body.body.push(build_class_field_counter(cov_fn, hoist, ctx));
            }
            body.body.push(element);
        }
    }
}
