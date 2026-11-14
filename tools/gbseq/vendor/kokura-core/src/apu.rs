use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::types::{ApuTraceEvent, HardwareMode};

#[cfg(test)]
mod accuracy_tests;
#[cfg(test)]
mod register_write_tests;
const APU_FRAME_SEQUENCER_CYCLES: u16 = 8192;
const APU_MIX_TRACE_CYCLES: u16 = 1024;
const APU_PCM_SAMPLE_CYCLES: u16 = 64;
pub const APU_OUTPUT_SAMPLE_RATE: u32 = 65_536;
const DEFAULT_APU_PCM_BUFFER_CAPACITY_FRAMES: usize = 65_536;
const APU_HPF_CHARGE_FACTOR_DMG: f64 = 0.999_958;
const APU_HPF_CHARGE_FACTOR_CGB: f64 = 0.998_943;
const APU_HPF_SAMPLE_FACTOR_DMG: f64 = sample_charge_factor(APU_HPF_CHARGE_FACTOR_DMG);
const APU_HPF_SAMPLE_FACTOR_CGB: f64 = sample_charge_factor(APU_HPF_CHARGE_FACTOR_CGB);
const REGISTER_READ_MASKS: [u8; 0x16] = [
    0x80, 0x3F, 0x00, 0xFF, 0xBF, 0xFF, 0x3F, 0x00, 0xFF, 0xBF, 0x7F, 0xFF, 0x9F, 0xFF, 0xBF, 0xFF,
    0xFF, 0x00, 0x00, 0xBF, 0x00, 0x00,
];

// Hardware charge factors are per T-cycle, not per output sample.
const fn sample_charge_factor(per_clock: f64) -> f64 {
    let mut factor = 1.0;
    let mut clocks = 0;
    while clocks < APU_PCM_SAMPLE_CYCLES {
        factor *= per_clock;
        clocks += 1;
    }
    factor
}
const APU_OUTPUT_LPF_ALPHA_DMG: f64 = 0.68;
const APU_OUTPUT_LPF_ALPHA_CGB: f64 = 0.78;
const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];
const NOISE_DIVISORS: [u16; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

const CONTROL_LATCHES_VALID: u8 = 0x80;
const ENVELOPE_RUNNING: u8 = 0x01;
const SWEEP_ENABLED: u8 = 0x02;
#[derive(Debug, Clone, Default)]
pub struct ApuTickResult {
    pub trace: Vec<ApuTraceEvent>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ChannelState {
    enabled: bool,
    dac_enabled: bool,
    length_counter: u16,
    envelope_volume: u8,
    envelope_period: u8,
    envelope_increase: bool,
    envelope_counter: u8,
    shadow_frequency: u16,
    sweep_period: u8,
    sweep_counter: u8,
    sweep_shift: u8,
    sweep_negate: bool,
    duty_position: u8,
    freq_timer: u16,
    wave_position: u8,
    wave_sample_buffer: u8,
    lfsr: u16,
    force_zero_until_advance: bool,
    sweep_negate_used: bool,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            length_counter: 0,
            envelope_volume: 0,
            envelope_period: 0,
            envelope_increase: false,
            envelope_counter: 0,
            shadow_frequency: 0,
            sweep_period: 0,
            sweep_counter: 0,
            sweep_shift: 0,
            sweep_negate: false,
            duty_position: 0,
            freq_timer: 0,
            wave_position: 0,
            wave_sample_buffer: 0,
            lfsr: 0x7FFF,
            force_zero_until_advance: true,
            sweep_negate_used: false,
        }
    }
}

impl ChannelState {
    // Only call for CH1/CH2/CH4. Legacy states lack trigger history; infer once,
    // before a live register write changes the parameters used for migration.
    fn ensure_control_latches(&mut self) {
        if self.wave_sample_buffer & CONTROL_LATCHES_VALID == 0 {
            self.wave_sample_buffer = CONTROL_LATCHES_VALID;
            if self.enabled {
                self.wave_sample_buffer |= ENVELOPE_RUNNING;
                if self.sweep_period != 0 || self.sweep_shift != 0 {
                    self.wave_sample_buffer |= SWEEP_ENABLED;
                }
            }
        }
    }

