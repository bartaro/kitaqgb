use serde::{Deserialize, Serialize};

use crate::mbc::read_rom_bank;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mbc2 {
    pub rom_bank: u8,
    pub ram_enabled: bool,
}

impl Default for Mbc2 {
    fn default() -> Self {
        Self {
            rom_bank: 1,
            ram_enabled: false,
        }
    }
}

impl Mbc2 {
    pub const RAM_LEN: usize = 0x0200;

    pub fn current_rom_bank(&self) -> u16 {
        let bank = self.rom_bank & 0x0F;
        if bank == 0 {
            1
        } else {
            bank as u16
        }
    }

    pub fn current_ram_bank(&self) -> u16 {
        0
    }

    pub fn ram_enabled(&self) -> bool {
        self.ram_enabled
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

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x3FFF if addr & 0x0100 == 0 => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x0000..=0x3FFF => {
                let bank = value & 0x0F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            _ => {}
        }
    }

    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        let index = (addr as usize) & 0x01FF;
        0xF0 | ram.get(index).copied().unwrap_or(0x0F)
    }

    pub fn write_ram(&self, ram: &mut [u8], addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }
        let index = (addr as usize) & 0x01FF;
        if let Some(byte) = ram.get_mut(index) {
            *byte = value & 0x0F;
        }
    }
}
