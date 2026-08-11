//! AST-native runtime setup for the satellite adapter.

use oxc_allocator::{Allocator, GetAllocator, Vec as ArenaVec};
use oxc_ast::{ast::*, builder::AstBuilder};
use oxc_coverage_types::FileCoverage;
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::{
    node::NodeId,
    number::NumberBase,
    operator::{
        AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator,
    },
    reference::{Reference, ReferenceFlags},
    scope::{ScopeFlags, ScopeId},
    symbol::{SymbolFlags, SymbolId},
};

use crate::ordered_value::{OrderedValue, OrderedValueError, to_ordered_value};

#[derive(Clone, Copy)]
pub struct RuntimeSetupInputs<'a> {
    pub(crate) coverage: &'a FileCoverage,
    pub(crate) coverage_hash: &'a str,
    pub(crate) coverage_var: &'a str,
    pub(crate) coverage_name: &'a str,
    pub(crate) truthy_helper: Option<&'a str>,
    pub(crate) optional_chain_helper: Option<&'a str>,
}

pub fn insert_runtime_setup<'arena>(
    allocator: &'arena Allocator,
    program: &mut Program<'arena>,
    mut scoping: Scoping,
    inputs: RuntimeSetupInputs<'_>,
) -> Result<Scoping, OrderedValueError> {
    let coverage_value = to_ordered_value(inputs.coverage)?;
    let mut builder = RuntimeBuilder::new(allocator, &mut scoping);
    let mut setup = builder.build_setup(inputs, coverage_value);
    while let Some(statement) = setup.pop() {
        program.body.insert(0, statement);
    }
    Ok(scoping)
}

struct RuntimeBuilder<'arena, 'scoping> {
    ast: AstBuilder<'arena>,
    scoping: &'scoping mut Scoping,
    root_scope: ScopeId,
}

impl<'arena, 'scoping> RuntimeBuilder<'arena, 'scoping> {
    fn new(allocator: &'arena Allocator, scoping: &'scoping mut Scoping) -> Self {
        Self { ast: AstBuilder::new(allocator), root_scope: scoping.root_scope_id(), scoping }
    }

