use kitaqgb::{assembler, expr::Expr, tags};
#[test]
fn invalid_restart_vectors_fail_instead_of_silently_generating_a_rom() {
    for vector in [-1, 1, 7, 9, 0x39, 0x40] {
        let lines = [
            Expr::new(tags::RST_MAP, vec![vector.into(), "target".into()]),
            Expr::new(tags::FUNCTION, vec!["target".into()]),
        ];
        let error = assembler::assemble(&lines, &assembler::Options::default()).unwrap_err();
        assert!(error.contains("invalid $rst_map vector"), "{error}");
    }
}
