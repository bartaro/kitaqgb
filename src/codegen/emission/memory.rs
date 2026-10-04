use super::*;
use crate::asm::Modifier;
impl Emitter {
    pub(super) fn index_address(&mut self, e: &Expr) {
        let mut base = child(e, 1);
        if base.is(t::SLICE) {
            self.slice_access(child(base, 2), child(e, 2));
            base = child(base, 1);
        } else {
            self.bounds_check(base, child(e, 2));
        }
        let ty = self.env.type_of(base);
        let stride = sub_type(&ty).map_or(1, |s| self.env.size(e, s));
        self.scaled_index(child(e, 2), stride);
        self.asm("PUSH_DE");
        self.into_hl(base);
        self.asm("POP_DE");
        self.asm("ADD_HL_DE");
    }
    pub(super) fn field_address(&mut self, e: &Expr) {
        let base = child(e, 1);
        let name = e.text(2).unwrap();
        let Some(field) = self.env.field(e, base, name) else {
            return;
        };
        self.address(base);
        if field.offset != 0 {
            self.word("LD_DE_IMM", field.offset);
            self.asm("ADD_HL_DE");
        }
    }
    pub(super) fn read_memory_a(&mut self, e: &Expr) {
        if e.is(t::FIELD) {
            let base = child(e, 1);
            if base.is(t::NAME) {
                if let Some(field) = self.env.field(e, base, e.text(2).unwrap()) {
                    if let Some(s) = self
                        .env
                        .find(base.text(1).unwrap())
                        .cloned()
                        .filter(|s| matches!(s.tag, SymbolTag::Global | SymbolTag::Local))
                    {
                        self.load(memory_operand(s.value + field.offset));
                        return;
                    }
                }
            }
            self.field_address(e);
            self.asm("LD_A_HL");
            return;
        }
        let base = child(e, 1);
        let index = child(e, 2);
        if base.is(t::NAME) {
            if let Some(s) = self
                .env
                .find(base.text(1).unwrap())
                .cloned()
                .filter(|s| s.tag == SymbolTag::ReadonlyData && s.ty.is_array())
            {
                let size = sub_type(&s.ty).map_or(1, |t| self.env.size(e, t));
                if size == 1 {
                    if let Some(n) = index
                        .int(1)
                        .filter(|_| index.is(t::INTEGER))
                        .filter(|n| *n >= 0)
                    {
                        let mut op = Operand::symbol(s.name, M::Absolute);
                        op.offset = n & 65535;
                        self.load(op);
                        return;
                    }
                    if self.size(index) == 1
                        && self
                            .env
                            .readonly_alignments
                            .get(&s.name)
                            .copied()
                            .unwrap_or(0)
                            .max(s.ty.forced_align)
                            >= 256
                    {
                        self.into_a(index);
                        self.asm("LD_L_A");
                        self.op(
                            "LD_H_IMM",
                            Operand::symbol(s.name, M::Immediate).with_modifier(Modifier::HighByte),
                        );
                        self.asm("LD_A_HL");
                        return;
                    }
                }
            }
        }
        self.index_address(e);
        self.asm("LD_A_HL");
    }
    pub(super) fn read_memory_hl(&mut self, e: &Expr) {
        let ty = self.env.type_of(e);
        let size = self.env.size(e, &ty);
        if e.is(t::FIELD) {
            self.field_address(e);
        } else {
            self.index_address(e);
        }
        if e.is(t::FIELD) && ty.is_array() {
            return;
        }
        if size == 1 {
            self.asm("LD_A_HL");
            self.extend_a(ty.is_signed());
        } else {
            for m in [
                "LD_A_HL", "LD_E_A", "INC_HL", "LD_A_HL", "LD_D_A", "LD_H_D", "LD_L_E",
            ] {
                self.asm(m);
            }
        }
    }
    pub(super) fn assign_memory(&mut self, left: &Expr, right: &Expr) {
        let size = self.size(left);
        if left.is(t::INDEX) {
            self.bounds_check(child(left, 1), child(left, 2));
        }
        if !matches!(size, 1 | 2) {
            self.unsupported(left, "aggregate memory assignment");
            return;
        }
        if left.is(t::FIELD) {
            let base = child(left, 1);
            if base.is(t::INDEX) || base.is(t::LOAD) {
                self.field_address(left);
                self.asm("PUSH_HL");
                if size == 1 {
                    self.into_a(right);
                    self.asm("POP_HL");
                    self.asm("LD_HL_A");
                } else {
                    self.into_hl(right);
                    self.asm("POP_DE");
                    for m in ["LD_A_L", "LD_DE_A", "INC_DE", "LD_A_H", "LD_DE_A"] {
                        self.asm(m);
                    }
                }
                return;
            }
            if base.is(t::NAME) {
                let name = base.text(1).unwrap();
                if let Some(s) = self
                    .env
                    .find_required(base, name)
                    .filter(|s| matches!(s.tag, SymbolTag::Global | SymbolTag::Local))
                {
                    if let Some(field) = self.env.field(left, base, left.text(2).unwrap()) {
                        let address = s.value + field.offset;
                        if size == 1 {
                            self.into_a(right);
                            self.store(memory_operand(address));
                        } else {
                            self.into_hl(right);
                            self.asm("LD_A_L");
                            self.store(memory_operand(address));
                            self.asm("LD_A_H");
                            self.store(memory_operand(address + 1));
                        }
                        return;
                    }
                }
            }
            if size == 1 {
                self.into_a(right);
                self.asm("PUSH_AF");
                self.address(left);
                self.asm("POP_AF");
                self.asm("LD_HL_A");
            } else {
                self.into_hl(right);
                self.asm("PUSH_HL");
                self.address(left);
                for m in ["POP_DE", "LD_A_E", "LD_HL_A", "INC_HL", "LD_A_D", "LD_HL_A"] {
                    self.asm(m);
                }
            }
            return;
        }
        let base = child(left, 1);
        let index = child(left, 2);
        if size == 1 {
            if self.size(right) == 2 {
                self.into_hl(right);
                self.asm("LD_A_L");
            } else {
                self.into_a(right);
            }
            self.asm("PUSH_AF");
        } else {
            self.into_hl(right);
            self.asm("PUSH_HL");
        }
        self.scaled_index(index, size);
        self.asm("PUSH_DE");
        self.into_hl(base);
        self.asm("POP_DE");
        self.asm("ADD_HL_DE");
        if size == 1 {
            self.asm("POP_AF");
            self.asm("LD_HL_A");
        } else {
            for m in ["POP_DE", "LD_A_E", "LD_HL_A", "INC_HL", "LD_A_D", "LD_HL_A"] {
                self.asm(m);
            }
        }
    }
}
