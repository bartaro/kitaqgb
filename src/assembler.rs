//! Banked Game Boy ROM layout and relocation, ported from Assembler.cs.
use crate::{
    asm::{AddressMode, Modifier, Operand, Region, Symbol, cpu_address},
    asm_info::{Format, OPCODES},
    expr::{Arg, Expr, Position},
    rom_header, tags,
};
use std::collections::{BTreeMap, BTreeSet};

pub const BANK_SIZE: i32 = 0x4000;
pub const CODE_START: i32 = 0x160;
pub const MAX_ROM_SIZE: i32 = 0x800000;
#[derive(Clone, Debug)]
pub struct Options {
    pub stack_top: i32,
    pub header_logo: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            stack_top: 0xdfff,
            header_logo: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub start: i32,
    pub end: i32,
    pub size: i32,
    pub bank: i32,
    pub cpu: i32,
    pub section: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub address: i32,
    pub size: i32,
    pub region: Region,
    pub bank: Option<i32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub bank: i32,
    pub address: i32,
    pub source: Position,
    pub symbol: String,
    pub section: String,
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub rom_size: usize,
    pub used_bytes: i32,
    pub bank_max_pc: Vec<i32>,
    pub functions: Vec<Function>,
    pub symbols: Vec<(String, Symbol)>,
    pub variables: Vec<Variable>,
    pub source_locations: Vec<SourceLocation>,
}
#[derive(Clone, Debug)]
pub struct Output {
    pub rom: Vec<u8>,
    pub report: Report,
    pub symbols: BTreeMap<String, Symbol>,
    pub assembly: Vec<Expr>,
    pub warnings: Vec<String>,
    pub debug: crate::debug_exporter::DebugInfo,
}
#[derive(Clone, Debug)]
struct Fixup {
    operand: Operand,
    location: i32,
    width: usize,
    relative: bool,
}

