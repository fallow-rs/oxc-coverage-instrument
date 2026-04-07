//! AST visitor that collects coverage spans from parsed JavaScript/TypeScript.
//!
//! Walks the Oxc AST to identify statements, functions, and branches,
//! building an Istanbul-compatible coverage map. Function names use the
//! same resolution logic as other Oxc-based tools (same parser = same names).

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use std::collections::BTreeMap;

use crate::types::{BranchEntry, FnEntry, Location, Position};

/// Collects statement, function, and branch locations from an Oxc AST.
pub(crate) struct CoverageVisitor {
    line_offsets: Vec<u32>,
    fn_counter: usize,
    stmt_counter: usize,
    branch_counter: usize,
    pub fn_map: BTreeMap<String, FnEntry>,
    pub statement_map: BTreeMap<String, Location>,
    pub branch_map: BTreeMap<String, BranchEntry>,
    /// Name inherited from a parent node (variable declarator, method definition).
    pending_name: Option<String>,
}

impl CoverageVisitor {
    pub fn new(source: &str) -> Self {
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
        }
    }

    fn span_to_location(&self, span: Span) -> Location {
        Location {
            start: self.offset_to_position(span.start),
            end: self.offset_to_position(span.end),
        }
    }

    fn offset_to_position(&self, offset: u32) -> Position {
        let line = self
            .line_offsets
            .partition_point(|&o| o <= offset)
            .saturating_sub(1);
        let col = offset - self.line_offsets[line];
        Position {
            line: (line + 1) as u32,
            column: col,
        }
    }

    fn add_function(&mut self, name: String, decl_span: Span, body_span: Span) {
        let id = self.fn_counter.to_string();
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
    }

    fn add_statement(&mut self, span: Span) {
        let id = self.stmt_counter.to_string();
        self.stmt_counter += 1;
        self.statement_map.insert(id, self.span_to_location(span));
    }

    fn add_branch(&mut self, branch_type: &str, span: Span, locations: Vec<Span>) {
        let id = self.branch_counter.to_string();
        self.branch_counter += 1;
        let line = self.offset_to_position(span.start).line;
        self.branch_map.insert(
            id,
            BranchEntry {
                line,
                branch_type: branch_type.to_string(),
                locations: locations.iter().map(|s| self.span_to_location(*s)).collect(),
            },
        );
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

impl<'a> Visit<'a> for CoverageVisitor {
    fn visit_function(&mut self, func: &Function<'a>, flags: oxc_semantic::ScopeFlags) {
        let name = self.resolve_function_name(func);
        let decl_span = if let Some(id) = &func.id {
            Span::new(func.span.start, id.span.end)
        } else {
            Span::new(func.span.start, func.span.start + 8)
        };
        if let Some(body) = &func.body {
            self.add_function(name, decl_span, body.span);
        }
        walk::walk_function(self, func, flags);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let name = self
            .pending_name
            .take()
            .unwrap_or_else(|| format!("(anonymous_{})", self.fn_counter));
        self.add_function(
            name,
            Span::new(arrow.span.start, arrow.span.start + 1),
            arrow.body.span,
        );
        walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        if let Some(id) = decl.id.get_binding_identifier() {
            if decl.init.as_ref().is_some_and(|init| {
                matches!(
                    init,
                    Expression::ArrowFunctionExpression(_)
                        | Expression::FunctionExpression(_)
                )
            }) {
                self.pending_name = Some(id.name.to_string());
            }
        }
        walk::walk_variable_declarator(self, decl);
        self.pending_name = None;
    }

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        if let PropertyKey::StaticIdentifier(id) = &method.key {
            self.pending_name = Some(id.name.to_string());
        }
        walk::walk_method_definition(self, method);
        self.pending_name = None;
    }

    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::BlockStatement(_) | Statement::EmptyStatement(_) => {}
            _ => self.add_statement(stmt.span()),
        }
        walk::walk_statement(self, stmt);
    }

    fn visit_if_statement(&mut self, stmt: &IfStatement<'a>) {
        let consequent_span = stmt.consequent.span();
        let alternate_span = stmt
            .alternate
            .as_ref()
            .map(|alt| alt.span())
            .unwrap_or(Span::new(stmt.span.end, stmt.span.end));
        self.add_branch("if", stmt.span, vec![consequent_span, alternate_span]);
        walk::walk_if_statement(self, stmt);
    }

    fn visit_conditional_expression(&mut self, expr: &ConditionalExpression<'a>) {
        self.add_branch(
            "cond-expr",
            expr.span,
            vec![expr.consequent.span(), expr.alternate.span()],
        );
        walk::walk_conditional_expression(self, expr);
    }

    fn visit_switch_statement(&mut self, stmt: &SwitchStatement<'a>) {
        let case_spans: Vec<Span> = stmt.cases.iter().map(|c| c.span).collect();
        self.add_branch("switch", stmt.span, case_spans);
        walk::walk_switch_statement(self, stmt);
    }

    fn visit_logical_expression(&mut self, expr: &LogicalExpression<'a>) {
        if matches!(
            expr.operator,
            LogicalOperator::And | LogicalOperator::Or
        ) {
            self.add_branch(
                "binary-expr",
                expr.span,
                vec![expr.left.span(), expr.right.span()],
            );
        }
        walk::walk_logical_expression(self, expr);
    }
}
