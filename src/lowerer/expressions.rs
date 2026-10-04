use super::*;
impl Context<'_> {
    pub(super) fn spill(&mut self, expr: Expr, ty: &CType, prefix: &mut Vec<Expr>) -> Expr {
        let name = self.temp(ty);
        let result = node(t::NAME, vec![Arg::from(name)], &expr.source);
        prefix.push(node(
            t::ASSIGN,
            vec![Arg::from(result.clone()), Arg::from(expr.clone())],
            &expr.source,
        ));
        result
    }
    fn pointer_temp(&self, expr: &Expr) -> CType {
        let ty = self.infer(expr);
        if ty.is_pointer() {
            ty
        } else if ty.is_array() {
            CType::pointer(sub_type(&ty).unwrap().clone())
        } else {
            CType::simple(Simple::UInt16)
        }
    }
    pub(super) fn expression(
        &mut self,
        expr: &Expr,
        extract: bool,
        prefix: &mut Vec<Expr>,
    ) -> Expr {
        let source = &expr.source;
        let tag = expr.tag().unwrap_or("");
        if tag == t::INTEGER {
            return expr.clone();
        }
        if tag == t::NAME {
            self.use_name(expr.text(1).unwrap());
            return expr.clone();
        }
        if tag == t::CAST {
            let sub = self.expression(child(expr, 2), extract, prefix);
            return node(
                tag,
                vec![Arg::from(expr.ty(1).unwrap().clone()), Arg::from(sub)],
                source,
            );
        }
        if tag == t::CONDITIONAL {
            let cond = self.expression(child(expr, 1), extract, prefix);
            let (mut yes_prefix, mut no_prefix) = (Vec::new(), Vec::new());
            let yes = self.expression(child(expr, 2), extract, &mut yes_prefix);
            let no = self.expression(child(expr, 3), extract, &mut no_prefix);
            if yes_prefix.is_empty() && no_prefix.is_empty() {
                return node(
                    tag,
                    vec![Arg::from(cond), Arg::from(yes), Arg::from(no)],
                    source,
                );
            }
            let result_type = self.infer(expr);
            let result = node(t::NAME, vec![Arg::from(self.temp(&result_type))], source);
            yes_prefix.push(node(
                t::ASSIGN,
                vec![Arg::from(result.clone()), Arg::from(yes)],
                source,
            ));
            no_prefix.push(node(
                t::ASSIGN,
                vec![Arg::from(result.clone()), Arg::from(no)],
                source,
            ));
            prefix.push(node(
                t::IF,
                vec![
                    Arg::from(cond),
                    Arg::from(sequence(yes_prefix)),
                    Arg::from(Expr::new(t::INTEGER, vec![Arg::Int(1)])),
                    Arg::from(sequence(no_prefix)),
                ],
                source,
            ));
            return result;
        }
        if extract && matches!(tag, t::LOGICAL_AND | t::LOGICAL_OR) {
            let left = self.expression(child(expr, 1), true, prefix);
            let mut right_prefix = Vec::new();
            let right = self.expression(child(expr, 2), true, &mut right_prefix);
            if right_prefix.is_empty() {
                return node(tag, vec![Arg::from(left), Arg::from(right)], source);
            }
            let result = node(
                t::NAME,
                vec![Arg::from(self.temp(&CType::simple(Simple::UInt8)))],
                source,
            );
            let boolean = |value: Expr| {
                Expr::new(
                    t::LOGICAL_NOT,
                    vec![Arg::from(Expr::new(t::LOGICAL_NOT, vec![Arg::from(value)]))],
                )
            };
            prefix.push(node(
                t::ASSIGN,
                vec![Arg::from(result.clone()), Arg::from(boolean(left))],
                source,
            ));
            right_prefix.push(node(
                t::ASSIGN,
                vec![Arg::from(result.clone()), Arg::from(boolean(right))],
                source,
            ));
            let guard = if tag == t::LOGICAL_AND {
                result.clone()
            } else {
                Expr::new(t::LOGICAL_NOT, vec![Arg::from(result.clone())])
            };
            prefix.push(node(
                t::IF,
                vec![Arg::from(guard), Arg::from(sequence(right_prefix))],
                source,
            ));
            return result;
        }
        if tag == t::CALL {
            return self.call(expr, extract, prefix);
        }
        if tag == t::INDEX {
            let left = self.expression(child(expr, 1), extract, prefix);
            let mut right = self.expression(child(expr, 2), extract, prefix);
            if extract && !atom(&right) {
                let ty = self.infer(&right);
                if scalar_size(&ty) <= 2 {
                    right = self.spill(right, &ty, prefix)
                }
            }
            right = self.index_range(expr, &left, right);
            return node(tag, vec![Arg::from(left), Arg::from(right)], source);
        }
        if tag == t::LOAD {
            let mut pointer = self.expression(child(expr, 1), extract, prefix);
            if extract && !atom(&pointer) {
                let ty = self.pointer_temp(&pointer);
                pointer = self.spill(pointer, &ty, prefix)
            }
            return node(tag, vec![Arg::from(pointer)], source);
        }
        if tag == t::FIELD {
            let base = self.expression(child(expr, 1), extract, prefix);
            let base = self.atomize_field_base(base, extract, prefix, false);
            return node(
                tag,
                vec![Arg::from(base), Arg::from(expr.text(2).unwrap())],
                source,
            );
        }
        if tag == t::ASSIGN {
            return self.assignment(expr, extract, prefix);
        }
        if tag == t::ASSIGN_MODIFY {
            let op = expr.text(1).unwrap();
            let left = self.lvalue(child(expr, 2), extract, prefix);
            let right = self.expression(child(expr, 3), extract, prefix);
            let lt = self.infer(&left);
            let rt = self.infer(&right);
            if lt.is_const {
                self.error(source, "cannot modify const-qualified object");
            }
            if lt.is_aggregate() || rt.is_aggregate() {
                self.error(
                    source,
                    "compound assignment is not supported for struct/union types",
                );
                return node(t::EMPTY, Vec::new(), source);
            }
            let widened = node(
                op,
                vec![Arg::from(left.clone()), Arg::from(right.clone())],
                source,
            );
            let widened_ty = self.infer(&widened);
            self.narrowing(&right, &widened_ty, &lt, source, "compound assignment");
            self.assignment_lints(&left, &right, &lt, &rt, source, Some(op));
            return node(
                tag,
                vec![Arg::from(op), Arg::from(left), Arg::from(right)],
                source,
            );
        }
        if matches!(
            tag,
            t::PRE_INCREMENT | t::POST_INCREMENT | t::PRE_DECREMENT | t::POST_DECREMENT
        ) {
            let sub = self.lvalue(child(expr, 1), extract, prefix);
            let ty = self.infer(&sub);
            if ty.is_const {
                self.error(source, "cannot modify const-qualified object");
            }
            if ty.is_enum() && ty.is_enum_strict {
                self.warning(
                    source,
                    format!(
                        "increment/decrement on __enum_strict {ty}; cast explicitly to silence"
                    ),
                );
            }
            if ty.is_integer() && !ty.is_enum() && ty.is_bitflags {
                self.warning(source,format!("increment/decrement on __bitflags {ty} is discouraged; cast explicitly to silence"));
            }
            return node(tag, vec![Arg::from(sub)], source);
        }
        if tag == t::ADDRESS_OF {
            let sub = self.lvalue(child(expr, 1), extract, prefix);
            return node(tag, vec![Arg::from(sub)], source);
        }
        if matches!(tag, t::SHIFT_LEFT | t::SHIFT_RIGHT) {
            let left = self.expression(child(expr, 1), extract, prefix);
            let right = self.expression(child(expr, 2), extract, prefix);
            if right.is(t::INTEGER) || !extract {
                return node(tag, vec![Arg::from(left), Arg::from(right)], source);
            }
            let ty = self.infer(&left);
            let size = scalar_size(&ty);
            let ty = CType::simple(match (size == 1, ty.is_signed()) {
                (true, true) => Simple::Int8,
                (true, false) => Simple::UInt8,
                (false, true) => Simple::Int16,
                (false, false) => Simple::UInt16,
            });
            let value = node(t::NAME, vec![Arg::from(self.temp(&ty))], source);
            prefix.push(node(
                t::ASSIGN,
                vec![Arg::from(value.clone()), Arg::from(left)],
                source,
            ));
            let count = node(
                t::NAME,
                vec![Arg::from(self.temp(&CType::simple(Simple::UInt8)))],
                source,
            );
            prefix.push(node(
                t::ASSIGN,
                vec![Arg::from(count.clone()), Arg::from(right)],
                source,
            ));
            let one = node(t::INTEGER, vec![Arg::Int(1)], source);
            let shift = Expr::new(tag, vec![Arg::from(value.clone()), Arg::from(one.clone())]);
            let subtract = Expr::new(t::SUBTRACT, vec![Arg::from(count.clone()), Arg::from(one)]);
            let body = sequence(vec![
                node(
                    t::ASSIGN,
                    vec![Arg::from(value.clone()), Arg::from(shift)],
                    source,
                ),
                node(
                    t::ASSIGN,
                    vec![Arg::from(count.clone()), Arg::from(subtract)],
                    source,
                ),
            ])
            .with_source(source.clone());
            prefix.push(node(
                t::FOR,
                vec![
                    Arg::from(Expr::new(t::EMPTY, Vec::new())),
                    Arg::from(count),
                    Arg::from(Expr::new(t::EMPTY, Vec::new())),
                    Arg::from(body),
                ],
                source,
            ));
            return value;
        }
        if matches!(tag, t::BITWISE_NOT | t::LOGICAL_NOT) {
            let sub = self.expression(child(expr, 1), extract, prefix);
            return node(tag, vec![Arg::from(sub)], source);
        }
        if binary(tag) {
            let mut left = self.expression(child(expr, 1), extract, prefix);
            let mut right = self.expression(child(expr, 2), extract, prefix);
            if tag == t::MULTIPLY {
                if left.is(t::INTEGER) && !right.is(t::INTEGER) {
                    std::mem::swap(&mut left, &mut right)
                }
                if let Some(n) = right.int(1).filter(|_| right.is(t::INTEGER)) {
                    if n == 1 {
                        return left.with_source(source.clone());
                    }
                    if n > 0 && n < 65536 && n & (n - 1) == 0 {
                        return node(
                            t::SHIFT_LEFT,
                            vec![
                                Arg::from(left),
                                Arg::from(node(
                                    t::INTEGER,
                                    vec![Arg::Int(n.trailing_zeros() as i32)],
                                    source,
                                )),
                            ],
                            source,
                        );
                    }
                }
            }
            if matches!(tag, t::DIVIDE | t::MODULUS) {
                if let Some(n) = right.int(1).filter(|_| right.is(t::INTEGER)) {
                    if tag == t::DIVIDE && n == 1 {
                        return left.with_source(source.clone());
                    }
                    if n > 0 && n < 65536 && n & (n - 1) == 0 && self.non_negative(&left) {
                        let reduced = if tag == t::DIVIDE {
                            t::SHIFT_RIGHT
                        } else {
                            t::BITWISE_AND
                        };
                        let operand = if tag == t::DIVIDE {
                            n.trailing_zeros() as i32
                        } else {
                            n - 1
                        };
                        return node(
                            reduced,
                            vec![
                                Arg::from(left),
                                Arg::from(node(t::INTEGER, vec![Arg::Int(operand)], source)),
                            ],
                            source,
                        );
                    }
                }
            }
            if extract && comparison(tag) {
                left = self.spill_comparison(left, prefix);
                right = self.spill_comparison(right, prefix)
            }
            let lt = self.infer(&left);
            let rt = self.infer(&right);
            self.binary_lints(tag, &left, &right, &lt, &rt, source);
            if comparison(tag) && (scalar_size(&lt) == 2 || scalar_size(&rt) == 2) {
                if scalar_size(&lt) == 1 {
                    left = node(
                        t::CAST,
                        vec![Arg::from(CType::simple(Simple::UInt16)), Arg::from(left)],
                        source,
                    )
                }
                if scalar_size(&rt) == 1 {
                    right = node(
                        t::CAST,
                        vec![Arg::from(CType::simple(Simple::UInt16)), Arg::from(right)],
                        source,
                    )
                }
            }
            return node(tag, vec![Arg::from(left), Arg::from(right)], source);
        }
        expr.clone()
    }
    fn spill_comparison(&mut self, expr: Expr, prefix: &mut Vec<Expr>) -> Expr {
        if atom(&expr) || self.global.try_eval(&expr).is_some() {
            return expr;
        }
        let ty = self.infer(&expr);
        if !ty.is_aggregate() && scalar_size(&ty) <= 2 {
            self.spill(expr, &ty, prefix)
        } else {
            expr
        }
    }
    fn call(&mut self, expr: &Expr, extract: bool, prefix: &mut Vec<Expr>) -> Expr {
        let source = &expr.source;
        let original_function = child(expr, 1);
        let original_args = expr.children().into_iter().skip(1).collect::<Vec<_>>();
        if original_function.is(t::NAME)
            && original_function.text(1) == Some("__cgb_is_cgb")
            && original_args.is_empty()
        {
            if let Some(n) = self.options.known_cgb {
                return node(t::INTEGER, vec![Arg::Int(n)], source);
            }
        }
        let function = self.expression(original_function, false, prefix);
        let name = function
            .text(1)
            .filter(|_| function.is(t::NAME))
            .map(str::to_owned);
        let param_types = if let Some(name) = &name {
            self.global
                .function_params
                .get(name)
                .cloned()
                .or_else(|| callable(&self.name_type(name)).map(|(_, p)| p.to_vec()))
        } else {
            callable(&self.infer(&function)).map(|(_, p)| p.to_vec())
        };
        let mut args = Vec::new();
        for (index, arg) in original_args.into_iter().enumerate() {
            let mut arg = self.expression(arg, extract, prefix);
            let intrinsic = arg.is(t::CALL)
                && child(&arg, 1).is(t::NAME)
                && matches!(child(&arg, 1).text(1), Some("__bankof" | "__cgb_is_cgb"));
            let constant = self.global.try_eval(&arg).is_some();
            let far_bank = name.as_deref() == Some("__farcall") && index == 0;
            if extract && !far_bank && !intrinsic && !constant && !atom(&arg) {
                let ty = if param_types
                    .as_ref()
                    .and_then(|p| p.get(index))
                    .is_some_and(|t| matches!(t.kind, Kind::Simple(Simple::UInt8 | Simple::Int8)))
                {
                    CType::simple(Simple::UInt8)
                } else {
                    self.infer(&arg)
                };
                if !ty.is_aggregate() && scalar_size(&ty) <= 2 {
                    arg = self.spill(arg, &ty, prefix)
                }
            }
            args.push(arg);
        }
        self.call_lints(
            expr,
            &function,
            &args,
            param_types.as_deref(),
            name.as_deref(),
        );
        if let Some(name) = &name {
            if name == "__slice"
                || (name == "slice" && !self.global.function_params.contains_key("slice"))
            {
                if args.len() != 2 {
                    self.error(
                        source,
                        format!("{name}: expected 2 arguments: __slice(ptr,len)"),
                    );
                    return expr.clone();
                }
                let pointer = self.infer(&args[0]);
                let length = self.infer(&args[1]);
                if !pointer.is_pointer() && !pointer.is_array() {
                    self.error(
                        source,
                        format!(
                            "{name}: first argument must be a pointer or array, got: {pointer}"
                        ),
                    );
                }
                if !length.is_integer() {
                    self.error(
                        source,
                        format!("{name}: second argument must be an integer length, got: {length}"),
                    );
                }
                return node(t::SLICE, args.into_iter().map(Arg::from).collect(), source);
            }
        }
        let mut values = vec![Arg::from(function)];
        values.extend(args.into_iter().map(Arg::from));
        node(t::CALL, values, source)
    }
    fn atomize_field_base(
        &mut self,
        mut base: Expr,
        extract: bool,
        prefix: &mut Vec<Expr>,
        lvalue: bool,
    ) -> Expr {
        if !extract {
            return base;
        }
        // The source lvalue path checks load before index; the value path checks index before load.
        let _ = lvalue;
        if base.is(t::INDEX) {
            let array = child(&base, 1).clone();
            let mut index = self.expression(child(&base, 2), extract, prefix);
            if !atom(&index) {
                let ty = self.infer(&index);
                if scalar_size(&ty) <= 2 {
                    index = self.spill(index, &ty, prefix)
                }
            }
            base = node(
                t::INDEX,
                vec![Arg::from(array), Arg::from(index)],
                &base.source,
            );
        } else if base.is(t::LOAD) {
            let mut pointer = self.expression(child(&base, 1), extract, prefix);
            if !atom(&pointer) {
                let ty = self.pointer_temp(&pointer);
                pointer = self.spill(pointer, &ty, prefix)
            }
            base = node(t::LOAD, vec![Arg::from(pointer)], &base.source)
        }
        base
    }
    pub(super) fn lvalue(&mut self, expr: &Expr, extract: bool, prefix: &mut Vec<Expr>) -> Expr {
        let source = &expr.source;
        match expr.tag() {
            Some(t::NAME) => {
                let name = expr.text(1).unwrap();
                if let Some(ty) = self.global.constant_types.get(name) {
                    if !(self.options.const_scalar_in_rom
                        && ty.is_const
                        && matches!(
                            ty.kind,
                            Kind::Simple(
                                Simple::UInt8 | Simple::Int8 | Simple::UInt16 | Simple::Int16
                            )
                        ))
                    {
                        self.error(
                            source,
                            format!(
                                "cannot use constant '{name}' as an lvalue or take its address"
                            ),
                        );
                    }
                }
                self.use_name(name);
                expr.clone()
            }
            Some(t::LOAD) => {
                let mut pointer = self.expression(child(expr, 1), extract, prefix);
                if extract && !atom(&pointer) {
                    let ty = self.pointer_temp(&pointer);
                    pointer = self.spill(pointer, &ty, prefix)
                }
                node(t::LOAD, vec![Arg::from(pointer)], source)
            }
            Some(t::INDEX) => {
                let array = child(expr, 1);
                let array = if lvalue_like(array) {
                    self.lvalue(array, extract, prefix)
                } else {
                    self.expression(array, extract, prefix)
                };
                let mut index = self.expression(child(expr, 2), extract, prefix);
                if extract && !atom(&index) {
                    let ty = self.infer(&index);
                    if scalar_size(&ty) <= 2 {
                        index = self.spill(index, &ty, prefix)
                    }
                }
                index = self.index_range(expr, &array, index);
                node(t::INDEX, vec![Arg::from(array), Arg::from(index)], source)
            }
            Some(t::FIELD) => {
                let base = child(expr, 1);
                let base = if lvalue_like(base) {
                    self.lvalue(base, extract, prefix)
                } else {
                    self.expression(base, extract, prefix)
                };
                let base = self.atomize_field_base(base, extract, prefix, true);
                node(
                    t::FIELD,
                    vec![Arg::from(base), Arg::from(expr.text(2).unwrap())],
                    source,
                )
            }
            _ => expr.clone(),
        }
    }
    fn assignment(&mut self, expr: &Expr, extract: bool, prefix: &mut Vec<Expr>) -> Expr {
        let source = &expr.source;
        let left = self.lvalue(child(expr, 1), extract, prefix);
        let right = self.expression(child(expr, 2), extract, prefix);
        let lt = self.infer(&left);
        let rt = self.infer(&right);
        if lt.is_const {
            self.error(source, "cannot modify const-qualified object");
        }
        self.narrowing(&right, &rt, &lt, source, "assignment");
        self.assignment_lints(&left, &right, &lt, &rt, source, None);
        if lt.is_aggregate() || rt.is_aggregate() {
            let name = match (&lt.kind, &rt.kind) {
                (Kind::Struct(a), Kind::Struct(b)) | (Kind::Union(a), Kind::Union(b))
                    if a == b && self.global.fields.contains_key(a) =>
                {
                    Some(a.clone())
                }
                _ => None,
            };
            let Some(name) = name else {
                self.error(
                    source,
                    format!("incompatible struct/union assignment: {lt} = {rt}"),
                );
                return node(t::EMPTY, Vec::new(), source);
            };
            let size = self.global.size(&lt, source);
            if size >= 64 {
                self.warning(
                    source,
                    format!("struct copy of {size} bytes in function {}", self.function),
                );
            }
            if self.contains_readonly(&left) {
                self.error(source, "cannot modify readonly ROM aggregate");
                return node(t::EMPTY, Vec::new(), source);
            }
            if self.contains_readonly(&right) {
                let fields = self.global.fields.get(&name).cloned().unwrap_or_default();
                for field in fields {
                    let dst = node(
                        t::FIELD,
                        vec![Arg::from(left.clone()), Arg::from(field.name.clone())],
                        source,
                    );
                    let src = node(
                        t::FIELD,
                        vec![Arg::from(right.clone()), Arg::from(field.name)],
                        source,
                    );
                    let copy = if field.ty.is_array() {
                        let size = self.global.size(&field.ty, source);
                        if size <= 0 {
                            node(t::EMPTY, Vec::new(), source)
                        } else {
                            node(
                                t::CALL,
                                vec![
                                    Arg::from(node(
                                        t::NAME,
                                        vec![Arg::from(if size > 16 && size <= 255 {
                                            "__memcpy_small"
                                        } else {
                                            "__memcpy"
                                        })],
                                        source,
                                    )),
                                    Arg::from(node(t::ADDRESS_OF, vec![Arg::from(dst)], source)),
                                    Arg::from(node(t::ADDRESS_OF, vec![Arg::from(src)], source)),
                                    Arg::from(node(t::INTEGER, vec![Arg::Int(size)], source)),
                                ],
                                source,
                            )
                        }
                    } else {
                        node(t::ASSIGN, vec![Arg::from(dst), Arg::from(src)], source)
                    };
                    prefix.extend(self.statement(&copy));
                }
                return node(t::EMPTY, Vec::new(), source);
            }
        }
        node(t::ASSIGN, vec![Arg::from(left), Arg::from(right)], source)
    }
    fn contains_readonly(&self, expr: &Expr) -> bool {
        let expr = strip_casts(expr);
        match expr.tag() {
            Some(t::NAME) => self.global.readonly.contains(expr.text(1).unwrap()),
            Some(t::FIELD | t::LOAD | t::ADDRESS_OF) => self.contains_readonly(child(expr, 1)),
            Some(t::INDEX) => {
                self.contains_readonly(child(expr, 1)) || self.contains_readonly(child(expr, 2))
            }
            _ => false,
        }
    }
}
fn lvalue_like(e: &Expr) -> bool {
    matches!(e.tag(), Some(t::NAME | t::LOAD | t::INDEX | t::FIELD))
}
pub(super) fn strip_casts(mut expr: &Expr) -> &Expr {
    while expr.is(t::CAST) {
        expr = child(expr, 2)
    }
    expr
}
