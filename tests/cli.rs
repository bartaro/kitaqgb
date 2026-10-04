use kitaqgb::{assembler, assembly_json, io, rom_header};
use std::{fs, path::PathBuf, process::Command};
#[test]
fn default_invocation_include_alias_output_and_companions() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp = std::env::temp_dir().join(format!("kitaqgb-cli-{}-{unique}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let include = temp.join("inc");
    fs::create_dir(&include).unwrap();
    fs::write(include.join("value.h"), "#define UNUSED 42\n").unwrap();
    let source = temp.join("main.c");
    fs::write(&source, "#include \"value.h\"\nvoid main(){}\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_kitaqgb"))
        .current_dir(&temp)
        .arg(format!("--include-dir={}", include.display()))
        .arg(&source)
        .args([
            "--no-cache",
            "--rst-disable",
            "--no-disasm",
            "--vlist",
            "--deps-out",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let (ir, options) = assembly_json::parse(
        &io::read_utf8(base.join("tests/emission-fixtures/empty.input.json")).unwrap(),
    )
    .unwrap();
    let expected = assembler::assemble(&ir, &options).unwrap();
    let (expected, _) = rom_header::patch(&expected.rom, &rom_header::Options::default()).unwrap();
    assert_eq!(fs::read(temp.join("out.gb")).unwrap(), expected);
    for extension in [
        "map",
        "banks.txt",
        "funcsizes.txt",
        "source_map.txt",
        "dbg",
        "dbc",
        "vlist.txt",
        "deps.txt",
    ] {
        assert!(
            temp.join(format!("out.{extension}")).is_file(),
            "{extension} was not emitted"
        );
    }
    let deps = io::read_utf8(temp.join("out.deps.txt")).unwrap();
    assert!(deps.contains("main.c") && deps.contains("value.h"));
    let map = io::read_utf8(temp.join("out.source_map.txt")).unwrap();
    assert!(map.contains("main.c:2 symbol=main"), "{map}");
    let nested = temp.join("nested/rom.gb");
    let result = Command::new(env!("CARGO_BIN_EXE_kitaqgb"))
        .current_dir(&temp)
        .arg("compile")
        .arg(&source)
        .arg(format!("-I{}", include.display()))
        .arg("-o")
        .arg(&nested)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(nested).unwrap(), expected);
    assert!(temp.starts_with(std::env::temp_dir()));
    fs::remove_dir_all(temp).unwrap();
}
