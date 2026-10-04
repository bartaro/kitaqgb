//! Static CALL-site selection for restart-vector compression.
use crate::{
    asm::{AddressMode as M, Operand},
    expr::Expr,
    tags,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Options {
    pub enabled: bool,
    pub unsafe_mode: bool,
    pub use_38: bool,
    pub max_calls: i32,
    pub max_vectors: usize,
    pub exclude: BTreeSet<String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            enabled: false,
            unsafe_mode: false,
            use_38: false,
            max_calls: i32::MAX,
            max_vectors: 0,
            exclude: BTreeSet::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub vector: i32,
    pub target: String,
    pub calls: i32,
    pub net_bytes: i32,
}
#[derive(Default)]
struct Safety {
    defined: bool,
    sensitive: bool,
    calls: BTreeSet<String>,
}
fn hardware_address(n: i32) -> bool {
    let n = n & 0xffff;
    (0xff00..0xff80).contains(&n) || matches!(n, 0 | 0x2000 | 0x4000 | 0x6000)
}
fn hardware_name(name: &str) -> bool {
    let name = name.trim();
    matches!(
        name,
        "P1" | "SB"
            | "SC"
            | "DIV"
            | "TIMA"
            | "TMA"
            | "TAC"
            | "IF"
            | "IE"
            | "LCDC"
            | "STAT"
            | "SCY"
            | "SCX"
            | "LY"
            | "LYC"
            | "DMA"
            | "BGP"
            | "OBP0"
            | "OBP1"
            | "WY"
            | "WX"
            | "VBK"
            | "SVBK"
            | "KEY1"
            | "BCPS"
            | "BCPD"
            | "OCPS"
            | "OCPD"
            | "HDMA1"
            | "HDMA2"
            | "HDMA3"
            | "HDMA4"
            | "HDMA5"
    ) || name.starts_with("NR")
        || name.starts_with("WAVE")
}
fn sensitive_instruction(m: &str, o: &Operand) -> bool {
    if matches!(m, "DI" | "EI" | "HALT" | "STOP" | "RETI") || m.starts_with("RST_") {
        return true;
    }
    if matches!(m, "LDH_A_MEM" | "LDH_MEM_A") {
        return if matches!(o.mode, M::Immediate | M::HighMem) && o.base.is_none() {
            o.offset & 255 < 128
        } else {
            true
        };
    }
    if matches!(m, "LD_A_MEM" | "LD_MEM_A") && o.mode == M::Absolute {
        return o
            .base
            .as_deref()
            .map_or_else(|| hardware_address(o.offset), hardware_name);
    }
    matches!(m, "LD_HL_IMM" | "LD_DE_IMM" | "LD_BC_IMM")
        && o.mode == M::Immediate16
        && o.base.is_none()
        && hardware_address(o.offset)
}
fn forced_name(name: &str, unsafe_mode: bool) -> bool {
    name.is_empty()
        || name == "main"
        || name.starts_with("__kq_")
        || (!unsafe_mode && !name.starts_with("__"))
}
fn call_target<'a>(m: &str, o: &'a Operand) -> Option<&'a str> {
    (m == "CALL" && o.mode == M::Absolute && o.offset == 0)
        .then_some(o.base.as_deref())
        .flatten()
}
pub fn select(lines: &[Expr], options: &Options) -> Vec<Selection> {
    if !options.enabled {
        return Vec::new();
    }
    let mut safety = BTreeMap::<String, Safety>::new();
    let mut counts = BTreeMap::<String, i32>::new();
    let mut function: Option<&str> = None;
    for e in lines {
        if e.matches(tags::FUNCTION, 1) {
            function = e.text(1);
            if let Some(name) = function {
                safety.entry(name.into()).or_default().defined = true;
            }
            continue;
        }
        let Some((m, o)) = e.asm_parts() else {
            continue;
        };
        if let Some(target) = call_target(m, o) {
            if target != "main" {
                *counts.entry(target.into()).or_default() += 1;
            }
            if let Some(name) = function {
                safety
                    .entry(name.into())
                    .or_default()
                    .calls
                    .insert(target.into());
                safety.entry(target.into()).or_default();
            }
        }
        if let Some(name) = function {
            if sensitive_instruction(m, o) {
                safety.entry(name.into()).or_default().sensitive = true;
            }
        }
    }
    for (name, info) in &mut safety {
        info.sensitive |= forced_name(name, options.unsafe_mode);
    }
    loop {
        let changed = safety
            .iter()
            .filter_map(|(name, info)| {
                (!info.sensitive
                    && info.calls.iter().any(|callee| {
                        forced_name(callee, options.unsafe_mode)
                            || safety.get(callee).is_none_or(|info| info.sensitive)
                    }))
                .then_some(name.clone())
            })
            .collect::<Vec<_>>();
        if changed.is_empty() {
            break;
        }
        for name in changed {
            safety.get_mut(&name).unwrap().sensitive = true;
        }
    }
    let mut candidates = counts
        .into_iter()
        .filter(|(name, calls)| {
            *calls >= 2
                && *calls <= options.max_calls
                && !options.exclude.contains(name)
                && (options.unsafe_mode
                    || safety.get(name).is_some_and(|s| s.defined && !s.sensitive))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(a, ac), (b, bc)| {
        bc.cmp(ac)
            .then_with(|| super::identifier_order::compare(a, b))
    });
    let available = if options.use_38 { 8 } else { 7 };
    let count = if options.max_vectors == 0 {
        available
    } else {
        options.max_vectors.min(available)
    };
    candidates
        .into_iter()
        .take(count)
        .enumerate()
        .map(|(i, (target, calls))| Selection {
            vector: (i as i32) * 8,
            target,
            calls,
            net_bytes: 2 * calls - 3,
        })
        .collect()
}
