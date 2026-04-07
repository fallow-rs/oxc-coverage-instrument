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
pub struct CoverageTransform {
    line_offsets: Vec<u32>,
    fn_counter: usize,
    stmt_counter: usize,
    branch_counter: usize,
    pub fn_map: BTreeMap<String, FnEntry>,
    pub statement_map: BTreeMap<String, Location>,
    pub branch_map: BTreeMap<String, BranchEntry>,
    /// Name inherited from a parent node (variable declarator, method definition).
    pending_name: Option<String>,
    /// Accumulated statements to inject before specific statements.
    pending_stmts: Vec<PendingInsertion>,
    /// Stack of pending function entry counters. Supports nested functions/arrows
    /// where an inner function is entered before the outer's body is visited.
    pending_fn_counters: Vec<usize>,
    /// When true, skip instrumentation for the next node.
    skip_next: bool,
    /// Coverage function name, cached to avoid cloning from state on every hook.
    cov_fn_name: String,
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

impl CoverageTransform {
    pub fn new(source: &str, cov_fn_name: String) -> Self {
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
            line_offsets,
            fn_counter: 0,
            stmt_counter: 0,
            branch_counter: 0,
            fn_map: BTreeMap::new(),
            statement_map: BTreeMap::new(),
            branch_map: BTreeMap::new(),
            pending_name: None,
            pending_stmts: Vec::new(),
            pending_fn_counters: Vec::new(),
            skip_next: false,
            cov_fn_name,
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
        let col = offset - self.line_offsets[line];
        Position { line: (line + 1) as u32, column: col }
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
    coverage_var: &str,
    cov_fn_name: &str,
) -> Result<String, serde_json::Error> {
    let estimated_size = 256
        + coverage.statement_map.len() * 80
        + coverage.fn_map.len() * 120
        + coverage.branch_map.len() * 120;
    let mut buf = String::with_capacity(estimated_size);
    let _ = write!(buf, "var {cov_fn_name} = (function () {{ var path = ");
    buf.push_str(&serde_json::to_string(&coverage.path)?);
    let _ = write!(buf, "; var gcv = '{coverage_var}'; var coverageData = ");
    buf.push_str(&serde_json::to_string(coverage)?);
    let _ = writeln!(
        buf,
        "; var coverage = typeof globalThis !== 'undefined' ? globalThis : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : this; if (!coverage[gcv]) {{ coverage[gcv] = {{}}; }} if (!coverage[gcv][path]) {{ coverage[gcv][path] = coverageData; }} var actualCoverage = coverage[gcv][path]; return actualCoverage; }});"
    );
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

/// Check if the parent of the current node is a logical expression.
/// Used to detect chained logical expressions (e.g., `a && b || c`).
fn is_parent_logical(ctx: &TraverseCtx<'_, CoverageState>) -> bool {
    use oxc_traverse::Ancestor;
    matches!(ctx.parent(), Ancestor::LogicalExpressionLeft(_) | Ancestor::LogicalExpressionRight(_))
}

/// Collect all leaf operand spans from a chained logical expression.
/// For `a && b || c`, returns spans of [a, b, c].
fn collect_logical_leaf_spans(expr: &LogicalExpression) -> Vec<Span> {
    let mut spans = Vec::new();
    collect_logical_leaves_inner(&expr.left, &mut spans);
    collect_logical_leaves_inner(&expr.right, &mut spans);
    spans
}

fn collect_logical_leaves_inner(expr: &Expression, spans: &mut Vec<Span>) {
    if let Expression::LogicalExpression(logical) = expr {
        collect_logical_leaves_inner(&logical.left, spans);
        collect_logical_leaves_inner(&logical.right, spans);
    } else {
        spans.push(expr.span());
    }
}

/// Recursively wrap each leaf operand in a chained logical expression with
/// its branch counter: `(cov().b[id][pathIdx]++, operand)`.
fn wrap_logical_leaves<'a>(
    expr: &mut LogicalExpression<'a>,
    cov_fn_name: &str,
    branch_id: usize,
    path_idx: &mut usize,
    ctx: &mut TraverseCtx<'a, CoverageState>,
) {
    // Process left side
    if let Expression::LogicalExpression(inner) = &mut expr.left {
        wrap_logical_leaves(inner, cov_fn_name, branch_id, path_idx, ctx);
    } else {
        let counter = build_branch_counter_expr(cov_fn_name, branch_id, *path_idx, ctx);
        let orig = mem::replace(&mut expr.left, dummy_expr(ctx));
        let mut items = ctx.ast.vec();
        items.push(counter);
        items.push(orig);
        expr.left = ctx.ast.expression_sequence(SPAN, items);
        *path_idx += 1;
    }

    // Process right side
    if let Expression::LogicalExpression(inner) = &mut expr.right {
        wrap_logical_leaves(inner, cov_fn_name, branch_id, path_idx, ctx);
    } else {
        let counter = build_branch_counter_expr(cov_fn_name, branch_id, *path_idx, ctx);
        let orig = mem::replace(&mut expr.right, dummy_expr(ctx));
        let mut items = ctx.ast.vec();
        items.push(counter);
        items.push(orig);
        expr.right = ctx.ast.expression_sequence(SPAN, items);
        *path_idx += 1;
    }
}

impl<'a> Traverse<'a, CoverageState> for CoverageTransform {
    fn enter_function(
        &mut self,
        func: &mut Function<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Check for ignore next pragma
        if ctx.state.pragmas.get(func.span.start) == Some(IgnoreType::Next) {
            self.skip_next = true;
            self.pending_name = None;
            return;
        }
        if self.skip_next {
            self.skip_next = false;
            self.pending_name = None;
            return;
        }

        let name = self.resolve_function_name(func);
        let decl_span = if let Some(id) = &func.id {
            Span::new(func.span.start, id.span.end)
        } else {
            // Anonymous function: span from keyword to body start.
            // Works for both "function() {" and "async function() {".
            let end = func.body.as_ref().map_or(func.span.start, |b| b.span.start);
            Span::new(func.span.start, end)
        };
        if let Some(body) = &func.body {
            let fn_id = self.add_function(name, decl_span, body.span);
            self.pending_fn_counters.push(fn_id);
        }
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
        if ctx.state.pragmas.get(arrow.span.start) == Some(IgnoreType::Next) || self.skip_next {
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
    }

    fn enter_variable_declarator(
        &mut self,
        decl: &mut VariableDeclarator<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
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
        if let PropertyKey::StaticIdentifier(id) = &method.key {
            self.pending_name = Some(id.name.to_string());
        }
    }

    fn exit_method_definition(
        &mut self,
        _method: &mut MethodDefinition<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        self.pending_name = None;
    }

    fn enter_statement(
        &mut self,
        stmt: &mut Statement<'a>,
        ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        let span = stmt.span();
        // Skip blocks, empty statements, and injected nodes (which have SPAN = 0:0)
        if matches!(stmt, Statement::BlockStatement(_) | Statement::EmptyStatement(_))
            || (span.start == 0 && span.end == 0)
        {
            return;
        }
        // Check for ignore next pragma
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

        let consequent_span = stmt.consequent.span();
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

                // Wrap each leaf operand with its branch counter
                let cov_fn = self.cov_fn_name.as_str();
                wrap_logical_leaves(expr, cov_fn, branch_id, &mut 0, ctx);
            }
        }
    }

    // Note: Istanbul does NOT instrument for/while/do-while loops as branches.
    // Loop coverage is tracked purely via statement counters on the body.

    fn enter_formal_parameter(
        &mut self,
        param: &mut FormalParameter<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Default parameter values: function f(x = 1) { }
        // Istanbul creates a 'default-arg' branch with 1 location for the default expression.
        if let Some(init) = &param.initializer {
            let init_span = init.span();
            self.add_branch("default-arg", param.span, &[init_span]);
        }
    }

    fn enter_assignment_pattern(
        &mut self,
        pattern: &mut AssignmentPattern<'a>,
        _ctx: &mut TraverseCtx<'a, CoverageState>,
    ) {
        // Destructuring defaults: const { x = 1 } = obj;
        // Istanbul also creates 'default-arg' for these.
        let right_span = pattern.right.span();
        self.add_branch("default-arg", pattern.span, &[right_span]);
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
