//! Native lowering of parser IR into the forms consumed by Game Boy code generation.
use crate::{
    ctype::{CType, Field, Kind, Simple},
    expr::{Arg, Expr, Position},
    tags as t,
    tokenizer::{Diagnostic, Severity},
};
use std::collections::{BTreeMap, BTreeSet};
mod environment;
mod expressions;
mod lints;
mod ranges;
mod statements;
use environment::Global;
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub known_cgb: Option<i32>,
    pub const_scalar_in_rom: bool,
}
#[derive(Clone, Debug)]
pub struct Output {
    pub tree: Expr,
    pub diagnostics: Vec<Diagnostic>,
}
fn node(tag: &str, args: Vec<Arg>, source: &Position) -> Expr {
    Expr::new(tag, args).with_source(source.clone())
}
fn sequence(items: Vec<Expr>) -> Expr {
    Expr::new(t::SEQUENCE, items.into_iter().map(Arg::from).collect())
}
fn children(e: &Expr) -> Vec<Expr> {
    e.children().into_iter().cloned().collect()
}
fn child(e: &Expr, n: usize) -> &Expr {
    e.child(n).expect("typed lowering IR child")
}
fn atom(e: &Expr) -> bool {
    e.is(t::INTEGER) || e.is(t::NAME)
}
fn scalar_size(ty: &CType) -> i32 {
    if *ty == CType::simple(Simple::UInt8) || *ty == CType::simple(Simple::Int8) {
        1
    } else if *ty == CType::simple(Simple::Void) {
        0
    } else {
        2
    }
}
fn sub_type(ty: &CType) -> Option<&CType> {
    match &ty.kind {
        Kind::Pointer(s)
        | Kind::Array(s, _)
        | Kind::ArrayExpression(s, _)
        | Kind::Function(s, _) => Some(s),
        _ => None,
    }
}
fn callable(ty: &CType) -> Option<(&CType, &[CType])> {
    match &ty.kind {
        Kind::Function(ret, p) => Some((ret, p)),
        Kind::Pointer(s) => {
            if let Kind::Function(ret, p) = &s.kind {
                Some((ret, p))
            } else {
                None
            }
        }
        _ => None,
    }
}
fn comparison(tag: &str) -> bool {
    matches!(
        tag,
        t::EQUAL
            | t::NOT_EQUAL
            | t::LESS_THAN
            | t::LESS_THAN_OR_EQUAL
            | t::GREATER_THAN
            | t::GREATER_THAN_OR_EQUAL
    )
}
fn binary(tag: &str) -> bool {
    comparison(tag)
        || matches!(
            tag,
            t::ADD
                | t::SUBTRACT
                | t::MULTIPLY
                | t::DIVIDE
                | t::MODULUS
                | t::BITWISE_AND
                | t::BITWISE_OR
                | t::BITWISE_XOR
                | t::SHIFT_LEFT
                | t::SHIFT_RIGHT
                | t::LOGICAL_AND
                | t::LOGICAL_OR
        )
}
pub fn lower(tree: &Expr, options: &Options) -> Output {
    let mut global = Global::default();
    global.build(tree);
    let mut diagnostics = std::mem::take(&mut global.diagnostics);
    if !tree.is(t::SEQUENCE) {
        return Output {
            tree: tree.clone(),
            diagnostics,
        };
    }
    let declarations = children(tree)
        .into_iter()
        .map(|decl| lower_top(&decl, &mut global, options, &mut diagnostics))
        .collect::<Vec<_>>();
    lints::unused_globals(&declarations, &mut diagnostics);
    diagnostics.append(&mut global.diagnostics);
    Output {
        tree: sequence(declarations).with_source(tree.source.clone()),
        diagnostics,
    }
}
fn lower_top(
    decl: &Expr,
    global: &mut Global,
    options: &Options,
    diagnostics: &mut Vec<Diagnostic>,
) -> Expr {
    if matches!(
        decl.tag(),
        Some(t::UNSAFE | t::STATIC | t::BANK | t::FIXED_BANK | t::FIXED_ORDER)
    ) {
        let index = if matches!(decl.tag(), Some(t::UNSAFE | t::STATIC)) {
            1
        } else {
            2
        };
        let mut result = decl.clone();
        if let Some(inner) = decl.child(index) {
            result.args[index] = Arg::from(lower_top(inner, global, options, diagnostics));
        }
        return result;
    }
    if matches!(decl.tag(), Some(t::FUNCTION | t::INLINE_FUNCTION)) {
        let (ty, name, params) = match (decl.ty(1), decl.text(2), decl.args.get(3)) {
            (Some(ty), Some(name), Some(Arg::Fields(p))) => (ty, name, p),
            _ => return decl.clone(),
        };
        let body = child(decl, if decl.args.len() == 6 { 5 } else { 4 });
        let mut ctx = Context::new(global, options, name, ty, params, &body.source);
        ctx.begin();
        let statements = ctx.statement(body);
        ctx.end();
        lints::unused_locals(&mut ctx);
        let mut all = ctx.temps.declarations.clone();
        all.extend(statements);
        let body = sequence(all).with_source(body.source.clone());
        diagnostics.append(&mut ctx.diagnostics);
        return node(
            decl.tag().unwrap(),
            vec![
                Arg::from(ty.clone()),
                Arg::from(name),
                Arg::Fields(params.clone()),
                Arg::Int(decl.int(4).unwrap_or(0)),
                Arg::from(body),
            ],
            &decl.source,
        );
    }
    decl.clone()
}
#[derive(Default)]
struct Temps {
    next_id: usize,
    free: BTreeMap<String, Vec<String>>,
    key_by_name: BTreeMap<String, String>,
    declarations: Vec<Expr>,
}
fn normalize_temp(ty: &CType) -> CType {
    let ty = ty.without_const();
    if ty.is_array() {
        CType::pointer(sub_type(&ty).unwrap().clone())
    } else if ty.is_aggregate() {
        CType::simple(Simple::UInt16)
    } else {
        ty
    }
}
impl Temps {
    fn acquire(&mut self, ty: &CType) -> (String, CType) {
        let ty = normalize_temp(ty);
        let key = ty.to_string();
        let free = self.free.entry(key.clone()).or_default();
        if let Some(name) = free.pop() {
            return (name, ty);
        }
        let name = format!(
            "{}{}",
            if scalar_size(&ty) == 1 {
                "__t8_"
            } else {
                "__t16_"
            },
            self.next_id
        );
        self.next_id += 1;
        self.key_by_name.insert(name.clone(), key);
        self.declarations.push(Expr::new(
            t::VARIABLE,
            vec![Arg::from(ty.clone()), Arg::from(name.clone())],
        ));
        (name, ty)
    }
    fn release(&mut self, name: String) {
        if let Some(key) = self.key_by_name.get(&name) {
            self.free.entry(key.clone()).or_default().push(name);
        }
    }
}
struct Context<'a> {
    global: &'a mut Global,
    options: &'a Options,
    locals: BTreeMap<String, CType>,
    params: BTreeMap<String, CType>,
    local_order: Vec<String>,
    param_order: Vec<String>,
    local_uses: BTreeMap<String, i32>,
    param_uses: BTreeMap<String, i32>,
    local_positions: BTreeMap<String, Position>,
    param_positions: BTreeMap<String, Position>,
    local_ranges: BTreeMap<String, [i32; 2]>,
    param_ranges: BTreeMap<String, [i32; 2]>,
    overrides: Vec<BTreeMap<String, [i32; 2]>>,
    function: String,
    return_type: CType,
    unsafe_depth: usize,
    temps: Temps,
    lifetimes: Vec<Vec<String>>,
    diagnostics: Vec<Diagnostic>,
}
impl<'a> Context<'a> {
    fn new(
        global: &'a mut Global,
        options: &'a Options,
        name: &str,
        return_type: &CType,
        params: &[Field],
        source: &Position,
    ) -> Self {
        let mut result = Self {
            global,
            options,
            locals: BTreeMap::new(),
            params: BTreeMap::new(),
            local_order: Vec::new(),
            param_order: Vec::new(),
            local_uses: BTreeMap::new(),
            param_uses: BTreeMap::new(),
            local_positions: BTreeMap::new(),
            param_positions: BTreeMap::new(),
            local_ranges: BTreeMap::new(),
            param_ranges: BTreeMap::new(),
            overrides: Vec::new(),
            function: name.into(),
            return_type: return_type.clone(),
            unsafe_depth: 0,
            temps: Temps::default(),
            lifetimes: Vec::new(),
            diagnostics: Vec::new(),
        };
        for p in params {
            if !result.params.contains_key(&p.name) {
                result.param_order.push(p.name.clone());
                result.params.insert(p.name.clone(), p.ty.clone());
                result.param_uses.insert(p.name.clone(), 0);
                result
                    .param_positions
                    .insert(p.name.clone(), source.clone());
            }
        }
        result
    }
    fn diagnostic(&mut self, severity: Severity, source: &Position, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(severity, source.clone(), message.into(), 0));
    }
    fn warning(&mut self, source: &Position, message: impl Into<String>) {
        self.diagnostic(Severity::Warning, source, message);
    }
    fn error(&mut self, source: &Position, message: impl Into<String>) {
        self.diagnostic(Severity::Error, source, message);
    }
    fn begin(&mut self) {
        self.lifetimes.push(Vec::new())
    }
    fn end(&mut self) {
        if let Some(names) = self.lifetimes.pop() {
            for name in names.into_iter().rev() {
                self.temps.release(name)
            }
        }
    }
    fn temp(&mut self, ty: &CType) -> String {
        let (name, ty) = self.temps.acquire(ty);
        self.register_local(&name, &ty, None, None);
        if let Some(lifetime) = self.lifetimes.last_mut() {
            lifetime.push(name.clone())
        }
        name
    }
    fn register_local(
        &mut self,
        name: &str,
        ty: &CType,
        range: Option<[i32; 2]>,
        source: Option<&Position>,
    ) {
        if !self.locals.contains_key(name) {
            self.locals.insert(name.into(), ty.clone());
            self.local_order.push(name.into());
        }
        self.local_uses.entry(name.into()).or_insert(0);
        if let Some(range) = range {
            self.local_ranges.entry(name.into()).or_insert(range);
        }
        if let Some(source) = source {
            self.local_positions
                .entry(name.into())
                .or_insert(source.clone());
        }
    }
    fn use_name(&mut self, name: &str) {
        if let Some(n) = self.local_uses.get_mut(name) {
            *n += 1
        }
        if let Some(n) = self.param_uses.get_mut(name) {
            *n += 1
        }
    }
    fn name_type(&self, name: &str) -> CType {
        self.locals
            .get(name)
            .or_else(|| self.params.get(name))
            .or_else(|| self.global.globals.get(name))
            .or_else(|| self.global.constant_types.get(name))
            .cloned()
            .unwrap_or_else(|| CType::simple(Simple::UInt8))
    }
    fn infer(&self, expr: &Expr) -> CType {
        let tag = expr.tag().unwrap_or("");
        let u8_type = || CType::simple(Simple::UInt8);
        match tag {
            t::INTEGER => {
                let n = expr.int(1).unwrap_or(0);
                CType::simple(if n < 0 {
                    Simple::Int16
                } else if n > 255 {
                    Simple::UInt16
                } else {
                    Simple::UInt8
                })
            }
            t::NAME => self.name_type(expr.text(1).unwrap_or("")),
            t::CAST => expr.ty(1).unwrap().clone(),
            t::SLICE => self.infer(child(expr, 1)),
            t::LOAD => {
                let ty = self.infer(child(expr, 1));
                if ty.is_pointer() {
                    sub_type(&ty).unwrap().clone()
                } else {
                    u8_type()
                }
            }
            t::ADDRESS_OF => CType::pointer(self.infer(child(expr, 1))),
            t::INDEX => {
                let ty = self.infer(child(expr, 1));
                if ty.is_pointer() || ty.is_array() {
                    sub_type(&ty).unwrap().clone()
                } else {
                    u8_type()
                }
            }
            t::FIELD => self
                .global
                .field_type(&self.infer(child(expr, 1)), expr.text(2).unwrap())
                .unwrap_or_else(u8_type),
            t::CALL => {
                let function = child(expr, 1);
                let ty = self.infer(function);
                if let Some((ret, _)) = callable(&ty) {
                    ret.clone()
                } else if function.is(t::NAME) {
                    self.global
                        .function_returns
                        .get(function.text(1).unwrap())
                        .cloned()
                        .unwrap_or_else(u8_type)
                } else {
                    u8_type()
                }
            }
            t::CONDITIONAL => {
                let mut yes = self.infer(child(expr, 2));
                let mut no = self.infer(child(expr, 3));
                if yes.is_array() {
                    yes = CType::pointer(sub_type(&yes).unwrap().clone())
                }
                if no.is_array() {
                    no = CType::pointer(sub_type(&no).unwrap().clone())
                }
                if yes.is_pointer() {
                    yes
                } else if no.is_pointer() {
                    no
                } else if yes.is_integer() || no.is_integer() {
                    CType::simple(if yes.is_signed() || no.is_signed() {
                        Simple::Int16
                    } else {
                        Simple::UInt16
                    })
                } else {
                    u8_type()
                }
            }
            t::PRE_INCREMENT
            | t::POST_INCREMENT
            | t::PRE_DECREMENT
            | t::POST_DECREMENT
            | t::BITWISE_NOT => self.infer(child(expr, 1)),
            t::LOGICAL_NOT => u8_type(),
            _ if binary(tag) => {
                if comparison(tag) || matches!(tag, t::LOGICAL_AND | t::LOGICAL_OR) {
                    return u8_type();
                }
                let left = self.infer(child(expr, 1));
                let right = self.infer(child(expr, 2));
                if matches!(tag, t::ADD | t::SUBTRACT) {
                    if left.is_pointer() && right.is_integer() {
                        return left;
                    }
                    if tag == t::ADD && right.is_pointer() && left.is_integer() {
                        return right;
                    }
                    if tag == t::SUBTRACT && left.is_pointer() && right.is_pointer() {
                        return CType::simple(Simple::UInt16);
                    }
                }
                if left.is_integer() || right.is_integer() {
                    CType::simple(if left.is_signed() || right.is_signed() {
                        Simple::Int16
                    } else {
                        Simple::UInt16
                    })
                } else if scalar_size(&left) == 2 || scalar_size(&right) == 2 {
                    CType::simple(Simple::UInt16)
                } else {
                    u8_type()
                }
            }
            _ => u8_type(),
        }
    }
}
