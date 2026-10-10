use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "gui")]
pub use gui::{run_gui, AssetStudioApp};

pub mod image_io;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TILE_SIZE: usize = 8;
pub const TILE_PIXELS: usize = TILE_SIZE * TILE_SIZE;
pub const TILE_BYTES_2BPP: usize = 16;
pub const DEFAULT_MAP_W: u8 = 32;
pub const DEFAULT_MAP_H: u8 = 32;
pub const DEFAULT_TILE_COUNT: usize = 64;
pub const MAX_TILE_ID: usize = 255;

#[cfg(feature = "gui")]
const WINDOW_TITLE: &str = "GBHUA";
#[cfg(feature = "gui")]
const SCREEN_TITLE: &str = "GBHUA";
#[cfg(feature = "gui")]
const AUTOEXPORT_C: &str = "gbhua_export.c";
const AUTOEXPORT_GBR: &str = "gbhua_tiles.gbr";
#[cfg(feature = "gui")]
const AUTOEXPORT_GBM: &str = "gbhua_map.gbm";
const GBM_OBJECT_MARKER: &[u8; 6] = b"HPJMTL";
const DMG_PALETTE: [PaletteColor; 4] = [
    PaletteColor::new(224, 232, 216),
    PaletteColor::new(154, 166, 142),
    PaletteColor::new(83, 95, 82),
    PaletteColor::new(24, 30, 29),
];

#[cfg(all(test, feature = "gui"))]
static TEST_CLIPBOARD_TEXT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[derive(Debug, Error)]
pub enum StudioError {
    #[error("coordinate outside {width}x{height}: ({x}, {y})")]
    OutOfBounds {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    },
    #[error("tile index {index} outside tile count {count}")]
    TileIndex { index: usize, count: usize },
    #[error("map size must be 1..=255, got {width}x{height}")]
    MapSize { width: usize, height: usize },
    #[error("autotile table must contain 16 tile ids")]
    AutotileTable,
    #[error("KITAQGB u8 tile map cannot reference tile id {0}")]
    TileIdOverflow(u16),
    #[error("asset format error: {0}")]
    Format(String),
    #[error("invalid project JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("file IO failed: {0}")]
    Io(#[from] std::io::Error),
}

pub type StudioResult<T> = std::result::Result<T, StudioError>;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl PaletteColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Tile {
    pixels: Vec<u8>,
    pub label: String,
    pub tags: Vec<String>,
}

impl Default for Tile {
    fn default() -> Self {
        Self::blank()
    }
}

impl Tile {
    pub fn blank() -> Self {
        Self {
            pixels: vec![0; TILE_PIXELS],
            label: String::new(),
            tags: Vec::new(),
        }
    }

