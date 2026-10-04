use super::*;
fn named(e: &Expr, name: &str) -> bool {
    e.is(t::NAME) && e.text(1) == Some(name)
}
fn literal(e: &Expr, value: i32) -> bool {
    e.is(t::INTEGER) && e.int(1) == Some(value)
}
fn nonzero(e: &Expr, name: &str) -> bool {
    if named(e, name) {
        return true;
    }
    if !matches!(
        e.tag(),
        Some(t::NOT_EQUAL | t::GREATER_THAN | t::GREATER_THAN_OR_EQUAL)
    ) {
        return false;
    }
    let a = child(e, 1);
    let b = child(e, 2);
    match e.tag().unwrap() {
        t::NOT_EQUAL => (named(a, name) && literal(b, 0)) || (named(b, name) && literal(a, 0)),
        t::GREATER_THAN => named(a, name) && literal(b, 0),
        _ => named(a, name) && literal(b, 1),
    }
}
fn writes(e: &Expr, name: &str) -> bool {
    if matches!(
        e.tag(),
        Some(
            t::ASSIGN
                | t::PRE_INCREMENT
                | t::POST_INCREMENT
                | t::PRE_DECREMENT
                | t::POST_DECREMENT
                | t::ADDRESS_OF
        )
    ) && named(child(e, 1), name)
    {
        return true;
    }
    e.children().iter().any(|e| writes(e, name))
}
impl Emitter {
    pub(super) fn countdown(&mut self, e: &Expr) -> bool {
        let init = child(e, 1);
        let test = child(e, 2);
        let induct = child(e, 3);
        let body = child(e, 4);
        if !matches!(induct.tag(), Some(t::PRE_DECREMENT | t::POST_DECREMENT)) {
            return false;
        }
        let variable = child(induct, 1);
        if !variable.is(t::NAME) {
            return false;
        }
        let name = variable.text(1).unwrap();
        if !nonzero(test, name) || writes(body, name) {
            return false;
        }
        let top = self.unique("loop_top");
        let cont = self.unique("loop_cont");
        let end = self.unique("loop_end");
        self.loops.push((cont.clone(), end.clone()));
        self.env.begin_scope();
        self.statement(init);
        let name_expr = Expr::new(t::NAME, vec![name.into()]);
        let op = if self.size(&name_expr) == 1 {
            self.operand(&name_expr)
        } else {
            None
        };
        let Some(op) = op else {
            self.env.end_scope();
            self.loops.pop();
            return false;
        };
        self.load(op.clone());
        self.asm("OR_A");
        self.op("JP_Z", end.clone());
        self.label(&top);
        self.breaks.push(end.clone());
        self.statement(body);
        self.breaks.pop();
        self.label(&cont);
        self.load(op.clone());
        self.asm("DEC_A");
        self.store(op);
        self.op("JP_NZ", top);
        self.label(&end);
        self.env.end_scope();
        self.loops.pop();
        true
    }
}
