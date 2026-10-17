use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use gbdaw_model as legacy;
use serde::{Deserialize, Serialize};

/// One physical Game Boy always owns four APU channels.
pub const TRACK_COUNT: usize = 4;
pub const MAX_PLAYER_COUNT: usize = 4;
pub const MAX_TRACK_COUNT: usize = TRACK_COUNT * MAX_PLAYER_COUNT;
pub const MAX_USER_WAVES: usize = 64;
pub const WAVE_SAMPLE_COUNT: usize = 32;
pub const WAVE_PRESET_COUNT: usize = 16;
/// LINK16 advances in sixteenth-note steps: 16 steps make one 4/4 measure.
pub const STEPS_PER_MEASURE: u32 = 16;
pub const ROWS_PER_MEASURE: usize = 16;
pub const TICKS_PER_ROW: u8 = 2;
pub const MIN_MEASURES: u32 = 1;
/// 256 measures * 16 steps fits the redesigned 128 KiB LINK16 SRAM layout.
pub const MAX_MEASURES: u32 = 256;
pub const LINK16_STEP_COUNT: u32 = MAX_MEASURES * STEPS_PER_MEASURE;

// Keep the built-in CH3 waveforms byte-identical to KITAQGB lib/audio.c.
// GBDAW's extended preview/export path accepts packed 32 x 4-bit samples,
// so using these bytes preserves what the Game Boy driver will actually play.
const KITAQGB_BUILTIN_WAVES: [[u8; 16]; 7] = [
    [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ],
    [
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
        0x00,
    ],
    [
        0x86, 0x42, 0x10, 0x01, 0x12, 0x46, 0x8A, 0xCE, 0xFF, 0xCE, 0x8A, 0x46, 0x12, 0x01, 0x10,
        0x42,
    ],
    [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88,
        0x77,
    ],
    [
        0x78, 0x9A, 0xB9, 0x87, 0x65, 0x43, 0x21, 0x10, 0x01, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xB9,
        0x87,
    ],
    [
        0x8A, 0xCF, 0xD9, 0x84, 0x10, 0x01, 0x12, 0x46, 0x8A, 0xCF, 0xD9, 0x84, 0x10, 0x01, 0x12,
        0x46,
    ],
    [
        0x45, 0x67, 0x89, 0x98, 0x76, 0x54, 0x32, 0x10, 0x01, 0x23, 0x45, 0x67, 0x89, 0x98, 0x76,
        0x54,
    ],
];

