use kitaqgb::compiler_api::{CompileRequest, IKitaqgbCompiler, KitaqgbCompiler};
use std::{fs, path::PathBuf};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kitaqgb-api-{}-{time}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn request(&self, name: &str, source: &str) -> CompileRequest {
        let input = self.0.join(format!("{name}.c"));
        fs::write(&input, source).unwrap();
        CompileRequest {
            source_files: vec![input.display().to_string()],
            output_filename: self.0.join(format!("{name}.gb")).display().to_string(),
            extra_args: vec![
                "--no-cache".into(),
                "--no-disasm".into(),
                "--cart=romonly".into(),
                "--romsize=32k".into(),
            ],
            ..Default::default()
        }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn request_arguments_preserve_order_and_default_paths() {
    assert!(CompileRequest::default().command_line_args().is_err());
    let req = CompileRequest {
        source_files: vec![" ".into(), "space dir/main.c".into()],
        include_directories: vec![" ".into(), "space dir/include".into()],
        output_filename: "space dir/game.gb".into(),
        strict_diagnostics: true,
        enable_debug_output: true,
        trace_stages: vec![" ".into(), "ir".into(), "asm".into()],
        emit_path_manifest: true,
        stack_top: Some(0xcfff),
        stack_reserve: Some(32),
        extra_args: vec![" ".into(), "--permissive".into()],
        ..Default::default()
    };
    assert_eq!(
        req.command_line_args().unwrap(),
        [
            "space dir/main.c",
            "-o",
            "space dir/game.gb",
            "-I",
            "space dir/include",
            "--strict",
            "--debug-out",
            "--trace=ir,asm",
            "--emit-path-manifest=space dir/game.artifacts.json",
            "--machine-readable",
            "--no-banner",
            "--stack-top=0xCFFF",
            "--stack-reserve=32",
            "--permissive"
        ]
    );
}
#[test]
fn invalid_compound_inputs_report_reference_codes_and_remain_hosted() {
    let temp = Scratch::new();
    let host = KitaqgbCompiler;
    for (name, source, code, message) in [
        (
            "aggregate-compound",
            "struct S {u8 a;};struct S x;struct S y;void main(){x+=y;}",
            "KQ1000",
            "compound assignment is not supported for struct/union types",
        ),
        (
            "aggregate-scalar",
            "struct S {u8 a;};struct S x;u8 y;void main(){y=x;}",
            "KQ1000",
            "incompatible struct/union assignment: uint8 = struct S",
        ),
        (
            "unsupported-expression",
            "u16 a;u16 b;void main(){a=(b=300);}",
            "KQ0000",
            "Not Implemented: Expression too complex for CompileIntoHL",
        ),
        (
            "undefined-function",
            "void main(){missing();}",
            "KQ0000",
            "Undefined function: missing",
        ),
        (
            "wrong-arity",
            "void f(u8 a){}void main(){f();}",
            "KQ0000",
            "Wrong number of arguments. Expected 1, got 0.",
        ),
    ] {
        let request = temp.request(name, source);
        let result = host.compile(&request).unwrap();
        assert!(!result.success && result.exit_code == 1, "{name}: {result:?}");
        assert!(!std::path::Path::new(&result.output_filename).exists());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == "error" && diagnostic.code == code && diagnostic.message == message
        }), "{name}: {:?}", result.diagnostics);
    }
    assert!(host.compile(&temp.request("recovered", "void main(){}")).unwrap().success);
}
#[test]
fn hosted_invocations_do_not_leak_diagnostics_or_accept_stale_output() {
    let temp = Scratch::new();
    let host = KitaqgbCompiler;
    let mut req = temp.request("game", "u8 result; void main(){result=7;}");
    req.emit_diag_json = true;
    req.diag_json_path = Some(temp.0.join("diag.json").display().to_string());
    req.emit_path_manifest = true;
    let good = host.compile(&req).unwrap();
    assert!(good.success);
    assert!(good.diagnostics.is_empty());
    for role in [
        "output_rom",
        "diag_json",
        "dbg2_json",
        "build_report_json",
        "path_manifest",
    ] {
        assert!(good.output_artifacts.contains_key(role), "{role}");
    }
    let old = fs::read(&good.output_filename).unwrap();
    fs::write(
        &req.source_files[0],
        "#error deliberate failure\nvoid main(){}\n",
    )
    .unwrap();
    let bad = host.compile(&req).unwrap();
    assert!(!bad.success);
    assert_eq!(bad.exit_code, 1);
    assert_eq!(fs::read(&bad.output_filename).unwrap(), old);
    assert!(
        bad.diagnostics
            .iter()
            .any(|d| d.severity == "error" && d.message.contains("deliberate failure"))
    );
    fs::write(&req.source_files[0], "u8 result; void main(){result=9;}").unwrap();
    let recovered = host.compile(&req).unwrap();
    assert!(recovered.success);
    assert!(recovered.diagnostics.is_empty());
    assert_ne!(fs::read(&recovered.output_filename).unwrap(), old);
}
#[test]
fn independent_api_sessions_compile_concurrently() {
    let jobs = (0..4)
        .map(|n| {
            std::thread::spawn(move || {
                let temp = Scratch::new();
                let mut req = temp.request(
                    "game",
                    "__must_check u8 answer(){return 7;}void main(){answer();}",
                );
                req.strict_diagnostics = n % 2 == 0;
                let result = KitaqgbCompiler.compile(&req).unwrap();
                assert_eq!(result.success, n % 2 != 0, "{:?}", result.diagnostics);
                assert!(result.diagnostics.iter().any(|d| d.code == "KQ2401"));
                assert_eq!(
                    result
                        .diagnostics
                        .iter()
                        .filter(|d| d.severity == "error")
                        .count(),
                    if n % 2 == 0 { 1 } else { 0 }
                );
            })
        })
        .collect::<Vec<_>>();
    for job in jobs {
        job.join().unwrap();
    }
}
#[test]
fn comparison_helpers_work_inside_the_api_host() {
    let temp = Scratch::new();
    let mut req = temp.request(
        "game",
        "u8 result;u8 twice(u8 n){return n+n;}void main(){result=twice(3);}",
    );
    let abi = temp.0.join("abi.txt");
    let repro = temp.0.join("repro.txt");
    req.extra_args.extend([
        format!("--abi-diff-report={}", abi.display()),
        format!("--repro-check={}", repro.display()),
    ]);
    let result = KitaqgbCompiler.compile(&req).unwrap();
    assert!(result.success, "{:?}", result.diagnostics);
    assert!(
        fs::read_to_string(abi)
            .unwrap()
            .contains("legacy_exit=0\nstack_exit=0")
    );
    assert!(fs::read_to_string(repro).unwrap().contains("result=PASS"));
    assert!(
        !fs::read_dir(&temp.0)
            .unwrap()
            .any(|e| e.unwrap().file_type().unwrap().is_dir())
    );
}
