//! Palette pragmas produce the original const-u16 ROM declarations.
use crate::{
    ctype::{CType, Simple},
    expr::{Arg, Expr},
    tags as t,
    tokenizer::{Diagnostic, Palette, Severity},
};
use std::{collections::BTreeSet, fmt::Write};
pub fn inject(mut tree: Expr, palettes: &[Palette], diagnostics: &mut Vec<Diagnostic>) -> Expr {
    if !tree.is(t::SEQUENCE) {
        return tree;
    }
    fn name(e: &Expr) -> Option<&str> {
        let mut e = e;
        for _ in 0..16 {
            if matches!(
                e.tag(),
                Some(t::BANK | t::FIXED_BANK | t::FIXED_ORDER | t::DECL_ALIGN | t::DECL_SECTION)
            ) {
                e = e.child(2)?;
            } else if matches!(e.tag(), Some(t::UNSAFE | t::STACK_CALL)) {
                e = e.child(1)?;
            } else {
                break;
            }
        }
        if matches!(
            e.tag(),
            Some(
                t::READONLY_DATA
                    | t::CONSTANT
                    | t::FUNCTION
                    | t::INLINE_FUNCTION
                    | t::FUNCTION_DECL
            )
        ) {
            e.text(2)
        } else if matches!(e.tag(), Some(t::VARIABLE | t::EXTERN_VARIABLE)) {
            e.text(3)
        } else {
            None
        }
    }
    let mut used = tree
        .children()
        .into_iter()
        .filter_map(name)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for p in palettes {
        if !used.insert(p.name.clone()) {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                p.position.clone(),
                format!(
                    "warning: cgb palette name collides with existing symbol: {}",
                    p.name
                ),
                0,
            ));
            continue;
        }
        let mut ty = CType::array(CType::simple(Simple::UInt16), 4);
        ty.is_const = true;
        let decl = Expr::new(
            t::READONLY_DATA,
            vec![
                Arg::Type(ty),
                Arg::Text(p.name.clone()),
                Arg::Ints(p.colors.to_vec()),
            ],
        )
        .with_source(p.position.clone());
        tree.args.push(decl.into());
    }
    tree
}
pub fn text(palettes: &[Palette]) -> String {
    let mut text = String::from("# KITAQGB CGB palette DSL\n# name, c0, c1, c2, c3\n");
    for p in palettes {
        writeln!(
            text,
            "{}, 0x{:04X}, 0x{:04X}, 0x{:04X}, 0x{:04X}",
            p.name, p.colors[0], p.colors[1], p.colors[2], p.colors[3]
        )
        .unwrap();
    }
    text
}
