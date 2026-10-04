use kitaqgb::{assembler, codegen, lowerer, parser, pipeline, rom_header, tokenizer::Severity};
use std::{fs, path::PathBuf};
#[test]
fn spilled_library_function_recompiles_its_cross_bank_calls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let parsed = parser::parse_files(
        &[root.join("relayout-fixtures/dot.c")],
        &[root.join("ast-fixtures/lib")],
        false,
    );
    let lowered = lowerer::lower(&parsed.tree, &lowerer::Options::default());
    assert!(
        !parsed
            .diagnostics
            .iter()
            .chain(&lowered.diagnostics)
            .any(|d| d.severity == Severity::Error)
    );
    let build = pipeline::compile_lowered(
        &lowered.tree,
        codegen::Options {
            fixed_stack: true,
            stack_top: 0xcfff,
            stack_reserve: 512,
            ..Default::default()
        },
        &assembler::Options {
            stack_top: 0xcfff,
            header_logo: true,
        },
    )
    .unwrap();
    assert_eq!(build.relayout_passes, 1);
    assert_eq!(build.generated.functions["kq3d_dot_q8_8"].bank, 3);
    let mut header = rom_header::Options::default();
    header.set("cgb", "cgb").unwrap();
    header.set("cart", "mbc5").unwrap();
    header.set("romsize", "128k").unwrap();
    let (rom, _) = rom_header::patch(&build.assembled.unwrap().rom, &header).unwrap();
    assert_eq!(
        rom,
        fs::read(root.join("relayout-fixtures/dot.gb")).unwrap()
    );
}
#[test]
fn fixed_bank_overflow_is_rejected() {
    use kitaqgb::{
        ctype::{CType, Simple},
        expr::{Arg, Expr},
        tags as t,
    };
    let mut functions = Vec::new();
    for name in ["main", "second"] {
        let body = Expr::new(
            t::SEQUENCE,
            (0..9000)
                .map(|_| Expr::asm("NOP", kitaqgb::asm::Operand::implicit()).into())
                .collect(),
        );
        let function = Expr::new(
            t::FUNCTION,
            vec![
                CType::simple(Simple::Void).into(),
                name.into(),
                Arg::Fields(vec![]),
                body.into(),
            ],
        );
        functions.push(Expr::new(t::FIXED_BANK, vec![0.into(), function.into()]));
    }
    let result = pipeline::compile_lowered(
        &Expr::new(t::SEQUENCE, functions.into_iter().map(Into::into).collect()),
        codegen::Options::default(),
        &assembler::Options::default(),
    );
    let Err(error) = result else {
        panic!("fixed bank overflow must fail");
    };
    assert!(error.contains("fixed-bank function spilled"), "{error}");
}
