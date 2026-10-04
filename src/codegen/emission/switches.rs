use super::*;
impl Emitter {
    pub(super) fn switch(&mut self, e: &Expr) {
        let test = child(e, 1);
        let Arg::Exprs(cases) = &e.args[2] else {
            return;
        };
        let default = child(e, 3);
        if cases.is_empty() {
            self.into_a(test);
            if !default.is(t::EMPTY) {
                let end = self.unique("switch_end");
                self.breaks.push(end.clone());
                self.statement(default);
                self.breaks.pop();
                self.label(&end);
            }
            return;
        }
        let ty = self.env.type_of(test);
        let mut values = Vec::new();
        let mut bodies = BTreeMap::new();
        for case in cases {
            let value = child(case, 1);
            if let Kind::Enum(name) = &ty.kind {
                let ct = self.env.type_of(value);
                let message = match ct.kind {
                    Kind::Enum(other) if other != *name => Some(format!(
                        "case label enum '{other}' does not match {}switch enum '{name}'",
                        if ty.is_enum_strict { "strict " } else { "" }
                    )),
                    Kind::Enum(_) => None,
                    _ => Some(format!(
                        "case label is not an enum value for {}switch(enum {name})",
                        if ty.is_enum_strict { "strict " } else { "" }
                    )),
                };
                if let Some(message) = message {
                    if ty.is_enum_strict {
                        self.env.error(&value.source, message);
                    } else {
                        self.env.warning(&value.source, message);
                    }
                }
            }
            let n = self.env.constant(value).value;
            if !(0..=255).contains(&n) {
                self.env.error(
                    &case.source,
                    format!("Switch case value out of u8 range: {n}"),
                );
            }
            if bodies.insert(n, child(case, 2)).is_some() {
                self.env
                    .error(&case.source, format!("duplicate case value: {n}"));
            }
            values.push(n);
        }
        let min = *values.iter().min().unwrap();
        let max = *values.iter().max().unwrap();
        let range = max - min + 1;
        if !(1..=256).contains(&range) {
            self.env.error(
                &e.source,
                "Not Implemented: Switch range too large for jump table (max 256)",
            );
            return;
        }
        let end = self.unique("switch_end");
        let fallback = self.unique("switch_default");
        let after = self.unique("switch_after_table");
        let table = self.unique("switch_table");
        self.breaks.push(end.clone());
        self.into_a(test);
        if min > 0 {
            self.immediate("CP_IMM", min & 255);
            self.op("JP_C", fallback.clone());
            self.immediate("SUB_IMM", min & 255);
        }
        if range < 256 {
            self.immediate("CP_IMM", range & 255);
            self.op("JP_NC", fallback.clone());
        }
        self.asm("LD_L_A");
        self.immediate("LD_H_IMM", 0);
        self.asm("ADD_HL_HL");
        self.op(
            "LD_DE_IMM",
            Operand::symbol(table.base.as_deref().unwrap(), M::Immediate16),
        );
        for m in [
            "ADD_HL_DE",
            "LD_A_HL",
            "LD_E_A",
            "INC_HL",
            "LD_A_HL",
            "LD_D_A",
            "PUSH_DE",
            "RET",
        ] {
            self.asm(m);
        }
        let mut labels = vec![fallback.clone(); range as usize];
        let mut seen = BTreeSet::new();
        for n in &values {
            if seen.insert(*n) {
                labels[(n - min) as usize] = self.unique(&format!("case_{n}"));
            }
        }
        for n in values {
            self.label(&labels[(n - min) as usize]);
            self.statement(bodies[&n]);
        }
        self.label(&fallback);
        if !default.is(t::EMPTY) {
            self.statement(default);
        }
        self.label(&end);
        self.breaks.pop();
        self.op("JP", after.clone());
        self.label(&table);
        for label in labels {
            self.emit(t::WORD, vec![label.base.unwrap().into()]);
        }
        self.label(&after);
    }
}
