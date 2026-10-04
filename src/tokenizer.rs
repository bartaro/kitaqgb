//! UTF-16 source scanning and shared include/macro preprocessing.
use crate::{
    expr::Position,
    io, preprocessor, rom_header,
    token::{Kind, Token},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub has_position: bool,
    pub code: u16,
    pub severity: Severity,
    pub position: Position,
    pub message: String,
}
impl Diagnostic {
    pub fn new(severity: Severity, position: Position, message: String, fallback: u16) -> Self {
        Self {
            has_position: true,
            code: crate::diagnostics::infer(&message, fallback),
            severity,
            position,
            message,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Macro {
    pub parameters: Option<Vec<String>>,
    pub replacement: Vec<Token>,
}
#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub colors: [i32; 4],
    pub position: Position,
}
#[derive(Clone, Debug, Default)]
pub struct Output {
    pub tokens: Vec<Token>,
    pub eof: Position,
    pub dependencies: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    pub header: rom_header::Options,
    pub palettes: Vec<Palette>,
}
impl Output {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}
#[derive(Default)]
struct Context {
    macros: BTreeMap<String, Macro>,
    once: BTreeSet<String>,
    dependencies: BTreeMap<String, PathBuf>,
    stack: Vec<String>,
    include_dirs: Vec<PathBuf>,
    diagnostics: Vec<Diagnostic>,
    header: rom_header::Options,
    palettes: Vec<Palette>,
}
fn path_key(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}
fn full_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut result = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(part),
        }
    }
    Ok(result)
}
pub fn tokenize_files(files: &[PathBuf], include_dirs: &[PathBuf]) -> Output {
    let mut context = Context::default();
    for dir in include_dirs {
        if let Ok(dir) = full_path(dir) {
            if !context
                .include_dirs
                .iter()
                .any(|d| path_key(d) == path_key(&dir))
            {
                context.include_dirs.push(dir);
            }
        }
    }
    let mut tokens = Vec::new();
    let mut eof = Position::default();
    for file in files {
        let (t, end) = read_file(file, &mut context);
        tokens.extend(t);
        eof = end;
    }
    Output {
        tokens,
        eof,
        dependencies: context.dependencies.into_values().collect(),
        diagnostics: context.diagnostics,
        header: context.header,
        palettes: context.palettes,
    }
}
fn read_file(path: &Path, context: &mut Context) -> (Vec<Token>, Position) {
    let path = match full_path(path) {
        Ok(p) => p,
        Err(e) => {
            context
                .diagnostics
                .push(Diagnostic::new(Severity::Error, Position::default(), e, 0));
            return (Vec::new(), Position::default());
        }
    };
    let position = Position {
        filename: path.to_string_lossy().into(),
        line: 0,
        column: 0,
    };
    let key = path_key(&path);
    if !path.is_file() {
        context.diagnostics.push(Diagnostic::new(
            Severity::Error,
            position.clone(),
            format!("source file not found: {}", path.display()),
            0,
        ));
        return (Vec::new(), position);
    }
    if context.once.contains(&key) {
        return (Vec::new(), position);
    }
    if context.stack.contains(&key) {
        context.diagnostics.push(Diagnostic::new(
            Severity::Error,
            position.clone(),
            format!("include cycle detected: {}", path.display()),
            0,
        ));
        return (Vec::new(), position);
    }
    context
        .dependencies
        .entry(key.clone())
        .or_insert(path.clone());
    context.stack.push(key.clone());
    let input = match io::read_utf8(&path) {
        Ok(s) => s,
        Err(e) => {
            context.stack.pop();
            context
                .diagnostics
                .push(Diagnostic::new(Severity::Error, position.clone(), e, 0));
            return (Vec::new(), position);
        }
    };
    let mut scanner = Scanner::new(&input, position, context, false);
    let tokens = scanner.run();
    let end = scanner.position.clone();
    let once = scanner.once;
    drop(scanner);
    if once {
        context.once.insert(key);
    }
    context.stack.pop();
    (tokens, end)
}
#[derive(Clone)]
struct Conditional {
    parent: bool,
    active: bool,
    taken: bool,
    seen_else: bool,
    start: Position,
}
struct Scanner<'a> {
    input: Vec<u16>,
    next: usize,
    position: Position,
    assembly: bool,
    once: bool,
    disable_macros: bool,
    conditionals: Vec<Conditional>,
    included: Vec<Token>,
    context: &'a mut Context,
}
fn name_char(c: u16) -> bool {
    matches!(c,36|95|48..=57|65..=90|97..=122)
}
fn name_start(c: u16) -> bool {
    name_char(c) && !(48..=57).contains(&c)
}
fn char16(c: u16) -> char {
    char::from_u32(u32::from(c)).unwrap_or('\u{fffd}')
}
impl<'a> Scanner<'a> {
    fn new(
        input: &str,
        position: Position,
        context: &'a mut Context,
        disable_macros: bool,
    ) -> Self {
        Self {
            input: input.encode_utf16().collect(),
            next: 0,
            position,
            assembly: false,
            once: false,
            disable_macros,
            conditionals: Vec::new(),
            included: Vec::new(),
            context,
        }
    }
    fn peek(&self) -> u16 {
        self.input.get(self.next).copied().unwrap_or(0)
    }
    fn fetch(&mut self) {
        if self.next < self.input.len() {
            self.next += 1;
        }
        if self.peek() == 10 {
            self.position.line += 1;
            self.position.column = 0;
        } else {
            self.position.column += 1;
        }
    }
    fn take(&mut self, c: u16) -> bool {
        if self.peek() == c {
            self.fetch();
            true
        } else {
            false
        }
    }
    // Original string overload intentionally advances without updating source coordinates.
    fn literal(&mut self, s: &str) -> bool {
        let chars = s.encode_utf16().collect::<Vec<_>>();
        if self.input.get(self.next..self.next + chars.len()) == Some(chars.as_slice()) {
            self.next += chars.len();
            true
        } else {
            false
        }
    }
    fn horizontal(&mut self) {
        while matches!(self.peek(), 32 | 9) {
            self.fetch();
        }
    }
    fn spaces(&mut self) {
        while matches!(self.peek(), 32 | 9 | 13) || (!self.assembly && self.peek() == 10) {
            self.fetch();
        }
    }
    fn line_end(&mut self) {
        while !matches!(self.peek(), 0 | 10) {
            self.fetch();
        }
    }
    fn raw_line(&mut self) -> String {
        let start = self.next;
        while !matches!(self.peek(), 0 | 10 | 13) {
            self.fetch();
        }
        String::from_utf16_lossy(&self.input[start..self.next])
    }
    fn identifier(&mut self) -> String {
        let start = self.next;
        while name_char(self.peek()) {
            self.fetch();
        }
        String::from_utf16_lossy(&self.input[start..self.next])
    }
    fn active(&self) -> bool {
        self.conditionals.last().is_none_or(|c| c.active)
    }
    fn diagnostic(&mut self, severity: Severity, position: &Position, message: impl Into<String>) {
        self.context.diagnostics.push(Diagnostic::new(
            severity,
            position.clone(),
            message.into(),
            0,
        ));
    }
    fn error(&mut self, position: &Position, message: impl Into<String>) {
        self.diagnostic(Severity::Error, position, message);
    }
    fn warning(&mut self, position: &Position, message: impl Into<String>) {
        self.diagnostic(Severity::Warning, position, message);
    }
    fn fragment(&mut self, input: &str, position: &Position) -> Vec<Token> {
        if input.trim().is_empty() {
            Vec::new()
        } else {
            Scanner::new(input, position.clone(), self.context, true).run()
        }
    }
    fn number(&mut self, text: &str, position: &Position) -> Option<i32> {
        let upper = text.to_ascii_uppercase();
        let first = upper.as_bytes().first().copied()?;
        if !first.is_ascii_digit() && first != b'$' {
            return None;
        }
        let (base, digits) = if let Some(s) = upper.strip_prefix("0X") {
            (16i32, s)
        } else if let Some(s) = upper.strip_prefix('$') {
            (16, s)
        } else if let Some(s) = upper.strip_prefix("0B") {
            (2, s)
        } else {
            (10, upper.as_str())
        };
        if digits.is_empty() {
            self.error(position, format!("number contains no digits: {text}"));
        }
        let mut value = 0i32;
        let mut invalid = false;
        for c in digits.chars() {
            let digit = c.to_digit(base as u32).map_or(-1, |v| v as i32);
            invalid |= digit < 0;
            value = value.wrapping_mul(base).wrapping_add(digit);
        }
        if invalid {
            self.error(
                position,
                format!("number contains invalid characters: {text}"),
            );
        }
        Some(value)
    }
    fn string_value(&mut self, allow_eol: bool) -> Option<String> {
        if !self.take(34) {
            return None;
        }
        let start = self.next;
        while self.peek() != 34 {
            if self.peek() == 0 || (!allow_eol && matches!(self.peek(), 10 | 13)) {
                return None;
            }
            self.fetch();
        }
        let value = String::from_utf16_lossy(&self.input[start..self.next]);
        self.fetch();
        Some(value)
    }
    fn signed_decimal(&mut self) -> Option<i32> {
        let neg = if self.take(43) { false } else { self.take(45) };
        let mut value = 0i32;
        let mut any = false;
        while char16(self.peek()).is_ascii_digit() {
            any = true;
            value = value
                .wrapping_mul(10)
                .wrapping_add(i32::from(self.peek() - 48));
            self.fetch();
        }
        any.then_some(if neg { value.wrapping_neg() } else { value })
    }
    fn unsigned_placement(&mut self) -> Option<i32> {
        let hex = self.peek() == 48 && matches!(self.input.get(self.next + 1), Some(120 | 88));
        if hex {
            self.fetch();
            self.fetch();
        }
        let mut value = 0i32;
        let mut any = false;
        while let Some(d) = char16(self.peek()).to_digit(if hex { 16 } else { 10 }) {
            any = true;
            value = value
                .wrapping_mul(if hex { 16 } else { 10 })
                .wrapping_add(d as i32);
            self.fetch();
        }
        any.then_some(value)
    }
    fn if_expression(&mut self, text: &str, position: &Position) -> i64 {
        let tokens = self.fragment(text, position);
        preprocessor::evaluate(
            &tokens,
            &self.context.macros,
            position,
            &mut self.context.diagnostics,
        )
    }
    fn conditional(&mut self, position: &Position) -> bool {
        let save = (self.next, self.position.clone());
        self.horizontal();
        let name = self.identifier();
        match name.as_str() {
            "if" | "ifdef" | "ifndef" => {
                let parent = self.active();
                self.horizontal();
                let selected = if name == "if" {
                    let text = self.raw_line();
                    parent && self.if_expression(&text, position) != 0
                } else {
                    let macro_name = self.identifier();
                    if macro_name.is_empty() {
                        self.warning(position, format!("malformed #{name} ignored"));
                        false
                    } else {
                        let defined = self.context.macros.contains_key(&macro_name);
                        if name == "ifdef" { defined } else { !defined }
                    }
                };
                let active = parent && selected;
                self.conditionals.push(Conditional {
                    parent,
                    active,
                    taken: active,
                    seen_else: false,
                    start: position.clone(),
                });
            }
            "elif" => {
                let Some(frame) = self.conditionals.last().cloned() else {
                    self.error(position, "unexpected #elif");
                    return true;
                };
                self.horizontal();
                let text = self.raw_line();
                if frame.seen_else {
                    self.error(position, "#elif after #else");
                    self.conditionals.last_mut().unwrap().active = false;
                } else {
                    let active =
                        frame.parent && !frame.taken && self.if_expression(&text, position) != 0;
                    let frame = self.conditionals.last_mut().unwrap();
                    frame.active = active;
                    frame.taken |= active;
                }
            }
            "else" => {
                let Some(frame) = self.conditionals.last().cloned() else {
                    self.error(position, "unexpected #else");
                    return true;
                };
                if frame.seen_else {
                    self.error(position, "duplicate #else");
                }
                let frame = self.conditionals.last_mut().unwrap();
                frame.seen_else = true;
                frame.active = frame.parent && !frame.taken;
                frame.taken |= frame.active;
            }
            "endif" => {
                if self.conditionals.pop().is_none() {
                    self.error(position, "unexpected #endif");
                }
            }
            _ => {
                self.next = save.0;
                self.position = save.1;
                return false;
            }
        }
        true
    }
    fn include(&mut self, position: &Position) {
        self.horizontal();
        let angle = self.take(60);
        let end = if angle {
            62
        } else if self.take(34) {
            34
        } else {
            self.warning(position, "preprocessor directives are ignored");
            return;
        };
        let start = self.next;
        while self.peek() != end && !matches!(self.peek(), 0 | 10 | 13) {
            self.fetch();
        }
        if self.peek() != end {
            self.warning(position, "preprocessor directives are ignored");
            return;
        }
        let name = String::from_utf16_lossy(&self.input[start..self.next]);
        self.fetch();
        let path = PathBuf::from(&name);
        let mut candidates = Vec::new();
        if path.is_absolute() {
            candidates.push(path)
        } else {
            if !angle {
                if let Some(parent) = Path::new(&self.position.filename).parent() {
                    candidates.push(parent.join(&path));
                }
            }
            candidates.extend(self.context.include_dirs.iter().map(|d| d.join(&path)));
            if let Ok(cwd) = std::env::current_dir() {
                candidates.push(cwd.join(&path));
            }
        }
        let Some(path) = candidates.into_iter().find(|p| p.is_file()) else {
            self.error(position, format!("include file not found: {name}"));
            return;
        };
        let (tokens, _) = read_file(&path, self.context);
        self.included.extend(tokens);
    }
    fn define(&mut self, position: &Position) {
        self.horizontal();
        if !name_start(self.peek()) {
            self.warning(position, "malformed #define ignored");
            return;
        }
        let name = self.identifier();
        let mut parameters = None;
        if self.take(40) {
            let mut names = Vec::new();
            self.horizontal();
            if !self.take(41) {
                loop {
                    self.horizontal();
                    if !name_start(self.peek()) {
                        self.warning(
                            position,
                            format!("malformed function-like #define ignored: {name}"),
                        );
                        return;
                    }
                    let param = self.identifier();
                    if names.contains(&param) {
                        self.warning(
                            position,
                            format!("malformed function-like #define ignored: {name}"),
                        );
                        return;
                    }
                    names.push(param);
                    self.horizontal();
                    if self.take(41) {
                        break;
                    }
                    if !self.take(44) {
                        self.warning(
                            position,
                            format!("malformed function-like #define ignored: {name}"),
                        );
                        return;
                    }
                }
            }
            parameters = Some(names);
        }
        self.horizontal();
        let text = self.raw_line();
        let replacement = self.fragment(&text, position);
        self.context.macros.insert(
            name,
            Macro {
                parameters,
                replacement,
            },
        );
    }
    fn pragma(&mut self, position: &Position) -> Option<Token> {
        self.horizontal();
        let save = (self.next, self.position.clone());
        for (name, kind) in [
            ("bank", Kind::PRAGMA_BANK),
            ("fixed_bank", Kind::PRAGMA_FIXED_BANK),
            ("fixed_order", Kind::PRAGMA_FIXED_ORDER),
        ] {
            self.next = save.0;
            self.position = save.1.clone();
            if self.literal(name) {
                self.horizontal();
                if let Some(integer) = self.signed_decimal() {
                    return Some(Token {
                        kind,
                        integer,
                        name: None,
                        position: position.clone(),
                    });
                }
            }
        }
        self.next = save.0;
        self.position = save.1.clone();
        if self.literal("once") {
            self.once = true;
            return None;
        }
        self.next = save.0;
        self.position = save.1.clone();
        let name = self.identifier();
        self.horizontal();
        let lower = name.to_ascii_lowercase();
        if lower == "cgb_palette" {
            let palette_name = self.identifier();
            let mut colors = Vec::new();
            if !palette_name.is_empty() {
                for _ in 0..4 {
                    while matches!(self.peek(), 32 | 9 | 44) {
                        self.fetch();
                    }
                    let color = if self.take(35) {
                        let mut rgb = 0u32;
                        let mut valid = true;
                        for _ in 0..6 {
                            if let Some(d) = char16(self.peek()).to_digit(16) {
                                rgb = (rgb << 4) | d;
                                self.fetch();
                            } else {
                                valid = false;
                                break;
                            }
                        }
                        valid.then(|| {
                            let r = ((rgb >> 16) & 255) * 31 + 127;
                            let g = ((rgb >> 8) & 255) * 31 + 127;
                            let b = (rgb & 255) * 31 + 127;
                            ((r / 255) | ((g / 255) << 5) | ((b / 255) << 10)) as i32
                        })
                    } else {
                        let start = self.next;
                        while !matches!(self.peek(), 0 | 10 | 13 | 32 | 9 | 44) {
                            self.fetch();
                        }
                        let text = String::from_utf16_lossy(&self.input[start..self.next]);
                        self.number(&text, position)
                            .filter(|n| (0..=0x7fff).contains(n))
                    };
                    let Some(color) = color else { break };
                    colors.push(color);
                }
            }
            if colors.len() == 4 {
                self.context.palettes.retain(|p| p.name != palette_name);
                self.context.palettes.push(Palette {
                    name: palette_name,
                    colors: colors.try_into().unwrap(),
                    position: position.clone(),
                });
                return None;
            }
        }
        if lower.starts_with("rom_")
            || matches!(
                lower.as_str(),
                "header_logo"
                    | "headerlogo"
                    | "cart"
                    | "cgb"
                    | "romsize"
                    | "ramsize"
                    | "sgb"
                    | "dest"
                    | "version"
            )
        {
            let value = if self.peek() == 34 {
                self.string_value(false)
            } else {
                let start = self.next;
                while !matches!(self.peek(), 0 | 10 | 13 | 32 | 9) {
                    self.fetch();
                }
                (self.next > start).then(|| String::from_utf16_lossy(&self.input[start..self.next]))
            };
            if let Some(value) = value {
                if let Err(message) = self.context.header.set(&name, &value) {
                    self.diagnostic(
                        if message.starts_with("error:") {
                            Severity::Error
                        } else {
                            Severity::Warning
                        },
                        position,
                        message,
                    );
                }
                return None;
            }
        }
        let (kind, integer, value) = match lower.as_str() {
            "hram" => (Kind::PRAGMA_REGION, 0, None),
            "wram0" => (Kind::PRAGMA_REGION, 3, None),
            "wramx" | "wram1" => (Kind::PRAGMA_REGION, 4, None),
            "wramx_bank" | "align" => {
                let Some(integer) = self.unsigned_placement() else {
                    self.warning(position, "preprocessor directives are ignored");
                    return None;
                };
                (
                    if lower == "align" {
                        Kind::PRAGMA_ALIGN
                    } else {
                        Kind::PRAGMA_WRAMX_BANK
                    },
                    integer,
                    None,
                )
            }
            "section" => {
                let Some(value) = self.string_value(false) else {
                    self.warning(position, "preprocessor directives are ignored");
                    return None;
                };
                (Kind::PRAGMA_SECTION, 0, Some(value))
            }
            _ => {
                self.warning(position, "preprocessor directives are ignored");
                return None;
            }
        };
        Some(Token {
            kind,
            integer,
            name: value,
            position: position.clone(),
        })
    }
    fn directive(&mut self, position: &Position) -> Option<Token> {
        if self.conditional(position) {
            self.line_end();
            return None;
        }
        if !self.active() {
            self.line_end();
            return None;
        }
        self.horizontal();
        let token = if self.literal("include") {
            self.include(position);
            None
        } else if self.literal("define") {
            self.define(position);
            None
        } else if self.literal("undef") {
            self.horizontal();
            if name_start(self.peek()) {
                let name = self.identifier();
                self.context.macros.remove(&name);
            } else {
                self.warning(position, "malformed #undef ignored");
            }
            None
        } else if self.literal("pragma") {
            self.pragma(position)
        } else {
            let directive = self.identifier();
            if matches!(directive.as_str(), "warning" | "error") {
                self.horizontal();
                let raw = self.raw_line().trim().to_owned();
                let message = if raw.is_empty() {
                    format!("#{directive}")
                } else {
                    format!("#{directive}: {raw}")
                };
                self.diagnostic(
                    if directive == "error" {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    position,
                    message,
                );
            } else {
                self.warning(position, "preprocessor directives are ignored");
            }
            None
        };
        self.line_end();
        token
    }
    fn raw_arguments(&mut self) -> Option<Vec<String>> {
        let mut args = Vec::new();
        let mut current = Vec::new();
        let mut depth = 0;
        loop {
            let c = self.peek();
            if c == 0 {
                return None;
            }
            if matches!(c, 34 | 39) {
                current.push(c);
                self.fetch();
                loop {
                    let ch = self.peek();
                    if ch == 0 {
                        return None;
                    }
                    current.push(ch);
                    self.fetch();
                    if ch == 92 {
                        let ch = self.peek();
                        if ch == 0 {
                            return None;
                        }
                        current.push(ch);
                        self.fetch();
                    } else if ch == c {
                        break;
                    }
                }
            } else if c == 40 {
                depth += 1;
                current.push(c);
                self.fetch();
            } else if c == 41 {
                self.fetch();
                if depth == 0 {
                    if !args.is_empty() || !String::from_utf16_lossy(&current).trim().is_empty() {
                        args.push(String::from_utf16_lossy(&current));
                    }
                    return Some(args);
                }
                depth -= 1;
                current.push(c);
            } else if c == 44 && depth == 0 {
                args.push(String::from_utf16_lossy(&current));
                current.clear();
                self.fetch();
            } else {
                current.push(c);
                self.fetch();
            }
        }
    }
    fn expand_raw(&mut self, token: &Token) -> Option<Vec<Token>> {
        let name = token.name.as_ref()?;
        let m = self.context.macros.get(name)?.clone();
        let mut active = BTreeSet::new();
        active.insert(name.clone());
        let replacement = if let Some(parameters) = m.parameters.as_ref() {
            let save = (self.next, self.position.clone());
            self.horizontal();
            if !self.take(40) {
                self.next = save.0;
                self.position = save.1;
                return None;
            }
            let Some(texts) = self.raw_arguments() else {
                self.next = save.0;
                self.position = save.1;
                return None;
            };
            if texts.len() != parameters.len() {
                self.error(
                    &token.position,
                    format!(
                        "macro '{name}' expects {} arguments, got {}",
                        parameters.len(),
                        texts.len()
                    ),
                );
                return Some(Vec::new());
            }
            let args = texts
                .iter()
                .map(|text| self.fragment(text, &token.position))
                .collect::<Vec<_>>();
            let args = args
                .iter()
                .map(|a| {
                    expand(
                        a,
                        &self.context.macros,
                        &mut self.context.diagnostics,
                        &active,
                        1,
                        &token.position,
                    )
                })
                .collect::<Vec<_>>();
            substitute(&m, &args, &token.position)
        } else {
            clone_at(&m.replacement, &token.position)
        };
        Some(expand(
            &replacement,
            &self.context.macros,
            &mut self.context.diagnostics,
            &active,
            1,
            &token.position,
        ))
    }
    fn run(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.spaces();
            let position = self.position.clone();
            let c = self.peek();
            if c == 0 {
                if let Some(frame) = self.conditionals.last() {
                    let start = frame.start.clone();
                    self.error(&start, "missing #endif for conditional block");
                }
                break;
            }
            if self
                .context
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count()
                >= 20
            {
                break;
            }
            if !self.assembly && !self.active() && c != 35 {
                self.line_end();
                continue;
            }
            let mut token = Token::new(Kind::INVALID, position.clone());
            if c == 10 {
                self.literal("\n");
                token.kind = Kind::NEWLINE;
            } else if c == 34 {
                token.kind = Kind::STRING;
                if let Some(s) = self.string_value(false) {
                    token.name = Some(s)
                } else {
                    self.error(
                        &position,
                        if self.peek() == 0 {
                            "unexpected end of file in string"
                        } else {
                            "unexpected end of line in string"
                        },
                    );
                    break;
                }
            } else if c == 39 {
                self.fetch();
                let ch = self.peek();
                if ch == 0 || ch > 127 || matches!(ch, 39 | 10 | 13) {
                    let pos = self.position.clone();
                    self.error(&pos, "invalid character constant");
                }
                token.kind = Kind::INT;
                token.integer = i32::from(ch);
                self.fetch();
                if self.peek() != 39 {
                    let pos = self.position.clone();
                    self.error(&pos, "expected '");
                }
                self.fetch();
            } else if c == 35 {
                self.fetch();
                if self.assembly {
                    token.kind = Kind::NUMBER_SIGN
                } else {
                    let t = self.directive(&position);
                    tokens.append(&mut self.included);
                    if let Some(t) = t {
                        tokens.push(t)
                    }
                    continue;
                }
            } else if c == 47 && self.input.get(self.next + 1) == Some(&47) {
                self.fetch();
                self.fetch();
                self.line_end();
                continue;
            } else if c == 47 && self.input.get(self.next + 1) == Some(&42) {
                self.fetch();
                self.fetch();
                loop {
                    if self.peek() == 0 {
                        let pos = self.position.clone();
                        self.error(&pos, "unexpected end of file in block comment");
                        break;
                    }
                    if self.take(42) {
                        if self.take(47) {
                            break;
                        }
                    } else {
                        self.fetch();
                    }
                }
                continue;
            } else if name_char(c) {
                let name = self.identifier();
                if let Some(n) = self.number(&name, &position) {
                    token.kind = Kind::INT;
                    token.integer = n;
                } else {
                    token.kind = Kind::NAME;
                    token.name = Some(name);
                    if !self.disable_macros && !self.assembly {
                        if let Some(expanded) = self.expand_raw(&token) {
                            tokens.extend(expanded);
                            continue;
                        }
                    }
                }
            } else {
                self.fetch();
                token.kind = match c {
                    33 => {
                        if self.literal("=") {
                            Kind::NOT_EQUAL
                        } else {
                            Kind::LOGICAL_NOT
                        }
                    }
                    37 => {
                        if self.take(61) {
                            Kind::PERCENT_EQUALS
                        } else {
                            Kind::PERCENT
                        }
                    }
                    38 => {
                        if self.take(38) {
                            Kind::LOGICAL_AND
                        } else if self.take(61) {
                            Kind::AMPERSAND_EQUALS
                        } else {
                            Kind::AMPERSAND
                        }
                    }
                    40 => Kind::LPAREN,
                    41 => Kind::RPAREN,
                    42 => {
                        if self.take(61) {
                            Kind::STAR_EQUALS
                        } else {
                            Kind::STAR
                        }
                    }
                    43 => {
                        if self.take(43) {
                            Kind::INCREMENT
                        } else if self.take(61) {
                            Kind::PLUS_EQUALS
                        } else {
                            Kind::PLUS
                        }
                    }
                    44 => Kind::COMMA,
                    45 => {
                        if self.take(62) {
                            Kind::ARROW
                        } else if self.take(45) {
                            Kind::DECREMENT
                        } else if self.take(61) {
                            Kind::MINUS_EQUALS
                        } else {
                            Kind::MINUS
                        }
                    }
                    46 => Kind::PERIOD,
                    47 => {
                        if self.take(61) {
                            Kind::SLASH_EQUALS
                        } else {
                            Kind::SLASH
                        }
                    }
                    58 => Kind::COLON,
                    59 => Kind::SEMICOLON,
                    60 => {
                        if self.take(61) {
                            Kind::LESS_THAN_OR_EQUAL
                        } else if self.take(60) {
                            if self.take(61) {
                                Kind::SHIFT_LEFT_EQUALS
                            } else {
                                Kind::SHIFT_LEFT
                            }
                        } else {
                            Kind::LESS_THAN
                        }
                    }
                    61 => {
                        if self.take(61) {
                            Kind::DOUBLE_EQUAL
                        } else {
                            Kind::EQUAL
                        }
                    }
                    62 => {
                        if self.take(61) {
                            Kind::GREATER_THAN_OR_EQUAL
                        } else if self.take(62) {
                            if self.take(61) {
                                Kind::SHIFT_RIGHT_EQUALS
                            } else {
                                Kind::SHIFT_RIGHT
                            }
                        } else {
                            Kind::GREATER_THAN
                        }
                    }
                    63 => Kind::QUESTION_MARK,
                    91 => Kind::LBRACKET,
                    93 => Kind::RBRACKET,
                    94 => {
                        if self.take(61) {
                            Kind::CARET_EQUALS
                        } else {
                            Kind::CARET
                        }
                    }
                    123 => Kind::LBRACE,
                    124 => {
                        if self.take(124) {
                            Kind::LOGICAL_OR
                        } else if self.take(61) {
                            Kind::PIPE_EQUALS
                        } else {
                            Kind::PIPE
                        }
                    }
                    125 => Kind::RBRACE,
                    126 => Kind::TILDE,
                    _ => {
                        self.error(&position, format!("invalid token: {}", char16(c)));
                        Kind::INVALID
                    }
                };
            }
            if !self.assembly && token.kind == Kind::NAME && token.name.as_deref() == Some("__asm")
            {
                self.assembly = true;
            }
            if self.assembly && token.kind == Kind::RBRACE {
                self.assembly = false;
            }
            tokens.push(token);
        }
        tokens
    }
}
fn clone_at(tokens: &[Token], position: &Position) -> Vec<Token> {
    tokens
        .iter()
        .map(|t| Token {
            position: position.clone(),
            ..t.clone()
        })
        .collect()
}
fn substitute(m: &Macro, args: &[Vec<Token>], position: &Position) -> Vec<Token> {
    let mut result = Vec::new();
    for t in &m.replacement {
        if let Some(index) = m.parameters.as_ref().and_then(|p| {
            p.iter()
                .position(|p| t.kind == Kind::NAME && t.name.as_ref() == Some(p))
        }) {
            result.extend(clone_at(&args[index], position));
        } else {
            result.push(Token {
                position: position.clone(),
                ..t.clone()
            });
        }
    }
    result
}
fn invocation(tokens: &[Token], start: usize) -> Option<(Vec<Vec<Token>>, usize)> {
    if tokens.get(start)?.kind != Kind::LPAREN {
        return None;
    }
    let mut args = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0;
    for (index, t) in tokens.iter().enumerate().skip(start + 1) {
        match t.kind {
            Kind::LPAREN => {
                depth += 1;
                current.push(t.clone());
            }
            Kind::RPAREN => {
                if depth == 0 {
                    if !args.is_empty() || !current.is_empty() {
                        args.push(current)
                    }
                    return Some((args, index));
                }
                depth -= 1;
                current.push(t.clone());
            }
            Kind::COMMA if depth == 0 => {
                args.push(current);
                current = Vec::new();
            }
            _ => current.push(t.clone()),
        }
    }
    None
}
fn expand(
    tokens: &[Token],
    macros: &BTreeMap<String, Macro>,
    diagnostics: &mut Vec<Diagnostic>,
    active: &BTreeSet<String>,
    depth: usize,
    call: &Position,
) -> Vec<Token> {
    if depth > 32 {
        diagnostics.push(Diagnostic::new(
            Severity::Warning,
            call.clone(),
            "macro expansion depth exceeded (32); expansion truncated".into(),
            0,
        ));
        return tokens.to_vec();
    }
    let mut result = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        index += 1;
        let Some((name, m)) = token
            .name
            .as_ref()
            .filter(|_| token.kind == Kind::NAME)
            .and_then(|name| macros.get(name).map(|m| (name, m)))
            .filter(|(name, _)| !active.contains(*name))
        else {
            result.push(token.clone());
            continue;
        };
        let mut next = active.clone();
        next.insert(name.clone());
        let replacement = if let Some(parameters) = m.parameters.as_ref() {
            let Some((args, end)) = invocation(tokens, index) else {
                result.push(token.clone());
                continue;
            };
            index = end + 1;
            if args.len() != parameters.len() {
                diagnostics.push(Diagnostic::new(
                    Severity::Error,
                    token.position.clone(),
                    format!(
                        "macro '{name}' expects {} arguments, got {}",
                        parameters.len(),
                        args.len()
                    ),
                    0,
                ));
                continue;
            }
            let args = args
                .iter()
                .map(|a| expand(a, macros, diagnostics, &next, depth + 1, &token.position))
                .collect::<Vec<_>>();
            substitute(m, &args, &token.position)
        } else {
            clone_at(&m.replacement, &token.position)
        };
        result.extend(expand(
            &replacement,
            macros,
            diagnostics,
            &next,
            depth + 1,
            &token.position,
        ));
    }
    result
}
