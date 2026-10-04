use crate::{
    assembler, assembly_json, codegen, dead_stripper, disassembler, io, ir_json, json::Value,
    lowerer, optimizer, parser, rom_header, tokenizer,
};
use std::{
    fs,
    io::BufWriter,
    path::{Path, PathBuf},
};
fn run(mut args: Vec<String>, session: &mut crate::observation::Session) -> Result<(), String> {
    if crate::minimizer::requested(&args) {
        return crate::minimizer::run(&args, session);
    }
    if let Some(result) = crate::debug_tools::try_run(&args) {
        return result;
    }
    if let Some(result) = crate::vibe_tools::try_run(&args) {
        return result;
    }
    if let Some(result) = crate::drivers::try_run(&args, session) {
        return result;
    }
    if args.is_empty()
        || args
            .iter()
            .any(|a| matches!(a.as_str(), "--help" | "-h" | "-?"))
    {
        println!(
            "KITAQGB native Rust migration (in progress)\n\nCommands:\n  disasm ROM -o dis.s\n  header ROM -o patched.gb [--title=NAME --cgb=dmg|cgb|cgb_only ...]\n  assemble-ir INPUT.assembly.json -o output.gb [header options]\n  tokenize SOURCE.c ... -I INCLUDE_DIR -o tokens.json\n  parse SOURCE.c ... -I INCLUDE_DIR -o ast.json\n  lower SOURCE.c ... -I INCLUDE_DIR -o lowered.json\n  emit-ir SOURCE.c ... -I INCLUDE_DIR -o raw.assembly.json\n  compile SOURCE.c ... -I INCLUDE_DIR -o output.gb [-O0|-O1]\n  optimize-ir INPUT.assembly.json -o optimized.json [-O0|-O1]\n  strip-ir INPUT.assembly.json -o stripped.json [--strip-keep=LABELS]\n\nC compilation, aggregates, banked calls and hardware intrinsics are implemented and covered by differential tests. Utility commands: kqhelp, symfind, src2asm, romdiff, template, snippet, conventions, fixhint, irsum, attrviz, recipe, test.\n\nDevelopment and recovery:\n  SOURCE.c ... --watch [--watch-max-builds=N]\n  devserver SOURCE.c ... [--once --poll-ms=N --debounce-ms=N --max-builds=N]\n  recipe [--history=FILE --out=FILE --script=FILE --last=N --stdout]\n  SOURCE.c ... --minimize[=FILE] [--minimize-out=FILE --minimize-work=DIR --minimize-quick --minimize-trace=final|all]\n  SOURCE.c ... --minimize-on-fail --repro-pack[=DIR]\n  SOURCE.c ... --debug-output --disasm-changed[=GIT_BASE]\nWatch loops stop with Ctrl+C; the Rust API also accepts a CancellationToken. Migration validation is still in progress. See PORT_STATUS.md."
        );
        return Ok(());
    }
    if args[0] == "--version" {
        println!(
            "kitaqgb {} (migration in progress)",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if !matches!(
        args[0].as_str(),
        "disasm"
            | "header"
            | "assemble-ir"
            | "tokenize"
            | "parse"
            | "lower"
            | "emit-ir"
            | "compile"
            | "optimize-ir"
            | "strip-ir"
    ) {
        args.insert(0, "compile".into());
    }
    let mut inputs = Vec::new();
    let mut includes = Vec::new();
    let mut output = None;
    let mut opt_level = 0;
    let mut fixed_stack = false;
    let mut abi_stack = false;
    let mut check_bounds = false;
    let mut full_checks = false;
    let mut rst = codegen::rst::Options::default();
    let mut const_scalar_in_rom = false;
    let mut ai_metadata: Option<String> = None;
    let mut enable_cache = true;
    let mut vlist: Option<String> = None;
    let mut deps: Option<String> = None;
    let mut requested_reports = std::collections::BTreeMap::new();
    let mut stack_top = None;
    let mut stack_reserve = 512;
    let mut strip_keep = std::collections::BTreeSet::new();
    let mut opt = rom_header::Options::default();
    let mut i = 1;
    while i < args.len() {
        if matches!(
            args[i].as_str(),
            "--stack-bank" | "--stack-top" | "--stack-reserve"
        ) {
            let Some(value) = args.get(i + 1).cloned() else {
                let message = match args[i].as_str() {
                    "--stack-top" => "--stack-top: expected an address literal",
                    "--stack-reserve" => "--stack-reserve: expected a non-negative integer",
                    _ => "error: --stack-bank requires fixed or wramx1",
                };
                session.global_error(message);
                i += 1;
                continue;
            };
            args[i] = format!("{}={value}", args[i]);
            args.remove(i + 1);
        }
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output = Some(args.get(i).ok_or("missing output path")?.clone());
            }
            "-I" => {
                i += 1;
                includes.push(PathBuf::from(
                    args.get(i).ok_or("missing include directory")?,
                ));
            }
            s if s.starts_with("-I") && s.len() > 2 => includes.push(PathBuf::from(&s[2..])),
            s if s.starts_with("--include-dir=") => includes.push(PathBuf::from(&s[14..])),
            "--no-cache" => enable_cache = false,
            "--cache" => enable_cache = true,
            "--no-banner" => {}
            "--repro-pack" | "--minimize-on-fail" | "--auto-minimize" => {}
            s if s.starts_with("--repro-pack=") => {}
            s if s.starts_with("--minimize-out=")
                || s.starts_with("--minimize-work=")
                || s.starts_with("--minimize-trace=")
                || s == "--minimize-quick" => {}
            "--no-disasm" => session.disable_disasm = true,
            "--disasm-changed" => session.changed_disasm = Some(String::new()),
            s if s.starts_with("--disasm-changed=") => {
                session.changed_disasm = Some(s[17..].into())
            }
            s if s.starts_with("--disasm-changed-out=") => {
                session.changed_disasm.get_or_insert_with(String::new);
                session.changed_disasm_path = Some(s[21..].into());
            }
            "--disasm" => session.disable_disasm = false,
            "--fast-build" | "--fast" => {
                session.disable_disasm = true;
                session.trace = false;
            }
            "--strict" => session.strict = true,
            "--permissive" => session.strict = false,
            "--machine-readable" => session.machine = true,
            "--diag-json" => session.diag_path = Some("kitaqgb.diag.json".into()),
            s if s.starts_with("--diag-json=") => {
                session.diag_path = Some(if s[12..].is_empty() {
                    "kitaqgb.diag.json".into()
                } else {
                    s[12..].into()
                })
            }
            "--emit-path-manifest" => {
                i += 1;
                session.manifest_path = Some(
                    args.get(i)
                        .ok_or("--emit-path-manifest requires a file path")?
                        .clone(),
                );
            }
            s if s.starts_with("--emit-path-manifest=") => {
                if s[21..].is_empty() {
                    return Err("--emit-path-manifest requires a file path".into());
                }
                session.manifest_path = Some(s[21..].into());
            }
            s if s.starts_with("--max-errors=") => {
                session.max_errors = s[13..]
                    .parse()
                    .map_err(|_| "--max-errors requires a positive integer")?;
                if session.max_errors == 0 {
                    return Err("--max-errors requires a positive integer".into());
                }
            }
            "--debug-output" | "--debug-out" => session.debug = true,
            "--no-debug-output" => session.debug = false,
            "--no-trace" => session.trace = false,
            s if s.starts_with("--debug-output=") || s.starts_with("--debug-out=") => {
                session.debug = true;
                let path = s.split_once('=').unwrap().1;
                if !path.is_empty() {
                    session.debug_dir = path.into();
                }
            }
            "--trace" => {
                session.trace = true;
                session.stages = ["tokens", "ast", "ir", "asm"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
            }
            s if s.starts_with("--trace=") => {
                session.trace = true;
                session.stages = s[8..]
                    .split([',', ';'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_lowercase())
                    .collect();
                if session.stages.is_empty() {
                    session.stages = ["tokens", "ast", "ir", "asm"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect();
                }
            }
            s if s.starts_with("--trace-out=") => {
                session.trace = true;
                if !s[12..].is_empty() {
                    session.trace_dir = s[12..].into();
                }
            }
            s if s.starts_with("--profile=") => match s[10..].trim().to_lowercase().as_str() {
                "dev" => {
                    enable_cache = true;
                    session.debug = true;
                    session.disable_disasm = false;
                    opt_level = 0;
                    rst.enabled = false;
                }
                "release" => {
                    enable_cache = true;
                    session.debug = false;
                    session.disable_disasm = true;
                    opt_level = 1;
                    rst.enabled = true;
                }
                "test" => {
                    enable_cache = false;
                    session.debug = true;
                    session.disable_disasm = true;
                    opt_level = 0;
                    rst.enabled = false;
                }
                _ => return Err("--profile must be dev|release|test".into()),
            },
            "--rst-disable" | "--no-rst" | "--rst-off" => rst.enabled = false,
            "--rst-enable" | "--rst" | "--rst-on" => rst.enabled = true,
            "--rst-safe" | "--rst-unsafe" => {
                rst.enabled = true;
                rst.unsafe_mode = args[i] == "--rst-unsafe";
            }
            "--rst-use-38" => {
                rst.enabled = true;
                rst.use_38 = true;
            }
            "--rst-speed-safe" => {
                rst.enabled = true;
                rst.max_calls = rst.max_calls.min(12);
            }
            s if s.starts_with("--rst-max-calls=") => {
                rst.enabled = true;
                rst.max_calls = s[16..]
                    .parse()
                    .map_err(|_| "--rst-max-calls requires a positive integer")?;
                if rst.max_calls <= 0 {
                    return Err("--rst-max-calls requires a positive integer".into());
                }
            }
            s if s.starts_with("--rst-max-vectors=") => {
                rst.enabled = true;
                let n: i32 = s[18..]
                    .parse()
                    .map_err(|_| "--rst-max-vectors requires a non-negative integer")?;
                if n < 0 {
                    return Err("--rst-max-vectors requires a non-negative integer".into());
                }
                rst.max_vectors = n as usize;
            }
            s if s.starts_with("--rst-exclude=") => {
                rst.enabled = true;
                rst.exclude.extend(
                    s[14..]
                        .split([',', ';'])
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            }
            "vlist" | "--vlist" => vlist = Some(String::new()),
            s if s.starts_with("--vlist=") || s.starts_with("--vlist-out=") => {
                vlist = Some(s.split_once('=').unwrap().1.into())
            }
            "--deps-out" => deps = Some(String::new()),
            "--emit-ai-metadata" => {
                i += 1;
                ai_metadata = Some(
                    args.get(i)
                        .ok_or("--emit-ai-metadata requires a file path")?
                        .clone(),
                );
            }
            s if s.starts_with("--emit-ai-metadata=") => {
                if s[19..].is_empty() {
                    return Err("--emit-ai-metadata requires a file path".into());
                }
                ai_metadata = Some(s[19..].into());
            }
            s if report_suffix(s.split_once('=').map_or(s, |x| x.0)).is_some() => {
                let (name, path) = s.split_once('=').unwrap_or((s, ""));
                requested_reports.insert(name.to_owned(), path.to_owned());
            }
            s if s.starts_with("--deps-out=") => deps = Some(s[11..].into()),
            s if s.starts_with("--rom-header=") => {
                let header = rom_header::Options::from_json(&io::read_utf8(&s[13..])?)?;
                opt.merge_from(&header, true);
            }
            "--header-logo" => opt.logo = Some(true),
            "--no-header-logo" => opt.logo = Some(false),
            "-O0" => opt_level = 0,
            "-O1" => opt_level = 1,
            "--abi=stack" => abi_stack = true,
            "--abi=legacy" => abi_stack = false,
            "-Zcheck-bounds" | "-Zcheck_bounds" => check_bounds = true,
            "-Zcheck" => {
                check_bounds = true;
                full_checks = true;
            }
            "-Zconst-scalar-in-rom" | "-Zconst_scalar_in_rom" => const_scalar_in_rom = true,
            s if s.starts_with("--stack-bank=") => {
                fixed_stack = match &s[13..] {
                    "fixed" | "wram0" => true,
                    "wramx1" | "wram1" => false,
                    _ => return Err("--stack-bank must be fixed|wramx1".into()),
                };
            }
            s if s.starts_with("--stack-top=") => {
                let raw = &s[12..];
                let value = parse_number(raw).ok().filter(|n| (0..=65535).contains(n));
                if let Some(value) = value {
                    stack_top = Some(value);
                } else {
                    session.global_error(&format!(
                        "--stack-top: expected a 16-bit address literal (got {raw})"
                    ));
                }
            }
            s if s.starts_with("--stack-reserve=") => {
                let raw = &s[16..];
                let value = parse_number(raw).ok().filter(|n| *n >= 0);
                if let Some(value) = value {
                    stack_reserve = value;
                } else {
                    session.global_error(&format!(
                        "--stack-reserve: expected a non-negative integer (got {raw})"
                    ));
                }
            }
            s if s.starts_with("--strip-keep=") => {
                strip_keep.extend(
                    s[13..]
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            }
            s if s.starts_with("--") => {
                let (k, v) = s[2..]
                    .split_once('=')
                    .ok_or_else(|| format!("unknown option: {s}"))?;
                opt.set(k, v)?;
            }
            s if s.starts_with('-') => return Err(format!("unknown option: {s}")),
            s => inputs.push(PathBuf::from(s)),
        }
        i += 1;
    }
    let mut output = match output {
        Some(path) => path,
        None if args[0] == "compile" => "out.gb".into(),
        None => return Err("provide -o OUTPUT to write an artifact".into()),
    };
    session.output = output.clone();
    let mut stack_top = stack_top.unwrap_or(if fixed_stack { 0xcfff } else { 0xdfff });
    let window_bottom = if fixed_stack { 0xc000 } else { 0xd000 };
    if !(window_bottom..=window_bottom + 0xfff).contains(&stack_top) {
        session.global_error(&format!("--stack-top: address must be within {} range {:04X}-{:04X} for --stack-bank={} (got {:04X})",if fixed_stack {"WRAM0"}else{"WRAMX bank1"},window_bottom,window_bottom+0xfff,if fixed_stack {"fixed"}else{"wramx1"},stack_top&65535));
    }
    stack_top = stack_top.clamp(window_bottom, window_bottom + 0xfff);
    if stack_reserve < 0 || stack_reserve > stack_top - window_bottom {
        session.global_error(&format!("--stack-reserve: value {stack_reserve} exceeds usable depth for stack top {:04X} in --stack-bank={}",stack_top&65535,if fixed_stack {"fixed"}else{"wramx1"}));
    }
    if session.errors() > 0 {
        return Err("invalid stack policy".into());
    }
    if matches!(args[0].as_str(), "parse" | "lower" | "emit-ir" | "compile") {
        if inputs.is_empty() {
            return Err("missing source file".into());
        }
        let cached = if args[0] == "compile"
            && enable_cache
            && !session.trace
            && !session.debug
            && session.diag_path.is_none()
            && ai_metadata.is_none()
            && vlist.is_none()
            && deps.is_none()
            && requested_reports.is_empty()
        {
            crate::cache::Entry::for_invocation(&args, &inputs, &includes, &opt)
        } else {
            None
        };
        if let Some(entry) = &cached {
            match entry.restore(Path::new(&output)) {
                Ok(true) => {
                    if !session.machine && !session.hosted {
                        eprintln!("[cache] hit: {}", entry.key());
                    }
                    return Ok(());
                }
                Ok(false) => {}
                Err(error) => session.warning(&format!("warning: cache restore skipped: {error}")),
            }
        }
        if session.traced("tokens") {
            let start = std::time::Instant::now();
            let mut text = String::new();
            for file in &inputs {
                let tokenized = tokenizer::tokenize_files(std::slice::from_ref(file), &includes);
                text.push_str(&format!(
                    "==== TOKENS: {} ====\n{}\n\n",
                    file.display(),
                    crate::observation::tokens(&tokenized.tokens)
                ));
            }
            session.trace_file("tokens.txt", &text)?;
            session.stat("tokens", start, format!("{} file(s)", inputs.len()));
        }
        let start = std::time::Instant::now();
        let mut result = parser::parse_files(&inputs, &includes, opt.cgb == Some(0xc0));
        session.dependencies = result.preprocessing.dependencies.clone();
        if !result.preprocessing.palettes.is_empty() {
            session.debug_file(
                "cgb_palettes.txt",
                &crate::palettes::text(&result.preprocessing.palettes),
            )?;
        }
        session.capture(&mut result.diagnostics);
        if !result
            .diagnostics
            .iter()
            .any(|d| d.severity == tokenizer::Severity::Error)
        {
            session.debug_file("syntax_tree.txt", &result.tree.show_multiline())?;
            if session.traced("ast") {
                session.trace_file(
                    "ast_simple.txt",
                    &crate::observation::ast_summary(&result.tree),
                )?;
                session.trace_file("ast_full.txt", &result.tree.show_multiline())?;
            }
        }
        session.stat("parse", start, "ast built");
        opt.merge_from(&result.preprocessing.header, false);
        let known_cgb = match opt.cgb {
            Some(0) => Some(0),
            Some(0xc0) => Some(1),
            _ => None,
        };
        if args[0] != "parse"
            && !result
                .diagnostics
                .iter()
                .any(|d| d.severity == tokenizer::Severity::Error)
        {
            let start = std::time::Instant::now();
            let lowered = lowerer::lower(
                &result.tree,
                &lowerer::Options {
                    known_cgb,
                    const_scalar_in_rom,
                },
            );
            result.tree = lowered.tree;
            result.diagnostics.extend(lowered.diagnostics);
            session.capture(&mut result.diagnostics);
            if !result
                .diagnostics
                .iter()
                .any(|d| d.severity == tokenizer::Severity::Error)
            {
                session.debug_file("syntax_tree_lowered.txt", &result.tree.show_multiline())?;
                if session.traced("ir") {
                    session.trace_file("ir_lowered.txt", &result.tree.show_multiline())?;
                }
            }
            session.stat("lower", start, "ir lowered");
        }
        let mut final_assembly = None;
        let mut generated_report = None;
        let generated = if matches!(args[0].as_str(), "emit-ir" | "compile")
            && !result
                .diagnostics
                .iter()
                .any(|d| d.severity == tokenizer::Severity::Error)
        {
            let start = std::time::Instant::now();
            let generation_options = codegen::Options {
                fixed_stack,
                stack_top,
                stack_reserve,
                known_cgb,
                cgb_only: opt.cgb == Some(0xc0),
                opt_level,
                abi_stack,
                check_bounds,
                check_slice_bounds: full_checks,
                check_stack: full_checks,
                check_mem_copy: full_checks,
                check_bank_calls: full_checks,
                const_scalar_in_rom,
                rst: rst.clone(),
                ..codegen::Options::default()
            };
            let generated = if args[0] == "compile" {
                let build = crate::pipeline::compile_lowered(
                    &result.tree,
                    generation_options,
                    &assembler::Options {
                        stack_top,
                        header_logo: opt.logo.unwrap_or(true),
                    },
                )?;
                if build.relayout_passes > 0 && !session.hosted && !session.machine {
                    eprintln!(
                        "[bank-layout] stabilized after {} recompile pass(es)",
                        build.relayout_passes
                    );
                }
                final_assembly = build.assembled;
                let mut detail = format!(
                    "{} asm node(s)",
                    optimizer::optimize(&build.generated.lines, opt_level)
                        .lines
                        .len()
                );
                if build.relayout_passes > 0 {
                    detail.push_str(&format!(", relayout_passes={}", build.relayout_passes));
                }
                session.stat_duration("codegen", build.codegen_time, detail);
                session.stat_duration(
                    "assemble",
                    build.assemble_time,
                    Path::new(&output)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                );
                build.generated
            } else {
                codegen::emission::compile(&result.tree, generation_options)
            };
            generated_report = Some(generated.analysis);
            result.diagnostics.extend(generated.diagnostics);
            session.capture(&mut result.diagnostics);
            if !result
                .diagnostics
                .iter()
                .any(|d| d.severity == tokenizer::Severity::Error)
            {
                let text = crate::observation::assembly(
                    &optimizer::optimize(&generated.lines, opt_level).lines,
                );
                session.debug_file("assembly_code.txt", &text)?;
                if session.traced("asm") {
                    session.trace_file("asm.txt", &text)?;
                }
            }
            if args[0] != "compile" {
                session.stat("codegen", start, "native code generation");
            }
            Some(generated.lines)
        } else {
            None
        };
        if matches!(args[0].as_str(), "emit-ir" | "compile") {
            if result
                .diagnostics
                .iter()
                .any(|d| d.severity == tokenizer::Severity::Error)
            {
                return Err("native compilation failed; output was not written".into());
            }
            let lines = generated.unwrap();
            if args[0] == "emit-ir" {
                let value = Value::Object(
                    [
                        ("stack_top".into(), Value::Number(i64::from(stack_top))),
                        ("header_logo".into(), Value::Bool(opt.logo.unwrap_or(true))),
                        (
                            "assembly".into(),
                            Value::Array(lines.iter().map(ir_json::expr).collect()),
                        ),
                    ]
                    .into(),
                );
                io::write_utf8(&output, &value.stringify())?;
            } else {
                let assembled = final_assembly.unwrap();
                let start = std::time::Instant::now();
                let (rom, warnings) = rom_header::patch(&assembled.rom, &opt)?;
                for warning in assembled.warnings.iter().chain(warnings.iter()) {
                    session.warning(warning);
                }
                let actual = io::write_bytes_robust(&output, &rom, true)?;
                if actual != Path::new(&output) {
                    session.warning(&format!(
                        "output is locked, wrote alternate file: {}",
                        actual.display()
                    ));
                    output = actual.display().to_string();
                    session.output = output.clone();
                }
                session.stat("header", start, "patched");
                let reports_start = std::time::Instant::now();
                let vlist_path = vlist.as_ref().map(|s| {
                    if s.is_empty() {
                        Path::new(&output).with_extension("vlist.txt")
                    } else {
                        PathBuf::from(s)
                    }
                });
                if let Err(error) = crate::reports::write_companions_with(
                    &assembled,
                    Path::new(&output),
                    vlist_path.as_deref(),
                    |key, path, text| session.write_artifact(key, path, text),
                ) {
                    session.warning(&format!(
                        "warning: failed to write companion reports: {error}"
                    ));
                }
                if let Some(path) = deps.as_ref() {
                    let path = if path.is_empty() {
                        Path::new(&output).with_extension("deps.txt")
                    } else {
                        PathBuf::from(path)
                    };
                    let text = result
                        .preprocessing
                        .dependencies
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    session.write_artifact("deps_list", path, &format!("{text}\n"))?;
                }
                session.write_artifact(
                    "source_map",
                    Path::new(&output).with_extension("source_map.txt"),
                    &crate::reports::augmented_source_map(&assembled, &inputs),
                )?;
                let report = generated_report.as_ref().unwrap();
                let optimized = optimizer::optimize(&lines, opt_level);
                let policy = crate::metadata::Policy {
                    fixed: fixed_stack,
                    top: stack_top,
                    reserve: stack_reserve,
                    abi_stack,
                    opt_level,
                };
                session.debug_file(
                    "assembly_sectioned.txt",
                    &crate::observation::assembly(&assembled.assembly),
                )?;
                session.debug_file("stack_policy.txt", &crate::observation::stack_text(&policy))?;
                session.write_artifact(
                    "dbg2_json",
                    Path::new(&output).with_extension("dbg2.json"),
                    &crate::metadata::rich(
                        Path::new(&output),
                        &inputs,
                        &assembled,
                        report,
                        &policy,
                    )
                    .stringify(),
                )?;
                session.write_artifact(
                    "build_report_json",
                    Path::new(&output).with_extension("build_report.json"),
                    &crate::metadata::build(
                        Path::new(&output),
                        &inputs,
                        &assembled,
                        report,
                        &optimized.report,
                        &opt,
                        &policy,
                    )
                    .stringify(),
                )?;
                if let Some(path) = &ai_metadata {
                    session.write_artifact(
                        "ai_metadata",
                        path,
                        &crate::metadata::ai(
                            Path::new(&output),
                            &inputs,
                            &assembled,
                            report,
                            &policy,
                        )
                        .stringify(),
                    )?;
                }
                let reports_time = reports_start.elapsed();
                if session.debug && !session.disable_disasm {
                    let start = std::time::Instant::now();
                    let path = session.debug_dir.join("dis.s");
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    let mut writer =
                        BufWriter::new(fs::File::create(&path).map_err(|e| e.to_string())?);
                    disassembler::disassemble(
                        &rom,
                        Path::new(&output)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .as_ref(),
                        &mut writer,
                    )
                    .map_err(|e| e.to_string())?;
                    std::io::Write::flush(&mut writer).map_err(|e| e.to_string())?;
                    session.stat("disasm", start, "debug_output/dis.s");
                } else if session.debug && session.disable_disasm {
                    session.stat_duration(
                        "disasm",
                        std::time::Duration::ZERO,
                        "skipped (--no-disasm)",
                    );
                }
                if session.debug {
                    if let Some(base) = session.changed_disasm.clone() {
                        if session.disable_disasm {
                            session.warning("warning: --disasm-changed was requested but disassembly is disabled (--no-disasm)");
                        } else {
                            let start = std::time::Instant::now();
                            match crate::debug_tools::write_changed(
                                &inputs,
                                &assembled,
                                &base,
                                session.changed_disasm_path.as_deref(),
                                &session.debug_dir,
                            ) {
                                Ok(path) => {
                                    let requested = session
                                        .changed_disasm_path
                                        .as_ref()
                                        .map(PathBuf::from)
                                        .unwrap_or_else(|| session.debug_dir.join("dis_changed.s"));
                                    if path != requested {
                                        session.warning(&format!(
                                            "output is locked, wrote alternate file: {}",
                                            path.display()
                                        ));
                                    }
                                    session.remember("changed_disasm", path);
                                }
                                Err(e) => session.warning(&format!(
                                    "failed to emit changed-function disasm: {e}"
                                )),
                            }
                            session.stat("disasm_changed", start, "filtered");
                        }
                    }
                }
                let reports_start = std::time::Instant::now();
                if !requested_reports.is_empty() {
                    let mut failures = Vec::new();
                    for (name, path) in &requested_reports {
                        let target = if path.trim().is_empty() {
                            Path::new(&output).with_extension(report_suffix(name).unwrap())
                        } else {
                            PathBuf::from(path)
                        };
                        if matches!(name.as_str(), "--abi-diff-report" | "--repro-check") {
                            let result = if name == "--abi-diff-report" {
                                crate::build_helpers::abi_diff(&args, &target)
                            } else {
                                crate::build_helpers::reproducibility(
                                    &args,
                                    Path::new(&output),
                                    &target,
                                    abi_stack,
                                )
                            };
                            if let Err(error) = result {
                                failures.push(error);
                            }
                            continue;
                        }
                        let text = match name.as_str() {
                            "--bank-sim" => {
                                crate::analysis_reports::bank_simulation(&optimized.lines)
                            }
                            "--farcall-suggest" => {
                                crate::analysis_reports::farcall_suggestions(report)
                            }
                            "--cross-bank-report" => crate::analysis_reports::cross_bank(report),
                            "--abi-verify" => {
                                if !report.abi_issues.is_empty() {
                                    failures.push(format!(
                                        "ABI verification failed ({} issue(s)). See: {}",
                                        report.abi_issues.len(),
                                        target.display()
                                    ));
                                }
                                crate::analysis_reports::abi_verify(report, abi_stack)
                            }
                            "--rst-report" => {
                                crate::analysis_reports::rst_apply(report, &optimized.report, &rst)
                            }
                            "--opt-diff" => {
                                crate::analysis_reports::optimizer_diff(&optimized.report)
                            }
                            "--func-size-report" => crate::reports::function_sizes(&assembled),
                            "--hotspot-report" => {
                                crate::analysis_reports::hotspots(report, &assembled)
                            }
                            "--cgb-consistency" => {
                                let (text, issues) = crate::analysis_reports::cgb_consistency(
                                    &optimized.lines,
                                    report,
                                    rom[0x143],
                                );
                                if !issues.is_empty() {
                                    failures.push(format!(
                                        "CGB consistency failed ({} issue(s)). See: {}",
                                        issues.len(),
                                        target.display()
                                    ));
                                }
                                text
                            }
                            "--verify-cgb-symbols" => {
                                let (text, missing) = crate::analysis_reports::cgb_symbols(
                                    &optimized.lines,
                                    &assembled,
                                );
                                if missing > 0 {
                                    failures.push(format!("CGB symbol verification failed ({missing} missing). See: {}",target.display()));
                                }
                                text
                            }
                            _ => unreachable!(),
                        };
                        session.write_artifact(
                            &format!("report:{}", name.trim_start_matches('-')),
                            &target,
                            &text,
                        )?;
                    }
                    if !failures.is_empty() {
                        return Err(failures.join("\n"));
                    }
                }
                session.stat_duration(
                    "reports",
                    reports_time + reports_start.elapsed(),
                    "analysis",
                );
            }
            if args[0] == "compile" {
                if let Some(entry) = &cached {
                    let start = std::time::Instant::now();
                    if let Err(error) = entry.save(Path::new(&output)) {
                        session.warning(&format!("warning: cache save skipped: {error}"));
                    }
                    session.stat("cache_save", start, entry.key());
                }
            }
            session.summary(inputs.len())?;
            return Ok(());
        }
        io::write_utf8(&output, &ir_json::expr(&result.tree).stringify())?;
        return if result
            .diagnostics
            .iter()
            .any(|d| d.severity == tokenizer::Severity::Error)
        {
            Err("source parsing failed".into())
        } else {
            Ok(())
        };
    }
    if args[0] == "tokenize" {
        if inputs.is_empty() {
            return Err("missing source file".into());
        }
        let mut result = tokenizer::tokenize_files(&inputs, &includes);
        session.capture(&mut result.diagnostics);
        let tokens = result
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
                        ("file".into(), Value::String(t.position.filename.clone())),
                        ("line".into(), Value::Number(t.position.line as i64)),
                        ("column".into(), Value::Number(t.position.column as i64)),
                    ]
                    .into(),
                )
            })
            .collect();
        io::write_utf8(
            &output,
            Value::Object(
                [
                    ("tokens".into(), Value::Array(tokens)),
                    (
                        "dependencies".into(),
                        Value::Array(
                            result
                                .dependencies
                                .iter()
                                .map(|p| Value::String(p.display().to_string()))
                                .collect(),
                        ),
                    ),
                ]
                .into(),
            )
            .stringify()
            .as_str(),
        )?;
        return if result.has_errors() {
            Err("source preprocessing failed".into())
        } else {
            Ok(())
        };
    }
    if inputs.len() != 1 {
        return Err("provide exactly one input file".into());
    }
    let input = &inputs[0];
    match args[0].as_str() {
        "optimize-ir" | "strip-ir" => {
            let (assembly, options) = assembly_json::parse(&io::read_utf8(input)?)?;
            let assembly = if args[0] == "optimize-ir" {
                optimizer::optimize(&assembly, opt_level).lines
            } else {
                dead_stripper::strip(&assembly, &strip_keep).lines
            };
            let value = Value::Object(
                [
                    (
                        "stack_top".into(),
                        Value::Number(i64::from(options.stack_top)),
                    ),
                    ("header_logo".into(), Value::Bool(options.header_logo)),
                    (
                        "assembly".into(),
                        Value::Array(assembly.iter().map(ir_json::expr).collect()),
                    ),
                ]
                .into(),
            );
            io::write_utf8(&output, &value.stringify())?;
        }
        "assemble-ir" => {
            let (assembly, options) = assembly_json::parse(&io::read_utf8(input)?)?;
            let assembled = assembler::assemble(&assembly, &options)?;
            for w in &assembled.warnings {
                eprintln!("{w}");
            }
            let (out, warnings) = rom_header::patch(&assembled.rom, &opt)?;
            for w in warnings {
                eprintln!("{w}");
            }
            io::write_bytes(&output, &out)?;
            crate::reports::write_companions(&assembled, Path::new(&output), None)?;
        }
        "disasm" => {
            let rom = fs::read(input).map_err(|e| e.to_string())?;
            let file = fs::File::create(&output).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            disassembler::disassemble(
                &rom,
                Path::new(input)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref(),
                &mut writer,
            )
            .map_err(|e| e.to_string())?;
            std::io::Write::flush(&mut writer).map_err(|e| e.to_string())?;
        }
        "header" => {
            let rom = fs::read(input).map_err(|e| e.to_string())?;
            let (out, warnings) = rom_header::patch(&rom, &opt)?;
            for w in warnings {
                eprintln!("{w}");
            }
            io::write_bytes(&output, &out)?;
        }
        _ => {
            return Err("unknown command".into());
        }
    }
    Ok(())
}
fn report_suffix(name: &str) -> Option<&'static str> {
    Some(match name {
        "--bank-sim" => "bank_sim.txt",
        "--farcall-suggest" => "farcall_suggestions.txt",
        "--cross-bank-report" => "cross_bank_calls.txt",
        "--abi-verify" => "abi_verify.txt",
        "--abi-diff-report" => "abi_diff.txt",
        "--repro-check" => "repro_check.txt",
        "--rst-report" => "rst_report.txt",
        "--opt-diff" => "opt_diff.txt",
        "--func-size-report" => "funcsizes.txt",
        "--hotspot-report" => "hotspots.txt",
        "--cgb-consistency" => "cgb_consistency.txt",
        "--verify-cgb-symbols" => "cgb_symbols.txt",
        _ => return None,
    })
}
fn parse_number(text: &str) -> Result<i32, String> {
    let parsed = if let Some(hex) = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .or_else(|| text.strip_prefix('$'))
    {
        i32::from_str_radix(hex, 16)
    } else {
        text.parse()
    };
    parsed.map_err(|_| format!("invalid integer option: {text}"))
}
/// Run one invocation without changing process arguments or terminating its host.
pub fn invoke(args: Vec<String>, session: &mut crate::observation::Session) -> i32 {
    if !session.hosted {
        crate::workflow::append_history(&args);
    }
    let result = run(args.clone(), session);
    if let Err(error) = &result {
        if session.errors() == 0 {
            session.global_error(error);
        }
    }
    let exit = session.exit_code.unwrap_or(i32::from(result.is_err()));
    if let Err(error) = session.finish(exit) {
        session.warning(&format!("failed to write invocation metadata: {error}"));
    }
    if exit != 0 {
        if let Err(error) = crate::repro::on_failure(&args, session, exit) {
            session.warning(&format!("failed to write repro package: {error}"));
        }
        if args
            .iter()
            .any(|a| matches!(a.as_str(), "--minimize-on-fail" | "--auto-minimize"))
            && !crate::minimizer::requested(&args)
        {
            let mut child_args = args
                .iter()
                .filter(|a| !matches!(a.as_str(), "--minimize-on-fail" | "--auto-minimize"))
                .cloned()
                .collect::<Vec<_>>();
            child_args.push("--minimize".into());
            let mut child = crate::observation::Session::default();
            child.hosted = session.hosted;
            if let Err(error) = crate::minimizer::run(&child_args, &mut child) {
                session.warning(&format!("automatic minimization failed: {error}"));
            }
        }
    }
    exit
}
