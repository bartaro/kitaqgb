use super::*;
impl Emitter {
    pub(super) fn into_a(&mut self, e: &Expr) {
        let e = self.env.fold(e);
        if e.is(t::NAME) {
            if let Some(s) = self
                .env
                .find(e.text(1).unwrap())
                .cloned()
                .filter(|s| s.tag == SymbolTag::StackParam)
            {
                self.stack_param_address(s.value, &format!("stack param {}", s.name));
                self.asm("LD_A_HL");
                return;
            }
        }
        if matches!(e.tag(), Some(t::MULTIPLY | t::DIVIDE | t::MODULUS)) {
            self.into_hl(&e);
            self.asm("LD_A_L");
            return;
        }
        if e.is(t::FIELD) || e.is(t::INDEX) {
            self.read_memory_a(&e);
            return;
        }
        match e.tag().unwrap_or("") {
            t::INTEGER => {
                self.immediate("LD_A_IMM", e.int(1).unwrap() & 255);
                return;
            }
            t::UNSAFE => {
                self.unsafe_depth += 1;
                self.into_a(child(&e, 1));
                self.unsafe_depth -= 1;
                return;
            }
            t::CAST => {
                self.into_a(child(&e, 2));
                return;
            }
            t::LOAD => {
                self.into_hl(child(&e, 1));
                self.asm("LD_A_HL");
                return;
            }
            t::SLICE => {
                self.discard(child(&e, 2));
                self.into_hl(child(&e, 1));
                self.asm("LD_A_L");
                return;
            }
            t::CONDITIONAL => {
                let end = self.unique("tern_end");
                self.ternary_a(&e, &end);
                self.label(&end);
                return;
            }
            t::BITWISE_NOT => {
                self.into_a(child(&e, 1));
                self.asm("CPL");
                return;
            }
            t::CALL => {
                self.call(&e);
                if self.size(&e) == 2 {
                    self.asm("LD_A_L");
                }
                return;
            }
            t::ASSIGN => {
                self.assign_expression(child(&e, 1), child(&e, 2));
                return;
            }
            t::PRE_INCREMENT | t::POST_INCREMENT | t::PRE_DECREMENT | t::POST_DECREMENT => {
                self.increment_a(&e);
                return;
            }
            t::LOGICAL_OR | t::LOGICAL_AND | t::LOGICAL_NOT => {
                let (prefix, condition) = if e.is(t::LOGICAL_OR) {
                    ("lor", true)
                } else if e.is(t::LOGICAL_AND) {
                    ("land", false)
                } else {
                    ("lnot", false)
                };
                let selected = self.unique(&format!(
                    "{prefix}_{}",
                    if e.is(t::LOGICAL_AND) { "f" } else { "t" }
                ));
                let end = self.unique(&format!("{prefix}_e"));
                self.jump_if(condition, child(&e, 1), &selected);
                if !e.is(t::LOGICAL_NOT) {
                    self.jump_if(condition, child(&e, 2), &selected);
                }
                let initial = if e.is(t::LOGICAL_AND) { 1 } else { 0 };
                self.immediate("LD_A_IMM", initial);
                self.op("JP", end.clone());
                self.label(&selected);
                self.immediate("LD_A_IMM", 1 - initial);
                self.label(&end);
                return;
            }
            t::SHIFT_LEFT | t::SHIFT_RIGHT => {
                let left = child(&e, 1);
                let right = child(&e, 2);
                let Some(count) = right.int(1).filter(|_| right.is(t::INTEGER)) else {
                    self.unsupported(&e, "variable shift");
                    return;
                };
                if e.is(t::SHIFT_LEFT) {
                    self.into_a(left);
                    if self.size(left) == 2 && count >= 8 {
                        self.asm("XOR_A");
                    } else {
                        for _ in 0..count {
                            self.asm("ADD_A");
                        }
                    }
                } else if self.size(left) == 2 {
                    self.into_hl(&e);
                    self.asm("LD_A_L");
                } else {
                    let signed = self.env.type_of(left).is_signed();
                    self.into_a(left);
                    for _ in 0..count {
                        if signed {
                            self.immediate("CP_IMM", 128);
                            self.asm("CCF");
                        } else {
                            self.asm("OR_A");
                        }
                        self.asm("RRA");
                    }
                }
                return;
            }
            _ => {}
        }
        if let Some(op) = self.operand(&e) {
            match op.mode {
                M::Immediate16 => {
                    self.op("LD_HL_IMM", op);
                    self.asm("LD_A_L");
                }
                M::Immediate => self.op("LD_A_IMM", op),
                _ => self.load(op),
            }
            return;
        }
        if matches!(
            e.tag(),
            Some(t::ADD | t::SUBTRACT | t::BITWISE_AND | t::BITWISE_OR | t::BITWISE_XOR)
        ) {
            let (immediate, register) = match e.tag().unwrap() {
                t::ADD => ("ADD_A_IMM", "ADD_B"),
                t::SUBTRACT => ("SUB_IMM", "SUB_B"),
                t::BITWISE_AND => ("AND_IMM", "AND_B"),
                t::BITWISE_OR => ("OR_IMM", "OR_B"),
                _ => ("XOR_IMM", "XOR_B"),
            };
            let left = child(&e, 1);
            let right = child(&e, 2);
            if let Some(op) = self.operand(right).filter(|o| o.mode == M::Immediate) {
                self.into_a(left);
                self.op(immediate, op);
            } else {
                self.into_a(left);
                self.asm("PUSH_AF");
                self.into_a(right);
                self.asm("LD_B_A");
                self.asm("POP_AF");
                self.asm(register);
            }
            return;
        }
        if comparison(&e) {
            self.comparison_a(&e);
            return;
        }
        self.unsupported(&e, "byte expression");
    }
    pub(super) fn into_hl(&mut self, e: &Expr) {
        if matches!(
            e.tag(),
            Some(t::PRE_INCREMENT | t::POST_INCREMENT | t::PRE_DECREMENT | t::POST_DECREMENT)
        ) {
            self.increment_hl(e);
            return;
        }
        let e = self.env.fold(e);
        if e.is(t::NAME) {
            if let Some(s) = self
                .env
                .find(e.text(1).unwrap())
                .cloned()
                .filter(|s| s.tag == SymbolTag::StackParam)
            {
                self.stack_param_address(s.value, &format!("stack param {}", s.name));
                self.asm("LD_A_HL");
                if self.size(&e) == 1 {
                    let signed = self.sign_extend(&e);
                    self.extend_a(signed);
                } else {
                    for m in ["LD_E_A", "INC_HL", "LD_A_HL", "LD_D_A", "LD_H_D", "LD_L_E"] {
                        self.asm(m);
                    }
                }
                return;
            }
        }
        if e.is(t::MULTIPLY) {
            self.multiply(&e);
            return;
        }
        if e.is(t::DIVIDE) || e.is(t::MODULUS) {
            self.divide(&e);
            return;
        }
        if e.is(t::SHIFT_LEFT) || e.is(t::SHIFT_RIGHT) {
            let right = child(&e, 2);
            if let Some(count) = right.int(1).filter(|_| right.is(t::INTEGER)) {
                let left = child(&e, 1);
                let signed = self.env.type_of(left).is_signed();
                self.into_hl(left);
                self.shift_hl(e.is(t::SHIFT_LEFT), signed, count);
                return;
            }
            if e.is(t::SHIFT_RIGHT) {
                self.unsupported(&e, "variable shift");
            }
        }
        if e.is(t::FIELD) || e.is(t::INDEX) {
            self.read_memory_hl(&e);
            return;
        }
        if e.is(t::INTEGER) {
            self.word("LD_HL_IMM", e.int(1).unwrap());
            return;
        }
        if e.is(t::NAME) {
            let name = e.text(1).unwrap();
            if self.env.find(name).is_none() && self.env.functions.contains_key(name) {
                self.op("LD_HL_IMM", Operand::symbol(name, M::Immediate16));
                return;
            }
        }
        if e.is(t::UNSAFE) {
            self.unsafe_depth += 1;
            self.into_hl(child(&e, 1));
            self.unsafe_depth -= 1;
            return;
        }
        if e.is(t::ADDRESS_OF) {
            self.address(child(&e, 1));
            return;
        }
        if e.is(t::SLICE) {
            self.discard(child(&e, 2));
            self.into_hl(child(&e, 1));
            return;
        }
        if e.is(t::CONDITIONAL) {
            if self.size(&e) == 1 {
                self.into_a(&e);
                let signed = self.sign_extend(&e);
                self.extend_a(signed);
            } else {
                let f = self.unique("tern_hl_false");
                let end = self.unique("tern_hl_end");
                self.jump_if(false, child(&e, 1), &f);
                self.into_hl(child(&e, 2));
                self.op("JP", end.clone());
                self.label(&f);
                self.into_hl(child(&e, 3));
                self.label(&end);
            }
            return;
        }
        if e.is(t::LOAD) {
            let size = self.size(&e);
            self.into_hl(child(&e, 1));
            self.read_hl(&e, size);
            return;
        }
        if let Some(op) = self.operand(&e) {
            if matches!(op.mode, M::Immediate | M::Immediate16) {
                self.op("LD_HL_IMM", op);
                return;
            }
            let size = self.size(&e);
            self.load(op.clone());
            self.asm("LD_L_A");
            if size == 1 {
                if self.sign_extend(&e) {
                    self.asm("LD_A_L");
                    self.asm("RLCA");
                    self.asm("SBC_A");
                    self.asm("LD_H_A");
                } else {
                    self.asm("XOR_A");
                    self.asm("LD_H_A");
                }
            } else {
                let mut high = op.clone();
                high.offset += 1;
                high.base = None;
                self.load(high);
                self.asm("LD_H_A");
            }
            return;
        }
        if e.is(t::CAST) {
            let sub = child(&e, 2);
            let ty = e.ty(1).unwrap();
            if self.env.size(&e, ty) == 1 {
                self.into_a(sub);
                self.extend_a(ty.is_signed());
            } else if self.size(sub) == 1 {
                self.into_a(sub);
                let signed = self.sign_extend(sub);
                self.extend_a(signed);
            } else {
                self.into_hl(sub);
            }
            return;
        }
        if e.is(t::BITWISE_NOT) {
            self.into_hl(child(&e, 1));
            for r in ["L", "H"] {
                self.asm(&format!("LD_A_{r}"));
                self.asm("CPL");
                self.asm(&format!("LD_{r}_A"));
            }
            return;
        }
        if matches!(
            e.tag(),
            Some(t::ADD | t::SUBTRACT | t::BITWISE_AND | t::BITWISE_OR | t::BITWISE_XOR)
        ) {
            let left = child(&e, 1);
            let right = child(&e, 2);
            let tag = e.tag().unwrap();
            if matches!(tag, t::ADD | t::SUBTRACT) {
                let lt = self.env.type_of(left);
                let rt = self.env.type_of(right);
                if (lt.is_pointer()
                    && (rt.is_integer() || matches!(rt.kind, crate::ctype::Kind::Enum(_))))
                    || (tag == t::ADD
                        && rt.is_pointer()
                        && (lt.is_integer() || matches!(lt.kind, crate::ctype::Kind::Enum(_))))
                {
                    let (ptr, index, ty) = if lt.is_pointer() {
                        (left, right, lt)
                    } else {
                        (right, left, rt)
                    };
                    let stride = self.env.size(&e, sub_type(&ty).unwrap());
                    self.scaled_index(index, stride);
                    self.asm("PUSH_DE");
                    self.into_hl(ptr);
                    self.asm("POP_DE");
                    if tag == t::ADD {
                        self.asm("ADD_HL_DE");
                    } else {
                        self.subtract_de();
                    }
                    return;
                }
            }
            self.into_hl(left);
            if right.is(t::INTEGER) {
                let n = right.int(1).unwrap();
                if tag == t::ADD || tag == t::SUBTRACT {
                    self.word(
                        "LD_DE_IMM",
                        if tag == t::SUBTRACT {
                            n.wrapping_neg() & 65535
                        } else {
                            n
                        },
                    );
                    self.asm("ADD_HL_DE");
                } else {
                    let op = if tag == t::BITWISE_AND {
                        "AND_IMM"
                    } else if tag == t::BITWISE_OR {
                        "OR_IMM"
                    } else {
                        "XOR_IMM"
                    };
                    for (r, v) in [("L", n & 255), ("H", (n >> 8) & 255)] {
                        self.asm(&format!("LD_A_{r}"));
                        self.immediate(op, v);
                        self.asm(&format!("LD_{r}_A"));
                    }
                }
            } else {
                self.asm("PUSH_HL");
                self.into_hl(right);
                self.asm("LD_D_H");
                self.asm("LD_E_L");
                self.asm("POP_HL");
                if tag == t::ADD {
                    self.asm("ADD_HL_DE");
                } else if tag == t::SUBTRACT {
                    self.subtract_de();
                } else {
                    let op = if tag == t::BITWISE_AND {
                        "AND"
                    } else if tag == t::BITWISE_OR {
                        "OR"
                    } else {
                        "XOR"
                    };
                    for (r, s) in [("L", "E"), ("H", "D")] {
                        self.asm(&format!("LD_A_{r}"));
                        self.asm(&format!("{op}_{s}"));
                        self.asm(&format!("LD_{r}_A"));
                    }
                }
            }
            return;
        }
        if e.is(t::CALL) {
            self.call(&e);
            if self.size(&e) != 2 {
                let signed = self.sign_extend(&e);
                self.extend_a(signed);
            }
            return;
        }
        if comparison(&e)
            || matches!(
                e.tag(),
                Some(t::LOGICAL_AND | t::LOGICAL_OR | t::LOGICAL_NOT)
            )
        {
            self.into_a(&e);
            self.extend_a(false);
            return;
        }
        self.unsupported(&e, "word expression");
    }
    fn subtract_de(&mut self) {
        for m in ["LD_A_L", "SUB_E", "LD_L_A", "LD_A_H", "SBC_D", "LD_H_A"] {
            self.asm(m);
        }
    }
    fn read_hl(&mut self, e: &Expr, size: i32) {
        if size == 1 {
            self.asm("LD_A_HL");
            let signed = self.sign_extend(e);
            self.extend_a(signed);
        } else {
            for m in [
                "LD_A_HL", "LD_E_A", "INC_HL", "LD_A_HL", "LD_D_A", "LD_H_D", "LD_L_E",
            ] {
                self.asm(m);
            }
        }
    }
    fn ternary_a(&mut self, e: &Expr, end: &Operand) {
        if !e.is(t::CONDITIONAL) {
            self.into_a(e);
            return;
        }
        let f = self.unique("tern_false");
        self.jump_if(false, child(e, 1), &f);
        self.into_a(child(e, 2));
        self.op("JP", end.clone());
        self.label(&f);
        self.ternary_a(child(e, 3), end);
    }
    pub(super) fn address(&mut self, e: &Expr) {
        if e.is(t::CALL) && self.env.type_of(e).is_aggregate() {
            self.call(e);
            if let Some(address) = self.last_sret_address {
                self.word("LD_HL_IMM", address);
            } else {
                self.env.error(
                    &e.source,
                    "missing struct/union return storage for call expression",
                );
            }
            return;
        }
        if e.is(t::FIELD) {
            self.field_address(e);
            return;
        }
        if e.is(t::NAME) {
            let name = e.text(1).unwrap();
            if self.env.functions.contains_key(name) {
                self.op("LD_HL_IMM", Operand::symbol(name, M::Immediate16));
                return;
            }
            if let Some(s) = self.env.find_required(e, name) {
                if s.tag == SymbolTag::StackParam {
                    self.stack_param_address(s.value, &format!("&stack param {}", s.name));
                } else if s.tag == SymbolTag::ReadonlyData {
                    self.op("LD_HL_IMM", Operand::symbol(name, M::Immediate16));
                } else {
                    self.word("LD_HL_IMM", s.value);
                }
            }
            return;
        }
        if e.is(t::LOAD) {
            self.into_hl(child(e, 1));
            return;
        }
        if e.is(t::INDEX) {
            let mut base = child(e, 1);
            if base.is(t::SLICE) {
                self.slice_access(child(base, 2), child(e, 2));
                base = child(base, 1);
            } else {
                self.bounds_check(base, child(e, 2));
            }
            let ty = self.env.type_of(base);
            let stride = sub_type(&ty).map_or(1, |s| self.env.size(e, s));
            self.into_hl(base);
            self.asm("PUSH_HL");
            self.scaled_index(child(e, 2), stride);
            self.asm("POP_HL");
            self.asm("ADD_HL_DE");
            return;
        }
        self.unsupported(e, "lvalue address");
    }
    pub(super) fn scaled_index(&mut self, e: &Expr, stride: i32) {
        let e = self.env.fold(e);
        let stride = stride.max(1);
        if stride == 1 {
            if self.size(&e) == 1 {
                if e.is(t::INTEGER) {
                    self.immediate("LD_E_IMM", e.int(1).unwrap() & 255);
                } else {
                    self.into_a(&e);
                    self.asm("LD_E_A");
                }
                self.immediate("LD_D_IMM", 0);
            } else {
                self.into_hl(&e);
                self.asm("LD_D_H");
                self.asm("LD_E_L");
            }
            return;
        }
        if e.is(t::INTEGER) {
            self.word("LD_DE_IMM", e.int(1).unwrap().wrapping_mul(stride) & 65535);
            return;
        }
        if self.size(&e) == 1 {
            self.into_a(&e);
            self.extend_a(false);
        } else {
            self.into_hl(&e);
        }
        if (stride as u32).is_power_of_two() {
            for _ in 0..stride.trailing_zeros() {
                self.asm("ADD_HL_HL");
            }
        } else if !self.constant_multiply(stride) {
            if stride > 255 {
                self.env.error(
                    &e.source,
                    "Element size too large for scaled index multiply loop.",
                );
            }
            self.asm("LD_D_H");
            self.asm("LD_E_L");
            self.word("LD_HL_IMM", 0);
            self.immediate("LD_A_IMM", stride & 255);
            let loop_label = self.unique("idxmul");
            self.label(&loop_label);
            self.asm("ADD_HL_DE");
            self.asm("DEC_A");
            self.op("JP_NZ", loop_label);
        }
        self.asm("LD_D_H");
        self.asm("LD_E_L");
    }
    pub(super) fn call(&mut self, e: &Expr) {
        let target = child(e, 1);
        let args = e.children().into_iter().skip(1).collect::<Vec<_>>();
        let Some(name) = target.text(1).filter(|_| target.is(t::NAME)) else {
            let ty = self.env.type_of(target);
            let (ty, address) = match &ty.kind {
                Kind::Pointer(function) => (function.as_ref(), target),
                Kind::Function(..) if target.is(t::LOAD) => (&ty, child(target, 1)),
                _ => {
                    self.unsupported(e, "indirect call");
                    return;
                }
            };
            if let Kind::Function(result, parameters) = &ty.kind {
                self.indirect_call(e, address, result, parameters, &args);
            } else {
                self.unsupported(e, "indirect call");
            }
            return;
        };
        let name = match name {
            "__settile_xy" => "__settilebg",
            "__vram_copy" | "__vram_copy_hblank" | "__vram_copy_dma" => "__vram_memcpy",
            "__vram_fill" | "__fill_tilemap" => "__vram_memset",
            "__farmemcpy" => "__far_memcpy",
            name => name,
        };
        if self.simple_intrinsic(e, name, &args) {
            return;
        }
        if self.scaled_intrinsic(e, name, &args) {
            return;
        }
        if self.memory_intrinsic(e, name, &args) || self.geometry_intrinsic(e, name, &args) {
            return;
        }
        if self.vram_intrinsic(e, name, &args) {
            return;
        }
        if self.tile_intrinsic(e, name, &args) {
            return;
        }
        if self.cgb_intrinsic(e, name, &args) {
            return;
        }
        if self.far_intrinsic(e, name, &args) {
            return;
        }
        if self.scroll_split_intrinsic(e, name, &args) {
            return;
        }
        if name == "__bankswitch" {
            if args.len() != 1 {
                self.env
                    .error(&e.source, "__bankswitch(bank) expects 1 argument");
                return;
            }
            self.into_a(args[0]);
            if self.current_bank == 0 {
                self.op("LD_MEM_A", Operand::integer(0x2000, M::Absolute));
                self.store(memory_operand(0xff82));
            } else {
                self.bankswitch_helper = true;
                self.op(
                    "CALL",
                    Operand::symbol("__kq_bankswitch_bank0", M::Absolute),
                );
            }
            return;
        }
        let Some(info) = self.env.functions.get(name).cloned() else {
            if let Some(symbol) = self.env.find(name).cloned() {
                if let Kind::Pointer(function) = symbol.ty.kind {
                    if let Kind::Function(result, parameters) = function.kind {
                        if args.len() != parameters.len() {
                            self.env.error(
                                &e.source,
                                format!(
                                    "Wrong number of arguments (fnptr). Expected {}, got {}.",
                                    parameters.len(),
                                    args.len()
                                ),
                            );
                            return;
                        }
                        if parameters.len() > 1 {
                            self.env.error(
                                &e.source,
                                "Function-pointer calls currently support only 0 or 1 argument",
                            );
                            return;
                        }
                        if let Some(arg) = args.first() {
                            if self.env.size(e, &parameters[0]) != 1 {
                                self.env.error(&e.source,"Function-pointer calls currently support only 1-byte arguments (fastcall-A).");
                                return;
                            }
                            self.into_a(arg);
                            self.asm("PUSH_AF");
                        }
                        if result.is_aggregate() {
                            self.prepare_indirect_return_destination(e, &result);
                        }
                        self.into_hl(target);
                        self.indirect_bank_check(false);
                        if !args.is_empty() {
                            self.asm("POP_AF");
                        }
                        let ret = self.unique("icall_ret");
                        self.op(
                            "LD_DE_IMM",
                            Operand::symbol(ret.base.as_deref().unwrap(), M::Immediate16),
                        );
                        self.asm("PUSH_DE");
                        self.asm("JP_HL");
                        self.label(&ret);
                        self.report_call(
                            e,
                            &format!("<indirect:{name}>"),
                            -1,
                            "indirect",
                            false,
                            false,
                            &args,
                            &parameters,
                        );
                        return;
                    }
                }
            }
            self.env.error(&e.source, format!("Undefined function: {name}"));
            return;
        };
        if args.len() != info.parameters.len() {
            self.env.error(
                &e.source,
                format!(
                    "Wrong number of arguments. Expected {}, got {}.",
                    info.parameters.len(), args.len()
                ),
            );
            return;
        }
        if info.inline {
            self.inline_call(e, name, &info, &args);
            return;
        }
        if !info.stack_call && name == self.current_function {
            self.env.error(&e.source,format!("direct recursion requires __stackcall (legacy ABI reuses static parameter/local slots): {name}"));
        }
        if info.stack_call {
            self.stack_call(e, name, &info, &args);
            return;
        }
        self.prepare_return_destination(&info);
        for (i, arg) in args.iter().enumerate() {
            let p = &info.parameter_symbols.as_ref().unwrap()[i];
            let size = self.env.size(e, &p.ty);
            if p.ty.is_aggregate() {
                self.aggregate_argument(e, name, i, arg, &p.ty, Some(p.value), None);
            } else if size == 1 {
                self.into_a(arg);
                if !info.fast_call {
                    self.store(memory_operand(p.value));
                }
            } else if size == 2 {
                self.into_hl(arg);
                if !info.fast_call {
                    self.asm("LD_A_L");
                    self.store(memory_operand(p.value));
                    self.asm("LD_A_H");
                    self.store(memory_operand(p.value + 1));
                }
            } else {
                self.unsupported(e, "aggregate argument");
            }
        }
        let target = if info.rom_bank != 0 && info.rom_bank != self.current_bank {
            self.bank_thunk(name, info.rom_bank)
        } else {
            let preserve = info.fast_call
                && info
                    .parameters
                    .first()
                    .is_some_and(|p| self.env.size(e, &p.ty) == 1);
            self.direct_bank_check(info.rom_bank, preserve);
            name.into()
        };
        self.op("CALL", Operand::symbol(target, M::Absolute));
        let thunk = info.rom_bank != 0 && info.rom_bank != self.current_bank;
        self.report_call(
            e,
            name,
            info.rom_bank,
            if thunk { "bank_thunk" } else { "direct" },
            thunk,
            false,
            &args,
            &info
                .parameters
                .iter()
                .map(|p| p.ty.clone())
                .collect::<Vec<_>>(),
        );
    }
    fn indirect_call(
        &mut self,
        e: &Expr,
        address: &Expr,
        result: &CType,
        parameters: &[CType],
        args: &[&Expr],
    ) {
        if parameters.len() != args.len() {
            self.env.error(
                &e.source,
                format!(
                    "function pointer call expects {} arguments, got {}",
                    parameters.len(),
                    args.len()
                ),
            );
            return;
        }
        if args.len() > 1 {
            self.env.error(
                &e.source,
                "function pointer calls currently support only 0 or 1 argument",
            );
            return;
        }
        if let Some(arg) = args.first() {
            if self.env.size(e, &parameters[0]) != 1 {
                self.env.error(
                    &e.source,
                    "function pointer calls currently support only 1-byte parameter (passed in A)",
                );
                return;
            }
            self.into_a(arg);
            self.asm("PUSH_AF");
        }
        if result.is_aggregate() {
            self.prepare_indirect_return_destination(e, result);
        }
        self.into_hl(address);
        self.indirect_bank_check(true);
        if !args.is_empty() {
            self.asm("POP_AF");
        }
        let returned = self.unique("icall_ret");
        self.op(
            "LD_DE_IMM",
            Operand::symbol(returned.base.as_deref().unwrap(), M::Immediate16),
        );
        self.asm("PUSH_DE");
        self.asm("JP_HL");
        self.label(&returned);
        let callee = if address.is(t::NAME) {
            format!("<indirect:{}>", address.text(1).unwrap())
        } else {
            "<indirect:expr>".into()
        };
        self.report_call(e, &callee, -1, "indirect", false, false, args, parameters);
    }
    pub(super) fn discard(&mut self, e: &Expr) {
        if e.is(t::EMPTY) {
            return;
        }
        if self.env.type_of(e).is_aggregate() {
            self.call(e);
        } else if self.size(e) == 1 {
            self.into_a(e);
        } else if self.size(e) > 1 {
            self.into_hl(e);
        }
    }
    fn assign_expression(&mut self, lhs: &Expr, rhs: &Expr) {
        if self.env.type_of(lhs).is_aggregate() || self.env.type_of(rhs).is_aggregate() {
            self.env.error(
                &lhs.source,
                "struct/union assignment cannot be used as a scalar expression",
            );
            self.immediate("LD_A_IMM", 0);
            return;
        }
        if let Some(op) = self.operand(lhs) {
            let size = self.size(lhs);
            if size == 1 {
                self.into_a(rhs);
                self.store(op);
            } else if self.size(rhs) == 2 {
                self.into_hl(rhs);
                self.asm("LD_A_L");
                self.store(op.clone());
                self.asm("LD_A_H");
                let mut hi = op;
                hi.offset += 1;
                self.store(hi);
                self.asm("LD_A_L");
            } else {
                self.into_a(rhs);
                self.asm("PUSH_AF");
                self.store(op.clone());
                self.immediate("LD_A_IMM", 0);
                let mut hi = op;
                hi.offset += 1;
                self.store(hi);
                self.asm("POP_AF");
            }
            return;
        }
        if lhs.is(t::LOAD) || lhs.is(t::INDEX) {
            let size = self.size(lhs);
            if lhs.is(t::LOAD) {
                self.into_hl(child(lhs, 1));
            } else {
                self.index_address(lhs);
            }
            if size == 1 && lhs.is(t::LOAD) {
                self.into_a(rhs);
                self.asm("LD_HL_A");
                return;
            }
            self.asm("PUSH_HL");
            if size == 1 {
                self.into_a(rhs);
                self.asm("POP_HL");
                self.asm("LD_HL_A");
            } else {
                if self.size(rhs) == 2 {
                    self.into_hl(rhs);
                    for m in ["LD_A_L", "LD_E_A", "LD_A_H", "LD_D_A"] {
                        self.asm(m);
                    }
                } else {
                    self.into_a(rhs);
                    self.asm("LD_E_A");
                    self.immediate("LD_D_IMM", 0);
                }
                for m in [
                    "POP_HL", "LD_A_E", "LD_HL_A", "INC_HL", "LD_A_D", "LD_HL_A", "LD_A_E",
                ] {
                    self.asm(m);
                }
            }
            return;
        }
        self.unsupported(lhs, "assignment expression lvalue");
    }
}
