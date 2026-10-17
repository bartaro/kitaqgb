pub mod camera;
pub mod huc1;
pub mod huc3;
pub mod mbc1;
pub mod mbc2;
pub mod mbc3;
pub mod mbc5;
pub mod mbc6;
pub mod mbc7;
pub mod mmm01;
pub mod rom_only;
pub mod tama5;

pub use camera::PocketCamera;
pub use huc1::HuC1;
pub use huc3::HuC3;
pub use mbc1::Mbc1;
pub use mbc2::Mbc2;
pub use mbc3::Mbc3;
pub use mbc5::Mbc5;
pub use mbc6::Mbc6;
pub use mbc7::Mbc7;
pub use mmm01::Mmm01;
pub use rom_only::RomOnly;
pub use tama5::Tama5;

use serde::{Deserialize, Serialize};

pub(crate) fn read_rom_bank(rom: &[u8], bank: usize, bank_size: usize, offset: usize) -> u8 {
    if rom.is_empty() || bank_size == 0 {
        return 0xFF;
    }
    let bank_count = rom.len().div_ceil(bank_size).max(1);
    let base = (bank % bank_count).saturating_mul(bank_size);
    rom.get(base.saturating_add(offset))
        .copied()
        .unwrap_or(0xFF)
}

pub(crate) fn read_ram_bank(ram: &[u8], bank: usize, bank_size: usize, offset: usize) -> u8 {
    if ram.is_empty() || bank_size == 0 {
        return 0xFF;
    }
    let bank_count = ram.len().div_ceil(bank_size).max(1);
    let base = (bank % bank_count).saturating_mul(bank_size);
    ram.get(base.saturating_add(offset))
        .copied()
        .unwrap_or(0xFF)
}

pub(crate) fn write_ram_bank(
    ram: &mut [u8],
    bank: usize,
    bank_size: usize,
    offset: usize,
    value: u8,
) {
    if ram.is_empty() || bank_size == 0 {
        return;
    }
    let bank_count = ram.len().div_ceil(bank_size).max(1);
    let base = (bank % bank_count).saturating_mul(bank_size);
    if let Some(byte) = ram.get_mut(base.saturating_add(offset)) {
        *byte = value;
    }
}

#[cfg(test)]
mod tests {
    use super::{read_ram_bank, read_rom_bank, write_ram_bank};

