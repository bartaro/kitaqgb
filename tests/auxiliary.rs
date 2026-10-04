use kitaqgb::zx0;
use std::{fs, path::PathBuf, process::Command};
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/auxiliary-fixtures")
}
#[test]
fn zx0_raw_rle_auto_and_headers_match_the_csharp_reference() {
    let root = fixtures();
    for file in fs::read_dir(&root).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|s| s != "input") {
            continue;
        }
        let input = fs::read(&path).unwrap();
        let prefix = path.with_extension("");
        for (suffix, actual) in [
            ("zx0", zx0::compress(&input)),
            ("rle", Ok(zx0::rle(&input))),
            ("kqa", zx0::automatic(&input).map(|v| v.0)),
        ] {
            let expected = prefix.with_extension(suffix);
            if expected.exists() {
                let packed = fs::read(expected).unwrap();
                assert_eq!(actual.unwrap(), packed, "{} {suffix}", path.display());
                if suffix == "zx0" {
                    assert_eq!(zx0::decompress(&packed, 65535).unwrap(), input);
                }
            } else {
                assert_eq!(
                    actual.unwrap_err(),
                    fs::read_to_string(prefix.with_extension(format!("{suffix}.error"))).unwrap()
                );
            }
        }
        assert_eq!(
            zx0::header(&input, input.len(), "asset_data")
                .unwrap()
                .as_bytes(),
            fs::read(prefix.with_extension("header")).unwrap()
        );
        if !input.is_empty() {
            let packed = zx0::compress(&input).unwrap();
            assert_eq!(
                zx0::decompress(&packed, input.len() - 1).unwrap_err(),
                "ZX0 output exceeds capacity."
            );
        }
    }
}
#[test]
fn malformed_streams_match_reference_errors_and_never_panic() {
    let root = fixtures();
    for file in fs::read_dir(root).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|s| s != "invalid") {
            continue;
        }
        let actual = zx0::decompress(&fs::read(&path).unwrap(), 65535);
        let decoded = path.with_extension("invalid.decoded");
        if decoded.exists() {
            assert_eq!(actual.unwrap(), fs::read(decoded).unwrap());
        } else {
            assert_eq!(
                actual.unwrap_err(),
                fs::read_to_string(path.with_extension("invalid.decoded.error")).unwrap()
            );
        }
    }
    assert!(zx0::compress(&vec![0; 65536]).is_err());
    assert!(zx0::automatic(&vec![0; 65536]).is_err());
    assert!(zx0::decompress(&[], 65536).is_err());
    assert!(zx0::header(&[], 0, "bad-id").is_err());
}
#[test]
fn native_zx0_cli_matches_frozen_outputs_and_rejects_overwrite() {
    let root = fixtures();
    let temp = std::env::temp_dir().join(format!("kitaqgb-zx0-cli-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let exe = env!("CARGO_BIN_EXE_kitaqgb-zx0");
    for (name, format, suffix) in [
        ("pattern-257", "zx0", "zx0"),
        ("rle-splits", "rle", "rle"),
        ("noise-25", "auto", "kqa"),
        ("zeros-0", "raw", "input"),
        ("zeros-65535", "zx0", "zx0"),
    ] {
        let out = temp.join("out.bin");
        let result = Command::new(exe)
            .arg(root.join(format!("{name}.input")))
            .arg(&out)
            .arg(format!("--format={format}"))
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
        assert_eq!(
            fs::read(&out).unwrap(),
            fs::read(root.join(format!("{name}.{suffix}"))).unwrap()
        );
    }
    let input = temp.join("input.bin");
    fs::write(&input, b"safe original").unwrap();
    let result = Command::new(exe)
        .arg(&input)
        .arg(temp.join("./input.bin"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(fs::read(&input).unwrap(), b"safe original");
    let result = Command::new(exe)
        .arg(root.join("noise-65535.input"))
        .arg(temp.join("overflow"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(!temp.join("overflow").exists());
    fs::remove_dir_all(temp).unwrap();
}
#[test]
fn gb_vblank_patch_matches_original_checksums_and_rejects_bad_symbols() {
    let mut rom = vec![0xFF; 32768];
    rom[0x210..0x214].copy_from_slice(&[0xF5, 0xC5, 0xD5, 0xE5]);
    let map = "0200 bank=0 __kq_vblank_vector\n";
    let mut normal = rom.clone();
    assert_eq!(
        kitaqgb::vblank_patch::patch(&mut normal, map, "__kq_vblank_vector", false).unwrap(),
        0x210
    );
    assert_eq!(&normal[0x40..0x43], &[0xC3, 0x10, 0x02]);
    assert_eq!(&normal[0x134..0x144], &[0; 16]);
    assert_eq!(
        normal[0x14D],
        normal[0x134..=0x14C]
            .iter()
            .fold(0u8, |s, &b| s.wrapping_sub(b).wrapping_sub(1))
    );
    assert_eq!(
        u16::from_be_bytes([normal[0x14E], normal[0x14F]]),
        normal
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 0x14E && i != 0x14F)
            .fold(0u16, |s, (_, &b)| s.wrapping_add(b as u16))
    );
    let mut unchanged = rom.clone();
    kitaqgb::vblank_patch::patch(&mut unchanged, map, "__kq_vblank_vector", true).unwrap();
    assert_eq!(&unchanged[0x43..], &rom[0x43..]);
    for bad in [
        "",
        "4000 bank=1 __kq_vblank_vector\n",
        "0200 bank=0 different\n",
    ] {
        let mut data = rom.clone();
        assert!(kitaqgb::vblank_patch::patch(&mut data, bad, "__kq_vblank_vector", false).is_err());
        assert_eq!(data, rom);
    }
    assert!(kitaqgb::vblank_patch::patch(&mut [0; 32], map, "__kq_vblank_vector", true).is_err());
}
