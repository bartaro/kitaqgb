use kitaqgb::{
    codegen::{
        Environment, Options, Symbol, SymbolTag,
        allocation::{AllocationRegion, Reservation},
    },
    ctype::{AggregateLayout, CType, Simple},
    expr::{Arg, Expr},
    io, ir_json,
    json::Value,
    parser, tags as t,
    tokenizer::Severity,
};
use std::path::PathBuf;
fn unwrap(e: &Expr) -> Expr {
    let mut e = e.clone();
    while matches!(
        e.tag(),
        Some(
            t::BANK
                | t::FIXED_BANK
                | t::DECL_ALIGN
                | t::FIXED_ORDER
                | t::DECL_SECTION
                | t::STATIC
                | t::UNSAFE
                | t::STACK_CALL
        )
    ) {
        e = (*e.children().last().unwrap()).clone()
    }
    e
}
fn expressions(e: &Expr, values: &mut Vec<Expr>) {
    if e.is(t::STATIC_ASSERT) {
        values.push(e.child(1).unwrap().clone())
    }
    for arg in e.args.iter().skip(1) {
        match arg {
            Arg::Expr(e) => expressions(e, values),
            Arg::Exprs(es) => {
                for e in es {
                    expressions(e, values)
                }
            }
            _ => {}
        }
    }
}
#[test]
fn constant_promotion_aggregate_layout_and_allocation_match_csharp() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/model-fixtures");
    let parsed = parser::parse_files(&[root.join("codegen-model.c")], &[], false);
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
    let mut env = Environment::new(Options::default());
    let decls = parsed
        .tree
        .children()
        .iter()
        .map(|e| unwrap(e))
        .collect::<Vec<_>>();
    for e in &decls {
        if e.is(t::CONSTANT) {
            env.add_constant(e, e.ty(1).unwrap(), e.text(2).unwrap(), e.child(3).unwrap())
        }
    }
    for e in &decls {
        if e.is(t::STRUCT) || e.is(t::UNION) {
            let Arg::Fields(fields) = &e.args[2] else {
                panic!()
            };
            env.define_aggregate(
                e,
                e.text(1).unwrap(),
                fields,
                if e.is(t::STRUCT) {
                    AggregateLayout::Struct
                } else {
                    AggregateLayout::Union
                },
                e.int(3).unwrap_or(0) != 0,
                e.int(4).unwrap_or(0),
            );
        } else if e.is(t::VARIABLE) {
            let ty = env.resolve_dimension(e.ty(2).unwrap());
            env.declare(
                e,
                Symbol {
                    tag: SymbolTag::Global,
                    value: 0,
                    ty,
                    name: e.text(3).unwrap().into(),
                    wram_bank: 0,
                },
            );
        }
    }
    for e in &decls {
        if let Some((ty, name, _)) = kitaqgb::codegen::readonly::declaration(e) {
            env.register_readonly(e, ty, name, 0, 0)
        }
    }
    let expected = Value::parse(&io::read_utf8(&root.join("codegen-model.json")).unwrap()).unwrap();
    let Value::Array(expected) = expected else {
        panic!()
    };
    let mut actual = Vec::new();
    for (name, a) in &env.aggregates {
        actual.push(ir_json::expr(&Expr::new(
            "aggregate",
            vec![
                Arg::from(name.clone()),
                Arg::Int(a.total_size),
                Arg::Int(a.alignment),
                Arg::Int(i32::from(a.is_packed)),
                Arg::Fields(a.fields.clone()),
            ],
        )))
    }
    let mut values = Vec::new();
    expressions(&parsed.tree, &mut values);
    assert!(values.len() >= 121);
    for e in values {
        let ty = env.type_of(&e);
        let value = env.try_constant(&e);
        let (ok, n, ct) = value.map_or((0, 0, CType::simple(Simple::UInt16)), |v| {
            (1, v.value, v.ty)
        });
        actual.push(ir_json::expr(&Expr::new(
            "expression",
            vec![
                Arg::from(e.clone()),
                Arg::from(ty),
                Arg::Int(ok),
                Arg::Int(n),
                Arg::from(ct),
                Arg::from(env.fold(&e)),
            ],
        )))
    }
    for e in &decls {
        if let Some((ty, name, values)) = kitaqgb::codegen::readonly::declaration(e) {
            let mut relocations = Vec::new();
            let bytes = env.encode_readonly(e, ty, &values, &mut relocations, 0);
            let align = env.readonly_alignment(e, ty, 0);
            actual.push(ir_json::expr(&Expr::new(
                "readonly",
                vec![
                    Arg::from(name),
                    Arg::Bytes(bytes),
                    Arg::Exprs(relocations),
                    Arg::Int(align),
                ],
            )))
        }
    }
    let mut region = AllocationRegion::new("TEST", 0xc000, 0xc07f);
    region.reservations = vec![
        Reservation {
            begin: 0xc010,
            end: 0xc01f,
            name: "first".into(),
        },
        Reservation {
            begin: 0xc030,
            end: 0xc037,
            name: "second".into(),
        },
    ];
    for row in expected.iter().skip(actual.len()).take(32) {
        let Value::Object(row) = row else { panic!() };
        let Value::Array(args) = &row["args"] else {
            panic!()
        };
        let Value::Number(size) = args[1] else {
            panic!()
        };
        let Value::Number(align) = args[2] else {
            panic!()
        };
        let address = region.allocate(size as i32, align as i32).unwrap_or(-1);
        actual.push(ir_json::expr(&Expr::new(
            "allocation",
            vec![
                Arg::Int(size as i32),
                Arg::Int(align as i32),
                Arg::Int(address),
                Arg::Int(region.next),
            ],
        )))
    }
    env.options.cgb_only = true;
    let origin = Expr::new("origin", Vec::new());
    let globals = [
        (
            kitaqgb::asm::Region::HighMem,
            CType::simple(Simple::UInt8),
            "hbyte",
            0,
        ),
        (
            kitaqgb::asm::Region::HighMem,
            CType::simple(Simple::UInt16),
            "hword",
            0,
        ),
        (
            kitaqgb::asm::Region::Wram0,
            CType::simple(Simple::UInt8),
            "gbyte",
            0,
        ),
        (
            kitaqgb::asm::Region::Ram,
            CType::array(CType::simple(Simple::UInt8), 5),
            "aligned",
            4,
        ),
        (
            kitaqgb::asm::Region::Ram,
            CType::simple(Simple::UInt16),
            "gword",
            0,
        ),
        (
            kitaqgb::asm::Region::Fixed(0xc020),
            CType::array(CType::simple(Simple::UInt8), 8),
            "fixed",
            0,
        ),
        (
            kitaqgb::asm::Region::Ram,
            CType::array(CType::simple(Simple::UInt8), 24),
            "afterfixed",
            0,
        ),
        (
            kitaqgb::asm::Region::Oam,
            CType::simple(Simple::UInt16),
            "oam",
            0,
        ),
        (
            kitaqgb::asm::Region::WramX(1),
            CType::array(CType::simple(Simple::UInt8), 5),
            "bank1",
            0,
        ),
        (
            kitaqgb::asm::Region::WramX(3),
            CType::array(CType::simple(Simple::UInt8), 7),
            "bank3",
            0,
        ),
    ];
    for (region, ty, name, align) in globals {
        let symbol = env.declare_global(&origin, region, &ty, name, align);
        actual.push(ir_json::expr(&Expr::new(
            "global",
            vec![
                Arg::Region(region),
                Arg::from(ty),
                Arg::from(name),
                Arg::Int(align),
                Arg::Int(symbol.value),
                Arg::Int(symbol.wram_bank),
            ],
        )))
    }
    fn state(env: &Environment, name: &str) -> Value {
        let [h, w0, w1] = env.allocator.snapshot();
        ir_json::expr(&Expr::new(
            "state",
            vec![Arg::from(name), Arg::Int(h), Arg::Int(w0), Arg::Int(w1)],
        ))
    }
    env.allocator.begin_frame();
    env.begin_scope();
    actual.push(state(&env, "begin"));
    env.allocator.allocate(0, 8, 1);
    env.begin_scope();
    env.allocator.allocate(0, 16, 2);
    env.allocator.allocate(1, 7, 2);
    actual.push(state(&env, "inner"));
    env.end_scope();
    actual.push(state(&env, "inner_end"));
    env.allocator.allocate(0, 10, 1);
    actual.push(state(&env, "reuse"));
    env.end_scope();
    actual.push(state(&env, "outer_end"));
    env.allocator.commit_frame();
    actual.push(state(&env, "frame_commit"));
    for level in [0, 1] {
        for pragma in [0, 2, 512] {
            env.options.opt_level = level;
            let ty = CType::array(CType::simple(Simple::UInt8), 256);
            let align = env.readonly_alignment(&origin, &ty, pragma);
            actual.push(ir_json::expr(&Expr::new(
                "readonly_alignment",
                vec![
                    Arg::Int(level),
                    Arg::from(ty),
                    Arg::Int(pragma),
                    Arg::Int(align),
                ],
            )))
        }
    }
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(actual, expected, "model row {i}")
    }
    assert!(
        !env.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "{:?}",
        env.diagnostics
    );
}
