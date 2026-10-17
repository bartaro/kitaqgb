use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::types::{ApuTraceEvent, HardwareMode};

const APU_FRAME_SEQUENCER_CYCLES: u16 = 8192;
const APU_MIX_TRACE_CYCLES: u16 = 1024;
const APU_PCM_SAMPLE_CYCLES: u16 = 64;
const APU_PCM_OVERSAMPLE_CYCLES: u16 = 16;
pub const APU_OUTPUT_SAMPLE_RATE: u32 = 65_536;
const DEFAULT_APU_PCM_BUFFER_CAPACITY_FRAMES: usize = 65_536;
const APU_HPF_CHARGE_FACTOR_DMG: f64 = 0.997_315_553_139_174_5;
const APU_HPF_CHARGE_FACTOR_CGB: f64 = 0.934_555_954_774_141;
const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];
const NOISE_DIVISORS: [u16; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

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
            force_zero_until_advance: false,
            sweep_negate_used: false,
        }
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
            frame_seq_step: 0,
            channels: [ChannelState::default(); 4],
            mix_trace_cycles: 0,
            pcm_sample_cycles: 0,
            pcm_buffer: VecDeque::with_capacity(default_pcm_buffer_capacity_frames() * 2),
            pcm_buffer_capacity_frames: default_pcm_buffer_capacity_frames(),
            pcm_frames_generated: 0,
            pcm_frames_dropped: 0,
            pcm_hpf_left: 0.0,
            pcm_hpf_right: 0.0,
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
        self.pcm_accum_left = 0;
        self.pcm_accum_right = 0;
        self.pcm_accum_cycles = 0;
        self.last_mixed_left = 0;
        self.last_mixed_right = 0;
        self.last_mixed_mask = 0;
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF25 => self.regs[(addr - 0xFF10) as usize],
            0xFF26 => 0x70 | (u8::from(self.master_enabled) << 7) | (self.channel_enable & 0x0F),
            0xFF30..=0xFF3F => {
                let requested = (addr - 0xFF30) as usize;
                if !self.cgb_mode && self.is_ch3_active() {
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
                        self.frame_seq_cycles = 0;
                        self.frame_seq_step = 0;
                        self.channels = [ChannelState::default(); 4];
                        self.mix_trace_cycles = 0;
                        self.pcm_sample_cycles = 0;
                        self.pcm_buffer.clear();
                        self.pcm_hpf_left = 0.0;
                        self.pcm_hpf_right = 0.0;
                        self.pcm_accum_left = 0;
                        self.pcm_accum_right = 0;
                        self.pcm_accum_cycles = 0;
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
                if !self.cgb_mode && self.is_ch3_active() {
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
        if !self.master_enabled {
            return out;
        }
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
        if !self.master_enabled {
            return;
        }
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
        chunk = chunk.min(APU_PCM_OVERSAMPLE_CYCLES);
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
        self.advance_channels(chunk);
        self.frame_seq_cycles = self.frame_seq_cycles.saturating_add(chunk);
        self.mix_trace_cycles = self.mix_trace_cycles.saturating_add(chunk);
        self.pcm_sample_cycles = self.pcm_sample_cycles.saturating_add(chunk);
        let (mixed_left, mixed_right, _) = self.mix_output();
        self.pcm_accum_left = self
            .pcm_accum_left
            .saturating_add(i32::from(mixed_left) * i32::from(chunk));
        self.pcm_accum_right = self
            .pcm_accum_right
            .saturating_add(i32::from(mixed_right) * i32::from(chunk));
        self.pcm_accum_cycles = self.pcm_accum_cycles.saturating_add(chunk);
        if self.pcm_sample_cycles >= APU_PCM_SAMPLE_CYCLES {
            self.pcm_sample_cycles -= APU_PCM_SAMPLE_CYCLES;
            self.enqueue_pcm_frame(dropped_frames);
            *generated_frames = generated_frames.saturating_add(1);
        }
        if self.frame_seq_cycles >= APU_FRAME_SEQUENCER_CYCLES {
            self.frame_seq_cycles -= APU_FRAME_SEQUENCER_CYCLES;
            self.frame_seq_step = (self.frame_seq_step + 1) & 0x07;
            if emit_trace {
                trace.push(ApuTraceEvent::FrameSequencerStep {
                    step: self.frame_seq_step,
                });
            }
            match self.frame_seq_step {
                0 | 2 | 4 | 6 => self.clock_length(trace),
                _ => {}
            }
            match self.frame_seq_step {
                2 | 6 => self.clock_sweep(trace),
                _ => {}
            }
            if self.frame_seq_step == 7 {
                self.clock_envelopes(trace);
            }
        }
        if self.mix_trace_cycles >= APU_MIX_TRACE_CYCLES {
            self.mix_trace_cycles %= APU_MIX_TRACE_CYCLES;
            if emit_trace {
                self.emit_mixed_output_if_changed(trace);
            }
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
        st.envelope_volume = (value >> 4) & 0x0F;
        st.envelope_increase = value & 0x08 != 0;
        st.envelope_period = value & 0x07;
        st.envelope_counter = if st.envelope_period == 0 {
            8
        } else {
            st.envelope_period
        };
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
        if !self.channels[idx].dac_enabled {
            self.channels[idx].enabled = false;
            self.channel_enable &= !(1 << idx);
            trace.push(ApuTraceEvent::ChannelDisabled {
                channel,
                reason: 1,
                reg,
            });
            return;
        }
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
        {
            let st = &mut self.channels[idx];
            st.enabled = true;
            if st.length_counter == 0 {
                st.length_counter = max_length;
            }
            st.shadow_frequency = freq;
            st.freq_timer = period;
            st.duty_position = 0;
            st.wave_position = 0;
            if matches!(channel, 1 | 2) {
                st.force_zero_until_advance = true;
            }
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
                st.envelope_volume = (env_reg >> 4) & 0x0F;
                st.envelope_increase = env_reg & 0x08 != 0;
                st.envelope_period = env_reg & 0x07;
                st.envelope_counter = if st.envelope_period == 0 {
                    8
                } else {
                    st.envelope_period
                };
            }
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
        let idx = (channel - 1) as usize;
        self.channels[idx].shadow_frequency = self.current_frequency(channel);
        if self.channels[idx].freq_timer == 0 {
            self.channels[idx].freq_timer = self.period_for_channel(channel).max(1);
        }
    }

    fn current_frequency(&self, channel: u8) -> u16 {
        match channel {
            1 => ((u16::from(self.regs[4] & 0x07)) << 8) | u16::from(self.regs[3]),
            2 => ((u16::from(self.regs[9] & 0x07)) << 8) | u16::from(self.regs[8]),
            3 => ((u16::from(self.regs[14] & 0x07)) << 8) | u16::from(self.regs[13]),
            _ => self.channels[3].shadow_frequency,
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
        self.channels[idx].freq_timer = self.period_for_channel(channel).max(1);
    }

    fn wave_ram_visible_index(&self, requested_index: usize) -> usize {
        if self.cgb_mode && self.is_ch3_active() {
            self.current_wave_ram_byte_index()
        } else {
            requested_index.min(self.wave_ram.len().saturating_sub(1))
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

    fn advance_channels(&mut self, cycles: u16) {
        self.advance_square(1, cycles);
        self.advance_square(2, cycles);
        self.advance_wave(cycles);
        self.advance_noise(cycles);
    }

    fn advance_square(&mut self, channel: u8, cycles: u16) {
        let idx = (channel - 1) as usize;
        if !self.channels[idx].enabled {
            return;
        }
        let period = self.period_for_channel(channel).max(1);
        let mut remaining = u32::from(cycles);
        while remaining > 0 {
            if self.channels[idx].freq_timer == 0 {
                self.channels[idx].freq_timer = period;
            }
            let timer = u32::from(self.channels[idx].freq_timer.max(1));
            if remaining < timer {
                self.channels[idx].freq_timer -= remaining as u16;
                break;
            }
            remaining -= timer;
            self.channels[idx].freq_timer = period;
            self.channels[idx].duty_position = (self.channels[idx].duty_position + 1) & 0x07;
            self.channels[idx].force_zero_until_advance = false;
        }
    }

    fn advance_wave(&mut self, cycles: u16) {
        let idx = 2usize;
        if !self.channels[idx].enabled {
            return;
        }
        let period = self.period_for_channel(3).max(1);
        let mut remaining = u32::from(cycles);
        while remaining > 0 {
            if self.channels[idx].freq_timer == 0 {
                self.channels[idx].freq_timer = period;
            }
            let timer = u32::from(self.channels[idx].freq_timer.max(1));
            if remaining < timer {
                self.channels[idx].freq_timer -= remaining as u16;
                break;
            }
            remaining -= timer;
            self.channels[idx].freq_timer = period;
            self.channels[idx].wave_position = (self.channels[idx].wave_position + 1) & 0x1F;
            let sample_index = self.channels[idx].wave_position as usize;
            self.channels[idx].wave_sample_buffer = self.wave_ram[sample_index / 2];
        }
    }

    fn advance_noise(&mut self, cycles: u16) {
        let idx = 3usize;
        if !self.channels[idx].enabled {
            return;
        }
        let nr43 = self.regs[0x12];
        if nr43 >> 4 >= 14 {
            return;
        }
        let period = self.period_for_channel(4).max(1);
        let mut remaining = u32::from(cycles);
        while remaining > 0 {
            if self.channels[idx].freq_timer == 0 {
                self.channels[idx].freq_timer = period;
            }
            let timer = u32::from(self.channels[idx].freq_timer.max(1));
            if remaining < timer {
                self.channels[idx].freq_timer -= remaining as u16;
                break;
            }
            remaining -= timer;
            self.channels[idx].freq_timer = period;
            let mut lfsr = self.channels[idx].lfsr;
            let xor_bit = (lfsr ^ (lfsr >> 1)) & 0x01;
            lfsr = (lfsr >> 1) | (xor_bit << 14);
            if self.regs[0x12] & 0x08 != 0 {
                lfsr = (lfsr & !(1 << 6)) | (xor_bit << 6);
            }
            self.channels[idx].lfsr = lfsr;
        }
    }

    fn clock_length(&mut self, trace: &mut Vec<ApuTraceEvent>) {
        for channel in 1..=4 {
            let idx = (channel - 1) as usize;
            if !self.channels[idx].enabled || !self.length_enabled(channel) {
                continue;
            }
            if self.channels[idx].length_counter > 0 {
                self.channels[idx].length_counter -= 1;
                if self.channels[idx].length_counter == 0 {
                    self.channels[idx].enabled = false;
                    self.channel_enable &= !(1 << idx);
                    trace.push(ApuTraceEvent::ChannelLengthExpired { channel });
                }
            }
        }
    }

    fn clock_envelopes(&mut self, trace: &mut Vec<ApuTraceEvent>) {
        for channel in [1u8, 2u8, 4u8] {
            let idx = (channel - 1) as usize;
            let st = &mut self.channels[idx];
            if !st.enabled || !st.dac_enabled || st.envelope_period == 0 {
                continue;
            }
            if st.envelope_counter > 0 {
                st.envelope_counter -= 1;
            }
            if st.envelope_counter == 0 {
                st.envelope_counter = if st.envelope_period == 0 {
                    8
                } else {
                    st.envelope_period
                };
                let old = st.envelope_volume;
                if st.envelope_increase {
                    st.envelope_volume = st.envelope_volume.saturating_add(1).min(15);
                } else {
                    st.envelope_volume = st.envelope_volume.saturating_sub(1);
                }
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
        if self.channels[0].sweep_period == 0 && self.channels[0].sweep_shift == 0 {
            return;
        }
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
        if self.channels[0].sweep_shift == 0 {
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
            APU_HPF_CHARGE_FACTOR_CGB
        } else {
            APU_HPF_CHARGE_FACTOR_DMG
        }
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
        let accum_cycles = i32::from(self.pcm_accum_cycles.max(1));
        let averaged_left = self.pcm_accum_left / accum_cycles;
        let averaged_right = self.pcm_accum_right / accum_cycles;
        self.pcm_accum_left = 0;
        self.pcm_accum_right = 0;
        self.pcm_accum_cycles = 0;
        let raw_left = f64::from(averaged_left * 64);
        let raw_right = f64::from(averaged_right * 64);
        let (filtered_left, filtered_right) = if self.any_channel_dac_enabled() {
            let charge = self.hpf_charge_factor();
            let out_left = raw_left - self.pcm_hpf_left;
            let out_right = raw_right - self.pcm_hpf_right;
            self.pcm_hpf_left = raw_left - out_left * charge;
            self.pcm_hpf_right = raw_right - out_right * charge;
            (
                out_left
                    .round()
                    .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16,
                out_right
                    .round()
                    .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16,
            )
        } else {
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
            _ => {
                let nr43 = self.regs[0x12];
                let shift = nr43 >> 4;
                let divisor = NOISE_DIVISORS[(nr43 & 0x07) as usize];
                divisor
                    .checked_shl(u32::from(shift))
                    .unwrap_or(u16::MAX)
                    .max(8)
            }
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
    if st.wave_position & 1 == 0 {
        st.wave_sample_buffer >> 4
    } else {
        st.wave_sample_buffer & 0x0F
    }
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
        assert!(apu.tick(8192).trace.is_empty());
        apu.write(0xFF26, 0x80);
        let trace = apu.tick(8192).trace;
        assert!(trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::FrameSequencerStep { step: 1 })));
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
        apu.advance_wave(apu.period_for_channel(3));
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
        apu.pcm_accum_left = 5120;
        apu.pcm_accum_right = -4096;
        apu.pcm_accum_cycles = APU_PCM_SAMPLE_CYCLES;
        let mut dropped = 0;
        apu.enqueue_pcm_frame(&mut dropped);
        assert_eq!(apu.drain_interleaved_i16(1), vec![0, 0]);
        assert_eq!(apu.pcm_hpf_left, 1234.0);
        assert_eq!(apu.pcm_hpf_right, -432.0);
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