    pub fn checker(a: u8, b: u8) -> Self {
        let mut tile = Self::blank();
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let value = if (x + y) & 1 == 0 { a } else { b };
                tile.set_pixel_unchecked(x, y, value);
            }
        }
        tile
    }

    pub fn normalize(&mut self) {
        self.pixels.resize(TILE_PIXELS, 0);
        self.pixels.truncate(TILE_PIXELS);
        for value in &mut self.pixels {
            *value &= 0x03;
        }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: usize, y: usize) -> StudioResult<u8> {
        if x >= TILE_SIZE || y >= TILE_SIZE {
            return Err(StudioError::OutOfBounds {
                x,
                y,
                width: TILE_SIZE,
                height: TILE_SIZE,
            });
        }
        Ok(self.pixels[y * TILE_SIZE + x] & 0x03)
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, value: u8) -> StudioResult<()> {
        if x >= TILE_SIZE || y >= TILE_SIZE {
            return Err(StudioError::OutOfBounds {
                x,
                y,
                width: TILE_SIZE,
                height: TILE_SIZE,
            });
        }
        self.set_pixel_unchecked(x, y, value);
        Ok(())
    }

    fn set_pixel_unchecked(&mut self, x: usize, y: usize, value: u8) {
        self.pixels[y * TILE_SIZE + x] = value & 0x03;
    }

    pub fn fill(&mut self, value: u8) {
        self.pixels.fill(value & 0x03);
    }

    pub fn replace_color(&mut self, from: u8, to: u8) -> usize {
        let from = from & 0x03;
        let to = to & 0x03;
        let mut changed = 0;
        for value in &mut self.pixels {
            if *value == from {
                *value = to;
                changed += 1;
            }
        }
        changed
    }

    pub fn flip_x(&mut self) {
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE / 2 {
                self.pixels
                    .swap(y * TILE_SIZE + x, y * TILE_SIZE + (TILE_SIZE - 1 - x));
            }
        }
    }

    pub fn flip_y(&mut self) {
        for y in 0..TILE_SIZE / 2 {
            for x in 0..TILE_SIZE {
                self.pixels
                    .swap(y * TILE_SIZE + x, (TILE_SIZE - 1 - y) * TILE_SIZE + x);
            }
        }
    }

    pub fn rotate_right(&mut self) {
        let before = self.pixels.clone();
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let dst = y * TILE_SIZE + x;
                let src = (TILE_SIZE - 1 - x) * TILE_SIZE + y;
                self.pixels[dst] = before[src];
            }
        }
    }

    pub fn shift(&mut self, dx: i32, dy: i32, wrap: bool) {
        let before = self.pixels.clone();
        self.pixels.fill(0);
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                let Some((nx, ny)) = wrap_or_clip(nx, ny, TILE_SIZE as i32, TILE_SIZE as i32, wrap)
                else {
                    continue;
                };
                self.pixels[ny as usize * TILE_SIZE + nx as usize] = before[y * TILE_SIZE + x];
            }
        }
    }

    pub fn encoded_2bpp(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(TILE_BYTES_2BPP);
        for y in 0..TILE_SIZE {
            let mut lo = 0u8;
            let mut hi = 0u8;
            for x in 0..TILE_SIZE {
                let value = self.pixels[y * TILE_SIZE + x] & 0x03;
                let bit = 7 - x;
                lo |= (value & 0x01) << bit;
                hi |= ((value >> 1) & 0x01) << bit;
            }
            bytes.push(lo);
            bytes.push(hi);
        }
        bytes
    }

    pub fn decode_2bpp(bytes: &[u8]) -> StudioResult<Self> {
        if bytes.len() != TILE_BYTES_2BPP {
            return Err(StudioError::Format(format!(
                "tile bytecode must contain {TILE_BYTES_2BPP} bytes, got {}",
                bytes.len()
            )));
        }
        let mut tile = Self::blank();
        for y in 0..TILE_SIZE {
            let lo = bytes[y * 2];
            let hi = bytes[y * 2 + 1];
            for x in 0..TILE_SIZE {
                let bit = 7 - x;
                let value = ((lo >> bit) & 0x01) | (((hi >> bit) & 0x01) << 1);
                tile.set_pixel_unchecked(x, y, value);
            }
        }
        Ok(tile)
    }

    pub fn density(&self) -> f32 {
        let nonzero = self.pixels.iter().filter(|value| **value != 0).count();
        nonzero as f32 / TILE_PIXELS as f32
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MapCell {
    pub tile: u16,
    pub palette: u8,
    pub vram_bank: u8,
    pub flip_x: bool,
    pub flip_y: bool,
    pub priority: bool,
    pub collision: u8,
    pub event: u8,
}

impl MapCell {
    pub fn with_tile(tile: u16) -> Self {
        Self {
            tile,
            ..Self::default()
        }
    }

    pub fn attr_byte(self) -> u8 {
        (self.palette & 0x07)
            | ((self.vram_bank & 0x01) << 3)
            | if self.flip_x { 0x20 } else { 0 }
            | if self.flip_y { 0x40 } else { 0 }
            | if self.priority { 0x80 } else { 0 }
    }

    fn normalize(&mut self, tile_count: usize) {
        if self.tile as usize >= tile_count {
            self.tile = 0;
        }
        self.palette &= 0x07;
        self.vram_bank &= 0x01;
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MetaSpritePart {
    pub dx: i8,
    pub dy: i8,
    pub tile: u16,
    pub flags: u8,
}

impl MetaSpritePart {
    fn normalize(&mut self, tile_count: usize) {
        if self.tile as usize >= tile_count {
            self.tile = 0;
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetaSpriteFrame {
    pub name: String,
    pub parts: Vec<MetaSpritePart>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetaSprite {
    pub name: String,
    pub frames: Vec<MetaSpriteFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub map_width: u8,
    pub map_height: u8,
    pub tiles: Vec<Tile>,
    pub map: Vec<MapCell>,
    pub palettes: Vec<Vec<PaletteColor>>,
    #[serde(default)]
    pub metasprites: Vec<MetaSprite>,
    pub notes: String,
}

impl Default for Project {
    fn default() -> Self {
        Self::new(DEFAULT_MAP_W, DEFAULT_MAP_H, DEFAULT_TILE_COUNT)
            .expect("default KITAQGB asset project is valid")
    }
}

impl Project {
    pub fn new(width: u8, height: u8, tile_count: usize) -> StudioResult<Self> {
        if width == 0 || height == 0 {
            return Err(StudioError::MapSize {
                width: width as usize,
                height: height as usize,
            });
        }
        let mut tiles = vec![Tile::blank(); tile_count.max(1)];
        seed_default_tiles(&mut tiles);
        let map = vec![MapCell::default(); width as usize * height as usize];
        Ok(Self {
            name: "kitaqgb_scene".to_string(),
            map_width: width,
            map_height: height,
            tiles,
            map,
            palettes: default_palettes(),
            metasprites: Vec::new(),
            notes: String::new(),
        })
    }

    pub fn load_json_file(path: impl AsRef<Path>) -> StudioResult<Self> {
        let text = fs::read_to_string(path)?;
        Self::from_json_str(&text)
    }

    pub fn save_json_file(&self, path: impl AsRef<Path>) -> StudioResult<()> {
        fs::write(path, self.to_json_pretty()?)?;
        Ok(())
    }

    pub fn load_project_file(path: impl AsRef<Path>) -> StudioResult<Self> {
        let path = path.as_ref();
        check_project_extension(path)?;
        match asset_file_kind(path) {
            AssetFileKind::Json => Self::load_json_file(path),
            AssetFileKind::Gbtd => Self::load_gbtd_file(path),
            AssetFileKind::Gbmb => Self::load_gbmb_file(path),
        }
    }

    pub fn save_project_file(&self, path: impl AsRef<Path>) -> StudioResult<()> {
        let path = path.as_ref();
        check_project_extension(path)?;
        match asset_file_kind(path) {
            AssetFileKind::Json => self.save_json_file(path),
            AssetFileKind::Gbtd => self.save_gbtd_file(path),
            AssetFileKind::Gbmb => self.save_gbmb_file(path),
        }
    }

    pub fn load_gbtd_file(path: impl AsRef<Path>) -> StudioResult<Self> {
        let bytes = fs::read(path)?;
        Self::from_gbtd_bytes(&bytes)
    }

    pub fn save_gbtd_file(&self, path: impl AsRef<Path>) -> StudioResult<()> {
        fs::write(path, self.to_gbtd_bytes()?)?;
        Ok(())
    }

    pub fn load_gbmb_file(path: impl AsRef<Path>) -> StudioResult<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        let mut import = import_gbmb_bytes(&bytes)?;
        if let Some(tile_file) = import.tile_file.clone() {
            let tile_path = resolve_related_path(path, &tile_file);
            if tile_path.exists() {
                if let Ok(tile_project) = Self::load_gbtd_file(&tile_path) {
                    import.project.tiles = tile_project.tiles;
                    import.project.palettes = tile_project.palettes;
                    import.project.normalize();
                }
            }
        }
        Ok(import.project)
    }

    pub fn save_gbmb_file(&self, path: impl AsRef<Path>) -> StudioResult<()> {
        let path = path.as_ref();
        let tile_path = path.with_extension("gbr");
        let tile_file = tile_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| AUTOEXPORT_GBR.to_string());
        self.save_gbtd_file(&tile_path)?;
        fs::write(path, self.to_gbmb_bytes(&tile_file)?)?;
        Ok(())
    }

    pub fn from_gbtd_bytes(bytes: &[u8]) -> StudioResult<Self> {
        import_gbtd_bytes(bytes)
    }

    pub fn to_gbtd_bytes(&self) -> StudioResult<Vec<u8>> {
        export_gbtd_bytes(self)
    }

    pub fn from_gbmb_bytes(bytes: &[u8]) -> StudioResult<Self> {
        Ok(import_gbmb_bytes(bytes)?.project)
    }

    pub fn to_gbmb_bytes(&self, tile_file: &str) -> StudioResult<Vec<u8>> {
        export_gbmb_bytes(self, tile_file)
    }

    pub fn from_json_str(text: &str) -> StudioResult<Self> {
        let mut project: Self = serde_json::from_str(text)?;
        project.normalize();
        Ok(project)
    }

    pub fn to_json_pretty(&self) -> StudioResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn normalize(&mut self) {
        if self.map_width == 0 {
            self.map_width = 1;
        }
        if self.map_height == 0 {
            self.map_height = 1;
        }
        if self.tiles.is_empty() {
            self.tiles.push(Tile::blank());
        }
        for tile in &mut self.tiles {
            tile.normalize();
        }
        let cells = self.map_width as usize * self.map_height as usize;
        self.map.resize(cells, MapCell::default());
        self.map.truncate(cells);
        for cell in &mut self.map {
            cell.normalize(self.tiles.len());
        }
        for metasprite in &mut self.metasprites {
            if metasprite.frames.is_empty() {
                metasprite.frames.push(MetaSpriteFrame::default());
            }
            for frame in &mut metasprite.frames {
                for part in &mut frame.parts {
                    part.normalize(self.tiles.len());
                }
            }
        }
        normalize_palettes(&mut self.palettes);
    }

    pub fn resize_map(&mut self, width: u8, height: u8) -> StudioResult<()> {
        if width == 0 || height == 0 {
            return Err(StudioError::MapSize {
                width: width as usize,
                height: height as usize,
            });
        }
        let old_w = self.map_width as usize;
        let old_h = self.map_height as usize;
        let mut next = vec![MapCell::default(); width as usize * height as usize];
        for y in 0..usize::min(old_h, height as usize) {
            for x in 0..usize::min(old_w, width as usize) {
                next[y * width as usize + x] = self.map[y * old_w + x];
            }
        }
        self.map_width = width;
        self.map_height = height;
        self.map = next;
        Ok(())
    }

    pub fn add_tile(&mut self, tile: Tile) -> u16 {
        self.tiles.push(tile);
        (self.tiles.len() - 1).min(u16::MAX as usize) as u16
    }

    pub fn duplicate_tile(&mut self, index: usize) -> StudioResult<u16> {
        let tile = self.tile(index)?.clone();
        Ok(self.add_tile(tile))
    }

    pub fn tile(&self, index: usize) -> StudioResult<&Tile> {
        self.tiles.get(index).ok_or(StudioError::TileIndex {
            index,
            count: self.tiles.len(),
        })
    }

    pub fn tile_mut(&mut self, index: usize) -> StudioResult<&mut Tile> {
        let count = self.tiles.len();
        self.tiles
            .get_mut(index)
            .ok_or(StudioError::TileIndex { index, count })
    }

    pub fn set_tile_pixel(
        &mut self,
        tile_index: usize,
        x: usize,
        y: usize,
        value: u8,
    ) -> StudioResult<()> {
        self.tile_mut(tile_index)?.set_pixel(x, y, value)
    }

    pub fn set_tile_from_2bpp_bytes(
        &mut self,
        tile_index: usize,
        bytes: &[u8],
    ) -> StudioResult<()> {
        let tile = Tile::decode_2bpp(bytes)?;
        *self.tile_mut(tile_index)? = tile;
        Ok(())
    }

    pub fn set_map_cell(&mut self, x: usize, y: usize, mut cell: MapCell) -> StudioResult<()> {
        let index = self.map_index(x, y)?;
        cell.normalize(self.tiles.len());
        self.map[index] = cell;
        Ok(())
    }

    pub fn set_map_tile(&mut self, x: usize, y: usize, tile: u16) -> StudioResult<()> {
        if tile as usize >= self.tiles.len() {
            return Err(StudioError::TileIndex {
                index: tile as usize,
                count: self.tiles.len(),
            });
        }
        let index = self.map_index(x, y)?;
        self.map[index].tile = tile;
        Ok(())
    }

    pub fn map_cell(&self, x: usize, y: usize) -> StudioResult<MapCell> {
        Ok(self.map[self.map_index(x, y)?])
    }

    pub fn paint_rect(&mut self, x: usize, y: usize, w: usize, h: usize, cell: MapCell) -> usize {
        let mut changed = 0;
        for py in y..usize::min(y + h, self.map_height as usize) {
            for px in x..usize::min(x + w, self.map_width as usize) {
                if self.set_map_cell(px, py, cell).is_ok() {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn flood_fill_map(
        &mut self,
        x: usize,
        y: usize,
        replacement: MapCell,
    ) -> StudioResult<usize> {
        let start = self.map_index(x, y)?;
        let target = self.map[start];
        if target == replacement {
            return Ok(0);
        }
        let mut queue = VecDeque::new();
        let mut visited = vec![false; self.map.len()];
        queue.push_back((x, y));
        let mut changed = 0;
        while let Some((cx, cy)) = queue.pop_front() {
            let index = self.map_index(cx, cy)?;
            if visited[index] || self.map[index] != target {
                continue;
            }
            visited[index] = true;
            self.map[index] = replacement;
            changed += 1;
            if cx > 0 {
                queue.push_back((cx - 1, cy));
            }
            if cy > 0 {
                queue.push_back((cx, cy - 1));
            }
            if cx + 1 < self.map_width as usize {
                queue.push_back((cx + 1, cy));
            }
            if cy + 1 < self.map_height as usize {
                queue.push_back((cx, cy + 1));
            }
        }
        Ok(changed)
    }

    pub fn auto_tag_collision_by_density(&mut self, threshold: f32) -> usize {
        let mut changed = 0;
        let threshold = threshold.clamp(0.0, 1.0);
        let densities: Vec<f32> = self.tiles.iter().map(Tile::density).collect();
        for cell in &mut self.map {
            let next = u8::from(densities[cell.tile as usize] >= threshold);
            if cell.collision != next {
                cell.collision = next;
                changed += 1;
            }
        }
        changed
    }

    pub fn apply_neighbor_autotile(&mut self, tiles_by_mask: &[u16]) -> StudioResult<usize> {
        if tiles_by_mask.len() != 16 {
            return Err(StudioError::AutotileTable);
        }
        for tile in tiles_by_mask {
            if *tile as usize >= self.tiles.len() {
                return Err(StudioError::TileIndex {
                    index: *tile as usize,
                    count: self.tiles.len(),
                });
            }
        }
        let before = self.map.clone();
        let mut changed = 0;
        for y in 0..self.map_height as usize {
            for x in 0..self.map_width as usize {
                let index = y * self.map_width as usize + x;
                if before[index].collision == 0 {
                    continue;
                }
                let mut mask = 0usize;
                if y > 0 && before[(y - 1) * self.map_width as usize + x].collision != 0 {
                    mask |= 1;
                }
                if x + 1 < self.map_width as usize
                    && before[y * self.map_width as usize + x + 1].collision != 0
                {
                    mask |= 2;
                }
                if y + 1 < self.map_height as usize
                    && before[(y + 1) * self.map_width as usize + x].collision != 0
                {
                    mask |= 4;
                }
                if x > 0 && before[y * self.map_width as usize + x - 1].collision != 0 {
                    mask |= 8;
                }
                let next = tiles_by_mask[mask];
                if self.map[index].tile != next {
                    self.map[index].tile = next;
                    changed += 1;
                }
            }
        }
        Ok(changed)
    }

    pub fn dedupe_tiles(&mut self) -> DedupeReport {
        let before = self.tiles.len();
        let mut unique = Vec::<Tile>::new();
        let mut lookup = HashMap::<Tile, u16>::new();
        let mut remap = Vec::with_capacity(self.tiles.len());
        for tile in &self.tiles {
            if let Some(index) = lookup.get(tile) {
                remap.push(*index);
            } else {
                let index = unique.len().min(u16::MAX as usize) as u16;
                lookup.insert(tile.clone(), index);
                unique.push(tile.clone());
                remap.push(index);
            }
        }
        for cell in &mut self.map {
            if let Some(next) = remap.get(cell.tile as usize) {
                cell.tile = *next;
            }
        }
        for metasprite in &mut self.metasprites {
            for frame in &mut metasprite.frames {
                for part in &mut frame.parts {
                    if let Some(next) = remap.get(part.tile as usize) {
                        part.tile = *next;
                    }
                }
            }
        }
        self.tiles = unique;
        DedupeReport {
            before,
            after: self.tiles.len(),
            removed: before.saturating_sub(self.tiles.len()),
            remap,
        }
    }

    pub fn budget_report(&self) -> BudgetReport {
        let tile_bytes = self.tiles.len() * TILE_BYTES_2BPP;
        let map_cells = self.map.len();
        let map_bytes = map_cells;
        let attr_bytes = map_cells;
        let collision_bytes = (map_cells + 7) / 8;
        let event_bytes = map_cells;
        let max_tile_id = self.map.iter().map(|cell| cell.tile).max().unwrap_or(0);
        let unique_tiles = self.tiles.iter().cloned().collect::<HashSet<_>>().len();
        let map_tile_bytes = self.map_tile_bytes_lossy();
        let rle_map_bytes = rle_encode(&map_tile_bytes).len();
        let mut warnings = Vec::new();
        if self.tiles.len() > MAX_TILE_ID + 1 {
            warnings.push(format!(
                "{} tiles exceed u8 tile id budget {}",
                self.tiles.len(),
                MAX_TILE_ID + 1
            ));
        }
        if max_tile_id as usize > MAX_TILE_ID {
            warnings.push(format!("map references tile id {max_tile_id}, above 255"));
        }
        if self.map_width > 32 || self.map_height > 32 {
            warnings
                .push("map exceeds one 32x32 BG map; camera/runtime streaming needed".to_string());
        }
        if tile_bytes > 6144 {
            warnings.push(format!(
                "{tile_bytes} tile bytes exceed the 0x8000-0x97FF BG tile VRAM window"
            ));
        }
        for metasprite in &self.metasprites {
            for frame in &metasprite.frames {
                if frame.parts.len() > 40 {
                    warnings.push(format!(
                        "metasprite {} frame {} uses {} OAM parts; hardware limit is 40",
                        metasprite.name,
                        frame.name,
                        frame.parts.len()
                    ));
                }
                if let Some(part) = frame
                    .parts
                    .iter()
                    .find(|part| part.tile > MAX_TILE_ID as u16)
                {
                    warnings.push(format!(
                        "metasprite {} references tile id {}, above 255",
                        metasprite.name, part.tile
                    ));
                }
            }
        }
        BudgetReport {
            tile_count: self.tiles.len(),
            unique_tiles,
            tile_bytes,
            map_cells,
            map_bytes,
            attr_bytes,
            collision_bytes,
            event_bytes,
            rle_map_bytes,
            max_tile_id,
            warnings,
        }
    }

    pub fn render_tile_rgba(
        &self,
        tile_index: usize,
        palette_index: usize,
    ) -> StudioResult<Vec<u8>> {
        let tile = self.tile(tile_index)?;
        Ok(render_tile_rgba(tile, self.palette(palette_index)))
    }

    pub fn render_atlas_rgba(&self, columns: usize) -> Vec<u8> {
        self.render_atlas_rgba_with_palette(columns, 0)
    }

    pub fn render_atlas_rgba_with_palette(&self, columns: usize, palette_index: usize) -> Vec<u8> {
        let columns = columns.max(1);
        let rows = (self.tiles.len() + columns - 1) / columns;
        let width = columns * TILE_SIZE;
        let height = rows.max(1) * TILE_SIZE;
        let palette = self.palette(palette_index);
        let mut pixels = vec![0; width * height * 4];
        for (index, tile) in self.tiles.iter().enumerate() {
            let ox = (index % columns) * TILE_SIZE;
            let oy = (index / columns) * TILE_SIZE;
            blit_tile(&mut pixels, width, ox, oy, tile, palette, false, false);
        }
        pixels
    }

    pub fn render_map_rgba(&self) -> Vec<u8> {
        let width = self.map_width as usize * TILE_SIZE;
        let height = self.map_height as usize * TILE_SIZE;
        let mut pixels = vec![0; width * height * 4];
        for y in 0..self.map_height as usize {
            for x in 0..self.map_width as usize {
                let cell = self.map[y * self.map_width as usize + x];
                let tile = &self.tiles[cell.tile as usize];
                blit_tile(
                    &mut pixels,
                    width,
                    x * TILE_SIZE,
                    y * TILE_SIZE,
                    tile,
                    self.palette(cell.palette as usize),
                    cell.flip_x,
                    cell.flip_y,
                );
                if cell.collision != 0 {
                    tint_tile(
                        &mut pixels,
                        width,
                        x * TILE_SIZE,
                        y * TILE_SIZE,
                        [210, 67, 67, 72],
                    );
                }
                if cell.event != 0 {
                    tint_tile(
                        &mut pixels,
                        width,
                        x * TILE_SIZE,
                        y * TILE_SIZE,
                        [64, 188, 214, 72],
                    );
                }
            }
        }
        pixels
    }

    pub fn render_map_art_rgba(&self) -> Vec<u8> {
        let width = self.map_width as usize * TILE_SIZE;
        let height = self.map_height as usize * TILE_SIZE;
        let mut pixels = vec![0; width * height * 4];
        for y in 0..self.map_height as usize {
            for x in 0..self.map_width as usize {
                let cell = self.map[y * self.map_width as usize + x];
                let tile = &self.tiles[cell.tile as usize];
                blit_tile(
                    &mut pixels,
                    width,
                    x * TILE_SIZE,
                    y * TILE_SIZE,
                    tile,
                    self.palette(cell.palette as usize),
                    cell.flip_x,
                    cell.flip_y,
                );
            }
        }
        pixels
    }

    pub fn add_metasprite(&mut self, name: impl Into<String>) -> usize {
        self.metasprites.push(MetaSprite {
            name: name.into(),
            frames: vec![MetaSpriteFrame {
                name: "frame_00".to_string(),
                parts: Vec::new(),
            }],
        });
        self.metasprites.len() - 1
    }

    pub fn add_metasprite_frame(
        &mut self,
        metasprite_index: usize,
        name: impl Into<String>,
    ) -> StudioResult<usize> {
        let metasprite = self.metasprites.get_mut(metasprite_index).ok_or_else(|| {
            StudioError::Format(format!(
                "metasprite index {metasprite_index} is out of range"
            ))
        })?;
        metasprite.frames.push(MetaSpriteFrame {
            name: name.into(),
            parts: Vec::new(),
        });
        Ok(metasprite.frames.len() - 1)
    }

    pub fn metasprite_frame(
        &self,
        metasprite_index: usize,
        frame_index: usize,
    ) -> StudioResult<&MetaSpriteFrame> {
        self.metasprites
            .get(metasprite_index)
            .and_then(|metasprite| metasprite.frames.get(frame_index))
            .ok_or_else(|| {
                StudioError::Format(format!(
                    "metasprite frame {metasprite_index}:{frame_index} is out of range"
                ))
            })
    }

    pub fn metasprite_frame_mut(
        &mut self,
        metasprite_index: usize,
        frame_index: usize,
    ) -> StudioResult<&mut MetaSpriteFrame> {
        self.metasprites
            .get_mut(metasprite_index)
            .and_then(|metasprite| metasprite.frames.get_mut(frame_index))
            .ok_or_else(|| {
                StudioError::Format(format!(
                    "metasprite frame {metasprite_index}:{frame_index} is out of range"
                ))
            })
    }

    pub fn render_metasprite_rgba(
        &self,
        metasprite_index: usize,
        frame_index: usize,
        width: usize,
        height: usize,
        origin_x: i32,
        origin_y: i32,
    ) -> StudioResult<Vec<u8>> {
        let frame = self.metasprite_frame(metasprite_index, frame_index)?;
        let mut pixels = vec![0; width * height * 4];
        for part in &frame.parts {
            let tile = self.tile(part.tile as usize)?;
            let palette = self.palette((part.flags & 0x07) as usize);
            let flip_x = part.flags & 0x20 != 0;
            let flip_y = part.flags & 0x40 != 0;
            for y in 0..TILE_SIZE {
                for x in 0..TILE_SIZE {
                    let sx = if flip_x { TILE_SIZE - 1 - x } else { x };
                    let sy = if flip_y { TILE_SIZE - 1 - y } else { y };
                    let color_index = tile.pixels[sy * TILE_SIZE + sx] as usize;
                    if color_index == 0 {
                        continue;
                    }
                    let dx = origin_x + part.dx as i32 + x as i32;
                    let dy = origin_y + part.dy as i32 + y as i32;
                    if dx < 0 || dy < 0 || dx >= width as i32 || dy >= height as i32 {
                        continue;
                    }
                    let color = palette[color_index];
                    let offset = (dy as usize * width + dx as usize) * 4;
                    pixels[offset] = color.r;
                    pixels[offset + 1] = color.g;
                    pixels[offset + 2] = color.b;
                    pixels[offset + 3] = 255;
                }
            }
        }
        Ok(pixels)
    }

    pub fn export_kitaqgb_c(&self, prefix: &str) -> StudioResult<String> {
        let prefix = sanitize_c_ident(prefix);
        let tile_bytes = self.tile_bytes_2bpp();
        let map_tiles = self.map_tile_bytes()?;
        let map_attrs = self.map_attr_bytes();
        let collision_bits = self.collision_bits();
        let events = self.event_bytes();

        let mut out = String::new();
        out.push_str("/* Generated by GBHUA. */\n");
        out.push_str("#include \"rpg.h\"\n");
        out.push_str("#include \"cgb_tile.h\"\n\n");
        out.push_str(&format!(
            "#define {upper}_MAP_W {}\n#define {upper}_MAP_H {}\n#define {upper}_TILE_COUNT {}\n\n",
            self.map_width,
            self.map_height,
            self.tiles.len(),
            upper = prefix.to_ascii_uppercase()
        ));
        push_u8_array(&mut out, &format!("{prefix}_tile_data"), &tile_bytes, 16);
        push_u8_array(&mut out, &format!("{prefix}_map_tiles"), &map_tiles, 32);
        push_u8_array(&mut out, &format!("{prefix}_map_attrs"), &map_attrs, 32);
        push_u8_array(
            &mut out,
            &format!("{prefix}_collision_bits"),
            &collision_bits,
            16,
        );
        push_u8_array(&mut out, &format!("{prefix}_events"), &events, 32);
        for (metasprite_index, metasprite) in self.metasprites.iter().enumerate() {
            let sprite_name = sanitize_c_ident(&metasprite.name);
            let name = format!("{prefix}_{sprite_name}_{metasprite_index}");
            let upper = name.to_ascii_uppercase();
            let mut parts = Vec::new();
            let mut offsets = Vec::new();
            let mut counts = Vec::new();
            for frame in &metasprite.frames {
                offsets.push((parts.len() / 4) as u16);
                counts.push(frame.parts.len().min(255) as u8);
                for part in frame.parts.iter().take(255) {
                    if part.tile > MAX_TILE_ID as u16 {
                        return Err(StudioError::TileIdOverflow(part.tile));
                    }
                    parts.extend_from_slice(&[
                        part.dx as u8,
                        part.dy as u8,
                        part.tile as u8,
                        part.flags,
                    ]);
                }
            }
            out.push_str(&format!(
                "#define {upper}_FRAME_COUNT {}\n",
                metasprite.frames.len()
            ));
            push_u8_array(&mut out, &format!("{name}_parts"), &parts, 16);
            push_u16_array(&mut out, &format!("{name}_offsets"), &offsets, 12);
            push_u8_array(&mut out, &format!("{name}_counts"), &counts, 18);
        }
        out.push_str(&format!("map_t {prefix}_map;\n\n"));
        out.push_str(&format!(
            "void {prefix}_build_map()\n{{\n    {prefix}_map.width = {w};\n    {prefix}_map.height = {h};\n    {prefix}_map.bank_tiles = 0;\n    {prefix}_map.tiles = {prefix}_map_tiles;\n    {prefix}_map.collision_bits = {prefix}_collision_bits;\n    {prefix}_map.events = {prefix}_events;\n}}\n\n",
            w = self.map_width,
            h = self.map_height
        ));
        out.push_str(&format!(
            "void {prefix}_upload_tiles()\n{{\n    __vram_copy(0x8000, {prefix}_tile_data, {len});\n}}\n\n",
            len = tile_bytes.len()
        ));
        out.push_str(&format!(
            "void {prefix}_apply_attrs()\n{{\n    u8 y = 0;\n    u8 rows = {h};\n    u8 cols = {w};\n    if (rows > 18) rows = 18;\n    if (cols > 20) cols = 20;\n    while (y < rows) {{\n        __settileattr_bulk_fast((u16)(0x9800 + ((u16)y * 32)), {prefix}_map_attrs + ((u16)y * {w}), cols);\n        y++;\n    }}\n}}\n\n",
            w = self.map_width,
            h = self.map_height
        ));
        out.push_str(&format!(
            "void {prefix}_load()\n{{\n    {prefix}_upload_tiles();\n    {prefix}_build_map();\n    map_load(0, &{prefix}_map);\n    {prefix}_apply_attrs();\n}}\n"
        ));
        Ok(out)
    }

    pub fn tile_bytes_2bpp(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.tiles.len() * TILE_BYTES_2BPP);
        for tile in &self.tiles {
            bytes.extend(tile.encoded_2bpp());
        }
        bytes
    }

    pub fn map_tile_bytes(&self) -> StudioResult<Vec<u8>> {
        self.map
            .iter()
            .map(|cell| {
                if cell.tile > MAX_TILE_ID as u16 {
                    Err(StudioError::TileIdOverflow(cell.tile))
                } else {
                    Ok(cell.tile as u8)
                }
            })
            .collect()
    }

    fn map_tile_bytes_lossy(&self) -> Vec<u8> {
        self.map.iter().map(|cell| cell.tile as u8).collect()
    }

    pub fn map_attr_bytes(&self) -> Vec<u8> {
        self.map.iter().map(|cell| cell.attr_byte()).collect()
    }

    pub fn collision_bits(&self) -> Vec<u8> {
        let mut bits = vec![0u8; (self.map.len() + 7) / 8];
        for (index, cell) in self.map.iter().enumerate() {
            if cell.collision != 0 {
                bits[index >> 3] |= 1 << (index & 7);
            }
        }
        bits
    }

    pub fn event_bytes(&self) -> Vec<u8> {
        self.map.iter().map(|cell| cell.event).collect()
    }

    fn map_index(&self, x: usize, y: usize) -> StudioResult<usize> {
        let width = self.map_width as usize;
        let height = self.map_height as usize;
        if x >= width || y >= height {
            return Err(StudioError::OutOfBounds {
                x,
                y,
                width,
                height,
            });
        }
        Ok(y * width + x)
    }

    fn palette(&self, index: usize) -> &[PaletteColor] {
        let index = index.min(self.palettes.len().saturating_sub(1));
        &self.palettes[index]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AssetFileKind {
    Json,
    Gbtd,
    Gbmb,
}

struct GbmbImport {
    project: Project,
    tile_file: Option<String>,
}

fn check_project_extension(path: &Path) -> StudioResult<()> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(
        extension.as_str(),
        "gbh" | "json" | "gbr" | "gbtd" | "gbtb" | "gbm" | "gbmb"
    ) {
        Ok(())
    } else {
        Err(StudioError::Format(
            "Unsupported project extension; use .gbh, .json, .gbr, .gbtd, .gbtb, .gbm or .gbmb"
                .into(),
        ))
    }
}

fn asset_file_kind(path: &Path) -> AssetFileKind {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("gbr" | "gbtd" | "gbtb") => AssetFileKind::Gbtd,
        Some("gbm" | "gbmb") => AssetFileKind::Gbmb,
        _ => AssetFileKind::Json,
    }
}

fn import_gbtd_bytes(bytes: &[u8]) -> StudioResult<Project> {
    if bytes.len() < 4 || &bytes[0..3] != b"GBO" {
        return Err(StudioError::Format(
            "GBTD/GBR file must start with GBO".to_string(),
        ));
    }

    let mut offset = 4;
    let mut tiles: Option<Vec<Tile>> = None;
    let mut name = None;
    let mut palettes = None;

    while offset < bytes.len() {
        let object_type = read_u16_le(bytes, &mut offset)?;
        let _object_id = read_u16_le(bytes, &mut offset)?;
        let object_len = read_u32_le(bytes, &mut offset)? as usize;
        let object = take_bytes(bytes, &mut offset, object_len)?;

        match object_type {
            0x02 => {
                let tile_data = parse_gbtd_tile_data(object)?;
                name = tile_data.name;
                tiles = Some(tile_data.tiles);
            }
            0x0D => {
                palettes = parse_gbtd_palettes(object);
            }
            _ => {}
        }
    }

    let tiles = tiles
        .ok_or_else(|| StudioError::Format("GBTD/GBR tile data object missing".to_string()))?;
    let mut project = Project::new(DEFAULT_MAP_W, DEFAULT_MAP_H, tiles.len())?;
    project.tiles = tiles;
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        project.name = name;
    } else {
        project.name = "gbtd_tiles".to_string();
    }
    if let Some(palettes) = palettes {
        project.palettes = palettes;
    }
    project.notes = "Imported from GBTD/GBR tile data.".to_string();
    project.normalize();
    Ok(project)
}

fn export_gbtd_bytes(project: &Project) -> StudioResult<Vec<u8>> {
    if project.tiles.len() > u16::MAX as usize {
        return Err(StudioError::Format(format!(
            "GBTD/GBR export supports at most {} tiles",
            u16::MAX
        )));
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"GBO0");

    let mut producer = Vec::new();
    push_fixed_string(&mut producer, "GBHUA", 30);
    push_fixed_string(&mut producer, env!("CARGO_PKG_VERSION"), 10);
    push_fixed_string(&mut producer, "Clean-room GBTD/GBR export", 80);
    push_gbtd_object(&mut out, 0x01, 0, &producer)?;

    let mut tile_data = Vec::new();
    push_fixed_string(&mut tile_data, &project.name, 30);
    push_u16_le(&mut tile_data, TILE_SIZE as u16);
    push_u16_le(&mut tile_data, TILE_SIZE as u16);
    push_u16_le(&mut tile_data, project.tiles.len() as u16);
    tile_data.extend_from_slice(&[0, 1, 2, 3]);
    for tile in &project.tiles {
        tile_data.extend(tile.pixels().iter().map(|value| value & 0x03));
    }
    push_gbtd_object(&mut out, 0x02, 1, &tile_data)?;

    let mut settings = Vec::new();
    push_u16_le(&mut settings, 1);
    settings.extend_from_slice(&[0, 1, 0, 0]);
    push_u16_le(&mut settings, 1);
    push_u16_le(&mut settings, 1);
    push_u32_le(&mut settings, 0xFFFF_0000);
    settings.push(0xFF);
    settings.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0]);
    push_gbtd_object(&mut out, 0x03, 1, &settings)?;

    let mut palette_object = Vec::new();
    push_u16_le(&mut palette_object, 1);
    push_u16_le(&mut palette_object, 8);
    for palette in normalized_palette_copy(&project.palettes) {
        for color in palette {
            palette_object.extend_from_slice(&[color.r, color.g, color.b, 0]);
        }
    }
    push_u16_le(&mut palette_object, 0);
    push_gbtd_object(&mut out, 0x0D, 1, &palette_object)?;

    let mut tile_pal = Vec::new();
    push_u16_le(&mut tile_pal, 1);
    push_u16_le(&mut tile_pal, project.tiles.len() as u16);
    tile_pal.resize(tile_pal.len() + project.tiles.len(), 0);
    push_u16_le(&mut tile_pal, 0);
    push_gbtd_object(&mut out, 0x0E, 1, &tile_pal)?;

    Ok(out)
}

fn import_gbmb_bytes(bytes: &[u8]) -> StudioResult<GbmbImport> {
    if bytes.len() < 4 || &bytes[0..3] != b"GBO" {
        return Err(StudioError::Format(
            "GBMB/GBM file must start with GBO".to_string(),
        ));
    }
    if bytes[3] != b'1' {
        return Err(StudioError::Format(format!(
            "unsupported GBMB/GBM version {}",
            bytes[3] as char
        )));
    }

    let mut offset = 4;
    let mut map_name = String::new();
    let mut map_width = None;
    let mut map_height = None;
    let mut tile_file = None;
    let mut tile_count = 1usize;
    let mut map_records: Option<Vec<MapCell>> = None;

    while offset < bytes.len() {
        if take_bytes(bytes, &mut offset, GBM_OBJECT_MARKER.len())? != GBM_OBJECT_MARKER {
            return Err(StudioError::Format(
                "GBMB/GBM object marker HPJMTL missing".to_string(),
            ));
        }
        let object_type = read_u16_le(bytes, &mut offset)?;
        let _object_id = read_u16_le(bytes, &mut offset)?;
        let _master_id = read_u16_le(bytes, &mut offset)?;
        let _crc = read_u32_le(bytes, &mut offset)?;
        let object_len = read_u32_le(bytes, &mut offset)? as usize;
        let object = take_bytes(bytes, &mut offset, object_len)?;

        match object_type {
            0x02 => {
                let mut inner = 0;
                map_name = read_fixed_string(object, &mut inner, 128)?;
                let width = read_u32_le(object, &mut inner)? as usize;
                let height = read_u32_le(object, &mut inner)? as usize;
                let _prop_count = read_u32_le(object, &mut inner)?;
                let related = read_fixed_string(object, &mut inner, 256)?;
                let count = read_u32_le(object, &mut inner)? as usize;
                let _prop_color_count = read_u32_le(object, &mut inner)?;
                map_width = Some(width);
                map_height = Some(height);
                if !related.is_empty() {
                    tile_file = Some(related);
                }
                tile_count = tile_count.max(count.max(1));
            }
            0x03 => {
                map_records = Some(parse_gbmb_map_records(object)?);
            }
            _ => {}
        }
    }

    let width =
        map_width.ok_or_else(|| StudioError::Format("GBMB/GBM map object missing".to_string()))?;
    let height =
        map_height.ok_or_else(|| StudioError::Format("GBMB/GBM map object missing".to_string()))?;
    if width == 0 || height == 0 || width > u8::MAX as usize || height > u8::MAX as usize {
        return Err(StudioError::MapSize { width, height });
    }

    let mut records = map_records.unwrap_or_default();
    let cells = width * height;
    records.resize(cells, MapCell::default());
    records.truncate(cells);
    let max_tile = records
        .iter()
        .map(|cell| cell.tile as usize)
        .max()
        .unwrap_or(0);

    let mut project = Project::new(width as u8, height as u8, tile_count.max(max_tile + 1))?;
    project.name = if map_name.is_empty() {
        "gbmb_map".to_string()
    } else {
        map_name
    };
    project.map = records;
    project.notes = "Imported from GBMB/GBM map data.".to_string();
    project.normalize();

    Ok(GbmbImport { project, tile_file })
}

fn export_gbmb_bytes(project: &Project, tile_file: &str) -> StudioResult<Vec<u8>> {
    for cell in &project.map {
        if cell.tile > 1023 {
            return Err(StudioError::Format(format!(
                "GBMB/GBM record cannot store tile id {}",
                cell.tile
            )));
        }
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"GBO1");

    let mut producer = Vec::new();
    push_fixed_string(&mut producer, "GBHUA", 128);
    push_fixed_string(&mut producer, env!("CARGO_PKG_VERSION"), 10);
    push_fixed_string(&mut producer, "Clean-room GBMB/GBM export", 128);
    push_gbmb_object(&mut out, 0x01, 0, 0, &producer)?;

    let mut map = Vec::new();
    push_fixed_string(&mut map, &project.name, 128);
    push_u32_le(&mut map, project.map_width as u32);
    push_u32_le(&mut map, project.map_height as u32);
    push_u32_le(&mut map, 0);
    push_fixed_string(&mut map, tile_file, 256);
    push_u32_le(&mut map, project.tiles.len() as u32);
    push_u32_le(&mut map, 0);
    push_gbmb_object(&mut out, 0x02, 1, 0, &map)?;

    let mut map_data = Vec::with_capacity(project.map.len() * 3);
    for cell in &project.map {
        let value = ((cell.tile as u32) & 0x03FF)
            | (((cell.palette as u32) & 0x1F) << 10)
            | (u32::from(cell.flip_x) << 22)
            | (u32::from(cell.flip_y) << 23);
        map_data.push(((value >> 16) & 0xFF) as u8);
        map_data.push(((value >> 8) & 0xFF) as u8);
        map_data.push((value & 0xFF) as u8);
    }
    push_gbmb_object(&mut out, 0x03, 2, 1, &map_data)?;

    let mut settings = Vec::new();
    push_u32_le(&mut settings, 900);
    push_u32_le(&mut settings, 620);
    settings.extend_from_slice(&[0, 1, 1, 0, 0]);
    push_u16_le(&mut settings, 2);
    push_u16_le(&mut settings, 0);
    settings.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
    push_u32_le(&mut settings, 0xFFFF_FFFF);
    push_u32_le(&mut settings, 1);
    push_u32_le(&mut settings, 1);
    push_gbmb_object(&mut out, 0x07, 3, 1, &settings)?;

    Ok(out)
}

struct GbtdTileData {
    name: Option<String>,
    tiles: Vec<Tile>,
}

fn parse_gbtd_tile_data(data: &[u8]) -> StudioResult<GbtdTileData> {
    let mut offset = 0;
    let name = read_fixed_string(data, &mut offset, 30)?;
    let width = read_u16_le(data, &mut offset)? as usize;
    let height = read_u16_le(data, &mut offset)? as usize;
    let count = read_u16_le(data, &mut offset)? as usize;
    let _color_set = take_bytes(data, &mut offset, 4)?;
    if width != TILE_SIZE || height != TILE_SIZE {
        return Err(StudioError::Format(format!(
            "unsupported GBTD/GBR tile size {width}x{height}; KITAQGB uses 8x8"
        )));
    }
    let pixels = take_bytes(data, &mut offset, count * TILE_PIXELS)?;
    let mut tiles = Vec::with_capacity(count.max(1));
    for (index, chunk) in pixels.chunks(TILE_PIXELS).enumerate() {
        tiles.push(Tile {
            pixels: chunk.iter().map(|value| value & 0x03).collect(),
            label: format!("tile_{index:03}"),
            tags: Vec::new(),
        });
    }
    if tiles.is_empty() {
        tiles.push(Tile::blank());
    }
    Ok(GbtdTileData {
        name: (!name.is_empty()).then_some(name),
        tiles,
    })
}

fn parse_gbtd_palettes(data: &[u8]) -> Option<Vec<Vec<PaletteColor>>> {
    let mut offset = 0;
    let _id = read_u16_le(data, &mut offset).ok()?;
    let count = read_u16_le(data, &mut offset).ok()? as usize;
    if count == 0 || data.len().saturating_sub(offset) < count * 16 {
        return None;
    }
    let mut palettes = Vec::new();
    for _ in 0..usize::min(count, 8) {
        let mut palette = Vec::new();
        for _ in 0..4 {
            let color = take_bytes(data, &mut offset, 4).ok()?;
            palette.push(PaletteColor::new(color[0], color[1], color[2]));
        }
        palettes.push(palette);
    }
    normalize_palettes(&mut palettes);
    Some(palettes)
}

fn parse_gbmb_map_records(data: &[u8]) -> StudioResult<Vec<MapCell>> {
    if data.len() % 3 != 0 {
        return Err(StudioError::Format(
            "GBMB/GBM map tile data length is not a multiple of 3".to_string(),
        ));
    }
    let mut records = Vec::with_capacity(data.len() / 3);
    for chunk in data.chunks(3) {
        let value = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | chunk[2] as u32;
        records.push(MapCell {
            tile: (value & 0x03FF) as u16,
            palette: ((value >> 10) & 0x07) as u8,
            flip_x: ((value >> 22) & 1) != 0,
            flip_y: ((value >> 23) & 1) != 0,
            ..MapCell::default()
        });
    }
    Ok(records)
}

fn normalized_palette_copy(palettes: &[Vec<PaletteColor>]) -> Vec<Vec<PaletteColor>> {
    let mut copy = palettes.to_vec();
    normalize_palettes(&mut copy);
    copy
}

fn push_gbtd_object(
    out: &mut Vec<u8>,
    object_type: u16,
    object_id: u16,
    data: &[u8],
) -> StudioResult<()> {
    push_u16_le(out, object_type);
    push_u16_le(out, object_id);
    push_len_u32(out, data.len())?;
    out.extend_from_slice(data);
    Ok(())
}

fn push_gbmb_object(
    out: &mut Vec<u8>,
    object_type: u16,
    object_id: u16,
    master_id: u16,
    data: &[u8],
) -> StudioResult<()> {
    out.extend_from_slice(GBM_OBJECT_MARKER);
    push_u16_le(out, object_type);
    push_u16_le(out, object_id);
    push_u16_le(out, master_id);
    push_u32_le(out, 0);
    push_len_u32(out, data.len())?;
    out.extend_from_slice(data);
    Ok(())
}

fn push_len_u32(out: &mut Vec<u8>, len: usize) -> StudioResult<()> {
    let len = u32::try_from(len)
        .map_err(|_| StudioError::Format("object too large for GBTD/GBMB".to_string()))?;
    push_u32_le(out, len);
    Ok(())
}

fn push_fixed_string(out: &mut Vec<u8>, value: &str, len: usize) {
    let mut bytes = vec![0u8; len];
    for (dst, src) in bytes.iter_mut().zip(value.as_bytes().iter().copied()) {
        *dst = src;
    }
    out.extend_from_slice(&bytes);
}

fn read_fixed_string(bytes: &[u8], offset: &mut usize, len: usize) -> StudioResult<String> {
    let data = take_bytes(bytes, offset, len)?;
    let end = data
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(data.len());
    Ok(String::from_utf8_lossy(&data[..end]).to_string())
}

fn take_bytes<'a>(bytes: &'a [u8], offset: &mut usize, len: usize) -> StudioResult<&'a [u8]> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| StudioError::Format("asset object length overflow".to_string()))?;
    if end > bytes.len() {
        return Err(StudioError::Format(
            "asset object ended unexpectedly".to_string(),
        ));
    }
    let slice = &bytes[*offset..end];
    *offset = end;
    Ok(slice)
}

fn read_u16_le(bytes: &[u8], offset: &mut usize) -> StudioResult<u16> {
    let data = take_bytes(bytes, offset, 2)?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32_le(bytes: &[u8], offset: &mut usize) -> StudioResult<u32> {
    let data = take_bytes(bytes, offset, 4)?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn push_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn resolve_related_path(base_file: &Path, related: &str) -> PathBuf {
    let path = PathBuf::from(related);
    if path.is_absolute() {
        path
    } else {
        base_file
            .parent()
            .map(|parent| parent.join(&path))
            .unwrap_or(path)
    }
}

#[cfg(feature = "gui")]
fn parse_tile_bytecode(text: &str) -> StudioResult<Vec<u8>> {
    let mut bytes = Vec::new();
    for token in text
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_ascii_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let value = if let Some(hex) = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
            .or_else(|| token.strip_prefix('$'))
        {
            u8::from_str_radix(hex, 16)
        } else {
            token.parse::<u8>()
        }
        .map_err(|_| StudioError::Format(format!("invalid bytecode token {token:?}")))?;
        bytes.push(value);
    }

    if bytes.len() != TILE_BYTES_2BPP {
        return Err(StudioError::Format(format!(
            "tile bytecode must contain {TILE_BYTES_2BPP} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

#[cfg(feature = "gui")]
fn format_tile_bytecode(bytes: &[u8]) -> String {
    bytes
        .chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .map(|value| format!("0x{value:02X}"))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DedupeReport {
    pub before: usize,
    pub after: usize,
    pub removed: usize,
    pub remap: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetReport {
    pub tile_count: usize,
    pub unique_tiles: usize,
    pub tile_bytes: usize,
    pub map_cells: usize,
    pub map_bytes: usize,
    pub attr_bytes: usize,
    pub collision_bytes: usize,
    pub event_bytes: usize,
    pub rle_map_bytes: usize,
    pub max_tile_id: u16,
    pub warnings: Vec<String>,
}

impl BudgetReport {
    pub fn summary(&self) -> String {
        let warn = if self.warnings.is_empty() {
            "OK".to_string()
        } else {
            self.warnings.join("; ")
        };
        format!(
            "tiles {} unique {} tile_bytes {} map {} attr {} col {} evt {} rle_map {} max_tile {} {}",
            self.tile_count,
            self.unique_tiles,
            self.tile_bytes,
            self.map_bytes,
            self.attr_bytes,
            self.collision_bytes,
            self.event_bytes,
            self.rle_map_bytes,
            self.max_tile_id,
            warn
        )
    }
}

fn wrap_or_clip(x: i32, y: i32, w: i32, h: i32, wrap: bool) -> Option<(i32, i32)> {
    if wrap {
        Some((x.rem_euclid(w), y.rem_euclid(h)))
    } else if x >= 0 && y >= 0 && x < w && y < h {
        Some((x, y))
    } else {
        None
    }
}

fn seed_default_tiles(tiles: &mut [Tile]) {
    if let Some(tile) = tiles.get_mut(1) {
        tile.fill(1);
        tile.label = "solid1".to_string();
    }
    if let Some(tile) = tiles.get_mut(2) {
        tile.fill(2);
        tile.label = "solid2".to_string();
    }
    if let Some(tile) = tiles.get_mut(3) {
        tile.fill(3);
        tile.label = "solid3".to_string();
    }
    if let Some(tile) = tiles.get_mut(4) {
        *tile = Tile::checker(0, 1);
        tile.label = "checker".to_string();
    }
    if let Some(tile) = tiles.get_mut(5) {
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let value = if x == 0 || y == 0 || x == TILE_SIZE - 1 || y == TILE_SIZE - 1 {
                    3
                } else {
                    0
                };
                tile.set_pixel_unchecked(x, y, value);
            }
        }
        tile.label = "frame".to_string();
    }
}

fn default_palettes() -> Vec<Vec<PaletteColor>> {
    vec![
        vec![
            PaletteColor::new(230, 238, 218),
            PaletteColor::new(146, 176, 124),
            PaletteColor::new(74, 103, 87),
            PaletteColor::new(24, 32, 40),
        ],
        vec![
            PaletteColor::new(240, 237, 218),
            PaletteColor::new(224, 177, 88),
            PaletteColor::new(174, 92, 61),
            PaletteColor::new(47, 45, 55),
        ],
        vec![
            PaletteColor::new(222, 239, 242),
            PaletteColor::new(76, 183, 192),
            PaletteColor::new(51, 91, 141),
            PaletteColor::new(20, 26, 52),
        ],
        vec![
            PaletteColor::new(238, 226, 235),
            PaletteColor::new(198, 99, 126),
            PaletteColor::new(109, 64, 105),
            PaletteColor::new(31, 26, 40),
        ],
        vec![
            PaletteColor::new(225, 239, 220),
            PaletteColor::new(116, 170, 107),
            PaletteColor::new(61, 118, 83),
            PaletteColor::new(26, 52, 44),
        ],
        vec![
            PaletteColor::new(237, 231, 216),
            PaletteColor::new(172, 157, 128),
            PaletteColor::new(89, 92, 91),
            PaletteColor::new(30, 34, 38),
        ],
        vec![
            PaletteColor::new(239, 240, 225),
            PaletteColor::new(151, 194, 214),
            PaletteColor::new(84, 111, 170),
            PaletteColor::new(32, 35, 67),
        ],
        vec![
            PaletteColor::new(236, 231, 218),
            PaletteColor::new(205, 141, 93),
            PaletteColor::new(129, 77, 70),
            PaletteColor::new(43, 35, 40),
        ],
    ]
}

fn normalize_palettes(palettes: &mut Vec<Vec<PaletteColor>>) {
    if palettes.is_empty() {
        *palettes = default_palettes();
        return;
    }
    while palettes.len() < 8 {
        palettes.push(default_palettes()[palettes.len()].clone());
    }
    palettes.truncate(8);
    let fallback = default_palettes();
    for (index, palette) in palettes.iter_mut().enumerate() {
        while palette.len() < 4 {
            palette.push(fallback[index][palette.len()]);
        }
        palette.truncate(4);
    }
}

fn render_tile_rgba(tile: &Tile, palette: &[PaletteColor]) -> Vec<u8> {
    let mut pixels = vec![0; TILE_PIXELS * 4];
    blit_tile(&mut pixels, TILE_SIZE, 0, 0, tile, palette, false, false);
    pixels
}

#[cfg(feature = "gui")]
fn graphic_pixel_location(
    base_tile: usize,
    size: GraphicSize,
    x: usize,
    y: usize,
) -> Option<(usize, usize, usize)> {
    let (width, height) = size.dimensions();
    if x >= width || y >= height {
        return None;
    }
    let (tile_columns, _) = size.tile_dimensions();
    let tile_index = base_tile
        .checked_add((y / TILE_SIZE) * tile_columns)?
        .checked_add(x / TILE_SIZE)?;
    Some((tile_index, x % TILE_SIZE, y % TILE_SIZE))
}

#[cfg(feature = "gui")]
fn ensure_graphic_tiles(
    project: &mut Project,
    base_tile: usize,
    size: GraphicSize,
) -> StudioResult<usize> {
    let required = base_tile
        .checked_add(size.tile_count())
        .ok_or_else(|| StudioError::Format("graphic tile range overflow".to_string()))?;
    if required > MAX_TILE_ID + 1 {
        return Err(StudioError::Format(format!(
            "{} graphic at tile {} exceeds the 256 tile KITAQGB limit",
            size.label(),
            base_tile
        )));
    }
    let added = required.saturating_sub(project.tiles.len());
    if added > 0 {
        project.tiles.resize_with(required, Tile::blank);
    }
    Ok(added)
}

#[cfg(feature = "gui")]
fn read_graphic_pixels(project: &Project, base_tile: usize, size: GraphicSize) -> Vec<u8> {
    let (width, height) = size.dimensions();
    let mut pixels = vec![0; width * height];
    for y in 0..height {
        for x in 0..width {
            let Some((tile_index, tile_x, tile_y)) = graphic_pixel_location(base_tile, size, x, y)
            else {
                continue;
            };
            if let Ok(tile) = project.tile(tile_index) {
                pixels[y * width + x] = tile.pixels[tile_y * TILE_SIZE + tile_x] & 0x03;
            }
        }
    }
    pixels
}

#[cfg(feature = "gui")]
fn write_graphic_pixels(
    project: &mut Project,
    base_tile: usize,
    size: GraphicSize,
    pixels: &[u8],
) -> StudioResult<usize> {
    let (width, height) = size.dimensions();
    if pixels.len() != width * height {
        return Err(StudioError::Format(format!(
            "{} graphic requires {} pixels, got {}",
            size.label(),
            width * height,
            pixels.len()
        )));
    }
    let added = ensure_graphic_tiles(project, base_tile, size)?;
    for y in 0..height {
        for x in 0..width {
            let Some((tile_index, tile_x, tile_y)) = graphic_pixel_location(base_tile, size, x, y)
            else {
                continue;
            };
            project.tile_mut(tile_index)?.set_pixel_unchecked(
                tile_x,
                tile_y,
                pixels[y * width + x],
            );
        }
    }
    Ok(added)
}

#[cfg(feature = "gui")]
fn stamp_graphic_map(
    project: &mut Project,
    base_tile: usize,
    size: GraphicSize,
    x: usize,
    y: usize,
    palette: u8,
) -> StudioResult<usize> {
    ensure_graphic_tiles(project, base_tile, size)?;
    let (tile_columns, tile_rows) = size.tile_dimensions();
    let mut changed = 0;
    for tile_y in 0..tile_rows {
        for tile_x in 0..tile_columns {
            let map_x = x + tile_x;
            let map_y = y + tile_y;
            let Ok(mut cell) = project.map_cell(map_x, map_y) else {
                continue;
            };
            cell.tile = (base_tile + tile_y * tile_columns + tile_x) as u16;
            cell.palette = palette & 0x07;
            project.set_map_cell(map_x, map_y, cell)?;
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(feature = "gui")]
fn flip_graphic_x(pixels: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut output = vec![0; pixels.len()];
    for y in 0..height {
        for x in 0..width {
            output[y * width + x] = pixels[y * width + (width - 1 - x)];
        }
    }
    output
}

#[cfg(feature = "gui")]
fn flip_graphic_y(pixels: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut output = vec![0; pixels.len()];
    for y in 0..height {
        for x in 0..width {
            output[y * width + x] = pixels[(height - 1 - y) * width + x];
        }
    }
    output
}

#[cfg(feature = "gui")]
fn rotate_graphic_right(pixels: &[u8], width: usize, height: usize) -> Vec<u8> {
    let output_width = height;
    let mut output = vec![0; pixels.len()];
    for y in 0..height {
        for x in 0..width {
            let output_x = height - 1 - y;
            let output_y = x;
            output[output_y * output_width + output_x] = pixels[y * width + x];
        }
    }
    output
}

fn blit_tile(
    pixels: &mut [u8],
    surface_width: usize,
    ox: usize,
    oy: usize,
    tile: &Tile,
    palette: &[PaletteColor],
    flip_x: bool,
    flip_y: bool,
) {
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let sx = if flip_x { TILE_SIZE - 1 - x } else { x };
            let sy = if flip_y { TILE_SIZE - 1 - y } else { y };
            let color_index = tile.pixels[sy * TILE_SIZE + sx] as usize;
            let color = palette[color_index.min(palette.len().saturating_sub(1))];
            let offset = ((oy + y) * surface_width + ox + x) * 4;
            if offset + 3 < pixels.len() {
                pixels[offset] = color.r;
                pixels[offset + 1] = color.g;
                pixels[offset + 2] = color.b;
                pixels[offset + 3] = 255;
            }
        }
    }
}

fn tint_tile(pixels: &mut [u8], surface_width: usize, ox: usize, oy: usize, rgba: [u8; 4]) {
    let alpha = rgba[3] as u16;
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let offset = ((oy + y) * surface_width + ox + x) * 4;
            if offset + 3 >= pixels.len() {
                continue;
            }
            pixels[offset] = blend_channel(pixels[offset], rgba[0], alpha);
            pixels[offset + 1] = blend_channel(pixels[offset + 1], rgba[1], alpha);
            pixels[offset + 2] = blend_channel(pixels[offset + 2], rgba[2], alpha);
        }
    }
}

fn blend_channel(base: u8, overlay: u8, alpha: u16) -> u8 {
    (((base as u16 * (255 - alpha)) + (overlay as u16 * alpha)) / 255) as u8
}

pub fn rle_encode(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let value = bytes[index];
        let mut count = 1usize;
        while index + count < bytes.len() && bytes[index + count] == value && count < 255 {
            count += 1;
        }
        out.push(count as u8);
        out.push(value);
        index += count;
    }
    out
}

fn push_u8_array(out: &mut String, name: &str, bytes: &[u8], per_line: usize) {
    out.push_str(&format!(
        "__prg_rom u8 {name}[{}] = {{\n",
        bytes.len().max(1)
    ));
    if bytes.is_empty() {
        out.push_str("    0x00,\n");
    }
    for chunk in bytes.chunks(per_line) {
        out.push_str("    ");
        for (index, value) in chunk.iter().enumerate() {
            if index > 0 {
                out.push(' ');
            }
            out.push_str(&format!("0x{value:02X},"));
        }
        out.push('\n');
    }
    out.push_str("};\n\n");
}

fn push_u16_array(out: &mut String, name: &str, values: &[u16], per_line: usize) {
    out.push_str(&format!(
        "__prg_rom u16 {name}[{}] = {{\n",
        values.len().max(1)
    ));
    if values.is_empty() {
        out.push_str("    0x0000,\n");
    }
    for chunk in values.chunks(per_line) {
        out.push_str("    ");
        for (index, value) in chunk.iter().enumerate() {
            if index > 0 {
                out.push(' ');
            }
            out.push_str(&format!("0x{value:04X},"));
        }
        out.push('\n');
    }
    out.push_str("};\n\n");
}

fn sanitize_c_ident(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else if ch == '-' || ch == ' ' {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("kq_asset");
    }
    if out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

#[cfg(feature = "gui")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GraphicSize {
    Size8x8,
    Size16x16,
    Size32x32,
    Size8x16,
    Size16x8,
    Size16x32,
    Size32x16,
}

#[cfg(feature = "gui")]
impl GraphicSize {
    const ALL: [Self; 7] = [
        Self::Size8x8,
        Self::Size16x16,
        Self::Size32x32,
        Self::Size8x16,
        Self::Size16x8,
        Self::Size16x32,
        Self::Size32x16,
    ];

    fn dimensions(self) -> (usize, usize) {
        match self {
            Self::Size8x8 => (8, 8),
            Self::Size16x16 => (16, 16),
            Self::Size32x32 => (32, 32),
            Self::Size8x16 => (8, 16),
            Self::Size16x8 => (16, 8),
            Self::Size16x32 => (16, 32),
            Self::Size32x16 => (32, 16),
        }
    }

    #[cfg(any(test, feature = "python"))]
    fn from_dimensions(width: usize, height: usize) -> StudioResult<Self> {
        Self::ALL
            .into_iter()
            .find(|size| size.dimensions() == (width, height))
            .ok_or_else(|| {
                StudioError::Format(format!(
                    "unsupported graphic size {width}x{height}; use 8x8, 16x16, 32x32, 8x16, 16x8, 16x32, or 32x16"
                ))
            })
    }

    fn tile_dimensions(self) -> (usize, usize) {
        let (width, height) = self.dimensions();
        (width / TILE_SIZE, height / TILE_SIZE)
    }

    fn tile_count(self) -> usize {
        let (columns, rows) = self.tile_dimensions();
        columns * rows
    }

    fn label(self) -> &'static str {
        match self {
            Self::Size8x8 => "8x8",
            Self::Size16x16 => "16x16",
            Self::Size32x32 => "32x32",
            Self::Size8x16 => "8x16",
            Self::Size16x8 => "16x8",
            Self::Size16x32 => "16x32",
            Self::Size32x16 => "32x16",
        }
    }

    fn rotated(self) -> Self {
        match self {
            Self::Size8x16 => Self::Size16x8,
            Self::Size16x8 => Self::Size8x16,
            Self::Size16x32 => Self::Size32x16,
            Self::Size32x16 => Self::Size16x32,
            square => square,
        }
    }
}

#[cfg(feature = "python")]
mod python_api {
    use super::*;
    use plita::python::{self as plita_python, prelude::*};
    use pyo3::exceptions::PyValueError;

    #[pyclass(name = "BudgetReport")]
    #[derive(Clone)]
    pub struct PyBudgetReport {
        inner: BudgetReport,
    }

    #[pymethods]
    impl PyBudgetReport {
        #[getter]
        fn tile_count(&self) -> usize {
            self.inner.tile_count
        }

        #[getter]
        fn unique_tiles(&self) -> usize {
            self.inner.unique_tiles
        }

        #[getter]
        fn tile_bytes(&self) -> usize {
            self.inner.tile_bytes
        }

        #[getter]
        fn map_cells(&self) -> usize {
            self.inner.map_cells
        }

        #[getter]
        fn map_bytes(&self) -> usize {
            self.inner.map_bytes
        }

        #[getter]
        fn attr_bytes(&self) -> usize {
            self.inner.attr_bytes
        }

        #[getter]
        fn collision_bytes(&self) -> usize {
            self.inner.collision_bytes
        }

        #[getter]
        fn event_bytes(&self) -> usize {
            self.inner.event_bytes
        }

        #[getter]
        fn rle_map_bytes(&self) -> usize {
            self.inner.rle_map_bytes
        }

        #[getter]
        fn max_tile_id(&self) -> u16 {
            self.inner.max_tile_id
        }

        #[getter]
        fn warnings(&self) -> Vec<String> {
            self.inner.warnings.clone()
        }

        fn summary(&self) -> String {
            self.inner.summary()
        }

        fn __repr__(&self) -> String {
            format!("BudgetReport({})", self.inner.summary())
        }
    }

    #[pyclass(name = "DedupeReport")]
    #[derive(Clone)]
    pub struct PyDedupeReport {
        inner: DedupeReport,
    }

    #[pymethods]
    impl PyDedupeReport {
        #[getter]
        fn before(&self) -> usize {
            self.inner.before
        }

        #[getter]
        fn after(&self) -> usize {
            self.inner.after
        }

        #[getter]
        fn removed(&self) -> usize {
            self.inner.removed
        }

        #[getter]
        fn remap(&self) -> Vec<u16> {
            self.inner.remap.clone()
        }

        fn __repr__(&self) -> String {
            format!(
                "DedupeReport(before={}, after={}, removed={})",
                self.inner.before, self.inner.after, self.inner.removed
            )
        }
    }

    #[pyclass(name = "Project")]
    pub struct PyProject {
        project: Project,
    }

    impl PyProject {
        fn py_result<T>(result: StudioResult<T>) -> PyResult<T> {
            result.map_err(|err| PyValueError::new_err(err.to_string()))
        }
    }

    #[pymethods]
    impl PyProject {
        #[new]
        #[pyo3(signature = (width = DEFAULT_MAP_W, height = DEFAULT_MAP_H, tile_count = DEFAULT_TILE_COUNT))]
        fn new(width: u8, height: u8, tile_count: usize) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::new(width, height, tile_count))?,
            })
        }

        #[staticmethod]
        fn load(path: &str) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::load_project_file(path))?,
            })
        }

        #[staticmethod]
        #[pyo3(signature = (path, mode = "cgb", width = None, height = None))]
        fn from_image(
            path: &str,
            mode: &str,
            width: Option<u32>,
            height: Option<u32>,
        ) -> PyResult<Self> {
            let resize = match (width, height) {
                (None, None) => None,
                (Some(w), Some(h)) => Some((w, h)),
                _ => {
                    return Err(PyValueError::new_err(
                        "width and height must be supplied together",
                    ))
                }
            };
            let options = image_io::ImportOptions {
                mode: Self::py_result(mode.parse())?,
                resize,
            };
            let (project, _) = Self::py_result(image_io::import_png(path, &options))?;
            Ok(Self { project })
        }

        #[pyo3(signature = (path, mode = "cgb", scale = 1))]
        fn save_preview(&self, path: &str, mode: &str, scale: u32) -> PyResult<()> {
            Self::py_result(image_io::preview_png(
                &self.project,
                path,
                Self::py_result(mode.parse())?,
                scale,
            ))
        }

        fn validate_json(&self) -> PyResult<String> {
            serde_json::to_string(&image_io::validate(&self.project))
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }

        #[staticmethod]
        fn load_gbtd(path: &str) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::load_gbtd_file(path))?,
            })
        }

        #[staticmethod]
        fn load_gbmb(path: &str) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::load_gbmb_file(path))?,
            })
        }

        #[staticmethod]
        fn from_json(text: &str) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::from_json_str(text))?,
            })
        }

        #[staticmethod]
        fn from_gbtd_bytes(bytes: &Bound<'_, PyBytes>) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::from_gbtd_bytes(bytes.as_bytes()))?,
            })
        }

        #[staticmethod]
        fn from_gbmb_bytes(bytes: &Bound<'_, PyBytes>) -> PyResult<Self> {
            Ok(Self {
                project: Self::py_result(Project::from_gbmb_bytes(bytes.as_bytes()))?,
            })
        }

        fn save(&self, path: &str) -> PyResult<()> {
            Self::py_result(self.project.save_project_file(path))
        }

        fn save_gbtd(&self, path: &str) -> PyResult<()> {
            Self::py_result(self.project.save_gbtd_file(path))
        }

        fn save_gbmb(&self, path: &str) -> PyResult<()> {
            Self::py_result(self.project.save_gbmb_file(path))
        }

        fn to_json(&self) -> PyResult<String> {
            Self::py_result(self.project.to_json_pretty())
        }

        fn gbtd_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
            let bytes = Self::py_result(self.project.to_gbtd_bytes())?;
            Ok(plita_python::py_bytes(py, &bytes))
        }

        #[pyo3(signature = (tile_file = AUTOEXPORT_GBR))]
        fn gbmb_bytes<'py>(
            &self,
            py: Python<'py>,
            tile_file: &str,
        ) -> PyResult<Bound<'py, PyBytes>> {
            let bytes = Self::py_result(self.project.to_gbmb_bytes(tile_file))?;
            Ok(plita_python::py_bytes(py, &bytes))
        }

        #[getter]
        fn name(&self) -> String {
            self.project.name.clone()
        }

        #[setter]
        fn set_name(&mut self, name: String) {
            self.project.name = name;
        }

        #[getter]
        fn width(&self) -> u8 {
            self.project.map_width
        }

        #[getter]
        fn height(&self) -> u8 {
            self.project.map_height
        }

        #[getter]
        fn tile_count(&self) -> usize {
            self.project.tiles.len()
        }

        fn resize_map(&mut self, width: u8, height: u8) -> PyResult<()> {
            Self::py_result(self.project.resize_map(width, height))
        }

        fn add_tile(&mut self) -> u16 {
            self.project.add_tile(Tile::blank())
        }

        fn duplicate_tile(&mut self, index: usize) -> PyResult<u16> {
            Self::py_result(self.project.duplicate_tile(index))
        }

        fn get_tile_pixel(&self, tile: usize, x: usize, y: usize) -> PyResult<u8> {
            let tile = Self::py_result(self.project.tile(tile))?;
            Self::py_result(tile.pixel(x, y))
        }

        fn set_tile_pixel(&mut self, tile: usize, x: usize, y: usize, value: u8) -> PyResult<()> {
            Self::py_result(self.project.set_tile_pixel(tile, x, y, value))
        }

        fn fill_tile(&mut self, tile: usize, value: u8) -> PyResult<()> {
            Self::py_result(self.project.tile_mut(tile).map(|tile| tile.fill(value)))
        }

        fn get_graphic_pixel(
            &self,
            base_tile: usize,
            width: usize,
            height: usize,
            x: usize,
            y: usize,
        ) -> PyResult<u8> {
            let size = Self::py_result(GraphicSize::from_dimensions(width, height))?;
            let (tile_index, tile_x, tile_y) = graphic_pixel_location(base_tile, size, x, y)
                .ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "coordinate outside {width}x{height}: ({x}, {y})"
                    ))
                })?;
            let tile = Self::py_result(self.project.tile(tile_index))?;
            Self::py_result(tile.pixel(tile_x, tile_y))
        }

        fn set_graphic_pixel(
            &mut self,
            base_tile: usize,
            width: usize,
            height: usize,
            x: usize,
            y: usize,
            value: u8,
        ) -> PyResult<()> {
            let size = Self::py_result(GraphicSize::from_dimensions(width, height))?;
            let (tile_index, tile_x, tile_y) = graphic_pixel_location(base_tile, size, x, y)
                .ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "coordinate outside {width}x{height}: ({x}, {y})"
                    ))
                })?;
            Self::py_result(ensure_graphic_tiles(&mut self.project, base_tile, size))?;
            Self::py_result(
                self.project
                    .set_tile_pixel(tile_index, tile_x, tile_y, value),
            )
        }

        fn fill_graphic(
            &mut self,
            base_tile: usize,
            width: usize,
            height: usize,
            value: u8,
        ) -> PyResult<usize> {
            let size = Self::py_result(GraphicSize::from_dimensions(width, height))?;
            let pixels = vec![value & 0x03; width * height];
            Self::py_result(write_graphic_pixels(
                &mut self.project,
                base_tile,
                size,
                &pixels,
            ))
        }

        #[pyo3(signature = (x, y, base_tile, width, height, palette = 0))]
        fn stamp_graphic(
            &mut self,
            x: usize,
            y: usize,
            base_tile: usize,
            width: usize,
            height: usize,
            palette: u8,
        ) -> PyResult<usize> {
            let size = Self::py_result(GraphicSize::from_dimensions(width, height))?;
            Self::py_result(stamp_graphic_map(
                &mut self.project,
                base_tile,
                size,
                x,
                y,
                palette,
            ))
        }

        fn tile_bytecode(&self, tile: usize) -> PyResult<String> {
            let tile = Self::py_result(self.project.tile(tile))?;
            Ok(format_tile_bytecode(&tile.encoded_2bpp()))
        }

        fn set_tile_bytecode(&mut self, tile: usize, text: &str) -> PyResult<()> {
            let bytes = Self::py_result(parse_tile_bytecode(text))?;
            Self::py_result(self.project.set_tile_from_2bpp_bytes(tile, &bytes))
        }

        fn set_tile_2bpp_bytes(&mut self, tile: usize, bytes: &Bound<'_, PyBytes>) -> PyResult<()> {
            Self::py_result(
                self.project
                    .set_tile_from_2bpp_bytes(tile, bytes.as_bytes()),
            )
        }

        fn replace_color(&mut self, tile: usize, from: u8, to: u8) -> PyResult<usize> {
            Self::py_result(
                self.project
                    .tile_mut(tile)
                    .map(|tile| tile.replace_color(from, to)),
            )
        }

        fn flip_tile_x(&mut self, tile: usize) -> PyResult<()> {
            Self::py_result(self.project.tile_mut(tile).map(Tile::flip_x))
        }

        fn flip_tile_y(&mut self, tile: usize) -> PyResult<()> {
            Self::py_result(self.project.tile_mut(tile).map(Tile::flip_y))
        }

        fn rotate_tile_right(&mut self, tile: usize) -> PyResult<()> {
            Self::py_result(self.project.tile_mut(tile).map(Tile::rotate_right))
        }

        #[pyo3(signature = (tile, dx, dy, wrap = true))]
        fn shift_tile(&mut self, tile: usize, dx: i32, dy: i32, wrap: bool) -> PyResult<()> {
            Self::py_result(
                self.project
                    .tile_mut(tile)
                    .map(|tile| tile.shift(dx, dy, wrap)),
            )
        }

        fn map_cell(
            &self,
            x: usize,
            y: usize,
        ) -> PyResult<(u16, u8, u8, bool, bool, bool, u8, u8)> {
            let cell = Self::py_result(self.project.map_cell(x, y))?;
            Ok((
                cell.tile,
                cell.palette,
                cell.vram_bank,
                cell.flip_x,
                cell.flip_y,
                cell.priority,
                cell.collision,
                cell.event,
            ))
        }

        fn set_map_tile(&mut self, x: usize, y: usize, tile: u16) -> PyResult<()> {
            Self::py_result(self.project.set_map_tile(x, y, tile))
        }

        #[pyo3(signature = (
            x,
            y,
            tile,
            palette = 0,
            vram_bank = 0,
            flip_x = false,
            flip_y = false,
            priority = false,
            collision = 0,
            event = 0
        ))]
        fn set_map_cell(
            &mut self,
            x: usize,
            y: usize,
            tile: u16,
            palette: u8,
            vram_bank: u8,
            flip_x: bool,
            flip_y: bool,
            priority: bool,
            collision: u8,
            event: u8,
        ) -> PyResult<()> {
            Self::py_result(self.project.set_map_cell(
                x,
                y,
                MapCell {
                    tile,
                    palette,
                    vram_bank,
                    flip_x,
                    flip_y,
                    priority,
                    collision,
                    event,
                },
            ))
        }

        #[pyo3(signature = (x, y, w, h, tile, palette = 0, collision = 0, event = 0))]
        fn paint_rect(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            tile: u16,
            palette: u8,
            collision: u8,
            event: u8,
        ) -> usize {
            self.project.paint_rect(
                x,
                y,
                w,
                h,
                MapCell {
                    tile,
                    palette,
                    collision,
                    event,
                    ..MapCell::default()
                },
            )
        }

        #[pyo3(signature = (x, y, tile, palette = 0, collision = 0, event = 0))]
        fn flood_fill(
            &mut self,
            x: usize,
            y: usize,
            tile: u16,
            palette: u8,
            collision: u8,
            event: u8,
        ) -> PyResult<usize> {
            Self::py_result(self.project.flood_fill_map(
                x,
                y,
                MapCell {
                    tile,
                    palette,
                    collision,
                    event,
                    ..MapCell::default()
                },
            ))
        }

        #[pyo3(signature = (threshold = 0.35))]
        fn auto_collision(&mut self, threshold: f32) -> usize {
            self.project.auto_tag_collision_by_density(threshold)
        }

        fn autotile(&mut self, tiles_by_mask: Vec<u16>) -> PyResult<usize> {
            Self::py_result(self.project.apply_neighbor_autotile(&tiles_by_mask))
        }

        fn dedupe_tiles(&mut self) -> PyDedupeReport {
            PyDedupeReport {
                inner: self.project.dedupe_tiles(),
            }
        }

        fn budget(&self) -> PyBudgetReport {
            PyBudgetReport {
                inner: self.project.budget_report(),
            }
        }

        #[pyo3(signature = (prefix = "kq_asset"))]
        fn export_c(&self, prefix: &str) -> PyResult<String> {
            Self::py_result(self.project.export_kitaqgb_c(prefix))
        }

        #[pyo3(signature = (path, prefix = "kq_asset"))]
        fn save_c(&self, path: &str, prefix: &str) -> PyResult<()> {
            let text = Self::py_result(self.project.export_kitaqgb_c(prefix))?;
            fs::write(path, text).map_err(|err| PyValueError::new_err(err.to_string()))
        }

        #[pyo3(signature = (palette = 0))]
        fn render_map_rgba<'py>(&self, py: Python<'py>, palette: usize) -> Bound<'py, PyBytes> {
            let _ = palette;
            plita_python::py_bytes(py, &self.project.render_map_rgba())
        }

        #[pyo3(signature = (tile, palette = 0))]
        fn render_tile_rgba<'py>(
            &self,
            py: Python<'py>,
            tile: usize,
            palette: usize,
        ) -> PyResult<Bound<'py, PyBytes>> {
            let pixels = Self::py_result(self.project.render_tile_rgba(tile, palette))?;
            Ok(plita_python::py_bytes(py, &pixels))
        }

        fn tile_bytes_2bpp<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
            plita_python::py_bytes(py, &self.project.tile_bytes_2bpp())
        }

        fn map_tile_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
            let bytes = Self::py_result(self.project.map_tile_bytes())?;
            Ok(plita_python::py_bytes(py, &bytes))
        }

        fn map_attr_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
            plita_python::py_bytes(py, &self.project.map_attr_bytes())
        }

        fn collision_bits<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
            plita_python::py_bytes(py, &self.project.collision_bits())
        }

        fn event_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
            plita_python::py_bytes(py, &self.project.event_bytes())
        }

        fn __repr__(&self) -> String {
            format!(
                "Project(name={:?}, size={}x{}, tiles={})",
                self.project.name,
                self.project.map_width,
                self.project.map_height,
                self.project.tiles.len()
            )
        }
    }

    #[pyfunction(name = "run_gui")]
    fn run_gui_py(py: Python<'_>) -> PyResult<()> {
        py.allow_threads(super::run_gui)
            .map_err(plita_python::app_error_to_py_err)
    }

    #[pymodule]
    fn gbhua(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
        plita_python::add_plita_classes(module)?;
        module.add_class::<PyProject>()?;
        module.add_class::<PyBudgetReport>()?;
        module.add_class::<PyDedupeReport>()?;
        module.add_function(wrap_pyfunction!(run_gui_py, module)?)?;
        module.add("TILE_SIZE", TILE_SIZE)?;
        module.add("TILE_BYTES_2BPP", TILE_BYTES_2BPP)?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn python_project_mutates_without_window() {
            let mut project = PyProject::new(8, 8, 8).expect("project");
            project.set_tile_pixel(0, 1, 2, 3).expect("pixel");
            project.set_map_tile(2, 3, 4).expect("map");
            project.fill_graphic(4, 16, 16, 1).expect("fill graphic");
            project
                .set_graphic_pixel(4, 16, 16, 9, 10, 2)
                .expect("graphic pixel");
            assert_eq!(
                project
                    .get_graphic_pixel(4, 16, 16, 9, 10)
                    .expect("read graphic pixel"),
                2
            );
            assert_eq!(
                project
                    .stamp_graphic(0, 0, 4, 16, 16, 3)
                    .expect("stamp graphic"),
                4
            );

            assert_eq!(project.get_tile_pixel(0, 1, 2).expect("read"), 3);
            assert_eq!(project.map_cell(2, 3).expect("cell").0, 4);
            assert_eq!(project.map_cell(1, 1).expect("graphic cell").0, 7);
            assert!(project.budget().summary().contains("tiles 8"));
        }

        #[test]
        fn python_project_exercises_ai_export_and_preview_api() {
            let base = std::env::temp_dir().join(format!("gbhua_py_{}", std::process::id()));
            let json_path = base.with_extension("gbh");
            let c_path = base.with_extension("c");
            let gbr_path = base.with_extension("gbr");
            let gbm_path = base.with_extension("gbm");
            let _ = std::fs::remove_file(&json_path);
            let _ = std::fs::remove_file(&c_path);
            let _ = std::fs::remove_file(&gbr_path);
            let _ = std::fs::remove_file(&gbm_path);

            let mut project = PyProject::new(12, 10, 24).expect("project");
            project.set_name("ai_room".to_string());
            project.resize_map(10, 9).expect("resize");
            let added = project.add_tile();
            let duplicate = project.duplicate_tile(0).expect("duplicate");
            assert!(duplicate > added);

            project.fill_tile(added as usize, 1).expect("fill tile");
            assert_eq!(
                project
                    .replace_color(added as usize, 1, 3)
                    .expect("replace"),
                TILE_PIXELS
            );
            project
                .set_tile_bytecode(
                    added as usize,
                    "0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00",
                )
                .expect("set bytecode");
            assert!(project
                .tile_bytecode(added as usize)
                .expect("tile bytecode")
                .contains("0x78"));
            project.flip_tile_x(added as usize).expect("flip x");
            project.flip_tile_y(added as usize).expect("flip y");
            project
                .rotate_tile_right(added as usize)
                .expect("rotate right");
            project
                .shift_tile(added as usize, 1, -1, true)
                .expect("shift");

            project
                .set_map_cell(1, 1, added, 2, 1, true, false, true, 1, 9)
                .expect("cell");
            assert_eq!(project.map_cell(1, 1).expect("read").0, added);
            assert_eq!(project.paint_rect(2, 2, 3, 2, added, 1, 1, 0), 6);
            assert!(project.flood_fill(8, 8, 4, 0, 0, 0).expect("fill") > 0);
            assert!(project.auto_collision(0.20) > 0);

            let tiles_by_mask: Vec<u16> = (0..16).collect();
            let _ = project.autotile(tiles_by_mask).expect("autotile");
            let dedupe = project.dedupe_tiles();
            assert!(dedupe.before() >= dedupe.after());
            assert!(project.budget().tile_count() > 0);
            assert!(project
                .export_c("ai_room")
                .expect("export")
                .contains("ai_room_load"));

            let json_text = project.to_json().expect("json");
            let from_json = PyProject::from_json(&json_text).expect("from json");
            assert_eq!(from_json.width(), project.width());
            let json_path_text = json_path.to_string_lossy().to_string();
            let c_path_text = c_path.to_string_lossy().to_string();
            let gbr_path_text = gbr_path.to_string_lossy().to_string();
            let gbm_path_text = gbm_path.to_string_lossy().to_string();
            project.save(&json_path_text).expect("save json");
            let loaded = PyProject::load(&json_path_text).expect("load json");
            assert_eq!(loaded.height(), project.height());
            project.save_c(&c_path_text, "ai_room").expect("save c");
            assert!(std::fs::read_to_string(&c_path)
                .expect("read c")
                .contains("ai_room_map"));
            project.save_gbtd(&gbr_path_text).expect("save gbtd");
            let loaded_tiles = PyProject::load_gbtd(&gbr_path_text).expect("load gbtd");
            assert_eq!(loaded_tiles.tile_count(), project.tile_count());
            project.save_gbmb(&gbm_path_text).expect("save gbmb");
            let loaded_map = PyProject::load_gbmb(&gbm_path_text).expect("load gbmb");
            assert_eq!(loaded_map.width(), project.width());
            assert_eq!(loaded_map.height(), project.height());

            pyo3::prepare_freethreaded_python();
            Python::with_gil(|py| {
                let gbtd = project.gbtd_bytes(py).expect("gbtd bytes");
                assert!(gbtd.as_bytes().starts_with(b"GBO"));
                let loaded = PyProject::from_gbtd_bytes(&gbtd).expect("from gbtd bytes");
                assert_eq!(loaded.tile_count(), project.tile_count());
                let gbmb = project.gbmb_bytes(py, "ai_room.gbr").expect("gbmb bytes");
                assert!(gbmb.as_bytes().starts_with(b"GBO1"));
                let loaded = PyProject::from_gbmb_bytes(&gbmb).expect("from gbmb bytes");
                assert_eq!(loaded.width(), project.width());
                assert_eq!(
                    project
                        .render_tile_rgba(py, 0, 0)
                        .expect("tile rgba")
                        .as_bytes()
                        .len(),
                    TILE_PIXELS * 4
                );
                assert_eq!(
                    project.render_map_rgba(py, 0).as_bytes().len(),
                    project.width() as usize * project.height() as usize * TILE_PIXELS * 4
                );
                assert_eq!(
                    project.tile_bytes_2bpp(py).as_bytes().len(),
                    project.tile_count() * TILE_BYTES_2BPP
                );
                assert_eq!(
                    project.map_tile_bytes(py).expect("map").as_bytes().len(),
                    project.width() as usize * project.height() as usize
                );
                assert_eq!(
                    project.map_attr_bytes(py).as_bytes().len(),
                    project.width() as usize * project.height() as usize
                );
                assert_eq!(
                    project.collision_bits(py).as_bytes().len(),
                    (project.width() as usize * project.height() as usize + 7) / 8
                );
                assert_eq!(
                    project.event_bytes(py).as_bytes().len(),
                    project.width() as usize * project.height() as usize
                );
            });

            let _ = std::fs::remove_file(&json_path);
            let _ = std::fs::remove_file(&c_path);
            let _ = std::fs::remove_file(&gbr_path);
            let _ = std::fs::remove_file(&gbm_path);
            let _ = std::fs::remove_file(gbm_path.with_extension("gbr"));
        }
    }
}
