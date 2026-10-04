use super::*;
fn internal(name: &str) -> bool {
    name.is_empty() || name == "_" || name.starts_with("__")
}
fn warning(diagnostics: &mut Vec<Diagnostic>, source: &Position, message: String) {
    diagnostics.push(Diagnostic::new(
        Severity::Warning,
        source.clone(),
        message,
        0,
    ))
}
pub(super) fn unused_locals(ctx: &mut Context<'_>) {
    for name in ctx.local_order.clone() {
        if ctx.local_uses.get(&name) == Some(&0) && !internal(&name) {
            let source = ctx.local_positions.get(&name).cloned().unwrap_or_default();
            ctx.warning(
                &source,
                format!(
                    "unused local variable '{name}' in function '{}'",
                    ctx.function
                ),
            )
        }
    }
    for name in ctx.param_order.clone() {
        if ctx.param_uses.get(&name) == Some(&0) && !internal(&name) {
            let source = ctx.param_positions.get(&name).cloned().unwrap_or_default();
            ctx.warning(
                &source,
                format!("unused parameter '{name}' in function '{}'", ctx.function),
            )
        }
    }
}
fn collect_names(e: &Expr, names: &mut BTreeSet<String>) {
    if e.is(t::NAME) {
        if let Some(name) = e.text(1).filter(|s| !s.is_empty()) {
            names.insert(name.into());
        }
    }
    for arg in e.args.iter().skip(1) {
        match arg {
            Arg::Expr(e) => collect_names(e, names),
            Arg::Exprs(es) => {
                for e in es {
                    collect_names(e, names)
                }
            }
            _ => {}
        }
    }
}
pub(super) fn unused_globals(declarations: &[Expr], diagnostics: &mut Vec<Diagnostic>) {
    let (mut globals, mut functions, mut names) = (Vec::new(), Vec::new(), BTreeSet::new());
    let (mut seen_g, mut seen_f) = (BTreeSet::new(), BTreeSet::new());
    for decl in declarations {
        let decl = environment::unwrap(decl);
        match decl.tag() {
            Some(t::FUNCTION | t::INLINE_FUNCTION) => {
                let name = decl.text(2).unwrap();
                if seen_f.insert(name.to_owned()) {
                    functions.push((name.to_owned(), decl.source.clone()))
                }
                collect_names(
                    child(&decl, if decl.args.len() == 6 { 5 } else { 4 }),
                    &mut names,
                )
            }
            Some(t::VARIABLE) if decl.ty(2).is_some() => {
                let name = decl.text(3).unwrap();
                if seen_g.insert(name.to_owned()) {
                    globals.push((name.to_owned(), decl.source.clone()))
                }
                if let Some(range) = decl.child(4) {
                    collect_names(range, &mut names)
                }
            }
            Some(t::CONSTANT) => collect_names(child(&decl, 3), &mut names),
            _ => {}
        }
    }
    for (name, source) in globals {
        if !internal(&name) && !names.contains(&name) {
            warning(
                diagnostics,
                &source,
                format!("unused global variable '{name}'"),
            )
        }
    }
    for (name, source) in functions {
        if name != "main" && !internal(&name) && !names.contains(&name) {
            warning(diagnostics, &source, format!("unused function '{name}'"))
        }
    }
}
fn strict(t: &CType) -> bool {
    t.is_enum() && t.is_enum_strict
}
fn same_enum(a: &CType, b: &CType) -> bool {
    matches!((&a.kind,&b.kind),(Kind::Enum(a),Kind::Enum(b))if a==b)
}
fn plain(t: &CType) -> bool {
    t.is_integer() && !t.is_enum()
}
fn flags(t: &CType) -> bool {
    plain(t) && t.is_bitflags
}
fn safe(t: &CType) -> bool {
    plain(t) && t.is_safe_index
}
fn pointer_const(t: &CType) -> bool {
    matches!(&t.kind,Kind::Pointer(s)if s.is_const)
}
fn pointer_mut(t: &CType) -> bool {
    matches!(&t.kind,Kind::Pointer(s)if !s.is_const)
}
fn array_const(t: &CType) -> bool {
    t.is_array() && sub_type(t).is_some_and(|s| s.is_const)
}
fn bitop(op: &str) -> bool {
    matches!(
        op,
        t::BITWISE_AND | t::BITWISE_OR | t::BITWISE_XOR | t::SHIFT_LEFT | t::SHIFT_RIGHT
    )
}
fn restrict_base<'a>(mut e: &'a Expr, cast: &mut bool) -> Option<&'a str> {
    while e.is(t::CAST) {
        *cast = true;
        e = child(e, 2)
    }
    match e.tag() {
        Some(t::NAME) => e.text(1),
        Some(t::ADDRESS_OF) => {
            let mut sub = child(e, 1);
            while sub.is(t::CAST) {
                *cast = true;
                sub = child(sub, 2)
            }
            match sub.tag() {
                Some(t::NAME) => sub.text(1),
                Some(t::INDEX | t::FIELD) => restrict_base(child(sub, 1), cast),
                _ => None,
            }
        }
        Some(t::ADD | t::SUBTRACT) => {
            if child(e, 2).is(t::INTEGER) {
                restrict_base(child(e, 1), cast)
            } else if child(e, 1).is(t::INTEGER) {
                restrict_base(child(e, 2), cast)
            } else {
                None
            }
        }
        Some(t::INDEX | t::FIELD) => restrict_base(child(e, 1), cast),
        _ => None,
    }
}
impl Context<'_> {
    pub(super) fn assignment_lints(
        &mut self,
        left: &Expr,
        right: &Expr,
        lt: &CType,
        rt: &CType,
        source: &Position,
        op: Option<&str>,
    ) {
        let compound = op.is_some();
        if !compound
            && self.unsafe_depth == 0
            && pointer_mut(lt)
            && (pointer_const(rt) || array_const(rt))
            && !right.is(t::CAST)
        {
            self.warning(
                source,
                "assignment: discards 'const' qualifier from pointee type",
            )
        }
        if strict(lt) {
            if compound {
                self.warning(source,if same_enum(lt,rt){format!("compound assignment on __enum_strict {lt}; cast explicitly to silence")}else{format!("compound assignment on __enum_strict {lt} with operand type {rt}; cast explicitly to silence")})
            } else if !same_enum(lt, rt) {
                self.warning(source,format!("assigning value of type {rt} to __enum_strict {lt}; cast explicitly to silence"))
            }
        } else if plain(lt) && strict(rt) {
            self.warning(source,if compound{format!("compound assignment mixes __enum_strict {rt} into integer type {lt}; cast explicitly to silence")}else{format!("assigning __enum_strict {rt} to integer type {lt}; cast explicitly to silence")})
        }
        if flags(lt) {
            if let Some(op) = op {
                if !bitop(op) {
                    self.warning(source,format!("compound op '{op}' on __bitflags {lt} is discouraged; use bitwise ops or cast explicitly"))
                } else if !flags(rt) {
                    self.warning(source,format!("mixing __bitflags {lt} with operand type {rt} in '{op}'; cast explicitly to silence"))
                }
            } else if !flags(rt) {
                self.warning(source,format!("assigning value of type {rt} to __bitflags {lt}; cast explicitly to silence"))
            }
        } else if plain(lt) && flags(rt) {
            self.warning(source,if compound{format!("compound assignment mixes __bitflags {rt} into integer type {lt}; cast explicitly to silence")}else{format!("assigning __bitflags {rt} to integer type {lt}; cast explicitly to silence")})
        }
        if !compound {
            self.safe_assignment_lint(right, lt, rt, source, op)
        }
        if left.is(t::NAME) && right.is(t::INTEGER) {
            let name = left.text(1).unwrap();
            let value = right.int(1).unwrap();
            if let Some([min, max]) = self.get_range(name) {
                if value < min || value > max {
                    self.warning(
                        source,
                        format!(
                            "value {value} is outside declared range [{min},{max}] for '{name}'"
                        ),
                    )
                }
            }
        }
        if compound {
            self.safe_assignment_lint(right, lt, rt, source, op)
        }
    }
    fn safe_assignment_lint(
        &mut self,
        right: &Expr,
        lt: &CType,
        rt: &CType,
        source: &Position,
        op: Option<&str>,
    ) {
        if safe(lt) && !safe(rt) && !right.is(t::INTEGER) {
            self.warning(source,if let Some(op)=op{format!("compound op '{op}' on __safe_index {lt} with operand type {rt}; cast explicitly to silence")}else{format!("assigning value of type {rt} to __safe_index {lt}; cast explicitly to silence")})
        } else if plain(lt) && safe(rt) {
            self.warning(source,if op.is_some(){format!("compound assignment mixes __safe_index {rt} into integer type {lt}; cast explicitly to silence")}else{format!("assigning __safe_index {rt} to integer type {lt}; cast explicitly to silence")})
        }
    }
    pub(super) fn binary_lints(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        lt: &CType,
        rt: &CType,
        source: &Position,
    ) {
        if matches!(op, t::ADD | t::SUBTRACT) && (lt.is_pointer() || rt.is_pointer()) {
            let message = if lt.is_pointer() && rt.is_pointer() {
                Some(format!(
                    "pointer arithmetic between two pointers in '{op}' may be unsafe"
                ))
            } else if op == t::SUBTRACT && !lt.is_pointer() && rt.is_pointer() {
                Some("subtracting a pointer from an integer may be unsafe".into())
            } else {
                let (ptr, offset, ot) = if lt.is_pointer() {
                    (lt, right, rt)
                } else {
                    (rt, left, lt)
                };
                if matches!(
                    sub_type(ptr).map(|t| &t.kind),
                    Some(Kind::Simple(Simple::Void))
                ) {
                    Some("pointer arithmetic on void* may be unsafe".into())
                } else if ot.is_signed() && !offset.is(t::CAST) {
                    Some(
                        "signed offset used in pointer arithmetic; cast explicitly if intentional"
                            .into(),
                    )
                } else if offset.is(t::INTEGER) && offset.int(1).is_some_and(|n| n < 0) {
                    Some("negative constant offset in pointer arithmetic may be unsafe".into())
                } else {
                    None
                }
            };
            if let Some(m) = message {
                self.warning(source, m)
            }
        }
        if (strict(lt) || strict(rt)) && !same_enum(lt, rt) {
            self.warning(source,format!("mixing __enum_strict enums with other types in '{op}': {lt} vs {rt}; cast explicitly to silence"))
        }
        if flags(lt) || flags(rt) {
            if !bitop(op) && !matches!(op, t::EQUAL | t::NOT_EQUAL) {
                self.warning(source,format!("operation '{op}' on __bitflags types is discouraged; use bitwise ops or cast explicitly"))
            } else if !(flags(lt) && flags(rt)) {
                self.warning(source,format!("mixing __bitflags with other types in '{op}': {lt} vs {rt}; cast explicitly to silence"))
            }
        }
        if safe(lt) || safe(rt) {
            let allowed = (comparison(op) || matches!(op, t::ADD | t::SUBTRACT))
                && ((safe(lt) && (safe(rt) || right.is(t::INTEGER)))
                    || (safe(rt) && (safe(lt) || left.is(t::INTEGER))));
            if !allowed {
                self.warning(source,format!("mixing __safe_index with other types in '{op}': {lt} vs {rt}; cast explicitly to silence"))
            }
        }
    }
    pub(super) fn call_lints(
        &mut self,
        origin: &Expr,
        function: &Expr,
        args: &[Expr],
        params: Option<&[CType]>,
        name: Option<&str>,
    ) {
        let fname = name.unwrap_or("<call>");
        let source = &origin.source;
        if let Some(params) = params {
            if self.unsafe_depth == 0 {
                for (arg, pt) in args.iter().zip(params) {
                    let ty = self.infer(arg);
                    if pointer_mut(pt)
                        && !arg.is(t::CAST)
                        && (pointer_const(&ty) || array_const(&ty))
                    {
                        let i = args.iter().position(|e| std::ptr::eq(e, arg)).unwrap();
                        self.warning(source,format!("argument {i} to '{fname}': discards 'const' qualifier from pointee type"));
                        break;
                    }
                }
            }
            for (i, (arg, pt)) in args.iter().zip(params).enumerate() {
                let ty = self.infer(arg);
                let position =
                    if arg.source.filename.is_empty() || arg.source.filename == "<unknown>" {
                        source
                    } else {
                        &arg.source
                    };
                self.narrowing(
                    arg,
                    &ty,
                    pt,
                    position,
                    &format!("argument {i} to '{fname}'"),
                )
            }
            for (i, (arg, pt)) in args.iter().zip(params).enumerate() {
                if arg.is(t::CAST) {
                    continue;
                }
                let ty = self.infer(arg);
                if strict(pt) && !same_enum(pt, &ty) {
                    self.warning(source,format!("argument {i} to '{fname}' expects __enum_strict {pt}, got {ty}; cast explicitly to silence"))
                } else if !strict(pt) && plain(pt) && strict(&ty) {
                    self.warning(source,format!("argument {i} to '{fname}' passes __enum_strict {ty} to integer parameter {pt}; cast explicitly to silence"))
                }
            }
        }
        if self.unsafe_depth == 0 && function.is(t::NAME) {
            let name = function.text(1).unwrap();
            if let Some(params) = self.global.function_params.get(name) {
                let indices = params
                    .iter()
                    .zip(args)
                    .enumerate()
                    .filter_map(|(i, (t, _))| {
                        if t.is_pointer() && t.is_restrict {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                for (a, &i) in indices.iter().enumerate() {
                    for &j in &indices[a + 1..] {
                        let (mut ci, mut cj) = (false, false);
                        let bi = restrict_base(&args[i], &mut ci);
                        let bj = restrict_base(&args[j], &mut cj);
                        if !ci && !cj && bi.is_some() && bi == bj {
                            self.warning(source,format!("__restrict may be violated in call to {name}: arg{i} and arg{j} share base '{}'",bi.unwrap()))
                        }
                    }
                }
            }
        }
    }
}