fn text<'a>(e: &'a Expr, tag: &str) -> Option<&'a str> {
    if e.matches(tag, 1) { e.text(1) } else { None }
}
fn integer(e: &Expr, tag: &str) -> Option<i32> {
    if e.matches(tag, 1) { e.int(1) } else { None }
}
fn round_up(n: i32, a: i32) -> i32 {
    if a <= 0 {
        n
    } else {
        n.wrapping_add(a - 1) & !(a - 1)
    }
}
fn backward_bank_skip(pc: i32, target: i32) -> bool {
    target != 0 && target & (BANK_SIZE - 1) == 0 && pc >= target
}
fn relative(mnemonic: &str) -> bool {
    matches!(mnemonic, "JR" | "JR_NZ" | "JR_Z" | "JR_NC" | "JR_C")
}
fn format(mode: AddressMode) -> Format {
    match mode {
        AddressMode::Immediate => Format::Immediate8,
        AddressMode::Immediate16 => Format::Immediate16,
        AddressMode::Absolute => Format::Absolute,
        AddressMode::HighMem => Format::HighMem,
        AddressMode::Indirect => Format::Indirect,
        AddressMode::Relative => Format::Relative,
        _ => Format::Implicit,
    }
}
pub fn instruction_layout_size(mnemonic: &str, operand: &Operand) -> i32 {
    let mut mode = operand.mode;
    if relative(mnemonic) && matches!(mode, AddressMode::Absolute | AddressMode::Immediate) {
        mode = AddressMode::Relative;
    }
    if matches!(mnemonic, "LD_A_MEM" | "LD_MEM_A")
        && mode == AddressMode::Absolute
        && operand.base.is_none()
        && operand.offset & 0xff00 == 0xff00
    {
        mode = AddressMode::Immediate;
    }
    1 + format(mode).size() as i32
}
fn item_size(e: &Expr) -> i32 {
    if let Some((_, bytes, _)) = e.readonly_parts() {
        bytes.len() as i32
    } else if e.is(tags::WORD) {
        2
    } else if let Some((m, o)) = e.asm_parts() {
        instruction_layout_size(m, o)
    } else {
        0
    }
}
pub fn function_body_sizes(assembly: &[Expr]) -> BTreeMap<String, i32> {
    let mut sizes = BTreeMap::new();
    let mut function = None;
    let mut bytes = 0;
    for e in assembly {
        if text(e, tags::FUNCTION).is_some() || integer(e, tags::SKIP_TO).is_some() {
            if let Some(name) = function.take() {
                sizes.insert(name, bytes);
            }
            bytes = 0;
            function = text(e, tags::FUNCTION)
                .filter(|n| !n.is_empty())
                .map(str::to_owned);
        } else if function.is_some() {
            bytes += item_size(e);
        }
    }
    if let Some(name) = function {
        sizes.insert(name, bytes);
    }
    sizes
}
pub fn relax_jumps(assembly: &[Expr]) -> Vec<Expr> {
    let mut current = assembly.to_vec();
    for _ in 0..16 {
        let mut global = BTreeMap::new();
        let mut local = BTreeMap::new();
        let mut function = String::new();
        let mut pc = CODE_START;
        for e in &current {
            if let Some(name) = text(e, tags::FUNCTION) {
                function = name.into();
                global.entry(name.to_owned()).or_insert(pc);
            } else if let Some(name) = text(e, tags::LABEL) {
                local.insert((function.clone(), name.to_owned()), pc);
            } else if let Some(target) = integer(e, tags::SKIP_TO) {
                if !backward_bank_skip(pc, target) {
                    pc = target;
                }
            } else if let Some(align) = integer(e, tags::ALIGN) {
                pc = pc.wrapping_add(align.wrapping_sub(1)) & !align.wrapping_sub(1);
            } else {
                pc += item_size(e);
            }
        }
        let mut changed = false;
        pc = CODE_START;
        function.clear();
        for e in &mut current {
            if let Some(name) = text(e, tags::FUNCTION) {
                function = name.into();
                continue;
            }
            if let Some(target) = integer(e, tags::SKIP_TO) {
                if !backward_bank_skip(pc, target) {
                    pc = target;
                }
                continue;
            }
            if let Some(align) = integer(e, tags::ALIGN) {
                pc = pc.wrapping_add(align.wrapping_sub(1)) & !align.wrapping_sub(1);
                continue;
            }
            if let Some((mnemonic, operand)) = e.asm_parts() {
                let size = instruction_layout_size(mnemonic, operand);
                let jr = match mnemonic {
                    "JP" => Some("JR"),
                    "JP_NZ" => Some("JR_NZ"),
                    "JP_Z" => Some("JR_Z"),
                    "JP_NC" => Some("JR_NC"),
                    "JP_C" => Some("JR_C"),
                    _ => None,
                };
                if let Some(jr) = jr.filter(|_| operand.modifier == Modifier::None) {
                    let target = match &operand.base {
                        None => Some(operand.offset),
                        Some(base) => local
                            .get(&(function.clone(), base.clone()))
                            .or_else(|| global.get(base))
                            .map(|n| n.wrapping_add(operand.offset)),
                    };
                    if target.is_some_and(|n| (-128..=127).contains(&(n - (pc + 2)))) {
                        *e = Expr::asm(jr, operand.clone()).with_source(e.source.clone());
                        changed = true;
                    }
                }
                pc += size;
            } else {
                pc += item_size(e);
            }
        }
        if !changed {
            break;
        }
    }
    current
}
pub fn reorder_sections(assembly: &[Expr]) -> Vec<Expr> {
    if !assembly.iter().any(|e| text(e, tags::SECTION).is_some()) {
        return assembly.to_vec();
    }
    let mut buckets: BTreeMap<(i32, String), Vec<Expr>> = BTreeMap::new();
    let mut order: BTreeMap<i32, Vec<String>> = BTreeMap::new();
    let mut bank = 0;
    let mut section = String::new();
    for e in assembly {
        if let Some(name) = text(e, tags::SECTION) {
            section = name.into();
            if !name.is_empty() {
                let row = order.entry(bank).or_default();
                if !row.contains(&section) {
                    row.push(section.clone());
                }
            }
            continue;
        }
        if let Some(target) = integer(e, tags::SKIP_TO) {
            if target & 0x3fff == 0 {
                bank = target >> 14;
                section.clear();
            }
            buckets
                .entry((bank, String::new()))
                .or_default()
                .push(e.clone());
            continue;
        }
        if integer(e, tags::ALIGN).is_some() {
            buckets
                .entry((bank, String::new()))
                .or_default()
                .push(e.clone());
            continue;
        }
        buckets
            .entry((bank, section.clone()))
            .or_default()
            .push(e.clone());
    }
    let banks = buckets.keys().map(|(b, _)| *b).collect::<BTreeSet<_>>();
    let mut out = Vec::with_capacity(assembly.len());
    for bank in banks {
        out.extend(buckets.remove(&(bank, String::new())).unwrap_or_default());
        if let Some(sections) = order.get(&bank) {
            for section in sections {
                let data = buckets.remove(&(bank, section.clone())).unwrap_or_default();
                if !data.is_empty() {
                    out.push(Expr::new(tags::SECTION, vec![Arg::Text(section.clone())]));
                    out.extend(data);
                }
            }
        }
    }
    out
}
pub fn insert_bank_boundary_skips(assembly: &[Expr]) -> Result<Vec<Expr>, String> {
    let sizes = function_body_sizes(assembly);
    let mut out = Vec::new();
    let mut pc = CODE_START;
    for e in assembly {
        let object_size = if let Some((_, bytes, _)) = e.readonly_parts() {
            bytes.len() as i32
        } else {
            text(e, tags::FUNCTION)
                .and_then(|n| sizes.get(n).copied())
                .unwrap_or(0)
        };
        if object_size > BANK_SIZE {
            return Err(format!(
                "assembler: object too large for one ROM bank ({object_size} bytes)"
            ));
        }
        let offset = pc & (BANK_SIZE - 1);
        if object_size > 0 && offset != 0 && offset + object_size > BANK_SIZE {
            pc = round_up(pc, BANK_SIZE);
            out.push(Expr::new(tags::SKIP_TO, vec![Arg::Int(pc)]).with_source(e.source.clone()));
        }
        out.push(e.clone());
        if let Some(target) = integer(e, tags::SKIP_TO) {
            if target == 0 {
                pc = pc.max(CODE_START);
            } else if !backward_bank_skip(pc, target) {
                pc = target;
            }
        } else if let Some(align) = integer(e, tags::ALIGN) {
            if align > 0 {
                pc = round_up(pc, align);
            }
        } else {
            pc += item_size(e);
        }
    }
    Ok(out)
}
pub fn estimate_max_pc(assembly: &[Expr]) -> Result<i32, String> {
    let mut pc = CODE_START;
    let mut max_pc = pc;
    for e in assembly {
        if let Some(target) = integer(e, tags::SKIP_TO) {
            if target == 0 {
                pc = pc.max(CODE_START);
            } else {
                if !(CODE_START..=MAX_ROM_SIZE).contains(&target) {
                    return Err("assembler: skip address is out of range".into());
                }
                if target < pc && !backward_bank_skip(pc, target) {
                    return Err(format!(
                        "assembler: cannot skip backward during estimate (pc=${pc:05X}, skip=${target:05X})"
                    ));
                }
                if !backward_bank_skip(pc, target) {
                    pc = target;
                }
            }
        } else if let Some(align) = integer(e, tags::ALIGN) {
            if align <= 0 || align & (align - 1) != 0 {
                return Err("assembler: $align expects power-of-two positive integer".into());
            }
            pc = round_up(pc, align);
            if pc > MAX_ROM_SIZE {
                return Err("assembler: alignment advances PC beyond max ROM size".into());
            }
        } else {
            pc += item_size(e);
        }
        max_pc = max_pc.max(pc);
    }
    Ok(max_pc)
}
pub fn infer_region(address: i32) -> Region {
    match address {
        0xff80..=0xfffe => Region::HighMem,
        0xfe00..=0xfe9f => Region::Oam,
        0xd000..=0xdfff => Region::WramX(0),
        0xc000..=0xcfff => Region::Wram0,
        0..=0x7fff => Region::Rom,
        _ => Region::Ram,
    }
}
pub fn logical_bank(region: Region, value: i32) -> Option<i32> {
    match region {
        Region::Rom if value >= 0 => Some(value >> 14),
        Region::WramX(b) if b > 0 => Some(b as i32),
        Region::WramX(_) if (0xd000..=0xdfff).contains(&value) => Some(1),
        _ => None,
    }
}
fn span(start: i32, size: i32, description: &str) -> Result<(), String> {
    if size > 0 && start >> 14 != (start + size - 1) >> 14 {
        Err(format!(
            "assembler: {description} crosses a ROM bank boundary (start=${start:05X}, size={size} bytes)"
        ))
    } else {
        Ok(())
    }
}
fn range(rom: &mut [u8], start: i32, size: usize) -> Result<&mut [u8], String> {
    if start < 0 {
        return Err("assembler: negative ROM address".into());
    }
    rom.get_mut(start as usize..start as usize + size)
        .ok_or_else(|| "assembler: ROM address out of range".into())
}
fn write_value(
    rom: &mut [u8],
    location: i32,
    width: usize,
    value: i32,
    relative: bool,
) -> Result<(), String> {
    if relative && width == 1 && !(-128..=127).contains(&value) {
        return Err(format!("Branch out of range at {location:04X}"));
    }
    let dst = range(rom, location, width)?;
    if width > 0 {
        dst[0] = value as u8;
    }
    if width > 1 {
        dst[1] = (value >> 8) as u8;
    }
    Ok(())
}
struct Emitter {
    symbols: BTreeMap<String, Symbol>,
    fixups: Vec<Fixup>,
    rom: Vec<u8>,
    report: Report,
    section: String,
    function: Option<(String, i32, i32, String)>,
}
impl Emitter {
    fn fixups(&mut self) -> Result<(), String> {
        for i in (0..self.fixups.len()).rev() {
            let fixup = &self.fixups[i];
            if let Some(value) =
                fixup
                    .operand
                    .resolve(&self.symbols, fixup.location, fixup.relative)
            {
                write_value(
                    &mut self.rom,
                    fixup.location,
                    fixup.width,
                    value,
                    fixup.relative,
                )?;
                self.fixups.remove(i);
            }
        }
        Ok(())
    }
    fn define(
        &mut self,
        name: &str,
        address: i32,
        is_label: bool,
        region: Region,
    ) -> Result<(), String> {
        let symbol = Symbol {
            value: address,
            is_label,
            region,
            section: self.section.clone(),
        };
        self.symbols
            .entry(name.into())
            .or_insert_with(|| symbol.clone());
        self.report.symbols.push((name.into(), symbol));
        self.fixups()
    }
    fn source(&mut self, e: &Expr, pc: i32, fallback: &str) {
        if e.source.filename == "<unknown>" {
            return;
        }
        self.report.source_locations.push(SourceLocation {
            bank: if pc < BANK_SIZE { 0 } else { pc >> 14 },
            address: cpu_address(pc) & 0xffff,
            source: e.source.clone(),
            symbol: self
                .function
                .as_ref()
                .map(|f| f.0.clone())
                .unwrap_or_else(|| fallback.into()),
            section: self.section.clone(),
        });
    }
    fn bank_max(&mut self, pc: i32) -> Result<(), String> {
        if pc >= 0 {
            let bank = (pc / BANK_SIZE) as usize;
            let value = self
                .report
                .bank_max_pc
                .get_mut(bank)
                .ok_or("assembler: PC beyond allocated ROM buffer")?;
            *value = (*value).max(pc);
        }
        Ok(())
    }
    fn finish_function(&mut self) -> Result<(), String> {
        if let Some((name, start, size, section)) = self.function.take() {
            span(start, size, &format!("function '{name}'"))?;
            self.report.functions.push(Function {
                name,
                start,
                end: start + size,
                size,
                bank: start >> 14,
                cpu: cpu_address(start),
                section,
            });
        }
        Ok(())
    }
    fn function_bytes(&mut self, size: i32) {
        if let Some(function) = &mut self.function {
            function.2 += size;
        }
    }
    fn operand(
        &mut self,
        operand: Operand,
        location: i32,
        width: usize,
        relative: bool,
    ) -> Result<(), String> {
        if let Some(value) = operand.resolve(&self.symbols, location, relative) {
            write_value(&mut self.rom, location, width, value, relative)
        } else {
            self.fixups.push(Fixup {
                operand,
                location,
                width,
                relative,
            });
            write_value(&mut self.rom, location, width, 0, relative)
        }
    }
}

