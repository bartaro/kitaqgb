use serde::{Deserialize, Serialize};

use crate::types::TimerTraceEvent;

#[derive(Debug, Clone, Default)]
pub struct TimerTickResult {
    pub interrupt_requested: bool,
    pub trace: Vec<TimerTraceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timer {
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    div_counter: u16,
    overflow_delay_cycles: u8,
}

impl Timer {
    pub fn initialize_post_boot_state(&mut self, cgb_mode: bool) {
        self.div_counter = if cgb_mode { 0 } else { 0xC600 };
        self.div = (self.div_counter >> 8) as u16;
        self.tima = 0x00;
        self.tma = 0x00;
        self.tac = 0x00;
        self.overflow_delay_cycles = 0;
    }

    pub fn tick(&mut self, cycles: u32) -> TimerTickResult {
        let mut out = TimerTickResult::default();
        for _ in 0..cycles {
            if self.overflow_delay_cycles > 0 {
                self.overflow_delay_cycles -= 1;
                if self.overflow_delay_cycles == 0 {
                    self.tima = self.tma;
                    out.interrupt_requested = true;
                    out.trace.push(TimerTraceEvent::TimaReload {
                        reloaded_tima: self.tima,
                        tma: self.tma,
                    });
                }
            }

            let old_div = self.div_counter;
            self.div_counter = self.div_counter.wrapping_add(1);
            self.div = (self.div_counter >> 8) as u16;
            if self.timer_signal(old_div, self.tac)
                && !self.timer_signal(self.div_counter, self.tac)
            {
                self.increment_tima(&mut out.trace);
            }
        }
        out
    }

    pub fn tick_fast(&mut self, cycles: u32) -> bool {
        let mut interrupt_requested = false;
        let mut remaining = cycles;

        while remaining > 0 && self.overflow_delay_cycles > 0 {
            self.overflow_delay_cycles -= 1;
            if self.overflow_delay_cycles == 0 {
                self.tima = self.tma;
                interrupt_requested = true;
            }

            let old_div = self.div_counter;
            self.div_counter = self.div_counter.wrapping_add(1);
            self.div = (self.div_counter >> 8) as u16;
            if self.timer_signal(old_div, self.tac)
                && !self.timer_signal(self.div_counter, self.tac)
            {
                self.increment_tima_fast();
            }
            remaining -= 1;
        }

        if remaining == 0 {
            return interrupt_requested;
        }

        if self.tac & 0x04 == 0 {
            self.advance_div_counter(remaining);
            return interrupt_requested;
        }

        let bit = match self.tac & 0x03 {
            0x00 => 9,
            0x01 => 3,
            0x02 => 5,
            _ => 7,
        };
        let period = 1u32 << (bit + 1);
        let mask = period - 1;

        while remaining > 0 {
            let offset = u32::from(self.div_counter) & mask;
            let cycles_until_edge = if offset == 0 { period } else { period - offset };
            if cycles_until_edge > remaining {
                self.advance_div_counter(remaining);
                break;
            }

            self.advance_div_counter(cycles_until_edge);
            self.increment_tima_fast();
            remaining -= cycles_until_edge;

            while remaining > 0 && self.overflow_delay_cycles > 0 {
                self.overflow_delay_cycles -= 1;
                if self.overflow_delay_cycles == 0 {
                    self.tima = self.tma;
                    interrupt_requested = true;
                }

                let old_div = self.div_counter;
                self.div_counter = self.div_counter.wrapping_add(1);
                self.div = (self.div_counter >> 8) as u16;
                if self.timer_signal(old_div, self.tac)
                    && !self.timer_signal(self.div_counter, self.tac)
                {
                    self.increment_tima_fast();
                }
                remaining -= 1;
            }
        }
        interrupt_requested
    }

    pub fn write_div(&mut self, trace: &mut Vec<TimerTraceEvent>) {
        let old_div = self.div_counter;
        if self.timer_signal(old_div, self.tac) {
            self.increment_tima(trace);
        }
        self.div_counter = 0;
        self.div = 0;
        trace.push(TimerTraceEvent::DivResetEdge { old_div });
    }

