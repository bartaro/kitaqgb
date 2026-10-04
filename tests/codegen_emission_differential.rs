use kitaqgb::{
    assembler, assembly_json,
    codegen::{self, emission},
    ir_json,
    json::Value,
    lowerer, parser,
    tokenizer::Severity,
};
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
                if let Some(d) = difference(a, b, &format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, a) in a {
                let Some(b) = b.get(k) else {
                    return Some(format!("{path}.{k}: missing"));
                };
                if let Some(d) = difference(a, b, &format!("{path}.{k}")) {
                    return Some(d);
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
fn normalize_report(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == "source" {
                    if let Value::String(s) = value {
                        if let Some((path, position)) = s.rsplit_once(" (line ") {
                            *s = format!(
                                "{} (line {position}",
                                path.rsplit(['/', '\\']).next().unwrap()
                            );
                        }
                    }
                } else {
                    normalize_report(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                normalize_report(value);
            }
        }
        _ => {}
    }
}
#[test]
fn native_emission_matches_csharp_instructions_and_rom() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/emission-fixtures");
    let mut files = fs::read_dir(&root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "c"))
        .collect::<Vec<_>>();
    files.sort();
    assert!(files.len() >= 37);
    let mut analysis_mismatches = Vec::new();
    for source in files {
        let parsed = parser::parse_files(&[source.clone()], &[root.clone()], false);
        assert!(
            !parsed
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{:?}",
            parsed.diagnostics
        );
        let cgb = parsed.preprocessing.header.cgb;
        let known_cgb = match cgb {
            Some(0) => Some(0),
            Some(0xc0) => Some(1),
            _ => None,
        };
        let lowered = lowerer::lower(
            &parsed.tree,
            &lowerer::Options {
                known_cgb,
                const_scalar_in_rom: source
                    .file_stem()
                    .is_some_and(|s| s == "const-rom" || s == "constants-runtime"),
                ..lowerer::Options::default()
            },
        );
        assert!(
            !lowered
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{:?}",
            lowered.diagnostics
        );
        let output = emission::compile(
            &lowered.tree,
            codegen::Options {
                known_cgb,
                const_scalar_in_rom: source
                    .file_stem()
                    .is_some_and(|s| s == "const-rom" || s == "constants-runtime"),
                cgb_only: cgb == Some(0xc0),
                abi_stack: source
                    .file_stem()
                    .is_some_and(|s| s == "aggregate-abi-stack"),
                check_bounds: source.file_stem().is_some_and(|s| {
                    s == "checks-bounds" || s == "checks-runtime" || s == "checks-all"
                }),
                check_slice_bounds: source.file_stem().is_some_and(|s| s == "checks-slices"),
                check_stack: source
                    .file_stem()
                    .is_some_and(|s| s == "checks-stack" || s == "checks-all"),
                check_mem_copy: source
                    .file_stem()
                    .is_some_and(|s| s == "checks-copy" || s == "checks-all"),
                check_bank_calls: source
                    .file_stem()
                    .is_some_and(|s| s == "checks-bank" || s == "checks-all"),
                rst: match source.file_stem().unwrap().to_str().unwrap() {
                    "rst-safe" | "rst-helpers" | "rst-order" => codegen::rst::Options {
                        enabled: true,
                        ..Default::default()
                    },
                    "rst-unsafe" => codegen::rst::Options {
                        enabled: true,
                        unsafe_mode: true,
                        ..Default::default()
                    },
                    "rst-limits" => codegen::rst::Options {
                        enabled: true,
                        max_calls: 3,
                        max_vectors: 2,
                        exclude: ["__b", "__c", "__e"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                        ..Default::default()
                    },
                    "rst-speed" => codegen::rst::Options {
                        enabled: true,
                        max_calls: 12,
                        ..Default::default()
                    },
                    "rst-use38" => codegen::rst::Options {
                        enabled: true,
                        use_38: true,
                        ..Default::default()
                    },
                    "rst-disabled" => codegen::rst::Options {
                        use_38: true,
                        ..Default::default()
                    },
                    _ => Default::default(),
                },
                ..codegen::Options::default()
            },
        );
        assert!(
            !output
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{} {:?}",
            source.display(),
            output.diagnostics
        );
        let actual = Value::Array(output.lines.iter().map(ir_json::expr).collect());
        let mut analysis = output.analysis.json();
        let mut expected_analysis =
            Value::parse(&fs::read_to_string(source.with_extension("codegen.json")).unwrap())
                .unwrap();
        normalize_report(&mut analysis);
        normalize_report(&mut expected_analysis);
        if let Some(d) = difference(&analysis, &expected_analysis, "codegen") {
            fs::write(
                source.with_extension("codegen.actual.json"),
                analysis.stringify(),
            )
            .unwrap();
            analysis_mismatches.push(format!("{}: {d}", source.display()));
        }
        let text = fs::read_to_string(source.with_extension("input.json")).unwrap();
        let expected = Value::parse(&text).unwrap();
        let Value::Object(object) = &expected else {
            panic!()
        };
        if let Some(d) = difference(&actual, &object["assembly"], "assembly") {
            fs::write(source.with_extension("actual.json"), actual.stringify()).unwrap();
            panic!("{}: {d}", source.display());
        }
        let (reference, options) = assembly_json::parse(&text).unwrap();
        let native = assembler::assemble(&output.lines, &options)
            .unwrap_or_else(|e| panic!("{}: {e}", source.display()));
        let reference = assembler::assemble(&reference, &options).unwrap();
        assert_eq!(native.rom, reference.rom, "{} ROM", source.display());
        for level in 0..=1 {
            let optimized = kitaqgb::optimizer::optimize(&output.lines, level).lines;
            let reference_text =
                fs::read_to_string(source.with_extension(format!("opt{level}.json"))).unwrap();
            let (reference, options) = assembly_json::parse(&reference_text).unwrap();
            assert_eq!(
                Value::Array(optimized.iter().map(ir_json::expr).collect()),
                Value::Array(reference.iter().map(ir_json::expr).collect()),
                "{} optimized O{level}",
                source.display()
            );
            assert_eq!(
                assembler::assemble(&optimized, &options).unwrap().rom,
                assembler::assemble(&reference, &options).unwrap().rom,
                "{} optimized ROM O{level}",
                source.display()
            );
        }
    }
    assert!(
        analysis_mismatches.is_empty(),
        "{}",
        analysis_mismatches.join("\n")
    );
}

#[test]
fn native_emission_matches_the_local_physics3d_library() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let source = root.join("ast-fixtures/local-physics3d.c");
    let parsed = parser::parse_files(&[source], &[root.join("ast-fixtures/lib")], false);
    let lowered = lowerer::lower(&parsed.tree, &lowerer::Options::default());
    let output = emission::compile(&lowered.tree, codegen::Options::default());
    for diagnostics in [
        &parsed.diagnostics,
        &lowered.diagnostics,
        &output.diagnostics,
    ] {
        assert!(
            !diagnostics.iter().any(|d| d.severity == Severity::Error),
            "{diagnostics:?}"
        );
    }
    let actual = Value::Array(output.lines.iter().map(ir_json::expr).collect());
    let text =
        fs::read_to_string(root.join("pipeline-fixtures/local-physics3d.input.json")).unwrap();
    let expected = Value::parse(&text).unwrap();
    let Value::Object(object) = &expected else {
        panic!()
    };
    assert!(output.lines.len() > 14000);
    if let Some(d) = difference(&actual, &object["assembly"], "physics3d assembly") {
        panic!("{d}");
    }
}
