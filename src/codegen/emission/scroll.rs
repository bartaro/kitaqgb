use super::*;
impl Emitter {
    pub(super) fn scroll_split_intrinsic(&mut self, e: &Expr, name: &str, args: &[&Expr]) -> bool {
        let (count, target) = match name {
            "__scroll_split_reset" => (0, "__kq_scroll_split_reset_core"),
            "__scroll_split_commit" => (0, "__kq_scroll_split_commit_core"),
            "__scroll_split_push" => (3, "__kq_scroll_split_push_core"),
            "__scroll_split_push_ex" => (6, "__kq_scroll_split_push_core"),
            _ => return false,
        };
        if args.len() != count {
            self.env
                .error(&e.source, format!("{name} expects {count} arguments"));
            return true;
        }
        self.ensure_scroll();
        self.scroll_split = true;
        if count == 6 {
            let flags = self.env.fold(args[5]);
            if flags.is(t::INTEGER) {
                let unknown = flags.int(1).unwrap() & 255 & !31;
                if unknown != 0 {
                    self.env.warning(
                        &args[5].source,
                        format!(
                            "__scroll_split_push_ex() ignores unknown flag bits 0x{unknown:02X}"
                        ),
                    );
                }
            }
        }
        for (arg, field) in args.iter().zip([
            "split_tmp_ly",
            "split_tmp_scx",
            "split_tmp_scy",
            "split_tmp_wx",
            "split_tmp_wy",
            "split_tmp_flags",
        ]) {
            self.into_a(arg);
            self.store(self.scroll_operand(&format!("__kq_scroll_{field}")));
        }
        if count == 3 {
            self.asm("XOR_A");
            self.store(self.scroll_operand("__kq_scroll_split_tmp_wx"));
            self.store(self.scroll_operand("__kq_scroll_split_tmp_wy"));
            self.immediate("LD_A_IMM", 1);
            self.store(self.scroll_operand("__kq_scroll_split_tmp_flags"));
        }
        self.op("CALL", Operand::symbol(target, M::Absolute));
        true
    }
    pub(super) fn ensure_scroll(&mut self) {
        if !self.scroll_vars.is_empty() {
            return;
        }
        for suffix in [
            "bg_x_cur",
            "bg_y_cur",
            "win_x_cur",
            "win_y_cur",
            "bg_x_next",
            "bg_y_next",
            "win_x_next",
            "win_y_next",
            "dirty",
            "win_visible",
            "split_count",
            "split_enabled",
            "split_index",
            "split_tmp_ly",
            "split_tmp_scx",
            "split_tmp_scy",
            "split_tmp_wx",
            "split_tmp_wy",
            "split_tmp_flags",
            "split_table",
        ] {
            let name = format!("__kq_scroll_{suffix}");
            let size = if suffix == "split_table" { 48 } else { 1 };
            let Some(value) = self.env.allocator.allocate(2, size, 1) else {
                self.env
                    .error(&Position::default(), "Out of memory for scroll state");
                return;
            };
            self.emit(
                t::VARIABLE,
                vec![name.clone().into(), value.into(), size.into()],
            );
            self.scroll_vars.insert(name, value);
        }
    }
    pub(super) fn scroll_operand(&self, name: &str) -> Operand {
        memory_operand(self.scroll_vars[name])
    }
}
