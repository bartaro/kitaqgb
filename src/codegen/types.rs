use super::*;
impl Environment {
    pub fn field(&mut self, origin: &Expr, base: &Expr, name: &str) -> Option<Field> {
        let mut ty = self.type_of(base);
        if ty.is_pointer() || ty.is_array() {
            ty = sub_type(&ty)?.clone()
        }
        let aggregate = match &ty.kind {
            Kind::Struct(n) | Kind::Union(n) => n,
            _ => {
                self.error(&origin.source, format!("Not a struct/union type: {ty}"));
                return None;
            }
        };
        let result = self
            .aggregates
            .get(aggregate)
            .and_then(|a| a.fields.iter().find(|f| f.name == name))
            .cloned();
        if result.is_none() {
            self.error(&origin.source, format!("Invalid field name: {name}"))
        }
        result
    }
    pub fn type_of(&mut self, e: &Expr) -> CType {
        let u8_type = || CType::simple(Simple::UInt8);
        let tag = e.tag().unwrap_or("");
        match tag {
            t::NAME => {
                let name = e.text(1).unwrap();
                if let Some(symbol) = self.find(name) {
                    if symbol.ty.is_array() {
                        CType::pointer(sub_type(&symbol.ty).unwrap().clone())
                    } else {
                        symbol.ty.clone()
                    }
                } else if let Some(function) = self.functions.get(name) {
                    CType::function(
                        function.return_type.clone(),
                        function.parameters.iter().map(|p| p.ty.clone()).collect(),
                    )
                } else {
                    u8_type()
                }
            }
            t::INTEGER => integer(e.int(1).unwrap_or(0)),
            t::SIZEOF | t::OFFSETOF => CType::simple(Simple::UInt16),
            t::CAST => e.ty(1).unwrap().clone(),
            t::PRE_INCREMENT
            | t::POST_INCREMENT
            | t::PRE_DECREMENT
            | t::POST_DECREMENT
            | t::SLICE => self.type_of(child(e, 1)),
            t::BITWISE_NOT => {
                let ty = self.type_of(child(e, 1));
                if ty.is_integer() {
                    promotion(&ty, &ty)
                } else if self.size(e, &ty) == 2 {
                    CType::simple(Simple::UInt16)
                } else {
                    u8_type()
                }
            }
            t::LOGICAL_NOT => u8_type(),
            t::LOAD => {
                let ty = self.type_of(child(e, 1));
                if ty.is_pointer() || ty.is_array() {
                    sub_type(&ty).unwrap().clone()
                } else {
                    u8_type()
                }
            }
            t::ADDRESS_OF => {
                let sub = child(e, 1);
                if sub.is(t::NAME) {
                    if let Some(symbol) =
                        self.find(sub.text(1).unwrap()).filter(|s| s.ty.is_array())
                    {
                        return CType::pointer(symbol.ty.clone());
                    }
                }
                CType::pointer(self.type_of(sub))
            }
            t::INDEX => {
                let ty = self.type_of(child(e, 1));
                if ty.is_array() || ty.is_pointer() {
                    sub_type(&ty).unwrap().clone()
                } else {
                    u8_type()
                }
            }
            t::FIELD => self
                .field(e, child(e, 1), e.text(2).unwrap())
                .map_or_else(u8_type, |f| f.ty),
            t::CALL => {
                let target = child(e, 1);
                let target_type = self.type_of(target);
                let callable = if let Kind::Pointer(s) = &target_type.kind {
                    s.as_ref()
                } else {
                    &target_type
                };
                if let Kind::Function(ret, _) = &callable.kind {
                    return (**ret).clone();
                }
                if target.is(t::NAME) {
                    let name = target.text(1).unwrap();
                    if name == "__farcall" {
                        if e.args.len() == 4 {
                            if let Some(name) =
                                child(e, 3).text(1).filter(|_| child(e, 3).is(t::NAME))
                            {
                                if let Some(f) = self.functions.get(name) {
                                    return f.return_type.clone();
                                }
                            }
                        }
                        return u8_type();
                    }
                    if let Some(ty) = super::intrinsic_types::intrinsic_type(name) {
                        return ty;
                    }
                    if let Some(f) = self.functions.get(name) {
                        return f.return_type.clone();
                    }
                }
                u8_type()
            }
            t::READONLY_DATA => {
                if let Some(ty) = e.ty(1) {
                    CType::pointer(if ty.is_array() {
                        sub_type(ty).unwrap().clone()
                    } else {
                        ty.clone()
                    })
                } else {
                    CType::pointer(u8_type())
                }
            }
            t::CONDITIONAL => {
                let mut yes = self.type_of(child(e, 2));
                let mut no = self.type_of(child(e, 3));
                if yes.is_array() {
                    yes = CType::pointer(sub_type(&yes).unwrap().clone())
                }
                if no.is_array() {
                    no = CType::pointer(sub_type(&no).unwrap().clone())
                }
                if yes.is_pointer() {
                    yes
                } else if no.is_pointer() {
                    no
                } else if yes.is_integer() || no.is_integer() {
                    promotion(&yes, &no)
                } else {
                    yes
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
            ) =>
            {
                let left = self.type_of(child(e, 1));
                let right = self.type_of(child(e, 2));
                if matches!(tag, t::ADD | t::SUBTRACT) {
                    if left.is_pointer() && right.is_integer() {
                        return left;
                    }
                    if tag == t::ADD && right.is_pointer() && left.is_integer() {
                        return right;
                    }
                    if tag == t::SUBTRACT && left.is_pointer() && right.is_pointer() {
                        return CType::simple(Simple::UInt16);
                    }
                }
                if matches!(tag, t::SHIFT_LEFT | t::SHIFT_RIGHT) {
                    if left.is_integer() {
                        promotion(&left, &left)
                    } else if self.size(child(e, 1), &left) == 2 {
                        CType::simple(Simple::UInt16)
                    } else {
                        u8_type()
                    }
                } else if left.is_integer() || right.is_integer() {
                    promotion(&left, &right)
                } else if self.size(child(e, 1), &left) == 2 || self.size(child(e, 2), &right) == 2
                {
                    CType::simple(Simple::UInt16)
                } else {
                    u8_type()
                }
            }
            _ => u8_type(),
        }
    }
}
