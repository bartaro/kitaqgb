//! Portable equivalent of scripts/patch_gb_vblank_irq.ps1.
pub fn patch(
    rom: &mut [u8],
    map: &str,
    symbol: &str,
    no_header_fix: bool,
) -> Result<usize, String> {
    let re = regex::Regex::new(&format!(
        r"^\s*([0-9A-Fa-f]{{4}})\s+.*\s{}\s*$",
        regex::escape(symbol)
    ))
    .map_err(|e| e.to_string())?;
    let vector = map
        .lines()
        .find_map(|line| {
            re.captures(line)
                .map(|c| usize::from_str_radix(&c[1], 16).unwrap())
        })
        .ok_or_else(|| format!("patch_gb_vblank_irq: {symbol} not found in map"))?;
    if vector >= 0x4000 {
        return Err(format!(
            "patch_gb_vblank_irq: {symbol} must be in fixed bank, got 0x{vector:04X}"
        ));
    }
    let minimum = if no_header_fix { 0x43 } else { 0x150 };
    if rom.len() < minimum {
        return Err("patch_gb_vblank_irq: ROM is too short".into());
    }
    // Preserve the original signature heuristic and its exclusive search bound.
    let end = (vector + 96).min(rom.len().saturating_sub(4));
    let entry = (vector..end)
        .find(|&at| rom[at..at + 4] == [0xF5, 0xC5, 0xD5, 0xE5])
        .ok_or_else(|| {
            format!("patch_gb_vblank_irq: raw ISR body signature not found near 0x{vector:04X}")
        })?;
    rom[0x40..0x43].copy_from_slice(&[0xC3, entry as u8, (entry >> 8) as u8]);
    if !no_header_fix {
        for b in &mut rom[0x134..=0x143] {
            if *b == 0xFF {
                *b = 0;
            }
        }
        rom[0x14D] = rom[0x134..=0x14C]
            .iter()
            .fold(0u8, |sum, &b| sum.wrapping_sub(b).wrapping_sub(1));
        let sum = rom
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 0x14E && i != 0x14F)
            .fold(0u16, |sum, (_, &b)| sum.wrapping_add(u16::from(b)));
        rom[0x14E..0x150].copy_from_slice(&sum.to_be_bytes());
    }
    Ok(entry)
}
pub fn cli(args: &[String]) -> Result<(), String> {
    if args == ["--version"] {
        println!("kitaqgb-patch-vblank {} (Rust)", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let usage = "Usage: kitaqgb-patch-vblank --rom ROM --map MAP [--symbol NAME] [--no-header-fix]";
    if args == ["--help"] {
        println!("{usage}");
        return Ok(());
    }
    let (mut rom_path, mut map_path, mut symbol, mut no_header_fix) =
        (None, None, "__kq_vblank_vector", false);
    let mut at = 0;
    while at < args.len() {
        let arg = &args[at];
        if arg == "--no-header-fix" || arg.eq_ignore_ascii_case("-NoHeaderFix") {
            no_header_fix = true;
            at += 1;
            continue;
        }
        at += 1;
        let value = args.get(at).ok_or(usage)?;
        if arg == "--rom" || arg.eq_ignore_ascii_case("-RomPath") {
            rom_path = Some(value);
        } else if arg == "--map" || arg.eq_ignore_ascii_case("-MapPath") {
            map_path = Some(value);
        } else if arg == "--symbol" || arg.eq_ignore_ascii_case("-Symbol") {
            symbol = value;
        } else {
            return Err(format!("Unknown option: {arg}"));
        }
        at += 1;
    }
    let path = rom_path.ok_or(usage)?;
    let mut rom = std::fs::read(path).map_err(|e| e.to_string())?;
    let map = std::fs::read_to_string(map_path.ok_or(usage)?).map_err(|e| e.to_string())?;
    let entry = patch(&mut rom, &map, symbol, no_header_fix)?;
    std::fs::write(path, rom).map_err(|e| e.to_string())?;
    println!(
        "patch_gb_vblank_irq: VBlank vector 0x0040 -> 0x{entry:04X} ({symbol}){}",
        if no_header_fix {
            ", header unchanged"
        } else {
            ", header checksums updated"
        }
    );
    Ok(())
}
