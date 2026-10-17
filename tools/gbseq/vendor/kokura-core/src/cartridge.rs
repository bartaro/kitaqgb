use crate::{
    error::CoreError,
    mbc::{
        HuC1, HuC3, Mbc, Mbc1, Mbc2, Mbc3, Mbc5, Mbc6, Mbc7, Mmm01, PocketCamera, RomOnly, Tama5,
    },
};
use serde::{Deserialize, Serialize};

use crate::types::MapperKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartridgeHeader {
    pub title: String,
    pub cartridge_type: u8,
    pub rom_size_code: u8,
    pub ram_size_code: u8,
    pub cgb_flag: u8,
    pub sgb_flag: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cartridge {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub mbc: Mbc,
    pub header: CartridgeHeader,
    #[serde(default)]
    pub battery_dirty: bool,
}

impl Cartridge {
    pub fn empty() -> Self {
        Self {
            rom: vec![0; 0x8000],
            ram: vec![],
            mbc: Mbc::RomOnly(RomOnly::default()),
            header: CartridgeHeader {
                title: String::from("EMPTY"),
                cartridge_type: 0x00,
                rom_size_code: 0x00,
                ram_size_code: 0x00,
                cgb_flag: 0x00,
                sgb_flag: 0x00,
            },
            battery_dirty: false,
        }
    }

    pub fn from_rom(rom: Vec<u8>) -> Result<Self, CoreError> {
        if rom.len() < 0x150 {
            return Err(CoreError::RomTooSmall(rom.len()));
        }
        let title_bytes = &rom[0x134..=0x143];
        let end = title_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(title_bytes.len());
        let title = String::from_utf8_lossy(&title_bytes[..end])
            .trim()
            .to_string();
        let cartridge_type = rom[0x147];
        let rom_size_code = rom[0x148];
        let ram_size_code = rom[0x149];
        let cgb_flag = rom[0x143];
        let sgb_flag = rom[0x146];
        let ram_len = ram_size_bytes(cartridge_type, ram_size_code);
        let mbc = match cartridge_type {
            0x00 => Mbc::RomOnly(RomOnly::default()),
            0x01..=0x03 => Mbc::Mbc1(Mbc1::default()),
            0x05..=0x06 => Mbc::Mbc2(Mbc2::default()),
            0x08..=0x09 => Mbc::RomOnly(RomOnly),
            0x0B..=0x0D => Mbc::Mmm01(Mmm01::default()),
            0x0F..=0x13 => Mbc::Mbc3(Mbc3::default()),
            0x19..=0x1B => Mbc::Mbc5(Mbc5::default()),
            0x1C..=0x1E => Mbc::Mbc5(Mbc5::with_rumble(true)),
            0x20 => Mbc::Mbc6(Mbc6::default()),
            0x22 => Mbc::Mbc7(Mbc7::default()),
            0xFC => Mbc::PocketCamera(PocketCamera::default()),
            0xFD => Mbc::Tama5(Tama5::default()),
            0xFE => Mbc::HuC3(HuC3::default()),
            0xFF => Mbc::HuC1(HuC1::default()),
            _ => return Err(CoreError::UnsupportedCartridgeType(cartridge_type)),
        };
        Ok(Self {
            rom,
            ram: vec![0; ram_len],
            mbc,
            header: CartridgeHeader {
                title,
                cartridge_type,
                rom_size_code,
                ram_size_code,
                cgb_flag,
                sgb_flag,
            },
            battery_dirty: false,
        })
    }

    pub fn current_rom_bank(&self) -> u16 {
        self.mbc.current_rom_bank()
    }

    pub fn current_ram_bank(&self) -> u16 {
        self.mbc.current_ram_bank()
    }

    pub fn supports_cgb(&self) -> bool {
        matches!(self.header.cgb_flag, 0x80 | 0xC0)
    }

    pub fn cgb_only(&self) -> bool {
        self.header.cgb_flag == 0xC0
    }

    pub fn supports_sgb(&self) -> bool {
        self.header.sgb_flag == 0x03
    }

    pub fn has_battery_backed_ram(&self) -> bool {
        matches!(
            self.header.cartridge_type,
            0x03 | 0x06
                | 0x09
                | 0x0D
                | 0x0F
                | 0x10
                | 0x13
                | 0x1B
                | 0x1E
                | 0x22
                | 0xFC
                | 0xFD
                | 0xFE
                | 0xFF
        ) && !self.ram.is_empty()
    }

    pub fn battery_save_bytes(&self) -> Option<Vec<u8>> {
        if self.has_battery_backed_ram() {
            Some(self.ram.clone())
        } else {
            None
        }
    }

    pub fn battery_save_dirty(&self) -> bool {
        self.has_battery_backed_ram() && self.battery_dirty
    }

    pub fn battery_ram_access_enabled(&self) -> bool {
        self.has_battery_backed_ram() && self.mbc.ram_access_enabled()
    }

    pub fn mark_battery_save_clean(&mut self) {
        self.battery_dirty = false;
    }

    pub fn load_battery_save_bytes(&mut self, bytes: &[u8]) -> usize {
        if !self.has_battery_backed_ram() {
            return 0;
        }
        let len = self.ram.len().min(bytes.len());
        self.ram[..len].copy_from_slice(&bytes[..len]);
        self.battery_dirty = false;
        len
    }

    pub fn mapper_kind(&self) -> MapperKind {
        match &self.mbc {
            Mbc::RomOnly(_) => MapperKind::RomOnly,
            Mbc::Mbc1(_) => MapperKind::Mbc1,
            Mbc::Mbc2(_) => MapperKind::Mbc2,
            Mbc::Mmm01(_) => MapperKind::Mmm01,
            Mbc::Mbc3(_) => MapperKind::Mbc3,
            Mbc::Mbc5(_) => MapperKind::Mbc5,
            Mbc::Mbc6(_) => MapperKind::Mbc6,
            Mbc::Mbc7(_) => MapperKind::Mbc7,
            Mbc::PocketCamera(_) => MapperKind::PocketCamera,
            Mbc::Tama5(_) => MapperKind::Tama5,
            Mbc::HuC3(_) => MapperKind::HuC3,
            Mbc::HuC1(_) => MapperKind::HuC1,
        }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.mbc.read_rom(&self.rom, addr)
    }

    pub fn write_mbc(&mut self, addr: u16, value: u8) {
        self.mbc.write(addr, value);
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        self.mbc.read_ram(&self.ram, addr)
    }

    pub fn write_ram(&mut self, addr: u16, value: u8) {
        let before = self.mbc.read_ram(&self.ram, addr);
        self.mbc.write_ram(&mut self.ram, addr, value);
        let after = self.mbc.read_ram(&self.ram, addr);
        if self.has_battery_backed_ram() && before != after {
            self.battery_dirty = true;
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        self.mbc.tick(cycles);
    }
}

fn ram_size_bytes(cartridge_type: u8, code: u8) -> usize {
    match cartridge_type {
        0x05..=0x06 => Mbc2::RAM_LEN,
        0x22 => 0x0100,
        _ => match code {
            0x00 => 0,
            0x01 => 0x0800,
            0x02 => 0x2000,
            0x03 => 0x8000,
            0x04 => 0x20000,
            0x05 => 0x10000,
            _ => 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rom(cartridge_type: u8, ram_size_code: u8) -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x00;
        rom[0x147] = cartridge_type;
        rom[0x148] = 0x00;
        rom[0x149] = ram_size_code;
        rom[0x143] = 0x00;
        rom[0x146] = 0x00;
        rom
    }

    #[test]
    fn all_supported_cartridge_types_load() {
        let supported = [
            0x00, 0x01, 0x02, 0x03, 0x05, 0x06, 0x08, 0x09, 0x0B, 0x0C, 0x0D, 0x0F, 0x10, 0x11,
            0x12, 0x13, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x20, 0x22, 0xFC, 0xFD, 0xFE, 0xFF,
        ];

        for cartridge_type in supported {
            let rom = make_rom(cartridge_type, 0x03);
            let cartridge = Cartridge::from_rom(rom);
            assert!(
                cartridge.is_ok(),
                "cartridge type 0x{cartridge_type:02X} should load"
            );
        }
    }

    #[test]
    fn special_ram_sizes_are_allocated() {
        let mbc2 = Cartridge::from_rom(make_rom(0x06, 0x00)).unwrap();
        assert_eq!(mbc2.ram.len(), Mbc2::RAM_LEN);

        let mbc7 = Cartridge::from_rom(make_rom(0x22, 0x00)).unwrap();
        assert_eq!(mbc7.ram.len(), 0x0100);
    }

    #[test]
    fn battery_backed_ram_detection_matches_common_types() {
        let battery = Cartridge::from_rom(make_rom(0x03, 0x03)).unwrap();
        assert!(battery.has_battery_backed_ram());

        let plain_ram = Cartridge::from_rom(make_rom(0x02, 0x03)).unwrap();
        assert!(!plain_ram.has_battery_backed_ram());
    }

    #[test]
    fn battery_save_bytes_round_trip_into_ram() {
        let mut cart = Cartridge::from_rom(make_rom(0x03, 0x03)).unwrap();
        let bytes = [0x12, 0x34, 0x56, 0x78];
        let loaded = cart.load_battery_save_bytes(&bytes);
        assert_eq!(loaded, bytes.len());
        assert_eq!(&cart.ram[..bytes.len()], &bytes);
        assert_eq!(cart.battery_save_bytes().unwrap()[..bytes.len()], bytes);
        assert!(!cart.battery_save_dirty());
    }

    #[test]
    fn battery_dirty_tracks_external_ram_writes() {
        let mut cart = Cartridge::from_rom(make_rom(0x03, 0x03)).unwrap();
        assert!(!cart.battery_save_dirty());
        cart.write_mbc(0x0000, 0x0A);
        cart.write_ram(0xA000, 0x42);
        assert!(cart.battery_save_dirty());
        cart.mark_battery_save_clean();
        assert!(!cart.battery_save_dirty());
    }

    #[test]
    fn battery_ram_access_enabled_tracks_mapper_gate() {
        let mut cart = Cartridge::from_rom(make_rom(0x03, 0x03)).unwrap();
        assert!(!cart.battery_ram_access_enabled());
        cart.write_mbc(0x0000, 0x0A);
        assert!(cart.battery_ram_access_enabled());
        cart.write_mbc(0x0000, 0x00);
        assert!(!cart.battery_ram_access_enabled());
    }
}

#[cfg(test)]
mod cgb_header_tests {
    use super::*;

    #[test]
    fn cgb_flag_is_exposed() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x00;
        rom[0x143] = 0x80;
        let cart = Cartridge::from_rom(rom).unwrap();
        assert!(cart.supports_cgb());
        assert!(!cart.cgb_only());
    }
}

#[cfg(test)]
mod sgb_header_tests {
    use super::*;

    #[test]
    fn sgb_flag_is_exposed() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x100] = 0x00;
        rom[0x146] = 0x03;
        let cart = Cartridge::from_rom(rom).unwrap();
        assert!(cart.supports_sgb());
    }
}
