use serde::{Deserialize, Serialize};

pub const INT_VBLANK: u8 = 0x01;
pub const INT_LCD_STAT: u8 = 0x02;
pub const INT_TIMER: u8 = 0x04;
pub const INT_SERIAL: u8 = 0x08;
pub const INT_JOYPAD: u8 = 0x10;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterruptState {
    pub ie: u8,
    pub iflag: u8,
}

impl InterruptState {
    pub fn request(&mut self, mask: u8) {
        self.iflag |= mask;
    }

    pub fn pending_mask(&self) -> u8 {
        self.ie & self.iflag & 0x1F
    }

    pub fn has_pending(&self) -> bool {
        self.pending_mask() != 0
    }

    pub fn highest_priority(&self) -> Option<(u8, u16)> {
        let pending = self.pending_mask();
        if pending & INT_VBLANK != 0 {
            Some((INT_VBLANK, 0x0040))
        } else if pending & INT_LCD_STAT != 0 {
            Some((INT_LCD_STAT, 0x0048))
        } else if pending & INT_TIMER != 0 {
            Some((INT_TIMER, 0x0050))
        } else if pending & INT_SERIAL != 0 {
            Some((INT_SERIAL, 0x0058))
        } else if pending & INT_JOYPAD != 0 {
            Some((INT_JOYPAD, 0x0060))
        } else {
            None
        }
    }

    pub fn acknowledge(&mut self, mask: u8) {
        self.iflag &= !mask;
    }
}
