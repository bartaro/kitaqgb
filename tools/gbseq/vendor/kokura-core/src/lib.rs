pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod dma;
pub mod error;
pub mod interrupt;
pub mod joypad;
pub mod mbc;
pub mod memory;
pub mod ppu;
pub mod serial;
pub mod state;
pub mod timer;
pub mod types;

use apu::Apu;
use cartridge::Cartridge;
use cpu::Cpu;
use dma::DmaState;
use interrupt::{InterruptState, INT_JOYPAD, INT_LCD_STAT, INT_SERIAL, INT_TIMER, INT_VBLANK};
use joypad::Joypad;
use memory::Memory;
use ppu::Ppu;
use serial::Serial;
use state::MachineState;
use timer::Timer;
use types::{
    ApuTraceEvent, CgbTraceEvent, ClockState, DmaTraceEvent, HardwareMode, HdmaDeferredReason,
    InterruptSource, InterruptTraceEvent, IoTraceEvent, JoypadTraceEvent, MapperTraceEvent,
    StepResult,
};

const CGB_DMA_BLOCK_STALL_CYCLES: u32 = 32;
const CGB_SPEED_SWITCH_FREEZE_CYCLES: u32 = 8200;

#[derive(Debug, Clone, Default)]
pub struct SerialLinkTraceResult {
    pub completed: bool,
    pub self_io_trace: Vec<IoTraceEvent>,
    pub self_interrupt_trace: Vec<InterruptTraceEvent>,
    pub peer_io_trace: Vec<IoTraceEvent>,
    pub peer_interrupt_trace: Vec<InterruptTraceEvent>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunSliceResult {
    pub instructions: usize,
    pub cycles: u32,
    pub frame_completed: bool,
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub cpu: Cpu,
    pub ppu: Ppu,
    pub timer: Timer,
    pub interrupt: InterruptState,
    pub joypad: Joypad,
    pub dma: DmaState,
    pub serial: Serial,
    pub apu: Apu,
    pub cartridge: Cartridge,
    pub memory: Memory,
    pub clocks: ClockState,
    pub mode: HardwareMode,
    pending_dma_trace: Vec<DmaTraceEvent>,
    pending_mapper_trace: Vec<MapperTraceEvent>,
    pending_io_trace: Vec<IoTraceEvent>,
    pending_apu_trace: Vec<ApuTraceEvent>,
    pending_cgb_trace: Vec<CgbTraceEvent>,
    pending_interrupt_trace: Vec<InterruptTraceEvent>,
    pending_cpu_stall_cycles: u32,
    pub cgb_double_speed: bool,
    pub cgb_speed_switch_armed: bool,
    pub cgb_speed_switch_freeze_cycles: u32,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::default(),
            ppu: Ppu::default(),
            timer: Timer::default(),
            interrupt: InterruptState::default(),
            joypad: Joypad::default(),
            dma: DmaState::default(),
            serial: Serial::default(),
            apu: Apu::default(),
            cartridge: Cartridge::empty(),
            memory: Memory::default(),
            clocks: ClockState::default(),
            mode: HardwareMode::Dmg,
            pending_dma_trace: Vec::new(),
            pending_mapper_trace: Vec::new(),
            pending_io_trace: Vec::new(),
            pending_apu_trace: Vec::new(),
            pending_cgb_trace: Vec::new(),
            pending_interrupt_trace: Vec::new(),
            pending_cpu_stall_cycles: 0,
            cgb_double_speed: false,
            cgb_speed_switch_armed: false,
            cgb_speed_switch_freeze_cycles: 0,
        }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) -> Result<(), error::CoreError> {
        self.load_rom_with_mode(rom, None)
    }

    pub fn load_rom_with_mode(
        &mut self,
        rom: Vec<u8>,
        forced_mode: Option<HardwareMode>,
    ) -> Result<(), error::CoreError> {
        let cartridge = Cartridge::from_rom(rom)?;
        let selected_mode = match forced_mode {
            Some(HardwareMode::Dmg) if cartridge.cgb_only() => {
                return Err(error::CoreError::InvalidState(
                    "CGB-only ROM cannot run in DMG mode".to_string(),
                ));
            }
            Some(mode) => mode,
            None => {
                if cartridge.supports_cgb() {
                    HardwareMode::Cgb
                } else {
                    HardwareMode::Dmg
                }
            }
        };

        self.cartridge = cartridge;
        self.cpu = Cpu::default();
        self.ppu = Ppu::default();
        self.timer = Timer::default();
        self.interrupt = InterruptState::default();
        self.joypad = Joypad::default();
        self.dma = DmaState::default();
        self.serial = Serial::default();
        self.apu = Apu::default();
        self.memory = Memory::default();
        self.clocks = ClockState::default();
        self.mode = selected_mode;
        self.apu.set_cgb_mode(self.mode == HardwareMode::Cgb);
        self.cgb_double_speed = false;
        self.cgb_speed_switch_armed = false;
        self.cgb_speed_switch_freeze_cycles = 0;
        self.initialize_post_boot_state();
        if self.mode == HardwareMode::Cgb && !self.cartridge.supports_cgb() {
            self.initialize_cgb_compatibility_palettes();
        }
        self.ppu.restore_runtime_state(&self.memory);
        self.sync_core_memory();
        self.ppu.render_visible_area_from_memory_with_mode(
            &self.memory,
            self.mode == HardwareMode::Cgb,
        );
        self.sync_apu_memory();
        self.sync_cgb_memory();
        self.pending_dma_trace.clear();
        self.pending_mapper_trace.clear();
        self.pending_io_trace.clear();
        self.pending_apu_trace.clear();
        self.pending_cgb_trace.clear();
        self.pending_interrupt_trace.clear();
        self.pending_cpu_stall_cycles = 0;
        self.cgb_speed_switch_freeze_cycles = 0;
        self.pending_cgb_trace.push(CgbTraceEvent::ModeSelected {
            cgb_enabled: self.mode == HardwareMode::Cgb,
            cgb_only: self.cartridge.cgb_only(),
        });
        Ok(())
    }

    fn initialize_post_boot_state(&mut self) {
        match self.mode {
            HardwareMode::Dmg => {
                self.cpu.a = 0x01;
                self.cpu.f.0 = 0xB0;
                self.cpu.b = 0x00;
                self.cpu.c = 0x13;
                self.cpu.d = 0x00;
                self.cpu.e = 0xD8;
                self.cpu.h = 0x01;
                self.cpu.l = 0x4D;
                self.cpu.sp = 0xFFFE;
                self.cpu.pc = 0x0100;
                self.cpu.ime = false;
                self.cpu.ime_enable_delay = 0;
                self.cpu.halted = false;
                self.cpu.halt_bug = false;
                self.cpu.stopped = false;

                self.ppu.lcdc = 0x91;
                self.ppu.stat = 0x80;
                self.ppu.ly = 0x91;
                self.ppu.lyc = 0x00;
                self.ppu.scx = 0x00;
                self.ppu.scy = 0x00;
                self.ppu.wx = 0x00;
                self.ppu.wy = 0x00;
                self.ppu.bgp = 0xFC;
                self.ppu.obp0 = 0xFF;
                self.ppu.obp1 = 0xFF;
                self.ppu.mode_cycles = 0;
            }
            HardwareMode::Cgb => {
                self.cpu.a = 0x11;
                self.cpu.f.0 = 0x80;
                self.cpu.b = 0x00;
                self.cpu.c = 0x00;
                self.cpu.d = 0xFF;
                self.cpu.e = 0x56;
                self.cpu.h = 0x00;
                self.cpu.l = 0x0D;
                self.cpu.sp = 0xFFFE;
                self.cpu.pc = 0x0100;
                self.cpu.ime = false;
                self.cpu.ime_enable_delay = 0;
                self.cpu.halted = false;
                self.cpu.halt_bug = false;
                self.cpu.stopped = false;

                self.ppu.lcdc = 0x91;
                self.ppu.stat = 0x80;
                self.ppu.ly = 0x00;
                self.ppu.lyc = 0x00;
                self.ppu.scx = 0x00;
                self.ppu.scy = 0x00;
                self.ppu.wx = 0x00;
                self.ppu.wy = 0x00;
                self.ppu.bgp = 0xFC;
                self.ppu.obp0 = 0xFF;
                self.ppu.obp1 = 0xFF;

                self.serial.sc = 0x01;
                self.dma.ff46 = 0x00;
            }
        }

        self.timer
            .initialize_post_boot_state(self.mode == HardwareMode::Cgb);
        self.interrupt.ie = 0x00;
        self.interrupt.iflag = 0x01;
        self.apu.initialize_post_boot_state(self.mode);
        self.clocks.frame_phase_ppu_cycles = u32::from(self.ppu.ly) * 456 + self.ppu.mode_cycles;
    }

    fn initialize_cgb_compatibility_palettes(&mut self) {
        fn rgb555(r: u16, g: u16, b: u16) -> u16 {
            (r & 0x1F) | ((g & 0x1F) << 5) | ((b & 0x1F) << 10)
        }

        let palette = [
            rgb555(31, 31, 26),
            rgb555(22, 27, 14),
            rgb555(10, 18, 8),
            rgb555(2, 6, 3),
        ];

        for palette_index in 0..8u8 {
            for (color_index, rgb) in palette.iter().copied().enumerate() {
                let base = palette_index * 8 + color_index as u8 * 2;
                self.memory.set_bg_palette_index(base);
                self.memory
                    .write_bg_palette_data((rgb & 0x00FF) as u8, false);
                self.memory.set_bg_palette_index(base + 1);
                self.memory.write_bg_palette_data((rgb >> 8) as u8, false);
                self.memory.set_obj_palette_index(base);
                self.memory
                    .write_obj_palette_data((rgb & 0x00FF) as u8, false);
                self.memory.set_obj_palette_index(base + 1);
                self.memory.write_obj_palette_data((rgb >> 8) as u8, false);
            }
        }
        self.memory.set_bg_palette_index(0);
        self.memory.set_obj_palette_index(0);
    }

    fn sync_core_memory(&mut self) {
        self.memory.write8(0xFF00, self.joypad.read_p1());
        self.memory.write8(0xFF01, self.serial.sb);
        self.memory.write8(0xFF02, self.serial.read_sc());
        self.memory.write8(0xFF04, self.timer.div as u8);
        self.memory.write8(0xFF05, self.timer.tima);
        self.memory.write8(0xFF06, self.timer.tma);
        self.memory.write8(0xFF07, self.timer.tac | 0xF8);
        self.memory.write8(0xFF0F, self.interrupt.iflag | 0xE0);
        self.memory.write8(0xFF40, self.ppu.lcdc);
        self.memory.write8(0xFF41, self.ppu.read_stat());
        self.memory.write8(0xFF42, self.ppu.scy);
        self.memory.write8(0xFF43, self.ppu.scx);
        self.memory.write8(0xFF44, self.ppu.ly);
        self.memory.write8(0xFF45, self.ppu.lyc);
        self.memory.write8(0xFF46, self.dma.ff46);
        self.memory.write8(0xFF47, self.ppu.bgp);
        self.memory.write8(0xFF48, self.ppu.obp0);
        self.memory.write8(0xFF49, self.ppu.obp1);
        self.memory.write8(0xFF4A, self.ppu.wy);
        self.memory.write8(0xFF4B, self.ppu.wx);
        self.memory.write8(0xFFFF, self.interrupt.ie);
    }

    pub fn exchange_serial_with_peer(&mut self, peer: &mut Self) -> SerialLinkTraceResult {
        let exchange = self.serial.exchange_with_peer(&mut peer.serial);
        let mut out = SerialLinkTraceResult {
            completed: exchange.completed,
            self_io_trace: exchange
                .self_trace
                .into_iter()
                .map(IoTraceEvent::Serial)
                .collect(),
            peer_io_trace: exchange
                .peer_trace
                .into_iter()
                .map(IoTraceEvent::Serial)
                .collect(),
            ..SerialLinkTraceResult::default()
        };
        if !out.completed {
            return out;
        }

        self.memory.write8(0xFF01, self.serial.sb);
        self.memory.write8(0xFF02, self.serial.read_sc());
        peer.memory.write8(0xFF01, peer.serial.sb);
        peer.memory.write8(0xFF02, peer.serial.read_sc());

        if exchange.self_interrupt_requested {
            let before = self.pending_interrupt_trace.len();
            self.request_interrupt(INT_SERIAL, InterruptSource::Serial);
            out.self_interrupt_trace
                .extend(self.pending_interrupt_trace.split_off(before));
        }
        if exchange.peer_interrupt_requested {
            let before = peer.pending_interrupt_trace.len();
            peer.request_interrupt(INT_SERIAL, InterruptSource::Serial);
            out.peer_interrupt_trace
                .extend(peer.pending_interrupt_trace.split_off(before));
        }

        out
    }

    pub fn step_instruction(&mut self) -> Result<StepResult, error::CoreError> {
        let mut serviced_interrupt = false;
        if self.interrupt.has_pending() {
            let pending_mask = self.interrupt.pending_mask();
            if !self.cpu.ime {
                self.pending_interrupt_trace
                    .push(InterruptTraceEvent::PendingBlocked {
                        pending_mask,
                        ime: self.cpu.ime,
                        halted: self.cpu.halted,
                    });
            }
            self.cpu.halted = false;
            if self.cpu.ime {
                self.service_interrupt()?;
                serviced_interrupt = true;
            }
        }

        if serviced_interrupt {
            return Ok(self.finish_step(20));
        }

        if self.cpu.halted {
            return Ok(self.finish_step(4));
        }

        let pc = self.cpu.pc;
        let opcode = self.read8(pc);
        if self.cpu.halt_bug {
            self.cpu.halt_bug = false;
        } else {
            self.cpu.pc = self.cpu.pc.wrapping_add(1);
        }
        let cycles = self.execute_opcode(opcode)?;
        Ok(self.finish_step(cycles))
    }

    pub fn step_instruction_fast(&mut self) -> Result<(), error::CoreError> {
        let mut serviced_interrupt = false;
        if self.interrupt.has_pending() {
            if !self.cpu.ime {
                self.cpu.halted = false;
            }
            if self.cpu.ime {
                self.service_interrupt()?;
                serviced_interrupt = true;
            }
        }

        if serviced_interrupt {
            self.finish_step_fast(20);
            return Ok(());
        }

        if self.cpu.halted {
            self.finish_step_fast(4);
            return Ok(());
        }

        let pc = self.cpu.pc;
        let opcode = self.read8(pc);
        if self.cpu.halt_bug {
            self.cpu.halt_bug = false;
        } else {
            self.cpu.pc = self.cpu.pc.wrapping_add(1);
        }
        let cycles = self.execute_opcode(opcode)?;
        self.finish_step_fast(cycles);
        Ok(())
    }

    pub fn run_frame(&mut self) -> Result<(), error::CoreError> {
        let start = self.clocks.frames;
        while self.clocks.frames == start {
            self.step_instruction()?;
        }
        Ok(())
    }

    pub fn run_frame_fast(&mut self) -> Result<(), error::CoreError> {
        let start = self.clocks.frames;
        while self.clocks.frames == start {
            self.step_instruction_fast()?;
        }
        Ok(())
    }

    pub fn run_audio_slice(
        &mut self,
        min_additional_frames: usize,
    ) -> Result<usize, error::CoreError> {
        if min_additional_frames == 0 {
            return Ok(0);
        }

        let start_buffered = self.audio_frames_available();
        let target_buffered = start_buffered.saturating_add(min_additional_frames);
        let max_cycles = self
            .clocks
            .cycles
            .saturating_add(self.ppu_cycles_to_cpu_cycles(self.ppu.frame_cycles()) as u64);
        let mut steps = 0usize;

        while self.audio_frames_available() < target_buffered
            && self.clocks.cycles < max_cycles
            && steps < 8_192
        {
            self.step_instruction()?;
            steps += 1;
        }

        Ok(self.audio_frames_available().saturating_sub(start_buffered))
    }

    pub fn run_slice(
        &mut self,
        max_instructions: usize,
        max_cpu_cycles: u32,
    ) -> Result<RunSliceResult, error::CoreError> {
        if max_instructions == 0 || max_cpu_cycles == 0 {
            return Ok(RunSliceResult::default());
        }

        let mut result = RunSliceResult::default();
        while result.instructions < max_instructions && result.cycles < max_cpu_cycles {
            let step = self.step_instruction()?;
            result.instructions += 1;
            result.cycles = result.cycles.saturating_add(step.cycles);
            if step.frame_completed {
                result.frame_completed = true;
                break;
            }
        }
        Ok(result)
    }

    pub fn set_joypad_mask(&mut self, mask: u8) {
        let trace = self.joypad.set_mask(mask);
        self.capture_joypad_trace(trace);
    }

    pub fn save_state(&self) -> MachineState {
        let mut snapshot = self.clone();
        snapshot.sync_core_memory();
        snapshot.sync_apu_memory();
        snapshot.sync_cgb_memory();
        MachineState::from_machine(&snapshot)
    }

    pub fn load_state(&mut self, state: &MachineState) {
        state.apply_to(self);
        self.apu.set_cgb_mode(self.mode == HardwareMode::Cgb);
        self.ppu.restore_runtime_state(&self.memory);
        self.ppu.render_visible_area_from_memory_with_mode(
            &self.memory,
            self.mode == HardwareMode::Cgb,
        );
        self.sync_apu_memory();
        self.sync_cgb_memory();
        self.pending_dma_trace.clear();
        self.pending_mapper_trace.clear();
        self.pending_io_trace.clear();
        self.pending_apu_trace.clear();
        self.pending_cgb_trace.clear();
        self.pending_interrupt_trace.clear();
        self.pending_cpu_stall_cycles = 0;
    }

    pub fn framebuffer(&self) -> &[u8; 160 * 144] {
        &self.ppu.framebuffer
    }

    pub fn framebuffer_rgb555(&self) -> &[u16] {
        &self.ppu.color_framebuffer
    }

    pub fn is_cgb_compat_mode(&self) -> bool {
        self.mode == HardwareMode::Cgb && !self.cartridge.supports_cgb()
    }

    pub fn current_rom_bank(&self) -> u16 {
        self.cartridge.current_rom_bank()
    }

    pub fn has_battery_backed_ram(&self) -> bool {
        self.cartridge.has_battery_backed_ram()
    }

    pub fn battery_save_bytes(&self) -> Option<Vec<u8>> {
        self.cartridge.battery_save_bytes()
    }

    pub fn battery_save_dirty(&self) -> bool {
        self.cartridge.battery_save_dirty()
    }

    pub fn battery_ram_access_enabled(&self) -> bool {
        self.cartridge.battery_ram_access_enabled()
    }

    pub fn mark_battery_save_clean(&mut self) {
        self.cartridge.mark_battery_save_clean();
    }

    pub fn load_battery_save_bytes(&mut self, bytes: &[u8]) -> usize {
        self.cartridge.load_battery_save_bytes(bytes)
    }

    pub fn audio_sample_rate(&self) -> u32 {
        self.apu.output_sample_rate()
    }

    pub fn audio_source_sample_rate(&self) -> u32 {
        self.audio_sample_rate()
    }

    pub fn audio_frames_available(&self) -> usize {
        self.apu.buffered_frames()
    }

    pub fn audio_frames_dropped(&self) -> u64 {
        self.apu.dropped_frames()
    }

    pub fn audio_buffer_capacity_frames(&self) -> usize {
        self.apu.buffer_capacity_frames()
    }

    pub fn set_audio_buffer_capacity_frames(
        &mut self,
        capacity_frames: usize,
    ) -> Result<(), String> {
        self.apu.set_buffer_capacity_frames(capacity_frames)
    }

    pub fn drain_audio_frames_interleaved_i16(&mut self, max_frames: usize) -> Vec<i16> {
        self.apu.drain_interleaved_i16(max_frames)
    }

    pub fn current_ram_bank(&self) -> u16 {
        self.cartridge.current_ram_bank()
    }

    pub fn peek8(&self, addr: u16) -> u8 {
        if self.dma_blocks_cpu_access(addr) {
            return 0xFF;
        }
        match addr {
            0x0000..=0x7FFF => self.cartridge.read_rom(addr),
            0x8000..=0x9FFF => {
                if self.ppu.can_cpu_access_vram() {
                    self.memory.read8(addr)
                } else {
                    0xFF
                }
            }
            0xA000..=0xBFFF => self.cartridge.read_ram(addr),
            0xFF00 => self.joypad.read_p1(),
            0xFF01 => self.serial.sb,
            0xFF02 => self.serial.read_sc(),
            0xFF04 => self.timer.div as u8,
            0xFF05 => self.timer.tima,
            0xFF06 => self.timer.tma,
            0xFF07 => self.timer.tac | 0xF8,
            0xFF10..=0xFF26 | 0xFF30..=0xFF3F => self.apu.read(addr),
            0xFF76..=0xFF77 => {
                if self.mode == HardwareMode::Cgb {
                    self.apu.read(addr)
                } else {
                    0xFF
                }
            }
            0xFF0F => self.interrupt.iflag | 0xE0,
            0xFF40 => self.ppu.lcdc,
            0xFF41 => self.ppu.read_stat(),
            0xFF42 => self.ppu.scy,
            0xFF43 => self.ppu.scx,
            0xFF44 => self.ppu.ly,
            0xFF45 => self.ppu.lyc,
            0xFF46 => self.dma.ff46,
            0xFF47 => self.ppu.bgp,
            0xFF48 => self.ppu.obp0,
            0xFF49 => self.ppu.obp1,
            0xFF4A => self.ppu.wy,
            0xFF4B => self.ppu.wx,
            0xFF4D => self.read_key1(),
            0xFF4F => {
                if self.mode == HardwareMode::Cgb {
                    self.memory.vbk_register()
                } else {
                    0xFF
                }
            }
            0xFF51 => self.dma.hdma1,
            0xFF52 => self.dma.hdma2 | 0x0F,
            0xFF53 => self.dma.hdma3 | 0xE0,
            0xFF54 => self.dma.hdma4 | 0x0F,
            0xFF55 => self.dma.hdma5,
            0xFF68 => {
                if self.mode == HardwareMode::Cgb {
                    self.memory.bg_palette_index_register()
                } else {
                    0xFF
                }
            }
            0xFF69 => {
                if self.mode == HardwareMode::Cgb {
                    self.memory
                        .read_bg_palette_data(self.cgb_palette_access_blocked())
                } else {
                    0xFF
                }
            }
            0xFF6A => {
                if self.mode == HardwareMode::Cgb {
                    self.memory.obj_palette_index_register()
                } else {
                    0xFF
                }
            }
            0xFF6B => {
                if self.mode == HardwareMode::Cgb {
                    self.memory
                        .read_obj_palette_data(self.cgb_palette_access_blocked())
                } else {
                    0xFF
                }
            }
            0xFF70 => {
                if self.mode == HardwareMode::Cgb {
                    self.memory.svbk_register()
                } else {
                    0xFF
                }
            }
            0xFFFF => self.interrupt.ie,
            0xFE00..=0xFE9F => {
                if self.ppu.can_cpu_access_oam() && !self.dma.active {
                    self.memory.read8(addr)
                } else {
                    0xFF
                }
            }
            _ => self.memory.read8(addr),
        }
    }

    pub fn read8(&mut self, addr: u16) -> u8 {
        let value = self.peek8(addr);
        if addr == 0xFF00 {
            self.pending_io_trace
                .push(IoTraceEvent::Joypad(JoypadTraceEvent::Read {
                    p1: value,
                    select: self.joypad.p1 & 0x30,
                    mask: self.joypad.mask,
                }));
        }
        value
    }

    fn read8_timed(&mut self, addr: u16, cpu_cycles_until_sample: u32) -> u8 {
        if let Some(value) = self.predict_timed_io_read(addr, cpu_cycles_until_sample) {
            value
        } else {
            self.read8(addr)
        }
    }

    fn predict_timed_io_read(&self, addr: u16, cpu_cycles_until_sample: u32) -> Option<u8> {
        match addr {
            // Tight STAT/LY polling loops often observe the register value near the end of a
            // load instruction. Predict the PPU state at that microstep so MMIO reads don't
            // lag one instruction behind line/mode edges.
            0xFF41 | 0xFF44 => {
                let ppu_cycles = self.cpu_cycles_to_ppu_cycles(cpu_cycles_until_sample);
                Some(match addr {
                    0xFF41 => self.ppu.predict_stat_after_cycles(ppu_cycles),
                    0xFF44 => self.ppu.predict_ly_after_cycles(ppu_cycles),
                    _ => unreachable!(),
                })
            }
            _ => None,
        }
    }

    pub fn write8(&mut self, addr: u16, value: u8) {
        if self.dma_blocks_cpu_access(addr) {
            return;
        }
        match addr {
            0x0000..=0x7FFF => {
                let mapper = self.cartridge.mapper_kind();
                let before_rom_bank = self.cartridge.current_rom_bank();
                let before_ram_bank = self.cartridge.current_ram_bank();
                self.cartridge.write_mbc(addr, value);
                let after_rom_bank = self.cartridge.current_rom_bank();
                let after_ram_bank = self.cartridge.current_ram_bank();
                if mapper.is_special() {
                    self.pending_mapper_trace
                        .push(MapperTraceEvent::ControlWrite {
                            mapper,
                            addr,
                            value,
                            rom_bank: after_rom_bank,
                            ram_bank: after_ram_bank,
                        });
                }
                if before_rom_bank != after_rom_bank {
                    self.pending_mapper_trace
                        .push(MapperTraceEvent::RomBankChange {
                            mapper,
                            addr,
                            value,
                            from: before_rom_bank,
                            to: after_rom_bank,
                        });
                }
                if before_ram_bank != after_ram_bank {
                    self.pending_mapper_trace
                        .push(MapperTraceEvent::RamBankChange {
                            mapper,
                            addr,
                            value,
                            from: before_ram_bank,
                            to: after_ram_bank,
                        });
                }
            }
            0x8000..=0x9FFF => {
                if self.ppu.can_cpu_access_vram() {
                    self.memory.write8(addr, value);
                }
            }
            0xA000..=0xBFFF => self.cartridge.write_ram(addr, value),
            0xFF00 => {
                let trace = self.joypad.write_p1(value);
                self.capture_joypad_trace(trace);
            }
            0xFF01 => {
                self.serial.write_sb(value);
                self.memory.write8(addr, value);
            }
            0xFF02 => {
                let trace = self.serial.write_sc(value);
                self.pending_io_trace
                    .extend(trace.into_iter().map(IoTraceEvent::Serial));
                self.memory.write8(addr, self.serial.read_sc());
            }
            0xFF04 => {
                let mut trace = Vec::new();
                self.timer.write_div(&mut trace);
                self.pending_io_trace
                    .extend(trace.into_iter().map(IoTraceEvent::Timer));
            }
            0xFF05 => self.timer.write_tima(value),
            0xFF06 => self.timer.write_tma(value),
            0xFF07 => {
                let mut trace = Vec::new();
                self.timer.write_tac(value, &mut trace);
                self.pending_io_trace
                    .extend(trace.into_iter().map(IoTraceEvent::Timer));
            }
            0xFF10..=0xFF26 | 0xFF30..=0xFF3F => {
                let trace = self.apu.write(addr, value);
                self.pending_apu_trace.extend(trace);
                self.sync_apu_memory();
            }
            0xFF0F => self.interrupt.iflag = value & 0x1F,
            0xFF40 => {
                let stat_line_raised = self.ppu.write_lcdc(value);
                self.clocks.frame_phase_ppu_cycles =
                    u32::from(self.ppu.ly) * 456 + self.ppu.mode_cycles;
                if stat_line_raised {
                    self.request_interrupt(INT_LCD_STAT, InterruptSource::LcdStat);
                }
            }
            0xFF41 => {
                let spurious = self.dmg_stat_write_causes_spurious_interrupt();
                let stat_line_raised = self.ppu.write_stat(value);
                if spurious || stat_line_raised {
                    self.request_interrupt(INT_LCD_STAT, InterruptSource::LcdStat);
                }
            }
            0xFF42 => self.ppu.scy = value,
            0xFF43 => self.ppu.scx = value,
            0xFF44 => {}
            0xFF45 => {
                if self.ppu.write_lyc(value) {
                    self.request_interrupt(INT_LCD_STAT, InterruptSource::LcdStat);
                }
            }
            0xFF46 => {
                self.dma.ff46 = value;
                self.memory.write8(addr, value);
                self.dma.active = true;
                self.dma.source = (value as u16) << 8;
                self.dma.bytes_copied = 0;
                self.dma.cycle_accum = 0;
                self.dma.start_delay_cycles = 4;
                self.pending_dma_trace.push(DmaTraceEvent::OamDmaStart {
                    source: self.dma.source,
                });
            }
            0xFF47 => self.ppu.bgp = value,
            0xFF48 => self.ppu.obp0 = value,
            0xFF49 => self.ppu.obp1 = value,
            0xFF4A => self.ppu.wy = value,
            0xFF4B => self.ppu.wx = value,
            0xFF4D => self.write_key1(value),
            0xFF4F => {
                if self.mode == HardwareMode::Cgb {
                    let bank = self.memory.set_vbk(value);
                    self.pending_cgb_trace
                        .push(CgbTraceEvent::VramBankSwitch { bank, value });
                }
            }
            0xFF51 => {
                self.dma.hdma1 = value;
                self.memory.write8(addr, value);
            }
            0xFF52 => {
                self.dma.hdma2 = value & 0xF0;
                self.memory.write8(addr, value & 0xF0);
            }
            0xFF53 => {
                self.dma.hdma3 = value & 0x1F;
                self.memory.write8(addr, value & 0x1F);
            }
            0xFF54 => {
                self.dma.hdma4 = value & 0xF0;
                self.memory.write8(addr, value & 0xF0);
            }
            0xFF55 => self.write_hdma_control(value),
            0xFF68 => {
                if self.mode == HardwareMode::Cgb {
                    let reg = self.memory.set_bg_palette_index(value);
                    self.pending_cgb_trace
                        .push(CgbTraceEvent::BgPaletteIndexWrite {
                            index: reg & 0x3F,
                            auto_increment: reg & 0x80 != 0,
                        });
                }
            }
            0xFF69 => {
                if self.mode == HardwareMode::Cgb {
                    let result = self
                        .memory
                        .write_bg_palette_data(value, self.cgb_palette_access_blocked());
                    self.pending_cgb_trace
                        .push(CgbTraceEvent::BgPaletteDataWrite {
                            index: result.index,
                            value: result.value,
                            blocked: result.blocked,
                            auto_increment: result.auto_increment,
                        });
                }
            }
            0xFF6A => {
                if self.mode == HardwareMode::Cgb {
                    let reg = self.memory.set_obj_palette_index(value);
                    self.pending_cgb_trace
                        .push(CgbTraceEvent::ObjPaletteIndexWrite {
                            index: reg & 0x3F,
                            auto_increment: reg & 0x80 != 0,
                        });
                }
            }
            0xFF6B => {
                if self.mode == HardwareMode::Cgb {
                    let result = self
                        .memory
                        .write_obj_palette_data(value, self.cgb_palette_access_blocked());
                    self.pending_cgb_trace
                        .push(CgbTraceEvent::ObjPaletteDataWrite {
                            index: result.index,
                            value: result.value,
                            blocked: result.blocked,
                            auto_increment: result.auto_increment,
                        });
                }
            }
            0xFF70 => {
                if self.mode == HardwareMode::Cgb {
                    let bank = self.memory.set_svbk(value);
                    self.pending_cgb_trace
                        .push(CgbTraceEvent::WramBankSwitch { bank, value });
                }
            }
            0xFF76..=0xFF77 => {}
            0xFFFF => self.interrupt.ie = value,
            0xFE00..=0xFE9F => {
                if self.ppu.can_cpu_access_oam() && !self.dma.active {
                    self.memory.write8(addr, value);
                }
            }
            _ => self.memory.write8(addr, value),
        }
    }

    fn read16_imm(&mut self) -> u16 {
        let lo = self.read8(self.cpu.pc) as u16;
        self.cpu.pc = self.cpu.pc.wrapping_add(1);
        let hi = self.read8(self.cpu.pc) as u16;
        self.cpu.pc = self.cpu.pc.wrapping_add(1);
        (hi << 8) | lo
    }

    fn push16(&mut self, value: u16) {
        self.cpu.sp = self.cpu.sp.wrapping_sub(1);
        self.write8(self.cpu.sp, (value >> 8) as u8);
        self.cpu.sp = self.cpu.sp.wrapping_sub(1);
        self.write8(self.cpu.sp, value as u8);
    }

    fn pop16(&mut self) -> u16 {
        let lo = self.read8(self.cpu.sp) as u16;
        self.cpu.sp = self.cpu.sp.wrapping_add(1);
        let hi = self.read8(self.cpu.sp) as u16;
        self.cpu.sp = self.cpu.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    fn jr(&mut self, offset: i8) {
        self.cpu.pc = self.cpu.pc.wrapping_add_signed(offset as i16);
    }

    fn inc8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h((value & 0x0F) + 1 > 0x0F);
        result
    }

    fn dec8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(true);
        self.cpu.f.set_h((value & 0x0F) == 0);
        result
    }

    fn and_a(&mut self, value: u8) {
        self.cpu.a &= value;
        self.cpu.f.set_z(self.cpu.a == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(true);
        self.cpu.f.set_c(false);
    }

    fn or_a(&mut self, value: u8) {
        self.cpu.a |= value;
        self.cpu.f.set_z(self.cpu.a == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(false);
    }

    fn xor_a(&mut self, value: u8) {
        self.cpu.a ^= value;
        self.cpu.f.set_z(self.cpu.a == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(false);
    }

    fn cp_a(&mut self, value: u8) {
        let a = self.cpu.a;
        let result = a.wrapping_sub(value);
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(true);
        self.cpu.f.set_h((a & 0x0F) < (value & 0x0F));
        self.cpu.f.set_c(a < value);
    }

    fn add_a(&mut self, value: u8) {
        let a = self.cpu.a;
        let result = a.wrapping_add(value);
        self.cpu.a = result;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(((a & 0x0F) + (value & 0x0F)) > 0x0F);
        self.cpu.f.set_c((a as u16 + value as u16) > 0xFF);
    }

    fn sub_a(&mut self, value: u8) {
        let a = self.cpu.a;
        let result = a.wrapping_sub(value);
        self.cpu.a = result;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(true);
        self.cpu.f.set_h((a & 0x0F) < (value & 0x0F));
        self.cpu.f.set_c(a < value);
    }

    fn adc_a(&mut self, value: u8) {
        let a = self.cpu.a;
        let carry = if self.cpu.f.c() { 1 } else { 0 };
        let result16 = a as u16 + value as u16 + carry as u16;
        let result = result16 as u8;
        self.cpu.a = result;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu
            .f
            .set_h(((a & 0x0F) + (value & 0x0F) + carry) > 0x0F);
        self.cpu.f.set_c(result16 > 0xFF);
    }

    fn sbc_a(&mut self, value: u8) {
        let a = self.cpu.a;
        let carry = if self.cpu.f.c() { 1 } else { 0 };
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        self.cpu.a = result;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(true);
        self.cpu.f.set_h((a & 0x0F) < ((value & 0x0F) + carry));
        self.cpu.f.set_c((a as u16) < (value as u16 + carry as u16));
    }

    fn rlca(&mut self) {
        let carry = (self.cpu.a & 0x80) != 0;
        self.cpu.a = self.cpu.a.rotate_left(1);
        self.cpu.f.set_z(false);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
    }
    fn rla(&mut self) {
        let carry_in = if self.cpu.f.c() { 1 } else { 0 };
        let carry = (self.cpu.a & 0x80) != 0;
        self.cpu.a = (self.cpu.a << 1) | carry_in;
        self.cpu.f.set_z(false);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
    }
    fn rrca(&mut self) {
        let carry = (self.cpu.a & 0x01) != 0;
        self.cpu.a = self.cpu.a.rotate_right(1);
        self.cpu.f.set_z(false);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
    }

    fn daa(&mut self) {
        let mut a = self.cpu.a;
        let mut adjust = 0u8;
        let mut carry = self.cpu.f.c();
        if !self.cpu.f.n() {
            if self.cpu.f.h() || (a & 0x0F) > 0x09 {
                adjust |= 0x06;
            }
            if carry || a > 0x99 {
                adjust |= 0x60;
                carry = true;
            }
            a = a.wrapping_add(adjust);
        } else {
            if self.cpu.f.h() {
                adjust |= 0x06;
            }
            if carry {
                adjust |= 0x60;
            }
            a = a.wrapping_sub(adjust);
        }
        self.cpu.a = a;
        self.cpu.f.set_z(a == 0);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
    }

    fn add_signed_to_sp_like_flags(&mut self, base: u16, offset: i8) -> u16 {
        let result = base.wrapping_add_signed(offset as i16);
        self.cpu.f.set_z(false);
        self.cpu.f.set_n(false);
        self.cpu
            .f
            .set_h(((base & 0x000F) + ((offset as i16 as u16) & 0x000F)) > 0x000F);
        self.cpu
            .f
            .set_c(((base & 0x00FF) + ((offset as i16 as u16) & 0x00FF)) > 0x00FF);
        result
    }

    fn read_r8(&mut self, index: u8) -> u8 {
        match index & 0x07 {
            0 => self.cpu.b,
            1 => self.cpu.c,
            2 => self.cpu.d,
            3 => self.cpu.e,
            4 => self.cpu.h,
            5 => self.cpu.l,
            6 => self.read8(self.cpu.hl()),
            _ => self.cpu.a,
        }
    }

    fn write_r8(&mut self, index: u8, value: u8) {
        match index & 0x07 {
            0 => self.cpu.b = value,
            1 => self.cpu.c = value,
            2 => self.cpu.d = value,
            3 => self.cpu.e = value,
            4 => self.cpu.h = value,
            5 => self.cpu.l = value,
            6 => {
                let addr = self.cpu.hl();
                self.write8(addr, value);
            }
            _ => self.cpu.a = value,
        }
    }

    fn ld_bc(&self) -> u16 {
        ((self.cpu.b as u16) << 8) | self.cpu.c as u16
    }
    fn set_bc(&mut self, value: u16) {
        self.cpu.b = (value >> 8) as u8;
        self.cpu.c = value as u8;
    }
    fn ld_de(&self) -> u16 {
        ((self.cpu.d as u16) << 8) | self.cpu.e as u16
    }
    fn set_de(&mut self, value: u16) {
        self.cpu.d = (value >> 8) as u8;
        self.cpu.e = value as u8;
    }
    fn add_hl(&mut self, value: u16) {
        let hl = self.cpu.hl();
        let result = hl.wrapping_add(value);
        self.cpu.f.set_n(false);
        self.cpu
            .f
            .set_h(((hl & 0x0FFF) + (value & 0x0FFF)) > 0x0FFF);
        self.cpu.f.set_c((hl as u32 + value as u32) > 0xFFFF);
        self.cpu.set_hl(result);
    }

    fn inc16(value: u16) -> u16 {
        value.wrapping_add(1)
    }
    fn dec16(value: u16) -> u16 {
        value.wrapping_sub(1)
    }

    fn service_interrupt(&mut self) -> Result<(), error::CoreError> {
        if let Some((mask, vector)) = self.interrupt.highest_priority() {
            self.cpu.ime = false;
            self.interrupt.acknowledge(mask);
            self.push16(self.cpu.pc);
            self.cpu.pc = vector;
            self.pending_interrupt_trace
                .push(InterruptTraceEvent::Serviced {
                    source: self.interrupt_source_from_mask(mask),
                    vector,
                });
            Ok(())
        } else {
            Err(error::CoreError::InvalidState(
                "interrupt service requested without pending source".into(),
            ))
        }
    }

    fn cgb_fast_domains_enabled(&self) -> bool {
        self.mode == HardwareMode::Cgb && self.cgb_double_speed
    }

    fn cpu_cycles_to_ppu_cycles(&self, cpu_cycles: u32) -> u32 {
        if self.cgb_fast_domains_enabled() {
            cpu_cycles / 2
        } else {
            cpu_cycles
        }
    }

    fn cpu_cycles_to_ppu_cycles_ceil(&self, cpu_cycles: u32) -> u32 {
        if self.cgb_fast_domains_enabled() {
            (cpu_cycles.saturating_add(1)) / 2
        } else {
            cpu_cycles
        }
    }

    fn ppu_cycles_to_cpu_cycles(&self, ppu_cycles: u32) -> u32 {
        if self.cgb_fast_domains_enabled() {
            ppu_cycles.saturating_mul(2)
        } else {
            ppu_cycles
        }
    }

    fn hdma_block_stall_cycles(&self) -> u32 {
        if self.cgb_fast_domains_enabled() {
            CGB_DMA_BLOCK_STALL_CYCLES.saturating_mul(2)
        } else {
            CGB_DMA_BLOCK_STALL_CYCLES
        }
    }

    fn finish_step(&mut self, cycles: u32) -> StepResult {
        let total_cycles =
            cycles.saturating_add(std::mem::take(&mut self.pending_cpu_stall_cycles));
        let total_ppu_cycles = self.cpu_cycles_to_ppu_cycles(total_cycles);
        let (
            frame_completed,
            ppu_trace,
            dma_trace_from_clocks,
            mapper_trace_from_clocks,
            io_trace_from_clocks,
            apu_trace_from_clocks,
        ) = self.advance_clocks(total_cycles, total_ppu_cycles);
        let mut dma_trace = std::mem::take(&mut self.pending_dma_trace);
        dma_trace.extend(dma_trace_from_clocks);
        let mut mapper_trace = std::mem::take(&mut self.pending_mapper_trace);
        mapper_trace.extend(mapper_trace_from_clocks);
        let mut io_trace = std::mem::take(&mut self.pending_io_trace);
        io_trace.extend(io_trace_from_clocks);
        let mut apu_trace = std::mem::take(&mut self.pending_apu_trace);
        apu_trace.extend(apu_trace_from_clocks);
        let cgb_trace = std::mem::take(&mut self.pending_cgb_trace);
        let interrupt_trace = std::mem::take(&mut self.pending_interrupt_trace);
        self.commit_ime_enable_delay();
        StepResult {
            cycles: total_cycles,
            frame_completed,
            ppu_trace,
            dma_trace,
            mapper_trace,
            io_trace,
            apu_trace,
            cgb_trace,
            interrupt_trace,
        }
    }

    fn finish_step_fast(&mut self, cycles: u32) {
        let total_cycles =
            cycles.saturating_add(std::mem::take(&mut self.pending_cpu_stall_cycles));
        let total_ppu_cycles = self.cpu_cycles_to_ppu_cycles(total_cycles);
        self.advance_clocks_fast(total_cycles, total_ppu_cycles);
        self.clear_pending_traces();
        self.commit_ime_enable_delay();
    }

    fn clear_pending_traces(&mut self) {
        self.pending_dma_trace.clear();
        self.pending_mapper_trace.clear();
        self.pending_io_trace.clear();
        self.pending_apu_trace.clear();
        self.pending_cgb_trace.clear();
        self.pending_interrupt_trace.clear();
    }

    fn commit_ime_enable_delay(&mut self) {
        if self.cpu.ime_enable_delay > 0 {
            self.cpu.ime_enable_delay -= 1;
            if self.cpu.ime_enable_delay == 0 {
                self.cpu.ime = true;
            }
        }
    }

    fn advance_clocks(
        &mut self,
        cpu_cycles: u32,
        ppu_cycles: u32,
    ) -> (
        bool,
        Vec<types::PpuTraceEvent>,
        Vec<DmaTraceEvent>,
        Vec<MapperTraceEvent>,
        Vec<IoTraceEvent>,
        Vec<ApuTraceEvent>,
    ) {
        let mut remaining_cpu = cpu_cycles;
        let mut remaining_ppu = ppu_cycles;
        let mut frame_completed = false;
        let mut ppu_trace = Vec::new();
        let mut dma_trace = Vec::new();
        let mapper_trace = Vec::new();
        let mut io_trace = Vec::new();
        let mut apu_trace = Vec::new();
        while remaining_cpu > 0 {
            if self.cgb_speed_switch_freeze_cycles > 0 {
                let freeze_step = remaining_cpu.min(self.cgb_speed_switch_freeze_cycles);
                self.cgb_speed_switch_freeze_cycles -= freeze_step;
                self.clocks.cycles += freeze_step as u64;
                remaining_cpu -= freeze_step;
                remaining_ppu =
                    remaining_ppu.saturating_sub(self.cpu_cycles_to_ppu_cycles(freeze_step));
                continue;
            }
            if remaining_ppu == 0 {
                let fast_only = remaining_cpu;
                self.clocks.cycles += fast_only as u64;
                let timer_result = self.timer.tick(fast_only);
                io_trace.extend(timer_result.trace.into_iter().map(IoTraceEvent::Timer));
                if timer_result.interrupt_requested {
                    self.request_interrupt(INT_TIMER, InterruptSource::Timer);
                }
                let serial_result = self.serial.tick(fast_only);
                io_trace.extend(serial_result.trace.into_iter().map(IoTraceEvent::Serial));
                if serial_result.interrupt_requested {
                    self.request_interrupt(INT_SERIAL, InterruptSource::Serial);
                }
                if self.dma.active {
                    self.tick_dma(fast_only, &mut dma_trace);
                }
                break;
            }
            if self.ppu.current_mode() == ppu::PpuMode::OamSearch && self.ppu.mode_cycles == 0 {
                self.ppu.latch_scanline_state(self.memory.oam());
            }

            let mut step_ppu = remaining_ppu;
            step_ppu = step_ppu.min(self.ppu.cycles_until_next_event());
            if self.dma.active {
                step_ppu = step_ppu
                    .min(self.cpu_cycles_to_ppu_cycles_ceil(self.cycles_until_next_dma_event()));
            }
            step_ppu = step_ppu.max(1);
            let step_cpu = self.ppu_cycles_to_cpu_cycles(step_ppu).min(remaining_cpu);

            self.clocks.cycles += step_cpu as u64;

            let timer_result = self.timer.tick(step_cpu);
            io_trace.extend(timer_result.trace.into_iter().map(IoTraceEvent::Timer));
            if timer_result.interrupt_requested {
                self.request_interrupt(INT_TIMER, InterruptSource::Timer);
            }
            let serial_result = self.serial.tick(step_cpu);
            io_trace.extend(serial_result.trace.into_iter().map(IoTraceEvent::Serial));
            if serial_result.interrupt_requested {
                self.request_interrupt(INT_SERIAL, InterruptSource::Serial);
            }
            let apu_result = self.apu.tick(step_ppu);
            apu_trace.extend(apu_result.trace);
            self.cartridge.tick(step_ppu);

            let ppu_result = self.ppu.tick(step_ppu);
            ppu_trace.extend(ppu_result.trace.iter().copied());
            if ppu_result.stat_interrupt {
                self.request_interrupt(INT_LCD_STAT, InterruptSource::LcdStat);
            }
            // Keep nominal frame slices moving even while LCDC is off.
            // Actual render-frame completion is still surfaced via PPU trace events.
            self.clocks.frame_phase_ppu_cycles =
                self.clocks.frame_phase_ppu_cycles.saturating_add(step_ppu);
            while self.clocks.frame_phase_ppu_cycles >= self.ppu.frame_cycles() {
                self.clocks.frame_phase_ppu_cycles -= self.ppu.frame_cycles();
                self.clocks.frames += 1;
                frame_completed = true;
            }
            if self.dma.active {
                self.tick_dma(step_cpu, &mut dma_trace);
            }
            for event in &ppu_result.trace {
                if let types::PpuTraceEvent::PpuModeChange { to_mode, .. } = *event {
                    if to_mode == ppu::PpuMode::HBlank as u8 {
                        let hdma_stall_cpu = self.tick_hdma_hblank(&mut dma_trace);
                        remaining_cpu = remaining_cpu.saturating_add(hdma_stall_cpu);
                        remaining_ppu = remaining_ppu
                            .saturating_add(self.cpu_cycles_to_ppu_cycles(hdma_stall_cpu));
                    }
                }
            }
            if let Some(_scanline) = ppu_result.render_scanline {
                self.ppu.render_current_scanline_from_memory_with_mode(
                    &self.memory,
                    self.mode == HardwareMode::Cgb,
                );
            }
            if ppu_result.vblank_enter {
                self.request_interrupt(INT_VBLANK, InterruptSource::Vblank);
            }

            remaining_cpu = remaining_cpu.saturating_sub(step_cpu);
            remaining_ppu = remaining_ppu.saturating_sub(step_ppu);
        }
        (
            frame_completed,
            ppu_trace,
            dma_trace,
            mapper_trace,
            io_trace,
            apu_trace,
        )
    }

    fn advance_clocks_fast(&mut self, cpu_cycles: u32, ppu_cycles: u32) -> bool {
        let mut remaining_cpu = cpu_cycles;
        let mut remaining_ppu = ppu_cycles;
        let mut frame_completed = false;
        let mut dma_trace = Vec::new();
        while remaining_cpu > 0 {
            if self.cgb_speed_switch_freeze_cycles > 0 {
                let freeze_step = remaining_cpu.min(self.cgb_speed_switch_freeze_cycles);
                self.cgb_speed_switch_freeze_cycles -= freeze_step;
                self.clocks.cycles += freeze_step as u64;
                remaining_cpu -= freeze_step;
                remaining_ppu =
                    remaining_ppu.saturating_sub(self.cpu_cycles_to_ppu_cycles(freeze_step));
                continue;
            }
            if remaining_ppu == 0 {
                let fast_only = remaining_cpu;
                self.clocks.cycles += fast_only as u64;
                if self.timer.tick_fast(fast_only) {
                    self.request_interrupt(INT_TIMER, InterruptSource::Timer);
                }
                if self.serial.tick_fast(fast_only) {
                    self.request_interrupt(INT_SERIAL, InterruptSource::Serial);
                }
                if self.dma.active {
                    self.tick_dma(fast_only, &mut dma_trace);
                }
                break;
            }
            if self.ppu.current_mode() == ppu::PpuMode::OamSearch && self.ppu.mode_cycles == 0 {
                self.ppu.latch_scanline_state(self.memory.oam());
            }

            let mut step_ppu = remaining_ppu;
            step_ppu = step_ppu.min(self.ppu.cycles_until_next_event());
            if self.dma.active {
                step_ppu = step_ppu
                    .min(self.cpu_cycles_to_ppu_cycles_ceil(self.cycles_until_next_dma_event()));
            }
            step_ppu = step_ppu.max(1);
            let step_cpu = self.ppu_cycles_to_cpu_cycles(step_ppu).min(remaining_cpu);

            self.clocks.cycles += step_cpu as u64;

            if self.timer.tick_fast(step_cpu) {
                self.request_interrupt(INT_TIMER, InterruptSource::Timer);
            }
            if self.serial.tick_fast(step_cpu) {
                self.request_interrupt(INT_SERIAL, InterruptSource::Serial);
            }
            self.apu.tick_fast(step_ppu);
            self.cartridge.tick(step_ppu);

            let ppu_result = self.ppu.tick_fast(step_ppu);
            if ppu_result.stat_interrupt {
                self.request_interrupt(INT_LCD_STAT, InterruptSource::LcdStat);
            }
            self.clocks.frame_phase_ppu_cycles =
                self.clocks.frame_phase_ppu_cycles.saturating_add(step_ppu);
            while self.clocks.frame_phase_ppu_cycles >= self.ppu.frame_cycles() {
                self.clocks.frame_phase_ppu_cycles -= self.ppu.frame_cycles();
                self.clocks.frames += 1;
                frame_completed = true;
            }
            if self.dma.active {
                self.tick_dma(step_cpu, &mut dma_trace);
            }
            if self.ppu.current_mode() == ppu::PpuMode::HBlank
                && ppu_result.render_scanline.is_some()
            {
                let hdma_stall_cpu = self.tick_hdma_hblank(&mut dma_trace);
                remaining_cpu = remaining_cpu.saturating_add(hdma_stall_cpu);
                remaining_ppu =
                    remaining_ppu.saturating_add(self.cpu_cycles_to_ppu_cycles(hdma_stall_cpu));
            }
            if ppu_result.render_scanline.is_some() {
                self.ppu.render_current_scanline_from_memory_with_mode(
                    &self.memory,
                    self.mode == HardwareMode::Cgb,
                );
            }
            if ppu_result.vblank_enter {
                self.request_interrupt(INT_VBLANK, InterruptSource::Vblank);
            }

            remaining_cpu = remaining_cpu.saturating_sub(step_cpu);
            remaining_ppu = remaining_ppu.saturating_sub(step_ppu);
        }
        frame_completed
    }

    fn sync_apu_memory(&mut self) {
        for addr in 0xFF10..=0xFF26 {
            self.memory.write8(addr, self.apu.read(addr));
        }
        for addr in 0xFF30..=0xFF3F {
            self.memory.write8(addr, self.apu.read(addr));
        }
        self.memory.write8(
            0xFF76,
            if self.mode == HardwareMode::Cgb {
                self.apu.read(0xFF76)
            } else {
                0xFF
            },
        );
        self.memory.write8(
            0xFF77,
            if self.mode == HardwareMode::Cgb {
                self.apu.read(0xFF77)
            } else {
                0xFF
            },
        );
    }

    fn sync_cgb_memory(&mut self) {
        self.memory.write8(0xFF4D, self.read_key1());
        self.memory.write8(
            0xFF4F,
            if self.mode == HardwareMode::Cgb {
                self.memory.vbk_register()
            } else {
                0xFF
            },
        );
        self.memory.write8(
            0xFF68,
            if self.mode == HardwareMode::Cgb {
                self.memory.bg_palette_index_register()
            } else {
                0xFF
            },
        );
        self.memory.write8(
            0xFF69,
            if self.mode == HardwareMode::Cgb {
                self.memory
                    .read_bg_palette_data(self.cgb_palette_access_blocked())
            } else {
                0xFF
            },
        );
        self.memory.write8(
            0xFF6A,
            if self.mode == HardwareMode::Cgb {
                self.memory.obj_palette_index_register()
            } else {
                0xFF
            },
        );
        self.memory.write8(
            0xFF6B,
            if self.mode == HardwareMode::Cgb {
                self.memory
                    .read_obj_palette_data(self.cgb_palette_access_blocked())
            } else {
                0xFF
            },
        );
        self.memory.write8(
            0xFF70,
            if self.mode == HardwareMode::Cgb {
                self.memory.svbk_register()
            } else {
                0xFF
            },
        );
    }

    fn cgb_palette_access_blocked(&self) -> bool {
        self.mode == HardwareMode::Cgb
            && self.ppu.lcd_enabled()
            && self.ppu.current_mode() == ppu::PpuMode::Transfer
    }

    fn read_key1(&self) -> u8 {
        if self.mode != HardwareMode::Cgb {
            return 0xFF;
        }
        ((self.cgb_double_speed as u8) << 7) | (self.cgb_speed_switch_armed as u8) | 0x7E
    }

    fn write_key1(&mut self, value: u8) {
        if self.mode != HardwareMode::Cgb {
            return;
        }
        self.cgb_speed_switch_armed = value & 0x01 != 0;
        self.sync_cgb_memory();
        self.pending_cgb_trace.push(CgbTraceEvent::Key1Write {
            armed: self.cgb_speed_switch_armed,
            double_speed: self.cgb_double_speed,
            value,
        });
    }

    fn perform_speed_switch(&mut self) {
        if self.mode != HardwareMode::Cgb || !self.cgb_speed_switch_armed {
            return;
        }
        self.cgb_speed_switch_armed = false;
        self.cgb_double_speed = !self.cgb_double_speed;
        self.cgb_speed_switch_freeze_cycles = self
            .cgb_speed_switch_freeze_cycles
            .saturating_add(CGB_SPEED_SWITCH_FREEZE_CYCLES);
        self.pending_cpu_stall_cycles = self
            .pending_cpu_stall_cycles
            .saturating_add(CGB_SPEED_SWITCH_FREEZE_CYCLES);
        self.sync_cgb_memory();
        self.pending_cgb_trace.push(CgbTraceEvent::SpeedSwitch {
            double_speed: self.cgb_double_speed,
            stop_stall_cycles: CGB_SPEED_SWITCH_FREEZE_CYCLES,
        });
        self.pending_cgb_trace
            .push(CgbTraceEvent::SpeedSwitchFreeze {
                cpu_cycles: CGB_SPEED_SWITCH_FREEZE_CYCLES,
                ppu_mode: self.ppu.current_mode() as u8,
            });
    }

    fn capture_joypad_trace(&mut self, trace: Vec<JoypadTraceEvent>) {
        for event in trace {
            if let JoypadTraceEvent::InterruptEdge { .. } = event {
                self.request_interrupt(INT_JOYPAD, InterruptSource::Joypad);
            }
            self.pending_io_trace.push(IoTraceEvent::Joypad(event));
        }
    }

    fn request_interrupt(&mut self, mask: u8, source: InterruptSource) {
        let before = self.interrupt.iflag;
        self.interrupt.request(mask);
        if self.interrupt.iflag != before {
            self.pending_interrupt_trace
                .push(InterruptTraceEvent::Requested {
                    source,
                    iflag: self.interrupt.iflag,
                });
        }
    }

    fn interrupt_source_from_mask(&self, mask: u8) -> InterruptSource {
        match mask {
            INT_VBLANK => InterruptSource::Vblank,
            INT_LCD_STAT => InterruptSource::LcdStat,
            INT_TIMER => InterruptSource::Timer,
            INT_SERIAL => InterruptSource::Serial,
            _ => InterruptSource::Joypad,
        }
    }

    fn dma_blocks_cpu_access(&self, addr: u16) -> bool {
        if !self.dma.active {
            return false;
        }
        match self.mode {
            HardwareMode::Dmg => !(0xFF80..=0xFFFE).contains(&addr),
            HardwareMode::Cgb => false,
        }
    }

    fn read8_for_dma(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cartridge.read_rom(addr),
            0xA000..=0xBFFF => self.cartridge.read_ram(addr),
            _ => self.memory.read8(addr),
        }
    }

    fn dmg_stat_write_causes_spurious_interrupt(&self) -> bool {
        if self.mode != HardwareMode::Dmg || !self.ppu.lcd_enabled() {
            return false;
        }

        self.ppu.stat_coincidence()
            || matches!(
                self.ppu.current_mode(),
                ppu::PpuMode::HBlank | ppu::PpuMode::VBlank | ppu::PpuMode::OamSearch
            )
    }

    fn tick_dma(&mut self, cycles: u32, trace: &mut Vec<DmaTraceEvent>) {
        if !self.dma.active {
            return;
        }

        let mut remaining = cycles;
        if self.dma.start_delay_cycles > 0 {
            let delay_step = remaining.min(u32::from(self.dma.start_delay_cycles));
            self.dma.start_delay_cycles -= delay_step as u8;
            remaining -= delay_step;
            if remaining == 0 {
                return;
            }
        }

        self.dma.cycle_accum = self.dma.cycle_accum.saturating_add(remaining as u8);
        while self.dma.cycle_accum >= 4 && self.dma.bytes_copied < 0xA0 {
            let index = self.dma.bytes_copied;
            let value = self.read8_for_dma(self.dma.source.wrapping_add(index));
            self.memory.write8(0xFE00u16 + index, value);
            self.dma.bytes_copied += 1;
            self.dma.cycle_accum -= 4;
        }

        if self.dma.bytes_copied >= 0xA0 {
            let source = self.dma.source;
            self.dma.active = false;
            self.dma.bytes_copied = 0;
            self.dma.cycle_accum = 0;
            self.dma.start_delay_cycles = 0;
            trace.push(DmaTraceEvent::OamDmaComplete {
                source,
                bytes: 0xA0,
            });
        }
    }

    fn cycles_until_next_dma_event(&self) -> u32 {
        if !self.dma.active {
            return u32::MAX;
        }
        if self.dma.start_delay_cycles > 0 {
            return u32::from(self.dma.start_delay_cycles);
        }
        let pending = 4u8.saturating_sub(self.dma.cycle_accum);
        u32::from(pending.max(1))
    }

    fn hdma_source_from_regs(&self) -> u16 {
        ((self.dma.hdma1 as u16) << 8) | u16::from(self.dma.hdma2 & 0xF0)
    }

    fn hdma_dest_from_regs(&self) -> u16 {
        0x8000 | (((self.dma.hdma3 as u16 & 0x1F) << 8) | u16::from(self.dma.hdma4 & 0xF0))
    }

    fn sync_hdma_registers(&mut self) {
        let source = self.dma.hdma_source;
        let dest_offset = self.dma.hdma_dest.wrapping_sub(0x8000);
        self.dma.hdma1 = (source >> 8) as u8;
        self.dma.hdma2 = (source as u8) & 0xF0;
        self.dma.hdma3 = ((dest_offset >> 8) as u8) & 0x1F;
        self.dma.hdma4 = (dest_offset as u8) & 0xF0;
        self.dma.hdma5 = if self.dma.hdma_active {
            self.dma.hdma_blocks_remaining.saturating_sub(1) & 0x7F
        } else if self.dma.hdma_blocks_remaining > 0 {
            0x80 | (self.dma.hdma_blocks_remaining.saturating_sub(1) & 0x7F)
        } else {
            0xFF
        };
        self.memory.write8(0xFF51, self.dma.hdma1);
        self.memory.write8(0xFF52, self.dma.hdma2);
        self.memory.write8(0xFF53, self.dma.hdma3);
        self.memory.write8(0xFF54, self.dma.hdma4);
        self.memory.write8(0xFF55, self.dma.hdma5);
    }

    fn transfer_hdma_block(&mut self, hblank_mode: bool, trace: &mut Vec<DmaTraceEvent>) {
        if !self.dma.hdma_active || self.dma.hdma_blocks_remaining == 0 {
            return;
        }

        let source = self.dma.hdma_source;
        let dest = self.dma.hdma_dest;
        let block_index = self
            .dma
            .hdma_total_blocks
            .saturating_sub(self.dma.hdma_blocks_remaining);
        for offset in 0..0x10u16 {
            let value = self.read8_for_dma(source.wrapping_add(offset));
            let vram_addr = 0x8000 | (dest.wrapping_sub(0x8000).wrapping_add(offset) & 0x1FFF);
            self.memory.write8(vram_addr, value);
        }

        self.dma.hdma_source = source.wrapping_add(0x10);
        self.dma.hdma_dest = 0x8000 | (dest.wrapping_sub(0x8000).wrapping_add(0x10) & 0x1FF0);
        self.dma.hdma_blocks_remaining = self.dma.hdma_blocks_remaining.saturating_sub(1);
        let remaining_blocks = self.dma.hdma_blocks_remaining;
        trace.push(DmaTraceEvent::HdmaBlock {
            source,
            dest,
            block_index,
            remaining_blocks,
            hblank_mode,
            stall_cycles: self.hdma_block_stall_cycles(),
        });

        if self.dma.hdma_blocks_remaining == 0 {
            let blocks = self.dma.hdma_total_blocks;
            self.dma.hdma_active = false;
            self.dma.hdma_hblank_mode = false;
            trace.push(DmaTraceEvent::HdmaComplete {
                source: self.dma.hdma_source,
                dest: self.dma.hdma_dest,
                blocks,
                hblank_mode,
            });
        }

        self.sync_hdma_registers();
    }

    fn tick_hdma_hblank(&mut self, trace: &mut Vec<DmaTraceEvent>) -> u32 {
        if self.dma.hdma_active && self.dma.hdma_hblank_mode {
            if self.cpu.halted {
                trace.push(DmaTraceEvent::HdmaDeferred {
                    remaining_blocks: self.dma.hdma_blocks_remaining,
                    ly: self.ppu.ly,
                    reason: HdmaDeferredReason::CpuHalted,
                });
                return 0;
            }
            self.transfer_hdma_block(true, trace);
            return self.hdma_block_stall_cycles();
        }
        0
    }

    fn write_hdma_control(&mut self, value: u8) {
        if self.mode != HardwareMode::Cgb {
            self.dma.hdma5 = 0xFF;
            self.memory.write8(0xFF55, 0xFF);
            return;
        }

        if self.dma.hdma_active && self.dma.hdma_hblank_mode {
            if value & 0x80 == 0 {
                let remaining_blocks = self.dma.hdma_blocks_remaining;
                self.dma.hdma_active = false;
                self.dma.hdma_hblank_mode = false;
                self.pending_dma_trace.push(DmaTraceEvent::HdmaCancel {
                    source: self.dma.hdma_source,
                    dest: self.dma.hdma_dest,
                    remaining_blocks,
                });
                self.sync_hdma_registers();
                return;
            }
            self.pending_dma_trace
                .push(DmaTraceEvent::HdmaWriteIgnored {
                    value,
                    remaining_blocks: self.dma.hdma_blocks_remaining,
                });
            self.sync_hdma_registers();
            return;
        }

        self.dma.hdma_source = self.hdma_source_from_regs();
        self.dma.hdma_dest = self.hdma_dest_from_regs();
        self.dma.hdma_blocks_remaining = (value & 0x7F).wrapping_add(1);
        self.dma.hdma_total_blocks = self.dma.hdma_blocks_remaining;
        self.dma.hdma_hblank_mode = value & 0x80 != 0;
        self.dma.hdma_active = true;
        self.pending_dma_trace.push(DmaTraceEvent::HdmaStart {
            source: self.dma.hdma_source,
            dest: self.dma.hdma_dest,
            blocks: self.dma.hdma_blocks_remaining,
            hblank_mode: self.dma.hdma_hblank_mode,
        });
        self.sync_hdma_registers();

        if !self.dma.hdma_hblank_mode {
            let mut trace = std::mem::take(&mut self.pending_dma_trace);
            let gdma_blocks = self.dma.hdma_blocks_remaining;
            let stall_cycles =
                u32::from(gdma_blocks).saturating_mul(self.hdma_block_stall_cycles());
            trace.push(DmaTraceEvent::GdmaStallEstimate {
                blocks: gdma_blocks,
                stall_cycles,
            });
            while self.dma.hdma_active {
                self.transfer_hdma_block(false, &mut trace);
            }
            self.pending_cpu_stall_cycles =
                self.pending_cpu_stall_cycles.saturating_add(stall_cycles);
            self.pending_dma_trace = trace;
        }
    }

    fn write_r8_cb(&mut self, index: u8, value: u8) {
        self.write_r8(index, value);
    }

    fn cb_rlc(&mut self, value: u8) -> u8 {
        let carry = (value & 0x80) != 0;
        let result = (value << 1) | if carry { 1 } else { 0 };
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn cb_rrc(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | if carry { 0x80 } else { 0 };
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn cb_rl(&mut self, value: u8) -> u8 {
        let carry_in = if self.cpu.f.c() { 1 } else { 0 };
        let carry = (value & 0x80) != 0;
        let result = (value << 1) | carry_in;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn cb_rr(&mut self, value: u8) -> u8 {
        let carry_in = if self.cpu.f.c() { 0x80 } else { 0 };
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | carry_in;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn cb_sla(&mut self, value: u8) -> u8 {
        let carry = (value & 0x80) != 0;
        let result = value << 1;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn cb_sra(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | (value & 0x80);
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn cb_swap(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(4);
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(false);
        result
    }

    fn cb_srl(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = value >> 1;
        self.cpu.f.set_z(result == 0);
        self.cpu.f.set_n(false);
        self.cpu.f.set_h(false);
        self.cpu.f.set_c(carry);
        result
    }

    fn execute_cb_opcode(&mut self, opcode: u8) -> u32 {
        let target = opcode & 0x07;
        let group = opcode >> 6;
        let y = (opcode >> 3) & 0x07;
        match group {
            0 => {
                let value = self.read_r8(target);
                let result = match y {
                    0 => self.cb_rlc(value),
                    1 => self.cb_rrc(value),
                    2 => self.cb_rl(value),
                    3 => self.cb_rr(value),
                    4 => self.cb_sla(value),
                    5 => self.cb_sra(value),
                    6 => self.cb_swap(value),
                    _ => self.cb_srl(value),
                };
                self.write_r8_cb(target, result);
                if target == 6 {
                    16
                } else {
                    8
                }
            }
            1 => {
                let value = self.read_r8(target);
                let bit = (value >> y) & 1;
                self.cpu.f.set_z(bit == 0);
                self.cpu.f.set_n(false);
                self.cpu.f.set_h(true);
                if target == 6 {
                    12
                } else {
                    8
                }
            }
            2 => {
                let value = self.read_r8(target) & !(1 << y);
                self.write_r8_cb(target, value);
                if target == 6 {
                    16
                } else {
                    8
                }
            }
            _ => {
                let value = self.read_r8(target) | (1 << y);
                self.write_r8_cb(target, value);
                if target == 6 {
                    16
                } else {
                    8
                }
            }
        }
    }

    fn execute_opcode(&mut self, opcode: u8) -> Result<u32, error::CoreError> {
        if (0x40..=0x7F).contains(&opcode) && opcode != 0x76 {
            let dst = (opcode >> 3) & 0x07;
            let src = opcode & 0x07;
            let value = self.read_r8(src);
            self.write_r8(dst, value);
            return Ok(if dst == 6 || src == 6 { 8 } else { 4 });
        }
        if (0x80..=0x87).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.add_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0x88..=0x8F).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.adc_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0x90..=0x97).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.sub_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0x98..=0x9F).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.sbc_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0xA0..=0xA7).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.and_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0xA8..=0xAF).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.xor_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0xB0..=0xB7).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.or_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        if (0xB8..=0xBF).contains(&opcode) {
            let v = self.read_r8(opcode & 0x07);
            self.cp_a(v);
            return Ok(if opcode & 0x07 == 6 { 8 } else { 4 });
        }
        match opcode {
            0x00 => Ok(4),
            0x01 => {
                let value = self.read16_imm();
                self.set_bc(value);
                Ok(12)
            }
            0x02 => {
                self.write8(self.ld_bc(), self.cpu.a);
                Ok(8)
            }
            0x03 => {
                let value = Self::inc16(self.ld_bc());
                self.set_bc(value);
                Ok(8)
            }
            0x04 => {
                self.cpu.b = self.inc8(self.cpu.b);
                Ok(4)
            }
            0x05 => {
                self.cpu.b = self.dec8(self.cpu.b);
                Ok(4)
            }
            0x06 => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.b = value;
                Ok(8)
            }
            0x07 => {
                self.rlca();
                Ok(4)
            }
            0x0A => {
                self.cpu.a = self.read8_timed(self.ld_bc(), 4);
                Ok(8)
            }
            0x09 => {
                self.add_hl(self.ld_bc());
                Ok(8)
            }
            0x0B => {
                let value = Self::dec16(self.ld_bc());
                self.set_bc(value);
                Ok(8)
            }
            0x0C => {
                self.cpu.c = self.inc8(self.cpu.c);
                Ok(4)
            }
            0x0D => {
                self.cpu.c = self.dec8(self.cpu.c);
                Ok(4)
            }
            0x0E => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.c = value;
                Ok(8)
            }
            0x0F => {
                self.rrca();
                Ok(4)
            }
            0x10 => {
                let _stop_padding = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                if self.mode == HardwareMode::Cgb && self.cgb_speed_switch_armed {
                    self.perform_speed_switch();
                    Ok(4)
                } else {
                    self.cpu.halted = true;
                    Ok(4)
                }
            }
            0x08 => {
                let addr = self.read16_imm();
                let sp = self.cpu.sp;
                self.write8(addr, sp as u8);
                self.write8(addr.wrapping_add(1), (sp >> 8) as u8);
                Ok(20)
            }
            0x11 => {
                let value = self.read16_imm();
                self.set_de(value);
                Ok(12)
            }
            0x12 => {
                self.write8(self.ld_de(), self.cpu.a);
                Ok(8)
            }
            0x13 => {
                let value = Self::inc16(self.ld_de());
                self.set_de(value);
                Ok(8)
            }
            0x19 => {
                self.add_hl(self.ld_de());
                Ok(8)
            }
            0x1A => {
                self.cpu.a = self.read8_timed(self.ld_de(), 4);
                Ok(8)
            }
            0x1B => {
                let value = Self::dec16(self.ld_de());
                self.set_de(value);
                Ok(8)
            }
            0x14 => {
                self.cpu.d = self.inc8(self.cpu.d);
                Ok(4)
            }
            0x15 => {
                self.cpu.d = self.dec8(self.cpu.d);
                Ok(4)
            }
            0x16 => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.d = value;
                Ok(8)
            }
            0x17 => {
                self.rla();
                Ok(4)
            }
            0x18 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.jr(offset);
                Ok(12)
            }
            0x1C => {
                self.cpu.e = self.inc8(self.cpu.e);
                Ok(4)
            }
            0x1D => {
                self.cpu.e = self.dec8(self.cpu.e);
                Ok(4)
            }
            0x1E => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.e = value;
                Ok(8)
            }
            0x1F => {
                let carry = if self.cpu.f.c() { 0x80 } else { 0 };
                let new_carry = (self.cpu.a & 0x01) != 0;
                self.cpu.a = (self.cpu.a >> 1) | carry;
                self.cpu.f.set_z(false);
                self.cpu.f.set_n(false);
                self.cpu.f.set_h(false);
                self.cpu.f.set_c(new_carry);
                Ok(4)
            }
            0x20 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                if !self.cpu.f.z() {
                    self.jr(offset);
                    Ok(12)
                } else {
                    Ok(8)
                }
            }
            0x21 => {
                let value = self.read16_imm();
                self.cpu.set_hl(value);
                Ok(12)
            }
            0x23 => {
                let value = Self::inc16(self.cpu.hl());
                self.cpu.set_hl(value);
                Ok(8)
            }
            0x22 => {
                let addr = self.cpu.hl();
                self.write8(addr, self.cpu.a);
                self.cpu.set_hl(addr.wrapping_add(1));
                Ok(8)
            }
            0x24 => {
                self.cpu.h = self.inc8(self.cpu.h);
                Ok(4)
            }
            0x25 => {
                self.cpu.h = self.dec8(self.cpu.h);
                Ok(4)
            }
            0x26 => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.h = value;
                Ok(8)
            }
            0x27 => {
                self.daa();
                Ok(4)
            }
            0x28 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                if self.cpu.f.z() {
                    self.jr(offset);
                    Ok(12)
                } else {
                    Ok(8)
                }
            }
            0x2A => {
                let addr = self.cpu.hl();
                self.cpu.a = self.read8_timed(addr, 4);
                self.cpu.set_hl(addr.wrapping_add(1));
                Ok(8)
            }
            0x29 => {
                self.add_hl(self.cpu.hl());
                Ok(8)
            }
            0x2B => {
                let value = Self::dec16(self.cpu.hl());
                self.cpu.set_hl(value);
                Ok(8)
            }
            0x2C => {
                self.cpu.l = self.inc8(self.cpu.l);
                Ok(4)
            }
            0x2D => {
                self.cpu.l = self.dec8(self.cpu.l);
                Ok(4)
            }
            0x2E => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.l = value;
                Ok(8)
            }
            0x2F => {
                self.cpu.a ^= 0xFF;
                self.cpu.f.set_n(true);
                self.cpu.f.set_h(true);
                Ok(4)
            }
            0x30 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                if !self.cpu.f.c() {
                    self.jr(offset);
                    Ok(12)
                } else {
                    Ok(8)
                }
            }
            0x31 => {
                self.cpu.sp = self.read16_imm();
                Ok(12)
            }
            0x33 => {
                self.cpu.sp = Self::inc16(self.cpu.sp);
                Ok(8)
            }
            0x32 => {
                let addr = self.cpu.hl();
                self.write8(addr, self.cpu.a);
                self.cpu.set_hl(addr.wrapping_sub(1));
                Ok(8)
            }
            0x39 => {
                self.add_hl(self.cpu.sp);
                Ok(8)
            }
            0x3A => {
                let addr = self.cpu.hl();
                self.cpu.a = self.read8_timed(addr, 4);
                self.cpu.set_hl(addr.wrapping_sub(1));
                Ok(8)
            }
            0x3B => {
                self.cpu.sp = Self::dec16(self.cpu.sp);
                Ok(8)
            }
            0x34 => {
                let addr = self.cpu.hl();
                let value = self.read8(addr);
                let result = self.inc8(value);
                self.write8(addr, result);
                Ok(12)
            }
            0x35 => {
                let addr = self.cpu.hl();
                let value = self.read8(addr);
                let result = self.dec8(value);
                self.write8(addr, result);
                Ok(12)
            }
            0x36 => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.write8(self.cpu.hl(), value);
                Ok(12)
            }
            0x37 => {
                self.cpu.f.set_n(false);
                self.cpu.f.set_h(false);
                self.cpu.f.set_c(true);
                Ok(4)
            }
            0x38 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                if self.cpu.f.c() {
                    self.jr(offset);
                    Ok(12)
                } else {
                    Ok(8)
                }
            }
            0x3C => {
                self.cpu.a = self.inc8(self.cpu.a);
                Ok(4)
            }
            0x3D => {
                self.cpu.a = self.dec8(self.cpu.a);
                Ok(4)
            }
            0x3E => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.a = value;
                Ok(8)
            }
            0x3F => {
                let c = self.cpu.f.c();
                self.cpu.f.set_n(false);
                self.cpu.f.set_h(false);
                self.cpu.f.set_c(!c);
                Ok(4)
            }
            0x40 => Ok(4),
            0x41 => {
                self.cpu.b = self.cpu.c;
                Ok(4)
            }
            0x42 => {
                self.cpu.b = self.cpu.d;
                Ok(4)
            }
            0x43 => {
                self.cpu.b = self.cpu.e;
                Ok(4)
            }
            0x47 => {
                self.cpu.b = self.cpu.a;
                Ok(4)
            }
            0x48 => {
                self.cpu.c = self.cpu.b;
                Ok(4)
            }
            0x49 => Ok(4),
            0x4A => {
                self.cpu.c = self.cpu.d;
                Ok(4)
            }
            0x4B => {
                self.cpu.c = self.cpu.e;
                Ok(4)
            }
            0x4C => {
                self.cpu.c = self.cpu.h;
                Ok(4)
            }
            0x4D => {
                self.cpu.c = self.cpu.l;
                Ok(4)
            }
            0x4F => {
                self.cpu.c = self.cpu.a;
                Ok(4)
            }
            0x50 => {
                self.cpu.d = self.cpu.b;
                Ok(4)
            }
            0x51 => {
                self.cpu.d = self.cpu.c;
                Ok(4)
            }
            0x52 => Ok(4),
            0x53 => {
                self.cpu.d = self.cpu.e;
                Ok(4)
            }
            0x54 => {
                self.cpu.d = self.cpu.h;
                Ok(4)
            }
            0x55 => {
                self.cpu.d = self.cpu.l;
                Ok(4)
            }
            0x57 => {
                self.cpu.d = self.cpu.a;
                Ok(4)
            }
            0x58 => {
                self.cpu.e = self.cpu.b;
                Ok(4)
            }
            0x59 => {
                self.cpu.e = self.cpu.c;
                Ok(4)
            }
            0x5A => {
                self.cpu.e = self.cpu.d;
                Ok(4)
            }
            0x5B => Ok(4),
            0x5C => {
                self.cpu.e = self.cpu.h;
                Ok(4)
            }
            0x5D => {
                self.cpu.e = self.cpu.l;
                Ok(4)
            }
            0x5F => {
                self.cpu.e = self.cpu.a;
                Ok(4)
            }
            0x60 => {
                self.cpu.h = self.cpu.b;
                Ok(4)
            }
            0x61 => {
                self.cpu.h = self.cpu.c;
                Ok(4)
            }
            0x62 => {
                self.cpu.h = self.cpu.d;
                Ok(4)
            }
            0x63 => {
                self.cpu.h = self.cpu.e;
                Ok(4)
            }
            0x64 => Ok(4),
            0x65 => {
                self.cpu.h = self.cpu.l;
                Ok(4)
            }
            0x67 => {
                self.cpu.h = self.cpu.a;
                Ok(4)
            }
            0x68 => {
                self.cpu.l = self.cpu.b;
                Ok(4)
            }
            0x69 => {
                self.cpu.l = self.cpu.c;
                Ok(4)
            }
            0x6A => {
                self.cpu.l = self.cpu.d;
                Ok(4)
            }
            0x6B => {
                self.cpu.l = self.cpu.e;
                Ok(4)
            }
            0x6C => {
                self.cpu.l = self.cpu.h;
                Ok(4)
            }
            0x6D => Ok(4),
            0x6F => {
                self.cpu.l = self.cpu.a;
                Ok(4)
            }
            0x76 => {
                if !self.cpu.ime && self.interrupt.has_pending() {
                    self.cpu.halt_bug = true;
                    self.cpu.halted = false;
                } else {
                    self.cpu.halted = true;
                }
                Ok(4)
            }
            0x77 => {
                let addr = self.cpu.hl();
                self.write8(addr, self.cpu.a);
                Ok(8)
            }
            0x78 => {
                self.cpu.a = self.cpu.b;
                Ok(4)
            }
            0x79 => {
                self.cpu.a = self.cpu.c;
                Ok(4)
            }
            0x7A => {
                self.cpu.a = self.cpu.d;
                Ok(4)
            }
            0x7B => {
                self.cpu.a = self.cpu.e;
                Ok(4)
            }
            0x7C => {
                self.cpu.a = self.cpu.h;
                Ok(4)
            }
            0x7D => {
                self.cpu.a = self.cpu.l;
                Ok(4)
            }
            0x7E => {
                let addr = self.cpu.hl();
                self.cpu.a = self.read8_timed(addr, 4);
                Ok(8)
            }
            0x80 => {
                self.add_a(self.cpu.b);
                Ok(4)
            }
            0x81 => {
                self.add_a(self.cpu.c);
                Ok(4)
            }
            0x82 => {
                self.add_a(self.cpu.d);
                Ok(4)
            }
            0x83 => {
                self.add_a(self.cpu.e);
                Ok(4)
            }
            0x84 => {
                self.add_a(self.cpu.h);
                Ok(4)
            }
            0x85 => {
                self.add_a(self.cpu.l);
                Ok(4)
            }
            0x86 => {
                let v = self.read8(self.cpu.hl());
                self.add_a(v);
                Ok(8)
            }
            0x87 => {
                self.add_a(self.cpu.a);
                Ok(4)
            }
            0x88 => {
                self.adc_a(self.cpu.b);
                Ok(4)
            }
            0x89 => {
                self.adc_a(self.cpu.c);
                Ok(4)
            }
            0x8A => {
                self.adc_a(self.cpu.d);
                Ok(4)
            }
            0x8B => {
                self.adc_a(self.cpu.e);
                Ok(4)
            }
            0x8C => {
                self.adc_a(self.cpu.h);
                Ok(4)
            }
            0x8D => {
                self.adc_a(self.cpu.l);
                Ok(4)
            }
            0x8E => {
                let v = self.read8(self.cpu.hl());
                self.adc_a(v);
                Ok(8)
            }
            0x8F => {
                self.adc_a(self.cpu.a);
                Ok(4)
            }
            0x90 => {
                self.sub_a(self.cpu.b);
                Ok(4)
            }
            0x91 => {
                self.sub_a(self.cpu.c);
                Ok(4)
            }
            0x92 => {
                self.sub_a(self.cpu.d);
                Ok(4)
            }
            0x93 => {
                self.sub_a(self.cpu.e);
                Ok(4)
            }
            0x94 => {
                self.sub_a(self.cpu.h);
                Ok(4)
            }
            0x95 => {
                self.sub_a(self.cpu.l);
                Ok(4)
            }
            0x96 => {
                let v = self.read8(self.cpu.hl());
                self.sub_a(v);
                Ok(8)
            }
            0x97 => {
                self.sub_a(self.cpu.a);
                Ok(4)
            }
            0x98 => {
                self.sbc_a(self.cpu.b);
                Ok(4)
            }
            0x99 => {
                self.sbc_a(self.cpu.c);
                Ok(4)
            }
            0x9A => {
                self.sbc_a(self.cpu.d);
                Ok(4)
            }
            0x9B => {
                self.sbc_a(self.cpu.e);
                Ok(4)
            }
            0x9C => {
                self.sbc_a(self.cpu.h);
                Ok(4)
            }
            0x9D => {
                self.sbc_a(self.cpu.l);
                Ok(4)
            }
            0x9E => {
                let v = self.read8(self.cpu.hl());
                self.sbc_a(v);
                Ok(8)
            }
            0x9F => {
                self.sbc_a(self.cpu.a);
                Ok(4)
            }
            0xA0 => {
                self.and_a(self.cpu.b);
                Ok(4)
            }
            0xA1 => {
                self.and_a(self.cpu.c);
                Ok(4)
            }
            0xA2 => {
                self.and_a(self.cpu.d);
                Ok(4)
            }
            0xA3 => {
                self.and_a(self.cpu.e);
                Ok(4)
            }
            0xA4 => {
                self.and_a(self.cpu.h);
                Ok(4)
            }
            0xA5 => {
                self.and_a(self.cpu.l);
                Ok(4)
            }
            0xA6 => {
                let v = self.read8(self.cpu.hl());
                self.and_a(v);
                Ok(8)
            }
            0xA7 => {
                self.and_a(self.cpu.a);
                Ok(4)
            }
            0xA8 => {
                self.xor_a(self.cpu.b);
                Ok(4)
            }
            0xA9 => {
                self.xor_a(self.cpu.c);
                Ok(4)
            }
            0xAA => {
                self.xor_a(self.cpu.d);
                Ok(4)
            }
            0xAB => {
                self.xor_a(self.cpu.e);
                Ok(4)
            }
            0xAC => {
                self.xor_a(self.cpu.h);
                Ok(4)
            }
            0xAD => {
                self.xor_a(self.cpu.l);
                Ok(4)
            }
            0xAE => {
                let v = self.read8(self.cpu.hl());
                self.xor_a(v);
                Ok(8)
            }
            0xAF => {
                self.xor_a(self.cpu.a);
                Ok(4)
            }
            0xB0 => {
                self.or_a(self.cpu.b);
                Ok(4)
            }
            0xB1 => {
                self.or_a(self.cpu.c);
                Ok(4)
            }
            0xB2 => {
                self.or_a(self.cpu.d);
                Ok(4)
            }
            0xB3 => {
                self.or_a(self.cpu.e);
                Ok(4)
            }
            0xB4 => {
                self.or_a(self.cpu.h);
                Ok(4)
            }
            0xB5 => {
                self.or_a(self.cpu.l);
                Ok(4)
            }
            0xB6 => {
                let v = self.read8(self.cpu.hl());
                self.or_a(v);
                Ok(8)
            }
            0xB7 => {
                self.or_a(self.cpu.a);
                Ok(4)
            }
            0xB8 => {
                self.cp_a(self.cpu.b);
                Ok(4)
            }
            0xB9 => {
                self.cp_a(self.cpu.c);
                Ok(4)
            }
            0xBA => {
                self.cp_a(self.cpu.d);
                Ok(4)
            }
            0xBB => {
                self.cp_a(self.cpu.e);
                Ok(4)
            }
            0xBC => {
                self.cp_a(self.cpu.h);
                Ok(4)
            }
            0xBD => {
                self.cp_a(self.cpu.l);
                Ok(4)
            }
            0xBE => {
                let v = self.read8(self.cpu.hl());
                self.cp_a(v);
                Ok(8)
            }
            0xBF => {
                self.cp_a(self.cpu.a);
                Ok(4)
            }
            0xCB => {
                let op = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                Ok(self.execute_cb_opcode(op))
            }
            0xC0 => {
                if !self.cpu.f.z() {
                    self.cpu.pc = self.pop16();
                    Ok(20)
                } else {
                    Ok(8)
                }
            }
            0xC1 => {
                let value = self.pop16();
                self.set_bc(value);
                Ok(12)
            }
            0xC2 => {
                let addr = self.read16_imm();
                if !self.cpu.f.z() {
                    self.cpu.pc = addr;
                    Ok(16)
                } else {
                    Ok(12)
                }
            }
            0xC3 => {
                self.cpu.pc = self.read16_imm();
                Ok(16)
            }
            0xC4 => {
                let addr = self.read16_imm();
                if !self.cpu.f.z() {
                    self.push16(self.cpu.pc);
                    self.cpu.pc = addr;
                    Ok(24)
                } else {
                    Ok(12)
                }
            }
            0xC5 => {
                self.push16(self.ld_bc());
                Ok(16)
            }
            0xC7 => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x00;
                Ok(16)
            }
            0xC8 => {
                if self.cpu.f.z() {
                    self.cpu.pc = self.pop16();
                    Ok(20)
                } else {
                    Ok(8)
                }
            }
            0xC9 => {
                self.cpu.pc = self.pop16();
                Ok(16)
            }
            0xCA => {
                let addr = self.read16_imm();
                if self.cpu.f.z() {
                    self.cpu.pc = addr;
                    Ok(16)
                } else {
                    Ok(12)
                }
            }
            0xCC => {
                let addr = self.read16_imm();
                if self.cpu.f.z() {
                    self.push16(self.cpu.pc);
                    self.cpu.pc = addr;
                    Ok(24)
                } else {
                    Ok(12)
                }
            }
            0xC6 => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.add_a(v);
                Ok(8)
            }
            0xCD => {
                let addr = self.read16_imm();
                self.push16(self.cpu.pc);
                self.cpu.pc = addr;
                Ok(24)
            }
            0xCF => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x08;
                Ok(16)
            }
            0xD0 => {
                if !self.cpu.f.c() {
                    self.cpu.pc = self.pop16();
                    Ok(20)
                } else {
                    Ok(8)
                }
            }
            0xD1 => {
                let value = self.pop16();
                self.set_de(value);
                Ok(12)
            }
            0xD2 => {
                let addr = self.read16_imm();
                if !self.cpu.f.c() {
                    self.cpu.pc = addr;
                    Ok(16)
                } else {
                    Ok(12)
                }
            }
            0xD4 => {
                let addr = self.read16_imm();
                if !self.cpu.f.c() {
                    self.push16(self.cpu.pc);
                    self.cpu.pc = addr;
                    Ok(24)
                } else {
                    Ok(12)
                }
            }
            0xD5 => {
                self.push16(self.ld_de());
                Ok(16)
            }
            0xD6 => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.sub_a(v);
                Ok(8)
            }
            0xD7 => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x10;
                Ok(16)
            }
            0xDF => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x18;
                Ok(16)
            }
            0xD8 => {
                if self.cpu.f.c() {
                    self.cpu.pc = self.pop16();
                    Ok(20)
                } else {
                    Ok(8)
                }
            }
            0xDA => {
                let addr = self.read16_imm();
                if self.cpu.f.c() {
                    self.cpu.pc = addr;
                    Ok(16)
                } else {
                    Ok(12)
                }
            }
            0xCE => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.adc_a(v);
                Ok(8)
            }
            0xDC => {
                let addr = self.read16_imm();
                if self.cpu.f.c() {
                    self.push16(self.cpu.pc);
                    self.cpu.pc = addr;
                    Ok(24)
                } else {
                    Ok(12)
                }
            }
            0xDE => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.sbc_a(v);
                Ok(8)
            }
            0xD9 => {
                self.cpu.pc = self.pop16();
                self.cpu.ime = true;
                Ok(16)
            }
            0xE0 => {
                let offset = self.read8(self.cpu.pc) as u16;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.write8(0xFF00 | offset, self.cpu.a);
                Ok(12)
            }
            0xE1 => {
                let value = self.pop16();
                self.cpu.set_hl(value);
                Ok(12)
            }
            0xE2 => {
                self.write8(0xFF00 | self.cpu.c as u16, self.cpu.a);
                Ok(8)
            }
            0xE5 => {
                self.push16(self.cpu.hl());
                Ok(16)
            }
            0xEA => {
                let addr = self.read16_imm();
                self.write8(addr, self.cpu.a);
                Ok(16)
            }
            0xE6 => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.and_a(v);
                Ok(8)
            }
            0xF0 => {
                let offset = self.read8(self.cpu.pc) as u16;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cpu.a = self.read8_timed(0xFF00 | offset, 8);
                Ok(12)
            }
            0xF2 => {
                self.cpu.a = self.read8_timed(0xFF00 | self.cpu.c as u16, 4);
                Ok(8)
            }
            0xE7 => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x20;
                Ok(16)
            }
            0xE9 => {
                self.cpu.pc = self.cpu.hl();
                Ok(4)
            }
            0xEE => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.xor_a(v);
                Ok(8)
            }
            0xEF => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x28;
                Ok(16)
            }
            0xF1 => {
                let value = self.pop16();
                self.cpu.a = (value >> 8) as u8;
                self.cpu.f.0 = (value as u8) & 0xF0;
                Ok(12)
            }
            0xF3 => {
                self.cpu.ime = false;
                self.cpu.ime_enable_delay = 0;
                Ok(4)
            }
            0xE8 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                let result = self.add_signed_to_sp_like_flags(self.cpu.sp, offset);
                self.cpu.sp = result;
                Ok(16)
            }
            0xF4 => {
                let addr = self.read16_imm();
                if !self.cpu.f.c() {
                    self.push16(self.cpu.pc);
                    self.cpu.pc = addr;
                    Ok(24)
                } else {
                    Ok(12)
                }
            }
            0xF6 => {
                let v = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.or_a(v);
                Ok(8)
            }
            0xF8 => {
                let offset = self.read8(self.cpu.pc) as i8;
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                let result = self.add_signed_to_sp_like_flags(self.cpu.sp, offset);
                self.cpu.set_hl(result);
                Ok(12)
            }
            0xF9 => {
                self.cpu.sp = self.cpu.hl();
                Ok(8)
            }
            0xFA => {
                let addr = self.read16_imm();
                self.cpu.a = self.read8_timed(addr, 12);
                Ok(16)
            }
            0xF5 => {
                let af = ((self.cpu.a as u16) << 8) | (self.cpu.f.0 as u16 & 0xF0);
                self.push16(af);
                Ok(16)
            }
            0xF7 => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x30;
                Ok(16)
            }
            0xFB => {
                self.cpu.ime_enable_delay = 2;
                Ok(4)
            }
            0xFF => {
                self.push16(self.cpu.pc);
                self.cpu.pc = 0x38;
                Ok(16)
            }
            0xFE => {
                let value = self.read8(self.cpu.pc);
                self.cpu.pc = self.cpu.pc.wrapping_add(1);
                self.cp_a(value);
                Ok(8)
            }
            _ => Err(error::CoreError::UnsupportedOpcode {
                opcode,
                pc: self.cpu.pc.wrapping_sub(1),
            }),
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::{INT_JOYPAD, INT_SERIAL, INT_TIMER, INT_VBLANK};
    use crate::types::{
        ApuTraceEvent, CgbTraceEvent, DmaTraceEvent, InterruptTraceEvent, IoTraceEvent,
        JoypadTraceEvent, MapperTraceEvent, SerialTraceEvent, TimerTraceEvent,
    };

    fn make_test_rom(program: &[u8]) -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        rom[0x147] = 0x00;
        rom[0x148] = 0x00;
        rom[0x149] = 0x00;
        let start = 0x0100;
        rom[start..start + program.len()].copy_from_slice(program);
        rom
    }

    fn make_machine(program: &[u8]) -> Machine {
        let mut machine = Machine::new();
        machine.load_rom(make_test_rom(program)).unwrap();
        machine
    }

    fn make_cgb_test_rom(program: &[u8], cgb_flag: u8) -> Vec<u8> {
        let mut rom = make_test_rom(program);
        rom[0x143] = cgb_flag;
        rom
    }

    fn make_cgb_machine(program: &[u8]) -> Machine {
        let mut machine = Machine::new();
        machine.load_rom(make_cgb_test_rom(program, 0x80)).unwrap();
        machine
    }

    #[test]
    fn ei_is_applied_after_the_following_instruction() {
        let mut machine = make_machine(&[0xFB, 0x00, 0x00]);
        machine.interrupt.ie = INT_TIMER;
        machine.interrupt.iflag = INT_TIMER;

        machine.step_instruction().unwrap();
        assert!(!machine.cpu.ime);
        assert_eq!(machine.cpu.ime_enable_delay, 1);
        assert_eq!(machine.cpu.pc, 0x0101);

        machine.step_instruction().unwrap();
        assert!(machine.cpu.ime);
        assert_eq!(machine.cpu.pc, 0x0102);

        machine.step_instruction().unwrap();
        assert_eq!(machine.cpu.pc, 0x0050);
        assert_eq!(machine.cpu.sp, 0xFFFC);
        assert_eq!(machine.interrupt.iflag & INT_TIMER, 0);
    }

    #[test]
    fn di_cancels_delayed_ei_enable() {
        let mut machine = make_machine(&[0xFB, 0xF3, 0x00]);
        machine.interrupt.ie = INT_TIMER;
        machine.interrupt.iflag = INT_TIMER;

        machine.step_instruction().unwrap();
        assert_eq!(machine.cpu.ime_enable_delay, 1);

        machine.step_instruction().unwrap();
        assert!(!machine.cpu.ime);
        assert_eq!(machine.cpu.ime_enable_delay, 0);
        assert_eq!(machine.cpu.pc, 0x0102);

        machine.step_instruction().unwrap();
        assert_eq!(machine.cpu.pc, 0x0103);
        assert!(!machine.cpu.ime);
    }

    #[test]
    fn halt_with_pending_interrupt_and_ime_clear_triggers_halt_bug() {
        let mut machine = make_machine(&[0x76, 0x3E, 0x12]);
        machine.interrupt.ie = INT_TIMER;
        machine.interrupt.iflag = INT_TIMER;
        machine.cpu.ime = false;

        machine.step_instruction().unwrap();
        assert!(!machine.cpu.halted);
        assert!(machine.cpu.halt_bug);
        assert_eq!(machine.cpu.pc, 0x0101);

        machine.step_instruction().unwrap();
        assert_eq!(machine.cpu.a, 0x3E);
        assert_eq!(machine.cpu.pc, 0x0102);
        assert!(!machine.cpu.halt_bug);
    }

    #[test]
    fn run_frame_advances_nominal_frame_time_while_lcd_is_disabled() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x00;

        machine.run_frame().unwrap();

        assert_eq!(machine.clocks.frames, 1);
        assert!(machine.clocks.frame_phase_ppu_cycles < machine.ppu.frame_cycles());
        assert_eq!(machine.ppu.ly, 0);
    }

    #[test]
    fn stat_interrupt_is_requested_on_mode_zero_entry() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.stat = 0x08;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 251;
        machine.ppu.stat_interrupt_line = false;
        machine.interrupt.iflag = 0;

        machine.advance_clocks(4, 4);

        assert_eq!(machine.ppu.current_mode(), ppu::PpuMode::HBlank);
        assert_ne!(machine.interrupt.iflag & INT_LCD_STAT, 0);
    }

    #[test]
    fn vram_is_blocked_during_mode_three() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 100;
        machine.memory.write8(0x8000, 0x12);

        assert_eq!(machine.read8(0x8000), 0xFF);
        machine.write8(0x8000, 0x34);
        assert_eq!(machine.memory.read8(0x8000), 0x12);
    }

    #[test]
    fn oam_dma_blocks_cpu_bus_and_copies_over_time() {
        let mut machine = make_machine(&[0x00]);
        for i in 0..0xA0u16 {
            machine.memory.write8(0xC000 + i, i as u8);
        }

        machine.write8(0xFF46, 0xC0);
        assert!(machine.dma.active);
        assert_eq!(machine.read8(0xC000), 0xFF);
        assert_eq!(machine.read8(0xFF80), 0x00);

        machine.advance_clocks(4, 4);
        assert!(machine.dma.active);
        assert_eq!(machine.dma.bytes_copied, 0);
        assert_eq!(machine.memory.read8(0xFE00), 0x00);

        machine.advance_clocks(4 * 160, 4 * 160);
        assert!(!machine.dma.active);
        assert_eq!(machine.memory.read8(0xFE00), 0x00);
        assert_eq!(machine.memory.read8(0xFE9F), 0x9F);
    }

    #[test]
    fn mid_scanline_dma_does_not_change_latched_scanline_render() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x93;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 236;
        machine.ppu.framebuffer.fill(0);
        machine.ppu.bg_color_ids.fill(0);

        machine.memory.write8(0x8010, 0x80);
        machine.memory.write8(0x8011, 0x00);
        machine.memory.write8(0xC000, 16);
        machine.memory.write8(0xC001, 8);
        machine.memory.write8(0xC002, 1);
        machine.memory.write8(0xC003, 0);

        machine.dma.active = true;
        machine.dma.source = 0xC000;
        machine.dma.bytes_copied = 0;
        machine.dma.cycle_accum = 0;
        machine.dma.start_delay_cycles = 0;
        machine.ppu.latch_scanline_state(machine.memory.oam());

        machine.advance_clocks(16, 16);

        assert_eq!(machine.memory.read8(0xFE00), 16);
        assert_eq!(machine.memory.read8(0xFE01), 8);
        assert_eq!(machine.memory.read8(0xFE02), 1);
        assert_eq!(machine.memory.read8(0xFE03), 0);
        assert_eq!(machine.ppu.current_mode(), ppu::PpuMode::HBlank);
        assert_eq!(machine.ppu.framebuffer[0], 0);
    }

    #[test]
    fn writing_matching_lyc_requests_stat_interrupt_immediately() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 7;
        machine.ppu.stat = 0x40;
        machine.interrupt.iflag = 0;

        machine.write8(0xFF45, 7);

        assert_ne!(machine.interrupt.iflag & INT_LCD_STAT, 0);
    }

    #[test]
    fn enabling_hblank_stat_source_while_in_hblank_requests_interrupt() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 300;
        machine.interrupt.iflag = 0;

        machine.write8(0xFF41, 0x08);

        assert_ne!(machine.interrupt.iflag & INT_LCD_STAT, 0);
    }

    #[test]
    fn dmg_stat_write_quirk_requests_interrupt_in_oam() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Dmg;
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 40;
        machine.interrupt.iflag = 0;

        machine.write8(0xFF41, 0x00);

        assert_ne!(machine.interrupt.iflag & INT_LCD_STAT, 0);
    }

    #[test]
    fn dmg_stat_write_quirk_still_updates_stat_sources() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Dmg;
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 40;
        machine.ppu.stat = 0x00;

        machine.write8(0xFF41, 0x40);

        assert_eq!(machine.ppu.stat & 0x78, 0x40);
        assert_eq!(machine.read8(0xFF41) & 0x78, 0x40);
    }

    #[test]
    fn cgb_mode_does_not_apply_dmg_stat_write_quirk() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Cgb;
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 40;
        machine.interrupt.iflag = 0;

        machine.write8(0xFF41, 0x00);

        assert_eq!(machine.interrupt.iflag & INT_LCD_STAT, 0);
    }

    #[test]
    fn enabling_lcdc_with_oam_stat_source_requests_interrupt() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x00;
        machine.ppu.stat = 0x20;
        machine.interrupt.iflag = 0;

        machine.write8(0xFF40, 0x91);

        assert_ne!(machine.interrupt.iflag & INT_LCD_STAT, 0);
        assert_eq!(machine.ppu.current_mode(), ppu::PpuMode::OamSearch);
    }

    #[test]
    fn lcdc_write_resyncs_nominal_frame_phase_to_current_ppu_position() {
        let mut machine = make_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 37;
        machine.ppu.mode_cycles = 123;
        machine.clocks.frame_phase_ppu_cycles = 17;

        machine.write8(0xFF40, 0xB1);

        assert_eq!(
            machine.clocks.frame_phase_ppu_cycles,
            u32::from(machine.ppu.ly) * 456 + machine.ppu.mode_cycles
        );
    }

    #[test]
    fn ldh_ff41_samples_stat_near_the_read_microstep() {
        let mut machine = make_machine(&[0xF0, 0x41]);
        machine.interrupt.ie = 0;
        machine.interrupt.iflag = 0;
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 143;
        machine.ppu.mode_cycles = 448;

        machine.step_instruction().unwrap();

        assert_eq!(machine.cpu.a, 0x81);
        assert_eq!(machine.interrupt.iflag & INT_VBLANK, INT_VBLANK);
    }

    #[test]
    fn finish_step_collects_oam_dma_trace_edges() {
        let mut machine = make_machine(&[0x00]);
        for i in 0..0xA0u16 {
            machine.memory.write8(0xC000 + i, i as u8);
        }

        machine.write8(0xFF46, 0xC0);
        let step = machine.finish_step(4 + (4 * 160));

        assert!(step.dma_trace.iter().any(
            |event| matches!(event, DmaTraceEvent::OamDmaStart { source } if *source == 0xC000)
        ));
        assert!(step.dma_trace.iter().any(|event| matches!(event, DmaTraceEvent::OamDmaComplete { source, bytes } if *source == 0xC000 && *bytes == 0xA0)));
        assert_eq!(machine.memory.read8(0xFE00), 0x00);
        assert_eq!(machine.memory.read8(0xFE9F), 0x9F);
    }

    #[test]
    fn cgb_gdma_transfers_immediately_and_emits_trace() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Cgb;
        for i in 0..0x20u16 {
            machine.memory.write8(0xC000 + i, (i as u8).wrapping_add(1));
        }

        machine.write8(0xFF51, 0xC0);
        machine.write8(0xFF52, 0x00);
        machine.write8(0xFF53, 0x00);
        machine.write8(0xFF54, 0x00);
        machine.write8(0xFF55, 0x01);

        assert!(!machine.dma.hdma_active);
        assert_eq!(machine.memory.read8(0x8000), 0x01);
        assert_eq!(machine.memory.read8(0x801F), 0x20);
        assert_eq!(machine.read8(0xFF55), 0xFF);
        assert!(machine.pending_dma_trace.iter().any(|event| matches!(
            event,
            DmaTraceEvent::HdmaStart {
                hblank_mode: false,
                blocks: 2,
                ..
            }
        )));
        assert!(machine.pending_dma_trace.iter().any(|event| {
            matches!(
                event,
                DmaTraceEvent::HdmaBlock {
                    hblank_mode: false,
                    block_index: 0,
                    remaining_blocks: 1,
                    stall_cycles,
                    ..
                } if *stall_cycles == machine.hdma_block_stall_cycles()
            )
        }));
        assert!(machine.pending_dma_trace.iter().any(|event| matches!(
            event,
            DmaTraceEvent::HdmaComplete {
                hblank_mode: false,
                blocks: 2,
                ..
            }
        )));
    }

    #[test]
    fn hblank_hdma_transfers_one_block_on_hblank_entry() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Cgb;
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 251;
        for i in 0..0x20u16 {
            machine
                .memory
                .write8(0xC000 + i, (0x80u8).wrapping_add(i as u8));
        }

        machine.write8(0xFF51, 0xC0);
        machine.write8(0xFF52, 0x00);
        machine.write8(0xFF53, 0x00);
        machine.write8(0xFF54, 0x00);
        machine.write8(0xFF55, 0x81);

        assert!(machine.dma.hdma_active);
        assert_eq!(machine.dma.hdma_blocks_remaining, 2);
        assert_eq!(machine.memory.read8(0x8000), 0x00);

        let (_, _, dma_trace, _, _, _) = machine.advance_clocks(4, 4);

        assert_eq!(machine.memory.read8(0x8000), 0x80);
        assert_eq!(machine.memory.read8(0x800F), 0x8F);
        assert!(machine.dma.hdma_active);
        assert_eq!(machine.dma.hdma_blocks_remaining, 1);
        assert_eq!(machine.read8(0xFF55), 0x00);
        assert!(dma_trace.iter().any(|event| {
            matches!(
                event,
                DmaTraceEvent::HdmaBlock {
                    hblank_mode: true,
                    block_index: 0,
                    remaining_blocks: 1,
                    stall_cycles,
                    ..
                } if *stall_cycles == machine.hdma_block_stall_cycles()
            )
        }));
    }

    #[test]
    fn gdma_adds_cpu_stall_cycles_to_finished_step() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Cgb;
        machine.write8(0xFF51, 0xC0);
        machine.write8(0xFF52, 0x00);
        machine.write8(0xFF53, 0x00);
        machine.write8(0xFF54, 0x00);
        machine.write8(0xFF55, 0x01);

        let step = machine.finish_step(4);

        assert_eq!(step.cycles, 4 + (2 * CGB_DMA_BLOCK_STALL_CYCLES));
        assert!(step
            .dma_trace
            .iter()
            .any(|event| matches!(event, DmaTraceEvent::GdmaStallEstimate { blocks: 2, stall_cycles } if *stall_cycles == 2 * CGB_DMA_BLOCK_STALL_CYCLES)));
    }

    #[test]
    fn active_hblank_hdma_write_with_bit7_set_is_ignored() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Cgb;
        machine.write8(0xFF51, 0xC0);
        machine.write8(0xFF52, 0x00);
        machine.write8(0xFF53, 0x00);
        machine.write8(0xFF54, 0x00);
        machine.write8(0xFF55, 0x81);

        machine.write8(0xFF55, 0x82);

        assert!(machine.dma.hdma_active);
        assert_eq!(machine.dma.hdma_blocks_remaining, 2);
        assert!(machine.pending_dma_trace.iter().any(|event| matches!(
            event,
            DmaTraceEvent::HdmaWriteIgnored {
                value: 0x82,
                remaining_blocks: 2
            }
        )));
    }

    #[test]
    fn hblank_hdma_defers_when_cpu_is_halted() {
        let mut machine = make_machine(&[0x00]);
        machine.mode = HardwareMode::Cgb;
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 251;
        machine.cpu.halted = true;
        machine.write8(0xFF51, 0xC0);
        machine.write8(0xFF52, 0x00);
        machine.write8(0xFF53, 0x00);
        machine.write8(0xFF54, 0x00);
        machine.write8(0xFF55, 0x80);

        let (_, _, dma_trace, _, _, _) = machine.advance_clocks(4, 4);

        assert!(machine.dma.hdma_active);
        assert_eq!(machine.dma.hdma_blocks_remaining, 1);
        assert!(dma_trace.iter().any(|event| matches!(
            event,
            DmaTraceEvent::HdmaDeferred {
                remaining_blocks: 1,
                ly: 0,
                reason: HdmaDeferredReason::CpuHalted
            }
        )));
    }

    #[test]
    fn div_reset_edge_can_increment_tima() {
        let mut machine = make_machine(&[0x00]);
        machine.timer.tac = 0x05;
        machine.timer.tima = 0x0F;
        machine.timer.div = 0;
        machine.write8(0xFF07, 0x05);
        machine.advance_clocks(8, 8);
        let before = machine.timer.tima;

        machine.write8(0xFF04, 0x00);
        let step = machine.finish_step(4);

        assert!(machine.timer.tima >= before);
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Timer(TimerTraceEvent::DivResetEdge { .. })
        )));
    }

    #[test]
    fn serial_internal_clock_completes_and_requests_interrupt() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF01, 0xA5);
        machine.write8(0xFF02, 0x81);

        let step = machine.finish_step(8 * 512);

        assert_eq!(machine.interrupt.iflag & INT_SERIAL, INT_SERIAL);
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Serial(SerialTraceEvent::TransferStart {
                sb: 0xA5,
                internal_clock: true,
                ..
            })
        )));
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Serial(SerialTraceEvent::TransferComplete {
                internal_clock: true,
                ..
            })
        )));
        assert!(step.interrupt_trace.iter().any(|event| matches!(
            event,
            InterruptTraceEvent::Requested {
                source: crate::types::InterruptSource::Serial,
                ..
            }
        )));
    }

    #[test]
    fn serial_link_exchange_completes_between_internal_and_external_peers() {
        let mut left = make_machine(&[0x00]);
        let mut right = make_machine(&[0x00]);
        left.write8(0xFF01, 0xA5);
        left.write8(0xFF02, 0x81);
        right.write8(0xFF01, 0x3C);
        right.write8(0xFF02, 0x80);

        let result = left.exchange_serial_with_peer(&mut right);

        assert!(result.completed);
        assert_eq!(left.serial.sb, 0x3C);
        assert_eq!(right.serial.sb, 0xA5);
        assert_eq!(left.interrupt.iflag & INT_SERIAL, INT_SERIAL);
        assert_eq!(right.interrupt.iflag & INT_SERIAL, INT_SERIAL);
        assert!(result.self_io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Serial(SerialTraceEvent::TransferComplete {
                sb: 0x3C,
                internal_clock: true,
                ..
            })
        )));
        assert!(result.peer_io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Serial(SerialTraceEvent::TransferComplete {
                sb: 0xA5,
                internal_clock: false,
                ..
            })
        )));
        assert!(result.self_interrupt_trace.iter().any(|event| matches!(
            event,
            InterruptTraceEvent::Requested {
                source: crate::types::InterruptSource::Serial,
                ..
            }
        )));
        assert!(result.peer_interrupt_trace.iter().any(|event| matches!(
            event,
            InterruptTraceEvent::Requested {
                source: crate::types::InterruptSource::Serial,
                ..
            }
        )));
    }

    #[test]
    fn joypad_press_requests_interrupt_when_selected() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF00, 0x20);
        machine.set_joypad_mask(0x01);
        let step = machine.finish_step(4);

        assert_eq!(machine.interrupt.iflag & INT_JOYPAD, INT_JOYPAD);
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Joypad(JoypadTraceEvent::InputEdge { new_mask: 0x01, .. })
        )));
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Joypad(JoypadTraceEvent::InterruptEdge { .. })
        )));
        assert!(step.interrupt_trace.iter().any(|event| matches!(
            event,
            InterruptTraceEvent::Requested {
                source: crate::types::InterruptSource::Joypad,
                ..
            }
        )));
    }

    #[test]
    fn joypad_action_button_requests_interrupt_when_selected() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF00, 0x10);
        machine.set_joypad_mask(0x80);
        let step = machine.finish_step(4);

        assert_eq!(machine.interrupt.iflag & INT_JOYPAD, INT_JOYPAD);
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Joypad(JoypadTraceEvent::InputEdge { new_mask: 0x80, .. })
        )));
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Joypad(JoypadTraceEvent::InterruptEdge { .. })
        )));
    }

    #[test]
    fn reading_ff00_emits_joypad_read_trace() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF00, 0x20);
        machine.set_joypad_mask(0x01);

        let value = machine.read8(0xFF00);
        let step = machine.finish_step(4);

        assert_eq!(value & 0x0F, 0x0E);
        assert!(step.io_trace.iter().any(|event| matches!(
            event,
            IoTraceEvent::Joypad(JoypadTraceEvent::Read {
                p1,
                select,
                mask
            }) if *p1 == value && *select == 0x20 && *mask == 0x01
        )));
    }

    #[test]
    fn pending_interrupt_with_ime_clear_emits_blocked_trace() {
        let mut machine = make_machine(&[0x00]);
        machine.interrupt.ie = INT_TIMER;
        machine.interrupt.iflag = INT_TIMER;
        machine.cpu.ime = false;

        let step = machine.step_instruction().unwrap();

        assert!(step.interrupt_trace.iter().any(|event| matches!(event, InterruptTraceEvent::PendingBlocked { pending_mask, ime: false, .. } if *pending_mask == INT_TIMER)));
    }

    #[test]
    fn special_mapper_control_write_emits_trace() {
        let mut rom = vec![0u8; 0x8000];
        rom[0x147] = 0x20;
        let mut machine = Machine::new();
        machine.load_rom(rom).unwrap();

        machine.write8(0x4000, 0x02);
        let step = machine.finish_step(0);

        assert!(step.mapper_trace.iter().any(|event| matches!(
            event,
            MapperTraceEvent::ControlWrite {
                mapper: crate::types::MapperKind::Mbc6,
                addr: 0x4000,
                value: 0x02,
                ..
            }
        )));
    }

    #[test]
    fn apu_master_toggle_and_trigger_emit_trace() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x00);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF14, 0x80);
        let step = machine.finish_step(0);

        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::MasterToggle { enabled: true, .. })));
        assert!(step.apu_trace.iter().any(|event| matches!(
            event,
            ApuTraceEvent::ChannelTrigger {
                channel: 1,
                reg: 0xFF14,
                value: 0x80
            }
        )));
    }

    #[test]
    fn apu_frame_sequencer_step_is_observable() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);

        let step = machine.finish_step(8192);

        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::FrameSequencerStep { step: 1 })));
    }

    #[test]
    fn apu_mixer_write_and_mixed_output_are_observable() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF24, 0x77);
        machine.write8(0xFF25, 0x11);
        machine.write8(0xFF11, 0xC0);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF13, 0xFF);
        machine.write8(0xFF14, 0x80);

        let step = machine.finish_step(2048);

        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::MixerControlWrite { nr50: 0x77, .. })));
        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::MixedOutput { .. })));
    }

    #[test]
    fn apu_length_expiry_is_observable() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF11, 0xFF);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF14, 0xC0 | 0x80);

        let step = machine.finish_step(16384);

        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::ChannelLengthExpired { channel: 1 })));
    }

    #[test]
    fn apu_sweep_step_is_observable() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF10, 0x11);
        machine.write8(0xFF11, 0x80);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF13, 0x80);
        machine.write8(0xFF14, 0x83 | 0x80);

        let step = machine.finish_step(16384);

        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::ChannelSweepStep { .. })));
    }

    #[test]
    fn apu_pcm_frames_reach_machine_audio_buffer() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF24, 0x77);
        machine.write8(0xFF25, 0x11);
        machine.write8(0xFF11, 0xC0);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF13, 0xFF);
        machine.write8(0xFF14, 0x80);

        let step = machine.finish_step(512);

        assert!(step
            .apu_trace
            .iter()
            .any(|event| matches!(event, ApuTraceEvent::PcmFramesBuffered { .. })));
        assert!(machine.audio_frames_available() > 0);
        let drained = machine.drain_audio_frames_interleaved_i16(machine.audio_frames_available());
        assert!(!drained.is_empty());
    }

    #[test]
    fn run_audio_slice_generates_pcm_before_full_frame() {
        let mut machine = make_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF24, 0x77);
        machine.write8(0xFF25, 0x11);
        machine.write8(0xFF11, 0xC0);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF13, 0xFF);
        machine.write8(0xFF14, 0x80);

        let generated = machine.run_audio_slice(1).expect("audio slice");

        assert!(generated > 0);
        assert!(machine.audio_frames_available() > 0);
        assert_eq!(machine.clocks.frames, 0);
    }

    #[test]
    fn run_slice_can_complete_frame_progressively() {
        let mut sliced = make_machine(&[0x00]);
        let mut whole = sliced.clone();

        whole.run_frame().unwrap();

        let mut completed = false;
        for _ in 0..10_000 {
            let result = sliced.run_slice(32, 512).unwrap();
            if result.frame_completed {
                completed = true;
                break;
            }
        }

        assert!(
            completed,
            "slice execution should eventually finish a frame"
        );
        assert_eq!(sliced.clocks.frames, whole.clocks.frames);
        assert_eq!(sliced.clocks.cycles, whole.clocks.cycles);
        assert_eq!(sliced.cpu.pc, whole.cpu.pc);
        assert_eq!(sliced.ppu.ly, whole.ppu.ly);
        assert_eq!(sliced.ppu.mode_cycles, whole.ppu.mode_cycles);
    }

    #[test]
    fn cgb_pcm_registers_expose_channel_digital_outputs() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.write8(0xFF26, 0x80);
        machine.write8(0xFF24, 0x77);
        machine.write8(0xFF25, 0x11);
        machine.write8(0xFF11, 0xC0);
        machine.write8(0xFF12, 0xF3);
        machine.write8(0xFF13, 0xFF);
        machine.write8(0xFF14, 0x87);
        machine.finish_step(132);

        assert_ne!(machine.read8(0xFF76) & 0x0F, 0);
    }

    #[test]
    fn cgb_header_selects_cgb_mode_on_load() {
        let mut machine = Machine::new();
        machine.load_rom(make_cgb_test_rom(&[0x00], 0x80)).unwrap();

        assert_eq!(machine.mode, HardwareMode::Cgb);
        assert!(machine.pending_cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::ModeSelected {
                cgb_enabled: true,
                cgb_only: false
            }
        )));
    }

    #[test]
    fn load_rom_with_forced_dmg_keeps_dual_mode_rom_in_dmg_mode() {
        let mut machine = Machine::new();
        machine
            .load_rom_with_mode(make_cgb_test_rom(&[0x00], 0x80), Some(HardwareMode::Dmg))
            .unwrap();

        assert_eq!(machine.mode, HardwareMode::Dmg);
    }

    #[test]
    fn load_rom_with_forced_cgb_allows_dmg_rom_in_cgb_mode() {
        let mut machine = Machine::new();
        machine
            .load_rom_with_mode(make_test_rom(&[0x00]), Some(HardwareMode::Cgb))
            .unwrap();

        assert_eq!(machine.mode, HardwareMode::Cgb);
    }

    #[test]
    fn load_rom_with_forced_dmg_rejects_cgb_only_rom() {
        let mut machine = Machine::new();
        let error = machine
            .load_rom_with_mode(make_cgb_test_rom(&[0x00], 0xC0), Some(HardwareMode::Dmg))
            .unwrap_err();

        assert!(matches!(error, error::CoreError::InvalidState(_)));
    }

    #[test]
    fn load_rom_with_forced_cgb_initializes_dmg_compatibility_palettes() {
        let mut machine = Machine::new();
        machine
            .load_rom_with_mode(make_test_rom(&[0x00]), Some(HardwareMode::Cgb))
            .unwrap();

        assert_eq!(machine.mode, HardwareMode::Cgb);
        assert_eq!(machine.memory.bg_palette_rgb555(0, 0), 0x6BFF);
        assert_eq!(machine.memory.bg_palette_rgb555(0, 3), 0x0CC2);
    }

    #[test]
    fn run_frame_fast_matches_run_frame_for_basic_timing() {
        let mut slow = make_machine(&[0x00]);
        let mut fast = slow.clone();

        slow.run_frame().unwrap();
        fast.run_frame_fast().unwrap();

        assert_eq!(slow.clocks.frames, fast.clocks.frames);
        assert_eq!(slow.clocks.cycles, fast.clocks.cycles);
        assert_eq!(slow.ppu.ly, fast.ppu.ly);
        assert_eq!(slow.ppu.mode_cycles, fast.ppu.mode_cycles);
        assert_eq!(slow.cpu.pc, fast.cpu.pc);
    }

    #[test]
    fn load_rom_applies_dmg_post_boot_registers_and_io() {
        let machine = make_machine(&[0x00]);

        assert_eq!(machine.cpu.a, 0x01);
        assert_eq!(machine.cpu.f.0, 0xB0);
        assert_eq!(machine.cpu.b, 0x00);
        assert_eq!(machine.cpu.c, 0x13);
        assert_eq!(machine.cpu.d, 0x00);
        assert_eq!(machine.cpu.e, 0xD8);
        assert_eq!(machine.cpu.h, 0x01);
        assert_eq!(machine.cpu.l, 0x4D);
        assert_eq!(machine.cpu.pc, 0x0100);
        assert_eq!(machine.cpu.sp, 0xFFFE);
        assert_eq!(machine.peek8(0xFF07), 0xF8);
        assert_eq!(machine.peek8(0xFF0F), 0xE1);
        assert_eq!(machine.peek8(0xFF47), 0xFC);
        assert_eq!(machine.peek8(0xFF48), 0xFF);
        assert_eq!(machine.peek8(0xFF49), 0xFF);
        assert_eq!(machine.peek8(0xFF26), 0xF1);
    }

    #[test]
    fn load_rom_resets_cpu_and_ppu_state() {
        let mut machine = make_machine(&[0x00]);
        machine.cpu.a = 0x99;
        machine.cpu.b = 0x88;
        machine.cpu.pc = 0x2345;
        machine.cpu.ime = true;
        machine.ppu.bgp = 0x00;
        machine.ppu.obp0 = 0x00;
        machine.ppu.obp1 = 0x00;
        machine.ppu.ly = 55;

        machine.load_rom(make_test_rom(&[0x00])).unwrap();

        assert_eq!(machine.cpu.a, 0x01);
        assert_eq!(machine.cpu.b, 0x00);
        assert_eq!(machine.cpu.pc, 0x0100);
        assert!(!machine.cpu.ime);
        assert_eq!(machine.ppu.bgp, 0xFC);
        assert_eq!(machine.ppu.obp0, 0xFF);
        assert_eq!(machine.ppu.obp1, 0xFF);
        assert_eq!(machine.ppu.ly, 0);
    }

    #[test]
    fn vbk_switch_selects_second_vram_bank() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.write8(0x8000, 0x12);
        machine.write8(0xFF4F, 0x01);
        machine.write8(0x8000, 0x34);
        machine.write8(0xFF4F, 0x00);

        assert_eq!(machine.memory.vram_bank(0)[0], 0x12);
        assert_eq!(machine.memory.vram_bank(1)[0], 0x34);
        assert!(machine.pending_cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::VramBankSwitch {
                bank: 1,
                value: 0x01
            }
        )));
    }

    #[test]
    fn svbk_switch_selects_switchable_wram_bank() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.write8(0xD000, 0x56);
        machine.write8(0xFF70, 0x02);
        machine.write8(0xD000, 0x78);
        machine.write8(0xFF70, 0x01);

        assert_eq!(machine.read8(0xD000), 0x56);
        machine.write8(0xFF70, 0x02);
        assert_eq!(machine.read8(0xD000), 0x78);
        assert!(machine.pending_cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::WramBankSwitch {
                bank: 2,
                value: 0x02
            }
        )));
    }

    #[test]
    fn upper_wram_tail_does_not_mirror_into_hram() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.write8(0xFFF5, 0xAA);
        machine.write8(0xDFF5, 0x55);

        assert_eq!(machine.read8(0xDFF5), 0x55);
        assert_eq!(machine.read8(0xFFF5), 0xAA);

        machine.write8(0xFF70, 0x02);

        assert_eq!(machine.read8(0xFFF5), 0xAA);
    }

    #[test]
    fn hram_scratch_survives_stack_usage_in_upper_wram() {
        let mut machine = make_cgb_machine(&[
            0x31, 0xF9, 0xDF, // LD SP,$DFF9
            0xF8, 0x00, // LD HL,SP+0
            0x7D, // LD A,L
            0xE0, 0xF4, // LDH [$F4],A
            0x7C, // LD A,H
            0xE0, 0xF5, // LDH [$F5],A
            0x3E, 0x02, // LD A,$02
            0xCD, 0x20, 0x01, // CALL $0120
            0xF0, 0xF4, // LDH A,[$F4]
            0x6F, // LD L,A
            0xF0, 0xF5, // LDH A,[$F5]
            0x67, // LD H,A
            0xF9, // LD SP,HL
            0x00, // NOP
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding to $0120
            0x47, // LD B,A
            0xF8, 0x00, // LD HL,SP+0
            0x2B, // DEC HL
            0x2B, // DEC HL
            0x78, // LD A,B
            0xF5, // PUSH AF
            0x3E, 0x07, // LD A,$07
            0x47, // LD B,A
            0xF1, // POP AF
            0xB8, // CP B
            0xC9, // RET
        ]);

        for _ in 0..25 {
            machine.step_instruction().unwrap();
        }

        assert_eq!(machine.cpu.sp, 0xDFF9);
        assert_eq!(machine.read8(0xFFF4), 0xF9);
        assert_eq!(machine.read8(0xFFF5), 0xDF);
    }

    #[test]
    fn cgb_bg_palette_auto_increment_advances_index() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.write8(0xFF68, 0x80);
        machine.write8(0xFF69, 0x1F);
        machine.write8(0xFF69, 0x03);

        assert_eq!(machine.memory.bg_palette_rgb555(0, 0), 0x031F);
        assert_eq!(machine.read8(0xFF68) & 0x3F, 0x02);
        assert!(machine.pending_cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::BgPaletteDataWrite {
                index: 0x00,
                blocked: false,
                auto_increment: true,
                ..
            }
        )));
    }

    #[test]
    fn cgb_palette_write_is_blocked_during_mode_three_but_index_still_advances() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 100;
        machine.write8(0xFF68, 0x80);
        machine.write8(0xFF69, 0x55);

        assert_eq!(machine.memory.bg_palette_rgb555(0, 0), 0xFFFF);
        assert_eq!(machine.read8(0xFF68) & 0x3F, 0x01);
        assert!(machine.pending_cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::BgPaletteDataWrite {
                index: 0x00,
                value: 0x55,
                blocked: true,
                auto_increment: true
            }
        )));
    }

    #[test]
    fn key1_write_followed_by_stop_switches_double_speed() {
        let mut machine = make_cgb_machine(&[0x10, 0x00, 0x00]);
        machine.write8(0xFF4D, 0x01);

        let step = machine.step_instruction().unwrap();

        assert_eq!(step.cycles, 4 + CGB_SPEED_SWITCH_FREEZE_CYCLES);
        assert_eq!(
            machine.clocks.cycles,
            u64::from(4 + CGB_SPEED_SWITCH_FREEZE_CYCLES)
        );
        assert!(machine.cgb_double_speed);
        assert!(!machine.cgb_speed_switch_armed);
        assert_eq!(machine.cgb_speed_switch_freeze_cycles, 0);
        assert!(step.cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::Key1Write {
                armed: true,
                double_speed: false,
                value: 0x01
            }
        )));
        assert!(step.cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::SpeedSwitch {
                double_speed: true,
                stop_stall_cycles: CGB_SPEED_SWITCH_FREEZE_CYCLES
            }
        )));
        assert!(step.cgb_trace.iter().any(|event| matches!(
            event,
            CgbTraceEvent::SpeedSwitchFreeze {
                cpu_cycles: CGB_SPEED_SWITCH_FREEZE_CYCLES,
                ..
            }
        )));
    }

    #[test]
    fn double_speed_keeps_lcd_progress_slower_than_timer_progress() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.cgb_double_speed = true;
        machine.ppu.lcdc = 0x91;
        machine.write8(0xFF07, 0x05);
        let before_mode_cycles = machine.ppu.mode_cycles;

        let step = machine.finish_step(16);

        assert_eq!(step.cycles, 16);
        assert_eq!(machine.timer.tima, 1);
        assert_eq!(machine.ppu.mode_cycles.wrapping_sub(before_mode_cycles), 8);
    }

    #[test]
    fn double_speed_gdma_stall_estimate_doubles_cpu_cycles() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.cgb_double_speed = true;
        machine.write8(0xFF51, 0x12);
        machine.write8(0xFF52, 0x30);
        machine.write8(0xFF53, 0x80);
        machine.write8(0xFF54, 0x00);
        machine.write8(0xFF55, 0x00);
        let step = machine.finish_step(0);

        assert!(step.dma_trace.iter().any(|event| matches!(
            event,
            DmaTraceEvent::GdmaStallEstimate {
                blocks: 1,
                stall_cycles: 64
            }
        )));
        assert!(step.dma_trace.iter().any(|event| matches!(
            event,
            DmaTraceEvent::HdmaBlock {
                stall_cycles: 64,
                ..
            }
        )));
    }

    #[test]
    fn blocked_palette_write_keeps_data_ff_visible() {
        let mut machine = make_cgb_machine(&[0x00]);
        machine.ppu.lcdc = 0x91;
        machine.ppu.ly = 0;
        machine.ppu.mode_cycles = 100;
        machine.write8(0xFF68, 0x80);
        machine.write8(0xFF69, 0x12);

        assert_eq!(machine.read8(0xFF69), 0xFF);
        assert_eq!(machine.read8(0xFF68) & 0x3F, 0x01);
    }
}
