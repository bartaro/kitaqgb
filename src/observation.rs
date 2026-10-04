//! Per-invocation diagnostics and optional compile-stage artifacts.
use crate::{
    asm::AddressMode,
    expr::{Arg, Expr, Position},
    io,
    json::Value,
    metadata::Policy,
    tags as t,
    token::{Kind, Token},
    tokenizer::{Diagnostic, Severity},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
pub struct Session {
    pub cancellation_token: Option<crate::drivers::CancellationToken>,
    pub exit_code: Option<i32>,
    /// API and internal comparison invocations retain diagnostics without console output.
    pub hosted: bool,
    pub strict: bool,
    pub machine: bool,
    pub max_errors: usize,
    pub diag_path: Option<String>,
    pub manifest_path: Option<String>,
    pub debug: bool,
    pub disable_disasm: bool,
    pub changed_disasm: Option<String>,
    pub changed_disasm_path: Option<String>,
    pub debug_dir: PathBuf,
    pub trace: bool,
    pub trace_dir: PathBuf,
    pub stages: BTreeSet<String>,
    pub output: String,
    pub dependencies: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    captured: usize,
    start: Instant,
    stats: Vec<(String, Duration, String)>,
    pub artifacts: BTreeMap<String, String>,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            cancellation_token: None,
            exit_code: None,
            hosted: false,
            strict: false,
            machine: false,
            max_errors: 20,
            diag_path: None,
            manifest_path: None,
            debug: false,
            disable_disasm: false,
            changed_disasm: None,
            changed_disasm_path: None,
            debug_dir: "debug_output".into(),
            trace: false,
            trace_dir: "trace_output".into(),
            stages: BTreeSet::new(),
            output: "out.gb".into(),
            dependencies: Vec::new(),
            diagnostics: Vec::new(),
            captured: 0,
            start: Instant::now(),
            stats: Vec::new(),
            artifacts: BTreeMap::new(),
        }
    }
}
impl Session {
    pub fn capture(&mut self, diagnostics: &mut Vec<Diagnostic>) {
        let mut length = diagnostics.len();
        for (i, d) in diagnostics.iter_mut().enumerate().skip(self.captured) {
            if self.errors() >= self.max_errors {
                length = i;
                break;
            }
            if d.code == 0 {
                d.code = crate::diagnostics::infer(&d.message, 0);
            }
            let promoted =
                self.strict && d.severity == Severity::Warning && (2400..2500).contains(&d.code);
            if promoted {
                d.severity = Severity::Error;
            }
            self.print(d, promoted);
            self.diagnostics.push(d.clone());
        }
        diagnostics.truncate(length);
        self.captured = length;
    }
    fn print(&self, d: &Diagnostic, promoted: bool) {
        if self.hosted {
            return;
        }
        let severity = if d.severity == Severity::Error {
            "error"
        } else {
            "warning"
        };
        let source = d.has_position;
        if source {
            eprint!("{} ", d.position);
        }
        eprintln!("{severity} KQ{:04}: {}", d.code, d.message);
        if !self.machine {
            if promoted {
                eprintln!(
                    "  hint: strict mode treats this lint warning as an error (use --permissive to keep warnings)"
                );
            }
            if source {
                if let Ok(text) = io::read_utf8(&d.position.filename) {
                    if let Some(line) = text.lines().nth(d.position.line) {
                        let line = line.chars().take(200).collect::<String>();
                        eprintln!("  {line}");
                        eprintln!("  {}^", " ".repeat(d.position.column.min(200)));
                    }
                }
            }
            let suggestion = crate::diagnostics::suggestion(d.code, &d.message);
            if !suggestion.is_empty() {
                eprintln!("  hint: {suggestion}");
            }
        }
    }
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
    pub fn global_error(&mut self, message: &str) {
        let mut d = Diagnostic::new(Severity::Error, Position::default(), message.into(), 0);
        d.has_position = false;
        self.print(&d, false);
        self.diagnostics.push(d);
    }
    pub fn warning(&mut self, message: &str) {
        let mut d = Diagnostic::new(Severity::Warning, Position::default(), message.into(), 0);
        d.has_position = false;
        self.print(&d, false);
        self.diagnostics.push(d);
    }
    pub fn traced(&self, stage: &str) -> bool {
        self.trace && (self.stages.contains(stage) || self.stages.contains("all"))
    }
    pub fn trace_file(&mut self, name: &str, text: &str) -> Result<(), String> {
        if self.trace {
            self.write_artifact(&format!("trace:{name}"), self.trace_dir.join(name), text)?;
            self.remember("trace_dir", self.trace_dir.clone());
        }
        Ok(())
    }
    pub fn debug_file(&mut self, name: &str, text: &str) -> Result<(), String> {
        if self.debug {
            self.write_artifact(&format!("debug:{name}"), self.debug_dir.join(name), text)?;
            self.remember("debug_dir", self.debug_dir.clone());
        }
        Ok(())
    }
    pub fn write_artifact(
        &mut self,
        key: &str,
        path: impl AsRef<Path>,
        text: &str,
    ) -> Result<PathBuf, String> {
        let requested = path.as_ref();
        let path = self
            .artifacts
            .get(key)
            .map(PathBuf::from)
            .unwrap_or_else(|| requested.to_owned());
        let actual = io::write_utf8_robust(&path, text, true)?;
        if actual != path {
            self.warning(&format!(
                "output is locked, wrote alternate file: {}",
                actual.display()
            ));
        }
        if !key.starts_with("trace:") && !key.starts_with("debug:") {
            self.remember(key, &actual);
        }
        Ok(actual)
    }
    pub fn stat(&mut self, name: &str, start: Instant, detail: impl Into<String>) {
        self.stats
            .push((name.into(), start.elapsed(), detail.into()));
    }
    pub fn stat_duration(&mut self, name: &str, time: Duration, detail: impl Into<String>) {
        self.stats.push((name.into(), time, detail.into()));
    }
    pub fn remember(&mut self, key: &str, path: impl AsRef<Path>) {
        let path = path.as_ref();
        let full = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        self.artifacts
            .insert(key.into(), full.display().to_string());
    }
    pub fn summary(&mut self, files: usize) -> Result<(), String> {
        let mut text = format!(
            "# KITAQGB trace short summary\nfiles={files}\noutput={}\ntotal_ms={}\n\n# stage, elapsed_ms, detail\n",
            self.output,
            self.start.elapsed().as_millis()
        );
        for (name, time, detail) in &self.stats {
            writeln!(text, "{name}, {}, {detail}", time.as_millis()).unwrap();
        }
        self.trace_file("trace_summary.txt", &text)
    }
    pub fn diagnostic_json(&self, exit: i32) -> Value {
        Value::Object(
            [
                ("exit_code".into(), Value::Number(exit.into())),
                ("errors".into(), Value::Number(self.errors() as i64)),
                (
                    "warnings".into(),
                    Value::Number(self.diagnostics.len() as i64 - self.errors() as i64),
                ),
                ("output".into(), Value::String(self.output.clone())),
                (
                    "diagnostics".into(),
                    Value::Array(
                        self.diagnostics
                            .iter()
                            .map(|d| {
                                let source = d.has_position;
                                Value::Object(
                                    [
                                        (
                                            "severity".into(),
                                            Value::String(
                                                if d.severity == Severity::Error {
                                                    "error"
                                                } else {
                                                    "warning"
                                                }
                                                .into(),
                                            ),
                                        ),
                                        ("code".into(), Value::String(format!("KQ{:04}", d.code))),
                                        ("message".into(), Value::String(d.message.clone())),
                                        (
                                            "suggestion".into(),
                                            Value::String(
                                                crate::diagnostics::suggestion(d.code, &d.message)
                                                    .into(),
                                            ),
                                        ),
                                        (
                                            "file".into(),
                                            Value::String(if source {
                                                d.position.filename.clone()
                                            } else {
                                                String::new()
                                            }),
                                        ),
                                        (
                                            "line".into(),
                                            Value::Number(if source {
                                                d.position.line as i64 + 1
                                            } else {
                                                0
                                            }),
                                        ),
                                        (
                                            "column".into(),
                                            Value::Number(if source {
                                                d.position.column as i64 + 1
                                            } else {
                                                0
                                            }),
                                        ),
                                    ]
                                    .into(),
                                )
                            })
                            .collect(),
                    ),
                ),
            ]
            .into(),
        )
    }
    pub fn finish(&mut self, exit: i32) -> Result<(), String> {
        if let Some(path) = self.diag_path.clone() {
            let text = self.diagnostic_json(exit).stringify();
            if path == "-" && !self.hosted {
                eprintln!("{text}");
            } else if path == "-" {
                // The API returns diagnostics directly; never emit them on the host's stderr.
            } else {
                let actual = self.write_artifact("diag_json", &path, &text)?;
                self.diag_path = Some(actual.display().to_string());
            }
        }
        let output = PathBuf::from(&self.output);
        for (key, extension) in [
            ("output_rom", None),
            ("dbg2_json", Some("dbg2.json")),
            ("build_report_json", Some("build_report.json")),
            ("map", Some("map")),
            ("dbg", Some("dbg")),
            ("dbc", Some("dbc")),
            ("banks_txt", Some("banks.txt")),
            ("funcsizes_txt", Some("funcsizes.txt")),
            ("source_map", Some("source_map.txt")),
            ("deps_list", Some("deps.txt")),
        ] {
            if self.artifacts.contains_key(key) {
                continue;
            }
            let path = extension.map_or_else(|| output.clone(), |s| output.with_extension(s));
            if path.is_file() {
                self.remember(key, path);
            }
        }
        for (key, path) in [
            ("debug_dir", self.debug_dir.clone()),
            ("trace_dir", self.trace_dir.clone()),
        ] {
            if path.is_dir() {
                self.remember(key, path);
            }
        }
        if let Some(path) = self.manifest_path.clone() {
            let mut fields = self
                .artifacts
                .iter()
                .filter(|(k, _)| k.as_str() != "path_manifest")
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect::<BTreeMap<_, _>>();
            fields.insert("exit_code".into(), Value::Number(exit.into()));
            self.write_artifact("path_manifest", &path, &Value::Object(fields).stringify())?;
        }
        Ok(())
    }
}
pub fn tokens(tokens: &[Token]) -> String {
    let mut out = String::new();
    for token in tokens {
        write!(out, "{}\t{:?}", token.position, token.kind).unwrap();
        match token.kind {
            Kind::INT => write!(out, "\t{}", token.integer).unwrap(),
            Kind::NAME => write!(out, "\t{}", token.name.as_deref().unwrap_or("")).unwrap(),
            Kind::STRING => write!(out, "\t\"{}\"", token.name.as_deref().unwrap_or("")).unwrap(),
            Kind::PRAGMA_BANK => write!(out, "\tbank={}", token.integer).unwrap(),
            Kind::PRAGMA_FIXED_BANK => write!(out, "\tfixed_bank={}", token.integer).unwrap(),
            Kind::PRAGMA_FIXED_ORDER => write!(out, "\tfixed_order={}", token.integer).unwrap(),
            Kind::PRAGMA_WRAMX_BANK => write!(out, "\twramx_bank={}", token.integer).unwrap(),
            _ => {}
        }
        out.push('\n');
    }
    out
}
pub fn assembly(lines: &[Expr]) -> String {
    let mut out = String::new();
    for e in lines {
        let top = e.is(t::FUNCTION);
        if top {
            out.push('\n');
        }
        if !top && !e.is(t::LABEL) {
            out.push('\t');
        }
        let indent = if !top && !e.is(t::LABEL) { "\t" } else { "" };
        let prefix = out.len() - indent.len();
        out.insert_str(prefix, &format!("{indent}; <{}>\n", e.source));
        if e.is(t::COMMENT) {
            write!(out, "; {}", e.text(1).unwrap_or("")).unwrap();
        } else if e.is(t::LABEL) {
            write!(out, "{}:", e.text(1).unwrap_or("")).unwrap();
        } else if top && e.text(1).is_some() {
            write!(out, "; function {}:", e.text(1).unwrap()).unwrap();
        } else if let Some((m, o)) = e.asm_parts() {
            out.push_str(m);
            if o.mode != AddressMode::Implicit {
                write!(out, " {o}").unwrap();
            }
        } else {
            out.push_str(&e.show());
        }
        out.push('\n');
    }
    out
}
pub fn ast_summary(tree: &Expr) -> String {
    if !tree.is(t::SEQUENCE)
        || tree.args.len() != 2
        || !matches!(tree.args.get(1), Some(Arg::Exprs(_)))
    {
        return format!("{}\n", tree.show());
    }
    let items = tree.children();
    let mut out = format!("Top-level items: {}\n", items.len());
    for e in items {
        let tag = e.tag().unwrap_or("");
        let source = &e.source;
        if matches!(tag, t::FUNCTION | t::INLINE_FUNCTION) && e.args.len() == 6 {
            let args = if let Arg::Fields(fields) = &e.args[3] {
                fields.len()
            } else {
                0
            };
            let body = e.child(5).unwrap();
            let stmts = if body.is(t::SEQUENCE) {
                body.children().len()
            } else {
                0
            };
            writeln!(
                out,
                "{tag}\t{}({args} args) -> {}\tstmts={stmts}\tmust_check={}\t@{source}",
                e.text(2).unwrap_or(""),
                e.ty(1).unwrap(),
                if e.int(4).unwrap_or(0) != 0 {
                    "True"
                } else {
                    "False"
                }
            )
            .unwrap();
        } else if tag == t::FUNCTION_DECL && e.args.len() == 5 {
            let args = if let Arg::Fields(fields) = &e.args[3] {
                fields.len()
            } else {
                0
            };
            writeln!(
                out,
                "{tag}\t{}({args} args) -> {}\tmust_check={}\t@{source}",
                e.text(2).unwrap_or(""),
                e.ty(1).unwrap(),
                if e.int(4).unwrap_or(0) != 0 {
                    "True"
                } else {
                    "False"
                }
            )
            .unwrap();
        } else if tag == t::CONSTANT && e.args.len() == 4 {
            writeln!(
                out,
                "{tag}\t{}: {} = {}\t@{source}",
                e.text(2).unwrap_or(""),
                e.ty(1).unwrap(),
                e.child(3).unwrap().show()
            )
            .unwrap();
        } else if tag == t::READONLY_DATA && e.args.len() == 5 {
            if let Arg::Ints(values) = &e.args[4] {
                writeln!(
                    out,
                    "{tag}\t{}: {}\tbank={}\tbytes={}\t@{source}",
                    e.text(2).unwrap_or(""),
                    e.ty(1).unwrap(),
                    e.int(3).unwrap_or(0),
                    values.len()
                )
                .unwrap();
            } else {
                writeln!(out, "{tag}\t{}\t@{source}", e.show()).unwrap();
            }
        } else {
            writeln!(out, "{tag}\t{}\t@{source}", e.show()).unwrap();
        }
    }
    out
}
pub fn stack_text(policy: &Policy) -> String {
    let addr = |v| format!("${:04X}", v & 65535);
    let range = if policy.reserve > 0 {
        format!(
            "{}-{}",
            addr(policy.top - policy.reserve + 1),
            addr(policy.top)
        )
    } else {
        "<none>".into()
    };
    format!(
        "stack.bank = {}\nstack.top = {}\nstack.reserve = {}\nstack.auto_limit = {}\nstack.reserved_range = {range}\n",
        if policy.fixed { "fixed" } else { "wramx1" },
        addr(policy.top),
        policy.reserve,
        addr(policy.top - policy.reserve)
    )
}
