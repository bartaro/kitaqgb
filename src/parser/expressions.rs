use super::*;
impl Parser {
    pub(super) fn expression(&mut self) -> Result<Expr> {
        self.assign_expression()
    }
    fn assign_expression(&mut self) -> Result<Expr> {
        let left = self.conditional_expression()?;
        if self.take(K::EQUAL) {
            let right = self.assign_expression()?;
            return Ok(make!(self, t::ASSIGN, left, right));
        }
        let op = match self.peek().kind {
            K::PLUS_EQUALS => t::ADD,
            K::MINUS_EQUALS => t::SUBTRACT,
            K::STAR_EQUALS => t::MULTIPLY,
            K::SLASH_EQUALS => t::DIVIDE,
            K::PERCENT_EQUALS => t::MODULUS,
            K::SHIFT_LEFT_EQUALS => t::SHIFT_LEFT,
            K::SHIFT_RIGHT_EQUALS => t::SHIFT_RIGHT,
            K::PIPE_EQUALS => t::BITWISE_OR,
            K::AMPERSAND_EQUALS => t::BITWISE_AND,
            K::CARET_EQUALS => t::BITWISE_XOR,
            _ => return Ok(left),
        };
        self.consume();
        let right = self.assign_expression()?;
        Ok(make!(self, t::ASSIGN_MODIFY, op, left, right))
    }
    fn conditional_expression(&mut self) -> Result<Expr> {
        let test = self.binary_expression(1)?;
        if !self.take(K::QUESTION_MARK) {
            return Ok(test);
        }
        let yes = self.expression()?;
        self.expect(K::COLON)?;
        let no = self.conditional_expression()?;
        Ok(make!(self, t::CONDITIONAL, test, yes, no))
    }
    fn binary_expression(&mut self, min: u8) -> Result<Expr> {
        let mut left = self.unary_expression()?;
        loop {
            let (precedence, tag) = match self.peek().kind {
                K::LOGICAL_OR => (1, t::LOGICAL_OR),
                K::LOGICAL_AND => (2, t::LOGICAL_AND),
                K::PIPE => (3, t::BITWISE_OR),
                K::CARET => (4, t::BITWISE_XOR),
                K::AMPERSAND => (5, t::BITWISE_AND),
                K::DOUBLE_EQUAL => (6, t::EQUAL),
                K::NOT_EQUAL => (6, t::NOT_EQUAL),
                K::LESS_THAN => (7, t::LESS_THAN),
                K::LESS_THAN_OR_EQUAL => (7, t::LESS_THAN_OR_EQUAL),
                K::GREATER_THAN => (7, t::GREATER_THAN),
                K::GREATER_THAN_OR_EQUAL => (7, t::GREATER_THAN_OR_EQUAL),
                K::SHIFT_LEFT => (8, t::SHIFT_LEFT),
                K::SHIFT_RIGHT => (8, t::SHIFT_RIGHT),
                K::PLUS => (9, t::ADD),
                K::MINUS => (9, t::SUBTRACT),
                K::STAR => (10, t::MULTIPLY),
                K::SLASH => (10, t::DIVIDE),
                K::PERCENT => (10, t::MODULUS),
                _ => break,
            };
            if precedence < min {
                break;
            }
            self.consume();
            let right = self.binary_expression(precedence + 1)?;
            left = make!(self, tag, left, right);
        }
        Ok(left)
    }
    fn unary_expression(&mut self) -> Result<Expr> {
        if self.keyword("offsetof") || self.keyword("__builtin_offsetof") {
            self.expect(K::LPAREN)?;
            let ty = self.expect_type()?;
            self.expect(K::COMMA)?;
            let mut path = vec![self.name()?];
            while self.take(K::PERIOD) {
                path.push(self.name()?)
            }
            self.expect(K::RPAREN)?;
            return Ok(make!(self, t::OFFSETOF, ty, path.join(".")));
        }
        if self.keyword("sizeof") {
            let arg = if self.take(K::LPAREN) {
                let looks_type = self.peek().kind == K::NAME
                    && self.peek().name.as_ref().is_some_and(|n| {
                        matches!(
                            n.as_str(),
                            "void"
                                | "uint8_t"
                                | "u8"
                                | "char"
                                | "bool"
                                | "uint16_t"
                                | "u16"
                                | "struct"
                                | "union"
                        ) || self.typedefs.contains_key(n)
                    });
                let arg = if looks_type {
                    Arg::Type(self.expect_type()?)
                } else {
                    Arg::from(self.expression()?)
                };
                self.expect(K::RPAREN)?;
                arg
            } else {
                Arg::from(self.unary_expression()?)
            };
            return Ok(Expr::new(t::SIZEOF, vec![arg]).with_source(self.source.clone()));
        }
        let tag = match self.peek().kind {
            K::STAR => t::LOAD,
            K::AMPERSAND => t::ADDRESS_OF,
            K::TILDE => t::BITWISE_NOT,
            K::LOGICAL_NOT => t::LOGICAL_NOT,
            K::INCREMENT => t::PRE_INCREMENT,
            K::DECREMENT => t::PRE_DECREMENT,
            _ => return self.suffix_expression(),
        };
        self.consume();
        let inner = self.unary_expression()?;
        Ok(make!(self, tag, inner))
    }
    fn suffix_expression(&mut self) -> Result<Expr> {
        let mut expr = self.primary_expression()?;
        loop {
            if self.take(K::LPAREN) {
                let mut args = vec![Arg::from(expr)];
                if !self.take(K::RPAREN) {
                    loop {
                        args.push(Arg::from(self.expression()?));
                        if self.take(K::RPAREN) {
                            break;
                        }
                        self.expect(K::COMMA)?;
                    }
                }
                expr = Expr::new(t::CALL, args);
            } else if self.take(K::PERIOD) {
                let field = self.name()?;
                expr = make!(self, t::FIELD, expr, field);
            } else if self.take(K::ARROW) {
                let field = self.name()?;
                expr = make!(self, t::FIELD, make!(self, t::LOAD, expr), field);
            } else if self.take(K::INCREMENT) {
                expr = make!(self, t::POST_INCREMENT, expr);
            } else if self.take(K::DECREMENT) {
                expr = make!(self, t::POST_DECREMENT, expr);
            } else if self.take(K::LBRACKET) {
                let index = self.expression()?;
                self.expect(K::RBRACKET)?;
                expr = make!(self, t::INDEX, expr, index);
            } else {
                break;
            }
        }
        Ok(expr)
    }
    fn primary_expression(&mut self) -> Result<Expr> {
        if let Some(integer) = self.signed_integer()? {
            return Ok(make!(self, t::INTEGER, integer));
        }
        if self.peek().kind == K::STRING {
            let value = self.consume().name.unwrap_or_default();
            let name = if let Some(name) = self.string_pool.get(&value) {
                name.clone()
            } else {
                let name = format!("$string{}", self.string_pool.len());
                let mut values = Vec::new();
                for c in value.encode_utf16() {
                    if c > 127 {
                        return Err("strings can only contain ASCII characters".into());
                    }
                    values.push(i32::from(c));
                }
                values.push(0);
                let decl = make!(
                    self,
                    t::FIXED_BANK,
                    0,
                    make!(
                        self,
                        t::BANK,
                        0,
                        make!(
                            self,
                            t::READONLY_DATA,
                            CType::array(CType::simple(Simple::UInt8), values.len() as i32),
                            name.clone(),
                            values
                        )
                    )
                );
                self.string_declarations.push(decl);
                self.string_pool.insert(value, name.clone());
                name
            };
            return Ok(make!(self, t::NAME, name));
        }
        if self.peek().kind == K::NAME {
            let name = self.name()?;
            return Ok(make!(self, t::NAME, self.resolve_name(&name)));
        }
        if self.take(K::LPAREN) {
            if let Some(ty) = self.try_type()? {
                self.expect(K::RPAREN)?;
                let expr = self.unary_expression()?;
                Ok(make!(self, t::CAST, ty, expr))
            } else {
                let expr = self.expression()?;
                self.expect(K::RPAREN)?;
                Ok(expr)
            }
        } else {
            Err("expected an expression".into())
        }
    }
    pub(super) fn aligned_attribute(&mut self) -> Result<Option<i32>> {
        if !self.keyword("__aligned") {
            return Ok(None);
        }
        self.expect(K::LPAREN)?;
        let n = self.integer()?;
        self.expect(K::RPAREN)?;
        if n <= 0 || n & (n - 1) != 0 {
            return Err(format!(
                "__aligned(N): N must be a power-of-two positive integer literal (got {n})"
            ));
        }
        Ok(Some(n))
    }
    pub(super) fn range_attribute(&mut self) -> Result<Option<Expr>> {
        if !self.keyword("__range") {
            return Ok(None);
        }
        self.expect(K::LPAREN)?;
        let min = self.expression()?;
        self.expect(K::COMMA)?;
        let max = self.expression()?;
        self.expect(K::RPAREN)?;
        Ok(Some(make!(self, t::RANGE, min, max)))
    }
    pub(super) fn expect_type(&mut self) -> Result<CType> {
        self.try_type()?.ok_or_else(|| "expected a type".into())
    }
    pub(super) fn try_type(&mut self) -> Result<Option<CType>> {
        let (mut is_const, mut strict, mut bitflags, mut safe, mut packed, mut align) =
            (false, false, false, false, false, 0);
        loop {
            if self.keyword("const") {
                is_const = true
            } else if self.keyword("__enum_strict") {
                strict = true
            } else if self.keyword("__bitflags") {
                bitflags = true
            } else if self.keyword("__safe_index") {
                safe = true
            } else if self.keyword("__packed") {
                packed = true
            } else if let Some(a) = self.aligned_attribute()? {
                align = a
            } else {
                break;
            }
        }
        let mut ty = if self.keyword("void") {
            CType::simple(Simple::Void)
        } else if self.keyword("uint8_t")
            || self.keyword("u8")
            || self.keyword("char")
            || self.keyword("bool")
        {
            CType::simple(Simple::UInt8)
        } else if self.keyword("uint16_t") || self.keyword("u16") {
            CType::simple(Simple::UInt16)
        } else if self.keyword("enum") {
            if self.keyword("class") || self.keyword("struct") {
                strict = true
            }
            let mut name = if self.peek().kind == K::NAME {
                Some(self.name()?)
            } else {
                None
            };
            if self.take(K::LBRACE) {
                if name.is_none() {
                    name = Some(self.anonymous_name("__anon_enum"))
                }
                let name = name.as_ref().unwrap().clone();
                let mut enum_ty = self
                    .enum_tags
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| CType::new(TypeKind::Enum(name.clone())));
                if strict {
                    self.strict_enums.insert(name.clone());
                }
                enum_ty.is_enum_strict = self.strict_enums.contains(&name);
                self.enum_tags.insert(name.clone(), enum_ty.clone());
                let mut current = make!(self, t::INTEGER, -1);
                while !self.take(K::RBRACE) {
                    let value_name = self.name()?;
                    if self.take(K::EQUAL) {
                        current = self.expression()?;
                    } else {
                        current = make!(self, t::ADD, current, make!(self, t::INTEGER, 1))
                    }
                    let decl = make!(
                        self,
                        t::CONSTANT,
                        enum_ty.clone(),
                        value_name,
                        current.clone()
                    );
                    self.pending.push(self.wrap_pragmas(decl));
                    self.take(K::COMMA);
                }
            }
            let name = name.ok_or("expected enum tag name or enum body")?;
            if strict {
                self.strict_enums.insert(name.clone());
            }
            let enum_ty = self
                .enum_tags
                .entry(name.clone())
                .or_insert_with(|| CType::new(TypeKind::Enum(name.clone())));
            enum_ty.is_enum_strict = self.strict_enums.contains(&name);
            enum_ty.clone()
        } else if self.keyword("struct") {
            CType::new(TypeKind::Struct(self.name()?))
        } else if self.keyword("union") {
            CType::new(TypeKind::Union(self.name()?))
        } else {
            let Some(ty) = self
                .peek()
                .name
                .as_ref()
                .filter(|_| self.peek().kind == K::NAME)
                .and_then(|n| self.typedefs.get(n))
                .cloned()
            else {
                return Ok(None);
            };
            self.consume();
            ty
        };
        while self.keyword("const") {
            is_const = true
        }
        if bitflags || safe {
            if !ty.is_integer() || ty.is_enum() {
                return Err(format!(
                    "{} can only be applied to integer scalar types (u8/u16)",
                    if bitflags {
                        "__bitflags"
                    } else {
                        "__safe_index"
                    }
                ));
            }
        }
        ty.is_const |= is_const;
        ty.is_bitflags |= bitflags;
        ty.is_safe_index |= safe;
        if align > 0 || packed {
            ty.forced_align = align;
            ty.is_packed = packed;
        }
        while self.take(K::STAR) {
            let (mut ptr_const, mut ptr_restrict) = (false, false);
            loop {
                if self.keyword("const") {
                    ptr_const = true
                } else if self.keyword("__restrict") || self.keyword("restrict") {
                    ptr_restrict = true
                } else {
                    break;
                }
            }
            ty = CType::pointer(ty);
            ty.is_const = ptr_const;
            ty.is_restrict = ptr_restrict;
        }
        Ok(Some(ty))
    }
    pub(super) fn array_suffix(&mut self, ty: &mut CType) -> Result<()> {
        if self.take(K::LBRACKET) {
            let dimension = if self.take(K::RBRACKET) {
                Expr::new(t::EMPTY, Vec::new())
            } else {
                let e = self.expression()?;
                self.expect(K::RBRACKET)?;
                e
            };
            let src = ty.clone();
            *ty = CType {
                kind: TypeKind::ArrayExpression(Box::new(src.clone()), Box::new(dimension)),
                ..src
            };
        }
        Ok(())
    }
    pub(super) fn storage_region(&mut self) -> Result<Option<Region>> {
        let region = if self.keyword("__hram") {
            Region::HighMem
        } else if self.keyword("__oam") {
            Region::Oam
        } else if self.keyword("__wram") {
            Region::Ram
        } else if self.keyword("__wramx_bank") {
            self.expect(K::LPAREN)?;
            let bank = self.integer()?;
            self.expect(K::RPAREN)?;
            if !(1..=7).contains(&bank) {
                return Err("__wramx_bank: bank must be 1..7".into());
            }
            Region::WramX(bank as u8)
        } else if self.keyword("__wramx") {
            Region::WramX(0)
        } else if self.keyword("__prg_rom") {
            Region::Rom
        } else if self.keyword("__location") {
            self.expect(K::LPAREN)?;
            let address = self.integer()?;
            self.expect(K::RPAREN)?;
            Region::Fixed(address)
        } else {
            return Ok(None);
        };
        Ok(Some(region))
    }
    pub(super) fn apply_region(&self, region: Region) -> Region {
        if region == Region::WramX(0) && self.wram_bank > 0 {
            Region::WramX(self.wram_bank)
        } else {
            region
        }
    }
    pub(super) fn static_assert(&mut self) -> Result<Expr> {
        self.expect(K::LPAREN)?;
        let cond = self.expression()?;
        let mut message = String::new();
        if self.take(K::COMMA) {
            if self.peek().kind != K::STRING {
                return Err("expected a string literal message for static_assert".into());
            }
            message = self.consume().name.unwrap_or_default();
        }
        self.expect(K::RPAREN)?;
        self.expect(K::SEMICOLON)?;
        Ok(make!(self, t::STATIC_ASSERT, cond, message))
    }
}
