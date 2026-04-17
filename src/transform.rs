//! AST-level coverage transform using `oxc_traverse`.
//!
//! Replaces the source-level text injection approach with proper AST mutation.
//! The transform:
//! 1. Collects coverage span metadata (same as the old visitor)
//! 2. Injects counter expression statements (`cov_fn().s[N]++`) into the AST
//! 3. Converts arrow expression bodies to block bodies when needed
//! 4. Prepends the coverage initialization preamble to the program

use std::collections::BTreeMap;
use std::fmt::Write;
use std::mem;

use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN, Span};
use oxc_syntax::operator::{LogicalOperator, UpdateOperator};
use oxc_traverse::{Traverse, TraverseCtx};

use crate::pragma::{IgnoreType, PragmaMap};
use crate::types::{BranchEntry, FileCoverage, FnEntry, Location, Position};

/// State carried through the traverse for coverage instrumentation.
pub struct CoverageState {
    /// Pragma map for istanbul/v8 ignore directives.
    pub pragmas: PragmaMap,
}

/// Collects coverage metadata and injects counter expressions via AST mutation.
pub struct CoverageTransform<'src> {
    source: &'src str,
    line_offsets: Vec<u32>,
    fn_counter: usize,
    stmt_counter: usize,
    branch_counter: usize,
    pub fn_map: BTreeMap<String, FnEntry>,
    pub statement_map: BTreeMap<String, Location>,
    pub branch_map: BTreeMap<String, BranchEntry>,
    /// Name inherited from a parent node (variable declarator, method definition).
    pending_name: Option<String>,
    /// `decl` span inherited from a class `MethodDefinition`. A method's inner
    /// `Function` has no `id` of its own, so without this override
    /// `enter_function` would fall back to the anonymous one-char marker at
    /// the start of `function`. For methods that start with a parameter list
    /// (e.g. `bar(x) {}`), `func.span.start` points at `(` — which is not a
    /// meaningful `decl`. We carry the method key span down instead.
    pending_method_decl: Option<Span>,
    /// Accumulated statements to inject before specific statements.
    pending_stmts: Vec<PendingInsertion>,
    /// Stack of pending function entry counters. Supports nested functions/arrows
    /// where an inner function is entered before the outer's body is visited.
    pending_fn_counters: Vec<usize>,
    /// Per-frame record of whether the current function or arrow is being ignored
    /// (i.e. its subtree should not be instrumented). Mirrors Istanbul's `path.skip()`:
    /// when true at any ancestor frame, statements in the body are not counted.
    ignored_fn_stack: Vec<bool>,
    /// When true, skip instrumentation for the next node.
    skip_next: bool,
    /// When true, the next function/arrow should skip its own function counter
    /// but keep instrumenting its body. Used by the `ignoreClassMethods` option,
    /// which is a softer skip than `/* istanbul ignore next */`.
    skip_fn_counter_only: bool,
    /// True while traversing a `VariableDeclaration` carrying an `ignore next`
    /// pragma. Consumed by `enter_variable_declarator` to skip both the
    /// per-declarator statement counter and any inner function counter.
    skip_current_var_decl: bool,
    /// Coverage function name, cached to avoid cloning from state on every hook.
    cov_fn_name: String,
    /// When true, adds truthy-value tracking (`bT`) for logical expression operands.
    report_logic: bool,
    /// Class method names to exclude from function coverage.
    ignore_class_methods: Vec<String>,
    /// Branch IDs of logical expression branches (for building the `bT` map).
    pub logical_branch_ids: Vec<usize>,
}

struct PendingInsertion {
    /// The span.start of the target statement (used for matching).
    target_start: u32,
    /// Counter expression to inject before the target.
    counter_id: usize,
    counter_type: CounterType,
}

#[derive(Clone, Copy)]
enum CounterType {
    Statement,
    /// Left branch of a logical assignment (path index 0).
    BranchLeft,
}

impl<'src> CoverageTransform<'src> {
    pub fn new(
        source: &'src str,
        cov_fn_name: String,
        report_logic: bool,
        ignore_class_methods: Vec<String>,
    ) -> Self {
        let line_offsets: Vec<u32> = std::iter::once(0)
            .chain(
                source
                    .bytes()
                    .enumerate()
                    .filter(|(_, b)| *b == b'\n')
                    .map(|(i, _)| (i + 1) as u32),
            )
            .collect();

        Self {
            source,
            line_offsets,
            fn_counter: 0,
            stmt_counter: 0,
            branch_counter: 0,
            fn_map: BTreeMap::new(),
            statement_map: BTreeMap::new(),
            branch_map: BTreeMap::new(),
            pending_name: None,
            pending_method_decl: None,
            pending_stmts: Vec::new(),
            pending_fn_counters: Vec::new(),
            ignored_fn_stack: Vec::new(),
            skip_next: false,
            skip_fn_counter_only: false,
            skip_current_var_decl: false,
            cov_fn_name,
            report_logic,
            ignore_class_methods,
            logical_branch_ids: Vec::new(),
        }
    }

    fn span_to_location(&self, span: Span) -> Location {
        Location {
            start: self.offset_to_position(span.start),
            end: self.offset_to_position(span.end),
        }
    }

