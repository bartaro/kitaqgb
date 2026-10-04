use super::*;
fn compatible(a: &CType, b: &CType) -> bool {
    if a == b {
        return true;
    }
    fn array(ty: &CType) -> Option<(&CType, Option<i32>, Option<&Expr>)> {
        match &ty.kind {
            Kind::Array(t, n) => Some((t, Some(*n), None)),
            Kind::ArrayExpression(t, e) => Some((t, None, Some(e))),
            _ => None,
        }
    }
    let (Some((at, an, ae)), Some((bt, bn, be))) = (array(a), array(b)) else {
        return false;
    };
    if at != bt {
        return false;
    }
    if ae.is_some_and(|e| e.is(t::EMPTY)) || be.is_some_and(|e| e.is(t::EMPTY)) {
        return true;
    }
    match (an, ae, bn, be) {
        (Some(a), None, Some(b), None) => a == b,
        (Some(a), None, None, Some(b)) => {
            b.is(t::INTEGER) && b.int(1).is_some_and(|n| n >= 0 && n == a)
        }
        (None, Some(a), Some(b), None) => {
            a.is(t::INTEGER) && a.int(1).is_some_and(|n| n >= 0 && n == b)
        }
        (None, Some(a), None, Some(b)) => a.show() == b.show(),
        _ => false,
    }
}
impl Emitter {
    pub(super) fn validate_externs(&mut self, declarations: &[super::declarations::Declaration]) {
        let mut declared = Vec::<(String, CType, Expr)>::new();
        for d in declarations {
            let e = &d.expr;
            if !e.is(t::EXTERN_VARIABLE) {
                continue;
            }
            let name = e.text(3).unwrap();
            let ty = e.ty(2).unwrap();
            if let Some((_, previous, _)) = declared.iter().find(|(n, _, _)| n == name) {
                if !compatible(previous, ty) {
                    self.env.error(&e.source,format!(
                    "extern declaration type mismatch for '{name}' (got {ty}, previously {previous})"));
                }
            } else {
                declared.push((name.into(), ty.clone(), e.clone()));
            }
        }
        for (name, ty, e) in declared {
            let Some(symbol) = self.env.find(&name).cloned() else {
                self.env
                    .error(&e.source, format!("extern symbol not defined: {name}"));
                continue;
            };
            if symbol.tag == SymbolTag::Constant && !self.env.options.const_scalar_in_rom {
                self.env.error(&e.source,format!("extern refers to an object, but '{name}' is a compile-time constant (enable -Zconst-scalar-in-rom to take its address)"));
                continue;
            }
            if !compatible(&ty, &symbol.ty) {
                self.env.error(&e.source,format!(
                "extern declaration does not match definition for '{name}' (declared {ty}, defined {})",symbol.ty));
            }
        }
    }
}
