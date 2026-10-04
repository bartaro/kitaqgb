use super::*;

#[derive(Clone)]
pub(super) struct Declaration {
    pub(super) expr: Expr,
    bank: i32,
    fixed_bank: bool,
    order: Option<i32>,
    align: i32,
    section: Option<String>,
    stack: bool,
    unsafe_body: bool,
}
fn unwrap(e: &Expr) -> Declaration {
    let mut d = Declaration {
        expr: e.clone(),
        bank: 1,
        fixed_bank: false,
        order: None,
        align: 0,
        section: None,
        stack: false,
        unsafe_body: false,
    };
    loop {
        let next = match d.expr.tag().unwrap_or("") {
            t::FIXED_BANK => {
                d.bank = d.expr.int(1).unwrap();
                d.fixed_bank = true;
                2
            }
            t::BANK => {
                d.bank = d.expr.int(1).unwrap();
                2
            }
            t::FIXED_ORDER => {
                d.order = d.expr.int(1);
                2
            }
            t::DECL_ALIGN => {
                d.align = d.align.max(d.expr.int(1).unwrap());
                2
            }
            t::DECL_SECTION => {
                d.section = d.expr.text(1).map(str::to_owned);
                2
            }
            t::STACK_CALL => {
                d.stack = true;
                1
            }
            t::UNSAFE => {
                d.unsafe_body = true;
                1
            }
            t::STATIC => 1,
            _ => break,
        };
        d.expr = child(&d.expr, next).with_source(d.expr.source.clone());
    }
    d
}
fn scan_near(e: &Expr, names: &BTreeSet<String>, near: &mut BTreeSet<String>) {
    if e.is(t::CALL) {
        if child(e, 1).text(1) == Some("__bankof") {
            return;
        }
        let paired = e
            .children()
            .into_iter()
            .skip(1)
            .filter(|a| {
                a.is(t::CALL) && child(a, 1).text(1) == Some("__bankof") && a.args.len() == 3
            })
            .filter_map(|a| {
                if child(a, 2).is(t::NAME) {
                    child(a, 2).text(1).map(str::to_owned)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        for a in e.children() {
            if a.is(t::NAME) && a.text(1).is_some_and(|n| paired.contains(n)) {
                continue;
            }
            scan_near(a, names, near);
        }
        return;
    }
    if e.is(t::NAME) && e.text(1).is_some_and(|n| names.contains(n)) {
        near.insert(e.text(1).unwrap().into());
        return;
    }
    for a in e.children() {
        scan_near(a, names, near);
    }
}
impl Emitter {
    pub(super) fn program(&mut self, program: &Expr) {
        if !program.is(t::SEQUENCE) {
            self.env.error(
                &program.source,
                "The top level of the syntax tree must be a sequence.",
            );
            return;
        }
        self.env.allocator.allocate(0, 2, 1); // register L/H scratch slots
        for (name, size, ty) in [
            ("__rom_bank", 1, CType::simple(Simple::UInt8)),
            ("__rom_bank_saved", 1, CType::simple(Simple::UInt8)),
            (
                "__kq_sret_ptr",
                2,
                CType::pointer(CType::simple(Simple::UInt8)),
            ),
            ("__kq_thunk_sp", 1, CType::simple(Simple::UInt8)),
            (
                "__kq_thunk_bank_stack",
                8,
                CType::array(CType::simple(Simple::UInt8), 8),
            ),
            (
                "__kq_thunk_retlo_stack",
                8,
                CType::array(CType::simple(Simple::UInt8), 8),
            ),
            (
                "__kq_thunk_rethi_stack",
                8,
                CType::array(CType::simple(Simple::UInt8), 8),
            ),
            ("__kq_critical_depth", 1, CType::simple(Simple::UInt8)),
            ("__kq_rng_lo", 1, CType::simple(Simple::UInt8)),
            ("__kq_rng_hi", 1, CType::simple(Simple::UInt8)),
        ] {
            let value = self.env.allocator.allocate(0, size, 1).unwrap();
            self.emit(t::VARIABLE, vec![name.into(), value.into(), size.into()]);
            self.env.declare(
                program,
                Symbol {
                    tag: SymbolTag::Global,
                    value,
                    ty,
                    name: name.into(),
                    wram_bank: 0,
                },
            );
        }
        fn uses_dma(e: &Expr) -> bool {
            (e.is(t::CALL) && child(e, 1).is(t::NAME) && child(e, 1).text(1) == Some("__oam_dma"))
                || e.children().iter().any(|e| uses_dma(e))
        }
        if uses_dma(program) {
            let value = self.env.allocator.allocate(0, 8, 1).unwrap();
            self.emit(
                t::VARIABLE,
                vec!["__kq_oam_dma_stub".into(), value.into(), 8.into()],
            );
            self.env.declare(
                program,
                Symbol {
                    tag: SymbolTag::Global,
                    value,
                    ty: CType::array(CType::simple(Simple::UInt8), 8),
                    name: "__kq_oam_dma_stub".into(),
                    wram_bank: 0,
                },
            );
        }
        let mut declarations = program
            .children()
            .into_iter()
            .map(unwrap)
            .collect::<Vec<_>>();
        let names = declarations
            .iter()
            .filter_map(|d| readonly::declaration(&d.expr).map(|(_, n, _)| n.to_owned()))
            .collect::<BTreeSet<_>>();
        let mut near = BTreeSet::new();
        scan_near(program, &names, &mut near);
        for d in &mut declarations {
            if !d.fixed_bank
                && readonly::declaration(&d.expr).is_some_and(|(_, n, _)| near.contains(n))
            {
                d.bank = 0;
                d.fixed_bank = true;
            }
            if !d.fixed_bank
                && matches!(
                    d.expr.tag(),
                    Some(t::FUNCTION | t::FUNCTION_DECL | t::INLINE_FUNCTION)
                )
            {
                if let Some(bank) = self.env.options.function_banks.get(d.expr.text(2).unwrap()) {
                    d.bank = *bank;
                }
            }
        }
        if self.env.options.const_scalar_in_rom {
            for d in &mut declarations {
                if d.expr.is(t::CONSTANT) {
                    let ty = d.expr.ty(1).unwrap();
                    if ty.is_const
                        && matches!(
                            ty.kind,
                            Kind::Simple(
                                Simple::UInt8 | Simple::Int8 | Simple::UInt16 | Simple::Int16
                            )
                        )
                    {
                        d.expr = Expr::new(
                            t::READONLY_DATA,
                            vec![
                                ty.clone().into(),
                                d.expr.text(2).unwrap().into(),
                                vec![child(&d.expr, 3).clone()].into(),
                            ],
                        )
                        .with_source(d.expr.source.clone());
                    }
                }
            }
        }
        for d in &declarations {
            let e = &d.expr;
            if e.is(t::CONSTANT) {
                self.env
                    .add_constant(e, e.ty(1).unwrap(), e.text(2).unwrap(), child(e, 3));
            }
        }
        for d in &declarations {
            let e = &d.expr;
            match e.tag().unwrap_or("") {
                t::CONSTANT => {
                    self.env.ensure_constant(e.text(2).unwrap(), e);
                }
                t::OPAQUE_STRUCT | t::OPAQUE_UNION => self.env.opaque(
                    e.text(1).unwrap(),
                    if e.is(t::OPAQUE_STRUCT) {
                        AggregateLayout::Struct
                    } else {
                        AggregateLayout::Union
                    },
                ),
                t::STRUCT | t::UNION => {
                    let Arg::Fields(fields) = &e.args[2] else {
                        continue;
                    };
                    self.env.define_aggregate(
                        e,
                        e.text(1).unwrap(),
                        fields,
                        if e.is(t::STRUCT) {
                            AggregateLayout::Struct
                        } else {
                            AggregateLayout::Union
                        },
                        e.int(3).unwrap_or(0) != 0,
                        e.int(4).unwrap_or(0),
                    );
                }
                _ => {}
            }
        }
        for d in declarations
            .iter()
            .filter(|d| d.expr.is(t::INLINE_FUNCTION) && d.expr.args.len() == 5)
        {
            self.declare_function(d);
        }
        for d in &declarations {
            let e = &d.expr;
            if matches!(e.tag(), Some(t::FUNCTION | t::FUNCTION_DECL)) {
                self.declare_function(d);
            } else if e.is(t::VARIABLE) {
                let Arg::Region(region) = e.args[1] else {
                    continue;
                };
                let ty = self.env.resolve_dimension(e.ty(2).unwrap());
                self.env
                    .declare_global(e, region, &ty, e.text(3).unwrap(), d.align);
                self.lines.append(&mut self.env.placements);
            } else if let Some((ty, name, _)) = readonly::declaration(e) {
                self.env.register_readonly(e, ty, name, d.align, d.bank);
            } else if e.is(t::STATIC_ASSERT) {
                let before = self.env.diagnostics.len();
                let value = self.env.constant(child(e, 1)).value;
                if value == 0
                    && !self.env.diagnostics[before..]
                        .iter()
                        .any(|d| d.severity == crate::tokenizer::Severity::Error)
                {
                    self.env
                        .static_assert_failure(e, child(e, 1), e.text(2).unwrap_or(""), 0);
                }
            }
        }
        self.validate_externs(&declarations);
        let banks = declarations.iter().map(|d| d.bank).collect::<BTreeSet<_>>();
        for bank in banks {
            self.emit(t::SKIP_TO, vec![(bank * 0x4000).into()]);
            let mut items = declarations
                .iter()
                .filter(|d| d.bank == bank)
                .cloned()
                .collect::<Vec<_>>();
            let positions = items
                .iter()
                .enumerate()
                .filter_map(|(i, d)| d.order.map(|_| i))
                .collect::<Vec<_>>();
            let mut marked = positions
                .iter()
                .map(|i| items[*i].clone())
                .collect::<Vec<_>>();
            marked.sort_by_key(|d| d.order);
            for (i, d) in positions.into_iter().zip(marked) {
                items[i] = d;
            }
            items.sort_by_key(|d| {
                if d.order.is_none()
                    && readonly::declaration(&d.expr).is_some_and(|(_, n, _)| near.contains(n))
                {
                    0
                } else {
                    1
                }
            });
            for d in items {
                if let Some((ty, name, values)) = readonly::declaration(&d.expr) {
                    self.env.declare_readonly(
                        &d.expr,
                        ty,
                        name,
                        &values,
                        d.align,
                        d.section.as_deref(),
                    );
                    self.lines.append(&mut self.env.placements);
                } else if d.expr.is(t::INLINE_FUNCTION) {
                    if !self.env.functions.contains_key(d.expr.text(2).unwrap()) {
                        self.declare_function(&d);
                    }
                } else if d.expr.is(t::FUNCTION) {
                    self.function(&d);
                }
            }
        }
    }
    fn declare_function(&mut self, d: &Declaration) {
        let e = &d.expr;
        let name = e.text(2).unwrap();
        let return_type = e.ty(1).unwrap().clone();
        let Arg::Fields(parameters) = &e.args[3] else {
            return;
        };
        let prototype = e.is(t::FUNCTION_DECL);
        let stack = !e.is(t::INLINE_FUNCTION) && (self.env.options.abi_stack || d.stack);
        let body = if prototype {
            None
        } else {
            e.args.last().and_then(|a| {
                if let Arg::Expr(e) = a {
                    Some((**e).clone())
                } else {
                    None
                }
            })
        };
        let must_check = e.int(4).unwrap_or(0) != 0;
        let fast_call = !stack
            && parameters.len() == 1
            && !parameters[0].ty.is_aggregate()
            && matches!(self.env.size(e, &parameters[0].ty), 1 | 2);
        if let Some(mut info) = self.env.functions.get(name).cloned() {
            if !prototype && !info.prototype {
                self.env
                    .error(&e.source, format!("function redefined: {name}"));
            }
            if info.parameters.len() != parameters.len() {
                self.env
                    .error(&e.source, format!("function prototype mismatch: {name}"));
            }
            if info.stack_call != stack {
                self.env.error(
                    &e.source,
                    format!("Function calling convention mismatch (__stackcall): {name}"),
                );
            }
            info.must_check |= must_check;
            if !prototype || info.prototype {
                info.set_definition_parameters(parameters.clone());
                info.return_type = return_type;
                info.fast_call = fast_call;
                info.stack_call = stack;
                info.rom_bank = d.bank;
                info.fixed_bank = d.fixed_bank;
                info.placement_order = d.order;
                info.prototype = prototype;
                info.body = body;
            }
            self.ensure_return_slot(e, name, &mut info);
            self.env.functions.insert(name.into(), info);
            return;
        }
        let parameter_symbols = if stack {
            None
        } else {
            Some(
                parameters
                    .iter()
                    .map(|p| {
                        let size = self.env.size(e, &p.ty);
                        let regions = if e.is(t::INLINE_FUNCTION) && e.args.len() > 5 {
                            [0, 1, 2]
                        } else {
                            [1, 2, 0]
                        };
                        let value = self.env.allocate_preferred(size, 1, &regions);
                        Symbol {
                            tag: SymbolTag::Local,
                            value,
                            ty: p.ty.clone(),
                            name: p.name.clone(),
                            wram_bank: 0,
                        }
                    })
                    .collect(),
            )
        };
        let mut info = FunctionInfo {
            parameters: parameters.clone(),
            parameter_symbols,
            return_type,
            return_symbol: None,
            return_pointer_symbol: None,
            fast_call,
            stack_call: stack,
            prototype,
            must_check,
            rom_bank: d.bank,
            fixed_bank: d.fixed_bank,
            placement_order: d.order,
            inline: e.is(t::INLINE_FUNCTION),
            body,
        };
        self.ensure_return_slot(e, name, &mut info);
        if !self.function_order.iter().any(|n| n == name) {
            self.function_order.push(name.into());
        }
        self.env.functions.insert(name.into(), info);
    }
    fn function(&mut self, d: &Declaration) {
        let saved_unsafe_depth = self.unsafe_depth;
        if d.unsafe_body {
            self.unsafe_depth += 1;
        }
        let e = &d.expr;
        let name = e.text(2).unwrap();
        let info = self.env.functions[name].clone();
        if let Some(section) = &d.section {
            self.emit(t::SECTION, vec![section.clone().into()]);
        }
        if d.align > 1 {
            self.emit(t::ALIGN, vec![d.align.into()]);
        }
        self.env.allocator.begin_frame();
        self.env.begin_scope();
        self.next_label = 0;
        self.current_bank = d.bank;
        self.current_function = name.into();
        self.return_type = info.return_type.clone();
        self.stack_base = None;
        self.emit(t::FUNCTION, vec![name.into()]);
        if name == "main" {
            self.asm("XOR_A");
            self.store(memory_operand(0xff9f));
            self.store(memory_operand(0xff86));
        }
        let body_start = self.lines.len();
        self.stack_usage = Some((0, 0));
        if let Some(parameters) = &info.parameter_symbols {
            for p in parameters {
                self.env.declare(e, p.clone());
            }
        }
        if info.stack_call {
            self.stack_parameters(e, &info);
        }
        if info.fast_call {
            let p = &info.parameter_symbols.as_ref().unwrap()[0];
            let size = self.env.size(e, &p.ty);
            if size == 1 {
                self.store(memory_operand(p.value));
            } else {
                self.asm("LD_A_L");
                self.store(memory_operand(p.value));
                self.asm("LD_A_H");
                self.store(memory_operand(p.value + 1));
            }
        }
        if !info.stack_call {
            self.save_incoming_return_pointer(&info);
        }
        let body = info.body.as_ref().unwrap();
        self.usage.clear();
        self.usage(body, 1);
        self.statement(body);
        self.return_from_function();
        self.env.end_scope();
        self.env.allocator.commit_frame();
        let peak = self.stack_usage.take().unwrap().1;
        let body = self.lines.split_off(body_start);
        self.function_stack_check(&info, e, peak);
        self.lines.extend(body);
        self.unsafe_depth = saved_unsafe_depth;
    }
}
