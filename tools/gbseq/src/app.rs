use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gbdaw_export_kitaqgb::{export_project_to_dir, ExportPolicy, StreamMode};
use gbdaw_kokura::{render_project_to_samples, render_project_to_wav_file};
use plita::prelude::*;
use rfd::FileDialog;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::editing::{edit_notes, NoteEdit};
use crate::link16_sav::export_link16_saves;
use crate::model::{
    note_label, position_label, Link16Mode, MidiNote, SequenceProject, TrackKind, UserWave,
    MAX_MEASURES, MAX_USER_WAVES, MIN_MEASURES, STEPS_PER_MEASURE, TRACK_COUNT, WAVE_SAMPLE_COUNT,
};
use std::collections::BTreeSet;

const DEFAULT_WINDOW_W: u32 = 1480;
const DEFAULT_WINDOW_H: u32 = 900;
const HEADER_H: f32 = 82.0;
const TOOLBAR_H: f32 = 90.0;
const RULER_H: f32 = 38.0;
const STATUS_H: f32 = 30.0;
const TRACK_HEADER_W: f32 = 190.0;
const INSPECTOR_W: f32 = 318.0;
const SCROLLBAR_H: f32 = 13.0;
const MAX_HALF_STEP_W: f32 = 96.0;
const NOTE_H: f32 = 17.0;
const NOTE_HANDLE_W: f32 = 8.0;
const MEASURES_INPUT_X: f32 = 806.0;
const MEASURES_INPUT_W: f32 = 58.0;
const GB_FRAME_RATE_HZ: f64 = 4_194_304.0 / 70_224.0;

const BG: Color = Color::rgb8(13, 16, 20);
const HEADER: Color = Color::rgb8(20, 24, 29);
const PANEL: Color = Color::rgb8(25, 30, 36);
const PANEL_ALT: Color = Color::rgb8(29, 35, 42);
const PANEL_RAISED: Color = Color::rgb8(35, 42, 50);
const BORDER: Color = Color::rgb8(63, 73, 84);
const GRID_HALF: Color = Color::rgb8(40, 47, 55);
const GRID_BEAT: Color = Color::rgb8(56, 65, 75);
const GRID_MEASURE: Color = Color::rgb8(91, 107, 123);
const TEXT: Color = Color::rgb8(229, 235, 239);
const MUTED: Color = Color::rgb8(143, 156, 168);
const ACCENT: Color = Color::rgb8(255, 191, 73);
const PLAYHEAD: Color = Color::rgb8(255, 93, 84);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdjustField {
    Start,
    Pitch,
    Length,
    Velocity,
    Pan,
    Duty,
    EnvelopePeriod,
    SweepPeriod,
    SweepShift,
    WaveLevel,
    WavePreset,
    WaveSource,
    WaveSlotLevel,
    NoiseShift,
    NoiseDivisor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Menu(MenuKind),
    SaveAs,
    SelectAll,
    Quantize,
    Transpose(i16),
    FitSong,
    Home,
    Snap,
    New,
    Open,
    Save,
    Undo,
    Redo,
    Add,
    Duplicate,
    Delete,
    Play,
    Stop,
    Preview,
    RenderWav,
    Export,
    ExportLink16Sav,
    AddUserWave,
    SetLink16Mode(Link16Mode),
    SetPlayer(usize),
    ToggleAllPlayers,
    ToggleLoop,
    Bpm(i16),
    Measures(i16),
    Zoom(i8),
    ToggleMute(usize),
    ToggleSolo(usize),
    Adjust(AdjustField, i8),
    ToggleEnvelope,
    ToggleSweepDirection,
    ToggleNoiseWidth,
    CloseInspector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuKind {
    File,
    Edit,
    View,
    Export,
}

fn normalized_key(key: &str) -> String {
    let logical = key.split('|').next().unwrap_or(key);
    logical
        .strip_prefix("Named(")
        .and_then(|name| name.strip_suffix(')'))
        .unwrap_or(logical)
        .to_ascii_uppercase()
}

fn action_icon(action: Action) -> Option<&'static str> {
    Some(match action {
        Action::Menu(MenuKind::File) | Action::Open => "folder-open",
        Action::Menu(MenuKind::Edit) => "pencil",
        Action::Menu(MenuKind::View) | Action::FitSong => "maximize",
        Action::Menu(MenuKind::Export) | Action::Export | Action::ExportLink16Sav => "package",
        Action::New => "file-plus",
        Action::Save | Action::SaveAs => "save",
        Action::Undo => "undo-2",
        Action::Redo => "redo-2",
        Action::Add => "plus",
        Action::Duplicate => "copy",
        Action::Delete => "trash-2",
        Action::Play => "play",
        Action::Stop => "square",
        Action::Preview => "volume-2",
        Action::RenderWav => "download",
        Action::SelectAll => "mouse-pointer",
        Action::Quantize | Action::Snap => "grid-2x2",
        Action::Transpose(n) => {
            if n > 0 {
                "arrow-up"
            } else {
                "arrow-down"
            }
        }
        Action::Zoom(n) => {
            if n > 0 {
                "zoom-in"
            } else {
                "zoom-out"
            }
        }
        Action::ToggleLoop => "rotate-cw",
        Action::Home => "arrow-left",
        _ => return None,
    })
}

#[derive(Clone)]
struct ButtonSpec {
    action: Action,
    rect: Rect,
    label: String,
}

#[derive(Clone, Copy, Debug)]
enum NoteHitPart {
    Body,
    Resize,
}

#[derive(Clone, Copy, Debug)]
struct NoteHit {
    id: u64,
    track: usize,
    part: NoteHitPart,
}

#[derive(Clone, Copy, Debug)]
enum DragKind {
    Move {
        id: u64,
        origin: Point,
        start: u32,
        pitch: u8,
    },
    Resize {
        id: u64,
        origin_x: f32,
        length: u32,
    },
}

struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
}

#[derive(Clone)]
struct SharedAudio {
    samples: Arc<[i16]>,
    position: usize,
    rate: u32,
    looped: bool,
}
impl Iterator for SharedAudio {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.looped && self.position == self.samples.len() {
            self.position = 0;
        }
        let sample = self.samples.get(self.position).copied()?;
        self.position += 1;
        Some(sample)
    }
}
impl Source for SharedAudio {
    fn current_frame_len(&self) -> Option<usize> {
        if !self.looped {
            return None;
        }
        let position = if self.position == self.samples.len() {
            0
        } else {
            self.position
        };
        // Match Rodio 0.19's buffered repeat boundaries without copying the PCM.
        Some((32768 - position % 32768).min(self.samples.len() - position))
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        if self.looped {
            return None;
        }
        Some(Duration::from_secs_f64(
            self.samples.len() as f64 / (self.rate as f64 * 2.0),
        ))
    }
}

struct PlaybackCache {
    project: SequenceProject,
    player: Option<usize>,
    rate: u32,
    samples: Arc<[i16]>,
}

