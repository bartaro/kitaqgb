//! Typed IR serialization for differential evidence and compiler reports.
use crate::{
    asm::Region,
    ctype::{CType, Kind},
    expr::{Arg, Expr},
    json::Value,
};
use std::path::Path;
fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn normalize_name(s: &str) -> String {
    if s.starts_with("__anon_") {
        let mut parts = s.rsplitn(3, '_');
        let col = parts.next().unwrap_or("");
        let line = parts.next().unwrap_or("");
        if col.parse::<usize>().is_ok() && line.parse::<usize>().is_ok() {
            return format!(
                "{}_{}_{}",
                if s.starts_with("__anon_enum_") {
                    "__anon_enum"
                } else {
                    "__anon"
                },
                line,
                col
            );
        }
    }
    s.into()
}
fn text(s: &str) -> Value {
    Value::String(normalize_name(s))
}
fn number(n: i32) -> Value {
    Value::Number(i64::from(n))
}
pub fn ty(t: &CType) -> Value {
    let mut simple = "Implied".to_owned();
    let mut name = Value::Null;
    let mut subtype = Value::Null;
    let mut dimension = 0;
    let mut dimension_expr = Value::Null;
    let mut params = Value::Null;
    let tag = match &t.kind {
        Kind::Simple(s) => {
            simple = format!("{s:?}");
            "Simple"
        }
        Kind::Pointer(s) => {
            subtype = ty(s);
            "Pointer"
        }
        Kind::Struct(n) => {
            name = text(n);
            "Struct"
        }
        Kind::Union(n) => {
            name = text(n);
            "Union"
        }
        Kind::Enum(n) => {
            name = text(n);
            "Enum"
        }
        Kind::Array(s, n) => {
            subtype = ty(s);
            dimension = *n;
            "Array"
        }
        Kind::ArrayExpression(s, e) => {
            subtype = ty(s);
            dimension_expr = expr(e);
            "ArrayWithDimensionExpression"
        }
        Kind::Function(s, p) => {
            subtype = ty(s);
            params = Value::Array(p.iter().map(ty).collect());
            "Function"
        }
    };
    object([
        ("type", text("ctype")),
        ("tag", text(tag)),
        ("simple", text(&simple)),
        ("name", name),
        ("subtype", subtype),
        ("dimension", number(dimension)),
        ("dimension_expr", dimension_expr),
        ("params", params),
        ("const", Value::Bool(t.is_const)),
        ("restrict", Value::Bool(t.is_restrict)),
        ("enum_strict", Value::Bool(t.is_enum_strict)),
        ("bitflags", Value::Bool(t.is_bitflags)),
        ("safe_index", Value::Bool(t.is_safe_index)),
        ("align", number(t.forced_align)),
        ("packed", Value::Bool(t.is_packed)),
    ])
}
fn arg(a: &Arg) -> Value {
    match a {
        Arg::Int(n) => number(*n),
        Arg::Text(s) => text(s),
        Arg::Type(t) => ty(t),
        Arg::Expr(e) => expr(e),
        Arg::Exprs(a) => Value::Array(a.iter().map(expr).collect()),
        Arg::Ints(a) => object([
            ("type", text("ints")),
            (
                "value",
                Value::Array(a.iter().map(|n| number(*n)).collect()),
            ),
        ]),
        Arg::Bytes(a) => object([
            ("type", text("bytes")),
            (
                "value",
                Value::Array(a.iter().map(|n| number(i32::from(*n))).collect()),
            ),
        ]),
        Arg::Fields(fields) => object([
            ("type", text("fields")),
            (
                "value",
                Value::Array(
                    fields
                        .iter()
                        .map(|f| {
                            object([
                                ("name", text(&f.name)),
                                ("offset", number(f.offset)),
                                ("type", ty(&f.ty)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
        Arg::Region(r) => {
            let (tag, bank, fixed) = match r {
                Region::HighMem => ("HighMem", 0, 0),
                Region::Oam => ("Oam", 0, 0),
                Region::Ram => ("Ram", 0, 0),
                Region::Wram0 => ("Wram0", 0, 0),
                Region::WramX(b) => ("WramX", i32::from(*b), 0),
                Region::Rom => ("ProgramRom", 0, 0),
                Region::Fixed(n) => ("Fixed", 0, *n),
            };
            object([
                ("type", text("region")),
                ("tag", text(tag)),
                ("bank", number(bank)),
                ("fixed", number(fixed)),
            ])
        }
        Arg::Operand(o) => object([
            ("type", text("operand")),
            ("base", o.base.as_ref().map_or(Value::Null, |s| text(s))),
            ("offset", number(o.offset)),
            ("mode", text(&format!("{:?}", o.mode))),
            ("modifier", text(&format!("{:?}", o.modifier))),
        ]),
    }
}
pub fn expr(e: &Expr) -> Value {
    object([
        ("args", Value::Array(e.args.iter().map(arg).collect())),
        (
            "source",
            object([
                (
                    "filename",
                    text(
                        Path::new(&e.source.filename)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .as_ref(),
                    ),
                ),
                ("line", Value::Number(e.source.line as i64)),
                ("column", Value::Number(e.source.column as i64)),
            ]),
        ),
    ])
}
