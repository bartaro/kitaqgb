use super::*;
#[derive(Clone, Debug)]
pub struct Constant {
    pub value: i32,
    pub ty: CType,
}
impl Constant {
    fn new(value: i32, ty: CType) -> Self {
        Self { value, ty }
    }
}
impl Environment {
    pub fn constant(&mut self, e: &Expr) -> Constant {
        self.evaluate(e, true)
            .unwrap_or_else(|| Constant::new(0, CType::simple(Simple::UInt16)))
    }
    pub fn try_constant(&mut self, e: &Expr) -> Option<Constant> {
        self.evaluate(e, false)
    }
    fn evaluate(&mut self, e: &Expr, required: bool) -> Option<Constant> {
        let tag = e.tag().unwrap_or("");
        let result = match tag {
            t::INTEGER => {
                let n = e.int(1)?;
                Some(Constant::new(n, integer(n)))
            }
            t::NAME => {
                let name = e.text(1)?;
                if !self
                    .find(name)
                    .is_some_and(|s| s.tag == SymbolTag::Constant)
                {
                    self.ensure_constant(name, e);
                }
                if let Some(symbol) = self.find(name).filter(|s| s.tag == SymbolTag::Constant) {
                    Some(Constant::new(symbol.value, symbol.ty.clone()))
                } else {
                    if required {
                        self.error(
                            &e.source,
                            format!("expected constant expression, got name: {name}"),
                        );
                        return Some(Constant::new(0, CType::simple(Simple::UInt16)));
                    }
                    None
                }
            }
            t::SIZEOF => {
                let ty = if let Some(ty) = e.ty(1) {
                    ty.clone()
                } else if let Some(value) = e.child(1) {
                    if value.is(t::NAME) {
                        self.find(value.text(1).unwrap())
                            .map(|s| s.ty.clone())
                            .unwrap_or_else(|| self.type_of(value))
                    } else {
                        self.type_of(value)
                    }
                } else {
                    return None;
                };
                let size = self.size(e, &ty);
                Some(Constant::new(size, CType::simple(Simple::UInt16)))
            }
            t::OFFSETOF => Some(Constant::new(
                self.offset_of(e, e.ty(1)?, e.text(2)?),
                CType::simple(Simple::UInt16),
            )),
            t::CAST => {
                let value = self.evaluate(child(e, 2), required)?;
                let ty = e.ty(1)?.clone();
                Some(Constant::new(normalize(value.value, &ty), ty))
            }
            t::CONDITIONAL => {
                let cond = self.evaluate(child(e, 1), required)?;
                self.evaluate(child(e, if cond.value != 0 { 2 } else { 3 }), required)
            }
            t::LOGICAL_AND | t::LOGICAL_OR => {
                let a = self.evaluate(child(e, 1), required)?.value;
                let value = if (tag == t::LOGICAL_AND && a == 0) || (tag == t::LOGICAL_OR && a != 0)
                {
                    i32::from(a != 0)
                } else {
                    i32::from(self.evaluate(child(e, 2), required)?.value != 0)
                };
                Some(Constant::new(value, CType::simple(Simple::UInt8)))
            }
            t::LOGICAL_NOT => Some(Constant::new(
                i32::from(self.evaluate(child(e, 1), required)?.value == 0),
                CType::simple(Simple::UInt8),
            )),
            t::BITWISE_NOT => {
                let sub = self.evaluate(child(e, 1), required)?;
                let ty = promotion(&sub.ty, &sub.ty);
                let value = if ty.is_signed() {
                    i32::from(sub.value as i16)
                } else {
                    sub.value & 65535
                };
                Some(Constant::new(normalize(!value, &ty), ty))
            }
            t::CALL if !required => {
                let target = child(e, 1);
                let name = target.text(1).filter(|_| target.is(t::NAME));
                if name == Some("__cgb_is_cgb") && e.args.len() == 2 {
                    self.options
                        .known_cgb
                        .map(|n| Constant::new(i32::from(n != 0), CType::simple(Simple::UInt8)))
                } else if name == Some("__bankof") && e.args.len() == 3 && child(e, 2).is(t::NAME) {
                    let name = child(e, 2).text(1)?;
                    self.functions
                        .get(name)
                        .map(|f| f.rom_bank)
                        .or_else(|| self.readonly_banks.get(name).copied())
                        .map(|n| Constant::new(n & 255, CType::simple(Simple::UInt8)))
                } else {
                    None
                }
            }
            _ if matches!(
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
                    | t::EQUAL
                    | t::NOT_EQUAL
                    | t::LESS_THAN
                    | t::LESS_THAN_OR_EQUAL
                    | t::GREATER_THAN
                    | t::GREATER_THAN_OR_EQUAL
            ) =>
            {
                let a = self.evaluate(child(e, 1), required)?;
                let b = self.evaluate(child(e, 2), required)?;
                let ty = if matches!(tag, t::SHIFT_LEFT | t::SHIFT_RIGHT) {
                    promotion(&a.ty, &a.ty)
                } else {
                    promotion(&a.ty, &b.ty)
                };
                let signed = ty.is_signed();
                let left = if signed {
                    i32::from(a.value as i16)
                } else {
                    a.value & 65535
                };
                let right = if signed {
                    i32::from(b.value as i16)
                } else {
                    b.value & 65535
                };
                let compare = match tag {
                    t::EQUAL => Some(left == right),
                    t::NOT_EQUAL => Some(left != right),
                    t::LESS_THAN => Some(left < right),
                    t::LESS_THAN_OR_EQUAL => Some(left <= right),
                    t::GREATER_THAN => Some(left > right),
                    t::GREATER_THAN_OR_EQUAL => Some(left >= right),
                    _ => None,
                };
                if let Some(compare) = compare {
                    Some(Constant::new(
                        i32::from(compare),
                        CType::simple(Simple::UInt8),
                    ))
                } else if matches!(tag, t::DIVIDE | t::MODULUS) && right == 0 {
                    if required {
                        self.error(&e.source, "division by zero in constant expression");
                        Some(Constant::new(0, ty))
                    } else {
                        None
                    }
                } else if a.ty.is_pointer() && b.ty.is_integer() {
                    let size = self.size(e, sub_type(&a.ty).unwrap());
                    let value = if tag == t::ADD {
                        left.wrapping_add(right.wrapping_mul(size))
                    } else if tag == t::SUBTRACT {
                        left.wrapping_sub(right.wrapping_mul(size))
                    } else {
                        0
                    };
                    Some(Constant::new(normalize(value, &a.ty), a.ty))
                } else if b.ty.is_pointer() && a.ty.is_integer() && tag == t::ADD {
                    let size = self.size(e, sub_type(&b.ty).unwrap());
                    Some(Constant::new(
                        normalize(right.wrapping_add(left.wrapping_mul(size)), &b.ty),
                        b.ty,
                    ))
                } else if a.ty.is_pointer() && b.ty.is_pointer() && tag == t::SUBTRACT {
                    let size = self.size(e, sub_type(&a.ty).unwrap());
                    let mut value = left.wrapping_sub(right);
                    if size > 1 {
                        value /= size
                    }
                    let ty = CType::simple(Simple::UInt16);
                    Some(Constant::new(normalize(value, &ty), ty))
                } else {
                    let value = match tag {
                        t::ADD => left.wrapping_add(right),
                        t::SUBTRACT => left.wrapping_sub(right),
                        t::MULTIPLY => left.wrapping_mul(right),
                        t::DIVIDE => left / right,
                        t::MODULUS => left % right,
                        t::BITWISE_AND => left & right,
                        t::BITWISE_OR => left | right,
                        t::BITWISE_XOR => left ^ right,
                        t::SHIFT_LEFT => left.wrapping_shl((b.value & 31) as u32),
                        t::SHIFT_RIGHT => left.wrapping_shr((b.value & 31) as u32),
                        _ => unreachable!(),
                    };
                    Some(Constant::new(normalize(value, &ty), ty))
                }
            }
            _ => None,
        };
        if result.is_none() && required {
            self.error(
                &e.source,
                format!("expected constant expression, got: {}", e.show()),
            );
            Some(Constant::new(0, CType::simple(Simple::UInt16)))
        } else {
            result
        }
    }
}
