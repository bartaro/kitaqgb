use serde::{Deserialize, Serialize};

use crate::types::SerialTraceEvent;

const SERIAL_INTERNAL_BIT_CYCLES: u16 = 512;

#[derive(Debug, Clone, Default)]
pub struct SerialTickResult {
    pub interrupt_requested: bool,
    pub trace: Vec<SerialTraceEvent>,
}

#[derive(Debug, Clone, Default)]
pub struct SerialLinkExchangeResult {
    pub completed: bool,
    pub self_interrupt_requested: bool,
    pub peer_interrupt_requested: bool,
    pub self_trace: Vec<SerialTraceEvent>,
    pub peer_trace: Vec<SerialTraceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Serial {
    pub sb: u8,
    pub sc: u8,
    cycle_accum: u16,
    bits_remaining: u8,
    transfer_active: bool,
}

impl Serial {
    pub fn read_sc(&self) -> u8 {
        self.sc | 0x7E
    }

    pub fn write_sb(&mut self, value: u8) {
        self.sb = value;
    }

    pub fn write_sc(&mut self, value: u8) -> Vec<SerialTraceEvent> {
        self.sc = value & 0x83;
        let mut trace = Vec::new();
        if self.sc & 0x80 != 0 {
            self.transfer_active = true;
            self.bits_remaining = 8;
            self.cycle_accum = 0;
            trace.push(SerialTraceEvent::TransferStart {
                sb: self.sb,
                sc: self.read_sc(),
                internal_clock: self.internal_clock(),
            });
        }
        trace
    }

    pub fn tick(&mut self, cycles: u32) -> SerialTickResult {
        let mut out = SerialTickResult::default();
        if !self.transfer_active || !self.internal_clock() {
            return out;
        }
        self.cycle_accum = self.cycle_accum.saturating_add(cycles as u16);
        while self.cycle_accum >= SERIAL_INTERNAL_BIT_CYCLES && self.bits_remaining > 0 {
            self.cycle_accum -= SERIAL_INTERNAL_BIT_CYCLES;
            self.sb = (self.sb << 1) | 1;
            self.bits_remaining -= 1;
            if self.bits_remaining == 0 {
                self.transfer_active = false;
                self.sc &= !0x80;
                out.interrupt_requested = true;
                out.trace.push(SerialTraceEvent::TransferComplete {
                    sb: self.sb,
                    sc: self.read_sc(),
                    internal_clock: true,
                });
            }
        }
        out
    }

    pub fn tick_fast(&mut self, cycles: u32) -> bool {
        if !self.transfer_active || !self.internal_clock() {
            return false;
        }
        self.cycle_accum = self.cycle_accum.saturating_add(cycles as u16);
        while self.cycle_accum >= SERIAL_INTERNAL_BIT_CYCLES && self.bits_remaining > 0 {
            self.cycle_accum -= SERIAL_INTERNAL_BIT_CYCLES;
            self.sb = (self.sb << 1) | 1;
            self.bits_remaining -= 1;
            if self.bits_remaining == 0 {
                self.transfer_active = false;
                self.sc &= !0x80;
                return true;
            }
        }
        false
    }

    pub fn transfer_active(&self) -> bool {
        self.transfer_active
    }

    pub fn internal_clock(&self) -> bool {
        self.sc & 0x01 != 0
    }

    pub fn exchange_with_peer(&mut self, peer: &mut Serial) -> SerialLinkExchangeResult {
        let mut out = SerialLinkExchangeResult::default();
        if !self.transfer_active || !peer.transfer_active {
            return out;
        }
        if self.internal_clock() == peer.internal_clock() {
            return out;
        }

        let self_out = self.sb;
        let peer_out = peer.sb;

        self.sb = peer_out;
        self.sc &= !0x80;
        self.cycle_accum = 0;
        self.bits_remaining = 0;
        self.transfer_active = false;

        peer.sb = self_out;
        peer.sc &= !0x80;
        peer.cycle_accum = 0;
        peer.bits_remaining = 0;
        peer.transfer_active = false;

        out.completed = true;
        out.self_interrupt_requested = true;
        out.peer_interrupt_requested = true;
        out.self_trace.push(SerialTraceEvent::TransferComplete {
            sb: self.sb,
            sc: self.read_sc(),
            internal_clock: self.internal_clock(),
        });
        out.peer_trace.push(SerialTraceEvent::TransferComplete {
            sb: peer.sb,
            sc: peer.read_sc(),
            internal_clock: peer.internal_clock(),
        });
        out
    }
}

impl Default for Serial {
    fn default() -> Self {
        Self {
            sb: 0,
            sc: 0,
            cycle_accum: 0,
            bits_remaining: 0,
            transfer_active: false,
        }
    }
}
