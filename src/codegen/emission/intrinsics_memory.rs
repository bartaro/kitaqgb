use super::*;
impl Emitter {
    pub(super) fn memory_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        let (copy, small) = match name {
            "__memcpy" => (true, false),
            "__memcpy_small" => (true, true),
            "__memset" => (false, false),
            "__memset_small" => (false, true),
            "__copy16" | "__copy32" => {
                if args.len() != 2 {
                    self.env
                        .error(&e.source, format!("{name}(dst, src) expects 2 arguments"));
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_hl(args[1]);
                self.asm("PUSH_HL");
                self.word("LD_BC_IMM", if name == "__copy16" { 16 } else { 32 });
                self.asm("POP_DE");
                self.asm("POP_HL");
                self.copy_bc(name.trim_start_matches('_'));
                return true;
            }
            _ => return false,
        };
        if args.len() != 3 {
            self.env.error(
                &e.source,
                if copy {
                    "__memcpy requires 3 arguments"
                } else {
                    "__memset requires 3 arguments"
                },
            );
            return true;
        }
        let count = args[2]
            .int(1)
            .filter(|_| args[2].is(t::INTEGER))
            .map(|n| if small { n & 255 } else { n });
        let check = self.env.options.check_mem_copy && self.unsafe_depth == 0;
        let dst = if check {
            self.fixed_array_bytes(args[0])
        } else {
            None
        };
        let src = if check && copy {
            self.fixed_array_bytes(args[1])
        } else {
            None
        };
        if let Some(n) = count {
            for (array, role) in [(&dst, if copy { "dst " } else { "" }), (&src, "src ")] {
                if let Some((name, size)) = array {
                    if n >= 0 && n > *size {
                        self.env.warning(&args[2].source,format!("{} count {n} exceeds fixed {role}array '{name}' size {size} bytes (-Zcheck)",if copy{"__memcpy"}else{"__memset"}));
                        let trap = self.check_trap();
                        self.op("JP", trap);
                        return true;
                    }
                }
            }
        }
        if let Some(n) = count.filter(|n| (0..=255).contains(n)) {
            if n == 0 {
                return true;
            }
            if copy {
                self.copy_pointers(args[0], args[1]);
            } else {
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("POP_HL");
            }
            if n <= 16 {
                for _ in 0..n {
                    self.memory_byte(copy);
                }
                return true;
            }
            let block = if n <= 64 { 8 } else { 16 };
            self.immediate("LD_B_IMM", n / block);
            let label = self.unique(&format!(
                "{}{}_loop",
                if copy { "memcpy" } else { "memset" },
                block
            ));
            self.label(&label);
            for _ in 0..block {
                self.memory_byte(copy);
            }
            self.asm("DEC_B");
            self.op("JP_NZ", label);
            for _ in 0..n % block {
                self.memory_byte(copy);
            }
            return true;
        }
        self.into_hl(args[0]);
        self.asm("PUSH_HL");
        if copy {
            self.into_hl(args[1]);
            self.asm("PUSH_HL");
        } else if small {
            self.into_a(args[1]);
            self.asm("LD_D_A");
        }
        if small {
            self.into_a(args[2]);
            self.asm("LD_B_A");
            for (_, size) in dst.iter().chain(src.iter()) {
                self.copy_length_check(*size, true);
            }
            let done = self.unique(if copy {
                "memcpy_small_done"
            } else {
                "memset_small_done"
            });
            let loop_label = self.unique(if copy {
                "memcpy_small_loop"
            } else {
                "memset_small_loop"
            });
            self.asm("OR_A");
            if copy {
                self.asm("POP_DE");
            }
            self.asm("POP_HL");
            self.op("JP_Z", done.clone());
            if !copy {
                self.asm("LD_A_D");
            }
            self.label(&loop_label);
            self.memory_byte(copy);
            self.asm("DEC_B");
            self.op("JP_NZ", loop_label);
            self.label(&done);
        } else {
            self.into_hl(args[2]);
            if count.is_none() {
                for (_, size) in dst.iter().chain(src.iter()) {
                    self.copy_length_check(*size, false);
                }
            }
            self.asm("LD_B_H");
            self.asm("LD_C_L");
            if copy {
                self.asm("POP_DE");
            } else {
                self.into_a(args[1]);
                self.asm("LD_D_A");
            }
            self.asm("POP_HL");
            let loop_label = self.unique(if copy { "memcpy_loop" } else { "memset_loop" });
            let done = self.unique(if copy { "memcpy_done" } else { "memset_done" });
            self.asm("LD_A_B");
            self.asm("OR_C");
            self.op("JP_Z", done.clone());
            self.label(&loop_label);
            if !copy {
                self.asm("LD_A_D");
            }
            self.memory_byte(copy);
            self.asm("DEC_BC");
            self.asm("LD_A_B");
            self.asm("OR_C");
            self.op("JP_NZ", loop_label);
            self.label(&done);
        }
        true
    }
    fn memory_byte(&mut self, copy: bool) {
        if copy {
            self.asm("LD_A_DE");
        }
        self.asm("LDI_HL_A");
        if copy {
            self.asm("INC_DE");
        }
    }
    fn copy_pointers(&mut self, dst: &Expr, src: &Expr) {
        self.into_hl(dst);
        self.asm("PUSH_HL");
        self.into_hl(src);
        self.asm("PUSH_HL");
        self.asm("POP_DE");
        self.asm("POP_HL");
    }
    pub(super) fn copy_bc(&mut self, prefix: &str) {
        let loop_label = self.unique(&format!("{prefix}_loop"));
        let done = self.unique(&format!("{prefix}_done"));
        self.label(&loop_label);
        self.asm("LD_A_B");
        self.asm("OR_C");
        self.op("JR_Z", done.clone());
        self.memory_byte(true);
        self.asm("DEC_BC");
        self.op("JR", loop_label);
        self.label(&done);
    }
}
