use super::*;
use crate::asm::{AddressMode, Modifier};
pub fn declaration(e: &Expr) -> Option<(&CType, &str, Vec<Expr>)> {
    if !e.is(t::READONLY_DATA) {
        return None;
    }
    let ty = e.ty(1)?;
    let name = e.text(2)?;
    let values = match e.args.get(3)? {
        Arg::Ints(ns) => ns
            .iter()
            .map(|n| Expr::new(t::INTEGER, vec![Arg::Int(*n)]).with_source(e.source.clone()))
            .collect(),
        Arg::Exprs(es) => es.clone(),
        _ => return None,
    };
    Some((ty, name, values))
}
impl Environment {
    pub fn readonly_alignment(&mut self, origin: &Expr, ty: &CType, pragma_align: i32) -> i32 {
        let mut align = ty.forced_align.max(pragma_align);
        if align <= 0 {
            align = self.natural_align(ty).max(1)
        }
        if self.options.opt_level >= 1 && align < 256 && pragma_align == 0 && ty.forced_align == 0 {
            if let Kind::Array(elem, 256) = &ty.kind {
                if self.size(origin, elem) == 1 {
                    align = 256
                }
            }
        }
        align
    }
    pub fn register_readonly(
        &mut self,
        origin: &Expr,
        ty: &CType,
        name: &str,
        align: i32,
        bank: i32,
    ) {
        if self.find(name).is_none() {
            self.declare(
                origin,
                Symbol {
                    tag: SymbolTag::ReadonlyData,
                    value: 0,
                    ty: ty.clone(),
                    name: name.into(),
                    wram_bank: 0,
                },
            );
        }
        let align = self.readonly_alignment(origin, ty, align);
        self.readonly_alignments.insert(name.into(), align);
        self.readonly_banks.insert(name.into(), bank);
    }
    pub fn declare_readonly(
        &mut self,
        origin: &Expr,
        ty: &CType,
        name: &str,
        values: &[Expr],
        align: i32,
        section: Option<&str>,
    ) {
        if let Some(section) = section.filter(|s| !s.is_empty()) {
            self.placements
                .push(Expr::new(t::SECTION, vec![Arg::from(section)]))
        }
        let align = self.readonly_alignment(origin, ty, align);
        self.readonly_alignments.insert(name.into(), align);
        if align > 1 {
            self.placements
                .push(Expr::new(t::ALIGN, vec![Arg::Int(align)]))
        }
        let mut relocations = Vec::new();
        let bytes = self.encode_readonly(origin, ty, values, &mut relocations, 0);
        if self.find(name).is_none() {
            self.declare(
                origin,
                Symbol {
                    tag: SymbolTag::ReadonlyData,
                    value: 0,
                    ty: ty.clone(),
                    name: name.into(),
                    wram_bank: 0,
                },
            );
        }
        let mut args = vec![Arg::from(name), Arg::Bytes(bytes)];
        if !relocations.is_empty() {
            args.push(Arg::Exprs(relocations))
        }
        self.placements.push(Expr::new(t::READONLY_DATA, args));
    }
    pub fn encode_readonly(
        &mut self,
        origin: &Expr,
        ty: &CType,
        values: &[Expr],
        relocations: &mut Vec<Expr>,
        offset: i32,
    ) -> Vec<u8> {
        if ty.is_array() {
            return self.encode_array(origin, ty, values, relocations, offset);
        }
        let size = self.size(origin, ty).max(1);
        if values.is_empty() {
            return vec![0; size as usize];
        }
        if values.len() > 1 {
            self.error(&origin.source, "too many initializers for readonly data")
        }
        self.encode_initializer(origin, ty, &values[0], relocations, offset)
    }
    fn encode_initializer(
        &mut self,
        origin: &Expr,
        ty: &CType,
        value: &Expr,
        relocations: &mut Vec<Expr>,
        offset: i32,
    ) -> Vec<u8> {
        if value.is(t::EMPTY) {
            return vec![0; self.size(origin, ty).max(1) as usize];
        }
        if ty.is_array() {
            let items = if value.is(t::SEQUENCE) {
                value.children().into_iter().cloned().collect()
            } else {
                vec![value.clone()]
            };
            return self.encode_array(origin, ty, &items, relocations, offset);
        }
        if ty.is_aggregate() {
            return self.encode_aggregate(origin, ty, value, relocations, offset);
        }
        let items = value.children();
        let value = if value.is(t::SEQUENCE) {
            if items.is_empty() {
                return vec![0; self.size(origin, ty).max(1) as usize];
            }
            if items.len() > 1 {
                self.error(
                    &origin.source,
                    "too many initializers for scalar readonly data",
                )
            }
            items[0]
        } else {
            value
        };
        let size = self.size(origin, ty).max(1);
        if size > 4 {
            self.error(
                &origin.source,
                format!(
                    "readonly data scalar initializer supports sizes up to 4 bytes (got {size})"
                ),
            )
        }
        if ty.is_pointer() && size == 2 {
            if let Some(address) = self.readonly_pointer(value, false) {
                relocations.push(
                    Expr::new(t::WORD, vec![Arg::Int(offset), Arg::Operand(address)])
                        .with_source(value.source.clone()),
                );
                return vec![0; size as usize];
            }
        }
        let value = self.constant(value).value as u32;
        (0..size)
            .map(|i| ((value.wrapping_shr((i * 8) as u32)) & 255) as u8)
            .collect()
    }
    fn encode_array(
        &mut self,
        origin: &Expr,
        ty: &CType,
        items: &[Expr],
        relocations: &mut Vec<Expr>,
        offset: i32,
    ) -> Vec<u8> {
        let elem = sub_type(ty).unwrap();
        let stride = self.size(origin, elem).max(1);
        let count = match &ty.kind {
            Kind::Array(_, n) => *n,
            Kind::ArrayExpression(_, n) => self.constant(n).value,
            _ => 1,
        }
        .max(0);
        if items.len() > count as usize {
            self.error(
                &origin.source,
                format!(
                    "too many initializers for readonly array (got {}, declared {count})",
                    items.len()
                ),
            )
        }
        let mut bytes = vec![0; count as usize * stride as usize];
        for (i, value) in items.iter().take(count as usize).enumerate() {
            if value.is(t::EMPTY) {
                continue;
            }
            let elem_bytes = self.encode_initializer(
                origin,
                elem,
                value,
                relocations,
                offset.wrapping_add((i as i32).wrapping_mul(stride)),
            );
            if elem_bytes.len() != stride as usize {
                self.error(
                    &origin.source,
                    format!("readonly array initializer size mismatch for element {i}"),
                )
            }
            let n = elem_bytes.len().min(stride as usize);
            let at = i * stride as usize;
            bytes[at..at + n].copy_from_slice(&elem_bytes[..n]);
        }
        bytes
    }
    fn encode_aggregate(
        &mut self,
        origin: &Expr,
        ty: &CType,
        value: &Expr,
        relocations: &mut Vec<Expr>,
        offset: i32,
    ) -> Vec<u8> {
        let size = self.size(origin, ty).max(1);
        let mut bytes = vec![0; size as usize];
        if !value.is(t::SEQUENCE) {
            if self.constant(value).value != 0 {
                self.error(
                    &origin.source,
                    "aggregate readonly initializer requires braces",
                )
            }
            return bytes;
        }
        let name = match &ty.kind {
            Kind::Struct(n) | Kind::Union(n) => n,
            _ => unreachable!(),
        };
        let Some(info) = self.aggregates.get(name).cloned() else {
            return bytes;
        };
        let items = value.children();
        if items.len() > info.fields.len() {
            self.error(&origin.source, format!("too many initializers for {ty}"))
        }
        for (value, field) in items.into_iter().zip(info.fields) {
            if value.is(t::EMPTY) {
                continue;
            }
            let field_bytes = self.encode_initializer(
                origin,
                &field.ty,
                value,
                relocations,
                offset.wrapping_add(field.offset),
            );
            let length = if info.layout == AggregateLayout::Union {
                field_bytes.len().min(size as usize)
            } else {
                let expected = self.size(origin, &field.ty).max(1) as usize;
                if field_bytes.len() != expected {
                    self.error(
                        &origin.source,
                        format!(
                            "readonly struct initializer size mismatch for field {}",
                            field.name
                        ),
                    )
                }
                expected.min(field_bytes.len())
            };
            let at = field.offset.max(0) as usize;
            let length = length.min(bytes.len().saturating_sub(at));
            if at <= bytes.len() {
                bytes[at..at + length].copy_from_slice(&field_bytes[..length])
            }
            if info.layout == AggregateLayout::Union {
                break;
            }
        }
        bytes
    }
    pub fn readonly_pointer(&mut self, e: &Expr, address_of: bool) -> Option<Operand> {
        if !address_of && e.is(t::CAST) {
            return if e.ty(1)?.is_pointer() {
                self.readonly_pointer(child(e, 2), false)
            } else {
                None
            };
        }
        if !address_of && e.is(t::ADDRESS_OF) {
            return self.readonly_pointer(child(e, 1), true);
        }
        if e.is(t::NAME) {
            let name = e.text(1)?;
            let symbol = self.find(name)?;
            if !address_of && !symbol.ty.is_array() {
                return None;
            }
            return match symbol.tag {
                SymbolTag::ReadonlyData => Some(Operand::symbol(name, AddressMode::Immediate)),
                SymbolTag::Global => Some(Operand::integer(symbol.value, AddressMode::Immediate)),
                _ => None,
            };
        }
        if address_of && e.is(t::INDEX) {
            let array = child(e, 1);
            let base = self.readonly_pointer(array, false)?;
            let ty = self.type_of(array);
            if !ty.is_pointer() && !ty.is_array() {
                return None;
            }
            let index = self.constant(child(e, 2)).value;
            let size = self.size(e, sub_type(&ty).unwrap());
            return Some(Operand {
                offset: base.offset.wrapping_add(index.wrapping_mul(size)),
                mode: AddressMode::Immediate,
                modifier: Modifier::None,
                ..base
            });
        }
        if address_of && e.is(t::FIELD) {
            let owner = child(e, 1);
            let base = self.readonly_pointer(owner, true)?;
            let field = self.field(e, owner, e.text(2)?)?;
            return Some(Operand {
                offset: base.offset.wrapping_add(field.offset),
                mode: AddressMode::Immediate,
                modifier: Modifier::None,
                ..base
            });
        }
        if !address_of && matches!(e.tag(), Some(t::ADD | t::SUBTRACT)) {
            let subtract = e.is(t::SUBTRACT);
            let (mut left, mut right) = (child(e, 1), child(e, 2));
            let mut ty = self.type_of(left);
            if !subtract && !ty.is_pointer() && !ty.is_array() {
                std::mem::swap(&mut left, &mut right);
                ty = self.type_of(left)
            }
            if ty.is_pointer() || ty.is_array() {
                let base = self.readonly_pointer(left, false)?;
                let index = self.constant(right).value;
                let size = self.size(e, sub_type(&ty).unwrap());
                let mut offset = index.wrapping_mul(size);
                if subtract {
                    offset = offset.wrapping_neg()
                }
                return Some(Operand {
                    offset: base.offset.wrapping_add(offset),
                    mode: AddressMode::Immediate,
                    modifier: Modifier::None,
                    ..base
                });
            }
        }
        None
    }
}
