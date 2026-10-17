use serde::{Deserialize, Serialize};

use crate::mbc::Mbc1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuC1 {
    pub inner: Mbc1,
    pub ir_mode: bool,
    pub ir_output: bool,
}

impl Default for HuC1 {
    fn default() -> Self {
        Self {
            inner: Mbc1::default(),
            ir_mode: false,
            ir_output: false,
        }
    }
}

impl HuC1 {
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
        if self.ir_mode {
            0xC0 | u8::from(self.ir_output)
        } else {
            self.inner.read_ram(ram, addr)
        }
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if self.ir_mode {
            self.ir_output = value & 0x01 != 0;
        } else {
            self.inner.write_ram(ram, addr, value);
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let mode = value & 0x0F;
                self.ir_mode = mode == 0x0E;
                self.inner.ram_enabled = !self.ir_mode;
            }
            _ => self.inner.write(addr, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_mode_is_not_disabled_by_non_ir_values() {
        let mut huc1 = HuC1::default();
        let mut ram = vec![0; 0x2000];

        huc1.write(0x0000, 0x00);
        huc1.write_ram(&mut ram, 0xA000, 0x42);

        assert_eq!(huc1.read_ram(&ram, 0xA000), 0x42);
    }

    #[test]
    fn ir_mode_exposes_ir_register() {
        let mut huc1 = HuC1::default();
        let mut ram = vec![0; 0x2000];

        huc1.write(0x0000, 0x0E);
        huc1.write_ram(&mut ram, 0xA000, 0x01);

        assert_eq!(huc1.read_ram(&ram, 0xA000), 0xC1);
    }
}
