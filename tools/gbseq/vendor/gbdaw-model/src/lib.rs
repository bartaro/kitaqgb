use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("pattern index {0} is out of range")]
    InvalidPatternIndex(usize),
    #[error("pattern {pattern} row count mismatch: expected {expected}, got {actual}")]
    PatternRowCount {
        pattern: usize,
        expected: usize,
        actual: usize,
    },
    #[error("invalid note value {0}; expected 0..47")]
    InvalidNote(u8),
    #[error("instrument '{0}' was not found")]
    MissingInstrument(String),
    #[error("wave '{0}' was not found")]
    MissingWave(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Ch1,
    Ch2,
    Ch3,
    Ch4,
}

impl Channel {
    pub const ALL: [Channel; 4] = [Channel::Ch1, Channel::Ch2, Channel::Ch3, Channel::Ch4];

    pub fn index(self) -> usize {
        match self {
            Channel::Ch1 => 0,
            Channel::Ch2 => 1,
            Channel::Ch3 => 2,
            Channel::Ch4 => 3,
        }
    }

    /// Music data and physical APIs share the zero-based hardware channel order.
    pub fn stream_id(self) -> u8 {
        self.index() as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pan {
    Center,
    Left,
    Right,
    Mute,
}

impl Pan {
    pub fn as_u8(self) -> u8 {
        match self {
            Pan::Center => 0,
            Pan::Left => 1,
            Pan::Right => 2,
            Pan::Mute => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub comment: String,
    pub license_tag: String,
    pub clean_room_note: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            title: "Untitled".to_string(),
            author: "Unknown".to_string(),
            comment: String::new(),
            license_tag: "MIT".to_string(),
            clean_room_note: "Designed from project-local specifications only.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerSettings {
    pub master_left: u8,
    pub master_right: u8,
    pub mute: [bool; 4],
    pub solo: [bool; 4],
}

impl Default for MixerSettings {
    fn default() -> Self {
        Self {
            master_left: 7,
            master_right: 7,
            mute: [false; 4],
            solo: [false; 4],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSettings {
    pub music_bank: u8,
    pub sfx_bank: u8,
    pub loop_enabled: bool,
    pub loop_strategy: LoopStrategy,
    pub force_inst_before_note: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            music_bank: 31,
            sfx_bank: 30,
            loop_enabled: false,
            loop_strategy: LoopStrategy::UnfoldOnce,
            force_inst_before_note: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopStrategy {
    StreamHeadOnly,
    UnfoldOnce,
    FixedRepeatN,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintSettings {
    pub page_size: String,
    pub orientation: String,
    pub mode: String,
    pub measures_per_system: u8,
    pub system_gap: f32,
    pub staff_gap: f32,
    pub staff_scale: f32,
    pub show_bar_numbers: bool,
    pub show_footer: bool,
    pub show_annotations: bool,
    pub show_note_labels: bool,
    pub engraving_preset: String,
    pub collision_avoidance: bool,
    pub auto_spacing_strength: f32,
    pub tie_curve_strength: f32,
    pub slur_curve_strength: f32,
    pub beam_grouping: String,
    pub stem_direction_mode: String,
    pub show_phrase_slurs: bool,
    pub show_ornaments: bool,
    pub show_dynamics: bool,
    pub ornament_y_offset: f32,
    pub layout_balance: f32,
}

impl PrintSettings {
    pub fn apply_preset(&mut self, preset: &str) {
        match preset.to_ascii_lowercase().as_str() {
            "dense" => {
                self.engraving_preset = "dense".to_string();
                self.mode = "compact_practice_score".to_string();
                self.measures_per_system = 6;
                self.system_gap = 20.0;
                self.staff_gap = 16.0;
                self.staff_scale = 0.90;
                self.show_bar_numbers = true;
                self.show_footer = true;
                self.show_annotations = false;
                self.show_note_labels = false;
                self.collision_avoidance = true;
                self.auto_spacing_strength = 0.55;
                self.tie_curve_strength = 0.85;
                self.slur_curve_strength = 0.80;
                self.beam_grouping = "beat".to_string();
                self.stem_direction_mode = "auto".to_string();
                self.show_phrase_slurs = false;
                self.show_ornaments = true;
                self.show_dynamics = false;
                self.ornament_y_offset = 1.0;
                self.layout_balance = 0.55;
            }
            "practice" => {
                self.engraving_preset = "practice".to_string();
                self.mode = "compact_practice_score".to_string();
                self.measures_per_system = 5;
                self.system_gap = 24.0;
                self.staff_gap = 18.0;
                self.staff_scale = 1.00;
                self.show_bar_numbers = true;
                self.show_footer = true;
                self.show_annotations = true;
                self.show_note_labels = true;
                self.collision_avoidance = true;
                self.auto_spacing_strength = 0.70;
                self.tie_curve_strength = 0.95;
                self.slur_curve_strength = 0.90;
                self.beam_grouping = "beat".to_string();
                self.stem_direction_mode = "auto".to_string();
                self.show_phrase_slurs = true;
                self.show_ornaments = true;
                self.show_dynamics = true;
                self.ornament_y_offset = 1.05;
                self.layout_balance = 0.72;
            }
            "lead" => {
                self.engraving_preset = "lead".to_string();
                self.mode = "full_score".to_string();
                self.measures_per_system = 4;
                self.system_gap = 28.0;
                self.staff_gap = 20.0;
                self.staff_scale = 1.05;
                self.show_bar_numbers = true;
                self.show_footer = true;
                self.show_annotations = true;
                self.show_note_labels = false;
                self.collision_avoidance = true;
                self.auto_spacing_strength = 0.82;
                self.tie_curve_strength = 1.00;
                self.slur_curve_strength = 0.95;
                self.beam_grouping = "auto".to_string();
                self.stem_direction_mode = "auto".to_string();
                self.show_phrase_slurs = true;
                self.show_ornaments = true;
                self.show_dynamics = true;
                self.ornament_y_offset = 1.08;
                self.layout_balance = 0.88;
            }
            _ => {
                self.engraving_preset = "publication".to_string();
                self.mode = "full_score".to_string();
                self.measures_per_system = 4;
                self.system_gap = 30.0;
                self.staff_gap = 22.0;
                self.staff_scale = 1.08;
                self.show_bar_numbers = true;
                self.show_footer = true;
                self.show_annotations = true;
                self.show_note_labels = false;
                self.collision_avoidance = true;
                self.auto_spacing_strength = 1.00;
                self.tie_curve_strength = 1.10;
                self.slur_curve_strength = 1.05;
                self.beam_grouping = "auto".to_string();
                self.stem_direction_mode = "auto".to_string();
                self.show_phrase_slurs = true;
                self.show_ornaments = true;
                self.show_dynamics = true;
                self.ornament_y_offset = 1.12;
                self.layout_balance = 1.0;
            }
        }
    }
}

impl Default for PrintSettings {
    fn default() -> Self {
        let mut value = Self {
            page_size: "A4".to_string(),
            orientation: "portrait".to_string(),
            mode: "full_score".to_string(),
            measures_per_system: 4,
            system_gap: 28.0,
            staff_gap: 20.0,
            staff_scale: 1.0,
            show_bar_numbers: true,
            show_footer: true,
            show_annotations: true,
            show_note_labels: false,
            engraving_preset: "publication".to_string(),
            collision_avoidance: true,
            auto_spacing_strength: 1.0,
            tie_curve_strength: 1.10,
            slur_curve_strength: 1.05,
            beam_grouping: "auto".to_string(),
            stem_direction_mode: "auto".to_string(),
            show_phrase_slurs: true,
            show_ornaments: true,
            show_dynamics: true,
            ornament_y_offset: 1.12,
            layout_balance: 1.0,
        };
        value.apply_preset("publication");
        value
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSettings {
    pub compare_lag_window: usize,
    pub emit_markdown: bool,
}

impl Default for ReportSettings {
    fn default() -> Self {
        Self {
            compare_lag_window: 2048,
            emit_markdown: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub version: String,
    pub metadata: Metadata,
    pub song: Song,
    pub instruments: Vec<Instrument>,
    pub waves: Vec<WaveDefinition>,
    pub mixer: MixerSettings,
    pub export_settings: ExportSettings,
    pub print_settings: PrintSettings,
    pub report_settings: ReportSettings,
}

impl Project {
    pub fn empty() -> Self {
        Self {
            version: "0.1.0".to_string(),
            metadata: Metadata {
                title: "Untitled".to_string(),
                author: "Unknown".to_string(),
                comment: String::new(),
                ..Metadata::default()
            },
            song: Song {
                tempo: 150,
                speed: 6,
                ticks_per_row: 2,
                frames_per_tick: 3,
                rows_per_pattern: 16,
                order_list: vec![0],
                patterns: vec![Pattern {
                    index: 0,
                    rows: vec![Row::default(); 16],
                }],
                loop_order: Some(0),
                loop_row: Some(0),
                time_signature_num: 4,
                time_signature_den: 4,
            },
            instruments: vec![
                Instrument::Pulse(PulseInstrument {
                    id: "lead".to_string(),
                    name: "Lead".to_string(),
                    channel: Channel::Ch1,
                    duty: 2,
                    envelope: 0xF1,
                    sweep: 0x00,
                    pan: Pan::Center,
                }),
                Instrument::Pulse(PulseInstrument {
                    id: "bass".to_string(),
                    name: "Bass".to_string(),
                    channel: Channel::Ch2,
                    duty: 1,
                    envelope: 0xD2,
                    sweep: 0x00,
                    pan: Pan::Left,
                }),
                Instrument::Wave(WaveInstrument {
                    id: "wave".to_string(),
                    name: "Wave".to_string(),
                    wave_ref: "tri".to_string(),
                    output_level: 0x20,
                    pan: Pan::Right,
                }),
                Instrument::Noise(NoiseInstrument {
                    id: "drum".to_string(),
                    name: "Drum".to_string(),
                    envelope: 0xF1,
                    noise_param: 0x23,
                    pan: Pan::Center,
                }),
            ],
            waves: vec![WaveDefinition {
                id: "tri".to_string(),
                name: "Triangle".to_string(),
                bytes16: [
                    0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76,
                    0x54, 0x32, 0x10,
                ],
                preview_label: "triangle".to_string(),
                tags: vec!["preset".to_string()],
            }],
            mixer: MixerSettings::default(),
            export_settings: ExportSettings::default(),
            print_settings: PrintSettings::default(),
            report_settings: ReportSettings::default(),
        }
    }

    pub fn demo() -> Self {
        let mut rows = vec![Row::default(); 16];
        rows[0].cells[0].note = Some(24);
        rows[0].cells[0].instrument_ref = Some("lead".to_string());
        rows[4].cells[0].note = Some(28);
        rows[4].cells[0].instrument_ref = Some("lead".to_string());
        rows[8].cells[0].note = Some(31);
        rows[8].cells[0].instrument_ref = Some("lead".to_string());
        rows[12].cells[0].note = Some(36);
        rows[12].cells[0].instrument_ref = Some("lead".to_string());

        rows[0].cells[1].note = Some(12);
        rows[0].cells[1].instrument_ref = Some("bass".to_string());
        rows[8].cells[1].note = Some(14);
        rows[8].cells[1].instrument_ref = Some("bass".to_string());

        rows[0].cells[2].note = Some(12);
        rows[0].cells[2].instrument_ref = Some("wave".to_string());
        rows[8].cells[2].note = Some(19);
        rows[8].cells[2].instrument_ref = Some("wave".to_string());

        rows[0].cells[3].note = Some(0x23);
        rows[0].cells[3].instrument_ref = Some("drum".to_string());
        rows[4].cells[3].note = Some(0x27);
        rows[4].cells[3].instrument_ref = Some("drum".to_string());
        rows[8].cells[3].note = Some(0x2B);
        rows[8].cells[3].instrument_ref = Some("drum".to_string());
        rows[12].cells[3].note = Some(0x27);
        rows[12].cells[3].instrument_ref = Some("drum".to_string());

        rows[4].row_automation.push(AutomationEvent {
            channel: Some(Channel::Ch1),
            target: AutomationTarget::Ch1Sweep,
            value: AutomationValue::U8(0x16),
            position_order: 0,
            position_row: 4,
            position_tick: 0,
            source_view: "tracker".to_string(),
        });
        rows[8].row_automation.push(AutomationEvent {
            channel: Some(Channel::Ch3),
            target: AutomationTarget::Ch3CustomWave,
            value: AutomationValue::Bytes16([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
                0x32, 0x10,
            ]),
            position_order: 0,
            position_row: 8,
            position_tick: 0,
            source_view: "wave_editor".to_string(),
        });
        rows[8].row_automation.push(AutomationEvent {
            channel: Some(Channel::Ch3),
            target: AutomationTarget::Ch3Level,
            value: AutomationValue::U8(0x40),
            position_order: 0,
            position_row: 8,
            position_tick: 0,
            source_view: "automation".to_string(),
        });
        rows[0].row_automation.push(AutomationEvent {
            channel: None,
            target: AutomationTarget::MasterVolume,
            value: AutomationValue::Pair(7, 7),
            position_order: 0,
            position_row: 0,
            position_tick: 0,
            source_view: "project".to_string(),
        });

        Self {
            version: "0.1.0".to_string(),
            metadata: Metadata {
                title: "GBDAW Demo".to_string(),
                author: "OpenAI".to_string(),
                comment: "Headless demo project".to_string(),
                ..Metadata::default()
            },
            song: Song {
                tempo: 150,
                speed: 6,
                ticks_per_row: 2,
                frames_per_tick: 3,
                rows_per_pattern: 16,
                order_list: vec![0],
                patterns: vec![Pattern { index: 0, rows }],
                loop_order: Some(0),
                loop_row: Some(0),
                time_signature_num: 4,
                time_signature_den: 4,
            },
            instruments: vec![
                Instrument::Pulse(PulseInstrument {
                    id: "lead".to_string(),
                    name: "Lead".to_string(),
                    channel: Channel::Ch1,
                    duty: 2,
                    envelope: 0xF1,
                    sweep: 0x00,
                    pan: Pan::Center,
                }),
                Instrument::Pulse(PulseInstrument {
                    id: "bass".to_string(),
                    name: "Bass".to_string(),
                    channel: Channel::Ch2,
                    duty: 1,
                    envelope: 0xD2,
                    sweep: 0x00,
                    pan: Pan::Left,
                }),
                Instrument::Wave(WaveInstrument {
                    id: "wave".to_string(),
                    name: "Wave".to_string(),
                    wave_ref: "tri".to_string(),
                    output_level: 0x20,
                    pan: Pan::Right,
                }),
                Instrument::Noise(NoiseInstrument {
                    id: "drum".to_string(),
                    name: "Drum".to_string(),
                    envelope: 0xF1,
                    noise_param: 0x23,
                    pan: Pan::Center,
                }),
            ],
            waves: vec![WaveDefinition {
                id: "tri".to_string(),
                name: "Triangle".to_string(),
                bytes16: [
                    0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76,
                    0x54, 0x32, 0x10,
                ],
                preview_label: "triangle".to_string(),
                tags: vec!["preset".to_string()],
            }],
            mixer: MixerSettings::default(),
            export_settings: ExportSettings::default(),
            print_settings: PrintSettings::default(),
            report_settings: ReportSettings::default(),
        }
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        for &pattern_index in &self.song.order_list {
            if pattern_index >= self.song.patterns.len() {
                return Err(ModelError::InvalidPatternIndex(pattern_index));
            }
        }
        for pattern in &self.song.patterns {
            if pattern.rows.len() != self.song.rows_per_pattern as usize {
                return Err(ModelError::PatternRowCount {
                    pattern: pattern.index,
                    expected: self.song.rows_per_pattern as usize,
                    actual: pattern.rows.len(),
                });
            }
            for row in &pattern.rows {
                for cell in &row.cells {
                    if let Some(note) = cell.note {
                        if cell_is_melodic(row, cell) && note > 47 {
                            return Err(ModelError::InvalidNote(note));
                        }
                    }
                    if let Some(instrument_ref) = &cell.instrument_ref {
                        if self.instrument_by_id(instrument_ref).is_none() {
                            return Err(ModelError::MissingInstrument(instrument_ref.clone()));
                        }
                    }
                }
                for automation in &row.row_automation {
                    if let AutomationTarget::Ch3Wave = automation.target {
                        if let AutomationValue::Text(wave) = &automation.value {
                            if self.wave_by_id(wave).is_none() {
                                return Err(ModelError::MissingWave(wave.clone()));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn instrument_by_id(&self, id: &str) -> Option<&Instrument> {
        self.instruments.iter().find(|inst| inst.id() == id)
    }

    pub fn wave_by_id(&self, id: &str) -> Option<&WaveDefinition> {
        self.waves.iter().find(|wave| wave.id == id)
    }
}

fn cell_is_melodic(_row: &Row, cell: &Cell) -> bool {
    if let Some(effect) = cell
        .effect_commands
        .iter()
        .find(|effect| effect.code == "channel")
    {
        return effect.value != 3;
    }
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub tempo: u16,
    pub speed: u8,
    pub ticks_per_row: u8,
    pub frames_per_tick: u8,
    pub rows_per_pattern: u8,
    pub order_list: Vec<usize>,
    pub patterns: Vec<Pattern>,
    pub loop_order: Option<usize>,
    pub loop_row: Option<u8>,
    pub time_signature_num: u8,
    pub time_signature_den: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub index: usize,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub cells: [Cell; 4],
    pub row_automation: Vec<AutomationEvent>,
    pub row_annotations: Vec<String>,
}

impl Default for Row {
    fn default() -> Self {
        Self {
            cells: [
                Cell::default(),
                Cell::default(),
                Cell::default(),
                Cell::default(),
            ],
            row_automation: Vec::new(),
            row_annotations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cell {
    pub note: Option<u8>,
    pub instrument_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_ticks: Option<u16>,
    pub volume: Option<u8>,
    pub effect_commands: Vec<EffectCommand>,
    pub tie_flag: bool,
    pub notation_hint: Option<String>,
    pub stop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectCommand {
    pub code: String,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Instrument {
    Pulse(PulseInstrument),
    Wave(WaveInstrument),
    Noise(NoiseInstrument),
}

impl Instrument {
    pub fn id(&self) -> &str {
        match self {
            Instrument::Pulse(v) => &v.id,
            Instrument::Wave(v) => &v.id,
            Instrument::Noise(v) => &v.id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseInstrument {
    pub id: String,
    pub name: String,
    pub channel: Channel,
    pub duty: u8,
    pub envelope: u8,
    pub sweep: u8,
    pub pan: Pan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveInstrument {
    pub id: String,
    pub name: String,
    pub wave_ref: String,
    pub output_level: u8,
    pub pan: Pan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseInstrument {
    pub id: String,
    pub name: String,
    pub envelope: u8,
    pub noise_param: u8,
    pub pan: Pan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveDefinition {
    pub id: String,
    pub name: String,
    pub bytes16: [u8; 16],
    pub preview_label: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationEvent {
    pub channel: Option<Channel>,
    pub target: AutomationTarget,
    pub value: AutomationValue,
    pub position_order: usize,
    pub position_row: u8,
    pub position_tick: u8,
    pub source_view: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationTarget {
    Ch1Duty,
    Ch2Duty,
    Ch1Sweep,
    Ch3Level,
    Ch3Wave,
    Ch3CustomWave,
    Ch4Param,
    Pan,
    MasterVolume,
    StopChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationValue {
    U8(u8),
    Pair(u8, u8),
    Text(String),
    Bytes16([u8; 16]),
}