    fn offset_to_position(&self, offset: u32) -> Position {
        let line = self.line_offsets.partition_point(|&o| o <= offset).saturating_sub(1);
        let line_start = self.line_offsets[line] as usize;
        let end = (offset as usize).min(self.source.len());
        // Istanbul/Babel report columns as UTF-16 code units (JavaScript string indices),
        // not UTF-8 bytes. Convert by walking chars from line start to the offset.
        let column =
            self.source[line_start..end].chars().map(char::len_utf16).sum::<usize>() as u32;
        Position { line: (line + 1) as u32, column }
    }

    fn add_function(&mut self, name: String, decl_span: Span, body_span: Span) -> usize {
        let id_num = self.fn_counter;
        let id = id_num.to_string();
        self.fn_counter += 1;
        let line = self.offset_to_position(decl_span.start).line;
        self.fn_map.insert(
            id,
            FnEntry {
                name,
                line,
                decl: self.span_to_location(decl_span),
                loc: self.span_to_location(body_span),
            },
        );
        id_num
    }

    fn add_statement(&mut self, span: Span) -> usize {
        let id_num = self.stmt_counter;
        let id = id_num.to_string();
        self.stmt_counter += 1;
        self.statement_map.insert(id, self.span_to_location(span));
        id_num
    }

    fn add_branch(&mut self, branch_type: &str, span: Span, locations: &[Span]) -> usize {
        let id_num = self.branch_counter;
        let id = id_num.to_string();
        self.branch_counter += 1;
        let loc = self.span_to_location(span);
        let line = loc.start.line;
        self.branch_map.insert(
            id,
            BranchEntry {
                loc,
                line,
                branch_type: branch_type.to_string(),
                locations: locations.iter().map(|s| self.span_to_location(*s)).collect(),
            },
        );
        id_num
    }

    fn resolve_function_name(&mut self, func: &Function) -> String {
        if let Some(name) = self.pending_name.take() {
            return name;
        }
        if let Some(id) = &func.id {
            return id.name.to_string();
        }
        format!("(anonymous_{})", self.fn_counter)
    }
}

/// Allocate a string into the arena so it has lifetime `'a`.
fn alloc_str<'a>(s: &str, ctx: &TraverseCtx<'a, CoverageState>) -> &'a str {
    ctx.ast.allocator.alloc_str(s)
}

/// Build a counter expression: `cov_fn().type[id]++`
fn build_counter_expr<'a>(
    cov_fn_name: &str,
    counter_type: &str,
    counter_id: usize,
    ctx: &TraverseCtx<'a, CoverageState>,
) -> Expression<'a> {
    let name = alloc_str(cov_fn_name, ctx);
    let callee = ctx.ast.expression_identifier(SPAN, name);
    let call = ctx.ast.expression_call(
        SPAN,
        callee,
        None::<TSTypeParameterInstantiation>,
        ctx.ast.vec(),
        false,
    );

    let ct = alloc_str(counter_type, ctx);
    let member =
        ctx.ast.member_expression_static(SPAN, call, ctx.ast.identifier_name(SPAN, ct), false);
    let member_expr = Expression::from(member);

    let computed = ctx.ast.member_expression_computed(
        SPAN,
        member_expr,
        ctx.ast.expression_numeric_literal(
            SPAN,
            counter_id as f64,
            None,
            oxc_syntax::number::NumberBase::Decimal,
        ),
        false,
    );

    let target = SimpleAssignmentTarget::from(computed);
    ctx.ast.expression_update(SPAN, UpdateOperator::Increment, true, target)
}

/// Build a branch counter expression: `cov_fn().b[branch_id][path_idx]++`
fn build_branch_counter_expr<'a>(
    cov_fn_name: &str,
    branch_id: usize,
    path_idx: usize,
    ctx: &TraverseCtx<'a, CoverageState>,
) -> Expression<'a> {
    let name = alloc_str(cov_fn_name, ctx);
    let callee = ctx.ast.expression_identifier(SPAN, name);
    let call = ctx.ast.expression_call(
        SPAN,
        callee,
        None::<TSTypeParameterInstantiation>,
        ctx.ast.vec(),
        false,
    );

    let member =
        ctx.ast.member_expression_static(SPAN, call, ctx.ast.identifier_name(SPAN, "b"), false);
    let member_expr = Expression::from(member);

    let computed1 = ctx.ast.member_expression_computed(
        SPAN,
        member_expr,
        ctx.ast.expression_numeric_literal(
            SPAN,
            branch_id as f64,
            None,
            oxc_syntax::number::NumberBase::Decimal,
        ),
        false,
    );
    let computed1_expr = Expression::from(computed1);

    let computed2 = ctx.ast.member_expression_computed(
        SPAN,
        computed1_expr,
        ctx.ast.expression_numeric_literal(
            SPAN,
            path_idx as f64,
            None,
            oxc_syntax::number::NumberBase::Decimal,
        ),
        false,
    );

    let target = SimpleAssignmentTarget::from(computed2);
    ctx.ast.expression_update(SPAN, UpdateOperator::Increment, true, target)
}

