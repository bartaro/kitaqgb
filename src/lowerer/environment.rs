use super::*;
#[derive(Default)]
pub(super) struct Global {
    pub globals: BTreeMap<String, CType>,
    pub ranges: BTreeMap<String, [i32; 2]>,
    pub readonly: BTreeSet<String>,
    pub constant_types: BTreeMap<String, CType>,
    pub constant_exprs: BTreeMap<String, Expr>,
    constant_values: BTreeMap<String, i32>,
    pub function_returns: BTreeMap<String, CType>,
    pub function_params: BTreeMap<String, Vec<CType>>,
    pub fields: BTreeMap<String, Vec<Field>>,
    pub diagnostics: Vec<Diagnostic>,
}
pub(super) fn unwrap(decl: &Expr) -> Expr {
    let mut decl = decl.clone();
    loop {
        let index = match decl.tag() {
            Some(t::UNSAFE | t::STATIC) => 1,
            Some(t::BANK | t::FIXED_BANK | t::FIXED_ORDER) => 2,
            _ => break,
        };
        let Some(inner) = decl.child(index) else {
            break;
        };
        decl = inner.with_source(decl.source.clone());
    }
    decl
}
impl Global {
    fn error(&mut self, source: &Position, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(
            Severity::Error,
            source.clone(),
            message.into(),
            0,
        ));
    }
    pub fn build(&mut self, program: &Expr) {
        if !program.is(t::SEQUENCE) {
            return;
        }
        for decl in program.children() {
            let decl = unwrap(decl);
            match decl.tag().unwrap_or("") {
                t::STRUCT | t::UNION => {
                    if let (Some(name), Some(Arg::Fields(fields))) =
                        (decl.text(1), decl.args.get(2))
                    {
                        let mut names = BTreeSet::new();
                        let fields = fields
                            .iter()
                            .filter(|f| names.insert(f.name.clone()))
                            .cloned()
                            .collect();
                        self.fields.entry(name.into()).or_insert(fields);
                    }
                }
                t::VARIABLE | t::EXTERN_VARIABLE => {
                    if let (Some(ty), Some(name)) = (decl.ty(2), decl.text(3)) {
                        if decl.is(t::EXTERN_VARIABLE) {
                            if let Some(previous) = self.globals.get(name) {
                                if !compatible(previous, ty) {
                                    self.error(&decl.source,format!("extern declaration type mismatch for '{name}' (got {ty}, previously {previous})"));
                                }
                            }
                        }
                        self.globals.entry(name.into()).or_insert(ty.clone());
                        if let Some(range) = decl.child(4) {
                            if !self.ranges.contains_key(name) {
                                let bounds = self.range(range);
                                self.ranges.insert(name.into(), bounds);
                            }
                        }
                    }
                }
                t::READONLY_DATA => {
                    if let (Some(ty), Some(name)) = (decl.ty(1), decl.text(2)) {
                        self.globals.entry(name.into()).or_insert(ty.clone());
                        self.readonly.insert(name.into());
                    }
                }
                t::CONSTANT => {
                    if let (Some(ty), Some(name), Some(value)) =
                        (decl.ty(1), decl.text(2), decl.child(3))
                    {
                        self.constant_types.entry(name.into()).or_insert(ty.clone());
                        self.constant_exprs
                            .entry(name.into())
                            .or_insert(value.clone());
                    }
                }
                t::FUNCTION | t::INLINE_FUNCTION | t::FUNCTION_DECL => {
                    if let (Some(ret), Some(name), Some(Arg::Fields(params))) =
                        (decl.ty(1), decl.text(2), decl.args.get(3))
                    {
                        self.function_returns
                            .entry(name.into())
                            .or_insert(ret.clone());
                        self.function_params
                            .entry(name.into())
                            .or_insert_with(|| params.iter().map(|p| p.ty.clone()).collect());
                    }
                }
                _ => {}
            }
        }
        for (name, ty) in [
            ("__mul8x8_hi", Simple::UInt8),
            ("__mul16x8", Simple::UInt16),
            ("__mac16", Simple::UInt16),
            ("__sdot3_q1_7", Simple::UInt16),
        ] {
            self.function_returns
                .entry(name.into())
                .or_insert(CType::simple(ty));
            self.function_params.entry(name.into()).or_default();
        }
    }
    pub fn range(&mut self, e: &Expr) -> [i32; 2] {
        if !e.matches(t::RANGE, 2) {
            self.error(&e.source, "__range: malformed attribute");
            return [0, 0];
        }
        let min = self.eval(child(e, 1));
        let max = self.eval(child(e, 2));
        if min > max {
            self.error(
                &e.source,
                format!("__range: min must be <= max (got {min}..{max})"),
            );
        }
        [min, max]
    }
    pub fn eval(&mut self, e: &Expr) -> i32 {
        if e.is(t::INTEGER) {
            return e.int(1).unwrap_or(0);
        }
        if e.is(t::NAME) {
            let name = e.text(1).unwrap();
            if let Some(n) = self.constant_values.get(name) {
                return *n;
            }
            if let Some(value) = self.constant_exprs.get(name).cloned() {
                self.constant_values.insert(name.into(), 0);
                let n = self.eval(&value);
                self.constant_values.insert(name.into(), n);
                return n;
            }
            self.error(
                &e.source,
                format!("expected constant expression, got name: {name}"),
            );
            return 0;
        }
        if e.is(t::SIZEOF) {
            if let Some(ty) = e.ty(1) {
                return self.size(ty, &e.source);
            }
            self.error(
                &e.source,
                "sizeof(expr) is not supported in __range constant expressions",
            );
            return 0;
        }
        if e.is(t::BITWISE_NOT) {
            return !self.eval(child(e, 1));
        }
        if arithmetic_tag(e.tag().unwrap_or("")) {
            if let Some((left, right)) = e.child(1).zip(e.child(2)) {
                let left = self.eval(left);
                let right = self.eval(right);
                if let Some(value) = arithmetic(e.tag().unwrap_or(""), left, right) {
                    return value;
                }
            }
        }
        self.error(
            &e.source,
            format!("expected constant expression in __range, got: {}", e.show()),
        );
        0
    }
    pub fn try_eval(&mut self, e: &Expr) -> Option<i32> {
        if e.is(t::INTEGER) {
            return e.int(1);
        }
        if e.is(t::NAME) {
            let name = e.text(1)?;
            if let Some(n) = self.constant_values.get(name) {
                return Some(*n);
            }
            let value = self.constant_exprs.get(name)?.clone();
            self.constant_values.insert(name.into(), 0);
            if let Some(n) = self.try_eval(&value) {
                self.constant_values.insert(name.into(), n);
                return Some(n);
            }
            self.constant_values.remove(name);
            return None;
        }
        if e.is(t::CAST) {
            let value = self.try_eval(child(e, 2))?;
            let ty = e.ty(1)?;
            return Some(match ty.kind {
                Kind::Simple(Simple::UInt8) => value & 255,
                Kind::Simple(Simple::Int8) => i32::from(value as i8),
                Kind::Simple(Simple::UInt16) | Kind::Pointer(_) | Kind::Enum(_) => value & 65535,
                Kind::Simple(Simple::Int16) => i32::from(value as i16),
                _ => value,
            });
        }
        if e.is(t::SIZEOF) {
            return e.ty(1).map(|ty| self.size(ty, &e.source));
        }
        if e.is(t::BITWISE_NOT) {
            return Some(!self.try_eval(child(e, 1))?);
        }
        if arithmetic_tag(e.tag().unwrap_or("")) {
            if let Some((left, right)) = e.child(1).zip(e.child(2)) {
                let left = self.try_eval(left)?;
                let right = self.try_eval(right)?;
                return arithmetic(e.tag().unwrap_or(""), left, right);
            }
        }
        None
    }
    pub fn size(&mut self, ty: &CType, source: &Position) -> i32 {
        match &ty.kind {
            Kind::Simple(Simple::UInt8 | Simple::Int8) => 1,
            Kind::Simple(Simple::Void) => 0,
            Kind::Pointer(_) => 2,
            Kind::Array(elem, n) => self.size(elem, source).wrapping_mul(*n),
            Kind::ArrayExpression(elem, n) => {
                let n = self.eval(n);
                self.size(elem, source).wrapping_mul(n)
            }
            Kind::Struct(name) | Kind::Union(name) => {
                let Some(fields) = self.fields.get(name).filter(|f| !f.is_empty()).cloned() else {
                    self.error(source, format!("incomplete type in sizeof: {ty}"));
                    return 0;
                };
                let sizes = fields
                    .iter()
                    .map(|f| self.size(&f.ty, source))
                    .collect::<Vec<_>>();
                if matches!(ty.kind, Kind::Union(_)) {
                    sizes.into_iter().max().unwrap_or(0)
                } else {
                    sizes.into_iter().fold(0i32, i32::wrapping_add)
                }
            }
            _ => 2,
        }
    }
    pub fn field_type(&self, ty: &CType, name: &str) -> Option<CType> {
        let ty = if ty.is_array() || ty.is_pointer() {
            sub_type(ty)?
        } else {
            ty
        };
        let aggregate = match &ty.kind {
            Kind::Struct(n) | Kind::Union(n) => n,
            _ => return None,
        };
        self.fields
            .get(aggregate)?
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.ty.clone())
    }
}
fn arithmetic_tag(tag: &str) -> bool {
    matches!(
        tag,
        t::ADD
            | t::SUBTRACT
            | t::MULTIPLY
            | t::DIVIDE
            | t::MODULUS
            | t::BITWISE_AND
            | t::BITWISE_OR
            | t::BITWISE_XOR
            | t::SHIFT_LEFT
            | t::SHIFT_RIGHT
    )
}
fn arithmetic(tag: &str, a: i32, b: i32) -> Option<i32> {
    Some(match tag {
        t::ADD => a.wrapping_add(b),
        t::SUBTRACT => a.wrapping_sub(b),
        t::MULTIPLY => a.wrapping_mul(b),
        t::DIVIDE => a.checked_div(b)?,
        t::MODULUS => a.checked_rem(b)?,
        t::BITWISE_AND => a & b,
        t::BITWISE_OR => a | b,
        t::BITWISE_XOR => a ^ b,
        t::SHIFT_LEFT => a.wrapping_shl((b & 31) as u32),
        t::SHIFT_RIGHT => a.wrapping_shr((b & 31) as u32),
        _ => return None,
    })
}
fn compatible(a: &CType, b: &CType) -> bool {
    if a == b {
        return true;
    }
    if !a.is_array() || !b.is_array() || sub_type(a) != sub_type(b) {
        return false;
    }
    if matches!(&a.kind,Kind::ArrayExpression(_,d)if d.is(t::EMPTY))
        || matches!(&b.kind,Kind::ArrayExpression(_,d)if d.is(t::EMPTY))
    {
        return true;
    }
    let length = |t: &CType| match &t.kind {
        Kind::Array(_, n) => Some(*n),
        Kind::ArrayExpression(_, n) if n.is(t::INTEGER) => n.int(1).filter(|n| *n >= 0),
        _ => None,
    };
    match (length(a), length(b)) {
        (Some(a), Some(b)) => a == b,
        _ => match (&a.kind, &b.kind) {
            (Kind::ArrayExpression(_, a), Kind::ArrayExpression(_, b)) => a.show() == b.show(),
            _ => false,
        },
    }
}
