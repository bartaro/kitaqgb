use super::*;
impl Emitter {
    pub(super) fn statement(&mut self, e: &Expr) {
        if !e.is(t::SEQUENCE) && !e.is(t::EMPTY) {
            let label = if e.is(t::INTEGER) {
                e.int(1).unwrap().to_string()
            } else if e.is(t::NAME) {
                e.text(1).unwrap().into()
            } else {
                e.tag().unwrap().replace('$', "")
            };
            self.emit(t::COMMENT, vec![label.into()]);
        }
        match e.tag().unwrap_or("") {
            t::EMPTY | t::FALLTHROUGH => {}
            t::SEQUENCE => {
                for item in e.children() {
                    self.statement(item)
                }
            }
            t::UNSAFE => {
                self.unsafe_depth += 1;
                self.statement(child(e, 1));
                self.unsafe_depth -= 1;
            }
            t::VARIABLE => {
                let name = e.text(2).unwrap();
                self.env.declare_local(
                    e,
                    e.ty(1).unwrap(),
                    name,
                    self.usage.get(name).is_some_and(|n| *n >= 8),
                );
            }
            t::STATIC_ASSERT => {
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
            t::ASSIGN_MODIFY => {
                let left = child(e, 2);
                let right = Expr::new(
                    e.text(1).unwrap(),
                    vec![left.clone().into(), child(e, 3).clone().into()],
                );
                self.statement(&Expr::new(
                    t::ASSIGN,
                    vec![left.clone().into(), right.into()],
                ));
            }
            t::ASM => {
                let (m, o) = e.asm_parts().unwrap();
                let mut m = m.to_owned();
                let mut o = o.clone();
                if let Some(s) = o
                    .base
                    .as_deref()
                    .and_then(|name| self.env.find(name))
                    .cloned()
                    .filter(|s| s.tag != SymbolTag::ReadonlyData)
                {
                    let high = s.value >= 0xff00;
                    let mode = if high && o.mode == M::Absolute {
                        M::HighMem
                    } else {
                        o.mode
                    };
                    o = o
                        .replace_base(if high { s.value & 255 } else { s.value })
                        .unwrap()
                        .with_mode(mode)
                        .with_comment(s.name);
                    if high {
                        if m == "LD_A_MEM" {
                            m = "LDH_A_MEM".into();
                        } else if m == "LD_MEM_A" {
                            m = "LDH_MEM_A".into();
                        }
                    }
                }
                self.op(&m, o);
            }
            t::ASSIGN => self.assignment(e, child(e, 1), child(e, 2)),
            t::IF => {
                let parts = e.children();
                let end = self.unique("end_if");
                let mut used = false;
                for (i, pair) in parts.chunks_exact(2).enumerate() {
                    let test = self.env.fold(pair[0]);
                    if test.is(t::INTEGER) {
                        if test.int(1).unwrap() == 0 {
                            continue;
                        }
                        self.env.begin_scope();
                        self.statement(pair[1]);
                        self.env.end_scope();
                        if used {
                            self.label(&end);
                        }
                        return;
                    }
                    let otherwise = self.unique("else");
                    self.jump_if(false, &test, &otherwise);
                    self.env.begin_scope();
                    self.statement(pair[1]);
                    self.env.end_scope();
                    if (i + 1) * 2 < parts.len() {
                        self.op("JP", end.clone());
                        used = true;
                    }
                    self.label(&otherwise);
                }
                if used {
                    self.label(&end);
                }
            }
            t::DO_WHILE => {
                let top = self.unique("do_top");
                let check = self.unique("do_check");
                let end = self.unique("do_end");
                self.loops.push((check.clone(), end.clone()));
                self.label(&top);
                self.env.begin_scope();
                self.breaks.push(end.clone());
                self.statement(child(e, 1));
                self.breaks.pop();
                self.label(&check);
                self.jump_if(true, child(e, 2), &top);
                self.label(&end);
                self.env.end_scope();
                self.loops.pop();
            }
            t::FOR => {
                if self.countdown(e) {
                    return;
                }
                let test = self.env.fold(child(e, 2));
                if test.is(t::INTEGER) && test.int(1).unwrap() == 0 {
                    self.env.begin_scope();
                    self.statement(child(e, 1));
                    self.env.end_scope();
                    return;
                }
                let top = self.unique("loop_top");
                let end = self.unique("loop_end");
                self.loops.push((top.clone(), end.clone()));
                self.env.begin_scope();
                self.statement(child(e, 1));
                self.label(&top);
                if !test.is(t::EMPTY) {
                    self.jump_if(false, &test, &end);
                }
                self.breaks.push(end.clone());
                self.statement(child(e, 4));
                self.breaks.pop();
                self.statement(child(e, 3));
                self.op("JP", top);
                self.label(&end);
                self.env.end_scope();
                self.loops.pop();
            }
            t::BREAK => {
                if let Some(label) = self.breaks.last().cloned() {
                    self.op("JP", label)
                } else {
                    self.env.error(&e.source, "break outside of loop or switch")
                }
            }
            t::CONTINUE => {
                if let Some((label, _)) = self.loops.last().cloned() {
                    self.op("JP", label)
                } else {
                    self.env.error(&e.source, "continue outside of loop")
                }
            }
            t::RETURN => {
                if let Some(value) = e.child(1) {
                    if self.return_type.is_aggregate() {
                        self.aggregate_return(value);
                    } else if self.env.size(e, &self.return_type.clone()) == 2 {
                        self.into_hl(value);
                    } else {
                        self.into_a(value);
                    }
                }
                if let Some(label) = self.inline_returns.last().cloned() {
                    self.op("JP", label);
                } else {
                    self.return_from_function();
                }
            }
            t::LABEL => self.lines.push(e.clone()),
            t::SWITCH => self.switch(e),
            t::JUMP => self.op("JP", Operand::symbol(e.text(1).unwrap(), M::Absolute)),
            t::CALL => {
                if let Some(name) = child(e, 1).text(1).filter(|_| child(e, 1).is(t::NAME)) {
                    if self.env.functions.get(name).is_some_and(|f| {
                        f.must_check && !matches!(f.return_type.kind, Kind::Simple(Simple::Void))
                    }) {
                        self.env
                            .warning(&e.source, format!("return value of '{name}' is ignored"));
                    }
                }
                if child(e, 1).is(t::NAME) && child(e, 1).text(1) == Some("__assert") {
                    self.runtime_assert(e, &e.children().into_iter().skip(1).collect::<Vec<_>>());
                    return;
                }
                self.call(e);
                if !self.env.type_of(e).is_aggregate() && self.size(e) == 2 {
                    self.asm("LD_A_L");
                }
            }
            t::PRE_INCREMENT | t::POST_INCREMENT | t::PRE_DECREMENT | t::POST_DECREMENT => {
                let target = child(e, 1);
                let inc = matches!(e.tag(), Some(t::PRE_INCREMENT | t::POST_INCREMENT));
                let size = self.size(target);
                if let Some(op) = self.operand(target) {
                    if size == 1 {
                        self.load(op.clone());
                        self.asm(if inc { "INC_A" } else { "DEC_A" });
                        self.store(op);
                    } else {
                        self.load(op.clone());
                        self.asm("LD_L_A");
                        let mut hi = op.clone();
                        hi.offset += 1;
                        self.load(hi.clone());
                        self.asm("LD_H_A");
                        let ty = self.env.type_of(target);
                        let step = if ty.is_pointer() {
                            self.env.size(target, sub_type(&ty).unwrap())
                        } else {
                            1
                        };
                        self.word("LD_DE_IMM", (if inc { step } else { -step }) & 65535);
                        self.asm("ADD_HL_DE");
                        self.asm("LD_A_L");
                        self.store(op);
                        self.asm("LD_A_H");
                        self.store(hi);
                    }
                } else {
                    self.increment_statement(target, inc);
                }
            }
            _ => self.into_a(e),
        }
    }
    fn assignment(&mut self, origin: &Expr, left: &Expr, right: &Expr) {
        if self.aggregate_assignment(origin, left, right) {
            return;
        }
        if left.is(t::FIELD) || left.is(t::INDEX) {
            self.assign_memory(left, right);
            return;
        }
        if let Some(op) = self.operand(left) {
            let size = self.size(left);
            if size == 1 {
                self.into_a(right);
                self.store(op);
            } else if size == 2 {
                self.into_hl(right);
                self.asm("LD_A_L");
                self.store(op.clone());
                self.asm("LD_A_H");
                let mut hi = op;
                hi.offset += 1;
                self.store(hi);
            } else {
                self.unsupported(left, "aggregate assignment");
            }
            return;
        }
        if left.is(t::LOAD) {
            let mut pointer = child(left, 1);
            if pointer.is(t::CAST) {
                pointer = child(pointer, 2);
            }
            if pointer.is(t::INTEGER) {
                let address = pointer.int(1).unwrap();
                if self.size(left) == 1 {
                    self.into_a(right);
                    self.store(memory_operand(address));
                } else {
                    self.into_hl(right);
                    self.asm("LD_A_L");
                    self.store(memory_operand(address));
                    self.asm("LD_A_H");
                    self.store(memory_operand(address + 1));
                }
                return;
            }
            if self.size(left) == 1 {
                self.into_a(right);
                self.asm("PUSH_AF");
                self.into_hl(pointer);
                self.asm("POP_AF");
                self.asm("LD_HL_A");
            } else {
                self.into_hl(right);
                self.asm("PUSH_HL");
                self.into_hl(pointer);
                self.asm("POP_DE");
                self.asm("LD_A_E");
                self.asm("LD_HL_A");
                self.asm("INC_HL");
                self.asm("LD_A_D");
                self.asm("LD_HL_A");
            }
            return;
        }
        self.unsupported(left, "complex assignment lvalue");
    }
}
