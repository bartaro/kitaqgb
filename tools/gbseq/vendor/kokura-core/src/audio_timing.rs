use crate::{types::PpuTraceEvent, Machine};

// Transient instruction progress, never serialized into a frame-boundary state.
#[derive(Debug, Clone, Default)]
pub(super) struct InstructionClock {
    pub cpu_cycles: u32,
    pub frame_completed: bool,
    pub ppu_trace: Vec<PpuTraceEvent>,
    pub fast: bool,
}

pub(super) fn is_audio_io(addr: u16) -> bool {
    matches!(addr, 0xFF04 | 0xFF10..=0xFF26 | 0xFF30..=0xFF3F | 0xFF76..=0xFF77)
}

impl Machine {
    pub(super) fn read8_audio_timed(&mut self, addr: u16, cpu_cycles_until_read: u32) -> u8 {
        if is_audio_io(addr) {
            self.advance_to_audio_io(cpu_cycles_until_read);
        }
        self.read8(addr)
    }

    pub(super) fn write8_timed(&mut self, addr: u16, value: u8, cpu_cycles_until_write: u32) {
        if is_audio_io(addr) {
            self.advance_to_audio_io(cpu_cycles_until_write);
        }
        self.write8(addr, value);
    }

    pub(super) fn advance_to_audio_io(&mut self, target: u32) {
        let cycles = target.saturating_sub(self.instruction_clock.cpu_cycles);
        if cycles == 0 {
            return;
        }
        let ppu_cycles = self.cpu_cycles_to_ppu_cycles(cycles);
        if self.instruction_clock.fast {
            self.instruction_clock.frame_completed |= self.advance_clocks_fast(cycles, ppu_cycles);
        } else {
            let (frame, ppu, dma, mapper, io, apu) = self.advance_clocks(cycles, ppu_cycles);
            self.instruction_clock.frame_completed |= frame;
            self.instruction_clock.ppu_trace.extend(ppu);
            self.pending_dma_trace.extend(dma);
            self.pending_mapper_trace.extend(mapper);
            self.pending_io_trace.extend(io);
            self.pending_apu_trace.extend(apu);
        }
        self.instruction_clock.cpu_cycles = target;
    }
}

#[cfg(test)]
mod tests;