    fn build_setup(
        &mut self,
        inputs: RuntimeSetupInputs<'_>,
        coverage_value: OrderedValue,
    ) -> Vec<Statement<'arena>> {
        let mut statements = vec![self.coverage_iife(&inputs, coverage_value)];
        if let Some(name) = inputs.truthy_helper {
            statements.extend(self.truthy_helper(name, inputs.coverage_name));
        }
        if let Some(name) = inputs.optional_chain_helper {
            statements.push(self.optional_chain_helper(name, inputs.coverage_name));
        }
        statements
    }

    fn coverage_iife(
        &mut self,
        inputs: &RuntimeSetupInputs<'_>,
        coverage_value: OrderedValue,
    ) -> Statement<'arena> {
        let scope = self.add_scope(self.root_scope, ScopeFlags::Function);
        let path = self.declare("path", scope, SymbolFlags::FunctionScopedVariable);
        let hash = self.declare("hash", scope, SymbolFlags::FunctionScopedVariable);
        let gcv = self.declare("gcv", scope, SymbolFlags::FunctionScopedVariable);
        let coverage_data =
            self.declare("coverageData", scope, SymbolFlags::FunctionScopedVariable);
        let coverage = self.declare("coverage", scope, SymbolFlags::FunctionScopedVariable);
        let actual = self.declare("actualCoverage", scope, SymbolFlags::FunctionScopedVariable);

        let mut body = ArenaVec::new_in(&self.ast);
        body.push(self.var_statement("path", path, self.string(inputs.coverage.path.as_str())));
        body.push(self.var_statement("hash", hash, self.string(inputs.coverage_hash)));
        body.push(self.var_statement("gcv", gcv, self.string(inputs.coverage_var)));
        let coverage_data_value = self.ordered_value(coverage_value);
        body.push(self.var_statement("coverageData", coverage_data, coverage_data_value));
        let coverage_data_read = self.read("coverageData", coverage_data, scope);
        let coverage_data_hash = self.static_member(coverage_data_read, "hash");
        let hash_value = self.read("hash", hash, scope);
        body.push(self.assign_member_statement(coverage_data_hash, hash_value));

        let global_value = self.global_selector(scope);
        body.push(self.var_statement("coverage", coverage, global_value));

        let store_scope = self.add_scope(scope, ScopeFlags::empty());
        let coverage_read = self.read("coverage", coverage, scope);
        let gcv_read = self.read("gcv", gcv, scope);
        let coverage_store = self.computed_member(coverage_read, gcv_read);
        let create_store = self.assign_member_statement(coverage_store, self.empty_object());
        let coverage_read = self.read("coverage", coverage, scope);
        let gcv_read = self.read("gcv", gcv, scope);
        let coverage_store = self.computed_member(coverage_read, gcv_read);
        body.push(Statement::new_if_statement(
            SPAN,
            self.not(Expression::from(coverage_store)),
            self.block(store_scope, vec![create_store]),
            None,
            &self.ast,
        ));

        let install_scope = self.add_scope(scope, ScopeFlags::empty());
        if inputs.coverage.path == "__proto__" {
            let has_own = self.object_has_own_call(coverage, gcv, path, scope);
            let missing_or_stale = self.missing_or_stale(coverage, gcv, path, hash, scope);
            let test = self.logical(self.not(has_own), LogicalOperator::Or, missing_or_stale);
            let define =
                self.object_define_property_call(coverage, gcv, path, coverage_data, scope);
            let define_statement = self.expression_statement(define);
            body.push(Statement::new_if_statement(
                SPAN,
                test,
                self.block(install_scope, vec![define_statement]),
                None,
                &self.ast,
            ));
        } else {
            let file_slot = self.coverage_file_slot(coverage, gcv, path, scope);
            let coverage_data_read = self.read("coverageData", coverage_data, scope);
            let install = self.assign_member_statement(file_slot, coverage_data_read);
            let missing_or_stale = self.missing_or_stale(coverage, gcv, path, hash, scope);
            body.push(Statement::new_if_statement(
                SPAN,
                missing_or_stale,
                self.block(install_scope, vec![install]),
                None,
                &self.ast,
            ));
        }

        let actual_value = Expression::from(self.coverage_file_slot(coverage, gcv, path, scope));
        body.push(self.var_statement("actualCoverage", actual, actual_value));
        body.push(Statement::new_return_statement(
            SPAN,
            Some(self.read("actualCoverage", actual, scope)),
            &self.ast,
        ));

        let params = FormalParameters::boxed(
            SPAN,
            FormalParameterKind::FormalParameter,
            ArenaVec::new_in(&self.ast),
            None::<FormalParameterRest<'arena>>,
            &self.ast,
        );
        let function_body = FunctionBody::boxed(SPAN, ArenaVec::new_in(&self.ast), body, &self.ast);
        let function = Expression::new_function_expression_with_scope_id_and_pure_and_pife(
            SPAN,
            FunctionType::FunctionExpression,
            None,
            false,
            false,
            false,
            None::<TSTypeParameterDeclaration<'arena>>,
            None::<TSThisParameter<'arena>>,
            params,
            None::<TSTypeAnnotation<'arena>>,
            Some(function_body),
            scope,
            false,
            true,
            &self.ast,
        );
        let call = self.call(function, vec![]);
        let coverage_symbol = self
            .scoping
            .get_root_binding(inputs.coverage_name.into())
            .expect("coverage transform registers its root binding");
        self.var_statement(inputs.coverage_name, coverage_symbol, call)
    }

    fn global_selector(&mut self, scope: ScopeId) -> Expression<'arena> {
        let global_this = self.unresolved("globalThis", scope);
        let global = self.unresolved("global", scope);
        let self_global = self.unresolved("self", scope);
        let undefined = self.string("undefined");
        let self_test = self.binary(
            Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, self_global, &self.ast),
            BinaryOperator::StrictInequality,
            self.string("undefined"),
        );
        let self_or_this = Expression::new_conditional_expression(
            SPAN,
            self_test,
            self.unresolved("self", scope),
            Expression::new_this_expression(SPAN, &self.ast),
            &self.ast,
        );
        let global_test = self.binary(
            Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, global, &self.ast),
            BinaryOperator::StrictInequality,
            undefined,
        );
        let global_or_self = Expression::new_conditional_expression(
            SPAN,
            global_test,
            self.unresolved("global", scope),
            self_or_this,
            &self.ast,
        );
        let global_this_test = self.binary(
            Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, global_this, &self.ast),
            BinaryOperator::StrictInequality,
            self.string("undefined"),
        );
        Expression::new_conditional_expression(
            SPAN,
            global_this_test,
            self.unresolved("globalThis", scope),
            global_or_self,
            &self.ast,
        )
    }

    fn truthy_helper(&mut self, helper_name: &str, coverage_name: &str) -> Vec<Statement<'arena>> {
        let temp_name = format!("{coverage_name}_temp");
        let temp = self.declare(&temp_name, self.root_scope, SymbolFlags::FunctionScopedVariable);
        let temp_decl = self.var_declaration(&temp_name, temp, None);
        let scope = self.add_scope(self.root_scope, ScopeFlags::Function);
        let val = self.declare("val", scope, SymbolFlags::FunctionScopedVariable);
        let id = self.declare("id", scope, SymbolFlags::FunctionScopedVariable);
        let idx = self.declare("idx", scope, SymbolFlags::FunctionScopedVariable);
        let helper = self.root_symbol(helper_name);
        let coverage = self.root_symbol(coverage_name);

        let mut body = ArenaVec::new_in(&self.ast);
        let val_read = self.read("val", val, scope);
        let assign_temp = self.assign_identifier_statement(&temp_name, temp, scope, val_read);
        body.push(assign_temp);
        let temp_read = self.read(&temp_name, temp, scope);
        let array = self.unresolved("Array", scope);
        let is_array = self.static_member(array, "isArray");
        let temp_for_array = self.read(&temp_name, temp, scope);
        let array_check = self.call(Expression::from(is_array), vec![temp_for_array]);
        let temp_for_length = self.read(&temp_name, temp, scope);
        let array_length = Expression::from(self.static_member(temp_for_length, "length"));
        let array_or_length =
            self.logical(self.not(array_check), LogicalOperator::Or, array_length);
        let object = self.unresolved("Object", scope);
        let get_prototype = self.static_member(object, "getPrototypeOf");
        let temp_for_prototype = self.read(&temp_name, temp, scope);
        let prototype = self.call(Expression::from(get_prototype), vec![temp_for_prototype]);
        let object = self.unresolved("Object", scope);
        let object_prototype = Expression::from(self.static_member(object, "prototype"));
        let non_plain = self.binary(prototype, BinaryOperator::StrictInequality, object_prototype);
        let object = self.unresolved("Object", scope);
        let values_method = self.static_member(object, "values");
        let temp_for_values = self.read(&temp_name, temp, scope);
        let values = self.call(Expression::from(values_method), vec![temp_for_values]);
        let values_length = Expression::from(self.static_member(values, "length"));
        let object_non_empty = self.logical(non_plain, LogicalOperator::Or, values_length);
        let test = self.logical(
            self.logical(temp_read, LogicalOperator::And, array_or_length),
            LogicalOperator::And,
            object_non_empty,
        );
        let counter = self.truthy_counter(coverage_name, coverage, id, idx, scope);
        let counter_scope = self.add_scope(scope, ScopeFlags::empty());
        body.push(Statement::new_if_statement(
            SPAN,
            test,
            self.block(counter_scope, vec![self.expression_statement(counter)]),
            None,
            &self.ast,
        ));
        body.push(Statement::new_return_statement(
            SPAN,
            Some(self.read(&temp_name, temp, scope)),
            &self.ast,
        ));

        vec![
            temp_decl,
            self.function_declaration(
                helper_name,
                helper,
                scope,
                vec![("val", val), ("id", id), ("idx", idx)],
                body,
            ),
        ]
    }

    fn optional_chain_helper(
        &mut self,
        helper_name: &str,
        coverage_name: &str,
    ) -> Statement<'arena> {
        let scope = self.add_scope(self.root_scope, ScopeFlags::Function);
        let val = self.declare("val", scope, SymbolFlags::FunctionScopedVariable);
        let id = self.declare("id", scope, SymbolFlags::FunctionScopedVariable);
        let helper = self.root_symbol(helper_name);
        let coverage = self.root_symbol(coverage_name);
        let val_read = self.read("val", val, scope);
        let nullish = self.binary(
            val_read,
            BinaryOperator::Equality,
            Expression::new_null_literal(SPAN, &self.ast),
        );
        let slot = Expression::new_conditional_expression(
            SPAN,
            nullish,
            self.number(0.0),
            self.number(1.0),
            &self.ast,
        );
        let counter = self.coverage_counter(coverage_name, coverage, "b", id, slot, scope);
        let mut body = ArenaVec::new_in(&self.ast);
        body.push(self.expression_statement(counter));
        body.push(Statement::new_return_statement(
            SPAN,
            Some(self.read("val", val, scope)),
            &self.ast,
        ));
        self.function_declaration(helper_name, helper, scope, vec![("val", val), ("id", id)], body)
    }

    fn function_declaration(
        &self,
        name: &str,
        symbol: SymbolId,
        scope: ScopeId,
        params: Vec<(&str, SymbolId)>,
        body: ArenaVec<'arena, Statement<'arena>>,
    ) -> Statement<'arena> {
        let id = BindingIdentifier::new_with_symbol_id(SPAN, self.alloc(name), symbol, &self.ast);
        let mut items = ArenaVec::new_in(&self.ast);
        for (name, symbol) in params {
            let pattern = BindingPattern::new_binding_identifier_with_symbol_id(
                SPAN,
                self.alloc(name),
                symbol,
                &self.ast,
            );
            items.push(FormalParameter::new(
                SPAN,
                ArenaVec::new_in(&self.ast),
                pattern,
                None::<TSTypeAnnotation<'arena>>,
                None::<Expression<'arena>>,
                false,
                None,
                false,
                false,
                &self.ast,
            ));
        }
        let params = FormalParameters::boxed(
            SPAN,
            FormalParameterKind::FormalParameter,
            items,
            None::<FormalParameterRest<'arena>>,
            &self.ast,
        );
        let body = FunctionBody::boxed(SPAN, ArenaVec::new_in(&self.ast), body, &self.ast);
        Statement::new_function_declaration_with_scope_id_and_pure_and_pife(
            SPAN,
            FunctionType::FunctionDeclaration,
            Some(id),
            false,
            false,
            false,
            None::<TSTypeParameterDeclaration<'arena>>,
            None::<TSThisParameter<'arena>>,
            params,
            None::<TSTypeAnnotation<'arena>>,
            Some(body),
            scope,
            false,
            false,
            &self.ast,
        )
    }

    fn truthy_counter(
        &mut self,
        coverage_name: &str,
        coverage: SymbolId,
        id: SymbolId,
        idx: SymbolId,
        scope: ScopeId,
    ) -> Expression<'arena> {
        let slot = self.read("idx", idx, scope);
        self.coverage_counter(coverage_name, coverage, "bT", id, slot, scope)
    }

    fn coverage_counter(
        &mut self,
        coverage_name: &str,
        coverage: SymbolId,
        map: &str,
        id: SymbolId,
        slot: Expression<'arena>,
        scope: ScopeId,
    ) -> Expression<'arena> {
        let coverage_read = self.read(coverage_name, coverage, scope);
        let map = self.static_member(coverage_read, map);
        let id_read = self.read("id", id, scope);
        let branch = self.computed_member(Expression::from(map), id_read);
        let counter = self.computed_member(Expression::from(branch), slot);
        Expression::new_update_expression(
            SPAN,
            UpdateOperator::Increment,
            true,
            SimpleAssignmentTarget::from(counter),
            &self.ast,
        )
    }

    fn missing_or_stale(
        &mut self,
        coverage: SymbolId,
        gcv: SymbolId,
        path: SymbolId,
        hash: SymbolId,
        scope: ScopeId,
    ) -> Expression<'arena> {
        let missing_slot = self.coverage_file_slot(coverage, gcv, path, scope);
        let missing = self.not(Expression::from(missing_slot));
        let hash_slot = self.coverage_file_slot(coverage, gcv, path, scope);
        let current_hash = self.static_member(Expression::from(hash_slot), "hash");
        let expected_hash = self.read("hash", hash, scope);
        let stale = self.binary(
            Expression::from(current_hash),
            BinaryOperator::StrictInequality,
            expected_hash,
        );
        self.logical(missing, LogicalOperator::Or, stale)
    }

    fn coverage_file_slot(
        &mut self,
        coverage: SymbolId,
        gcv: SymbolId,
        path: SymbolId,
        scope: ScopeId,
    ) -> MemberExpression<'arena> {
        let coverage_read = self.read("coverage", coverage, scope);
        let gcv_read = self.read("gcv", gcv, scope);
        let store = self.computed_member(coverage_read, gcv_read);
        let path_read = self.read("path", path, scope);
        self.computed_member(Expression::from(store), path_read)
    }

    fn object_has_own_call(
        &mut self,
        coverage: SymbolId,
        gcv: SymbolId,
        path: SymbolId,
        scope: ScopeId,
    ) -> Expression<'arena> {
        let object = self.unresolved("Object", scope);
        let prototype = self.static_member(object, "prototype");
        let has_own = self.static_member(Expression::from(prototype), "hasOwnProperty");
        let call = self.static_member(Expression::from(has_own), "call");
        let coverage_read = self.read("coverage", coverage, scope);
        let gcv_read = self.read("gcv", gcv, scope);
        let store = self.computed_member(coverage_read, gcv_read);
        let path_read = self.read("path", path, scope);
        self.call(Expression::from(call), vec![Expression::from(store), path_read])
    }

    fn object_define_property_call(
        &mut self,
        coverage: SymbolId,
        gcv: SymbolId,
        path: SymbolId,
        coverage_data: SymbolId,
        scope: ScopeId,
    ) -> Expression<'arena> {
        let object = self.unresolved("Object", scope);
        let callee = self.static_member(object, "defineProperty");
        let coverage_read = self.read("coverage", coverage, scope);
        let gcv_read = self.read("gcv", gcv, scope);
        let store = self.computed_member(coverage_read, gcv_read);
        let coverage_data_read = self.read("coverageData", coverage_data, scope);
        let descriptor = self.object(vec![
            ("value", coverage_data_read),
            ("enumerable", self.boolean(true)),
            ("writable", self.boolean(true)),
            ("configurable", self.boolean(true)),
        ]);
        let path_read = self.read("path", path, scope);
        self.call(Expression::from(callee), vec![Expression::from(store), path_read, descriptor])
    }

    fn ordered_value(&mut self, value: OrderedValue) -> Expression<'arena> {
        match value {
            OrderedValue::Null => Expression::new_null_literal(SPAN, &self.ast),
            OrderedValue::Bool(value) => self.boolean(value),
            OrderedValue::Number(value) => self.number(value),
            OrderedValue::String(value) => self.string(&value),
            OrderedValue::Array(values) => {
                let mut elements = ArenaVec::new_in(&self.ast);
                for value in values {
                    elements.push(ArrayExpressionElement::from(self.ordered_value(value)));
                }
                Expression::new_array_expression(SPAN, elements, &self.ast)
            }
            OrderedValue::Object(entries) => {
                let mut properties = ArenaVec::new_in(&self.ast);
                for (key, value) in entries {
                    let computed = key == "__proto__";
                    let key =
                        PropertyKey::new_string_literal(SPAN, self.alloc(&key), None, &self.ast);
                    let value = self.ordered_value(value);
                    properties.push(ObjectPropertyKind::new_object_property(
                        SPAN,
                        PropertyKind::Init,
                        key,
                        value,
                        false,
                        false,
                        computed,
                        &self.ast,
                    ));
                }
                Expression::new_object_expression(SPAN, properties, &self.ast)
            }
        }
    }

    fn object(&self, entries: Vec<(&str, Expression<'arena>)>) -> Expression<'arena> {
        let mut properties = ArenaVec::new_in(&self.ast);
        for (key, value) in entries {
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(SPAN, self.alloc(key), &self.ast),
                value,
                false,
                false,
                false,
                &self.ast,
            ));
        }
        Expression::new_object_expression(SPAN, properties, &self.ast)
    }

    fn empty_object(&self) -> Expression<'arena> {
        Expression::new_object_expression(SPAN, ArenaVec::new_in(&self.ast), &self.ast)
    }

    fn var_statement(
        &self,
        name: &str,
        symbol: SymbolId,
        init: Expression<'arena>,
    ) -> Statement<'arena> {
        self.var_declaration(name, symbol, Some(init))
    }

    fn var_declaration(
        &self,
        name: &str,
        symbol: SymbolId,
        init: Option<Expression<'arena>>,
    ) -> Statement<'arena> {
        let pattern = BindingPattern::new_binding_identifier_with_symbol_id(
            SPAN,
            self.alloc(name),
            symbol,
            &self.ast,
        );
        let declarator = VariableDeclarator::new(
            SPAN,
            VariableDeclarationKind::Var,
            pattern,
            None::<TSTypeAnnotation<'arena>>,
            init,
            false,
            &self.ast,
        );
        Statement::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Var,
            ArenaVec::from_array_in([declarator], &self.ast),
            false,
            &self.ast,
        )
    }

    fn assign_identifier_statement(
        &mut self,
        name: &str,
        symbol: SymbolId,
        scope: ScopeId,
        value: Expression<'arena>,
    ) -> Statement<'arena> {
        let reference = self.reference(symbol, scope, ReferenceFlags::write());
        let target = AssignmentTarget::new_assignment_target_identifier_with_reference_id(
            SPAN,
            self.alloc(name),
            reference,
            &self.ast,
        );
        self.expression_statement(Expression::new_assignment_expression(
            SPAN,
            AssignmentOperator::Assign,
            target,
            value,
            &self.ast,
        ))
    }

    fn assign_member_statement(
        &self,
        member: MemberExpression<'arena>,
        value: Expression<'arena>,
    ) -> Statement<'arena> {
        self.expression_statement(Expression::new_assignment_expression(
            SPAN,
            AssignmentOperator::Assign,
            AssignmentTarget::from(member),
            value,
            &self.ast,
        ))
    }

    fn expression_statement(&self, expression: Expression<'arena>) -> Statement<'arena> {
        Statement::new_expression_statement(SPAN, expression, &self.ast)
    }

    fn block(&self, scope: ScopeId, statements: Vec<Statement<'arena>>) -> Statement<'arena> {
        Statement::new_block_statement_with_scope_id(
            SPAN,
            ArenaVec::from_iter_in(statements, &self.ast),
            scope,
            &self.ast,
        )
    }

    fn call(
        &self,
        callee: Expression<'arena>,
        arguments: Vec<Expression<'arena>>,
    ) -> Expression<'arena> {
        let arguments =
            ArenaVec::from_iter_in(arguments.into_iter().map(Argument::from), &self.ast);
        Expression::new_call_expression(
            SPAN,
            callee,
            None::<TSTypeParameterInstantiation<'arena>>,
            arguments,
            false,
            &self.ast,
        )
    }

    fn static_member(
        &self,
        object: Expression<'arena>,
        property: &str,
    ) -> MemberExpression<'arena> {
        MemberExpression::new_static_member_expression(
            SPAN,
            object,
            IdentifierName::new(SPAN, self.alloc(property), &self.ast),
            false,
            &self.ast,
        )
    }

    fn computed_member(
        &self,
        object: Expression<'arena>,
        property: Expression<'arena>,
    ) -> MemberExpression<'arena> {
        MemberExpression::new_computed_member_expression(SPAN, object, property, false, &self.ast)
    }

    fn logical(
        &self,
        left: Expression<'arena>,
        operator: LogicalOperator,
        right: Expression<'arena>,
    ) -> Expression<'arena> {
        Expression::new_logical_expression(SPAN, left, operator, right, &self.ast)
    }

    fn binary(
        &self,
        left: Expression<'arena>,
        operator: BinaryOperator,
        right: Expression<'arena>,
    ) -> Expression<'arena> {
        Expression::new_binary_expression(SPAN, left, operator, right, &self.ast)
    }

    fn not(&self, expression: Expression<'arena>) -> Expression<'arena> {
        Expression::new_unary_expression(SPAN, UnaryOperator::LogicalNot, expression, &self.ast)
    }

    fn string(&self, value: &str) -> Expression<'arena> {
        Expression::new_string_literal(SPAN, self.alloc(value), None, &self.ast)
    }

    fn number(&self, value: f64) -> Expression<'arena> {
        Expression::new_numeric_literal(SPAN, value, None, NumberBase::Decimal, &self.ast)
    }

    fn boolean(&self, value: bool) -> Expression<'arena> {
        Expression::new_boolean_literal(SPAN, value, &self.ast)
    }

    fn read(&mut self, name: &str, symbol: SymbolId, scope: ScopeId) -> Expression<'arena> {
        let reference = self.reference(symbol, scope, ReferenceFlags::read());
        Expression::new_identifier_with_reference_id(SPAN, self.alloc(name), reference, &self.ast)
    }

    fn unresolved(&mut self, name: &str, scope: ScopeId) -> Expression<'arena> {
        let reference = self.scoping.create_reference(Reference::new(
            NodeId::DUMMY,
            scope,
            ReferenceFlags::read(),
        ));
        self.scoping.add_root_unresolved_reference(name.into(), reference);
        Expression::new_identifier_with_reference_id(SPAN, self.alloc(name), reference, &self.ast)
    }

    fn reference(
        &mut self,
        symbol: SymbolId,
        scope: ScopeId,
        flags: ReferenceFlags,
    ) -> oxc_syntax::reference::ReferenceId {
        let reference = self.scoping.create_reference(Reference::new_with_symbol_id(
            NodeId::DUMMY,
            symbol,
            scope,
            flags,
        ));
        self.scoping.add_resolved_reference(symbol, reference);
        reference
    }

    fn declare(&mut self, name: &str, scope: ScopeId, flags: SymbolFlags) -> SymbolId {
        if self.scoping.scope_has_binding(scope, name.into()) {
            return self
                .scoping
                .get_binding(scope, name.into())
                .expect("scope_has_binding guarantees a local binding");
        }
        let symbol = self.scoping.create_symbol(SPAN, name.into(), flags, scope, NodeId::DUMMY);
        self.scoping.add_binding(scope, name.into(), symbol);
        symbol
    }

    fn root_symbol(&self, name: &str) -> SymbolId {
        self.scoping
            .get_root_binding(name.into())
            .unwrap_or_else(|| panic!("coverage transform did not register {name}"))
    }

    fn add_scope(&mut self, parent: ScopeId, flags: ScopeFlags) -> ScopeId {
        let flags = self.scoping.get_new_scope_flags(flags, parent);
        self.scoping.add_scope(Some(parent), NodeId::DUMMY, flags)
    }

    fn alloc(&self, value: &str) -> &'arena str {
        self.ast.allocator().alloc_str(value)
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::{Expression, ObjectPropertyKind};
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    use super::{OrderedValue, RuntimeBuilder};

    #[test]
    fn proto_key_is_emitted_as_a_computed_own_property() {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, "", SourceType::mjs()).parse();
        let mut scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();
        let mut builder = RuntimeBuilder::new(&allocator, &mut scoping);

        let expression = builder.ordered_value(OrderedValue::Object(vec![(
            "__proto__".to_string(),
            OrderedValue::Bool(true),
        )]));

        let Expression::ObjectExpression(object) = expression else {
            panic!("expected object expression");
        };
        let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
            panic!("expected object property");
        };
        assert!(property.computed);
    }
}
