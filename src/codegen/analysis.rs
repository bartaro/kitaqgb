//! ABI and source-level code-generation evidence.
use crate::{codegen::rst::Selection, json::Value};
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub bank: i32,
    pub fixed_bank: bool,
    pub placement_order: Option<i32>,
    pub prototype: bool,
    pub inline: bool,
    pub stack_call: bool,
    pub fast_call: bool,
    pub return_size: i32,
    pub param_sizes: Vec<i32>,
}
#[derive(Clone, Debug)]
pub struct Call {
    pub caller: String,
    pub callee: String,
    pub caller_bank: i32,
    pub callee_bank: i32,
    pub kind: String,
    pub via_thunk: bool,
    pub via_farcall: bool,
    pub count: i32,
    pub actual_sizes: Vec<i32>,
    pub expected_sizes: Vec<i32>,
    pub source: String,
}
#[derive(Clone, Debug)]
pub struct AggregateCopy {
    pub function: String,
    pub ty: String,
    pub size: i32,
    pub strategy: String,
    pub source: String,
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub functions: Vec<Function>,
    pub calls: Vec<Call>,
    pub aggregate_copies: Vec<AggregateCopy>,
    pub rst_selections: Vec<Selection>,
    pub abi_issues: Vec<String>,
    pub cgb_runtime_checks: i32,
    pub cgb_guarded_writes: i32,
    pub cgb_guarded_registers: Vec<String>,
}
fn obj<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn s(x: &str) -> Value {
    Value::String(x.into())
}
fn n(x: i32) -> Value {
    Value::Number(i64::from(x))
}
fn b(x: bool) -> Value {
    Value::Bool(x)
}
fn ints(x: &[i32]) -> Value {
    Value::Array(x.iter().map(|x| n(*x)).collect())
}
impl Report {
    pub fn json(&self) -> Value {
        obj([
            (
                "functions",
                Value::Array(
                    self.functions
                        .iter()
                        .map(|f| {
                            obj([
                                ("name", s(&f.name)),
                                ("bank", n(f.bank)),
                                ("fixed_bank", b(f.fixed_bank)),
                                ("placement_order", n(f.placement_order.unwrap_or(i32::MAX))),
                                ("fixed_order", b(f.placement_order.is_some())),
                                ("prototype", b(f.prototype)),
                                ("inline", b(f.inline)),
                                ("stack_call", b(f.stack_call)),
                                ("fast_call", b(f.fast_call)),
                                ("return_size", n(f.return_size)),
                                ("param_sizes", ints(&f.param_sizes)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "calls",
                Value::Array(
                    self.calls
                        .iter()
                        .map(|c| {
                            obj([
                                ("caller", s(&c.caller)),
                                ("callee", s(&c.callee)),
                                ("caller_bank", n(c.caller_bank)),
                                ("callee_bank", n(c.callee_bank)),
                                ("kind", s(&c.kind)),
                                ("via_thunk", b(c.via_thunk)),
                                ("via_farcall", b(c.via_farcall)),
                                ("count", n(c.count)),
                                ("actual_sizes", ints(&c.actual_sizes)),
                                ("expected_sizes", ints(&c.expected_sizes)),
                                ("source", s(&c.source)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "aggregate_copies",
                Value::Array(
                    self.aggregate_copies
                        .iter()
                        .map(|c| {
                            obj([
                                ("function", s(&c.function)),
                                ("type", s(&c.ty)),
                                ("size", n(c.size)),
                                ("strategy", s(&c.strategy)),
                                ("source", s(&c.source)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "rst_selections",
                Value::Array(
                    self.rst_selections
                        .iter()
                        .map(|r| {
                            obj([
                                ("vector", n(r.vector)),
                                ("target", s(&r.target)),
                                ("calls", n(r.calls)),
                                ("net_bytes", n(r.net_bytes)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "abi_issues",
                Value::Array(self.abi_issues.iter().map(|x| s(x)).collect()),
            ),
            ("cgb_runtime_checks", n(self.cgb_runtime_checks)),
            ("cgb_guarded_writes", n(self.cgb_guarded_writes)),
            (
                "cgb_guarded_registers",
                Value::Array(self.cgb_guarded_registers.iter().map(|x| s(x)).collect()),
            ),
        ])
    }
}
