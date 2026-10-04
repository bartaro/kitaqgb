use super::*;
impl Emitter {
    pub(super) fn scaled_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        if name == "__mul8x8_hi" {
            if args.len() != 2 {
                self.env
                    .error(&e.source, "__mul8x8_hi requires 2 arguments");
                return true;
            }
            self.into_a(args[0]);
            self.asm("PUSH_AF");
            self.into_a(args[1]);
            self.asm("LD_C_A");
            self.asm("POP_AF");
            self.asm("LD_E_A");
            self.mul_bytes_prefix("__mul8x8_hi");
            self.asm("LD_A_H");
            return true;
        }
        let specification = match name {
            "__mul16x8" => Some((2, false, false, "__mul16x8", 0)),
            "__smul16x8" => Some((2, true, false, "__smul16x8", 0)),
            "__smul16x8_q1_7" => Some((2, true, true, "__smul16x8_q1_7", 0)),
            "__mac16" => Some((3, false, false, "__mac16_mul", 1)),
            "__smac16" => Some((3, true, false, "__smac16_mul", 1)),
            "__smac16_q1_7" => Some((3, true, true, "__smac16_q1_7_mul", 1)),
            "__dot3_q8_8" => Some((6, false, false, "__dot3", 2)),
            "__sdot3_q8_8" => Some((6, true, false, "__sdot3", 2)),
            "__sdot3_q1_7" => Some((6, true, true, "__sdot3_q1_7", 2)),
            "__dot2_q8_8" => Some((4, false, false, "__dot2", 3)),
            "__sdot2_q8_8" => Some((4, true, false, "__sdot2", 3)),
            "__sdot2_q1_7" => Some((4, true, true, "__sdot2_q1_7", 3)),
            _ => None,
        };
        let Some((count, signed, q17, prefix, shape)) = specification else {
            return false;
        };
        if args.len() != count {
            self.env
                .error(&e.source, format!("{name} requires {count} arguments"));
            return true;
        }
        match shape {
            0 => self.dot_term(args[0], args[1], signed, q17, prefix),
            1 => {
                self.dot_term(args[1], args[2], signed, q17, prefix);
                self.asm("PUSH_HL");
                self.into_hl(args[0]);
                self.asm("POP_DE");
                self.asm("ADD_HL_DE");
            }
            3 => {
                let ax = self.env.fold(args[2]);
                let ay = self.env.fold(args[3]);
                let zero = |a: &Expr| a.is(t::INTEGER) && a.int(1).is_some_and(|n| n & 255 == 0);
                if !q17 && zero(&ax) && zero(&ay) {
                    self.discard(args[0]);
                    self.discard(args[1]);
                    self.asm("XOR_A");
                    self.asm("LD_H_A");
                    self.asm("LD_L_A");
                } else if !q17 && zero(&ax) {
                    self.discard(args[0]);
                    self.dot_term(args[1], args[3], signed, q17, &format!("{prefix}_y"));
                } else if !q17 && zero(&ay) {
                    self.dot_term(args[0], args[2], signed, q17, &format!("{prefix}_x"));
                    self.asm("PUSH_HL");
                    self.discard(args[1]);
                    self.asm("POP_HL");
                } else {
                    self.dot_term(args[0], args[2], signed, q17, &format!("{prefix}_x"));
                    self.asm("PUSH_HL");
                    self.dot_term(args[1], args[3], signed, q17, &format!("{prefix}_y"));
                    self.asm("POP_DE");
                    self.asm("ADD_HL_DE");
                }
            }
            _ => {
                self.dot_term(args[0], args[3], signed, q17, &format!("{prefix}_x"));
                self.asm("PUSH_HL");
                self.dot_term(args[1], args[4], signed, q17, &format!("{prefix}_y"));
                self.asm("POP_DE");
                self.asm("ADD_HL_DE");
                self.asm("PUSH_HL");
                self.dot_term(args[2], args[5], signed, q17, &format!("{prefix}_z"));
                self.asm("POP_DE");
                self.asm("ADD_HL_DE");
            }
        }
        true
    }
    fn dot_term(
        &mut self,
        value: &Expr,
        coefficient: &Expr,
        signed: bool,
        q17: bool,
        prefix: &str,
    ) {
        let coef = self.env.fold(coefficient);
        if coef.is(t::INTEGER) {
            let byte = coef.int(1).unwrap() & 255;
            let c = if signed { i32::from(byte as i8) } else { byte };
            if c == 0 {
                self.discard(value);
                for m in ["XOR_A", "LD_H_A", "LD_L_A"] {
                    self.asm(m);
                }
                return;
            }
            let magnitude = c.abs();
            if (magnitude as u32).is_power_of_two() {
                let shift = 8 - magnitude.trailing_zeros() as i32;
                self.into_hl(value);
                if signed {
                    let positive = self.unique("__scaled_pow2_nonnegative");
                    self.asm("LD_A_H");
                    self.immediate("AND_IMM", 128);
                    self.op("JP_Z", positive.clone());
                    self.asm("LD_A_L");
                    self.immediate("ADD_A_IMM", (1 << shift) - 1);
                    self.asm("LD_L_A");
                    self.asm("LD_A_H");
                    self.immediate("ADC_IMM", 0);
                    self.asm("LD_H_A");
                    self.label(&positive);
                    self.shift_hl(false, true, shift);
                    if c < 0 {
                        self.negate_hl();
                    }
                } else {
                    self.shift_hl(false, false, shift);
                }
                if q17 {
                    self.asm("ADD_HL_HL");
                }
                return;
            }
        }
        self.into_hl(value);
        self.asm("PUSH_HL");
        self.into_a(coefficient);
        self.asm("LD_C_A");
        self.asm("POP_HL");
        if signed {
            self.scaled_signed(prefix);
        } else {
            self.scaled_unsigned(prefix);
        }
        if q17 {
            self.asm("ADD_HL_HL");
        }
    }
    fn negate_hl(&mut self) {
        for r in ["L", "H"] {
            self.asm(&format!("LD_A_{r}"));
            self.asm("CPL");
            self.asm(&format!("LD_{r}_A"));
        }
        self.asm("INC_HL");
    }
    fn scaled_unsigned(&mut self, prefix: &str) {
        for m in ["PUSH_HL", "POP_DE", "PUSH_DE", "LD_A_D", "LD_E_A"] {
            self.asm(m);
        }
        self.mul_bytes_prefix(&format!("{prefix}_hi"));
        for m in ["POP_DE", "PUSH_HL", "PUSH_DE", "POP_HL", "LD_A_L", "LD_E_A"] {
            self.asm(m);
        }
        self.mul_bytes_prefix(&format!("{prefix}_lo"));
        for m in [
            "LD_A_H", "POP_DE", "LD_H_D", "LD_L_E", "LD_B_A", "LD_A_L", "ADD_B", "LD_L_A", "LD_A_H",
        ] {
            self.asm(m);
        }
        self.immediate("ADC_IMM", 0);
        self.asm("LD_H_A");
    }
    fn scaled_signed(&mut self, prefix: &str) {
        self.asm("LD_A_H");
        self.immediate("AND_IMM", 128);
        self.asm("LD_D_A");
        self.asm("LD_A_C");
        self.immediate("AND_IMM", 128);
        self.asm("LD_E_A");
        for m in ["LD_A_D", "XOR_E", "PUSH_AF"] {
            self.asm(m);
        }
        let a = self.unique(&format!("{prefix}_absA"));
        self.asm("LD_A_D");
        self.asm("OR_A");
        self.op("JP_Z", a.clone());
        self.negate_hl();
        self.label(&a);
        let b = self.unique(&format!("{prefix}_absB"));
        self.asm("LD_A_E");
        self.asm("OR_A");
        self.op("JP_Z", b.clone());
        for m in ["LD_A_C", "CPL", "INC_A", "LD_C_A"] {
            self.asm(m);
        }
        self.label(&b);
        self.scaled_unsigned(&format!("{prefix}_umul"));
        let sign = self.unique(&format!("{prefix}_sign"));
        self.asm("POP_AF");
        self.asm("OR_A");
        self.op("JP_Z", sign.clone());
        self.negate_hl();
        self.label(&sign);
    }
}
