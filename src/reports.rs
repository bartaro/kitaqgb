//! Companion files consumed by KOKURA and the existing KITAQGB tools.
use crate::{
    asm::{Region, cpu_address},
    assembler::{self, Output},
    io,
};
use std::{fmt::Write, path::Path};
pub fn region_name(region: Region, address: i32) -> String {
    let name = match region {
        Region::Rom => "ROM",
        Region::HighMem => "HRAM",
        Region::Oam => "OAM",
        Region::Wram0 => "WRAM0",
        Region::WramX(0) => "WRAMX",
        Region::WramX(bank) => return format!("WRAMX[{bank}]"),
        _ => match address {
            0xff00..=0xff7f => "IO",
            0xff80..=0xfffe => "HRAM",
            0xfe00..=0xfe9f => "OAM",
            0xd000..=0xdfff => "WRAMX",
            0xc000..=0xcfff => "WRAM0",
            0xa000..=0xbfff => "SRAM",
            0x8000..=0x9fff => "VRAM",
            0..=0x7fff => "ROM",
            _ => {
                return if let Region::Fixed(a) = region {
                    format!("FIXED({}{:04X})", '$', a as u32)
                } else {
                    region.to_string().to_uppercase()
                };
            }
        },
    };
    name.into()
}
fn region_offset(region: Region, address: i32) -> i32 {
    if region == Region::Rom {
        return address & 0x3fff;
    }
    let base = match address {
        0xff80..=0xfffe => 0xff80,
        0xff00..=0xff7f => 0xff00,
        0xfe00..=0xfe9f => 0xfe00,
        0xd000..=0xdfff => 0xd000,
        0xc000..=0xcfff => 0xc000,
        0xa000..=0xbfff => 0xa000,
        0x8000..=0x9fff => 0x8000,
        _ => 0,
    };
    (address - base) & 0xffff
}
fn ordinal(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
pub fn symbol_map(output: &Output) -> String {
    let mut items = output
        .symbols
        .iter()
        .map(|(name, s)| {
            let address = if s.region == Region::Rom {
                cpu_address(s.value)
            } else {
                s.value & 0xffff
            };
            (
                name,
                s,
                address,
                assembler::logical_bank(s.region, s.value),
                region_name(s.region, address),
            )
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        ordinal(&a.4, &b.4)
            .then(a.3.unwrap_or(-1).cmp(&b.3.unwrap_or(-1)))
            .then(a.2.cmp(&b.2))
            .then(ordinal(a.0, b.0))
    });
    let mut out = if cfg!(windows) {
        String::from("; KITAQGB symbol map\r\n; Addr(CPU) Bank  Off   Kind Region Name\r\n")
    } else {
        String::from("; KITAQGB symbol map\n; Addr(CPU) Bank  Off   Kind Region Name\n")
    };
    for (name, s, address, bank, region) in items {
        writeln!(
            out,
            "{:04X}  {:>4}  {:04X}   {}    {:<8} {name}",
            address as u32,
            bank.unwrap_or(0),
            region_offset(s.region, s.value) as u32,
            if s.is_label { "L" } else { "S" },
            region
        )
        .unwrap();
    }
    out
}
pub fn bank_sizes(output: &Output) -> String {
    let mut out = String::from("# KITAQGB bank size report\n# bank, start, end, used, free\n");
    for (bank, &pc) in output.report.bank_max_pc.iter().enumerate() {
        let start = bank as i32 * assembler::BANK_SIZE;
        let end = start + assembler::BANK_SIZE;
        let used = (pc.min(end) - start).max(0);
        let free = (end - start - used).max(0);
        writeln!(
            out,
            "{bank:>2}, 0x{start:04X}, 0x{end:04X}, {used:>5}, {free:>5}"
        )
        .unwrap();
    }
    out
}
pub fn function_sizes(output: &Output) -> String {
    let mut items = output.report.functions.iter().collect::<Vec<_>>();
    items.sort_by(|a, b| b.size.cmp(&a.size).then(ordinal(&a.name, &b.name)));
    let mut out = String::from(
        "# KITAQGB function size report\n# name, bank, cpu_addr, start_file, end_file, size\n",
    );
    for f in items {
        writeln!(
            out,
            "{}, {}, 0x{:04X}, 0x{:05X}, 0x{:05X}, {}",
            f.name, f.bank, f.cpu as u32, f.start as u32, f.end as u32, f.size
        )
        .unwrap();
    }
    out
}
pub fn variable_list(output: &Output) -> String {
    let mut items = output.report.variables.iter().collect::<Vec<_>>();
    items.sort_by(|a, b| ordinal(&a.name, &b.name).then(a.address.cmp(&b.address)));
    let mut out = String::from("; KITAQGB variable list\n; Name  Region  Bank  Addr  Size\n");
    for v in items {
        let bank = v.bank.map(|b| b.to_string()).unwrap_or_default();
        writeln!(
            out,
            "{:<24} {:<6} {:>4}  {:04X}  {:>5}",
            v.name,
            region_name(v.region, v.address),
            bank,
            v.address as u32,
            v.size
        )
        .unwrap();
    }
    out
}
pub fn source_map(output: &Output) -> String {
    let mut out =
        String::from("# KITAQGB source map\n# bank:addr path:line:column symbol=... section=...\n");
    for s in &output.report.source_locations {
        if s.source.filename.is_empty() {
            continue;
        }
        write!(
            out,
            "{:02X}:{:04X} {}:{}:{}",
            s.bank & 0xffff,
            s.address & 0xffff,
            s.source.filename,
            s.source.line + 1,
            s.source.column + 1
        )
        .unwrap();
        if !s.symbol.is_empty() {
            write!(out, " symbol={}", s.symbol).unwrap();
        }
        if !s.section.is_empty() {
            write!(out, " section={}", s.section).unwrap();
        }
        out.push('\n');
    }
    out
}
/// Match the line-oriented function-definition fallback used by Program.DebugTools.
/// This deliberately searches the supplied sources, before preprocessing.
pub(crate) fn source_functions(text: &str) -> Vec<(String, usize)> {
    let bytes = text.as_bytes();
    let mut starts = vec![0];
    starts.extend(
        bytes
            .iter()
            .enumerate()
            .filter_map(|(i, b)| (*b == b'\n').then_some(i + 1)),
    );
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut functions = Vec::new();
    for (line, &start) in starts.iter().enumerate() {
        let mut i = start;
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i == bytes.len() || !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            continue;
        }
        let prefix_start = i;
        while i < bytes.len()
            && (ident(bytes[i])
                || bytes[i].is_ascii_whitespace()
                || matches!(bytes[i], b'*' | b'[' | b']'))
        {
            i += 1;
        }
        if bytes.get(i) != Some(&b'(') {
            continue;
        }
        let mut end = i;
        while end > prefix_start && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        let mut name_start = end;
        while name_start > prefix_start && ident(bytes[name_start - 1]) {
            name_start -= 1;
        }
        if name_start == prefix_start || name_start == end {
            continue;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..end]).into_owned();
        if matches!(
            name.as_str(),
            "if" | "for" | "while" | "switch" | "do" | "else" | "return" | "sizeof"
        ) {
            continue;
        }
        let mut brace = None;
        i += 1;
        while i < bytes.len() && !matches!(bytes[i], b';' | b'{' | b'}') {
            if bytes[i] == b')' {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'{') {
                    brace = Some(j);
                    break;
                }
            }
            i += 1;
        }
        let Some(mut i) = brace else {
            continue;
        };
        let mut depth = 0;
        let mut state = 0u8;
        let mut closed = false;
        while i < bytes.len() {
            let c = bytes[i];
            let n = bytes.get(i + 1).copied().unwrap_or(0);
            match state {
                1 => {
                    if c == b'\n' {
                        state = 0;
                    }
                }
                2 => {
                    if c == b'*' && n == b'/' {
                        state = 0;
                        i += 1;
                    }
                }
                3 | 4 => {
                    if c == b'\\' {
                        i += 1;
                    } else if c == if state == 3 { b'"' } else { b'\'' } {
                        state = 0;
                    }
                }
                _ => {
                    if c == b'/' && n == b'/' {
                        state = 1;
                        i += 1;
                    } else if c == b'/' && n == b'*' {
                        state = 2;
                        i += 1;
                    } else if c == b'"' {
                        state = 3;
                    } else if c == b'\'' {
                        state = 4;
                    } else if c == b'{' {
                        depth += 1;
                    } else if c == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            closed = true;
                            break;
                        }
                    }
                }
            }
            i += 1;
        }
        if closed {
            functions.push((name, line + 1));
        }
    }
    functions
}
pub fn augmented_source_map(output: &Output, sources: &[std::path::PathBuf]) -> String {
    let mut out = source_map(output);
    let mut fallback = std::collections::BTreeMap::new();
    for source in sources {
        if let Ok(text) = io::read_utf8(source) {
            for (name, line) in source_functions(&text) {
                fallback
                    .entry(name)
                    .or_insert((source.display().to_string(), line));
            }
        }
    }
    let with_source = output
        .report
        .source_locations
        .iter()
        .map(|s| s.symbol.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for f in &output.report.functions {
        if with_source.contains(f.name.as_str()) {
            continue;
        }
        if let Some((path, line)) = fallback.get(&f.name) {
            write!(
                out,
                "{:02X}:{:04X} {path}:{line} symbol={}",
                f.bank & 65535,
                f.cpu & 65535,
                f.name
            )
            .unwrap();
            if !f.section.is_empty() {
                write!(out, " section={}", f.section).unwrap();
            }
            out.push('\n');
        }
    }
    out
}
pub fn write_companions(output: &Output, path: &Path, vlist: Option<&Path>) -> Result<(), String> {
    write_companions_with(output, path, vlist, |_, path, text| {
        io::write_utf8(path, text)?;
        Ok(path.to_owned())
    })
}
pub(crate) fn write_companions_with(
    output: &Output,
    path: &Path,
    vlist: Option<&Path>,
    mut publish: impl FnMut(&str, &Path, &str) -> Result<std::path::PathBuf, String>,
) -> Result<(), String> {
    for (extension, text) in [
        ("map", symbol_map(output)),
        ("banks.txt", bank_sizes(output)),
        ("funcsizes.txt", function_sizes(output)),
        ("source_map.txt", source_map(output)),
    ] {
        let key = match extension {
            "map" => "map",
            "banks.txt" => "banks_txt",
            "funcsizes.txt" => "funcsizes_txt",
            _ => "source_map",
        };
        publish(key, &path.with_extension(extension), &text)?;
    }
    let dbc_path = path.with_extension("dbc");
    let (_, dbc) = output
        .debug
        .export(&dbc_path.file_name().unwrap().to_string_lossy());
    let actual_dbc = publish("dbc", &dbc_path, &dbc)?;
    let (dbg, _) = output
        .debug
        .export(&actual_dbc.file_name().unwrap().to_string_lossy());
    publish("dbg", &path.with_extension("dbg"), &dbg)?;
    if let Some(vlist) = vlist {
        publish("vlist", vlist, &variable_list(output))?;
    }
    Ok(())
}
