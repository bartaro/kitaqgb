use super::*;
impl Emitter {
    pub(super) fn vram_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        if name == "__oam_dma" {
            if args.len() != 1 {
                self.env
                    .error(&e.source, "__oam_dma(src_ptr) expects 1 argument");
                return true;
            }
            let addr = self.env.find("__kq_oam_dma_stub").unwrap().value;
            self.into_hl(args[0]);
            self.op("LD_A_MEM", Operand::integer(0xffff, M::Absolute));
            self.asm("PUSH_AF");
            self.asm("XOR_A");
            self.op("LD_MEM_A", Operand::integer(0xffff, M::Absolute));
            for (i, n) in [0xe0, 0x46, 0x06, 0x28, 0x05, 0x20, 0xfd, 0xc9]
                .into_iter()
                .enumerate()
            {
                self.immediate("LD_A_IMM", n);
                self.op(
                    "LDH_MEM_A",
                    Operand::integer((addr + i as i32) & 255, M::HighMem),
                );
            }
            self.asm("LD_A_H");
            self.op("CALL", Operand::integer(addr, M::Absolute));
            self.asm("POP_AF");
            self.op("LD_MEM_A", Operand::integer(0xffff, M::Absolute));
            return true;
        }
        let (copy, safe, prefix) = match name {
            "__vram_memcpy" => (true, true, "vramcpy"),
            "__vram_memcpy_unsafe" => (true, false, "vramcpyu"),
            "__vram_memset" => (false, true, "vramset"),
            "__vram_memset_unsafe" => (false, false, "vramsetu"),
            _ => return false,
        };
        if args.len() != 3 {
            self.env
                .error(&e.source, format!("{name} expects 3 arguments"));
            return true;
        }
        self.into_hl(args[0]);
        self.asm("PUSH_HL");
        if copy {
            self.into_hl(args[1]);
            self.asm("PUSH_HL");
        } else {
            self.into_a(args[1]);
            self.asm("LD_D_A");
        }
        let length = self.env.fold(args[2]);
        self.length_bc(&length);
        if copy {
            self.asm("POP_DE");
        }
        self.asm("POP_HL");
        if let Some(n) = length
            .int(1)
            .filter(|n| length.is(t::INTEGER) && (0..=8).contains(n))
        {
            let pair = if safe && n > 0 {
                let fast = self.unique(&format!("{prefix}_small_fast"));
                let done = self.unique(&format!("{prefix}_small_done"));
                self.lcdc_test();
                self.op("JP_Z", fast.clone());
                for _ in 0..n {
                    let wait = self.unique(&format!("{prefix}_small_safe_wait"));
                    self.vram_wait(&wait);
                    self.vram_byte(copy, true);
                }
                self.op("JP", done.clone());
                self.label(&fast);
                Some(done)
            } else {
                None
            };
            for _ in 0..n {
                self.vram_byte(copy, false);
            }
            if let Some(done) = pair {
                self.label(&done);
            }
            return true;
        }
        if safe {
            let fast = self.unique(&format!("{prefix}_fast"));
            let safe_loop = self.unique(&format!("{prefix}_safe_loop"));
            let wait = self.unique(&format!("{prefix}_safe_wait"));
            let fast_loop = self.unique(&format!("{prefix}_fast_loop"));
            let done = self.unique(&format!("{prefix}_done"));
            self.count_test(&done);
            self.lcdc_test();
            self.op("JR_Z", fast.clone());
            self.label(&safe_loop);
            self.count_test(&done);
            self.vram_wait(&wait);
            self.vram_byte(copy, true);
            self.asm("DEC_BC");
            self.op("JR", safe_loop);
            self.label(&fast);
            self.label(&fast_loop);
            self.count_test(&done);
            self.vram_byte(copy, false);
            self.asm("DEC_BC");
            self.op("JR", fast_loop);
            self.label(&done);
        } else {
            let loop_label = self.unique(&format!("{prefix}_loop"));
            let done = self.unique(&format!("{prefix}_done"));
            self.label(&loop_label);
            self.count_test(&done);
            self.vram_byte(copy, false);
            self.asm("DEC_BC");
            self.op("JR", loop_label);
            self.label(&done);
        }
        true
    }
    pub(super) fn length_bc(&mut self, e: &Expr) {
        let e = self.env.fold(e);
        if e.is(t::INTEGER) {
            let n = e.int(1).unwrap();
            self.immediate("LD_B_IMM", (n >> 8) & 255);
            self.immediate("LD_C_IMM", n & 255);
        } else if self.size(&e) == 1 {
            self.into_a(&e);
            self.asm("LD_C_A");
            self.immediate("LD_B_IMM", 0);
        } else {
            self.into_hl(&e);
            self.asm("LD_B_H");
            self.asm("LD_C_L");
        }
    }
    pub(super) fn vram_fill_loop(&mut self, safe: bool, prefix: &str) {
        let fast = self.unique(&format!("{prefix}_fast"));
        let safe_loop = self.unique(&format!("{prefix}_safe_loop"));
        let wait = self.unique(&format!("{prefix}_safe_wait"));
        let fast_loop = self.unique(&format!("{prefix}_fast_loop"));
        let done = self.unique(&format!("{prefix}_done"));
        self.count_test(&done);
        if safe {
            self.lcdc_test();
            self.op("JR_Z", fast.clone());
            self.label(&safe_loop);
            self.count_test(&done);
            self.vram_wait(&wait);
            self.vram_byte(false, true);
            self.asm("DEC_BC");
            self.op("JR", safe_loop);
        }
        self.label(&fast);
        self.label(&fast_loop);
        self.count_test(&done);
        self.vram_byte(false, false);
        self.asm("DEC_BC");
        self.op("JR", fast_loop);
        self.label(&done);
    }
    fn count_test(&mut self, done: &Operand) {
        self.asm("LD_A_B");
        self.asm("OR_C");
        self.op("JR_Z", done.clone());
    }
    fn lcdc_test(&mut self) {
        self.op("LDH_A_MEM", Operand::integer(0x40, M::HighMem));
        self.immediate("AND_IMM", 128);
    }
    fn vram_wait(&mut self, wait: &Operand) {
        self.asm("DI");
        self.label(wait);
        self.op("LDH_A_MEM", Operand::integer(0x41, M::HighMem));
        self.immediate("AND_IMM", 2);
        self.op("JR_NZ", wait.clone());
    }
    fn vram_byte(&mut self, copy: bool, safe: bool) {
        self.asm(if copy { "LD_A_DE" } else { "LD_A_D" });
        self.asm("LDI_HL_A");
        if safe {
            self.asm("EI");
        }
        if copy {
            self.asm("INC_DE");
        }
    }
}