/// Build a counter expression statement: `cov_fn().type[id]++;`
fn build_counter_stmt<'a>(
    cov_fn_name: &str,
    counter_type: &str,
    counter_id: usize,
    ctx: &TraverseCtx<'a, CoverageState>,
) -> Statement<'a> {
    let expr = build_counter_expr(cov_fn_name, counter_type, counter_id, ctx);
    ctx.ast.statement_expression(SPAN, expr)
}

/// Build a branch counter statement: `cov_fn().b[branch_id][path_idx]++;`
fn build_branch_counter_stmt<'a>(
    cov_fn_name: &str,
    branch_id: usize,
    path_idx: usize,
    ctx: &TraverseCtx<'a, CoverageState>,
) -> Statement<'a> {
    let expr = build_branch_counter_expr(cov_fn_name, branch_id, path_idx, ctx);
    ctx.ast.statement_expression(SPAN, expr)
}

/// Generate the preamble as source text.
///
/// Since building the IIFE via AST nodes is verbose and error-prone,
/// we generate the preamble as a source string and prepend it.
/// This matches the approach used by istanbul-lib-instrument.
pub fn generate_preamble_source(
    coverage: &FileCoverage,
    coverage_hash: &str,
    coverage_var: &str,
    cov_fn_name: &str,
    report_logic: bool,
) -> Result<String, serde_json::Error> {
    let estimated_size = 256
        + coverage.statement_map.len() * 80
        + coverage.fn_map.len() * 120
        + coverage.branch_map.len() * 120;
    let mut buf = String::with_capacity(estimated_size);
    let _ = write!(buf, "var {cov_fn_name} = (function () {{ var path = ");
    buf.push_str(&serde_json::to_string(&coverage.path)?);
    let _ = write!(buf, "; var hash = ");
    buf.push_str(&serde_json::to_string(coverage_hash)?);
    let _ = write!(buf, "; var gcv = '{coverage_var}'; var coverageData = ");
    buf.push_str(&serde_json::to_string(coverage)?);
    let _ = writeln!(
        buf,
        "; coverageData.hash = hash; var coverage = typeof globalThis !== 'undefined' ? globalThis : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : this; if (!coverage[gcv]) {{ coverage[gcv] = {{}}; }} if (!coverage[gcv][path] || coverage[gcv][path].hash !== hash) {{ coverage[gcv][path] = coverageData; }} var actualCoverage = coverage[gcv][path]; return actualCoverage; }});"
    );
    if report_logic {
        // Declare temp variable and truthy tracking helper function.
        // The helper captures the value, checks if it's a "non-trivial" truthy
        // value, and if so increments the bT counter. Returns the original value.
        //
        // Istanbul's non-trivial check:
        //   _temp && (!Array.isArray(_temp) || _temp.length)
        //         && (Object.getPrototypeOf(_temp) !== Object.prototype
        //             || Object.values(_temp).length)
        //
        // This means empty arrays [] and empty plain objects {} are NOT counted
        // as truthy. Non-plain objects (class instances, etc.) are always counted.
        let _ = writeln!(buf, "var {cov_fn_name}_temp;");
        let _ = writeln!(
            buf,
            "function {cov_fn_name}_bt(val, id, idx) {{ {cov_fn_name}_temp = val; if ({cov_fn_name}_temp && (!Array.isArray({cov_fn_name}_temp) || {cov_fn_name}_temp.length) && (Object.getPrototypeOf({cov_fn_name}_temp) !== Object.prototype || Object.values({cov_fn_name}_temp).length)) {{ ++{cov_fn_name}().bT[id][idx]; }} return {cov_fn_name}_temp; }}"
        );
    }
    Ok(buf)
}

/// Generate a deterministic coverage function name from the file path.
pub fn generate_cov_fn_name(file_path: &str) -> String {
    let mut hash: u64 = 0;
    for byte in file_path.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    format!("cov_{hash:x}")
}

/// Create a dummy expression for `mem::replace` operations.
fn dummy_expr<'a>(ctx: &TraverseCtx<'a, CoverageState>) -> Expression<'a> {
    ctx.ast.expression_numeric_literal(SPAN, 0.0, None, oxc_syntax::number::NumberBase::Decimal)
}

/// Check if the nearest non-parenthesized ancestor is a logical expression.
/// Oxc preserves `ParenthesizedExpression` nodes (Babel strips them), so to
/// match istanbul-lib-instrument's chain flattening we must look through
/// any wrapping parens when deciding if we are an inner logical operand.
fn is_parent_logical(ctx: &TraverseCtx<'_, CoverageState>) -> bool {
    use oxc_traverse::Ancestor;
    for a in ctx.ancestors() {
        match a {
            Ancestor::ParenthesizedExpressionExpression(_) => {}
            Ancestor::LogicalExpressionLeft(_) | Ancestor::LogicalExpressionRight(_) => {
                return true;
            }
            _ => return false,
        }
    }
    false
}

/// Collect all leaf operand spans from a chained logical expression.
/// For `a && b || c`, returns spans of [a, b, c]. Also flattens through
/// `ParenthesizedExpression` nodes so `a && (b || c)` is treated as one
/// three-leaf chain, matching istanbul-lib-instrument.
fn collect_logical_leaf_spans(expr: &LogicalExpression) -> Vec<Span> {
    let mut spans = Vec::new();
    collect_logical_leaves_inner(&expr.left, &mut spans);
    collect_logical_leaves_inner(&expr.right, &mut spans);
    spans
}

