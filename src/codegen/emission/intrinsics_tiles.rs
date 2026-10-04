use super::*;
impl Emitter {
    pub(super) fn tile_vbk(&mut self, value: i32) {
        self.report_cgb_write("VBK");
        self.immediate("LD_A_IMM", value & 1);
        self.op("LDH_MEM_A", Operand::integer(0x4f, M::HighMem));
    }
    pub(super) fn tile_call_core(&mut self, fast: bool) {
        self.op(
            "CALL",
            Operand::symbol(
                if fast {
                    "__settile_fast_core"
                } else {
                    "__settile_core"
                },
                M::Absolute,
            ),
        );
    }
    pub(super) fn tile_call_bulk(&mut self, fast: bool) {
        self.op(
            "CALL",
            Operand::symbol(
                if fast {
                    "__settile_bulk_fast_core"
                } else {
                    "__settile_bulk_core"
                },
                M::Absolute,
            ),
        );
    }
    pub(super) fn tile_map_base_de(&mut self, mask: i32, prefix: &str) {
        let base0 = self.unique(&format!("{prefix}_base0"));
        let done = self.unique(&format!("{prefix}_done"));
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.immediate("AND_IMM", mask);
        self.op("JR_Z", base0.clone());
        self.immediate("LD_DE_IMM", 0x9c00);
        self.op("JR", done.clone());
        self.label(&base0);
        self.immediate("LD_DE_IMM", 0x9800);
        self.label(&done);
    }
    pub(super) fn tile_write_target(&mut self, target: TileTarget, safe: bool, prefix: &str) {
        let suffix = match target {
            TileTarget::At => "at",
            TileTarget::Window => {
                self.asm("LD_H_A");
                self.tile_map_base_de(0x40, &format!("{prefix}_winbase"));
                self.asm("LD_A_H");
                "win"
            }
            TileTarget::Bg => {
                self.asm("LD_H_A");
                self.tile_map_base_de(8, &format!("{prefix}_bgbase"));
                self.asm("LD_A_H");
                "bg"
            }
        };
        self.emit_tile_write_at_de_from_regs(safe, &format!("{prefix}_{suffix}"));
    }
    pub(super) fn tile_map_address(&mut self, base: &Expr, x: &Expr, y: &Expr) {
        self.tile_address(base, x, y);
    }
    pub(super) fn tile_map_base(&mut self, mask: i32, prefix: &str) {
        let base0 = self.unique(&format!("{prefix}_base0"));
        let done = self.unique(&format!("{prefix}_done"));
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.immediate("AND_IMM", mask);
        self.op("JR_Z", base0.clone());
        self.word("LD_HL_IMM", 0x9c00);
        self.op("JR", done.clone());
        self.label(&base0);
        self.word("LD_HL_IMM", 0x9800);
        self.label(&done);
    }
    pub(super) fn vram_copy_loop(&mut self, safe: bool, prefix: &str) {
        let fast = self.unique(&format!("{prefix}_fast"));
        let loop_safe = self.unique(&format!("{prefix}_safe_loop"));
        let wait = self.unique(&format!("{prefix}_safe_wait"));
        let loop_fast = self.unique(&format!("{prefix}_fast_loop"));
        let done = self.unique(&format!("{prefix}_done"));
        self.asm("LD_A_B");
        self.asm("OR_C");
        self.op("JR_Z", done.clone());
        if safe {
            self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
            self.immediate("AND_IMM", 128);
            self.op("JR_Z", fast.clone());
            self.label(&loop_safe);
            self.asm("LD_A_B");
            self.asm("OR_C");
            self.op("JR_Z", done.clone());
            self.asm("DI");
            self.label(&wait);
            self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
            self.immediate("AND_IMM", 2);
            self.op("JR_NZ", wait);
            self.asm("LD_A_DE");
            self.asm("LDI_HL_A");
            self.asm("EI");
            self.asm("INC_DE");
            self.asm("DEC_BC");
            self.op("JR", loop_safe);
        }
        self.label(&fast);
        self.label(&loop_fast);
        self.asm("LD_A_B");
        self.asm("OR_C");
        self.op("JR_Z", done.clone());
        self.asm("LD_A_DE");
        self.asm("LDI_HL_A");
        self.asm("INC_DE");
        self.asm("DEC_BC");
        self.op("JR", loop_fast);
        self.label(&done);
    }
    pub(super) fn vram_store_c(&mut self, safe: bool, prefix: &str) {
        if !safe {
            self.asm("LD_HL_C");
            return;
        }
        let now = self.unique(&format!("{prefix}_write_now"));
        let wait = self.unique(&format!("{prefix}_wait"));
        let done = self.unique(&format!("{prefix}_done"));
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.immediate("AND_IMM", 128);
        self.op("JR_Z", now.clone());
        self.asm("DI");
        self.label(&wait);
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.immediate("AND_IMM", 2);
        self.op("JR_NZ", wait);
        self.asm("LD_HL_C");
        self.asm("EI");
        self.op("JR", done.clone());
        self.label(&now);
        self.asm("LD_HL_C");
        self.label(&done);
    }
    pub(super) fn vram_column_const(&mut self, safe: bool, count: i32, prefix: &str) -> bool {
        if !(0..=8).contains(&count) {
            return false;
        }
        let done = if safe && count > 0 {
            let fast = self.unique(&format!("{prefix}_fast"));
            let done = self.unique(&format!("{prefix}_done"));
            self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
            self.immediate("AND_IMM", 128);
            self.op("JP_Z", fast.clone());
            for i in 0..count {
                let wait = self.unique(&format!("{prefix}_safe_wait"));
                self.asm("DI");
                self.label(&wait);
                self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
                self.immediate("AND_IMM", 2);
                self.op("JR_NZ", wait);
                self.asm("LD_A_DE");
                self.asm("LD_HL_A");
                self.asm("EI");
                self.asm("INC_DE");
                if i + 1 < count {
                    self.tile_column_stride();
                }
            }
            self.op("JP", done.clone());
            self.label(&fast);
            Some(done)
        } else {
            None
        };
        for i in 0..count {
            self.asm("LD_A_DE");
            self.asm("LD_HL_A");
            self.asm("INC_DE");
            if i + 1 < count {
                self.tile_column_stride();
            }
        }
        if let Some(done) = done {
            self.label(&done);
        }
        true
    }
    fn tile_column_stride(&mut self) {
        self.asm("LD_A_L");
        self.immediate("ADD_A_IMM", 32);
        self.asm("LD_L_A");
        self.asm("LD_A_H");
        self.immediate("ADC_IMM", 0);
        self.asm("LD_H_A");
    }
    pub(super) fn tile_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        if self.cgb_tile_intrinsic(e, name, args) {
            return true;
        }
        if self.tile_region_intrinsic(e, name, args) {
            return true;
        }
        let safe = match name {
            "__settile_bulk" => true,
            "__settile_bulk_fast" => false,
            _ => return false,
        };
        if args.len() != 3 {
            self.env
                .error(&e.source, format!("{name} requires 3 arguments"));
            return true;
        }
        let count = self.env.fold(args[2]);
        self.into_hl(args[0]);
        self.asm("PUSH_HL");
        self.into_hl(args[1]);
        self.asm("PUSH_HL");
        self.into_a(&count);
        self.asm("LD_B_A");
        self.asm("POP_DE");
        self.asm("POP_HL");
        let prefix = if safe {
            "settilebulk_small"
        } else {
            "settilebulkf_small"
        };
        if count.is(t::INTEGER) && self.vram_copy_const(safe, count.int(1).unwrap() & 255, prefix) {
            return true;
        }
        self.op(
            "CALL",
            Operand::symbol(
                if safe {
                    "__settile_bulk_core"
                } else {
                    "__settile_bulk_fast_core"
                },
                M::Absolute,
            ),
        );
        true
    }
    pub(super) fn vram_copy_const(&mut self, safe: bool, count: i32, prefix: &str) -> bool {
        if !(0..=8).contains(&count) {
            return false;
        }
        let done = if safe && count > 0 {
            let fast = self.unique(&format!("{prefix}_fast"));
            let done = self.unique(&format!("{prefix}_done"));
            self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
            self.immediate("AND_IMM", 0x80);
            self.op("JP_Z", fast.clone());
            for _ in 0..count {
                let wait = self.unique(&format!("{prefix}_safe_wait"));
                self.asm("DI");
                self.label(&wait);
                self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
                self.immediate("AND_IMM", 2);
                self.op("JR_NZ", wait);
                self.asm("LD_A_DE");
                self.asm("LDI_HL_A");
                self.asm("EI");
                self.asm("INC_DE");
            }
            self.op("JP", done.clone());
            self.label(&fast);
            Some(done)
        } else {
            None
        };
        for _ in 0..count {
            self.asm("LD_A_DE");
            self.asm("LDI_HL_A");
            self.asm("INC_DE");
        }
        if let Some(done) = done {
            self.label(&done);
        }
        true
    }
}
