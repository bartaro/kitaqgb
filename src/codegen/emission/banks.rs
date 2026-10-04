use super::*;
impl Emitter {
    pub(super) fn bank_thunk(&mut self, name: &str, bank: i32) -> String {
        let bank = bank & 255;
        let thunk = format!("__kq_thunk_b{bank}_{name}");
        self.thunks
            .entry((bank, name.into()))
            .or_insert(thunk.clone());
        thunk
    }
    pub(super) fn append_bank_helpers(&mut self) {
        if self.thunks.is_empty()
            && !self.bankswitch_helper
            && !self.far_memcpy
            && !self.far_pointer
            && self.far_thunks.is_empty()
        {
            return;
        }
        let body = std::mem::take(&mut self.lines);
        self.emit(
            t::COMMENT,
            vec!["[KITAQGB] injected bank0 banking helpers".into()],
        );
        if self.bankswitch_helper {
            self.emit(t::FUNCTION, vec!["__kq_bankswitch_bank0".into()]);
            self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
            self.op("LDH_MEM_A", Operand::integer(0x82, M::Immediate));
            self.asm("RET");
        }
        self.append_far_helpers();
        if !self.stack_thunks.is_empty() {
            self.emit(t::FUNCTION, vec!["__kq_thunk_stack_trap".into()]);
            self.op("JP", Operand::symbol("__kq_thunk_stack_trap", M::Absolute));
        }
        let mut thunks = self.thunks.clone().into_iter().collect::<Vec<_>>();
        thunks.sort_by(|((ab, an), _), ((bb, bn), _)| {
            ab.cmp(bb)
                .then_with(|| super::super::identifier_order::compare(an, bn))
        });
        for ((bank, target), thunk) in thunks {
            self.emit(t::FUNCTION, vec![thunk.clone().into()]);
            if self.stack_thunks.contains(&thunk) {
                self.stack_thunk_body(&thunk, bank, &target);
                continue;
            }
            self.asm("PUSH_AF");
            self.op("LDH_A_MEM", Operand::integer(0x82, M::Immediate));
            self.asm("PUSH_AF");
            self.immediate("LD_A_IMM", bank);
            self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
            self.op("LDH_MEM_A", Operand::integer(0x82, M::Immediate));
            for m in ["POP_BC", "POP_AF", "PUSH_BC"] {
                self.asm(m);
            }
            self.op("CALL", Operand::symbol(target, M::Absolute));
            for m in ["PUSH_AF", "PUSH_HL", "POP_DE", "POP_BC", "POP_AF"] {
                self.asm(m);
            }
            self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
            self.op("LDH_MEM_A", Operand::integer(0x82, M::Immediate));
            for m in ["PUSH_BC", "PUSH_DE", "POP_HL", "POP_AF", "RET"] {
                self.asm(m);
            }
        }
        self.lines.extend(body);
    }
}
