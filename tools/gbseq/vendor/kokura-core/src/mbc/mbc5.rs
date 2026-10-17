use serde::{Deserialize, Serialize};

use crate::mbc::{read_ram_bank, read_rom_bank, write_ram_bank};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mbc5 {
    pub rom_bank_low: u8,
    pub rom_bank_high: u8,
    pub ram_bank: u8,
    pub ram_enabled: bool,
    pub has_rumble: bool,
    pub rumble_enabled: bool,
}

impl Default for Mbc5 {
    fn default() -> Self {
        Self {
            rom_bank_low: 1,
            rom_bank_high: 0,
            ram_bank: 0,
            ram_enabled: false,
            has_rumble: false,
            rumble_enabled: false,
        }
    }
}

impl Mbc5 {
    pub fn with_rumble(has_rumble: bool) -> Self {
        Self {
            has_rumble,
            ..Self::default()
        }
    }

    pub fn current_rom_bank(&self) -> u16 {
        (self.rom_bank_low as u16) | (((self.rom_bank_high & 0x01) as u16) << 8)
    }

    pub fn current_ram_bank(&self) -> u16 {
        if self.has_rumble {
            (self.ram_bank & 0x07) as u16
        } else {
            (self.ram_bank & 0x0F) as u16
        }
    }

    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let offset = (addr as usize) - 0x4000;
                read_rom_bank(rom, self.current_rom_bank() as usize, 0x4000, offset)
            }
            _ => 0xFF,
        }
    }

    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        let offset = (addr as usize).saturating_sub(0xA000);
        read_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset)
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }
        let offset = (addr as usize).saturating_sub(0xA000);
        write_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset, value);
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            0x2000..=0x2FFF => self.rom_bank_low = value,
            0x3000..=0x3FFF => self.rom_bank_high = value & 0x01,
            0x4000..=0x5FFF => {
                self.rumble_enabled = self.has_rumble && (value & 0x08 != 0);
                self.ram_bank = if self.has_rumble {
                    value & 0x07
                } else {
                    value & 0x0F
                };
            }
            _ => {}
        }
    }
}
