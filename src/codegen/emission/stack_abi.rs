use super::*;
fn writes(e: &Expr, names: &mut BTreeSet<String>) {
    let lhs = match e.tag() {
        Some(
            t::ASSIGN | t::PRE_INCREMENT | t::PRE_DECREMENT | t::POST_INCREMENT | t::POST_DECREMENT,
        ) => e.child(1),
        Some(t::ASSIGN_MODIFY) => e.child(2),
        _ => None,
    };
    if let Some(mut lhs) = lhs {
        while lhs.is(t::CAST) {
            lhs = child(lhs, 2);
        }
        if lhs.is(t::NAME) {
            names.insert(lhs.text(1).unwrap().into());
        }
    }
    for e in e.children() {
        writes(e, names);
    }
}
impl Emitter {
    pub(super) fn stack_parameters(&mut self, e: &Expr, info: &FunctionInfo) {
        let mut modified = BTreeSet::new();
        writes(info.body.as_ref().unwrap(), &mut modified);
        let mut offset = 0;
        let mut copies = Vec::new();
        let mut snapshot = false;
        for p in &info.parameters {
            let size = self.env.size(e, &p.ty);
            if size != 1 && size != 2 && !p.ty.is_aggregate() {
                self.unsupported(e, "aggregate stack parameter");
                return;
            }
            let local = modified.contains(&p.name) && !p.ty.is_const;
            let value = if local {
                self.env.allocate_preferred(size, 1, &[0, 1, 2])
            } else {
                offset
            };
            let symbol = Symbol {
                tag: if local {
                    SymbolTag::Local
                } else {
                    SymbolTag::StackParam
                },
                value,
                ty: p.ty.clone(),
                name: p.name.clone(),
                wram_bank: 0,
            };
            self.env.declare(e, symbol.clone());
            if local {
                copies.push((symbol, offset, size));
                if offset + 2 > 127 {
                    snapshot = true;
                }
            } else {
                snapshot = true;
            }
            offset += size;
        }
        self.save_incoming_return_pointer(info);
        if snapshot {
            let base = self.env.allocate_preferred(2, 1, &[0, 1, 2]);
            self.stack_base = Some(base);
            self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
            self.asm("LD_A_L");
            self.store(memory_operand(base));
            self.asm("LD_A_H");
            self.store(memory_operand(base + 1));
        }
        for (symbol, offset, size) in copies {
            if snapshot {
                self.stack_param_address(offset, &format!("stk arg:{}", symbol.name));
            } else {
                self.op("LD_HL_SP_IMM", Operand::integer(2 + offset, M::Relative));
            }
            if size > 2 {
                self.asm("PUSH_HL");
                self.word("LD_HL_IMM", symbol.value);
                self.asm("POP_DE");
                self.aggregate_copy_bytes(size);
                continue;
            }
            self.asm(if size == 1 { "LD_A_HL" } else { "LDI_A_HL" });
            self.store(memory_operand(symbol.value));
            if size == 2 {
                self.asm("LD_A_HL");
                self.store(memory_operand(symbol.value + 1));
            }
        }
    }
    fn stack_base_hl(&mut self) {
        if let Some(base) = self.stack_base {
            self.load(memory_operand(base));
            self.asm("LD_L_A");
            self.load(memory_operand(base + 1));
            self.asm("LD_H_A");
        } else {
            self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
        }
    }
    pub(super) fn stack_param_address(&mut self, offset: i32, comment: &str) {
        self.stack_base_hl();
        self.word("LD_DE_IMM", 2 + offset);
        self.asm("ADD_HL_DE");
        self.emit(t::COMMENT, vec![comment.into()]);
    }
    pub(super) fn return_from_function(&mut self) {
        if self.stack_base.is_some() {
            let ty = self.return_type.clone();
            let size = if matches!(ty.kind, Kind::Simple(Simple::Void)) {
                0
            } else if ty.is_aggregate() {
                1
            } else {
                self.env.size(&Expr::new(t::EMPTY, vec![]), &ty)
            };
            if size == 2 {
                self.asm("LD_D_H");
                self.asm("LD_E_L");
            } else if size == 1 {
                self.asm("LD_B_A");
            }
            self.stack_base_hl();
            self.asm("LD_SP_HL");
            if size == 2 {
                self.asm("LD_H_D");
                self.asm("LD_L_E");
            } else if size == 1 {
                self.asm("LD_A_B");
            }
        }
        self.asm("RET");
    }
    pub(super) fn stack_call(&mut self, e: &Expr, name: &str, info: &FunctionInfo, args: &[&Expr]) {
        let mut sizes = Vec::new();
        let mut total = 0;
        for p in &info.parameters {
            let size = self.env.size(e, &p.ty);
            if size != 1 && size != 2 && !p.ty.is_aggregate() {
                self.unsupported(e, "aggregate stack argument");
                return;
            }
            sizes.push(size);
            total += size;
        }
        if total > 127 {
            self.env
                .error(&e.source, "Too many stack arguments (total bytes > 127)");
            return;
        }
        if total > 0 {
            self.op("ADD_SP_IMM", Operand::integer(-total, M::Relative));
        }
        let mut offset = 0;
        for (i, (arg, size)) in args.iter().zip(sizes).enumerate() {
            let ty = &info.parameters[i].ty;
            if ty.is_aggregate() {
                self.aggregate_argument(e, name, i, arg, ty, None, Some(offset));
            } else if size == 1 {
                self.into_a(arg);
                self.op("LD_HL_SP_IMM", Operand::integer(offset, M::Relative));
                self.asm("LD_HL_A");
            } else {
                self.into_hl(arg);
                self.asm("LD_D_H");
                self.asm("LD_E_L");
                self.asm("LD_A_E");
                self.op("LD_HL_SP_IMM", Operand::integer(offset, M::Relative));
                self.asm("LD_HL_A");
                self.asm("LD_A_D");
                self.op("LD_HL_SP_IMM", Operand::integer(offset + 1, M::Relative));
                self.asm("LD_HL_A");
            }
            offset += size;
        }
        self.prepare_return_destination(info);
        let target = if info.rom_bank != 0 && info.rom_bank != self.current_bank {
            let thunk = self.bank_thunk(name, info.rom_bank);
            self.stack_thunks.insert(thunk.clone());
            thunk
        } else {
            self.direct_bank_check(info.rom_bank, false);
            name.into()
        };
        self.op("CALL", Operand::symbol(target, M::Absolute));
        let thunk = info.rom_bank != 0 && info.rom_bank != self.current_bank;
        self.report_call(
            e,
            name,
            info.rom_bank,
            if thunk { "bank_thunk" } else { "direct" },
            thunk,
            false,
            args,
            &info
                .parameters
                .iter()
                .map(|p| p.ty.clone())
                .collect::<Vec<_>>(),
        );
        if total > 0 {
            self.op("ADD_SP_IMM", Operand::integer(total, M::Relative));
        }
    }
    pub(super) fn stack_thunk_body(&mut self, thunk: &str, bank: i32, target: &str) {
        let init = Operand::symbol(format!("kq_thunk_depth_init_{thunk}"), M::Absolute);
        let ok = Operand::symbol(format!("kq_thunk_depth_ok_{thunk}"), M::Absolute);
        let trap = Operand::symbol("__kq_thunk_stack_trap", M::Absolute);
        self.op("LDH_A_MEM", Operand::integer(0x86, M::Immediate));
        self.immediate("CP_IMM", 0x80);
        self.op("JP_C", init.clone());
        self.immediate("CP_IMM", 0x89);
        self.op("JP_C", ok.clone());
        self.label(&init);
        self.immediate("LD_A_IMM", 0x80);
        self.op("LDH_MEM_A", Operand::integer(0x86, M::Immediate));
        self.label(&ok);
        self.immediate("CP_IMM", 0x88);
        self.op("JP_Z", trap.clone());
        self.asm("LD_D_A");
        self.asm("INC_A");
        self.op("LDH_MEM_A", Operand::integer(0x86, M::Immediate));
        self.asm("LD_A_D");
        self.immediate("AND_IMM", 15);
        self.asm("LD_C_A");
        self.immediate("LD_B_IMM", 0);
        self.asm("POP_DE");
        self.op("LDH_A_MEM", Operand::integer(0x82, M::Immediate));
        self.op("LD_HL_IMM", Operand::integer(0xff87, M::Immediate));
        self.asm("ADD_HL_BC");
        self.asm("LD_HL_A");
        self.asm("LD_A_E");
        self.op("LD_HL_IMM", Operand::integer(0xff8f, M::Immediate));
        self.asm("ADD_HL_BC");
        self.asm("LD_HL_A");
        self.asm("LD_A_D");
        self.op("LD_HL_IMM", Operand::integer(0xff97, M::Immediate));
        self.asm("ADD_HL_BC");
        self.asm("LD_HL_A");
        self.immediate("LD_A_IMM", bank);
        self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
        self.op("LDH_MEM_A", Operand::integer(0x82, M::Immediate));
        self.op("CALL", Operand::symbol(target, M::Absolute));
        self.asm("PUSH_AF");
        self.asm("PUSH_HL");
        self.op("LDH_A_MEM", Operand::integer(0x86, M::Immediate));
        self.immediate("CP_IMM", 0x81);
        self.op("JP_C", trap.clone());
        self.immediate("CP_IMM", 0x89);
        self.op("JP_NC", trap);
        self.asm("DEC_A");
        self.op("LDH_MEM_A", Operand::integer(0x86, M::Immediate));
        self.immediate("AND_IMM", 15);
        self.asm("LD_C_A");
        self.immediate("LD_B_IMM", 0);
        self.op("LD_HL_IMM", Operand::integer(0xff87, M::Immediate));
        self.asm("ADD_HL_BC");
        self.asm("LD_A_HL");
        self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
        self.op("LDH_MEM_A", Operand::integer(0x82, M::Immediate));
        self.op("LD_HL_IMM", Operand::integer(0xff8f, M::Immediate));
        self.asm("ADD_HL_BC");
        self.asm("LD_A_HL");
        self.asm("LD_E_A");
        self.op("LD_HL_IMM", Operand::integer(0xff97, M::Immediate));
        self.asm("ADD_HL_BC");
        self.asm("LD_A_HL");
        self.asm("LD_D_A");
        for m in ["POP_HL", "POP_AF", "PUSH_DE", "RET"] {
            self.asm(m);
        }
    }
}
