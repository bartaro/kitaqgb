use kitaqgb::{assembler, assembly_json, disassembler, io, rom_header};
use std::{fs, path::PathBuf};
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn assert_bytes(actual: &[u8], expected: &[u8], name: &str) {
    assert_eq!(actual.len(), expected.len(), "{name}: length differs");
    if let Some(index) = actual.iter().zip(expected).position(|(a, b)| a != b) {
        panic!(
            "{name}: first mismatch at ${index:06X}: Rust=${:02X}, C#=${:02X}",
            actual[index], expected[index]
        );
    }
}
#[test]
fn assembler_matches_csharp_at_every_rom_byte() {
    let base = fixtures();
    let mut files = fs::read_dir(&base)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".gb.assembly.json"))
        .collect::<Vec<_>>();
    files.sort();
    assert!(files.len() >= 4, "C# oracle fixtures are missing");
    for file in files {
        let name = file
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".gb.assembly.json")
            .to_owned();
        let text = io::read_utf8(&file).unwrap();
        let (assembly, options) = assembly_json::parse(&text).unwrap();
        let output = assembler::assemble(&assembly, &options)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let expected = fs::read(base.join(format!("{name}.gb.raw"))).unwrap();
        assert_bytes(&output.rom, &expected, &name);
    }
}
#[test]
fn disassembler_matches_csharp_through_a_truncated_bank_boundary() {
    let base = fixtures();
    let rom = fs::read(base.join("disassembler.gb")).unwrap();
    let expected = io::read_utf8(&base.join("disassembler.s"))
        .unwrap()
        .replace("\r\n", "\n");
    let mut actual = Vec::new();
    disassembler::disassemble(&rom, "disassembler.gb", &mut actual).unwrap();
    let actual = String::from_utf8(actual).unwrap();
    if actual != expected {
        let a = actual.lines().collect::<Vec<_>>();
        let b = expected.lines().collect::<Vec<_>>();
        let index = a
            .iter()
            .zip(&b)
            .position(|(a, b)| a != b)
            .unwrap_or(a.len().min(b.len()));
        panic!(
            "disassembly differs at line {}: Rust={:?}, C#={:?}",
            index + 1,
            a.get(index),
            b.get(index)
        );
    }
}
#[test]
fn header_fields_padding_unicode_and_checksums_match_csharp() {
    for index in 0..2 {
        let base = fixtures();
        let options = rom_header::Options::from_json(
            &io::read_utf8(&base.join(format!("header-{index}.json"))).unwrap(),
        )
        .unwrap();
        let input = fs::read(base.join(format!("header-{index}.input.gb"))).unwrap();
        let expected = fs::read(base.join(format!("header-{index}.gb"))).unwrap();
        let (actual, _) = rom_header::patch(&input, &options).unwrap();
        assert_bytes(&actual, &expected, &format!("header-{index}"));
    }
}
