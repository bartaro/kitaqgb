use kitaqgb::{cli, io, json::Value, observation::Session, workflow};
use std::{fs, path::PathBuf};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "kitaqgb-recovery-{}-{t}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).display().to_string()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(args: Vec<String>) -> (i32, Session) {
    let mut s = Session::default();
    s.hosted = true;
    let code = cli::invoke(args, &mut s);
    (code, s)
}
#[test]
fn minimization_preserves_failure_and_original_outputs() {
    let t = Scratch::new();
    let source = t.path("bug.c");
    let out = t.path("existing.gb");
    let text = "// before\r\nu8 unused;\r\n#error deliberate\r\nvoid main() {}\r\n// after\r\n";
    io::write_utf8(&source, text).unwrap();
    fs::write(&out, b"preserve").unwrap();
    let args = vec![
        source.clone(),
        "-o".into(),
        out.clone(),
        "--minimize".into(),
        format!("--minimize-work={}", t.path("work")),
        "--trace=tokens".into(),
        format!("--trace-out={}", t.path("trace")),
    ];
    let (code, s) = run(args);
    assert_eq!(code, 0);
    let minimized = fs::read_to_string(&s.artifacts["minimized_source"]).unwrap();
    assert_eq!(minimized, "#error deliberate\r\n");
    assert_eq!(fs::read(&out).unwrap(), b"preserve");
    assert_eq!(fs::read_to_string(&source).unwrap(), text);
    assert_eq!(
        fs::read_to_string(t.path("work/final/run_exitcode.txt")).unwrap(),
        "1"
    );
    assert!(PathBuf::from(t.path("trace/tokens.txt")).is_file());
    assert!(!PathBuf::from(t.path("work/trace")).exists());
}
#[test]
fn all_candidate_traces_retain_only_failures() {
    let t = Scratch::new();
    io::write_utf8(
        t.path("bug.c"),
        "// before\n#error deliberate\nvoid main(){}\n// after\n",
    )
    .unwrap();
    let (code, _) = run(vec![
        t.path("bug.c"),
        "--minimize".into(),
        format!("--minimize-work={}", t.path("work")),
        "--trace=tokens".into(),
        "--minimize-trace=all".into(),
    ]);
    assert_eq!(code, 0);
    let dirs = fs::read_dir(t.path("work/candidates"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(!dirs.is_empty());
    for d in dirs {
        assert_eq!(
            fs::read_to_string(d.path().join("run_exitcode.txt")).unwrap(),
            "1"
        );
        assert!(
            fs::read_to_string(d.path().join("run_stderr.txt"))
                .unwrap()
                .contains("KQ")
        );
        assert!(d.path().join("trace/tokens.txt").is_file());
    }
}
#[test]
fn failure_package_snapshots_resolved_dependencies_and_auto_minimizes() {
    let t = Scratch::new();
    io::write_utf8(
        t.path("bug.c"),
        "#include \"broken.h\"\n#error deliberate\nvoid main(){}\n",
    )
    .unwrap();
    io::write_utf8(t.path("broken.h"), "u8 unused;\n").unwrap();
    let (code, s) = run(vec![
        t.path("bug.c"),
        "-o".into(),
        t.path("game.gb"),
        format!("--repro-pack={}", t.path("pack")),
        "--auto-minimize".into(),
        format!("--minimize-work={}", t.path("work")),
    ]);
    assert_eq!(code, 1);
    assert!(s.errors() > 0);
    let Value::Object(files) =
        Value::parse(&fs::read_to_string(t.path("pack/inputs.json")).unwrap()).unwrap()
    else {
        panic!()
    };
    assert_eq!(files.len(), 2);
    assert!(files.keys().any(|k| k.ends_with("broken.h")));
    for v in files.values() {
        let Value::String(rel) = v else { panic!() };
        assert!(t.0.join("pack/inputs").join(rel).is_file());
    }
    assert_eq!(
        fs::read_to_string(t.path("bug.min.c")).unwrap(),
        "#error deliberate\n"
    );
}
#[test]
fn recipe_sorts_history_and_quotes_replay_arguments() {
    let t = Scratch::new();
    let source = t.path("apostrophe's.c");
    io::write_utf8(&source, "void main(){}\n").unwrap();
    let args = [
        source,
        "-o".into(),
        t.path("replay.gb"),
        "--title=$x;literal".into(),
        "--no-cache".into(),
    ];
    let line = args
        .iter()
        .map(|a| workflow::quote_arg(a))
        .collect::<Vec<_>>()
        .join(" ");
    let dir = t.0.display().to_string();
    let history = format!(
        "2026-01-02T09:00:00+09:00\t{dir}\t{line}\ninvalid\t{dir}\ta.c\n2026-01-01T00:00:00Z\t{dir}\told.c\n"
    );
    io::write_utf8(t.path("history.log"), &history).unwrap();
    let (code, _) = run(vec![
        "recipe".into(),
        format!("--history={}", t.path("history.log")),
        format!("--out={}", t.path("recipe.md")),
        format!("--script={}", t.path("replay.ps1")),
    ]);
    assert_eq!(code, 0);
    let md = fs::read_to_string(t.path("recipe.md")).unwrap();
    assert!(md.contains("history_entries=2"));
    assert!(md.contains("2026-01-02T00:00:00.0000000Z"));
    assert!(md.contains(&format!(
        "### Compile\n- cwd: `{dir}`\n- cmd: `kitaqgb {line}`"
    )));
    let ps = fs::read_to_string(t.path("replay.ps1")).unwrap();
    assert!(ps.contains("apostrophe''s.c"));
    assert!(ps.contains("$startInfo.UseShellExecute = $false"));
    let sh = fs::read_to_string(t.path("replay.sh")).unwrap();
    assert!(sh.contains("apostrophe'\\''s.c"));
    assert!(!sh.contains("eval"));
    #[cfg(windows)]
    {
        let code = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &t.path("replay.ps1"),
                "-Compiler",
                env!("CARGO_BIN_EXE_kitaqgb"),
            ])
            .status()
            .unwrap();
        assert!(code.success());
        assert!(t.0.join("replay.gb").is_file());
    }
}
#[test]
fn changed_disassembly_and_summary_cover_emitted_ranges() {
    let t = Scratch::new();
    io::write_utf8(
        t.path("functions.c"),
        "u8 result;\nu8 helper(){return 7;}\nvoid main(){result=helper();}\n",
    )
    .unwrap();
    let (code, s) = run(vec![
        t.path("functions.c"),
        "-o".into(),
        t.path("game.gb"),
        format!("--debug-out={}", t.path("debug")),
        "--disasm-changed=nonexistent-ref".into(),
        format!("--trace-out={}", t.path("trace")),
        "--no-cache".into(),
    ]);
    assert_eq!(code, 0);
    let listing = fs::read_to_string(&s.artifacts["changed_disasm"]).unwrap();
    assert!(listing.contains("; emitted_functions=2\n"));
    assert!(listing.contains("; function main  file_off=$"));
    assert!(listing.contains("; function helper  file_off=$"));
    assert!(listing.contains("; file_off=$"));
    let summary = fs::read_to_string(t.path("trace/trace_summary.txt")).unwrap();
    let stages = summary
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once(", ").map(|p| p.0))
        .collect::<Vec<_>>();
    assert_eq!(
        stages,
        [
            "parse",
            "lower",
            "codegen",
            "assemble",
            "header",
            "disasm",
            "disasm_changed",
            "reports"
        ]
    );
    assert!(summary.contains("asm node(s)"));
    let (code, s) = run(vec![
        t.path("functions.c"),
        "-o".into(),
        t.path("game.gb"),
        format!("--debug-out={}", t.path("disabled")),
        "--disasm-changed".into(),
        "--no-disasm".into(),
        "--no-cache".into(),
    ]);
    assert_eq!(code, 0);
    assert!(
        s.diagnostics
            .iter()
            .any(|d| d.message.contains("disassembly is disabled"))
    );
    assert!(!PathBuf::from(t.path("disabled/dis_changed.s")).exists());
}
#[test]
fn alternate_rom_output_is_reported_without_using_a_stale_target() {
    use kitaqgb::compiler_api::{CompileRequest, IKitaqgbCompiler, KitaqgbCompiler};
    let t = Scratch::new();
    let source = t.path("main.c");
    io::write_utf8(&source, "void main(){}\n").unwrap();
    let target = t.0.join("locked.gb");
    #[cfg(windows)]
    let locked = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::write(&target, b"original").unwrap();
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .unwrap()
    };
    #[cfg(not(windows))]
    fs::create_dir(&target).unwrap();
    let result = KitaqgbCompiler
        .compile(&CompileRequest {
            source_files: vec![source],
            output_filename: target.display().to_string(),
            extra_args: vec!["--no-cache".into()],
            ..Default::default()
        })
        .unwrap();
    assert!(result.success);
    assert_ne!(PathBuf::from(&result.output_filename), target);
    assert!(result.output_filename.contains(".lockretry_"));
    assert!(
        PathBuf::from(&result.output_filename)
            .with_extension("map")
            .is_file()
    );
    #[cfg(windows)]
    {
        drop(locked);
        assert_eq!(fs::read(&target).unwrap(), b"original");
    }
    #[cfg(not(windows))]
    assert!(target.is_dir());
}
#[test]
fn locked_companions_and_manifest_use_the_actual_paths() {
    use kitaqgb::compiler_api::{CompileRequest, IKitaqgbCompiler, KitaqgbCompiler};
    let t = Scratch::new();
    let source = t.path("main.c");
    io::write_utf8(&source, "u8 value;void main(){value=7;}\n").unwrap();
    let names = [
        "game.map",
        "game.dbc",
        "game.source_map.txt",
        "diagnostics.json",
    ];
    #[cfg(windows)]
    let handles = {
        use std::os::windows::fs::OpenOptionsExt;
        names
            .iter()
            .map(|name| {
                let path = t.0.join(name);
                fs::write(&path, b"preserve").unwrap();
                fs::OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(path)
                    .unwrap()
            })
            .collect::<Vec<_>>()
    };
    #[cfg(not(windows))]
    for name in names {
        fs::create_dir(t.0.join(name)).unwrap();
    }
    let result = KitaqgbCompiler
        .compile(&CompileRequest {
            source_files: vec![source],
            output_filename: t.path("game.gb"),
            extra_args: vec![
                "--no-cache".into(),
                format!("--diag-json={}", t.path("diagnostics.json")),
                format!("--emit-path-manifest={}", t.path("paths.json")),
            ],
            ..Default::default()
        })
        .unwrap();
    assert!(result.success, "{:?}", result.diagnostics);
    for (key, name) in ["map", "dbc", "source_map", "diag_json"]
        .into_iter()
        .zip(names)
    {
        let path = &result.output_artifacts[key];
        assert_ne!(path, &t.path(name));
        assert!(path.contains(".lockretry_"));
        assert!(PathBuf::from(path).is_file());
    }
    let dbc_name = PathBuf::from(&result.output_artifacts["dbc"])
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        fs::read_to_string(&result.output_artifacts["dbg"])
            .unwrap()
            .contains(&dbc_name)
    );
    let paths = Value::parse(&fs::read_to_string(t.path("paths.json")).unwrap()).unwrap();
    let Value::Object(paths) = paths else {
        panic!("object required")
    };
    assert_eq!(
        paths["map"],
        Value::String(result.output_artifacts["map"].clone())
    );
    #[cfg(windows)]
    {
        drop(handles);
        for name in names {
            assert_eq!(fs::read(t.path(name)).unwrap(), b"preserve");
        }
    }
    #[cfg(not(windows))]
    for name in names {
        assert!(t.0.join(name).is_dir());
    }
}
#[test]
fn a_locked_template_source_is_used_by_its_generated_scripts() {
    let t = Scratch::new();
    let source = t.0.join("sample.c");
    #[cfg(windows)]
    let handle = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::write(&source, b"preserve").unwrap();
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&source)
            .unwrap()
    };
    #[cfg(not(windows))]
    fs::create_dir(&source).unwrap();
    let (code, _) = run(vec![
        "template".into(),
        source.display().to_string(),
        "--overwrite".into(),
    ]);
    assert_eq!(code, 0);
    let actual = fs::read_dir(&t.0)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "c") && p.is_file() && p != &source)
        .unwrap();
    let stem = actual.file_stem().unwrap().to_string_lossy();
    let ps = fs::read_to_string(t.0.join(format!("{stem}_build.ps1"))).unwrap();
    let sh = fs::read_to_string(t.0.join(format!("{stem}_build.sh"))).unwrap();
    let filename = actual.file_name().unwrap().to_string_lossy();
    assert!(ps.contains(filename.as_ref()));
    assert!(sh.contains(filename.as_ref()));
    #[cfg(windows)]
    {
        drop(handle);
        assert_eq!(fs::read(source).unwrap(), b"preserve");
    }
}