impl AudioPlayer {
    fn new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream: stream,
            handle,
            sink: None,
        })
    }

    fn play(
        &mut self,
        sample_rate: u32,
        samples: impl Into<Arc<[i16]>>,
        looped: bool,
    ) -> Result<()> {
        self.stop();
        let sink = Sink::try_new(&self.handle)?;
        let source = SharedAudio {
            samples: samples.into(),
            position: 0,
            rate: sample_rate,
            looped,
        };
        sink.append(source);
        sink.play();
        self.sink = Some(sink);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
    }

    fn finished(&self) -> bool {
        self.sink.as_ref().is_some_and(Sink::empty)
    }

    fn is_playing(&self) -> bool {
        self.sink.as_ref().is_some_and(|sink| !sink.empty())
    }

    fn position(&self) -> Option<Duration> {
        self.sink.as_ref().map(Sink::get_pos)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlaybackTiming {
    frames_per_step: u32,
    total_steps: u32,
    looped: bool,
}

impl PlaybackTiming {
    fn from_project(project: &SequenceProject) -> Self {
        Self {
            frames_per_step: project.frames_per_step(),
            total_steps: project.total_steps(),
            looped: project.loop_playback,
        }
    }

    fn step_at(self, elapsed: Duration) -> f32 {
        let mut step =
            elapsed.as_secs_f64() * GB_FRAME_RATE_HZ / f64::from(self.frames_per_step.max(1));
        let total = f64::from(self.total_steps);
        if self.looped && total > 0.0 {
            step %= total;
        } else {
            step = step.min(total);
        }
        step as f32
    }
}

type RenderMessage = std::result::Result<(u32, Vec<i16>), String>;

#[cfg(test)]
fn render_mixed_projects(
    projects: &[gbdaw_model::Project],
    max_frames: u32,
) -> Result<(u32, Vec<i16>)> {
    crate::services::render_mixed_projects(projects, max_frames)
}

pub struct GbseqApp {
    size: Size,
    project: SequenceProject,
    current_path: Option<PathBuf>,
    selected_note: Option<u64>,
    selected_notes: BTreeSet<u64>,
    selection_modifier: bool,
    space_down: bool,
    menu: Option<MenuKind>,
    snap_steps: u32,
    saved_project: SequenceProject,
    selected_track: usize,
    active_player: usize,
    all_players_view: bool,
    cursor_step: u32,
    detail_open: bool,
    half_step_w: f32,
    timeline_scroll: ScrollAreaState,
    buttons: ButtonInteractionState<Action>,
    drag: Option<DragKind>,
    drag_before: Option<SequenceProject>,
    undo: Vec<SequenceProject>,
    redo: Vec<SequenceProject>,
    clipboard: Option<MidiNote>,
    measures_input: Option<String>,
    pending_measure_text: Option<String>,
    status: String,
    audio: Option<AudioPlayer>,
    render_rx: Option<Receiver<RenderMessage>>,
    export_rx: Option<Receiver<std::result::Result<String, String>>>,
    render_cancel: Option<Arc<AtomicBool>>,
    render_started: Option<Instant>,
    render_key: Option<(SequenceProject, Option<usize>)>,
    playback_cache: Option<PlaybackCache>,
    rendering: bool,
    play_started: Option<Instant>,
    playback_timing: Option<PlaybackTiming>,
    playback_all_players: bool,
    playback_looped: bool,
    wave_dragging: bool,
    wave_drag_before: Option<SequenceProject>,
}

impl GbseqApp {
    pub fn new(size: Size) -> Self {
        Self::new_with_audio(size, true)
    }

    fn new_with_audio(size: Size, enable_audio: bool) -> Self {
        let audio = enable_audio
            .then(AudioPlayer::new)
            .transpose()
            .ok()
            .flatten();
        let status = if enable_audio && audio.is_none() {
            "READY - audio output unavailable; WAV/export remain available".to_string()
        } else {
            "READY - double-click empty lane to add; double-click a bar for detail".to_string()
        };
        Self {
            size: size.clamped(),
            project: SequenceProject::demo(),
            current_path: None,
            selected_note: None,
            selected_notes: BTreeSet::new(),
            selection_modifier: false,
            space_down: false,
            menu: None,
            snap_steps: 1,
            saved_project: SequenceProject::demo(),
            selected_track: 0,
            active_player: 0,
            all_players_view: false,
            cursor_step: 0,
            detail_open: false,
            half_step_w: 34.0,
            timeline_scroll: ScrollAreaState::new(),
            buttons: ButtonInteractionState::new(),
            drag: None,
            drag_before: None,
            undo: Vec::new(),
            redo: Vec::new(),
            clipboard: None,
            measures_input: None,
            pending_measure_text: None,
            status,
            audio,
            render_rx: None,
            export_rx: None,
            render_cancel: None,
            render_started: None,
            render_key: None,
            playback_cache: None,
            rendering: false,
            play_started: None,
            playback_timing: None,
            playback_all_players: false,
            playback_looped: false,
            wave_dragging: false,
            wave_drag_before: None,
        }
    }

    fn root_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.size.width, self.size.height)
    }

    fn toolbar_rect(&self) -> Rect {
        Rect::new(0.0, HEADER_H, self.size.width, TOOLBAR_H)
    }

    fn inspector_rect(&self) -> Rect {
        let width = INSPECTOR_W.min((self.size.width * 0.32).max(250.0));
        Rect::new(
            self.size.width - width,
            HEADER_H + TOOLBAR_H,
            width,
            (self.size.height - HEADER_H - TOOLBAR_H - STATUS_H).max(120.0),
        )
    }

    fn ruler_rect(&self) -> Rect {
        let inspector = self.inspector_rect();
        Rect::new(
            TRACK_HEADER_W,
            HEADER_H + TOOLBAR_H,
            (inspector.x - TRACK_HEADER_W).max(160.0),
            RULER_H,
        )
    }

    fn timeline_viewport(&self) -> Rect {
        let inspector = self.inspector_rect();
        Rect::new(
            TRACK_HEADER_W,
            HEADER_H + TOOLBAR_H + RULER_H,
            (inspector.x - TRACK_HEADER_W).max(160.0),
            (self.size.height - HEADER_H - TOOLBAR_H - RULER_H - STATUS_H - SCROLLBAR_H).max(180.0),
        )
    }

    fn scrollbar_rect(&self) -> Rect {
        let timeline = self.timeline_viewport();
        Rect::new(timeline.x, timeline.bottom(), timeline.width, SCROLLBAR_H)
    }

    fn scroll_area_rect(&self) -> Rect {
        let timeline = self.timeline_viewport();
        Rect::new(
            timeline.x,
            timeline.y,
            timeline.width,
            timeline.height + SCROLLBAR_H,
        )
    }

    fn status_rect(&self) -> Rect {
        Rect::new(0.0, self.size.height - STATUS_H, self.size.width, STATUS_H)
    }

    fn lane_rect(&self, track_index: usize) -> Rect {
        let viewport = self.timeline_viewport();
        let lane_h = viewport.height / self.visible_track_count() as f32;
        let lane_index = track_index.saturating_sub(self.visible_track_start());
        Rect::new(
            viewport.x,
            viewport.y + lane_h * lane_index as f32,
            viewport.width,
            lane_h,
        )
    }

    fn all_players_visible(&self) -> bool {
        self.all_players_view && self.project.link16_mode == Link16Mode::Dmg07_16
    }

    fn visible_track_start(&self) -> usize {
        if self.all_players_visible() {
            0
        } else {
            self.active_player * TRACK_COUNT
        }
    }

    fn visible_track_count(&self) -> usize {
        if self.all_players_visible() {
            self.project.player_count() * TRACK_COUNT
        } else {
            TRACK_COUNT
        }
    }

    fn header_lane_rect(&self, track_index: usize) -> Rect {
        let lane = self.lane_rect(track_index);
        Rect::new(0.0, lane.y, TRACK_HEADER_W, lane.height)
    }

    fn timeline_content_size(&self) -> Size {
        Size::new(
            self.project.total_steps() as f32 * self.half_step_w,
            self.timeline_viewport().height,
        )
    }

    fn scroll_x(&self) -> f32 {
        self.timeline_scroll.offset().x
    }

    fn step_x(&self, step: u32) -> f32 {
        self.timeline_viewport().x + step as f32 * self.half_step_w - self.scroll_x()
    }

    fn point_step(&self, point: Point) -> u32 {
        let local = point.x - self.timeline_viewport().x + self.scroll_x();
        let step =
            (local / self.half_step_w / self.snap_steps as f32).round() * self.snap_steps as f32;
        step.round()
            .clamp(0.0, self.project.total_steps().saturating_sub(1) as f32) as u32
    }

    fn point_track(&self, point: Point) -> Option<usize> {
        let viewport = self.timeline_viewport();
        if !viewport.contains(point) {
            return None;
        }
        let track_count = self.visible_track_count();
        let lane_h = viewport.height / track_count as f32;
        let local = ((point.y - viewport.y) / lane_h)
            .floor()
            .clamp(0.0, (track_count - 1) as f32) as usize;
        Some(self.visible_track_start() + local)
    }

    fn point_pitch(&self, track_index: usize, point: Point) -> u8 {
        if self.all_players_visible() {
            return if track_index % TRACK_COUNT == 3 {
                35
            } else {
                24
            };
        }
        if track_index % TRACK_COUNT == 3 {
            return 35;
        }
        let lane = self.lane_rect(track_index);
        let top = lane.y + 27.0;
        let usable = (lane.height - 42.0).max(24.0);
        let ratio = ((point.y - top) / usable).clamp(0.0, 1.0);
        (47.0 - ratio * 47.0).round().clamp(0.0, 47.0) as u8
    }

    fn note_rect(&self, track_index: usize, note: &MidiNote) -> Rect {
        let lane = self.lane_rect(track_index);
        let y = if self.all_players_visible() {
            lane.y + (lane.height - NOTE_H) * 0.5
        } else if track_index % TRACK_COUNT == 3 {
            lane.y + (lane.height - NOTE_H) * 0.56
        } else {
            let top = lane.y + 27.0;
            let usable = (lane.height - 42.0 - NOTE_H).max(20.0);
            top + (47 - note.pitch.min(47)) as f32 / 47.0 * usable
        };
        Rect::new(
            self.step_x(note.start),
            y,
            (note.length as f32 * self.half_step_w - 3.0).max(12.0),
            NOTE_H,
        )
    }

    fn note_hit(&self, point: Point) -> Option<NoteHit> {
        let viewport = self.timeline_viewport();
        if !viewport.contains(point) {
            return None;
        }
        let start = self.visible_track_start();
        let end = start + self.visible_track_count();
        for track_index in (start..end).rev() {
            let track = &self.project.tracks[track_index];
            for note in track.notes.iter().rev() {
                let rect = self.note_rect(track_index, note);
                if rect.contains(point) {
                    let part = if point.x >= rect.right() - NOTE_HANDLE_W {
                        NoteHitPart::Resize
                    } else {
                        NoteHitPart::Body
                    };
                    return Some(NoteHit {
                        id: note.id,
                        track: track_index,
                        part,
                    });
                }
            }
        }
        None
    }

    fn active_user_wave_index(&self) -> Option<usize> {
        let id = self.selected_note?;
        let (track_index, note_index) = self.project.note_location(id)?;
        if self.project.tracks[track_index].kind != TrackKind::Wave {
            return None;
        }
        let preset = self.project.tracks[track_index].notes[note_index]
            .effects
            .wave_preset
            .min(15) as usize;
        let source = self.project.wave_slots.get(preset)?.source;
        (source >= 0x80)
            .then_some(usize::from(source & 0x3F))
            .filter(|index| *index < self.project.user_waves.len())
    }

    fn wave_editor_rect(&self) -> Option<Rect> {
        self.active_user_wave_index()?;
        let inspector = self.inspector_rect();
        Some(Rect::new(
            inspector.x + 14.0,
            inspector.bottom() - 154.0,
            inspector.width - 28.0,
            112.0,
        ))
    }

    fn edit_wave_at(&mut self, point: Point) -> bool {
        let Some(rect) = self.wave_editor_rect() else {
            return false;
        };
        if !rect.contains(point) {
            return false;
        }
        let Some(wave_index) = self.active_user_wave_index() else {
            return false;
        };
        let sample = (((point.x - rect.x) / rect.width) * WAVE_SAMPLE_COUNT as f32)
            .floor()
            .clamp(0.0, (WAVE_SAMPLE_COUNT - 1) as f32) as usize;
        let value = (15.0 - ((point.y - rect.y) / rect.height * 15.0))
            .round()
            .clamp(0.0, 15.0) as u8;
        let changed = self.project.user_waves[wave_index].samples[sample] != value;
        self.project.user_waves[wave_index].samples[sample] = value;
        changed
    }

    fn toolbar_buttons(&self) -> Vec<ButtonSpec> {
        let mut x = 12.0;
        let y = HEADER_H + 7.0;
        let mut buttons = Vec::new();
        let specs = [
            (Action::Menu(MenuKind::File), "FILE", 86.0),
            (Action::Menu(MenuKind::Edit), "EDIT", 86.0),
            (Action::Menu(MenuKind::View), "VIEW", 86.0),
            (Action::Menu(MenuKind::Export), "EXPORT", 102.0),
            (Action::Play, "PLAY", 90.0),
            (Action::Stop, "STOP", 86.0),
            (Action::Preview, "AUDITION", 112.0),
            (Action::Save, "SAVE", 86.0),
            (Action::Undo, "", 34.0),
            (Action::Redo, "", 34.0),
            (
                Action::ToggleLoop,
                if self.project.loop_playback {
                    "LOOP ON"
                } else {
                    "LOOP OFF"
                },
                110.0,
            ),
        ];
        for (action, label, width) in specs {
            buttons.push(ButtonSpec {
                action,
                rect: Rect::new(x, y, width, 31.0),
                label: label.to_string(),
            });
            x += width + 6.0;
        }
        x = 12.0;
        let y = HEADER_H + 49.0;
        for (action, label, width) in [
            (Action::Add, "ADD", 72.0),
            (Action::Duplicate, "DUP", 88.0),
            (Action::Delete, "DEL", 76.0),
            (Action::Quantize, "ALIGN", 88.0),
            (
                Action::Snap,
                match self.snap_steps {
                    1 => "1/16",
                    2 => "1/8",
                    _ => "1/4",
                },
                100.0,
            ),
            (Action::Zoom(-1), "", 34.0),
            (Action::Zoom(1), "", 34.0),
            (Action::FitSong, "FIT", 80.0),
            (Action::Bpm(-5), "BPM-", 52.0),
            (Action::Bpm(5), "BPM+", 52.0),
            (Action::Measures(-1), "BAR-", 52.0),
        ] {
            buttons.push(ButtonSpec {
                action,
                rect: Rect::new(x, y, width, 31.0),
                label: label.to_string(),
            });
            x += width + 6.0;
        }
        x += MEASURES_INPUT_W + 6.0;
        for (action, label, width) in [(Action::Measures(1), "BAR+", 52.0)] {
            buttons.push(ButtonSpec {
                action,
                rect: Rect::new(x, y, width, 31.0),
                label: label.to_string(),
            });
            x += width + 6.0;
        }
        buttons
    }

    fn measures_input_rect(&self) -> Rect {
        Rect::new(MEASURES_INPUT_X, HEADER_H + 49.0, MEASURES_INPUT_W, 31.0)
    }

    fn track_buttons(&self) -> Vec<ButtonSpec> {
        let mut buttons = Vec::new();
        let first = self.visible_track_start();
        for index in first..first + self.visible_track_count() {
            let lane = self.header_lane_rect(index);
            let button_y = lane.y + (lane.height - 24.0) * 0.5;
            buttons.push(ButtonSpec {
                action: Action::ToggleMute(index),
                rect: Rect::new(lane.x + 116.0, button_y, 28.0, 24.0),
                label: "M".to_string(),
            });
            buttons.push(ButtonSpec {
                action: Action::ToggleSolo(index),
                rect: Rect::new(lane.x + 150.0, button_y, 28.0, 24.0),
                label: "S".to_string(),
            });
        }
        buttons
    }

    fn inspector_buttons(&self) -> Vec<ButtonSpec> {
        if !self.detail_open || self.selected_note.is_none() {
            return Vec::new();
        }
        let rect = self.inspector_rect();
        let mut buttons = vec![
            ButtonSpec {
                action: Action::CloseInspector,
                rect: Rect::new(rect.right() - 34.0, rect.y + 12.0, 22.0, 22.0),
                label: "X".to_string(),
            },
            ButtonSpec {
                action: Action::Preview,
                rect: Rect::new(rect.x + 14.0, rect.y + 52.0, 84.0, 29.0),
                label: "AUDITION".to_string(),
            },
            ButtonSpec {
                action: Action::Duplicate,
                rect: Rect::new(rect.x + 104.0, rect.y + 52.0, 76.0, 29.0),
                label: "DUPLICATE".to_string(),
            },
            ButtonSpec {
                action: Action::Delete,
                rect: Rect::new(rect.x + 186.0, rect.y + 52.0, 66.0, 29.0),
                label: "DELETE".to_string(),
            },
        ];
        let fields = self.inspector_fields();
        for (row, field) in fields.iter().enumerate() {
            let y = rect.y + 112.0 + row as f32 * 39.0;
            buttons.push(ButtonSpec {
                action: Action::Adjust(*field, -1),
                rect: Rect::new(rect.x + 132.0, y, 30.0, 26.0),
                label: "-".to_string(),
            });
            buttons.push(ButtonSpec {
                action: Action::Adjust(*field, 1),
                rect: Rect::new(rect.right() - 44.0, y, 30.0, 26.0),
                label: "+".to_string(),
            });
        }

        let toggle_y = rect.y + 112.0 + fields.len() as f32 * 39.0;
        if let Some((track_index, _)) = self
            .selected_note
            .and_then(|id| self.project.note_location(id))
        {
            match self.project.tracks[track_index].kind {
                TrackKind::Pulse1 | TrackKind::Pulse2 => {
                    buttons.push(ButtonSpec {
                        action: Action::ToggleEnvelope,
                        rect: Rect::new(rect.x + 14.0, toggle_y, 126.0, 28.0),
                        label: "ENV DIRECTION".to_string(),
                    });
                    if track_index % TRACK_COUNT == 0 {
                        buttons.push(ButtonSpec {
                            action: Action::ToggleSweepDirection,
                            rect: Rect::new(rect.x + 148.0, toggle_y, 126.0, 28.0),
                            label: "SWEEP SIGN".to_string(),
                        });
                    }
                }
                TrackKind::Noise => {
                    buttons.push(ButtonSpec {
                        action: Action::ToggleNoiseWidth,
                        rect: Rect::new(rect.x + 14.0, toggle_y, 126.0, 28.0),
                        label: "NOISE WIDTH".to_string(),
                    });
                }
                TrackKind::Wave => {
                    buttons.push(ButtonSpec {
                        action: Action::AddUserWave,
                        rect: Rect::new(rect.x + 14.0, toggle_y, 126.0, 28.0),
                        label: "NEW USER WAVE".to_string(),
                    });
                }
            }
        }
        buttons
    }

    fn inspector_fields(&self) -> Vec<AdjustField> {
        let Some((track_index, _)) = self
            .selected_note
            .and_then(|id| self.project.note_location(id))
        else {
            return Vec::new();
        };
        let mut fields = vec![
            AdjustField::Start,
            AdjustField::Pitch,
            AdjustField::Length,
            AdjustField::Velocity,
            AdjustField::Pan,
        ];
        match self.project.tracks[track_index].kind {
            TrackKind::Pulse1 => fields.extend([
                AdjustField::Duty,
                AdjustField::EnvelopePeriod,
                AdjustField::SweepPeriod,
                AdjustField::SweepShift,
            ]),
            TrackKind::Pulse2 => fields.extend([AdjustField::Duty, AdjustField::EnvelopePeriod]),
            TrackKind::Wave => fields.extend([
                AdjustField::WavePreset,
                AdjustField::WaveSource,
                AdjustField::WaveSlotLevel,
                AdjustField::WaveLevel,
            ]),
            TrackKind::Noise => fields.extend([
                AdjustField::EnvelopePeriod,
                AdjustField::NoiseShift,
                AdjustField::NoiseDivisor,
            ]),
        }
        fields
    }

    fn all_buttons(&self) -> Vec<ButtonSpec> {
        let mut buttons = self.toolbar_buttons();
        buttons.extend(self.link16_buttons());
        buttons.extend(self.track_buttons());
        buttons.extend(self.inspector_buttons());
        buttons
    }

    fn link16_buttons(&self) -> Vec<ButtonSpec> {
        let y = 46.0;
        let mut x = 104.0;
        let mut buttons = Vec::new();
        for (mode, label, width) in [
            (Link16Mode::Local4, "LOCAL 4CH", 82.0),
            (Link16Mode::Cable8, "CABLE 8CH", 86.0),
            (Link16Mode::Dmg07_16, "DMG07 16CH", 92.0),
        ] {
            buttons.push(ButtonSpec {
                action: Action::SetLink16Mode(mode),
                rect: Rect::new(x, y, width, 28.0),
                label: label.to_string(),
            });
            x += width + 6.0;
        }
        x += 10.0;
        for player in 0..self.project.player_count() {
            buttons.push(ButtonSpec {
                action: Action::SetPlayer(player),
                rect: Rect::new(x, y, 38.0, 28.0),
                label: format!("P{}", player + 1),
            });
            x += 44.0;
        }
        if self.project.link16_mode == Link16Mode::Dmg07_16 {
            buttons.push(ButtonSpec {
                action: Action::ToggleAllPlayers,
                rect: Rect::new(x, y, 58.0, 28.0),
                label: "ALL16".to_string(),
            });
            x += 64.0;
        }
        x += 8.0;
        buttons.push(ButtonSpec {
            action: Action::ExportLink16Sav,
            rect: Rect::new(x, y, 102.0, 28.0),
            label: "LINK16 SAV".to_string(),
        });
        buttons
    }

    fn action_at(&self, point: Point) -> Option<Action> {
        if self.menu.is_some() {
            return self
                .menu_buttons()
                .into_iter()
                .chain(
                    self.toolbar_buttons()
                        .into_iter()
                        .filter(|b| matches!(b.action, Action::Menu(_))),
                )
                .find_map(|b| {
                    (b.rect.contains(point) && self.action_enabled(b.action)).then_some(b.action)
                });
        }
        self.all_buttons().into_iter().find_map(|button| {
            (button.rect.contains(point) && self.action_enabled(button.action))
                .then_some(button.action)
        })
    }

    fn action_enabled(&self, action: Action) -> bool {
        match action {
            Action::RenderWav => self.export_rx.is_none(),
            Action::Undo => !self.undo.is_empty(),
            Action::Redo => !self.redo.is_empty(),
            Action::Preview
            | Action::Duplicate
            | Action::Delete
            | Action::Quantize
            | Action::Transpose(_) => !self.selection_ids().is_empty(),
            Action::Stop => {
                self.rendering
                    || self.play_started.is_some()
                    || self.audio.as_ref().is_some_and(AudioPlayer::is_playing)
            }
            _ => true,
        }
    }

    fn menu_buttons(&self) -> Vec<ButtonSpec> {
        let Some(menu) = self.menu else {
            return Vec::new();
        };
        let (x, items): (f32, Vec<(Action, &str)>) = match menu {
            MenuKind::File => (
                12.0,
                vec![
                    (Action::New, "New          Ctrl+N"),
                    (Action::Open, "Open         Ctrl+O"),
                    (Action::Save, "Save         Ctrl+S"),
                    (Action::SaveAs, "Save as  Ctrl+Shift+S"),
                ],
            ),
            MenuKind::Edit => (
                104.0,
                vec![
                    (Action::Undo, "Undo         Ctrl+Z"),
                    (Action::Redo, "Redo         Ctrl+Y"),
                    (Action::SelectAll, "Select lane  Ctrl+A"),
                    (Action::Duplicate, "Duplicate    Ctrl+D"),
                    (Action::Delete, "Delete          Del"),
                    (Action::Quantize, "Align to snap     Q"),
                    (Action::Transpose(12), "Octave up"),
                    (Action::Transpose(-12), "Octave down"),
                ],
            ),
            MenuKind::View => (
                196.0,
                vec![
                    (Action::FitSong, "Fit song          F"),
                    (Action::Home, "Go to start    Home"),
                    (Action::Zoom(1), "Zoom in           +"),
                    (Action::Zoom(-1), "Zoom out          -"),
                    (Action::Snap, "Cycle snap        G"),
                ],
            ),
            MenuKind::Export => (
                288.0,
                vec![
                    (Action::RenderWav, "Render WAV"),
                    (Action::Export, "KITAQGB current player"),
                    (Action::ExportLink16Sav, "LINK16 saves"),
                ],
            ),
        };
        items
            .into_iter()
            .enumerate()
            .map(|(i, (action, label))| ButtonSpec {
                action,
                label: label.to_string(),
                rect: Rect::new(x, HEADER_H + 40.0 + i as f32 * 34.0, 264.0, 34.0),
            })
            .collect()
    }

    fn selection_ids(&self) -> Vec<u64> {
        if self.selected_notes.is_empty() {
            self.selected_note.into_iter().collect()
        } else {
            self.selected_notes.iter().copied().collect()
        }
    }

    fn edit_selection(&mut self, edit: NoteEdit) {
        let before = self.project.clone();
        let ids = self.selection_ids();
        match edit_notes(&mut self.project, &ids, edit) {
            Ok(ids) => {
                self.selected_note = ids.first().copied();
                self.selected_notes = ids.into_iter().collect();
                self.push_undo(before);
                self.status = format!("EDIT {:?} / {} selected", edit, self.selected_notes.len());
            }
            Err(error) => self.status = format!("EDIT unchanged: {error}"),
        }
    }

    fn confirm_replace(&mut self) -> bool {
        if self.project == self.saved_project {
            return true;
        }
        match rfd::MessageDialog::new()
            .set_title("GBSEQ - Unsaved changes")
            .set_description("Save this sequence before replacing it?")
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show()
        {
            rfd::MessageDialogResult::Yes => {
                self.save_dialog();
                self.project == self.saved_project
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    fn push_undo(&mut self, before: SequenceProject) {
        if before == self.project {
            return;
        }
        self.undo.push(before);
        if self.undo.len() > 96 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn begin_measures_input(&mut self) {
        self.pending_measure_text = None;
        self.measures_input = Some(String::new());
        self.buttons.clear();
        self.status = format!(
            "TYPE MEASURES {}..{} AND PRESS ENTER",
            MIN_MEASURES, MAX_MEASURES
        );
    }

    fn append_measures_input(&mut self, text: &str) -> bool {
        let Some(input) = self.measures_input.as_mut() else {
            return false;
        };
        for character in text.chars().filter(char::is_ascii_digit) {
            if input.len() < 3 {
                input.push(character);
            }
        }
        true
    }

    fn finish_measures_input(&mut self, commit: bool) {
        self.pending_measure_text = None;
        let Some(input) = self.measures_input.take() else {
            return;
        };
        if !commit || input.is_empty() {
            self.status = format!("MEASURES unchanged at {}", self.project.measures);
            return;
        }
        match input.parse::<u32>() {
            Ok(measures) if (MIN_MEASURES..=MAX_MEASURES).contains(&measures) => {
                self.edit_project(|project| project.set_measures(measures));
                self.status = format!("MEASURES {}", self.project.measures);
            }
            _ => {
                self.status = format!(
                    "MEASURES must be {}..{}; unchanged at {}",
                    MIN_MEASURES, MAX_MEASURES, self.project.measures
                );
            }
        }
    }

    fn handle_measures_input_key(&mut self, key: &str) -> UiResponse {
        match normalized_key(key).as_str() {
            "ENTER" => self.finish_measures_input(true),
            "ESCAPE" => self.finish_measures_input(false),
            "BACKSPACE" => {
                if let Some(input) = self.measures_input.as_mut() {
                    input.pop();
                }
            }
            digits if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) => {
                // plita also supports backends that deliver printable keys without TextInput.
                self.append_measures_input(digits);
                self.pending_measure_text = Some(digits.to_string());
            }
            _ => {}
        }
        UiResponse::redraw()
    }

    fn edit_project(&mut self, edit: impl FnOnce(&mut SequenceProject)) {
        let before = self.project.clone();
        edit(&mut self.project);
        self.project.repair();
        self.push_undo(before);
        self.ensure_selection_exists();
        self.clamp_scroll();
    }

    fn ensure_selection_exists(&mut self) {
        self.selected_notes
            .retain(|id| self.project.note(*id).is_some());
        if self
            .selected_note
            .is_some_and(|id| self.project.note(id).is_none())
        {
            self.selected_note = None;
            self.detail_open = false;
        }
        self.cursor_step = self
            .cursor_step
            .min(self.project.total_steps().saturating_sub(1));
    }

    fn undo(&mut self) {
        let Some(previous) = self.undo.pop() else {
            self.status = "UNDO stack is empty".to_string();
            return;
        };
        self.redo
            .push(std::mem::replace(&mut self.project, previous));
        self.ensure_selection_exists();
        self.clamp_scroll();
        self.status = "UNDO".to_string();
    }

    fn redo(&mut self) {
        let Some(next) = self.redo.pop() else {
            self.status = "REDO stack is empty".to_string();
            return;
        };
        self.undo.push(std::mem::replace(&mut self.project, next));
        self.ensure_selection_exists();
        self.clamp_scroll();
        self.status = "REDO".to_string();
    }

    fn clamp_scroll(&mut self) {
        let content = self.timeline_content_size();
        self.timeline_scroll.set_offset(
            self.timeline_scroll.offset(),
            self.scroll_area_rect(),
            content,
        );
    }

    fn add_note_at_cursor(&mut self) {
        self.selected_notes.clear();
        let before = self.project.clone();
        let pitch = if self.selected_track % TRACK_COUNT == 1 {
            12
        } else {
            24
        };
        match self
            .project
            .add_note(self.selected_track, self.cursor_step, pitch)
        {
            Ok(id) => {
                self.selected_note = Some(id);
                self.detail_open = true;
                self.push_undo(before);
                self.status = format!(
                    "ADDED {} at {}",
                    note_label(pitch),
                    position_label(self.cursor_step)
                );
            }
            Err(error) => self.status = format!("ADD failed: {error}"),
        }
    }

    fn delete_selected(&mut self) {
        if self.selected_notes.len() > 1 {
            self.edit_selection(NoteEdit::Delete);
            return;
        }
        self.selected_notes.clear();
        let Some(id) = self.selected_note else {
            self.status = "Select a MIDI bar first".to_string();
            return;
        };
        let before = self.project.clone();
        if self.project.delete_note(id) {
            self.push_undo(before);
            self.selected_note = None;
            self.detail_open = false;
            self.status = "MIDI bar deleted".to_string();
        }
    }

    fn duplicate_selected(&mut self) {
        if self.selected_notes.len() > 1 {
            self.edit_selection(NoteEdit::Duplicate);
            return;
        }
        self.selected_notes.clear();
        let Some(id) = self.selected_note else {
            self.status = "Select a MIDI bar first".to_string();
            return;
        };
        let before = self.project.clone();
        match self.project.duplicate_note(id) {
            Ok(new_id) => {
                self.selected_note = Some(new_id);
                self.detail_open = true;
                self.push_undo(before);
                self.status = "MIDI bar duplicated".to_string();
            }
            Err(error) => self.status = format!("DUPLICATE failed: {error}"),
        }
    }

    fn copy_selected(&mut self) {
        self.clipboard = self
            .selected_note
            .and_then(|id| self.project.note(id))
            .cloned();
        self.status = if self.clipboard.is_some() {
            "COPIED MIDI bar".to_string()
        } else {
            "Select a MIDI bar first".to_string()
        };
    }

    fn paste_at_cursor(&mut self) {
        let Some(source) = self.clipboard.clone() else {
            self.status = "Clipboard is empty".to_string();
            return;
        };
        let before = self.project.clone();
        let track_index = self.selected_track;
        match self
            .project
            .copy_note_to_track(track_index, self.cursor_step, &source)
        {
            Ok(id) => {
                self.selected_note = Some(id);
                self.detail_open = true;
                self.push_undo(before);
                self.status = "PASTED MIDI bar".to_string();
            }
            Err(error) => self.status = format!("PASTE failed: {error}"),
        }
    }

    fn adjust_selected(&mut self, field: AdjustField, delta: i8) {
        let Some(id) = self.selected_note else {
            return;
        };
        let before = self.project.clone();
        let Some((track_index, note_index)) = self.project.note_location(id) else {
            return;
        };
        let wave_preset = self.project.tracks[track_index].notes[note_index]
            .effects
            .wave_preset
            .min(15) as usize;
        let user_wave_count = self.project.user_waves.len();

        let changed = match field {
            AdjustField::Start => {
                let note = &self.project.tracks[track_index].notes[note_index];
                let start = add_signed_u32(note.start, i32::from(delta));
                self.project.move_note(id, start, note.pitch)
            }
            AdjustField::Pitch => {
                let note = &self.project.tracks[track_index].notes[note_index];
                let pitch = add_signed_u8(note.pitch, delta, 0, 47);
                self.project.move_note(id, note.start, pitch)
            }
            AdjustField::Length => {
                let length = self.project.tracks[track_index].notes[note_index].length;
                self.project
                    .resize_note(id, add_signed_u32(length, i32::from(delta)).max(1))
            }
            AdjustField::Velocity => {
                let note = &mut self.project.tracks[track_index].notes[note_index];
                let next = add_signed_u8(note.velocity, delta, 0, 15);
                let changed = note.velocity != next;
                note.velocity = next;
                changed
            }
            AdjustField::Pan => {
                let note = &mut self.project.tracks[track_index].notes[note_index];
                let next = note.effects.pan.shifted(delta);
                let changed = note.effects.pan != next;
                note.effects.pan = next;
                changed
            }
            AdjustField::Duty => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .duty,
                delta,
                0,
                3,
            ),
            AdjustField::EnvelopePeriod => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .envelope_period,
                delta,
                0,
                7,
            ),
            AdjustField::SweepPeriod => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .sweep_period,
                delta,
                0,
                7,
            ),
            AdjustField::SweepShift => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .sweep_shift,
                delta,
                0,
                7,
            ),
            AdjustField::WaveLevel => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .wave_level,
                delta,
                1,
                3,
            ),
            AdjustField::WavePreset => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .wave_preset,
                delta,
                0,
                15,
            ),
            AdjustField::WaveSource => {
                let slot = &mut self.project.wave_slots[wave_preset];
                let current = if slot.source >= 0x80 {
                    7 + usize::from(slot.source & 0x3F)
                } else {
                    usize::from(slot.source.min(6))
                };
                let count = 7 + user_wave_count;
                let next = if delta < 0 {
                    (current + count - 1) % count
                } else {
                    (current + 1) % count
                };
                let source = if next < 7 {
                    next as u8
                } else {
                    0x80 | (next - 7) as u8
                };
                let changed = slot.source != source;
                slot.source = source;
                changed
            }
            AdjustField::WaveSlotLevel => {
                adjust_effect_u8(&mut self.project.wave_slots[wave_preset].level, delta, 1, 3)
            }
            AdjustField::NoiseShift => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .noise_shift,
                delta,
                0,
                15,
            ),
            AdjustField::NoiseDivisor => adjust_effect_u8(
                &mut self.project.tracks[track_index].notes[note_index]
                    .effects
                    .noise_divisor,
                delta,
                0,
                7,
            ),
        };
        if changed {
            self.push_undo(before);
            self.status = format!("DETAIL {} changed", field_label(field));
        }
    }

    fn toggle_selected(&mut self, action: Action) {
        let Some(id) = self.selected_note else {
            return;
        };
        let before = self.project.clone();
        let Some(note) = self.project.note_mut(id) else {
            return;
        };
        match action {
            Action::ToggleEnvelope => {
                note.effects.envelope_increase = !note.effects.envelope_increase
            }
            Action::ToggleSweepDirection => note.effects.sweep_negate = !note.effects.sweep_negate,
            Action::ToggleNoiseWidth => note.effects.noise_width7 = !note.effects.noise_width7,
            _ => return,
        }
        self.push_undo(before);
        self.status = "DETAIL switch changed".to_string();
    }

    fn add_user_wave(&mut self) {
        if self.project.user_waves.len() >= MAX_USER_WAVES {
            self.status = "USER WAVE limit is 64".to_string();
            return;
        }
        let Some(id) = self.selected_note else {
            self.status = "Select a CH3 MIDI bar first".to_string();
            return;
        };
        let Some((track_index, note_index)) = self.project.note_location(id) else {
            return;
        };
        if self.project.tracks[track_index].kind != TrackKind::Wave {
            self.status = "User waves are available on CH3".to_string();
            return;
        }
        let before = self.project.clone();
        let index = self.project.user_waves.len();
        let mut wave = UserWave::default();
        wave.name = format!("User Wave {}", index + 1);
        self.project.user_waves.push(wave);
        let preset = self.project.tracks[track_index].notes[note_index]
            .effects
            .wave_preset
            .min(15) as usize;
        self.project.wave_slots[preset].source = 0x80 | index as u8;
        self.push_undo(before);
        self.status = format!(
            "USER WAVE {} created and assigned to preset {}",
            index + 1,
            preset
        );
    }

    fn apply_action(&mut self, action: Action) {
        if !matches!(action, Action::Menu(_)) {
            self.menu = None;
        }
        match action {
            Action::Menu(menu) => {
                self.menu = if self.menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                }
            }
            Action::SaveAs => {
                if let Some(path) = FileDialog::new()
                    .add_filter("GBSEQ project", &["gbseq"])
                    .set_file_name("sequence.gbseq")
                    .save_file()
                {
                    if let Err(error) = self.save_path(&path) {
                        self.status = format!("SAVE failed: {error}");
                    }
                }
            }
            Action::SelectAll => {
                self.selected_notes = self.project.tracks[self.selected_track]
                    .notes
                    .iter()
                    .map(|n| n.id)
                    .collect();
                self.selected_note = self.selected_notes.first().copied();
                self.status = format!(
                    "{} notes selected in current lane / arrows move and transpose",
                    self.selected_notes.len()
                );
            }
            Action::Quantize => self.edit_selection(NoteEdit::Quantize(self.snap_steps)),
            Action::Transpose(delta) => self.edit_selection(NoteEdit::Transpose(delta)),
            Action::FitSong => {
                self.half_step_w = (self.timeline_viewport().width
                    / self.project.total_steps() as f32)
                    .clamp(0.02, MAX_HALF_STEP_W);
                self.timeline_scroll = ScrollAreaState::new();
                self.status = "FIT whole song / + zooms back in".to_string();
            }
            Action::Home => {
                self.cursor_step = 0;
                self.timeline_scroll = ScrollAreaState::new();
            }
            Action::Snap => {
                self.snap_steps = match self.snap_steps {
                    1 => 2,
                    2 => 4,
                    _ => 1,
                };
                self.status = format!("SNAP {} steps", self.snap_steps);
            }
            Action::New => {
                if !self.confirm_replace() {
                    return;
                }
                self.stop_audio();
                self.drag = None;
                self.drag_before = None;
                self.wave_dragging = false;
                self.wave_drag_before = None;
                self.project = SequenceProject::empty();
                self.saved_project = self.project.clone();
                self.selected_notes.clear();
                self.current_path = None;
                self.selected_note = None;
                self.active_player = 0;
                self.all_players_view = false;
                self.selected_track = 0;
                self.detail_open = false;
                self.undo.clear();
                self.redo.clear();
                self.timeline_scroll = ScrollAreaState::new();
                self.status = "NEW sequence".to_string();
            }
            Action::Open => {
                if self.confirm_replace() {
                    self.open_dialog();
                }
            }
            Action::Save => self.save_dialog(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Add => self.add_note_at_cursor(),
            Action::Duplicate => self.duplicate_selected(),
            Action::Delete => self.delete_selected(),
            Action::Play => self.start_playback(),
            Action::Stop => self.stop_audio(),
            Action::Preview => self.preview_selected(),
            Action::RenderWav => self.render_wav_dialog(),
            Action::Export => self.export_dialog(),
            Action::ExportLink16Sav => self.export_link16_sav_dialog(),
            Action::AddUserWave => self.add_user_wave(),
            Action::SetLink16Mode(mode) => {
                self.selected_notes.clear();
                let before = self.project.clone();
                self.project.set_link16_mode(mode);
                self.active_player = self.active_player.min(self.project.player_count() - 1);
                self.all_players_view = false;
                self.selected_track = self.active_player * TRACK_COUNT;
                self.selected_note = None;
                self.detail_open = false;
                self.push_undo(before);
                self.status = format!("{} / {} players", mode.label(), mode.player_count());
            }
            Action::SetPlayer(player) => {
                self.selected_notes.clear();
                if player < self.project.player_count() {
                    self.active_player = player;
                    self.all_players_view = false;
                    self.selected_track = player * TRACK_COUNT;
                    self.selected_note = None;
                    self.detail_open = false;
                    self.status = format!(
                        "EDIT PLAYER {} / {}",
                        player + 1,
                        self.project.link16_mode.label()
                    );
                }
            }
            Action::ToggleAllPlayers => {
                self.selected_notes.clear();
                if self.project.link16_mode == Link16Mode::Dmg07_16 {
                    self.all_players_view = !self.all_players_view;
                    if self.all_players_view {
                        self.status = "ALL16 overview - PLAY mixes all four players".to_string();
                    } else {
                        self.active_player = self.selected_track / TRACK_COUNT;
                        self.status = format!("EDIT PLAYER {}", self.active_player + 1);
                    }
                    self.selected_note = None;
                    self.detail_open = false;
                }
            }
            Action::ToggleLoop => {
                self.edit_project(|project| project.loop_playback = !project.loop_playback);
                self.status = if self.project.loop_playback {
                    "LOOP ON - playback repeats".to_string()
                } else {
                    "LOOP OFF - playback stops at the finale".to_string()
                };
            }
            Action::Bpm(delta) => {
                self.edit_project(|project| {
                    project.bpm = (i32::from(project.bpm) + i32::from(delta)).clamp(30, 300) as u16;
                });
                self.status = format!("BPM {}", self.project.bpm);
            }
            Action::Measures(delta) => {
                let next = (self.project.measures as i32 + i32::from(delta))
                    .clamp(MIN_MEASURES as i32, MAX_MEASURES as i32)
                    as u32;
                self.edit_project(|project| project.set_measures(next));
                self.status = format!("MEASURES {}", self.project.measures);
            }
            Action::Zoom(delta) => self.zoom_timeline(delta),
            Action::ToggleMute(index) => {
                self.edit_project(|project| {
                    project.tracks[index].mute = !project.tracks[index].mute
                });
                self.status = format!(
                    "{} MUTE {}",
                    self.project.tracks[index].kind.short_label(),
                    on_off(self.project.tracks[index].mute)
                );
            }
            Action::ToggleSolo(index) => {
                self.edit_project(|project| {
                    project.tracks[index].solo = !project.tracks[index].solo
                });
                self.status = format!(
                    "{} SOLO {}",
                    self.project.tracks[index].kind.short_label(),
                    on_off(self.project.tracks[index].solo)
                );
            }
            Action::Adjust(field, delta) => self.adjust_selected(field, delta),
            Action::ToggleEnvelope | Action::ToggleSweepDirection | Action::ToggleNoiseWidth => {
                self.toggle_selected(action)
            }
            Action::CloseInspector => {
                self.detail_open = false;
                self.status = "DETAIL closed".to_string();
            }
        }
    }

    fn zoom_timeline(&mut self, delta: i8) {
        let old = self.half_step_w;
        let anchor_step = (self.scroll_x() + self.timeline_viewport().width * 0.5) / old.max(1.0);
        self.half_step_w = (self.half_step_w * if delta > 0 { 1.2 } else { 1.0 / 1.2 })
            .clamp(0.02, MAX_HALF_STEP_W);
        let target = anchor_step * self.half_step_w - self.timeline_viewport().width * 0.5;
        self.timeline_scroll.set_offset(
            Point::new(target, 0.0),
            self.scroll_area_rect(),
            self.timeline_content_size(),
        );
        self.status = format!("ZOOM {:.0} px/beat", self.half_step_w * 2.0);
    }

    fn load_path(&mut self, path: &Path) -> Result<()> {
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut project: SequenceProject =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        project.repair();
        project.validate()?;
        self.stop_audio();
        self.drag = None;
        self.drag_before = None;
        self.wave_dragging = false;
        self.wave_drag_before = None;
        self.menu = None;
        self.buttons.clear();
        self.project = project;
        self.saved_project = self.project.clone();
        self.selected_notes.clear();
        self.active_player = 0;
        self.all_players_view = false;
        self.selected_track = 0;
        self.current_path = Some(path.to_path_buf());
        self.selected_note = None;
        self.detail_open = false;
        self.undo.clear();
        self.redo.clear();
        self.timeline_scroll = ScrollAreaState::new();
        self.status = format!("OPENED {}", path.display());
        Ok(())
    }

    fn save_path(&mut self, path: &Path) -> Result<()> {
        self.project.validate()?;
        let text = serde_json::to_string_pretty(&self.project)?;
        fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
        self.saved_project = self.project.clone();
        self.current_path = Some(path.to_path_buf());
        self.status = format!("SAVED {}", path.display());
        Ok(())
    }

    fn open_dialog(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("GBSEQ project", &["gbseq", "json"])
            .pick_file()
        else {
            return;
        };
        if let Err(error) = self.load_path(&path) {
            self.status = format!("OPEN failed: {error}");
        }
    }

    fn save_dialog(&mut self) {
        let path = self.current_path.clone().or_else(|| {
            FileDialog::new()
                .add_filter("GBSEQ project", &["gbseq"])
                .set_file_name("sequence.gbseq")
                .save_file()
        });
        if let Some(path) = path {
            if let Err(error) = self.save_path(&path) {
                self.status = format!("SAVE failed: {error}");
            }
        }
    }

    fn export_dialog(&mut self) {
        let Some(out_dir) = FileDialog::new().pick_folder() else {
            return;
        };
        let result = self
            .project
            .to_gbdaw_player(self.active_player)
            .and_then(|project| {
                export_project_to_dir(
                    &project,
                    &out_dir,
                    StreamMode::Extended,
                    ExportPolicy::default(),
                )
                .map(|manifest| manifest.songs.first().map(|song| song.bytes).unwrap_or(0))
            });
        match result {
            Ok(bytes) => {
                self.status = format!("KITAQGB export {} bytes -> {}", bytes, out_dir.display())
            }
            Err(error) => self.status = format!("EXPORT failed: {error}"),
        }
    }

    fn export_link16_sav_dialog(&mut self) {
        let Some(out_dir) = FileDialog::new().pick_folder() else {
            return;
        };
        self.status = match export_link16_saves(&self.project, &out_dir) {
            Ok(manifest) => format!(
                "LINK16 {} players / {} steps / {} BPM -> {}",
                manifest
                    .files
                    .iter()
                    .map(|file| file.player)
                    .max()
                    .unwrap_or(1),
                manifest.steps,
                manifest.effective_bpm,
                out_dir.display()
            ),
            Err(error) => format!("LINK16 SAV failed: {error}"),
        };
    }

    fn render_wav_dialog(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("Wave audio", &["wav"])
            .set_file_name("gbseq_mix.wav")
            .save_file()
        else {
            return;
        };
        self.start_wav_export(path);
    }

    fn start_wav_export(&mut self, path: PathBuf) {
        if self.export_rx.is_some() {
            return;
        }
        let project = self.project.clone();
        let player = if self.all_players_visible() {
            None
        } else {
            Some(self.active_player)
        };
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = crate::services::render_wav(&project, &path, player)
                .map(|()| format!("WAV rendered -> {}", path.display()))
                .map_err(|error| format!("WAV failed: {error}"));
            let _ = sender.send(result);
        });
        self.export_rx = Some(receiver);
        self.status =
            "EXPORTING WAV snapshot in background - editing remains available".to_string();
    }

    fn poll_export(&mut self) -> bool {
        let Some(receiver) = self.export_rx.as_ref() else {
            return false;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.export_rx = None;
                self.status = result.unwrap_or_else(|error| error);
            }
            Err(TryRecvError::Disconnected) => {
                self.export_rx = None;
                self.status = "WAV worker disconnected".to_string();
            }
            Err(TryRecvError::Empty) => {}
        }
        true
    }

    fn start_playback(&mut self) {
        let all_players = self.all_players_visible();
        let player = if all_players {
            None
        } else {
            Some(self.active_player)
        };
        let key = (self.project.clone(), player);
        let playback_source = self.project.clone();
        let playback_timing = PlaybackTiming::from_project(&playback_source);
        let max_frames = project_frame_count(&playback_source);
        self.stop_audio();
        self.playback_timing = Some(playback_timing);
        self.playback_all_players = all_players;
        self.playback_looped = playback_timing.looped;
        self.timeline_scroll.set_offset(
            Point::new(0.0, 0.0),
            self.scroll_area_rect(),
            self.timeline_content_size(),
        );
        if let Some(cache) = self
            .playback_cache
            .as_ref()
            .filter(|cache| cache.project == key.0 && cache.player == key.1)
        {
            if let Some(audio) = self.audio.as_mut() {
                match audio.play(cache.rate, Arc::clone(&cache.samples), self.playback_looped) {
                    Ok(()) => {
                        self.play_started = Some(Instant::now());
                        self.status = "PLAYING / CACHE HIT - no render needed".to_string();
                    }
                    Err(error) => self.status = format!("PLAY failed: {error}"),
                }
                return;
            }
        }
        let projects = match crate::services::playback_projects(&playback_source, player) {
            Ok(projects) => projects,
            Err(error) => {
                self.status = format!("PLAY failed: {error}");
                return;
            }
        };
        self.render_key = Some(key);
        self.render_started = Some(Instant::now());
        let cancelled = Arc::new(AtomicBool::new(false));
        self.render_cancel = Some(Arc::clone(&cancelled));
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = crate::services::render_mixed_cancellable(
                &projects,
                max_frames,
                true,
                Some(&cancelled),
            )
            .map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        self.render_rx = Some(receiver);
        self.rendering = true;
        self.status = if all_players {
            "RENDERING ALL16 playback with KOKURA-Core...".to_string()
        } else {
            "RENDERING playback with KOKURA-Core...".to_string()
        };
    }

    fn preview_selected(&mut self) {
        let Some(id) = self.selected_note else {
            return;
        };
        self.stop_audio();
        let Some((track_index, note_index)) = self.project.note_location(id) else {
            return;
        };
        let mut preview = self.project.clone();
        preview.bpm = self.project.bpm;
        preview.measures = 1;
        preview
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        let mut note = self.project.tracks[track_index].notes[note_index].clone();
        note.start = 0;
        note.length = note.length.min(STEPS_PER_MEASURE);
        preview.tracks[track_index].notes.push(note);
        let result = preview
            .to_gbdaw_player(track_index / TRACK_COUNT)
            .and_then(|project| {
                render_project_to_samples(&project, StreamMode::Extended, 180)
                    .map(|(sample_rate, samples, _)| (sample_rate, samples))
            });
        match result {
            Ok((sample_rate, samples)) => {
                if let Some(audio) = self.audio.as_mut() {
                    match audio.play(sample_rate, samples, false) {
                        Ok(()) => self.status = "AUDITION selected MIDI bar".to_string(),
                        Err(error) => self.status = format!("AUDITION failed: {error}"),
                    }
                } else {
                    self.status = "AUDITION unavailable: no audio output".to_string();
                }
            }
            Err(error) => self.status = format!("AUDITION failed: {error}"),
        }
    }

    fn stop_audio(&mut self) {
        if let Some(cancelled) = self.render_cancel.take() {
            cancelled.store(true, Ordering::Relaxed);
        }
        self.render_key = None;
        self.render_started = None;
        if let Some(audio) = self.audio.as_mut() {
            audio.stop();
        }
        self.render_rx = None;
        self.rendering = false;
        self.play_started = None;
        self.playback_timing = None;
        self.playback_all_players = false;
        self.playback_looped = false;
        self.status = "STOPPED".to_string();
    }

    fn poll_audio_render(&mut self) -> bool {
        let Some(receiver) = self.render_rx.as_ref() else {
            if self.play_started.is_some() && self.audio.as_ref().is_some_and(AudioPlayer::finished)
            {
                self.play_started = None;
                self.playback_timing = None;
                self.status = "PLAYBACK finished".to_string();
                return true;
            }
            return self.play_started.is_some();
        };
        match receiver.try_recv() {
            Ok(Ok((sample_rate, samples))) => {
                self.render_rx = None;
                self.rendering = false;
                self.render_cancel = None;
                let elapsed_ms = self
                    .render_started
                    .take()
                    .map(|t| t.elapsed().as_secs_f64() * 1000.0)
                    .unwrap_or(0.0);
                let samples: Arc<[i16]> = samples.into();
                if let Some((project, player)) = self.render_key.take() {
                    // Keep at most one 128 MiB PCM buffer. Playback shares it without copying.
                    self.playback_cache = if samples.len() <= 64 * 1024 * 1024 {
                        Some(PlaybackCache {
                            project,
                            player,
                            rate: sample_rate,
                            samples: Arc::clone(&samples),
                        })
                    } else {
                        None
                    };
                }
                if let Some(audio) = self.audio.as_mut() {
                    match audio.play(sample_rate, samples, self.playback_looped) {
                        Ok(()) => {
                            self.play_started = Some(Instant::now());
                            self.status = if self.playback_all_players {
                                if self.playback_looped {
                                    "PLAYING ALL16 / LOOP ON".to_string()
                                } else {
                                    "PLAYING ALL16 / LOOP OFF".to_string()
                                }
                            } else if self.playback_looped {
                                "PLAYING / LOOP ON".to_string()
                            } else {
                                "PLAYING / LOOP OFF".to_string()
                            };
                            self.status
                                .push_str(&format!(" / render {:.0} ms", elapsed_ms));
                        }
                        Err(error) => {
                            self.playback_timing = None;
                            self.status = format!("PLAY failed: {error}");
                        }
                    }
                } else {
                    self.playback_timing = None;
                    self.status = "PLAY unavailable: no audio output".to_string();
                }
                true
            }
            Ok(Err(error)) => {
                self.render_rx = None;
                self.rendering = false;
                self.playback_timing = None;
                self.status = format!("PLAY render failed: {error}");
                true
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                self.render_rx = None;
                self.rendering = false;
                self.playback_timing = None;
                self.status = "PLAY render worker disconnected".to_string();
                true
            }
        }
    }

    fn playhead_step(&self) -> Option<f32> {
        let started = self.play_started?;
        let timing = self.playback_timing?;
        // Sink::get_pos follows the source consumed by the audio stream. The
        // wall clock remains only as a headless-test/fallback path.
        let elapsed = self
            .audio
            .as_ref()
            .and_then(AudioPlayer::position)
            .unwrap_or_else(|| started.elapsed());
        Some(timing.step_at(elapsed))
    }

    fn follow_playhead(&mut self) {
        let Some(step) = self.playhead_step() else {
            return;
        };
        let viewport = self.timeline_viewport();
        let content_x = step * self.half_step_w;
        let scroll = self.scroll_x();
        if content_x < scroll + viewport.width * 0.08 || content_x > scroll + viewport.width * 0.82
        {
            let target = (content_x - viewport.width * 0.18).max(0.0);
            self.timeline_scroll.set_offset(
                Point::new(target, 0.0),
                self.scroll_area_rect(),
                self.timeline_content_size(),
            );
        }
    }

    fn handle_note_drag(&mut self, point: Point) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        match drag {
            DragKind::Move {
                id,
                origin,
                start,
                pitch,
            } => {
                let delta_steps = ((point.x - origin.x) / self.half_step_w).round() as i32;
                let new_start = add_signed_u32(start, delta_steps);
                let track_index = self
                    .project
                    .note_location(id)
                    .map(|(track, _)| track)
                    .unwrap_or(self.selected_track);
                let new_pitch = if self.all_players_visible() || track_index % TRACK_COUNT == 3 {
                    pitch
                } else {
                    let lane = self.lane_rect(track_index);
                    let semitone_h = ((lane.height - 42.0) / 48.0).max(2.0);
                    let delta_pitch = (-(point.y - origin.y) / semitone_h).round() as i8;
                    add_signed_u8(pitch, delta_pitch, 0, 47)
                };
                self.project.move_note(id, new_start, new_pitch)
            }
            DragKind::Resize {
                id,
                origin_x,
                length,
            } => {
                let delta = ((point.x - origin_x) / self.half_step_w).round() as i32;
                self.project
                    .resize_note(id, add_signed_u32(length, delta).max(1))
            }
        }
    }

    fn handle_key(&mut self, key: &str, ctrl: bool, shift: bool) -> UiResponse {
        let upper = normalized_key(key);
        if self.menu.is_some() && !ctrl {
            let items: Vec<_> = self
                .menu_buttons()
                .into_iter()
                .filter(|b| self.action_enabled(b.action))
                .collect();
            match upper.as_str() {
                "ESCAPE" => {
                    self.menu = None;
                    self.buttons.clear();
                }
                "ARROWDOWN" | "ARROWUP" if !items.is_empty() => {
                    let current = items
                        .iter()
                        .position(|b| self.buttons.hovered() == Some(&b.action));
                    let next = match (current, upper.as_str()) {
                        (Some(i), "ARROWDOWN") => (i + 1) % items.len(),
                        (Some(i), _) => (i + items.len() - 1) % items.len(),
                        (None, _) => 0,
                    };
                    self.buttons.set_hovered(Some(items[next].action));
                }
                "ENTER" => {
                    if let Some(action) = items
                        .iter()
                        .find(|b| self.buttons.hovered() == Some(&b.action))
                        .or(items.first())
                        .map(|b| b.action)
                    {
                        self.apply_action(action);
                    }
                }
                _ => {}
            }
            return UiResponse::redraw();
        }
        if ctrl {
            match upper.as_str() {
                "N" => self.apply_action(Action::New),
                "O" => self.apply_action(Action::Open),
                "S" => self.apply_action(if shift { Action::SaveAs } else { Action::Save }),
                "A" => self.apply_action(Action::SelectAll),
                "Z" if shift => self.redo(),
                "Z" => self.undo(),
                "Y" => self.redo(),
                "C" => self.copy_selected(),
                "V" => self.paste_at_cursor(),
                "D" => self.duplicate_selected(),
                _ => return UiResponse::none(),
            }
            return UiResponse::redraw();
        }

        match upper.as_str() {
            "F" => self.apply_action(Action::FitSong),
            "HOME" => self.apply_action(Action::Home),
            "G" => self.apply_action(Action::Snap),
            "Q" => self.apply_action(Action::Quantize),
            "ARROWLEFT" | "ARROWRIGHT" | "ARROWUP" | "ARROWDOWN"
                if self.selected_notes.len() > 1 =>
            {
                let edit = match upper.as_str() {
                    "ARROWLEFT" => NoteEdit::Move(-(self.snap_steps as i32)),
                    "ARROWRIGHT" => NoteEdit::Move(self.snap_steps as i32),
                    "ARROWUP" => NoteEdit::Transpose(if shift { 12 } else { 1 }),
                    _ => NoteEdit::Transpose(if shift { -12 } else { -1 }),
                };
                self.edit_selection(edit);
            }
            " " | "SPACE" => {
                if self.play_started.is_some()
                    || self.rendering
                    || self.audio.as_ref().is_some_and(AudioPlayer::is_playing)
                {
                    self.stop_audio();
                } else {
                    self.start_playback();
                }
            }
            "DELETE" | "BACKSPACE" => self.delete_selected(),
            "ENTER" => {
                if self.selected_note.is_some() {
                    self.detail_open = true;
                    self.status = "DETAIL opened".to_string();
                } else {
                    self.add_note_at_cursor();
                }
            }
            "ESCAPE" => {
                self.menu = None;
                self.selected_notes.clear();
                self.detail_open = false;
                self.drag = None;
                self.status = "DETAIL closed".to_string();
            }
            "ARROWLEFT" => {
                if self.selected_note.is_some() {
                    self.adjust_selected(AdjustField::Start, if shift { -2 } else { -1 });
                } else {
                    self.cursor_step = self.cursor_step.saturating_sub(if shift { 2 } else { 1 });
                }
            }
            "ARROWRIGHT" => {
                if self.selected_note.is_some() {
                    self.adjust_selected(AdjustField::Start, if shift { 2 } else { 1 });
                } else {
                    self.cursor_step = self
                        .cursor_step
                        .saturating_add(if shift { 2 } else { 1 })
                        .min(self.project.total_steps().saturating_sub(1));
                }
            }
            "ARROWUP" => self.adjust_selected(AdjustField::Pitch, if shift { 12 } else { 1 }),
            "ARROWDOWN" => self.adjust_selected(AdjustField::Pitch, if shift { -12 } else { -1 }),
            "+" | "=" => self.zoom_timeline(1),
            "-" => self.zoom_timeline(-1),
            _ => return UiResponse::none(),
        }
        UiResponse::redraw()
    }

    fn draw_header(&self, list: &mut DrawList) {
        list.fill_vertical_gradient_rect(
            Rect::new(0.0, 0.0, self.size.width, HEADER_H),
            Color::rgb8(31, 38, 45),
            HEADER,
        );
        list.fill_rect(Rect::new(0.0, HEADER_H - 2.0, self.size.width, 2.0), ACCENT);
        draw_text(list, 15.0, 15.0, "GBSEQ", 1.55, TEXT);
        let timing_label = self
            .project
            .frames_per_step_override
            .map(|frames| format!("{} FRAME/STEP", frames))
            .unwrap_or_else(|| format!("{} BPM", self.project.bpm));
        draw_text(
            list,
            104.0,
            17.0,
            &format!(
                "{}   {}   {} BARS / {} STEPS   {}",
                format!(
                    "{}{}",
                    if self.project != self.saved_project {
                        "* "
                    } else {
                        ""
                    },
                    self.project.title
                ),
                timing_label,
                self.project.measures,
                self.project.total_steps(),
                if self.project.frames_per_step_override.is_some() {
                    "FRAME GRID"
                } else {
                    "1/4-BEAT SNAP"
                }
            ),
            1.0,
            MUTED,
        );
        let mode = if self.rendering {
            "RENDERING"
        } else if self.play_started.is_some() {
            "PLAYING"
        } else {
            "STOPPED"
        };
        draw_text(
            list,
            self.size.width - 118.0,
            17.0,
            mode,
            0.95,
            if mode == "STOPPED" { MUTED } else { PLAYHEAD },
        );
        for spec in self.link16_buttons() {
            let active = match spec.action {
                Action::SetLink16Mode(value) => value == self.project.link16_mode,
                Action::SetPlayer(value) => {
                    value == self.active_player && !self.all_players_visible()
                }
                Action::ToggleAllPlayers => self.all_players_visible(),
                _ => false,
            };
            self.draw_button(list, &spec, active);
        }
    }

    fn draw_button(&self, list: &mut DrawList, spec: &ButtonSpec, active: bool) {
        let enabled = self.action_enabled(spec.action);
        let hovered = self.buttons.hovered() == Some(&spec.action);
        let pressed = self.buttons.pressed() == Some(&spec.action);
        let fill = if active {
            ACCENT
        } else if pressed {
            Color::rgb8(77, 91, 105)
        } else if hovered {
            Color::rgb8(57, 68, 79)
        } else {
            PANEL_RAISED
        };
        list.fill_rect(spec.rect, fill);
        list.stroke_rect(
            spec.rect,
            if active { 2.0 } else { 1.0 },
            if active { ACCENT } else { BORDER },
        );
        let text_color = if !enabled {
            MUTED.with_alpha(0.4)
        } else if active {
            BG
        } else {
            TEXT
        };
        let icon = action_icon(spec.action);
        if let Some(icon) = icon {
            draw_lucide_icon(
                list,
                Rect::new(spec.rect.x + 8.0, spec.rect.y + 7.0, 18.0, 18.0),
                icon,
                text_color,
            );
        }
        draw_text(
            list,
            spec.rect.x + if icon.is_some() { 33.0 } else { 7.0 },
            spec.rect.y + 9.0,
            &spec.label,
            0.82,
            text_color,
        );
    }

    fn draw_toolbar(&self, list: &mut DrawList) {
        list.fill_rect(self.toolbar_rect(), Color::rgb8(18, 22, 27));
        list.fill_rect(
            Rect::new(
                0.0,
                self.toolbar_rect().bottom() - 1.0,
                self.size.width,
                1.0,
            ),
            BORDER,
        );
        for spec in self.toolbar_buttons() {
            let active = (matches!(spec.action, Action::Play) && self.play_started.is_some())
                || (matches!(spec.action, Action::ToggleLoop) && self.project.loop_playback);
            self.draw_button(list, &spec, active);
        }
        let input = self.measures_input_rect();
        let active = self.measures_input.is_some();
        list.fill_rect(
            input,
            if active {
                Color::rgb8(53, 62, 71)
            } else {
                Color::rgb8(15, 19, 23)
            },
        );
        list.stroke_rect(
            input,
            if active { 2.0 } else { 1.0 },
            if active { ACCENT } else { BORDER },
        );
        let value = self
            .measures_input
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.project.measures.to_string());
        draw_text(
            list,
            input.x + 9.0,
            input.y + 9.0,
            &value,
            0.86,
            if active { ACCENT } else { TEXT },
        );
    }

    fn draw_ruler(&self, list: &mut DrawList) {
        let ruler = self.ruler_rect();
        list.fill_rect(Rect::new(0.0, ruler.y, TRACK_HEADER_W, ruler.height), PANEL);
        list.fill_rect(ruler, Color::rgb8(22, 27, 32));
        list.push_clip(ruler);
        let first = (self.scroll_x() / self.half_step_w).floor().max(0.0) as u32;
        let last = ((self.scroll_x() + ruler.width) / self.half_step_w)
            .ceil()
            .min(self.project.total_steps() as f32) as u32;
        for step in first..=last {
            let x = self.step_x(step);
            if step % STEPS_PER_MEASURE == 0
                && (step / STEPS_PER_MEASURE)
                    % ((64.0 / (self.half_step_w * STEPS_PER_MEASURE as f32))
                        .ceil()
                        .max(1.0) as u32)
                    == 0
            {
                draw_text(
                    list,
                    x + 5.0,
                    ruler.y + 11.0,
                    &(step / STEPS_PER_MEASURE + 1).to_string(),
                    0.92,
                    TEXT,
                );
            } else if step % 4 == 0 && self.half_step_w >= 12.0 {
                draw_text(
                    list,
                    x + 3.0,
                    ruler.y + 16.0,
                    &((step % STEPS_PER_MEASURE) / 4 + 1).to_string(),
                    0.72,
                    MUTED,
                );
            }
        }
        list.pop_clip();
        draw_text(list, 14.0, ruler.y + 12.0, "TRACKS", 0.92, MUTED);
        list.stroke_rect(
            Rect::new(0.0, ruler.y, self.inspector_rect().x, ruler.height),
            1.0,
            BORDER,
        );
    }

    fn draw_tracks(&self, list: &mut DrawList) {
        let viewport = self.timeline_viewport();
        let first_track = self.visible_track_start();
        let track_count = self.visible_track_count();
        for track_index in first_track..first_track + track_count {
            let header = self.header_lane_rect(track_index);
            list.fill_rect(
                header,
                if self.selected_track == track_index {
                    Color::rgb8(37, 46, 55)
                } else if track_index % TRACK_COUNT % 2 == 0 {
                    PANEL
                } else {
                    PANEL_ALT
                },
            );
            list.stroke_rect(header, 1.0, BORDER);
            let track = &self.project.tracks[track_index];
            let color = track_color(track_index);
            list.fill_rect(Rect::new(header.x, header.y, 5.0, header.height), color);
            if self.all_players_visible() {
                draw_text(
                    list,
                    header.x + 15.0,
                    header.y + (header.height - 12.0) * 0.5,
                    &format!(
                        "P{} {}",
                        track_index / TRACK_COUNT + 1,
                        track.kind.short_label()
                    ),
                    0.76,
                    color,
                );
                draw_text(
                    list,
                    header.x + 72.0,
                    header.y + (header.height - 12.0) * 0.5,
                    &track.notes.len().to_string(),
                    0.70,
                    MUTED,
                );
            } else {
                draw_text(
                    list,
                    header.x + 15.0,
                    header.y + 15.0,
                    &format!(
                        "P{} {}",
                        track_index / TRACK_COUNT + 1,
                        track.kind.short_label()
                    ),
                    1.05,
                    color,
                );
                draw_text(
                    list,
                    header.x + 15.0,
                    header.y + 42.0,
                    &track.name,
                    0.98,
                    TEXT,
                );
                draw_text(
                    list,
                    header.x + 15.0,
                    header.bottom() - 24.0,
                    &format!("{} MIDI BARS", track.notes.len()),
                    0.76,
                    MUTED,
                );
            }
        }

        for spec in self.track_buttons() {
            let active = match spec.action {
                Action::ToggleMute(index) => self.project.tracks[index].mute,
                Action::ToggleSolo(index) => self.project.tracks[index].solo,
                _ => false,
            };
            self.draw_button(list, &spec, active);
        }

        list.push_clip(viewport);
        for track_index in first_track..first_track + track_count {
            let lane = self.lane_rect(track_index);
            list.fill_rect(
                lane,
                if track_index % TRACK_COUNT % 2 == 0 {
                    Color::rgb8(18, 22, 27)
                } else {
                    Color::rgb8(21, 26, 31)
                },
            );
            list.fill_rect(
                Rect::new(lane.x, lane.bottom() - 1.0, lane.width, 1.0),
                BORDER,
            );
        }

        let first = (self.scroll_x() / self.half_step_w).floor().max(0.0) as u32;
        let last = ((self.scroll_x() + viewport.width) / self.half_step_w)
            .ceil()
            .min(self.project.total_steps() as f32) as u32;
        for step in first..=last {
            let x = self.step_x(step);
            let color = if step % STEPS_PER_MEASURE == 0 {
                GRID_MEASURE
            } else if step % 4 == 0 {
                GRID_BEAT
            } else {
                GRID_HALF
            };
            let width = if step % STEPS_PER_MEASURE == 0 {
                2.0
            } else {
                1.0
            };
            list.fill_rect(Rect::new(x, viewport.y, width, viewport.height), color);
        }

        let cursor_x = self.step_x(self.cursor_step);
        list.fill_rect(
            Rect::new(cursor_x, viewport.y, 1.0, viewport.height),
            ACCENT.with_alpha(0.72),
        );

        for track_index in first_track..first_track + track_count {
            let track = &self.project.tracks[track_index];
            for note in &track.notes {
                let rect = self.note_rect(track_index, note);
                if !rect.intersects(viewport) {
                    continue;
                }
                let selected =
                    self.selected_note == Some(note.id) || self.selected_notes.contains(&note.id);
                let color = track_color(track_index);
                list.fill_vertical_gradient_rect(
                    rect,
                    color.with_alpha(if selected { 1.0 } else { 0.92 }),
                    color.with_alpha(0.70),
                );
                list.stroke_rect(
                    rect,
                    if selected { 2.0 } else { 1.0 },
                    if selected { TEXT } else { color },
                );
                list.fill_rect(
                    Rect::new(
                        rect.right() - NOTE_HANDLE_W,
                        rect.y,
                        NOTE_HANDLE_W,
                        rect.height,
                    ),
                    if selected {
                        TEXT.with_alpha(0.90)
                    } else {
                        BG.with_alpha(0.72)
                    },
                );
                if rect.width > 42.0 {
                    let label = if track_index % TRACK_COUNT == 3 {
                        format!("NOISE {:02}", note.pitch)
                    } else {
                        note_label(note.pitch)
                    };
                    draw_text(list, rect.x + 5.0, rect.y + 5.0, &label, 0.70, BG);
                }
            }
        }

        if let Some(step) = self.playhead_step() {
            let x = viewport.x + step * self.half_step_w - self.scroll_x();
            list.fill_rect(Rect::new(x, viewport.y, 2.0, viewport.height), PLAYHEAD);
        }
        list.pop_clip();

        self.timeline_scroll.draw_scrollbars(
            list,
            self.scroll_area_rect(),
            self.timeline_content_size(),
            ScrollbarStyle {
                thickness: SCROLLBAR_H,
                ..ScrollbarStyle::default()
            },
        );
    }

    fn draw_inspector(&self, list: &mut DrawList) {
        let rect = self.inspector_rect();
        list.fill_vertical_gradient_rect(rect, Color::rgb8(30, 36, 43), PANEL);
        list.stroke_rect(rect, 1.0, BORDER);
        draw_text(
            list,
            rect.x + 14.0,
            rect.y + 16.0,
            "MIDI BAR DETAIL",
            1.05,
            TEXT,
        );

        let Some(id) = self.selected_note else {
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 58.0,
                "SELECT A MIDI BAR",
                0.95,
                MUTED,
            );
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 86.0,
                "Double-click empty track space",
                0.82,
                MUTED,
            );
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 108.0,
                "to create a bar.",
                0.82,
                MUTED,
            );
            return;
        };
        let Some((track_index, note_index)) = self.project.note_location(id) else {
            return;
        };
        let note = &self.project.tracks[track_index].notes[note_index];
        let track = &self.project.tracks[track_index];
        draw_text(
            list,
            rect.x + 14.0,
            rect.y + 40.0,
            &format!("{} / {}", track.kind.short_label(), note_label(note.pitch)),
            0.9,
            track_color(track_index),
        );
        if !self.detail_open {
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 78.0,
                "DOUBLE-CLICK THE BAR",
                0.98,
                ACCENT,
            );
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 104.0,
                "to edit timing and sound effects.",
                0.82,
                MUTED,
            );
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 142.0,
                &format!("START   {}", position_label(note.start)),
                0.86,
                TEXT,
            );
            draw_text(
                list,
                rect.x + 14.0,
                rect.y + 168.0,
                &format!("LENGTH  {:.2} beats", note.length as f32 * 0.25),
                0.86,
                TEXT,
            );
            return;
        }

        for spec in self.inspector_buttons() {
            self.draw_button(list, &spec, false);
        }
        let fields = self.inspector_fields();
        for (row, field) in fields.iter().enumerate() {
            let y = rect.y + 112.0 + row as f32 * 39.0;
            draw_text(
                list,
                rect.x + 14.0,
                y + 8.0,
                field_label(*field),
                0.83,
                MUTED,
            );
            let value = field_value(&self.project, *field, note);
            draw_text(list, rect.x + 173.0, y + 8.0, &value, 0.88, TEXT);
        }
        let toggles_y = rect.y + 112.0 + fields.len() as f32 * 39.0 + 38.0;
        match track.kind {
            TrackKind::Pulse1 | TrackKind::Pulse2 => {
                draw_text(
                    list,
                    rect.x + 14.0,
                    toggles_y,
                    &format!(
                        "ENV {}",
                        if note.effects.envelope_increase {
                            "INCREASE"
                        } else {
                            "DECREASE"
                        }
                    ),
                    0.78,
                    TEXT,
                );
                if track.kind == TrackKind::Pulse1 {
                    draw_text(
                        list,
                        rect.x + 148.0,
                        toggles_y,
                        &format!(
                            "SWEEP {}",
                            if note.effects.sweep_negate {
                                "DOWN"
                            } else {
                                "UP"
                            }
                        ),
                        0.78,
                        TEXT,
                    );
                }
            }
            TrackKind::Noise => draw_text(
                list,
                rect.x + 14.0,
                toggles_y,
                &format!(
                    "WIDTH {}",
                    if note.effects.noise_width7 {
                        "7-BIT"
                    } else {
                        "15-BIT"
                    }
                ),
                0.78,
                TEXT,
            ),
            TrackKind::Wave => {}
        }
        if let (Some(wave_index), Some(wave_rect)) =
            (self.active_user_wave_index(), self.wave_editor_rect())
        {
            let wave = &self.project.user_waves[wave_index];
            draw_text(
                list,
                wave_rect.x,
                wave_rect.y - 22.0,
                &format!("{} / DRAW 32x4-BIT", wave.name),
                0.76,
                MUTED,
            );
            list.fill_rect(wave_rect, Color::rgb8(14, 18, 22));
            list.stroke_rect(wave_rect, 1.0, BORDER);
            let cell_w = wave_rect.width / WAVE_SAMPLE_COUNT as f32;
            for (index, sample) in wave.samples.iter().enumerate() {
                let height = ((*sample as f32 + 1.0) / 16.0) * wave_rect.height;
                list.fill_rect(
                    Rect::new(
                        wave_rect.x + index as f32 * cell_w,
                        wave_rect.bottom() - height,
                        (cell_w - 1.0).max(1.0),
                        height,
                    ),
                    track_color(2),
                );
            }
        }
    }

    fn draw_status(&self, list: &mut DrawList) {
        let rect = self.status_rect();
        list.fill_rect(rect, Color::rgb8(17, 21, 25));
        list.fill_rect(Rect::new(rect.x, rect.y, rect.width, 1.0), BORDER);
        draw_text(
            list,
            rect.x + 12.0,
            rect.y + 10.0,
            &self.status,
            0.82,
            MUTED,
        );
        draw_text(
            list,
            rect.right() - 316.0,
            rect.y + 10.0,
            "CTRL+S SAVE  SPACE PLAY  DEL DELETE  +/- ZOOM",
            0.72,
            MUTED,
        );
    }
}

