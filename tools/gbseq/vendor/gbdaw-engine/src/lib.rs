use anyhow::{anyhow, Result};
use gbdaw_model::{
    AutomationTarget, AutomationValue, Channel, Instrument, Project, WaveDefinition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingModel {
    pub fps: u32,
    pub rows_per_pattern: u8,
    pub ticks_per_row: u8,
    pub frames_per_tick: u8,
    pub frames_per_row: u32,
    pub order_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSong {
    pub timing: TimingModel,
    pub channel_streams: [Vec<ChannelEvent>; 4],
    pub global_automation: Vec<ChannelEvent>,
    pub wave_library: Vec<WaveDefinition>,
    pub instrument_library: Vec<Instrument>,
    pub export_options: NormalizedExportOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedExportOptions {
    pub loop_enabled: bool,
    pub force_inst_before_note: bool,
    pub music_bank: u8,
    pub sfx_bank: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvent {
    pub abs_frame: u32,
    pub channel: Option<Channel>,
    pub kind: EventKind,
    pub payload: EventPayload,
    pub source_view: String,
    pub priority_hint: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    NoteOn,
    NoteOff,
    SetInst,
    SetPan,
    SetSweep,
    SetWavePreset,
    SetWaveCustom,
    SetDuty,
    SetCh3Level,
    SetCh4Param,
    SetMasterVolume,
    StopChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPayload {
    None,
    Note {
        note_value: u8,
        instrument_ref: Option<String>,
    },
    Inst {
        instrument_ref: String,
    },
    Pan(u8),
    Sweep(u8),
    WavePreset(String),
    WaveCustom([u8; 16]),
    Duty(u8),
    Ch3Level(u8),
    Ch4Param(u8),
    MasterVolume {
        left: u8,
        right: u8,
    },
}

pub fn normalize_project(project: &Project) -> Result<NormalizedSong> {
    project.validate().map_err(|e| anyhow!(e))?;

    let timing = TimingModel {
        fps: 60,
        rows_per_pattern: project.song.rows_per_pattern,
        ticks_per_row: project.song.ticks_per_row,
        frames_per_tick: project.song.frames_per_tick,
        frames_per_row: project.song.ticks_per_row as u32 * project.song.frames_per_tick as u32,
        order_count: project.song.order_list.len(),
    };

    let mut channel_streams: [Vec<ChannelEvent>; 4] =
        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut global_automation = Vec::new();
    let mut last_instrument_per_channel: [Option<String>; 4] = [None, None, None, None];

    for (order_pos, pattern_index) in project.song.order_list.iter().copied().enumerate() {
        let pattern = &project.song.patterns[pattern_index];
        for (row_index, row) in pattern.rows.iter().enumerate() {
            let abs_tick = ((order_pos * project.song.rows_per_pattern as usize) + row_index)
                * project.song.ticks_per_row as usize;
            let abs_frame = (abs_tick * project.song.frames_per_tick as usize) as u32;

            for channel in Channel::ALL {
                let idx = channel.index();
                let cell = &row.cells[idx];
                if let Some(note_value) = cell.note {
                    let instrument_ref = cell
                        .instrument_ref
                        .clone()
                        .or_else(|| last_instrument_per_channel[idx].clone());
                    if let Some(ref id) = instrument_ref {
                        last_instrument_per_channel[idx] = Some(id.clone());
                    }
                    channel_streams[idx].push(ChannelEvent {
                        abs_frame,
                        channel: Some(channel),
                        kind: EventKind::NoteOn,
                        payload: EventPayload::Note {
                            note_value,
                            instrument_ref,
                        },
                        source_view: "tracker".to_string(),
                        priority_hint: 100,
                    });
                    if let Some(length_ticks) = cell.length_ticks {
                        if length_ticks > 0 {
                            channel_streams[idx].push(ChannelEvent {
                                abs_frame: abs_frame
                                    + length_ticks as u32 * project.song.frames_per_tick as u32,
                                channel: Some(channel),
                                kind: EventKind::StopChannel,
                                payload: EventPayload::None,
                                source_view: "note_length".to_string(),
                                priority_hint: 90,
                            });
                        }
                    }
                }
                if cell.stop {
                    channel_streams[idx].push(ChannelEvent {
                        abs_frame,
                        channel: Some(channel),
                        kind: EventKind::StopChannel,
                        payload: EventPayload::None,
                        source_view: "tracker".to_string(),
                        priority_hint: 90,
                    });
                }
            }

            for automation in &row.row_automation {
                let automation_frame = abs_frame
                    + automation
                        .position_tick
                        .min(project.song.ticks_per_row.saturating_sub(1))
                        as u32
                        * project.song.frames_per_tick as u32;
                let mut event = ChannelEvent {
                    abs_frame: automation_frame,
                    channel: automation.channel,
                    kind: EventKind::StopChannel,
                    payload: EventPayload::None,
                    source_view: automation.source_view.clone(),
                    priority_hint: 10,
                };
                match automation.target {
                    AutomationTarget::Ch1Duty | AutomationTarget::Ch2Duty => {
                        let value = expect_u8(&automation.value)?;
                        event.kind = EventKind::SetDuty;
                        event.payload = EventPayload::Duty(value);
                    }
                    AutomationTarget::Ch1Sweep => {
                        event.kind = EventKind::SetSweep;
                        event.payload = EventPayload::Sweep(expect_u8(&automation.value)?);
                    }
                    AutomationTarget::Ch3Level => {
                        event.kind = EventKind::SetCh3Level;
                        event.payload = EventPayload::Ch3Level(expect_u8(&automation.value)?);
                    }
                    AutomationTarget::Ch3Wave => {
                        event.kind = EventKind::SetWavePreset;
                        event.payload =
                            EventPayload::WavePreset(expect_text(&automation.value)?.to_string());
                    }
                    AutomationTarget::Ch3CustomWave => {
                        event.kind = EventKind::SetWaveCustom;
                        event.payload =
                            EventPayload::WaveCustom(expect_bytes16(&automation.value)?);
                    }
                    AutomationTarget::Ch4Param => {
                        event.kind = EventKind::SetCh4Param;
                        event.payload = EventPayload::Ch4Param(expect_u8(&automation.value)?);
                    }
                    AutomationTarget::Pan => {
                        event.kind = EventKind::SetPan;
                        event.payload = EventPayload::Pan(expect_u8(&automation.value)?);
                    }
                    AutomationTarget::MasterVolume => {
                        let (left, right) = expect_pair(&automation.value)?;
                        event.kind = EventKind::SetMasterVolume;
                        event.payload = EventPayload::MasterVolume { left, right };
                    }
                    AutomationTarget::StopChannel => {
                        event.kind = EventKind::StopChannel;
                        event.payload = EventPayload::None;
                    }
                }

                if let Some(channel) = event.channel {
                    channel_streams[channel.index()].push(event);
                } else {
                    global_automation.push(event);
                }
            }
        }
    }

    for stream in &mut channel_streams {
        stream.sort_by_key(|event| (event.abs_frame, event.priority_hint));
    }
    global_automation.sort_by_key(|event| (event.abs_frame, event.priority_hint));

    Ok(NormalizedSong {
        timing,
        channel_streams,
        global_automation,
        wave_library: project.waves.clone(),
        instrument_library: project.instruments.clone(),
        export_options: NormalizedExportOptions {
            loop_enabled: project.export_settings.loop_enabled,
            force_inst_before_note: project.export_settings.force_inst_before_note,
            music_bank: project.export_settings.music_bank,
            sfx_bank: project.export_settings.sfx_bank,
        },
    })
}

fn expect_u8(value: &AutomationValue) -> Result<u8> {
    match value {
        AutomationValue::U8(v) => Ok(*v),
        _ => Err(anyhow!("expected u8 automation value")),
    }
}

fn expect_pair(value: &AutomationValue) -> Result<(u8, u8)> {
    match value {
        AutomationValue::Pair(a, b) => Ok((*a, *b)),
        _ => Err(anyhow!("expected pair automation value")),
    }
}

fn expect_text(value: &AutomationValue) -> Result<&str> {
    match value {
        AutomationValue::Text(v) => Ok(v),
        _ => Err(anyhow!("expected text automation value")),
    }
}

fn expect_bytes16(value: &AutomationValue) -> Result<[u8; 16]> {
    match value {
        AutomationValue::Bytes16(v) => Ok(*v),
        _ => Err(anyhow!("expected bytes16 automation value")),
    }
}
