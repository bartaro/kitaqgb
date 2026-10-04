use kitaqgb::{codegen, lowerer, parser, tokenizer::Severity};
#[test]
fn legacy_self_recursion_is_rejected_and_stackcall_is_accepted() {
    for (filename, rejected) in [("recursion-static.c", true), ("recursion-stack.c", false)] {
        let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/diagnostics-fixtures")
            .join(filename);
        let parsed = parser::parse_files(&[source], &[], false);
        assert!(
            !parsed
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{:?}",
            parsed.diagnostics
        );
        let lowered = lowerer::lower(&parsed.tree, &Default::default());
        assert!(
            !lowered
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{:?}",
            lowered.diagnostics
        );
        let output = codegen::emission::compile(&lowered.tree, Default::default());
        assert_eq!(
            output
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            rejected,
            "{:?}",
            output.diagnostics
        );
        if rejected {
            assert!(
                output
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("direct recursion requires __stackcall"))
            );
        }
    }
}
