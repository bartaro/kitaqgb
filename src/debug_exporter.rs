//! Native export of the original debugger's segment/span/symbol format.
use crate::{asm::Region, reports::region_name};
use std::fmt::Write;
#[derive(Clone, Debug, Default)]
pub struct DebugInfo {
    spans: Vec<Span>,
    symbols: Vec<Symbol>,
    lines: Vec<Line>,
    comments: Vec<String>,
}
#[derive(Clone, Debug)]
struct Span {
    segment: usize,
    offset: i32,
    size: i32,
    data: bool,
}
#[derive(Clone, Debug)]
struct Symbol {
    name: String,
    address: i32,
    segment: usize,
    size: i32,
    region: String,
    bank: Option<i32>,
}
#[derive(Clone, Debug)]
struct Line {
    number: usize,
    span: usize,
}
const SEGMENTS: [(i32, i32, bool, bool); 7] = [
    (0, 0x8000, true, true),
    (0x8000, 0x2000, false, false),
    (0xa000, 0x2000, false, false),
    (0xc000, 0x2000, false, true),
    (0xe000, 0x1e00, false, false),
    (0xfe00, 0xa0, false, false),
    (0xff00, 0x100, false, true),
];
fn segment(address: i32) -> Option<usize> {
    SEGMENTS
        .iter()
        .position(|&(start, size, _, _)| address >= start && address < start + size)
}
impl DebugInfo {
    fn span(&mut self, address: i32, size: i32, data: bool) -> Option<usize> {
        let segment = segment(address)?;
        let id = self.spans.len();
        self.spans.push(Span {
            segment,
            offset: address - SEGMENTS[segment].0,
            size,
            data,
        });
        Some(id)
    }
    pub fn variable(
        &mut self,
        name: &str,
        address: i32,
        size: i32,
        region: Region,
        bank: Option<i32>,
    ) {
        if self.span(address, size, true).is_some() {
            self.symbol(name, address, size, region_name(region, address), bank);
        }
    }
    pub fn function(&mut self, name: &str, address: i32, bank: Option<i32>) {
        self.symbol(name, address, 1, "ROM".into(), bank);
    }
    fn symbol(&mut self, name: &str, address: i32, size: i32, region: String, bank: Option<i32>) {
        if let Some(segment) = segment(address) {
            self.symbols.push(Symbol {
                name: name.replace(':', "_").replace('$', "@"),
                address,
                segment,
                size,
                region,
                bank,
            });
        }
    }
    pub fn instruction(&mut self, address: i32, size: i32, comments: &[String]) {
        if let Some(span) = self.span(address, size, false) {
            if !comments.is_empty() {
                self.comments
                    .extend(comments.iter().map(|c| format!("; {c}")));
                self.comments.push("NOP".into());
                self.lines.push(Line {
                    number: self.comments.len() - 1,
                    span,
                });
            }
        }
    }
    pub fn export(&self, dbc_name: &str) -> (String, String) {
        let export = |segment: usize| SEGMENTS[segment].3;
        let mut out = String::from("version\tmajor=2,minor=0\n");
        let spans = self.spans.iter().filter(|s| export(s.segment)).count();
        let symbols = self.symbols.iter().filter(|s| export(s.segment)).count();
        writeln!(
            out,
            "info\tcsym=0,file=1,lib=0,line=0,mod=0,scope=0,seg=3,span={spans},sym={symbols},type=0"
        )
        .unwrap();
        writeln!(out, "file\tid=0,name=\"{dbc_name}\"").unwrap();
        for (id, &(start, size, rom, enabled)) in SEGMENTS.iter().enumerate() {
            if enabled {
                writeln!(
                    out,
                    "seg\tid={id},start=0x{start:04X},size=0x{size:04X},{}",
                    if rom { "type=ro,ooffs=16" } else { "type=rw" }
                )
                .unwrap();
            }
        }
        for (id, s) in self
            .spans
            .iter()
            .enumerate()
            .filter(|(_, s)| export(s.segment))
        {
            writeln!(
                out,
                "span\tid={id},seg={},start={},size={}{}",
                s.segment,
                s.offset,
                s.size,
                if s.data { ",type=0" } else { "" }
            )
            .unwrap();
        }
        for (id, s) in self
            .symbols
            .iter()
            .enumerate()
            .filter(|(_, s)| export(s.segment))
        {
            write!(
                out,
                "sym\tid={id},name=\"{}\",size={},val=0x{:04X},seg={}",
                s.name, s.size, s.address as u32, s.segment
            )
            .unwrap();
            if !s.region.is_empty() {
                write!(out, ",region=\"{}\"", s.region).unwrap();
            }
            if let Some(bank) = s.bank {
                write!(
                    out,
                    ",{}={bank}",
                    if s.region.starts_with("WRAMX") {
                        "wbank"
                    } else {
                        "bank"
                    }
                )
                .unwrap();
            }
            out.push('\n');
        }
        for (id, line) in self.lines.iter().enumerate() {
            writeln!(
                out,
                "line\tid={id},file=0,line={},span={}",
                line.number, line.span
            )
            .unwrap();
        }
        out.pop();
        let dbc = self.comments.join("\n");
        (out, dbc)
    }
}
