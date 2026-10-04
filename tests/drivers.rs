use kitaqgb::{
    compiler_api::{CompileRequest, IKitaqgbCompiler, KitaqgbCompiler},
    drivers::CancellationToken,
    observation::Session,
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};
struct Scratch(PathBuf);
struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl Scratch {
    fn new() -> Self {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "kitaqgb-drivers-{}-{t}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn wait_for(mut condition: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < until, "development loop did not respond");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn dependency_aware_devserver_rebuilds_changed_includes() {
    let temp = Scratch::new();
    let source = temp.0.join("main.c");
    let header = temp.0.join("value.h");
    let rom = temp.0.join("game.gb");
    let deps = temp.0.join("deps.txt");
    fs::write(
        &source,
        "#include \"value.h\"\nu8 result;void main(){result=VALUE;}\n",
    )
    .unwrap();
    fs::write(&header, "#define VALUE 7\n").unwrap();
    let args = vec![
        "devserver".into(),
        "--poll-ms=50".into(),
        "--debounce-ms=0".into(),
        "--max-builds=2".into(),
        format!("--deps={}", deps.display()),
        source.display().to_string(),
        "-o".into(),
        rom.display().to_string(),
        "--no-cache".into(),
    ];
    let cancel = CancellationToken::new();
    let _guard = CancelOnDrop(cancel.clone());
    let stop = cancel.clone();
    let job = std::thread::spawn(move || {
        let mut s = Session::default();
        s.hosted = true;
        s.cancellation_token = Some(stop);
        kitaqgb::cli::invoke(args, &mut s)
    });
    wait_for(|| rom.is_file() && deps.is_file());
    let before = fs::read(&rom).unwrap();
    std::thread::sleep(Duration::from_millis(250));
    fs::write(&header, "#define VALUE 19\n").unwrap();
    wait_for(|| fs::read(&rom).is_ok_and(|b| b != before));
    cancel.cancel();
    assert_eq!(job.join().unwrap(), 0);
    assert!(fs::read_to_string(&deps).unwrap().contains("value.h"));
}
#[test]
fn api_can_cancel_a_source_watch_after_rebuild() {
    let temp = Scratch::new();
    let source = temp.0.join("main.c");
    let rom = temp.0.join("game.gb");
    fs::write(&source, "u8 result;void main(){result=7;}\n").unwrap();
    let token = CancellationToken::new();
    let _guard = CancelOnDrop(token.clone());
    let stop = token.clone();
    let req = CompileRequest {
        source_files: vec![source.display().to_string()],
        output_filename: rom.display().to_string(),
        cancellation_token: Some(token),
        extra_args: vec!["--watch".into(), "--no-cache".into(), "--no-disasm".into()],
        ..Default::default()
    };
    let job = std::thread::spawn(move || KitaqgbCompiler.compile(&req).unwrap());
    wait_for(|| rom.is_file());
    let before = fs::read(&rom).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    fs::write(&source, "u8 result;void main(){result=19;}\n").unwrap();
    wait_for(|| fs::read(&rom).is_ok_and(|b| b != before));
    stop.cancel();
    assert!(job.join().unwrap().success);
}
#[test]
fn bounded_driver_propagates_compile_failure() {
    let temp = Scratch::new();
    let source = temp.0.join("main.c");
    fs::write(&source, "#error fail\nvoid main(){}\n").unwrap();
    let mut s = Session::default();
    s.hosted = true;
    let args = vec![
        "devserver".into(),
        "--once".into(),
        source.display().to_string(),
        "-o".into(),
        temp.0.join("game.gb").display().to_string(),
        format!("--deps={}", temp.0.join("deps.txt").display()),
        "--no-cache".into(),
    ];
    assert_eq!(kitaqgb::cli::invoke(args, &mut s), 1);
}
#[test]
fn external_test_launcher_preserves_arguments_and_status() {
    let t = Scratch::new();
    fs::write(
        t.0.join("Cargo.toml"),
        "[package]\nname = \"kitaqgb\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let scripts = t.0.join("integration_test/scripts");
    fs::create_dir_all(&scripts).unwrap();
    #[cfg(windows)]
    fs::write(
        scripts.join("run_all.ps1"),
        "[IO.File]::WriteAllText((Join-Path (Get-Location) 'argument.txt'), $args[0])\nexit 7\n",
    )
    .unwrap();
    #[cfg(not(windows))]
    fs::write(
        scripts.join("run_all.sh"),
        "#!/bin/sh\nprintf '%s' \"$1\" > argument.txt\nexit 7\n",
    )
    .unwrap();
    let argument = "space and $x;literal";
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_kitaqgb"))
        .current_dir(&t.0)
        .args(["test", argument])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(7));
    assert_eq!(
        fs::read_to_string(t.0.join("argument.txt")).unwrap(),
        argument
    );
}
