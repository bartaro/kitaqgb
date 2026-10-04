//! Text reports compatible with the C# command-line compiler.
use crate::{
    asm::AddressMode as M,
    assembler,
    codegen::{analysis::Report, rst},
    expr::Expr,
    optimizer, tags,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};
fn ordinal(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
fn cross_edges(report: &Report, exclude_far: bool) -> Vec<&crate::codegen::analysis::Call> {
    let mut rows = report
        .calls
        .iter()
        .filter(|c| {
            c.caller_bank >= 0
                && c.callee_bank >= 0
                && c.caller_bank != c.callee_bank
                && (!exclude_far || !c.via_farcall)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then(ordinal(&a.caller, &b.caller))
            .then(ordinal(&a.callee, &b.callee))
    });
    rows
}
pub fn cross_bank(report: &Report) -> String {
    let mut out="# KITAQGB cross-bank call report\n# caller, caller_bank, callee, callee_bank, count, kind, via_thunk, via_farcall\n".to_owned();
    for c in cross_edges(report, false) {
        writeln!(
            out,
            "{}, {}, {}, {}, {}, {}, {}, {}",
            c.caller,
            c.caller_bank,
            c.callee,
            c.callee_bank,
            c.count,
            c.kind,
            i32::from(c.via_thunk),
            i32::from(c.via_farcall)
        )
        .unwrap();
    }
    out
}
pub fn farcall_suggestions(report: &Report) -> String {
    let mut out = "# KITAQGB farcall suggestion report\n".to_owned();
    let rows = cross_edges(report, true);
    if rows.is_empty() {
        out.push_str("no farcall candidates\n");
        return out;
    }
    out.push_str(
        "# caller -> callee, count, caller_bank, callee_bank, current_kind, recommendation\n",
    );
    for c in rows {
        writeln!(
            out,
            "{} -> {}, {}, {}, {}, {}, {}",
            c.caller,
            c.callee,
            c.count,
            c.caller_bank,
            c.callee_bank,
            c.kind,
            if c.via_thunk {
                "consider __farcall(bank, func) for explicit cross-bank intent"
            } else {
                "check bank safety"
            }
        )
        .unwrap();
    }
    out
}
pub fn abi_verify(report: &Report, stack: bool) -> String {
    let mut out = format!(
        "# KITAQGB ABI verification report\nabi_mode={}\nfunctions={}\ncalls={}\nissues={}\n\n",
        if stack { "stack" } else { "legacy" },
        report.functions.len(),
        report.calls.len(),
        report.abi_issues.len()
    );
    if report.abi_issues.is_empty() {
        out.push_str("PASS\n");
    } else {
        out.push_str("FAIL\n");
        for issue in &report.abi_issues {
            writeln!(out, "- {issue}").unwrap();
        }
    }
    out
}
pub fn rst_apply(report: &Report, opt: &optimizer::Report, options: &rst::Options) -> String {
    let mut out = format!(
        "# KITAQGB RST apply report\nrst_enabled={}\nrst_use_38={}\n\n# selection\n",
        i32::from(options.enabled),
        i32::from(options.use_38)
    );
    for r in &report.rst_selections {
        writeln!(
            out,
            "RST_{:02X} -> {} (calls={}, net_bytes={})",
            r.vector, r.target, r.calls, r.net_bytes
        )
        .unwrap();
    }
    write!(
        out,
        "\n# optimizer rewrites\ntotal={}\n",
        opt.total_rst_rewrites
    )
    .unwrap();
    for (vector, count) in &opt.rst_rewrite_counts_by_vector {
        writeln!(out, "RST_{vector:02X}: {count}").unwrap();
    }
    out
}
pub fn optimizer_diff(report: &optimizer::Report) -> String {
    let mut out = "# KITAQGB optimizer pass diff report\n".to_owned();
    for p in &report.passes {
        write!(
            out,
            "\n## {}\nbefore={} after={} changed={} added={} removed={}\n{}\n",
            p.name,
            p.before_lines,
            p.after_lines,
            p.changed_lines,
            p.added_lines,
            p.removed_lines,
            p.diff_text
        )
        .unwrap();
    }
    out
}
pub fn hotspots(report: &Report, output: &assembler::Output) -> String {
    let mut rows = BTreeMap::<String, (i32, i32)>::new();
    for f in &output.report.functions {
        rows.entry(f.name.clone()).or_default().0 =
            rows.get(&f.name).map_or(f.size, |r| r.0.max(f.size));
    }
    for c in &report.calls {
        if !c.callee.is_empty() && !c.callee.starts_with('<') {
            rows.entry(c.callee.clone()).or_default().1 += c.count;
        }
    }
    let mut rows = rows.into_iter().collect::<Vec<_>>();
    rows.sort_by(|(a, (asize, acalls)), (b, (bsize, bcalls))| {
        (i64::from(*bsize) * i64::from(*bcalls))
            .cmp(&(i64::from(*asize) * i64::from(*acalls)))
            .then(bcalls.cmp(acalls))
            .then(bsize.cmp(asize))
            .then(ordinal(a, b))
    });
    let mut out="# KITAQGB hotspot estimate\n# score = incoming_call_count * function_size\n# name, calls, size, score\n".to_owned();
    for (name, (size, calls)) in rows {
        writeln!(
            out,
            "{name}, {calls}, {size}, {}",
            i64::from(size) * i64::from(calls)
        )
        .unwrap();
    }
    out
}
pub fn bank_simulation(lines: &[Expr]) -> String {
    let mut pc = assembler::CODE_START;
    let mut max_pc = pc;
    let mut banks = BTreeMap::<i32, i32>::new();
    banks.insert(0, pc);
    let mut current: Option<(String, i32, i32)> = None;
    let mut functions = Vec::new();
    for e in lines {
        if e.matches(tags::FUNCTION, 1) {
            if let Some(f) = current.take() {
                functions.push(f);
            }
            current = Some((e.text(1).unwrap().into(), pc, 0));
            continue;
        }
        let mut count = 0;
        if e.matches(tags::SKIP_TO, 1) {
            let skip = e.int(1).unwrap();
            pc = if skip == 0 {
                pc.max(assembler::CODE_START)
            } else {
                skip
            };
        } else if e.matches(tags::ALIGN, 1) {
            let align = e.int(1).unwrap();
            if align > 0 {
                pc = pc.wrapping_add(align - 1) & !(align - 1);
            }
        } else if e.matches(tags::READONLY_DATA, 2) {
            count = e.readonly_parts().unwrap().1.len() as i32;
        } else if e.matches(tags::WORD, 1) && e.text(1).is_some() {
            count = 2;
        } else if let Some((_, o)) = e.asm_parts() {
            count = 1 + match o.mode {
                M::Implicit => 0,
                M::Immediate | M::HighMem | M::HighMemX | M::Relative => 1,
                _ => 2,
            };
        } else {
            continue;
        }
        pc += count;
        if let Some(f) = current.as_mut() {
            f.2 += count;
        }
        if pc >= 0 {
            let entry = banks.entry(pc / assembler::BANK_SIZE).or_default();
            *entry = (*entry).max(pc);
            max_pc = max_pc.max(pc);
        }
    }
    if let Some(f) = current.take() {
        functions.push(f);
    }
    let size = ((max_pc + assembler::BANK_SIZE - 1) & !(assembler::BANK_SIZE - 1))
        .clamp(0x8000, assembler::MAX_ROM_SIZE);
    let mut out = format!(
        "# KITAQGB bank allocation simulator\n# estimated pre-assembly layout\nrom_size={size} used={}\n# bank, used, free\n",
        pc.max(assembler::CODE_START)
    );
    for b in 0..size / assembler::BANK_SIZE {
        let used = (banks
            .get(&b)
            .copied()
            .unwrap_or(0)
            .min((b + 1) * assembler::BANK_SIZE)
            - b * assembler::BANK_SIZE)
            .max(0);
        writeln!(out, "{b:>2}, {used:>5}, {:>5}", assembler::BANK_SIZE - used).unwrap();
    }
    out.push_str("\n# estimated function sizes\n# name, bank, start, end, size\n");
    functions.sort_by(|a, b| b.2.cmp(&a.2).then(ordinal(&a.0, &b.0)));
    for (name, start, size) in functions {
        writeln!(
            out,
            "{name}, {}, 0x{start:05X}, 0x{:05X}, {size}",
            start >> 14,
            start + size
        )
        .unwrap();
    }
    out
}
const CGB_REGS: [(i32, &str); 12] = [
    (0x4d, "KEY1"),
    (0x4f, "VBK"),
    (0x70, "SVBK"),
    (0x68, "BCPS"),
    (0x69, "BCPD"),
    (0x6a, "OCPS"),
    (0x6b, "OCPD"),
    (0x51, "HDMA1"),
    (0x52, "HDMA2"),
    (0x53, "HDMA3"),
    (0x54, "HDMA4"),
    (0x55, "HDMA5"),
];
pub fn cgb_consistency(lines: &[Expr], report: &Report, flag: u8) -> (String, Vec<String>) {
    let mut function = "";
    let mut writes = 0;
    let mut registers = BTreeSet::new();
    for e in lines {
        if e.matches(tags::FUNCTION, 1) {
            function = e.text(1).unwrap();
            continue;
        }
        if function == "__kq_is_cgb" {
            continue;
        }
        let Some((m, o)) = e.asm_parts() else {
            continue;
        };
        if m != "LDH_MEM_A" && m != "LD_MEM_A" {
            continue;
        }
        let reg = if let Some(name) = o.base.as_deref() {
            CGB_REGS.iter().find(|(_, r)| *r == name)
        } else if m == "LDH_MEM_A" || o.offset & 0xff00 == 0xff00 {
            CGB_REGS.iter().find(|(n, _)| *n == o.offset & 255)
        } else {
            None
        };
        if let Some((_, reg)) = reg {
            writes += 1;
            registers.insert(*reg);
        }
    }
    let guarded = report.cgb_guarded_writes.max(0);
    let unguarded = (writes - guarded).max(0);
    let checks = report.cgb_runtime_checks.max(0);
    let mut issues = Vec::new();
    if flag == 0 && unguarded > 0 {
        issues.push(
            "header is DMG-only (0x00) but unguarded CGB register writes were detected".into(),
        );
    }
    if flag == 0x80 && unguarded > 0 {
        issues.push(
            "header is CGB-compatible (0x80) but unguarded CGB register writes were detected"
                .into(),
        );
    }
    if flag == 0x80 && guarded > 0 && checks == 0 {
        issues.push(
            "CGB-compatible build uses guarded writes but no runtime CGB check calls were emitted"
                .into(),
        );
    }
    let mut out = format!(
        "# KITAQGB CGB consistency report\nheader_flag=0x{flag:02X}\nheader_mode={}\ntotal_cgb_writes={writes}\nguarded_writes={guarded}\nunguarded_writes={unguarded}\nruntime_checks={checks}\nresult={}\n\n# registers\n",
        match flag {
            0 => "dmg_only",
            0x80 => "cgb_compatible",
            0xc0 => "cgb_only",
            _ => "unknown",
        },
        if issues.is_empty() { "PASS" } else { "FAIL" }
    );
    for r in registers {
        writeln!(out, "- {r}").unwrap();
    }
    out.push_str("\n# issues\n");
    if issues.is_empty() {
        out.push_str("none\n");
    } else {
        for i in &issues {
            writeln!(out, "- {i}").unwrap();
        }
    }
    (out, issues)
}
pub fn cgb_symbols(lines: &[Expr], output: &assembler::Output) -> (String, usize) {
    let required = CGB_REGS.iter().map(|(_, r)| *r).collect::<Vec<_>>();
    let referenced = lines
        .iter()
        .filter_map(|e| e.asm_parts()?.1.base.as_deref())
        .filter(|s| required.contains(s))
        .collect::<BTreeSet<_>>();
    let missing = required
        .iter()
        .copied()
        .filter(|r| !output.symbols.contains_key(*r))
        .collect::<BTreeSet<_>>();
    let mut out = format!(
        "# KITAQGB CGB symbol verification report\nmap_symbol_count={}\nrequired_symbol_count={}\nreferenced_symbol_count={}\nmissing_symbol_count={}\nresult={}\n\n# required\n",
        output.symbols.len(),
        required.len(),
        referenced.len(),
        missing.len(),
        if missing.is_empty() { "PASS" } else { "FAIL" }
    );
    for r in required {
        writeln!(out, "- {r}").unwrap();
    }
    out.push_str("\n# referenced\n");
    if referenced.is_empty() {
        out.push_str("none\n");
    } else {
        for r in referenced {
            writeln!(out, "- {r}").unwrap();
        }
    }
    out.push_str("\n# missing\n");
    if missing.is_empty() {
        out.push_str("none\n");
    } else {
        for r in &missing {
            writeln!(out, "- {r}").unwrap();
        }
    }
    (out, missing.len())
}
