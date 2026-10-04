use kitaqgb::{io, json::Value, tokenizer};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn position(p: &kitaqgb::expr::Position) -> Value {
    Value::Object(
        [
            (
                "filename".into(),
                Value::String(
                    Path::new(&p.filename)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into(),
                ),
            ),
            ("line".into(), Value::Number(p.line as i64)),
            ("column".into(), Value::Number(p.column as i64)),
        ]
        .into(),
    )
}
#[test]
fn native_tokens_match_csharp_payloads_and_utf16_source_coordinates() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/token-fixtures");
    let mut cases = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    cases.sort();
    assert!(cases.len() >= 8);
    for case in cases {
        let Value::Array(files) =
            Value::parse(&io::read_utf8(&case.join("files.json")).unwrap()).unwrap()
        else {
            panic!("invalid files manifest")
        };
        let files = files
            .iter()
            .map(|f| {
                let Value::String(s) = f else { panic!() };
                case.join(s)
            })
            .collect::<Vec<_>>();
        let output = tokenizer::tokenize_files(&files, &[case.clone()]);
        assert!(
            !output.has_errors(),
            "{}: {:?}",
            case.display(),
            output.diagnostics
        );
        assert!(
            output.diagnostics.is_empty(),
            "{}: {:?}",
            case.display(),
            output.diagnostics
        );
        let tokens = output
            .tokens
            .iter()
            .map(|t| {
                Value::Object(
                    [
                        ("kind".into(), Value::String(format!("{:?}", t.kind))),
                        ("integer".into(), Value::Number(i64::from(t.integer))),
                        (
                            "name".into(),
                            t.name
                                .as_ref()
                                .map_or(Value::Null, |s| Value::String(s.clone())),
                        ),
                        ("position".into(), position(&t.position)),
                    ]
                    .into(),
                )
            })
            .collect();
        let dependencies = output
            .dependencies
            .iter()
            .map(|p| Value::String(p.file_name().unwrap().to_string_lossy().into()))
            .collect();
        let actual = Value::Object(
            [
                ("tokens".into(), Value::Array(tokens)),
                ("eof".into(), position(&output.eof)),
                ("dependencies".into(), Value::Array(dependencies)),
            ]
            .into(),
        );
        let expected = Value::parse(&io::read_utf8(&case.join("expected.json")).unwrap()).unwrap();
        if actual != expected {
            fs::write(case.join("actual.json"), actual.stringify()).unwrap();
            panic!(
                "{}: token output differs; compare actual.json and expected.json",
                case.display()
            );
        }
        if case.file_name().unwrap() == "pragmas" {
            assert_eq!(output.header.title.as_deref(), Some("RUST TEST"));
            assert_eq!(output.header.cgb, Some(0xc0));
            assert_eq!(output.header.cart, Some(0x19));
            assert_eq!(output.header.romsize, Some(2));
            assert_eq!(output.header.logo, Some(false));
            assert_eq!(output.palettes.len(), 1);
            assert_eq!(output.palettes[0].colors, [0x7fff, 31, 0x3e0, 0x7c00]);
        }
    }
}