    #[test]
    fn rom_bank_reads_wrap_when_bank_exceeds_rom_size() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x0000] = 0x12;
        rom[0x4000] = 0x34;
        assert_eq!(read_rom_bank(&rom, 0, 0x4000, 0), 0x12);
        assert_eq!(read_rom_bank(&rom, 1, 0x4000, 0), 0x34);
        assert_eq!(read_rom_bank(&rom, 2, 0x4000, 0), 0x12);
        assert_eq!(read_rom_bank(&rom, 3, 0x4000, 0), 0x34);
    }

    #[test]
    fn ram_bank_reads_and_writes_wrap_when_bank_exceeds_ram_size() {
        let mut ram = vec![0u8; 0x4000];
        write_ram_bank(&mut ram, 3, 0x2000, 0x0010, 0xAB);
        assert_eq!(read_ram_bank(&ram, 1, 0x2000, 0x0010), 0xAB);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mbc {
    RomOnly(RomOnly),
    Mbc1(Mbc1),
    Mbc2(Mbc2),
    Mmm01(Mmm01),
    Mbc3(Mbc3),
    Mbc5(Mbc5),
    Mbc6(Mbc6),
    Mbc7(Mbc7),
    PocketCamera(PocketCamera),
    Tama5(Tama5),
    HuC3(HuC3),
    HuC1(HuC1),
}

impl Mbc {
    pub fn ram_access_enabled(&self) -> bool {
        match self {
            Self::RomOnly(_) => true,
            Self::Mbc1(m) => m.ram_enabled,
            Self::Mbc2(m) => m.ram_enabled(),
            Self::Mmm01(m) => m.inner.ram_enabled,
            Self::Mbc3(m) => m.ram_enabled,
            Self::Mbc5(m) => m.ram_enabled,
            Self::Mbc6(m) => m.ram_enabled,
            Self::Mbc7(m) => m.ram_enable_1 && m.ram_enable_2,
            Self::PocketCamera(m) => m.ram_write_enabled && !m.camera_selected,
            Self::Tama5(m) => m.ram_enabled,
            Self::HuC3(m) => (m.mode & 0x0F) == 0x0A,
            Self::HuC1(m) => !m.ir_mode && m.inner.ram_enabled,
        }
    }

    pub fn current_rom_bank(&self) -> u16 {
        match self {
            Self::RomOnly(m) => m.current_rom_bank(),
            Self::Mbc1(m) => m.current_rom_bank(),
            Self::Mbc2(m) => m.current_rom_bank(),
            Self::Mmm01(m) => m.current_rom_bank(),
            Self::Mbc3(m) => m.current_rom_bank(),
            Self::Mbc5(m) => m.current_rom_bank(),
            Self::Mbc6(m) => m.current_rom_bank(),
            Self::Mbc7(m) => m.current_rom_bank(),
            Self::PocketCamera(m) => m.current_rom_bank(),
            Self::Tama5(m) => m.current_rom_bank(),
            Self::HuC3(m) => m.current_rom_bank(),
            Self::HuC1(m) => m.current_rom_bank(),
        }
    }

    pub fn current_ram_bank(&self) -> u16 {
        match self {
            Self::RomOnly(m) => m.current_ram_bank(),
            Self::Mbc1(m) => m.current_ram_bank(),
            Self::Mbc2(m) => m.current_ram_bank(),
            Self::Mmm01(m) => m.current_ram_bank(),
            Self::Mbc3(m) => m.current_ram_bank(),
            Self::Mbc5(m) => m.current_ram_bank(),
            Self::Mbc6(m) => m.current_ram_bank(),
            Self::Mbc7(m) => m.current_ram_bank(),
            Self::PocketCamera(m) => m.current_ram_bank(),
            Self::Tama5(m) => m.current_ram_bank(),
            Self::HuC3(m) => m.current_ram_bank(),
            Self::HuC1(m) => m.current_ram_bank(),
        }
    }

    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match self {
            Self::RomOnly(m) => m.read_rom(rom, addr),
            Self::Mbc1(m) => m.read_rom(rom, addr),
            Self::Mbc2(m) => m.read_rom(rom, addr),
            Self::Mmm01(m) => m.read_rom(rom, addr),
            Self::Mbc3(m) => m.read_rom(rom, addr),
            Self::Mbc5(m) => m.read_rom(rom, addr),
            Self::Mbc6(m) => m.read_rom(rom, addr),
            Self::Mbc7(m) => m.read_rom(rom, addr),
            Self::PocketCamera(m) => m.read_rom(rom, addr),
            Self::Tama5(m) => m.read_rom(rom, addr),
            Self::HuC3(m) => m.read_rom(rom, addr),
            Self::HuC1(m) => m.read_rom(rom, addr),
        }
    }

    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        match self {
            Self::RomOnly(m) => m.read_ram(ram, addr),
            Self::Mbc1(m) => m.read_ram(ram, addr),
            Self::Mbc2(m) => m.read_ram(ram, addr),
            Self::Mmm01(m) => m.read_ram(ram, addr),
            Self::Mbc3(m) => m.read_ram(ram, addr),
            Self::Mbc5(m) => m.read_ram(ram, addr),
            Self::Mbc6(m) => m.read_ram(ram, addr),
            Self::Mbc7(m) => m.read_ram(ram, addr),
            Self::PocketCamera(m) => m.read_ram(ram, addr),
            Self::Tama5(m) => m.read_ram(ram, addr),
            Self::HuC3(m) => m.read_ram(ram, addr),
            Self::HuC1(m) => m.read_ram(ram, addr),
        }
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        match self {
            Self::RomOnly(m) => m.write_ram(ram, addr, value),
            Self::Mbc1(m) => m.write_ram(ram, addr, value),
            Self::Mbc2(m) => m.write_ram(ram, addr, value),
            Self::Mmm01(m) => m.write_ram(ram, addr, value),
            Self::Mbc3(m) => m.write_ram(ram, addr, value),
            Self::Mbc5(m) => m.write_ram(ram, addr, value),
            Self::Mbc6(m) => m.write_ram(ram, addr, value),
            Self::Mbc7(m) => m.write_ram(ram, addr, value),
            Self::PocketCamera(m) => m.write_ram(ram, addr, value),
            Self::Tama5(m) => m.write_ram(ram, addr, value),
            Self::HuC3(m) => m.write_ram(ram, addr, value),
            Self::HuC1(m) => m.write_ram(ram, addr, value),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match self {
            Self::RomOnly(m) => m.write(addr, value),
            Self::Mbc1(m) => m.write(addr, value),
            Self::Mbc2(m) => m.write(addr, value),
            Self::Mmm01(m) => m.write(addr, value),
            Self::Mbc3(m) => m.write(addr, value),
            Self::Mbc5(m) => m.write(addr, value),
            Self::Mbc6(m) => m.write(addr, value),
            Self::Mbc7(m) => m.write(addr, value),
            Self::PocketCamera(m) => m.write(addr, value),
            Self::Tama5(m) => m.write(addr, value),
            Self::HuC3(m) => m.write(addr, value),
            Self::HuC1(m) => m.write(addr, value),
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        match self {
            Self::RomOnly(_) => {}
            Self::Mbc1(_) => {}
            Self::Mbc2(_) => {}
            Self::Mmm01(_) => {}
            Self::Mbc3(m) => m.tick(cycles),
            Self::Mbc5(_) => {}
            Self::Mbc6(m) => m.tick(cycles),
            Self::Mbc7(m) => m.tick(cycles),
            Self::PocketCamera(m) => m.tick(cycles),
            Self::Tama5(m) => m.tick(cycles),
            Self::HuC3(m) => m.tick(cycles),
            Self::HuC1(_) => {}
        }
    }
}
