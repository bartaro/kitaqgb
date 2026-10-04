use std::collections::BTreeMap;
use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressMode {
    Implicit,
    Immediate,
    Immediate16,
    HighMem,
    HighMemX,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    Relative,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Modifier {
    #[default]
    None,
    LowByte,
    HighByte,
    Bank,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operand {
    pub base: Option<String>,
    pub offset: i32,
    pub mode: AddressMode,
    pub modifier: Modifier,
    pub comment: Option<String>,
}
impl Operand {
    pub fn implicit() -> Self {
        Self::integer(0, AddressMode::Implicit)
    }
    pub fn integer(offset: i32, mode: AddressMode) -> Self {
        Self {
            base: None,
            offset,
            mode,
            modifier: Modifier::None,
            comment: None,
        }
    }
    pub fn symbol(base: impl Into<String>, mode: AddressMode) -> Self {
        Self {
            base: Some(base.into()),
            ..Self::integer(0, mode)
        }
    }
    pub fn with_mode(&self, mode: AddressMode) -> Self {
        Self {
            mode,
            ..self.clone()
        }
    }
    pub fn with_modifier(&self, modifier: Modifier) -> Self {
        Self {
            modifier,
            ..self.clone()
        }
    }
    pub fn with_comment(&self, comment: impl Into<String>) -> Self {
        Self {
            comment: Some(comment.into()),
            ..self.clone()
        }
    }
    pub fn replace_base(&self, value: i32) -> Result<Self, String> {
        if self.base.is_none() {
            return Err("this operand has no base symbol".into());
        }
        Ok(Self {
            base: None,
            offset: value.wrapping_add(self.offset),
            ..self.clone()
        })
    }
    pub fn resolve(
        &self,
        symbols: &BTreeMap<String, Symbol>,
        location: i32,
        relative: bool,
    ) -> Option<i32> {
        let symbol = self
            .base
            .as_ref()
            .map(|b| symbols.get(b))
            .transpose_option()?;
        let n = symbol.map_or(0, |s| s.value).wrapping_add(self.offset);
        if self.modifier == Modifier::Bank {
            return Some(
                match symbol.map(|s| s.region) {
                    Some(Region::Rom) | None => n >> 14,
                    Some(Region::WramX(b)) => {
                        if b > 0 {
                            b as i32
                        } else {
                            i32::from((0xd000..=0xdfff).contains(&n))
                        }
                    }
                    _ => 0,
                } & 255,
            );
        }
        let n = if relative {
            n.wrapping_sub(location + 1)
        } else if symbol.is_some_and(|s| s.region == Region::Rom) {
            cpu_address(n)
        } else {
            n
        };
        Some(match self.modifier {
            Modifier::LowByte => n & 255,
            Modifier::HighByte => (n >> 8) & 255,
            _ => n,
        })
    }
}
trait TransposeOption<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}
impl<T> TransposeOption<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(None) => None,
            Some(Some(v)) => Some(Some(v)),
        }
    }
}
pub fn format_integer(n: i32) -> String {
    if n < 256 {
        n.to_string()
    } else {
        format!("${n:X}")
    }
}
impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match &self.base {
            None => format_integer(self.offset),
            Some(b) if self.offset == 0 => b.clone(),
            Some(b) => format!("{b}+{}", format_integer(self.offset)),
        };
        let s = match self.modifier {
            Modifier::None => s,
            Modifier::LowByte => format!("LOW({s})"),
            Modifier::HighByte => format!("HIGH({s})"),
            Modifier::Bank => format!("BANK({s})"),
        };
        match self.mode {
            AddressMode::Absolute => write!(f, "[{s}]"),
            AddressMode::Indirect => write!(f, "[HL]"),
            AddressMode::HighMemX => write!(f, "[{s}+X]"),
            _ => write!(f, "{s}"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    HighMem,
    Oam,
    Ram,
    Wram0,
    WramX(u8),
    Rom,
    Fixed(i32),
}
impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HighMem => write!(f, "HighMem"),
            Self::Oam => write!(f, "Oam"),
            Self::Ram => write!(f, "Ram"),
            Self::Wram0 => write!(f, "Wram0"),
            Self::WramX(0) => write!(f, "WramX"),
            Self::WramX(b) => write!(f, "WramX[{b}]"),
            Self::Rom => write!(f, "ProgramRom"),
            Self::Fixed(a) => write!(f, "Fixed=${a:04X}"),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub value: i32,
    pub is_label: bool,
    pub region: Region,
    pub section: String,
}
pub fn cpu_address(offset: i32) -> i32 {
    if offset < 0x4000 {
        offset
    } else {
        0x4000 + (offset & 0x3fff)
    }
}