fn collect_logical_leaves_inner(expr: &Expression, spans: &mut Vec<Span>) {
    if let Expression::ParenthesizedExpression(paren) = expr {
        collect_logical_leaves_inner(&paren.expression, spans);
        return;
    }
    if let Expression::LogicalExpression(logical) = expr {
        collect_logical_leaves_inner(&logical.left, spans);
        collect_logical_leaves_inner(&logical.right, spans);
    } else {
        spans.push(expr.span());
    }
}

struct LogicalWrapState<'b> {
    cov_fn_name: &'b str,
    branch_id: usize,
    report_logic: bool,
    path_idx: usize,
}

impl<'b> LogicalWrapState<'b> {
    fn new(cov_fn_name: &'b str, branch_id: usize, report_logic: bool) -> Self {
        Self { cov_fn_name, branch_id, report_logic, path_idx: 0 }
    }

    fn current_path_idx(&self) -> usize {
        self.path_idx
    }

    fn advance_path(&mut self) {
        self.path_idx += 1;
    }
}

/// Wrap a single logical expression leaf with its branch counter.
/// Without report_logic: `(cov().b[id][pathIdx]++, operand)`
/// With report_logic: additionally wrapped with truthy tracking via a
/// preamble helper function.
fn wrap_expression_with_branch_counter<'a>(
    operand: &mut Expression<'a>,
    state: &LogicalWrapState<'_>,
    ctx: &TraverseCtx<'a, CoverageState>,
) {
    let counter = build_branch_counter_expr(
        state.cov_fn_name,
        state.branch_id,
        state.current_path_idx(),
        ctx,
    );
    let orig = mem::replace(operand, dummy_expr(ctx));
    let mut items = ctx.ast.vec();
    items.push(counter);
    items.push(orig);
    *operand = ctx.ast.expression_sequence(SPAN, items);
}

fn wrap_logical_leaf<'a>(
    operand: &mut Expression<'a>,
    state: &mut LogicalWrapState<'_>,
    ctx: &TraverseCtx<'a, CoverageState>,
) {
    wrap_expression_with_branch_counter(operand, state, ctx);
    let branch_wrapped = mem::replace(operand, dummy_expr(ctx));

    if state.report_logic {
        // Wrap with truthy tracking helper: cov_fn_bt(wrapped, branch_id, path_idx)
        let bt_name = alloc_str(&format!("{}_bt", state.cov_fn_name), ctx);
        let callee = ctx.ast.expression_identifier(SPAN, bt_name);
        let mut args = ctx.ast.vec();
        args.push(Argument::from(branch_wrapped));
        args.push(Argument::from(ctx.ast.expression_numeric_literal(
            SPAN,
            state.branch_id as f64,
            None,
            oxc_syntax::number::NumberBase::Decimal,
        )));
        args.push(Argument::from(ctx.ast.expression_numeric_literal(
            SPAN,
            state.current_path_idx() as f64,
            None,
            oxc_syntax::number::NumberBase::Decimal,
        )));
        *operand = ctx.ast.expression_call(
            SPAN,
            callee,
            None::<TSTypeParameterInstantiation>,
            args,
            false,
        );
    } else {
        *operand = branch_wrapped;
    }
    state.advance_path();
}

/// Recursively wrap each leaf operand in a chained logical expression with
/// its branch counter: `(cov().b[id][pathIdx]++, operand)`. Looks through
/// `ParenthesizedExpression` so `a && (b || c)` wraps all three leaves.
fn wrap_logical_leaves<'a>(
    expr: &mut LogicalExpression<'a>,
    state: &mut LogicalWrapState<'_>,
    ctx: &mut TraverseCtx<'a, CoverageState>,
) {
    wrap_logical_operand(&mut expr.left, state, ctx);
    wrap_logical_operand(&mut expr.right, state, ctx);
}

fn wrap_logical_operand<'a>(
    operand: &mut Expression<'a>,
    state: &mut LogicalWrapState<'_>,
    ctx: &mut TraverseCtx<'a, CoverageState>,
) {
    // Unwrap parens transparently (matches Babel's AST shape).
    if let Expression::ParenthesizedExpression(paren) = operand {
        return wrap_logical_operand(&mut paren.expression, state, ctx);
    }
    if let Expression::LogicalExpression(inner) = operand {
        wrap_logical_leaves(inner, state, ctx);
    } else {
        wrap_logical_leaf(operand, state, ctx);
    }
}

