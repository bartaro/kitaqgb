//! Command-history recipes and replay scripts. Recorded arguments remain data.
use crate::{io, workflow};
use std::{fmt::Write, path::Path};

/// Decode the native argument quoting used by command history, including legacy
/// unquoted arguments. This is the inverse of `workflow::quote_arg`.
pub fn arguments(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut chars = line.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut text = String::new();
        let mut quoted = false;
        while let Some(c) = chars.peek().copied() {
            if !quoted && c.is_whitespace() {
                break;
            }
            chars.next();
            if c == '\\' {
                let mut slashes = 1;
                while chars.peek() == Some(&'\\') {
                    chars.next();
                    slashes += 1;
                }
                if chars.peek() == Some(&'"') {
                    text.push_str(&"\\".repeat(slashes / 2));
                    chars.next();
                    if slashes % 2 != 0 {
                        text.push('"');
                    } else {
                        quoted = !quoted;
                    }
                } else {
                    text.push_str(&"\\".repeat(slashes));
                }
            } else if c == '"' {
                quoted = !quoted;
            } else {
                text.push(c);
            }
        }
        args.push(text);
    }
    args
}

// Histories written by KITAQGB use ISO 8601. Accept optional fractional seconds,
// UTC offsets and a space separator as well as the original round-trip format.
fn datetime(s: &str) -> Option<(i64, u32)> {
    let re = regex::Regex::new(r"^(\d{4})[-/](\d{1,2})[-/](\d{1,2})[T ](\d{1,2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?(Z|[+-]\d{2}:?\d{2})?$").ok()?;
    let c = re.captures(s.trim())?;
    let n = |i| c.get(i)?.as_str().parse::<i64>().ok();
    let (y, m, d, h, mi, se) = (n(1)?, n(2)?, n(3)?, n(4)?, n(5)?, n(6)?);
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return None,
    };
    if d < 1 || d > days || h > 23 || mi > 59 || se > 59 {
        return None;
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let mut secs = (era * 146097 + doe - 719468) * 86400 + h * 3600 + mi * 60 + se;
    if let Some(tz) = c.get(8).map(|s| s.as_str()).filter(|s| *s != "Z") {
        let digits = tz[1..].replace(':', "");
        let hours = digits[..2].parse::<i64>().ok()?;
        let minutes = digits[2..].parse::<i64>().ok()?;
        if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
            return None;
        }
        secs -= (hours * 60 + minutes) * 60 * if tz.starts_with('-') { -1 } else { 1 };
    }
    let fraction = c.get(7).map_or(0, |v| {
        let mut s = v.as_str().chars().take(7).collect::<String>();
        while s.len() < 7 {
            s.push('0');
        }
        s.parse().unwrap_or(0)
    });
    Some((secs, fraction))
}
struct Entry {
    time: (i64, u32),
    dir: String,
    line: String,
}
fn first(e: &Entry) -> String {
    arguments(&e.line)
        .first()
        .map_or(String::new(), |s| s.to_lowercase())
}
fn utility(name: &str) -> bool {
    matches!(
        name,
        "test"
            | "attrviz"
            | "src2asm"
            | "symfind"
            | "romdiff"
            | "kqhelp"
            | "template"
            | "fixhint"
            | "irsum"
            | "conventions"
            | "snippet"
            | "devserver"
            | "recipe"
            | "disasm"
            | "header"
            | "assemble-ir"
            | "tokenize"
            | "parse"
            | "lower"
            | "emit-ir"
            | "optimize-ir"
            | "strip-ir"
    )
}
pub fn run(args: &[String]) -> Result<(), String> {
    let mut history = workflow::history_path().display().to_string();
    let (mut out, mut script) = (String::new(), String::new());
    let (mut stdout, mut last) = (false, 30usize);
    for a in &args[1..] {
        if a == "--stdout" || a == "-" {
            stdout = true;
        } else if let Some(s) = a.strip_prefix("--history=") {
            history = s.into();
        } else if let Some(s) = a.strip_prefix("--out=") {
            out = s.into();
        } else if let Some(s) = a.strip_prefix("--script=") {
            script = s.into();
        } else if let Some(s) = a.strip_prefix("--last=") {
            last = s
                .parse()
                .ok()
                .filter(|n| (1..=10000).contains(n))
                .ok_or("--last must be 1..10000")?;
        } else {
            return Err(format!("unknown recipe option: {a}"));
        }
    }
    if history.trim().is_empty() {
        return Err("recipe history path is empty".into());
    }
    if !Path::new(&history).is_file() {
        return Err(format!("history file not found: {history}"));
    }
    let mut entries = Vec::new();
    for line in io::read_utf8(&history)?.lines() {
        let p = line.splitn(3, '\t').collect::<Vec<_>>();
        if p.len() == 3 {
            if let Some(time) = datetime(p[0]) {
                entries.push(Entry {
                    time,
                    dir: p[1].into(),
                    line: p[2].into(),
                });
            }
        }
    }
    if entries.is_empty() {
        return Err(format!("history is empty: {history}"));
    }
    entries.sort_by_key(|e| e.time);
    let tail = &entries[entries.len().saturating_sub(last)..];
    let compile = tail.iter().rev().find(|e| {
        let f = first(e);
        !f.is_empty() && !utility(&f)
    });
    let test = tail.iter().rev().find(|e| first(e) == "test");
    let dev = tail.iter().rev().find(|e| first(e) == "devserver");
    let selected = [("Compile", compile), ("Test", test), ("Devserver", dev)];
    let mut text = format!(
        "# KITAQGB Repro Recipe\n\ngenerated_utc={}\nhistory_file={}\nhistory_entries={}\n\n## Recommended Steps\n1. Move to working directory.\n2. Re-run the latest compile command.\n3. Re-run test/devserver command if needed.\n\n",
        workflow::utc_now(),
        workflow::full(&history).display(),
        tail.len()
    );
    for (label, e) in selected {
        if let Some(e) = e {
            write!(
                text,
                "### {label}\n- cwd: `{}`\n- cmd: `kitaqgb {}`\n\n",
                e.dir, e.line
            )
            .unwrap();
        } else if label == "Compile" {
            text.push_str("### Compile\n- Not found in selected history window.\n\n");
        }
    }
    text.push_str("## Recent Commands\n");
    for e in &tail[tail.len().saturating_sub(10)..] {
        writeln!(
            text,
            "- {} | `{}`",
            workflow::timestamp_signed(e.time.0, e.time.1),
            e.line
        )
        .unwrap();
    }
    if stdout {
        print!("{text}");
        return Ok(());
    }
    if out.trim().is_empty() {
        out = workflow::repo_root()
            .join("integration_test/reports/RECIPE_LATEST.md")
            .display()
            .to_string();
    }
    if script.trim().is_empty() {
        script = Path::new(&out)
            .parent()
            .unwrap_or(Path::new("."))
            .join("RECIPE_RUN.ps1")
            .display()
            .to_string();
    }
    let out = io::publish_utf8(&out, &text)?;
    let mut ps = include_str!("recipe_header.ps1").to_owned();
    let literal = |s: &str| format!("'{}'", s.replace('\'', "''"));
    let shliteral = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let mut sh = String::from(
        "#!/bin/sh\nset -eu\ncompiler=${1:-kitaqgb}\ncase \"$compiler\" in /*) ;; */*) compiler=\"$(pwd)/$compiler\" ;; esac\n",
    );
    for (_, e) in selected {
        if let Some(e) = e {
            let dir = if e.dir.trim().is_empty() {
                std::env::current_dir()
                    .unwrap_or_default()
                    .display()
                    .to_string()
            } else {
                e.dir.clone()
            };
            write!(ps,"$replayExit = Invoke-RecordedCompiler -Directory {} -Arguments {}\nif ($replayExit -ne 0) {{ exit $replayExit }}\n\n",literal(&dir),literal(&e.line)).unwrap();
            write!(sh, "( cd -- {}\n  \"$compiler\"", shliteral(&dir)).unwrap();
            for a in arguments(&e.line) {
                write!(sh, " {}", shliteral(&a)).unwrap();
            }
            sh.push_str("\n)\n");
        }
    }
    ps.push_str("exit 0\n");
    let script = io::publish_utf8(&script, &ps)?;
    let shpath = Path::new(&script).with_extension("sh");
    let shpath = io::publish_utf8(&shpath, &sh)?;
    println!(
        "[recipe] {}\n[recipe] script: {}\n[recipe] script: {}",
        out.display(),
        script.display(),
        shpath.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn recorded_arguments_round_trip() {
        let args = [
            "",
            "a.c",
            "a b",
            "quote\"",
            "C:\\end\\",
            "\\\\\"",
            "$(anything); & %PATH%",
        ];
        let line = args
            .iter()
            .map(|a| crate::workflow::quote_arg(a))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(super::arguments(&line), args);
        assert_eq!(
            super::arguments("a.c -o \"a b.gb\""),
            ["a.c", "-o", "a b.gb"]
        );
    }
    #[test]
    fn history_timestamps_normalize_offsets() {
        assert_eq!(
            super::datetime("2000-02-29T09:00:00.123+09:00"),
            Some((951782400, 1230000))
        );
        assert!(super::datetime("2001-02-29T00:00:00Z").is_none());
        assert!(super::datetime("2000-02-29T00:00:00+14:01").is_none());
    }
}
