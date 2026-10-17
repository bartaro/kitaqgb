pub mod regs;
pub mod render;

use crate::{memory::Memory, types::PpuTraceEvent};
use serde::{Deserialize, Serialize};

const CYCLES_PER_SCANLINE: u32 = 456;
const VISIBLE_SCANLINES: u8 = 144;
const TOTAL_SCANLINES: u8 = 154;
const FRAME_CYCLES: u32 = CYCLES_PER_SCANLINE * TOTAL_SCANLINES as u32;
const FRAMEBUFFER_LEN: usize = 160 * 144;
const SCREEN_WIDTH: usize = 160;

fn default_bg_color_ids() -> Vec<u8> {
    vec![0; FRAMEBUFFER_LEN]
}

fn default_bg_priority_flags() -> Vec<u8> {
    vec![0; FRAMEBUFFER_LEN]
}

fn default_color_framebuffer() -> Vec<u16> {
    vec![0; FRAMEBUFFER_LEN]
}

mod framebuffer_serde {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(
        framebuffer: &[u8; super::FRAMEBUFFER_LEN],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        framebuffer.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; super::FRAMEBUFFER_LEN], D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = Vec::<u8>::deserialize(deserializer)?;
        if data.len() != super::FRAMEBUFFER_LEN {
            return Err(D::Error::custom(format!(
                "expected framebuffer of {} bytes, got {}",
                super::FRAMEBUFFER_LEN,
                data.len()
            )));
        }

