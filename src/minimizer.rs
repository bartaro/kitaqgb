//! Line-deletion delta debugging using isolated in-process compiler sessions.
use crate::{cli, drivers, io, observation::Session, workflow};
use std::{
    fmt::Write,
    path::{Path, PathBuf},
};
pub fn requested(args: &[String]) -> bool {
    args.iter()
        .any(|a| a == "--minimize" || a.starts_with("--minimize="))
}
struct Options {
    base: Vec<String>,
    target: PathBuf,
    out: PathBuf,
    work: PathBuf,
    quick: bool,
    trace: Option<String>,
    user_trace: Option<PathBuf>,
    all: bool,
}
fn options(args: &[String]) -> Result<Options, String> {
    let (mut base, mut target, mut out, mut work, mut quick, mut trace, mut user_trace, mut all) = (
        Vec::new(),
        None,
        None,
        PathBuf::from("minimize_output"),
        false,
        None,
        None,
        false,
    );
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-o" {
            i += 1;
            if i >= args.len() {
                return Err("missing output path".into());
            }
        } else if a == "--minimize"
            || matches!(a.as_str(), "--auto-minimize" | "--minimize-on-fail")
        {
        } else if let Some(p) = a.strip_prefix("--minimize=") {
            target = Some(PathBuf::from(p));
        } else if let Some(p) = a.strip_prefix("--minimize-out=") {
            out = Some(PathBuf::from(p));
        } else if let Some(p) = a.strip_prefix("--minimize-work=") {
            work = p.into();
        } else if a == "--minimize-quick" {
            quick = true;
        } else if let Some(p) = a.strip_prefix("--minimize-trace=") {
            all = match p {
                "all" => true,
                "final" => false,
                _ => return Err("--minimize-trace must be 'final' or 'all'".into()),
            };
        } else if a == "--trace" || a.starts_with("--trace=") {
            trace = Some(a.clone());
        } else if let Some(p) = a.strip_prefix("--trace-out=") {
            user_trace = Some(PathBuf::from(p));
            if trace.is_none() {
                trace = Some("--trace".into());
            }
        } else if a.starts_with("--debug-out=")
            || a.starts_with("--debug-output=")
            || a.starts_with("--repro-pack")
            || a.starts_with("--disasm-changed")
            || a.starts_with("--watch")
        {
        } else if matches!(a.as_str(), "--emit-ai-metadata" | "--emit-path-manifest") {
            i += 1;
            if i >= args.len() {
                return Err(format!("{a} requires a file path"));
            }
        } else if a.starts_with("--emit-ai-metadata=")
            || a.starts_with("--emit-path-manifest=")
            || a.starts_with("--diag-json")
            || a.starts_with("--deps-out")
            || a.starts_with("--vlist")
            || a == "vlist"
            || super::build_helpers::is_analysis(a)
        {
        } else {
            base.push(a.clone());
        }
        i += 1;
    }
    let files = drivers::sources(&base);
    if files.is_empty() {
        return Err("--minimize requires at least one source file".into());
    }
    let target = workflow::full(target.unwrap_or_else(|| files[0].clone()));
    if !files.iter().any(|p| workflow::full(p) == target) {
        return Err(format!(
            "--minimize target file not found in inputs: {}",
            target.display()
        ));
    }
    let out = out.unwrap_or_else(|| {
        target.with_file_name(format!(
            "{}.min.{}",
            target.file_stem().unwrap_or_default().to_string_lossy(),
            target.extension().unwrap_or_default().to_string_lossy()
        ))
    });
    Ok(Options {
        base,
        target,
        out,
        work: workflow::full(work),
        quick,
        trace,
        user_trace,
        all,
    })
}
struct Run {
    code: i32,
    stderr: String,
}
fn run_child(opt: &Options, replacement: &Path, dir: &Path, trace: bool) -> Run {
    let mut args = opt
        .base
        .iter()
        .map(|a| {
            if !a.starts_with('-') && workflow::full(a) == opt.target {
                replacement.display().to_string()
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>();
    args.extend([
        "-o".into(),
        dir.join("candidate.gb").display().to_string(),
        "--no-cache".into(),
        "--no-debug-output".into(),
        "--no-trace".into(),
    ]);
    // A relocated candidate must still resolve quoted includes beside the original.
    if let Some(parent) = opt.target.parent() {
        args.extend(["-I".into(), parent.display().to_string()]);
    }
    if trace {
        args.push(opt.trace.clone().unwrap_or_else(|| "--trace".into()));
        args.push(format!("--trace-out={}", dir.join("trace").display()));
        args.push(format!("--debug-out={}", dir.join("debug").display()));
    }
    let mut session = Session::default();
    session.hosted = true;
    let code = cli::invoke(args, &mut session);
    let mut stderr = String::new();
    for d in session.diagnostics {
        if d.has_position {
            write!(stderr, "{} ", d.position).unwrap();
        }
        writeln!(
            stderr,
            "{} KQ{:04}: {}",
            if d.severity == crate::tokenizer::Severity::Error {
                "error"
            } else {
                "warning"
            },
            d.code,
            d.message
        )
        .unwrap();
    }
    Run { code, stderr }
}
fn fingerprint(output: &str) -> String {
    for pattern in [r"\bKQ\d{4}\b", r"\bSystem\.[A-Za-z0-9_.]+Exception\b"] {
        if let Some(m) = regex::Regex::new(pattern).unwrap().find(output) {
            return m.as_str().into();
        }
    }
    output
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty() && !s.starts_with("[kitaqgb]"))
        .unwrap_or("")
        .chars()
        .take(120)
        .collect()
}
fn logs(dir: &Path, r: &Run) -> Result<(), String> {
    io::write_utf8(dir.join("run_stdout.txt"), "")?;
    io::write_utf8(dir.join("run_stderr.txt"), &r.stderr)?;
    io::write_utf8(dir.join("run_exitcode.txt"), &r.code.to_string())
}
fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let kind = e.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            copy_tree(&e.path(), &dst.join(e.file_name()))?;
        } else if kind.is_file() {
            io::write_bytes(
                dst.join(e.file_name()),
                &std::fs::read(e.path()).map_err(|e| e.to_string())?,
            )?;
        }
    }
    Ok(())
}
pub fn run(args: &[String], session: &mut Session) -> Result<(), String> {
    let opt = options(args)?;
    let original = io::read_utf8(&opt.target)?;
    let mut current = if original.is_empty() {
        vec!["\n".to_owned()]
    } else {
        original
            .split_inclusive('\n')
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let count = current.len();
    std::fs::create_dir_all(&opt.work).map_err(|e| e.to_string())?;
    let baseline = run_child(&opt, &opt.target, &opt.work, false);
    if baseline.code == 0 {
        if !session.hosted {
            println!("[minimize] input does not fail; nothing to minimize.");
        }
        return Ok(());
    }
    let id = fingerprint(&baseline.stderr);
    if !session.hosted {
        println!(
            "[minimize] fingerprint: {}",
            if id.is_empty() { "<none>" } else { &id }
        );
    }
    let (mut n, mut iter, mut serial) = (2usize, 0usize, 0usize);
    while current.len() >= 2 {
        if session
            .cancellation_token
            .as_ref()
            .is_some_and(drivers::CancellationToken::is_cancelled)
        {
            return Err("minimization cancelled".into());
        }
        iter += 1;
        let chunk = current.len().div_ceil(n);
        let mut reduced = false;
        for i in 0..n {
            let start = i * chunk;
            if start >= current.len() {
                break;
            }
            let end = (start + chunk).min(current.len());
            let candidate = current[..start]
                .iter()
                .chain(&current[end..])
                .cloned()
                .collect::<Vec<_>>();
            if candidate.is_empty() {
                continue;
            }
            let save = opt.all && opt.trace.is_some();
            let dir = if save {
                let parent = opt.work.join("candidates");
                std::fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
                loop {
                    serial += 1;
                    let p = parent.join(format!("cand_{serial:05}"));
                    match std::fs::create_dir(&p) {
                        Ok(()) => break p,
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(e.to_string()),
                    }
                }
            } else {
                opt.work.clone()
            };
            let file = dir.join("candidate.c");
            io::write_utf8(&file, &candidate.concat())?;
            let r = run_child(&opt, &file, &dir, save);
            let hit = r.code != 0
                && (opt.quick
                    || id.is_empty()
                    || r.stderr.to_lowercase().contains(&id.to_lowercase()));
            if save {
                if hit {
                    logs(&dir, &r)?;
                } else {
                    let root = std::fs::canonicalize(opt.work.join("candidates"))
                        .map_err(|e| e.to_string())?;
                    let path = std::fs::canonicalize(&dir).map_err(|e| e.to_string())?;
                    if path.parent() != Some(root.as_path()) {
                        return Err("candidate cleanup escaped owned directory".into());
                    }
                    std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
                }
            }
            if hit {
                current = candidate;
                reduced = true;
                n = n.saturating_sub(1).max(2);
                if !session.hosted {
                    println!("[minimize] reduced: lines={} (iter {iter})", current.len());
                }
                break;
            }
        }
        if !reduced {
            if n >= current.len() {
                break;
            }
            n = (n * 2).min(current.len());
        }
    }
    io::write_utf8(&opt.out, &current.concat())?;
    let report = opt.work.join("minimize_report.txt");
    io::write_utf8(
        &report,
        &format!(
            "--minimize report\ntarget: {}\noutput: {}\norig_lines: {count}\nmin_lines: {}\nfingerprint: {id}\n",
            opt.target.display(),
            opt.out.display(),
            current.len()
        ),
    )?;
    session.remember("minimized_source", &opt.out);
    session.remember("minimize_report", &report);
    if !session.hosted {
        println!(
            "[minimize] wrote: {}\n[minimize] report: {}",
            opt.out.display(),
            report.display()
        );
    }
    if opt.trace.is_some() {
        let dir = opt.work.join("final");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let r = run_child(&opt, &opt.out, &dir, true);
        logs(&dir, &r)?;
        io::write_bytes(
            dir.join(
                opt.out
                    .file_name()
                    .ok_or("minimized output has no filename")?,
            ),
            &std::fs::read(&opt.out).map_err(|e| e.to_string())?,
        )?;
        if let Some(dest) = &opt.user_trace {
            if let Err(e) = copy_tree(&dir.join("trace"), dest) {
                session.warning(&format!("trace bundling failed: {e}"));
            }
        }
        session.remember("minimize_trace_bundle", &dir);
        if !session.hosted {
            println!("[minimize] trace bundle: {}", dir.display());
        }
    }
    Ok(())
}
