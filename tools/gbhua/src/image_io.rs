//! Shared, deterministic PNG conversion for the GUI, Python bindings and CLI.
use std::collections::BTreeMap;
use std::str::FromStr;

use image::{imageops::FilterType, ImageReader, RgbaImage};

use crate::*;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageMode {
    Dmg,
    #[default]
    Cgb,
}

impl FromStr for ImageMode {
    type Err = StudioError;

    fn from_str(value: &str) -> StudioResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "dmg" => Ok(Self::Dmg),
            "cgb" => Ok(Self::Cgb),
            _ => Err(StudioError::Format("mode must be dmg or cgb".into())),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImportOptions {
    pub mode: ImageMode,
    pub resize: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportReport {
    pub source_size: [u32; 2],
    pub output_size: [u32; 2],
    pub mode: ImageMode,
    pub tile_count: usize,
    pub map_cells: usize,
    pub reused_tiles: usize,
    pub palette_count: usize,
    pub mean_squared_error: f64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

fn format_error(message: impl Into<String>) -> StudioError {
    StudioError::Format(message.into())
}

fn image_error(error: image::ImageError) -> StudioError {
    format_error(format!("image: {error}"))
}

fn check_dimensions(width: u32, height: u32) -> StudioResult<()> {
    if width == 0
        || height == 0
        || width > 2040
        || height > 2040
        || width % 8 != 0
        || height % 8 != 0
    {
        return Err(format_error(
            "image size must be multiples of 8 in 8..=2040; use an explicit resize",
        ));
    }
    Ok(())
}

fn rgb555(value: u8) -> u8 {
    let channel = (u32::from(value) * 31 + 127) / 255;
    ((channel * 255 + 15) / 31) as u8
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u64 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| {
            let delta = i64::from(a) - i64::from(b);
            (delta * delta) as u64
        })
        .sum()
}

fn nearest(color: [u8; 3], palette: &[[u8; 3]; 4]) -> (u8, u64) {
    palette
        .iter()
        .enumerate()
        .map(|(index, entry)| (index as u8, distance(color, *entry)))
        .min_by_key(|entry| entry.1)
        .expect("four colors")
}

// Weighted median cut on each 8x8 tile. Stable ordering makes identical inputs reproducible.
fn tile_palette(pixels: &[[u8; 3]]) -> [[u8; 3]; 4] {
    let mut histogram = BTreeMap::<[u8; 3], u32>::new();
    for color in pixels {
        *histogram.entry(color.map(rgb555)).or_default() += 1;
    }
    let mut groups = vec![histogram.into_iter().collect::<Vec<_>>()];
    while groups.len() < 4 {
        let Some(index) = groups
            .iter()
            .enumerate()
            .filter(|(_, group)| group.len() > 1)
            .max_by_key(|(_, group)| group.iter().map(|entry| entry.1).sum::<u32>())
            .map(|(index, _)| index)
        else {
            break;
        };
        let group = &mut groups[index];
        let channel = (0..3)
            .max_by_key(|&channel| {
                group.iter().map(|entry| entry.0[channel]).max().unwrap()
                    - group.iter().map(|entry| entry.0[channel]).min().unwrap()
            })
            .unwrap();
        group.sort_by_key(|entry| (entry.0[channel], entry.0));
        let half = group.iter().map(|entry| entry.1).sum::<u32>().div_ceil(2);
        let mut count = 0;
        let split = group
            .iter()
            .position(|entry| {
                count += entry.1;
                count >= half
            })
            .unwrap_or(0)
            .saturating_add(1)
            .min(group.len() - 1);
        let other = group.split_off(split);
        groups.push(other);
    }
    let mut colors = groups
        .iter()
        .map(|group| {
            let count = group.iter().map(|entry| entry.1).sum::<u32>();
            std::array::from_fn(|channel| {
                rgb555(
                    ((group
                        .iter()
                        .map(|entry| u32::from(entry.0[channel]) * entry.1)
                        .sum::<u32>()
                        + count / 2)
                        / count) as u8,
                )
            })
        })
        .collect::<Vec<[u8; 3]>>();
    colors.sort_by_key(|c| {
        (
            std::cmp::Reverse(
                u32::from(c[0]) * 299 + u32::from(c[1]) * 587 + u32::from(c[2]) * 114,
            ),
            *c,
        )
    });
    let fill = colors[0];
    colors.resize(4, fill);
    colors.try_into().expect("four colors")
}

struct PaletteCandidate {
    palette: [[u8; 3]; 4],
    tile_count: usize,
    colors: Vec<([u8; 3], u32)>,
}

impl PaletteCandidate {
    fn error(&self, palette: &[[u8; 3]; 4]) -> u64 {
        self.colors
            .iter()
            .map(|(color, count)| nearest(*color, palette).1 * u64::from(*count))
            .sum()
    }
}

fn automatic_palettes(tiles: &[Vec<[u8; 3]>], warnings: &mut Vec<String>) -> Vec<[[u8; 3]; 4]> {
    let mut groups = BTreeMap::<[[u8; 3]; 4], (usize, BTreeMap<[u8; 3], u32>)>::new();
    for pixels in tiles {
        let (count, colors) = groups.entry(tile_palette(pixels)).or_default();
        *count += 1;
        for color in pixels {
            *colors.entry(*color).or_default() += 1;
        }
    }
    let mut candidates = groups
        .into_iter()
        .map(|(palette, (tile_count, colors))| PaletteCandidate {
            palette,
            tile_count,
            colors: colors.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|c| (std::cmp::Reverse(c.tile_count), c.palette));
    let baseline = candidates
        .iter()
        .take(8)
        .map(|c| c.palette)
        .collect::<Vec<_>>();
    if candidates.len() <= 8 {
        return baseline;
    }
    warnings.push(format!(
        "{} candidate palettes reduced to 8; inspect the preview.",
        candidates.len()
    ));

    // Give scarce palette slots to colors not represented yet, including rare
    // highlights/outlines. Histogram weights retain the importance of large areas.
    let intrinsic = candidates
        .iter()
        .map(|c| c.error(&c.palette))
        .collect::<Vec<_>>();
    let mut errors = candidates
        .iter()
        .map(|c| c.error(&candidates[0].palette))
        .collect::<Vec<_>>();
    let mut selected = vec![0];
    while selected.len() < 8 {
        let next = (0..candidates.len())
            .filter(|i| !selected.contains(i))
            .max_by_key(|&i| (errors[i].saturating_sub(intrinsic[i]), std::cmp::Reverse(i)))
            .expect("more than eight candidates");
        selected.push(next);
        for (candidate, error) in candidates.iter().zip(&mut errors) {
            *error = (*error).min(candidate.error(&candidates[next].palette));
        }
    }
    let proposed = selected
        .iter()
        .map(|&i| candidates[i].palette)
        .collect::<Vec<_>>();
    let image_error = |palettes: &Vec<[[u8; 3]; 4]>| {
        tiles
            .iter()
            .map(|pixels| {
                palettes
                    .iter()
                    .map(|palette| {
                        pixels
                            .iter()
                            .map(|color| nearest(*color, palette).1)
                            .sum::<u64>()
                    })
                    .min()
                    .expect("nonempty palettes")
            })
            .sum::<u64>()
    };
    // Compare real per-tile assignments so the new selection cannot increase MSE.
    if image_error(&proposed) <= image_error(&baseline) {
        proposed
    } else {
        baseline
    }
}

pub fn import_png(
    path: impl AsRef<Path>,
    options: &ImportOptions,
) -> StudioResult<(Project, ImportReport)> {
    let path = path.as_ref();
    let mut reader = ImageReader::open(path)?;
    reader.set_format(image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let source = reader.decode().map_err(image_error)?.to_rgba8();
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    import_rgba(&source, name, options)
}

pub fn import_rgba(
    source: &RgbaImage,
    name: &str,
    options: &ImportOptions,
) -> StudioResult<(Project, ImportReport)> {
    let (width, height) = options.resize.unwrap_or(source.dimensions());
    check_dimensions(width, height)?;
    if source.width() == 0 || source.height() == 0 {
        return Err(format_error("source image is empty"));
    }
    let resized;
    let input = if source.dimensions() != (width, height) {
        resized = image::imageops::resize(source, width, height, FilterType::Nearest);
        &resized
    } else {
        source
    };
    let mut tiles_rgb = Vec::new();
    let mut has_alpha = false;
    for ty in (0..height).step_by(8) {
        for tx in (0..width).step_by(8) {
            let mut pixels = Vec::with_capacity(64);
            for y in 0..8 {
                for x in 0..8 {
                    let rgba = input.get_pixel(tx + x, ty + y).0;
                    let alpha = u32::from(rgba[3]);
                    has_alpha |= alpha != 255;
                    pixels.push(std::array::from_fn(|c| {
                        ((u32::from(rgba[c]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
                    }));
                }
            }
            tiles_rgb.push(pixels);
        }
    }
    let mut warnings = Vec::new();
    if has_alpha {
        warnings.push("Alpha composited on white; background maps have no transparency.".into());
    }
    let palettes = match options.mode {
        ImageMode::Dmg => vec![[[255; 3], [170; 3], [85; 3], [0; 3]]],
        ImageMode::Cgb => automatic_palettes(&tiles_rgb, &mut warnings),
    };
    let mut project = Project::new((width / 8) as u8, (height / 8) as u8, 1)?;
    project.name = name.to_string();
    project.tiles.clear();
    project.palettes = palettes
        .iter()
        .map(|p| {
            p.iter()
                .map(|c| PaletteColor::new(c[0], c[1], c[2]))
                .collect()
        })
        .collect();
    normalize_palettes(&mut project.palettes);
    let mut lookup = HashMap::<Vec<u8>, u16>::new();
    let mut error_sum = 0u64;
    for (index, pixels) in tiles_rgb.iter().enumerate() {
        let (palette_id, error) = palettes
            .iter()
            .enumerate()
            .map(|(id, palette)| {
                (
                    id,
                    pixels
                        .iter()
                        .map(|color| nearest(*color, palette).1)
                        .sum::<u64>(),
                )
            })
            .min_by_key(|entry| entry.1)
            .expect("at least one palette");
        error_sum += error;
        let indexed = pixels
            .iter()
            .map(|color| nearest(*color, &palettes[palette_id]).0)
            .collect::<Vec<_>>();
        let tile_id = if let Some(id) = lookup.get(&indexed) {
            *id
        } else {
            if project.tiles.len() >= MAX_TILE_ID + 1 {
                return Err(format_error(
                    "image needs more than 256 unique tiles; reduce image size or detail",
                ));
            }
            let id = project.tiles.len() as u16;
            let mut tile = Tile::blank();
            for (pixel, color) in indexed.iter().enumerate() {
                tile.set_pixel(pixel % 8, pixel / 8, *color)?;
            }
            project.tiles.push(tile);
            lookup.insert(indexed, id);
            id
        };
        project.map[index] = MapCell {
            tile: tile_id,
            palette: palette_id as u8,
            ..MapCell::default()
        };
    }
    warnings.extend(project.budget_report().warnings);
    let report = ImportReport {
        source_size: [source.width(), source.height()],
        output_size: [width, height],
        mode: options.mode,
        tile_count: project.tiles.len(),
        map_cells: project.map.len(),
        reused_tiles: project.map.len() - project.tiles.len(),
        palette_count: palettes.len(),
        mean_squared_error: error_sum as f64 / (f64::from(width) * f64::from(height) * 3.0),
        warnings,
    };
    Ok((project, report))
}

pub fn validate(project: &Project) -> ValidationReport {
    let mut errors = Vec::new();
    if project.map_width == 0
        || project.map_height == 0
        || project.map.len() != usize::from(project.map_width) * usize::from(project.map_height)
    {
        errors.push("Map dimensions and cell count disagree.".into());
    }
    if project.tiles.is_empty() || project.tiles.len() > 256 {
        errors.push("Tile count must be 1..=256.".into());
    }
    if project
        .tiles
        .iter()
        .any(|tile| tile.pixels().len() != 64 || tile.pixels().iter().any(|c| *c > 3))
    {
        errors.push("Each tile must have 64 pixels in 0..=3.".into());
    }
    if project.palettes.is_empty()
        || project.palettes.len() > 8
        || project.palettes.iter().any(|p| p.len() != 4)
    {
        errors.push("Use 1..=8 palettes of four colors.".into());
    }
    if project.map.iter().any(|cell| {
        usize::from(cell.tile) >= project.tiles.len()
            || usize::from(cell.palette) >= project.palettes.len()
            || cell.vram_bank != 0
    }) {
        errors.push(
            "Map has missing tiles/palettes or unsupported VRAM bank (only bank 0 exported)."
                .into(),
        );
    }
    for sprite in &project.metasprites {
        for frame in &sprite.frames {
            if frame
                .parts
                .iter()
                .any(|part| usize::from(part.tile) >= project.tiles.len() || part.flags & 0x08 != 0)
            {
                errors.push(format!(
                    "Invalid tile or VRAM bank in {} / {}.",
                    sprite.name, frame.name
                ));
            }
        }
    }
    ValidationReport {
        valid: errors.is_empty(),
        errors,
        warnings: project.budget_report().warnings,
    }
}

pub fn require_valid(project: &Project) -> StudioResult<()> {
    let report = validate(project);
    if report.valid {
        Ok(())
    } else {
        Err(format_error(report.errors.join(" ")))
    }
}

/// JSON is intentionally not normalized here: validation must expose malformed assets.
pub fn load_checked(path: impl AsRef<Path>) -> StudioResult<Project> {
    let path = path.as_ref();
    check_project_extension(path)?;
    let project = match asset_file_kind(path) {
        AssetFileKind::Json => serde_json::from_str(&fs::read_to_string(path)?)?,
        _ => Project::load_project_file(path)?,
    };
    Ok(project)
}

pub fn preview_png(
    project: &Project,
    path: impl AsRef<Path>,
    mode: ImageMode,
    scale: u32,
) -> StudioResult<()> {
    fs::write(path, preview_png_bytes(project, mode, scale)?)?;
    Ok(())
}

pub fn preview_png_bytes(project: &Project, mode: ImageMode, scale: u32) -> StudioResult<Vec<u8>> {
    require_valid(project)?;
    if !(1..=8).contains(&scale) {
        return Err(format_error("preview scale must be 1..=8"));
    }
    let width = u32::from(project.map_width) * 8;
    let height = u32::from(project.map_height) * 8;
    if u64::from(width) * u64::from(height) * u64::from(scale).pow(2) > 16_777_216 {
        return Err(format_error("preview exceeds 16 megapixels; reduce scale"));
    }
    let pixels = if mode == ImageMode::Dmg {
        let mut dmg = project.clone();
        dmg.palettes = vec![DMG_PALETTE.to_vec(); 8];
        dmg.render_map_art_rgba()
    } else {
        project.render_map_art_rgba()
    };
    let image = RgbaImage::from_raw(width, height, pixels).expect("validated map dimensions");
    let image = image::imageops::resize(&image, width * scale, height * scale, FilterType::Nearest);
    let mut output = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(image_error)?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metallic_fixture(inverted: bool) -> RgbaImage {
        RgbaImage::from_fn(64, 104, |x, y| {
            let tile = (y / 8) * 8 + x / 8;
            let shade = (x % 4) as usize;
            let color = if tile == 96 {
                [
                    [255, 255, 255],
                    [214, 230, 247],
                    [115, 165, 214],
                    [8, 16, 33],
                ][shade]
            } else {
                let group = if tile < 96 { tile / 12 } else { 0 };
                [
                    group as u8 * 8 + shade as u8 * 8,
                    8 + shade as u8 * 8,
                    16 + shade as u8 * 8,
                ]
            };
            let color = color.map(|v| if inverted { 255 - v } else { v });
            image::Rgba([color[0], color[1], color[2], 255])
        })
    }

    #[test]
    fn cgb_palette_budget_preserves_rare_highlights_and_dark_outlines() {
        for inverted in [false, true] {
            let image = metallic_fixture(inverted);
            let (project, report) =
                import_rgba(&image, "metal", &ImportOptions::default()).unwrap();
            let rendered = project.render_map_art_rgba();
            let offset = (96 * 64) * 4;
            let expected = image.get_pixel(0, 96).0;
            let actual = &rendered[offset..offset + 3];
            assert!(
                actual.iter().zip(expected).all(|(&a, b)| a.abs_diff(b) <= 4),
                "rare extreme was lost: inverted={inverted}, expected={expected:?}, actual={actual:?}, MSE={}",
                report.mean_squared_error
            );
            assert_eq!(report.palette_count, 8);
            assert!(
                report.mean_squared_error < 100.0,
                "{}",
                report.mean_squared_error
            );
            assert!(validate(&project).valid);
            let (again, _) = import_rgba(&image, "metal", &ImportOptions::default()).unwrap();
            assert_eq!(project, again);
        }
    }

    #[test]
    fn dmg_deduplicates_and_encodes_two_bitplanes() {
        let image = RgbaImage::from_fn(16, 8, |x, _| {
            let color = [255, 170, 85, 0][(x % 4) as usize];
            image::Rgba([color, color, color, 255])
        });
        let (p, r) = import_rgba(
            &image,
            "stripes",
            &ImportOptions {
                mode: ImageMode::Dmg,
                resize: None,
            },
        )
        .unwrap();
        assert_eq!((r.tile_count, r.reused_tiles, r.palette_count), (1, 1, 1));
        assert_eq!(&p.tiles[0].encoded_2bpp()[..2], &[0x55, 0x33]);
        assert_eq!(r.mean_squared_error, 0.0);
        assert!(validate(&p).valid);
    }

    #[test]
    fn cgb_preserves_simple_palettes_and_is_reproducible() {
        let image = RgbaImage::from_fn(16, 8, |x, y| {
            image::Rgba(if x < 8 {
                [if y < 4 { 255 } else { 0 }, 0, 0, 255]
            } else {
                [0, if y < 4 { 255 } else { 0 }, 0, 255]
            })
        });
        let (a, report) = import_rgba(&image, "colors", &ImportOptions::default()).unwrap();
        let (b, _) = import_rgba(&image, "colors", &ImportOptions::default()).unwrap();
        assert_eq!(a, b);
        assert_eq!(report.palette_count, 2);
        assert_eq!(a.render_map_art_rgba(), image.into_raw());
    }

    #[test]
    fn rejects_dimensions_and_invalid_projects_without_panicking() {
        assert!(import_rgba(&RgbaImage::new(9, 8), "bad", &ImportOptions::default()).is_err());
        let mut p = Project::default();
        p.map[0].tile = 250;
        assert!(!validate(&p).valid);
        p.tiles.clear();
        assert!(!validate(&p).valid);
    }

    #[test]
    fn resize_alpha_and_budget_failures_are_explicit() {
        let input = RgbaImage::new(10, 10);
        let (_, report) = import_rgba(
            &input,
            "alpha",
            &ImportOptions {
                mode: ImageMode::Dmg,
                resize: Some((16, 16)),
            },
        )
        .unwrap();
        assert_eq!(report.output_size, [16, 16]);
        assert_eq!(report.warnings.len(), 1);
        let many = RgbaImage::from_fn(136, 128, |x, y| {
            let tile = (y / 8) * 17 + x / 8;
            let bit = ((y % 8) * 8 + x % 8) % 9;
            let color = if tile & (1 << bit) == 0 { 255 } else { 0 };
            image::Rgba([color, color, color, 255])
        });
        assert!(import_rgba(
            &many,
            "overflow",
            &ImportOptions {
                mode: ImageMode::Dmg,
                resize: None
            }
        )
        .is_err());
    }
}
