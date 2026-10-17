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
pub struct HuC3 {
    pub rom_bank: u8,
    pub mode: u8,
    pub ram_bank: u8,
    pub rtc_memory: Vec<u8>,
    pub rtc_access: u8,
    pub rtc_command: u8,
    pub rtc_argument: u8,
    pub rtc_response: u8,
    pub rtc_ready: bool,
    pub rtc_last_timestamp_secs: u64,
    pub rtc_subminute_secs: u8,
    pub ir_value: u8,
}

impl Default for HuC3 {
    fn default() -> Self {
        Self {
            rom_bank: 1,
            mode: 0,
            ram_bank: 0,
            rtc_memory: vec![0; 0x100],
            rtc_access: 0,
            rtc_command: 0,
            rtc_argument: 0,
            rtc_response: 0,
            rtc_ready: true,
            rtc_last_timestamp_secs: unix_now_secs(),
            rtc_subminute_secs: 0,
            ir_value: 0,
        }
    }
}

impl HuC3 {
    pub fn current_rom_bank(&self) -> u16 {
        u16::from(self.rom_bank & 0x7F)
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
        match self.mode & 0x0F {
            0x00 | 0x0A => {
                let offset = (addr as usize).saturating_sub(0xA000);
                read_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset)
            }
            0x0C => 0x80 | ((self.rtc_command & 0x07) << 4) | (self.rtc_response & 0x0F),
            0x0D => 0x80 | u8::from(self.rtc_ready),
            0x0E => 0xC0 | (self.ir_value & 0x01),
            _ => 0xFF,
        }
    }

    pub fn write_ram(&mut self, ram: &mut [u8], addr: u16, value: u8) {
        match self.mode & 0x0F {
            0x00 => {}
            0x0A => {
                let offset = (addr as usize).saturating_sub(0xA000);
                write_ram_bank(ram, self.current_ram_bank() as usize, 0x2000, offset, value);
            }
            0x0B => {
                self.rtc_command = (value >> 4) & 0x07;
                self.rtc_argument = value & 0x0F;
            }
            0x0D => {
                if value & 0x01 == 0 {
                    self.execute_rtc_command();
                }
            }
            0x0E => self.ir_value = value & 0x01,
            _ => {}
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.update_rtc();
        match addr {
            0x0000..=0x1FFF => self.mode = value & 0x0F,
            0x2000..=0x3FFF => self.rom_bank = value & 0x7F,
            0x4000..=0x5FFF => self.ram_bank = value & 0x0F,
            _ => {}
        }
    }

    pub fn tick(&mut self, _cycles: u32) {
        self.update_rtc();
    }

    fn execute_rtc_command(&mut self) {
        self.rtc_ready = false;
        self.update_rtc();
        self.rtc_response = match self.rtc_command {
            0x01 => {
                let value = self.read_rtc_nibble(self.rtc_access);
                self.rtc_access = self.rtc_access.wrapping_add(1);
                value
            }
            0x03 => {
                self.write_rtc_nibble(self.rtc_access, self.rtc_argument);
                self.rtc_access = self.rtc_access.wrapping_add(1);
                self.rtc_argument
            }
            0x04 => {
                self.rtc_access = (self.rtc_access & 0xF0) | (self.rtc_argument & 0x0F);
                self.rtc_access & 0x0F
            }
            0x05 => {
                self.rtc_access = ((self.rtc_argument & 0x0F) << 4) | (self.rtc_access & 0x0F);
                (self.rtc_access >> 4) & 0x0F
            }
            0x06 => self.execute_extended_command(self.rtc_argument),
            _ => 0x0F,
        };
        self.rtc_ready = true;
    }

    fn execute_extended_command(&mut self, argument: u8) -> u8 {
        match argument & 0x0F {
            0x0 => {
                self.copy_current_time_to_window();
                0
            }
            0x1 => {
                self.copy_window_to_current_time();
                0
            }
            0x2 => 0x1,
            0xE => 0x0,
            _ => 0x0,
        }
    }

    fn copy_current_time_to_window(&mut self) {
        for offset in 0..3u8 {
            self.rtc_memory[offset as usize] = self.rtc_memory[(0x10 + offset) as usize];
            self.rtc_memory[(0x03 + offset) as usize] = self.rtc_memory[(0x13 + offset) as usize];
        }
        self.rtc_memory[0x06] = 0;
    }

    fn copy_window_to_current_time(&mut self) {
        let old_current = self.read_absolute_time(0x10, 0x13);
        let old_event = self.read_absolute_time(0x58, 0x5B);
        for offset in 0..3u8 {
            self.rtc_memory[(0x10 + offset) as usize] = self.rtc_memory[offset as usize] & 0x0F;
            self.rtc_memory[(0x13 + offset) as usize] =
                self.rtc_memory[(0x03 + offset) as usize] & 0x0F;
        }
        let new_current = self.read_absolute_time(0x10, 0x13);
        let remaining = old_event as i64 - old_current as i64;
        let new_event = (new_current as i64 + remaining).max(0) as u64;
        self.write_absolute_time(0x58, 0x5B, new_event);
        self.rtc_subminute_secs = 0;
    }

    fn read_rtc_nibble(&self, index: u8) -> u8 {
        self.rtc_memory.get(index as usize).copied().unwrap_or(0) & 0x0F
    }

    fn write_rtc_nibble(&mut self, index: u8, value: u8) {
        if let Some(slot) = self.rtc_memory.get_mut(index as usize) {
            *slot = value & 0x0F;
        }
    }

    fn update_rtc(&mut self) {
        let now = unix_now_secs();
        if now <= self.rtc_last_timestamp_secs {
            return;
        }

        let elapsed = now - self.rtc_last_timestamp_secs;
        self.rtc_last_timestamp_secs = now;
        let total_seconds = u64::from(self.rtc_subminute_secs) + elapsed;
        let delta_minutes = total_seconds / 60;
        self.rtc_subminute_secs = (total_seconds % 60) as u8;
        if delta_minutes == 0 {
            return;
        }

        let current_minutes = self.read_nibble_triplet(0x10);
        let current_days = self.read_nibble_triplet(0x13);
        let total_minutes = current_minutes + delta_minutes;
        let day_increment = total_minutes / 1440;
        let wrapped_minutes = total_minutes % 1440;

        self.write_nibble_triplet(0x10, wrapped_minutes);
        self.write_nibble_triplet(0x13, current_days + day_increment);
    }

    fn read_nibble_triplet(&self, start: u8) -> u64 {
        u64::from(self.read_rtc_nibble(start))
            | (u64::from(self.read_rtc_nibble(start.wrapping_add(1))) << 4)
            | (u64::from(self.read_rtc_nibble(start.wrapping_add(2))) << 8)
    }

    fn write_nibble_triplet(&mut self, start: u8, value: u64) {
        self.write_rtc_nibble(start, value as u8 & 0x0F);
        self.write_rtc_nibble(start.wrapping_add(1), ((value >> 4) as u8) & 0x0F);
        self.write_rtc_nibble(start.wrapping_add(2), ((value >> 8) as u8) & 0x0F);
    }

    fn read_absolute_time(&self, minute_start: u8, day_start: u8) -> u64 {
        self.read_nibble_triplet(day_start) * 1440 + self.read_nibble_triplet(minute_start)
    }

    fn write_absolute_time(&mut self, minute_start: u8, day_start: u8, absolute_minutes: u64) {
        let days = absolute_minutes / 1440;
        let minutes = absolute_minutes % 1440;
        self.write_nibble_triplet(minute_start, minutes);
        self.write_nibble_triplet(day_start, days);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execute(huc3: &mut HuC3, command: u8, argument: u8) -> u8 {
        huc3.mode = 0x0B;
        huc3.write_ram(&mut [], 0xA000, (command << 4) | argument);
        huc3.mode = 0x0D;
        huc3.write_ram(&mut [], 0xA000, 0xFE);
        huc3.mode = 0x0C;
        huc3.read_ram(&[], 0xA000) & 0x0F
    }

    #[test]
    fn bank_zero_can_be_mapped_in_switchable_region() {
        let mut huc3 = HuC3::default();
        huc3.write(0x2000, 0x00);
        assert_eq!(huc3.current_rom_bank(), 0);
    }

    #[test]
    fn status_request_returns_ready_value() {
        let mut huc3 = HuC3::default();
        assert_eq!(execute(&mut huc3, 0x06, 0x02), 0x01);
    }

    #[test]
    fn rtc_window_commands_read_and_write_nibbles() {
        let mut huc3 = HuC3::default();

        assert_eq!(execute(&mut huc3, 0x04, 0x0E), 0x0E);
        assert_eq!(execute(&mut huc3, 0x05, 0x0), 0x0);
        assert_eq!(execute(&mut huc3, 0x03, 0x0A), 0x0A);
        assert_eq!(execute(&mut huc3, 0x04, 0x0E), 0x0E);
        assert_eq!(execute(&mut huc3, 0x05, 0x0), 0x0);
        assert_eq!(execute(&mut huc3, 0x01, 0x0), 0x0A);
    }

    #[test]
    fn writing_current_time_preserves_event_delta() {
        let mut huc3 = HuC3::default();
        huc3.write_nibble_triplet(0x10, 120);
        huc3.write_nibble_triplet(0x13, 1);
        huc3.write_nibble_triplet(0x58, 180);
        huc3.write_nibble_triplet(0x5B, 1);
        huc3.write_nibble_triplet(0x00, 240);
        huc3.write_nibble_triplet(0x03, 2);

        assert_eq!(execute(&mut huc3, 0x06, 0x01), 0);

        assert_eq!(huc3.read_nibble_triplet(0x10), 240);
        assert_eq!(huc3.read_nibble_triplet(0x13), 2);
        assert_eq!(huc3.read_nibble_triplet(0x58), 300);
        assert_eq!(huc3.read_nibble_triplet(0x5B), 2);
    }
}
