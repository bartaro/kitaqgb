use super::*;
use crate::asm::{AddressMode, Modifier, Operand};
impl Parser {
    fn recover_statement(&mut self, switch: bool) {
        let mut braces = 0;
        while self.peek().kind != K::EOF {
            if braces == 0
                && (self.peek().kind == K::RBRACE
                    || (switch && matches!(self.peek().name.as_deref(), Some("case" | "default"))))
            {
                return;
            }
            match self.consume().kind {
                K::SEMICOLON => return,
                K::LBRACE => braces += 1,
                K::RBRACE => braces -= 1,
                _ => {}
            }
        }
    }
    pub(super) fn block_contents(&mut self) -> Result<Vec<Expr>> {
        let mut parts = Vec::new();
        while !self.take(K::RBRACE) {
            if self.peek().kind == K::EOF {
                self.record_error("expected }, got EOF".into());
                break;
            }
            match self.statement(true) {
                Ok(stmt) => parts.push(stmt),
                Err(e) => {
                    self.record_error(e);
                    self.recover_statement(false);
                }
            }
        }
        Ok(parts)
    }
    fn block(&mut self) -> Result<Expr> {
        if self.take(K::LBRACE) {
            self.push_scope();
            let result = self.block_contents();
            self.pop_scope();
            Ok(self.sequence(result?))
        } else {
            self.statement(true)
        }
    }
    fn long_statement(allow: bool) -> Result<()> {
        if allow {
            Ok(())
        } else {
            Err("statement is not allowed in for initializer".into())
        }
    }
    pub(super) fn statement(&mut self, allow_long: bool) -> Result<Expr> {
        if let Some(region) = self.storage_region()? {
            return Err(if matches!(region,Region::WramX(bank)if bank>0) {
                "banked WRAM is allowed only for globals/file-statics in MVP"
            } else {
                "memory region qualifiers are only allowed on global variables"
            }
            .into());
        }
        let mut local_static = false;
        while self.keyword("static") {
            local_static = true
        }
        if self.keyword("static_assert") || self.keyword("_Static_assert") {
            Self::long_statement(allow_long)?;
            return self.static_assert();
        }
        if self.keyword("constexpr") {
            return Err("constexpr declarations are currently supported only at file scope".into());
        }
        let range = self.range_attribute()?;
        if self.keyword("__unsafe") {
            let inner = self.block()?;
            return Ok(make!(self, t::UNSAFE, inner));
        }
        if let Some(ty) = self.try_type()? {
            return self.local_declaration(ty, range, local_static);
        }
        if local_static {
            return Err("expected a local declaration after 'static'".into());
        }
        if self.keyword("if") {
            Self::long_statement(allow_long)?;
            let mut args = Vec::new();
            self.expect(K::LPAREN)?;
            args.push(Arg::from(self.expression()?));
            self.expect(K::RPAREN)?;
            args.push(Arg::from(self.block()?));
            while self.keyword("else") {
                if self.keyword("if") {
                    self.expect(K::LPAREN)?;
                    args.push(Arg::from(self.expression()?));
                    self.expect(K::RPAREN)?;
                    args.push(Arg::from(self.block()?));
                } else {
                    args.push(Arg::from(make!(self, t::INTEGER, 1)));
                    args.push(Arg::from(self.block()?));
                    break;
                }
            }
            return Ok(Expr::new(t::IF, args).with_source(self.source.clone()));
        }
        if self.keyword("for") {
            Self::long_statement(allow_long)?;
            self.push_scope();
            let result = (|| {
                self.expect(K::LPAREN)?;
                let init = self.statement(false)?;
                let test = self.expression()?;
                self.expect(K::SEMICOLON)?;
                let induct = self.expression()?;
                self.expect(K::RPAREN)?;
                let body = self.block()?;
                Ok(make!(self, t::FOR, init, test, induct, body))
            })();
            self.pop_scope();
            return result;
        }
        if self.keyword("while") {
            Self::long_statement(allow_long)?;
            self.expect(K::LPAREN)?;
            let test = self.expression()?;
            self.expect(K::RPAREN)?;
            let body = self.block()?;
            return Ok(make!(
                self,
                t::FOR,
                make!(self, t::EMPTY),
                test,
                make!(self, t::EMPTY),
                body
            ));
        }
        if self.keyword("do") {
            Self::long_statement(allow_long)?;
            let body = self.block()?;
            if !self.keyword("while") {
                return Err("expected 'while' after do-statement body".into());
            }
            self.expect(K::LPAREN)?;
            let test = self.expression()?;
            self.expect(K::RPAREN)?;
            self.expect(K::SEMICOLON)?;
            return Ok(make!(self, t::DO_WHILE, body, test));
        }
        for (word, tag) in [("continue", t::CONTINUE), ("break", t::BREAK)] {
            if self.keyword(word) {
                Self::long_statement(allow_long)?;
                self.expect(K::SEMICOLON)?;
                return Ok(make!(self, tag));
            }
        }
        if self.keyword("fallthrough") || self.keyword("__fallthrough") {
            Self::long_statement(allow_long)?;
            if self.switch_depth == 0 {
                return Err("'fallthrough' is only valid inside switch case blocks".into());
            }
            self.expect(K::SEMICOLON)?;
            return Ok(make!(self, t::FALLTHROUGH));
        }
        if self.keyword("return") {
            Self::long_statement(allow_long)?;
            if self.take(K::SEMICOLON) {
                return Ok(make!(self, t::RETURN));
            }
            let value = self.expression()?;
            self.expect(K::SEMICOLON)?;
            return Ok(make!(self, t::RETURN, value));
        }
        if self.keyword("goto") {
            let label = self.name()?;
            self.expect(K::SEMICOLON)?;
            return Ok(make!(self, t::JUMP, label));
        }
        if self.keyword("__asm") {
            Self::long_statement(allow_long)?;
            return self.assembly_block();
        }
        if self.peek().kind == K::NAME && self.ahead(1).kind == K::COLON {
            let label = self.name()?;
            self.expect(K::COLON)?;
            return Ok(make!(self, t::LABEL, label));
        }
        if self.keyword("switch") {
            Self::long_statement(allow_long)?;
            return self.switch_statement();
        }
        if self.peek().kind == K::LBRACE {
            Self::long_statement(allow_long)?;
            return self.block();
        }
        let mut parts = vec![self.expression()?];
        while self.take(K::COMMA) {
            parts.push(self.expression()?)
        }
        self.expect(K::SEMICOLON)?;
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            self.sequence(parts)
        })
    }
    fn local_declaration(
        &mut self,
        ty: CType,
        range: Option<Expr>,
        is_static: bool,
    ) -> Result<Expr> {
        let mut declarations = Vec::new();
        loop {
            let mut var_ty = ty.clone();
            let source_name = self.name()?;
            let name = if is_static {
                let function = self
                    .function_name
                    .as_ref()
                    .ok_or("function-local 'static' is only supported inside function bodies")?;
                let name = format!(
                    "__kq_fstatic_{}_{}_{}",
                    self.static_id, function, source_name
                );
                self.static_id += 1;
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(source_name, name.clone());
                }
                name
            } else {
                self.local_name(&source_name)?
            };
            self.array_suffix(&mut var_ty)?;
            let mut initializers = Vec::new();
            if self.take(K::EQUAL) {
                let target = make!(self, t::NAME, name.clone());
                self.local_initializer(&mut var_ty, target, &mut initializers)?;
            }
            if !is_static {
                let decl = if let Some(range) = &range {
                    make!(self, t::VARIABLE, var_ty, name, range.clone())
                } else {
                    make!(self, t::VARIABLE, var_ty, name)
                };
                if initializers.is_empty() {
                    declarations.push(decl)
                } else {
                    let mut seq = vec![decl];
                    seq.extend(initializers);
                    declarations.push(self.sequence(seq));
                }
            } else {
                self.function_statics.push(make!(
                    self,
                    t::STATIC,
                    make!(self, t::VARIABLE, Region::Ram, var_ty, name.clone())
                ));
                if !initializers.is_empty() {
                    let guard = format!("{name}__init");
                    self.function_statics.push(make!(
                        self,
                        t::STATIC,
                        make!(
                            self,
                            t::VARIABLE,
                            Region::Ram,
                            CType::simple(Simple::UInt8),
                            guard.clone()
                        )
                    ));
                    initializers.push(make!(
                        self,
                        t::ASSIGN,
                        make!(self, t::NAME, guard.clone()),
                        make!(self, t::INTEGER, 1)
                    ));
                    let cond = make!(
                        self,
                        t::EQUAL,
                        make!(self, t::NAME, guard),
                        make!(self, t::INTEGER, 0)
                    );
                    declarations.push(make!(self, t::IF, cond, self.sequence(initializers)));
                }
            }
            if !self.take(K::COMMA) {
                break;
            }
        }
        self.expect(K::SEMICOLON)?;
        Ok(match declarations.len() {
            0 => make!(self, t::EMPTY),
            1 => declarations.remove(0),
            _ => self.sequence(declarations),
        })
    }
    fn assembly_operand(&mut self, mode: AddressMode) -> Result<Operand> {
        let modifier = if self.take(K::LESS_THAN) {
            Modifier::LowByte
        } else if self.take(K::GREATER_THAN) {
            Modifier::HighByte
        } else {
            Modifier::None
        };
        let bracket = mode == AddressMode::Absolute && self.take(K::LBRACKET);
        let (base, offset) = if let Some(n) = self.signed_integer()? {
            (None, n)
        } else if self.peek().kind == K::NAME {
            let name = self.name()?;
            let offset = if self.take(K::PLUS) {
                self.integer()?
            } else {
                0
            };
            (Some(name), offset)
        } else {
            return Err("expected an operand".into());
        };
        if bracket {
            self.expect(K::RBRACKET)?
        }
        Ok(Operand {
            base,
            offset,
            mode,
            modifier,
            comment: None,
        })
    }
    fn assembly_block(&mut self) -> Result<Expr> {
        while self.take(K::NEWLINE) {}
        self.expect(K::LBRACE)?;
        let mut parts = Vec::new();
        while !self.take(K::RBRACE) {
            while self.take(K::NEWLINE) || self.take(K::SEMICOLON) {}
            if self.peek().kind == K::RBRACE {
                continue;
            }
            let symbol = self.name()?;
            if self.take(K::COLON) {
                parts.push(Expr::new(t::LABEL, vec![Arg::from(symbol)]));
                continue;
            }
            let operand = if self.take(K::NUMBER_SIGN) {
                self.assembly_operand(AddressMode::Immediate)?
            } else if self.take(K::PLUS) {
                self.assembly_operand(AddressMode::Relative)?
            } else if self.take(K::LPAREN) {
                let mut operand = self.assembly_operand(AddressMode::Indirect)?;
                if self.take(K::RPAREN) {
                    if self.take(K::COMMA) {
                        if !self.keyword("Y") {
                            return Err("expected 'Y'".into());
                        }
                        operand.mode = AddressMode::IndirectY;
                    }
                } else if self.take(K::COMMA) {
                    if !self.keyword("X") {
                        return Err("expected 'X'".into());
                    }
                    self.expect(K::RPAREN)?;
                    operand.mode = AddressMode::IndirectX;
                } else {
                    return Err("expected (zp,X) or (zp),Y operand".into());
                }
                operand
            } else if matches!(self.peek().kind, K::NEWLINE | K::SEMICOLON | K::RBRACE) {
                Operand::implicit()
            } else {
                let mut operand = self.assembly_operand(AddressMode::Absolute)?;
                if self.take(K::COMMA) {
                    operand.mode = if self.keyword("X") {
                        AddressMode::AbsoluteX
                    } else if self.keyword("Y") {
                        AddressMode::AbsoluteY
                    } else {
                        return Err("expected 'X' or 'Y'".into());
                    };
                }
                operand
            };
            parts.push(Expr::asm(&symbol, operand));
        }
        Ok(self.sequence(parts))
    }
    fn switch_statement(&mut self) -> Result<Expr> {
        self.expect(K::LPAREN)?;
        let test = self.expression()?;
        self.expect(K::RPAREN)?;
        self.expect(K::LBRACE)?;
        self.switch_depth += 1;
        let result = (|| {
            let mut cases = Vec::new();
            let mut default = None;
            while !self.take(K::RBRACE) {
                let value = if self.keyword("case") {
                    Some(self.expression()?)
                } else if self.keyword("default") {
                    None
                } else {
                    return Err("expected 'case' or 'default' inside switch".into());
                };
                self.expect(K::COLON)?;
                let mut statements = Vec::new();
                while !matches!(self.peek().kind, K::RBRACE | K::EOF)
                    && !matches!(self.peek().name.as_deref(), Some("case" | "default"))
                {
                    match self.statement(true) {
                        Ok(stmt) => statements.push(stmt),
                        Err(e) => {
                            self.record_error(e);
                            self.recover_statement(true);
                        }
                    }
                }
                let body = self.sequence(statements);
                if let Some(value) = value {
                    cases.push(make!(self, t::CASE, value, body))
                } else {
                    default = Some(body)
                }
            }
            let default = default.unwrap_or_else(|| make!(self, t::EMPTY));
            self.validate_fallthrough(&cases, &default)?;
            Ok(make!(self, t::SWITCH, test, cases, default))
        })();
        self.switch_depth -= 1;
        result
    }
    fn body_statements(body: &Expr) -> Vec<&Expr> {
        if body.is(t::EMPTY) {
            Vec::new()
        } else if body.is(t::SEQUENCE) {
            body.args
                .iter()
                .skip(1)
                .filter_map(|a| {
                    if let Arg::Expr(e) = a {
                        Some(e.as_ref())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![body]
        }
    }
    fn terminates(stmt: &Expr) -> bool {
        if matches!(stmt.tag(), Some(t::BREAK | t::RETURN | t::JUMP)) {
            return true;
        }
        if stmt.is(t::SEQUENCE) {
            return Self::body_statements(stmt)
                .last()
                .is_some_and(|e| Self::terminates(e));
        }
        if stmt.is(t::IF) {
            let parts = Self::body_statements(&Expr {
                args: std::iter::once(Arg::from(t::SEQUENCE))
                    .chain(stmt.args.iter().skip(1).cloned())
                    .collect(),
                source: stmt.source.clone(),
            })
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
            if parts.len() < 2 || parts.len() % 2 != 0 {
                return false;
            }
            let last = &parts[parts.len() - 2];
            return last.is(t::INTEGER)
                && last.int(1).is_some_and(|n| n != 0)
                && parts.chunks(2).all(|p| Self::terminates(&p[1]));
        }
        false
    }
    fn validate_fallthrough(&mut self, cases: &[Expr], default: &Expr) -> Result<()> {
        let has_default = !default.is(t::EMPTY);
        for (index, case) in cases.iter().enumerate() {
            let Some(Arg::Expr(body)) = case.args.get(2) else {
                continue;
            };
            let statements = Self::body_statements(body);
            let Some(last) = statements.last() else {
                continue;
            };
            let mut explicit = false;
            for (i, stmt) in statements.iter().enumerate() {
                if stmt.is(t::FALLTHROUGH) {
                    explicit = true;
                    if i + 1 != statements.len() {
                        return Err(
                            "'fallthrough' must be the last statement in a case block".into()
                        );
                    }
                }
            }
            let target = index + 1 < cases.len() || has_default;
            let message = if explicit && !target {
                Some("'fallthrough' has no target case/default")
            } else if !explicit && target && !Self::terminates(last) {
                Some(
                    "implicit fallthrough in switch case; use 'fallthrough;' to make intent explicit",
                )
            } else {
                None
            };
            if let Some(message) = message {
                self.diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    last.source.clone(),
                    message.into(),
                    0,
                ));
            }
        }
        if has_default {
            for stmt in Self::body_statements(default) {
                if stmt.is(t::FALLTHROUGH) {
                    self.diagnostics.push(Diagnostic::new(
                        Severity::Warning,
                        stmt.source.clone(),
                        "'fallthrough' inside default has no target".into(),
                        0,
                    ));
                }
            }
        }
        Ok(())
    }
}
