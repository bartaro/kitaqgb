use std::{fs, path::Path};

use anyhow::{bail, Result};
use gbdaw_export_kitaqgb::{preview_stream_json, ExportPolicy, StreamCommand, StreamMode};
use gbdaw_model::{Channel, Instrument, Project};
use kokura_core::apu::Apu;
use serde::{Deserialize, Serialize};

const GB_FRAME_TCYCLES: u32 = 70_224;
const GB_FREQ_TABLE: [u16; 48] = [
    44, 156, 262, 363, 457, 547, 631, 710, 786, 854, 923, 986, 1046, 1102, 1155, 1205, 1253, 1297,
    1339, 1379, 1417, 1452, 1486, 1517, 1546, 1575, 1602, 1627, 1650, 1673, 1694, 1714, 1732, 1750,
    1767, 1783, 1798, 1812, 1825, 1837, 1849, 1860, 1871, 1881, 1890, 1899, 1907, 1915,
];

pub fn default_output_sample_rate() -> u32 {
    KokuraApuHost::default().audio_sample_rate()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApuSnapshot {
    pub nr10_to_nr14: [u8; 5],
    pub nr21_to_nr24: [u8; 4],
    pub nr30_to_nr34: [u8; 5],
    pub nr41_to_nr44: [u8; 4],
    pub nr50: u8,
    pub nr51: u8,
    pub nr52: u8,
    pub wave_ram: [u8; 16],
    pub channel_meters: [u8; 4],
    pub cycle_counter: u64,
}

#[derive(Debug, Clone)]
pub struct KokuraApuHost {
    apu: Apu,
    cycle_counter: u64,
    ch1_duty: u8,
    ch2_duty: u8,
    ch1_env: u8,
    ch2_env: u8,
    ch4_env: u8,
    ch4_param: u8,
    ch3_level: u8,
    ch1_sweep: u8,
}

impl Default for KokuraApuHost {
    fn default() -> Self {
        let mut apu = Apu::default();
        apu.set_cgb_mode(true);
        apu.write(0xFF26, 0x80);
        apu.write(0xFF24, 0x77);
        apu.write(0xFF25, 0xFF);
        let mut host = Self {
            apu,
            cycle_counter: 0,
            ch1_duty: 2,
            ch2_duty: 2,
            ch1_env: 0xF1,
            ch2_env: 0xF2,
            ch4_env: 0xF1,
            ch4_param: 0x00,
            ch3_level: 0x20,
            ch1_sweep: 0x00,
        };
        host.load_wave_preset(0);
        host
    }
}

impl KokuraApuHost {
    pub fn advance_tcycles(&mut self, cycles: u32) {
        self.apu.tick(cycles);
        self.cycle_counter += cycles as u64;
    }

    pub fn write_apu_reg(&mut self, addr: u16, value: u8) {
        self.apu.write(addr, value);
    }

    pub fn read_apu_reg(&self, addr: u16) -> u8 {
        self.apu.read(addr)
    }

    pub fn write_wave_ram(&mut self, index: u8, value: u8) {
        self.apu.write(0xFF30 + index as u16, value);
    }

    pub fn reset_apu_only(&mut self) {
        *self = Self::default();
    }

    pub fn drain_audio_frames_interleaved_i16(&mut self, max_frames: usize) -> Vec<i16> {
        self.apu.drain_interleaved_i16(max_frames)
    }

    pub fn audio_sample_rate(&self) -> u32 {
        self.apu.output_sample_rate()
    }

    pub fn capture_apu_snapshot(&self) -> ApuSnapshot {
        let mut nr10_to_nr14 = [0u8; 5];
        for (i, slot) in nr10_to_nr14.iter_mut().enumerate() {
            *slot = self.apu.read(0xFF10 + i as u16);
        }
        let mut nr21_to_nr24 = [0u8; 4];
        for (i, slot) in nr21_to_nr24.iter_mut().enumerate() {
            *slot = self.apu.read(0xFF16 + i as u16);
        }
        let mut nr30_to_nr34 = [0u8; 5];
        for (i, slot) in nr30_to_nr34.iter_mut().enumerate() {
            *slot = self.apu.read(0xFF1A + i as u16);
        }
        let mut nr41_to_nr44 = [0u8; 4];
        for (i, slot) in nr41_to_nr44.iter_mut().enumerate() {
            *slot = self.apu.read(0xFF20 + i as u16);
        }
        let mut wave_ram = [0u8; 16];
        for (i, slot) in wave_ram.iter_mut().enumerate() {
            *slot = self.apu.read(0xFF30 + i as u16);
        }
        let pcm12 = self.apu.read(0xFF76);
        let pcm34 = self.apu.read(0xFF77);
        ApuSnapshot {
            nr10_to_nr14,
            nr21_to_nr24,
            nr30_to_nr34,
            nr41_to_nr44,
            nr50: self.apu.read(0xFF24),
            nr51: self.apu.read(0xFF25),
            nr52: self.apu.read(0xFF26),
            wave_ram,
            channel_meters: [
                pcm12 & 0x0F,
                (pcm12 >> 4) & 0x0F,
                pcm34 & 0x0F,
                (pcm34 >> 4) & 0x0F,
            ],
            cycle_counter: self.cycle_counter,
        }
    }

    pub fn set_master_volume(&mut self, left: u8, right: u8) {
        let left = (left.min(7)) << 4;
        let right = right.min(7);
        self.apu.write(0xFF24, left | right);
    }

    pub fn set_pan(&mut self, ch: u8, pan: u8) {
        let (r_bit, l_bit) = match ch {
            0 => (0x01, 0x10),
            1 => (0x02, 0x20),
            2 => (0x04, 0x40),
            _ => (0x08, 0x80),
        };
        let mask = r_bit | l_bit;
        let mut nr51 = self.apu.read(0xFF25) & !mask;
        match pan {
            0 => nr51 |= mask,
            1 => nr51 |= l_bit,
            2 => nr51 |= r_bit,
            _ => {}
        }
        self.apu.write(0xFF25, nr51);
    }

    pub fn set_ch1_duty(&mut self, duty: u8) {
        self.ch1_duty = duty & 3;
    }
    pub fn set_ch2_duty(&mut self, duty: u8) {
        self.ch2_duty = duty & 3;
    }
    pub fn set_ch3_level(&mut self, level: u8) {
        self.ch3_level = level;
    }
    pub fn set_ch4_param(&mut self, param: u8) {
        self.ch4_param = param;
    }
    pub fn set_ch1_sweep(&mut self, sweep: u8) {
        self.ch1_sweep = sweep;
    }
    pub fn set_inst(&mut self, ch: u8, arg0: u8, arg1: u8) {
        match ch {
            0 => {
                self.ch1_duty = arg0 & 3;
                self.ch1_env = arg1;
            }
            1 => {
                self.ch2_duty = arg0 & 3;
                self.ch2_env = arg1;
            }
            2 => {
                self.ch3_level = arg1;
            }
            3 => {
                self.ch4_env = arg1;
            }
            _ => {}
        }
    }

    pub fn load_wave_preset(&mut self, wave_id: u8) {
        let bytes = match wave_id {
            0 => [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
                0x32, 0x10,
            ],
            1 => [
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
                0xFF, 0x00,
            ],
            2 => [
                0x86, 0x42, 0x10, 0x01, 0x12, 0x46, 0x8A, 0xCE, 0xFF, 0xCE, 0x8A, 0x46, 0x12, 0x01,
                0x10, 0x42,
            ],
            3 => [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99,
                0x88, 0x77,
            ],
            4 => [
                0x78, 0x9A, 0xB9, 0x87, 0x65, 0x43, 0x21, 0x10, 0x01, 0x12, 0x34, 0x56, 0x78, 0x9A,
                0xB9, 0x87,
            ],
            5 => [
                0x8A, 0xCF, 0xD9, 0x84, 0x10, 0x01, 0x12, 0x46, 0x8A, 0xCF, 0xD9, 0x84, 0x10, 0x01,
                0x12, 0x46,
            ],
            6 => [
                0x45, 0x67, 0x89, 0x98, 0x76, 0x54, 0x32, 0x10, 0x01, 0x23, 0x45, 0x67, 0x89, 0x98,
                0x76, 0x54,
            ],
            _ => [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
                0x32, 0x10,
            ],
        };
        self.load_custom_wave(bytes);
    }

    pub fn load_custom_wave(&mut self, bytes: [u8; 16]) {
        self.apu.write(0xFF1A, 0x00);
        for (i, byte) in bytes.iter().copied().enumerate() {
            self.apu.write(0xFF30 + i as u16, byte);
        }
        self.apu.write(0xFF1A, 0x80);
    }

    pub fn note_on(&mut self, ch: u8, value: u8) {
        match ch {
            0 => self.ch1_note_on(value, self.ch1_env, self.ch1_duty),
            1 => self.ch2_note_on(value, self.ch2_env, self.ch2_duty),
            2 => self.ch3_note_on(value, self.ch3_level),
            3 => self.ch4_note_on(value, self.ch4_env),
            _ => {}
        }
    }

    pub fn stop_channel(&mut self, ch: u8) {
        match ch {
            0 => {
                self.apu.write(0xFF12, 0);
                self.apu.write(0xFF14, 0x80);
            }
            1 => {
                self.apu.write(0xFF17, 0);
                self.apu.write(0xFF19, 0x80);
            }
            2 => {
                self.apu.write(0xFF1C, 0);
                self.apu.write(0xFF1A, 0x00);
            }
            3 => {
                self.apu.write(0xFF21, 0);
                self.apu.write(0xFF23, 0x80);
            }
            _ => {}
        }
    }

    fn ch1_note_on(&mut self, note: u8, vol_env: u8, duty2: u8) {
        let freq = GB_FREQ_TABLE[note.min(47) as usize];
        let duty_reg = match duty2 & 3 {
            0 => 0,
            1 => 64,
            3 => 192,
            _ => 128,
        };
        self.apu.write(0xFF10, self.ch1_sweep);
        self.apu.write(0xFF11, duty_reg);
        self.apu.write(0xFF12, vol_env);
        self.apu.write(0xFF13, (freq & 0xFF) as u8);
        self.apu.write(0xFF14, ((freq >> 8) as u8) | 0x80);
    }

    fn ch2_note_on(&mut self, note: u8, vol_env: u8, duty2: u8) {
        let freq = GB_FREQ_TABLE[note.min(47) as usize];
        let duty_reg = match duty2 & 3 {
            0 => 0,
            1 => 64,
            3 => 192,
            _ => 128,
        };
        self.apu.write(0xFF16, duty_reg);
        self.apu.write(0xFF17, vol_env);
        self.apu.write(0xFF18, (freq & 0xFF) as u8);
        self.apu.write(0xFF19, ((freq >> 8) as u8) | 0x80);
    }

    fn ch3_note_on(&mut self, note: u8, level: u8) {
        let freq = GB_FREQ_TABLE[note.min(47) as usize];
        self.apu.write(0xFF1A, 0x80);
        self.apu.write(0xFF1B, 0x00);
        self.apu.write(0xFF1C, level);
        self.apu.write(0xFF1D, (freq & 0xFF) as u8);
        self.apu.write(0xFF1E, ((freq >> 8) as u8) | 0x80);
    }

    fn ch4_note_on(&mut self, param: u8, vol_env: u8) {
        self.ch4_param = param;
        self.apu.write(0xFF20, 0x20);
        self.apu.write(0xFF21, vol_env);
        self.apu.write(0xFF22, self.ch4_param);
        self.apu.write(0xFF23, 0x80);
    }
}

#[derive(Debug, Clone)]
pub struct ProjectAudioRendererState {
    host: KokuraApuHost,
    event_index: usize,
    current_frame: u32,
    finished: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectAudioSeekIndex {
    pub interval_frames: u32,
    checkpoints: Vec<ProjectAudioRendererState>,
}

impl ProjectAudioSeekIndex {
    pub fn build(project: &Project, mode: StreamMode, interval_frames: u32) -> Result<Self> {
        let interval_frames = interval_frames.max(1);
        let mut renderer = ProjectAudioRenderer::new(project, mode)?;
        let mut checkpoints = vec![renderer.capture_state()];
        let max_frames = estimate_project_frames(project).max(interval_frames);
        while !renderer.is_finished() && renderer.current_frame() < max_frames {
            let target = (renderer.current_frame() + interval_frames).min(max_frames);
            renderer.skip_to_frame(target)?;
            checkpoints.push(renderer.capture_state());
            if renderer.current_frame() >= max_frames {
                break;
            }
        }
        Ok(Self {
            interval_frames,
            checkpoints,
        })
    }

    pub fn checkpoint_for_frame(&self, target_frame: u32) -> Option<&ProjectAudioRendererState> {
        self.checkpoints
            .iter()
            .take_while(|checkpoint| checkpoint.current_frame <= target_frame)
            .last()
    }
}

#[derive(Debug, Clone)]
pub struct ProjectAudioRenderer {
    host: KokuraApuHost,
    events: Vec<StreamCommand>,
    event_index: usize,
    current_frame: u32,
    finished: bool,
}

impl ProjectAudioRenderer {
    pub fn new(project: &Project, mode: StreamMode) -> Result<Self> {
        let preview = preview_stream_json(project, mode, ExportPolicy::default())?;
        Ok(Self {
            host: KokuraApuHost::default(),
            events: preview.events,
            event_index: 0,
            current_frame: 0,
            finished: false,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.host.audio_sample_rate()
    }

    pub fn current_frame(&self) -> u32 {
        self.current_frame
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn snapshot(&self) -> ApuSnapshot {
        self.host.capture_apu_snapshot()
    }

    pub fn reset(&mut self) {
        self.host = KokuraApuHost::default();
        self.event_index = 0;
        self.current_frame = 0;
        self.finished = false;
    }

    pub fn capture_state(&self) -> ProjectAudioRendererState {
        ProjectAudioRendererState {
            host: self.host.clone(),
            event_index: self.event_index,
            current_frame: self.current_frame,
            finished: self.finished,
        }
    }

    pub fn restore_state(&mut self, state: &ProjectAudioRendererState) {
        self.host = state.host.clone();
        self.event_index = state.event_index;
        self.current_frame = state.current_frame;
        self.finished = state.finished;
    }

    pub fn seek_to_frame_with_preroll(
        &mut self,
        target_frame: u32,
        preroll_frames: u32,
        seek_index: Option<&ProjectAudioSeekIndex>,
    ) -> Result<u32> {
        let preroll_start = target_frame.saturating_sub(preroll_frames);
        self.reset();
        if let Some(checkpoint) =
            seek_index.and_then(|index| index.checkpoint_for_frame(preroll_start))
        {
            self.restore_state(checkpoint);
        }
        self.skip_to_frame(preroll_start)?;
        let warmup_frames = target_frame.saturating_sub(self.current_frame);
        if warmup_frames > 0 {
            let _ = self.render_next_frames(warmup_frames)?;
        }
        Ok(self.current_frame)
    }

    pub fn skip_to_frame(&mut self, target_frame: u32) -> Result<()> {
        while self.current_frame < target_frame && !self.finished {
            self.advance_one_frame(None)?;
        }
        Ok(())
    }

    pub fn render_next_frames(&mut self, frames: u32) -> Result<Vec<i16>> {
        let mut samples = Vec::new();
        for _ in 0..frames {
            if self.finished {
                break;
            }
            self.advance_one_frame(Some(&mut samples))?;
        }
        Ok(samples)
    }

    fn advance_one_frame(&mut self, out: Option<&mut Vec<i16>>) -> Result<()> {
        while self.event_index < self.events.len()
            && self.events[self.event_index].frame == self.current_frame
        {
            let event = &self.events[self.event_index];
            match event.cmd.as_str() {
                "SET_MASTER_VOLUME" => self.host.set_master_volume(event.args[0], event.args[1]),
                "SET_PAN" => self.host.set_pan(event.args[0], event.args[1]),
                "SET_CH1_DUTY" => self.host.set_ch1_duty(event.args[0]),
                "SET_CH2_DUTY" => self.host.set_ch2_duty(event.args[0]),
                "SET_SWEEP" => self.host.set_ch1_sweep(event.args[0]),
                "SET_CH3_LEVEL" => self.host.set_ch3_level(event.args[0]),
                "SET_WAVE" => self.host.load_wave_preset(event.args[0]),
                "SET_CH3_CUSTOM_WAVE" => {
                    let mut bytes = [0u8; 16];
                    bytes.copy_from_slice(&event.args[..16]);
                    self.host.load_custom_wave(bytes);
                }
                "SET_CH4_PARAM" => self.host.set_ch4_param(event.args[0]),
                "SET_INST" => self
                    .host
                    .set_inst(event.args[0], event.args[1], event.args[2]),
                "NOTE" => self.host.note_on(event.args[0], event.args[1]),
                "STOP_CHANNEL" => self.host.stop_channel(event.args[0]),
                "LOOP" => {}
                "STOP" => self.finished = true,
                "WAIT" => {}
                other => bail!("unsupported preview command: {other}"),
            }
            self.event_index += 1;
        }
        if self.finished {
            return Ok(());
        }
        self.host.advance_tcycles(GB_FRAME_TCYCLES);
        let drained = self
            .host
            .drain_audio_frames_interleaved_i16(self.host.audio_sample_rate() as usize);
        if let Some(out) = out {
            out.extend(drained);
        }
        self.current_frame += 1;
        Ok(())
    }
}

pub fn estimate_project_frames(project: &Project) -> u32 {
    let base = project.song.order_list.len() as u32
        * project.song.rows_per_pattern as u32
        * project.song.ticks_per_row as u32
        * project.song.frames_per_tick as u32;
    base.max(1) + 60
}

pub fn render_project_song_to_samples(
    project: &Project,
    mode: StreamMode,
) -> Result<(u32, Vec<i16>, ApuSnapshot)> {
    render_project_to_samples(project, mode, estimate_project_frames(project))
}

pub fn render_preview_note_samples(
    project: &Project,
    channel: Channel,
    note_value: u8,
    instrument_ref: Option<&str>,
    frames: u32,
) -> Result<(u32, Vec<i16>, ApuSnapshot)> {
    let mut host = KokuraApuHost::default();
    host.set_master_volume(project.mixer.master_left, project.mixer.master_right);

    if let Some(instrument) = instrument_ref.and_then(|id| project.instrument_by_id(id)) {
        apply_preview_instrument(&mut host, project, channel, instrument);
    }

    let stream_channel = channel.stream_id();
    host.note_on(stream_channel, note_value);

    let mut samples = Vec::new();
    let sustain_frames = frames.saturating_sub(12).max(1);
    for _ in 0..sustain_frames {
        host.advance_tcycles(GB_FRAME_TCYCLES);
        let available = host.audio_sample_rate() as usize;
        samples.extend(host.drain_audio_frames_interleaved_i16(available));
    }
    host.stop_channel(stream_channel);
    let release_frames = frames.saturating_sub(sustain_frames).max(12);
    for _ in 0..release_frames {
        host.advance_tcycles(GB_FRAME_TCYCLES);
        let available = host.audio_sample_rate() as usize;
        samples.extend(host.drain_audio_frames_interleaved_i16(available));
    }

    let snapshot = host.capture_apu_snapshot();
    Ok((host.audio_sample_rate(), samples, snapshot))
}

pub fn render_preview_wave_samples(
    wave_bytes16: [u8; 16],
    note_value: u8,
    level: u8,
    frames: u32,
) -> Result<(u32, Vec<i16>, ApuSnapshot)> {
    let mut host = KokuraApuHost::default();
    host.load_custom_wave(wave_bytes16);
    host.set_ch3_level(level);
    host.set_pan(Channel::Ch3.stream_id(), 0);
    host.note_on(Channel::Ch3.stream_id(), note_value);

    let mut samples = Vec::new();
    let sustain_frames = frames.saturating_sub(12).max(1);
    for _ in 0..sustain_frames {
        host.advance_tcycles(GB_FRAME_TCYCLES);
        let available = host.audio_sample_rate() as usize;
        samples.extend(host.drain_audio_frames_interleaved_i16(available));
    }
    host.stop_channel(Channel::Ch3.stream_id());
    let release_frames = frames.saturating_sub(sustain_frames).max(12);
    for _ in 0..release_frames {
        host.advance_tcycles(GB_FRAME_TCYCLES);
        let available = host.audio_sample_rate() as usize;
        samples.extend(host.drain_audio_frames_interleaved_i16(available));
    }

    let snapshot = host.capture_apu_snapshot();
    Ok((host.audio_sample_rate(), samples, snapshot))
}

fn apply_preview_instrument(
    host: &mut KokuraApuHost,
    project: &Project,
    channel: Channel,
    instrument: &Instrument,
) {
    match (channel, instrument) {
        (Channel::Ch1, Instrument::Pulse(v)) if v.channel == Channel::Ch1 => {
            host.set_inst(Channel::Ch1.stream_id(), v.duty, v.envelope);
            host.set_ch1_sweep(v.sweep);
            host.set_pan(Channel::Ch1.stream_id(), v.pan.as_u8());
        }
        (Channel::Ch2, Instrument::Pulse(v)) if v.channel == Channel::Ch2 => {
            host.set_inst(Channel::Ch2.stream_id(), v.duty, v.envelope);
            host.set_pan(Channel::Ch2.stream_id(), v.pan.as_u8());
        }
        (Channel::Ch3, Instrument::Wave(v)) => {
            if let Some(wave) = project.wave_by_id(&v.wave_ref) {
                host.load_custom_wave(wave.bytes16);
            }
            host.set_ch3_level(v.output_level);
            host.set_pan(Channel::Ch3.stream_id(), v.pan.as_u8());
        }
        (Channel::Ch4, Instrument::Noise(v)) => {
            host.set_inst(Channel::Ch4.stream_id(), 0, v.envelope);
            host.set_ch4_param(v.noise_param);
            host.set_pan(Channel::Ch4.stream_id(), v.pan.as_u8());
        }
        _ => {}
    }
}

pub fn render_project_to_samples(
    project: &Project,
    mode: StreamMode,
    max_frames: u32,
) -> Result<(u32, Vec<i16>, ApuSnapshot)> {
    let mut renderer = ProjectAudioRenderer::new(project, mode)?;
    let samples = renderer.render_next_frames(max_frames)?;
    let snapshot = renderer.snapshot();
    Ok((renderer.sample_rate(), samples, snapshot))
}

pub fn render_project_to_wav_file(
    project: &Project,
    out_path: impl AsRef<Path>,
    mode: StreamMode,
    max_frames: u32,
) -> Result<ApuSnapshot> {
    let out_path = out_path.as_ref();
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let (sample_rate, samples, snapshot) = render_project_to_samples(project, mode, max_frames)?;
    write_wav_i16_stereo(out_path, sample_rate, &samples)?;
    Ok(snapshot)
}

pub fn write_wav_i16_stereo(
    path: impl AsRef<Path>,
    sample_rate: u32,
    samples: &[i16],
) -> Result<()> {
    let path = path.as_ref();
    let channels = 2u16;
    let bits_per_sample = 16u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;
    let mut out = Vec::with_capacity(data_len as usize + 44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_len.to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, out)?;
    Ok(())
}

pub fn decode_wav_i16_stereo(path: impl AsRef<Path>) -> Result<(u32, Vec<i16>)> {
    let data = fs::read(path)?;
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        bail!("unsupported wav header")
    }
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let data_len = u32::from_le_bytes([data[40], data[41], data[42], data[43]]) as usize;
    let start = 44;
    let end = start + data_len;
    let mut samples = Vec::new();
    for chunk in data[start..end].chunks_exact(2) {
        samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok((sample_rate, samples))
}

#[cfg(test)]
mod channel_order_tests {
    use super::*;

    fn sounding_host() -> KokuraApuHost {
        let mut host = KokuraApuHost::default();
        for channel in 0..4 { host.note_on(channel, if channel == 3 { 0x35 } else { 24 + channel * 3 }); }
        host
    }

    #[test]
    fn fast_tick_preserves_pcm_and_apu_state() {
        let mut fast = sounding_host();
        let mut traced = fast.clone();
        for frame in 0..180 {
            if frame % 17 == 0 {
                fast.write_apu_reg(0xff25, if frame % 2 == 0 { 0xf3 } else { 0xcf });
                traced.write_apu_reg(0xff25, if frame % 2 == 0 { 0xf3 } else { 0xcf });
            }
            for cycles in [3, 8191, GB_FRAME_TCYCLES - 8194] {
                fast.apu.tick_fast(cycles);
                fast.cycle_counter += u64::from(cycles);
                traced.apu.tick(cycles);
                traced.cycle_counter += u64::from(cycles);
            }
            assert_eq!(fast.drain_audio_frames_interleaved_i16(usize::MAX),traced.drain_audio_frames_interleaved_i16(usize::MAX));
            assert_eq!(serde_json::to_value(fast.capture_apu_snapshot()).unwrap(),serde_json::to_value(traced.capture_apu_snapshot()).unwrap());
        }
    }

    #[test]
    #[ignore = "manual release-mode timing measurement"]
    fn benchmark_tick_paths() {
        for fast in [false,true,false,true] {
            let mut host = sounding_host();
            let started = std::time::Instant::now();
            let mut samples = 0;
            for _ in 0..600 {
                if fast { host.apu.tick_fast(GB_FRAME_TCYCLES); }
                else { host.apu.tick(GB_FRAME_TCYCLES); }
                samples += host.drain_audio_frames_interleaved_i16(usize::MAX).len();
            }
            println!("fast={fast} frames=600 samples={samples} elapsed_ms={:.3}",started.elapsed().as_secs_f64()*1000.0);
        }
    }

    #[test]
    fn stream_note_and_stop_target_the_same_physical_voice() {
        for (channel, id, mask, value) in [(Channel::Ch3, 2, 4, 33), (Channel::Ch4, 3, 8, 0x35)] {
            assert_eq!(channel.stream_id(), id);
            let mut host = KokuraApuHost::default();
            host.set_inst(id, 0, if id == 2 { 0x20 } else { 0xC0 });
            host.note_on(id, value);
            assert_eq!(host.read_apu_reg(0xFF26) & 15, mask);
            host.stop_channel(id);
            assert_eq!(host.read_apu_reg(0xFF26) & 15, 0);
        }
    }
}
