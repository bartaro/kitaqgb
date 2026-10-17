use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardwareMode {
    #[default]
    Dmg,
    Cgb,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapperKind {
    RomOnly,
    Mbc1,
    Mbc2,
    Mmm01,
    Mbc3,
    Mbc5,
    Mbc6,
    Mbc7,
    PocketCamera,
    Tama5,
    HuC3,
    HuC1,
}

impl MapperKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::RomOnly => "ROM_ONLY",
            Self::Mbc1 => "MBC1",
            Self::Mbc2 => "MBC2",
            Self::Mmm01 => "MMM01",
            Self::Mbc3 => "MBC3",
            Self::Mbc5 => "MBC5",
            Self::Mbc6 => "MBC6",
            Self::Mbc7 => "MBC7",
            Self::PocketCamera => "POCKET_CAMERA",
            Self::Tama5 => "TAMA5",
            Self::HuC3 => "HuC3",
            Self::HuC1 => "HuC1",
        }
    }

    pub fn is_special(self) -> bool {
        matches!(
            self,
            Self::Mmm01
                | Self::Mbc6
                | Self::Mbc7
                | Self::PocketCamera
                | Self::Tama5
                | Self::HuC3
                | Self::HuC1
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClockState {
    pub cycles: u64,
    pub frames: u64,
    #[serde(default)]
    pub frame_phase_ppu_cycles: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PpuTraceEvent {
    ScanlineAdvance { from_ly: u8, to_ly: u8 },
    PpuModeChange { from_mode: u8, to_mode: u8, ly: u8 },
    StatSignal { coincidence: bool, ly: u8, lyc: u8 },
    VblankEnter { ly: u8 },
    FrameComplete { frame_serial: u64 },
    ScanlineRender { ly: u8 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HdmaDeferredReason {
    CpuHalted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DmaTraceEvent {
    OamDmaStart {
        source: u16,
    },
    OamDmaComplete {
        source: u16,
        bytes: u16,
    },
    HdmaStart {
        source: u16,
        dest: u16,
        blocks: u8,
        hblank_mode: bool,
    },
    GdmaStallEstimate {
        blocks: u8,
        stall_cycles: u32,
    },
    HdmaBlock {
        source: u16,
        dest: u16,
        block_index: u8,
        remaining_blocks: u8,
        hblank_mode: bool,
        stall_cycles: u32,
    },
    HdmaComplete {
        source: u16,
        dest: u16,
        blocks: u8,
        hblank_mode: bool,
    },
    HdmaCancel {
        source: u16,
        dest: u16,
        remaining_blocks: u8,
    },
    HdmaDeferred {
        remaining_blocks: u8,
        ly: u8,
        reason: HdmaDeferredReason,
    },
    HdmaWriteIgnored {
        value: u8,
        remaining_blocks: u8,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapperTraceEvent {
    ControlWrite {
        mapper: MapperKind,
        addr: u16,
        value: u8,
        rom_bank: u16,
        ram_bank: u16,
    },
    RomBankChange {
        mapper: MapperKind,
        addr: u16,
        value: u8,
        from: u16,
        to: u16,
    },
    RamBankChange {
        mapper: MapperKind,
        addr: u16,
        value: u8,
        from: u16,
        to: u16,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterruptSource {
    Vblank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimerTraceEvent {
    DivResetEdge { old_div: u16 },
    TacWrite { old_tac: u8, new_tac: u8 },
    TimaOverflow { old_tima: u8 },
    TimaReload { reloaded_tima: u8, tma: u8 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SerialTraceEvent {
    TransferStart {
        sb: u8,
        sc: u8,
        internal_clock: bool,
    },
    TransferComplete {
        sb: u8,
        sc: u8,
        internal_clock: bool,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum JoypadTraceEvent {
    Read { p1: u8, select: u8, mask: u8 },
    SelectionWrite { old_p1: u8, new_p1: u8 },
    InputEdge { old_mask: u8, new_mask: u8, p1: u8 },
    InterruptEdge { p1: u8 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApuTraceEvent {
    MasterToggle {
        enabled: bool,
        nr52: u8,
    },
    ChannelTrigger {
        channel: u8,
        reg: u16,
        value: u8,
    },
    ChannelLengthExpired {
        channel: u8,
    },
    ChannelEnvelopeStep {
        channel: u8,
        volume: u8,
        increasing: bool,
    },
    ChannelSweepStep {
        old_frequency: u16,
        new_frequency: u16,
        negate: bool,
        shift: u8,
    },
    ChannelDisabled {
        channel: u8,
        reason: u8,
        reg: u16,
    },
    DacStateChange {
        channel: u8,
        enabled: bool,
        reg: u16,
        active: bool,
    },
    PopRisk {
        source: u8,
        reg: u16,
    },
    FrameSequencerStep {
        step: u8,
    },
    MixerControlWrite {
        nr50: u8,
        nr51: u8,
        nr52: u8,
    },
    MixedOutput {
        left: i16,
        right: i16,
        active_mask: u8,
    },
    PcmFramesBuffered {
        frames: u16,
        buffered_frames: u32,
    },
    PcmBufferWrapped {
        dropped_frames: u16,
        dropped_total: u64,
    },
    WaveRamWrite {
        index: u8,
        value: u8,
    },
    WaveRamAccessAliased {
        requested_index: u8,
        actual_index: u8,
        is_write: bool,
    },
    Ch3TriggerRetainsSample {
        buffered_sample: u8,
        next_index: u8,
    },
    NoiseClockFrozen {
        shift: u8,
        divisor_code: u8,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CgbTraceEvent {
    ModeSelected {
        cgb_enabled: bool,
        cgb_only: bool,
    },
    VramBankSwitch {
        bank: u8,
        value: u8,
    },
    WramBankSwitch {
        bank: u8,
        value: u8,
    },
    BgPaletteIndexWrite {
        index: u8,
        auto_increment: bool,
    },
    BgPaletteDataWrite {
        index: u8,
        value: u8,
        blocked: bool,
        auto_increment: bool,
    },
    ObjPaletteIndexWrite {
        index: u8,
        auto_increment: bool,
    },
    ObjPaletteDataWrite {
        index: u8,
        value: u8,
        blocked: bool,
        auto_increment: bool,
    },
    Key1Write {
        armed: bool,
        double_speed: bool,
        value: u8,
    },
    SpeedSwitch {
        double_speed: bool,
        stop_stall_cycles: u32,
    },
    SpeedSwitchFreeze {
        cpu_cycles: u32,
        ppu_mode: u8,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IoTraceEvent {
    Timer(TimerTraceEvent),
    Serial(SerialTraceEvent),
    Joypad(JoypadTraceEvent),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterruptTraceEvent {
    Requested {
        source: InterruptSource,
        iflag: u8,
    },
    Serviced {
        source: InterruptSource,
        vector: u16,
    },
    PendingBlocked {
        pending_mask: u8,
        ime: bool,
        halted: bool,
    },
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub cycles: u32,
    pub frame_completed: bool,
    pub ppu_trace: Vec<PpuTraceEvent>,
    pub dma_trace: Vec<DmaTraceEvent>,
    pub mapper_trace: Vec<MapperTraceEvent>,
    pub io_trace: Vec<IoTraceEvent>,
    pub apu_trace: Vec<ApuTraceEvent>,
    pub cgb_trace: Vec<CgbTraceEvent>,
    pub interrupt_trace: Vec<InterruptTraceEvent>,
}
