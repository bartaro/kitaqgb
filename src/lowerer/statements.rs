use super::*;
impl Context<'_> {
    pub(super) fn statement(&mut self, stmt: &Expr) -> Vec<Expr> {
        let src = &stmt.source;
        match stmt.tag().unwrap_or("") {
            t::SEQUENCE => {
                let mut flat = Vec::new();
                let mut terminated = false;
                for item in stmt.children() {
                    if terminated && !item.is(t::LABEL) {
                        self.warning(&item.source, "unreachable code")
                    }
                    if item.is(t::LABEL) {
                        terminated = false
                    }
                    self.begin();
                    let items = self.statement(item);
                    self.end();
                    if items.last().is_some_and(ends_flow) {
                        terminated = true
                    }
                    flat.extend(items);
                }
                vec![sequence(flat).with_source(src.clone())]
            }
            t::STATIC_ASSERT => {
                let condition = self.expression(child(stmt, 1), false, &mut Vec::new());
                vec![node(
                    t::STATIC_ASSERT,
                    vec![Arg::from(condition), Arg::from(stmt.text(2).unwrap_or(""))],
                    src,
                )]
            }
            t::UNSAFE => {
                self.unsafe_depth += 1;
                let inner = child(stmt, 1);
                let mut lowered = self.statement(inner);
                self.unsafe_depth -= 1;
                let body = if lowered.len() == 1 {
                    lowered.remove(0)
                } else {
                    sequence(lowered).with_source(inner.source.clone())
                };
                vec![node(t::UNSAFE, vec![Arg::from(body)], src)]
            }
            t::VARIABLE if stmt.ty(1).is_some() => {
                let ty = stmt.ty(1).unwrap();
                let name = stmt.text(2).unwrap();
                let range = stmt.child(3).map(|e| self.global.range(e));
                self.register_local(name, ty, range, Some(src));
                vec![stmt.clone()]
            }
            t::RETURN => {
                if stmt.child(1).is_none() {
                    return vec![stmt.clone()];
                }
                let mut result = Vec::new();
                let value = self.expression(child(stmt, 1), true, &mut result);
                let ty = self.infer(&value);
                let ret = self.return_type.clone();
                self.narrowing(
                    &value,
                    &ty,
                    &ret,
                    src,
                    &format!("return in '{}'", self.function),
                );
                result.push(node(t::RETURN, vec![Arg::from(value)], src));
                result
            }
            t::FOR => {
                let mut init = self.single(child(stmt, 1));
                let mut induct = self.single(child(stmt, 3));
                self.begin();
                let body = self.statement(child(stmt, 4));
                self.end();
                let mut body = combine(body, None);
                let mut prefix = Vec::new();
                let test = if child(stmt, 2).is(t::EMPTY) {
                    child(stmt, 2).clone()
                } else {
                    self.expression(child(stmt, 2), true, &mut prefix)
                };
                if !prefix.is_empty() {
                    let prefix = sequence(prefix).with_source(src.clone());
                    init = concat(init, prefix.clone(), src);
                    induct = concat(induct, prefix, src)
                }
                if !induct.is(t::EMPTY) {
                    body = rewrite_continue(&body, &induct, src)
                }
                if test.is(t::INTEGER) && test.int(1) == Some(0) {
                    return if init.is(t::EMPTY) {
                        Vec::new()
                    } else {
                        vec![init.with_source(src.clone())]
                    };
                }
                vec![node(
                    t::FOR,
                    vec![
                        Arg::from(init),
                        Arg::from(test),
                        Arg::from(induct),
                        Arg::from(body),
                    ],
                    src,
                )]
            }
            t::DO_WHILE => {
                self.begin();
                let body = self.statement(child(stmt, 1));
                self.end();
                let mut body = if body.is_empty() {
                    Expr::new(t::EMPTY, Vec::new())
                } else {
                    combine(body, None)
                };
                let mut prefix = Vec::new();
                let test = self.expression(child(stmt, 2), true, &mut prefix);
                if !prefix.is_empty() {
                    let prefix = sequence(prefix).with_source(src.clone());
                    body = rewrite_continue(&body, &prefix, src);
                    body = concat(body, prefix, src)
                }
                vec![node(
                    t::DO_WHILE,
                    vec![Arg::from(body), Arg::from(test)],
                    src,
                )]
            }
            t::IF => vec![self.if_arm(&children(stmt), 0, src)],
            t::SWITCH => {
                let mut prefix = Vec::new();
                let test = self.expression(child(stmt, 1), true, &mut prefix);
                let original_type = self.infer(child(stmt, 1));
                let cases = match stmt.args.get(2) {
                    Some(Arg::Exprs(cases)) => cases,
                    _ => return vec![stmt.clone()],
                };
                let mut lowered = Vec::new();
                for case in cases {
                    if !case.is(t::CASE) {
                        lowered.push(case.clone());
                        continue;
                    }
                    let value = child(case, 1);
                    if let Kind::Enum(name) = &original_type.kind {
                        let ty = self.infer(value);
                        let message = if let Kind::Enum(other) = &ty.kind {
                            if name == other {
                                None
                            } else {
                                Some(format!(
                                    "case label enum '{other}' does not match {}switch enum '{name}'",
                                    if original_type.is_enum_strict {
                                        "strict "
                                    } else {
                                        ""
                                    }
                                ))
                            }
                        } else {
                            Some(format!(
                                "case label is not an enum value for {}switch(enum {name})",
                                if original_type.is_enum_strict {
                                    "strict "
                                } else {
                                    ""
                                }
                            ))
                        };
                        if let Some(message) = message {
                            if original_type.is_enum_strict {
                                self.error(&value.source, message)
                            } else {
                                self.warning(&value.source, message)
                            }
                        }
                    }
                    self.begin();
                    let body = self.statement(child(case, 2));
                    self.end();
                    lowered.push(node(
                        t::CASE,
                        vec![Arg::from(value.clone()), Arg::from(combine(body, None))],
                        &case.source,
                    ));
                }
                self.begin();
                let default = self.statement(child(stmt, 3));
                self.end();
                prefix.push(node(
                    t::SWITCH,
                    vec![
                        Arg::from(test),
                        Arg::Exprs(lowered),
                        Arg::from(combine(default, None)),
                    ],
                    src,
                ));
                prefix
            }
            t::LABEL | t::JUMP | t::CONTINUE | t::BREAK | t::FALLTHROUGH | t::ASM => {
                vec![stmt.clone()]
            }
            _ => {
                let mut prefix = Vec::new();
                let value = self.expression(stmt, true, &mut prefix);
                prefix.push(value);
                prefix
            }
        }
    }
    fn single(&mut self, stmt: &Expr) -> Expr {
        if stmt.is(t::EMPTY) {
            return stmt.clone();
        }
        self.begin();
        let items = self.statement(stmt);
        self.end();
        combine(items, Some(&stmt.source))
    }
    fn if_arm(&mut self, parts: &[Expr], index: usize, src: &Position) -> Expr {
        let mut prefix = Vec::new();
        let cond = self.expression(&parts[index], true, &mut prefix);
        let yes = self.refine(&parts[index], true);
        let no = self.refine(&parts[index], false);
        if !yes.is_empty() {
            self.overrides.push(yes.clone())
        }
        let body = self.single(&parts[index + 1]);
        if !yes.is_empty() {
            self.overrides.pop();
        }
        let known = if cond.is(t::INTEGER) {
            cond.int(1)
        } else if prefix.is_empty() {
            self.condition_from_ranges(&cond)
        } else {
            None
        };
        if known.is_some_and(|n| n != 0) {
            if prefix.is_empty() {
                return body;
            }
            prefix.push(body);
            return sequence(prefix).with_source(src.clone());
        }
        let mut tail = None;
        if index + 2 < parts.len() {
            if !no.is_empty() {
                self.overrides.push(no.clone())
            }
            tail = Some(self.if_arm(parts, index + 2, src));
            if !no.is_empty() {
                self.overrides.pop();
            }
        }
        if known == Some(0) {
            let taken = tail.unwrap_or_else(|| node(t::EMPTY, Vec::new(), src));
            if prefix.is_empty() {
                return taken;
            }
            prefix.push(taken);
            return sequence(prefix).with_source(src.clone());
        }
        let mut args = vec![Arg::from(cond), Arg::from(body)];
        if let Some(tail) = tail {
            args.push(Arg::from(Expr::new(t::INTEGER, vec![Arg::Int(1)])));
            args.push(Arg::from(tail))
        }
        let result = node(t::IF, args, src);
        if prefix.is_empty() {
            result
        } else {
            prefix.push(result);
            sequence(prefix).with_source(src.clone())
        }
    }
}
fn combine(mut items: Vec<Expr>, source: Option<&Position>) -> Expr {
    if items.len() == 1 {
        items.remove(0)
    } else {
        let e = sequence(items);
        source.map_or_else(|| e.clone(), |s| e.with_source(s.clone()))
    }
}
fn concat(first: Expr, second: Expr, source: &Position) -> Expr {
    if first.is(t::EMPTY) {
        return second;
    }
    if second.is(t::EMPTY) {
        return first;
    }
    let mut items = if first.is(t::SEQUENCE) {
        children(&first)
    } else {
        vec![first]
    };
    if second.is(t::SEQUENCE) {
        items.extend(children(&second))
    } else {
        items.push(second)
    }
    sequence(items).with_source(source.clone())
}
fn rewrite_continue(stmt: &Expr, induct: &Expr, source: &Position) -> Expr {
    match stmt.tag() {
        Some(t::UNSAFE) => node(
            t::UNSAFE,
            vec![Arg::from(rewrite_continue(child(stmt, 1), induct, source))],
            &stmt.source,
        ),
        Some(t::CONTINUE) => {
            concat(induct.clone(), stmt.clone(), source).with_source(stmt.source.clone())
        }
        Some(t::SEQUENCE) => sequence(
            stmt.children()
                .into_iter()
                .map(|s| rewrite_continue(s, induct, source))
                .collect(),
        )
        .with_source(stmt.source.clone()),
        Some(t::IF) => {
            let mut args = Vec::new();
            let parts = stmt.children();
            for pair in parts.chunks_exact(2) {
                args.push(Arg::from(pair[0].clone()));
                args.push(Arg::from(rewrite_continue(pair[1], induct, source)))
            }
            node(t::IF, args, &stmt.source)
        }
        Some(t::SWITCH) => {
            let cases = match &stmt.args[2] {
                Arg::Exprs(c) => c,
                _ => return stmt.clone(),
            };
            let cases = cases
                .iter()
                .map(|c| {
                    if c.is(t::CASE) {
                        node(
                            t::CASE,
                            vec![
                                Arg::from(child(c, 1).clone()),
                                Arg::from(rewrite_continue(child(c, 2), induct, source)),
                            ],
                            &c.source,
                        )
                    } else {
                        c.clone()
                    }
                })
                .collect();
            node(
                t::SWITCH,
                vec![
                    Arg::from(child(stmt, 1).clone()),
                    Arg::Exprs(cases),
                    Arg::from(rewrite_continue(child(stmt, 3), induct, source)),
                ],
                &stmt.source,
            )
        }
        _ => stmt.clone(),
    }
}
pub(super) fn ends_flow(stmt: &Expr) -> bool {
    match stmt.tag() {
        Some(t::RETURN | t::JUMP | t::BREAK | t::CONTINUE | t::FALLTHROUGH) => true,
        Some(t::UNSAFE) => ends_flow(child(stmt, 1)),
        Some(t::SEQUENCE) => stmt.children().last().is_some_and(|s| ends_flow(s)),
        Some(t::IF) => {
            let parts = stmt.children();
            parts.len() >= 4
                && parts[parts.len() - 2].is(t::INTEGER)
                && parts[parts.len() - 2].int(1).is_some_and(|n| n != 0)
                && parts.chunks_exact(2).all(|p| ends_flow(p[1]))
        }
        _ => false,
    }
}
