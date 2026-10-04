use super::*;
impl Emitter {
    pub(super) fn inline_call(
        &mut self,
        e: &Expr,
        name: &str,
        info: &FunctionInfo,
        args: &[&Expr],
    ) {
        self.env.begin_scope();
        let end = self.unique("inline_end");
        self.inline_returns.push(end.clone());
        if let Some(symbol) = &info.return_symbol {
            self.inline_destinations.push(symbol.value);
        }
        for (i, (parameter, arg)) in info.parameters.iter().zip(args).enumerate() {
            let size = self.env.size(e, &parameter.ty);
            let address = self.env.allocate_preferred(size, 1, &[0, 1, 2]);
            self.env.declare(
                e,
                Symbol {
                    tag: SymbolTag::Local,
                    value: address,
                    ty: parameter.ty.clone(),
                    name: parameter.name.clone(),
                    wram_bank: 0,
                },
            );
            if parameter.ty.is_aggregate() {
                self.aggregate_argument(e, name, i, arg, &parameter.ty, Some(address), None);
            } else if size == 1 {
                self.into_a(arg);
                self.store(memory_operand(address));
            } else if size == 2 {
                self.into_hl(arg);
                self.asm("LD_A_L");
                self.store(memory_operand(address));
                self.asm("LD_A_H");
                self.store(memory_operand(address + 1));
            } else {
                self.env.error(&e.source, "Argument size not supported");
            }
        }
        let saved_type = self.return_type.clone();
        self.return_type = info.return_type.clone();
        if let Some(body) = &info.body {
            self.statement(body);
        }
        self.return_type = saved_type;
        self.label(&end);
        self.inline_returns.pop();
        if let Some(symbol) = &info.return_symbol {
            self.inline_destinations.pop();
            self.last_sret_address = Some(symbol.value);
        }
        self.env.end_scope();
    }
}
