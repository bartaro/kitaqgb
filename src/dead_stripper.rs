//! Conservative closure of functions and readonly objects recognized in assembly IR.
use crate::{
    expr::{Arg, Expr},
    tags as t,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub kept_functions: Vec<String>,
    pub removed_functions: Vec<String>,
    pub kept_readonly: Vec<String>,
    pub removed_readonly: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Output {
    pub lines: Vec<Expr>,
    pub report: Report,
}
fn refs(e: &Expr) -> Option<&str> {
    if let Some((_, o)) = e.asm_parts() {
        o.base.as_deref().filter(|s| !s.is_empty())
    } else if e.matches(t::WORD, 1) {
        e.text(1).filter(|s| !s.is_empty())
    } else {
        None
    }
}
fn enqueue(
    name: &str,
    defined: &BTreeSet<String>,
    live: &mut BTreeSet<String>,
    queue: &mut VecDeque<String>,
) {
    if !name.is_empty() && defined.contains(name) && live.insert(name.into()) {
        queue.push_back(name.into())
    }
}
pub fn strip(assembly: &[Expr], keep_labels: &BTreeSet<String>) -> Output {
    let first_data = assembly.iter().position(|e| e.is(t::READONLY_DATA));
    let scan_end = first_data.unwrap_or(assembly.len());
    let functions = assembly[..scan_end]
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            if e.matches(t::FUNCTION, 1) {
                e.text(1).map(|n| (i, n.to_owned()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let defined = functions
        .iter()
        .map(|(_, name)| name.clone())
        .collect::<BTreeSet<_>>();
    let mut ranges = BTreeMap::new();
    for (k, (start, name)) in functions.iter().enumerate() {
        let end = functions.get(k + 1).map_or(scan_end, |(i, _)| *i);
        ranges.insert(name.clone(), (*start, end));
    }
    let (mut live, mut queue) = (BTreeSet::new(), VecDeque::new());
    enqueue("main", &defined, &mut live, &mut queue);
    for name in keep_labels {
        enqueue(name, &defined, &mut live, &mut queue)
    }
    for e in assembly {
        if e.matches(t::RST_MAP, 2) {
            if let Some(name) = e.text(2) {
                enqueue(name, &defined, &mut live, &mut queue)
            }
        }
    }
    for e in &assembly[..scan_end] {
        if e.is(t::FUNCTION) {
            continue;
        }
        if let Some(name) = refs(e) {
            enqueue(name, &defined, &mut live, &mut queue)
        }
    }
    while let Some(name) = queue.pop_front() {
        if let Some(&(start, end)) = ranges.get(&name) {
            for e in &assembly[start..end] {
                if let Some(name) = refs(e) {
                    enqueue(name, &defined, &mut live, &mut queue)
                }
            }
        }
    }
    let data_names = assembly[scan_end..]
        .iter()
        .filter_map(|e| {
            e.readonly_parts()
                .map(|(name, _, _)| name.to_owned())
                .filter(|n| !n.is_empty())
        })
        .collect::<BTreeSet<_>>();
    let mut live_data = BTreeSet::new();
    // Every code expression is intentionally scanned, including bodies of dead functions.
    for e in &assembly[..scan_end] {
        if e.is(t::FUNCTION) {
            continue;
        }
        if let Some(name) = refs(e) {
            if data_names.contains(name) {
                live_data.insert(name.to_owned());
            }
        }
    }
    loop {
        let count = live_data.len();
        for e in &assembly[scan_end..] {
            if let Some((owner, _, relocations)) = e.readonly_parts() {
                if live_data.contains(owner) {
                    for r in relocations {
                        if r.matches(t::WORD, 2) && r.int(1).is_some() {
                            if let Some(Arg::Operand(o)) = r.args.get(2) {
                                if let Some(base) = &o.base {
                                    if data_names.contains(base) {
                                        live_data.insert(base.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if live_data.len() == count {
            break;
        }
    }
    let mut mask = vec![true; assembly.len()];
    for (_, name) in &functions {
        if !live.contains(name) {
            if let Some(&(start, end)) = ranges.get(name) {
                mask[start..end].fill(false)
            }
        }
    }
    for (i, e) in assembly.iter().enumerate().skip(scan_end) {
        if let Some((name, _, _)) = e.readonly_parts() {
            if !live_data.contains(name) {
                mask[i] = false
            }
        }
    }
    let mut removed_functions = functions
        .iter()
        .filter(|(_, n)| !live.contains(n))
        .map(|(_, n)| n.clone())
        .collect::<Vec<_>>();
    removed_functions.sort();
    let report = Report {
        kept_functions: live.into_iter().collect(),
        removed_functions,
        kept_readonly: live_data.iter().cloned().collect(),
        removed_readonly: data_names.difference(&live_data).cloned().collect(),
    };
    Output {
        lines: assembly
            .iter()
            .zip(mask)
            .filter(|(_, keep)| *keep)
            .map(|(e, _)| e.clone())
            .collect(),
        report,
    }
}