    pub fn write_tima(&mut self, value: u8) {
        if self.overflow_delay_cycles > 0 {
            self.overflow_delay_cycles = 0;
        }
        self.tima = value;
    }

    pub fn write_tma(&mut self, value: u8) {
        self.tma = value;
    }

    pub fn write_tac(&mut self, value: u8, trace: &mut Vec<TimerTraceEvent>) {
        let old_tac = self.tac;
        let old_signal = self.timer_signal(self.div_counter, old_tac);
        let new_tac = value & 0x07;
        let new_signal = self.timer_signal(self.div_counter, new_tac);
        self.tac = new_tac;
        trace.push(TimerTraceEvent::TacWrite { old_tac, new_tac });
        if old_signal && !new_signal {
            self.increment_tima(trace);
        }
    }

    fn increment_tima(&mut self, trace: &mut Vec<TimerTraceEvent>) {
        if self.overflow_delay_cycles > 0 {
            return;
        }
        let old_tima = self.tima;
        let (next, overflow) = self.tima.overflowing_add(1);
        self.tima = next;
        if overflow {
            self.overflow_delay_cycles = 4;
            trace.push(TimerTraceEvent::TimaOverflow { old_tima });
        }
    }

    fn increment_tima_fast(&mut self) {
        if self.overflow_delay_cycles > 0 {
            return;
        }
        let (next, overflow) = self.tima.overflowing_add(1);
        self.tima = next;
        if overflow {
            self.overflow_delay_cycles = 4;
        }
    }

    fn advance_div_counter(&mut self, cycles: u32) {
        let mut remaining = cycles;
        while remaining > 0 {
            let chunk = remaining.min(u16::MAX as u32) as u16;
            self.div_counter = self.div_counter.wrapping_add(chunk);
            remaining -= u32::from(chunk);
        }
        self.div = (self.div_counter >> 8) as u16;
    }

    fn timer_signal(&self, div_value: u16, tac: u8) -> bool {
        if tac & 0x04 == 0 {
            return false;
        }
        let bit = match tac & 0x03 {
            0x00 => 9,
            0x01 => 3,
            0x02 => 5,
            _ => 7,
        };
        ((div_value >> bit) & 1) != 0
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_counter: 0,
            overflow_delay_cycles: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_fast_matches_tick_for_basic_progress() {
        let mut slow = Timer::default();
        let mut fast = Timer::default();
        slow.tac = 0x05;
        fast.tac = 0x05;
        slow.tma = 0x77;
        fast.tma = 0x77;

        let slow_result = slow.tick(512);
        let fast_interrupt = fast.tick_fast(512);

        assert_eq!(fast_interrupt, slow_result.interrupt_requested);
        assert_eq!(fast.div_counter, slow.div_counter);
        assert_eq!(fast.div, slow.div);
        assert_eq!(fast.tima, slow.tima);
        assert_eq!(fast.overflow_delay_cycles, slow.overflow_delay_cycles);
    }

    #[test]
    fn tick_fast_matches_tick_across_overflow_reload_window() {
        let mut slow = Timer::default();
        let mut fast = Timer::default();
        slow.tac = 0x05;
        fast.tac = 0x05;
        slow.tma = 0xAB;
        fast.tma = 0xAB;
        slow.tima = 0xFF;
        fast.tima = 0xFF;
        slow.div_counter = 15;
        fast.div_counter = 15;
        slow.div = (slow.div_counter >> 8) as u16;
        fast.div = (fast.div_counter >> 8) as u16;

        let slow_result = slow.tick(8);
        let fast_interrupt = fast.tick_fast(8);

        assert_eq!(fast_interrupt, slow_result.interrupt_requested);
        assert_eq!(fast.div_counter, slow.div_counter);
        assert_eq!(fast.div, slow.div);
        assert_eq!(fast.tima, slow.tima);
        assert_eq!(fast.overflow_delay_cycles, slow.overflow_delay_cycles);
    }
}