    fn envelope_running(&self) -> bool {
        self.wave_sample_buffer & ENVELOPE_RUNNING != 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Apu {
    regs: [u8; 0x17],
    wave_ram: [u8; 0x10],
    pub master_enabled: bool,
    pub channel_enable: u8,
    frame_seq_cycles: u16,
    frame_seq_step: u8,
    channels: [ChannelState; 4],
    mix_trace_cycles: u16,
    pcm_sample_cycles: u16,
    pcm_buffer: VecDeque<i16>,
    #[serde(default = "default_pcm_buffer_capacity_frames")]
    pcm_buffer_capacity_frames: usize,
    pcm_frames_generated: u64,
    pcm_frames_dropped: u64,
    pcm_hpf_left: f64,
    pcm_hpf_right: f64,
    #[serde(skip, default)]
    pcm_lpf_left: f64,
    #[serde(skip, default)]
    pcm_lpf_right: f64,
    pcm_accum_left: i32,
    pcm_accum_right: i32,
    pcm_accum_cycles: u16,
    last_mixed_left: i16,
    last_mixed_right: i16,
    last_mixed_mask: u8,
    cgb_mode: bool,
}

impl Default for Apu {
    fn default() -> Self {
        Self {
            regs: [0; 0x17],
            wave_ram: [0; 0x10],
            master_enabled: false,
            channel_enable: 0,
            frame_seq_cycles: 0,
            frame_seq_step: 7,
            channels: [ChannelState::default(); 4],
            mix_trace_cycles: 0,
            pcm_sample_cycles: 0,
            pcm_buffer: VecDeque::with_capacity(default_pcm_buffer_capacity_frames() * 2),
            pcm_buffer_capacity_frames: default_pcm_buffer_capacity_frames(),
            pcm_frames_generated: 0,
            pcm_frames_dropped: 0,
            pcm_hpf_left: 0.0,
            pcm_hpf_right: 0.0,
            pcm_lpf_left: 0.0,
            pcm_lpf_right: 0.0,
            pcm_accum_left: 0,
            pcm_accum_right: 0,
            pcm_accum_cycles: 0,
            last_mixed_left: 0,
            last_mixed_right: 0,
            last_mixed_mask: 0,
            cgb_mode: false,
        }
    }
}

const fn default_pcm_buffer_capacity_frames() -> usize {
    DEFAULT_APU_PCM_BUFFER_CAPACITY_FRAMES
}

impl Apu {
    pub fn initialize_post_boot_state(&mut self, mode: HardwareMode) {
        self.regs = [
            0x80, 0xBF, 0xF3, 0xFF, 0xBF, 0x3F, 0x00, 0xFF, 0xBF, 0x7F, 0xFF, 0x9F, 0xFF, 0xBF,
            0xFF, 0x00, 0x00, 0xBF, 0x77, 0xF3, 0x00, 0x00, 0x00,
        ];
        self.wave_ram = [0; 0x10];
        self.master_enabled = true;
        self.channel_enable = if mode == HardwareMode::Cgb {
            0x00
        } else {
            0x01
        };
        self.frame_seq_cycles = 0;
        self.frame_seq_step = 0;
        self.channels = [ChannelState::default(); 4];
        if self.channel_enable & 0x01 != 0 {
            self.channels[0].enabled = true;
            self.channels[0].dac_enabled = true;
        }
        self.mix_trace_cycles = 0;
        self.pcm_sample_cycles = 0;
        self.pcm_buffer.clear();
        self.pcm_hpf_left = 0.0;
        self.pcm_hpf_right = 0.0;
        self.pcm_lpf_left = 0.0;
        self.pcm_lpf_right = 0.0;
        self.pcm_accum_left = 0;
        self.pcm_accum_right = 0;
        self.pcm_accum_cycles = 0;
        self.last_mixed_left = 0;
        self.last_mixed_right = 0;
        self.last_mixed_mask = 0;
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF25 => {
                let index = (addr - 0xFF10) as usize;
                self.regs[index] | REGISTER_READ_MASKS[index]
            }
            0xFF26 => 0x70 | (u8::from(self.master_enabled) << 7) | (self.channel_enable & 0x0F),
            0xFF30..=0xFF3F => {
                let requested = (addr - 0xFF30) as usize;
                if self.wave_ram_access_blocked() {
                    0xFF
                } else {
                    self.wave_ram[self.wave_ram_visible_index(requested)]
                }
            }
            0xFF76 => self.pcm12(),
            0xFF77 => self.pcm34(),
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) -> Vec<ApuTraceEvent> {
        let mut trace = Vec::new();
        match addr {
            0xFF10..=0xFF25 => {
                if !self.master_enabled {
                    if !self.cgb_mode {
                        let length = match addr {
                            0xFF11 => Some((0, 64 - u16::from(value & 0x3F))),
                            0xFF16 => Some((1, 64 - u16::from(value & 0x3F))),
                            0xFF1B => Some((2, 256 - u16::from(value))),
                            0xFF20 => Some((3, 64 - u16::from(value & 0x3F))),
                            _ => None,
                        };
                        if let Some((index, counter)) = length {
                            self.channels[index].length_counter = counter;
                        }
                    }
                    return trace;
                }
                let reg_index = (addr - 0xFF10) as usize;
                let old_value = self.regs[reg_index];
                self.regs[reg_index] = value;
                self.apply_register_side_effects(addr, value, &mut trace);
                if addr == 0xFF24 || addr == 0xFF25 {
                    if self.master_enabled && old_value != value {
                        trace.push(ApuTraceEvent::PopRisk {
                            source: if addr == 0xFF24 { 2 } else { 3 },
                            reg: addr,
                        });
                    }
                    trace.push(ApuTraceEvent::MixerControlWrite {
                        nr50: self.regs[0x14],
                        nr51: self.regs[0x15],
                        nr52: self.read(0xFF26),
                    });
                }
                if let Some(channel) = trigger_channel(addr) {
                    if self.frame_seq_step & 1 == 0 && old_value & 0x40 == 0 && value & 0x40 != 0 {
                        self.clock_channel_length(channel, value & 0x80 == 0, &mut trace);
                    }
                    if value & 0x80 != 0 && self.master_enabled {
                        self.trigger_channel(channel, &mut trace, addr, value);
                    }
                }
            }
            0xFF26 => {
                let enabled = value & 0x80 != 0;
                if enabled != self.master_enabled {
                    self.master_enabled = enabled;
                    if !enabled {
                        self.regs = [0; 0x17];
                        self.channel_enable = 0;
                        self.frame_seq_step = 7;
                        self.channels = [ChannelState::default(); 4];
                        self.mix_trace_cycles = 0;
                        self.pcm_hpf_left = 0.0;
                        self.pcm_hpf_right = 0.0;
                        self.pcm_lpf_left = 0.0;
                        self.pcm_lpf_right = 0.0;
                        self.last_mixed_left = 0;
                        self.last_mixed_right = 0;
                        self.last_mixed_mask = 0;
                    }
                    trace.push(ApuTraceEvent::MasterToggle {
                        enabled,
                        nr52: self.read(0xFF26),
                    });
                    trace.push(ApuTraceEvent::MixerControlWrite {
                        nr50: self.regs[0x14],
                        nr51: self.regs[0x15],
                        nr52: self.read(0xFF26),
                    });
                }
            }
            0xFF30..=0xFF3F => {
                let requested_index = (addr - 0xFF30) as usize;
                if self.wave_ram_access_blocked() {
                    trace.push(ApuTraceEvent::WaveRamAccessAliased {
                        requested_index: requested_index as u8,
                        actual_index: 0xFF,
                        is_write: true,
                    });
                    return trace;
                }
                let actual_index = self.wave_ram_visible_index(requested_index);
                self.wave_ram[actual_index] = value;
                trace.push(ApuTraceEvent::WaveRamWrite {
                    index: actual_index as u8,
                    value,
                });
                if requested_index != actual_index {
                    trace.push(ApuTraceEvent::WaveRamAccessAliased {
                        requested_index: requested_index as u8,
                        actual_index: actual_index as u8,
                        is_write: true,
                    });
                }
            }
            _ => {}
        }
        trace
    }

    pub fn tick(&mut self, cycles: u32) -> ApuTickResult {
        let mut out = ApuTickResult::default();
        let mut remaining = cycles;
        let mut generated_frames = 0u16;
        let mut dropped_frames = 0u16;
        while remaining > 0 {
            let chunk = self.next_timed_chunk(remaining);
            self.process_timed_chunk(
                chunk,
                &mut out.trace,
                true,
                &mut generated_frames,
                &mut dropped_frames,
            );
            remaining -= u32::from(chunk);
        }
        if generated_frames > 0 {
            out.trace.push(ApuTraceEvent::PcmFramesBuffered {
                frames: generated_frames,
                buffered_frames: self.buffered_frames() as u32,
            });
        }
        if dropped_frames > 0 {
            out.trace.push(ApuTraceEvent::PcmBufferWrapped {
                dropped_frames,
                dropped_total: self.pcm_frames_dropped,
            });
        }
        self.emit_mixed_output_if_changed(&mut out.trace);
        out
    }

    pub fn tick_fast(&mut self, cycles: u32) {
        let mut remaining = cycles;
        let mut dropped_frames = 0u16;
        let mut generated_frames = 0u16;
        let mut dummy_trace = Vec::new();
        while remaining > 0 {
            let chunk = self.next_timed_chunk(remaining);
            self.process_timed_chunk(
                chunk,
                &mut dummy_trace,
                false,
                &mut generated_frames,
                &mut dropped_frames,
            );
            dummy_trace.clear();
            remaining -= u32::from(chunk);
        }
        self.emit_mixed_output_if_changed(&mut dummy_trace);
    }

    pub fn frame_sequencer_step(&self) -> u8 {
        self.frame_seq_step
    }

    pub(crate) fn sync_divider_phase(&mut self, divider: u16, double_speed: bool) {
        self.frame_seq_cycles = (divider >> u8::from(double_speed)) & 0x1FFF;
    }

    pub(crate) fn reset_divider(&mut self, divider: u16, double_speed: bool) -> Vec<ApuTraceEvent> {
        self.sync_divider_phase(divider, double_speed);
        let mut trace = Vec::new();
        if self.frame_seq_cycles & 0x1000 != 0 {
            self.clock_frame_sequencer(&mut trace, true);
        }
        self.frame_seq_cycles = 0;
        trace
    }

    pub(crate) fn observe_divider_only(
        &mut self,
        before: u16,
        after: u16,
        double_speed: bool,
    ) -> Vec<ApuTraceEvent> {
        let mask = 1 << (12 + u8::from(double_speed));
        let mut trace = Vec::new();
        if before & mask != 0 && after & mask == 0 {
            self.clock_frame_sequencer(&mut trace, true);
        }
        self.sync_divider_phase(after, double_speed);
        trace
    }

    pub fn output_sample_rate(&self) -> u32 {
        APU_OUTPUT_SAMPLE_RATE
    }

    pub fn set_cgb_mode(&mut self, cgb_mode: bool) {
        self.cgb_mode = cgb_mode;
    }

    pub fn buffered_frames(&self) -> usize {
        self.pcm_buffer.len() / 2
    }

    pub fn generated_frames(&self) -> u64 {
        self.pcm_frames_generated
    }

    pub fn dropped_frames(&self) -> u64 {
        self.pcm_frames_dropped
    }

    pub fn buffer_capacity_frames(&self) -> usize {
        self.pcm_buffer_capacity_frames
    }

    pub fn set_buffer_capacity_frames(&mut self, capacity_frames: usize) -> Result<(), String> {
        if capacity_frames == 0 {
            return Err("audio buffer capacity must be non-zero".to_string());
        }
        self.pcm_buffer_capacity_frames = capacity_frames;
        let target_samples = capacity_frames.saturating_mul(2);
        while self.pcm_buffer.len() > target_samples {
            let _ = self.pcm_buffer.pop_front();
        }
        if self.pcm_buffer.capacity() < target_samples {
            self.pcm_buffer
                .reserve(target_samples.saturating_sub(self.pcm_buffer.capacity()));
        }
        Ok(())
    }

    pub fn drain_interleaved_i16(&mut self, max_frames: usize) -> Vec<i16> {
        let frames = max_frames.min(self.buffered_frames());
        let mut out = Vec::with_capacity(frames * 2);
        for _ in 0..(frames * 2) {
            if let Some(sample) = self.pcm_buffer.pop_front() {
                out.push(sample);
            }
        }
        out
    }

    pub fn pcm12(&self) -> u8 {
        self.channel_digital_output(1) | (self.channel_digital_output(2) << 4)
    }

    pub fn pcm34(&self) -> u8 {
        self.channel_digital_output(3) | (self.channel_digital_output(4) << 4)
    }

    fn next_timed_chunk(&self, remaining: u32) -> u16 {
        let mut chunk = remaining.min(u32::from(u16::MAX)) as u16;
        chunk = chunk.min((APU_PCM_SAMPLE_CYCLES - self.pcm_sample_cycles).max(1));
        chunk = chunk.min((APU_FRAME_SEQUENCER_CYCLES - self.frame_seq_cycles).max(1));
        chunk = chunk.min((APU_MIX_TRACE_CYCLES - self.mix_trace_cycles).max(1));
        chunk.max(1)
    }

    fn process_timed_chunk(
        &mut self,
        chunk: u16,
        trace: &mut Vec<ApuTraceEvent>,
        emit_trace: bool,
        generated_frames: &mut u16,
        dropped_frames: &mut u16,
    ) {
        let (mixed_left, mixed_right) = self.advance_channels(chunk);
        self.pcm_sample_cycles = self.pcm_sample_cycles.saturating_add(chunk);
        self.pcm_accum_left += mixed_left;
        self.pcm_accum_right += mixed_right;
        self.pcm_accum_cycles = self.pcm_accum_cycles.saturating_add(chunk);
        if self.pcm_sample_cycles >= APU_PCM_SAMPLE_CYCLES {
            self.pcm_sample_cycles -= APU_PCM_SAMPLE_CYCLES;
            self.enqueue_pcm_frame(dropped_frames);
            *generated_frames = generated_frames.saturating_add(1);
        }
        self.frame_seq_cycles += chunk;
        if self.frame_seq_cycles >= APU_FRAME_SEQUENCER_CYCLES {
            self.frame_seq_cycles -= APU_FRAME_SEQUENCER_CYCLES;
            self.clock_frame_sequencer(trace, emit_trace);
        }
        if !self.master_enabled {
            return;
        }
        self.mix_trace_cycles += chunk;
        if self.mix_trace_cycles >= APU_MIX_TRACE_CYCLES {
            self.mix_trace_cycles %= APU_MIX_TRACE_CYCLES;
            if emit_trace {
                self.emit_mixed_output_if_changed(trace);
            }
        }
    }

    fn clock_frame_sequencer(&mut self, trace: &mut Vec<ApuTraceEvent>, emit_trace: bool) {
        if !self.master_enabled {
            return;
        }
        self.frame_seq_step = (self.frame_seq_step + 1) & 7;
            if emit_trace {
                trace.push(ApuTraceEvent::FrameSequencerStep {
                    step: self.frame_seq_step,
                });
            }
        if self.frame_seq_step & 1 == 0 {
            self.clock_length(trace);
            }
        if matches!(self.frame_seq_step, 2 | 6) {
            self.clock_sweep(trace);
            }
            if self.frame_seq_step == 7 {
                self.clock_envelopes(trace);
        }
    }

    fn apply_register_side_effects(
        &mut self,
        addr: u16,
        value: u8,
        trace: &mut Vec<ApuTraceEvent>,
    ) {
        match addr {
            0xFF10 => {
                self.channels[0].ensure_control_latches();
                let old_negate = self.channels[0].sweep_negate;
                let used_negate = self.channels[0].sweep_negate_used;
                let was_enabled = self.channels[0].enabled;
                {
                    let st = &mut self.channels[0];
                    st.sweep_period = (value >> 4) & 0x07;
                    st.sweep_negate = value & 0x08 != 0;
                    st.sweep_shift = value & 0x07;
                }
                if old_negate && (value & 0x08 == 0) && used_negate && was_enabled {
                    self.channels[0].enabled = false;
                    self.channel_enable &= !0x01;
                    trace.push(ApuTraceEvent::ChannelDisabled {
                        channel: 1,
                        reason: 4,
                        reg: 0xFF10,
                    });
                }
            }
            0xFF11 => self.channels[0].length_counter = decode_length_square(value),
            0xFF12 => self.update_envelope_from_reg(1, value, trace),
            0xFF13 | 0xFF14 => self.sync_frequency_from_regs(1),
            0xFF16 => self.channels[1].length_counter = decode_length_square(value),
            0xFF17 => self.update_envelope_from_reg(2, value, trace),
            0xFF18 | 0xFF19 => self.sync_frequency_from_regs(2),
            0xFF1A => self.update_dac_state(3, value & 0x80 != 0, 0xFF1A, trace),
            0xFF1B => self.channels[2].length_counter = decode_length_wave(value),
            0xFF1D | 0xFF1E => self.sync_frequency_from_regs(3),
            0xFF20 => self.channels[3].length_counter = decode_length_square(value),
            0xFF21 => self.update_envelope_from_reg(4, value, trace),
            0xFF22 => self.sync_frequency_from_regs(4),
            _ => {}
        }
    }

    fn update_envelope_from_reg(&mut self, channel: u8, value: u8, trace: &mut Vec<ApuTraceEvent>) {
        let idx = (channel - 1) as usize;
        let st = &mut self.channels[idx];
        st.ensure_control_latches();
        if st.enabled {
            let old_volume = st.envelope_volume;
            if self.cgb_mode {
                // Documented CGB-02/04 zombie behavior. DMG revisions differ.
                if st.envelope_period == 0 && st.envelope_running() {
                    st.envelope_volume += 1;
                } else if !st.envelope_increase {
                    st.envelope_volume += 2;
                }
                if st.envelope_increase != (value & 0x08 != 0) {
                    st.envelope_volume = 16u8.wrapping_sub(st.envelope_volume);
                }
                st.envelope_volume &= 0x0F;
            } else if st.envelope_period == 0 && st.envelope_increase && value & 0x0F == 0x08 {
                // The portable manual-volume case is consistent across DMGs.
                st.envelope_volume = (st.envelope_volume + 1) & 0x0F;
            }
            if st.envelope_volume != old_volume {
                trace.push(ApuTraceEvent::ChannelEnvelopeStep {
                    channel,
                    volume: st.envelope_volume,
                    increasing: st.envelope_volume > old_volume,
                });
            }
        }
        st.envelope_increase = value & 0x08 != 0;
        st.envelope_period = value & 0x07;
        let dac_enabled = value & 0xF8 != 0;
        self.update_dac_state(
            channel,
            dac_enabled,
            match channel {
                1 => 0xFF12,
                2 => 0xFF17,
                4 => 0xFF21,
                _ => 0x0000,
            },
            trace,
        );
    }

    fn update_dac_state(
        &mut self,
        channel: u8,
        dac_enabled: bool,
        reg: u16,
        trace: &mut Vec<ApuTraceEvent>,
    ) {
        let idx = (channel - 1) as usize;
        let old_dac = self.channels[idx].dac_enabled;
        let was_active = self.channels[idx].enabled && (self.channel_enable & (1 << idx) != 0);
        self.channels[idx].dac_enabled = dac_enabled;
        if old_dac != dac_enabled {
            trace.push(ApuTraceEvent::DacStateChange {
                channel,
                enabled: dac_enabled,
                reg,
                active: was_active,
            });
            if self.master_enabled {
                trace.push(ApuTraceEvent::PopRisk { source: 1, reg });
            }
        }
        if !dac_enabled {
            self.channels[idx].enabled = false;
            self.channel_enable &= !(1 << idx);
            trace.push(ApuTraceEvent::ChannelDisabled {
                channel,
                reason: 1,
                reg,
            });
        }
    }

    fn trigger_channel(
        &mut self,
        channel: u8,
        trace: &mut Vec<ApuTraceEvent>,
        reg: u16,
        value: u8,
    ) {
        let idx = (channel - 1) as usize;
        let max_length = max_length(channel);
        let extra_length_clock = self.frame_seq_step & 1 == 0 && self.length_enabled(channel);
        let freq = self.current_frequency(channel);
        let period = self.period_for_channel(channel).max(1);
        let sweep_period = (self.regs[0] >> 4) & 0x07;
        let sweep_negate = self.regs[0] & 0x08 != 0;
        let sweep_shift = self.regs[0] & 0x07;
        let env_reg = match channel {
            1 => self.regs[2],
            2 => self.regs[7],
            4 => self.regs[0x11],
            _ => 0,
        };
        let buffered_sample_before = self.channels[idx].wave_sample_buffer;
        if channel == 3 {
            self.corrupt_wave_ram_on_retrigger();
            let st = &mut self.channels[idx];
            // Keep the playing nibble when resetting the wave index, including
            // legacy states that did not store a separate nibble selector.
            if st.duty_position & 0x80 == 0 {
                st.duty_position = 0x80 | (st.wave_position & 1);
            }
            st.envelope_counter = 0;
        }
        {
            let st = &mut self.channels[idx];
            st.enabled = st.dac_enabled;
            if st.length_counter == 0 {
                st.length_counter = max_length - u16::from(extra_length_clock);
            }
            st.shadow_frequency = freq;
            st.freq_timer = match channel {
                1 | 2 => (period & !3) | (st.freq_timer & 3),
                3 => period + 6,
                _ => period,
            };
            st.wave_position = 0;
            st.lfsr = 0x7FFF;
            if channel == 1 {
                st.sweep_period = sweep_period;
                st.sweep_negate = sweep_negate;
                st.sweep_shift = sweep_shift;
                st.sweep_counter = if st.sweep_period == 0 {
                    8
                } else {
                    st.sweep_period
                };
                st.sweep_negate_used = false;
            }
            if matches!(channel, 1 | 2 | 4) {
                st.wave_sample_buffer = CONTROL_LATCHES_VALID | ENVELOPE_RUNNING;
                if channel == 1 && (sweep_period != 0 || sweep_shift != 0) {
                    st.wave_sample_buffer |= SWEEP_ENABLED;
                }
                st.envelope_volume = (env_reg >> 4) & 0x0F;
                st.envelope_increase = env_reg & 0x08 != 0;
                st.envelope_period = env_reg & 0x07;
                st.envelope_counter = if st.envelope_period == 0 {
                    8
                } else {
                    st.envelope_period
                } + u8::from(self.frame_seq_step == 6);
            }
        }
        if channel == 4 {
            self.set_noise_timer(self.noise_period());
        }
        if !self.channels[idx].enabled {
            self.channel_enable &= !(1 << idx);
            trace.push(ApuTraceEvent::ChannelDisabled {
                channel,
                reason: 1,
                reg,
            });
            return;
        }
        self.channel_enable |= 1 << idx;
        trace.push(ApuTraceEvent::ChannelTrigger {
            channel,
            reg,
            value,
        });
        if channel == 3 {
            trace.push(ApuTraceEvent::Ch3TriggerRetainsSample {
                buffered_sample: buffered_sample_before,
                next_index: 1,
            });
        }
        if channel == 1 && sweep_shift != 0 {
            let delta = freq >> sweep_shift;
            let next = if sweep_negate {
                self.channels[0].sweep_negate_used = true;
                freq.saturating_sub(delta)
            } else {
                freq.saturating_add(delta)
            };
            if next > 2047 {
                self.channels[0].enabled = false;
                self.channel_enable &= !0x01;
                trace.push(ApuTraceEvent::ChannelDisabled {
                    channel: 1,
                    reason: 3,
                    reg: 0xFF10,
                });
            }
        }
        if channel == 4 {
            let nr43 = self.regs[0x12];
            let shift = nr43 >> 4;
            if shift >= 14 {
                trace.push(ApuTraceEvent::NoiseClockFrozen {
                    shift,
                    divisor_code: nr43 & 0x07,
                });
            }
        }
    }

    fn sync_frequency_from_regs(&mut self, channel: u8) {
        if channel == 4 {
            if self.noise_timer() == 0 {
                self.set_noise_timer(self.noise_period());
            }
            return;
        }
        let idx = (channel - 1) as usize;
        if self.channels[idx].freq_timer == 0 {
            self.channels[idx].freq_timer = self.period_for_channel(channel).max(1);
        }
    }

    fn current_frequency(&self, channel: u8) -> u16 {
        match channel {
            1 => ((u16::from(self.regs[4] & 0x07)) << 8) | u16::from(self.regs[3]),
            2 => ((u16::from(self.regs[9] & 0x07)) << 8) | u16::from(self.regs[8]),
            3 => ((u16::from(self.regs[14] & 0x07)) << 8) | u16::from(self.regs[13]),
            _ => 0,
        }
    }

    fn set_current_frequency(&mut self, channel: u8, frequency: u16) {
        let low = (frequency & 0x00FF) as u8;
        let high = ((frequency >> 8) as u8) & 0x07;
        match channel {
            1 => {
                self.regs[3] = low;
                self.regs[4] = (self.regs[4] & !0x07) | high;
            }
            2 => {
                self.regs[8] = low;
                self.regs[9] = (self.regs[9] & !0x07) | high;
            }
            3 => {
                self.regs[13] = low;
                self.regs[14] = (self.regs[14] & !0x07) | high;
            }
            _ => {}
        }
        let idx = (channel - 1) as usize;
        self.channels[idx].shadow_frequency = frequency;
    }

    fn wave_ram_visible_index(&self, requested_index: usize) -> usize {
        if self.is_ch3_active() {
            self.current_wave_ram_byte_index()
        } else {
            requested_index.min(self.wave_ram.len().saturating_sub(1))
        }
    }

    fn wave_ram_access_blocked(&self) -> bool {
        !self.cgb_mode && self.is_ch3_active() && self.channels[2].envelope_counter == 0
    }

    fn corrupt_wave_ram_on_retrigger(&mut self) {
        if self.cgb_mode || !self.is_ch3_active() || self.channels[2].freq_timer != 2 {
            return;
        }
        let next_byte = usize::from(((self.channels[2].wave_position + 1) & 31) / 2);
        if next_byte < 4 {
            self.wave_ram[0] = self.wave_ram[next_byte];
        } else {
            let start = next_byte & !3;
            self.wave_ram.copy_within(start..start + 4, 0);
        }
    }

    fn current_wave_ram_byte_index(&self) -> usize {
        ((self.channels[2].wave_position as usize) / 2).min(self.wave_ram.len().saturating_sub(1))
    }

    fn is_ch3_active(&self) -> bool {
        self.channels[2].enabled
            && self.channels[2].dac_enabled
            && (self.channel_enable & 0x04 != 0)
    }

    fn advance_channels(&mut self, cycles: u16) -> (i32, i32) {
        if !self.master_enabled {
            return (0, 0);
    }

        let areas = [
            self.advance_square(1, cycles),
            self.advance_square(2, cycles),
            self.advance_wave(cycles),
            self.advance_noise(cycles),
        ];
        let mut left = 0;
        let mut right = 0;
        for (index, area) in areas.into_iter().enumerate() {
            if !self.channels[index].dac_enabled {
                continue;
            }
            let analog_area = 15 * i32::from(cycles) - 2 * area;
            if self.regs[0x15] & (1 << index) != 0 {
                right += analog_area;
            }
            if self.regs[0x15] & (1 << (index + 4)) != 0 {
                left += analog_area;
            }
        }
        (
            left * i32::from(((self.regs[0x14] >> 4) & 7) + 1),
            right * i32::from((self.regs[0x14] & 7) + 1),
        )
    }

    fn advance_square(&mut self, channel: u8, cycles: u16) -> i32 {
        let idx = (channel - 1) as usize;
        if !self.channels[idx].enabled {
            return 0;
        }
        let period = self.period_for_channel(channel).max(1);
        let mut remaining = u32::from(cycles);
        let mut area = 0;
        while remaining > 0 {
            if self.channels[idx].freq_timer == 0 {
                self.channels[idx].freq_timer = period;
            }
            let timer = u32::from(self.channels[idx].freq_timer.max(1));
            area += i32::from(self.channel_digital_output(channel)) * remaining.min(timer) as i32;
            if remaining < timer {
                self.channels[idx].freq_timer -= remaining as u16;
                break;
            }
            remaining -= timer;
            self.channels[idx].freq_timer = period;
            self.channels[idx].duty_position = (self.channels[idx].duty_position + 1) & 0x07;
            self.channels[idx].force_zero_until_advance = false;
        }
        area
    }

    fn advance_wave(&mut self, cycles: u16) -> i32 {
        let idx = 2usize;
        if !self.channels[idx].enabled {
            return 0;
        }
        let period = self.period_for_channel(3).max(1);
        let mut remaining = u32::from(cycles);
        let mut area = 0;
        while remaining > 0 {
            if self.channels[idx].freq_timer == 0 {
                self.channels[idx].freq_timer = period;
            }
            let timer = u32::from(self.channels[idx].freq_timer.max(1));
            let elapsed = remaining.min(timer);
            area += i32::from(self.channel_digital_output(3)) * elapsed as i32;
            self.channels[idx].envelope_counter = self.channels[idx]
                .envelope_counter
                .saturating_sub(elapsed.min(2) as u8);
            if remaining < timer {
                self.channels[idx].freq_timer -= remaining as u16;
                break;
            }
            remaining -= timer;
            self.channels[idx].freq_timer = period;
            self.channels[idx].wave_position = (self.channels[idx].wave_position + 1) & 0x1F;
            let sample_index = self.channels[idx].wave_position as usize;
            self.channels[idx].wave_sample_buffer = self.wave_ram[sample_index / 2];
            self.channels[idx].duty_position = 0x80 | (sample_index as u8 & 1);
            self.channels[idx].envelope_counter = 2;
        }
        area
    }

    fn noise_period(&self) -> u32 {
        let nr43 = self.regs[0x12];
        u32::from(NOISE_DIVISORS[(nr43 & 7) as usize]) << (nr43 >> 4)
    }

    fn noise_timer(&self) -> u32 {
        (u32::from(self.channels[3].shadow_frequency) << 16)
            | u32::from(self.channels[3].freq_timer)
    }

    fn set_noise_timer(&mut self, cycles: u32) {
        self.channels[3].freq_timer = cycles as u16;
        self.channels[3].shadow_frequency = (cycles >> 16) as u16;
    }

    fn advance_noise(&mut self, cycles: u16) -> i32 {
        let idx = 3usize;
        if !self.channels[idx].enabled {
            return 0;
        }
        let nr43 = self.regs[0x12];
        if nr43 >> 4 >= 14 {
            return i32::from(self.channel_digital_output(4)) * i32::from(cycles);
        }
        let period = self.noise_period();
        let mut remaining = u32::from(cycles);
        let mut area = 0;
        while remaining > 0 {
            if self.noise_timer() == 0 {
                self.set_noise_timer(period);
            }
            let timer = self.noise_timer();
            area += i32::from(self.channel_digital_output(4)) * remaining.min(timer) as i32;
            if remaining < timer {
                self.set_noise_timer(timer - remaining);
                break;
            }
            remaining -= timer;
            self.set_noise_timer(period);
            let mut lfsr = self.channels[idx].lfsr;
            let xor_bit = (lfsr ^ (lfsr >> 1)) & 0x01;
            lfsr = (lfsr >> 1) | (xor_bit << 14);
            if self.regs[0x12] & 0x08 != 0 {
                lfsr = (lfsr & !(1 << 6)) | (xor_bit << 6);
            }
            self.channels[idx].lfsr = lfsr;
        }
        area
    }

    fn clock_length(&mut self, trace: &mut Vec<ApuTraceEvent>) {
        for channel in 1..=4 {
            if self.length_enabled(channel) {
                self.clock_channel_length(channel, true, trace);
            }
        }
    }

    fn clock_channel_length(
        &mut self,
        channel: u8,
        disable_on_expiry: bool,
        trace: &mut Vec<ApuTraceEvent>,
    ) {
        let idx = usize::from(channel - 1);
        let st = &mut self.channels[idx];
        if st.length_counter > 0 {
            st.length_counter -= 1;
            if st.length_counter == 0 && disable_on_expiry {
                if st.enabled {
                    trace.push(ApuTraceEvent::ChannelLengthExpired { channel });
                }
                st.enabled = false;
                self.channel_enable &= !(1 << idx);
            }
        }
    }

    fn clock_envelopes(&mut self, trace: &mut Vec<ApuTraceEvent>) {
        for channel in [1u8, 2u8, 4u8] {
            let idx = (channel - 1) as usize;
            let st = &mut self.channels[idx];
            if !st.enabled || !st.dac_enabled {
                continue;
            }
            st.ensure_control_latches();
            if st.envelope_counter > 0 {
                st.envelope_counter -= 1;
            }
            if st.envelope_counter == 0 {
                st.envelope_counter = if st.envelope_period == 0 {
                    8
                } else {
                    st.envelope_period
                };
                if st.envelope_period == 0 || !st.envelope_running() {
                    continue;
                }
                let old = st.envelope_volume;
                if (st.envelope_increase && old == 15) || (!st.envelope_increase && old == 0) {
                    st.wave_sample_buffer &= !ENVELOPE_RUNNING;
                    continue;
                }
                st.envelope_volume = if st.envelope_increase {
                    old + 1
                } else {
                    old - 1
                };
                if st.envelope_volume != old {
                    trace.push(ApuTraceEvent::ChannelEnvelopeStep {
                        channel,
                        volume: st.envelope_volume,
                        increasing: st.envelope_increase,
                    });
                }
            }
        }
    }

    fn clock_sweep(&mut self, trace: &mut Vec<ApuTraceEvent>) {
        if !self.channels[0].enabled || !self.channels[0].dac_enabled {
            return;
        }
        self.channels[0].ensure_control_latches();
        if self.channels[0].sweep_counter > 0 {
            self.channels[0].sweep_counter -= 1;
        }
        if self.channels[0].sweep_counter > 0 {
            return;
        }
        self.channels[0].sweep_counter = if self.channels[0].sweep_period == 0 {
            8
        } else {
            self.channels[0].sweep_period
        };
        if self.channels[0].wave_sample_buffer & SWEEP_ENABLED == 0
            || self.channels[0].sweep_period == 0
        {
            return;
        }
        let old = self.channels[0].shadow_frequency;
        let shift = self.channels[0].sweep_shift;
        let negate = self.channels[0].sweep_negate;
        let delta = old >> shift;
        let next = if negate {
            self.channels[0].sweep_negate_used = true;
            old.saturating_sub(delta)
        } else {
            old.saturating_add(delta)
        };
        if next > 2047 {
            self.channels[0].enabled = false;
            self.channel_enable &= !0x01;
            trace.push(ApuTraceEvent::ChannelDisabled {
                channel: 1,
                reason: 3,
                reg: 0xFF10,
            });
            return;
        }
        if shift == 0 {
            return;
        }
        let delta2 = next >> shift;
        let second = if negate {
            next.saturating_sub(delta2)
        } else {
            next.saturating_add(delta2)
        };
        self.set_current_frequency(1, next);
        if second > 2047 {
            self.channels[0].enabled = false;
            self.channel_enable &= !0x01;
            trace.push(ApuTraceEvent::ChannelDisabled {
                channel: 1,
                reason: 3,
                reg: 0xFF10,
            });
        }
        self.channels[0].shadow_frequency = next;
        trace.push(ApuTraceEvent::ChannelSweepStep {
            old_frequency: old,
            new_frequency: next,
            negate,
            shift,
        });
    }

    fn emit_mixed_output_if_changed(&mut self, trace: &mut Vec<ApuTraceEvent>) {
        let (left, right, active_mask) = self.mix_output();
        if left != self.last_mixed_left
            || right != self.last_mixed_right
            || active_mask != self.last_mixed_mask
        {
            self.last_mixed_left = left;
            self.last_mixed_right = right;
            self.last_mixed_mask = active_mask;
            trace.push(ApuTraceEvent::MixedOutput {
                left,
                right,
                active_mask,
            });
        }
    }

    fn mix_output(&self) -> (i16, i16, u8) {
        if !self.master_enabled {
            return (0, 0, 0);
        }
        let nr50 = self.regs[0x14];
        let nr51 = self.regs[0x15];
        let left_vol = i16::from(((nr50 >> 4) & 0x07) + 1);
        let right_vol = i16::from((nr50 & 0x07) + 1);
        let mut left = 0i16;
        let mut right = 0i16;
        let mut active_mask = 0u8;
        for channel in 1..=4 {
            let sample = self.channel_analog_output(channel);
            let idx = (channel - 1) as usize;
            if self.channels[idx].enabled && self.channel_enable & (1 << idx) != 0 {
                active_mask |= 1 << (channel - 1);
            }
            if nr51 & (1 << (channel - 1)) != 0 {
                right += sample * right_vol;
            }
            if nr51 & (1 << (channel + 3)) != 0 {
                left += sample * left_vol;
            }
        }
        (left, right, active_mask & self.channel_enable)
    }

    fn channel_analog_output(&self, channel: u8) -> i16 {
        let idx = (channel - 1) as usize;
        let st = &self.channels[idx];
        if !st.dac_enabled {
            return 0;
        }
        let digital = if st.enabled && self.channel_enable & (1 << idx) != 0 {
            i16::from(self.channel_digital_output(channel))
        } else {
            0
        };
        15 - (digital * 2)
    }

    fn any_channel_dac_enabled(&self) -> bool {
        self.channels.iter().any(|channel| channel.dac_enabled)
    }

    fn hpf_charge_factor(&self) -> f64 {
        if self.cgb_mode {
            APU_HPF_SAMPLE_FACTOR_CGB
        } else {
            APU_HPF_SAMPLE_FACTOR_DMG
        }
    }

    fn output_lpf_alpha(&self) -> f64 {
        if self.cgb_mode {
            APU_OUTPUT_LPF_ALPHA_CGB
        } else {
            APU_OUTPUT_LPF_ALPHA_DMG
        }
    }

    fn apply_output_low_pass(&mut self, left: f64, right: f64) -> (f64, f64) {
        let alpha = self.output_lpf_alpha();
        self.pcm_lpf_left += (left - self.pcm_lpf_left) * alpha;
        self.pcm_lpf_right += (right - self.pcm_lpf_right) * alpha;
        (self.pcm_lpf_left, self.pcm_lpf_right)
    }

    fn channel_digital_output(&self, channel: u8) -> u8 {
        let idx = (channel - 1) as usize;
        let st = &self.channels[idx];
        if !st.enabled || !st.dac_enabled || self.channel_enable & (1 << idx) == 0 {
            return 0;
        }
        match channel {
            1 => square_digital_output(self.regs[1], st) as u8,
            2 => square_digital_output(self.regs[6], st) as u8,
            3 => match (self.regs[0x0C] >> 5) & 0x03 {
                0 => 0,
                1 => wave_channel_sample(st),
                2 => wave_channel_sample(st) >> 1,
                _ => wave_channel_sample(st) >> 2,
            },
            _ => {
                if st.lfsr & 0x01 == 0 {
                    st.envelope_volume
                } else {
                    0
                }
            }
        }
    }

    fn enqueue_pcm_frame(&mut self, dropped_frames: &mut u16) {
        let has_partial_signal = self.pcm_accum_left != 0 || self.pcm_accum_right != 0;
        let accum_cycles = f64::from(self.pcm_accum_cycles.max(1));
        let raw_left = f64::from(self.pcm_accum_left) * 64.0 / accum_cycles;
        let raw_right = f64::from(self.pcm_accum_right) * 64.0 / accum_cycles;
        self.pcm_accum_left = 0;
        self.pcm_accum_right = 0;
        self.pcm_accum_cycles = 0;
        let (filtered_left, filtered_right) =
            if self.any_channel_dac_enabled() || has_partial_signal {
            let charge = self.hpf_charge_factor();
            let out_left = raw_left - self.pcm_hpf_left;
            let out_right = raw_right - self.pcm_hpf_right;
            self.pcm_hpf_left = raw_left - out_left * charge;
            self.pcm_hpf_right = raw_right - out_right * charge;
                let (out_left, out_right) = self.apply_output_low_pass(out_left, out_right);
                (clamp_pcm_i16(out_left), clamp_pcm_i16(out_right))
        } else {
                self.pcm_lpf_left = 0.0;
                self.pcm_lpf_right = 0.0;
            (0, 0)
        };
        while self.buffered_frames() >= self.pcm_buffer_capacity_frames {
            let _ = self.pcm_buffer.pop_front();
            let _ = self.pcm_buffer.pop_front();
            self.pcm_frames_dropped = self.pcm_frames_dropped.saturating_add(1);
            *dropped_frames = dropped_frames.saturating_add(1);
        }
        self.pcm_buffer.push_back(filtered_left);
        self.pcm_buffer.push_back(filtered_right);
        self.pcm_frames_generated = self.pcm_frames_generated.saturating_add(1);
    }

    fn period_for_channel(&self, channel: u8) -> u16 {
        match channel {
            1 | 2 => {
                let freq = self.current_frequency(channel).min(2047);
                (2048u16.saturating_sub(freq)).max(1) * 4
            }
            3 => {
                let freq = self.current_frequency(3).min(2047);
                (2048u16.saturating_sub(freq)).max(1) * 2
            }
            _ => 0,
        }
    }

    fn length_enabled(&self, channel: u8) -> bool {
        match channel {
            1 => self.regs[4] & 0x40 != 0,
            2 => self.regs[9] & 0x40 != 0,
            3 => self.regs[14] & 0x40 != 0,
            _ => self.regs[0x13] & 0x40 != 0,
        }
    }
}

fn square_digital_output(duty_reg: u8, st: &ChannelState) -> i16 {
    if st.force_zero_until_advance {
        return 0;
    }
    let duty = (duty_reg >> 6) as usize;
    let bit = DUTY_PATTERNS[duty][st.duty_position as usize];
    if bit != 0 {
        i16::from(st.envelope_volume)
    } else {
        0
    }
}

fn wave_channel_sample(st: &ChannelState) -> u8 {
    let selector = if st.duty_position & 0x80 != 0 {
        st.duty_position
    } else {
        st.wave_position
    };
    if selector & 1 == 0 {
        st.wave_sample_buffer >> 4
    } else {
        st.wave_sample_buffer & 0x0F
    }
}

fn clamp_pcm_i16(sample: f64) -> i16 {
    sample
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn decode_length_square(value: u8) -> u16 {
    let raw = 64 - u16::from(value & 0x3F);
    if raw == 0 {
        64
    } else {
        raw
    }
}

fn decode_length_wave(value: u8) -> u16 {
    let raw = 256 - u16::from(value);
    if raw == 0 {
        256
    } else {
        raw
    }
}

fn max_length(channel: u8) -> u16 {
    if channel == 3 {
        256
    } else {
        64
    }
}

fn trigger_channel(addr: u16) -> Option<u8> {
    match addr {
        0xFF14 => Some(1),
        0xFF19 => Some(2),
        0xFF1E => Some(3),
        0xFF23 => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_toggle_clears_channel_state() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF12, 0xF3);
        apu.write(0xFF14, 0x80);
        assert_eq!(apu.channel_enable & 0x01, 0x01);
        let trace = apu.write(0xFF26, 0x00);
        assert!(trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::MasterToggle { enabled: false, .. })));
        assert_eq!(apu.channel_enable, 0);
    }