impl PlitaProgram for GbseqApp {
    fn resize(&mut self, size: Size) -> UiResponse {
        self.size = size.clamped();
        self.clamp_scroll();
        UiResponse::redraw()
    }

    fn handle_event(&mut self, event: UiEvent) -> UiResponse {
        match event {
            UiEvent::PointerMove { position } => {
                if self.wave_dragging {
                    self.edit_wave_at(position);
                    return UiResponse::redraw();
                }
                if self.timeline_scroll.drag().is_some() {
                    self.timeline_scroll.drag_scrollbar(
                        self.scroll_area_rect(),
                        self.timeline_content_size(),
                        position,
                        ScrollbarStyle {
                            thickness: SCROLLBAR_H,
                            ..ScrollbarStyle::default()
                        },
                    );
                    return UiResponse::redraw();
                }
                if self.drag.is_some() {
                    self.handle_note_drag(position);
                    return UiResponse::redraw();
                }
                if self.buttons.set_hovered(self.action_at(position)) {
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::PointerDown {
                position,
                button: PointerButton::Primary,
            } => {
                if self.menu.is_some() {
                    if let Some(action) = self.action_at(position) {
                        self.buttons.press(action);
                    } else {
                        self.menu = None;
                        self.buttons.clear();
                    }
                    return UiResponse::redraw();
                }
                if self
                    .wave_editor_rect()
                    .is_some_and(|rect| rect.contains(position))
                {
                    self.wave_drag_before = Some(self.project.clone());
                    self.wave_dragging = true;
                    self.edit_wave_at(position);
                    self.status = "DRAWING USER WAVE".to_string();
                    return UiResponse::redraw();
                }
                if self
                    .timeline_scroll
                    .begin_scrollbar_drag(
                        self.scroll_area_rect(),
                        self.timeline_content_size(),
                        position,
                        ScrollbarStyle {
                            thickness: SCROLLBAR_H,
                            ..ScrollbarStyle::default()
                        },
                    )
                    .is_some()
                {
                    return UiResponse::redraw();
                }
                if self.measures_input_rect().contains(position) {
                    self.begin_measures_input();
                    return UiResponse::redraw();
                }
                if self.measures_input.is_some() {
                    self.finish_measures_input(true);
                }
                if let Some(action) = self.action_at(position) {
                    self.buttons.press(action);
                    return UiResponse::redraw();
                }
                if let Some(hit) = self.note_hit(position) {
                    if self.selection_modifier {
                        if self.selected_notes.is_empty() {
                            self.selected_notes.extend(self.selected_note);
                        }
                        if !self.selected_notes.insert(hit.id) {
                            self.selected_notes.remove(&hit.id);
                        }
                        self.selected_note = self.selected_notes.first().copied();
                        self.selected_track = hit.track;
                        self.status = format!(
                            "{} notes selected / Ctrl or Shift click to toggle",
                            self.selected_notes.len()
                        );
                        return UiResponse::redraw();
                    }
                    self.selected_notes.clear();
                    let same_note = self.selected_note == Some(hit.id);
                    self.selected_note = Some(hit.id);
                    self.selected_track = hit.track;
                    self.detail_open &= same_note;
                    let note = self.project.note(hit.id).cloned();
                    if let Some(note) = note {
                        self.drag_before = Some(self.project.clone());
                        self.drag = Some(match hit.part {
                            NoteHitPart::Body => DragKind::Move {
                                id: hit.id,
                                origin: position,
                                start: note.start,
                                pitch: note.pitch,
                            },
                            NoteHitPart::Resize => DragKind::Resize {
                                id: hit.id,
                                origin_x: position.x,
                                length: note.length,
                            },
                        });
                        self.cursor_step = note.start;
                    }
                    self.status = format!("SELECTED MIDI bar {}", hit.id);
                    return UiResponse::redraw();
                }
                if position.x < TRACK_HEADER_W {
                    if let Some(track) = (self.visible_track_start()
                        ..self.visible_track_start() + self.visible_track_count())
                        .find(|track| self.header_lane_rect(*track).contains(position))
                    {
                        self.selected_track = track;
                        self.selected_note = None;
                        self.selected_notes.clear();
                        self.detail_open = false;
                        self.status = "LANE selected / Ctrl+A selects its notes".to_string();
                        return UiResponse::redraw();
                    }
                }
                if let Some(track) = self.point_track(position) {
                    self.selected_notes.clear();
                    self.selected_track = track;
                    self.cursor_step = self.point_step(position);
                    self.selected_note = None;
                    self.detail_open = false;
                    self.status = format!(
                        "CURSOR {} {}",
                        self.project.tracks[track].kind.short_label(),
                        position_label(self.cursor_step)
                    );
                    return UiResponse::redraw();
                }
                UiResponse::none()
            }
            UiEvent::PointerDown {
                position,
                button: PointerButton::Secondary,
            } => {
                if self.menu.take().is_some() {
                    self.buttons.clear();
                    return UiResponse::redraw();
                }
                if let Some(hit) = self.note_hit(position) {
                    self.selected_notes.clear();
                    self.selected_note = Some(hit.id);
                    self.selected_track = hit.track;
                    self.status = "RIGHT CLICK: DEL removes selected bar".to_string();
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::PointerUp { position, .. } => {
                if self.wave_dragging {
                    self.wave_dragging = false;
                    if let Some(before) = self.wave_drag_before.take() {
                        self.push_undo(before);
                    }
                    self.status = "USER WAVE updated".to_string();
                    return UiResponse::redraw();
                }
                if self.timeline_scroll.end_scrollbar_drag() {
                    return UiResponse::redraw();
                }
                if self.drag.take().is_some() {
                    if let Some(before) = self.drag_before.take() {
                        self.push_undo(before);
                    }
                    self.status = "MIDI bar edit committed".to_string();
                    return UiResponse::redraw();
                }
                let release = self.buttons.release_over(self.action_at(position));
                if let Some(action) = release.activated {
                    self.apply_action(action);
                    return UiResponse::redraw();
                }
                if release.changed {
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
            } => {
                if self.menu.is_some() {
                    return UiResponse::redraw();
                }
                self.selected_notes.clear();
                if let Some(hit) = self.note_hit(position) {
                    self.selected_note = Some(hit.id);
                    self.selected_track = hit.track;
                    self.detail_open = true;
                    self.status = format!("DETAIL opened for MIDI bar {}", hit.id);
                    return UiResponse::redraw();
                }
                if let Some(track) = self.point_track(position) {
                    let start = self.point_step(position);
                    let pitch = self.point_pitch(track, position);
                    let before = self.project.clone();
                    match self.project.add_note(track, start, pitch) {
                        Ok(id) => {
                            self.selected_note = Some(id);
                            self.selected_track = track;
                            self.cursor_step = start;
                            self.detail_open = true;
                            self.push_undo(before);
                            self.status = format!(
                                "ADDED {} at {} - DETAIL opened",
                                note_label(pitch),
                                position_label(start)
                            );
                        }
                        Err(error) => self.status = format!("ADD failed: {error}"),
                    }
                    return UiResponse::redraw();
                }
                UiResponse::none()
            }
            UiEvent::PointerWheel {
                position,
                delta_x,
                delta_y,
            } => {
                if self.menu.is_some() {
                    return UiResponse::none();
                }
                if self.timeline_viewport().contains(position)
                    || self.scrollbar_rect().contains(position)
                    || self.ruler_rect().contains(position)
                {
                    let changed = self.timeline_scroll.scroll_by(
                        Point::new(-delta_x - delta_y, 0.0),
                        self.scroll_area_rect(),
                        self.timeline_content_size(),
                    );
                    if changed {
                        UiResponse::redraw()
                    } else {
                        UiResponse::none()
                    }
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::KeyDown {
                key, ctrl, shift, ..
            } => {
                self.selection_modifier = ctrl || shift;
                if matches!(normalized_key(&key).as_str(), "SPACE" | " ") {
                    if self.space_down {
                        return UiResponse::none();
                    }
                    self.space_down = true;
                }
                if self.measures_input.is_some() {
                    self.handle_measures_input_key(&key)
                } else {
                    self.handle_key(&key, ctrl, shift)
                }
            }
            UiEvent::TextInput { text } | UiEvent::ImeCommit { text } => {
                if self.pending_measure_text.take().as_deref() == Some(text.as_str()) {
                    return UiResponse::redraw();
                }
                if self.append_measures_input(&text) {
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::FileDropped { path } => {
                if !self.confirm_replace() {
                    return UiResponse::redraw();
                }
                if let Err(error) = self.load_path(&path) {
                    self.status = format!("DROP failed: {error}");
                }
                UiResponse::redraw()
            }
            UiEvent::FocusChanged { focused: false } => {
                self.selection_modifier = false;
                self.space_down = false;
                self.menu = None;
                self.finish_measures_input(true);
                self.drag = None;
                if let Some(before) = self.drag_before.take() {
                    self.push_undo(before);
                }
                self.wave_dragging = false;
                if let Some(before) = self.wave_drag_before.take() {
                    self.push_undo(before);
                }
                self.buttons.clear();
                self.timeline_scroll.end_scrollbar_drag();
                UiResponse::redraw()
            }
            UiEvent::KeyUp {
                key, ctrl, shift, ..
            } => {
                if matches!(normalized_key(&key).as_str(), "SPACE" | " ") {
                    self.space_down = false;
                }
                self.pending_measure_text = None;
                self.selection_modifier = ctrl || shift;
                UiResponse::none()
            }
            _ => UiResponse::none(),
        }
    }

    fn tick(&mut self) -> UiResponse {
        let redraw = self.poll_audio_render();
        let export_redraw = self.poll_export();
        self.follow_playhead();
        if redraw || export_redraw {
            UiResponse::redraw()
        } else {
            UiResponse::none()
        }
    }

    fn draw_list(&self) -> DrawList {
        let mut list = DrawList::new(BG);
        list.fill_rect(self.root_rect(), BG);
        self.draw_header(&mut list);
        self.draw_toolbar(&mut list);
        self.draw_ruler(&mut list);
        self.draw_tracks(&mut list);
        self.draw_inspector(&mut list);
        self.draw_status(&mut list);
        for spec in self.menu_buttons() {
            self.draw_button(&mut list, &spec, false);
        }
        if self.menu.is_none() {
            if let Some(action) = self.buttons.hovered() {
                let hint = match action {
                    Action::Play => "Play from start [Space] - repeat playback uses cached audio",
                    Action::Stop => "Stop [Space]",
                    Action::Save => "Save [Ctrl+S] / Save as [Ctrl+Shift+S]",
                    Action::Undo => "Undo [Ctrl+Z]",
                    Action::Redo => "Redo [Ctrl+Y]",
                    Action::Duplicate => "Duplicate selection [Ctrl+D]",
                    Action::Delete => "Delete selection [Del]",
                    Action::Snap => "Cycle grid [G]: 1/16, 1/8, 1/4 notes",
                    Action::Quantize => "Align selected starts to grid [Q]; rejects overlaps",
                    Action::FitSong => "Fit entire song [F]",
                    Action::Preview => "Audition selected note",
                    _ => "Ctrl+A selects the current lane; Ctrl/Shift-click toggles notes",
                };
                let rect = Rect::new(12.0, self.size.height - STATUS_H - 34.0, 700.0, 28.0);
                list.fill_rect(rect, PANEL_RAISED);
                draw_text(&mut list, rect.x + 8.0, rect.y + 8.0, hint, 0.8, TEXT);
            }
        }
        list
    }
}

pub fn run_gui() -> plita::prelude::Result<()> {
    run_gui_with_path(None)
}

/// Explicit CI mode: exercise the real editor in the native window loop.
/// No audio device or user document is opened, and no settings are persisted.
pub fn run_startup_smoke(receipt: PathBuf) -> plita::prelude::Result<()> {
    struct Probe {
        app: GbseqApp,
        receipt: PathBuf,
        ticks: u32,
        draws: std::cell::Cell<u32>,
    }
    impl PlitaProgram for Probe {
        fn resize(&mut self, size: Size) -> UiResponse {
            self.app.resize(size)
        }
        fn handle_event(&mut self, event: UiEvent) -> UiResponse {
            self.app.handle_event(event)
        }
        fn draw_list(&self) -> DrawList {
            self.draws.set(self.draws.get() + 1);
            self.app.draw_list()
        }
        fn tick(&mut self) -> UiResponse {
            self.ticks += 1;
            if self.ticks >= 16 && self.draws.get() >= 2 {
                let report = serde_json::json!({
                    "passed": true, "ticks": self.ticks, "draw_lists": self.draws.get(),
                    "tracks": self.app.project.tracks.len(), "ui_toolkit": "plita",
                    "font_source": plita::active_font_source(),
                    "audio_device_tested": false, "physical_input_tested": false
                });
                if let Some(parent) = self.receipt.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(&self.receipt, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
                std::process::exit(0);
            }
            self.app.tick()
        }
    }
    App::new()
        .title("GBSEQ - plita startup verification")
        .window_size(DEFAULT_WINDOW_W, DEFAULT_WINDOW_H)
        .continuous_redraw(true)
        .run_program(move |size| Probe {
            app: GbseqApp::new_with_audio(size, false),
            receipt: receipt.clone(),
            ticks: 0,
            draws: std::cell::Cell::new(0),
        })
}

impl Drop for GbseqApp {
    fn drop(&mut self) {
        if let Some(cancelled) = self.render_cancel.take() {
            cancelled.store(true, Ordering::Relaxed);
        }
    }
}

pub fn run_gui_with_path(path: Option<PathBuf>) -> plita::prelude::Result<()> {
    App::new()
        .title("GBSEQ - plita Game Boy Sequencer")
        .window_size(DEFAULT_WINDOW_W, DEFAULT_WINDOW_H)
        .clear_color(ClearColor::rgb(0.05, 0.06, 0.075))
        .continuous_redraw(true)
        .run_program(move |size| {
            let mut app = GbseqApp::new(size);
            if let Some(path) = &path {
                if let Err(error) = app.load_path(path) {
                    app.status = format!("OPEN failed: {error}");
                }
            }
            app
        })
}

fn track_color(index: usize) -> Color {
    match index % TRACK_COUNT {
        0 => Color::rgb8(65, 191, 225),
        1 => Color::rgb8(255, 184, 76),
        2 => Color::rgb8(119, 207, 132),
        _ => Color::rgb8(238, 112, 105),
    }
}

fn field_label(field: AdjustField) -> &'static str {
    match field {
        AdjustField::Start => "START",
        AdjustField::Pitch => "PITCH",
        AdjustField::Length => "LENGTH",
        AdjustField::Velocity => "LEVEL",
        AdjustField::Pan => "PAN",
        AdjustField::Duty => "DUTY",
        AdjustField::EnvelopePeriod => "ENV PERIOD",
        AdjustField::SweepPeriod => "SWEEP TIME",
        AdjustField::SweepShift => "SWEEP SHIFT",
        AdjustField::WaveLevel => "WAVE LEVEL",
        AdjustField::WavePreset => "LINK16 WAVE",
        AdjustField::WaveSource => "WAVE SOURCE",
        AdjustField::WaveSlotLevel => "SLOT LEVEL",
        AdjustField::NoiseShift => "NOISE SHIFT",
        AdjustField::NoiseDivisor => "NOISE DIV",
    }
}

fn field_value(project: &SequenceProject, field: AdjustField, note: &MidiNote) -> String {
    match field {
        AdjustField::Start => position_label(note.start),
        AdjustField::Pitch => note_label(note.pitch),
        AdjustField::Length => format!("{:.2} beat", note.length as f32 * 0.25),
        AdjustField::Velocity => note.velocity.to_string(),
        AdjustField::Pan => note.effects.pan.label().to_string(),
        AdjustField::Duty => match note.effects.duty {
            0 => "12.5%".to_string(),
            1 => "25%".to_string(),
            2 => "50%".to_string(),
            _ => "75%".to_string(),
        },
        AdjustField::EnvelopePeriod => note.effects.envelope_period.to_string(),
        AdjustField::SweepPeriod => note.effects.sweep_period.to_string(),
        AdjustField::SweepShift => note.effects.sweep_shift.to_string(),
        AdjustField::WaveLevel => match note.effects.wave_level {
            1 => "100%".to_string(),
            2 => "50%".to_string(),
            _ => "25%".to_string(),
        },
        AdjustField::WavePreset => format!("PRESET {}", note.effects.wave_preset),
        AdjustField::WaveSource => {
            let slot = &project.wave_slots[note.effects.wave_preset.min(15) as usize];
            if slot.source >= 0x80 {
                let index = usize::from(slot.source & 0x3F);
                project
                    .user_waves
                    .get(index)
                    .map(|wave| wave.name.clone())
                    .unwrap_or_else(|| "MISSING USER WAVE".to_string())
            } else {
                [
                    "TRIANGLE", "SAW", "VOWEL A", "VOWEL I", "VOWEL U", "VOWEL E", "VOWEL O",
                ][slot.source.min(6) as usize]
                    .to_string()
            }
        }
        AdjustField::WaveSlotLevel => {
            let level = project.wave_slots[note.effects.wave_preset.min(15) as usize].level;
            match level {
                1 => "100%",
                2 => "50%",
                _ => "25%",
            }
            .to_string()
        }
        AdjustField::NoiseShift => note.effects.noise_shift.to_string(),
        AdjustField::NoiseDivisor => note.effects.noise_divisor.to_string(),
    }
}

fn adjust_effect_u8(value: &mut u8, delta: i8, min: u8, max: u8) -> bool {
    let next = add_signed_u8(*value, delta, min, max);
    let changed = *value != next;
    *value = next;
    changed
}

fn add_signed_u8(value: u8, delta: i8, min: u8, max: u8) -> u8 {
    (i16::from(value) + i16::from(delta)).clamp(i16::from(min), i16::from(max)) as u8
}

fn add_signed_u32(value: u32, delta: i32) -> u32 {
    if delta >= 0 {
        value.saturating_add(delta as u32)
    } else {
        value.saturating_sub(delta.unsigned_abs())
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

fn project_frame_count(project: &SequenceProject) -> u32 {
    // Match the actual tracker timing passed to KOKURA. Using ideal BPM here
    // truncated BPM 125 projects before their last measures because the
    // tracker uses an integer frame count per tick.
    let base = project
        .measures
        .saturating_mul(STEPS_PER_MEASURE)
        .saturating_mul(project.frames_per_step());
    if project.loop_playback {
        base.max(1)
    } else {
        base.max(1).saturating_add(60)
    }
}

pub fn load_sequence(path: &Path) -> Result<SequenceProject> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut project: SequenceProject =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    project.repair();
    project.validate()?;
    Ok(project)
}

pub fn save_sequence(project: &SequenceProject, path: &Path) -> Result<()> {
    project.validate()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(project)?)
        .with_context(|| format!("write {}", path.display()))
}

pub fn export_sequence(project: &SequenceProject, out_dir: &Path) -> Result<usize> {
    let legacy = project.to_gbdaw_project()?;
    let manifest = export_project_to_dir(
        &legacy,
        out_dir,
        StreamMode::Extended,
        ExportPolicy::default(),
    )?;
    Ok(manifest.songs.first().map(|song| song.bytes).unwrap_or(0))
}

pub fn render_sequence_wav(project: &SequenceProject, out_path: &Path) -> Result<()> {
    let legacy = project.to_gbdaw_project()?;
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    render_project_to_wav_file(
        &legacy,
        out_path,
        StreamMode::Extended,
        project_frame_count(project),
    )?;
    Ok(())
}

pub fn inspect_sequence(project: &SequenceProject) -> Result<String> {
    project.validate()?;
    let note_count: usize = project.tracks.iter().map(|track| track.notes.len()).sum();
    let legacy = project.to_gbdaw_project()?;
    Ok(format!(
        "title={} mode={} loop={} bpm={} frame_step={} measures={} steps={} tracks={} notes={} gbdaw_patterns={} ticks_per_row={} frames_per_tick={}",
        project.title,
        project.link16_mode.label(),
        project.loop_playback,
        project.bpm,
        project.frames_per_step(),
        project.measures,
        project.total_steps(),
        project.link16_mode.track_count(),
        note_count,
        legacy.song.patterns.len(),
        legacy.song.ticks_per_row,
        legacy.song.frames_per_tick
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn selection_transpose_is_one_undo_step() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        let before = app.project.clone();
        app.apply_action(Action::SelectAll);
        assert_eq!(app.selection_ids().len(), 8);
        app.handle_key("Named(ArrowUp)|Code(ArrowUp)", false, false);
        assert_eq!(
            app.project.tracks[0].notes[0].pitch,
            before.tracks[0].notes[0].pitch + 1
        );
        assert_eq!(app.undo.len(), 1);
        app.undo();
        assert_eq!(app.project, before);
    }

    #[test]
    fn fit_and_snap_support_a_hundred_bars() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.project.set_measures(100);
        app.apply_action(Action::FitSong);
        assert!((app.timeline_content_size().width - app.timeline_viewport().width).abs() < 1.0);
        app.half_step_w = 34.0;
        app.apply_action(Action::Snap);
        assert_eq!(app.snap_steps, 2);
        let point = Point::new(app.step_x(3), app.timeline_viewport().y + 20.0);
        assert_eq!(app.point_step(point), 4);
    }

    #[test]
    fn plita_named_keys_and_text_input_are_not_double_counted() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.begin_measures_input();
        for ch in ["1", "0", "0"] {
            app.handle_measures_input_key(ch);
            app.handle_event(UiEvent::TextInput {
                text: ch.to_string(),
            });
        }
        assert_eq!(app.measures_input.as_deref(), Some("100"));
        app.handle_measures_input_key("Named(Enter)|Code(Enter)");
        assert_eq!(app.project.measures, 100);
        assert!(app.measures_input.is_none());
        assert_eq!(normalized_key("Named(Space)|Code(Space)"), "SPACE");
        assert_eq!(normalized_key("Named(Escape)|Code(Escape)"), "ESCAPE");
    }

    #[test]
    fn held_space_does_not_repeatedly_restart_rendering() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        let space = || UiEvent::KeyDown {
            key: "Named(Space)|Code(Space)".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        };
        app.handle_event(space());
        assert!(app.rendering);
        app.handle_event(space());
        assert!(app.rendering);
        app.handle_event(UiEvent::KeyUp {
            key: "Named(Space)|Code(Space)".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        app.handle_event(space());
        assert!(!app.rendering);
    }

    #[test]
    fn menus_have_real_icons_and_disabled_actions_do_not_hit() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        for menu in [
            MenuKind::File,
            MenuKind::Edit,
            MenuKind::View,
            MenuKind::Export,
        ] {
            app.menu = Some(menu);
            for button in app.menu_buttons() {
                let name = action_icon(button.action).expect("menu icon");
                assert!(lucide_icon_names().any(|n| n == name));
                if !app.action_enabled(button.action) {
                    assert!(app
                        .action_at(Point::new(button.rect.x + 5.0, button.rect.y + 5.0))
                        .is_none());
                }
            }
        }
        app.menu = None;
        let a = app.toolbar_buttons();
        let bar = app.measures_input_rect();
        assert!(a.iter().all(|b| !b.rect.intersects(bar)));
    }

    #[test]
    fn shared_audio_replay_does_not_copy_the_pcm() {
        let samples: Arc<[i16]> = vec![1, 2, 3, 4].into();
        let source = SharedAudio {
            samples: Arc::clone(&samples),
            position: 0,
            rate: 48000,
            looped: false,
        };
        let replay = source.clone();
        assert!(Arc::ptr_eq(&source.samples, &replay.samples));
        assert_eq!(source.collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn shared_audio_matches_rodio_buffer_after_looping_and_resampling() {
        use rodio::{buffer::SamplesBuffer, source::UniformSourceIterator};
        for length in [4, 100, 32768, 32770, 66000] {
            let samples: Vec<i16> = (0..length)
                .map(|i| ((i * 1777) % 60000) as i32 as i16)
                .collect();
            for rate in [48000, 65536] {
                for looped in [false, true] {
                    let source = SharedAudio {
                        samples: samples.clone().into(),
                        position: 0,
                        rate,
                        looped,
                    };
                    let original = SamplesBuffer::new(2, rate, samples.clone());
                    let limit = length * 2 + 4096;
                    let expected: Vec<f32> = if looped {
                        UniformSourceIterator::new(original.repeat_infinite(), 2, 44100)
                            .take(limit)
                            .collect()
                    } else {
                        UniformSourceIterator::new(original, 2, 44100)
                            .take(limit)
                            .collect()
                    };
                    let actual: Vec<f32> = UniformSourceIterator::new(source, 2, 44100)
                        .take(limit)
                        .collect();
                    assert_eq!(
                        actual, expected,
                        "length={length}, rate={rate}, loop={looped}"
                    );
                }
            }
        }
        let mut empty = SharedAudio {
            samples: Vec::<i16>::new().into(),
            position: 0,
            rate: 48000,
            looped: true,
        };
        assert_eq!(empty.next(), None);
    }

    #[test]
    fn plita_double_click_opens_selected_bar_detail() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        let first = app.project.tracks[0].notes[0].clone();
        let rect = app.note_rect(0, &first);
        app.handle_event(UiEvent::PointerDoubleClick {
            position: Point::new(rect.x + 4.0, rect.y + 4.0),
            button: PointerButton::Primary,
        });
        assert_eq!(app.selected_note, Some(first.id));
        assert!(app.detail_open);
    }

    #[test]
    fn double_clicking_empty_lane_creates_a_bar() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.project.tracks[0].notes.clear();
        let before = app.project.tracks[0].notes.len();
        let lane = app.lane_rect(0);
        let position = Point::new(app.step_x(10), lane.y + lane.height * 0.5);
        app.handle_event(UiEvent::PointerDoubleClick {
            position,
            button: PointerButton::Primary,
        });
        assert_eq!(app.project.tracks[0].notes.len(), before + 1);
        assert!(app.detail_open);
    }

    #[test]
    fn right_edge_drag_resizes_in_sixteenth_note_units() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.project.tracks[0].notes.clear();
        let id = app.project.insert_note_exact(0, 10, 2, 24, 15).unwrap();
        let first = app.project.note(id).unwrap().clone();
        let rect = app.note_rect(0, &first);
        let down = Point::new(rect.right() - 2.0, rect.y + 4.0);
        app.handle_event(UiEvent::PointerDown {
            position: down,
            button: PointerButton::Primary,
        });
        app.handle_event(UiEvent::PointerMove {
            position: Point::new(down.x + app.half_step_w, down.y),
        });
        app.handle_event(UiEvent::PointerUp {
            position: Point::new(down.x + app.half_step_w, down.y),
            button: PointerButton::Primary,
        });
        assert_eq!(app.project.note(first.id).unwrap().length, first.length + 1);
    }

    #[test]
    fn inspector_effect_change_reaches_shared_project_model() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        let id = app.project.tracks[0].notes[0].id;
        app.selected_note = Some(id);
        app.detail_open = true;
        let before = app.project.note(id).unwrap().effects.duty;
        app.adjust_selected(AdjustField::Duty, 1);
        assert_eq!(
            app.project.note(id).unwrap().effects.duty,
            (before + 1).min(3)
        );
    }

    #[test]
    fn measures_field_accepts_an_arbitrary_value() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.begin_measures_input();
        for digit in ["1", "0", "0"] {
            app.handle_measures_input_key(digit);
        }
        app.handle_measures_input_key("Enter");
        assert_eq!(app.project.measures, 100);
        assert!(app.measures_input.is_none());
    }

    #[test]
    fn loop_toolbar_action_is_undoable() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        assert!(app.project.loop_playback);
        app.apply_action(Action::ToggleLoop);
        assert!(!app.project.loop_playback);
        app.apply_action(Action::Undo);
        assert!(app.project.loop_playback);
    }

    #[test]
    fn frame_count_uses_tracker_timing_and_non_loop_tail() {
        let mut project = SequenceProject::empty();
        project.measures = 64;
        project.bpm = 125;
        project.loop_playback = true;
        assert_eq!(project_frame_count(&project), 8192);
        project.loop_playback = false;
        assert_eq!(project_frame_count(&project), 8252);
    }

    #[test]
    fn exact_frame_override_drives_render_and_playback_timing() {
        let mut project = SequenceProject::empty();
        project.measures = 3;
        project.frames_per_step_override = Some(1);
        project.loop_playback = true;
        assert_eq!(project_frame_count(&project), 48);
        let timing = PlaybackTiming::from_project(&project);
        assert_eq!(timing.frames_per_step, 1);
        assert!(
            (timing.step_at(Duration::from_secs_f64(10.0 / GB_FRAME_RATE_HZ)) - 10.0).abs() < 0.001
        );
    }

    #[test]
    fn all16_mode_exposes_all_tracks_on_one_timeline() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.apply_action(Action::SetLink16Mode(Link16Mode::Dmg07_16));
        app.apply_action(Action::ToggleAllPlayers);
        assert!(app.all_players_visible());
        assert_eq!(app.visible_track_start(), 0);
        assert_eq!(app.visible_track_count(), 16);
        let bottom = app.timeline_viewport().bottom() - 1.0;
        assert_eq!(
            app.point_track(Point::new(app.timeline_viewport().x + 8.0, bottom)),
            Some(15)
        );
    }

    #[test]
    fn playback_follow_scrolls_to_the_current_sixteenth_note() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.project.measures = 64;
        app.project.bpm = 120;
        app.playback_timing = Some(PlaybackTiming::from_project(&app.project));
        app.play_started = Some(Instant::now() - Duration::from_secs(30));
        app.follow_playhead();
        assert!(app.scroll_x() > 0.0);
        let step = app.playhead_step().unwrap();
        assert!(step >= 223.0 && step <= 225.0);
    }

    #[test]
    fn playback_timing_stays_on_the_rendered_project_snapshot() {
        let mut app = GbseqApp::new_with_audio(Size::new(1480.0, 900.0), false);
        app.project.measures = 64;
        app.project.bpm = 120;
        app.playback_timing = Some(PlaybackTiming::from_project(&app.project));
        app.play_started = Some(Instant::now() - Duration::from_secs(30));

        app.project.measures = 1;
        app.project.bpm = 300;

        let step = app.playhead_step().unwrap();
        assert!(step >= 223.0 && step <= 225.0);
        assert!(step > app.project.total_steps() as f32);
    }

    #[test]
    fn all16_playback_renders_and_mixes_all_four_players() {
        let mut source = SequenceProject::empty();
        source.set_link16_mode(Link16Mode::Dmg07_16);
        source.measures = 1;
        source
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        for player in 0..4 {
            source
                .insert_note_exact(player * TRACK_COUNT, 0, 4, 18 + player as u8 * 4, 15)
                .unwrap();
        }
        let projects = (0..4)
            .map(|player| source.to_gbdaw_player(player).unwrap())
            .collect::<Vec<_>>();
        let (sample_rate, samples) = render_mixed_projects(&projects, 120).unwrap();
        assert!(sample_rate > 0);
        assert!(!samples.is_empty());
        assert!(samples.iter().any(|sample| *sample != 0));
    }
}
