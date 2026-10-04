use kitaqgb::{assembly_json, dead_stripper, io, json::Value, optimizer};
use std::{collections::BTreeSet, fs, path::PathBuf};
fn number(n: usize) -> Value {
    Value::Number(n as i64)
}
fn report(r: &optimizer::Report) -> Value {
    Value::Object(
        [
            ("total_rst_rewrites".into(), number(r.total_rst_rewrites)),
            (
                "rst_rewrite_counts_by_vector".into(),
                Value::Object(
                    r.rst_rewrite_counts_by_vector
                        .iter()
                        .map(|(k, v)| (k.to_string(), number(*v)))
                        .collect(),
                ),
            ),
            (
                "passes".into(),
                Value::Array(
                    r.passes
                        .iter()
                        .map(|p| {
                            Value::Object(
                                [
                                    ("name".into(), Value::String(p.name.clone())),
                                    ("before_lines".into(), number(p.before_lines)),
                                    ("after_lines".into(), number(p.after_lines)),
                                    ("changed_lines".into(), number(p.changed_lines)),
                                    ("added_lines".into(), number(p.added_lines)),
                                    ("removed_lines".into(), number(p.removed_lines)),
                                    (
                                        "diff_text".into(),
                                        Value::String(p.diff_text.replace("\r\n", "\n")),
                                    ),
                                ]
                                .into(),
                            )
                        })
                        .collect(),
                ),
            ),
        ]
        .into(),
    )
}
#[test]
fn optimizer_and_dead_strip_match_csharp_instructions_positions_and_reports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/pipeline-fixtures");
    let mut inputs = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".input.json")
        })
        .collect::<Vec<_>>();
    inputs.sort();
    assert!(inputs.len() >= 13);
    for input in inputs {
        let prefix = input
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".input.json")
            .to_owned();
        let (raw, _) = assembly_json::parse(&io::read_utf8(&input).unwrap()).unwrap();
        for level in 0..2 {
            let optimized = optimizer::optimize(&raw, level);
            let (expected, _) = assembly_json::parse(
                &io::read_utf8(&root.join(format!("{prefix}.opt{level}.json"))).unwrap(),
            )
            .unwrap();
            assert_eq!(
                optimized.lines.len(),
                expected.len(),
                "{prefix} -O{level} length"
            );
            for (i, (actual, expected)) in optimized.lines.iter().zip(&expected).enumerate() {
                assert_eq!(actual, expected, "{prefix} -O{level} instruction {i}")
            }
            let mut expected_report = Value::parse(
                &io::read_utf8(&root.join(format!("{prefix}.report{level}.json"))).unwrap(),
            )
            .unwrap();
            if let Value::Object(r) = &mut expected_report {
                if let Some(Value::Array(passes)) = r.get_mut("passes") {
                    for p in passes {
                        if let Value::Object(p) = p {
                            if let Some(Value::String(diff)) = p.get_mut("diff_text") {
                                *diff = diff.replace("\r\n", "\n")
                            }
                        }
                    }
                }
            }
            assert_eq!(
                report(&optimized.report),
                expected_report,
                "{prefix} -O{level} analysis report"
            );
            let stripped = dead_stripper::strip(&optimized.lines, &BTreeSet::from(["kept".into()]));
            let (expected, _) = assembly_json::parse(
                &io::read_utf8(&root.join(format!("{prefix}.strip{level}.json"))).unwrap(),
            )
            .unwrap();
            assert_eq!(stripped.lines, expected, "{prefix} -O{level} dead strip");
        }
    }
}
