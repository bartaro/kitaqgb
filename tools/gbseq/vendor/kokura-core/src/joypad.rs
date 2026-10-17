use serde::{Deserialize, Serialize};

use crate::types::JoypadTraceEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joypad {
    pub mask: u8,
    pub p1: u8,
}

impl Joypad {
    pub fn set_mask(&mut self, mask: u8) -> Vec<JoypadTraceEvent> {
        let old_mask = self.mask;
        let old_p1 = self.read_p1();
        self.mask = mask;
        let new_p1 = self.read_p1();
        let mut trace = Vec::new();
        if old_mask != mask {
            trace.push(JoypadTraceEvent::InputEdge {
                old_mask,
                new_mask: mask,
                p1: new_p1,
            });
        }
        if Self::interrupt_edge(old_p1, new_p1) {
            trace.push(JoypadTraceEvent::InterruptEdge { p1: new_p1 });
        }
        trace
    }

    pub fn write_p1(&mut self, value: u8) -> Vec<JoypadTraceEvent> {
        let old_p1 = self.read_p1();
        self.p1 = 0xC0 | (value & 0x30) | 0x0F;
        let new_p1 = self.read_p1();
        let mut trace = Vec::new();
        if (old_p1 & 0x30) != (new_p1 & 0x30) {
            trace.push(JoypadTraceEvent::SelectionWrite { old_p1, new_p1 });
        }
        if Self::interrupt_edge(old_p1, new_p1) {
            trace.push(JoypadTraceEvent::InterruptEdge { p1: new_p1 });
        }
        trace
    }

    pub fn read_p1(&self) -> u8 {
        let select = self.p1 & 0x30;
        let mut lower = 0x0F;
        if select & 0x10 == 0 {
            if self.mask & 0x01 != 0 {
                lower &= !0x01;
            }
            if self.mask & 0x02 != 0 {
                lower &= !0x02;
            }
            if self.mask & 0x04 != 0 {
                lower &= !0x04;
            }
            if self.mask & 0x08 != 0 {
                lower &= !0x08;
            }
        }
        if select & 0x20 == 0 {
            if self.mask & 0x10 != 0 {
                lower &= !0x01;
            }
            if self.mask & 0x20 != 0 {
                lower &= !0x02;
            }
            if self.mask & 0x40 != 0 {
                lower &= !0x04;
            }
            if self.mask & 0x80 != 0 {
                lower &= !0x08;
            }
        }
        0xC0 | select | lower
    }

    fn interrupt_edge(old_p1: u8, new_p1: u8) -> bool {
        ((old_p1 ^ new_p1) & old_p1 & 0x0F) != 0
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Self { mask: 0, p1: 0xCF }
    }
}