pub fn assemble(assembly: &[Expr], options: &Options) -> Result<Output, String> {
    let assembly = insert_bank_boundary_skips(&reorder_sections(&relax_jumps(assembly)))?;
    let sizes = function_body_sizes(&assembly);
    let max_pc = estimate_max_pc(&assembly)?;
    let rom_size = round_up(max_pc, BANK_SIZE).max(0x8000);
    if rom_size > MAX_ROM_SIZE {
        return Err(format!(
            "assembler: program requires ROM larger than supported maximum ({rom_size} bytes > {MAX_ROM_SIZE} bytes)"
        ));
    }
    let report = Report {
        rom_size: rom_size as usize,
        bank_max_pc: vec![0; (rom_size / BANK_SIZE) as usize],
        ..Report::default()
    };
    let mut state = Emitter {
        rom: vec![0xff; rom_size as usize],
        symbols: BTreeMap::new(),
        fixups: Vec::new(),
        report,
        section: String::new(),
        function: None,
    };
    let warnings = Vec::new();
    let mut rst = BTreeMap::new();
    let mut functions = BTreeSet::new();
    for e in &assembly {
        if e.matches(tags::RST_MAP, 2) {
            if let (Some(vector), Some(target)) = (e.int(1), e.text(2)) {
                if !(0..=0x38).contains(&vector) || vector & 7 != 0 {
                    return Err(format!(
                        "Ignoring invalid $rst_map vector: {vector:02X} (target {target})"
                    ));
                } else {
                    rst.insert(vector, target.to_owned());
                }
            }
        }
        if let Some(name) = text(e, tags::FUNCTION) {
            functions.insert(name.to_owned());
        }
    }
    for vector in (0..=0x38).step_by(8) {
        state.rom[vector..vector + 8].fill(0);
        state.rom[vector..vector + 3].copy_from_slice(&[0xc3, 0x50, 1]);
        if let Some(target) = rst.get(&(vector as i32)) {
            state.operand(
                Operand::symbol(target, AddressMode::Absolute),
                vector as i32 + 1,
                2,
                false,
            )?;
        }
    }
    for vector in (0x40..=0x60).step_by(8) {
        state.rom[vector..vector + 8].fill(0);
        state.rom[vector] = 0xd9;
    }
    state.rom[0x40..0x48].copy_from_slice(&[0xf5, 0x3e, 1, 0xea, 0x9c, 0xc2, 0xf1, 0xd9]);
    for (vector, target) in [
        (0x40, "__kq_vblank_vector"),
        (0x48, "__kq_stat_vector"),
        (0x58, "__kq_serial_vector"),
    ] {
        if functions.contains(target) {
            state.rom[vector..vector + 8].fill(0);
            state.rom[vector] = 0xc3;
            state.operand(
                Operand::symbol(target, AddressMode::Absolute),
                vector as i32 + 1,
                2,
                false,
            )?;
        }
    }
    state.rom[0x100..0x104].copy_from_slice(&[0, 0xc3, 0x50, 1]);
    rom_header::write_logo(&mut state.rom, options.header_logo);
    state.rom[0x134..0x144].fill(0);
    state.rom[0x134..0x13b].copy_from_slice(b"KITAQGB");
    state.rom[0x147] = if rom_size > 0x8000 { 0x19 } else { 0 };
    state.rom[0x148] = rom_header::rom_size_code(rom_size as usize);
    state.rom[0x149] = 0;
    state.rom[0x150..0x160].copy_from_slice(&[
        0xf3,
        0x31,
        options.stack_top as u8,
        (options.stack_top >> 8) as u8,
        0x3e,
        0,
        0xea,
        0,
        0x20,
        0xe0,
        0x82,
        0xcd,
        0,
        0,
        0x18,
        0xfe,
    ]);
    state.operand(
        Operand::symbol("main", AddressMode::Immediate).with_modifier(Modifier::Bank),
        0x155,
        1,
        false,
    )?;
    state.operand(
        Operand::symbol("main", AddressMode::Absolute),
        0x15c,
        2,
        false,
    )?;
    let mut pc = CODE_START;
    state.bank_max(pc)?;
    let mut debug = crate::debug_exporter::DebugInfo::default();
    let mut comments = Vec::new();
    for e in &assembly {
        if let Some((name, bytes, relocations)) = e.readonly_parts() {
            span(pc, bytes.len() as i32, &format!("readonly data '{name}'"))?;
            state.define(name, pc, false, Region::Rom)?;
            debug.variable(
                name,
                cpu_address(pc),
                bytes.len() as i32,
                Region::Rom,
                Some(pc >> 14),
            );
            state.report.variables.push(Variable {
                name: name.into(),
                address: cpu_address(pc),
                size: bytes.len() as i32,
                region: Region::Rom,
                bank: Some(pc >> 14),
            });
            state.source(e, pc, name);
            range(&mut state.rom, pc, bytes.len())?.copy_from_slice(bytes);
            for relocation in relocations {
                let [_, Arg::Int(offset), Arg::Operand(operand)] = relocation.args.as_slice()
                else {
                    return Err("assembler: invalid readonly word relocation".into());
                };
                if !relocation.is(tags::WORD) || *offset < 0 || *offset as usize + 2 > bytes.len() {
                    return Err("assembler: invalid readonly word relocation".into());
                }
                state.fixups.push(Fixup {
                    operand: operand.clone(),
                    location: pc + offset,
                    width: 2,
                    relative: false,
                });
            }
            pc += bytes.len() as i32;
            state.bank_max(pc)?;
            continue;
        }
        if let Some(name) = text(e, tags::FUNCTION) {
            state.finish_function()?;
            if let Some(size) = sizes.get(name) {
                let offset = pc & (BANK_SIZE - 1);
                if *size > 0 && *size <= BANK_SIZE && offset != 0 && offset + size > BANK_SIZE {
                    pc = round_up(pc, BANK_SIZE);
                    state.bank_max(pc)?;
                }
            }
            state.define(name, pc, false, Region::Rom)?;
            debug.function(name, cpu_address(pc), Some(pc >> 14));
            state.function = Some((name.into(), pc, 0, state.section.clone()));
            state.symbols.retain(|_, s| !s.is_label);
        } else if let Some(name) = text(e, tags::LABEL) {
            state.define(name, pc, true, Region::Rom)?;
            debug.function(name, cpu_address(pc), Some(pc >> 14));
        } else if let Some(comment) = text(e, tags::COMMENT) {
            comments.push(comment.to_owned());
        } else if let Some(target) = integer(e, tags::SKIP_TO) {
            if target == 0 {
                pc = pc.max(CODE_START);
                state.bank_max(pc)?;
                continue;
            }
            if target < CODE_START || target > rom_size {
                return Err("assembler: skip address is out of range".into());
            }
            if pc > target {
                if backward_bank_skip(pc, target) {
                    state.bank_max(pc)?;
                    continue;
                }
                return Err(format!(
                    "assembler: cannot skip backward during emit (pc=${pc:05X}, skip=${target:05X})"
                ));
            }
            range(&mut state.rom, pc, (target - pc) as usize)?.fill(0);
            pc = target;
            state.bank_max(pc)?;
        } else if let Some(align) = integer(e, tags::ALIGN) {
            if align <= 0 || align & (align - 1) != 0 {
                return Err("assembler: $align expects power-of-two positive integer".into());
            }
            let next = round_up(pc, align);
            range(&mut state.rom, pc, (next - pc) as usize)?.fill(0);
            pc = next;
            state.bank_max(pc)?;
        } else if let Some(section) = text(e, tags::SECTION) {
            state.section = section.into();
        } else if let Some(label) = text(e, tags::WORD) {
            span(pc, 2, &format!("$word '{label}'"))?;
            state.operand(Operand::symbol(label, AddressMode::Immediate), pc, 2, false)?;
            pc += 2;
            state.function_bytes(2);
            state.bank_max(pc)?;
        } else if e.is(tags::VARIABLE) && matches!(e.args.len(), 4 | 5) {
            if let (Some(name), Some(address), Some(size)) = (e.text(1), e.int(2), e.int(3)) {
                let mut region = match e.args.get(4) {
                    Some(Arg::Region(r)) => *r,
                    _ => infer_region(address),
                };
                if region == Region::Ram
                    || region == Region::WramX(0) && (0xc000..=0xcfff).contains(&address)
                {
                    region = infer_region(address);
                }
                state.define(name, address, false, region)?;
                debug.variable(name, address, size, region, logical_bank(region, address));
                state.report.variables.push(Variable {
                    name: name.into(),
                    address: address & 0xffff,
                    size,
                    region,
                    bank: logical_bank(region, address),
                });
            }
        } else if let Some((m, original)) = e.asm_parts() {
            let mut mnemonic = m;
            let mut operand = original.clone();
            if matches!(mnemonic, "LD_A_MEM" | "LD_MEM_A") {
                let high = if operand.mode == AddressMode::Immediate {
                    true
                } else if operand.mode == AddressMode::Absolute {
                    let value = if operand.base.is_none() {
                        Some(operand.offset)
                    } else {
                        operand.resolve(&state.symbols, pc, false)
                    };
                    if let Some(value) = value.filter(|n| n & 0xff00 == 0xff00) {
                        operand = Operand::integer(value & 255, AddressMode::Immediate);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if high {
                    mnemonic = if mnemonic == "LD_A_MEM" {
                        "LDH_A_MEM"
                    } else {
                        "LDH_MEM_A"
                    };
                }
            }
            let is_relative = relative(mnemonic);
            let mut mode = operand.mode;
            if is_relative && matches!(mode, AddressMode::Immediate | AddressMode::Absolute) {
                mode = AddressMode::Relative;
            }
            let actual = format(mode);
            let candidates = OPCODES
                .iter()
                .enumerate()
                .filter(|(_, op)| {
                    op.mnemonic == mnemonic
                        && (op.format == actual
                            || matches!(
                                op.format,
                                Format::Immediate8 | Format::Immediate16 | Format::HighMem
                            ) && matches!(actual, Format::Absolute | Format::Immediate8))
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                return Err(format!(
                    "{} instruction: {mnemonic} {operand}",
                    if candidates.is_empty() {
                        "invalid"
                    } else {
                        "ambiguous"
                    }
                ));
            }
            let (opcode, op) = candidates[0];
            let width = op.format.size();
            debug.instruction(pc, 1 + width as i32, &comments);
            comments.clear();
            span(pc, 1 + width as i32, &format!("instruction '{mnemonic}'"))?;
            state.source(e, pc, "");
            range(&mut state.rom, pc, 1)?[0] = opcode as u8;
            pc += 1;
            state.operand(operand, pc, width, is_relative)?;
            pc += width as i32;
            state.function_bytes(1 + width as i32);
            state.bank_max(pc)?;
        }
    }
    state.finish_function()?;
    state.fixups()?;
    if !state.fixups.is_empty() {
        return Err(format!(
            "Assembly failed due to unresolved symbols: {}",
            state
                .fixups
                .iter()
                .filter_map(|f| f.operand.base.as_deref())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    state.report.used_bytes = pc;
    state.report.source_locations.sort_by(|a, b| {
        a.bank
            .cmp(&b.bank)
            .then(a.address.cmp(&b.address))
            .then(a.source.filename.cmp(&b.source.filename))
            .then(a.source.line.cmp(&b.source.line))
            .then(a.source.column.cmp(&b.source.column))
    });
    let mut unique = Vec::new();
    for row in state.report.source_locations.drain(..) {
        if !unique.iter().any(|other: &SourceLocation| {
            other.bank == row.bank
                && other.address == row.address
                && other.source == row.source
                && other.symbol == row.symbol
        }) {
            unique.push(row);
        }
    }
    state.report.source_locations = unique;
    rom_header::fix_checksums(&mut state.rom)?;
    Ok(Output {
        rom: state.rom,
        report: state.report,
        symbols: state.symbols,
        assembly,
        warnings,
        debug,
    })
}
