use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::mbc::read_rom_bank;

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tama5 {
    pub rom_bank: u8,
    pub ram_enabled: bool,
    pub selected_register: u8,
    pub registers: Vec<u8>,
    pub rtc_latched: [u8; 8],
    pub rtc_last_timestamp_secs: u64,
}

impl Default for Tama5 {
    fn default() -> Self {
        Self {
            rom_bank: 1,
            ram_enabled: false,
            selected_register: 0,
            registers: vec![0; 0x20],
            rtc_latched: [0; 8],
            rtc_last_timestamp_secs: unix_now_secs(),
        }
    }
}

impl Tama5 {
    pub fn current_rom_bank(&self) -> u16 {
        let bank = self.rom_bank & 0x7F;
        if bank == 0 {
            1
        } else {
            bank as u16
        }
    }

    pub fn current_ram_bank(&self) -> u16 {
        0
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

    pub fn read_ram(&self, _ram: &[u8], _addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        match self.selected_register {
            0x10..=0x17 => self.rtc_latched[(self.selected_register - 0x10) as usize],
            _ => self
                .registers
                .get(self.selected_register as usize)
                .copied()
                .unwrap_or(0xFF),
        }
    }

    pub fn write_ram(&mut self, _ram: &mut [u8], _addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }

        match self.selected_register {
            0x00 => self.rom_bank = (self.rom_bank & 0x70) | (value & 0x0F),
            0x01 => self.rom_bank = ((value & 0x07) << 4) | (self.rom_bank & 0x0F),
            0x0F => {
                if value & 0x01 != 0 {
                    self.latch_rtc_snapshot();
                }
            }
            0x10..=0x17 => {
                self.rtc_latched[(self.selected_register - 0x10) as usize] = value;
            }
            _ => {
                if let Some(byte) = self.registers.get_mut(self.selected_register as usize) {
                    *byte = value;
                }
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => {
                let bank = value & 0x7F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => self.selected_register = value & 0x1F,
            _ => {}
        }
    }

    pub fn tick(&mut self, _cycles: u32) {
        let now = unix_now_secs();
        if now != self.rtc_last_timestamp_secs {
            self.rtc_last_timestamp_secs = now;
            self.latch_rtc_snapshot();
        }
    }

    fn latch_rtc_snapshot(&mut self) {
        let now = unix_now_secs();
        let second = (now % 60) as u8;
        let minute = ((now / 60) % 60) as u8;
        let hour = ((now / 3600) % 24) as u8;
        let day = ((now / 86_400) % 100) as u8;
        let weekday = ((now / 86_400 + 4) % 7) as u8;

        self.rtc_latched[0] = to_bcd(second);
        self.rtc_latched[1] = to_bcd(minute);
        self.rtc_latched[2] = to_bcd(hour);
        self.rtc_latched[3] = to_bcd(day);
        self.rtc_latched[4] = to_bcd(weekday);
        self.rtc_latched[5] = self.rom_bank & 0x0F;
        self.rtc_latched[6] = (self.rom_bank >> 4) & 0x07;
        self.rtc_latched[7] = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_writes_can_rebuild_rom_bank() {
        let mut tama5 = Tama5::default();
        tama5.ram_enabled = true;

        tama5.selected_register = 0x00;
        tama5.write_ram(&mut [], 0xA000, 0x05);
        tama5.selected_register = 0x01;
        tama5.write_ram(&mut [], 0xA000, 0x02);

        assert_eq!(tama5.current_rom_bank(), 0x25);
    }

    #[test]
    fn rtc_latch_registers_are_populated() {
        let mut tama5 = Tama5::default();
        tama5.tick(0);
        tama5.ram_enabled = true;
        tama5.selected_register = 0x10;

        assert_ne!(tama5.read_ram(&[], 0xA000), 0xFF);
    }
}
