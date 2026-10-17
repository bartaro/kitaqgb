use serde::{Deserialize, Serialize};

use crate::mbc::Mbc1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mmm01 {
    pub mapped: bool,
    pub inner: Mbc1,
}

impl Default for Mmm01 {
    fn default() -> Self {
        Self {
            mapped: false,
            inner: Mbc1::default(),
        }
    }
}

impl Mmm01 {
    pub fn current_rom_bank(&self) -> u16 {
        self.inner.current_rom_bank()
    }

    pub fn current_ram_bank(&self) -> u16 {
        self.inner.current_ram_bank()
    }

    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        self.inner.read_rom(rom, addr)
    }

    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        self.inner.read_ram(ram, addr)
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        self.inner.write_ram(ram, addr, value);
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        if (0x0000..=0x7FFF).contains(&addr) {
            self.mapped = true;
        }
        self.inner.write(addr, value);
    }
}
