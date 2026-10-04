//! Source-aware debugger, build, and SARAKURA handoff schemas from Program.cs.
use crate::{
    asm::{Region, cpu_address},
    assembler::{self, Output},
    codegen::analysis::{AggregateCopy, Function, Report},
    hash,
    json::Value,
    optimizer, reports, rom_header,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
fn obj<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn s(text: impl ToString) -> Value {
    Value::String(text.to_string())
}
fn n(value: impl Into<i64>) -> Value {
    Value::Number(value.into())
}
fn b(value: bool) -> Value {
    Value::Bool(value)
}
fn arr(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}
fn put(value: &mut Value, key: &str, field: Value) {
    if let Value::Object(o) = value {
        o.insert(key.into(), field);
    }
}
fn optional(value: &mut Value, key: &str, text: &str) {
    if !text.is_empty() {
        put(value, key, s(text));
    }
}
fn abi_fields(value: &mut Value, abi: Option<&Function>, fixed: bool) {
    if let Some(f) = abi {
        put(value, "is_stack_call", b(f.stack_call));
        put(value, "is_fast_call", b(f.fast_call));
        if fixed {
            put(value, "has_fixed_bank", b(f.fixed_bank));
            put(value, "has_fixed_order", b(f.placement_order.is_some()));
        }
        put(value, "return_size", n(f.return_size.max(0)));
        put(
            value,
            "param_sizes",
            arr(f.param_sizes.iter().map(|x| n((*x).max(0)))),
        );
    }
}
fn copy_json(copy: &AggregateCopy) -> Value {
    let mut v = obj([
        ("function", s(&copy.function)),
        ("type", s(&copy.ty)),
        ("size_bytes", n(copy.size.max(0))),
        ("strategy", s(&copy.strategy)),
    ]);
    optional(&mut v, "source", &copy.source);
    v
}
#[derive(Clone, Debug)]
pub struct Policy {
    pub fixed: bool,
    pub top: i32,
    pub reserve: i32,
    pub abi_stack: bool,
    pub opt_level: i32,
}
impl Policy {
    pub fn json(&self) -> Value {
        obj([
            ("bank", s(if self.fixed { "fixed" } else { "wramx1" })),
            ("top", n(self.top)),
            ("reserve", n(self.reserve)),
            ("auto_limit", n(self.top - self.reserve)),
            ("reserved_begin", n(self.top - self.reserve + 1)),
            ("reserved_end", n(self.top)),
        ])
    }
}
pub fn rich(
    path: &Path,
    sources: &[PathBuf],
    output: &Output,
    code: &Report,
    policy: &Policy,
) -> Value {
    let asm = &output.report;
    let mut fallback = BTreeMap::new();
    for path in sources {
        if let Ok(text) = crate::io::read_utf8(path) {
            for (name, line) in reports::source_functions(&text) {
                fallback
                    .entry(name)
                    .or_insert((path.to_string_lossy().into_owned(), line));
            }
        }
    }
    let end = |f: &assembler::Function| {
        if f.size > 0 {
            cpu_address((f.end - 1).max(f.start)) + 1
        } else {
            f.cpu & 65535
        }
    };
    let symbols = asm.symbols.iter().map(|(name, symbol)| {
        let start = if symbol.region == Region::Rom {
            cpu_address(symbol.value)
        } else {
            symbol.value & 65535
        };
        let finish = asm
            .functions
            .iter()
            .find(|f| f.name == *name)
            .filter(|f| f.size > 0)
            .map_or(start + 1, end);
        let mut v = obj([
            ("name", s(name)),
            (
                "bank",
                n(assembler::logical_bank(symbol.region, symbol.value).unwrap_or(0)),
            ),
            ("start", n(start & 65535)),
            ("end", n(finish)),
            ("kind", s(if symbol.is_label { "label" } else { "symbol" })),
            ("region", s(reports::region_name(symbol.region, start))),
        ]);
        optional(&mut v, "section", &symbol.section);
        v
    });
    let mut with_source = BTreeSet::new();
    let mut locations = asm
        .source_locations
        .iter()
        .map(|l| {
            let mut v = obj([
                ("bank", n(l.bank)),
                ("addr", n(l.address & 65535)),
                ("path", s(&l.source.filename)),
                ("line", n(l.source.line as i64 + 1)),
                ("column", n(l.source.column as i64 + 1)),
            ]);
            if !l.symbol.is_empty() {
                with_source.insert(l.symbol.clone());
                optional(&mut v, "symbol", &l.symbol);
            }
            optional(&mut v, "section", &l.section);
            v
        })
        .collect::<Vec<_>>();
    for f in &asm.functions {
        if !with_source.contains(&f.name) {
            if let Some((path, line)) = fallback.get(&f.name) {
                let mut v = obj([
                    ("bank", n(f.bank)),
                    ("addr", n(f.cpu & 65535)),
                    ("path", s(path)),
                    ("line", n(*line as i64)),
                    ("symbol", s(&f.name)),
                ]);
                optional(&mut v, "section", &f.section);
                locations.push(v);
            }
        }
    }
    let functions = asm.functions.iter().map(|f| {
        let mut v = obj([
            ("name", s(&f.name)),
            ("bank", n(f.bank)),
            ("start", n(f.cpu & 65535)),
            ("end", n(end(f))),
            ("size_bytes", n(f.size.max(0))),
        ]);
        optional(&mut v, "section", &f.section);
        abi_fields(
            &mut v,
            code.functions.iter().find(|a| a.name == f.name),
            true,
        );
        v
    });
    let estimates = asm.functions.iter().map(|f| {
        let count = |predicate: fn(&crate::codegen::analysis::Call, &str) -> bool| {
            code.calls
                .iter()
                .filter(|c| predicate(c, &f.name))
                .map(|c| c.count.max(0))
                .sum::<i32>()
        };
        let mut v = obj([
            ("name", s(&f.name)),
            ("bank", n(f.bank)),
            ("size_bytes", n(f.size.max(0))),
            ("incoming_call_count", n(count(|c, name| c.callee == name))),
            ("outgoing_call_count", n(count(|c, name| c.caller == name))),
            (
                "cross_bank_outgoing_count",
                n(count(|c, name| {
                    c.caller == name
                        && c.caller_bank >= 0
                        && c.callee_bank >= 0
                        && c.caller_bank != c.callee_bank
                })),
            ),
            (
                "far_call_count",
                n(count(|c, name| c.caller == name && c.via_farcall)),
            ),
        ]);
        abi_fields(
            &mut v,
            code.functions.iter().find(|a| a.name == f.name),
            false,
        );
        v
    });
    obj([
        ("schema_version", s("1")),
        (
            "rom",
            obj([
                ("path", s(path.display())),
                ("sha256", s(hash::file(path))),
                ("size_bytes", n(asm.rom_size as i64)),
            ]),
        ),
        ("stack_policy", policy.json()),
        ("symbols", arr(symbols)),
        ("source_locations", arr(locations)),
        ("functions", arr(functions)),
        (
            "variables",
            arr(asm.variables.iter().map(|v| {
                let mut row = obj([
                    ("name", s(&v.name)),
                    ("address", n(v.address & 65535)),
                    ("size", n(v.size.max(0))),
                    ("region", s(reports::region_name(v.region, v.address))),
                ]);
                if let Some(bank) = v.bank {
                    put(&mut row, "bank", n(bank));
                }
                row
            })),
        ),
        ("static_estimates", arr(estimates)),
        (
            "call_edges",
            arr(code.calls.iter().map(|c| {
                let mut v = obj([
                    ("caller", s(&c.caller)),
                    ("callee", s(&c.callee)),
                    ("caller_bank", n(c.caller_bank)),
                    ("callee_bank", n(c.callee_bank)),
                    ("kind", s(&c.kind)),
                    ("via_thunk", b(c.via_thunk)),
                    ("via_farcall", b(c.via_farcall)),
                    ("count", n(c.count.max(0))),
                ]);
                optional(&mut v, "last_source", &c.source);
                v
            })),
        ),
        (
            "aggregate_copies",
            arr(code.aggregate_copies.iter().map(copy_json)),
        ),
        ("abi_issues", arr(code.abi_issues.iter().map(s))),
        (
            "bank_usage",
            arr(asm.bank_max_pc.iter().enumerate().map(|(bank, pc)| {
                let start = bank as i32 * 0x4000;
                let used = (pc.min(&(start + 0x4000)) - start).max(0);
                obj([
                    ("bank", n(bank as i64)),
                    ("used_bytes", n(used)),
                    ("free_bytes", n((0x4000 - used).max(0))),
                ])
            })),
        ),
    ])
}
fn source_list(sources: &[PathBuf]) -> Value {
    let mut seen = BTreeSet::new();
    arr(sources
        .iter()
        .filter(|p| seen.insert(*p))
        .map(|p| s(p.display())))
}
pub fn build(
    path: &Path,
    sources: &[PathBuf],
    output: &Output,
    code: &Report,
    opt: &optimizer::Report,
    header: &rom_header::Options,
    policy: &Policy,
) -> Value {
    let mut sizes = BTreeMap::<String, i32>::new();
    for f in &output.report.functions {
        let size = sizes.entry(f.name.clone()).or_default();
        *size = (*size).max(f.size);
    }
    let mut incoming = BTreeMap::<String, i32>::new();
    for c in &code.calls {
        if !c.callee.is_empty() && !c.callee.starts_with('<') {
            *incoming.entry(c.callee.clone()).or_default() += c.count;
        }
    }
    let names = sizes
        .keys()
        .chain(incoming.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut hotspots = names
        .into_iter()
        .map(|name| {
            let calls = *incoming.get(&name).unwrap_or(&0);
            let size = *sizes.get(&name).unwrap_or(&0);
            let score = i64::from(calls) * i64::from(size);
            (name, calls, size, score)
        })
        .collect::<Vec<_>>();
    hotspots.sort_by(|a, b| {
        b.3.cmp(&a.3)
            .then(b.1.cmp(&a.1))
            .then(b.2.cmp(&a.2))
            .then(a.0.encode_utf16().cmp(b.0.encode_utf16()))
    });
    let mut cross = code
        .calls
        .iter()
        .filter(|c| c.caller_bank >= 0 && c.callee_bank >= 0 && c.caller_bank != c.callee_bank)
        .collect::<Vec<_>>();
    cross.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then(a.caller.encode_utf16().cmp(b.caller.encode_utf16()))
            .then(a.callee.encode_utf16().cmp(b.callee.encode_utf16()))
    });
    cross.truncate(16);
    obj([("schema_version",s("1")),("output_rom",s(path.display())),("output_sha256",s(hash::file(path))),
        ("rom_size_bytes",n(output.report.rom_size as i64)),("used_bytes",n(output.report.used_bytes)),("abi_mode",s(if policy.abi_stack{"stack"}else{"legacy"})),
        ("function_count",n(output.report.functions.len() as i64)),("call_edge_count",n(code.calls.len() as i64)),("aggregate_copy_count",n(code.aggregate_copies.len() as i64)),("optimizer_pass_count",n(opt.passes.len() as i64)),("abi_issue_count",n(code.abi_issues.len() as i64)),("cross_bank_call_count",n(cross.len() as i64)),
        ("source_files",source_list(sources)),("abi_issues",arr(code.abi_issues.iter().map(s))),
        ("header",obj([("title",s(header.title.as_deref().unwrap_or(""))),("cgb_flag",n(header.cgb.unwrap_or(0))),("cart_type",n(header.cart.unwrap_or(if output.rom.len()>32768{0x19}else{0}))),("rom_size_code",n(header.romsize.unwrap_or_else(||rom_header::rom_size_code(output.rom.len())))),("ram_size_code",n(header.ramsize.unwrap_or(0)))])),
        ("stack_policy",policy.json()),("hotspots",arr(hotspots.iter().take(16).map(|(name,calls,size,score)|obj([("name",s(name)),("incoming_calls",n(*calls)),("size_bytes",n(*size)),("score",n(*score))])))),
        ("cross_bank_edges",arr(cross.iter().map(|c|obj([("caller",s(&c.caller)),("callee",s(&c.callee)),("caller_bank",n(c.caller_bank)),("callee_bank",n(c.callee_bank)),("count",n(c.count.max(0))),("kind",s(&c.kind)),("via_thunk",b(c.via_thunk)),("via_farcall",b(c.via_farcall))])))),
        ("aggregate_copies",arr(code.aggregate_copies.iter().map(copy_json))),
        ("notes",arr(["dbg2.json is emitted alongside the ROM so KOKURA can load richer source-aware metadata directly.","source_map.txt is emitted alongside the ROM for lightweight line-level lookup without requiring the full JSON payload.","static_estimates are currently structural estimates driven by codegen/call topology and function size, not cycle-perfect timing."].into_iter().map(s)))])
}
pub fn ai(
    path: &Path,
    sources: &[PathBuf],
    output: &Output,
    code: &Report,
    policy: &Policy,
) -> Value {
    let hash = hash::file(path);
    obj([("schema",s("kitaqgb-ai-build-metadata")),("schema_version",n(1)),("producer",s("KITAQGB")),("platform",s("gb")),("build_id",s(&hash)),
        ("rom",obj([("path",s(path.display())),("hash",s(&hash)),("sha256",s(&hash)),("target",s("gb")),("size_bytes",n(output.report.rom_size as i64))])),("source_files",source_list(sources)),("stack_policy",policy.json()),
        ("compiler",obj([("abi",s(if policy.abi_stack{"stack"}else{"legacy"})),("opt_level",n(policy.opt_level)),("function_count",n(output.report.functions.len() as i64)),("call_edge_count",n(code.calls.len() as i64)),("aggregate_copy_count",n(code.aggregate_copies.len() as i64)),("abi_issue_count",n(code.abi_issues.len() as i64))])),
        ("artifacts",obj([("dbg2_json",s(path.with_extension("dbg2.json").display())),("build_report_json",s(path.with_extension("build_report.json").display())),("source_map",s(path.with_extension("source_map.txt").display()))])),
        ("notes",arr(["Metadata is a compact SARAKURA/KOKURA handoff view; detailed symbols remain in dbg2.json.","ROM hash is the build_id so emitter output can be joined to this compiler artifact."].into_iter().map(s)))])
}
