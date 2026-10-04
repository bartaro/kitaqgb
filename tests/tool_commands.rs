use kitaqgb::{io, json::Value};
use std::{fs, path::PathBuf, process::Command};
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
            "kitaqgb-tools-{}-{time}-{serial}",
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
        panic!()
    }
}
fn text(value: &Value) -> &str {
    if let Value::String(s) = value {
        s
    } else {
        panic!()
    }
}
fn normalize(s: &str, root: &std::path::Path) -> String {
    let s = s
        .replace("\r\n", "\n")
        .replace(&root.display().to_string(), "{ROOT}")
        .replace(&root.display().to_string().replace('\\', "/"), "{ROOT}")
        .replace("{ROOT}\\", "{ROOT}/");
    regex::Regex::new(r"generated_utc=[^\n]*")
        .unwrap()
        .replace_all(&s, "generated_utc=<UTC>")
        .into_owned()
}
fn cases(group: &str) {
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/{group}-fixtures"));
    let expected = Value::parse(&io::read_utf8(fixtures.join("expected.json")).unwrap()).unwrap();
    let Value::Array(cases) = expected else {
        panic!()
    };
    for case in cases {
        let temp = Scratch::new();
        for entry in fs::read_dir(&fixtures).unwrap() {
            let file = entry.unwrap();
            if file.file_name() != "expected.json" {
                fs::copy(file.path(), temp.0.join(file.file_name())).unwrap();
            }
        }
        let name = text(field(&case, "case"));
        let Value::Array(args) = field(&case, "args") else {
            panic!()
        };
        let args = args
            .iter()
            .map(|a| text(a).replace("{ROOT}", &temp.0.display().to_string()))
            .collect::<Vec<_>>();
        let result = Command::new(env!("CARGO_BIN_EXE_kitaqgb"))
            .current_dir(&temp.0)
            .args(args)
            .output()
            .unwrap();
        let Value::Number(exit) = field(&case, "exit_code") else {
            panic!()
        };
        assert_eq!(
            result.status.code().unwrap_or(1) as i64,
            *exit,
            "{name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            normalize(&String::from_utf8_lossy(&result.stdout), &temp.0),
            text(field(&case, "stdout")),
            "{name}"
        );
        if let Value::Object(fields) = &case {
            if let Some(Value::Object(files)) = fields.get("files") {
                for (name, expected) in files {
                    assert_eq!(
                        normalize(&io::read_utf8(temp.0.join(name)).unwrap(), &temp.0),
                        text(expected),
                        "{name}"
                    );
                }
            }
        }
    }
}
#[test]
fn diagnostic_symbol_source_and_rom_tools_match_reference() {
    cases("debug")
}
#[test]
fn templates_snippets_attributes_hints_and_ir_summaries_match_reference() {
    cases("vibe")
}
