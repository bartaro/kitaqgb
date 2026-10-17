use serde::{Deserialize, Serialize};

const VRAM_BANK_SIZE: usize = 0x2000;
const WRAM_BANK_SIZE: usize = 0x1000;
const BG_PALETTE_RAM_LEN: usize = 0x40;
const OBJ_PALETTE_RAM_LEN: usize = 0x40;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaletteAccessResult {
    pub index: u8,
    pub value: u8,
    pub blocked: bool,
    pub auto_increment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    data: Vec<u8>,
    vram_banks: Vec<u8>,
    wram_banks: Vec<u8>,
    vbk: u8,
    svbk: u8,
    bg_palette_index: u8,
    obj_palette_index: u8,
    bg_palette_ram: Vec<u8>,
    obj_palette_ram: Vec<u8>,
}

impl Memory {
    pub fn read8(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.read_vram8(addr),
            0xC000..=0xCFFF => self.read_wram8(addr),
            0xD000..=0xDFFF => self.read_wram8(addr),
            0xE000..=0xFDFF => self.read_wram8(addr - 0x2000),
            _ => self.data[addr as usize],
        }
    }

    pub fn write8(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => self.write_vram8(addr, value),
            0xC000..=0xCFFF => self.write_wram8(addr, value),
            0xD000..=0xDFFF => self.write_wram8(addr, value),
            0xE000..=0xFDFF => self.write_wram8(addr - 0x2000, value),
            _ => self.data[addr as usize] = value,
        }
    }

    pub fn vram(&self) -> &[u8] {
        self.vram_bank(self.vbk)
    }

    pub fn vram_mut(&mut self) -> &mut [u8] {
        let bank = self.vbk;
        self.vram_bank_mut(bank)
    }

    pub fn vram_bank(&self, bank: u8) -> &[u8] {
        let bank = usize::from(bank & 0x01);
        let start = bank * VRAM_BANK_SIZE;
        &self.vram_banks[start..start + VRAM_BANK_SIZE]
    }

    pub fn vram_bank_mut(&mut self, bank: u8) -> &mut [u8] {
        let bank = usize::from(bank & 0x01);
        let start = bank * VRAM_BANK_SIZE;
        &mut self.vram_banks[start..start + VRAM_BANK_SIZE]
    }

    pub fn active_vram_bank(&self) -> u8 {
        self.vbk & 0x01
    }

    pub fn set_vbk(&mut self, value: u8) -> u8 {
        self.vbk = value & 0x01;
        self.data[0xFF4F] = 0xFE | self.vbk;
        self.refresh_vram_window();
        self.vbk
    }

    pub fn vbk_register(&self) -> u8 {
        0xFE | self.vbk
    }

    pub fn oam(&self) -> &[u8] {
        &self.data[0xFE00..0xFEA0]
    }

    pub fn oam_mut(&mut self) -> &mut [u8] {
        &mut self.data[0xFE00..0xFEA0]
    }

    pub fn active_wram_bank(&self) -> u8 {
        let bank = self.svbk & 0x07;
        if bank == 0 {
            1
        } else {
            bank
        }
    }

    pub fn set_svbk(&mut self, value: u8) -> u8 {
        self.svbk = value & 0x07;
        self.data[0xFF70] = 0xF8 | self.svbk;
        self.refresh_wram_window();
        self.active_wram_bank()
    }

    pub fn svbk_register(&self) -> u8 {
        0xF8 | self.svbk
    }

    pub fn read_bg_palette_data(&self, blocked: bool) -> u8 {
        if blocked {
            0xFF
        } else {
            self.bg_palette_ram[usize::from(self.bg_palette_index & 0x3F)]
        }
    }

    pub fn write_bg_palette_data(&mut self, value: u8, blocked: bool) -> PaletteAccessResult {
        let index = self.bg_palette_index & 0x3F;
        let auto_increment = self.bg_palette_index & 0x80 != 0;
        if !blocked {
            self.bg_palette_ram[usize::from(index)] = value;
        }
        let result = PaletteAccessResult {
            index,
            value,
            blocked,
            auto_increment,
        };
        if auto_increment {
            self.bg_palette_index = 0x80 | index.wrapping_add(1) & 0x3F;
            self.data[0xFF68] = self.bg_palette_index;
        }
        self.data[0xFF69] = self.read_bg_palette_data(blocked);
        result
    }

    pub fn set_bg_palette_index(&mut self, value: u8) -> u8 {
        self.bg_palette_index = value & 0xBF;
        self.data[0xFF68] = self.bg_palette_index;
        self.bg_palette_index
    }

    pub fn bg_palette_index_register(&self) -> u8 {
        self.bg_palette_index
    }

    pub fn read_obj_palette_data(&self, blocked: bool) -> u8 {
        if blocked {
            0xFF
        } else {
            self.obj_palette_ram[usize::from(self.obj_palette_index & 0x3F)]
        }
    }

    pub fn write_obj_palette_data(&mut self, value: u8, blocked: bool) -> PaletteAccessResult {
        let index = self.obj_palette_index & 0x3F;
        let auto_increment = self.obj_palette_index & 0x80 != 0;
        if !blocked {
            self.obj_palette_ram[usize::from(index)] = value;
        }
        let result = PaletteAccessResult {
            index,
            value,
            blocked,
            auto_increment,
        };
        if auto_increment {
            self.obj_palette_index = 0x80 | index.wrapping_add(1) & 0x3F;
            self.data[0xFF6A] = self.obj_palette_index;
        }
        self.data[0xFF6B] = self.read_obj_palette_data(blocked);
        result
    }

    pub fn set_obj_palette_index(&mut self, value: u8) -> u8 {
        self.obj_palette_index = value & 0xBF;
        self.data[0xFF6A] = self.obj_palette_index;
        self.obj_palette_index
    }

    pub fn obj_palette_index_register(&self) -> u8 {
        self.obj_palette_index
    }

    pub fn bg_palette_rgb555(&self, palette: u8, color: u8) -> u16 {
        self.palette_rgb555(&self.bg_palette_ram, palette, color)
    }

    pub fn obj_palette_rgb555(&self, palette: u8, color: u8) -> u16 {
        self.palette_rgb555(&self.obj_palette_ram, palette, color)
    }

    fn palette_rgb555(&self, ram: &[u8], palette: u8, color: u8) -> u16 {
        let base = usize::from((palette & 0x07) * 8 + (color & 0x03) * 2);
        let lo = ram.get(base).copied().unwrap_or(0) as u16;
        let hi = ram.get(base + 1).copied().unwrap_or(0) as u16;
        (hi << 8) | lo
    }

    fn read_vram8(&self, addr: u16) -> u8 {
        let offset = usize::from(addr - 0x8000);
        self.vram_bank(self.vbk)[offset]
    }

    fn write_vram8(&mut self, addr: u16, value: u8) {
        let offset = usize::from(addr - 0x8000);
        let bank = usize::from(self.vbk);
        self.vram_banks[bank * VRAM_BANK_SIZE + offset] = value;
        self.data[addr as usize] = value;
    }

    fn read_wram8(&self, addr: u16) -> u8 {
        match addr {
            0xC000..=0xCFFF => self.wram_banks[usize::from(addr - 0xC000)],
            0xD000..=0xDFFF => {
                let bank = usize::from(self.active_wram_bank());
                let offset = usize::from(addr - 0xD000);
                self.wram_banks[bank * WRAM_BANK_SIZE + offset]
            }
            _ => self.data[addr as usize],
        }
    }

    fn write_wram8(&mut self, addr: u16, value: u8) {
        match addr {
            0xC000..=0xCFFF => {
                let offset = usize::from(addr - 0xC000);
                self.wram_banks[offset] = value;
                self.data[addr as usize] = value;
                self.data[usize::from(addr + 0x2000)] = value;
            }
            0xD000..=0xDFFF => {
                let bank = usize::from(self.active_wram_bank());
                let offset = usize::from(addr - 0xD000);
                self.wram_banks[bank * WRAM_BANK_SIZE + offset] = value;
                self.data[addr as usize] = value;
                if addr <= 0xDDFF {
                    self.data[usize::from(addr + 0x2000)] = value;
                }
            }
            _ => self.data[addr as usize] = value,
        }
    }

    fn refresh_vram_window(&mut self) {
        let start = usize::from(self.vbk) * VRAM_BANK_SIZE;
        self.data[0x8000..0xA000].copy_from_slice(&self.vram_banks[start..start + VRAM_BANK_SIZE]);
    }

    fn refresh_wram_window(&mut self) {
        self.data[0xC000..0xD000].copy_from_slice(&self.wram_banks[0..WRAM_BANK_SIZE]);
        let bank = usize::from(self.active_wram_bank());
        let start = bank * WRAM_BANK_SIZE;
        self.data[0xD000..0xE000].copy_from_slice(&self.wram_banks[start..start + WRAM_BANK_SIZE]);
        self.data.copy_within(0xC000..0xD000, 0xE000);
        self.data.copy_within(0xD000..0xDE00, 0xF000);
    }
}

impl Default for Memory {
    fn default() -> Self {
        let mut out = Self {
            data: vec![0; 0x10000],
            vram_banks: vec![0; VRAM_BANK_SIZE * 2],
            wram_banks: vec![0; WRAM_BANK_SIZE * 8],
            vbk: 0,
            svbk: 1,
            bg_palette_index: 0,
            obj_palette_index: 0,
            bg_palette_ram: vec![0xFF; BG_PALETTE_RAM_LEN],
            obj_palette_ram: vec![0; OBJ_PALETTE_RAM_LEN],
        };
        out.refresh_vram_window();
        out.refresh_wram_window();
        out.data[0xFF4F] = 0xFE;
        out.data[0xFF68] = 0x00;
        out.data[0xFF69] = 0xFF;
        out.data[0xFF6A] = 0x00;
        out.data[0xFF6B] = 0x00;
        out.data[0xFF70] = 0xF9;
        out
    }
}
