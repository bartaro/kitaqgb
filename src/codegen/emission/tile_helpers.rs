// Verified native templates for the shared tile map cores.
use super::*;
impl Emitter {
    pub(super) fn append_tile_helpers(&mut self) {
        let used: BTreeSet<String> = self
            .lines
            .iter()
            .filter_map(|e| e.asm_parts())
            .filter_map(|(_, o)| o.base.clone())
            .collect();
        if !used.iter().any(|s| {
            matches!(
                s.as_str(),
                "__settile_core"
                    | "__settile_fast_core"
                    | "__settile_bulk_core"
                    | "__settile_bulk_fast_core"
            )
        }) {
            return;
        }
        let body = std::mem::take(&mut self.lines);
        self.emit(
            t::COMMENT,
            vec!["[KITAQGB] injected v9 __settile call-core routines".into()],
        );
        if used.contains("__settile_core") {
            self.emit(t::FUNCTION, vec!["__settile_core".into()]);
            self.asm("LD_D_A");
            self.asm("LD_A_B");
            self.op("CP_IMM", Operand::integer(32, M::Immediate));
            self.op(
                "JP_NC",
                Operand::symbol("_kitaqgb_settile_v9_core_ret", M::Absolute),
            );
            self.asm("LD_A_D");
            self.op("CP_IMM", Operand::integer(32, M::Immediate));
            self.op(
                "JP_NC",
                Operand::symbol("_kitaqgb_settile_v9_core_ret", M::Absolute),
            );
            self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
            self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
            self.op(
                "JR_Z",
                Operand::symbol("_kitaqgb_settile_v9_core_write_now", M::Absolute),
            );
            self.asm("DI");
            self.asm("LD_A_D");
            self.asm("LD_L_A");
            self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
            self.asm("ADD_HL_DE");
            self.asm("LD_A_B");
            self.asm("LD_E_A");
            self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
            self.asm("ADD_HL_DE");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_v9_core_wait".into()]);
            self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
            self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
            self.op(
                "JR_NZ",
                Operand::symbol("_kitaqgb_settile_v9_core_wait", M::Absolute),
            );
            self.asm("LD_HL_C");
            self.asm("EI");
            self.asm("RET");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_v9_core_write_now".into()]);
            self.asm("LD_A_D");
            self.asm("LD_L_A");
            self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
            self.asm("ADD_HL_DE");
            self.asm("LD_A_B");
            self.asm("LD_E_A");
            self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
            self.asm("ADD_HL_DE");
            self.asm("LD_HL_C");
            self.asm("RET");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_v9_core_ret".into()]);
            self.asm("RET");
        }
        if used.contains("__settile_fast_core") {
            self.emit(t::FUNCTION, vec!["__settile_fast_core".into()]);
            self.asm("LD_D_A");
            self.asm("LD_A_B");
            self.op("CP_IMM", Operand::integer(32, M::Immediate));
            self.op(
                "JP_NC",
                Operand::symbol("_kitaqgb_settile_v9_fast_ret", M::Absolute),
            );
            self.asm("LD_A_D");
            self.op("CP_IMM", Operand::integer(32, M::Immediate));
            self.op(
                "JP_NC",
                Operand::symbol("_kitaqgb_settile_v9_fast_ret", M::Absolute),
            );
            self.asm("LD_L_A");
            self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.asm("ADD_HL_HL");
            self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
            self.asm("ADD_HL_DE");
            self.asm("LD_A_B");
            self.asm("LD_E_A");
            self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
            self.asm("ADD_HL_DE");
            self.asm("LD_HL_C");
            self.asm("RET");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_v9_fast_ret".into()]);
            self.asm("RET");
        }
        if used.contains("__settile_bulk_core") {
            self.emit(t::FUNCTION, vec!["__settile_bulk_core".into()]);
            self.asm("LD_A_B");
            self.asm("OR_A");
            self.op(
                "JP_Z",
                Operand::symbol("_kitaqgb_settile_bulk_v9_ret", M::Absolute),
            );
            self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
            self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
            self.op(
                "JR_Z",
                Operand::symbol("_kitaqgb_settile_bulk_v9_fast", M::Absolute),
            );
            self.asm("DI");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_v9_loop".into()]);
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_v9_wait".into()]);
            self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
            self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
            self.op(
                "JR_NZ",
                Operand::symbol("_kitaqgb_settile_bulk_v9_wait", M::Absolute),
            );
            self.asm("LD_A_DE");
            self.asm("LDI_HL_A");
            self.asm("INC_DE");
            self.asm("DEC_B");
            self.op(
                "JR_NZ",
                Operand::symbol("_kitaqgb_settile_bulk_v9_loop", M::Absolute),
            );
            self.asm("EI");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_v9_ret".into()]);
            self.asm("RET");
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_v9_fast".into()]);
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_v9_loop_off".into()]);
            self.asm("LD_A_DE");
            self.asm("LDI_HL_A");
            self.asm("INC_DE");
            self.asm("DEC_B");
            self.op(
                "JR_NZ",
                Operand::symbol("_kitaqgb_settile_bulk_v9_loop_off", M::Absolute),
            );
            self.asm("RET");
        }
        if used.contains("__settile_bulk_fast_core") {
            self.emit(t::FUNCTION, vec!["__settile_bulk_fast_core".into()]);
            self.asm("LD_A_B");
            self.asm("OR_A");
            self.op(
                "JP_Z",
                Operand::symbol("_kitaqgb_settile_bulk_fast_v9_ret", M::Absolute),
            );
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_fast_v9_loop".into()]);
            self.asm("LD_A_DE");
            self.asm("LDI_HL_A");
            self.asm("INC_DE");
            self.asm("DEC_B");
            self.op(
                "JR_NZ",
                Operand::symbol("_kitaqgb_settile_bulk_fast_v9_loop", M::Absolute),
            );
            self.emit(t::LABEL, vec!["_kitaqgb_settile_bulk_fast_v9_ret".into()]);
            self.asm("RET");
        }
        self.lines.extend(body);
    }
}
