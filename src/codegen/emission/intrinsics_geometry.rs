use super::*;
impl Emitter {
    pub(super) fn geometry_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        let arity = match name {
            "__tile_addr" | "__map_index" => 3,
            "__xy_in_rect" => 6,
            "__manhattan" => 4,
            "__bit_test" | "__bit_set" | "__bit_clear" | "__bit_toggle" => 2,
            _ => return false,
        };
        if args.len() != arity {
            self.env
                .error(&e.source, format!("{name} expects {arity} arguments"));
            return true;
        }
        let folded = args.iter().map(|a| self.env.fold(a)).collect::<Vec<_>>();
        let byte = |i: usize| {
            folded[i]
                .int(1)
                .filter(|_| folded[i].is(t::INTEGER))
                .map(|n| n & 255)
        };
        match name {
            "__tile_addr" => self.tile_address(args[0], args[1], args[2]),
            "__map_index" => {
                if let (Some(x), Some(y), Some(w)) = (byte(0), byte(1), byte(2)) {
                    self.word("LD_HL_IMM", (y * w + x) & 65535);
                    return true;
                }
                if let Some(w) = byte(2).filter(|w| *w > 0 && (*w as u32).is_power_of_two()) {
                    self.into_a(&folded[1]);
                    self.extend_a(false);
                    for _ in 0..w.trailing_zeros() {
                        self.asm("ADD_HL_HL");
                    }
                    if byte(0) != Some(0) {
                        self.into_a(&folded[0]);
                        self.asm("LD_E_A");
                        self.immediate("LD_D_IMM", 0);
                        self.asm("ADD_HL_DE");
                    }
                    return true;
                }
                for a in args {
                    self.push_byte_word(a);
                }
                self.sp(0);
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.sp(2);
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.mul_bytes_prefix("__map_index");
                self.asm("PUSH_HL");
                self.sp(6);
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.immediate("LD_D_IMM", 0);
                self.asm("POP_HL");
                self.asm("ADD_HL_DE");
                self.op("ADD_SP_IMM", Operand::integer(6, M::Relative));
            }
            "__xy_in_rect" => {
                if let (Some(x), Some(y), Some(rx), Some(ry), Some(w), Some(h)) =
                    (byte(0), byte(1), byte(2), byte(3), byte(4), byte(5))
                {
                    if x >= rx && y >= ry && x - rx < w && y - ry < h {
                        self.immediate("LD_A_IMM", 1);
                    } else {
                        self.asm("XOR_A");
                    }
                    return true;
                }
                for a in args {
                    self.push_byte_word(a);
                }
                let zero = self.unique("xyrect_ret0");
                let done = self.unique("xyrect_done");
                for (coord, origin, length) in [(10, 6, 2), (8, 4, 0)] {
                    self.sp(coord);
                    self.asm("LD_A_HL");
                    self.asm("LD_B_A");
                    self.sp(origin);
                    self.asm("LD_A_HL");
                    self.asm("LD_C_A");
                    self.asm("LD_A_B");
                    self.asm("CP_C");
                    self.op("JR_C", zero.clone());
                    self.asm("SUB_C");
                    self.asm("LD_B_A");
                    self.sp(length);
                    self.asm("LD_A_HL");
                    self.asm("LD_C_A");
                    self.asm("LD_A_B");
                    self.asm("CP_C");
                    self.op("JR_NC", zero.clone());
                }
                self.immediate("LD_A_IMM", 1);
                self.op("JR", done.clone());
                self.label(&zero);
                self.asm("XOR_A");
                self.label(&done);
                self.op("ADD_SP_IMM", Operand::integer(12, M::Relative));
            }
            "__manhattan" => {
                if let (Some(x), Some(y), Some(x2), Some(y2)) = (byte(0), byte(1), byte(2), byte(3))
                {
                    self.immediate("LD_A_IMM", ((x - x2).abs() + (y - y2).abs()) & 255);
                    return true;
                }
                for a in args {
                    self.push_byte_word(a);
                }
                let dxge = self.unique("manhattan_dx_ge");
                let dxdone = self.unique("manhattan_dx_done");
                let dyge = self.unique("manhattan_dy_ge");
                let dydone = self.unique("manhattan_dy_done");
                self.sp(6);
                self.asm("LD_A_HL");
                self.asm("LD_B_A");
                self.sp(2);
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("LD_A_B");
                self.asm("CP_C");
                self.op("JR_NC", dxge.clone());
                self.asm("LD_A_C");
                self.asm("SUB_B");
                self.op("JR", dxdone.clone());
                self.label(&dxge);
                self.asm("LD_A_B");
                self.asm("SUB_C");
                self.label(&dxdone);
                self.asm("LD_B_A");
                self.sp(4);
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.sp(0);
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("LD_A_C");
                self.asm("CP_D");
                self.op("JR_NC", dyge.clone());
                self.asm("LD_A_D");
                self.asm("SUB_C");
                self.op("JR", dydone.clone());
                self.label(&dyge);
                self.asm("LD_A_C");
                self.asm("SUB_D");
                self.label(&dydone);
                self.asm("ADD_B");
                self.op("ADD_SP_IMM", Operand::integer(8, M::Relative));
            }
            _ => self.bit_intrinsic(name, args),
        }
        true
    }
    pub(super) fn push_byte_word(&mut self, e: &Expr) {
        self.into_a(e);
        self.extend_a(false);
        self.asm("PUSH_HL");
    }
    fn sp(&mut self, n: i32) {
        self.op("LD_HL_SP_IMM", Operand::integer(n, M::Relative));
    }
    fn add_offset(&mut self, n: i32) {
        if n == 0 {
            return;
        }
        self.word("LD_DE_IMM", n);
        self.asm("ADD_HL_DE");
    }
    pub(super) fn tile_address(&mut self, base: &Expr, x: &Expr, y: &Expr) {
        let b = self.env.fold(base);
        let x = self.env.fold(x);
        let y = self.env.fold(y);
        let xc = x.int(1).filter(|_| x.is(t::INTEGER)).map(|n| n & 255);
        let yc = y.int(1).filter(|_| y.is(t::INTEGER)).map(|n| n & 255);
        if let (Some(x), Some(y)) = (xc, yc) {
            self.into_hl(&b);
            self.add_offset((y << 5) + x);
            return;
        }
        if let Some(y) = yc {
            self.into_hl(&b);
            self.add_offset(y << 5);
            self.into_a(&x);
            self.asm("LD_E_A");
            self.immediate("LD_D_IMM", 0);
            self.asm("ADD_HL_DE");
            return;
        }
        self.into_a(&y);
        self.extend_a(false);
        for _ in 0..5 {
            self.asm("ADD_HL_HL");
        }
        if let Some(x) = xc {
            self.add_offset(x);
        }
        self.asm("PUSH_HL");
        self.into_hl(&b);
        self.asm("POP_DE");
        self.asm("ADD_HL_DE");
        if xc.is_none() {
            self.into_a(&x);
            self.asm("LD_E_A");
            self.immediate("LD_D_IMM", 0);
            self.asm("ADD_HL_DE");
        }
    }
    fn bit_intrinsic(&mut self, name: &str, args: &[&Expr]) {
        self.into_hl(args[0]);
        self.asm("PUSH_HL");
        self.into_hl(args[1]);
        self.asm("PUSH_HL");
        self.sp(0);
        self.asm("LD_A_HL");
        self.immediate("AND_IMM", 7);
        self.asm("LD_B_A");
        self.sp(0);
        self.asm("LD_A_HL");
        self.asm("LD_E_A");
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.asm("LD_D_A");
        self.asm("LD_H_D");
        self.asm("LD_L_E");
        self.shift_hl(false, false, 3);
        self.asm("PUSH_HL");
        self.sp(4);
        self.asm("LD_A_HL");
        self.asm("LD_E_A");
        self.asm("INC_HL");
        self.asm("LD_A_HL");
        self.asm("LD_D_A");
        self.asm("POP_HL");
        self.asm("ADD_HL_DE");
        let ready = self.unique("bit_mask_ready");
        let loop_label = self.unique("bit_mask_loop");
        self.asm("LD_A_B");
        self.asm("OR_A");
        self.immediate("LD_A_IMM", 1);
        self.op("JR_Z", ready.clone());
        self.label(&loop_label);
        self.asm("ADD_A");
        self.asm("DEC_B");
        self.op("JR_NZ", loop_label);
        self.label(&ready);
        self.asm("LD_B_A");
        match name {
            "__bit_test" => {
                let zero = self.unique("bit_test_zero");
                let done = self.unique("bit_test_done");
                self.asm("LD_A_HL");
                self.asm("AND_B");
                self.op("JR_Z", zero.clone());
                self.immediate("LD_A_IMM", 1);
                self.op("JR", done.clone());
                self.label(&zero);
                self.asm("XOR_A");
                self.label(&done);
            }
            "__bit_set" => {
                self.asm("LD_A_HL");
                self.asm("OR_B");
                self.asm("LD_HL_A");
            }
            "__bit_clear" => {
                self.asm("LD_A_B");
                self.asm("CPL");
                self.asm("LD_B_A");
                self.asm("LD_A_HL");
                self.asm("AND_B");
                self.asm("LD_HL_A");
            }
            _ => {
                self.asm("LD_A_HL");
                self.asm("XOR_B");
                self.asm("LD_HL_A");
            }
        }
        self.op("ADD_SP_IMM", Operand::integer(4, M::Relative));
    }
}
