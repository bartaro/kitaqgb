use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::mbc::{read_ram_bank, read_rom_bank, write_ram_bank};

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mbc3 {
    pub rom_bank: u8,
    pub ram_bank: u8,
    pub ram_enabled: bool,
    pub rtc_registers: [u8; 5],
    pub rtc_latched_registers: [u8; 5],
    pub latch_value: u8,
    pub rtc_last_timestamp_secs: u64,
}

impl Default for Mbc3 {
    fn default() -> Self {
        let now = unix_now_secs();
        Self {
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            rtc_registers: [0; 5],
            rtc_latched_registers: [0; 5],
            latch_value: 0,
            rtc_last_timestamp_secs: now,
        }
    }
}

impl Mbc3 {
    pub fn current_rom_bank(&self) -> u16 {
        let bank = if self.rom_bank == 0 { 1 } else { self.rom_bank };
        bank as u16
    }

    pub fn current_ram_bank(&self) -> u16 {
        (self.ram_bank & 0x03) as u16
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
        match self.ram_bank {
            0x08..=0x0C => self.rtc_latched_registers[(self.ram_bank - 0x08) as usize],
            _ => {
                let offset = (addr as usize).saturating_sub(0xA000);
                read_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset)
            }
        }
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }
        self.update_rtc();
        match self.ram_bank {
            0x08..=0x0C => self.write_rtc_register(self.ram_bank - 0x08, value),
            _ => {
                let offset = (addr as usize).saturating_sub(0xA000);
                write_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset, value);
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.update_rtc();
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => {
                let value = value & 0x7F;
                self.rom_bank = if value == 0 { 1 } else { value };
            }
            0x4000..=0x5FFF => self.ram_bank = value,
            0x6000..=0x7FFF => {
                if self.latch_value == 0 && value == 1 {
                    self.rtc_latched_registers = self.rtc_registers;
                }
                self.latch_value = value;
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, _cycles: u32) {
        self.update_rtc();
    }

    fn update_rtc(&mut self) {
        let now = unix_now_secs();
        if now <= self.rtc_last_timestamp_secs {
            return;
        }

        let elapsed = now - self.rtc_last_timestamp_secs;
        self.rtc_last_timestamp_secs = now;

        if self.rtc_registers[4] & 0x40 != 0 {
            return;
        }

        self.advance_rtc_seconds(elapsed);
    }

    fn advance_rtc_seconds(&mut self, elapsed_seconds: u64) {
        if elapsed_seconds == 0 {
            return;
        }

        let seconds = self.rtc_registers[0] as u64 + elapsed_seconds;
        self.rtc_registers[0] = (seconds % 60) as u8;

        let mut carry = seconds / 60;
        let minutes = self.rtc_registers[1] as u64 + carry;
        self.rtc_registers[1] = (minutes % 60) as u8;

        carry = minutes / 60;
        let hours = self.rtc_registers[2] as u64 + carry;
        self.rtc_registers[2] = (hours % 24) as u8;

        carry = hours / 24;
        let current_days =
            u16::from(self.rtc_registers[3]) | (u16::from(self.rtc_registers[4] & 0x01) << 8);
        let total_days = u64::from(current_days) + carry;
        let wrapped_days = (total_days & 0x01FF) as u16;

        self.rtc_registers[3] = wrapped_days as u8;
        self.rtc_registers[4] = (self.rtc_registers[4] & 0xFE) | ((wrapped_days >> 8) as u8 & 0x01);
        if total_days > 0x01FF {
            self.rtc_registers[4] |= 0x80;
        }
    }

    fn write_rtc_register(&mut self, index: u8, value: u8) {
        let slot = index as usize;
        match index {
            0 => self.rtc_registers[slot] = value % 60,
            1 => self.rtc_registers[slot] = value % 60,
            2 => self.rtc_registers[slot] = value % 24,
            3 => self.rtc_registers[slot] = value,
            4 => {
                self.rtc_registers[slot] = (value & 0xC1) | (self.rtc_registers[slot] & 0x3E);
            }
            _ => {}
        }
        self.rtc_latched_registers[slot] = self.rtc_registers[slot];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_edge_copies_live_rtc_registers() {
        let mut mbc = Mbc3::default();
        mbc.ram_enabled = true;
        mbc.ram_bank = 0x08;
        mbc.rtc_registers = [12, 34, 5, 0xAA, 0x01];
        mbc.rtc_latched_registers = [0; 5];
        mbc.rtc_last_timestamp_secs = unix_now_secs();

        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.read_ram(&[], 0xA000), 12);
        assert_eq!(mbc.rtc_latched_registers, [12, 34, 5, 0xAA, 0x01]);
    }

    #[test]
    fn rtc_advances_and_sets_carry_on_day_overflow() {
        let mut mbc = Mbc3::default();
        mbc.rtc_registers = [59, 59, 23, 0xFF, 0x01];
        mbc.rtc_latched_registers = mbc.rtc_registers;
        mbc.rtc_last_timestamp_secs = unix_now_secs().saturating_sub(1);

        mbc.tick(0);

        assert_eq!(mbc.rtc_registers[0], 0);
        assert_eq!(mbc.rtc_registers[1], 0);
        assert_eq!(mbc.rtc_registers[2], 0);
        assert_eq!(mbc.rtc_registers[3], 0);
        assert_ne!(mbc.rtc_registers[4] & 0x80, 0);
    }
}
