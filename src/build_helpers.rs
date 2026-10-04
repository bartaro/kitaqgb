//! Isolated compiler invocations for ABI and reproducibility comparisons.
use crate::{hash, io};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::{Path, PathBuf},
};
pub fn is_analysis(arg: &str) -> bool {
    matches!(
        arg.split('=').next().unwrap_or(arg),
        "--bank-sim"
            | "--farcall-suggest"
            | "--cross-bank-report"
            | "--abi-verify"
            | "--abi-diff-report"
            | "--rst-report"
            | "--opt-diff"
            | "--func-size-report"
            | "--hotspot-report"
            | "--repro-check"
            | "--cgb-consistency"
            | "--verify-cgb-symbols"
    )
}
struct Scratch(PathBuf);
impl Scratch {
    fn new(report: &Path, kind: &str) -> Result<Self, String> {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let parent = report
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let dir = parent.join(format!(
            "kitaqgb_{kind}_{}_{time}_{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
        Ok(Self(dir))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn child(args: &[String], output: &Path, abi: &str) -> Result<i32, String> {
    let mut kept = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-o" {
            i += 2;
            continue;
        }
        if a.starts_with("--abi=")
            || matches!(
                a.as_str(),
                "--cache" | "--no-cache" | "--watch" | "--machine-readable"
            )
            || a.starts_with("--minimize")
            || a.starts_with("--auto-minimize")
            || a.starts_with("--disasm-changed")
            || a.starts_with("--repro-pack")
            || is_analysis(a)
        {
            i += 1;
            continue;
        }
        // Do not overwrite a parent's explicit diagnostic or metadata path during a comparison.
        if matches!(a.as_str(), "--emit-ai-metadata" | "--emit-path-manifest") {
            i += 2;
            continue;
        }
        if a.starts_with("--diag-json")
            || a.starts_with("--emit-ai-metadata=")
            || a.starts_with("--emit-path-manifest=")
            || a.starts_with("--debug-out")
            || a.starts_with("--trace")
        {
            i += 1;
            continue;
        }
        kept.push(a.clone());
        i += 1;
    }
    kept.extend([
        "--no-cache".into(),
        "--no-disasm".into(),
        "--no-debug-output".into(),
        "--no-trace".into(),
        format!("--abi={abi}"),
        "-o".into(),
        output.display().to_string(),
    ]);
    let mut session = crate::observation::Session::default();
    session.hosted = true;
    Ok(crate::cli::invoke(kept, &mut session))
}
fn function_sizes(path: &Path) -> BTreeMap<String, i32> {
    let mut map = BTreeMap::new();
    if let Ok(text) = io::read_utf8(path) {
        for line in text.lines().filter(|s| !s.starts_with('#')) {
            let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
            if parts.len() >= 6 {
                if let Ok(size) = parts[5].parse() {
                    map.insert(parts[0].into(), size);
                }
            }
        }
    }
    map
}
pub fn abi_diff(args: &[String], report: &Path) -> Result<(), String> {
    let scratch = Scratch::new(report, "abidiff")?;
    let legacy = scratch.0.join("legacy.gb");
    let stack = scratch.0.join("stack.gb");
    let le = child(args, &legacy, "legacy")?;
    let se = child(args, &stack, "stack")?;
    let mut text = format!("# KITAQGB ABI diff report\nlegacy_exit={le}\nstack_exit={se}\n");
    for (name, path, code) in [("legacy", &legacy, le), ("stack", &stack, se)] {
        if code == 0 && path.is_file() {
            writeln!(
                text,
                "{name}_size={}",
                std::fs::metadata(path).map_err(|e| e.to_string())?.len()
            )
            .unwrap();
            writeln!(text, "{name}_sha256={}", hash::file(path)).unwrap();
        }
    }
    let l = function_sizes(&legacy.with_extension("funcsizes.txt"));
    let s = function_sizes(&stack.with_extension("funcsizes.txt"));
    let all = l.keys().chain(s.keys()).cloned().collect::<BTreeSet<_>>();
    let mut rows = all
        .into_iter()
        .map(|name| {
            let left = *l.get(&name).unwrap_or(&0);
            let right = *s.get(&name).unwrap_or(&0);
            (name, left, right, right - left)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.3.abs()
            .cmp(&a.3.abs())
            .then(a.0.encode_utf16().cmp(b.0.encode_utf16()))
    });
    text.push_str("\n# function size delta (stack - legacy)\n# name, legacy, stack, delta\n");
    for (name, l, s, d) in rows {
        writeln!(text, "{name}, {l}, {s}, {d}").unwrap();
    }
    io::write_utf8(report, &text)?;
    if le != 0 || se != 0 {
        Err(format!(
            "ABI diff report generation failed (legacy={le}, stack={se}). See: {}",
            report.display()
        ))
    } else {
        Ok(())
    }
}
pub fn reproducibility(
    args: &[String],
    output: &Path,
    report: &Path,
    stack: bool,
) -> Result<(), String> {
    let scratch = Scratch::new(report, "repro")?;
    let repro = scratch.0.join("repro.gb");
    let code = child(args, &repro, if stack { "stack" } else { "legacy" })?;
    let current_rom = hash::file(output);
    let repro_rom = hash::file(&repro);
    // Companion text uses native platform line endings in the C# baseline.
    let current_map = hash::file(output.with_extension("map"));
    let repro_map = hash::file(repro.with_extension("map"));
    let same_rom = !current_rom.is_empty() && current_rom == repro_rom;
    let same_map = !current_map.is_empty() && current_map == repro_map;
    let pass = code == 0 && same_rom && same_map;
    io::write_utf8(
        report,
        &format!(
            "# KITAQGB reproducibility check\nexit={code}\nsame_gb={}\nsame_map={}\ncurrent_gb_sha256={current_rom}\nrepro_gb_sha256={repro_rom}\ncurrent_map_sha256={current_map}\nrepro_map_sha256={repro_map}\nresult={}\n",
            i32::from(same_rom),
            i32::from(same_map),
            if pass { "PASS" } else { "FAIL" }
        ),
    )?;
    if pass {
        Ok(())
    } else {
        Err(format!(
            "Reproducibility check failed. See: {}",
            report.display()
        ))
    }
}
