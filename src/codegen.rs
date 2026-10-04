//! Code-generation state. Native emission is being connected to these models.
use crate::{
    asm::{Operand, Region},
    ctype::{Aggregate, AggregateLayout, CType, Field, Kind, Simple},
    expr::{Arg, Expr, Position},
    tags as t,
    tokenizer::{Diagnostic, Severity},
};
use std::collections::{BTreeMap, BTreeSet};
pub mod allocation;
pub mod analysis;
mod constants;
pub mod emission;
mod folding;
mod identifier_order;
mod intrinsic_types;
pub mod readonly;
pub mod rst;
mod types;
use allocation::Allocator;
#[derive(Clone, Debug)]
pub struct Options {
    pub fixed_stack: bool,
    pub stack_top: i32,
    pub stack_reserve: i32,
    pub cgb_only: bool,
    pub known_cgb: Option<i32>,
    pub const_scalar_in_rom: bool,
    pub abi_stack: bool,
    pub opt_level: i32,
    pub function_banks: BTreeMap<String, i32>,
    pub check_bounds: bool,
    pub check_slice_bounds: bool,
    pub check_stack: bool,
    pub check_mem_copy: bool,
    pub check_bank_calls: bool,
    pub rst: rst::Options,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            fixed_stack: false,
            stack_top: 0xdfff,
            stack_reserve: 0,
            cgb_only: false,
            known_cgb: None,
            const_scalar_in_rom: false,
            abi_stack: false,
            opt_level: 0,
            function_banks: BTreeMap::new(),
            check_bounds: false,
            check_slice_bounds: false,
            check_stack: false,
            check_mem_copy: false,
            check_bank_calls: false,
            rst: rst::Options::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolTag {
    Constant,
    ReadonlyData,
    Global,
    Local,
    StackParam,
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub tag: SymbolTag,
    pub value: i32,
    pub ty: CType,
    pub name: String,
    pub wram_bank: i32,
}
#[derive(Clone, Debug)]
pub struct FunctionInfo {
    pub parameters: Vec<Field>,
    pub parameter_symbols: Option<Vec<Symbol>>,
    pub return_type: CType,
    pub return_symbol: Option<Symbol>,
    pub return_pointer_symbol: Option<Symbol>,
    pub fast_call: bool,
    pub stack_call: bool,
    pub prototype: bool,
    pub must_check: bool,
    pub rom_bank: i32,
    pub fixed_bank: bool,
    pub placement_order: Option<i32>,
    pub inline: bool,
    pub body: Option<Expr>,
}
impl FunctionInfo {
    pub fn set_definition_parameters(&mut self, parameters: Vec<Field>) {
        if let Some(symbols) = &mut self.parameter_symbols {
            if symbols.len() == parameters.len() {
                for (sym, p) in symbols.iter_mut().zip(&parameters) {
                    sym.name = p.name.clone()
                }
            }
        }
        self.parameters = parameters
    }
}
#[derive(Clone, Debug)]
struct PendingConstant {
    origin: Expr,
    ty: CType,
    value: Expr,
}
#[derive(Clone, Debug)]
struct Scope {
    symbols: BTreeMap<String, Symbol>,
    saved: [i32; 3],
}
#[derive(Clone, Debug)]
pub struct Environment {
    pub options: Options,
    pub allocator: Allocator,
    pub aggregates: BTreeMap<String, Aggregate>,
    pub functions: BTreeMap<String, FunctionInfo>,
    pub readonly_banks: BTreeMap<String, i32>,
    pub readonly_alignments: BTreeMap<String, i32>,
    pub diagnostics: Vec<Diagnostic>,
    pub placements: Vec<Expr>,
    scopes: Vec<Scope>,
    pending_constants: BTreeMap<String, PendingConstant>,
    resolving_constants: BTreeSet<String>,
    manual_bank_warnings: BTreeSet<String>,
}
fn child(e: &Expr, index: usize) -> &Expr {
    e.child(index).expect("typed code-generation expression")
}
fn sub_type(ty: &CType) -> Option<&CType> {
    match &ty.kind {
        Kind::Pointer(t)
        | Kind::Array(t, _)
        | Kind::ArrayExpression(t, _)
        | Kind::Function(t, _) => Some(t),
        _ => None,
    }
}
fn integer(n: i32) -> CType {
    CType::simple(if n < 0 {
        Simple::Int16
    } else if n > 255 {
        Simple::UInt16
    } else {
        Simple::UInt8
    })
}
fn promotion(a: &CType, b: &CType) -> CType {
    CType::simple(if a.is_signed() || b.is_signed() {
        Simple::Int16
    } else {
        Simple::UInt16
    })
}
fn normalize(n: i32, ty: &CType) -> i32 {
    match &ty.kind {
        Kind::Simple(Simple::UInt8) => n & 255,
        Kind::Simple(Simple::Int8) => i32::from(n as i8),
        Kind::Simple(Simple::UInt16) | Kind::Pointer(_) | Kind::Enum(_) => n & 65535,
        Kind::Simple(Simple::Int16) => i32::from(n as i16),
        _ => n,
    }
}
impl Environment {
    pub fn new(options: Options) -> Self {
        let allocator = Allocator::new(
            options.fixed_stack,
            options.stack_top.wrapping_sub(options.stack_reserve),
        );
        Self {
            options,
            allocator,
            aggregates: BTreeMap::new(),
            functions: BTreeMap::new(),
            readonly_banks: BTreeMap::new(),
            readonly_alignments: BTreeMap::new(),
            diagnostics: Vec::new(),
            placements: Vec::new(),
            scopes: vec![Scope {
                symbols: BTreeMap::new(),
                saved: [0; 3],
            }],
            pending_constants: BTreeMap::new(),
            resolving_constants: BTreeSet::new(),
            manual_bank_warnings: BTreeSet::new(),
        }
    }
    fn diagnostic(&mut self, severity: Severity, source: &Position, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(severity, source.clone(), message.into(), 0))
    }
    pub fn error(&mut self, source: &Position, message: impl Into<String>) {
        self.diagnostic(Severity::Error, source, message)
    }
    pub fn warning(&mut self, source: &Position, message: impl Into<String>) {
        self.diagnostic(Severity::Warning, source, message)
    }
    pub fn static_assert_failure(
        &mut self,
        origin: &Expr,
        condition: &Expr,
        message: &str,
        raw: i32,
    ) {
        let ty = self.type_of(condition);
        let extra = format!(
            "expr='{}', value={raw} (raw={raw}), type={ty}",
            condition.show()
        );
        self.error(
            &origin.source,
            if message.is_empty() {
                format!("static_assert failed: {extra}")
            } else {
                format!("static_assert failed: {message} [{extra}]")
            },
        );
    }
    pub fn find(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|s| s.symbols.get(name))
    }
    pub fn find_required(&mut self, origin: &Expr, name: &str) -> Option<Symbol> {
        let symbol = self.find(name).cloned();
        if let Some(symbol) = &symbol {
            if symbol.tag == SymbolTag::Global
                && symbol.wram_bank > 1
                && self.manual_bank_warnings.insert(name.into())
            {
                self.warning(&origin.source,format!("symbol '{name}' is placed in WRAMX bank {}; compiler does not auto-switch SVBK in MVP",symbol.wram_bank))
            }
        } else {
            self.error(&origin.source, format!("Undefined symbol: {name}"))
        }
        symbol
    }
    pub fn declare(&mut self, origin: &Expr, symbol: Symbol) -> Symbol {
        let scope = self.scopes.last_mut().unwrap();
        if scope.symbols.contains_key(&symbol.name) {
            self.error(&origin.source, format!("Symbol redefined: {}", symbol.name));
            return symbol;
        }
        self.scopes
            .last_mut()
            .unwrap()
            .symbols
            .insert(symbol.name.clone(), symbol.clone());
        symbol
    }
    pub fn begin_scope(&mut self) {
        self.scopes.push(Scope {
            symbols: BTreeMap::new(),
            saved: self.allocator.snapshot(),
        })
    }
    pub fn end_scope(&mut self) {
        if self.scopes.len() > 1 {
            let scope = self.scopes.pop().unwrap();
            self.allocator.restore(scope.saved)
        }
    }
    pub fn add_constant(&mut self, origin: &Expr, ty: &CType, name: &str, value: &Expr) {
        if self.pending_constants.contains_key(name) {
            self.error(&origin.source, format!("Symbol redefined: {name}"));
            return;
        }
        self.pending_constants.insert(
            name.into(),
            PendingConstant {
                origin: origin.clone(),
                ty: ty.clone(),
                value: value.clone(),
            },
        );
    }
    pub fn ensure_constant(&mut self, name: &str, origin: &Expr) -> bool {
        if self
            .find(name)
            .is_some_and(|s| s.tag == SymbolTag::Constant)
        {
            return true;
        }
        let Some(info) = self.pending_constants.get(name).cloned() else {
            return false;
        };
        if !self.resolving_constants.insert(name.into()) {
            self.error(
                &origin.source,
                format!("cyclic constant definition: {name}"),
            );
            return false;
        }
        let value = self.constant(&info.value).value;
        if self.find(name).is_none() {
            self.declare(
                &info.origin,
                Symbol {
                    tag: SymbolTag::Constant,
                    value,
                    ty: info.ty,
                    name: name.into(),
                    wram_bank: 0,
                },
            );
        }
        self.resolving_constants.remove(name);
        true
    }
    pub fn opaque(&mut self, name: &str, layout: AggregateLayout) {
        self.aggregates.entry(name.into()).or_insert(Aggregate {
            layout,
            total_size: -1,
            alignment: 1,
            is_packed: false,
            fields: Vec::new(),
        });
    }
    pub fn define_aggregate(
        &mut self,
        origin: &Expr,
        name: &str,
        parsed: &[Field],
        layout: AggregateLayout,
        packed: bool,
        forced_align: i32,
    ) {
        let (mut offset, mut max_size, mut align) = (0i32, 1i32, 1i32);
        let mut fields = Vec::new();
        for f in parsed {
            let ty = self.resolve_dimension(&f.ty);
            let size = self.size(origin, &ty);
            let fa = if packed { 1 } else { self.natural_align(&ty) };
            align = align.max(fa);
            let field_offset = if layout == AggregateLayout::Struct {
                offset = allocation::align_up(offset, fa);
                let result = offset;
                offset = offset.wrapping_add(size);
                result
            } else {
                0
            };
            max_size = max_size.max(size);
            fields.push(Field {
                ty,
                name: f.name.clone(),
                offset: field_offset,
            });
        }
        align = align.max(forced_align);
        if layout == AggregateLayout::Union {
            offset = max_size
        }
        let info = Aggregate {
            layout,
            total_size: allocation::align_up(offset, align),
            alignment: align,
            is_packed: packed,
            fields,
        };
        if self.aggregates.get(name).is_some_and(|a| a.total_size >= 0) {
            self.error(
                &origin.source,
                format!("duplicate struct/union definition: {name}"),
            );
            return;
        }
        self.aggregates.insert(name.into(), info);
    }
    pub fn natural_align(&self, ty: &CType) -> i32 {
        if ty.forced_align > 0 {
            return ty.forced_align;
        }
        match &ty.kind {
            Kind::Array(s, _) | Kind::ArrayExpression(s, _) => self.natural_align(s),
            Kind::Pointer(_) | Kind::Enum(_) | Kind::Simple(Simple::UInt16 | Simple::Int16) => 2,
            Kind::Struct(name) | Kind::Union(name) => self
                .aggregates
                .get(name)
                .filter(|a| a.total_size >= 0)
                .map_or(1, |a| a.alignment.max(1)),
            _ => 1,
        }
    }
    pub fn size(&mut self, origin: &Expr, ty: &CType) -> i32 {
        match &ty.kind {
            Kind::Pointer(_)
            | Kind::Function(_, _)
            | Kind::Enum(_)
            | Kind::Simple(Simple::UInt16 | Simple::Int16) => 2,
            Kind::Array(s, n) => {
                let size = self.size(origin, s);
                let n = if *n <= 0 {
                    self.error(
                        &origin.source,
                        format!("array dimension is not a constant positive integer: {ty}"),
                    );
                    1
                } else {
                    *n
                };
                size.wrapping_mul(n)
            }
            Kind::ArrayExpression(s, n) => {
                let size = self.size(origin, s);
                let mut n = self.constant(n).value;
                if n <= 0 {
                    self.error(
                        &origin.source,
                        format!("array dimension is not a constant positive integer: {ty}"),
                    );
                    n = 1
                }
                size.wrapping_mul(n)
            }
            Kind::Struct(name) | Kind::Union(name) => {
                if let Some(size) = self
                    .aggregates
                    .get(name)
                    .map(|a| a.total_size)
                    .filter(|n| *n >= 0)
                {
                    size
                } else {
                    self.error(
                        &origin.source,
                        format!("incomplete struct/union type: {ty}"),
                    );
                    1
                }
            }
            _ => 1,
        }
    }
    pub fn resolve_dimension(&mut self, ty: &CType) -> CType {
        if let Kind::ArrayExpression(s, n) = &ty.kind {
            CType::array((**s).clone(), self.constant(n).value)
        } else {
            ty.clone()
        }
    }
    pub fn offset_of(&mut self, origin: &Expr, ty: &CType, path: &str) -> i32 {
        let mut current = ty.without_const();
        if !current.is_aggregate() && current.is_pointer() {
            current = sub_type(&current).unwrap().clone()
        }
        if !current.is_aggregate() {
            self.error(
                &origin.source,
                format!("offsetof requires a struct/union type, got: {ty}"),
            );
            return 0;
        }
        if path.is_empty() {
            self.error(&origin.source, "offsetof requires a member name");
            return 0;
        }
        let parts = path
            .split('.')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let mut offset = 0i32;
        for (i, part) in parts.iter().enumerate() {
            let name = match &current.kind {
                Kind::Struct(n) | Kind::Union(n) => n,
                _ => unreachable!(),
            };
            let field = self
                .aggregates
                .get(name)
                .and_then(|a| a.fields.iter().find(|f| f.name == *part))
                .cloned();
            let Some(field) = field else {
                self.error(
                    &origin.source,
                    format!("offsetof: no such field '{part}' in {current}"),
                );
                return 0;
            };
            offset = offset.wrapping_add(field.offset);
            if i + 1 < parts.len() {
                if field.ty.is_pointer() {
                    self.error(&origin.source,format!("offsetof: field '{part}' is a pointer; nested member requires an embedded struct/union, not pointer"));
                    return 0;
                }
                if !field.ty.is_aggregate() {
                    self.error(&origin.source,format!("offsetof: field '{part}' is not a struct/union; cannot access nested member"));
                    return 0;
                }
                current = field.ty;
            }
        }
        offset
    }
    pub fn allocate_preferred(&mut self, size: i32, align: i32, regions: &[usize]) -> i32 {
        for &region in regions {
            if let Some(address) = self.allocator.allocate(region, size, align) {
                return address;
            }
        }
        self.error(
            &Position::default(),
            format!("Out of memory: could not allocate {size} bytes in any preferred region."),
        );
        0
    }
    pub fn declare_local(&mut self, origin: &Expr, ty: &CType, name: &str, hot: bool) -> Symbol {
        let size = self.size(origin, ty);
        let align = self.natural_align(ty).max(1);
        let address =
            self.allocate_preferred(size, align, if hot { &[0, 1, 2] } else { &[1, 2, 0] });
        self.declare(
            origin,
            Symbol {
                tag: SymbolTag::Local,
                value: address,
                ty: ty.clone(),
                name: name.into(),
                wram_bank: 0,
            },
        )
    }
    pub fn declare_global(
        &mut self,
        origin: &Expr,
        region: Region,
        ty: &CType,
        name: &str,
        pragma_align: i32,
    ) -> Symbol {
        let size = self.size(origin, ty);
        let alignment = self.natural_align(ty).max(pragma_align).max(1);
        let mut actual_region = region;
        let requested = match region {
            Region::HighMem => Some(0),
            Region::Oam => Some(3),
            Region::Wram0 => Some(1),
            Region::WramX(bank) => Some(Allocator::bank_region(i32::from(bank))),
            _ => None,
        };
        let address = if let Region::Fixed(address) = region {
            if let Err(message) = self.allocator.reserve_fixed(name, address, size) {
                self.error(&origin.source, message)
            }
            if self.options.stack_reserve > 0 {
                let end = address.wrapping_add(size.saturating_sub(1).max(0));
                let reserved_begin = self.options.stack_top - self.options.stack_reserve + 1;
                if address <= self.options.stack_top && end >= reserved_begin {
                    self.error(&origin.source,format!("fixed allocation '{name}' at {:04X}-{:04X} overlaps reserved stack range {:04X}-{:04X}",address&65535,end&65535,reserved_begin&65535,self.options.stack_top&65535))
                }
            }
            address
        } else if let Some(region_index) = requested {
            if let Region::WramX(bank) = region {
                if bank > 1 && !self.options.cgb_only {
                    self.error(
                        &origin.source,
                        "banked WRAM requires #pragma rom_cgb cgb_only or --cgb=cgb_only",
                    )
                }
            }
            if let Some(address) = self.allocator.allocate(region_index, size, alignment) {
                address
            } else if let Region::WramX(bank) = region {
                if bank > 0 {
                    self.error(
                        &origin.source,
                        format!("WRAMX bank {bank} overflow while placing '{name}' ({size} bytes)"),
                    );
                    self.allocator.regions[region_index].bottom
                } else {
                    let name = self.allocator.regions[region_index].name.clone();
                    self.error(&Position::default(), format!("Out of memory in {name}"));
                    -1
                }
            } else {
                let name = self.allocator.regions[region_index].name.clone();
                self.error(&Position::default(), format!("Out of memory in {name}"));
                -1
            }
        } else {
            let address = self.allocate_preferred(size, alignment, &[1, 2]);
            actual_region = infer_region(address);
            address
        };
        self.placements.push(Expr::new(
            t::VARIABLE,
            vec![
                Arg::from(name),
                Arg::Int(address),
                Arg::Int(size),
                Arg::Region(actual_region),
            ],
        ));
        let bank = if let Region::WramX(bank) = actual_region {
            i32::from(bank)
        } else {
            0
        };
        self.declare(
            origin,
            Symbol {
                tag: SymbolTag::Global,
                value: address,
                ty: ty.clone(),
                name: name.into(),
                wram_bank: bank,
            },
        )
    }
}
pub fn memory_operand(address: i32) -> Operand {
    if address >= 0xff00 {
        Operand::integer(address & 255, crate::asm::AddressMode::HighMem)
    } else {
        Operand::integer(address, crate::asm::AddressMode::Absolute)
    }
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
