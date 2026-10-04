// Verified branch-only native templates for CGB attributes and buffered 16-pixel tiles.
use super::*;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum TileTarget {
    At,
    Window,
    Bg,
}
impl Emitter {
    pub(super) fn emit_compute_tile16_cell_offset_to_hl_from_regs(
        &mut self,
        double_x: bool,
        double_y: bool,
    ) {
        self.asm("LD_A_D");
        self.asm("LD_L_A");
        self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_HL");
        if double_y {
            self.asm("ADD_HL_HL");
        }
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("LD_A_B");
        if double_x {
            self.asm("RLCA");
        }
        self.asm("LD_E_A");
        self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_DE");
    }
    pub(super) fn emit_tile16_buffered_write_intrinsic(
        &mut self,
        buf_expr: &Expr,
        x_expr: &Expr,
        y_expr: &Expr,
        value_expr: &Expr,
        increment_quad: bool,
        _label_prefix: &str,
    ) {
        self.into_hl(buf_expr);
        self.asm("PUSH_HL");
        self.into_a(x_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(y_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(value_expr);
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_B_A");
        self.emit_compute_tile16_cell_offset_to_hl_from_regs(true, true);
        self.asm("POP_DE");
        self.asm("ADD_HL_DE");
        self.asm("LD_HL_C");
        self.asm("INC_HL");
        if increment_quad {
            self.asm("INC_C");
        }
        self.asm("LD_HL_C");
        self.op("LD_DE_IMM", Operand::integer(31, M::Immediate));
        self.asm("ADD_HL_DE");
        if increment_quad {
            self.asm("INC_C");
        }
        self.asm("LD_HL_C");
        self.asm("INC_HL");
        if increment_quad {
            self.asm("INC_C");
        }
        self.asm("LD_HL_C");
    }
    pub(super) fn emit_tile16_flush_rows_intrinsic(
        &mut self,
        buf_expr: &Expr,
        y_expr: &Expr,
        x_expr: &Expr,
        count_expr: &Expr,
        cgb_attr: bool,
        label_prefix: &str,
    ) {
        let folded_count_expr = self.env.fold(count_expr);
        self.into_hl(buf_expr);
        self.asm("PUSH_HL");
        self.into_a(y_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(x_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(&folded_count_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_B_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("PUSH_HL");
        self.emit_compute_tile16_cell_offset_to_hl_from_regs(false, true);
        self.asm("POP_DE");
        self.asm("PUSH_HL");
        self.asm("ADD_HL_DE");
        self.asm("LD_D_H");
        self.asm("LD_E_L");
        self.asm("POP_HL");
        self.asm("PUSH_DE");
        self.tile_map_base_de(0x08, &format!("{label_prefix}{}", "_bgbase"));
        self.asm("ADD_HL_DE");
        self.asm("POP_DE");
        self.asm("PUSH_HL");
        self.asm("PUSH_DE");
        self.asm("LD_B_C");
        if folded_count_expr.is(t::INTEGER)
            && self.vram_copy_const(
                true,
                folded_count_expr.int(1).unwrap() & 255,
                &format!("{label_prefix}{}", "_row0_small"),
            )
        {
        } else {
            self.tile_call_bulk(false);
        }
        self.asm("POP_DE");
        self.asm("POP_HL");
        self.asm("PUSH_DE");
        self.op("LD_DE_IMM", Operand::integer(32, M::Immediate));
        self.asm("ADD_HL_DE");
        self.asm("POP_DE");
        self.asm("PUSH_HL");
        self.asm("LD_H_D");
        self.asm("LD_L_E");
        self.op("LD_DE_IMM", Operand::integer(32, M::Immediate));
        self.asm("ADD_HL_DE");
        self.asm("LD_D_H");
        self.asm("LD_E_L");
        self.asm("POP_HL");
        self.asm("LD_B_C");
        if folded_count_expr.is(t::INTEGER)
            && self.vram_copy_const(
                true,
                folded_count_expr.int(1).unwrap() & 255,
                &format!("{label_prefix}{}", "_row1_small"),
            )
        {
        } else {
            self.tile_call_bulk(false);
        }
        if cgb_attr {
            self.tile_vbk(0);
        }
    }
    pub(super) fn emit_tile_write_at_de_from_regs(&mut self, safe: bool, label_prefix: &str) {
        let end = self.unique(&format!("{label_prefix}{}", "_end"));
        self.asm("LD_L_A");
        self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
        self.asm("LD_A_B");
        self.op("CP_IMM", Operand::integer(32, M::Immediate));
        self.op("JR_NC", end.clone());
        self.asm("LD_A_L");
        self.op("CP_IMM", Operand::integer(32, M::Immediate));
        self.op("JR_NC", end.clone());
        self.asm("LD_L_A");
        self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_HL");
        self.asm("ADD_HL_DE");
        self.asm("LD_A_B");
        self.asm("LD_E_A");
        self.op("LD_D_IMM", Operand::integer(0, M::Immediate));
        self.asm("ADD_HL_DE");
        if safe {
            let write_now = self.unique(&format!("{label_prefix}{}", "_write_now"));
            let wait = self.unique(&format!("{label_prefix}{}", "_wait"));
            let done = self.unique(&format!("{label_prefix}{}", "_done"));
            self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
            self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
            self.op("JR_Z", write_now.clone());
            self.asm("DI");
            self.label(&wait);
            self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
            self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
            self.op("JR_NZ", wait.clone());
            self.asm("LD_HL_C");
            self.asm("EI");
            self.op("JR", done.clone());
            self.label(&write_now);
            self.asm("LD_HL_C");
            self.label(&done);
        } else {
            self.asm("LD_HL_C");
        }
        self.label(&end);
    }
    pub(super) fn emit_set_tile_attr_intrinsic(
        &mut self,
        x_expr: &Expr,
        y_expr: &Expr,
        attr_expr: &Expr,
        fast: bool,
    ) {
        if self.env.options.known_cgb.is_some_and(|n| n != 0) {
            self.into_a(x_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(y_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(attr_expr);
            self.asm("LD_C_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_D_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_B_A");
            self.tile_vbk(1);
            self.asm("LD_A_D");
            self.tile_call_core(fast);
            self.tile_vbk(0);
            return;
        }
        let skip = self.unique("settileattr_skip");
        let done = self.unique("settileattr_done");
        self.into_a(x_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(y_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(attr_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.cgb_check();
        self.asm("OR_A");
        self.op("JR_Z", skip.clone());
        self.tile_vbk(1);
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_B_A");
        self.asm("LD_A_D");
        self.tile_call_core(fast);
        self.tile_vbk(0);
        self.op("JR", done.clone());
        self.label(&skip);
        for _ in 0..3 {
            self.asm("POP_HL");
        }
        self.label(&done);
    }
    pub(super) fn emit_set_tile_cgb_intrinsic(
        &mut self,
        x_expr: &Expr,
        y_expr: &Expr,
        tile_expr: &Expr,
        attr_expr: &Expr,
        fast: bool,
    ) {
        if self.env.options.known_cgb.is_some_and(|n| n != 0) {
            self.into_a(x_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(y_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(tile_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(attr_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_D_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_C_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("POP_HL");
            self.asm("LD_B_L");
            self.asm("LD_L_B");
            self.immediate("LD_H_IMM", 0);
            self.asm("PUSH_HL");
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.asm("LD_L_D");
            self.immediate("LD_H_IMM", 0);
            self.asm("PUSH_HL");
            self.tile_call_core(fast);
            self.tile_vbk(1);
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_C_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_D_A");
            self.asm("POP_HL");
            self.asm("LD_B_L");
            self.asm("LD_A_D");
            self.tile_call_core(fast);
            self.tile_vbk(0);
            return;
        }
        let skip = self.unique("settilecgb_skip");
        let done = self.unique("settilecgb_done");
        self.into_a(x_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(y_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(tile_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(attr_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("POP_HL");
        self.asm("LD_B_L");
        self.asm("LD_L_B");
        self.immediate("LD_H_IMM", 0);
        self.asm("PUSH_HL");
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.asm("LD_L_D");
        self.immediate("LD_H_IMM", 0);
        self.asm("PUSH_HL");
        self.tile_call_core(fast);
        self.cgb_check();
        self.asm("OR_A");
        self.op("JR_Z", skip.clone());
        self.tile_vbk(1);
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_B_A");
        self.asm("LD_A_D");
        self.tile_call_core(fast);
        self.tile_vbk(0);
        self.op("JR", done.clone());
        self.label(&skip);
        for _ in 0..3 {
            self.asm("POP_HL");
        }
        self.label(&done);
    }
    pub(super) fn emit_tile_target_attr_intrinsic(
        &mut self,
        target: TileTarget,
        base_expr: &Expr,
        x_expr: &Expr,
        y_expr: &Expr,
        attr_expr: &Expr,
        safe: bool,
        label_prefix: &str,
    ) {
        if self.env.options.known_cgb.is_some_and(|n| n != 0) {
            if target == TileTarget::At {
                self.into_hl(base_expr);
                self.asm("PUSH_HL");
            }
            self.into_a(x_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(y_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(attr_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.tile_vbk(1);
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_C_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("POP_HL");
            self.asm("LD_B_L");
            if target == TileTarget::At {
                self.asm("POP_HL");
                self.asm("LD_D_H");
                self.asm("LD_E_L");
            }
            self.tile_write_target(target, safe, label_prefix);
            self.tile_vbk(0);
            return;
        }
        let skip = self.unique(&format!("{label_prefix}{}", "_skip"));
        let done = self.unique(&format!("{label_prefix}{}", "_done"));
        let word_count = if target == TileTarget::At { 4 } else { 3 };
        if target == TileTarget::At {
            self.into_hl(base_expr);
            self.asm("PUSH_HL");
        }
        self.into_a(x_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(y_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(attr_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.cgb_check();
        self.asm("OR_A");
        self.op("JR_Z", skip.clone());
        self.tile_vbk(1);
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("POP_HL");
        self.asm("LD_B_L");
        if target == TileTarget::At {
            self.asm("POP_HL");
            self.asm("LD_D_H");
            self.asm("LD_E_L");
        }
        self.tile_write_target(target, safe, label_prefix);
        self.tile_vbk(0);
        self.op("JR", done.clone());
        self.label(&skip);
        for _ in 0..word_count {
            self.asm("POP_HL");
        }
        self.label(&done);
    }
    pub(super) fn emit_tile_target_cgb_intrinsic(
        &mut self,
        target: TileTarget,
        base_expr: &Expr,
        x_expr: &Expr,
        y_expr: &Expr,
        tile_expr: &Expr,
        attr_expr: &Expr,
        safe: bool,
        label_prefix: &str,
    ) {
        if self.env.options.known_cgb.is_some_and(|n| n != 0) {
            if target == TileTarget::At {
                self.into_hl(base_expr);
                self.asm("PUSH_HL");
            }
            self.into_a(x_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(y_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(tile_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.into_a(attr_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_D_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_C_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("POP_HL");
            self.asm("LD_B_L");
            if target == TileTarget::At {
                self.asm("POP_HL");
                self.asm("PUSH_HL");
                self.asm("PUSH_DE");
                self.asm("LD_D_H");
                self.asm("LD_E_L");
                self.asm("LD_L_B");
                self.immediate("LD_H_IMM", 0);
                self.asm("PUSH_HL");
                self.extend_a(false);
                self.asm("PUSH_HL");
            } else {
                self.asm("LD_L_B");
                self.immediate("LD_H_IMM", 0);
                self.asm("PUSH_HL");
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.asm("LD_L_D");
                self.immediate("LD_H_IMM", 0);
                self.asm("PUSH_HL");
            }
            self.tile_write_target(target, safe, &format!("{label_prefix}{}", "_tile"));
            self.tile_vbk(1);
            if target == TileTarget::At {
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("POP_HL");
                self.asm("LD_B_L");
                self.asm("POP_HL");
                self.asm("LD_C_H");
                self.asm("POP_HL");
                self.asm("LD_D_H");
                self.asm("LD_E_L");
            } else {
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("POP_HL");
                self.asm("LD_B_L");
            }
            self.tile_write_target(target, safe, &format!("{label_prefix}{}", "_attr"));
            self.tile_vbk(0);
            return;
        }
        let skip = self.unique(&format!("{label_prefix}{}", "_skip"));
        let done = self.unique(&format!("{label_prefix}{}", "_done"));
        let word_count = if target == TileTarget::At { 4 } else { 3 };
        if target == TileTarget::At {
            self.into_hl(base_expr);
            self.asm("PUSH_HL");
        }
        self.into_a(x_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(y_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(tile_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.into_a(attr_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("LD_C_A");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("POP_HL");
        self.asm("LD_B_L");
        if target == TileTarget::At {
            self.asm("POP_HL");
            self.asm("PUSH_HL");
            self.asm("PUSH_DE");
            self.asm("LD_D_H");
            self.asm("LD_E_L");
            self.asm("LD_L_B");
            self.immediate("LD_H_IMM", 0);
            self.asm("PUSH_HL");
            self.extend_a(false);
            self.asm("PUSH_HL");
        } else {
            self.asm("LD_L_B");
            self.immediate("LD_H_IMM", 0);
            self.asm("PUSH_HL");
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.asm("LD_L_D");
            self.immediate("LD_H_IMM", 0);
            self.asm("PUSH_HL");
        }
        self.tile_write_target(target, safe, &format!("{label_prefix}{}", "_tile"));
        self.cgb_check();
        self.asm("OR_A");
        self.op("JR_Z", skip.clone());
        self.tile_vbk(1);
        if target == TileTarget::At {
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("POP_HL");
            self.asm("LD_B_L");
            self.asm("POP_HL");
            self.asm("LD_C_H");
            self.asm("POP_HL");
            self.asm("LD_D_H");
            self.asm("LD_E_L");
        } else {
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("LD_C_A");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("POP_HL");
            self.asm("LD_B_L");
        }
        self.tile_write_target(target, safe, &format!("{label_prefix}{}", "_attr"));
        self.tile_vbk(0);
        self.op("JR", done.clone());
        self.label(&skip);
        for _ in 0..word_count {
            self.asm("POP_HL");
        }
        self.label(&done);
    }
    pub(super) fn emit_set_tile_attr_bulk_intrinsic(
        &mut self,
        dest_expr: &Expr,
        src_expr: &Expr,
        count_expr: &Expr,
        fast: bool,
        label_prefix: &str,
    ) {
        let folded_count_expr = self.env.fold(count_expr);
        if self.env.options.known_cgb.is_some_and(|n| n != 0) {
            self.into_hl(dest_expr);
            self.asm("PUSH_HL");
            self.into_hl(src_expr);
            self.asm("PUSH_HL");
            self.into_a(&folded_count_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.tile_vbk(1);
            self.asm("POP_HL");
            self.asm("LD_B_L");
            self.asm("POP_DE");
            self.asm("POP_HL");
            if folded_count_expr.is(t::INTEGER)
                && self.vram_copy_const(
                    !fast,
                    folded_count_expr.int(1).unwrap() & 255,
                    &format!("{label_prefix}{}", "_small"),
                )
            {
            } else {
                self.tile_call_bulk(fast);
            }
            self.tile_vbk(0);
            return;
        }
        let skip = self.unique(&format!("{label_prefix}{}", "_skip"));
        let done = self.unique(&format!("{label_prefix}{}", "_done"));
        self.into_hl(dest_expr);
        self.asm("PUSH_HL");
        self.into_hl(src_expr);
        self.asm("PUSH_HL");
        self.into_a(&folded_count_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.cgb_check();
        self.asm("OR_A");
        self.op("JR_Z", skip.clone());
        self.tile_vbk(1);
        self.asm("POP_HL");
        self.asm("LD_B_L");
        self.asm("POP_DE");
        self.asm("POP_HL");
        if folded_count_expr.is(t::INTEGER)
            && self.vram_copy_const(
                !fast,
                folded_count_expr.int(1).unwrap() & 255,
                &format!("{label_prefix}{}", "_small"),
            )
        {
        } else {
            self.tile_call_bulk(fast);
        }
        self.tile_vbk(0);
        self.op("JP", done.clone());
        self.label(&skip);
        for _ in 0..3 {
            self.asm("POP_HL");
        }
        self.label(&done);
    }
    pub(super) fn emit_set_tile_cgb_bulk_intrinsic(
        &mut self,
        dest_expr: &Expr,
        tile_src_expr: &Expr,
        attr_src_expr: &Expr,
        count_expr: &Expr,
        fast: bool,
        label_prefix: &str,
    ) {
        let folded_count_expr = self.env.fold(count_expr);
        if self.env.options.known_cgb.is_some_and(|n| n != 0) {
            self.into_hl(dest_expr);
            self.asm("PUSH_HL");
            self.into_hl(tile_src_expr);
            self.asm("PUSH_HL");
            self.into_hl(attr_src_expr);
            self.asm("PUSH_HL");
            self.into_a(&folded_count_expr);
            self.extend_a(false);
            self.asm("PUSH_HL");
            self.asm("POP_HL");
            self.asm("LD_A_L");
            self.asm("POP_DE");
            self.asm("POP_BC");
            self.asm("POP_HL");
            self.asm("PUSH_HL");
            self.asm("PUSH_DE");
            self.asm("LD_D_B");
            self.asm("LD_E_C");
            self.asm("LD_C_A");
            self.asm("XOR_A");
            self.asm("LD_B_A");
            self.asm("PUSH_BC");
            self.asm("LD_B_C");
            if folded_count_expr.is(t::INTEGER)
                && self.vram_copy_const(
                    !fast,
                    folded_count_expr.int(1).unwrap() & 255,
                    &format!("{label_prefix}{}", "_tile_small"),
                )
            {
            } else {
                self.tile_call_bulk(fast);
            }
            self.tile_vbk(1);
            self.asm("POP_BC");
            self.asm("POP_DE");
            self.asm("POP_HL");
            self.asm("LD_B_C");
            if folded_count_expr.is(t::INTEGER)
                && self.vram_copy_const(
                    !fast,
                    folded_count_expr.int(1).unwrap() & 255,
                    &format!("{label_prefix}{}", "_attr_small"),
                )
            {
            } else {
                self.tile_call_bulk(fast);
            }
            self.tile_vbk(0);
            return;
        }
        let skip = self.unique(&format!("{label_prefix}{}", "_skip"));
        let done = self.unique(&format!("{label_prefix}{}", "_done"));
        self.into_hl(dest_expr);
        self.asm("PUSH_HL");
        self.into_hl(tile_src_expr);
        self.asm("PUSH_HL");
        self.into_hl(attr_src_expr);
        self.asm("PUSH_HL");
        self.into_a(&folded_count_expr);
        self.extend_a(false);
        self.asm("PUSH_HL");
        self.asm("POP_HL");
        self.asm("LD_A_L");
        self.asm("POP_DE");
        self.asm("POP_BC");
        self.asm("POP_HL");
        self.asm("PUSH_HL");
        self.asm("PUSH_DE");
        self.asm("LD_D_B");
        self.asm("LD_E_C");
        self.asm("LD_C_A");
        self.asm("XOR_A");
        self.asm("LD_B_A");
        self.asm("PUSH_BC");
        self.asm("LD_B_C");
        if folded_count_expr.is(t::INTEGER)
            && self.vram_copy_const(
                !fast,
                folded_count_expr.int(1).unwrap() & 255,
                &format!("{label_prefix}{}", "_tile_small"),
            )
        {
        } else {
            self.tile_call_bulk(fast);
        }
        self.cgb_check();
        self.asm("OR_A");
        self.op("JR_Z", skip.clone());
        self.tile_vbk(1);
        self.asm("POP_BC");
        self.asm("POP_DE");
        self.asm("POP_HL");
        self.asm("LD_B_C");
        if folded_count_expr.is(t::INTEGER)
            && self.vram_copy_const(
                !fast,
                folded_count_expr.int(1).unwrap() & 255,
                &format!("{label_prefix}{}", "_attr_small"),
            )
        {
        } else {
            self.tile_call_bulk(fast);
        }
        self.tile_vbk(0);
        self.op("JP", done.clone());
        self.label(&skip);
        for _ in 0..3 {
            self.asm("POP_HL");
        }
        self.label(&done);
    }
    pub(super) fn cgb_tile_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        match name {
            "__settileattr" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settileattr requires 3 arguments");
                    return true;
                }
                self.emit_set_tile_attr_intrinsic(args[0], args[1], args[2], false);
                return true;
            }
            "__settileattr_unsafe" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settileattr_unsafe requires 3 arguments");
                    return true;
                }
                self.emit_set_tile_attr_intrinsic(args[0], args[1], args[2], true);
                return true;
            }
            "__settilecgb" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilecgb requires 4 arguments");
                    return true;
                }
                self.emit_set_tile_cgb_intrinsic(args[0], args[1], args[2], args[3], false);
                return true;
            }
            "__settilecgb_unsafe" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilecgb_unsafe requires 4 arguments");
                    return true;
                }
                self.emit_set_tile_cgb_intrinsic(args[0], args[1], args[2], args[3], true);
                return true;
            }
            "__settileatattr" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settileatattr requires 4 arguments");
                    return true;
                }
                self.emit_tile_target_attr_intrinsic(
                    TileTarget::At,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    true,
                    "settileatattr",
                );
                return true;
            }
            "__settileatattr_unsafe" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settileatattr_unsafe requires 4 arguments");
                    return true;
                }
                self.emit_tile_target_attr_intrinsic(
                    TileTarget::At,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    false,
                    "settileatattru",
                );
                return true;
            }
            "__settileatcgb" => {
                if args.len() != 5 {
                    self.env
                        .error(&e.source, "__settileatcgb requires 5 arguments");
                    return true;
                }
                self.emit_tile_target_cgb_intrinsic(
                    TileTarget::At,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    true,
                    "settileatcgb",
                );
                return true;
            }
            "__settileatcgb_unsafe" => {
                if args.len() != 5 {
                    self.env
                        .error(&e.source, "__settileatcgb_unsafe requires 5 arguments");
                    return true;
                }
                self.emit_tile_target_cgb_intrinsic(
                    TileTarget::At,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    false,
                    "settileatcgbu",
                );
                return true;
            }
            "__settilewinattr" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilewinattr requires 3 arguments");
                    return true;
                }
                self.emit_tile_target_attr_intrinsic(
                    TileTarget::Window,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    true,
                    "settilewinattr",
                );
                return true;
            }
            "__settilewinattr_unsafe" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilewinattr_unsafe requires 3 arguments");
                    return true;
                }
                self.emit_tile_target_attr_intrinsic(
                    TileTarget::Window,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    false,
                    "settilewinattru",
                );
                return true;
            }
            "__settilewincgb" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilewincgb requires 4 arguments");
                    return true;
                }
                self.emit_tile_target_cgb_intrinsic(
                    TileTarget::Window,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    true,
                    "settilewincgb",
                );
                return true;
            }
            "__settilewincgb_unsafe" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilewincgb_unsafe requires 4 arguments");
                    return true;
                }
                self.emit_tile_target_cgb_intrinsic(
                    TileTarget::Window,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    false,
                    "settilewincgbu",
                );
                return true;
            }
            "__settilebgattr" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilebgattr requires 3 arguments");
                    return true;
                }
                self.emit_tile_target_attr_intrinsic(
                    TileTarget::Bg,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    true,
                    "settilebgattr",
                );
                return true;
            }
            "__settilebgattr_unsafe" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilebgattr_unsafe requires 3 arguments");
                    return true;
                }
                self.emit_tile_target_attr_intrinsic(
                    TileTarget::Bg,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    false,
                    "settilebgattru",
                );
                return true;
            }
            "__settilebgcgb" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilebgcgb requires 4 arguments");
                    return true;
                }
                self.emit_tile_target_cgb_intrinsic(
                    TileTarget::Bg,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    true,
                    "settilebgcgb",
                );
                return true;
            }
            "__settilebgcgb_unsafe" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilebgcgb_unsafe requires 4 arguments");
                    return true;
                }
                self.emit_tile_target_cgb_intrinsic(
                    TileTarget::Bg,
                    &args[0],
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    false,
                    "settilebgcgbu",
                );
                return true;
            }
            "__settileattr_bulk" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settileattr_bulk requires 3 arguments");
                    return true;
                }
                self.emit_set_tile_attr_bulk_intrinsic(
                    args[0],
                    args[1],
                    args[2],
                    false,
                    "settileattrbulk",
                );
                return true;
            }
            "__settileattr_bulk_fast" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settileattr_bulk_fast requires 3 arguments");
                    return true;
                }
                self.emit_set_tile_attr_bulk_intrinsic(
                    args[0],
                    args[1],
                    args[2],
                    true,
                    "settileattrbulkf",
                );
                return true;
            }
            "__settilecgb_bulk" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilecgb_bulk requires 4 arguments");
                    return true;
                }
                self.emit_set_tile_cgb_bulk_intrinsic(
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    false,
                    "settilecgbbulk",
                );
                return true;
            }
            "__settilecgb_bulk_fast" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilecgb_bulk_fast requires 4 arguments");
                    return true;
                }
                self.emit_set_tile_cgb_bulk_intrinsic(
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    true,
                    "settilecgbbulkf",
                );
                return true;
            }
            "__settilebg16_buf" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilebg16_buf requires 4 arguments");
                    return true;
                }
                self.emit_tile16_buffered_write_intrinsic(
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    true,
                    "settilebg16buf",
                );
                return true;
            }
            "__settilebg16cgb_buf" => {
                if args.len() != 6 {
                    self.env
                        .error(&e.source, "__settilebg16cgb_buf requires 6 arguments");
                    return true;
                }
                self.emit_tile16_buffered_write_intrinsic(
                    args[0],
                    args[2],
                    args[3],
                    args[4],
                    true,
                    "settilebg16cgbbuf_tile",
                );
                self.emit_tile16_buffered_write_intrinsic(
                    args[1],
                    args[2],
                    args[3],
                    args[5],
                    false,
                    "settilebg16cgbbuf_attr",
                );
                return true;
            }
            "__settilebg16_flush" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settilebg16_flush requires 4 arguments");
                    return true;
                }
                self.emit_tile16_flush_rows_intrinsic(
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    false,
                    "settilebg16flush",
                );
                return true;
            }
            "__settilebg16cgb_flush" => {
                if args.len() != 5 {
                    self.env
                        .error(&e.source, "__settilebg16cgb_flush requires 5 arguments");
                    return true;
                }
                self.emit_tile16_flush_rows_intrinsic(
                    args[0],
                    args[2],
                    args[3],
                    args[4],
                    false,
                    "settilebg16cgbflush_tile",
                );
                if self.env.options.known_cgb.is_some_and(|n| n != 0) {
                    self.tile_vbk(1);
                    self.emit_tile16_flush_rows_intrinsic(
                        args[1],
                        args[2],
                        args[3],
                        args[4],
                        true,
                        "settilebg16cgbflush_attr",
                    );
                } else {
                    let skip = self.unique("settilebg16cgbflush_skip");
                    let done = self.unique("settilebg16cgbflush_done");
                    self.cgb_check();
                    self.asm("OR_A");
                    self.op("JP_Z", skip.clone());
                    self.tile_vbk(1);
                    self.emit_tile16_flush_rows_intrinsic(
                        args[1],
                        args[2],
                        args[3],
                        args[4],
                        true,
                        "settilebg16cgbflush_attr",
                    );
                    self.op("JR", done.clone());
                    self.label(&skip);
                    self.label(&done);
                }
                return true;
            }
            _ => false,
        }
    }
}
