use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
};

use anyhow::{anyhow, bail, Result};
use gbdaw_engine::{normalize_project, EventKind, EventPayload, NormalizedSong};
use gbdaw_model::{Channel, Instrument, LoopStrategy, Pan, Project, WaveDefinition};
use serde::{Deserialize, Serialize};

pub const AUDIO_CMD_WAIT: u8 = 0x00;
pub const AUDIO_CMD_NOTE: u8 = 0x10;
pub const AUDIO_CMD_SET_INST: u8 = 0x11;
pub const AUDIO_CMD_SET_PAN: u8 = 0x12;
pub const AUDIO_CMD_SET_SWEEP: u8 = 0x13;
pub const AUDIO_CMD_SET_WAVE: u8 = 0x14;
pub const AUDIO_CMD_SET_CH3_CUSTOM_WAVE: u8 = 0x15;
pub const AUDIO_CMD_SET_CH4_PARAM: u8 = 0x16;
pub const AUDIO_CMD_SET_MASTER_VOLUME: u8 = 0x17;
pub const AUDIO_CMD_SET_CH1_DUTY: u8 = 0x18;
pub const AUDIO_CMD_SET_CH2_DUTY: u8 = 0x19;
pub const AUDIO_CMD_SET_CH3_LEVEL: u8 = 0x1A;
pub const AUDIO_CMD_STOP_CHANNEL: u8 = 0x20;
pub const AUDIO_CMD_LOOP: u8 = 0x30;
pub const AUDIO_CMD_STOP: u8 = 0x40;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamMode {
    Legacy,
    Extended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPolicy {
    pub note_range: NoteRangePolicy,
    pub legacy_custom_wave_policy: LegacyCustomWavePolicy,
    pub legacy_master_volume_policy: LegacyMasterVolumePolicy,
    pub loop_strategy: LoopStrategy,
    pub force_inst_before_note: bool,
}

impl Default for ExportPolicy {
    fn default() -> Self {
        Self {
            note_range: NoteRangePolicy::Clamp,
            legacy_custom_wave_policy: LegacyCustomWavePolicy::PrebakeWaveAssets,
            legacy_master_volume_policy: LegacyMasterVolumePolicy::WarnAndDrop,
            loop_strategy: LoopStrategy::UnfoldOnce,
            force_inst_before_note: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteRangePolicy {
    Clamp,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyCustomWavePolicy {
    Reject,
    PrebakeWaveAssets,
    ApproximateToPreset,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyMasterVolumePolicy {
    WarnAndDrop,
    FlattenToInitial,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCommand {
    pub frame: u32,
    pub cmd: String,
    pub args: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongExportPreview {
    pub song: String,
    pub events: Vec<StreamCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongExport {
    pub name: String,
    pub bank: u8,
    pub preview: SongExportPreview,
    pub bytes: Vec<u8>,
    pub warnings: Vec<String>,
    pub counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveManifestEntry {
    pub wave_id: String,
    pub hash: String,
    pub bytes16: [u8; 16],
    pub used_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifestSong {
    pub name: String,
    pub bank: u8,
    pub bytes: usize,
    pub loop_strategy: String,
    pub commands: BTreeMap<String, usize>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub exporter_version: String,
    pub stream_mode: StreamMode,
    pub songs: Vec<ExportManifestSong>,
    pub waves: Vec<WaveManifestEntry>,
    pub sfx: Vec<String>,
}

#[derive(Debug, Clone)]
struct PulseResolved {
    duty: u8,
    envelope: u8,
    sweep: u8,
    pan: Pan,
}

#[derive(Debug, Clone)]
struct WaveResolved {
    wave_id: Option<String>,
    wave_bytes16: Option<[u8; 16]>,
    output_level: u8,
    pan: Pan,
}

#[derive(Debug, Clone)]
struct NoiseResolved {
    envelope: u8,
    noise_param: u8,
    pan: Pan,
}

#[derive(Debug, Clone)]
enum ResolvedInstrument {
    Pulse(PulseResolved),
    Wave(WaveResolved),
    Noise(NoiseResolved),
}

#[derive(Debug, Default, Clone)]
struct ExporterAudioState {
    ch1_duty: Option<u8>,
    ch2_duty: Option<u8>,
    ch1_env: Option<u8>,
    ch2_env: Option<u8>,
    ch4_env: Option<u8>,
    ch1_sweep: Option<u8>,
    ch3_wave_id: Option<String>,
    ch3_wave_hash: Option<String>,
    ch3_level: Option<u8>,
    ch4_param: Option<u8>,
    pan: [Option<u8>; 4],
    master_left: Option<u8>,
    master_right: Option<u8>,
    last_inst_id: [Option<String>; 4],
}

#[derive(Debug, Default)]
struct FrameAggregate {
    master_volume: Option<(u8, u8)>,
    pan: [Option<u8>; 4],
    ch1_duty: Option<u8>,
    ch2_duty: Option<u8>,
    sweep: Option<u8>,
    ch3_level: Option<u8>,
    ch3_wave_preset: Option<String>,
    ch3_wave_custom: Option<[u8; 16]>,
    ch4_param: Option<u8>,
    set_inst: [Option<(u8, u8)>; 4],
    note: [Option<u8>; 4],
    stop: [bool; 4],
}

pub fn export_project_to_dir(
    project: &Project,
    out_dir: impl AsRef<Path>,
    mode: StreamMode,
    policy: ExportPolicy,
) -> Result<ExportManifest> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;
    let song_export = export_project(project, mode, policy.clone())?;

    fs::write(
        out_dir.join(format!("music_bank{:02}.c", song_export.bank)),
        render_music_c(&song_export),
    )?;
    fs::write(out_dir.join("music_decl.h"), render_decl_h(&song_export))?;
    fs::write(
        out_dir.join("preview_stream.json"),
        serde_json::to_string_pretty(&song_export.preview)?,
    )?;
    let waves = collect_waves_manifest(&song_export.preview.events);
    if !waves.is_empty() {
        fs::write(out_dir.join("waves_bank31.c"), render_waves_c(&waves))?;
    }

    let manifest = ExportManifest {
        exporter_version: "1.2".to_string(),
        stream_mode: mode,
        songs: vec![ExportManifestSong {
            name: song_export.name.clone(),
            bank: song_export.bank,
            bytes: song_export.bytes.len(),
            loop_strategy: format!("{:?}", policy.loop_strategy).to_lowercase(),
            commands: song_export.counts.clone(),
            warnings: song_export.warnings.clone(),
        }],
        waves,
        sfx: Vec::new(),
    };
    fs::write(
        out_dir.join("export_manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(manifest)
}

pub fn preview_stream_json(
    project: &Project,
    mode: StreamMode,
    policy: ExportPolicy,
) -> Result<SongExportPreview> {
    Ok(export_project(project, mode, policy)?.preview)
}

pub fn export_project(
    project: &Project,
    mode: StreamMode,
    policy: ExportPolicy,
) -> Result<SongExport> {
    let normalized = normalize_project(project)?;
    let instrument_map = build_instrument_map(project);
    let wave_map = build_wave_map(project);
    let frames = aggregate_frames(
        project,
        &normalized,
        &instrument_map,
        &wave_map,
        mode,
        &policy,
    )?;
    let (preview, bytes, counts, warnings) = encode_frames(project, frames, mode, &policy)?;

    Ok(SongExport {
        name: project.metadata.title.clone(),
        bank: project.export_settings.music_bank,
        preview,
        bytes,
        warnings,
        counts,
    })
}

fn build_instrument_map(project: &Project) -> HashMap<String, ResolvedInstrument> {
    let mut map = HashMap::new();
    for instrument in &project.instruments {
        match instrument {
            Instrument::Pulse(v) => {
                map.insert(
                    v.id.clone(),
                    ResolvedInstrument::Pulse(PulseResolved {
                        duty: v.duty,
                        envelope: v.envelope,
                        sweep: v.sweep,
                        pan: v.pan,
                    }),
                );
            }
            Instrument::Wave(v) => {
                let wave_bytes16 = project.wave_by_id(&v.wave_ref).map(|w| w.bytes16);
                map.insert(
                    v.id.clone(),
                    ResolvedInstrument::Wave(WaveResolved {
                        wave_id: Some(v.wave_ref.clone()),
                        wave_bytes16,
                        output_level: v.output_level,
                        pan: v.pan,
                    }),
                );
            }
            Instrument::Noise(v) => {
                map.insert(
                    v.id.clone(),
                    ResolvedInstrument::Noise(NoiseResolved {
                        envelope: v.envelope,
                        noise_param: v.noise_param,
                        pan: v.pan,
                    }),
                );
            }
        }
    }
    map
}

fn build_wave_map(project: &Project) -> HashMap<String, WaveDefinition> {
    project
        .waves
        .iter()
        .cloned()
        .map(|wave| (wave.id.clone(), wave))
        .collect()
}

fn aggregate_frames(
    project: &Project,
    normalized: &NormalizedSong,
    instruments: &HashMap<String, ResolvedInstrument>,
    wave_map: &HashMap<String, WaveDefinition>,
    mode: StreamMode,
    policy: &ExportPolicy,
) -> Result<BTreeMap<u32, FrameAggregate>> {
    let mut frames: BTreeMap<u32, FrameAggregate> = BTreeMap::new();
    let mut current_inst_by_channel: [Option<String>; 4] = [None, None, None, None];
    let mut current_state = ExporterAudioState::default();

    for event in &normalized.global_automation {
        let aggregate = frames.entry(event.abs_frame).or_default();
        apply_event_to_aggregate(aggregate, event, wave_map)?;
    }

    for channel in Channel::ALL {
        for event in &normalized.channel_streams[channel.index()] {
            let aggregate = frames.entry(event.abs_frame).or_default();
            match event.kind {
                EventKind::NoteOn => {
                    let (note_value, instrument_ref) = match &event.payload {
                        EventPayload::Note {
                            note_value,
                            instrument_ref,
                        } => (*note_value, instrument_ref.clone()),
                        _ => bail!("invalid payload for note_on"),
                    };
                    if let Some(ref inst_id) = instrument_ref {
                        current_inst_by_channel[channel.index()] = Some(inst_id.clone());
                    }
                    let inst_id = current_inst_by_channel[channel.index()]
                        .clone()
                        .ok_or_else(|| anyhow!("channel {:?} note has no instrument", channel))?;
                    let resolved = instruments
                        .get(&inst_id)
                        .ok_or_else(|| anyhow!("instrument '{}' not found", inst_id))?
                        .clone();
                    append_note_commands(
                        aggregate,
                        channel,
                        note_value,
                        &inst_id,
                        resolved,
                        &mut current_state,
                        mode,
                        policy,
                    )?;
                }
                EventKind::StopChannel | EventKind::NoteOff => {
                    aggregate.stop[channel.index()] = true;
                }
                _ => apply_event_to_aggregate(aggregate, event, wave_map)?,
            }
        }
    }

    for aggregate in frames.values_mut() {
        for channel in Channel::ALL {
            if aggregate.note[channel.index()].is_some() {
                aggregate.stop[channel.index()] = false;
            }
        }
    }

    if project.export_settings.loop_enabled {
        let tail =
            frames.keys().next_back().copied().unwrap_or(0) + normalized.timing.frames_per_row;
        frames.entry(tail).or_default();
    }

    Ok(frames)
}

fn append_note_commands(
    aggregate: &mut FrameAggregate,
    channel: Channel,
    note_value: u8,
    inst_id: &str,
    resolved: ResolvedInstrument,
    current_state: &mut ExporterAudioState,
    mode: StreamMode,
    policy: &ExportPolicy,
) -> Result<()> {
    match resolved {
        ResolvedInstrument::Pulse(pulse) => match channel {
            Channel::Ch1 => {
                if mode == StreamMode::Extended {
                    if current_state.ch1_duty != Some(pulse.duty) {
                        aggregate.ch1_duty = Some(pulse.duty);
                    }
                    if current_state.ch1_sweep != Some(pulse.sweep) {
                        aggregate.sweep = Some(pulse.sweep);
                    }
                    if current_state.ch1_env != Some(pulse.envelope)
                        || current_state.last_inst_id[channel.index()].as_deref() != Some(inst_id)
                        || policy.force_inst_before_note
                    {
                        aggregate.set_inst[channel.index()] = Some((pulse.duty, pulse.envelope));
                    }
                } else {
                    if current_state.ch1_env != Some(pulse.envelope)
                        || current_state.ch1_duty != Some(pulse.duty)
                        || current_state.last_inst_id[channel.index()].as_deref() != Some(inst_id)
                    {
                        aggregate.set_inst[channel.index()] = Some((pulse.duty, pulse.envelope));
                    }
                    if current_state.ch1_sweep != Some(pulse.sweep) {
                        aggregate.sweep = Some(pulse.sweep);
                    }
                }
                if current_state.pan[channel.index()] != Some(pulse.pan.as_u8()) {
                    aggregate.pan[channel.index()] = Some(pulse.pan.as_u8());
                }
                aggregate.note[channel.index()] = Some(note_value);
                current_state.ch1_duty = Some(pulse.duty);
                current_state.ch1_env = Some(pulse.envelope);
                current_state.ch1_sweep = Some(pulse.sweep);
                current_state.pan[channel.index()] = Some(pulse.pan.as_u8());
                current_state.last_inst_id[channel.index()] = Some(inst_id.to_string());
            }
            Channel::Ch2 => {
                if mode == StreamMode::Extended {
                    if current_state.ch2_duty != Some(pulse.duty) {
                        aggregate.ch2_duty = Some(pulse.duty);
                    }
                    if current_state.ch2_env != Some(pulse.envelope)
                        || current_state.last_inst_id[channel.index()].as_deref() != Some(inst_id)
                        || policy.force_inst_before_note
                    {
                        aggregate.set_inst[channel.index()] = Some((pulse.duty, pulse.envelope));
                    }
                } else if current_state.ch2_env != Some(pulse.envelope)
                    || current_state.ch2_duty != Some(pulse.duty)
                    || current_state.last_inst_id[channel.index()].as_deref() != Some(inst_id)
                {
                    aggregate.set_inst[channel.index()] = Some((pulse.duty, pulse.envelope));
                }
                if current_state.pan[channel.index()] != Some(pulse.pan.as_u8()) {
                    aggregate.pan[channel.index()] = Some(pulse.pan.as_u8());
                }
                aggregate.note[channel.index()] = Some(note_value);
                current_state.ch2_duty = Some(pulse.duty);
                current_state.ch2_env = Some(pulse.envelope);
                current_state.pan[channel.index()] = Some(pulse.pan.as_u8());
                current_state.last_inst_id[channel.index()] = Some(inst_id.to_string());
            }
            _ => bail!("pulse instrument used on invalid channel"),
        },
        ResolvedInstrument::Wave(wave) => {
            if channel != Channel::Ch3 {
                bail!("wave instrument used on invalid channel");
            }
            if let Some(bytes) = wave.wave_bytes16 {
                let hash = hash_wave(bytes);
                if mode == StreamMode::Extended {
                    if current_state.ch3_wave_hash.as_deref() != Some(&hash) {
                        aggregate.ch3_wave_custom = Some(bytes);
                    }
                } else if current_state.ch3_wave_id != wave.wave_id {
                    match policy.legacy_custom_wave_policy {
                        LegacyCustomWavePolicy::Reject => {
                            bail!("legacy mode cannot inject custom wave mid-song")
                        }
                        LegacyCustomWavePolicy::PrebakeWaveAssets
                        | LegacyCustomWavePolicy::ApproximateToPreset => {
                            if let Some(id) = &wave.wave_id {
                                aggregate.ch3_wave_preset = Some(id.clone());
                            }
                        }
                    }
                }
                current_state.ch3_wave_hash = Some(hash);
            }
            if let Some(id) = &wave.wave_id {
                if current_state.ch3_wave_id.as_deref() != Some(id) && mode == StreamMode::Legacy {
                    aggregate.ch3_wave_preset = Some(id.clone());
                    current_state.ch3_wave_id = Some(id.clone());
                }
            }
            if current_state.ch3_level != Some(wave.output_level) {
                if mode == StreamMode::Extended {
                    aggregate.ch3_level = Some(wave.output_level);
                } else {
                    aggregate.set_inst[channel.index()] = Some((0, wave.output_level));
                }
            }
            if current_state.pan[channel.index()] != Some(wave.pan.as_u8()) {
                aggregate.pan[channel.index()] = Some(wave.pan.as_u8());
            }
            aggregate.note[channel.index()] = Some(note_value);
            current_state.ch3_level = Some(wave.output_level);
            current_state.pan[channel.index()] = Some(wave.pan.as_u8());
            current_state.last_inst_id[channel.index()] = Some(inst_id.to_string());
        }
        ResolvedInstrument::Noise(noise) => {
            if channel != Channel::Ch4 {
                bail!("noise instrument used on invalid channel");
            }
            let param = note_value.max(noise.noise_param);
            if mode == StreamMode::Extended && current_state.ch4_param != Some(param) {
                aggregate.ch4_param = Some(param);
            }
            if current_state.ch4_env != Some(noise.envelope)
                || current_state.last_inst_id[channel.index()].as_deref() != Some(inst_id)
                || (mode == StreamMode::Extended && policy.force_inst_before_note)
            {
                aggregate.set_inst[channel.index()] = Some((0, noise.envelope));
            }
            if current_state.pan[channel.index()] != Some(noise.pan.as_u8()) {
                aggregate.pan[channel.index()] = Some(noise.pan.as_u8());
            }
            aggregate.note[channel.index()] = Some(param);
            current_state.ch4_env = Some(noise.envelope);
            current_state.ch4_param = Some(param);
            current_state.pan[channel.index()] = Some(noise.pan.as_u8());
            current_state.last_inst_id[channel.index()] = Some(inst_id.to_string());
        }
    }
    Ok(())
}

fn apply_event_to_aggregate(
    aggregate: &mut FrameAggregate,
    event: &gbdaw_engine::ChannelEvent,
    wave_map: &HashMap<String, WaveDefinition>,
) -> Result<()> {
    match (&event.kind, &event.payload) {
        (EventKind::SetPan, EventPayload::Pan(pan)) => {
            if let Some(channel) = event.channel {
                aggregate.pan[channel.index()] = Some(*pan);
            }
        }
        (EventKind::SetSweep, EventPayload::Sweep(value)) => aggregate.sweep = Some(*value),
        (EventKind::SetDuty, EventPayload::Duty(value)) => {
            if let Some(channel) = event.channel {
                match channel {
                    Channel::Ch1 => aggregate.ch1_duty = Some(*value),
                    Channel::Ch2 => aggregate.ch2_duty = Some(*value),
                    _ => {}
                }
            }
        }
        (EventKind::SetCh3Level, EventPayload::Ch3Level(value)) => {
            aggregate.ch3_level = Some(*value)
        }
        (EventKind::SetWavePreset, EventPayload::WavePreset(id)) => {
            let _ = wave_map
                .get(id)
                .ok_or_else(|| anyhow!("wave '{}' not found", id))?;
            aggregate.ch3_wave_preset = Some(id.clone());
        }
        (EventKind::SetWaveCustom, EventPayload::WaveCustom(bytes)) => {
            aggregate.ch3_wave_custom = Some(*bytes)
        }
        (EventKind::SetCh4Param, EventPayload::Ch4Param(value)) => {
            aggregate.ch4_param = Some(*value)
        }
        (EventKind::SetMasterVolume, EventPayload::MasterVolume { left, right }) => {
            aggregate.master_volume = Some((*left, *right))
        }
        (EventKind::StopChannel, _) => {
            if let Some(channel) = event.channel {
                aggregate.stop[channel.index()] = true;
            }
        }
        _ => {}
    }
    Ok(())
}

fn encode_frames(
    project: &Project,
    frames: BTreeMap<u32, FrameAggregate>,
    mode: StreamMode,
    policy: &ExportPolicy,
) -> Result<(
    SongExportPreview,
    Vec<u8>,
    BTreeMap<String, usize>,
    Vec<String>,
)> {
    let mut preview_events = Vec::new();
    let mut bytes = Vec::new();
    let mut counts = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut last_frame = 0u32;

    for (frame, aggregate) in &frames {
        if *frame > last_frame {
            let mut delta = *frame - last_frame;
            // Audio_ServiceMusic resumes after delay+1 VBlank calls. Encode
            // the desired event-frame distance as interval-1 so exported ROM
            // timing matches the absolute frames used by GBDAW/GBSEQ preview.
            while delta > 256 {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "WAIT",
                    &[255],
                    AUDIO_CMD_WAIT,
                );
                delta -= 256;
            }
            if delta > 0 {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "WAIT",
                    &[(delta - 1) as u8],
                    AUDIO_CMD_WAIT,
                );
            }
        }

        if let Some((left, right)) = aggregate.master_volume {
            match mode {
                StreamMode::Extended => push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_MASTER_VOLUME",
                    &[left, right],
                    AUDIO_CMD_SET_MASTER_VOLUME,
                ),
                StreamMode::Legacy => match policy.legacy_master_volume_policy {
                    LegacyMasterVolumePolicy::Reject => {
                        bail!("legacy mode does not support master volume automation")
                    }
                    LegacyMasterVolumePolicy::FlattenToInitial
                    | LegacyMasterVolumePolicy::WarnAndDrop => {
                        warnings.push("legacy mode dropped master volume automation".to_string())
                    }
                },
            }
        }

        for channel in Channel::ALL {
            if let Some(pan) = aggregate.pan[channel.index()] {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_PAN",
                    &[channel.index() as u8, pan],
                    AUDIO_CMD_SET_PAN,
                );
            }
        }
        if let Some(duty) = aggregate.ch1_duty {
            if mode == StreamMode::Extended {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_CH1_DUTY",
                    &[duty],
                    AUDIO_CMD_SET_CH1_DUTY,
                );
            }
        }
        if let Some(duty) = aggregate.ch2_duty {
            if mode == StreamMode::Extended {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_CH2_DUTY",
                    &[duty],
                    AUDIO_CMD_SET_CH2_DUTY,
                );
            }
        }
        if let Some(sweep) = aggregate.sweep {
            push_cmd(
                &mut preview_events,
                &mut bytes,
                &mut counts,
                *frame,
                "SET_SWEEP",
                &[sweep],
                AUDIO_CMD_SET_SWEEP,
            );
        }
        if let Some(level) = aggregate.ch3_level {
            if mode == StreamMode::Extended {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_CH3_LEVEL",
                    &[level],
                    AUDIO_CMD_SET_CH3_LEVEL,
                );
            }
        }
        if let Some(id) = &aggregate.ch3_wave_preset {
            push_cmd(
                &mut preview_events,
                &mut bytes,
                &mut counts,
                *frame,
                "SET_WAVE",
                &[wave_symbol_to_audio_id(id)],
                AUDIO_CMD_SET_WAVE,
            );
        }
        if let Some(bytes16) = aggregate.ch3_wave_custom {
            if mode == StreamMode::Extended {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_CH3_CUSTOM_WAVE",
                    &bytes16,
                    AUDIO_CMD_SET_CH3_CUSTOM_WAVE,
                );
            }
        }
        if let Some(param) = aggregate.ch4_param {
            if mode == StreamMode::Extended {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_CH4_PARAM",
                    &[param],
                    AUDIO_CMD_SET_CH4_PARAM,
                );
            }
        }
        for channel in Channel::ALL {
            if let Some((arg0, arg1)) = aggregate.set_inst[channel.index()] {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "SET_INST",
                    &[channel.stream_id(), arg0, arg1],
                    AUDIO_CMD_SET_INST,
                );
            }
        }
        for channel in Channel::ALL {
            if let Some(note_value) = aggregate.note[channel.index()] {
                let note = match policy.note_range {
                    NoteRangePolicy::Clamp => {
                        if channel == Channel::Ch4 {
                            note_value
                        } else {
                            note_value.min(47)
                        }
                    }
                    NoteRangePolicy::Error => {
                        if channel != Channel::Ch4 && note_value > 47 {
                            bail!("note value {} is out of range", note_value);
                        }
                        note_value
                    }
                };
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "NOTE",
                    &[channel.stream_id(), note],
                    AUDIO_CMD_NOTE,
                );
            }
        }
        for channel in Channel::ALL {
            if aggregate.stop[channel.index()] {
                push_cmd(
                    &mut preview_events,
                    &mut bytes,
                    &mut counts,
                    *frame,
                    "STOP_CHANNEL",
                    &[channel.stream_id()],
                    AUDIO_CMD_STOP_CHANNEL,
                );
            }
        }
        last_frame = *frame;
    }

    if project.export_settings.loop_enabled
        && matches!(policy.loop_strategy, LoopStrategy::StreamHeadOnly)
    {
        push_cmd(
            &mut preview_events,
            &mut bytes,
            &mut counts,
            last_frame,
            "LOOP",
            &[],
            AUDIO_CMD_LOOP,
        );
    } else {
        if project.export_settings.loop_enabled {
            warnings
                .push("loop was unfolded to a single pass in initial implementation".to_string());
        }
        push_cmd(
            &mut preview_events,
            &mut bytes,
            &mut counts,
            last_frame,
            "STOP",
            &[],
            AUDIO_CMD_STOP,
        );
    }

    Ok((
        SongExportPreview {
            song: project.metadata.title.clone(),
            events: preview_events,
        },
        bytes,
        counts,
        warnings,
    ))
}

fn push_cmd(
    preview: &mut Vec<StreamCommand>,
    bytes: &mut Vec<u8>,
    counts: &mut BTreeMap<String, usize>,
    frame: u32,
    name: &str,
    args: &[u8],
    opcode: u8,
) {
    preview.push(StreamCommand {
        frame,
        cmd: name.to_string(),
        args: args.to_vec(),
    });
    bytes.push(opcode);
    bytes.extend_from_slice(args);
    *counts.entry(name.to_string()).or_insert(0) += 1;
}

fn wave_symbol_to_audio_id(id: &str) -> u8 {
    match id {
        "tri" | "triangle" => 0,
        "saw" => 1,
        "vowel_a" => 2,
        "vowel_i" => 3,
        "vowel_u" => 4,
        "vowel_e" => 5,
        "vowel_o" => 6,
        _ => 7,
    }
}

fn hash_wave(bytes16: [u8; 16]) -> String {
    let mut acc: u64 = 0xcbf29ce484222325;
    for byte in bytes16 {
        acc ^= byte as u64;
        acc = acc.wrapping_mul(0x100000001b3);
    }
    format!("{:012x}", acc & 0x0000_FFFF_FFFF_FFFF)
}

fn render_music_c(song: &SongExport) -> String {
    let rows = song
        .bytes
        .iter()
        .map(|b| format!("0x{b:02X}"))
        .collect::<Vec<_>>()
        .chunks(12)
        .map(|chunk| format!("    {},", chunk.join(", ")))
        .collect::<Vec<_>>()
        .join("\n");
    format!("#pragma bank {}\n#include \"music_decl.h\"\n\nconst unsigned char song_main[] = {{\n{}\n}};\n", song.bank, rows)
}

fn render_decl_h(song: &SongExport) -> String {
    format!(
        "#pragma once\n\n#define SONG_MAIN_BANK {}\nextern const unsigned char song_main[];\n",
        song.bank
    )
}

fn render_waves_c(waves: &[WaveManifestEntry]) -> String {
    let mut out = String::from("#pragma bank 31\n\n");
    for wave in waves {
        out.push_str(&format!("const unsigned char {}[] = {{", wave.wave_id));
        for (idx, byte) in wave.bytes16.iter().enumerate() {
            if idx % 8 == 0 {
                out.push_str("\n    ");
            }
            out.push_str(&format!("0x{byte:02X}, "));
        }
        out.push_str("\n};\n\n");
    }
    out
}

fn collect_waves_manifest(events: &[StreamCommand]) -> Vec<WaveManifestEntry> {
    let mut seen = BTreeSet::new();
    let mut waves = Vec::new();
    for event in events {
        if event.cmd == "SET_CH3_CUSTOM_WAVE" && event.args.len() == 16 {
            let mut bytes16 = [0u8; 16];
            bytes16.copy_from_slice(&event.args[..16]);
            let hash = hash_wave(bytes16);
            if seen.insert(hash.clone()) {
                waves.push(WaveManifestEntry {
                    wave_id: format!("wav_{}", &hash[..6]),
                    hash,
                    bytes16,
                    used_by: vec![format!("song:{}:frame{}", event.cmd, event.frame)],
                });
            }
        }
    }
    waves
}

#[cfg(test)]
mod channel_order_tests {
    use super::*;

    #[test]
    fn exported_wave_and_noise_commands_use_physical_ids() {
        let mut project = Project::demo();
        project.song.patterns[0].rows[2].cells[2].stop = true;
        project.song.patterns[0].rows[2].cells[3].stop = true;
        for mode in [StreamMode::Legacy, StreamMode::Extended] {
            let song = export_project(&project, mode, ExportPolicy::default()).unwrap();
            for (id, note) in [(2, 12), (3, 0x23)] {
                assert!(song.preview.events.iter().any(|e| e.cmd == "NOTE" && e.frame == 0 && e.args == vec![id, note]));
                // Pulse/noise instruments use SET_INST; wave settings use dedicated commands.
                if id == 3 { assert!(song.preview.events.iter().any(|e| e.cmd == "SET_INST" && e.args[0] == id)); }
                assert!(song.preview.events.iter().any(|e| e.cmd == "STOP_CHANNEL" && e.args == vec![id]));
                assert!(song.bytes.windows(3).any(|b| b == [AUDIO_CMD_NOTE, id, note]));
                assert!(song.bytes.windows(2).any(|b| b == [AUDIO_CMD_STOP_CHANNEL, id]));
            }
        }
    }
}
