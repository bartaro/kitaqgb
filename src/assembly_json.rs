//! Lossless assembly fixture format used for differential tests and embedders.
use crate::{
    asm::{AddressMode, Modifier, Operand, Region},
    assembler::Options,
    expr::{Arg, Expr, Position},
    json::Value,
};
use std::collections::BTreeMap;
fn object(v: &Value) -> Result<&BTreeMap<String, Value>, String> {
    if let Value::Object(o) = v {
        Ok(o)
    } else {
        Err("expected JSON object".into())
    }
}
fn field<'a>(o: &'a BTreeMap<String, Value>, name: &str) -> Result<&'a Value, String> {
    o.get(name).ok_or_else(|| format!("missing '{name}'"))
}
fn string(v: &Value) -> Result<&str, String> {
    if let Value::String(s) = v {
        Ok(s)
    } else {
        Err("expected JSON string".into())
    }
}
fn integer(v: &Value) -> Result<i32, String> {
    if let Value::Number(n) = v {
        i32::try_from(*n).map_err(|_| "integer out of range".into())
    } else {
        Err("expected JSON integer".into())
    }
}
fn array(v: &Value) -> Result<&[Value], String> {
    if let Value::Array(a) = v {
        Ok(a)
    } else {
        Err("expected JSON array".into())
    }
}
fn arg(v: &Value) -> Result<Arg, String> {
    match v {
        Value::String(s) => Ok(Arg::Text(s.clone())),
        Value::Number(_) => Ok(Arg::Int(integer(v)?)),
        Value::Array(a) => Ok(Arg::Exprs(a.iter().map(expr).collect::<Result<_, _>>()?)),
        Value::Object(o) if o.contains_key("args") => Ok(Arg::Expr(Box::new(expr(v)?))),
        Value::Object(o) => match string(field(o, "type")?)? {
            "bytes" => Ok(Arg::Bytes(
                array(field(o, "value")?)?
                    .iter()
                    .map(|v| u8::try_from(integer(v)?).map_err(|_| "byte out of range".into()))
                    .collect::<Result<_, String>>()?,
            )),
            "region" => {
                let bank = integer(field(o, "bank")?)? as u8;
                let r = match string(field(o, "tag")?)? {
                    "HighMem" => Region::HighMem,
                    "Oam" => Region::Oam,
                    "Ram" => Region::Ram,
                    "Wram0" => Region::Wram0,
                    "WramX" => Region::WramX(bank),
                    "ProgramRom" => Region::Rom,
                    "Fixed" => Region::Fixed(integer(field(o, "fixed")?)?),
                    s => return Err(format!("unknown region: {s}")),
                };
                Ok(Arg::Region(r))
            }
            "operand" => {
                let mode = match string(field(o, "mode")?)? {
                    "Implicit" => AddressMode::Implicit,
                    "Immediate" => AddressMode::Immediate,
                    "Immediate16" => AddressMode::Immediate16,
                    "HighMem" => AddressMode::HighMem,
                    "HighMemX" => AddressMode::HighMemX,
                    "Absolute" => AddressMode::Absolute,
                    "AbsoluteX" => AddressMode::AbsoluteX,
                    "AbsoluteY" => AddressMode::AbsoluteY,
                    "Indirect" => AddressMode::Indirect,
                    "IndirectX" => AddressMode::IndirectX,
                    "IndirectY" => AddressMode::IndirectY,
                    "Relative" => AddressMode::Relative,
                    s => return Err(format!("unknown address mode: {s}")),
                };
                let modifier = match string(field(o, "modifier")?)? {
                    "None" => Modifier::None,
                    "LowByte" => Modifier::LowByte,
                    "HighByte" => Modifier::HighByte,
                    "Bank" => Modifier::Bank,
                    s => return Err(format!("unknown modifier: {s}")),
                };
                let base = match field(o, "base")? {
                    Value::Null => None,
                    v => Some(string(v)?.into()),
                };
                Ok(Arg::Operand(Operand {
                    base,
                    offset: integer(field(o, "offset")?)?,
                    mode,
                    modifier,
                    comment: None,
                }))
            }
            s => Err(format!("unknown argument type: {s}")),
        },
        _ => Err("unsupported assembly argument".into()),
    }
}
fn expr(v: &Value) -> Result<Expr, String> {
    let o = object(v)?;
    let args = array(field(o, "args")?)?
        .iter()
        .map(arg)
        .collect::<Result<Vec<_>, _>>()?;
    let mut source = Position::default();
    if let Some(p) = o.get("source") {
        let p = object(p)?;
        source.filename = match field(p, "filename")? {
            Value::Null => "<unknown>".into(),
            s => string(s)?.into(),
        };
        source.line = integer(field(p, "line")?)?.max(0) as usize;
        source.column = integer(field(p, "column")?)?.max(0) as usize;
    }
    Ok(Expr { args, source })
}
pub fn parse(text: &str) -> Result<(Vec<Expr>, Options), String> {
    let v = Value::parse(text)?;
    let o = object(&v)?;
    let stack_top = integer(field(o, "stack_top")?)?;
    let Value::Bool(header_logo) = field(o, "header_logo")? else {
        return Err("expected header_logo boolean".into());
    };
    let assembly = array(field(o, "assembly")?)?
        .iter()
        .map(expr)
        .collect::<Result<_, _>>()?;
    Ok((
        assembly,
        Options {
            stack_top,
            header_logo: *header_logo,
        },
    ))
}
