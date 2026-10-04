//! Declaration and expression IR with lexical name scopes and persistent placement pragmas.
use crate::{
    asm::Region,
    ctype::{CType, Field, Kind as TypeKind, Simple},
    expr::{Arg, Expr, Position},
    tags as t,
    token::{Kind as K, Token},
    tokenizer::{self, Diagnostic, Severity},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
macro_rules! make {($p:expr,$tag:expr $(,$a:expr)* $(,)?)=>{{let args=vec![$(Arg::from($a)),*];Expr::new($tag,args).with_source($p.source.clone())}}}
mod declarations;
mod expressions;
mod initializers;
mod statements;
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug)]
#[allow(dead_code)] // Retain aggregate layout metadata for later lowering/code generation.
struct Aggregate {
    is_union: bool,
    fields: Vec<Field>,
    packed: bool,
    align: i32,
}
#[derive(Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
    source: Position,
    typedefs: BTreeMap<String, CType>,
    enum_tags: BTreeMap<String, CType>,
    strict_enums: BTreeSet<String>,
    aggregates: BTreeMap<String, Aggregate>,
    bank: i32,
    fixed_bank: Option<i32>,
    fixed_order: Option<i32>,
    region: Region,
    wram_bank: u8,
    align: i32,
    section: Option<String>,
    cgb_only: bool,
    static_symbols: BTreeMap<String, BTreeMap<String, String>>,
    static_id: usize,
    local_id: usize,
    scopes: Vec<BTreeMap<String, String>>,
    used_locals: BTreeSet<String>,
    function_name: Option<String>,
    function_statics: Vec<Expr>,
    switch_depth: usize,
    string_pool: BTreeMap<String, String>,
    string_declarations: Vec<Expr>,
    pending: Vec<Expr>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Clone, Debug)]
