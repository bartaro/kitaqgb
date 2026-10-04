//! Conservative assembly optimizations, in the same pass order as the C# compiler.
use crate::{
    asm::{AddressMode as Mode, Modifier, Operand},
    expr::Expr,
    tags as t,
};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct PassReport {
    pub name: String,
    pub before_lines: usize,
    pub after_lines: usize,
    pub changed_lines: usize,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub diff_text: String,
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub passes: Vec<PassReport>,
    pub total_rst_rewrites: usize,
    pub rst_rewrite_counts_by_vector: BTreeMap<i32, usize>,
}
#[derive(Clone, Debug)]
pub struct Output {
    pub lines: Vec<Expr>,
    pub report: Report,
}
fn asm(m: &str) -> Expr {
    Expr::asm(m, Operand::implicit())
}
fn asm_operand(m: &str, operand: &Operand) -> Expr {
    Expr::asm(m, operand.clone())
}
fn show(e: &Expr) -> String {
    if let Some((m, o)) = e.asm_parts() {
        if o.mode == Mode::Implicit {
            m.into()
        } else {
            format!("{m} {o}")
        }
    } else {
        e.show()
    }
}
fn diff(before: &[String], after: &[String]) -> String {
    let nl = if cfg!(windows) { "\r\n" } else { "\n" };
    let mut lines = Vec::new();
    let mut emitted = 0;
    for (i, (a, b)) in before.iter().zip(after).enumerate() {
        if emitted == 120 {
            break;
        }
        if a != b {
            lines.extend([
                format!("@@ line {} @@", i + 1),
                format!("- {a}"),
                format!("+ {b}"),
            ]);
            emitted += 1
        }
    }
    if emitted < 120 {
        if after.len() > before.len() {
            for b in &after[before.len()..] {
                if emitted == 120 {
                    break;
                }
                lines.push(format!("+ {b}"));
                emitted += 1
            }
        } else if before.len() > after.len() {
            for a in &before[after.len()..] {
                if emitted == 120 {
                    break;
                }
                lines.push(format!("- {a}"));
                emitted += 1
            }
        }
    }
    if lines.is_empty() {
        lines.push("(no textual changes)".into())
    } else if emitted >= 120 {
        lines.push("... (truncated)".into())
    }
    lines.join(nl) + nl
}
fn pass_report(name: &str, before: &[Expr], after: &[Expr]) -> PassReport {
    let a = before.iter().map(show).collect::<Vec<_>>();
    let b = after.iter().map(show).collect::<Vec<_>>();
    PassReport {
        name: name.into(),
        before_lines: a.len(),
        after_lines: b.len(),
        changed_lines: a.iter().zip(&b).filter(|(a, b)| a != b).count(),
        added_lines: b.len().saturating_sub(a.len()),
        removed_lines: a.len().saturating_sub(b.len()),
        diff_text: diff(&a, &b),
    }
}
pub fn optimize(source: &[Expr], level: i32) -> Output {
    let mut report = Report::default();
    let mut rst = BTreeMap::new();
    for e in source {
        if e.is(t::RST_MAP) {
            if let (Some(vector), Some(target)) = (e.int(1), e.text(2)) {
                rst.insert(target.to_owned(), vector);
            }
        }
    }
    let first = remove_unreachable(&run_pass(source, &rst, &mut report));
    report
        .passes
        .push(pass_report("run_pass_1", source, &first));
    let second = remove_unreachable(&run_pass(&first, &rst, &mut report));
    report
        .passes
        .push(pass_report("run_pass_2", &first, &second));
    let mut lines = mini_cse(&second);
    report.passes.push(pass_report("mini_cse", &second, &lines));
    if level >= 1 {
        let next = o1_peepholes(&lines);
        report
            .passes
            .push(pass_report("o1_peepholes", &lines, &next));
        lines = next
    }
    Output { lines, report }
}
fn no_op_move(m: &str) -> bool {
    matches!(
        m,
        "LD_A_A" | "LD_B_B" | "LD_C_C" | "LD_D_D" | "LD_E_E" | "LD_H_H" | "LD_L_L"
    )
}
fn o1_peepholes(lines: &[Expr]) -> Vec<Expr> {
    let mut out = Vec::new();
    for (i, e) in lines.iter().enumerate() {
        if let Some((m, o)) = e.asm_parts() {
            if no_op_move(m) {
                continue;
            }
            if matches!(m, "JP" | "JR")
                && o.mode == Mode::Absolute
                && o.offset == 0
                && o.base.is_some()
            {
                let mut j = i + 1;
                while j < lines.len()
                    && matches!(lines[j].tag(), Some(t::COMMENT | t::SECTION | t::ALIGN))
                {
                    j += 1
                }
                if lines
                    .get(j)
                    .is_some_and(|e| e.is(t::LABEL) && e.text(1) == o.base.as_deref())
                {
                    continue;
                }
            }
        }
        out.push(e.clone())
    }
    out
}
fn run_pass(lines: &[Expr], rst: &BTreeMap<String, i32>, report: &mut Report) -> Vec<Expr> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let current = &lines[i];
        let Some((m, o)) = current.asm_parts() else {
            out.push(current.clone());
            i += 1;
            continue;
        };
        if m == "CALL" && o.mode == Mode::Absolute && o.offset == 0 {
            if let Some(vector) = o.base.as_ref().and_then(|b| rst.get(b)) {
                out.push(asm(&format!("RST_{vector:02X}")));
                report.total_rst_rewrites += 1;
                *report
                    .rst_rewrite_counts_by_vector
                    .entry(*vector)
                    .or_default() += 1;
                i += 1;
                continue;
            }
        }
        if let (Some((m1, o1)), Some((m2, _))) = (
            lines.get(i + 1).and_then(Expr::asm_parts),
            lines.get(i + 2).and_then(Expr::asm_parts),
        ) {
            if m == "LD_A_HL" && m2 == "LD_HL_A" {
                let increment = matches!(m1, "INC_A" | "ADD_A_IMM");
                let immediate = matches!(m1, "ADD_A_IMM" | "SUB_IMM");
                if (matches!(m1, "INC_A" | "DEC_A")
                    || (immediate
                        && o1.mode == Mode::Immediate
                        && o1.offset == 1
                        && o1.base.is_none()))
                    && can_fold_memory_update(lines, i + 3, immediate)
                {
                    out.push(asm(if increment {
                        "INC_HL_REF"
                    } else {
                        "DEC_HL_REF"
                    }));
                    i += 3;
                    continue;
                }
            }
            if m == "LD_HL_IMM"
                && m1 == "LD_DE_IMM"
                && m2 == "ADD_HL_DE"
                && matches!(o.mode, Mode::Immediate | Mode::Immediate16)
                && matches!(o1.mode, Mode::Immediate | Mode::Immediate16)
            {
                out.push(asm_operand(
                    "LD_HL_IMM",
                    &Operand {
                        base: o.base.clone(),
                        offset: o.offset.wrapping_add(o1.offset),
                        mode: Mode::Immediate16,
                        modifier: Modifier::None,
                        comment: None,
                    },
                ));
                i += 3;
                continue;
            }
        }
        if let Some(next) = lines.get(i + 1) {
            if let Some((nm, no)) = next.asm_parts() {
                if m == "CP_IMM"
                    && o.mode == Mode::Immediate
                    && o.base.is_none()
                    && o.offset == 0
                    && matches!(nm, "JP_Z" | "JP_NZ" | "JR_Z" | "JR_NZ")
                {
                    if i == 0
                        || !lines[i - 1]
                            .asm_parts()
                            .is_some_and(|(p, _)| sets_z_from_a(p))
                    {
                        out.push(asm("OR_A"))
                    }
                    out.push(next.clone());
                    i += 2;
                    continue;
                }
                if matches!(
                    (m, nm),
                    ("PUSH_HL", "POP_HL")
                        | ("PUSH_BC", "POP_BC")
                        | ("PUSH_DE", "POP_DE")
                        | ("PUSH_AF", "POP_AF")
                ) {
                    i += 2;
                    continue;
                }
                let transfer = match (m, nm) {
                    ("PUSH_HL", "POP_DE") => Some(["LD_D_H", "LD_E_L"]),
                    ("PUSH_DE", "POP_HL") => Some(["LD_H_D", "LD_L_E"]),
                    ("PUSH_BC", "POP_DE") => Some(["LD_D_B", "LD_E_C"]),
                    ("PUSH_DE", "POP_BC") => Some(["LD_B_D", "LD_C_E"]),
                    ("PUSH_BC", "POP_HL") => Some(["LD_H_B", "LD_L_C"]),
                    ("PUSH_HL", "POP_BC") => Some(["LD_B_H", "LD_C_L"]),
                    _ => None,
                };
                if let Some(transfer) = transfer {
                    out.extend(transfer.map(asm));
                    i += 2;
                    continue;
                }
                if m == "LD_A_IMM" && o.mode == Mode::Immediate {
                    let replacement = match nm {
                        "LD_B_A" => Some("LD_B_IMM"),
                        "LD_C_A" => Some("LD_C_IMM"),
                        "LD_D_A" => Some("LD_D_IMM"),
                        "LD_E_A" => Some("LD_E_IMM"),
                        "LD_H_A" => Some("LD_H_IMM"),
                        "LD_L_A" => Some("LD_L_IMM"),
                        _ => None,
                    };
                    if let Some(replacement) = replacement {
                        out.push(asm_operand(replacement, o));
                        i += 2;
                        continue;
                    }
                }
                if m == "LD_DE_IMM" && nm == "ADD_HL_DE" && o.mode == Mode::Immediate16 {
                    let n = o.offset as i16;
                    if (1..=4).contains(&n) || (-4..=-1).contains(&n) {
                        for _ in 0..n.abs() {
                            out.push(asm(if n > 0 { "INC_HL" } else { "DEC_HL" }))
                        }
                        i += 2;
                        continue;
                    }
                }
                let fused = match (m, nm) {
                    ("LD_HL_A", "INC_HL") => Some("LDI_HL_A"),
                    ("LD_A_HL", "INC_HL") => Some("LDI_A_HL"),
                    ("LD_HL_A", "DEC_HL") => Some("LDD_HL_A"),
                    ("LD_A_HL", "DEC_HL") => Some("LDD_A_HL"),
                    _ => None,
                };
                if let Some(fused) = fused {
                    out.push(asm(fused));
                    i += 2;
                    continue;
                }
                if m == "LD_HL_IMM"
                    && o.mode == Mode::Immediate16
                    && matches!(nm, "INC_HL" | "DEC_HL")
                {
                    let delta = if nm == "INC_HL" { 1 } else { -1 };
                    out.push(asm_operand(
                        "LD_HL_IMM",
                        &Operand {
                            base: o.base.clone(),
                            offset: o.offset.wrapping_add(delta),
                            mode: Mode::Immediate16,
                            modifier: Modifier::None,
                            comment: None,
                        },
                    ));
                    i += 2;
                    continue;
                }
                // Preserve the source's unreachable shape: this check is inside an assembly-only branch.
                let _ = no;
            }
        }
        if !no_op_move(m) {
            out.push(current.clone())
        }
        i += 1;
    }
    out
}
fn can_fold_memory_update(lines: &[Expr], start: usize, carry_differs: bool) -> bool {
    let (mut need_a, mut need_carry) = (true, carry_differs);
    for e in lines.iter().skip(start).take(16) {
        let Some((m, _)) = e.asm_parts() else {
            return false;
        };
        if matches!(m, "XOR_A" | "POP_AF") {
            need_a = false;
            need_carry = false
        } else if m == "LD_HL_SP_IMM" {
            need_carry = false
        } else if (m.starts_with("LD_A_") && m != "LD_A_A") || m.starts_with("LDH_A_") {
            need_a = false
        } else if m.starts_with("LD_")
            || m.starts_with("LDH_")
            || matches!(m, "LDI_HL_A" | "LDD_HL_A")
        {
            if m.ends_with("_A") && need_a {
                return false;
            }
        } else if matches!(
            m,
            "NOP"
                | "DI"
                | "EI"
                | "INC_HL"
                | "DEC_HL"
                | "INC_BC"
                | "DEC_BC"
                | "INC_DE"
                | "DEC_DE"
                | "INC_SP"
                | "DEC_SP"
        ) {
        } else if ["AND", "OR", "XOR", "ADD_A", "SUB"]
            .iter()
            .any(|p| m.starts_with(p))
        {
            if need_a {
                return false;
            }
            need_carry = false
        } else {
            return false;
        }
        if !need_a && !need_carry {
            return true;
        }
    }
    false
}
fn control_flow(m: &str) -> bool {
    ["JP", "JR", "RET", "RST_"].iter().any(|p| m.starts_with(p))
        || matches!(m, "HALT" | "STOP" | "CALL")
}
fn memory_write(m: &str) -> bool {
    m.starts_with("LD_MEM_")
        || m.starts_with("LDH_MEM_")
        || matches!(m, "LD_HL_A" | "LDI_HL_A" | "LDD_HL_A")
        || m.ends_with("HL_REF")
        || m.contains("HL_REF_")
}
fn fixed_load_key(m: &str, o: &Operand) -> Option<String> {
    if o.base.is_some() {
        return None;
    }
    if m == "LD_A_MEM" && o.mode == Mode::Absolute {
        let addr = o.offset & 65535;
        if !(0xff00..0xff80).contains(&addr) {
            return Some(format!("ABS:{addr}"));
        }
    }
    if m == "LDH_A_MEM" && o.mode == Mode::HighMem {
        let zp = o.offset & 255;
        if zp >= 128 {
            return Some(format!("ZP:{zp}"));
        }
    }
    None
}
fn mini_cse(lines: &[Expr]) -> Vec<Expr> {
    let mut out = Vec::new();
    let mut registers = BTreeMap::<String, String>::new();
    for e in lines {
        let Some((m, o)) = e.asm_parts() else {
            out.push(e.clone());
            registers.clear();
            continue;
        };
        if control_flow(m) || memory_write(m) {
            out.push(e.clone());
            registers.clear();
            continue;
        }
        let movement = match m {
            "LD_B_A" => Some(("B", "A")),
            "LD_C_A" => Some(("C", "A")),
            "LD_D_A" => Some(("D", "A")),
            "LD_E_A" => Some(("E", "A")),
            "LD_H_A" => Some(("H", "A")),
            "LD_L_A" => Some(("L", "A")),
            "LD_A_B" => Some(("A", "B")),
            "LD_A_C" => Some(("A", "C")),
            "LD_A_D" => Some(("A", "D")),
            "LD_A_E" => Some(("A", "E")),
            "LD_A_H" => Some(("A", "H")),
            "LD_A_L" => Some(("A", "L")),
            "LD_D_H" => Some(("D", "H")),
            "LD_E_L" => Some(("E", "L")),
            _ => None,
        };
        if let Some((dst, src)) = movement {
            if let Some(value) = registers.get(src).cloned() {
                registers.insert(dst.into(), value);
            } else {
                registers.remove(dst);
            }
            out.push(e.clone());
            continue;
        }
        if let Some(key) = fixed_load_key(m, o) {
            if registers.get("A") == Some(&key) {
                continue;
            }
            if let Some(reg) = ["B", "C", "D", "E", "H", "L"]
                .iter()
                .find(|r| registers.get(**r) == Some(&key))
            {
                out.push(asm(&format!("LD_A_{reg}")));
            } else {
                out.push(e.clone())
            }
            registers.insert("A".into(), key);
            continue;
        }
        invalidate_registers(m, &mut registers);
        out.push(e.clone())
    }
    out
}
fn invalidate_registers(m: &str, registers: &mut BTreeMap<String, String>) {
    if m == "LD_A_IMM" || m.starts_with("LD_A_") || m.starts_with("LDH_A_") || m == "XOR_A" {
        registers.remove("A");
        return;
    }
    if ["ADD_A", "ADC_A", "SUB", "SBC", "AND", "OR", "XOR"]
        .iter()
        .any(|p| m.starts_with(p))
        || matches!(m, "INC_A" | "DEC_A")
    {
        if m != "OR_A" {
            registers.remove("A");
        }
        return;
    }
    for r in ["B", "C", "D", "E", "H", "L"] {
        if m.starts_with(&format!("LD_{r}_")) || m == format!("INC_{r}") || m == format!("DEC_{r}")
        {
            registers.remove(r);
        }
    }
    if matches!(m, "INC_HL" | "DEC_HL") || m.starts_with("ADD_HL") || m.starts_with("LD_HL") {
        registers.remove("H");
        registers.remove("L");
    }
    if m.starts_with("POP_") || m.starts_with("PUSH_") {
        registers.clear()
    }
}
fn remove_unreachable(lines: &[Expr]) -> Vec<Expr> {
    let mut out = Vec::new();
    let mut skipping = false;
    for e in lines {
        if skipping {
            if e.is(t::LABEL) || !e.is(t::ASM) {
                skipping = false;
                out.push(e.clone())
            }
            continue;
        }
        out.push(e.clone());
        if e.asm_parts()
            .is_some_and(|(m, _)| matches!(m, "JP" | "JR" | "JP_HL" | "RET" | "RETI"))
        {
            skipping = true
        }
    }
    out
}
fn sets_z_from_a(m: &str) -> bool {
    ["OR_", "AND_", "XOR_", "ADD_A_", "ADC_A_", "SUB_", "SBC_A_"]
        .iter()
        .any(|p| m.starts_with(p))
        || matches!(m, "INC_A" | "DEC_A")
}
