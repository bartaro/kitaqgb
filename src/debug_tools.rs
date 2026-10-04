//! Native diagnostic, symbol, source-range and ROM comparison tools.
use crate::{diagnostic_help, diagnostics, hash, io};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::{Path, PathBuf},
};

fn ordinal(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
fn integer(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Some(hex) = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .or_else(|| s.strip_prefix('$'))
    {
        i32::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}
fn hex(s: &str) -> Option<i32> {
    i32::from_str_radix(
        s.trim().trim_start_matches("0x").trim_start_matches("0X"),
        16,
    )
    .ok()
}
fn full(path: &str) -> PathBuf {
    #[cfg(windows)]
    let path = &path.replace('/', "\\");
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    }
}
pub(crate) fn newest(suffix: &str) -> String {
    fn scan(dir: &Path, suffix: &str, found: &mut Vec<(std::time::SystemTime, PathBuf)>) {
        if let Ok(items) = std::fs::read_dir(dir) {
            for item in items.flatten() {
                if let Ok(kind) = item.file_type() {
                    if kind.is_dir() {
                        scan(&item.path(), suffix, found)
                    } else if kind.is_file() && item.file_name().to_string_lossy().ends_with(suffix)
                    {
                        if let Ok(time) = item.metadata().and_then(|m| m.modified()) {
                            found.push((time, item.path()));
                        }
                    }
                }
            }
        }
    }
    let mut found = Vec::new();
    scan(
        &std::env::current_dir().unwrap_or_default(),
        suffix,
        &mut found,
    );
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    found
        .first()
        .map_or(String::new(), |r| r.1.display().to_string())
}
pub fn try_run(args: &[String]) -> Option<Result<(), String>> {
    Some(match args.first()?.to_lowercase().as_str() {
        "kqhelp" => kqhelp(args),
        "symfind" => symfind(args),
        "src2asm" => src2asm(args),
        "romdiff" => romdiff(args),
        _ => return None,
    })
}
fn options(
    args: &[String],
    start: usize,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for arg in &args[start..] {
        let (k, v) = arg.split_once('=').unwrap_or((arg, ""));
        if !allowed.contains(&k) {
            return Err(format!("unknown {} option: {arg}", args[0].to_lowercase()));
        }
        out.insert(k.into(), v.into());
    }
    Ok(out)
}
fn publish(name: &str, text: &str, opt: &BTreeMap<String, String>) -> Result<(), String> {
    let path = opt
        .get("--out")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("kitaqgb_{name}.txt"));
    if path == "-" {
        print!("{text}")
    } else {
        let actual = io::write_utf8_robust(&path, text, true)?;
        if actual != Path::new(&path) {
            eprintln!(
                "warning KQ0000: output is locked, wrote alternate file: {}",
                actual.display()
            );
        }
        println!("[{name}] {}", actual.display());
    }
    Ok(())
}
fn kqhelp(args: &[String]) -> Result<(), String> {
    let s = args
        .get(1)
        .ok_or("kqhelp requires a code (example: KQ2416)")?
        .trim()
        .to_uppercase();
    let n = s
        .strip_prefix("KQ")
        .unwrap_or(&s)
        .parse::<u16>()
        .ok()
        .filter(|n| *n <= 9999)
        .ok_or_else(|| format!("invalid diagnostic code: {}", args[1]))?;
    let (name, desc) = diagnostic_help::help_entry(n);
    let category = match n {
        1000..=1999 => "parser/front-end",
        2000..=2199 => "type/semantic",
        2200..=2299 => "const/qualifier",
        2300..=2399 => "compile-time assert",
        2400..=2499 => "lint/warning",
        2500..=2599 => "extern linkage",
        9000.. => "internal",
        _ => "generic",
    };
    let strict = if (2400..2500).contains(&n) {
        "strict modeではerrorに昇格"
    } else {
        "通常ルール"
    };
    let mut text = format!(
        "# KITAQGB diagnostic help\ncode=KQ{n:04}\nenum={name}\ncategory={category}\nseverity_behavior={strict}\n\n{desc}\n"
    );
    let hint = diagnostics::suggestion(n, "");
    if !hint.is_empty() {
        writeln!(text, "\nsuggestion: {hint}").unwrap();
    }
    print!("{text}");
    Ok(())
}
#[derive(Clone)]
struct Symbol {
    name: String,
    address: Option<i32>,
    bank: Option<i32>,
    offset: i32,
    size: Option<i32>,
    kind: String,
    region: String,
    source: &'static str,
}
fn region(a: i32) -> &'static str {
    match a {
        0xff80..=0xfffe => "HRAM",
        0xff00..=0xff7f => "IO",
        0xfe00..=0xfe9f => "OAM",
        0xd000..=0xdfff => "WRAMX",
        0xc000..=0xcfff => "WRAM0",
        0xa000..=0xbfff => "SRAM",
        0x8000..=0x9fff => "VRAM",
        0..=0x7fff => "ROM",
        _ => "",
    }
}
fn map_symbols(path: &str) -> Vec<Symbol> {
    let mut result = Vec::new();
    if let Ok(text) = io::read_utf8(path) {
        for line in text.lines() {
            let s = line.trim();
            if s.starts_with([';', '#']) {
                continue;
            }
            let parts = s.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 5 {
                continue;
            }
            let Some(address) = hex(parts[0]) else {
                continue;
            };
            let mut i = 1;
            let bank = parts[i].parse::<i32>().ok();
            if bank.is_some() {
                i += 1;
            }
            let Some(offset) = parts.get(i).and_then(|v| hex(v)) else {
                continue;
            };
            i += 1;
            let Some(kind) = parts.get(i) else { continue };
            i += 1;
            let (area, name) = if parts.len() - i >= 2 {
                (parts[i], parts[i + 1])
            } else {
                (region(address), parts[i])
            };
            result.push(Symbol {
                name: name.into(),
                address: Some(address),
                bank,
                offset,
                size: None,
                kind: (*kind).into(),
                region: area.into(),
                source: "map",
            });
        }
    }
    result
}
pub(crate) fn dbg_fields(payload: &str) -> BTreeMap<String, String> {
    let b = payload.as_bytes();
    let mut i = 0;
    let mut fields = BTreeMap::new();
    while i < b.len() {
        while i < b.len() && matches!(b[i], b' ' | b',') {
            i += 1;
        }
        let Some(eq) = payload[i..].find('=') else {
            break;
        };
        let key = payload[i..i + eq].trim().to_lowercase();
        i += eq + 1;
        let value = if b.get(i) == Some(&b'"') {
            i += 1;
            let mut value = String::new();
            let mut escaped = false;
            while i < b.len() {
                let c = payload[i..].chars().next().unwrap();
                i += c.len_utf8();
                if escaped {
                    value.push(c);
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    break;
                } else {
                    value.push(c);
                }
            }
            value
        } else {
            let end = payload[i..].find(',').map_or(b.len(), |n| i + n);
            let value = payload[i..end].trim().to_owned();
            i = end;
            value
        };
        if !key.is_empty() {
            fields.insert(key, value);
        }
    }
    fields
}
fn dbg_symbols(path: &str) -> Vec<Symbol> {
    let mut result = Vec::new();
    if let Ok(text) = io::read_utf8(path) {
        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("sym\t") else {
                continue;
            };
            let f = dbg_fields(rest);
            let name = f.get("name").cloned().unwrap_or_default();
            if name.trim().is_empty() {
                continue;
            }
            let address = f.get("val").and_then(|v| integer(v));
            let bank = ["wbank", "bank", "seg"]
                .iter()
                .find_map(|k| f.get(*k).and_then(|v| integer(v)));
            let area = f
                .get("region")
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| address.map_or("", region).into());
            result.push(Symbol {
                name,
                address,
                bank,
                offset: 0,
                size: f.get("size").and_then(|v| integer(v)),
                kind: "sym".into(),
                region: area,
                source: "dbg",
            });
        }
    }
    result
}
fn symfind(args: &[String]) -> Result<(), String> {
    let pattern = args.get(1).ok_or("symfind requires a pattern")?;
    let opt = options(
        args,
        2,
        &["--map", "--dbg", "--regex", "--ignore-case", "--out"],
    )?;
    let map = opt
        .get("--map")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| newest(".map"));
    let dbg = opt
        .get("--dbg")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| newest(".dbg"));
    let mut symbols = map_symbols(&map);
    symbols.extend(dbg_symbols(&dbg));
    if symbols.is_empty() {
        return Err("no symbols loaded (specify --map and/or --dbg)".into());
    }
    let ignore = opt.contains_key("--ignore-case");
    let use_regex = opt.contains_key("--regex");
    let re = if use_regex {
        Some(
            fancy_regex::RegexBuilder::new(pattern)
                .case_insensitive(ignore)
                .build()
                .map_err(|e| format!("invalid regex: {e}"))?,
        )
    } else {
        None
    };
    let mut matched = Vec::new();
    for s in symbols {
        let hit = if let Some(re) = &re {
            re.is_match(&s.name)
                .map_err(|e| format!("regex match failed: {e}"))?
        } else if ignore {
            s.name.to_lowercase().contains(&pattern.to_lowercase())
        } else {
            s.name.contains(pattern)
        };
        if hit {
            matched.push(s);
        }
    }
    let mut symbols = matched;
    symbols.sort_by(|a, b| ordinal(&a.name, &b.name).then(a.source.cmp(b.source)));
    let mut text = format!(
        "# KITAQGB symbol search\npattern={pattern}\nuse_regex={}\nignore_case={}\nmap={}\ndbg={}\nmatch_count={}\n\n# name, address, bank, size, kind, region, source\n",
        u8::from(use_regex),
        u8::from(ignore),
        if map.is_empty() { "<none>" } else { &map },
        if dbg.is_empty() { "<none>" } else { &dbg },
        symbols.len()
    );
    for s in symbols {
        writeln!(
            text,
            "{}, {}, {}, {}, {}, {}, {}",
            s.name,
            s.address
                .map_or(String::new(), |a| format!("0x{:04X}", a & 65535)),
            s.bank.map_or(String::new(), |n| n.to_string()),
            s.size.map_or(String::new(), |n| n.to_string()),
            s.kind,
            s.region,
            s.source
        )
        .unwrap();
    }
    publish("symfind", &text, &opt)
}
#[derive(Clone)]
pub(crate) struct FunctionRange {
    pub name: String,
    pub start: i32,
    pub end: i32,
}
impl FunctionRange {
    fn size(&self) -> i32 {
        (self.end - self.start).max(0)
    }
}
pub(crate) fn function_ranges(path: &str) -> Vec<FunctionRange> {
    let mut out = Vec::new();
    if let Ok(text) = io::read_utf8(path) {
        for raw in text.lines() {
            if raw.starts_with('#') {
                continue;
            }
            let p = raw.split(',').map(str::trim).collect::<Vec<_>>();
            if p.len() < 6 || p[0].is_empty() {
                continue;
            }
            let (Some(start), Some(end)) = (integer(p[3]), integer(p[4])) else {
                continue;
            };
            out.push(FunctionRange {
                name: p[0].into(),
                start,
                end,
            });
        }
    }
    out
}
fn map_ranges(path: &str, size: usize) -> Vec<FunctionRange> {
    let mut entries = map_symbols(path)
        .into_iter()
        .filter(|s| s.region.eq_ignore_ascii_case("ROM") && s.kind.eq_ignore_ascii_case("S"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|s| (s.bank.unwrap_or(0), s.offset));
    entries
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let bank = s.bank.unwrap_or(0);
            let start = bank * 0x4000 + (s.offset & 0x3fff);
            let end = entries[i + 1..]
                .iter()
                .take_while(|t| t.bank.unwrap_or(0) == bank)
                .map(|t| bank * 0x4000 + (t.offset & 0x3fff))
                .find(|n| *n > start)
                .unwrap_or(start + 1);
            FunctionRange {
                name: s.name.clone(),
                start,
                end: if size > 0 { end.min(size as i32) } else { end },
            }
        })
        .collect()
}
fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let (mut i, mut depth, mut state) = (open, 0i32, 0u8);
    while i < bytes.len() {
        let c = bytes[i];
        let n = bytes.get(i + 1).copied().unwrap_or(0);
        match state {
            1 => {
                if c == b'\n' {
                    state = 0
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
                    state = 0
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
                        return Some(i);
                    }
                }
            }
        }
        i += 1;
    }
    None
}
pub(crate) fn source_functions(text: &str) -> Vec<(String, usize, usize)> {
    let re=Regex::new(r"(?m)^[ \t]*(?:[A-Za-z_][A-Za-z0-9_\s\*\[\]]*?)\b([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*\{").unwrap();
    let mut out = Vec::new();
    for m in re.captures_iter(text) {
        let all = m.get(0).unwrap();
        let name = m.get(1).unwrap().as_str();
        if matches!(
            name,
            "if" | "for"
                | "while"
                | "switch"
                | "return"
                | "sizeof"
                | "static_assert"
                | "_Static_assert"
        ) {
            continue;
        }
        if let Some(end) = matching_brace(text.as_bytes(), all.end() - 1) {
            out.push((
                name.to_owned(),
                text[..all.start()].bytes().filter(|b| *b == b'\n').count() + 1,
                text[..end].bytes().filter(|b| *b == b'\n').count() + 1,
            ));
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1).then(ordinal(&a.0, &b.0)));
    out
}
fn src2asm(args: &[String]) -> Result<(), String> {
    let spec = args.get(1).ok_or("src2asm requires source.c:line")?;
    let opt = options(
        args,
        2,
        &["--disasm", "--funcsizes", "--map", "--context", "--out"],
    )?;
    let context = opt
        .get("--context")
        .map_or(Ok(2), |s| s.parse::<usize>())
        .ok()
        .filter(|n| *n <= 64)
        .ok_or("--context must be 0..64")?;
    let (source, line) = spec
        .rsplit_once(':')
        .ok_or("src2asm expects source.c:line")?;
    let line = line
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or("src2asm expects source.c:line")?;
    let source = full(source);
    if !source.is_file() {
        return Err(format!("source file not found: {}", source.display()));
    }
    let dis = opt
        .get("--disasm")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            if Path::new("debug_output/dis.s").is_file() {
                Path::new("debug_output")
                    .join("dis.s")
                    .display()
                    .to_string()
            } else {
                newest("dis.s")
            }
        });
    let funcs = opt
        .get("--funcsizes")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| newest(".funcsizes.txt"));
    let map = opt
        .get("--map")
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| newest(".map"));
    if !Path::new(&dis).is_file() {
        return Err("disassembly not found (use --disasm)".into());
    }
    let functions = source_functions(&io::read_utf8(&source)?);
    let target = functions
        .iter()
        .find(|f| line >= f.1 && line <= f.2)
        .ok_or_else(|| format!("no function found at line {line}"))?;
    let ranges = if Path::new(&funcs).is_file() {
        function_ranges(&funcs)
    } else {
        map_ranges(&map, 0)
    };
    let range = ranges
        .iter()
        .find(|r| r.name == target.0)
        .ok_or_else(|| format!("function '{}' not found in funcsizes/map", target.0))?;
    let listing = io::read_utf8(&dis)?;
    let lines = listing.lines().collect::<Vec<_>>();
    let re = Regex::new(r";\s*file_off=\$([0-9A-Fa-f]+)").unwrap();
    let hits = lines
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            re.captures(s)
                .and_then(|m| hex(&m[1]))
                .filter(|n| *n >= 0 && *n >= range.start && *n < range.end)
                .map(|_| i)
        })
        .collect::<Vec<_>>();
    let mut text = format!(
        "# KITAQGB source -> assembly lookup\nsource={}:{}\nfunction={} ({}-{})\nrange=0x{:05X}..0x{:05X}\ndisasm={dis}\n\n",
        source.display(),
        line,
        target.0,
        target.1,
        target.2,
        range.start,
        range.end
    );
    if let (Some(first), Some(last)) = (hits.first(), hits.last()) {
        text.push_str("# disassembly excerpt\n");
        for s in &lines[first.saturating_sub(context)..=(*last + context).min(lines.len() - 1)] {
            writeln!(text, "{s}").unwrap();
        }
    } else {
        text.push_str("# no disassembly lines matched this function range\n");
    }
    publish("src2asm", &text, &opt)
}
/// File-based Git selection, with the original source-list fallback.
pub(crate) fn write_changed(
    inputs: &[PathBuf],
    assembled: &crate::assembler::Output,
    base: &str,
    out: Option<&str>,
    debug: &Path,
) -> Result<PathBuf, String> {
    fn git(args: &[&str]) -> Option<Vec<u8>> {
        let o = std::process::Command::new("git").args(args).output().ok()?;
        if o.status.success() {
            Some(o.stdout)
        } else {
            None
        }
    }
    let mut files = BTreeSet::new();
    if let Some(root) = git(&["rev-parse", "--show-toplevel"]) {
        let root = PathBuf::from(String::from_utf8_lossy(&root).trim());
        let revision = if base.trim().is_empty() {
            Some(String::new())
        } else {
            git(&[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{base}^{{commit}}"),
            ])
            .map(|s| String::from_utf8_lossy(&s).trim().to_owned())
        };
        if let Some(revision) = revision {
            let mut args = vec!["diff", "--name-only", "-z", "--diff-filter=ACMRTUXB"];
            if !revision.is_empty() {
                args.push(&revision);
            }
            args.push("--");
            let mut paths = git(&args).unwrap_or_default();
            if base.trim().is_empty() {
                paths.extend(
                    git(&["ls-files", "--others", "--exclude-standard", "-z"]).unwrap_or_default(),
                );
            }
            for path in paths.split(|b| *b == 0).filter(|b| !b.is_empty()) {
                let path = root.join(String::from_utf8_lossy(path).as_ref());
                if path.is_file()
                    && path.extension().is_some_and(|s| {
                        matches!(s.to_string_lossy().to_lowercase().as_str(), "c" | "h")
                    })
                {
                    files.insert(path);
                }
            }
        }
    }
    if files.is_empty() {
        files.extend(
            inputs
                .iter()
                .map(crate::workflow::full)
                .filter(|p| p.is_file()),
        );
    }
    let mut names = BTreeSet::new();
    for file in &files {
        for (name, _, _) in source_functions(&io::read_utf8(file)?) {
            names.insert(name);
        }
    }
    let mut ranges = assembled
        .report
        .functions
        .iter()
        .filter(|f| names.contains(&f.name))
        .collect::<Vec<_>>();
    ranges.sort_by_key(|f| f.start);
    let path = out
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| debug.join("dis_changed.s"), PathBuf::from);
    let mut text = format!(
        "; KITAQGB changed-function disassembly\n; base_ref={}\n; changed_source_files={}\n; changed_functions={}\n; emitted_functions={}\n\n",
        if base.trim().is_empty() {
            "<working_tree>"
        } else {
            base
        },
        files.len(),
        names.len(),
        ranges.len()
    );
    if ranges.is_empty() {
        text.push_str("; no changed functions found\n");
    } else {
        let listing = io::read_utf8(debug.join("dis.s"))?;
        let re = Regex::new(r";\s*file_off=\$([0-9A-Fa-f]+)").unwrap();
        let lines = listing
            .lines()
            .filter_map(|s| re.captures(s).and_then(|m| hex(&m[1])).map(|n| (n, s)))
            .collect::<Vec<_>>();
        for f in ranges {
            write!(text,"; ------------------------------------------------------------\n; function {}  file_off=${:05X}..${:05X}\n; ------------------------------------------------------------\n",f.name,f.start,f.end).unwrap();
            let mut count = 0;
            for (_, line) in lines.iter().filter(|(n, _)| *n >= f.start && *n < f.end) {
                writeln!(text, "{line}").unwrap();
                count += 1;
            }
            if count == 0 {
                text.push_str("; (no disassembly lines found)\n");
            }
            text.push('\n');
        }
    }
    io::write_utf8_robust(&path, &text, true)
}
fn slice<'a>(rom: &'a [u8], range: Option<&FunctionRange>) -> &'a [u8] {
    if let Some(r) = range {
        let start = r.start.clamp(0, rom.len() as i32) as usize;
        let end = r.end.clamp(start as i32, rom.len() as i32) as usize;
        &rom[start..end]
    } else {
        &[]
    }
}
fn short_hash(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        String::new()
    } else {
        hash::sha256(bytes)[..8].into()
    }
}
fn romdiff(args: &[String]) -> Result<(), String> {
    if args.len() < 3 {
        return Err("romdiff requires <old.gb> <new.gb>".into());
    }
    let opt = options(
        args,
        3,
        &[
            "--old-func",
            "--new-func",
            "--old-map",
            "--new-map",
            "--out",
        ],
    )?;
    let mut files = Vec::new();
    let mut ranges = Vec::new();
    let mut roms = Vec::new();
    for (side, path) in [("old", &args[1]), ("new", &args[2])] {
        if !Path::new(path).is_file() {
            return Err("ROM file not found".into());
        }
        let rom = std::fs::read(path).map_err(|e| e.to_string())?;
        let f = opt
            .get(&format!("--{side}-func"))
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                Path::new(path)
                    .with_extension("funcsizes.txt")
                    .display()
                    .to_string()
            });
        let m = opt
            .get(&format!("--{side}-map"))
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| Path::new(path).with_extension("map").display().to_string());
        let list = if Path::new(&f).is_file() {
            files.push(f.clone());
            function_ranges(&f)
        } else {
            files.push(m.clone());
            map_ranges(&m, rom.len())
        };
        let mut map = BTreeMap::new();
        for r in list {
            map.entry(r.name.clone()).or_insert(r);
        }
        ranges.push(map);
        roms.push(rom);
    }
    if ranges.iter().all(|r| r.is_empty()) {
        return Err("function range info not found (funcsizes/map)".into());
    }
    let names = ranges
        .iter()
        .flat_map(|r| r.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();
    for name in names {
        let old = ranges[0].get(&name);
        let new = ranges[1].get(&name);
        let os = old.map_or(0, FunctionRange::size);
        let ns = new.map_or(0, FunctionRange::size);
        let ob = slice(&roms[0], old);
        let nb = slice(&roms[1], new);
        let changed = if old.is_none() {
            ns
        } else if new.is_none() {
            os
        } else {
            (ob.len().abs_diff(nb.len()) + ob.iter().zip(nb).filter(|(a, b)| a != b).count()) as i32
        };
        let status = if old.is_none() {
            "added"
        } else if new.is_none() {
            "removed"
        } else if changed == 0 {
            "same"
        } else {
            "changed"
        };
        rows.push((
            name,
            status,
            os,
            ns,
            ns - os,
            changed,
            short_hash(ob),
            short_hash(nb),
        ));
    }
    rows.sort_by(|a, b| {
        b.5.cmp(&a.5)
            .then(b.4.abs().cmp(&a.4.abs()))
            .then(ordinal(&a.0, &b.0))
    });
    let count = |status: &str| rows.iter().filter(|r| r.1 == status).count();
    let mut text = format!(
        "# KITAQGB ROM diff (function-level)\nold_rom={}\nnew_rom={}\nold_ranges={}\nnew_ranges={}\nfunction_count={}\nchanged={}, added={}, removed={}, same={}\n\n# name, status, old_size, new_size, delta, changed_bytes, old_hash8, new_hash8\n",
        args[1],
        args[2],
        files[0],
        files[1],
        rows.len(),
        count("changed"),
        count("added"),
        count("removed"),
        count("same")
    );
    for (n, s, o, v, d, c, h, j) in rows {
        writeln!(text, "{n}, {s}, {o}, {v}, {d}, {c}, {h}, {j}").unwrap();
    }
    publish("romdiff", &text, &opt)
}
