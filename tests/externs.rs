use kitaqgb::{codegen, lowerer, parser, tokenizer::Severity};
#[test]
fn externs_require_matching_objects_without_allocating_declarations() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    for (file, message) in [
        ("emission-fixtures/extern-valid.c", ""),
        (
            "diagnostics-fixtures/extern-undefined.c",
            "extern symbol not defined",
        ),
        (
            "diagnostics-fixtures/extern-mismatch.c",
            "extern declaration type mismatch",
        ),
        (
            "diagnostics-fixtures/extern-constant.c",
            "compile-time constant",
        ),
        (
            "diagnostics-fixtures/extern-array.c",
            "does not match definition",
        ),
    ] {
        let parsed = parser::parse_files(&[root.join(file)], &[], false);
        assert!(
            !parsed
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{:?}",
            parsed.diagnostics
        );
        let lowered = lowerer::lower(&parsed.tree, &Default::default());
        let output = codegen::emission::compile(&lowered.tree, Default::default());
        let diagnostics = lowered
            .diagnostics
            .iter()
            .chain(&output.diagnostics)
            .collect::<Vec<_>>();
        assert_eq!(
            diagnostics.iter().any(|d| d.severity == Severity::Error),
            !message.is_empty(),
            "{file}: {:?}",
            diagnostics
        );
        if !message.is_empty() {
            assert!(
                diagnostics.iter().any(|d| d.message.contains(message)),
                "{:?}",
                diagnostics
            );
        }
    }
}
