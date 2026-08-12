use oxc_allocator::Allocator;
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::{Visit, walk::walk_program};
use oxc_coverage_transform::{PragmaMap, TransformInit, TransformProgramInput, transform_program};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_syntax::reference::ReferenceId;

struct GeneratedReferenceAudit {
    references: Vec<(String, Option<ReferenceId>)>,
}

impl<'a> Visit<'a> for GeneratedReferenceAudit {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if identifier.span.is_empty() && identifier.name.starts_with("cov_") {
            self.references.push((identifier.name.to_string(), identifier.reference_id.get()));
        }
    }
}

#[test]
fn generated_counter_references_are_bound_in_returned_scoping() {
    let source = "const value = left && right; object?.property;";
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.diagnostics.has_errors());
    let scoping = SemanticBuilder::new().build(&parsed.program).semantic.into_scoping();
    let (pragmas, _) = PragmaMap::from_program(&parsed.program, source);

    let output = transform_program(TransformProgramInput {
        program: &mut parsed.program,
        scoping,
        pragmas,
        transform: TransformInit {
            allocator: &allocator,
            cov_fn_name: "cov_semantic_test".to_string(),
            report_logic: true,
            track_optional_chain: true,
            ignore_class_methods: Vec::new(),
            name_callback_arguments: false,
            istanbul_compat: false,
        },
    })
    .expect("valid transform input");

    let mut audit = GeneratedReferenceAudit { references: Vec::new() };
    walk_program(&mut audit, &parsed.program);
    assert!(!audit.references.is_empty());
    for (name, reference_id) in audit.references {
        let reference_id = reference_id.unwrap_or_else(|| panic!("{name} has no ReferenceId"));
        assert!(
            output.scoping.get_reference(reference_id).symbol_id().is_some(),
            "{name} is not resolved to a generated binding",
        );
    }
}
