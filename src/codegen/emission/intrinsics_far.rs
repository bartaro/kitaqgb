use super::*;
impl Emitter {
    pub(super) fn far_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        let count = match name {
            "__bankof" => 1,
            "__farcall" | "__farcall_ptr" | "__farpeek8" | "__farpeek16" => 2,
            "__far_memcpy" => 4,
            _ => return false,
        };
        if args.len() != count {
            self.env
                .error(&e.source, format!("{name} expects {count} arguments"));
            return true;
        }
        match name {
            "__bankof" => {
                if let Some(name) = args[0].text(1).filter(|_| args[0].is(t::NAME)) {
                    let mut operand = Operand::symbol(name, M::Immediate);
                    operand.modifier = crate::asm::Modifier::Bank;
                    self.op("LD_A_IMM", operand);
                } else {
                    self.env.error(
                        &e.source,
                        "__bankof(symbol) requires a symbol name argument",
                    );
                }
            }
            "__far_memcpy" => {
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("PUSH_AF");
                self.into_hl(args[2]);
                self.asm("PUSH_HL");
                self.into_hl(args[3]);
                self.asm("LD_B_H");
                self.asm("LD_C_L");
                self.asm("POP_DE");
                self.asm("POP_AF");
                self.asm("POP_HL");
                self.far_copy_call();
            }
            "__farpeek8" | "__farpeek16" => {
                self.push_byte_word(args[0]);
                self.into_hl(args[1]);
                self.asm("PUSH_HL");
                self.op("ADD_SP_IMM", Operand::integer(-2, M::Relative));
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                for m in ["LD_A_HL", "LD_E_A", "INC_HL", "LD_A_HL", "LD_D_A"] {
                    self.asm(m);
                }
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("POP_HL");
                self.word("LD_BC_IMM", if name == "__farpeek8" { 1 } else { 2 });
                self.far_copy_call();
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("LD_A_HL");
                if name == "__farpeek16" {
                    for m in ["LD_E_A", "INC_HL", "LD_A_HL", "LD_D_A", "LD_H_D", "LD_L_E"] {
                        self.asm(m);
                    }
                }
                self.op("ADD_SP_IMM", Operand::integer(6, M::Relative));
            }
            "__farcall_ptr" => {
                let ty = self.env.type_of(args[1]);
                let ty = if let Kind::Pointer(sub) = ty.kind {
                    *sub
                } else {
                    ty
                };
                if let Kind::Function(_, params) = ty.kind {
                    if !params.is_empty() {
                        self.env.error(
                            &e.source,
                            "__farcall_ptr requires a callback with no parameters",
                        );
                        return true;
                    }
                }
                self.into_a(args[0]);
                self.asm("PUSH_AF");
                self.into_hl(args[1]);
                self.asm("POP_AF");
                self.far_pointer = true;
                self.report_call(
                    e,
                    "<indirect-far>",
                    -1,
                    "farcall_pointer",
                    true,
                    true,
                    &[],
                    &[],
                );
                self.op(
                    "CALL",
                    Operand::symbol("__kq_farcall_pointer_bank0", M::Absolute),
                );
            }
            _ => {
                let Some(target) = args[1].text(1).filter(|_| args[1].is(t::NAME)) else {
                    self.env.error(
                        &e.source,
                        "__farcall(bank, func) requires 2nd argument to be a function name",
                    );
                    return true;
                };
                let Some(info) = self.env.functions.get(target).cloned() else {
                    self.env
                        .error(&e.source, "__farcall requires a declared function name");
                    return true;
                };
                if !info.parameters.is_empty() {
                    self.env.error(
                        &e.source,
                        "__farcall requires a callback with no parameters",
                    );
                    return true;
                }
                self.prepare_return_destination(&info);
                let bank = self.env.fold(args[0]);
                let thunk = if let Some(bank) = bank.int(1).filter(|_| bank.is(t::INTEGER)) {
                    let bank = bank & 255;
                    if bank != 0 && bank != self.current_bank {
                        let thunk = self.bank_thunk(target, bank);
                        if info.stack_call {
                            self.stack_thunks.insert(thunk.clone());
                        }
                        self.report_call(
                            e,
                            target,
                            bank,
                            "farcall_bank_thunk",
                            true,
                            true,
                            &[],
                            &[],
                        );
                        thunk
                    } else {
                        self.direct_bank_check(bank, false);
                        self.report_call(e, target, bank, "farcall_direct", false, true, &[], &[]);
                        target.into()
                    }
                } else {
                    self.into_a(&bank);
                    let thunk = format!("__kq_farcall_{target}");
                    self.far_thunks.insert(thunk.clone(), target.into());
                    self.report_call(
                        e,
                        target,
                        info.rom_bank,
                        "farcall_intrinsic",
                        true,
                        true,
                        &[],
                        &[],
                    );
                    thunk
                };
                self.op("CALL", Operand::symbol(thunk, M::Absolute));
            }
        }
        true
    }
    pub(super) fn far_copy_call(&mut self) {
        self.far_memcpy = true;
        self.op(
            "CALL",
            Operand::symbol("__kq_far_memcpy_bank0", M::Absolute),
        );
    }
    pub(super) fn append_far_helpers(&mut self) {
        if self.far_memcpy {
            self.emit(t::FUNCTION, vec!["__kq_far_memcpy_bank0".into()]);
            self.asm("PUSH_AF");
            self.op("LDH_A_MEM", Operand::integer(0x82, M::Immediate));
            self.op("LDH_MEM_A", Operand::integer(0x83, M::Immediate));
            self.asm("POP_AF");
            self.switch_a();
            let loop_label = Operand::symbol("__kq_far_memcpy_loop", M::Absolute);
            let done = Operand::symbol("__kq_far_memcpy_done", M::Absolute);
            self.label(&loop_label);
            self.asm("LD_A_B");
            self.asm("OR_C");
            self.op("JP_Z", done.clone());
            for m in ["LD_A_DE", "LDI_HL_A", "INC_DE", "DEC_BC"] {
                self.asm(m);
            }
            self.op("JP", loop_label);
            self.label(&done);
            self.op("LDH_A_MEM", Operand::integer(0x83, M::Immediate));
            self.switch_a();
            self.asm("RET");
        }
        if self.far_pointer {
            self.emit(t::FUNCTION, vec!["__kq_farcall_pointer_bank0".into()]);
            self.runtime_bank_entry();
            let returned = Operand::symbol("__kq_farcall_pointer_return", M::Absolute);
            self.op(
                "LD_DE_IMM",
                Operand::symbol("__kq_farcall_pointer_return", M::Immediate16),
            );
            self.asm("PUSH_DE");
            self.asm("JP_HL");
            self.label(&returned);
            self.restore_bank_return();
        }
        let mut thunks = self.far_thunks.clone().into_iter().collect::<Vec<_>>();
        thunks.sort_by(|(a, _), (b, _)| super::super::identifier_order::compare(a, b));
        for (thunk, target) in thunks {
            self.emit(t::FUNCTION, vec![thunk.into()]);
            self.runtime_bank_entry();
            self.op("CALL", Operand::symbol(target, M::Absolute));
            self.restore_bank_return();
        }
    }
    fn switch_a(&mut self) {
        self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
        self.op("LDH_MEM_A", Operand::integer(0x82, M::Immediate));
    }
    fn runtime_bank_entry(&mut self) {
        self.asm("PUSH_AF");
        self.op("LDH_A_MEM", Operand::integer(0x82, M::Immediate));
        for m in ["PUSH_AF", "POP_BC", "POP_AF", "PUSH_BC"] {
            self.asm(m);
        }
        self.switch_a();
    }
    fn restore_bank_return(&mut self) {
        for m in ["PUSH_AF", "PUSH_HL", "POP_DE", "POP_BC", "POP_AF"] {
            self.asm(m);
        }
        self.switch_a();
        for m in ["PUSH_BC", "PUSH_DE", "POP_HL", "POP_AF", "RET"] {
            self.asm(m);
        }
    }
}