impl<'a> Traverse<'a, CoverageState> for CoverageTransform<'_> {
    fn enter_function(
        &mut self,
        func: &mut Function<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let has_pragma = ctx.state.pragmas.get(func.span.start) == Some(IgnoreType::Next);
        // Pragma-driven skip cascades into the body (Istanbul subtree semantics).
        let pragma_skip = has_pragma || self.skip_next;
        // `ignoreClassMethods` is a softer skip: drop the fn counter but keep body.
        let fn_counter_only_skip = self.skip_fn_counter_only;
        self.skip_next = false;
        self.skip_fn_counter_only = false;
        // Only pragma-driven skips suppress body statements.
        self.ignored_fn_stack.push(pragma_skip);
        if pragma_skip || fn_counter_only_skip {
            self.pending_name = None;
            return;
        }

        let name = self.resolve_function_name(func);
        // `decl` should point at the identifier itself, matching istanbul-lib-instrument:
        //   `function foo(…)`               → decl is the `foo` identifier span
        //   class methods `bar(…) {…}`      → decl is the method key span (set by
        //                                      `enter_method_definition` before we get here)
        //   `function(…)` (anonymous)       → decl is a zero-ish-width marker at the start of
        //                                      `function`, which is where the name would go
        let decl_span = if let Some(id) = &func.id {
            id.span
        } else if let Some(span) = self.pending_method_decl.take() {
            span
        } else {
            // Anonymous: one-character span at the start of the `function` keyword.
            // Matches istanbul's output for `const f = function(…) {…}` (decl = col 10–11).
            Span::new(func.span.start, func.span.start + 1)
        };
        if let Some(body) = &func.body {
            let fn_id = self.add_function(name, decl_span, body.span);
            self.pending_fn_counters.push(fn_id);
        }
    }

    fn exit_function(
        &mut self,
        _func: &mut Function<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        self.ignored_fn_stack.pop();
    }

    fn enter_function_body(
        &mut self,
        body: &mut FunctionBody<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        if let Some(fn_id) = self.pending_fn_counters.pop() {
            let cov_fn = self.cov_fn_name.as_str();
            let counter = build_counter_stmt(cov_fn, "f", fn_id, ctx);
            body.statements.insert(0, counter);
        }
    }

    fn enter_arrow_function_expression(
        &mut self,
        arrow: &mut ArrowFunctionExpression<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let pragma_skip =
            ctx.state.pragmas.get(arrow.span.start) == Some(IgnoreType::Next) || self.skip_next;
        // Only pragma-driven skips suppress body statements.
        self.ignored_fn_stack.push(pragma_skip);
        if pragma_skip {
            self.skip_next = false;
            self.pending_name = None;
            return;
        }

        let name =
            self.pending_name.take().unwrap_or_else(|| format!("(anonymous_{})", self.fn_counter));
        let fn_id = self.add_function(
            name,
            Span::new(arrow.span.start, arrow.span.start + 1),
            arrow.body.span,
        );

        // DON'T modify body here — it breaks scope tracking in the traverse.
        // Set pending_fn_counter for enter_function_body to insert the counter.
        // For expression-bodied arrows, exit_arrow_function_expression converts
        // the body to a block with return after traversal completes.
        self.pending_fn_counters.push(fn_id);
    }

    fn exit_arrow_function_expression(
        &mut self,
        arrow: &mut ArrowFunctionExpression<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // For expression-bodied arrows, if a counter was supposed to be inserted
        // but wasn't (because enter_function_body inserts into block bodies only),
        // we need to handle it here. However, enter_function_body SHOULD be called
        // for arrow bodies too. If the counter was already inserted, pending_fn_counter
        // will be None. Only need special handling if it wasn't inserted.
        // Actually, enter_function_body handles both block and expression bodies
        // by inserting at index 0 of the statements vec, which works even for
        // expression bodies (they have one ExpressionStatement).
        // The conversion to block body with return happens here, AFTER traversal
        // of the body is complete.
        if arrow.expression && !arrow.body.statements.is_empty() {
            // Convert expression body to block body: change ExpressionStatement to ReturnStatement
            if let Some(Statement::ExpressionStatement(expr_stmt)) =
                arrow.body.statements.last_mut()
            {
                let dummy = dummy_expr(ctx);
                let expr = mem::replace(&mut expr_stmt.expression, dummy);
                let last_idx = arrow.body.statements.len() - 1;
                arrow.body.statements[last_idx] = ctx.ast.statement_return(SPAN, Some(expr));
            }
            arrow.expression = false;
        }
        self.ignored_fn_stack.pop();
    }

    fn enter_variable_declaration(
        &mut self,
        decl: &mut VariableDeclaration<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Honor `/* istanbul ignore next */` attached to this declaration.
        // `enter_statement` used to handle this for us, but variable declarations
        // are now treated as containers (per-declarator counters), so pragmas
        // must be consulted here instead.
        if ctx.state.pragmas.get(decl.span.start) == Some(IgnoreType::Next) {
            self.skip_current_var_decl = true;
        }
    }

    fn exit_variable_declaration(
        &mut self,
        _decl: &mut VariableDeclaration<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        self.skip_current_var_decl = false;
    }

    fn enter_variable_declarator(
        &mut self,
        decl: &mut VariableDeclarator<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // If the enclosing declaration is ignored, skip both the statement
        // counter wrap and any inner function counter. Set `skip_next` so the
        // inner arrow/function hook consumes it.
        if self.skip_current_var_decl {
            if matches!(
                decl.init,
                Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
            ) {
                self.skip_next = true;
            }
            return;
        }

        // Set inherited name for function/arrow init so coverFunction can use it.
        if let Some(id) = decl.id.get_binding_identifier()
            && decl.init.as_ref().is_some_and(|init| {
                matches!(
                    init,
                    Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                )
            })
        {
            self.pending_name = Some(id.name.to_string());
        }

        // Per-declarator statement counter: wrap the init with (++cov().s[N], init).
        // Mirrors istanbul-lib-instrument's coverVariableDeclarator, which calls
        // insertStatementCounter on path.get('init'). Declarators without an init
        // (`let x;`) produce no statement counter.
        let Some(init) = decl.init.as_mut() else { return };
        // Skip if inside an ignored function/arrow body.
        if self.ignored_fn_stack.iter().any(|&ignored| ignored) {
            return;
        }
        let init_span = init.span();
        if init_span.start == 0 && init_span.end == 0 {
            return;
        }
        let stmt_id = self.add_statement(init_span);
        let cov_fn = self.cov_fn_name.as_str();
        let counter = build_counter_expr(cov_fn, "s", stmt_id, ctx);
        let orig = mem::replace(init, dummy_expr(ctx));
        let mut items = ctx.ast.vec();
        items.push(counter);
        items.push(orig);
        *init = ctx.ast.expression_sequence(SPAN, items);
    }

    fn exit_variable_declarator(
        &mut self,
        _decl: &mut VariableDeclarator<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        self.pending_name = None;
    }

    fn enter_method_definition(
        &mut self,
        method: &mut MethodDefinition<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let (name, key_span) = match &method.key {
            PropertyKey::StaticIdentifier(id) => (id.name.to_string(), id.span),
            PropertyKey::StringLiteral(s) => (s.value.to_string(), s.span),
            _ => return,
        };
        if self.ignore_class_methods.contains(&name) {
            // Softer skip: drop the fn counter but keep body statement counters.
            self.skip_fn_counter_only = true;
            return;
        }
        self.pending_name = Some(name);
        // `decl` for a method is the method key's span (e.g. `bar` in
        // `class C { bar(x) {} }`). Matches the rule we apply for named
        // function declarations — see `fn_decl_span_matches_istanbul`.
        self.pending_method_decl = Some(key_span);
    }

    fn exit_method_definition(
        &mut self,
        _method: &mut MethodDefinition<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        self.pending_name = None;
        self.pending_method_decl = None;
    }

    fn enter_property_definition(
        &mut self,
        prop: &mut PropertyDefinition<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Class property initializers: class Foo { x = expr; #y = expr; }
        // Istanbul creates a statement counter for each initializer expression.
        // Since PropertyDefinition is a class element (not a Statement), enter_statement
        // won't catch it. We wrap the initializer: x = (++cov().s[N], expr).
        let Some(value) = &prop.value else { return };
        let span = value.span();
        if span.start == 0 && span.end == 0 {
            return;
        }
        if ctx.state.pragmas.get(prop.span.start) == Some(IgnoreType::Next) || self.skip_next {
            self.skip_next = false;
            return;
        }
        if self.ignored_fn_stack.iter().any(|&ignored| ignored) {
            return;
        }
        let stmt_id = self.add_statement(span);
        let cov_fn = self.cov_fn_name.as_str();
        let counter = build_counter_expr(cov_fn, "s", stmt_id, ctx);
        let orig = mem::replace(prop.value.as_mut().unwrap(), dummy_expr(ctx));
        let mut items = ctx.ast.vec();
        items.push(counter);
        items.push(orig);
        *prop.value.as_mut().unwrap() = ctx.ast.expression_sequence(SPAN, items);
    }

    fn enter_statement(
        &mut self,
        stmt: &mut Statement<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let span = stmt.span();
        // Injected nodes have SPAN = 0:0 — never treat them as real statements.
        if span.start == 0 && span.end == 0 {
            return;
        }
        // istanbul-lib-instrument treats these variants as containers, not statements:
        //   FunctionDeclaration / ClassDeclaration  — covered via function counters
        //   VariableDeclaration                     — covered per-declarator (see
        //                                             enter_variable_declarator)
        //   Import / Export* / TS type-only decls   — skipped entirely
        //   BlockStatement / EmptyStatement         — never counted
        // See istanbul-lib-instrument's visitor.js wiring.
        if matches!(
            stmt,
            Statement::BlockStatement(_)
                | Statement::EmptyStatement(_)
                | Statement::FunctionDeclaration(_)
                | Statement::ClassDeclaration(_)
                | Statement::VariableDeclaration(_)
                | Statement::ImportDeclaration(_)
                | Statement::ExportNamedDeclaration(_)
                | Statement::ExportDefaultDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::TSTypeAliasDeclaration(_)
                | Statement::TSInterfaceDeclaration(_)
                | Statement::TSEnumDeclaration(_)
                | Statement::TSModuleDeclaration(_)
                | Statement::TSImportEqualsDeclaration(_)
                | Statement::TSExportAssignment(_)
                | Statement::TSNamespaceExportDeclaration(_)
        ) {
            return;
        }
        // If any enclosing function or arrow is ignored, skip its body statements
        // too. This matches Istanbul's subtree-skip semantics for
        // `/* istanbul ignore next */` on the enclosing callable.
        if self.ignored_fn_stack.iter().any(|&ignored| ignored) {
            return;
        }
        // Check for ignore next pragma on this statement.
        // Setting `skip_next` lets nested functions/arrows in the subtree skip
        // their own counters. It must NOT leak to the next sibling statement —
        // `exit_statement` clears it defensively.
        if ctx.state.pragmas.get(span.start) == Some(IgnoreType::Next) {
            self.skip_next = true;
            return;
        }
        if self.skip_next {
            self.skip_next = false;
            return;
        }
        let stmt_id = self.add_statement(span);
        self.pending_stmts.push(PendingInsertion {
            target_start: span.start,
            counter_id: stmt_id,
            counter_type: CounterType::Statement,
        });
    }

    fn exit_statement(
        &mut self,
        _stmt: &mut Statement<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Ensure `skip_next` cannot leak from an ignored statement to its next
        // sibling. Nested enter hooks consume it when they fire; if no such hook
        // fires (e.g. `/* istanbul ignore next */ return 1;`), this clears it.
        self.skip_next = false;
    }

    fn exit_statements(
        &mut self,
        stmts: &mut ArenaVec<'a, Statement<'a>>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        if self.pending_stmts.is_empty() {
            return;
        }

        let cov_fn = self.cov_fn_name.as_str();
        let mut insertions: Vec<(usize, Statement<'a>)> = Vec::new();
        let pending = &mut self.pending_stmts;

        for (idx, stmt) in stmts.iter().enumerate() {
            if pending.is_empty() {
                break;
            }
            let span = stmt.span();
            // Skip injected nodes (SPAN = 0:0) to prevent offset-0 collision
            if span.start == 0 && span.end == 0 {
                continue;
            }
            let start = span.start;
            let mut i = 0;
            while i < pending.len() {
                if pending[i].target_start == start {
                    let p = pending.swap_remove(i);
                    let counter = match p.counter_type {
                        CounterType::Statement => {
                            build_counter_stmt(cov_fn, "s", p.counter_id, ctx)
                        }
                        CounterType::BranchLeft => {
                            build_branch_counter_stmt(cov_fn, p.counter_id, 0, ctx)
                        }
                    };
                    insertions.push((idx, counter));
                } else {
                    i += 1;
                }
            }
        }

        if insertions.is_empty() {
            return;
        }

        insertions.sort_by(|a, b| b.0.cmp(&a.0));
        for (idx, counter) in insertions {
            stmts.insert(idx, counter);
        }
    }

    fn enter_if_statement(
        &mut self,
        stmt: &mut IfStatement<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let pragma = ctx.state.pragmas.get(stmt.span.start);

        // istanbul-lib-instrument's `coverIfBranches` passes `n.loc` (the whole
        // `IfStatement` span) as the consequent location, not the consequent
        // block's narrower span. See istanbul-lib-instrument/src/visitor.js
        // insertBranchCounter(path.get('consequent'), branch, n.loc). Match it
        // so downstream reporters (html-reporter, sonar) highlight the same
        // range in hover tooltips.
        let consequent_span = stmt.span;
        let alternate_span = stmt
            .alternate
            .as_ref()
            .map_or_else(|| Span::new(stmt.span.end, stmt.span.end), |alt| alt.span());
        let branch_id = self.add_branch("if", stmt.span, &[consequent_span, alternate_span]);

        let cov_fn = self.cov_fn_name.as_str();

        // istanbul ignore if: skip the if-branch counter
        if pragma != Some(IgnoreType::If) {
            inject_branch_counter_into_statement(&mut stmt.consequent, cov_fn, branch_id, 0, ctx);
        }

        // istanbul ignore else: skip the else-branch counter
        if let Some(alt) = &mut stmt.alternate
            && pragma != Some(IgnoreType::Else)
        {
            inject_branch_counter_into_statement(alt, cov_fn, branch_id, 1, ctx);
        }
    }

    fn enter_conditional_expression(
        &mut self,
        expr: &mut ConditionalExpression<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let branch_id = self.add_branch(
            "cond-expr",
            expr.span,
            &[expr.consequent.span(), expr.alternate.span()],
        );

        let cov_fn = self.cov_fn_name.as_str();

        // Wrap consequent: (cov().b[id][0]++, originalExpr)
        let counter0 = build_branch_counter_expr(cov_fn, branch_id, 0, ctx);
        let orig_consequent = mem::replace(&mut expr.consequent, dummy_expr(ctx));
        let mut items = ctx.ast.vec();
        items.push(counter0);
        items.push(orig_consequent);
        expr.consequent = ctx.ast.expression_sequence(SPAN, items);

        // Wrap alternate: (cov().b[id][1]++, originalExpr)
        let counter1 = build_branch_counter_expr(cov_fn, branch_id, 1, ctx);
        let orig_alternate = mem::replace(&mut expr.alternate, dummy_expr(ctx));
        let mut items = ctx.ast.vec();
        items.push(counter1);
        items.push(orig_alternate);
        expr.alternate = ctx.ast.expression_sequence(SPAN, items);
    }

    fn enter_switch_statement(
        &mut self,
        stmt: &mut SwitchStatement<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let case_spans: Vec<Span> = stmt.cases.iter().map(|c| c.span).collect();
        let branch_id = self.add_branch("switch", stmt.span, &case_spans);

        let cov_fn = self.cov_fn_name.as_str();
        for (path_idx, case) in stmt.cases.iter_mut().enumerate() {
            let branch_stmt = build_branch_counter_stmt(cov_fn, branch_id, path_idx, ctx);
            case.consequent.insert(0, branch_stmt);
        }
    }

    fn enter_logical_expression(
        &mut self,
        expr: &mut LogicalExpression<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        match expr.operator {
            LogicalOperator::And | LogicalOperator::Or | LogicalOperator::Coalesce => {
                // Check if parent is also a logical expression — if so, skip.
                // Istanbul flattens chained logical expressions into a single branch
                // with N locations (one per leaf operand). Only the outermost creates
                // the branch entry.
                if is_parent_logical(ctx) {
                    return;
                }

                // Collect all leaf operand spans by flattening the chain
                let leaf_spans = collect_logical_leaf_spans(expr);
                let branch_id = self.add_branch("binary-expr", expr.span, &leaf_spans);

                if self.report_logic {
                    self.logical_branch_ids.push(branch_id);
                }

                // Wrap each leaf operand with its branch counter
                let cov_fn = self.cov_fn_name.as_str();
                let mut state = LogicalWrapState::new(cov_fn, branch_id, self.report_logic);
                wrap_logical_leaves(expr, &mut state, ctx);
            }
        }
    }

    // Note: Istanbul does NOT instrument for/while/do-while loops as branches.
    // Loop coverage is tracked purely via statement counters on the body.

    fn enter_formal_parameter(
        &mut self,
        param: &mut FormalParameter<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Default parameter values: function f(x = 1) { }
        // Istanbul creates a 'default-arg' branch with 1 location for the default expression.
        if let Some(init) = &mut param.initializer {
            let init_span = init.span();
            let branch_id = self.add_branch("default-arg", param.span, &[init_span]);
            let cov_fn = self.cov_fn_name.as_str();
            let state = LogicalWrapState::new(cov_fn, branch_id, false);
            wrap_expression_with_branch_counter(init, &state, ctx);
        }
    }

    fn enter_assignment_pattern(
        &mut self,
        pattern: &mut AssignmentPattern<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Destructuring defaults: const { x = 1 } = obj;
        // Istanbul also creates 'default-arg' for these.
        let right_span = pattern.right.span();
        let branch_id = self.add_branch("default-arg", pattern.span, &[right_span]);
        let cov_fn = self.cov_fn_name.as_str();
        let state = LogicalWrapState::new(cov_fn, branch_id, false);
        wrap_expression_with_branch_counter(&mut pattern.right, &state, ctx);
    }

    fn enter_assignment_expression(
        &mut self,
        expr: &mut AssignmentExpression<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        use oxc_syntax::operator::AssignmentOperator;

        // Logical assignment operators: x ??= y, x ||= y, x &&= y
        // These short-circuit and only assign if the condition holds.
        // Track them as binary-expr branches with 2 locations (left, right).
        if matches!(
            expr.operator,
            AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalNullish
        ) {
            let left_span = expr.left.span();
            let right_span = expr.right.span();
            let branch_id = self.add_branch("binary-expr", expr.span, &[left_span, right_span]);

            let cov_fn = self.cov_fn_name.as_str();

            // The left branch (no assignment) is always entered — increment before
            // the assignment. The right branch (assignment happens) is conditional.
            // We insert the left counter as a pending statement before this expression,
            // and wrap the right side with the right counter.
            self.pending_stmts.push(PendingInsertion {
                target_start: expr.span.start,
                counter_id: branch_id,
                counter_type: CounterType::BranchLeft,
            });

            // Wrap the right side: x ??= (++cov().b[id][1], y)
            let counter = build_branch_counter_expr(cov_fn, branch_id, 1, ctx);
            let orig_right = mem::replace(&mut expr.right, dummy_expr(ctx));
            let mut items = ctx.ast.vec();
            items.push(counter);
            items.push(orig_right);
            expr.right = ctx.ast.expression_sequence(SPAN, items);
        }
    }
}

/// Inject a branch counter into a statement, wrapping in a block if necessary.
fn inject_branch_counter_into_statement<'a>(
    stmt: &mut Statement<'a>,
    cov_fn_name: &str,
    branch_id: usize,
    path_idx: usize,
    ctx: &mut TraverseCtx<'a, CoverageState>,
) {
    let counter_stmt = build_branch_counter_stmt(cov_fn_name, branch_id, path_idx, ctx);

    match stmt {
        Statement::BlockStatement(block) => {
            block.body.insert(0, counter_stmt);
        }
        _ => {
            // Replace statement with dummy, then build block with counter + original.
            // Must create a scope for the new block to avoid traverse panics.
            let scope_id =
                ctx.create_child_scope_of_current(oxc_syntax::scope::ScopeFlags::empty());
            let original = mem::replace(stmt, ctx.ast.statement_empty(SPAN));
            let mut stmts = ctx.ast.vec();
            stmts.push(counter_stmt);
            stmts.push(original);
            *stmt = ctx.ast.statement_block_with_scope_id(SPAN, stmts, scope_id);
        }
    }
}
