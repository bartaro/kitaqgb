use super::*;
impl Parser {
    fn scalar_initializer(&mut self) -> Result<Expr> {
        if !self.take(K::LBRACE) {
            return self.expression();
        }
        if self.take(K::RBRACE) {
            return Ok(make!(self, t::INTEGER, 0));
        }
        let value = self.scalar_initializer()?;
        if self.take(K::COMMA) {
            if self.take(K::RBRACE) {
                return Ok(value);
            }
            return Err("too many initializers for scalar".into());
        }
        self.expect(K::RBRACE)?;
        Ok(value)
    }
    fn array_parts(ty: &CType) -> Result<(CType, i32, bool)> {
        match &ty.kind {
            TypeKind::Array(elem, n) => Ok(((**elem).clone(), *n, false)),
            TypeKind::ArrayExpression(elem, dim) => Ok((
                (**elem).clone(),
                if dim.is(t::INTEGER) {
                    dim.int(1).unwrap_or(-1)
                } else {
                    -1
                },
                dim.is(t::EMPTY),
            )),
            _ => Err("expected array type".into()),
        }
    }
    fn infer_array(ty: &mut CType, length: i32, declared: i32, is_unsized: bool) -> Result<i32> {
        if is_unsized {
            if length <= 0 {
                return Err("cannot infer array size from empty initializer".into());
            }
            let (elem, _, _) = Self::array_parts(ty)?;
            ty.kind = TypeKind::Array(Box::new(elem), length);
            Ok(length)
        } else if declared >= 0 && length > declared {
            Err(format!(
                "too many initializers for array (got {length}, declared {declared})"
            ))
        } else {
            Ok(declared)
        }
    }
    fn aggregate_info(&self, ty: &CType) -> Result<Aggregate> {
        let name = match &ty.kind {
            TypeKind::Struct(n) | TypeKind::Union(n) => n,
            _ => {
                return Err(format!(
                    "initializer requires a complete struct/union type: {ty}"
                ));
            }
        };
        self.aggregates
            .get(name)
            .filter(|a| !a.fields.is_empty())
            .cloned()
            .ok_or_else(|| format!("initializer requires a complete struct/union type: {ty}"))
    }
    fn field(info: &Aggregate, name: &str) -> Result<(usize, Field)> {
        info.fields
            .iter()
            .enumerate()
            .find(|(_, f)| f.name == name)
            .map(|(i, f)| (i, f.clone()))
            .ok_or_else(|| format!("unknown field in designated initializer: {name}"))
    }
    fn initializer_newlines(&mut self) {
        while self.take(K::NEWLINE) {}
    }
    fn initializer_string(&mut self, elem: &CType) -> Result<Option<Vec<i32>>> {
        if self.peek().kind != K::STRING {
            return Ok(None);
        }
        let value = self.consume().name.unwrap_or_default();
        if !matches!(elem.kind, TypeKind::Simple(Simple::UInt8)) {
            return Err("string initializer is only supported for u8 arrays".into());
        }
        let mut values = Vec::new();
        for c in value.encode_utf16() {
            if c > 127 {
                return Err("strings can only contain ASCII characters".into());
            }
            values.push(i32::from(c))
        }
        values.push(0);
        Ok(Some(values))
    }
    pub(super) fn readonly_array(
        &mut self,
        ty: &mut CType,
        require_braces: bool,
    ) -> Result<Vec<Expr>> {
        let (elem, declared, is_unsized) = Self::array_parts(ty)?;
        let mut values = Vec::new();
        if let Some(chars) = self.initializer_string(&elem)? {
            for c in chars {
                values.push(make!(self, t::INTEGER, c));
            }
        } else if !self.take(K::LBRACE) {
            if require_braces {
                return Err("array initializer requires braces".into());
            }
            values.push(self.readonly_value(&mut elem.clone())?);
        } else {
            self.initializer_newlines();
            if !self.take(K::RBRACE) {
                loop {
                    self.initializer_newlines();
                    values.push(self.readonly_value(&mut elem.clone())?);
                    self.initializer_newlines();
                    if self.take(K::COMMA) {
                        self.initializer_newlines();
                        if self.take(K::RBRACE) {
                            break;
                        }
                    } else {
                        self.initializer_newlines();
                        self.expect(K::RBRACE)?;
                        break;
                    }
                }
            }
        }
        Self::infer_array(ty, values.len() as i32, declared, is_unsized)?;
        Ok(values)
    }
    fn readonly_value(&mut self, ty: &mut CType) -> Result<Expr> {
        if ty.is_array() {
            let values = self.readonly_array(ty, false)?;
            Ok(self.sequence(values))
        } else if ty.is_aggregate() {
            self.readonly_aggregate(ty)
        } else {
            self.scalar_initializer()
        }
    }
    fn readonly_aggregate(&mut self, ty: &CType) -> Result<Expr> {
        let info = self.aggregate_info(ty)?;
        let mut values = info
            .fields
            .iter()
            .map(|_| make!(self, t::EMPTY))
            .collect::<Vec<_>>();
        let (mut next, mut union_count) = (0, 0);
        let braced = self.take(K::LBRACE);
        if braced {
            self.initializer_newlines();
            if self.take(K::RBRACE) {
                return Ok(self.sequence(values));
            }
        }
        loop {
            self.initializer_newlines();
            let (index, field) = if self.take(K::PERIOD) {
                let name = self.name()?;
                let field = Self::field(&info, &name)?;
                self.expect(K::EQUAL)?;
                field
            } else {
                if info.is_union && union_count > 0 {
                    return Err("too many initializers for union".into());
                }
                let index = if info.is_union { 0 } else { next };
                let field = info
                    .fields
                    .get(index)
                    .cloned()
                    .ok_or("too many initializers for struct")?;
                (index, field)
            };
            values[index] = self.readonly_value(&mut field.ty.clone())?;
            if info.is_union {
                union_count += 1;
                if union_count > 1 {
                    return Err("too many initializers for union".into());
                }
            } else {
                next = next.max(index + 1)
            }
            if !braced {
                break;
            }
            self.initializer_newlines();
            if self.take(K::COMMA) {
                self.initializer_newlines();
                if self.take(K::RBRACE) {
                    break;
                }
            } else {
                self.initializer_newlines();
                self.expect(K::RBRACE)?;
                break;
            }
        }
        Ok(self.sequence(values))
    }
    pub(super) fn local_initializer(
        &mut self,
        ty: &mut CType,
        target: Expr,
        statements: &mut Vec<Expr>,
    ) -> Result<()> {
        if ty.is_array() {
            self.local_array(ty, target, statements)
        } else if ty.is_aggregate() {
            self.local_aggregate(ty, target, statements)
        } else {
            let value = self.scalar_initializer()?;
            statements.push(make!(self, t::ASSIGN, target, value));
            Ok(())
        }
    }
    fn local_array(
        &mut self,
        ty: &mut CType,
        target: Expr,
        statements: &mut Vec<Expr>,
    ) -> Result<()> {
        let (elem, declared, is_unsized) = Self::array_parts(ty)?;
        let mut explicit = Vec::new();
        let mut count = 0i32;
        if let Some(chars) = self.initializer_string(&elem)? {
            for (index, value) in chars.into_iter().enumerate() {
                let dst = make!(
                    self,
                    t::INDEX,
                    target.clone(),
                    make!(self, t::INTEGER, index as i32)
                );
                explicit.push(make!(self, t::ASSIGN, dst, make!(self, t::INTEGER, value)));
                count = index as i32 + 1;
            }
        } else {
            let braced = self.take(K::LBRACE);
            if !braced || !self.take(K::RBRACE) {
                loop {
                    let dst = make!(
                        self,
                        t::INDEX,
                        target.clone(),
                        make!(self, t::INTEGER, count)
                    );
                    self.local_initializer(&mut elem.clone(), dst, &mut explicit)?;
                    count += 1;
                    if !braced {
                        break;
                    }
                    if self.take(K::COMMA) {
                        if self.take(K::RBRACE) {
                            break;
                        }
                    } else {
                        self.expect(K::RBRACE)?;
                        break;
                    }
                }
            }
        }
        let declared = Self::infer_array(ty, count, declared, is_unsized)?;
        for index in 0..declared {
            let dst = make!(
                self,
                t::INDEX,
                target.clone(),
                make!(self, t::INTEGER, index)
            );
            self.zero_initializer(&elem, dst, statements)?;
        }
        statements.extend(explicit);
        Ok(())
    }
    fn designated_path(&mut self, ty: &CType, target: Expr) -> Result<(Expr, CType, usize)> {
        let info = self.aggregate_info(ty)?;
        let name = self.name()?;
        let (index, field) = Self::field(&info, &name)?;
        let mut dst = make!(self, t::FIELD, target, name);
        let mut dst_ty = field.ty;
        while self.take(K::PERIOD) {
            let name = self.name()?;
            let nested = self
                .aggregate_info(&dst_ty)
                .map_err(|_| format!("designated path member is not a struct/union: {name}"))?;
            let (_, field) = Self::field(&nested, &name)?;
            dst = make!(self, t::FIELD, dst, name);
            dst_ty = field.ty;
        }
        self.expect(K::EQUAL)?;
        Ok((dst, dst_ty, index))
    }
    fn local_aggregate(
        &mut self,
        ty: &CType,
        target: Expr,
        statements: &mut Vec<Expr>,
    ) -> Result<()> {
        let info = self.aggregate_info(ty)?;
        let (mut next, mut union_count) = (0, 0);
        let mut explicit = Vec::new();
        let braced = self.take(K::LBRACE);
        if !braced && self.peek().kind == K::NAME && self.ahead(1).kind == K::LPAREN {
            let value = self.expression()?;
            statements.push(make!(self, t::ASSIGN, target, value));
            return Ok(());
        }
        if !braced || !self.take(K::RBRACE) {
            loop {
                let (dst, mut dst_ty, index) = if self.take(K::PERIOD) {
                    self.designated_path(ty, target.clone())?
                } else {
                    if info.is_union && union_count > 0 {
                        return Err("too many initializers for union".into());
                    }
                    let index = if info.is_union { 0 } else { next };
                    let field = info
                        .fields
                        .get(index)
                        .cloned()
                        .ok_or("too many initializers for struct")?;
                    (
                        make!(self, t::FIELD, target.clone(), field.name),
                        field.ty,
                        index,
                    )
                };
                self.local_initializer(&mut dst_ty, dst, &mut explicit)?;
                if info.is_union {
                    union_count += 1;
                    if union_count > 1 {
                        return Err("too many initializers for union".into());
                    }
                } else {
                    next = next.max(index + 1)
                }
                if !braced {
                    break;
                }
                if self.take(K::COMMA) {
                    if self.take(K::RBRACE) {
                        break;
                    }
                } else {
                    self.expect(K::RBRACE)?;
                    break;
                }
            }
        }
        let count = if info.is_union { 1 } else { info.fields.len() };
        for field in info.fields.iter().take(count) {
            let dst = make!(self, t::FIELD, target.clone(), field.name.clone());
            self.zero_initializer(&field.ty, dst, statements)?;
        }
        statements.extend(explicit);
        Ok(())
    }
    fn zero_initializer(&self, ty: &CType, target: Expr, statements: &mut Vec<Expr>) -> Result<()> {
        if ty.is_array() {
            let (elem, n, _) = Self::array_parts(ty)?;
            for i in 0..n {
                let dst = make!(self, t::INDEX, target.clone(), make!(self, t::INTEGER, i));
                self.zero_initializer(&elem, dst, statements)?;
            }
            return Ok(());
        }
        if ty.is_aggregate() {
            if let Ok(info) = self.aggregate_info(ty) {
                let count = if info.is_union { 1 } else { info.fields.len() };
                for field in info.fields.iter().take(count) {
                    let dst = make!(self, t::FIELD, target.clone(), field.name.clone());
                    self.zero_initializer(&field.ty, dst, statements)?;
                }
                return Ok(());
            }
        }
        statements.push(make!(self, t::ASSIGN, target, make!(self, t::INTEGER, 0)));
        Ok(())
    }
}
