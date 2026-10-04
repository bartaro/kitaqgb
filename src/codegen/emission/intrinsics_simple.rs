// Native instruction templates ported from the verified CodeGenerator.cs.
use super::*;
impl Emitter {
    pub(super) fn simple_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        match name {
            "__wait_vblank" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__wait_vblank() expects 0 arguments");
                    return true;
                }
                let wb_ret = self.unique("waitvb_ret");
                let wb_exit_vblank = self.unique("waitvb_exit");
                let wb_enter_vblank = self.unique("waitvb_enter");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
                self.op("JR_Z", wb_ret.clone());
                self.label(&wb_exit_vblank);
                self.op("LDH_A_MEM", Operand::integer(0x44, M::HighMem));
                self.op("CP_IMM", Operand::integer(144, M::Immediate));
                self.op("JR_C", wb_enter_vblank.clone());
                self.op("JR", wb_exit_vblank.clone());
                self.label(&wb_enter_vblank);
                self.op("LDH_A_MEM", Operand::integer(0x44, M::HighMem));
                self.op("CP_IMM", Operand::integer(144, M::Immediate));
                self.op("JR_C", wb_enter_vblank.clone());
                self.label(&wb_ret);
                true
            }
            "__wait_ly" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__wait_ly(target) expects 1 argument");
                    return true;
                }
                self.into_a(args[0]);
                self.asm("LD_B_A");
                let wly_loop = self.unique("waitly_loop");
                let wly_done = self.unique("waitly_done");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
                self.op("JR_Z", wly_done.clone());
                self.label(&wly_loop);
                self.op("LDH_A_MEM", Operand::integer(0x44, M::HighMem));
                self.asm("CP_B");
                self.op("JR_NZ", wly_loop.clone());
                self.label(&wly_done);
                true
            }
            "__rng_seed" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__rng_seed(seed) expects 1 argument");
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("LD_A_L");
                self.store(memory_operand(65440));
                self.asm("LD_A_H");
                self.store(memory_operand(65441));
                true
            }
            "__rng8" => {
                if args.len() != 0 {
                    self.env.error(&e.source, "__rng8() expects 0 arguments");
                    return true;
                }
                self.load(memory_operand(65440));
                self.asm("LD_B_A");
                self.load(memory_operand(65441));
                self.asm("LD_C_A");
                let rng_seeded = self.unique("rng_seeded");
                self.asm("LD_A_B");
                self.asm("OR_C");
                self.op("JR_NZ", rng_seeded.clone());
                self.op("LD_A_IMM", Operand::integer(0xA5, M::Immediate));
                self.asm("LD_B_A");
                self.op("LD_A_IMM", Operand::integer(0x5A, M::Immediate));
                self.asm("LD_C_A");
                self.label(&rng_seeded);
                self.asm("LD_A_B");
                self.op("ADD_A_IMM", Operand::integer(0x17, M::Immediate));
                self.asm("LD_B_A");
                self.asm("LD_A_C");
                self.op("ADC_IMM", Operand::integer(0x5D, M::Immediate));
                self.asm("LD_C_A");
                self.asm("LD_A_B");
                self.asm("XOR_C");
                self.asm("LD_B_A");
                self.asm("LD_A_C");
                self.asm("RLCA");
                self.asm("ADD_B");
                self.asm("LD_C_A");
                self.asm("LD_A_B");
                self.store(memory_operand(65440));
                self.asm("LD_A_C");
                self.store(memory_operand(65441));
                self.asm("LD_A_C");
                true
            }
            "__critical_enter" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__critical_enter() expects 0 arguments");
                    return true;
                }
                self.load(memory_operand(65439));
                self.asm("LD_B_A");
                self.asm("INC_A");
                self.store(memory_operand(65439));
                self.asm("DI");
                self.asm("LD_A_B");
                true
            }
            "__critical_leave" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__critical_leave(token) expects 1 argument");
                    return true;
                }
                self.into_a(args[0]);
                self.asm("LD_B_A");
                let cl_after_dec = self.unique("critleave_afterdec");
                let cl_done = self.unique("critleave_done");
                self.load(memory_operand(65439));
                self.asm("OR_A");
                self.op("JR_Z", cl_after_dec.clone());
                self.asm("DEC_A");
                self.store(memory_operand(65439));
                self.label(&cl_after_dec);
                self.asm("LD_A_B");
                self.asm("OR_A");
                self.op("JR_NZ", cl_done.clone());
                self.asm("EI");
                self.label(&cl_done);
                true
            }
            "__sram_read8" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__sram_read8(addr) expects 1 argument");
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.op("LD_A_IMM", Operand::integer(0x0A, M::Immediate));
                self.op("LD_MEM_A", Operand::integer(0x0000, M::Absolute));
                self.asm("POP_HL");
                self.asm("LD_A_HL");
                self.asm("LD_B_A");
                self.op("LD_A_IMM", Operand::integer(0x00, M::Immediate));
                self.op("LD_MEM_A", Operand::integer(0x0000, M::Absolute));
                self.asm("LD_A_B");
                true
            }
            "__sram_write8" => {
                if args.len() != 2 {
                    self.env
                        .error(&e.source, "__sram_write8(addr, value) expects 2 arguments");
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_C_A");
                self.op("LD_A_IMM", Operand::integer(0x0A, M::Immediate));
                self.op("LD_MEM_A", Operand::integer(0x0000, M::Absolute));
                self.asm("POP_HL");
                self.asm("LD_A_C");
                self.asm("LD_HL_A");
                self.op("LD_A_IMM", Operand::integer(0x00, M::Immediate));
                self.op("LD_MEM_A", Operand::integer(0x0000, M::Absolute));
                true
            }
            "__getbgmapbase" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__getbgmapbase requires 0 arguments");
                    return true;
                }
                let gb_base0 = self.unique("getbgbase_base0");
                let gb_done = self.unique("getbgbase_done");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x08, M::Immediate));
                self.op("JR_Z", gb_base0.clone());
                self.op("LD_HL_IMM", Operand::integer(0x9C00, M::Immediate16));
                self.op("JR", gb_done.clone());
                self.label(&gb_base0);
                self.op("LD_HL_IMM", Operand::integer(0x9800, M::Immediate16));
                self.label(&gb_done);
                true
            }
            "__getwinmapbase" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__getwinmapbase requires 0 arguments");
                    return true;
                }
                let win_base0 = self.unique("getwinbase_base0");
                let win_done = self.unique("getwinbase_done");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x40, M::Immediate));
                self.op("JR_Z", win_base0.clone());
                self.op("LD_HL_IMM", Operand::integer(0x9C00, M::Immediate16));
                self.op("JR", win_done.clone());
                self.label(&win_base0);
                self.op("LD_HL_IMM", Operand::integer(0x9800, M::Immediate16));
                self.label(&win_done);
                true
            }
            "__readpad" => {
                if args.len() != 0 {
                    self.env.error(&e.source, "__readpad requires 0 arguments");
                    return true;
                }
                let p1 = Operand::integer(0x00, M::HighMem);
                self.op("LD_A_IMM", Operand::integer(0x20, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.asm("CPL");
                self.op("AND_IMM", Operand::integer(0x0F, M::Immediate));
                self.asm("LD_C_A");
                self.op("LD_A_IMM", Operand::integer(0x10, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.asm("CPL");
                self.op("AND_IMM", Operand::integer(0x0F, M::Immediate));
                self.asm("RLCA");
                self.asm("RLCA");
                self.asm("RLCA");
                self.asm("RLCA");
                self.asm("OR_C");
                self.asm("LD_C_A");
                self.op("LD_A_IMM", Operand::integer(0x30, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.asm("LD_A_C");
                true
            }
            "__readpaddir" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__readpaddir requires 0 arguments");
                    return true;
                }
                let p1 = Operand::integer(0x00, M::HighMem);
                self.op("LD_A_IMM", Operand::integer(0x20, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.asm("CPL");
                self.op("AND_IMM", Operand::integer(0x0F, M::Immediate));
                self.asm("LD_C_A");
                self.op("LD_A_IMM", Operand::integer(0x30, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.asm("LD_A_C");
                true
            }
            "__readpadbtn" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__readpadbtn requires 0 arguments");
                    return true;
                }
                let p1 = Operand::integer(0x00, M::HighMem);
                self.op("LD_A_IMM", Operand::integer(0x10, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.asm("CPL");
                self.op("AND_IMM", Operand::integer(0x0F, M::Immediate));
                self.asm("LD_C_A");
                self.op("LD_A_IMM", Operand::integer(0x30, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.asm("LD_A_C");
                true
            }
            "__readpadex" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__readpadex requires 1 argument (prev_keys)");
                    return true;
                }
                self.into_a(args[0]);
                self.asm("LD_B_A");
                let p1 = Operand::integer(0x00, M::HighMem);
                self.op("LD_A_IMM", Operand::integer(0x20, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.asm("CPL");
                self.op("AND_IMM", Operand::integer(0x0F, M::Immediate));
                self.asm("LD_C_A");
                self.op("LD_A_IMM", Operand::integer(0x10, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.op("LDH_A_MEM", p1.clone());
                self.asm("CPL");
                self.op("AND_IMM", Operand::integer(0x0F, M::Immediate));
                self.asm("RLCA");
                self.asm("RLCA");
                self.asm("RLCA");
                self.asm("RLCA");
                self.asm("OR_C");
                self.asm("LD_L_A");
                self.op("LD_A_IMM", Operand::integer(0x30, M::Immediate));
                self.op("LDH_MEM_A", p1.clone());
                self.asm("LD_A_B");
                self.asm("CPL");
                self.asm("LD_B_A");
                self.asm("LD_A_L");
                self.asm("AND_B");
                self.asm("LD_H_A");
                true
            }
            "__padrep_init" => {
                if args.len() != 3 {
                    self.env.error(
                        &e.source,
                        "__padrep_init requires 3 arguments (state*, das, arr)",
                    );
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_A_B");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_A_C");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                true
            }
            "__padrep_reset" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__padrep_reset requires 1 argument (state*)");
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                true
            }
            "__padrep_lr" => {
                if args.len() != 3 {
                    self.env.error(
                        &e.source,
                        "__padrep_lr requires 3 arguments (state*, keys, trigger)",
                    );
                    return true;
                }
                let pr_done = self.unique("padrep_done");
                let pr_ret0 = self.unique("padrep_ret0");
                let pr_check_left = self.unique("padrep_chkL");
                let pr_no_trig = self.unique("padrep_notrig");
                let pr_reset_all = self.unique("padrep_reset");
                let pr_dir0 = self.unique("padrep_dir0");
                let pr_have_dir = self.unique("padrep_havedir");
                let pr_first_rep = self.unique("padrep_firstrep");
                let pr_ret_dir = self.unique("padrep_retdir");
                let pr_skip_dasinc = self.unique("padrep_skipdasinc");
                let pr_skip_arrinc = self.unique("padrep_skiparrinc");
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_C");
                self.op("AND_IMM", Operand::integer(0x01, M::Immediate));
                self.op("JP_Z", pr_check_left.clone());
                self.op("LD_A_IMM", Operand::integer(0x01, M::Immediate));
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.op("LD_A_IMM", Operand::integer(0x01, M::Immediate));
                self.op("JP", pr_done.clone());
                self.label(&pr_check_left);
                self.asm("LD_A_C");
                self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
                self.op("JP_Z", pr_no_trig.clone());
                self.op("LD_A_IMM", Operand::integer(0x02, M::Immediate));
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.op("LD_A_IMM", Operand::integer(0x02, M::Immediate));
                self.op("JP", pr_done.clone());
                self.label(&pr_no_trig);
                self.asm("LD_A_B");
                self.op("AND_IMM", Operand::integer(0x03, M::Immediate));
                self.asm("LD_D_A");
                self.op("JP_Z", pr_reset_all.clone());
                self.op("CP_IMM", Operand::integer(0x03, M::Immediate));
                self.op("JP_Z", pr_ret0.clone());
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("LD_A_E");
                self.asm("OR_A");
                self.op("JP_Z", pr_dir0.clone());
                self.asm("LD_A_B");
                self.asm("AND_E");
                self.op("JP_NZ", pr_have_dir.clone());
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("LD_E_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.op("JP", pr_dir0.clone());
                self.label(&pr_dir0);
                self.asm("LD_A_E");
                self.asm("OR_A");
                self.op("JP_NZ", pr_have_dir.clone());
                self.asm("LD_A_D");
                self.asm("LD_HL_A");
                self.asm("LD_E_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.op("JP", pr_done.clone());
                self.label(&pr_have_dir);
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("CP_IMM", Operand::integer(0xFF, M::Immediate));
                self.op("JP_Z", pr_skip_dasinc.clone());
                self.asm("INC_A");
                self.asm("LD_HL_A");
                self.asm("CP_C");
                self.op("JP_C", pr_ret0.clone());
                self.op("JP_Z", pr_first_rep.clone());
                self.label(&pr_skip_dasinc);
                self.asm("LD_A_D");
                self.asm("OR_A");
                self.op("JP_Z", pr_ret_dir.clone());
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("CP_IMM", Operand::integer(0xFF, M::Immediate));
                self.op("JP_Z", pr_skip_arrinc.clone());
                self.asm("INC_A");
                self.label(&pr_skip_arrinc);
                self.asm("LD_HL_A");
                self.asm("CP_D");
                self.op("JP_C", pr_ret0.clone());
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.op("JP", pr_ret_dir.clone());
                self.label(&pr_first_rep);
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.op("JP", pr_ret_dir.clone());
                self.label(&pr_ret_dir);
                self.asm("LD_A_E");
                self.op("JP", pr_done.clone());
                self.label(&pr_ret0);
                self.asm("XOR_A");
                self.op("JP", pr_done.clone());
                self.label(&pr_reset_all);
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.op("JP", pr_done.clone());
                self.label(&pr_done);
                true
            }
            "__padrep_down" => {
                if args.len() != 3 {
                    self.env.error(
                        &e.source,
                        "__padrep_down requires 3 arguments (state*, keys, trigger)",
                    );
                    return true;
                }
                let pd_done = self.unique("padrepD_done");
                let pd_ret0 = self.unique("padrepD_ret0");
                let pd_no_trig = self.unique("padrepD_notrig");
                let pd_dir0 = self.unique("padrepD_dir0");
                let pd_have_dir = self.unique("padrepD_havedir");
                let pd_first_rep = self.unique("padrepD_firstrep");
                let pd_ret_dir = self.unique("padrepD_retdir");
                let pd_skip_dasinc = self.unique("padrepD_skipdasinc");
                let pd_skip_arrinc = self.unique("padrepD_skiparrinc");
                let pd_reset_all = self.unique("padrepD_reset");
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_C");
                self.op("AND_IMM", Operand::integer(0x08, M::Immediate));
                self.op("JP_Z", pd_no_trig.clone());
                self.op("LD_A_IMM", Operand::integer(0x08, M::Immediate));
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.op("LD_A_IMM", Operand::integer(0x08, M::Immediate));
                self.op("JP", pd_done.clone());
                self.label(&pd_no_trig);
                self.asm("LD_A_B");
                self.op("AND_IMM", Operand::integer(0x08, M::Immediate));
                self.op("JP_Z", pd_reset_all.clone());
                self.asm("LD_D_A");
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("LD_A_E");
                self.asm("OR_A");
                self.op("JP_Z", pd_dir0.clone());
                self.asm("LD_A_B");
                self.asm("AND_E");
                self.op("JP_NZ", pd_have_dir.clone());
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("LD_E_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.op("JP", pd_dir0.clone());
                self.label(&pd_dir0);
                self.asm("LD_A_E");
                self.asm("OR_A");
                self.op("JP_NZ", pd_have_dir.clone());
                self.asm("LD_A_D");
                self.asm("LD_HL_A");
                self.asm("LD_E_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.op("JP", pd_done.clone());
                self.label(&pd_have_dir);
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("CP_IMM", Operand::integer(0xFF, M::Immediate));
                self.op("JP_Z", pd_skip_dasinc.clone());
                self.asm("INC_A");
                self.asm("LD_HL_A");
                self.asm("CP_C");
                self.op("JP_C", pd_ret0.clone());
                self.op("JP_Z", pd_first_rep.clone());
                self.label(&pd_skip_dasinc);
                self.asm("LD_A_D");
                self.asm("OR_A");
                self.op("JP_Z", pd_ret_dir.clone());
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("CP_IMM", Operand::integer(0xFF, M::Immediate));
                self.op("JP_Z", pd_skip_arrinc.clone());
                self.asm("INC_A");
                self.label(&pd_skip_arrinc);
                self.asm("LD_HL_A");
                self.asm("CP_D");
                self.op("JP_C", pd_ret0.clone());
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.op("JP", pd_ret_dir.clone());
                self.label(&pd_first_rep);
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.op("JP", pd_ret_dir.clone());
                self.label(&pd_ret_dir);
                self.asm("LD_A_E");
                self.op("JP", pd_done.clone());
                self.label(&pd_ret0);
                self.asm("XOR_A");
                self.op("JP", pd_done.clone());
                self.label(&pd_reset_all);
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.op("JP", pd_done.clone());
                self.label(&pd_done);
                true
            }
            "__padrep" | "__padrep_mask" => {
                if args.len() != 4 {
                    self.env.error(
                        &e.source,
                        "__padrep requires 4 arguments (state*, keys, trigger, mask)",
                    );
                    return true;
                }
                let pm_done = self.unique("padrepM_done");
                let pm_ret0 = self.unique("padrepM_ret0");
                let pm_no_trig = self.unique("padrepM_notrig");
                let pm_reset_all = self.unique("padrepM_reset");
                let pm_dir0 = self.unique("padrepM_dir0");
                let pm_have_dir = self.unique("padrepM_havedir");
                let pm_first_rep = self.unique("padrepM_firstrep");
                let pm_ret_dir = self.unique("padrepM_retdir");
                let pm_skip_dasinc = self.unique("padrepM_skipdasinc");
                let pm_skip_arrinc = self.unique("padrepM_skiparrinc");
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                self.into_a(args[3]);
                self.asm("LD_E_A");
                self.asm("POP_HL");
                self.asm("LD_A_C");
                self.asm("AND_E");
                self.asm("LD_D_A");
                self.asm("OR_A");
                self.op("JP_Z", pm_no_trig.clone());
                self.asm("LD_A_D");
                self.asm("CPL");
                self.asm("INC_A");
                self.asm("AND_D");
                self.asm("LD_C_A");
                self.asm("LD_A_C");
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("LD_A_C");
                self.op("JP", pm_done.clone());
                self.label(&pm_no_trig);
                self.asm("LD_A_B");
                self.asm("AND_E");
                self.asm("LD_D_A");
                self.asm("OR_A");
                self.op("JP_Z", pm_reset_all.clone());
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("LD_A_C");
                self.asm("OR_A");
                self.op("JP_Z", pm_dir0.clone());
                self.asm("LD_A_C");
                self.asm("AND_D");
                self.op("JP_NZ", pm_have_dir.clone());
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("LD_C_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.op("JP", pm_dir0.clone());
                self.label(&pm_dir0);
                self.asm("LD_A_C");
                self.asm("OR_A");
                self.op("JP_NZ", pm_have_dir.clone());
                self.asm("LD_A_D");
                self.asm("CPL");
                self.asm("INC_A");
                self.asm("AND_D");
                self.asm("LD_C_A");
                self.asm("LD_A_C");
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.op("JP", pm_done.clone());
                self.label(&pm_have_dir);
                self.asm("LD_A_C");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("CP_IMM", Operand::integer(0xFF, M::Immediate));
                self.op("JP_Z", pm_skip_dasinc.clone());
                self.asm("INC_A");
                self.asm("LD_HL_A");
                self.asm("CP_C");
                self.op("JP_C", pm_ret0.clone());
                self.op("JP_Z", pm_first_rep.clone());
                self.label(&pm_skip_dasinc);
                self.asm("LD_A_D");
                self.asm("OR_A");
                self.op("JP_Z", pm_ret_dir.clone());
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("CP_IMM", Operand::integer(0xFF, M::Immediate));
                self.op("JP_Z", pm_skip_arrinc.clone());
                self.asm("INC_A");
                self.label(&pm_skip_arrinc);
                self.asm("LD_HL_A");
                self.asm("CP_D");
                self.op("JP_C", pm_ret0.clone());
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.op("JP", pm_ret_dir.clone());
                self.label(&pm_first_rep);
                self.asm("INC_HL");
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.op("JP", pm_ret_dir.clone());
                self.label(&pm_ret_dir);
                self.asm("LD_A_E");
                self.op("JP", pm_done.clone());
                self.label(&pm_ret0);
                self.asm("XOR_A");
                self.op("JP", pm_done.clone());
                self.label(&pm_reset_all);
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("PUSH_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                self.asm("POP_HL");
                self.asm("XOR_A");
                self.op("JP", pm_done.clone());
                self.label(&pm_done);
                true
            }
            "__scroll_bg_add" => {
                if args.len() != 2 {
                    self.env
                        .error(&e.source, "__scroll_bg_add(dx, dy) expects 2 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_B_L");
                self.load(self.scroll_operand("__kq_scroll_bg_x_cur"));
                self.asm("ADD_B");
                self.store(self.scroll_operand("__kq_scroll_bg_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.op("LDH_MEM_A", Operand::integer(0x43, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_bg_y_cur"));
                self.asm("ADD_C");
                self.store(self.scroll_operand("__kq_scroll_bg_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.op("LDH_MEM_A", Operand::integer(0x42, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(254, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_bg_set" => {
                if args.len() != 2 {
                    self.env
                        .error(&e.source, "__scroll_bg_set(scx, scy) expects 2 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.store(self.scroll_operand("__kq_scroll_bg_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.op("LDH_MEM_A", Operand::integer(0x43, M::HighMem));
                self.asm("LD_A_B");
                self.store(self.scroll_operand("__kq_scroll_bg_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.op("LDH_MEM_A", Operand::integer(0x42, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(254, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_bg_set_buffered" => {
                if args.len() != 2 {
                    self.env.error(
                        &e.source,
                        "__scroll_bg_set_buffered(scx, scy) expects 2 arguments",
                    );
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.store(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.asm("LD_A_B");
                self.store(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("OR_IMM", Operand::integer(1, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_bg_x_get" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_bg_x_get() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.load(self.scroll_operand("__kq_scroll_bg_x_cur"));
                true
            }
            "__scroll_bg_x_set" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__scroll_bg_x_set(scx) expects 1 argument");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_bg_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.op("LDH_MEM_A", Operand::integer(0x43, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_bg_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(254, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_bg_x_set_buffered" => {
                if args.len() != 1 {
                    self.env.error(
                        &e.source,
                        "__scroll_bg_x_set_buffered(scx) expects 1 argument",
                    );
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("OR_IMM", Operand::integer(1, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_bg_y_get" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_bg_y_get() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.load(self.scroll_operand("__kq_scroll_bg_y_cur"));
                true
            }
            "__scroll_bg_y_set" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__scroll_bg_y_set(scy) expects 1 argument");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_bg_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.op("LDH_MEM_A", Operand::integer(0x42, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_bg_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(254, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_bg_y_set_buffered" => {
                if args.len() != 1 {
                    self.env.error(
                        &e.source,
                        "__scroll_bg_y_set_buffered(scy) expects 1 argument",
                    );
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("OR_IMM", Operand::integer(1, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_flush" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_flush() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                let flush_skip_bg = self.unique("scroll_flush_skip_bg");
                let flush_skip_win = self.unique("scroll_flush_skip_win");
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(1, M::Immediate));
                self.op("JR_Z", flush_skip_bg.clone());
                self.load(self.scroll_operand("__kq_scroll_bg_x_next"));
                self.store(self.scroll_operand("__kq_scroll_bg_x_cur"));
                self.op("LDH_MEM_A", Operand::integer(0x43, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_bg_y_next"));
                self.store(self.scroll_operand("__kq_scroll_bg_y_cur"));
                self.op("LDH_MEM_A", Operand::integer(0x42, M::HighMem));
                self.label(&flush_skip_bg);
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(2, M::Immediate));
                self.op("JR_Z", flush_skip_win.clone());
                self.load(self.scroll_operand("__kq_scroll_win_x_next"));
                self.store(self.scroll_operand("__kq_scroll_win_x_cur"));
                self.op("LDH_MEM_A", Operand::integer(0x4B, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_win_y_next"));
                self.store(self.scroll_operand("__kq_scroll_win_y_cur"));
                self.op("LDH_MEM_A", Operand::integer(0x4A, M::HighMem));
                self.label(&flush_skip_win);
                self.asm("XOR_A");
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_add" => {
                if args.len() != 2 {
                    self.env
                        .error(&e.source, "__scroll_win_add(dx, dy) expects 2 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_B_L");
                self.load(self.scroll_operand("__kq_scroll_win_x_cur"));
                self.asm("ADD_B");
                self.store(self.scroll_operand("__kq_scroll_win_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_x_next"));
                self.op("LDH_MEM_A", Operand::integer(0x4B, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_win_y_cur"));
                self.asm("ADD_C");
                self.store(self.scroll_operand("__kq_scroll_win_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_y_next"));
                self.op("LDH_MEM_A", Operand::integer(0x4A, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(253, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_hide" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_win_hide() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0xDF, M::Immediate));
                self.op("LDH_MEM_A", Operand::integer(0x40, M::HighMem));
                self.asm("XOR_A");
                self.store(self.scroll_operand("__kq_scroll_win_visible"));
                true
            }
            "__scroll_win_set" => {
                if args.len() != 2 {
                    self.env
                        .error(&e.source, "__scroll_win_set(wx, wy) expects 2 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.store(self.scroll_operand("__kq_scroll_win_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_x_next"));
                self.op("LDH_MEM_A", Operand::integer(0x4B, M::HighMem));
                self.asm("LD_A_B");
                self.store(self.scroll_operand("__kq_scroll_win_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_y_next"));
                self.op("LDH_MEM_A", Operand::integer(0x4A, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(253, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_set_buffered" => {
                if args.len() != 2 {
                    self.env.error(
                        &e.source,
                        "__scroll_win_set_buffered(wx, wy) expects 2 arguments",
                    );
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.store(self.scroll_operand("__kq_scroll_win_x_next"));
                self.asm("LD_A_B");
                self.store(self.scroll_operand("__kq_scroll_win_y_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("OR_IMM", Operand::integer(2, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_show" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_win_show() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("OR_IMM", Operand::integer(0x20, M::Immediate));
                self.op("LDH_MEM_A", Operand::integer(0x40, M::HighMem));
                self.op("LD_A_IMM", Operand::integer(0x20, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_win_visible"));
                true
            }
            "__scroll_win_x_get" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_win_x_get() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.load(self.scroll_operand("__kq_scroll_win_x_cur"));
                true
            }
            "__scroll_win_x_set" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__scroll_win_x_set(wx) expects 1 argument");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_win_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_x_next"));
                self.op("LDH_MEM_A", Operand::integer(0x4B, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_win_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_y_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(253, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_x_set_buffered" => {
                if args.len() != 1 {
                    self.env.error(
                        &e.source,
                        "__scroll_win_x_set_buffered(wx) expects 1 argument",
                    );
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_win_x_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("OR_IMM", Operand::integer(2, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_y_get" => {
                if args.len() != 0 {
                    self.env
                        .error(&e.source, "__scroll_win_y_get() expects 0 arguments");
                    return true;
                }
                self.ensure_scroll();
                self.load(self.scroll_operand("__kq_scroll_win_y_cur"));
                true
            }
            "__scroll_win_y_set" => {
                if args.len() != 1 {
                    self.env
                        .error(&e.source, "__scroll_win_y_set(wy) expects 1 argument");
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_win_y_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_y_next"));
                self.op("LDH_MEM_A", Operand::integer(0x4A, M::HighMem));
                self.load(self.scroll_operand("__kq_scroll_win_x_cur"));
                self.store(self.scroll_operand("__kq_scroll_win_x_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("AND_IMM", Operand::integer(253, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__scroll_win_y_set_buffered" => {
                if args.len() != 1 {
                    self.env.error(
                        &e.source,
                        "__scroll_win_y_set_buffered(wy) expects 1 argument",
                    );
                    return true;
                }
                self.ensure_scroll();
                self.into_a(args[0]);
                self.store(self.scroll_operand("__kq_scroll_win_y_next"));
                self.load(self.scroll_operand("__kq_scroll_dirty"));
                self.op("OR_IMM", Operand::integer(2, M::Immediate));
                self.store(self.scroll_operand("__kq_scroll_dirty"));
                true
            }
            "__rle_decode_vram" => {
                if args.len() != 3 {
                    self.env.error(
                        &e.source,
                        "__rle_decode_vram(dst, bank, src) expects 3 arguments",
                    );
                    return true;
                }
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.extend_a(false);
                self.asm("PUSH_HL");
                self.into_hl(args[2]);
                self.asm("PUSH_HL");
                self.op("ADD_SP_IMM", Operand::integer(-4, M::Relative));
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("XOR_A");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_HL_A");
                let loop_label = self.unique("rlevram_loop");
                let count_done = self.unique("rlevram_count_inc_done");
                let value_done = self.unique("rlevram_value_inc_done");
                let finished = self.unique("rlevram_finished");
                self.label(&loop_label);
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.op("LD_HL_SP_IMM", Operand::integer(8, M::Relative));
                self.asm("LD_A_HL");
                self.asm("POP_HL");
                self.op("LD_BC_IMM", Operand::integer(1, M::Immediate16));
                self.far_copy_call();
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("OR_A");
                self.op("JP_Z", finished.clone());
                self.op("LD_HL_SP_IMM", Operand::integer(3, M::Relative));
                self.asm("LD_HL_A");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.op("ADD_A_IMM", Operand::integer(1, M::Immediate));
                self.asm("LD_HL_A");
                self.op("JR_NC", count_done.clone());
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_HL_A");
                self.label(&count_done);
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(6, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.op("LD_HL_SP_IMM", Operand::integer(8, M::Relative));
                self.asm("LD_A_HL");
                self.asm("POP_HL");
                self.op("LD_BC_IMM", Operand::integer(1, M::Immediate16));
                self.far_copy_call();
                self.op("LD_HL_SP_IMM", Operand::integer(2, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.op("LD_HL_SP_IMM", Operand::integer(3, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_C_A");
                self.op("LD_B_IMM", Operand::integer(0, M::Immediate));
                self.op("LD_HL_SP_IMM", Operand::integer(8, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("LD_H_D");
                self.asm("LD_L_E");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                self.vram_fill_loop(true, "rlevram_fill");
                self.asm("PUSH_HL");
                self.op("LD_HL_SP_IMM", Operand::integer(10, M::Relative));
                self.asm("POP_DE");
                self.asm("LD_A_E");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_A_D");
                self.asm("LD_HL_A");
                self.op("LD_HL_SP_IMM", Operand::integer(3, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_B_A");
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("LD_A_HL");
                self.asm("ADD_B");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_HL_A");
                self.op("LD_HL_SP_IMM", Operand::integer(4, M::Relative));
                self.asm("LD_A_HL");
                self.op("ADD_A_IMM", Operand::integer(1, M::Immediate));
                self.asm("LD_HL_A");
                self.op("JR_NC", value_done.clone());
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.op("ADC_IMM", Operand::integer(0, M::Immediate));
                self.asm("LD_HL_A");
                self.label(&value_done);
                self.op("JP", loop_label.clone());
                self.label(&finished);
                self.op("LD_HL_SP_IMM", Operand::integer(0, M::Relative));
                self.asm("LD_A_HL");
                self.asm("LD_E_A");
                self.asm("INC_HL");
                self.asm("LD_A_HL");
                self.asm("LD_D_A");
                self.asm("LD_H_D");
                self.asm("LD_L_E");
                self.op("ADD_SP_IMM", Operand::integer(10, M::Relative));
                true
            }
            "__settile" => {
                if args.len() != 3 {
                    self.env.error(&e.source, "__settile requires 3 arguments");
                    return true;
                }
                self.into_a(args[0]);
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("LD_A_D");
                self.op("CALL", Operand::symbol("__settile_core", M::Absolute));
                true
            }
            "__settile_unsafe" | "__settile_fast" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settile_unsafe requires 3 arguments");
                    return true;
                }
                self.into_a(args[0]);
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[1]);
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_D_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("LD_A_D");
                self.op("CALL", Operand::symbol("__settile_fast_core", M::Absolute));
                true
            }
            "__settileat" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settileat requires 4 arguments");
                    return true;
                }
                let st_end = self.unique("settileat_end");
                self.into_a(args[1]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_B_A");
                self.into_a(args[2]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("LD_A_B");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[3]);
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_C_A");
                self.asm("POP_DE");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
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
                let st_write_now = self.unique("settileat_write_now");
                let st_wait = self.unique("settileat_wait");
                let st_done = self.unique("settileat_done");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
                self.op("JR_Z", st_write_now.clone());
                self.asm("DI");
                self.label(&st_wait);
                self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
                self.op("JR_NZ", st_wait.clone());
                self.asm("LD_HL_C");
                self.asm("EI");
                self.op("JR", st_done.clone());
                self.label(&st_write_now);
                self.asm("LD_HL_C");
                self.label(&st_done);
                self.label(&st_end);
                true
            }
            "__settileat_unsafe" => {
                if args.len() != 4 {
                    self.env
                        .error(&e.source, "__settileat_unsafe requires 4 arguments");
                    return true;
                }
                let st_end = self.unique("settileatu_end");
                self.into_a(args[1]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_B_A");
                self.into_a(args[2]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("LD_A_B");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_hl(args[0]);
                self.asm("PUSH_HL");
                self.into_a(args[3]);
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_C_A");
                self.asm("POP_DE");
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
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
                self.asm("LD_HL_C");
                self.label(&st_end);
                true
            }
            "__settilewin" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilewin requires 3 arguments");
                    return true;
                }
                let st_end = self.unique("settilewin_end");
                self.into_a(args[0]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_B_A");
                self.into_a(args[1]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("LD_A_B");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                let st_base0 = self.unique("settilewin_base0");
                let st_base_done = self.unique("settilewin_basedone");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x40, M::Immediate));
                self.op("JR_Z", st_base0.clone());
                self.op("LD_DE_IMM", Operand::integer(0x9C00, M::Immediate));
                self.op("JR", st_base_done.clone());
                self.label(&st_base0);
                self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
                self.label(&st_base_done);
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
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
                let st_write_now = self.unique("settilewin_write_now");
                let st_wait = self.unique("settilewin_wait");
                let st_done = self.unique("settilewin_done");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
                self.op("JR_Z", st_write_now.clone());
                self.asm("DI");
                self.label(&st_wait);
                self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
                self.op("JR_NZ", st_wait.clone());
                self.asm("LD_HL_C");
                self.asm("EI");
                self.op("JR", st_done.clone());
                self.label(&st_write_now);
                self.asm("LD_HL_C");
                self.label(&st_done);
                self.label(&st_end);
                true
            }
            "__settilewin_unsafe" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilewin_unsafe requires 3 arguments");
                    return true;
                }
                let st_end = self.unique("settilewinu_end");
                self.into_a(args[0]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_B_A");
                self.into_a(args[1]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("LD_A_B");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                let st_base0 = self.unique("settilewinu_base0");
                let st_base_done = self.unique("settilewinu_basedone");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x40, M::Immediate));
                self.op("JR_Z", st_base0.clone());
                self.op("LD_DE_IMM", Operand::integer(0x9C00, M::Immediate));
                self.op("JR", st_base_done.clone());
                self.label(&st_base0);
                self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
                self.label(&st_base_done);
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
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
                self.asm("LD_HL_C");
                self.label(&st_end);
                true
            }
            "__settilebg" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilebg requires 3 arguments");
                    return true;
                }
                let st_end = self.unique("settilebg_end");
                self.into_a(args[0]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_B_A");
                self.into_a(args[1]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("LD_A_B");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                let st_base0 = self.unique("settilebg_base0");
                let st_base_done = self.unique("settilebg_basedone");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x08, M::Immediate));
                self.op("JR_Z", st_base0.clone());
                self.op("LD_DE_IMM", Operand::integer(0x9C00, M::Immediate));
                self.op("JR", st_base_done.clone());
                self.label(&st_base0);
                self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
                self.label(&st_base_done);
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
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
                let st_write_now = self.unique("settilebg_write_now");
                let st_wait = self.unique("settilebg_wait");
                let st_done = self.unique("settilebg_done");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x80, M::Immediate));
                self.op("JR_Z", st_write_now.clone());
                self.asm("DI");
                self.label(&st_wait);
                self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x02, M::Immediate));
                self.op("JR_NZ", st_wait.clone());
                self.asm("LD_HL_C");
                self.asm("EI");
                self.op("JR", st_done.clone());
                self.label(&st_write_now);
                self.asm("LD_HL_C");
                self.label(&st_done);
                self.label(&st_end);
                true
            }
            "__settilebg_unsafe" => {
                if args.len() != 3 {
                    self.env
                        .error(&e.source, "__settilebg_unsafe requires 3 arguments");
                    return true;
                }
                let st_end = self.unique("settilebgu_end");
                self.into_a(args[0]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_B_A");
                self.into_a(args[1]);
                self.op("CP_IMM", Operand::integer(32, M::Immediate));
                self.op("JR_NC", st_end.clone());
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.asm("LD_A_B");
                self.asm("LD_L_A");
                self.op("LD_H_IMM", Operand::integer(0, M::Immediate));
                self.asm("PUSH_HL");
                self.into_a(args[2]);
                self.asm("LD_C_A");
                let st_base0 = self.unique("settilebgu_base0");
                let st_base_done = self.unique("settilebgu_basedone");
                self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
                self.op("AND_IMM", Operand::integer(0x08, M::Immediate));
                self.op("JR_Z", st_base0.clone());
                self.op("LD_DE_IMM", Operand::integer(0x9C00, M::Immediate));
                self.op("JR", st_base_done.clone());
                self.label(&st_base0);
                self.op("LD_DE_IMM", Operand::integer(0x9800, M::Immediate));
                self.label(&st_base_done);
                self.asm("POP_HL");
                self.asm("LD_A_L");
                self.asm("LD_B_A");
                self.asm("POP_HL");
                self.asm("LD_A_L");
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
                self.asm("LD_HL_C");
                self.label(&st_end);
                true
            }
            _ => false,
        }
    }
}
