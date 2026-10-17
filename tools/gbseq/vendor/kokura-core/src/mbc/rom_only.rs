use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RomOnly;

impl RomOnly {
    pub fn current_rom_bank(&self) -> u16 {
        1
    }

    pub fn current_ram_bank(&self) -> u16 {
        0
    }

    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        rom.get(addr as usize).copied().unwrap_or(0xFF)
    }

    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        let offset = (addr as usize).saturating_sub(0xA000);
        ram.get(offset).copied().unwrap_or(0xFF)
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        let offset = (addr as usize).saturating_sub(0xA000);
        if let Some(byte) = ram.get_mut(offset) {
            *byte = value;
        }
    }

    pub fn write(&mut self, _addr: u16, _value: u8) {}
}
