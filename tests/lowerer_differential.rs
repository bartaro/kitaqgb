use kitaqgb::{io, ir_json, json::Value, lowerer, parser, tokenizer::Severity};
use std::{fs, path::PathBuf};
fn difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    if a == b {
        return None;
    }
    match (a, b) {
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: lengths {} vs {}", a.len(), b.len()));
            }
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                if let Some(s) = difference(a, b, &format!("{path}[{i}]")) {
                    return Some(s);
                }
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, a) in a {
                let Some(b) = b.get(k) else {
                    return Some(format!("{path}.{k}: missing"));
                };
                if let Some(s) = difference(a, b, &format!("{path}.{k}")) {
                    return Some(s);
                }
            }
        }
        _ => {}
    }
    Some(format!(
        "{path}: Rust={} C#={}",
        a.stringify(),
        b.stringify()
    ))
}
#[test]
fn lowerer_matches_csharp_control_flow_temporary_lifetimes_and_ranges() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ast-fixtures");
    let mut cases = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "c"))
        .collect::<Vec<_>>();
    cases.sort();
    assert!(cases.len() >= 10);
    for source in cases {
        let output = parser::parse_files(&[source.clone()], &[root.join("lib")], false);
        let errors = output
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{}: {:?}", source.display(), errors);
        let expected =
            Value::parse(&io::read_utf8(&source.with_extension("lower.json")).unwrap()).unwrap();
        let output = lowerer::lower(&output.tree, &lowerer::Options::default());
        let errors = output
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{}: {:?}", source.display(), errors);
        let actual = ir_json::expr(&output.tree);
        if let Some(difference) = difference(&actual, &expected, "root") {
            fs::write(source.with_extension("actual.json"), actual.stringify()).unwrap();
            panic!("{}: {difference}", source.display())
        }
    }
}
