//! In-process compiler host with invocation-local options, diagnostics and artifacts.
use crate::{
    diagnostics,
    observation::Session,
    tokenizer::{Diagnostic, Severity},
};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug)]
pub struct CompileRequest {
    pub cancellation_token: Option<crate::drivers::CancellationToken>,
    pub source_files: Vec<String>,
    pub output_filename: String,
    pub include_directories: Vec<String>,
    pub profile: Option<String>,
    pub strict_diagnostics: bool,
    pub enable_debug_output: bool,
    pub debug_output_path: Option<String>,
    pub enable_trace: bool,
    pub trace_output_path: Option<String>,
    pub trace_stages: Vec<String>,
    pub emit_diag_json: bool,
    pub diag_json_path: Option<String>,
    pub emit_path_manifest: bool,
    pub path_manifest_path: Option<String>,
    pub machine_readable: bool,
    pub no_banner: bool,
    pub stack_bank: Option<String>,
    pub stack_top: Option<i32>,
    pub stack_reserve: Option<i32>,
    /// Appended after typed options; the shared CLI applies its usual last-option precedence.
    pub extra_args: Vec<String>,
}
impl Default for CompileRequest {
    fn default() -> Self {
        Self {
            cancellation_token: None,
            source_files: Vec::new(),
            output_filename: "out.gb".into(),
            include_directories: Vec::new(),
            profile: None,
            strict_diagnostics: false,
            enable_debug_output: false,
            debug_output_path: None,
            enable_trace: false,
            trace_output_path: None,
            trace_stages: Vec::new(),
            emit_diag_json: false,
            diag_json_path: None,
            emit_path_manifest: false,
            path_manifest_path: None,
            machine_readable: true,
            no_banner: true,
            stack_bank: None,
            stack_top: None,
            stack_reserve: None,
            extra_args: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticDto {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub suggestion: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}
impl From<&Diagnostic> for DiagnosticDto {
    fn from(d: &Diagnostic) -> Self {
        Self {
            severity: if d.severity == Severity::Error {
                "error"
            } else {
                "warning"
            }
            .into(),
            code: format!("KQ{:04}", d.code),
            message: d.message.clone(),
            suggestion: diagnostics::suggestion(d.code, &d.message).into(),
            file: if d.has_position {
                d.position.filename.clone()
            } else {
                String::new()
            },
            line: if d.has_position {
                d.position.line + 1
            } else {
                0
            },
            column: if d.has_position {
                d.position.column + 1
            } else {
                0
            },
        }
    }
}
#[derive(Clone, Debug)]
pub struct CompileResult {
    pub success: bool,
    pub exit_code: i32,
    pub output_filename: String,
    pub diagnostics: Vec<DiagnosticDto>,
    pub diag_json_path: Option<String>,
    pub debug_metadata_json_path: Option<String>,
    pub build_report_json_path: Option<String>,
    pub output_artifacts: BTreeMap<String, String>,
}
pub trait IKitaqgbCompiler {
    fn compile(&self, request: &CompileRequest) -> Result<CompileResult, String>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct KitaqgbCompiler;
fn nonblank(text: &Option<String>) -> Option<&str> {
    text.as_deref().filter(|s| !s.trim().is_empty())
}
impl CompileRequest {
    /// Translate options to an argument vector; no shell or process environment is involved.
    pub fn command_line_args(&self) -> Result<Vec<String>, String> {
        if self.source_files.is_empty() {
            return Err("CompileRequest.SourceFiles must contain at least one source file.".into());
        }
        let mut args = self
            .source_files
            .iter()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>();
        if !self.output_filename.trim().is_empty() {
            args.extend(["-o".into(), self.output_filename.clone()]);
        }
        for dir in self
            .include_directories
            .iter()
            .filter(|s| !s.trim().is_empty())
        {
            args.extend(["-I".into(), dir.clone()]);
        }
        if let Some(profile) = nonblank(&self.profile) {
            args.push(format!("--profile={profile}"));
        }
        if self.strict_diagnostics {
            args.push("--strict".into());
        }
        if self.enable_debug_output {
            args.push(
                nonblank(&self.debug_output_path)
                    .map_or_else(|| "--debug-out".into(), |p| format!("--debug-out={p}")),
            );
        }
        if self.enable_trace
            || !self.trace_stages.is_empty()
            || nonblank(&self.trace_output_path).is_some()
        {
            let stages = self
                .trace_stages
                .iter()
                .filter(|s| !s.trim().is_empty())
                .map(String::as_str)
                .collect::<Vec<_>>();
            args.push(if stages.is_empty() {
                "--trace".into()
            } else {
                format!("--trace={}", stages.join(","))
            });
            if let Some(path) = nonblank(&self.trace_output_path) {
                args.push(format!("--trace-out={path}"));
            }
        }
        if self.emit_diag_json {
            args.push(
                nonblank(&self.diag_json_path)
                    .map_or_else(|| "--diag-json".into(), |p| format!("--diag-json={p}")),
            );
        }
        if self.emit_path_manifest {
            let path = nonblank(&self.path_manifest_path).map_or_else(
                || {
                    PathBuf::from(&self.output_filename)
                        .with_extension("artifacts.json")
                        .display()
                        .to_string()
                },
                str::to_owned,
            );
            args.push(format!("--emit-path-manifest={path}"));
        }
        if self.machine_readable {
            args.push("--machine-readable".into());
        }
        if self.no_banner {
            args.push("--no-banner".into());
        }
        if let Some(bank) = nonblank(&self.stack_bank) {
            args.push(format!("--stack-bank={bank}"));
        }
        if let Some(top) = self.stack_top {
            args.push(format!("--stack-top=0x{top:04X}"));
        }
        if let Some(reserve) = self.stack_reserve {
            args.push(format!("--stack-reserve={reserve}"));
        }
        args.extend(
            self.extra_args
                .iter()
                .filter(|s| !s.trim().is_empty())
                .cloned(),
        );
        Ok(args)
    }
}
impl IKitaqgbCompiler for KitaqgbCompiler {
    fn compile(&self, request: &CompileRequest) -> Result<CompileResult, String> {
        let args = request.command_line_args()?;
        let mut session = Session::default();
        session.hosted = true;
        session.cancellation_token = request.cancellation_token.clone();
        let exit_code = crate::cli::invoke(args, &mut session);
        let output_filename = session
            .artifacts
            .get("output_rom")
            .cloned()
            .unwrap_or_else(|| {
                let path = PathBuf::from(&session.output);
                if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir().unwrap_or_default().join(path)
                }
                .display()
                .to_string()
            });
        Ok(CompileResult {
            success: exit_code == 0
                && PathBuf::from(&output_filename).is_file()
                && session.errors() == 0,
            exit_code,
            output_filename,
            diagnostics: session
                .diagnostics
                .iter()
                .map(DiagnosticDto::from)
                .collect(),
            diag_json_path: session.artifacts.get("diag_json").cloned(),
            debug_metadata_json_path: session.artifacts.get("dbg2_json").cloned(),
            build_report_json_path: session.artifacts.get("build_report_json").cloned(),
            output_artifacts: session.artifacts,
        })
    }
}
