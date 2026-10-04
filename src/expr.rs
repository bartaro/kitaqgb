use crate::{
    asm::{Operand, Region},
    ctype::{CType, Field},
    tags,
};
fn format_integer(n: i32) -> String {
    if n < 128 {
        n.to_string()
    } else {
        format!("${n:X}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub filename: String,
    pub line: usize,
    pub column: usize,
}
impl Default for Position {
    fn default() -> Self {
        Self {
            filename: "<unknown>".into(),
            line: 0,
            column: 0,
        }
    }
}
impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (line {}, column {})",
            self.filename,
            self.line + 1,
            self.column + 1
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Int(i32),
    Text(String),
    Region(Region),
    Type(CType),
    Fields(Vec<Field>),
    Expr(Box<Expr>),
    Exprs(Vec<Expr>),
    Operand(Operand),
    Ints(Vec<i32>),
    Bytes(Vec<u8>),
}
impl From<i32> for Arg {
    fn from(v: i32) -> Self {
        Self::Int(v)
    }
}
impl From<String> for Arg {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<&str> for Arg {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<Region> for Arg {
    fn from(v: Region) -> Self {
        Self::Region(v)
    }
}
impl From<CType> for Arg {
    fn from(v: CType) -> Self {
        Self::Type(v)
    }
}
impl From<Expr> for Arg {
    fn from(v: Expr) -> Self {
        Self::Expr(Box::new(v))
    }
}
impl From<Vec<Expr>> for Arg {
    fn from(v: Vec<Expr>) -> Self {
        Self::Exprs(v)
    }
}
impl From<Vec<Field>> for Arg {
    fn from(v: Vec<Field>) -> Self {
        Self::Fields(v)
    }
}
impl From<Vec<i32>> for Arg {
    fn from(v: Vec<i32>) -> Self {
        Self::Ints(v)
    }
}
impl From<Vec<u8>> for Arg {
    fn from(v: Vec<u8>) -> Self {
        Self::Bytes(v)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub args: Vec<Arg>,
    pub source: Position,
}
impl Expr {
    pub fn new(tag: &str, mut args: Vec<Arg>) -> Self {
        args.insert(0, Arg::Text(tag.into()));
        Self {
            args,
            source: Position::default(),
        }
    }
    pub fn with_source(&self, source: Position) -> Self {
        Self {
            source,
            ..self.clone()
        }
    }
    pub fn tag(&self) -> Option<&str> {
        match self.args.first() {
            Some(Arg::Text(t)) => Some(t),
            _ => None,
        }
    }
    pub fn is(&self, tag: &str) -> bool {
        self.tag() == Some(tag)
    }
    pub fn matches(&self, tag: &str, arity: usize) -> bool {
        self.is(tag) && self.args.len() == arity + 1
    }
    pub fn asm(mnemonic: &str, operand: Operand) -> Self {
        Self::new(
            tags::ASM,
            vec![Arg::Text(mnemonic.into()), Arg::Operand(operand)],
        )
    }
    pub fn asm_parts(&self) -> Option<(&str, &Operand)> {
        if !self.matches(tags::ASM, 2) {
            return None;
        }
        match (&self.args[1], &self.args[2]) {
            (Arg::Text(m), Arg::Operand(o)) => Some((m, o)),
            _ => None,
        }
    }
    pub fn readonly_parts(&self) -> Option<(&str, &[u8], &[Expr])> {
        if !self.is(tags::READONLY_DATA) {
            return None;
        }
        match self.args.as_slice() {
            [_, Arg::Text(n), Arg::Bytes(b)] => Some((n, b, &[])),
            [_, Arg::Text(n), Arg::Bytes(b), Arg::Exprs(r)] => Some((n, b, r)),
            _ => None,
        }
    }
    pub fn text(&self, index: usize) -> Option<&str> {
        match self.args.get(index) {
            Some(Arg::Text(s)) => Some(s),
            _ => None,
        }
    }
    pub fn int(&self, index: usize) -> Option<i32> {
        match self.args.get(index) {
            Some(Arg::Int(n)) => Some(*n),
            _ => None,
        }
    }
    pub fn child(&self, index: usize) -> Option<&Expr> {
        if let Some(Arg::Expr(e)) = self.args.get(index) {
            Some(e)
        } else {
            None
        }
    }
    pub fn ty(&self, index: usize) -> Option<&CType> {
        if let Some(Arg::Type(t)) = self.args.get(index) {
            Some(t)
        } else {
            None
        }
    }
    pub fn children(&self) -> Vec<&Expr> {
        self.args
            .iter()
            .skip(1)
            .filter_map(|a| {
                if let Arg::Expr(e) = a {
                    Some(e.as_ref())
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn show(&self) -> String {
        self.tree().show(false)
    }
    pub fn show_multiline(&self) -> String {
        self.tree().show(true)
    }
    fn tree(&self) -> Tree {
        Tree::List(
            self.args
                .iter()
                .map(|a| match a {
                    Arg::Int(n) => Tree::Text(format_integer(*n)),
                    Arg::Text(s) => Tree::Text(s.clone()),
                    Arg::Region(r) => Tree::Text(r.to_string()),
                    Arg::Type(t) => Tree::Text(format!("<{t}>")),
                    Arg::Fields(_) => Tree::Text("<fields>".into()),
                    Arg::Operand(o) => Tree::Text(o.to_string()),
                    Arg::Expr(e) => e.tree(),
                    Arg::Exprs(e) => Tree::List(e.iter().map(Self::tree).collect()),
                    Arg::Ints(n) => Tree::Text(format!(
                        "{{ {} }}",
                        n.iter()
                            .map(|n| format_integer(*n))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    Arg::Bytes(b) => Tree::Text(format!(
                        "{{ {} }}",
                        b.iter()
                            .map(|n| format!("{n:02X}"))
                            .collect::<Vec<_>>()
                            .join(", 0x")
                    )),
                })
                .collect(),
        )
    }
}
enum Tree {
    Text(String),
    List(Vec<Tree>),
}
impl Tree {
    fn show(&self, multi: bool) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::List(a) => {
                let small = a.len() <= 2 || !a.iter().any(|t| matches!(t, Self::List(_)));
                let ss = a.iter().map(|t| t.show(multi)).collect::<Vec<_>>();
                if small || !multi {
                    format!("({})", ss.join(" "))
                } else {
                    format!(
                        "({})",
                        ss.iter()
                            .map(|s| s.replace('\n', "\n    "))
                            .collect::<Vec<_>>()
                            .join("\n    ")
                    )
                }
            }
        }
    }
}
