use serde::{Deserialize, Serialize};

use crate::mbc::{read_ram_bank, read_rom_bank, write_ram_bank};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PocketCamera {
    pub rom_bank: u8,
    pub ram_bank: u8,
    pub ram_write_enabled: bool,
    pub camera_selected: bool,
    pub capture_active: bool,
    pub capture_cycles_remaining: u32,
    pub camera_registers: Vec<u8>,
}

impl Default for PocketCamera {
    fn default() -> Self {
        Self {
            rom_bank: 1,
            ram_bank: 0,
            ram_write_enabled: false,
            camera_selected: false,
            capture_active: false,
            capture_cycles_remaining: 0,
            camera_registers: vec![0; 0x80],
        }
    }
}

impl PocketCamera {
    pub fn current_rom_bank(&self) -> u16 {
        self.rom_bank as u16
    }

    pub fn current_ram_bank(&self) -> u16 {
        self.ram_bank as u16
    }

    pub fn read_rom(&self, rom: &[u8], addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let offset = (addr as usize) - 0x4000;
                read_rom_bank(rom, self.rom_bank as usize, 0x4000, offset)
            }
            _ => 0xFF,
        }
    }

    pub fn read_ram(&self, ram: &[u8], addr: u16) -> u8 {
        if self.camera_selected {
            let index = (addr as usize) & 0x7F;
            if index == 0 {
                (self.camera_registers[0] & 0x06) | u8::from(self.capture_active)
            } else {
                self.camera_registers.get(index).copied().unwrap_or(0)
            }
        } else if self.capture_active {
            0x00
        } else {
            let offset = (addr as usize).saturating_sub(0xA000);
            read_ram_bank(ram, self.ram_bank as usize, 0x2000, offset)
        }
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if self.camera_selected {
            let index = (addr as usize) & 0x7F;
            if let Some(byte) = self.camera_registers.get_mut(index) {
                *byte = value;
            }
            if index == 0 {
                let capture_requested = value & 0x01 != 0;
                self.camera_registers[0] = value & 0x07;
                if capture_requested {
                    if self.capture_cycles_remaining == 0 {
                        self.capture_cycles_remaining = self.capture_duration_cycles();
                    }
                    self.capture_active = true;
                } else {
                    self.capture_active = false;
                }
            }
        } else if self.ram_write_enabled && !self.capture_active {
            let offset = (addr as usize).saturating_sub(0xA000);
            write_ram_bank(ram, self.ram_bank as usize, 0x2000, offset, value);
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_write_enabled = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => self.rom_bank = value & 0x3F,
            0x4000..=0x5FFF => {
                self.camera_selected = value & 0x10 != 0;
                self.ram_bank = value & 0x0F;
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        if !self.capture_active {
            return;
        }

        self.capture_cycles_remaining = self.capture_cycles_remaining.saturating_sub(cycles);
        if self.capture_cycles_remaining == 0 {
            self.capture_active = false;
            self.camera_registers[0] &= !0x01;
        }
    }

    fn capture_duration_cycles(&self) -> u32 {
        let n_bit = if self.camera_registers.get(1).copied().unwrap_or(0) & 0x80 != 0 {
            0u32
        } else {
            512u32
        };
        let exposure = (u32::from(self.camera_registers.get(2).copied().unwrap_or(0)) << 8)
            | u32::from(self.camera_registers.get(3).copied().unwrap_or(0));
        (32446u32 + n_bit + 16 * exposure).saturating_mul(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_blocks_ram_until_duration_elapses() {
        let mut camera = PocketCamera::default();
        let mut ram = vec![0xFF; 0x2000];
        camera.camera_selected = true;
        camera.ram_write_enabled = true;

        camera.write_ram(&mut ram, 0xA002, 0x00);
        camera.write_ram(&mut ram, 0xA003, 0x01);
        camera.write_ram(&mut ram, 0xA000, 0x01);
        assert!(camera.capture_active);

        camera.camera_selected = false;
        assert_eq!(camera.read_ram(&ram, 0xA000), 0x00);

        camera.tick(camera.capture_cycles_remaining);
        assert!(!camera.capture_active);
        assert_eq!(camera.read_ram(&ram, 0xA000), 0xFF);
    }
}
