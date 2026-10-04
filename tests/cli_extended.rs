use kitaqgb::{io, json::Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kitaqgb-cli-extended-{}-{unique}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    if let Value::Object(fields) = value {
        fields.get(key).unwrap()
    } else {
        panic!("expected object")
    }
}
fn strings(value: &Value) -> Vec<String> {
    if let Value::Array(values) = value {
        values
            .iter()
            .map(|v| {
                if let Value::String(s) = v {
                    s.clone()
                } else {
                    panic!("expected string")
                }
            })
            .collect()
    } else {
        panic!("expected array")
    }
}
fn load(path: impl AsRef<Path>) -> Value {
    Value::parse(&io::read_utf8(path).unwrap()).unwrap()
}
fn run(dir: &Path, args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kitaqgb"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
fn normalize(
    value: &mut Value,
    sources: &Path,
    output: &Path,
    debug: Option<&Path>,
    trace: Option<&Path>,
    diag: Option<&Path>,
) {
    match value {
        Value::String(s) => {
            *s = s.replace('\\', "/");
            for (old, new) in [(debug, "<DEBUG>"), (trace, "<TRACE>"), (diag, "<DIAG>")] {
                if let Some(path) = old {
                    *s = s.replace(&path.to_string_lossy().replace('\\', "/"), new);
                }
            }
            *s = s
                .replace(
                    &output
                        .with_extension("")
                        .display()
                        .to_string()
                        .replace('\\', "/"),
                    "<STEM>",
                )
                .replace(
                    &sources.display().to_string().replace('\\', "/"),
                    "<SOURCES>",
                );
        }
        Value::Array(values) => {
            for value in values {
                normalize(value, sources, output, debug, trace, diag)
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                normalize(value, sources, output, debug, trace, diag)
            }
        }
        _ => {}
    }
}
#[test]
fn rich_debug_build_and_handoff_json_match_csharp() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = base.join("tests/cli-fixtures");
    let sources = base.join("tests/emission-fixtures");
    for name in [
        "banked0",
        "banked1",
        "rst-safe2",
        "far-memory3",
        "aggregate-copies4",
        "constants-runtime5",
    ] {
        let expected = load(fixtures.join(format!("{name}.metadata.json")));
        let temp = Scratch::new();
        let output = temp.0.join("out.gb");
        let ai = temp.0.join("ai.json");
        let Value::String(fixture) = field(&expected, "fixture") else {
            panic!()
        };
        let mut args = vec![
            sources.join(fixture).display().to_string(),
            "-o".into(),
            output.display().to_string(),
            "--no-cache".into(),
            "--no-disasm".into(),
            "--cart=mbc5".into(),
            "--romsize=128k".into(),
            "--cgb=cgb".into(),
            "--stack-bank=fixed".into(),
            format!("--emit-ai-metadata={}", ai.display()),
        ];
        args.extend(strings(field(&expected, "flags")));
        let result = run(&temp.0, &args);
        assert!(
            result.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        for (key, path) in [
            ("dbg2", output.with_extension("dbg2.json")),
            ("build_report", output.with_extension("build_report.json")),
            ("ai", ai),
        ] {
            let mut actual = load(path);
            normalize(&mut actual, &sources, &output, None, None, None);
            assert_eq!(&actual, field(&expected, key), "{name}/{key}");
        }
    }
}
#[test]
fn traces_profiles_and_diagnostics_match_csharp() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cli-fixtures");
    for name in [
        "trace",
        "dev",
        "release",
        "test",
        "warning",
        "error",
        "unused",
        "unused-strict",
        "nodiscard",
        "nodiscard-strict",
        "const",
        "extern",
        "narrow",
        "syntax",
        "static_assert",
        "limited",
    ] {
        let expected = load(fixtures.join(format!("{name}.observation.json")));
        let temp = Scratch::new();
        let source = temp.0.join("main.c");
        fs::copy(fixtures.join(format!("{name}.c")), &source).unwrap();
        let output = temp.0.join("out.gb");
        let diag = temp.0.join("diag.json");
        let manifest = temp.0.join("paths.json");
        let debug = temp.0.join("debug");
        let trace = temp.0.join("trace");
        let flags = strings(field(&expected, "flags"));
        let mut args = vec![
            source.display().to_string(),
            "-o".into(),
            output.display().to_string(),
            "--cart=mbc5".into(),
            "--romsize=128k".into(),
            "--cgb=cgb".into(),
        ];
        args.extend(flags.clone());
        args.extend([
            "--no-cache".into(),
            "--machine-readable".into(),
            format!("--diag-json={}", diag.display()),
            format!("--emit-path-manifest={}", manifest.display()),
        ]);
        if flags.iter().any(|s| s == "--trace") {
            args.push(format!("--trace-out={}", trace.display()));
        }
        if flags.iter().any(|s| {
            matches!(
                s.as_str(),
                "--debug-output" | "--profile=dev" | "--profile=test"
            )
        }) {
            args.push(format!("--debug-out={}", debug.display()));
        }
        let result = run(&temp.0, &args);
        for (key, path) in [("diagnostics", &diag), ("paths", &manifest)] {
            let mut actual = load(path);
            normalize(
                &mut actual,
                &temp.0,
                &output,
                Some(&debug),
                Some(&trace),
                Some(&diag),
            );
            assert_eq!(
                &actual,
                field(&expected, key),
                "{name}/{key}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        assert_eq!(
            result.status.code().unwrap_or(1) as i64,
            if let Value::Number(n) = field(field(&expected, "diagnostics"), "exit_code") {
                *n
            } else {
                panic!()
            },
            "{name} exit"
        );
        let Value::Object(files) = field(&expected, "files") else {
            panic!()
        };
        for (relative, expected) in files {
            let (group, file) = relative.split_once('/').unwrap();
            let folder = if group == "debug" { &debug } else { &trace };
            let mut actual = Value::String(
                io::read_utf8(folder.join(file))
                    .unwrap()
                    .replace("\r\n", "\n")
                    .replace("out.gb", "<ROM>"),
            );
            normalize(
                &mut actual,
                &temp.0,
                &output,
                Some(&debug),
                Some(&trace),
                Some(&diag),
            );
            assert_eq!(&actual, expected, "{name}/{relative}");
        }
    }
}
#[test]
fn cache_reuses_valid_entries_and_invalidates_include_content() {
    let temp = Scratch::new();
    let source = temp.0.join("main.c");
    let header = temp.0.join("value.h");
    fs::write(
        &source,
        "#include \"value.h\"\nu8 value;void main(){value=VALUE;}\n",
    )
    .unwrap();
    fs::write(&header, "#define VALUE 7\n").unwrap();
    let args = vec![
        source.display().to_string(),
        "--cache".into(),
        "--cart=romonly".into(),
        "--romsize=32k".into(),
    ];
    let cold = run(&temp.0, &args);
    assert!(cold.status.success());
    let before = fs::read(temp.0.join("out.gb")).unwrap();
    fs::remove_file(temp.0.join("out.gb")).unwrap();
    let warm = run(&temp.0, &args);
    assert!(warm.status.success());
    assert!(String::from_utf8_lossy(&warm.stderr).contains("[cache] hit:"));
    assert_eq!(fs::read(temp.0.join("out.gb")).unwrap(), before);
    fs::write(&header, "#define VALUE 9\n").unwrap();
    let changed = run(&temp.0, &args);
    assert!(changed.status.success());
    assert!(!String::from_utf8_lossy(&changed.stderr).contains("[cache] hit:"));
    assert_ne!(fs::read(temp.0.join("out.gb")).unwrap(), before);

    let config = temp.0.join("header.json");
    fs::write(&config, r#"{"title":"FIRST"}"#).unwrap();
    let mut args = args;
    args.push(format!("--rom-header={}", config.display()));
    assert!(run(&temp.0, &args).status.success());
    let first = fs::read(temp.0.join("out.gb")).unwrap();
    fs::write(&config, r#"{"title":"SECOND"}"#).unwrap();
    let changed = run(&temp.0, &args);
    assert!(changed.status.success());
    assert!(!String::from_utf8_lossy(&changed.stderr).contains("[cache] hit:"));
    let second = fs::read(temp.0.join("out.gb")).unwrap();
    assert_ne!(first, second);
    assert_eq!(&second[0x134..0x13a], b"SECOND");
    let mut prefix = vec![args[0].clone(), "--rom-title=BEFORE".into()];
    prefix.extend(args.into_iter().skip(1));
    assert!(run(&temp.0, &prefix).status.success());
    assert_eq!(
        &fs::read(temp.0.join("out.gb")).unwrap()[0x134..0x13a],
        b"SECOND"
    );
    prefix.push("--rom-title=AFTER".into());
    assert!(run(&temp.0, &prefix).status.success());
    assert_eq!(
        &fs::read(temp.0.join("out.gb")).unwrap()[0x134..0x139],
        b"AFTER"
    );
}
