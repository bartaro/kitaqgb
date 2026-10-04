use super::*;
impl Emitter {
    pub(super) fn cgb_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        let register = match name {
            "__cgb_safe_set_vbk" => Some(0x4f),
            "__cgb_safe_set_svbk" => Some(0x70),
            "__cgb_safe_set_bgpi" | "__cgb_safe_set_bcps" => Some(0x68),
            "__cgb_safe_set_bgpd" | "__cgb_safe_set_bcpd" => Some(0x69),
            "__cgb_safe_set_obpi" | "__cgb_safe_set_ocps" => Some(0x6a),
            "__cgb_safe_set_obpd" | "__cgb_safe_set_ocpd" => Some(0x6b),
            "__cgb_safe_set_hdma1" => Some(0x51),
            "__cgb_safe_set_hdma2" => Some(0x52),
            "__cgb_safe_set_hdma3" => Some(0x53),
            "__cgb_safe_set_hdma4" => Some(0x54),
            "__cgb_safe_set_hdma5" => Some(0x55),
            _ => None,
        };
        if let Some(reg) = register {
            if args.len() != 1 {
                self.env
                    .error(&e.source, format!("{name}(value_u8) expects 1 argument"));
                return true;
            }
            self.into_a(args[0]);
            if self.env.options.known_cgb.is_some_and(|n| n != 0) {
                if reg == 0x4f {
                    self.immediate("AND_IMM", 1);
                }
                self.op("LDH_MEM_A", Operand::integer(reg, M::HighMem));
            } else {
                self.asm("LD_B_A");
                self.cgb_check();
                let skip = self.unique("cgbsafe_skip");
                self.asm("OR_A");
                self.op("JR_Z", skip.clone());
                self.asm("LD_A_B");
                if reg == 0x4f {
                    self.immediate("AND_IMM", 1);
                }
                self.op("LDH_MEM_A", Operand::integer(reg, M::HighMem));
                self.label(&skip);
            }
            self.report_cgb_write(match reg {
                0x4f => "VBK",
                0x70 => "SVBK",
                0x68 => "BCPS",
                0x69 => "BCPD",
                0x6a => "OCPS",
                0x6b => "OCPD",
                0x51 => "HDMA1",
                0x52 => "HDMA2",
                0x53 => "HDMA3",
                0x54 => "HDMA4",
                0x55 => "HDMA5",
                _ => unreachable!(),
            });
            return true;
        }
        match name {
            "__cgb_is_cgb" => {
                if !args.is_empty() {
                    self.env
                        .error(&e.source, "__cgb_is_cgb() expects 0 arguments");
                    return true;
                }
                self.cgb_check();
            }
            "__svbk_get" | "__svbk_set" => {
                let count = usize::from(name == "__svbk_set");
                if args.len() != count {
                    self.env
                        .error(&e.source, format!("{name} expects {count} arguments"));
                    return true;
                }
                if !self.env.options.cgb_only {
                    self.env.error(
                        &e.source,
                        format!("{name}() requires #pragma rom_cgb cgb_only or --cgb=cgb_only"),
                    );
                    return true;
                }
                if count == 1 {
                    let arg = self.env.fold(args[0]);
                    if arg.is(t::INTEGER) && arg.int(1).is_some_and(|n| !(1..=7).contains(&n)) {
                        self.env.error(
                            &e.source,
                            format!(
                                "__svbk_set() bank must be in range 1..7 (got {})",
                                arg.int(1).unwrap()
                            ),
                        );
                        return true;
                    }
                    self.svbk_normalized();
                    self.asm("LD_B_A");
                    self.into_a(&arg);
                    self.op("LDH_MEM_A", Operand::integer(0x70, M::HighMem));
                    self.asm("LD_A_B");
                } else {
                    self.svbk_normalized();
                }
            }
            _ => return false,
        }
        true
    }
    fn svbk_normalized(&mut self) {
        self.op("LDH_A_MEM", Operand::integer(0x70, M::HighMem));
        self.immediate("AND_IMM", 7);
        let ok = self.unique("svbk_norm_ok");
        self.asm("OR_A");
        self.op("JR_NZ", ok.clone());
        self.immediate("LD_A_IMM", 1);
        self.label(&ok);
    }
    pub(super) fn cgb_check(&mut self) {
        if let Some(n) = self.env.options.known_cgb {
            self.immediate("LD_A_IMM", n);
        } else {
            self.cgb_helper = true;
            self.analysis.cgb_runtime_checks += 1;
            self.op("CALL", Operand::symbol("__kq_is_cgb", M::Absolute));
        }
    }
    pub(super) fn append_cgb_helper(&mut self) {
        if !self.cgb_helper {
            return;
        }
        let body = std::mem::take(&mut self.lines);
        self.emit(
            t::COMMENT,
            vec!["[KITAQGB] injected CGB runtime-detect helper".into()],
        );
        self.emit(t::FUNCTION, vec!["__kq_is_cgb".into()]);
        let yes = self.unique("kq_is_cgb_true");
        let done = self.unique("kq_is_cgb_done");
        self.asm("PUSH_BC");
        self.asm("PUSH_DE");
        self.op("LDH_A_MEM", Operand::integer(0x4f, M::HighMem));
        self.asm("LD_D_A");
        self.asm("XOR_A");
        self.op("LDH_MEM_A", Operand::integer(0x4f, M::HighMem));
        self.op("LDH_A_MEM", Operand::integer(0x4f, M::HighMem));
        self.asm("LD_B_A");
        self.immediate("LD_A_IMM", 1);
        self.op("LDH_MEM_A", Operand::integer(0x4f, M::HighMem));
        self.op("LDH_A_MEM", Operand::integer(0x4f, M::HighMem));
        self.asm("LD_C_A");
        self.asm("LD_A_D");
        self.op("LDH_MEM_A", Operand::integer(0x4f, M::HighMem));
        self.asm("LD_A_B");
        self.immediate("CP_IMM", 0xfe);
        self.op("JR_NZ", done.clone());
        self.asm("LD_A_C");
        self.immediate("CP_IMM", 0xff);
        self.op("JR_Z", yes.clone());
        self.label(&done);
        self.asm("XOR_A");
        self.asm("POP_DE");
        self.asm("POP_BC");
        self.asm("RET");
        self.label(&yes);
        self.immediate("LD_A_IMM", 1);
        self.asm("POP_DE");
        self.asm("POP_BC");
        self.asm("RET");
        self.lines.extend(body);
    }
}
