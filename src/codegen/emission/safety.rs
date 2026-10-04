use super::*;
impl Emitter {
    fn stack_check(&mut self, bytes: i32) {
        if !self.env.options.check_stack || self.unsafe_depth > 0 {
            return;
        }
        self.check_trap();
        self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
        let bytes = bytes.max(0);
        if bytes != 0 {
            self.word("LD_DE_IMM", (65536 - (bytes & 65535)) & 65535);
            self.asm("ADD_HL_DE");
        }
        let limit = if self.env.options.stack_reserve > 0 {
            self.env.options.stack_top - self.env.options.stack_reserve
        } else {
            0xc000
        };
        let ok = self.unique("kq_sp_ok");
        let trap = self.check_trap();
        self.word("LD_DE_IMM", limit & 65535);
        self.asm("LD_A_H");
        self.asm("CP_D");
        self.op("JP_C", trap.clone());
        self.op("JP_NZ", ok.clone());
        self.asm("LD_A_L");
        self.asm("CP_E");
        self.op("JP_C", trap);
        self.label(&ok);
    }
    pub(super) fn function_stack_check(&mut self, info: &FunctionInfo, e: &Expr, bytes: i32) {
        if !self.env.options.check_stack || self.unsafe_depth > 0 {
            return;
        }
        let size = if info.fast_call {
            info.parameter_symbols
                .as_ref()
                .and_then(|p| p.first())
                .map(|p| self.env.size(e, &p.ty))
                .unwrap_or(0)
        } else {
            0
        };
        match size {
            1 => self.asm("LD_B_A"),
            2 => {
                self.asm("LD_B_H");
                self.asm("LD_C_L");
            }
            _ => {}
        }
        self.stack_check(bytes);
        match size {
            1 => self.asm("LD_A_B"),
            2 => {
                self.asm("LD_H_B");
                self.asm("LD_L_C");
            }
            _ => {}
        }
    }
    pub(super) fn fixed_array_bytes(&mut self, e: &Expr) -> Option<(String, i32)> {
        let mut value = e;
        while value.is(t::CAST) {
            value = child(value, 2);
        }
        if !value.is(t::NAME) {
            return None;
        }
        let name = value.text(1)?;
        let symbol = self.env.find(name)?.clone();
        if matches!(symbol.ty.kind,Kind::Array(_,n) if n>0) {
            Some((name.into(), self.env.size(value, &symbol.ty)))
        } else {
            None
        }
    }
    pub(super) fn copy_length_check(&mut self, limit: i32, byte: bool) {
        if limit < 0 || byte && limit >= 255 {
            return;
        }
        let ok = self.unique(if byte { "kq_chk8_ok" } else { "kq_chk_ok" });
        if !byte {
            self.asm("LD_A_H");
            self.immediate("CP_IMM", (limit >> 8) & 255);
            self.op("JP_C", ok.clone());
            let trap = self.check_trap();
            self.op("JP_NZ", trap);
            self.asm("LD_A_L");
        }
        self.immediate("CP_IMM", limit & 255);
        self.op("JP_C", ok.clone());
        self.op("JP_Z", ok.clone());
        let trap = self.check_trap();
        self.op("JP", trap);
        self.label(&ok);
    }
    pub(super) fn direct_bank_check(&mut self, bank: i32, preserve_a: bool) {
        if !self.env.options.check_bank_calls || self.unsafe_depth > 0 || bank == 0 {
            return;
        }
        let ok = self.unique("kq_bank_ok");
        if preserve_a {
            self.asm("PUSH_AF");
        }
        self.load(memory_operand(0xff82));
        self.immediate("CP_IMM", bank & 255);
        self.op("JP_Z", ok.clone());
        let trap = self.check_trap();
        self.op("JP", trap);
        self.label(&ok);
        if preserve_a {
            self.asm("POP_AF");
        }
    }
    pub(super) fn indirect_bank_check(&mut self, expression: bool) {
        if !self.env.options.check_bank_calls
            || self.unsafe_depth > 0
            || !self.env.functions.values().any(|f| f.rom_bank >= 2)
        {
            return;
        }
        let ok = self.unique(if expression {
            "icall_expr_bank0_ok"
        } else {
            "icall_bank0_ok"
        });
        self.asm("LD_A_H");
        self.immediate("CP_IMM", 0x40);
        self.op("JP_C", ok.clone());
        self.immediate("CP_IMM", 0x80);
        self.op("JP_NC", ok.clone());
        let trap = self.check_trap();
        self.op("JP", trap);
        self.label(&ok);
    }
    pub(super) fn check_trap(&mut self) -> Operand {
        if self.check_trap_label.is_none() {
            self.check_trap_label = Some(self.unique("kq_trap_check"));
        }
        self.check_trap_label.clone().unwrap()
    }
    pub(super) fn bounds_check(&mut self, base: &Expr, index: &Expr) {
        if !self.env.options.check_bounds || self.unsafe_depth > 0 {
            return;
        }
        let mut uncast = base;
        while uncast.is(t::CAST) {
            uncast = child(uncast, 2);
        }
        let ty = if uncast.is(t::NAME) {
            self.env
                .find(uncast.text(1).unwrap())
                .map(|s| s.ty.clone())
                .filter(|ty| ty.is_array())
        } else {
            None
        }
        .unwrap_or_else(|| self.env.type_of(base));
        let ty = self.env.resolve_dimension(&ty);
        let Kind::Array(_, len) = ty.kind else {
            return;
        };
        if len <= 0 {
            return;
        }
        if index.is(t::INTEGER) && index.int(1).is_some_and(|i| i >= 0 && i < len) {
            return;
        }
        if self.env.type_of(index).is_safe_index {
            return;
        }
        let ok = self.unique("kq_bounds_ok");
        if self.size(index) == 1 && len <= 255 {
            self.into_a(index);
            self.immediate("CP_IMM", len & 255);
            self.op("JP_C", ok.clone());
            let trap = self.check_trap();
            self.op("JP", trap);
        } else {
            self.into_hl(index);
            if len > 65535 {
                return;
            }
            self.asm("LD_A_H");
            self.immediate("CP_IMM", (len >> 8) & 255);
            self.op("JP_C", ok.clone());
            let trap = self.check_trap();
            self.op("JP_NZ", trap.clone());
            self.asm("LD_A_L");
            self.immediate("CP_IMM", len & 255);
            self.op("JP_C", ok.clone());
            self.op("JP", trap);
        }
        self.label(&ok);
    }
    pub(super) fn slice_access(&mut self, len: &Expr, index: &Expr) {
        if !self.env.options.check_slice_bounds || self.unsafe_depth > 0 {
            self.discard(len);
            return;
        }
        if len.is(t::INTEGER)
            && index.is(t::INTEGER)
            && index
                .int(1)
                .is_some_and(|i| i >= 0 && i < len.int(1).unwrap())
        {
            return;
        }
        self.into_hl(len);
        self.asm("LD_D_H");
        self.asm("LD_E_L");
        self.into_hl(index);
        let ok = self.unique("kq_slice_ok");
        self.asm("LD_A_H");
        self.asm("CP_D");
        self.op("JP_C", ok.clone());
        let trap = self.check_trap();
        self.op("JP_NZ", trap.clone());
        self.asm("LD_A_L");
        self.asm("CP_E");
        self.op("JP_C", ok.clone());
        self.op("JP", trap);
        self.label(&ok);
    }
    pub(super) fn runtime_assert(&mut self, e: &Expr, args: &[&Expr]) {
        if !(1..=2).contains(&args.len()) {
            self.env
                .error(&e.source, "__assert(cond[, code]) expects 1 or 2 arguments");
            return;
        }
        let ok = self.unique("assert_ok");
        self.jump_if(true, args[0], &ok);
        if args.len() == 2 {
            self.into_a(args[1]);
        } else {
            self.immediate("LD_A_IMM", 1);
        }
        self.assert_panic = true;
        self.op("JP", Operand::symbol("__kq_panic", M::Absolute));
        self.label(&ok);
    }
    pub(super) fn append_check_helpers(&mut self) {
        if let Some(trap) = self.check_trap_label.clone() {
            let body = std::mem::take(&mut self.lines);
            self.emit(
                t::COMMENT,
                vec!["[KITAQGB] injected bank0 check trap".into()],
            );
            self.emit(t::FUNCTION, vec![trap.base.as_deref().unwrap().into()]);
            self.op("JP", trap);
            self.lines.extend(body);
        }
        if self.assert_panic {
            let body = std::mem::take(&mut self.lines);
            self.emit(
                t::COMMENT,
                vec!["[KITAQGB] injected runtime assert panic helper".into()],
            );
            self.emit(t::FUNCTION, vec!["__kq_panic".into()]);
            self.asm("DI");
            let spin = Operand::symbol("__kq_panic_spin", M::Absolute);
            self.label(&spin);
            self.asm("HALT");
            self.op("JP", spin);
            self.lines.extend(body);
        }
    }
}
