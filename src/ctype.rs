use crate::expr::Expr;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Simple {
    Implied,
    Void,
    UInt8,
    Int8,
    UInt16,
    Int16,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Simple(Simple),
    Pointer(Box<CType>),
    Struct(String),
    Union(String),
    Enum(String),
    Array(Box<CType>, i32),
    ArrayExpression(Box<CType>, Box<Expr>),
    Function(Box<CType>, Vec<CType>),
}
#[derive(Clone, Debug)]
pub struct CType {
    pub kind: Kind,
    pub is_const: bool,
    pub is_restrict: bool,
    pub is_enum_strict: bool,
    pub is_bitflags: bool,
    pub is_safe_index: bool,
    pub forced_align: i32,
    pub is_packed: bool,
}
impl CType {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            is_const: false,
            is_restrict: false,
            is_enum_strict: false,
            is_bitflags: false,
            is_safe_index: false,
            forced_align: 0,
            is_packed: false,
        }
    }
    pub fn simple(t: Simple) -> Self {
        Self::new(Kind::Simple(t))
    }
    pub fn pointer(sub: Self) -> Self {
        Self::new(Kind::Pointer(Box::new(sub)))
    }
    pub fn array(sub: Self, n: i32) -> Self {
        Self::new(Kind::Array(Box::new(sub), n))
    }
    pub fn array_expression(sub: Self, n: Expr) -> Self {
        Self::new(Kind::ArrayExpression(Box::new(sub), Box::new(n)))
    }
    pub fn function(ret: Self, params: Vec<Self>) -> Self {
        Self::new(Kind::Function(Box::new(ret), params))
    }
    /// Preserve the source WithoutConst method, including its qualifier resets.
    pub fn without_const(&self) -> Self {
        if !self.is_const {
            return self.clone();
        }
        Self {
            is_const: false,
            is_restrict: false,
            is_enum_strict: false,
            is_safe_index: false,
            ..self.clone()
        }
    }
    pub fn is_simple(&self) -> bool {
        matches!(self.kind, Kind::Simple(_))
    }
    pub fn is_pointer(&self) -> bool {
        matches!(self.kind, Kind::Pointer(_))
    }
    pub fn is_aggregate(&self) -> bool {
        matches!(self.kind, Kind::Struct(_) | Kind::Union(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self.kind, Kind::Array(..) | Kind::ArrayExpression(..))
    }
    pub fn is_enum(&self) -> bool {
        matches!(self.kind, Kind::Enum(_))
    }
    pub fn is_function(&self) -> bool {
        matches!(self.kind, Kind::Function(..))
    }
    pub fn is_integer(&self) -> bool {
        matches!(
            self.kind,
            Kind::Simple(Simple::UInt8 | Simple::Int8 | Simple::UInt16 | Simple::Int16)
                | Kind::Enum(_)
        )
    }
    pub fn is_unsigned(&self) -> bool {
        matches!(self.kind, Kind::Simple(Simple::UInt8 | Simple::UInt16))
    }
    pub fn is_signed(&self) -> bool {
        matches!(self.kind, Kind::Simple(Simple::Int8 | Simple::Int16))
    }
}
impl PartialEq for CType {
    fn eq(&self, b: &Self) -> bool {
        if self.is_const != b.is_const
            || self.is_restrict != b.is_restrict
            || self.is_bitflags != b.is_bitflags
            || self.is_safe_index != b.is_safe_index
            || self.forced_align != b.forced_align
            || self.is_packed != b.is_packed
        {
            return false;
        }
        match (&self.kind, &b.kind) {
            (Kind::Simple(a), Kind::Simple(b)) => a == b,
            (Kind::Pointer(a), Kind::Pointer(b)) => a == b,
            // Source type equality intentionally ignores array dimension values.
            (Kind::Array(a, _), Kind::Array(b, _))
            | (Kind::ArrayExpression(a, _), Kind::ArrayExpression(b, _)) => a == b,
            (Kind::Struct(a), Kind::Struct(b)) | (Kind::Union(a), Kind::Union(b)) => a == b,
            (Kind::Enum(a), Kind::Enum(n)) => a == n && self.is_enum_strict == b.is_enum_strict,
            (Kind::Function(a, pa), Kind::Function(b, pb)) => a == b && pa == pb,
            _ => false,
        }
    }
}
impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = match &self.kind {
            Kind::Simple(t) => format!("{t:?}").to_lowercase(),
            Kind::Pointer(t) => format!("{t}*"),
            Kind::Struct(n) => format!("struct {n}"),
            Kind::Union(n) => format!("union {n}"),
            Kind::Enum(n) => {
                if n.is_empty() {
                    "enum".into()
                } else {
                    format!("enum {n}")
                }
            }
            Kind::Array(t, n) => format!("{t}[{n}]"),
            Kind::ArrayExpression(t, n) => format!("{t}[{}]", n.show()),
            Kind::Function(t, p) => format!(
                "fn({})->{t}",
                p.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        };
        if self.is_bitflags && matches!(self.kind, Kind::Simple(_) | Kind::Enum(_)) {
            s = format!("__bitflags {s}");
        }
        if self.is_enum_strict && self.is_enum() {
            s = format!("__enum_strict {s}");
        }
        if self.is_pointer() {
            if self.is_restrict {
                s.push_str(" __restrict");
            }
            if self.is_const {
                s.push_str(" const");
            }
        } else if self.is_const {
            s = format!("const {s}");
        }
        write!(f, "{s}")
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub ty: CType,
    pub name: String,
    pub offset: i32,
}
impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Field: {}, {}, {}", self.offset, self.ty, self.name)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregateLayout {
    Struct,
    Union,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Aggregate {
    pub layout: AggregateLayout,
    pub total_size: i32,
    pub alignment: i32,
    pub is_packed: bool,
    pub fields: Vec<Field>,
}