    #[test]
    fn frame_sequencer_advances_only_when_enabled() {
        let mut apu = Apu::default();
        assert!(!apu
            .tick(8192)
            .trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::FrameSequencerStep { .. })));
        assert_eq!(apu.frame_sequencer_step(), 7);
        apu.write(0xFF26, 0x80);
        let trace = apu.tick(8192).trace;
        assert!(trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::FrameSequencerStep { step: 0 })));
    }

    #[test]
    fn mixer_write_and_mixed_output_are_observable() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        let trace = apu.write(0xFF24, 0x77);
        assert!(trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::MixerControlWrite { nr50: 0x77, .. })));
        apu.write(0xFF25, 0x11);
        apu.write(0xFF11, 0xC0);
        apu.write(0xFF12, 0xF3);
        apu.write(0xFF13, 0xFF);
        apu.write(0xFF14, 0x80);
        let trace = apu.tick(2048).trace;
        assert!(trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::MixedOutput { .. })));
    }

    #[test]
    fn pcm_frames_are_buffered_and_drained() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF24, 0x77);
        apu.write(0xFF25, 0x11);
        apu.write(0xFF11, 0xC0);
        apu.write(0xFF12, 0xF3);
        apu.write(0xFF13, 0xFF);
        apu.write(0xFF14, 0x80);
        let trace = apu.tick(512).trace;
        assert!(trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::PcmFramesBuffered { .. })));
        assert!(apu.buffered_frames() > 0);
        let drained = apu.drain_interleaved_i16(apu.buffered_frames());
        assert!(!drained.is_empty());
        assert_eq!(apu.buffered_frames(), 0);
    }

    #[test]
    fn pcm_buffer_capacity_can_be_resized() {
        let mut apu = Apu::default();
        assert_eq!(
            apu.buffer_capacity_frames(),
            default_pcm_buffer_capacity_frames()
        );
        apu.set_buffer_capacity_frames(64)
            .expect("resize audio buffer");
        assert_eq!(apu.buffer_capacity_frames(), 64);
        assert!(apu.set_buffer_capacity_frames(0).is_err());
    }

    #[test]
    fn pcm_registers_reflect_current_channel_outputs() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF24, 0x77);
        apu.write(0xFF25, 0x11);
        apu.write(0xFF11, 0xC0);
        apu.write(0xFF12, 0xF3);
        apu.write(0xFF13, 0xFF);
        apu.write(0xFF14, 0x87);
        apu.tick(132);
        assert_ne!(apu.pcm12() & 0x0F, 0);
    }

    #[test]
    fn cgb_wave_ram_access_aliases_current_byte_while_ch3_active() {
        let mut apu = Apu::default();
        apu.set_cgb_mode(true);
        apu.write(0xFF26, 0x80);
        apu.write(0xFF1A, 0x80);
        apu.write(0xFF1C, 0x20);
        apu.write(0xFF1E, 0x80);
        apu.channels[2].enabled = true;
        apu.channel_enable |= 0x04;
        apu.channels[2].wave_position = 6;
        let trace = apu.write(0xFF3F, 0x9A);
        assert_eq!(apu.wave_ram[3], 0x9A);
        assert!(trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::WaveRamAccessAliased {
                requested_index: 0x0F,
                actual_index: 0x03,
                is_write: true
            }
        )));
    }

    #[test]
    fn dmg_wave_ram_reads_ff_and_writes_are_ignored_while_ch3_active() {
        let mut apu = Apu::default();
        apu.set_cgb_mode(false);
        apu.write(0xFF26, 0x80);
        apu.write(0xFF1A, 0x80);
        apu.write(0xFF1C, 0x20);
        apu.write(0xFF1E, 0x80);
        apu.channels[2].enabled = true;
        apu.channel_enable |= 0x04;
        apu.wave_ram[0] = 0x12;
        let read_back = apu.read(0xFF30);
        let trace = apu.write(0xFF30, 0x9A);
        assert_eq!(read_back, 0xFF);
        assert_eq!(apu.wave_ram[0], 0x12);
        assert!(trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::WaveRamAccessAliased {
                requested_index: 0x00,
                actual_index: 0xFF,
                is_write: true
            }
        )));
    }

    #[test]
    fn ch3_trigger_retains_existing_sample_buffer_byte_and_skips_first_sample() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF1A, 0x80);
        apu.write(0xFF1C, 0x20);
        apu.channels[2].wave_sample_buffer = 0xBC;
        apu.wave_ram[0] = 0x12;
        let trace = apu.write(0xFF1E, 0x80);
        assert_eq!(apu.channels[2].wave_sample_buffer, 0xBC);
        assert_eq!(apu.channel_digital_output(3), 0x0B);
        apu.advance_wave(apu.period_for_channel(3) + 6);
        assert_eq!(apu.channels[2].wave_position, 1);
        assert_eq!(apu.channels[2].wave_sample_buffer, 0x12);
        assert_eq!(apu.channel_digital_output(3), 0x02);
        assert!(trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::Ch3TriggerRetainsSample {
                buffered_sample: 0xBC,
                next_index: 1
            }
        )));
    }

    #[test]
    fn disabling_dac_reports_pop_risk() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF12, 0xF3);
        let trace = apu.write(0xFF12, 0x00);
        assert!(trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::DacStateChange {
                channel: 1,
                enabled: false,
                ..
            }
        )));
        assert!(trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::PopRisk {
                source: 1,
                reg: 0xFF12
            }
        )));
    }

    #[test]
    fn high_shift_noise_mode_reports_frozen_clock() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF21, 0xF3);
        apu.write(0xFF22, 0xE0);
        let trace = apu.write(0xFF23, 0x80);
        assert!(trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::NoiseClockFrozen {
                shift: 14,
                divisor_code: 0
            }
        )));
    }

    #[test]
    fn hpf_disconnects_when_all_dacs_are_off() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.pcm_hpf_left = 1234.0;
        apu.pcm_hpf_right = -432.0;
        apu.pcm_accum_left = 0;
        apu.pcm_accum_right = 0;
        apu.pcm_accum_cycles = APU_PCM_SAMPLE_CYCLES;
        let mut dropped = 0;
        apu.enqueue_pcm_frame(&mut dropped);
        assert_eq!(apu.drain_interleaved_i16(1), vec![0, 0]);
        assert_eq!(apu.pcm_hpf_left, 1234.0);
        assert_eq!(apu.pcm_hpf_right, -432.0);
    }

    #[test]
    fn output_low_pass_smooths_abrupt_pcm_edges() {
        let mut apu = Apu::default();

        let (first_left, first_right) = apu.apply_output_low_pass(1_000.0, -1_000.0);
        let (second_left, second_right) = apu.apply_output_low_pass(1_000.0, -1_000.0);
        let (reversed_left, reversed_right) = apu.apply_output_low_pass(-1_000.0, 1_000.0);

        assert!(first_left > 0.0 && first_left < 1_000.0);
        assert!(first_right < 0.0 && first_right > -1_000.0);
        assert!(second_left > first_left && second_left < 1_000.0);
        assert!(second_right < first_right && second_right > -1_000.0);
        assert!(reversed_left < second_left && reversed_left > -1_000.0);
        assert!(reversed_right > second_right && reversed_right < 1_000.0);
    }

    #[test]
    fn cgb_hpf_rejects_constant_dc_input() {
        let mut apu = Apu::default();
        apu.set_cgb_mode(true);
        apu.write(0xFF26, 0x80);
        apu.channels[0].dac_enabled = true;
        let mut dropped = 0;
        let mut drained = vec![0, 0];

        for _ in 0..120 {
            apu.pcm_accum_left = 32 * i32::from(APU_PCM_SAMPLE_CYCLES);
            apu.pcm_accum_right = 32 * i32::from(APU_PCM_SAMPLE_CYCLES);
            apu.pcm_accum_cycles = APU_PCM_SAMPLE_CYCLES;
            apu.enqueue_pcm_frame(&mut dropped);
            drained = apu.drain_interleaved_i16(1);
        }

        assert!(drained[0].abs() < 2);
        assert!(drained[1].abs() < 2);
    }

    #[test]
    fn noise_channel_produces_varying_pcm_frames() {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(0xFF24, 0x77);
        apu.write(0xFF25, 0x88);
        apu.write(0xFF21, 0xF3);
        apu.write(0xFF22, 0x03);
        apu.write(0xFF23, 0x80);
        apu.tick(2048);
        let drained = apu.drain_interleaved_i16(apu.buffered_frames());
        assert!(!drained.is_empty());
        assert!(drained.iter().any(|sample| *sample != 0));
        assert!(drained.windows(2).any(|pair| pair[0] != pair[1]));
    }
}
