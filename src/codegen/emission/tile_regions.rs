// Verified native tile-region templates and their constant-count specializations.
use super::*;
impl Emitter {
    pub(super) fn tile_region_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        match name {
            "__settile_rect" => {
                if args.len() != 5 {
                    self.env.error(
                        &e.source,
                        "__settile_rect(x, y, w, h, tile) expects 5 arguments",
                    );
                    return true;
                }
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[3]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[4]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                let loop_label = self.unique("settilerect_loop");
                let done = self.unique("settilerect_done");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", done.clone());
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", done.clone());
                self.tile_map_base(0x08, "settilerect_base");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(8, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("POP_DE");
                self.asm("ADD_HL_DE");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(10, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("ADD_C");
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.label(&loop_label);
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("DEC_A");
                self.asm("LD_HL_A");
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.op("LD_B_IMM", Operand::integer(0, M::Immediate));
                self.asm("POP_HL");
                self.vram_fill_loop(true, "settilerect_fill");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.asm("POP_HL");
                self.op("JP_Z", done.clone());
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.op("LD_A_IMM", Operand::integer(32, M::Immediate));
                self.asm("SUB_C");
                self.asm("LD_C_A");
                self.asm("LD_A_L");
                self.asm("ADD_C");
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.op("JP", loop_label.clone());
                self.label(&done);
                self.op("ADD_SP_IMM", Operand::integer(10, M::Relative));
                return true;
            }
            "__settile_row" => {
                if args.len() != 4 {
                    self.env.error(
                        &e.source,
                        "__settile_row(x, y, src, len) expects 4 arguments",
                    );
                    return true;
                }
                let row_len = self.env.fold(args[3]);
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_hl(args[2]);
                self.asm("PUSH_HL");
                self.into_a(&row_len);
                self.extend_a(false);
                self.asm("PUSH_HL");
                let done = self.unique("settilerow_done");
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", done.clone());
                self.tile_map_base(0x08, "settilerow_base");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("POP_DE");
                self.asm("ADD_HL_DE");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(8, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("ADD_C");
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.op("LD_B_IMM", Operand::integer(0, M::Immediate));
                self.asm("POP_HL");
                if !(row_len.is(t::INTEGER)
                    && self.vram_copy_const(
                        true,
                        row_len.int(1).unwrap() & 255,
                        "settilerow_small",
                    ))
                {
                    self.vram_copy_loop(true, "settilerow_copy");
                }
                self.label(&done);
                self.op("ADD_SP_IMM", Operand::integer(8, M::Relative));
                return true;
            }
            "__settile_col" => {
                if args.len() != 4 {
                    self.env.error(
                        &e.source,
                        "__settile_col(x, y, src, len) expects 4 arguments",
                    );
                    return true;
                }
                let col_len = self.env.fold(args[3]);
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_hl(args[2]);
                self.asm("PUSH_HL");
                self.into_a(&col_len);
                self.extend_a(false);
                self.asm("PUSH_HL");
                let loop_label = self.unique("settilecol_loop");
                let done = self.unique("settilecol_done");
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", done.clone());
                self.tile_map_base(0x08, "settilecol_base");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("POP_DE");
                self.asm("ADD_HL_DE");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(8, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("ADD_C");
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                if col_len.is(t::INTEGER)
                    && self.vram_column_const(
                        true,
                        col_len.int(1).unwrap() & 255,
                        "settilecol_small",
                    )
                {
                    self.label(&done);
                    self.op("ADD_SP_IMM", Operand::integer(8, M::Relative));
                    return true;
                }
                self.label(&loop_label);
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("DEC_A");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("LD_A_DE");
                self.asm("LD_C_A");
                self.vram_store_c(true, "settilecol_store");
                self.asm("INC_DE");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.asm("POP_HL");
                self.op("JP_Z", done.clone());
                self.asm("LD_A_L");
                self.op("ADD_A_IMM", Operand::integer(32, M::Immediate));
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.op("JP", loop_label.clone());
                self.label(&done);
                self.op("ADD_SP_IMM", Operand::integer(8, M::Relative));
                return true;
            }
            "__settilemap_rect" => {
                if args.len() != 6 {
                    self.env.error(
                        &e.source,
                        "__settilemap_rect(base, x, y, w, h, src) expects 6 arguments",
                    );
                    return true;
                }
                let rect_w = self.env.fold(args[3]);
                let rect_h = self.env.fold(args[4]);
                if rect_h.is(t::INTEGER) && (rect_h.int(1).unwrap() & 255) == 1 {
                    self.length_bc(&rect_w);
                    self.asm("PUSH_BC");
                    self.into_hl(args[5]);
                    self.asm("PUSH_HL");
                    self.tile_map_address(args[0], args[1], args[2]);
                    self.asm("POP_DE");
                    self.asm("POP_BC");
                    if !(rect_w.is(t::INTEGER)
                        && self.vram_copy_const(
                            true,
                            rect_w.int(1).unwrap() & 255,
                            "settilemaprect_row_small",
                        ))
                    {
                        self.vram_copy_loop(true, "settilemaprect_row");
                    }
                    return true;
                }
                if rect_w.is(t::INTEGER) && (rect_w.int(1).unwrap() & 255) == 1 {
                    self.into_a(&rect_h);
                    self.asm("LD_B_A");
                    self.into_hl(args[5]);
                    self.asm("PUSH_HL");
                    self.tile_map_address(args[0], args[1], args[2]);
                    self.asm("POP_DE");
                    if rect_h.is(t::INTEGER)
                        && self.vram_column_const(
                            true,
                            rect_h.int(1).unwrap() & 255,
                            "settilemaprect_col_small",
                        )
                    {
                        return true;
                    }
                    let col_loop = self.unique("settilemaprect_col_loop");
                    let col_done = self.unique("settilemaprect_col_done");
                    self.label(&col_loop);
                    self.asm("LD_A_B");
                    self.asm("OR_A");
                    self.op("JR_Z", col_done.clone());
                    self.asm("DEC_B");
                    self.asm("LD_A_DE");
                    self.asm("LD_C_A");
                    self.vram_store_c(true, "settilemaprect_col_store");
                    self.asm("INC_DE");
                    self.asm("LD_A_L");
                    self.op("ADD_A_IMM", Operand::integer(32, M::Immediate));
                    self.asm("LD_L_A");
                    self.asm("LD_A_H");
                    self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                    self.asm("LD_H_A");
                    self.op("JR", col_loop.clone());
                    self.label(&col_done);
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[3]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[4]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_hl(args[5]);
                self.asm("PUSH_HL");
                let loop_label = self.unique("settilemaprect_loop");
                let done = self.unique("settilemaprect_done");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", done.clone());
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", done.clone());
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("ADD_HL_HL");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(12, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                self.asm("ADD_HL_DE");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(10, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("ADD_C");
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                self.label(&loop_label);
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("DEC_A");
                self.asm("LD_HL_A");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.op("LD_B_IMM", Operand::integer(0, M::Immediate));
                self.asm("POP_HL");
                self.vram_copy_loop(true, "settilemaprect_copy");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.asm("POP_HL");
                self.op("JP_Z", done.clone());
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.op("LD_A_IMM", Operand::integer(32, M::Immediate));
                self.asm("SUB_C");
                self.asm("LD_C_A");
                self.asm("LD_A_L");
                self.asm("ADD_C");
                self.asm("LD_L_A");
                self.asm("LD_A_H");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_H_A");
                self.op("JP", loop_label.clone());
                self.label(&done);
                self.op("ADD_SP_IMM", Operand::integer(12, M::Relative));
                return true;
            }
            _ => false,
        }
    }
}
