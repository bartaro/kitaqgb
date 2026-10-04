use super::*;
impl Emitter {
    pub(super) fn constant_multiply(&mut self, factor: i32) -> bool {
        if factor <= 0 || (factor as u32).count_ones() > 4 {
            return false;
        }
        self.asm("LD_D_H");
        self.asm("LD_E_L");
        let msb = (0..16)
            .filter(|i| factor & (1 << i) != 0)
            .max()
            .unwrap_or(0);
        for i in (0..msb).rev() {
            self.asm("ADD_HL_HL");
            if factor & (1 << i) != 0 {
                self.asm("ADD_HL_DE");
            }
        }
        true
    }
    pub(super) fn multiply(&mut self, e: &Expr) {
        let left = child(e, 1);
        let right = child(e, 2);
        if self.size(left) == 1 && self.size(right) == 1 {
            self.into_a(left);
            self.asm("LD_E_A");
            self.into_a(right);
            self.asm("LD_C_A");
            self.mul_bytes();
            return;
        }
        if right.is(t::INTEGER) {
            let factor = right.int(1).unwrap() & 65535;
            if factor == 0 {
                self.word("LD_HL_IMM", 0);
                return;
            }
            self.into_hl(left);
            if factor == 1 {
                return;
            }
            if (factor as u32).is_power_of_two() {
                for _ in 0..factor.trailing_zeros() {
                    self.asm("ADD_HL_HL");
                }
                return;
            }
            if self.constant_multiply(factor) {
                return;
            }
            self.asm("LD_B_H");
            self.asm("LD_C_L");
            self.word("LD_DE_IMM", factor);
        } else {
            self.into_hl(left);
            self.asm("LD_B_H");
            self.asm("LD_C_L");
            self.into_hl(right);
            self.asm("PUSH_HL");
            self.asm("POP_DE");
        }
        self.word("LD_HL_IMM", 0);
        self.immediate("LD_A_IMM", 16);
        let top = self.unique("mul16_loop");
        self.label(&top);
        for m in [
            "PUSH_AF",
            "ADD_HL_HL",
            "PUSH_HL",
            "LD_H_B",
            "LD_L_C",
            "ADD_HL_HL",
            "LD_B_H",
            "LD_C_L",
            "POP_HL",
        ] {
            self.asm(m);
        }
        let skip = self.unique("mul16_skip");
        self.op("JP_NC", skip.clone());
        self.asm("ADD_HL_DE");
        self.label(&skip);
        self.asm("POP_AF");
        self.asm("DEC_A");
        self.op("JP_NZ", top);
    }
    fn mul_bytes(&mut self) {
        self.mul_bytes_prefix("mul8");
    }
    pub(super) fn mul_bytes_prefix(&mut self, prefix: &str) {
        self.asm("PUSH_BC");
        self.immediate("LD_D_IMM", 0);
        self.word("LD_HL_IMM", 0);
        self.immediate("LD_B_IMM", 8);
        let top = self.unique(&format!("{prefix}_loop"));
        let skip = self.unique(&format!("{prefix}_skip"));
        self.label(&top);
        for m in ["LD_A_C", "RRCA", "LD_C_A"] {
            self.asm(m);
        }
        self.op("JP_NC", skip.clone());
        self.asm("ADD_HL_DE");
        self.label(&skip);
        for m in [
            "LD_A_E", "ADD_E", "LD_E_A", "LD_A_D", "ADC_D", "LD_D_A", "DEC_B",
        ] {
            self.asm(m);
        }
        self.op("JP_NZ", top);
        self.asm("POP_BC");
    }
    pub(super) fn divide(&mut self, e: &Expr) {
        let left = child(e, 1);
        let right = child(e, 2);
        let signed = self.env.type_of(left).is_signed() || self.env.type_of(right).is_signed();
        self.into_hl(left);
        self.asm("PUSH_HL");
        self.into_hl(right);
        self.asm("LD_B_H");
        self.asm("LD_C_L");
        self.asm("POP_DE");
        let end = self.unique("div_end");
        let nonzero = self.unique("div_nonzero");
        self.asm("LD_A_B");
        self.asm("OR_C");
        self.op("JP_NZ", nonzero.clone());
        self.word("LD_DE_IMM", 0);
        self.word("LD_HL_IMM", 0);
        self.op("JP", end.clone());
        self.label(&nonzero);
        if signed {
            self.asm("LD_A_D");
            self.immediate("AND_IMM", 128);
            self.asm("PUSH_AF");
            self.asm("LD_A_D");
            self.asm("XOR_B");
            self.immediate("AND_IMM", 128);
            self.asm("PUSH_AF");
            let absde = self.unique("div_absde_done");
            self.asm("LD_A_D");
            self.immediate("AND_IMM", 128);
            self.op("JP_Z", absde.clone());
            self.negate_pair("D", "E");
            self.label(&absde);
            let absbc = self.unique("div_absbc_done");
            self.asm("LD_A_B");
            self.immediate("AND_IMM", 128);
            self.op("JP_Z", absbc.clone());
            self.negate_pair("B", "C");
            self.label(&absbc);
        }
        self.unsigned_division();
        if signed {
            let q = self.unique("div_qsign_done");
            self.asm("POP_AF");
            self.asm("OR_A");
            self.op("JP_Z", q.clone());
            self.negate_pair("D", "E");
            self.label(&q);
            let r = self.unique("div_rsign_done");
            self.asm("POP_AF");
            self.asm("OR_A");
            self.op("JP_Z", r.clone());
            for reg in ["L", "H"] {
                self.asm(&format!("LD_A_{reg}"));
                self.asm("CPL");
                self.asm(&format!("LD_{reg}_A"));
            }
            self.asm("INC_HL");
            self.label(&r);
        }
        if e.is(t::DIVIDE) {
            self.asm("LD_H_D");
            self.asm("LD_L_E");
        }
        self.label(&end);
    }
    fn negate_pair(&mut self, hi: &str, lo: &str) {
        self.asm(&format!("LD_A_{lo}"));
        self.asm("CPL");
        self.immediate("ADD_A_IMM", 1);
        self.asm(&format!("LD_{lo}_A"));
        self.asm(&format!("LD_A_{hi}"));
        self.asm("CPL");
        self.immediate("ADC_IMM", 0);
        self.asm(&format!("LD_{hi}_A"));
    }
    fn unsigned_division(&mut self) {
        self.word("LD_HL_IMM", 0);
        self.immediate("LD_A_IMM", 16);
        let top = self.unique("div_loop");
        self.label(&top);
        for m in [
            "PUSH_AF", "LD_A_E", "ADD_E", "LD_E_A", "LD_A_D", "ADC_D", "LD_D_A",
        ] {
            self.asm(m);
        }
        self.immediate("LD_A_IMM", 0);
        self.immediate("ADC_IMM", 0);
        self.asm("ADD_HL_HL");
        let no_input = self.unique("div_noin");
        self.asm("OR_A");
        self.op("JP_Z", no_input.clone());
        self.asm("INC_HL");
        self.label(&no_input);
        for m in ["LD_A_L", "SUB_C", "LD_L_A", "LD_A_H", "SBC_B", "LD_H_A"] {
            self.asm(m);
        }
        let borrow = self.unique("div_borrow");
        let after = self.unique("div_after");
        self.op("JP_C", borrow.clone());
        self.asm("LD_A_E");
        self.immediate("OR_IMM", 1);
        self.asm("LD_E_A");
        self.op("JP", after.clone());
        self.label(&borrow);
        for m in ["PUSH_DE", "LD_D_B", "LD_E_C", "ADD_HL_DE", "POP_DE"] {
            self.asm(m);
        }
        self.label(&after);
        self.asm("POP_AF");
        self.asm("DEC_A");
        self.op("JP_NZ", top);
    }
}