const KITAQGB_BUILTIN_WAVE_NAMES: [&str; 7] = [
    "Triangle", "Saw", "Vowel A", "Vowel I", "Vowel U", "Vowel E", "Vowel O",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Link16Mode {
    #[default]
    Local4,
    Cable8,
    Dmg07_16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserWave {
    pub name: String,
    /// Thirty-two unsigned 4-bit Game Boy CH3 samples.
    pub samples: Vec<u8>,
}

impl Default for UserWave {
    fn default() -> Self {
        Self {
            name: "User Wave".to_string(),
            samples: (0..WAVE_SAMPLE_COUNT)
                .map(|index| {
                    if index < 16 {
                        index as u8
                    } else {
                        (31 - index) as u8
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WavePresetSlot {
    /// 0..6 are built-ins; 0x80..0xBF address user wave 0..63.
    pub source: u8,
    /// Game Boy CH3 output level: 1=100%, 2=50%, 3=25%.
    pub level: u8,
}

impl Default for WavePresetSlot {
    fn default() -> Self {
        Self {
            source: 0,
            level: 1,
        }
    }
}

fn default_wave_slots() -> Vec<WavePresetSlot> {
    (0..WAVE_PRESET_COUNT)
        .map(|index| {
            if index < 7 {
                WavePresetSlot {
                    source: index as u8,
                    level: 1,
                }
            } else if index < 14 {
                WavePresetSlot {
                    source: (index - 7) as u8,
                    level: 2,
                }
            } else {
                WavePresetSlot {
                    source: (index - 14) as u8,
                    level: 3,
                }
            }
        })
        .collect()
}

impl Link16Mode {
    pub const fn player_count(self) -> usize {
        match self {
            Self::Local4 => 1,
            Self::Cable8 => 2,
            Self::Dmg07_16 => 4,
        }
    }

    pub const fn track_count(self) -> usize {
        self.player_count() * TRACK_COUNT
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Local4 => "LOCAL 4CH",
            Self::Cable8 => "CABLE 8CH",
            Self::Dmg07_16 => "DMG-07 16CH",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Pulse1,
    Pulse2,
    Wave,
    Noise,
}

impl TrackKind {
    pub const ALL: [Self; TRACK_COUNT] = [Self::Pulse1, Self::Pulse2, Self::Wave, Self::Noise];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pulse1 => "PULSE 1",
            Self::Pulse2 => "PULSE 2",
            Self::Wave => "WAVE",
            Self::Noise => "NOISE",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Pulse1 => "CH1",
            Self::Pulse2 => "CH2",
            Self::Wave => "CH3",
            Self::Noise => "CH4",
        }
    }

    pub const fn default_name(self) -> &'static str {
        match self {
            Self::Pulse1 => "Lead",
            Self::Pulse2 => "Bass",
            Self::Wave => "Wave Pad",
            Self::Noise => "Drums",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanPosition {
    Left,
    #[default]
    Center,
    Right,
}

impl PanPosition {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::Center => "CENTER",
            Self::Right => "RIGHT",
        }
    }

    pub const fn shifted(self, delta: i8) -> Self {
        let index = match self {
            Self::Left => 0_i8,
            Self::Center => 1,
            Self::Right => 2,
        };
        match (index + delta).rem_euclid(3) {
            0 => Self::Left,
            1 => Self::Center,
            _ => Self::Right,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteEffects {
    pub pan: PanPosition,
    pub duty: u8,
    pub envelope_period: u8,
    pub envelope_increase: bool,
    pub sweep_period: u8,
    pub sweep_shift: u8,
    pub sweep_negate: bool,
    pub wave_level: u8,
    /// LINK16 CH3 wave/level preset, 0..15. This is stored per sequencer step.
    pub wave_preset: u8,
    pub noise_shift: u8,
    pub noise_divisor: u8,
    pub noise_width7: bool,
}

impl Default for NoteEffects {
    fn default() -> Self {
        Self {
            pan: PanPosition::Center,
            duty: 2,
            envelope_period: 2,
            envelope_increase: false,
            sweep_period: 0,
            sweep_shift: 0,
            sweep_negate: false,
            wave_level: 1,
            wave_preset: 7,
            noise_shift: 4,
            noise_divisor: 3,
            noise_width7: false,
        }
    }
}

impl NoteEffects {
    pub fn for_track(kind: TrackKind) -> Self {
        let mut effects = Self::default();
        match kind {
            TrackKind::Pulse1 => {
                effects.duty = 2;
                effects.envelope_period = 1;
            }
            TrackKind::Pulse2 => {
                effects.duty = 1;
                effects.envelope_period = 2;
                effects.pan = PanPosition::Left;
            }
            TrackKind::Wave => {
                effects.pan = PanPosition::Right;
                effects.wave_level = 1;
            }
            TrackKind::Noise => {
                effects.envelope_period = 1;
                effects.noise_shift = 4;
                effects.noise_divisor = 3;
            }
        }
        effects
    }

    pub fn clamp(&mut self) {
        self.duty = self.duty.min(3);
        self.envelope_period = self.envelope_period.min(7);
        self.sweep_period = self.sweep_period.min(7);
        self.sweep_shift = self.sweep_shift.min(7);
        self.wave_level = self.wave_level.clamp(1, 3);
        self.wave_preset = self.wave_preset.min(15);
        self.noise_shift = self.noise_shift.min(15);
        self.noise_divisor = self.noise_divisor.min(7);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiNote {
    pub id: u64,
    /// Position in sixteenth-note (quarter-beat) units.
    pub start: u32,
    /// Length in sixteenth-note units. One unit is 0.25 beat.
    pub length: u32,
    /// Game Boy pitch index, 0..47.
    pub pitch: u8,
    /// Initial envelope level, 0..15.
    pub velocity: u8,
    #[serde(default)]
    pub effects: NoteEffects,
}

impl MidiNote {
    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.length)
    }

    pub fn clamp(&mut self, total_steps: u32) {
        self.length = self.length.max(1);
        self.pitch = self.pitch.min(47);
        self.velocity = self.velocity.min(15);
        self.effects.clamp();
        self.start = self.start.min(total_steps.saturating_sub(1));
        self.length = self
            .length
            .min(total_steps.saturating_sub(self.start).max(1));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub kind: TrackKind,
    pub name: String,
    pub mute: bool,
    pub solo: bool,
    pub notes: Vec<MidiNote>,
}

impl Track {
    pub fn new(kind: TrackKind) -> Self {
        Self {
            kind,
            name: kind.default_name().to_string(),
            mute: false,
            solo: false,
            notes: Vec::new(),
        }
    }

    fn sort_notes(&mut self) {
        self.notes
            .sort_by_key(|note| (note.start, note.pitch, note.id));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SequenceProject {
    pub version: String,
    pub title: String,
    pub author: String,
    pub bpm: u16,
    /// Optional exact Game Boy frame count for one sequencer step. This is
    /// intended for frame-authored SFX; normal songs derive timing from BPM.
    pub frames_per_step_override: Option<u8>,
    pub measures: u32,
    pub link16_mode: Link16Mode,
    /// Repeat the selected song range. Missing legacy fields inherit the
    /// historical looping behavior from `SequenceProject::empty()`.
    pub loop_playback: bool,
    pub tracks: Vec<Track>,
    pub user_waves: Vec<UserWave>,
    pub wave_slots: Vec<WavePresetSlot>,
    pub next_note_id: u64,
}

impl Default for SequenceProject {
    fn default() -> Self {
        Self::empty()
    }
}

impl SequenceProject {
    pub fn empty() -> Self {
        Self {
            version: "0.2.0".to_string(),
            title: "Untitled Sequence".to_string(),
            author: "Unknown".to_string(),
            bpm: 150,
            frames_per_step_override: None,
            measures: 16,
            link16_mode: Link16Mode::Local4,
            loop_playback: true,
            tracks: TrackKind::ALL.into_iter().map(Track::new).collect(),
            user_waves: Vec::new(),
            wave_slots: default_wave_slots(),
            next_note_id: 1,
        }
    }

    pub fn demo() -> Self {
        let mut project = Self {
            title: "GBSEQ Demo".to_string(),
            author: "GBSEQ".to_string(),
            measures: 16,
            ..Self::empty()
        };

        for (step, pitch) in [
            (0, 24),
            (2, 28),
            (4, 31),
            (6, 36),
            (8, 31),
            (10, 28),
            (12, 26),
            (14, 24),
        ] {
            let _ = project.insert_note_exact(0, step, 2, pitch, 15);
        }
        for (step, pitch) in [(0, 12), (8, 14), (16, 15), (24, 19)] {
            let _ = project.insert_note_exact(1, step, 8, pitch, 13);
        }
        for step in [0, 16, 32, 48] {
            let _ = project.insert_note_exact(2, step, 16, 19, 12);
        }
        for step in (0..64).step_by(2) {
            let pitch = if step % 8 == 4 { 43 } else { 35 };
            let _ = project.insert_note_exact(3, step, 1, pitch, 15);
        }
        project
    }

    pub fn total_steps(&self) -> u32 {
        self.measures.saturating_mul(STEPS_PER_MEASURE)
    }

    pub fn timing_ticks_per_row(&self) -> u8 {
        if self.frames_per_step_override.is_some() {
            1
        } else {
            TICKS_PER_ROW
        }
    }

    pub fn timing_frames_per_tick(&self) -> u8 {
        self.frames_per_step_override
            .unwrap_or_else(|| ((450.0 / f32::from(self.bpm)).round() as u8).clamp(1, 15))
    }

    pub fn frames_per_step(&self) -> u32 {
        u32::from(self.timing_ticks_per_row()) * u32::from(self.timing_frames_per_tick())
    }

    pub fn repair(&mut self) {
        // Version 0.1 projects used eighth-note units. Preserve their musical
        // timing while migrating to LINK16's sixteenth-note grid.
        if self.version.starts_with("0.1") {
            for track in &mut self.tracks {
                for note in &mut track.notes {
                    note.start = note.start.saturating_mul(2);
                    note.length = note.length.saturating_mul(2);
                }
            }
        }
        self.version = "0.2.0".to_string();
        self.measures = self.measures.clamp(MIN_MEASURES, MAX_MEASURES);
        self.bpm = self.bpm.clamp(30, 300);
        self.frames_per_step_override = self
            .frames_per_step_override
            .map(|frames| frames.clamp(1, 15));
        self.tracks.truncate(MAX_TRACK_COUNT);
        self.user_waves.truncate(MAX_USER_WAVES);
        for (index, wave) in self.user_waves.iter_mut().enumerate() {
            wave.samples.resize(WAVE_SAMPLE_COUNT, 0);
            wave.samples.truncate(WAVE_SAMPLE_COUNT);
            for sample in &mut wave.samples {
                *sample = (*sample).min(15);
            }
            if wave.name.is_empty() {
                wave.name = format!("User Wave {}", index + 1);
            }
        }
        if self.wave_slots.is_empty() {
            self.wave_slots = default_wave_slots();
        }
        self.wave_slots
            .resize(WAVE_PRESET_COUNT, WavePresetSlot::default());
        self.wave_slots.truncate(WAVE_PRESET_COUNT);
        for slot in &mut self.wave_slots {
            slot.level = slot.level.clamp(1, 3);
            if slot.source >= 0x80 {
                let index = (slot.source & 0x3F) as usize;
                if index >= self.user_waves.len() {
                    slot.source = 0;
                }
            } else {
                slot.source = slot.source.min(6);
            }
        }
        while self.tracks.len() < self.link16_mode.track_count() {
            let kind = TrackKind::ALL[self.tracks.len() % TRACK_COUNT];
            self.tracks.push(Track::new(kind));
        }
        let total_steps = self.total_steps();
        let mut max_id = 0;
        for (index, track) in self.tracks.iter_mut().enumerate() {
            track.kind = TrackKind::ALL[index % TRACK_COUNT];
            for note in &mut track.notes {
                note.clamp(total_steps);
                max_id = max_id.max(note.id);
            }
            track.sort_notes();
        }
        self.next_note_id = self.next_note_id.max(max_id.saturating_add(1)).max(1);
    }

    pub fn validate(&self) -> Result<()> {
        if !(MIN_MEASURES..=MAX_MEASURES).contains(&self.measures) {
            bail!("measures must be {MIN_MEASURES}..={MAX_MEASURES}");
        }
        if !(30..=300).contains(&self.bpm) {
            bail!("bpm must be 30..=300");
        }
        if self
            .frames_per_step_override
            .is_some_and(|frames| !(1..=15).contains(&frames))
        {
            bail!("frames_per_step_override must be 1..=15");
        }
        if self.tracks.len() < self.link16_mode.track_count()
            || self.tracks.len() > MAX_TRACK_COUNT
            || self.tracks.len() % TRACK_COUNT != 0
        {
            bail!("track count must contain complete 4-channel player groups");
        }
        if self.user_waves.len() > MAX_USER_WAVES
            || self.user_waves.iter().any(|wave| {
                wave.samples.len() != WAVE_SAMPLE_COUNT
                    || wave.samples.iter().any(|sample| *sample > 15)
            })
        {
            bail!("user waves must contain exactly 32 four-bit samples");
        }
        if self.wave_slots.len() != WAVE_PRESET_COUNT {
            bail!("CH3 must contain 16 wave preset assignments");
        }
        let total_steps = self.total_steps();
        let mut ids = HashSet::new();
        for (track_index, track) in self.tracks.iter().enumerate() {
            if track.kind != TrackKind::ALL[track_index % TRACK_COUNT] {
                bail!(
                    "track {} has the wrong Game Boy channel kind",
                    track_index + 1
                );
            }
            let mut ordered: Vec<&MidiNote> = track.notes.iter().collect();
            ordered.sort_by_key(|note| (note.start, note.id));
            let mut previous_end = 0;
            for (note_index, note) in ordered.into_iter().enumerate() {
                if !ids.insert(note.id) {
                    bail!("duplicate note id {}", note.id);
                }
                if note.length == 0 {
                    bail!("note {} has zero length", note.id);
                }
                if note.start >= total_steps || note.end() > total_steps {
                    bail!("note {} is outside the project length", note.id);
                }
                if note.pitch > 47 || note.velocity > 15 {
                    bail!("note {} has an invalid pitch or level", note.id);
                }
                if note_index > 0 && note.start < previous_end {
                    bail!("track {} contains overlapping notes", track_index + 1);
                }
                previous_end = note.end();
            }
        }
        Ok(())
    }

    pub fn note(&self, id: u64) -> Option<&MidiNote> {
        self.tracks
            .iter()
            .flat_map(|track| track.notes.iter())
            .find(|note| note.id == id)
    }

    pub fn note_mut(&mut self, id: u64) -> Option<&mut MidiNote> {
        self.tracks
            .iter_mut()
            .flat_map(|track| track.notes.iter_mut())
            .find(|note| note.id == id)
    }

    pub fn note_location(&self, id: u64) -> Option<(usize, usize)> {
        self.tracks
            .iter()
            .enumerate()
            .find_map(|(track_index, track)| {
                track
                    .notes
                    .iter()
                    .position(|note| note.id == id)
                    .map(|note_index| (track_index, note_index))
            })
    }

    pub fn can_place_note(
        &self,
        track_index: usize,
        id: Option<u64>,
        start: u32,
        length: u32,
    ) -> bool {
        let end = start.saturating_add(length);
        if length == 0 || start >= self.total_steps() || end > self.total_steps() {
            return false;
        }
        self.tracks.get(track_index).is_some_and(|track| {
            track
                .notes
                .iter()
                .all(|other| Some(other.id) == id || end <= other.start || start >= other.end())
        })
    }

    pub fn add_note(&mut self, track_index: usize, requested_start: u32, pitch: u8) -> Result<u64> {
        let default_length = if track_index % TRACK_COUNT == 3 { 1 } else { 2 };
        let last_start = self.total_steps().saturating_sub(default_length);
        let requested_start = requested_start.min(last_start);
        let start = (requested_start..=last_start)
            .find(|start| self.can_place_note(track_index, None, *start, default_length))
            .ok_or_else(|| anyhow!("no free space remains on this track"))?;
        self.insert_note_exact(track_index, start, default_length, pitch, 15)
    }

    pub fn insert_note_exact(
        &mut self,
        track_index: usize,
        start: u32,
        length: u32,
        pitch: u8,
        velocity: u8,
    ) -> Result<u64> {
        let kind = self
            .tracks
            .get(track_index)
            .ok_or_else(|| anyhow!("track index out of range"))?
            .kind;
        if !self.can_place_note(track_index, None, start, length) {
            bail!("note overlaps another note or exceeds the project");
        }
        let id = self.next_note_id.max(1);
        self.next_note_id = id.saturating_add(1);
        self.tracks[track_index].notes.push(MidiNote {
            id,
            start,
            length,
            pitch: pitch.min(47),
            velocity: velocity.min(15),
            effects: NoteEffects::for_track(kind),
        });
        self.tracks[track_index].sort_notes();
        Ok(id)
    }

    pub fn delete_note(&mut self, id: u64) -> bool {
        for track in &mut self.tracks {
            if let Some(index) = track.notes.iter().position(|note| note.id == id) {
                track.notes.remove(index);
                return true;
            }
        }
        false
    }

    pub fn move_note(&mut self, id: u64, start: u32, pitch: u8) -> bool {
        let Some((track_index, note_index)) = self.note_location(id) else {
            return false;
        };
        let length = self.tracks[track_index].notes[note_index].length;
        let start = start.min(self.total_steps().saturating_sub(length));
        if !self.can_place_note(track_index, Some(id), start, length) {
            return false;
        }
        let note = &mut self.tracks[track_index].notes[note_index];
        let changed = note.start != start || note.pitch != pitch.min(47);
        note.start = start;
        note.pitch = pitch.min(47);
        self.tracks[track_index].sort_notes();
        changed
    }

    pub fn resize_note(&mut self, id: u64, length: u32) -> bool {
        let Some((track_index, note_index)) = self.note_location(id) else {
            return false;
        };
        let note = &self.tracks[track_index].notes[note_index];
        let max_length = self.total_steps().saturating_sub(note.start).max(1);
        let length = length.clamp(1, max_length);
        if !self.can_place_note(track_index, Some(id), note.start, length) {
            return false;
        }
        if self.tracks[track_index].notes[note_index].length == length {
            return false;
        }
        self.tracks[track_index].notes[note_index].length = length;
        true
    }

    pub fn duplicate_note(&mut self, id: u64) -> Result<u64> {
        let (track_index, note_index) = self
            .note_location(id)
            .ok_or_else(|| anyhow!("selected note no longer exists"))?;
        let source = self.tracks[track_index].notes[note_index].clone();
        self.copy_note_to_track(track_index, source.end(), &source)
    }

    pub fn copy_note_to_track(
        &mut self,
        track_index: usize,
        requested_start: u32,
        source: &MidiNote,
    ) -> Result<u64> {
        let last_start = self.total_steps().saturating_sub(source.length);
        let requested_start = requested_start.min(last_start);
        let start = (requested_start..=last_start)
            .find(|start| self.can_place_note(track_index, None, *start, source.length))
            .ok_or_else(|| anyhow!("no free space remains for this bar length"))?;
        let new_id = self.insert_note_exact(
            track_index,
            start,
            source.length,
            source.pitch,
            source.velocity,
        )?;
        if let Some(note) = self.note_mut(new_id) {
            note.effects = source.effects.clone();
        }
        Ok(new_id)
    }

    pub fn set_measures(&mut self, measures: u32) {
        self.measures = measures.clamp(MIN_MEASURES, MAX_MEASURES);
        let total_steps = self.total_steps();
        for track in &mut self.tracks {
            track.notes.retain(|note| note.start < total_steps);
            for note in &mut track.notes {
                note.length = note
                    .length
                    .min(total_steps.saturating_sub(note.start).max(1));
            }
            track.sort_notes();
        }
    }

    pub fn set_link16_mode(&mut self, mode: Link16Mode) {
        self.link16_mode = mode;
        while self.tracks.len() < mode.track_count() {
            let kind = TrackKind::ALL[self.tracks.len() % TRACK_COUNT];
            self.tracks.push(Track::new(kind));
        }
    }

    pub fn player_count(&self) -> usize {
        self.link16_mode.player_count()
    }

    pub fn to_gbdaw_project(&self) -> Result<legacy::Project> {
        self.to_gbdaw_player(0)
    }

    /// Build the four-channel preview/export project for one linked console.
    pub fn to_gbdaw_player(&self, player: usize) -> Result<legacy::Project> {
        self.validate()?;
        if player >= self.player_count() {
            bail!(
                "player {} is outside {}",
                player + 1,
                self.link16_mode.label()
            );
        }
        let track_base = player * TRACK_COUNT;
        let player_tracks = &self.tracks[track_base..track_base + TRACK_COUNT];

        let mut project = legacy::Project::empty();
        project.metadata.title = self.title.clone();
        project.metadata.author = self.author.clone();
        project.metadata.comment = "Created with GBSEQ / plita".to_string();
        project.song.tempo = self.bpm;
        project.song.speed = 6;
        project.song.ticks_per_row = self.timing_ticks_per_row();
        project.song.frames_per_tick = self.timing_frames_per_tick();
        project.song.rows_per_pattern = ROWS_PER_MEASURE as u8;
        project.song.patterns = (0..self.measures as usize)
            .map(|index| legacy::Pattern {
                index,
                rows: vec![legacy::Row::default(); ROWS_PER_MEASURE],
            })
            .collect();
        project.song.order_list = (0..self.measures as usize).collect();
        project.song.loop_order = self.loop_playback.then_some(0);
        project.song.loop_row = self.loop_playback.then_some(0);
        project.export_settings.loop_enabled = self.loop_playback;
        project.instruments.clear();
        project.waves.clear();
        project.mixer.mute = std::array::from_fn(|index| player_tracks[index].mute);
        project.mixer.solo = std::array::from_fn(|index| player_tracks[index].solo);

        for (local_track, track) in player_tracks.iter().enumerate() {
            let track_index = track_base + local_track;
            for note in &track.notes {
                let instrument_id = format!("gbseq_t{track_index}_n{}", note.id);
                let wave_ref = if track.kind == TrackKind::Wave {
                    let definition = self.gbdaw_wave_definition(note.effects.wave_preset)?;
                    let id = definition.id.clone();
                    if !project.waves.iter().any(|wave| wave.id == id) {
                        project.waves.push(definition);
                    }
                    Some(id)
                } else {
                    None
                };
                project.instruments.push(instrument_for_note(
                    track.kind,
                    &instrument_id,
                    note,
                    wave_ref.as_deref(),
                ));

                let absolute_row = note.start as usize;
                let measure = absolute_row / ROWS_PER_MEASURE;
                let row = absolute_row % ROWS_PER_MEASURE;
                let cell = &mut project.song.patterns[measure].rows[row].cells[local_track];
                cell.note = Some(note.pitch);
                cell.instrument_ref = Some(instrument_id);
                cell.length_ticks = Some(
                    note.length
                        .saturating_mul(u32::from(project.song.ticks_per_row))
                        .min(u32::from(u16::MAX)) as u16,
                );
                cell.volume = Some(note.velocity);
            }
        }

        project.validate().map_err(|error| anyhow!(error))?;
        Ok(project)
    }

    fn gbdaw_wave_definition(&self, preset: u8) -> Result<legacy::WaveDefinition> {
        let preset_index = usize::from(preset);
        let slot = self
            .wave_slots
            .get(preset_index)
            .ok_or_else(|| anyhow!("CH3 wave preset {} is outside 0..15", preset))?;
        let (name, bytes16, tags) = if slot.source <= 6 {
            let source = usize::from(slot.source);
            (
                KITAQGB_BUILTIN_WAVE_NAMES[source].to_string(),
                KITAQGB_BUILTIN_WAVES[source],
                vec!["preset".to_string(), "kitaqgb".to_string()],
            )
        } else if (0x80..=0xBF).contains(&slot.source) {
            let user_index = usize::from(slot.source & 0x3F);
            let wave = self.user_waves.get(user_index).ok_or_else(|| {
                anyhow!(
                    "CH3 wave preset {} references missing user wave {}",
                    preset,
                    user_index + 1
                )
            })?;
            let mut bytes16 = [0_u8; 16];
            for (index, byte) in bytes16.iter_mut().enumerate() {
                *byte =
                    (wave.samples[index * 2].min(15) << 4) | wave.samples[index * 2 + 1].min(15);
            }
            (
                wave.name.clone(),
                bytes16,
                vec!["user".to_string(), "gbseq".to_string()],
            )
        } else {
            bail!(
                "CH3 wave preset {} has invalid source 0x{:02X}",
                preset,
                slot.source
            );
        };

        Ok(legacy::WaveDefinition {
            id: format!("gbseq_wave_slot_{preset_index:02}"),
            name: format!("Slot {}: {name}", preset_index + 1),
            bytes16,
            preview_label: name,
            tags,
        })
    }
}

fn instrument_for_note(
    kind: TrackKind,
    id: &str,
    note: &MidiNote,
    wave_ref: Option<&str>,
) -> legacy::Instrument {
    let effects = &note.effects;
    let pan = match effects.pan {
        PanPosition::Left => legacy::Pan::Left,
        PanPosition::Center => legacy::Pan::Center,
        PanPosition::Right => legacy::Pan::Right,
    };
    let envelope = (note.velocity.min(15) << 4)
        | (u8::from(effects.envelope_increase) << 3)
        | effects.envelope_period.min(7);
    match kind {
        TrackKind::Pulse1 | TrackKind::Pulse2 => {
            let sweep = (effects.sweep_period.min(7) << 4)
                | (u8::from(effects.sweep_negate) << 3)
                | effects.sweep_shift.min(7);
            legacy::Instrument::Pulse(legacy::PulseInstrument {
                id: id.to_string(),
                name: id.to_string(),
                channel: if kind == TrackKind::Pulse1 {
                    legacy::Channel::Ch1
                } else {
                    legacy::Channel::Ch2
                },
                duty: effects.duty.min(3),
                envelope,
                sweep,
                pan,
            })
        }
        TrackKind::Wave => legacy::Instrument::Wave(legacy::WaveInstrument {
            id: id.to_string(),
            name: id.to_string(),
            wave_ref: wave_ref.unwrap_or("tri").to_string(),
            output_level: match effects.wave_level.clamp(1, 3) {
                1 => 0x20,
                2 => 0x40,
                _ => 0x60,
            },
            pan,
        }),
        TrackKind::Noise => legacy::Instrument::Noise(legacy::NoiseInstrument {
            id: id.to_string(),
            name: id.to_string(),
            envelope,
            noise_param: (effects.noise_shift.min(15) << 4)
                | (u8::from(effects.noise_width7) << 3)
                | effects.noise_divisor.min(7),
            pan,
        }),
    }
}

pub fn note_label(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let note = note.min(47);
    format!("{}{}", NAMES[note as usize % 12], note / 12 + 2)
}

pub fn position_label(step: u32) -> String {
    let measure = step / STEPS_PER_MEASURE + 1;
    let within = step % STEPS_PER_MEASURE;
    let beat = within / 4 + 1;
    let sixteenth = match within % 4 {
        0 => "00",
        1 => "25",
        2 => "50",
        _ => "75",
    };
    format!("{measure}:{beat}.{sixteenth}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_is_valid_and_uses_four_distinct_tracks() {
        let project = SequenceProject::demo();
        project.validate().unwrap();
        assert_eq!(project.tracks.len(), TRACK_COUNT);
        assert!(project.tracks.iter().all(|track| !track.notes.is_empty()));
    }

    #[test]
    fn sixteenth_note_length_survives_gbdaw_conversion() {
        let mut project = SequenceProject::empty();
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        let id = project.insert_note_exact(0, 0, 1, 24, 15).unwrap();
        let converted = project.to_gbdaw_project().unwrap();
        let cell = &converted.song.patterns[0].rows[0].cells[0];
        assert_eq!(cell.note, Some(24));
        assert_eq!(cell.length_ticks, Some(2));
        assert!(cell
            .instrument_ref
            .as_deref()
            .is_some_and(|value| value.ends_with(&id.to_string())));
    }

    #[test]
    fn move_and_resize_reject_track_overlap() {
        let mut project = SequenceProject::empty();
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        let first = project.insert_note_exact(0, 0, 2, 24, 15).unwrap();
        let second = project.insert_note_exact(0, 4, 2, 28, 15).unwrap();
        assert!(!project.move_note(second, 1, 28));
        assert!(!project.resize_note(first, 5));
        assert!(project.move_note(second, 2, 28));
    }

    #[test]
    fn json_round_trip_preserves_effect_details() {
        let project = SequenceProject::demo();
        let json = serde_json::to_string_pretty(&project).unwrap();
        let decoded: SequenceProject = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, project);
    }

    #[test]
    fn legacy_json_without_loop_field_keeps_historical_looping() {
        let project = SequenceProject::empty();
        let mut value = serde_json::to_value(project).unwrap();
        value.as_object_mut().unwrap().remove("loop_playback");
        let decoded: SequenceProject = serde_json::from_value(value).unwrap();
        assert!(decoded.loop_playback);
    }

    #[test]
    fn loop_setting_reaches_gbdaw_transport_and_export() {
        let mut project = SequenceProject::empty();
        project.loop_playback = false;
        let converted = project.to_gbdaw_project().unwrap();
        assert_eq!(converted.song.loop_order, None);
        assert_eq!(converted.song.loop_row, None);
        assert!(!converted.export_settings.loop_enabled);

        project.loop_playback = true;
        let converted = project.to_gbdaw_project().unwrap();
        assert_eq!(converted.song.loop_order, Some(0));
        assert_eq!(converted.song.loop_row, Some(0));
        assert!(converted.export_settings.loop_enabled);
    }

    #[test]
    fn duplicate_preserves_length_level_and_effects() {
        let mut project = SequenceProject::empty();
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        let id = project.insert_note_exact(0, 0, 7, 24, 11).unwrap();
        let source = project.note_mut(id).unwrap();
        source.effects.duty = 3;
        source.effects.sweep_shift = 5;
        let duplicate = project.duplicate_note(id).unwrap();
        let duplicate = project.note(duplicate).unwrap();
        assert_eq!(duplicate.start, 7);
        assert_eq!(duplicate.length, 7);
        assert_eq!(duplicate.velocity, 11);
        assert_eq!(duplicate.effects.duty, 3);
        assert_eq!(duplicate.effects.sweep_shift, 5);
    }

    #[test]
    fn detailed_pulse_effects_reach_the_gbdaw_project() {
        let mut project = SequenceProject::empty();
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        let id = project.insert_note_exact(0, 0, 2, 24, 11).unwrap();
        let note = project.note_mut(id).unwrap();
        note.effects.pan = PanPosition::Right;
        note.effects.duty = 3;
        note.effects.envelope_period = 7;
        note.effects.envelope_increase = true;
        note.effects.sweep_period = 4;
        note.effects.sweep_shift = 2;
        note.effects.sweep_negate = true;

        let converted = project.to_gbdaw_project().unwrap();
        let instrument = converted
            .instruments
            .iter()
            .find(|instrument| instrument.id() == format!("gbseq_t0_n{id}"))
            .unwrap();
        let legacy::Instrument::Pulse(pulse) = instrument else {
            panic!("expected a pulse instrument");
        };
        assert_eq!(pulse.duty, 3);
        assert_eq!(pulse.envelope, 0xBF);
        assert_eq!(pulse.sweep, 0x4A);
        assert_eq!(pulse.pan, legacy::Pan::Right);
    }

    #[test]
    fn ch3_saw_preset_reaches_gbdaw_preview_bytes() {
        let mut project = SequenceProject::empty();
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        let id = project.insert_note_exact(2, 0, 4, 12, 15).unwrap();
        project.note_mut(id).unwrap().effects.wave_preset = 1;

        let converted = project.to_gbdaw_project().unwrap();
        let instrument = converted
            .instruments
            .iter()
            .find(|instrument| instrument.id() == format!("gbseq_t2_n{id}"))
            .unwrap();
        let legacy::Instrument::Wave(wave_instrument) = instrument else {
            panic!("expected a wave instrument");
        };
        let wave = converted
            .wave_by_id(&wave_instrument.wave_ref)
            .expect("converted SAW wave definition");
        assert_eq!(wave.preview_label, "Saw");
        assert_eq!(wave.bytes16, KITAQGB_BUILTIN_WAVES[1]);
    }

    #[test]
    fn ch3_user_wave_is_packed_for_gbdaw_preview() {
        let mut project = SequenceProject::empty();
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        project.user_waves = vec![UserWave {
            name: "Alternating".to_string(),
            samples: (0..WAVE_SAMPLE_COUNT)
                .map(|index| if index % 2 == 0 { 0x0F } else { 0x00 })
                .collect(),
        }];
        project.wave_slots[15].source = 0x80;
        let id = project.insert_note_exact(2, 0, 4, 12, 15).unwrap();
        project.note_mut(id).unwrap().effects.wave_preset = 15;

        let converted = project.to_gbdaw_project().unwrap();
        let wave = converted.wave_by_id("gbseq_wave_slot_15").unwrap();
        assert_eq!(wave.bytes16, [0xF0; 16]);
        assert_eq!(wave.preview_label, "Alternating");
    }

    #[test]
    fn exact_frame_step_override_reaches_gbdaw_timing() {
        let mut project = SequenceProject::empty();
        project.frames_per_step_override = Some(1);
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        project.insert_note_exact(0, 0, 1, 16, 15).unwrap();
        project.insert_note_exact(0, 1, 1, 13, 15).unwrap();

        let converted = project.to_gbdaw_project().unwrap();
        assert_eq!(converted.song.ticks_per_row, 1);
        assert_eq!(converted.song.frames_per_tick, 1);
        assert_eq!(
            converted.song.patterns[0].rows[0].cells[0].length_ticks,
            Some(1)
        );

        let exported = gbdaw_export_kitaqgb::export_project(
            &converted,
            gbdaw_export_kitaqgb::StreamMode::Extended,
            gbdaw_export_kitaqgb::ExportPolicy::default(),
        )
        .unwrap();
        assert!(exported
            .preview
            .events
            .iter()
            .any(|event| event.frame == 1 && event.cmd == "WAIT" && event.args == [0]));
    }
}
