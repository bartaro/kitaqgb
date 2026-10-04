use super::*;
pub(super) fn comparison(e: &Expr) -> bool {
    matches!(
        e.tag(),
        Some(
            t::EQUAL
                | t::NOT_EQUAL
                | t::LESS_THAN
                | t::LESS_THAN_OR_EQUAL
                | t::GREATER_THAN
                | t::GREATER_THAN_OR_EQUAL
        )
    )
}
impl Emitter {
    fn signed_comparison(&mut self, left: &Expr, right: &Expr) -> bool {
        let l = self.env.type_of(left);
        let r = self.env.type_of(right);
        !l.is_pointer() && !r.is_pointer() && (l.is_signed() || r.is_signed())
    }
    pub(super) fn jump_if(&mut self, condition: bool, e: &Expr, target: &Operand) {
        let e = self.env.fold(e);
        if e.is(t::INTEGER) {
            if (e.int(1).unwrap() != 0) == condition {
                self.op("JP", target.clone());
            }
            return;
        }
        if e.is(t::LOGICAL_NOT) {
            self.jump_if(!condition, child(&e, 1), target);
            return;
        }
        if e.is(t::LOGICAL_AND) || e.is(t::LOGICAL_OR) {
            let and = e.is(t::LOGICAL_AND);
            if (and && !condition) || (!and && condition) {
                self.jump_if(condition, child(&e, 1), target);
                self.jump_if(condition, child(&e, 2), target);
            } else {
                let skip = self.unique(if and { "and_sc_skip" } else { "or_sc_end" });
                self.jump_if(!condition, child(&e, 1), &skip);
                self.jump_if(condition, child(&e, 2), target);
                self.label(&skip);
            }
            return;
        }
        if comparison(&e) {
            let left = child(&e, 1);
            let right = child(&e, 2);
            let tag = e.tag().unwrap();
            let signed = self.signed_comparison(left, right);
            if self.size(left) == 1 && self.size(right) == 1 {
                if matches!(tag, t::EQUAL | t::NOT_EQUAL)
                    && left.is(t::INTEGER)
                    && left.int(1).unwrap() & 255 == 0
                {
                    self.into_a(right);
                    self.asm("OR_A");
                    self.branch(tag, condition, target);
                    return;
                }
                self.into_a(left);
                if signed {
                    self.immediate("XOR_IMM", 128);
                }
                if right.is(t::INTEGER) {
                    let n = (right.int(1).unwrap() & 255) ^ if signed { 128 } else { 0 };
                    if !signed && n == 0 && matches!(tag, t::EQUAL | t::NOT_EQUAL) {
                        self.asm("OR_A");
                    } else {
                        self.immediate("CP_IMM", n);
                    }
                } else {
                    self.asm("PUSH_AF");
                    self.into_a(right);
                    if signed {
                        self.immediate("XOR_IMM", 128);
                    }
                    self.asm("LD_B_A");
                    self.asm("POP_AF");
                    self.asm("CP_B");
                }
                self.branch(tag, condition, target);
                return;
            }
            if matches!(tag, t::EQUAL | t::NOT_EQUAL) {
                let zero = |e: &Expr| e.is(t::INTEGER) && e.int(1).unwrap() & 65535 == 0;
                if (zero(right) && self.size(left) == 2) || (zero(left) && self.size(right) == 2) {
                    self.into_hl(if zero(right) { left } else { right });
                    self.asm("LD_A_H");
                    self.asm("OR_L");
                    self.op(
                        if (tag == t::EQUAL) == condition {
                            "JP_Z"
                        } else {
                            "JP_NZ"
                        },
                        target.clone(),
                    );
                    return;
                }
            }
            self.into_hl(left);
            if right.is(t::INTEGER) {
                self.word("LD_DE_IMM", right.int(1).unwrap());
            } else {
                self.asm("PUSH_HL");
                self.into_hl(right);
                self.asm("PUSH_HL");
                self.asm("POP_DE");
                self.asm("POP_HL");
            }
            if signed && !matches!(tag, t::EQUAL | t::NOT_EQUAL) {
                for r in ["H", "D"] {
                    self.asm(&format!("LD_A_{r}"));
                    self.immediate("XOR_IMM", 128);
                    self.asm(&format!("LD_{r}_A"));
                }
            }
            self.asm("LD_A_H");
            self.asm("CP_D");
            if matches!(tag, t::EQUAL | t::NOT_EQUAL) {
                for m in ["LD_A_H", "XOR_D", "LD_B_A", "LD_A_L", "XOR_E", "OR_B"] {
                    self.asm(m);
                }
                self.op(
                    if (tag == t::EQUAL) == condition {
                        "JP_Z"
                    } else {
                        "JP_NZ"
                    },
                    target.clone(),
                );
            } else {
                let skip = self.unique("cmp16_skip");
                self.op("JP_NZ", skip.clone());
                self.asm("LD_A_L");
                self.asm("CP_E");
                self.label(&skip);
                self.branch(tag, condition, target);
            }
            return;
        }
        if self.size(&e) == 2 {
            self.into_hl(&e);
            self.asm("LD_A_H");
            self.asm("OR_L");
        } else {
            self.into_a(&e);
            self.asm("OR_A");
        }
        self.op(if condition { "JP_NZ" } else { "JP_Z" }, target.clone());
    }
    fn branch(&mut self, tag: &str, condition: bool, target: &Operand) {
        let opcode = match tag {
            t::EQUAL => {
                if condition {
                    "JP_Z"
                } else {
                    "JP_NZ"
                }
            }
            t::NOT_EQUAL => {
                if condition {
                    "JP_NZ"
                } else {
                    "JP_Z"
                }
            }
            t::LESS_THAN => {
                if condition {
                    "JP_C"
                } else {
                    "JP_NC"
                }
            }
            t::GREATER_THAN_OR_EQUAL => {
                if condition {
                    "JP_NC"
                } else {
                    "JP_C"
                }
            }
            _ => "",
        };
        if !opcode.is_empty() {
            self.op(opcode, target.clone());
            return;
        }
        if (tag == t::GREATER_THAN && !condition) || (tag == t::LESS_THAN_OR_EQUAL && condition) {
            self.op("JP_C", target.clone());
            self.op("JP_Z", target.clone());
        } else {
            let skip = self.unique(if tag == t::GREATER_THAN {
                "no_gt"
            } else {
                "no_le"
            });
            self.op("JP_C", skip.clone());
            self.op("JP_Z", skip.clone());
            self.op("JP", target.clone());
            self.label(&skip);
        }
    }
    pub(super) fn comparison_a(&mut self, e: &Expr) {
        let left = child(e, 1);
        let right = child(e, 2);
        let tag = e.tag().unwrap();
        if self.size(left) == 2 || self.size(right) == 2 || self.signed_comparison(left, right) {
            let yes = self.unique("cmp_t");
            let end = self.unique("cmp_e");
            self.jump_if(true, e, &yes);
            self.bool_labels(&yes, &end);
            return;
        }
        let reverse = matches!(tag, t::GREATER_THAN | t::LESS_THAN_OR_EQUAL);
        self.into_a(if reverse { right } else { left });
        self.asm("PUSH_AF");
        self.into_a(if reverse { left } else { right });
        self.asm(if reverse { "LD_C_A" } else { "LD_B_A" });
        self.asm("POP_AF");
        self.asm(if reverse { "CP_C" } else { "CP_B" });
        let (prefix, opcode) = match tag {
            t::EQUAL => ("eq", "JP_Z"),
            t::NOT_EQUAL => ("neq", "JP_NZ"),
            t::LESS_THAN => ("lt", "JP_C"),
            t::GREATER_THAN_OR_EQUAL => ("ge", "JP_NC"),
            t::GREATER_THAN => ("gt", "JP_C"),
            _ => ("le", "JP_NC"),
        };
        let yes = self.unique(&format!("{prefix}_t"));
        let end = self.unique(&format!("{prefix}_e"));
        self.op(opcode, yes.clone());
        self.bool_labels(&yes, &end);
    }
    fn bool_labels(&mut self, yes: &Operand, end: &Operand) {
        self.immediate("LD_A_IMM", 0);
        self.op("JP", end.clone());
        self.label(yes);
        self.immediate("LD_A_IMM", 1);
        self.label(end);
    }
}
