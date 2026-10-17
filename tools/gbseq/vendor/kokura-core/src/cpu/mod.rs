pub mod decode;
pub mod exec;
pub mod ops_alu;
pub mod ops_jump;
pub mod ops_load;
pub mod ops_misc;
pub mod regs;

use regs::Flags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cpu {
    pub a: u8,
    pub f: Flags,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
    #[serde(skip)]
    pub ime_enable_delay: u8,
    pub halted: bool,
    #[serde(skip)]
    pub halt_bug: bool,
    pub stopped: bool,
}

impl Cpu {
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            a: 0,
            f: Flags::default(),
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
            ime_enable_delay: 0,
            halted: false,
            halt_bug: false,
            stopped: false,
        }
    }
}
