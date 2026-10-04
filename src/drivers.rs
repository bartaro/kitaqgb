//! Dependency-aware development loops with invocation-local cancellation.
use crate::{cli, io, observation::Session, workflow};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};
#[derive(Clone, Default, Debug)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
pub(crate) fn sources(args: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if matches!(
            a.as_str(),
            "-o" | "-I"
                | "--stack-bank"
                | "--stack-top"
                | "--stack-reserve"
                | "--emit-ai-metadata"
                | "--emit-path-manifest"
        ) {
            i += 2;
            continue;
        }
        if !a.starts_with('-') && a != "compile" {
            files.push(PathBuf::from(a));
        }
        i += 1;
    }
    files
}
pub(crate) fn includes(args: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-I" {
            if let Some(v) = args.get(i + 1) {
                out.push(v.into())
            }
            i += 1;
        } else if let Some(v) = a
            .strip_prefix("-I")
            .filter(|s| !s.is_empty())
            .or_else(|| a.strip_prefix("--include-dir="))
        {
            out.push(v.into());
        }
        i += 1;
    }
    out
}
fn compile(args: &[String], outer: &mut Session) -> i32 {
    let mut child = Session::default();
    child.hosted = true;
    let code = cli::invoke(args.to_vec(), &mut child);
    if !outer.hosted {
        for d in &child.diagnostics {
            if d.has_position {
                eprint!("{} ", d.position);
            }
            eprintln!(
                "{} KQ{:04}: {}",
                if d.severity == crate::tokenizer::Severity::Error {
                    "error"
                } else {
                    "warning"
                },
                d.code,
                d.message
            );
        }
    }
    outer.output = child.output;
    outer.dependencies = child.dependencies;
    outer.artifacts = child.artifacts;
    outer.diagnostics = child.diagnostics;
    code
}
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    size: u64,
    time: Option<SystemTime>,
    digest: String,
}
fn stamp(path: &Path, content: bool) -> Option<Stamp> {
    std::fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| Stamp {
            size: m.len(),
            time: m.modified().ok(),
            digest: if content {
                crate::hash::file(path)
            } else {
                String::new()
            },
        })
}
fn stopped(s: &Session) -> bool {
    s.cancellation_token
        .as_ref()
        .is_some_and(CancellationToken::is_cancelled)
}
fn wait(s: &Session, ms: u64) -> bool {
    let until = std::time::Instant::now() + Duration::from_millis(ms);
    while !stopped(s) {
        let now = std::time::Instant::now();
        if now >= until {
            return true;
        }
        std::thread::sleep((until - now).min(Duration::from_millis(50)));
    }
    false
}
fn snapshot(files: &BTreeSet<PathBuf>, content: bool) -> BTreeMap<PathBuf, Option<Stamp>> {
    files
        .iter()
        .map(|p| (p.clone(), stamp(p, content)))
        .collect()
}
fn recursively_list(dir: &Path, files: &mut BTreeSet<PathBuf>) {
    if let Ok(items) = std::fs::read_dir(dir) {
        for entry in items.flatten() {
            if let Ok(kind) = entry.file_type() {
                if kind.is_dir() {
                    if !matches!(
                        entry.file_name().to_string_lossy().as_ref(),
                        "target" | ".git" | ".kitaqgb_cache"
                    ) {
                        recursively_list(&entry.path(), files);
                    }
                } else if kind.is_file() {
                    let p = entry.path();
                    if p.extension().is_some_and(|e| {
                        matches!(
                            e.to_string_lossy().to_lowercase().as_str(),
                            "c" | "h" | "inc"
                        )
                    }) {
                        files.insert(p);
                    }
                }
            }
        }
    }
}
pub fn try_run(args: &[String], session: &mut Session) -> Option<Result<(), String>> {
    if args
        .first()
        .is_some_and(|s| s.eq_ignore_ascii_case("devserver"))
    {
        Some(devserver(args, session))
    } else if args.iter().any(|s| s.eq_ignore_ascii_case("--watch")) {
        Some(watch(args, session))
    } else if args.first().is_some_and(|s| s.eq_ignore_ascii_case("test")) {
        Some(test(args, session))
    } else {
        None
    }
}
fn watch(args: &[String], session: &mut Session) -> Result<(), String> {
    let mut limit = 0usize;
    let mut kept = Vec::new();
    for a in args {
        if a.eq_ignore_ascii_case("--watch") {
            continue;
        }
        if let Some(v) = a.strip_prefix("--watch-max-builds=") {
            limit = v
                .parse()
                .ok()
                .filter(|v| *v > 0)
                .ok_or("--watch-max-builds must be positive")?;
        } else {
            kept.push(a.clone());
        }
    }
    let files = sources(&kept);
    if files.is_empty() {
        return Err("--watch requires at least one source file".into());
    }
    let mut dirs = BTreeSet::new();
    for source in &files {
        if let Some(dir) = workflow::full(source).parent() {
            if dir.is_dir() {
                dirs.insert(dir.to_owned());
            }
        }
    }
    for dir in includes(&kept) {
        let dir = workflow::full(dir);
        if dir.is_dir() {
            dirs.insert(dir);
        }
    }
    let mut builds = 0;
    while !stopped(session) {
        let code = compile(&kept, session);
        builds += 1;
        if !session.hosted {
            println!("[watch] build exit code: {code}");
        }
        if limit > 0 && builds >= limit {
            break;
        }
        let collect = || {
            let mut files = BTreeSet::new();
            for dir in &dirs {
                recursively_list(dir, &mut files);
            }
            files
        };
        let before = snapshot(&collect(), true);
        if !session.hosted {
            println!("[watch] waiting for file changes...");
        }
        loop {
            if !wait(session, 200) {
                break;
            }
            if snapshot(&collect(), true) != before {
                wait(session, 180);
                break;
            }
        }
    }
    session.exit_code = Some(0);
    Ok(())
}
fn devserver(args: &[String], session: &mut Session) -> Result<(), String> {
    let (mut poll, mut debounce, mut once, mut max, mut deps) =
        (250u64, 180u64, false, 0usize, String::new());
    let mut kept = Vec::new();
    for a in &args[1..] {
        if a.trim().is_empty() {
            continue;
        }
        if a == "--once" {
            once = true;
        } else if let Some(v) = a.strip_prefix("--poll-ms=") {
            poll = v
                .parse()
                .ok()
                .filter(|n| (50..=5000).contains(n))
                .ok_or("--poll-ms must be 50..5000")?;
        } else if let Some(v) = a.strip_prefix("--debounce-ms=") {
            debounce = v
                .parse()
                .ok()
                .filter(|n| *n <= 3000)
                .ok_or("--debounce-ms must be 0..3000")?;
        } else if let Some(v) = a.strip_prefix("--max-builds=") {
            max = v
                .parse()
                .ok()
                .filter(|n| *n <= 100000)
                .ok_or("--max-builds must be 0..100000")?;
        } else if let Some(v) = a
            .strip_prefix("--deps=")
            .or_else(|| a.strip_prefix("--deps-out="))
        {
            deps = v.into();
        } else if a != "--watch" {
            kept.push(a.clone());
        }
    }
    if kept.is_empty() {
        return Err("devserver requires compile args".into());
    }
    if deps.trim().is_empty() {
        deps = workflow::repo_root()
            .join("integration_test/reports/devserver.deps.txt")
            .display()
            .to_string();
    }
    deps = workflow::full(deps).display().to_string();
    if !kept
        .iter()
        .any(|a| matches!(a.as_str(), "--fast-build" | "--fast" | "--no-disasm"))
    {
        kept.push("--fast-build".into());
    }
    if !kept
        .iter()
        .any(|a| matches!(a.as_str(), "--cache" | "--no-cache"))
    {
        kept.push("--cache".into());
    }
    if !kept
        .iter()
        .any(|a| a == "--deps-out" || a.starts_with("--deps-out="))
    {
        kept.push(format!("--deps-out={deps}"));
    }
    let mut count = 0;
    let mut last = 0;
    while !stopped(session) {
        count += 1;
        if !session.hosted {
            println!("[devserver] build #{count}");
        }
        last = compile(&kept, session);
        if !session.hosted {
            println!("[devserver] exit={last}");
        }
        if once || (max > 0 && count >= max) || stopped(session) {
            break;
        }
        let mut files = sources(&kept)
            .into_iter()
            .map(workflow::full)
            .collect::<BTreeSet<_>>();
        if let Ok(text) = io::read_utf8(&deps) {
            for line in text
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.starts_with('#'))
            {
                files.insert(workflow::full(line));
            }
        }
        let before = snapshot(&files, false);
        if !session.hosted {
            println!("[devserver] watching {} file(s)", files.len());
        }
        loop {
            if !wait(session, poll) {
                break;
            }
            let now = snapshot(&files, false);
            let changed = files
                .iter()
                .filter(|f| now.get(*f) != before.get(*f))
                .take(10)
                .collect::<Vec<_>>();
            if !changed.is_empty() {
                if debounce > 0 {
                    wait(session, debounce);
                }
                if !session.hosted {
                    println!(
                        "[devserver] changed: {}",
                        changed
                            .iter()
                            .map(|p| p.file_name().unwrap_or_default().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                break;
            }
        }
    }
    session.exit_code = Some(last);
    Ok(())
}
fn test(args: &[String], session: &mut Session) -> Result<(), String> {
    let root = workflow::repo_root();
    let scripts = root.join("integration_test/scripts");
    let mut cmd = if cfg!(windows) && scripts.join("run_all.ps1").is_file() {
        let mut c = std::process::Command::new("powershell");
        c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(scripts.join("run_all.ps1"));
        c
    } else if scripts.join("run_all.sh").is_file() {
        let mut c = std::process::Command::new("sh");
        c.arg(scripts.join("run_all.sh"));
        c
    } else if root.join("Cargo.toml").is_file() {
        let mut c = std::process::Command::new("cargo");
        c.args(["test", "--locked"]);
        c
    } else {
        return Err("integration_test/scripts/run_all.ps1 not found".into());
    };
    cmd.args(&args[1..]).current_dir(root);
    let code = cmd.status().map_err(|e| e.to_string())?.code().unwrap_or(1);
    session.exit_code = Some(code);
    Ok(())
}