        let mut framebuffer = [0; super::FRAMEBUFFER_LEN];
        framebuffer.copy_from_slice(&data);
        Ok(framebuffer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PpuTickResult {
    pub vblank_enter: bool,
    pub frame_completed: bool,
    pub stat_interrupt: bool,
    pub render_scanline: Option<u8>,
    pub trace: Vec<PpuTraceEvent>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PpuTickFastResult {
    pub vblank_enter: bool,
    pub frame_completed: bool,
    pub stat_interrupt: bool,
    pub render_scanline: Option<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PpuMode {
    HBlank = 0,
    VBlank = 1,
    OamSearch = 2,
    Transfer = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ppu {
    pub lcdc: u8,
    pub stat: u8,
    pub ly: u8,
    pub lyc: u8,
    pub scx: u8,
    pub scy: u8,
    pub wx: u8,
    pub wy: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    #[serde(with = "framebuffer_serde")]
    pub framebuffer: [u8; FRAMEBUFFER_LEN],
    #[serde(skip, default = "default_color_framebuffer")]
    pub color_framebuffer: Vec<u16>,
    #[serde(skip, default = "default_bg_color_ids")]
    pub bg_color_ids: Vec<u8>,
    #[serde(skip, default = "default_bg_priority_flags")]
    pub bg_priority_flags: Vec<u8>,
    #[serde(skip, default)]
    pub scanline_sprites: Vec<(usize, [u8; 4])>,
    #[serde(default)]
    pub window_line_counter: u8,
    pub mode_cycles: u32,
    pub frame_serial: u64,
    #[serde(skip)]
    pub stat_interrupt_line: bool,
    #[serde(skip)]
    pub mode3_sprite_penalty: u32,
}

impl Ppu {
    pub fn tick(&mut self, cycles: u32) -> PpuTickResult {
        if self.lcdc & 0x80 == 0 {
            self.ly = 0;
            self.mode_cycles = 0;
            let stat_interrupt = self.refresh_stat_interrupt_line();
            return PpuTickResult {
                vblank_enter: false,
                frame_completed: false,
                stat_interrupt,
                render_scanline: None,
                trace: Vec::new(),
            };
        }
        let mut result = PpuTickResult {
            vblank_enter: false,
            frame_completed: false,
            stat_interrupt: false,
            render_scanline: None,
            trace: Vec::new(),
        };
        let mut remaining = cycles;
        while remaining > 0 {
            let mode = self.current_mode();
            let old_mode = mode;
            let old_ly = self.ly;
            let old_coincidence = self.stat_coincidence();
            let boundary = match mode {
                PpuMode::OamSearch => 80u32.saturating_sub(self.mode_cycles),
                PpuMode::Transfer => self
                    .mode3_end_cycles(self.ly)
                    .saturating_sub(self.mode_cycles),
                PpuMode::HBlank | PpuMode::VBlank => {
                    CYCLES_PER_SCANLINE.saturating_sub(self.mode_cycles)
                }
            }
            .max(1);
            let step = remaining.min(boundary);
            self.mode_cycles += step;
            remaining -= step;

            if self.ly < VISIBLE_SCANLINES
                && mode == PpuMode::Transfer
                && self.mode_cycles >= self.mode3_end_cycles(self.ly)
            {
                result.render_scanline = Some(self.ly);
                result
                    .trace
                    .push(PpuTraceEvent::ScanlineRender { ly: self.ly });
            }

            if self.mode_cycles >= CYCLES_PER_SCANLINE {
                self.mode_cycles -= CYCLES_PER_SCANLINE;
                self.ly = self.ly.wrapping_add(1);
                result.trace.push(PpuTraceEvent::ScanlineAdvance {
                    from_ly: old_ly,
                    to_ly: self.ly,
                });
                if self.ly == VISIBLE_SCANLINES {
                    result.vblank_enter = true;
                    self.scanline_sprites.clear();
                    self.mode3_sprite_penalty = 0;
                    result
                        .trace
                        .push(PpuTraceEvent::VblankEnter { ly: self.ly });
                } else if self.ly >= TOTAL_SCANLINES {
                    self.ly = 0;
                    self.frame_serial = self.frame_serial.wrapping_add(1);
                    self.window_line_counter = 0;
                    result.frame_completed = true;
                    result.trace.push(PpuTraceEvent::FrameComplete {
                        frame_serial: self.frame_serial,
                    });
                }
            }

            let new_mode = self.current_mode();
            if new_mode != old_mode {
                result.trace.push(PpuTraceEvent::PpuModeChange {
                    from_mode: old_mode as u8,
                    to_mode: new_mode as u8,
                    ly: self.ly,
                });
            }
            let new_coincidence = self.stat_coincidence();
            if new_coincidence != old_coincidence {
                result.trace.push(PpuTraceEvent::StatSignal {
                    coincidence: new_coincidence,
                    ly: self.ly,
                    lyc: self.lyc,
                });
            }

            if self.refresh_stat_interrupt_line() {
                result.stat_interrupt = true;
            }
        }
        result
    }

    pub fn tick_fast(&mut self, cycles: u32) -> PpuTickFastResult {
        if self.lcdc & 0x80 == 0 {
            self.ly = 0;
            self.mode_cycles = 0;
            let stat_interrupt = self.refresh_stat_interrupt_line();
            return PpuTickFastResult {
                vblank_enter: false,
                frame_completed: false,
                stat_interrupt,
                render_scanline: None,
            };
        }
        let mut result = PpuTickFastResult::default();
        let mut remaining = cycles;
        while remaining > 0 {
            let mode = self.current_mode();
            let old_coincidence = self.stat_coincidence();
            let boundary = match mode {
                PpuMode::OamSearch => 80u32.saturating_sub(self.mode_cycles),
                PpuMode::Transfer => self
                    .mode3_end_cycles(self.ly)
                    .saturating_sub(self.mode_cycles),
                PpuMode::HBlank | PpuMode::VBlank => {
                    CYCLES_PER_SCANLINE.saturating_sub(self.mode_cycles)
                }
            }
            .max(1);
            let step = remaining.min(boundary);
            self.mode_cycles += step;
            remaining -= step;

            if self.ly < VISIBLE_SCANLINES
                && mode == PpuMode::Transfer
                && self.mode_cycles >= self.mode3_end_cycles(self.ly)
            {
                result.render_scanline = Some(self.ly);
            }

            if self.mode_cycles >= CYCLES_PER_SCANLINE {
                self.mode_cycles -= CYCLES_PER_SCANLINE;
                self.ly = self.ly.wrapping_add(1);
                if self.ly == VISIBLE_SCANLINES {
                    result.vblank_enter = true;
                    self.scanline_sprites.clear();
                    self.mode3_sprite_penalty = 0;
                } else if self.ly >= TOTAL_SCANLINES {
                    self.ly = 0;
                    self.frame_serial = self.frame_serial.wrapping_add(1);
                    self.window_line_counter = 0;
                    result.frame_completed = true;
                }
            }

            let new_coincidence = self.stat_coincidence();
            if new_coincidence != old_coincidence && self.refresh_stat_interrupt_line() {
                result.stat_interrupt = true;
                continue;
            }
            if self.refresh_stat_interrupt_line() {
                result.stat_interrupt = true;
            }
        }
        result
    }

    pub fn frame_cycles(&self) -> u32 {
        FRAME_CYCLES
    }
    pub fn cycles_until_next_event(&self) -> u32 {
        if !self.lcd_enabled() {
            return u32::MAX;
        }
        match self.current_mode() {
            PpuMode::OamSearch => 80u32.saturating_sub(self.mode_cycles).max(1),
            PpuMode::Transfer => self
                .mode3_end_cycles(self.ly)
                .saturating_sub(self.mode_cycles)
                .max(1),
            PpuMode::HBlank | PpuMode::VBlank => {
                CYCLES_PER_SCANLINE.saturating_sub(self.mode_cycles).max(1)
            }
        }
    }
    pub fn current_mode(&self) -> PpuMode {
        if !self.lcd_enabled() {
            PpuMode::HBlank
        } else if self.ly >= VISIBLE_SCANLINES {
            PpuMode::VBlank
        } else if self.mode_cycles < 80 {
            PpuMode::OamSearch
        } else if self.mode_cycles < self.mode3_end_cycles(self.ly) {
            PpuMode::Transfer
        } else {
            PpuMode::HBlank
        }
    }
    pub fn stat_coincidence(&self) -> bool {
        self.ly == self.lyc
    }
    pub fn read_stat(&self) -> u8 {
        let coincidence = if self.stat_coincidence() { 0x04 } else { 0x00 };
        0x80 | (self.stat & 0x78) | coincidence | (self.current_mode() as u8)
    }

    pub fn predict_ly_after_cycles(&self, cycles: u32) -> u8 {
        self.predict_state_after_cycles(cycles).0
    }

    pub fn predict_stat_after_cycles(&self, cycles: u32) -> u8 {
        let (ly, mode_cycles) = self.predict_state_after_cycles(cycles);
        let coincidence = if ly == self.lyc { 0x04 } else { 0x00 };
        0x80 | (self.stat & 0x78) | coincidence | (self.mode_for_state(ly, mode_cycles) as u8)
    }

    fn predict_state_after_cycles(&self, cycles: u32) -> (u8, u32) {
        if !self.lcd_enabled() {
            return (0, 0);
        }

        let mut ly = self.ly;
        let mut mode_cycles = self.mode_cycles.saturating_add(cycles);
        while mode_cycles >= CYCLES_PER_SCANLINE {
            mode_cycles -= CYCLES_PER_SCANLINE;
            ly = ly.wrapping_add(1);
            if ly >= TOTAL_SCANLINES {
                ly = 0;
            }
        }
        (ly, mode_cycles)
    }

    fn mode_for_state(&self, ly: u8, mode_cycles: u32) -> PpuMode {
        if !self.lcd_enabled() {
            PpuMode::HBlank
        } else if ly >= VISIBLE_SCANLINES {
            PpuMode::VBlank
        } else if mode_cycles < 80 {
            PpuMode::OamSearch
        } else if mode_cycles < self.mode3_end_cycles(ly) {
            PpuMode::Transfer
        } else {
            PpuMode::HBlank
        }
    }

    pub fn restore_runtime_state(&mut self, memory: &Memory) {
        if self.color_framebuffer.len() != FRAMEBUFFER_LEN {
            self.color_framebuffer = default_color_framebuffer();
        }
        if self.bg_color_ids.len() != FRAMEBUFFER_LEN {
            self.bg_color_ids = default_bg_color_ids();
        }
        if self.bg_priority_flags.len() != FRAMEBUFFER_LEN {
            self.bg_priority_flags = default_bg_priority_flags();
        }
        self.stat_interrupt_line = false;
        self.update_stat();
        self.refresh_stat_interrupt_line();
        self.refresh_mode3_penalty(memory.oam());
    }
    pub fn can_cpu_access_vram(&self) -> bool {
        !self.lcd_enabled() || self.current_mode() != PpuMode::Transfer
    }
    pub fn can_cpu_access_oam(&self) -> bool {
        !self.lcd_enabled() || matches!(self.current_mode(), PpuMode::HBlank | PpuMode::VBlank)
    }
    pub fn lcd_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }
    pub fn bg_enabled(&self) -> bool {
        self.lcdc & 0x01 != 0
    }
    pub fn sprite_enabled(&self) -> bool {
        self.lcdc & 0x02 != 0
    }
    pub fn sprite_height(&self) -> usize {
        if self.lcdc & 0x04 != 0 {
            16
        } else {
            8
        }
    }
    pub fn bg_map_base(&self) -> usize {
        if self.lcdc & 0x08 != 0 {
            0x1C00
        } else {
            0x1800
        }
    }
    pub fn tile_data_8000(&self) -> bool {
        self.lcdc & 0x10 != 0
    }
    pub fn window_enabled(&self) -> bool {
        self.lcdc & 0x20 != 0
    }
    pub fn window_map_base(&self) -> usize {
        if self.lcdc & 0x40 != 0 {
            0x1C00
        } else {
            0x1800
        }
    }

    fn mode3_end_cycles(&self, line: u8) -> u32 {
        252 + self.mode3_penalty_dots(line)
    }

    fn mode3_penalty_dots(&self, line: u8) -> u32 {
        let mut penalty = 0u32;
        if self.bg_enabled() {
            penalty += u32::from(self.scx & 0x07);
        }
        if self.window_visible_on_line(line) {
            penalty += 6;
        }
        penalty += self.mode3_sprite_penalty;
        penalty
    }

    fn window_visible_on_line(&self, line: u8) -> bool {
        self.bg_enabled()
            && self.window_enabled()
            && self.wy < VISIBLE_SCANLINES
            && line >= self.wy
            && self.wx <= 166
    }

    fn sprite_penalty_from_selected(&self, line: u8, ordered: &mut Vec<(usize, [u8; 4])>) -> u32 {
        ordered.sort_by(|(lhs_index, lhs), (rhs_index, rhs)| {
            lhs[1].cmp(&rhs[1]).then(lhs_index.cmp(rhs_index))
        });

        let mut penalty = 0u32;
        let mut seen_tiles = Vec::new();
        for (_, chunk) in ordered {
            let oam_x = chunk[1];
            if oam_x == 0 {
                penalty += 11;
                continue;
            }

            let sx = oam_x as i16 - 8;
            let pixel_x = sx.max(0) as usize;
            let tile_id = self.bg_tile_id_for_pixel_for_penalty(line as usize, pixel_x);
            if !seen_tiles.contains(&tile_id) {
                seen_tiles.push(tile_id);
                let fine_x = pixel_x & 7;
                penalty += (5usize.saturating_sub(fine_x)) as u32;
            }
            penalty += 6;
        }
        penalty
    }

    fn bg_tile_id_for_pixel_for_penalty(&self, y: usize, x: usize) -> u16 {
        let window_line = (y == self.ly as usize && self.window_visible_on_line(self.ly))
            .then_some(self.window_line_counter as usize);
        self.bg_tile_id_for_pixel_with_window_line(y, x, window_line)
    }

    fn bg_tile_id_for_pixel_with_window_line(
        &self,
        y: usize,
        x: usize,
        window_line: Option<usize>,
    ) -> u16 {
        let use_window = self.window_visible_on_line(y as u8) && x + 7 >= self.wx as usize;
        let (base, px, py) = if use_window {
            (
                self.window_map_base() as u16,
                x + 7 - self.wx as usize,
                window_line.unwrap_or(y - self.wy as usize),
            )
        } else {
            (
                self.bg_map_base() as u16,
                x.wrapping_add(self.scx as usize) & 0xFF,
                y.wrapping_add(self.scy as usize) & 0xFF,
            )
        };
        let tile_x = ((px / 8) & 0x1F) as u16;
        let tile_y = ((py / 8) & 0x1F) as u16;
        base + tile_y * 32 + tile_x
    }

    fn selected_sprites_on_line<'a>(&self, oam: &'a [u8], y: usize) -> Vec<(usize, &'a [u8])> {
        let height = self.sprite_height();
        let screen_y = y as i16;
        let mut visible = Vec::with_capacity(10);
        for (oam_index, chunk) in oam.chunks_exact(4).take(40).enumerate() {
            let sy = chunk[0] as i16 - 16;
            if (sy..(sy + height as i16)).contains(&screen_y) {
                visible.push((oam_index, chunk));
                if visible.len() == 10 {
                    break;
                }
            }
        }
        visible
    }

    pub fn render_visible_area_from_memory(&mut self, memory: &Memory) {
        self.render_visible_area_from_memory_with_mode(memory, false);
    }

    pub fn render_visible_area_from_memory_with_mode(&mut self, memory: &Memory, cgb_mode: bool) {
        for y in 0..VISIBLE_SCANLINES as usize {
            self.render_scanline_from_memory_with_mode(memory, y, cgb_mode);
        }
    }

    pub fn refresh_mode3_penalty(&mut self, oam: &[u8]) {
        if !self.lcd_enabled() || self.ly >= VISIBLE_SCANLINES {
            self.scanline_sprites.clear();
            self.mode3_sprite_penalty = 0;
            return;
        }
        self.latch_scanline_state(oam);
    }

    pub fn latch_scanline_state(&mut self, oam: &[u8]) {
        if !self.lcd_enabled() || self.ly >= VISIBLE_SCANLINES {
            self.scanline_sprites.clear();
            self.mode3_sprite_penalty = 0;
            return;
        }

        let mut selected: Vec<(usize, [u8; 4])> = self
            .selected_sprites_on_line(oam, self.ly as usize)
            .into_iter()
            .map(|(index, chunk)| (index, [chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        self.mode3_sprite_penalty = self.sprite_penalty_from_selected(self.ly, &mut selected);
        self.scanline_sprites = selected;
    }

    pub fn render_scanline_from_memory(&mut self, memory: &Memory, y: usize) {
        self.render_scanline_from_memory_with_mode(memory, y, false);
    }

    pub fn render_scanline_from_memory_with_mode(
        &mut self,
        memory: &Memory,
        y: usize,
        cgb_mode: bool,
    ) {
        if y >= VISIBLE_SCANLINES as usize {
            return;
        }
        let vram0 = memory.vram_bank(0);
        let vram1 = memory.vram_bank(1);
        self.render_scanline_from_vram_with_mode(y, vram0, vram1, memory, cgb_mode, None);
        if self.sprite_enabled() {
            self.overlay_sprites_on_scanline_with_vram_mode(
                vram0,
                vram1,
                memory,
                memory.oam(),
                y,
                cgb_mode,
            );
        }
    }

    pub fn render_current_scanline_from_memory(&mut self, memory: &Memory) {
        self.render_current_scanline_from_memory_with_mode(memory, false);
    }

    pub fn render_current_scanline_from_memory_with_mode(
        &mut self,
        memory: &Memory,
        cgb_mode: bool,
    ) {
        let y = self.ly as usize;
        if y >= VISIBLE_SCANLINES as usize {
            return;
        }

        let vram0 = memory.vram_bank(0);
        let vram1 = memory.vram_bank(1);
        let window_line = self
            .window_visible_on_line(self.ly)
            .then_some(self.window_line_counter as usize);
        self.render_scanline_from_vram_with_mode(y, vram0, vram1, memory, cgb_mode, window_line);
        if self.sprite_enabled() {
            self.overlay_latched_sprites_on_scanline_with_vram_mode(
                vram0, vram1, memory, y, cgb_mode,
            );
        }
        if window_line.is_some() {
            self.window_line_counter = self.window_line_counter.wrapping_add(1);
        }
    }

    pub fn visible_sprite_count_estimate(&self, oam: &[u8]) -> u32 {
        let h = self.sprite_height() as i16;
        let mut count = 0u32;
        for chunk in oam.chunks_exact(4) {
            let sy = chunk[0] as i16 - 16;
            let sx = chunk[1] as i16 - 8;
            if sx > -8 && sx < 160 && sy > -h && sy < 144 {
                count += 1;
            }
        }
        count
    }

    pub fn nonzero_oam_entries(&self, oam: &[u8]) -> u32 {
        oam.chunks_exact(4)
            .filter(|c| c.iter().any(|&b| b != 0))
            .count() as u32
    }

    pub fn write_lcdc(&mut self, value: u8) -> bool {
        let was_enabled = self.lcd_enabled();
        self.lcdc = value;
        if was_enabled && !self.lcd_enabled() {
            self.ly = 0;
            self.mode_cycles = 0;
            self.window_line_counter = 0;
            self.scanline_sprites.clear();
            self.mode3_sprite_penalty = 0;
            self.framebuffer.fill(0);
            self.color_framebuffer.fill(Self::dmg_gray_rgb555(0));
            self.bg_color_ids.fill(0);
            self.bg_priority_flags.fill(0);
        } else if !was_enabled && self.lcd_enabled() {
            self.ly = 0;
            self.mode_cycles = 0;
            self.window_line_counter = 0;
            self.scanline_sprites.clear();
            self.mode3_sprite_penalty = 0;
        }
        self.refresh_stat_interrupt_line()
    }

    pub fn write_stat(&mut self, value: u8) -> bool {
        self.stat = (self.stat & 0x87) | (value & 0x78);
        self.refresh_stat_interrupt_line()
    }

    pub fn write_lyc(&mut self, value: u8) -> bool {
        self.lyc = value;
        self.refresh_stat_interrupt_line()
    }

    fn update_stat(&mut self) {
        let coincidence = self.ly == self.lyc;
        if coincidence {
            self.stat |= 0x04;
        } else {
            self.stat &= !0x04;
        }
        self.stat = (self.stat & !0x03) | (self.current_mode() as u8);
    }

    fn stat_interrupt_selected(&self) -> bool {
        if !self.lcd_enabled() {
            return false;
        }
        let coincidence_selected = self.stat & 0x40 != 0 && self.stat_coincidence();
        let mode_selected = match self.current_mode() {
            PpuMode::HBlank => self.stat & 0x08 != 0,
            PpuMode::VBlank => self.stat & 0x10 != 0,
            PpuMode::OamSearch => self.stat & 0x20 != 0,
            PpuMode::Transfer => false,
        };
        coincidence_selected || mode_selected
    }

    fn refresh_stat_interrupt_line(&mut self) -> bool {
        let old_line = self.stat_interrupt_line;
        self.update_stat();
        self.stat_interrupt_line = self.stat_interrupt_selected();
        !old_line && self.stat_interrupt_line
    }

    fn render_scanline_from_vram_with_mode(
        &mut self,
        y: usize,
        vram0: &[u8],
        vram1: &[u8],
        memory: &Memory,
        cgb_mode: bool,
        window_line: Option<usize>,
    ) {
        let base = y * SCREEN_WIDTH;
        if !self.bg_enabled() {
            for x in 0..SCREEN_WIDTH {
                self.bg_color_ids[base + x] = 0;
                self.bg_priority_flags[base + x] = 0;
                self.framebuffer[base + x] = 0;
                self.color_framebuffer[base + x] = Self::dmg_gray_rgb555(0);
            }
            return;
        }
        let tile_data_8000 = self.tile_data_8000();
        let bg_map_base = self.bg_map_base();
        let window_enabled = self.window_visible_on_line(y as u8);
        let window_map_base = self.window_map_base();
        for x in 0..SCREEN_WIDTH {
            let use_window = window_enabled && x + 7 >= self.wx as usize;
            let (map_base, px, py) = if use_window {
                (
                    window_map_base,
                    x + 7 - self.wx as usize,
                    window_line.unwrap_or(y - self.wy as usize),
                )
            } else {
                (
                    bg_map_base,
                    x.wrapping_add(self.scx as usize) & 0xFF,
                    y.wrapping_add(self.scy as usize) & 0xFF,
                )
            };
            let tile_x = (px / 8) & 0x1F;
            let tile_y = (py / 8) & 0x1F;
            let map_index = map_base + tile_y * 32 + tile_x;
            let tile_number = vram0.get(map_index).copied().unwrap_or(0);
            let attr = if cgb_mode {
                vram1.get(map_index).copied().unwrap_or(0)
            } else {
                0
            };
            let palette = attr & 0x07;
            let tile_bank = if cgb_mode && (attr & 0x08 != 0) {
                vram1
            } else {
                vram0
            };
            let xflip = cgb_mode && (attr & 0x20 != 0);
            let yflip = cgb_mode && (attr & 0x40 != 0);
            let priority = cgb_mode && (attr & 0x80 != 0);
            let fine_x = if xflip {
                (px & 7) as u8
            } else {
                7 - (px & 7) as u8
            };
            let fine_y = if yflip {
                7usize.saturating_sub(py & 7)
            } else {
                py & 7
            };
            let tile_addr = if tile_data_8000 {
                tile_number as usize * 16
            } else {
                let signed_index = tile_number as i8 as i16;
                (0x1000i32 + signed_index as i32 * 16) as usize
            };
            let lo = tile_bank.get(tile_addr + fine_y * 2).copied().unwrap_or(0);
            let hi = tile_bank
                .get(tile_addr + fine_y * 2 + 1)
                .copied()
                .unwrap_or(0);
            let color_id = (((hi >> fine_x) & 1) << 1) | ((lo >> fine_x) & 1);
            self.bg_color_ids[base + x] = color_id;
            self.bg_priority_flags[base + x] = priority as u8;
            let (gray, rgb555) = if cgb_mode {
                let rgb555 = memory.bg_palette_rgb555(palette, color_id);
                (Self::apply_cgb_palette(rgb555), rgb555)
            } else {
                let gray = Self::apply_palette(self.bgp, color_id);
                (gray, Self::dmg_gray_rgb555(gray))
            };
            self.framebuffer[base + x] = gray;
            self.color_framebuffer[base + x] = rgb555;
        }
    }

    #[allow(dead_code, unused_variables)]
    fn overlay_sprites(&mut self, oam: &[u8]) {
        let height = self.sprite_height();
        for chunk in oam.chunks_exact(4).take(40) {
            let sy = chunk[0] as i16 - 16;
            let sx = chunk[1] as i16 - 8;
            let tile = chunk[2];
            let attr = chunk[3];
            if sx <= -8 || sx >= 160 || sy <= -(height as i16) || sy >= 144 {
                continue;
            }
            let palette = if attr & 0x10 != 0 {
                self.obp1
            } else {
                self.obp0
            };
            let xflip = attr & 0x20 != 0;
            let yflip = attr & 0x40 != 0;
            let priority_behind_bg = attr & 0x80 != 0;
            for py in 0..height {
                let screen_y = sy + py as i16;
                if !(0..144).contains(&screen_y) {
                    continue;
                }
                let tile_y = if yflip { height - 1 - py } else { py };
                let tile_index = if height == 16 { tile & 0xFE } else { tile };
                let tile_addr = tile_index as usize * 16 + tile_y * 2;
                let row_base = tile_addr.min(0x1FFE);
                let lo = oam; // placeholder to keep expression simple in generated text
                let _ = lo;
            }
        }
        // second pass actually reads from framebuffer using VRAM fetched via render_scanline; sprite drawing is approximated in render_visible_area_from_memory_ext below.
    }

    pub fn overlay_sprites_with_vram(&mut self, vram: &[u8], oam: &[u8]) {
        for y in 0..VISIBLE_SCANLINES as usize {
            self.overlay_sprites_on_scanline_with_vram(vram, oam, y);
        }
    }

    fn overlay_sprites_on_scanline_with_vram(&mut self, vram: &[u8], oam: &[u8], y: usize) {
        self.overlay_sprites_on_scanline_with_vram_mode(
            vram,
            vram,
            &Memory::default(),
            oam,
            y,
            false,
        );
    }

    fn overlay_sprites_on_scanline_with_vram_mode(
        &mut self,
        vram0: &[u8],
        vram1: &[u8],
        memory: &Memory,
        oam: &[u8],
        y: usize,
        cgb_mode: bool,
    ) {
        let visible: Vec<(usize, [u8; 4])> = self
            .selected_sprites_on_line(oam, y)
            .into_iter()
            .map(|(index, chunk)| (index, [chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        self.overlay_sprite_list_on_scanline_with_vram_mode(
            vram0, vram1, memory, &visible, y, cgb_mode,
        );
    }

    fn overlay_latched_sprites_on_scanline_with_vram_mode(
        &mut self,
        vram0: &[u8],
        vram1: &[u8],
        memory: &Memory,
        y: usize,
        cgb_mode: bool,
    ) {
        let visible = self.scanline_sprites.clone();
        self.overlay_sprite_list_on_scanline_with_vram_mode(
            vram0, vram1, memory, &visible, y, cgb_mode,
        );
    }

    fn overlay_sprite_list_on_scanline_with_vram_mode(
        &mut self,
        vram0: &[u8],
        vram1: &[u8],
        memory: &Memory,
        sprites: &[(usize, [u8; 4])],
        y: usize,
        cgb_mode: bool,
    ) {
        let height = self.sprite_height();
        let screen_y = y as i16;
        let mut visible = sprites.to_vec();
        visible.sort_by(|(lhs_index, lhs), (rhs_index, rhs)| {
            rhs[1].cmp(&lhs[1]).then(rhs_index.cmp(lhs_index))
        });

        for (_, chunk) in visible {
            let sy = chunk[0] as i16 - 16;
            let sx = chunk[1] as i16 - 8;
            let tile = chunk[2];
            let attr = chunk[3];
            if sx <= -8 || sx >= SCREEN_WIDTH as i16 || sy <= -(height as i16) || sy >= 144 {
                continue;
            }
            let dmg_palette = if attr & 0x10 != 0 {
                self.obp1
            } else {
                self.obp0
            };
            let cgb_palette = attr & 0x07;
            let tile_bank = if cgb_mode && (attr & 0x08 != 0) {
                vram1
            } else {
                vram0
            };
            let xflip = attr & 0x20 != 0;
            let yflip = attr & 0x40 != 0;
            let priority_behind_bg = attr & 0x80 != 0;
            let sprite_row = (screen_y - sy) as usize;
            let tile_y = if yflip {
                height - 1 - sprite_row
            } else {
                sprite_row
            };
            let tile_index = if height == 16 { tile & 0xFE } else { tile };
            let tile_addr = tile_index as usize * 16 + tile_y * 2;
            let lo = tile_bank.get(tile_addr).copied().unwrap_or(0);
            let hi = tile_bank.get(tile_addr + 1).copied().unwrap_or(0);
            for px in 0..8usize {
                let screen_x = sx + px as i16;
                if !(0..SCREEN_WIDTH as i16).contains(&screen_x) {
                    continue;
                }
                let tx = if xflip { px } else { 7 - px };
                let color_id = (((hi >> tx) & 1) << 1) | ((lo >> tx) & 1);
                if color_id == 0 {
                    continue;
                }
                let idx = y * SCREEN_WIDTH + screen_x as usize;
                let bg_blocks_obj = self.bg_priority_flags[idx] != 0 && self.bg_color_ids[idx] != 0;
                if bg_blocks_obj || (priority_behind_bg && self.bg_color_ids[idx] != 0) {
                    continue;
                }
                let (gray, rgb555) = if cgb_mode {
                    let rgb555 = memory.obj_palette_rgb555(cgb_palette, color_id);
                    (Self::apply_cgb_palette(rgb555), rgb555)
                } else {
                    let gray = Self::apply_palette(dmg_palette, color_id);
                    (gray, Self::dmg_gray_rgb555(gray))
                };
                self.framebuffer[idx] = gray;
                self.color_framebuffer[idx] = rgb555;
            }
        }
    }

    fn apply_palette(palette: u8, color_id: u8) -> u8 {
        (palette >> (color_id * 2)) & 0x03
    }

    fn apply_cgb_palette(rgb555: u16) -> u8 {
        let r = (rgb555 & 0x1F) as u32;
        let g = ((rgb555 >> 5) & 0x1F) as u32;
        let b = ((rgb555 >> 10) & 0x1F) as u32;
        (((r * 3) + (g * 6) + b) / 10) as u8
    }

    fn dmg_gray_rgb555(gray: u8) -> u16 {
        const DMG_SHADES: [u16; 4] = [
            0x7FFF, // white
            0x56B5, // light gray
            0x294A, // dark gray
            0x0000, // black
        ];
        DMG_SHADES[usize::from(gray.min(3))]
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            lcdc: 0x91,
            stat: 0x85,
            ly: 0,
            lyc: 0,
            scx: 0,
            scy: 0,
            wx: 7,
            wy: 0,
            bgp: 0xE4,
            obp0: 0xE4,
            obp1: 0xE4,
            framebuffer: [0; FRAMEBUFFER_LEN],
            color_framebuffer: default_color_framebuffer(),
            bg_color_ids: default_bg_color_ids(),
            bg_priority_flags: default_bg_priority_flags(),
            scanline_sprites: Vec::new(),
            window_line_counter: 0,
            mode_cycles: 0,
            frame_serial: 0,
            stat_interrupt_line: false,
            mode3_sprite_penalty: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    #[test]
    fn transfer_to_hblank_requests_scanline_render() {
        let mut ppu = Ppu::default();
        ppu.lcdc = 0x91;
        ppu.ly = 3;
        ppu.mode_cycles = 248;

        let result = ppu.tick(8);

        assert_eq!(result.render_scanline, Some(3));
        assert_eq!(ppu.current_mode(), PpuMode::HBlank);
    }

    #[test]
    fn scx_penalty_delays_transfer_end() {
        let mut ppu = Ppu::default();
        ppu.lcdc = 0x91;
        ppu.scx = 7;
        ppu.ly = 0;
        ppu.mode_cycles = 251;

        let early = ppu.tick(1);
        assert_eq!(early.render_scanline, None);
        assert_eq!(ppu.current_mode(), PpuMode::Transfer);

        let late = ppu.tick(7);
        assert_eq!(late.render_scanline, Some(0));
        assert_eq!(ppu.current_mode(), PpuMode::HBlank);
    }

    #[test]
    fn window_penalty_delays_transfer_end() {
        let mut ppu = Ppu::default();
        ppu.lcdc = 0xB1;
        ppu.wx = 7;
        ppu.wy = 0;
        ppu.scx = 0;
        ppu.ly = 0;
        ppu.mode_cycles = 251;

        let early = ppu.tick(1);
        assert_eq!(early.render_scanline, None);
        assert_eq!(ppu.current_mode(), PpuMode::Transfer);

        let late = ppu.tick(6);
        assert_eq!(late.render_scanline, Some(0));
        assert_eq!(ppu.current_mode(), PpuMode::HBlank);
    }

    #[test]
    fn sprite_penalty_delays_transfer_end() {
        let mut ppu = Ppu::default();
        let mut memory = Memory::default();
        ppu.lcdc = 0x93;
        ppu.ly = 0;
        ppu.mode_cycles = 251;
        memory.write8(0xFE00, 16);
        memory.write8(0xFE01, 8);
        memory.write8(0xFE02, 0);
        memory.write8(0xFE03, 0);
        ppu.refresh_mode3_penalty(memory.oam());

        let early = ppu.tick(10);
        assert_eq!(early.render_scanline, None);
        assert_eq!(ppu.current_mode(), PpuMode::Transfer);

        let late = ppu.tick(2);
        assert_eq!(late.render_scanline, Some(0));
        assert_eq!(ppu.current_mode(), PpuMode::HBlank);
    }

    #[test]
    fn only_first_ten_selected_sprites_contribute_to_mode3_penalty() {
        let mut ppu = Ppu::default();
        let mut memory = Memory::default();
        ppu.lcdc = 0x93;
        ppu.ly = 0;

        for index in 0..11u16 {
            let base = 0xFE00 + index * 4;
            memory.write8(base, 16);
            memory.write8(base + 1, 8);
            memory.write8(base + 2, 0);
            memory.write8(base + 3, 0);
        }

        ppu.refresh_mode3_penalty(memory.oam());
        assert_eq!(ppu.mode3_sprite_penalty, 65);
    }

    #[test]
    fn sprite_priority_uses_bg_color_id_not_palette_output() {
        let mut ppu = Ppu::default();
        let mut memory = Memory::default();
        ppu.lcdc = 0x93;
        ppu.bgp = 0b0000_0000;
        ppu.obp0 = 0b1110_0100;

        memory.write8(0x9800, 0);
        memory.write8(0x8000, 0x80);
        memory.write8(0x8001, 0x00);

        memory.write8(0xFE00, 16);
        memory.write8(0xFE01, 8);
        memory.write8(0xFE02, 1);
        memory.write8(0xFE03, 0x80);
        memory.write8(0x8010, 0x80);
        memory.write8(0x8011, 0x00);

        ppu.render_scanline_from_memory(&memory, 0);

        assert_eq!(ppu.bg_color_ids[0], 1);
        assert_eq!(ppu.framebuffer[0], 0);
    }

    #[test]
    fn only_first_ten_sprites_on_a_scanline_are_drawn() {
        let mut ppu = Ppu::default();
        let mut memory = Memory::default();
        ppu.lcdc = 0x93;
        ppu.bgp = 0b1110_0100;
        ppu.obp0 = 0b1110_0100;

        memory.write8(0x8000, 0x00);
        memory.write8(0x8001, 0x00);
        memory.write8(0x8010, 0x80);
        memory.write8(0x8011, 0x00);

        for index in 0..11u16 {
            let base = 0xFE00 + index * 4;
            memory.write8(base, 16);
            memory.write8(base + 1, if index < 10 { 0 } else { 8 });
            memory.write8(base + 2, 1);
            memory.write8(base + 3, 0);
        }

        ppu.render_scanline_from_memory(&memory, 0);

        assert_eq!(ppu.framebuffer[0], 0);
    }

    #[test]
    fn lcd_disable_clears_scanline_buffers() {
        let mut ppu = Ppu::default();
        ppu.framebuffer[0] = 3;
        ppu.bg_color_ids[0] = 2;
        ppu.scanline_sprites.push((0, [16, 8, 1, 0]));
        ppu.window_line_counter = 5;

        ppu.write_lcdc(0x00);

        assert_eq!(ppu.framebuffer[0], 0);
        assert_eq!(ppu.bg_color_ids[0], 0);
        assert!(ppu.scanline_sprites.is_empty());
        assert_eq!(ppu.window_line_counter, 0);
        assert_eq!(ppu.ly, 0);
    }

    #[test]
    fn window_line_counter_only_advances_when_window_draws() {
        let mut ppu = Ppu::default();
        let mut memory = Memory::default();
        ppu.lcdc = 0xB1;
        ppu.wx = 7;
        ppu.wy = 5;
        ppu.bgp = 0b1110_0100;

        memory.write8(0x9800, 1);
        memory.write8(0x8010, 0x80);
        memory.write8(0x8011, 0x00);
        memory.write8(0x8012, 0x80);
        memory.write8(0x8013, 0x80);
        memory.write8(0x8014, 0x00);
        memory.write8(0x8015, 0x80);

        ppu.ly = 5;
        ppu.render_current_scanline_from_memory(&memory);
        assert_eq!(ppu.framebuffer[5 * SCREEN_WIDTH], 1);
        assert_eq!(ppu.window_line_counter, 1);

        ppu.lcdc &= !0x20;
        ppu.ly = 6;
        ppu.render_current_scanline_from_memory(&memory);
        assert_eq!(ppu.window_line_counter, 1);

        ppu.lcdc |= 0x20;
        ppu.ly = 7;
        ppu.render_current_scanline_from_memory(&memory);
        assert_eq!(ppu.framebuffer[7 * SCREEN_WIDTH], 3);
        assert_eq!(ppu.window_line_counter, 2);
    }

    #[test]
    fn window_is_not_visible_when_bg_is_disabled() {
        let mut ppu = Ppu::default();
        ppu.lcdc = 0xA0;
        ppu.wx = 7;
        ppu.wy = 0;

        assert!(!ppu.window_visible_on_line(0));
        assert_eq!(ppu.mode3_penalty_dots(0), 0);
    }

    #[test]
    fn predicted_ff44_matches_tick_near_scanline_boundary() {
        let mut ppu = Ppu::default();
        ppu.lcdc = 0x91;
        ppu.ly = 0;
        ppu.mode_cycles = 454;

        let mut cloned = ppu.clone();
        let _ = cloned.tick(4);

        assert_eq!(ppu.predict_ly_after_cycles(4), cloned.ly);
    }

    #[test]
    fn predicted_ff41_matches_tick_across_mode_boundary() {
        let mut ppu = Ppu::default();
        ppu.lcdc = 0x91;
        ppu.ly = 0;
        ppu.lyc = 1;
        ppu.mode_cycles = 78;

        let mut cloned = ppu.clone();
        let _ = cloned.tick(4);

        assert_eq!(ppu.predict_stat_after_cycles(4), cloned.read_stat());
    }
}