pub struct Output {
    pub tree: Expr,
    pub preprocessing: tokenizer::Output,
    pub diagnostics: Vec<Diagnostic>,
}
pub fn parse_files(files: &[PathBuf], includes: &[PathBuf], cgb_only: bool) -> Output {
    let preprocessing = tokenizer::tokenize_files(files, includes);
    let mut parser = Parser::new(
        preprocessing.tokens.clone(),
        preprocessing.eof.clone(),
        cgb_only || preprocessing.header.cgb == Some(0xc0),
    );
    parser.predeclare();
    let tree = parser.parse_all();
    let mut diagnostics = preprocessing.diagnostics.clone();
    diagnostics.extend(parser.diagnostics);
    let tree = crate::palettes::inject(tree, &preprocessing.palettes, &mut diagnostics);
    Output {
        tree,
        preprocessing,
        diagnostics,
    }
}
impl Parser {
    pub fn new(mut tokens: Vec<Token>, eof: Position, cgb_only: bool) -> Self {
        tokens.push(Token::new(K::EOF, eof));
        let typedefs = [
            ("s8", Simple::Int8),
            ("int8_t", Simple::Int8),
            ("s16", Simple::Int16),
            ("int16_t", Simple::Int16),
        ]
        .into_iter()
        .map(|(name, ty)| (name.into(), CType::simple(ty)))
        .collect();
        Self {
            tokens,
            index: 0,
            source: Position::default(),
            typedefs,
            enum_tags: BTreeMap::new(),
            strict_enums: BTreeSet::new(),
            aggregates: BTreeMap::new(),
            bank: 1,
            fixed_bank: None,
            fixed_order: None,
            region: Region::Ram,
            wram_bank: 0,
            align: 0,
            section: None,
            cgb_only,
            static_symbols: BTreeMap::new(),
            static_id: 0,
            local_id: 0,
            scopes: Vec::new(),
            used_locals: BTreeSet::new(),
            function_name: None,
            function_statics: Vec::new(),
            switch_depth: 0,
            string_pool: BTreeMap::new(),
            string_declarations: Vec::new(),
            pending: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
    fn peek(&self) -> &Token {
        self.tokens
            .get(self.index)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }
    fn ahead(&self, n: usize) -> &Token {
        self.tokens
            .get(self.index + n)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }
    fn consume(&mut self) -> Token {
        let token = self.peek().clone();
        self.source = token.position.clone();
        if self.index < self.tokens.len() {
            self.index += 1
        }
        token
    }
    fn take(&mut self, kind: K) -> bool {
        if self.peek().kind == kind {
            self.consume();
            true
        } else {
            false
        }
    }
    fn keyword(&mut self, name: &str) -> bool {
        if self.peek().kind == K::NAME && self.peek().name.as_deref() == Some(name) {
            self.consume();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, kind: K) -> Result<()> {
        if self.take(kind) {
            Ok(())
        } else {
            Err(format!(
                "expected {}, got {}",
                kind.spelling(),
                self.peek().show()
            ))
        }
    }
    fn name(&mut self) -> Result<String> {
        if self.peek().kind == K::NAME {
            Ok(self.consume().name.unwrap_or_default())
        } else {
            Err("expected a name".into())
        }
    }
    fn signed_integer(&mut self) -> Result<Option<i32>> {
        let sign = self.peek().kind;
        let next = self.ahead(1);
        if matches!(sign, K::PLUS | K::MINUS) && next.kind == K::INT {
            let value = if sign == K::MINUS {
                -i64::from(next.integer)
            } else {
                i64::from(next.integer)
            };
            let value = i32::try_from(value).map_err(|_| "integer literal out of range")?;
            self.consume();
            self.consume();
            return Ok(Some(value));
        }
        if self.peek().kind == K::INT {
            Ok(Some(self.consume().integer))
        } else {
            Ok(None)
        }
    }
    fn integer(&mut self) -> Result<i32> {
        self.signed_integer()?
            .ok_or_else(|| "expected an integer".into())
    }
    fn sequence(&self, items: Vec<Expr>) -> Expr {
        Expr::new(t::SEQUENCE, items.into_iter().map(Arg::from).collect())
            .with_source(self.source.clone())
    }
    fn warn(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(
            Severity::Warning,
            self.source.clone(),
            message.into(),
            0,
        ));
    }
    fn record_error(&mut self, message: String) {
        self.diagnostics.push(Diagnostic::new(
            Severity::Error,
            self.peek().position.clone(),
            message,
            1000,
        ));
    }
    fn push_scope(&mut self) {
        if self.scopes.is_empty() {
            self.used_locals.clear()
        }
        self.scopes.push(BTreeMap::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    fn local_name(&mut self, name: &str) -> Result<String> {
        if self.scopes.is_empty() {
            return Ok(name.into());
        }
        if self.scopes.last().unwrap().contains_key(name) {
            return Err(format!(
                "duplicate declaration in the same local scope: {name}"
            ));
        }
        let mut resolved = name.to_owned();
        while !self.used_locals.insert(resolved.clone()) {
            self.local_id += 1;
            resolved = format!("__kq_local_{}_{name}", self.local_id);
        }
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.into(), resolved.clone());
        Ok(resolved)
    }
    fn static_name(&mut self, file: &str, name: &str) -> String {
        let map = self.static_symbols.entry(file.to_lowercase()).or_default();
        if let Some(n) = map.get(name) {
            return n.clone();
        }
        let resolved = format!("__kq_static_{}_{name}", self.static_id);
        self.static_id += 1;
        map.insert(name.into(), resolved.clone());
        resolved
    }
    fn resolve_name(&self, name: &str) -> String {
        for scope in self.scopes.iter().rev() {
            if let Some(n) = scope.get(name) {
                return n.clone();
            }
        }
        self.static_symbols
            .get(&self.source.filename.to_lowercase())
            .and_then(|m| m.get(name))
            .cloned()
            .unwrap_or_else(|| name.into())
    }
    fn anonymous_name(&self, prefix: &str) -> String {
        let file = self
            .source
            .filename
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        format!(
            "{prefix}_{file}_{}_{}",
            self.source.line + 1,
            self.source.column + 1
        )
    }
    fn wrap_pragmas(&self, mut decl: Expr) -> Expr {
        let tag = decl.tag().unwrap_or("").to_owned();
        let rom = matches!(
            tag.as_str(),
            t::FUNCTION | t::INLINE_FUNCTION | t::FUNCTION_DECL | t::READONLY_DATA
        );
        let eligible = rom || matches!(tag.as_str(), t::VARIABLE | t::EXTERN_VARIABLE);
        if rom {
            if let Some(n) = self.fixed_bank {
                decl = make!(self, t::FIXED_BANK, n, decl)
            }
            if let Some(n) = self.fixed_order {
                decl = make!(self, t::FIXED_ORDER, n, decl)
            }
            decl = make!(self, t::BANK, self.bank, decl);
        }
        if eligible {
            if let Some(s) = &self.section {
                if !s.is_empty() {
                    decl = make!(self, t::DECL_SECTION, s.clone(), decl)
                }
            }
            if self.align > 1 {
                decl = make!(self, t::DECL_ALIGN, self.align, decl)
            }
        }
        decl
    }
    fn append_top(items: &mut Vec<Expr>, decl: Expr) {
        if decl.is(t::EMPTY) {
            return;
        }
        if decl.is(t::SEQUENCE) {
            for a in decl.args.into_iter().skip(1) {
                if let Arg::Expr(e) = a {
                    Self::append_top(items, *e)
                }
            }
        } else {
            items.push(decl)
        }
    }
    fn recover_top(&mut self) {
        while self.peek().kind != K::EOF {
            if matches!(
                self.peek().kind,
                K::PRAGMA_BANK
                    | K::PRAGMA_FIXED_BANK
                    | K::PRAGMA_FIXED_ORDER
                    | K::PRAGMA_REGION
                    | K::PRAGMA_WRAMX_BANK
                    | K::PRAGMA_ALIGN
                    | K::PRAGMA_SECTION
            ) {
                return;
            }
            if matches!(self.consume().kind, K::SEMICOLON | K::RBRACE) {
                return;
            }
        }
    }
    fn parse_all(&mut self) -> Expr {
        let mut declarations = Vec::new();
        while !self.take(K::EOF) {
            let kind = self.peek().kind;
            if matches!(
                kind,
                K::PRAGMA_BANK
                    | K::PRAGMA_FIXED_BANK
                    | K::PRAGMA_FIXED_ORDER
                    | K::PRAGMA_REGION
                    | K::PRAGMA_WRAMX_BANK
                    | K::PRAGMA_ALIGN
                    | K::PRAGMA_SECTION
            ) {
                let token = self.consume();
                match kind {
                    K::PRAGMA_BANK => self.bank = token.integer,
                    K::PRAGMA_FIXED_BANK => {
                        self.fixed_bank = (token.integer >= 0).then_some(token.integer)
                    }
                    K::PRAGMA_FIXED_ORDER => {
                        self.fixed_order = (token.integer >= 0).then_some(token.integer)
                    }
                    K::PRAGMA_REGION => {
                        self.region = match token.integer {
                            0 => Region::HighMem,
                            3 => Region::Wram0,
                            4 => Region::WramX(0),
                            _ => Region::Ram,
                        }
                    }
                    K::PRAGMA_WRAMX_BANK => {
                        if (1..=7).contains(&token.integer) {
                            self.wram_bank = token.integer as u8
                        } else {
                            self.record_error("#pragma wramx_bank: bank must be 1..7".into())
                        }
                    }
                    K::PRAGMA_ALIGN => self.align = token.integer,
                    K::PRAGMA_SECTION => self.section = token.name,
                    _ => unreachable!(),
                }
                continue;
            }
            self.pending.clear();
            match self.declaration() {
                Ok(decl) => {
                    for pending in std::mem::take(&mut self.pending) {
                        Self::append_top(&mut declarations, pending)
                    }
                    Self::append_top(&mut declarations, decl)
                }
                Err(e) => {
                    self.record_error(e);
                    self.recover_top();
                }
            }
        }
        declarations.append(&mut self.string_declarations);
        self.sequence(declarations)
    }
    fn predeclare(&mut self) {
        let mut slices = Vec::new();
        let mut i = 0;
        let mut braces = 0;
        while i < self.tokens.len() {
            let token = &self.tokens[i];
            match token.kind {
                K::LBRACE => braces += 1,
                K::RBRACE => braces = (braces - 1).max(0),
                _ => {}
            }
            if braces == 0 && token.kind == K::NAME && token.name.as_deref() == Some("typedef") {
                let start = i;
                let (mut b, mut p, mut s) = (0i32, 0i32, 0i32);
                while i < self.tokens.len() {
                    match self.tokens[i].kind {
                        K::LBRACE => b += 1,
                        K::RBRACE => b = (b - 1).max(0),
                        K::LPAREN => p += 1,
                        K::RPAREN => p = (p - 1).max(0),
                        K::LBRACKET => s += 1,
                        K::RBRACKET => s = (s - 1).max(0),
                        _ => {}
                    }
                    if self.tokens[i].kind == K::SEMICOLON && b == 0 && p == 0 && s == 0 {
                        break;
                    }
                    i += 1;
                }
                if i < self.tokens.len() {
                    slices.push(self.tokens[start..=i].to_vec());
                }
            }
            i += 1;
        }
        let mut done = BTreeSet::new();
        loop {
            let mut changed = false;
            for (i, slice) in slices.iter().enumerate() {
                if done.contains(&i) {
                    continue;
                }
                let mut probe = self.clone();
                probe.tokens = slice.clone();
                probe
                    .tokens
                    .push(Token::new(K::EOF, slice.last().unwrap().position.clone()));
                probe.index = 0;
                probe.source = Position::default();
                if probe.declaration().is_ok() && probe.take(K::EOF) {
                    self.typedefs = probe.typedefs;
                    self.enum_tags = probe.enum_tags;
                    self.strict_enums = probe.strict_enums;
                    self.aggregates = probe.aggregates;
                    done.insert(i);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }
}
