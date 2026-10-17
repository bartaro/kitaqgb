use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Flags(pub u8);

impl Flags {
    const Z: u8 = 0x80;
    const N: u8 = 0x40;
    const H: u8 = 0x20;
    const C: u8 = 0x10;

    pub fn z(self) -> bool {
        self.0 & Self::Z != 0
    }
    pub fn n(self) -> bool {
        self.0 & Self::N != 0
    }
    pub fn h(self) -> bool {
        self.0 & Self::H != 0
    }
    pub fn c(self) -> bool {
        self.0 & Self::C != 0
    }

    pub fn set_z(&mut self, value: bool) {
        self.set(Self::Z, value);
    }
    pub fn set_n(&mut self, value: bool) {
        self.set(Self::N, value);
    }
    pub fn set_h(&mut self, value: bool) {
        self.set(Self::H, value);
    }
    pub fn set_c(&mut self, value: bool) {
        self.set(Self::C, value);
    }

    fn set(&mut self, mask: u8, value: bool) {
        if value {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
}
