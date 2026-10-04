use super::*;
impl Emitter {
    pub(super) fn ensure_return_slot(&mut self, e: &Expr, name: &str, info: &mut FunctionInfo) {
        if !info.return_type.is_aggregate() || info.return_symbol.is_some() {
            return;
        }
        let ty = &info.return_type;
        let size = self.env.size(e, ty);
        let align = ty.forced_align.max(self.env.natural_align(ty)).max(1);
        let value = self.env.allocate_preferred(size, align, &[1, 2, 0]);
        let region = infer_region(value);
        let slot = format!("__kq_sret_{name}");
        self.emit(
            t::VARIABLE,
            vec![
                slot.clone().into(),
                value.into(),
                size.into(),
                region.into(),
            ],
        );
        if size > 16 {
            self.env.warning(&e.source,format!("struct return value of {size} bytes from {name}; prefer an output pointer for large aggregates"));
        }
        info.return_symbol = Some(Symbol {
            tag: SymbolTag::Global,
            value,
            ty: ty.clone(),
            name: slot,
            wram_bank: 0,
        });
        let value = self.env.allocate_preferred(2, 1, &[0, 1, 2]);
        let slot = format!("__kq_sret_dst_{name}");
        self.emit(
            t::VARIABLE,
            vec![
                slot.clone().into(),
                value.into(),
                2.into(),
                infer_region(value).into(),
            ],
        );
        info.return_pointer_symbol = Some(Symbol {
            tag: SymbolTag::Global,
            value,
            ty: CType::pointer(ty.clone()),
            name: slot,
            wram_bank: 0,
        });
    }
    pub(super) fn prepare_return_destination(&mut self, info: &FunctionInfo) {
        if let Some(s) = &info.return_symbol {
            self.word("LD_HL_IMM", s.value);
            self.asm("LD_A_L");
            self.store(memory_operand(0xff84));
            self.asm("LD_A_H");
            self.store(memory_operand(0xff85));
            self.emit(t::COMMENT, vec!["kitaqgb.struct_return_dest".into()]);
            self.last_sret_address = Some(s.value);
        } else {
            self.last_sret_address = None;
        }
    }
    pub(super) fn prepare_indirect_return_destination(&mut self, e: &Expr, ty: &CType) {
        let size = self.env.size(e, ty);
        let align = ty.forced_align.max(self.env.natural_align(ty)).max(1);
        let address = self.env.allocate_preferred(size, align, &[1, 2, 0]);
        self.return_temp_counter += 1;
        let name = format!("__kq_sret_tmp_{}_fp", self.return_temp_counter);
        self.emit(
            t::VARIABLE,
            vec![
                name.into(),
                address.into(),
                size.into(),
                infer_region(address).into(),
            ],
        );
        self.word("LD_HL_IMM", address);
        self.asm("LD_A_L");
        self.store(memory_operand(0xff84));
        self.asm("LD_A_H");
        self.store(memory_operand(0xff85));
        self.emit(t::COMMENT, vec!["kitaqgb.struct_return_dest".into()]);
        self.last_sret_address = Some(address);
    }
    pub(super) fn save_incoming_return_pointer(&mut self, info: &FunctionInfo) {
        if let Some(s) = &info.return_pointer_symbol {
            self.load(memory_operand(0xff84));
            self.store(memory_operand(s.value));
            self.load(memory_operand(0xff85));
            self.store(memory_operand(s.value + 1));
        }
    }
    pub(super) fn aggregate_argument(
        &mut self,
        call: &Expr,
        name: &str,
        index: usize,
        arg: &Expr,
        ty: &CType,
        destination: Option<i32>,
        offset: Option<i32>,
    ) {
        let actual = self.env.type_of(arg);
        if ty.kind != actual.kind {
            self.env.error(&arg.source,format!("incompatible struct/union value argument for {name} argument {index}: expected {ty}, got {actual}"));
            return;
        }
        let size = self.env.size(call, ty);
        if size > 16 {
            self.env.warning(&call.source,format!("struct value argument copy of {size} bytes when calling {name} argument {index}; prefer pointer passing for large aggregates"));
        }
        if !self.aggregate_storage(arg, false, size) {
            return;
        }
        if let Some(address) = destination {
            if let Some(region) = special_copy_region(address, size) {
                self.env.error(
                    &call.source,
                    format!(
                        "implicit struct/union value argument copy to {region} is not supported"
                    ),
                );
                return;
            }
        }
        let strategy = copy_strategy(size);
        self.report_copy(call, ty, size, strategy);
        let extra = offset.map_or(String::new(), |o| format!(" stack_off={o}"));
        self.emit(t::COMMENT,vec![format!("kitaqgb.struct_value_arg callee={name} arg={index} type={} size={size} strategy={strategy}{extra}",ty.without_const()).into()]);
        self.address(arg);
        if let Some(address) = destination {
            self.asm("PUSH_HL");
            self.word("LD_HL_IMM", address);
            self.asm("POP_DE");
        } else {
            self.asm("LD_D_H");
            self.asm("LD_E_L");
            self.op(
                "LD_HL_SP_IMM",
                Operand::integer(offset.unwrap(), M::Relative),
            );
        }
        self.aggregate_copy_bytes(size);
    }
    pub(super) fn aggregate_return(&mut self, e: &Expr) {
        let ty = self.return_type.clone();
        let actual = self.env.type_of(e);
        if ty.kind != actual.kind {
            self.env.error(
                &e.source,
                format!("incompatible struct/union return: expected {ty}, got {actual}"),
            );
            return;
        }
        let size = self.env.size(e, &ty);
        if !self.aggregate_storage(e, false, size) {
            return;
        }
        let name = if self.inline_returns.is_empty() {
            self.current_function.clone()
        } else {
            "<inline>".into()
        };
        self.report_copy(e, &ty, size, copy_strategy(size));
        self.emit(
            t::COMMENT,
            vec![
                format!(
                    "kitaqgb.struct_return callee={name} type={} size={size} strategy={}",
                    ty.without_const(),
                    copy_strategy(size)
                )
                .into(),
            ],
        );
        self.address(e);
        self.asm("PUSH_HL");
        if let Some(address) = self.inline_destinations.last().copied() {
            self.word("LD_HL_IMM", address);
            self.asm("POP_DE");
            self.aggregate_copy_bytes(size);
            return;
        }
        let Some(symbol) = self
            .env
            .functions
            .get(&name)
            .and_then(|f| f.return_pointer_symbol.clone())
        else {
            self.env
                .error(&e.source, "missing struct return destination");
            return;
        };
        self.load(memory_operand(symbol.value));
        self.asm("LD_L_A");
        self.load(memory_operand(symbol.value + 1));
        self.asm("LD_H_A");
        self.asm("POP_DE");
        self.aggregate_copy_bytes(size);
    }
    pub(super) fn aggregate_assignment(
        &mut self,
        origin: &Expr,
        left: &Expr,
        right: &Expr,
    ) -> bool {
        let ty = self.env.type_of(left);
        let source_ty = self.env.type_of(right);
        if !ty.is_aggregate() && !source_ty.is_aggregate() {
            return false;
        }
        let complete = match &ty.kind {
            Kind::Struct(n) | Kind::Union(n) => self
                .env
                .aggregates
                .get(n)
                .is_some_and(|a| a.total_size >= 0),
            _ => false,
        };
        if !complete || ty.kind != source_ty.kind {
            self.env.error(
                &left.source,
                format!("incompatible struct/union assignment: {ty} = {source_ty}"),
            );
            return true;
        }
        let size = self.env.size(left, &ty);
        if !self.aggregate_storage(left, true, size) || !self.aggregate_storage(right, false, size)
        {
            return true;
        }
        let strategy = if size <= 2 {
            "scalar"
        } else if size <= 16 {
            "unrolled"
        } else if size <= 255 {
            "__memcpy_small"
        } else {
            "__memcpy"
        };
        self.report_copy(origin, &ty, size, strategy);
        self.emit(
            t::COMMENT,
            vec![
                format!(
                    "kitaqgb.struct_copy type={} size={size} strategy={strategy}",
                    ty.without_const()
                )
                .into(),
            ],
        );
        if size <= 0 {
            return true;
        }
        self.address(right);
        self.asm("PUSH_HL");
        self.address(left);
        self.asm("POP_DE");
        self.aggregate_copy_bytes(size);
        true
    }
    pub(super) fn aggregate_storage(&mut self, e: &Expr, destination: bool, size: i32) -> bool {
        let mut e = e;
        while e.is(t::CAST) {
            e = child(e, 2);
        }
        let direction = if destination { "to" } else { "from" };
        if !destination && e.is(t::CALL) && self.env.type_of(e).is_aggregate() {
            if let Some(name) = child(e, 1).text(1).filter(|_| child(e, 1).is(t::NAME)) {
                if let Some(address) = self
                    .env
                    .functions
                    .get(name)
                    .and_then(|f| f.return_symbol.as_ref())
                    .map(|s| s.value)
                {
                    if let Some(region) = special_copy_region(address, size) {
                        self.env.error(&e.source, format!("implicit struct/union copy from struct return storage in {region} is not supported"));
                        return false;
                    }
                }
            }
            return true;
        }
        let reason = if e.is(t::NAME) {
            let name = e.text(1).unwrap();
            match self.env.find(name).cloned() {
                None=>Some(format!("unknown aggregate object: {name}")),
                Some(s)if s.tag==SymbolTag::ReadonlyData=>Some(format!("implicit struct/union copy {direction} readonly ROM data is not supported; use an explicit copy routine")),
                Some(s)if s.tag==SymbolTag::Constant=>Some(format!("implicit struct/union copy {direction} a constant is not supported")),
                Some(s)if s.tag==SymbolTag::Global&&s.wram_bank>1=>Some(format!("implicit struct/union copy {direction} WRAMX bank {} is not supported; switch SVBK and copy explicitly",s.wram_bank)),
                Some(s)if matches!(s.tag,SymbolTag::Global|SymbolTag::Local)=>special_copy_region(s.value,size).map(|region|format!("implicit struct/union copy {direction} {region} is not supported; use the explicit hardware/far copy API")),
                _=>None,
            }
        } else if e.is(t::FIELD) || e.is(t::INDEX) {
            return self.aggregate_storage(child(e, 1), destination, size);
        } else if e.is(t::LOAD) {
            self.aggregate_constant_address(child(e,1)).and_then(|addr|special_copy_region(addr,size)).map(|region|format!("implicit struct/union copy {direction} {region} is not supported; use the explicit hardware/far copy API"))
        } else {
            Some(format!(
                "struct/union copy requires an addressable {}",
                if destination { "destination" } else { "source" }
            ))
        };
        if let Some(reason) = reason {
            self.env.error(&e.source, reason);
            false
        } else {
            true
        }
    }
    fn aggregate_constant_address(&self, mut e: &Expr) -> Option<i32> {
        while e.is(t::CAST) {
            e = child(e, 2);
        }
        if e.is(t::INTEGER) {
            return e.int(1);
        }
        if e.is(t::ADDRESS_OF) {
            let e = child(e, 1);
            if e.is(t::NAME) {
                return self
                    .env
                    .find(e.text(1)?)
                    .filter(|s| {
                        matches!(
                            s.tag,
                            SymbolTag::Global | SymbolTag::Local | SymbolTag::ReadonlyData
                        )
                    })
                    .map(|s| s.value);
            }
        }
        None
    }
    pub(super) fn aggregate_copy_bytes(&mut self, size: i32) {
        if size <= 0 {
            return;
        }
        if size > 16 && size <= 255 {
            let block = if size <= 64 { 8 } else { 16 };
            let blocks = size / block;
            let remainder = size % block;
            if blocks > 0 {
                self.immediate("LD_B_IMM", blocks);
                let loop_label = self.unique("aggcpy_loop");
                self.label(&loop_label);
                for _ in 0..block {
                    self.asm("LD_A_DE");
                    self.asm("LDI_HL_A");
                    self.asm("INC_DE");
                }
                self.asm("DEC_B");
                self.op("JP_NZ", loop_label);
            }
            for i in 0..remainder {
                self.asm("LD_A_DE");
                self.asm("LDI_HL_A");
                if i != remainder - 1 {
                    self.asm("INC_DE");
                }
            }
        } else if size > 255 {
            self.word("LD_BC_IMM", size & 65535);
            let loop_label = self.unique("aggcpy16_loop");
            self.label(&loop_label);
            for m in ["LD_A_DE", "LDI_HL_A", "INC_DE", "DEC_BC", "LD_A_B", "OR_C"] {
                self.asm(m);
            }
            self.op("JP_NZ", loop_label);
        } else {
            for i in 0..size {
                self.asm("LD_A_DE");
                self.asm("LDI_HL_A");
                if i != size - 1 {
                    self.asm("INC_DE");
                }
            }
        }
    }
}
fn copy_strategy(size: i32) -> &'static str {
    if size <= 2 {
        "scalar"
    } else if size <= 16 {
        "unrolled"
    } else if size <= 255 {
        "__memcpy_small"
    } else {
        "__memcpy"
    }
}
fn special_copy_region(address: i32, size: i32) -> Option<&'static str> {
    let start = address & 65535;
    let end = (start + size.max(1) - 1) & 65535;
    for (lo, hi, name) in [
        (0, 0x7fff, "ROM"),
        (0x8000, 0x9fff, "VRAM"),
        (0xfe00, 0xfe9f, "OAM"),
        (0xff00, 0xff7f, "IO"),
    ] {
        if end < start
            || (lo..=hi).contains(&start)
            || (lo..=hi).contains(&end)
            || (start <= lo && end >= hi)
        {
            return Some(name);
        }
    }
    None
}
