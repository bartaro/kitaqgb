// Verified native instruction templates for split-scroll runtime helpers.
use super::*;
impl Emitter {
    pub(super) fn append_scroll_helpers(&mut self) {
        let referenced = self
            .lines
            .iter()
            .filter_map(|e| e.asm_parts())
            .any(|(_, o)| {
                matches!(
                    o.base.as_deref(),
                    Some(
                        "__kq_scroll_split_reset_core"
                            | "__kq_scroll_split_push_core"
                            | "__kq_scroll_split_commit_core"
                    )
                )
            });
        if !self.scroll_split && !referenced {
            return;
        }
        self.ensure_scroll();
        let body = std::mem::take(&mut self.lines);
        self.emit(
            t::COMMENT,
            vec!["[KITAQGB] injected scroll split helpers".into()],
        );
        self.emit(t::FUNCTION, vec!["__kq_scroll_split_reset_core".into()]);
        self.asm("XOR_A");
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_count"));
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_enabled"));
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_index"));
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("AND_IMM", Operand::integer(0xBF, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x41, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_scroll_split_reset_done".into()]);
        self.asm("RET");
        self.emit(t::FUNCTION, vec!["__kq_scroll_split_push_core".into()]);
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_count"));
        self.op("CP_IMM", Operand::integer(8, M::Immediate));
        self.op(
            "JR_C",
            Operand::symbol("__kq_scroll_split_push_have_room", M::Absolute),
        );
        self.asm("RET");
        self.emit(t::LABEL, vec!["__kq_scroll_split_push_have_room".into()]);
        self.asm("LD_E_A");
        self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
        self.asm("LD_L_A");
        self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_DE");
        self.asm("ADD_HL_HL");
        self.op(
            "LD_DE_IMM",
            Operand::integer(self.scroll_vars["__kq_scroll_split_table"], M::Immediate),
        );
        self.asm("ADD_HL_DE");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_tmp_ly"));
        self.asm("LDI_HL_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_tmp_scx"));
        self.asm("LDI_HL_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_tmp_scy"));
        self.asm("LDI_HL_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_tmp_wx"));
        self.asm("LDI_HL_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_tmp_wy"));
        self.asm("LDI_HL_A");
        self.op(
            "LD_A_MEM",
            self.scroll_operand("__kq_scroll_split_tmp_flags"),
        );
        self.op("AND_IMM", Operand::integer(0x1F, M::Immediate));
        self.asm("LDI_HL_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_count"));
        self.asm("INC_A");
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_count"));
        self.asm("RET");
        self.emit(t::FUNCTION, vec!["__kq_scroll_schedule_next".into()]);
        self.asm("LD_B_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_count"));
        self.asm("CP_B");
        self.op(
            "JR_C",
            Operand::symbol("__kq_scroll_schedule_next_disable", M::Absolute),
        );
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_schedule_next_disable", M::Absolute),
        );
        self.emit(
            t::LABEL,
            vec!["__kq_scroll_schedule_next_have_target".into()],
        );
        self.asm("LD_A_B");
        self.asm("LD_E_A");
        self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
        self.asm("LD_L_A");
        self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_DE");
        self.asm("ADD_HL_HL");
        self.op(
            "LD_DE_IMM",
            Operand::integer(self.scroll_vars["__kq_scroll_split_table"], M::Immediate),
        );
        self.asm("ADD_HL_DE");
        self.asm("LD_A_HL");
        self.op("LDH_MEM_A", Operand::integer(0x45, M::HighMem));
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("OR_IMM", Operand::integer(0x40, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x41, M::HighMem));
        self.asm("RET");
        self.emit(t::LABEL, vec!["__kq_scroll_schedule_next_disable".into()]);
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("AND_IMM", Operand::integer(0x40, M::Immediate));
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_schedule_next_disabled", M::Absolute),
        );
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("AND_IMM", Operand::integer(0xBF, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x41, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_scroll_schedule_next_disabled".into()]);
        self.asm("RET");
        self.emit(t::FUNCTION, vec!["__kq_scroll_apply_entry".into()]);
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.asm("LD_B_A");
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.asm("LD_C_A");
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.asm("LD_D_A");
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.asm("LD_E_A");
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.op("AND_IMM", Operand::integer(0x1F, M::Immediate));
        self.asm("LD_H_A");
        self.asm("LD_A_H");
        self.op("AND_IMM", Operand::integer(0x01, M::Immediate));
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_apply_entry_skip_bg", M::Absolute),
        );
        self.asm("LD_A_B");
        self.op("LDH_MEM_A", Operand::integer(0x43, M::HighMem));
        self.asm("LD_A_C");
        self.op("LDH_MEM_A", Operand::integer(0x42, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_scroll_apply_entry_skip_bg".into()]);
        self.asm("LD_A_H");
        self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_apply_entry_skip_win", M::Absolute),
        );
        self.asm("LD_A_D");
        self.op("LDH_MEM_A", Operand::integer(0x4B, M::HighMem));
        self.asm("LD_A_E");
        self.op("LDH_MEM_A", Operand::integer(0x4A, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_scroll_apply_entry_skip_win".into()]);
        self.asm("LD_A_H");
        self.op("AND_IMM", Operand::integer(0x08, M::Immediate));
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_apply_entry_skip_hide", M::Absolute),
        );
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.op("AND_IMM", Operand::integer(0xDF, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x40, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_scroll_apply_entry_skip_hide".into()]);
        self.asm("LD_A_H");
        self.op("AND_IMM", Operand::integer(0x04, M::Immediate));
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_apply_entry_skip_bg_color0", M::Absolute),
        );
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.op("OR_IMM", Operand::integer(0x20, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x40, M::HighMem));
        self.emit(
            t::LABEL,
            vec!["__kq_scroll_apply_entry_skip_bg_color0".into()],
        );
        self.asm("LD_A_H");
        self.op("AND_IMM", Operand::integer(0x10, M::Immediate));
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_apply_entry_done", M::Absolute),
        );
        self.emit(
            t::LABEL,
            vec!["__kq_scroll_apply_entry_wait_palette".into()],
        );
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
        self.op(
            "JR_NZ",
            Operand::symbol("__kq_scroll_apply_entry_wait_palette", M::Absolute),
        );
        self.op("LD_A_IMM", Operand::integer(0x80, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x68, M::HighMem));
        self.asm("LD_A_B");
        self.op("LDH_MEM_A", Operand::integer(0x69, M::HighMem));
        self.asm("LD_A_C");
        self.op("LDH_MEM_A", Operand::integer(0x69, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_scroll_apply_entry_done".into()]);
        self.asm("RET");
        self.emit(t::FUNCTION, vec!["__kq_scroll_split_commit_core".into()]);
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.op("AND_IMM", Operand::integer(0x20, M::Immediate));
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_win_visible"));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_count"));
        self.asm("OR_A");
        self.op(
            "JR_Z",
            Operand::symbol("__kq_scroll_split_commit_disable", M::Absolute),
        );
        self.op("LD_A_IMM", Operand::integer(1, M::Immediate));
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_enabled"));
        self.asm("XOR_A");
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_index"));
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_schedule_next", M::Absolute),
        );
        self.op("LD_A_MEM", Operand::integer(0xFFFF, M::Absolute));
        self.op("OR_IMM", Operand::integer(0x03, M::Immediate));
        self.op("LD_MEM_A", Operand::integer(0xFFFF, M::Absolute));
        self.asm("EI");
        self.asm("RET");
        self.emit(t::LABEL, vec!["__kq_scroll_split_commit_disable".into()]);
        self.asm("XOR_A");
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_enabled"));
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_index"));
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_schedule_next", M::Absolute),
        );
        self.asm("RET");
        self.emit(t::FUNCTION, vec!["__kq_vblank_vector".into()]);
        self.asm("PUSH_AF");
        self.asm("PUSH_BC");
        self.asm("PUSH_DE");
        self.asm("PUSH_HL");
        self.op("LDH_A_MEM", Operand::integer(0x70, M::HighMem));
        self.asm("PUSH_AF");
        self.op("LD_A_IMM", Operand::integer(1, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x70, M::HighMem));
        self.op("LD_A_IMM", Operand::integer(1, M::Immediate));
        self.op("LD_MEM_A", Operand::integer(0xC29C, M::Absolute));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_bg_x_cur"));
        self.op("LDH_MEM_A", Operand::integer(0x43, M::HighMem));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_bg_y_cur"));
        self.op("LDH_MEM_A", Operand::integer(0x42, M::HighMem));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_win_x_cur"));
        self.op("LDH_MEM_A", Operand::integer(0x4B, M::HighMem));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_win_y_cur"));
        self.op("LDH_MEM_A", Operand::integer(0x4A, M::HighMem));
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.op("AND_IMM", Operand::integer(0xDF, M::Immediate));
        self.asm("LD_B_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_win_visible"));
        self.asm("OR_B");
        self.op("LDH_MEM_A", Operand::integer(0x40, M::HighMem));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_enabled"));
        self.asm("OR_A");
        self.op("JR_Z", Operand::symbol("__kq_vblank_disable", M::Absolute));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_count"));
        self.asm("OR_A");
        self.op("JR_Z", Operand::symbol("__kq_vblank_disable", M::Absolute));
        self.asm("XOR_A");
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_index"));
        self.op(
            "LD_A_MEM",
            Operand::integer(self.scroll_vars["__kq_scroll_split_table"], M::Absolute),
        );
        self.asm("OR_A");
        self.op(
            "JR_NZ",
            Operand::symbol("__kq_vblank_first_nonzero", M::Absolute),
        );
        self.op(
            "LD_HL_IMM",
            Operand::integer(self.scroll_vars["__kq_scroll_split_table"], M::Immediate),
        );
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_apply_entry", M::Absolute),
        );
        self.op("LD_A_IMM", Operand::integer(1, M::Immediate));
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_index"));
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_schedule_next", M::Absolute),
        );
        self.op("JR", Operand::symbol("__kq_vblank_done", M::Absolute));
        self.emit(t::LABEL, vec!["__kq_vblank_first_nonzero".into()]);
        self.asm("XOR_A");
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_schedule_next", M::Absolute),
        );
        self.op("JR", Operand::symbol("__kq_vblank_done", M::Absolute));
        self.emit(t::LABEL, vec!["__kq_vblank_disable".into()]);
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_schedule_next", M::Absolute),
        );
        self.emit(t::LABEL, vec!["__kq_vblank_done".into()]);
        self.asm("POP_AF");
        self.op("LDH_MEM_A", Operand::integer(0x70, M::HighMem));
        self.asm("POP_HL");
        self.asm("POP_DE");
        self.asm("POP_BC");
        self.asm("POP_AF");
        self.asm("RETI");
        self.emit(t::FUNCTION, vec!["__kq_stat_vector".into()]);
        self.asm("PUSH_AF");
        self.asm("PUSH_BC");
        self.asm("PUSH_DE");
        self.asm("PUSH_HL");
        self.op("LDH_A_MEM", Operand::integer(0x70, M::HighMem));
        self.asm("PUSH_AF");
        self.op("LD_A_IMM", Operand::integer(1, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x70, M::HighMem));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_enabled"));
        self.asm("OR_A");
        self.op("JR_Z", Operand::symbol("__kq_stat_disable", M::Absolute));
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_index"));
        self.asm("LD_B_A");
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_count"));
        self.asm("CP_B");
        self.op("JR_C", Operand::symbol("__kq_stat_disable", M::Absolute));
        self.op("JR_Z", Operand::symbol("__kq_stat_disable", M::Absolute));
        self.asm("LD_A_B");
        self.asm("LD_E_A");
        self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
        self.asm("LD_L_A");
        self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_DE");
        self.asm("ADD_HL_HL");
        self.op(
            "LD_DE_IMM",
            Operand::integer(self.scroll_vars["__kq_scroll_split_table"], M::Immediate),
        );
        self.asm("ADD_HL_DE");
        self.asm("LD_A_HL");
        self.asm("LD_B_A");
        self.op("LDH_A_MEM", Operand::integer(0x44, M::HighMem));
        self.asm("CP_B");
        self.op("JR_NZ", Operand::symbol("__kq_stat_done", M::Absolute));
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_apply_entry", M::Absolute),
        );
        self.op("LD_A_MEM", self.scroll_operand("__kq_scroll_split_index"));
        self.asm("INC_A");
        self.op("LD_MEM_A", self.scroll_operand("__kq_scroll_split_index"));
        self.op(
            "CALL",
            Operand::symbol("__kq_scroll_schedule_next", M::Absolute),
        );
        self.op("JR", Operand::symbol("__kq_stat_done", M::Absolute));
        self.emit(t::LABEL, vec!["__kq_stat_disable".into()]);
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("AND_IMM", Operand::integer(0x40, M::Immediate));
        self.op("JR_Z", Operand::symbol("__kq_stat_done", M::Absolute));
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.op("AND_IMM", Operand::integer(0xBF, M::Immediate));
        self.op("LDH_MEM_A", Operand::integer(0x41, M::HighMem));
        self.emit(t::LABEL, vec!["__kq_stat_done".into()]);
        self.asm("POP_AF");
        self.op("LDH_MEM_A", Operand::integer(0x70, M::HighMem));
        self.asm("POP_HL");
        self.asm("POP_DE");
        self.asm("POP_BC");
        self.asm("POP_AF");
        self.asm("RETI");
        self.lines.extend(body);
    }
}
