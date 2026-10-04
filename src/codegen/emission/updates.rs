use super::*;
impl Emitter {
    pub(super) fn increment_a(&mut self, e: &Expr) {
        let target = child(e, 1);
        let post = matches!(e.tag(), Some(t::POST_INCREMENT | t::POST_DECREMENT));
        let increment = matches!(e.tag(), Some(t::PRE_INCREMENT | t::POST_INCREMENT));
        if self.size(target) == 1 {
            if let Some(op) = self.operand(target) {
                self.load(op.clone());
                if post {
                    self.asm("LD_B_A");
                }
                self.asm(if increment { "INC_A" } else { "DEC_A" });
                self.store(op);
                if post {
                    self.asm("LD_A_B");
                }
                return;
            }
        }
        self.increment_hl(e);
        self.asm("LD_A_L");
    }
    pub(super) fn increment_hl(&mut self, e: &Expr) {
        let target = child(e, 1);
        let ty = self.env.type_of(target);
        let size = self.env.size(target, &ty);
        if !matches!(size, 1 | 2) {
            self.env.error(
                &e.source,
                "Increment/decrement requires a byte, word or pointer lvalue",
            );
            return;
        }
        let post = matches!(e.tag(), Some(t::POST_INCREMENT | t::POST_DECREMENT));
        let increment = matches!(e.tag(), Some(t::PRE_INCREMENT | t::POST_INCREMENT));
        let step = if let Kind::Pointer(sub) = &ty.kind {
            self.env.size(target, sub)
        } else {
            1
        };
        self.address(target);
        self.asm("PUSH_HL");
        self.asm("LD_A_HL");
        self.asm("LD_E_A");
        if size == 2 {
            for m in ["INC_HL", "LD_A_HL", "LD_D_A"] {
                self.asm(m);
            }
        } else {
            self.immediate("LD_D_IMM", 0);
        }
        self.asm("LD_H_D");
        self.asm("LD_L_E");
        if post {
            self.asm("LD_B_H");
            self.asm("LD_C_L");
        }
        if step == 1 {
            self.asm(if increment { "INC_HL" } else { "DEC_HL" });
        } else {
            self.word("LD_DE_IMM", (if increment { step } else { -step }) & 65535);
            self.asm("ADD_HL_DE");
        }
        for m in ["LD_D_H", "LD_E_L", "POP_HL", "LD_A_E", "LD_HL_A"] {
            self.asm(m);
        }
        if size == 2 {
            for m in ["INC_HL", "LD_A_D", "LD_HL_A"] {
                self.asm(m);
            }
        }
        if size == 1 {
            self.asm(if post { "LD_A_C" } else { "LD_A_E" });
            self.extend_a(ty.is_signed());
        } else {
            self.asm(if post { "LD_H_B" } else { "LD_H_D" });
            self.asm(if post { "LD_L_C" } else { "LD_L_E" });
        }
    }
    pub(super) fn increment_statement(&mut self, target: &Expr, increment: bool) {
        let ty = self.env.type_of(target);
        let size = self.env.size(target, &ty);
        self.address(target);
        if size == 1 {
            self.asm("LD_A_HL");
            self.asm(if increment { "INC_A" } else { "DEC_A" });
            self.asm("LD_HL_A");
        } else if size == 2 {
            for m in [
                "PUSH_HL", "LD_A_HL", "LD_E_A", "INC_HL", "LD_A_HL", "LD_D_A", "LD_H_D", "LD_L_E",
            ] {
                self.asm(m);
            }
            let step = if let Kind::Pointer(sub) = &ty.kind {
                self.env.size(target, sub)
            } else {
                1
            };
            let adjustment = if increment { step } else { -step };
            if adjustment == 1 {
                self.asm("INC_HL");
            } else if adjustment == -1 {
                self.asm("DEC_HL");
            } else {
                self.word("LD_DE_IMM", adjustment & 65535);
                self.asm("ADD_HL_DE");
            }
            for m in [
                "LD_D_H", "LD_E_L", "POP_HL", "LD_A_E", "LD_HL_A", "INC_HL", "LD_A_D", "LD_HL_A",
            ] {
                self.asm(m);
            }
        } else {
            self.env.error(
                &target.source,
                "Increment/decrement requires a byte, word or pointer lvalue",
            );
        }
    }
}
