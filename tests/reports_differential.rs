use kitaqgb::{assembler, assembly_json, io, reports};
use std::{fs, path::PathBuf};
fn normalized(s: &str) -> String {
    s.replace("\r\n", "\n")
        .trim_start_matches('\u{feff}')
        .to_owned()
}
#[test]
fn companion_maps_and_debugger_files_match_csharp() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = fs::read_dir(&base)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".gb.assembly.json"))
        .collect::<Vec<_>>();
    files.sort();
    assert_eq!(files.len(), 14);
    for file in files {
        let name = file
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".gb.assembly.json")
            .to_owned();
        let (assembly, options) = assembly_json::parse(&io::read_utf8(&file).unwrap()).unwrap();
        let out = assembler::assemble(&assembly, &options).unwrap();
        let (dbg, dbc) = out.debug.export(&format!("{name}.dbc"));
        for (extension, actual) in [
            ("map", reports::symbol_map(&out)),
            ("banks.txt", reports::bank_sizes(&out)),
            ("funcsizes.txt", reports::function_sizes(&out)),
            ("dbg", dbg),
            ("dbc", dbc),
        ] {
            let expected = io::read_utf8(base.join(format!("{name}.{extension}"))).unwrap();
            let actual = normalized(&actual);
            let expected = normalized(&expected);
            if actual != expected {
                let a = actual.lines().collect::<Vec<_>>();
                let b = expected.lines().collect::<Vec<_>>();
                let i = a
                    .iter()
                    .zip(&b)
                    .position(|(a, b)| a != b)
                    .unwrap_or(a.len().min(b.len()));
                panic!(
                    "{name}.{extension} line {}: Rust {:?}, C# {:?}",
                    i + 1,
                    a.get(i),
                    b.get(i)
                );
            }
        }
    }
}
