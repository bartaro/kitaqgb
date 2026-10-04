//! Prototype, snippet, diagnostic-hint and IR-summary commands.
use crate::{debug_tools, diagnostics, io, json::Value, workflow};
use regex::Regex;
use std::{collections::BTreeMap, fmt::Write, path::Path};
fn field<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    if let Value::Object(f) = v {
        f.get(k)
    } else {
        None
    }
}
fn string(v: &Value, k: &str) -> String {
    if let Some(Value::String(s)) = field(v, k) {
        s.clone()
    } else {
        String::new()
    }
}
fn number(v: &Value, k: &str) -> i64 {
    if let Some(Value::Number(n)) = field(v, k) {
        *n
    } else {
        0
    }
}
fn data() -> Value {
    Value::parse(include_str!("tools_data.json")).expect("audited built-in tool data")
}
fn publish(name: &str, path: &str, text: &str) -> Result<(), String> {
    if path == "-" {
        print!("{text}")
    } else {
        let actual = io::write_utf8_robust(path, text, true)?;
        if actual != Path::new(path) {
            eprintln!(
                "warning KQ0000: output is locked, wrote alternate file: {}",
                actual.display()
            );
        }
        println!("[{name}] {}", actual.display());
    }
    Ok(())
}
pub fn try_run(args: &[String]) -> Option<Result<(), String>> {
    Some(match args.first()?.to_lowercase().as_str() {
        "template" => template(args),
        "conventions" => conventions(args),
        "snippet" => snippet(args),
        "fixhint" => fixhint(args),
        "irsum" => irsum(args),
        "attrviz" => attrviz(args),
        "recipe" => crate::recipe::run(args),
        _ => return None,
    })
}
fn template(args: &[String]) -> Result<(), String> {
    let mut output = workflow::full("quickstart.c").display().to_string();
    let (mut overwrite, mut script, mut readme, mut cgb) = (false, true, true, false);
    for a in &args[1..] {
        match a.as_str() {
            "--overwrite" => overwrite = true,
            "--no-build-script" => script = false,
            "--no-readme" => readme = false,
            "--cgb" => cgb = true,
            s if s.starts_with("--out=") => output = s[6..].into(),
            s if !s.starts_with('-') => output = s.into(),
            _ => return Err(format!("unknown template option: {a}")),
        }
    }
    if output.trim().is_empty() {
        return Err("template output path is empty".into());
    }
    if Path::new(&output).extension().is_none() {
        output.push_str(".c");
    }
    let output = workflow::full(output);
    if output.is_file() && !overwrite {
        return Err(format!(
            "output exists (use --overwrite): {}",
            output.display()
        ));
    }
    let original_stem = output
        .file_stem()
        .ok_or("template path has no filename")?
        .to_string_lossy();
    let data = data();
    let output = io::publish_utf8(
        &output,
        &string(&data, if cgb { "template_cgb" } else { "template_dmg" })
            .replace("__STEM__", &original_stem),
    )?;
    let stem = output.file_stem().unwrap().to_string_lossy();
    let source = output.file_name().unwrap().to_string_lossy();
    let rom = format!("{stem}.gb");
    println!("[template] source: {}", output.display());
    let parent = output.parent().unwrap();
    if script {
        let path = parent.join(format!("{stem}_build.ps1"));
        let text = string(&data, "build_ps1")
            .replace("__SOURCE__", &source.replace('\'', "''"))
            .replace("__ROM__", &rom.replace('\'', "''"));
        let path = io::publish_utf8(&path, &text)?;
        println!("[template] build script: {}", path.display());
        // Keep the original PowerShell sidecar and provide a native POSIX entry point.
        let sh_quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let sh = format!(
            "#!/bin/sh\nset -eu\ncd -- \"$(dirname -- \"$0\")\"\nprofile=dev\nif [ \"${{1-}}\" = --release ]; then profile=release; fi\nexec kitaqgb {} -o {} --profile=\"$profile\" --cache --deps-out\n",
            sh_quote(&source),
            sh_quote(&rom)
        );
        let path = parent.join(format!("{stem}_build.sh"));
        io::publish_utf8(&path, &sh)?;
    }
    if readme {
        let path = parent.join(format!("{stem}_README.md"));
        let text = format!(
            "# Quick Prototype\n\n1. Build:\n   `kitaqgb {source} -o {rom} --profile=dev --fast-build --cache --deps-out`\n2. Run on emulator/hardware and iterate.\n3. Add snippets with `kitaqgb snippet list` / `kitaqgb snippet get <id>`.\n"
        );
        let path = io::publish_utf8(&path, &text)?;
        println!("[template] readme: {}", path.display());
    }
    let display_arg = |s: &str| {
        if s.chars().any(|c| c.is_whitespace() || c == '"') {
            workflow::quote_arg(s)
        } else {
            s.into()
        }
    };
    println!(
        "[template] instant build:\nkitaqgb {} -o {} --profile=dev --fast-build --cache --deps-out",
        display_arg(&source),
        display_arg(&rom)
    );
    Ok(())
}
fn attrviz(args: &[String]) -> Result<(), String> {
    let input = args.get(1).ok_or("attrviz requires an input file")?;
    let (mut width, mut height, mut output) = (32usize, 0usize, String::new());
    for arg in &args[2..] {
        if let Some(s) = arg.strip_prefix("--width=") {
            width = s
                .parse()
                .ok()
                .filter(|n| (1..=1024).contains(n))
                .ok_or("--width must be 1..1024")?;
        } else if let Some(s) = arg.strip_prefix("--height=") {
            height = s
                .parse()
                .ok()
                .filter(|n| *n <= 1024)
                .ok_or("--height must be 0..1024")?;
        } else if let Some(s) = arg.strip_prefix("--out=") {
            output = s.into();
        } else {
            return Err(format!("unknown attrviz option: {arg}"));
        }
    }
    let bytes = std::fs::read(input).map_err(|_| format!("attrviz input not found: {input}"))?;
    if height == 0 {
        height = bytes.len().div_ceil(width);
    }
    let mut text = format!(
        "# KITAQGB tile attribute map visualization\nsource={input}\nbytes={}\nwidth={width}\nheight={height}\ncells={}\n\n# bit layout: b0-2=palette b3=vram_bank b4=dmg_palette b5=xflip b6=yflip b7=priority\n\n# summary\n",
        bytes.len(),
        width * height
    );
    for pal in 0..8 {
        writeln!(
            text,
            "palette_{pal}={}",
            bytes.iter().filter(|b| **b & 7 == pal).count()
        )
        .unwrap();
    }
    for (mask, name) in [
        (8, "vram_bank_1"),
        (16, "dmg_palette_1"),
        (32, "xflip_1"),
        (64, "yflip_1"),
        (128, "priority_1"),
    ] {
        writeln!(
            text,
            "{name}={}",
            bytes.iter().filter(|b| **b & mask != 0).count()
        )
        .unwrap();
    }
    text.push_str("\n# grid raw(hex)\n");
    for y in 0..height {
        write!(text, "{y:03}: ").unwrap();
        for x in 0..width {
            if x > 0 {
                text.push(' ');
            }
            if let Some(b) = bytes.get(y * width + x) {
                write!(text, "{b:02X}").unwrap();
            } else {
                text.push_str("--");
            }
        }
        text.push('\n');
    }
    text.push_str("\n# grid decoded\n# token format: p<pal><B|.><D|.><X|.><Y|.><P|.>\n");
    for y in 0..height {
        write!(text, "{y:03}: ").unwrap();
        for x in 0..width {
            if x > 0 {
                text.push(' ');
            }
            if let Some(b) = bytes.get(y * width + x) {
                write!(text, "p{}", b & 7).unwrap();
                for (mask, c) in [(8, 'B'), (16, 'D'), (32, 'X'), (64, 'Y'), (128, 'P')] {
                    text.push(if b & mask != 0 { c } else { '.' });
                }
            } else {
                text.push_str("------");
            }
        }
        text.push('\n');
    }
    if output.trim().is_empty() {
        output = workflow::full(input)
            .with_extension("attrviz.txt")
            .display()
            .to_string();
    }
    publish("attrviz", &output, &text)
}
fn conventions(args: &[String]) -> Result<(), String> {
    let mut path = String::new();
    let mut stdout = false;
    for a in &args[1..] {
        if matches!(a.as_str(), "--stdout" | "-") {
            stdout = true
        } else if let Some(p) = a.strip_prefix("--out=") {
            path = p.into()
        } else {
            return Err(format!("unknown conventions option: {a}"));
        }
    }
    if stdout {
        path = "-".into()
    } else if path.trim().is_empty() {
        path = workflow::repo_root()
            .join("integration_test/reports/PROJECT_CONVENTIONS.md")
            .display()
            .to_string();
    }
    publish(
        "conventions",
        &path,
        &string(&data(), "conventions").replace("__UTC__", &workflow::utc_now()),
    )
}
fn snippet(args: &[String]) -> Result<(), String> {
    let sub = args.get(1).ok_or("snippet requires subcommand")?;
    let value = data();
    let Some(Value::Array(items)) = field(&value, "snippets") else {
        unreachable!()
    };
    let mut items = items.iter().collect::<Vec<_>>();
    items.sort_by_key(|s| (string(s, "category"), string(s, "id")));
    if sub.eq_ignore_ascii_case("list") {
        let mut text = String::from("# KITAQGB snippet library\n");
        for s in items {
            writeln!(
                text,
                "{}  [{}]  {}",
                string(s, "id"),
                string(s, "category"),
                string(s, "title")
            )
            .unwrap();
        }
        print!("{text}");
        return Ok(());
    }
    let selector = args.get(2).ok_or_else(|| {
        format!(
            "snippet {sub} requires <{}>",
            if sub.eq_ignore_ascii_case("get") {
                "id"
            } else {
                "category|all"
            }
        )
    })?;
    let mut path = String::from("-");
    for a in &args[3..] {
        if matches!(a.as_str(), "--stdout" | "-") {
            path = "-".into()
        } else if let Some(p) = a.strip_prefix("--out=") {
            path = p.into()
        } else {
            return Err(format!("unknown snippet {sub} option: {a}"));
        }
    }
    let text = if sub.eq_ignore_ascii_case("get") {
        let s = items
            .into_iter()
            .find(|s| string(s, "id") == *selector)
            .ok_or_else(|| format!("snippet not found: {selector}"))?;
        format!("{}\n", string(s, "body").trim_end())
    } else if sub.eq_ignore_ascii_case("emit") {
        items.retain(|s| {
            selector.eq_ignore_ascii_case("all")
                || string(s, "category").eq_ignore_ascii_case(selector)
        });
        if items.is_empty() {
            return Err(format!("no snippets found for category: {selector}"));
        }
        let mut text = format!(
            "// KITAQGB snippet bundle\n// category={}\n\n",
            selector.trim()
        );
        for s in items {
            writeln!(
                text,
                "// --- {} [{}] {} ---\n{}\n",
                string(s, "id"),
                string(s, "category"),
                string(s, "title"),
                string(s, "body").trim_end()
            )
            .unwrap();
        }
        text
    } else {
        return Err(format!("unknown snippet subcommand: {sub}"));
    };
    publish("snippet", &path, &text)
}
#[derive(Default)]
struct Diag {
    severity: String,
    code: String,
    message: String,
    suggestion: String,
    file: String,
    line: i64,
}
fn fallback(message: &str) -> &'static str {
    let m = message.to_lowercase();
    if m.trim().is_empty() {
        "Run again with --diag-json for structured hints."
    } else if m.contains("unknown option") {
        "Unknown option. Check spelling with `kitaqgb --help`."
    } else if m.contains("no source files") {
        "Pass one or more `.c` source files."
    } else if m.contains("expected") {
        "Check nearby tokens like `;`, `)`, `]`, or `}`."
    } else if m.contains("undefined") || m.contains("not found") {
        "Verify declarations/definitions and symbol scope."
    } else if m.contains("type") {
        "Review type annotations and add explicit casts where needed."
    } else if m.contains("range") {
        "Adjust the declared `__range` or clamp assigned values."
    } else if m.contains("overflow") {
        "Reduce ROM/bank pressure or increase cartridge size config."
    } else {
        "Reduce the code to a minimal repro, then reintroduce pieces incrementally."
    }
}
fn fixhint(args: &[String]) -> Result<(), String> {
    let (mut input, mut output, mut max) = (String::new(), String::from("-"), 20usize);
    for a in &args[1..] {
        if matches!(a.as_str(), "--stdout" | "-") {
            output = "-".into()
        } else if let Some(p) = a.strip_prefix("--in=") {
            input = p.into()
        } else if let Some(p) = a.strip_prefix("--out=") {
            output = p.into()
        } else if let Some(p) = a.strip_prefix("--max=") {
            max = p
                .parse()
                .ok()
                .filter(|n| (1..=200).contains(n))
                .ok_or("--max must be 1..200")?;
        } else if !a.starts_with('-') && input.trim().is_empty() {
            input = a.clone()
        } else {
            return Err(format!("unknown fixhint option: {a}"));
        }
    }
    if input.trim().is_empty() {
        input = if Path::new("kitaqgb.diag.json").is_file() {
            "kitaqgb.diag.json".into()
        } else {
            [".diag.json", ".stderr.log", ".log"]
                .into_iter()
                .map(debug_tools::newest)
                .find(|s| !s.is_empty())
                .unwrap_or_default()
        };
    }
    if !Path::new(&input).is_file() {
        return Err("no input log/diag found for fixhint".into());
    }
    let text = io::read_utf8(&input)?;
    let mut records = Vec::new();
    if text.contains("\"diagnostics\"") && text.contains("\"severity\"") {
        // Parse fields by name rather than requiring the C# serializer's property order.
        if let Ok(json) = Value::parse(&text) {
            if let Some(Value::Array(values)) = field(&json, "diagnostics") {
                for v in values {
                    records.push(Diag {
                        severity: string(v, "severity"),
                        code: string(v, "code"),
                        message: string(v, "message"),
                        suggestion: string(v, "suggestion"),
                        file: string(v, "file"),
                        line: number(v, "line"),
                    });
                }
            }
        }
    } else {
        let re=Regex::new(r"(?i)^(?:(.+?) \(line (\d+), column (\d+)\) )?(warning|error|fatal|internalerror) (KQ\d{4}): (.*)$").unwrap();
        for line in text.replace('\r', "").lines() {
            if let Some(m) = re.captures(line) {
                records.push(Diag {
                    severity: m[4].to_lowercase(),
                    code: m[5].into(),
                    message: m[6].into(),
                    file: m.get(1).map_or(String::new(), |s| s.as_str().into()),
                    line: m.get(2).and_then(|s| s.as_str().parse().ok()).unwrap_or(0),
                    ..Default::default()
                });
            }
        }
    }
    let found = records.len();
    for r in &mut records {
        if r.suggestion.trim().is_empty() {
            r.suggestion = r
                .code
                .to_uppercase()
                .strip_prefix("KQ")
                .and_then(|s| s.parse().ok())
                .map_or("", |n| diagnostics::suggestion(n, &r.message))
                .into();
        }
        if r.suggestion.trim().is_empty() {
            r.suggestion = fallback(&r.message).into();
        }
    }
    records.retain(|r| !r.message.trim().is_empty());
    let rank = |s: &str| match s.to_lowercase().as_str() {
        "fatal" => 0,
        "error" | "internalerror" => 1,
        "warning" => 2,
        _ => 3,
    };
    records.sort_by_key(|r| (rank(&r.severity), r.code.clone()));
    records.truncate(max);
    let mut text = format!(
        "# KITAQGB shortest fix suggestions\ninput={}\ndiagnostics_found={found}\nsuggestions_emitted={}\n\n",
        workflow::full(&input).display(),
        records.len()
    );
    for (i, r) in records.iter().enumerate() {
        let where_ = if !r.file.trim().is_empty() && r.line > 0 {
            format!(" @ {}:{}", r.file, r.line)
        } else {
            String::new()
        };
        writeln!(
            text,
            "{}. [{}{}] {}{}\n   shortest_fix: {}\n",
            i + 1,
            if r.severity.trim().is_empty() {
                "diag"
            } else {
                &r.severity
            },
            if r.code.trim().is_empty() {
                String::new()
            } else {
                format!(" {}", r.code)
            },
            r.message,
            where_,
            r.suggestion
        )
        .unwrap();
    }
    if records.is_empty() {
        text.push_str(
            "No diagnostics were parsed.\nRun compiler with --diag-json and re-run fixhint.\n",
        );
    }
    publish("fixhint", &output, &text)
}
fn irsum(args: &[String]) -> Result<(), String> {
    let (mut ir, mut asm, mut output, mut top) =
        (String::new(), String::new(), String::from("-"), 12usize);
    for a in &args[1..] {
        if matches!(a.as_str(), "--stdout" | "-") {
            output = "-".into()
        } else if let Some(p) = a.strip_prefix("--ir=") {
            ir = p.into()
        } else if let Some(p) = a.strip_prefix("--asm=") {
            asm = p.into()
        } else if let Some(p) = a.strip_prefix("--out=") {
            output = p.into()
        } else if let Some(p) = a.strip_prefix("--top=") {
            top = p
                .parse()
                .ok()
                .filter(|n| (1..=200).contains(n))
                .ok_or("--top must be 1..200")?;
        } else {
            return Err(format!("unknown irsum option: {a}"));
        }
    }
    if ir.trim().is_empty() {
        ir = debug_tools::newest("syntax_tree_lowered.txt");
    }
    if asm.trim().is_empty() {
        asm = debug_tools::newest("assembly_code.txt");
    }
    if !Path::new(&ir).is_file() && !Path::new(&asm).is_file() {
        return Err("no IR/ASM input found for irsum".into());
    }
    let irtext = io::read_utf8(&ir).unwrap_or_default();
    let asmtext = io::read_utf8(&asm).unwrap_or_default();
    let lines = |s: &str| {
        if s.is_empty() {
            0
        } else {
            s.bytes().filter(|b| *b == b'\n').count() + 1
        }
    };
    let mut text = format!(
        "# KITAQGB AI IR short summary\ngenerated_utc={}\nir_file={}\nasm_file={}\n\ncounts.ir_lines={}\ncounts.asm_lines={}\n",
        workflow::utc_now(),
        if ir.trim().is_empty() {
            "<none>".into()
        } else {
            workflow::full(&ir).display().to_string()
        },
        if asm.trim().is_empty() {
            "<none>".into()
        } else {
            workflow::full(&asm).display().to_string()
        },
        lines(&irtext),
        lines(&asmtext)
    );
    for (label, tag) in [
        ("functions", "function"),
        ("function_decls", "function_decl"),
        ("banks", "bank"),
        ("calls", "call"),
        ("variables", "variable"),
    ] {
        let re = Regex::new(&format!(r"\(\${tag}\b")).unwrap();
        writeln!(text, "counts.ir_{label}={}", re.find_iter(&irtext).count()).unwrap();
    }
    let mut functions = BTreeMap::new();
    let mut current = String::new();
    let mut total = 0;
    for raw in asmtext.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let line = raw.trim();
        if line.to_lowercase().starts_with("; function ") {
            current = line[11..].trim().trim_end_matches(':').trim().into();
            functions.entry(current.clone()).or_insert(0usize);
            continue;
        }
        if current.trim().is_empty()
            || line.is_empty()
            || line.starts_with(';')
            || line.ends_with(':')
        {
            continue;
        }
        *functions.get_mut(&current).unwrap() += 1;
        total += 1;
    }
    writeln!(
        text,
        "counts.asm_functions={}\ncounts.asm_instructions={total}\n\ntop_functions_by_asm_insn:",
        functions.len()
    )
    .unwrap();
    let mut rows = functions.into_iter().collect::<Vec<_>>();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (name, count) in rows.into_iter().take(top) {
        writeln!(text, "- {name} = {count}").unwrap();
    }
    publish("irsum", &output, &text)
}
