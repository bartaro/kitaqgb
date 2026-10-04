use super::*;
impl Emitter {
    pub(super) fn shift_hl(&mut self, left: bool, signed: bool, count: i32) {
        if count <= 0 {
            return;
        }
        if left {
            if count >= 16 {
                self.word("LD_HL_IMM", 0);
            } else if count >= 8 {
                self.asm("LD_A_L");
                for _ in 0..count - 8 {
                    self.asm("ADD_A");
                }
                self.asm("LD_H_A");
                self.asm("XOR_A");
                self.asm("LD_L_A");
            } else {
                for _ in 0..count {
                    self.asm("ADD_HL_HL");
                }
            }
        } else if !signed && count >= 16 {
            self.word("LD_HL_IMM", 0);
        } else if count == 8 {
            self.asm("LD_A_H");
            self.asm("LD_L_A");
            if signed {
                self.asm("RLCA");
                self.asm("SBC_A");
            } else {
                self.asm("XOR_A");
            }
            self.asm("LD_H_A");
        } else {
            for _ in 0..if signed { count.min(16) } else { count } {
                self.asm("LD_A_H");
                if signed {
                    self.asm("RLCA");
                    self.asm("LD_A_H");
                } else {
                    self.asm("OR_A");
                }
                for m in ["RRA", "LD_H_A", "LD_A_L", "RRA", "LD_L_A"] {
                    self.asm(m);
                }
            }
        }
    }
}
