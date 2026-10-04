use super::*;
use crate::codegen::analysis::{AggregateCopy, Call, Function};
impl Emitter {
    fn report_size(&mut self, ty: &CType) -> i32 {
        match &ty.kind {
            Kind::Simple(Simple::Void) => 0,
            Kind::Function(..) => 1,
            Kind::Array(element, dimension) => {
                self.report_size(element).max(1) * (*dimension).max(1)
            }
            Kind::ArrayExpression(element, dimension) => {
                let count = self.env.constant(dimension).value.max(1);
                self.report_size(element).max(1) * count
            }
            Kind::Struct(name) | Kind::Union(name) => self
                .env
                .aggregates
                .get(name)
                .map(|a| a.total_size)
                .filter(|n| *n >= 0)
                .unwrap_or(1),
            _ => self.env.size(&Expr::new(t::EMPTY, vec![]), ty),
        }
    }
    pub(super) fn report_call(
        &mut self,
        e: &Expr,
        callee: &str,
        bank: i32,
        kind: &str,
        thunk: bool,
        far: bool,
        args: &[&Expr],
        parameters: &[CType],
    ) {
        let actual = args.iter().map(|a| self.size(a)).collect();
        let expected = parameters.iter().map(|t| self.env.size(e, t)).collect();
        let caller = if self.current_function.is_empty() {
            "<global>"
        } else {
            &self.current_function
        };
        if let Some(edge) = self.analysis.calls.iter_mut().find(|c| {
            c.caller == caller
                && c.callee == callee
                && c.caller_bank == self.current_bank
                && c.callee_bank == bank
                && c.kind == kind
                && c.via_thunk == thunk
                && c.via_farcall == far
        }) {
            edge.count += 1;
            edge.actual_sizes = actual;
            edge.expected_sizes = expected;
            edge.source = e.source.to_string();
        } else {
            self.analysis.calls.push(Call {
                caller: caller.into(),
                callee: callee.into(),
                caller_bank: self.current_bank,
                callee_bank: bank,
                kind: kind.into(),
                via_thunk: thunk,
                via_farcall: far,
                count: 1,
                actual_sizes: actual,
                expected_sizes: expected,
                source: e.source.to_string(),
            });
        }
    }
    pub(super) fn report_copy(&mut self, e: &Expr, ty: &CType, size: i32, strategy: &str) {
        self.analysis.aggregate_copies.push(AggregateCopy {
            function: if self.current_function.is_empty() {
                "<global>".into()
            } else {
                self.current_function.clone()
            },
            ty: ty.without_const().to_string(),
            size,
            strategy: strategy.into(),
            source: e.source.to_string(),
        });
    }
    pub(super) fn report_cgb_write(&mut self, register: &str) {
        self.analysis.cgb_guarded_writes += 1;
        if !self
            .analysis
            .cgb_guarded_registers
            .iter()
            .any(|r| r == register)
        {
            self.analysis.cgb_guarded_registers.push(register.into());
        }
    }
    pub(super) fn finish_analysis(&mut self, rst: &[rst::Selection]) {
        for name in self.function_order.clone() {
            let f = self.env.functions[&name].clone();
            let param_sizes = f
                .parameters
                .iter()
                .map(|p| self.report_size(&p.ty))
                .collect::<Vec<_>>();
            let return_size = self.report_size(&f.return_type);
            if f.stack_call {
                for (i, size) in param_sizes.iter().enumerate() {
                    if *size != 1 && *size != 2 {
                        self.analysis.abi_issues.push(format!(
                            "stack ABI parameter must be 1 or 2 bytes: {name} param#{i} size={size}"
                        ));
                    }
                }
            }
            if return_size > 2 {
                self.analysis.abi_issues.push(format!(
                    "return size > 2 bytes is not ABI-safe on GB: {name} returnSize={return_size}"
                ));
            }
            self.analysis.functions.push(Function {
                name,
                bank: f.rom_bank,
                fixed_bank: f.fixed_bank,
                placement_order: f.placement_order,
                prototype: f.prototype,
                inline: f.inline,
                stack_call: f.stack_call,
                fast_call: f.fast_call,
                return_size,
                param_sizes,
            });
        }
        for c in &self.analysis.calls {
            for (i, expected) in c.expected_sizes.iter().enumerate() {
                let actual = c.actual_sizes.get(i).copied().unwrap_or(-1);
                if *expected > 0 && actual != *expected {
                    self.analysis.abi_issues.push(format!(
                        "arg-size mismatch: {} -> {} arg#{i} actual={actual} expected={expected}",
                        c.caller, c.callee
                    ));
                }
            }
        }
        self.analysis.functions.sort_by(|a, b| a.name.cmp(&b.name));
        self.analysis
            .calls
            .sort_by(|a, b| (&a.caller, &a.callee, &a.kind).cmp(&(&b.caller, &b.callee, &b.kind)));
        self.analysis
            .aggregate_copies
            .sort_by(|a, b| {
                // C#'s Windows reports put missing locations before drive-qualified
                // paths. POSIX paths start with '/', so keep that sentinel ordering
                // explicitly instead of letting the host's path prefix change it.
                (&a.function, !a.source.starts_with("<unknown>"), &a.source, &a.ty)
                    .cmp(&(&b.function, !b.source.starts_with("<unknown>"), &b.source, &b.ty))
            });
        self.analysis.cgb_guarded_registers.sort();
        self.analysis.rst_selections = rst.to_vec();
    }
}
