use super::*;
impl Parser {
    fn nodiscard(&mut self) -> bool {
        if self.peek().kind == K::LBRACKET
            && self.ahead(1).kind == K::LBRACKET
            && self.ahead(2).kind == K::NAME
            && self.ahead(2).name.as_deref() == Some("nodiscard")
            && self.ahead(3).kind == K::RBRACKET
            && self.ahead(4).kind == K::RBRACKET
        {
            for _ in 0..5 {
                self.consume();
            }
            true
        } else {
            false
        }
    }
    pub(super) fn declaration(&mut self) -> Result<Expr> {
        let file = self.peek().position.filename.clone();
        let (mut is_static, mut is_extern, mut is_unsafe) = (false, false, false);
        while self.keyword("static") {
            is_static = true
        }
        while self.keyword("extern") {
            is_extern = true
        }
        if is_static && is_extern {
            self.warn("warning: 'static' with 'extern' is treated as 'static'");
            is_extern = false
        }
        let inline = self.keyword("inline") || self.keyword("__forceinline");
        while self.keyword("__unsafe") {
            is_unsafe = true
        }
        let (mut must_check, mut range, mut align) = (false, None, 0);
        loop {
            if self.keyword("__must_check") || self.nodiscard() {
                must_check = true
            } else if let Some(r) = self.range_attribute()? {
                range = Some(r)
            } else if let Some(a) = self.aligned_attribute()? {
                align = a
            } else {
                break;
            }
        }
        if self.keyword("static_assert") || self.keyword("_Static_assert") {
            if is_static {
                self.warn("warning: 'static' has no effect on static_assert");
            }
            return self.static_assert();
        }
        if self.keyword("typedef") {
            if is_static {
                self.warn("warning: 'static' has no effect on typedef");
            }
            return self.typedef_declaration();
        }
        let constexpr = self.keyword("constexpr");
        let define = if constexpr {
            false
        } else {
            self.keyword("define")
        };
        if constexpr || define {
            if constexpr && is_extern {
                self.warn("warning: 'extern' on constexpr has no effect")
            }
            let mut ty = self.expect_type()?;
            if align > 0 && ty.forced_align == 0 {
                ty.forced_align = align
            }
            let source_name = self.name()?;
            let name = if is_static {
                self.static_name(&file, &source_name)
            } else {
                source_name
            };
            if constexpr {
                self.array_suffix(&mut ty)?;
                if ty.is_array() || ty.is_aggregate() || ty.is_function() {
                    return Err("constexpr currently supports only scalar and enum types".into());
                }
                ty.is_const = true;
            }
            self.expect(K::EQUAL)?;
            let value = self.expression()?;
            self.expect(K::SEMICOLON)?;
            let mut decl = make!(self, t::CONSTANT, ty, name, value);
            if constexpr {
                decl = self.wrap_pragmas(decl)
            }
            if is_static {
                decl = make!(self, t::STATIC, decl)
            }
            if constexpr && is_unsafe {
                decl = make!(self, t::UNSAFE, decl)
            }
            return Ok(decl);
        }
        let explicit = self.storage_region()?;
        let region = self.apply_region(explicit.clone().unwrap_or_else(|| self.region.clone()));
        let mut ty = self.expect_type()?;
        if align > 0 && ty.forced_align == 0 {
            ty.forced_align = align
        }
        if ty.is_enum() && self.take(K::SEMICOLON) {
            return Ok(self.sequence(Vec::new()));
        }
        if ty.is_aggregate() && self.take(K::LBRACE) {
            if is_static {
                self.warn("warning: 'static' has no effect on type declarations");
            }
            if explicit.is_some() {
                return Err("memory region qualifiers are only allowed on global variables".into());
            }
            let fields = self.aggregate_fields()?;
            self.expect(K::SEMICOLON)?;
            let (name, is_union) = match &ty.kind {
                TypeKind::Struct(n) => (n.clone(), false),
                TypeKind::Union(n) => (n.clone(), true),
                _ => unreachable!(),
            };
            self.aggregates.insert(
                name.clone(),
                Aggregate {
                    is_union,
                    fields: fields.clone(),
                    packed: ty.is_packed,
                    align: ty.forced_align,
                },
            );
            return Ok(make!(
                self,
                if is_union { t::UNION } else { t::STRUCT },
                name,
                fields,
                i32::from(ty.is_packed),
                ty.forced_align
            ));
        }
        let source_name = self.name()?;
        let mut name = if is_static {
            self.static_name(&file, &source_name)
        } else {
            source_name
        };
        let stackcall = name == "__stackcall";
        if stackcall {
            let source_name = self.name()?;
            name = if is_static {
                self.static_name(&file, &source_name)
            } else {
                source_name
            };
            if self.peek().kind != K::LPAREN {
                return Err("__stackcall is only allowed on function declarations".into());
            }
        }
        if self.take(K::LPAREN) {
            if explicit.is_some() {
                return Err("memory region qualifiers are only allowed on global variables".into());
            }
            if is_extern {
                self.warn("warning: 'extern' on function declarations has no effect");
            }
            let mut fields = Vec::new();
            if !self.take(K::RPAREN) {
                loop {
                    let ty = self.expect_type()?;
                    let name = self.name()?;
                    fields.push(Field {
                        ty,
                        name,
                        offset: 0,
                    });
                    if self.take(K::RPAREN) {
                        break;
                    }
                    self.expect(K::COMMA)?;
                }
            }
            let mut decl = if self.take(K::SEMICOLON) {
                self.wrap_pragmas(make!(
                    self,
                    t::FUNCTION_DECL,
                    ty,
                    name.clone(),
                    fields,
                    i32::from(must_check)
                ))
            } else {
                self.expect(K::LBRACE)?;
                let old_name = self.function_name.replace(name.clone());
                let old_statics = std::mem::take(&mut self.function_statics);
                self.push_scope();
                let result: Result<Expr> = (|| {
                    for field in &fields {
                        self.local_name(&field.name)?;
                    }
                    let statements = self.block_contents()?;
                    let body = self.sequence(statements);
                    Ok(self.wrap_pragmas(make!(
                        self,
                        if inline {
                            t::INLINE_FUNCTION
                        } else {
                            t::FUNCTION
                        },
                        ty,
                        name.clone(),
                        fields,
                        i32::from(must_check),
                        body
                    )))
                })();
                self.pop_scope();
                let statics = std::mem::replace(&mut self.function_statics, old_statics);
                self.function_name = old_name;
                let mut decl = result?;
                if is_static {
                    decl = make!(self, t::STATIC, decl)
                }
                if stackcall {
                    decl = make!(self, t::STACK_CALL, decl)
                }
                if is_unsafe {
                    decl = make!(self, t::UNSAFE, decl)
                }
                if !statics.is_empty() {
                    let mut all = statics;
                    all.push(decl);
                    return Ok(self.sequence(all));
                }
                return Ok(decl);
            };
            if is_static {
                decl = make!(self, t::STATIC, decl)
            }
            if stackcall {
                decl = make!(self, t::STACK_CALL, decl)
            }
            if is_unsafe {
                decl = make!(self, t::UNSAFE, decl)
            }
            return Ok(decl);
        }
        self.array_suffix(&mut ty)?;
        if matches!(region,Region::WramX(bank)if bank>1) && !self.cgb_only {
            return Err("banked WRAM requires #pragma rom_cgb cgb_only or --cgb=cgb_only".into());
        }
        let mut decl = if self.take(K::EQUAL) {
            if is_extern {
                self.warn("warning: 'extern' on a definition has no effect");
            }
            if ty.is_const && !ty.is_array() {
                let value = self.expression()?;
                self.expect(K::SEMICOLON)?;
                let mut decl = self.wrap_pragmas(make!(self, t::CONSTANT, ty, name, value));
                if is_static {
                    decl = make!(self, t::STATIC, decl)
                }
                if is_unsafe {
                    decl = make!(self, t::UNSAFE, decl)
                }
                return Ok(decl);
            }
            if region == Region::Rom || ty.is_const {
                let values = if ty.is_array() {
                    self.readonly_array(&mut ty, true)?
                } else {
                    vec![self.expression()?]
                };
                self.expect(K::SEMICOLON)?;
                self.wrap_pragmas(make!(self, t::READONLY_DATA, ty, name, values))
            } else {
                return Err("global variables cannot be initialized".into());
            }
        } else {
            if ty.is_const && !is_extern {
                return Err("const global must be initialized".into());
            }
            self.expect(K::SEMICOLON)?;
            let tag = if is_extern {
                t::EXTERN_VARIABLE
            } else {
                t::VARIABLE
            };
            let decl = if let Some(range) = range {
                make!(self, tag, region, ty, name, range)
            } else {
                make!(self, tag, region, ty, name)
            };
            self.wrap_pragmas(decl)
        };
        if is_static {
            decl = make!(self, t::STATIC, decl)
        }
        Ok(decl)
    }
    pub(super) fn aggregate_fields(&mut self) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        while !self.take(K::RBRACE) {
            let mut ty = self.expect_type()?;
            let name = self.name()?;
            self.array_suffix(&mut ty)?;
            fields.push(Field {
                ty: ty.clone(),
                name,
                offset: 0,
            });
            while self.take(K::COMMA) {
                fields.push(Field {
                    ty: ty.clone(),
                    name: self.name()?,
                    offset: 0,
                });
            }
            self.expect(K::SEMICOLON)?;
        }
        Ok(fields)
    }
    fn typedef_declaration(&mut self) -> Result<Expr> {
        if self.storage_region()?.is_some() {
            return Err("memory region qualifiers are not allowed on typedef".into());
        }
        let (mut packed, mut align) = (false, 0);
        loop {
            if self.keyword("__packed") {
                packed = true
            } else if let Some(a) = self.aligned_attribute()? {
                align = a
            } else {
                break;
            }
        }
        let is_union = self.keyword("union");
        let aggregate = is_union || self.keyword("struct");
        let mut extra = None;
        let mut ty = if aggregate {
            let mut name = if self.peek().kind == K::NAME {
                Some(self.name()?)
            } else {
                None
            };
            if self.take(K::LBRACE) {
                if name.is_none() {
                    name = Some(self.anonymous_name("__anon"))
                }
                let name = name.as_ref().unwrap().clone();
                let fields = self.aggregate_fields()?;
                self.aggregates.insert(
                    name.clone(),
                    Aggregate {
                        is_union,
                        fields: fields.clone(),
                        packed,
                        align,
                    },
                );
                extra = Some(make!(
                    self,
                    if is_union { t::UNION } else { t::STRUCT },
                    name,
                    fields,
                    i32::from(packed),
                    align
                ));
            } else {
                let name = name
                    .as_ref()
                    .ok_or("expected a struct/union tag name")?
                    .clone();
                extra = Some(make!(
                    self,
                    if is_union {
                        t::OPAQUE_UNION
                    } else {
                        t::OPAQUE_STRUCT
                    },
                    name
                ));
            }
            let name = name.unwrap();
            let mut ty = CType::new(if is_union {
                TypeKind::Union(name)
            } else {
                TypeKind::Struct(name)
            });
            ty.is_packed = packed;
            ty.forced_align = align;
            ty
        } else {
            self.expect_type()?
        };
        if self.take(K::LPAREN) {
            let mut depth = 0;
            while self.take(K::STAR) {
                depth += 1
            }
            if depth == 0 {
                return Err("expected '*' in function pointer typedef declarator".into());
            }
            let name = self.name()?;
            self.expect(K::RPAREN)?;
            self.expect(K::LPAREN)?;
            let mut params = Vec::new();
            if !self.take(K::RPAREN) {
                loop {
                    let mut param = self.expect_type()?;
                    if self.peek().kind == K::NAME {
                        self.consume();
                    }
                    self.array_suffix(&mut param)?;
                    params.push(param);
                    if self.take(K::RPAREN) {
                        break;
                    }
                    self.expect(K::COMMA)?;
                }
            }
            if params.len() == 1 && matches!(params[0].kind, TypeKind::Simple(Simple::Void)) {
                params.clear()
            }
            self.expect(K::SEMICOLON)?;
            ty = CType::function(ty, params);
            for _ in 0..depth {
                ty = CType::pointer(ty)
            }
            self.typedefs.insert(name, ty);
        } else {
            let name = self.name()?;
            self.array_suffix(&mut ty)?;
            self.expect(K::SEMICOLON)?;
            self.typedefs.insert(name, ty);
        }
        Ok(extra.unwrap_or_else(|| self.sequence(Vec::new())))
    }
}
