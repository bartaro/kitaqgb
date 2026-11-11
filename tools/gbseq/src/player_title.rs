//! Bake the score title into 20 x 4 DMG tiles once at export, not while playing.
use anyhow::{anyhow, Result};
use fontdue::{Font, FontSettings};
use std::sync::OnceLock;

const FONT: &[u8] = include_bytes!("../assets/title-font/NotoSansJP-Regular.otf");
const WIDTH: usize = 160;
const PX: f32 = 13.0;

pub(super) fn tiles(title: &str) -> Result<[u8; 1280]> {
    static FACE: OnceLock<Result<Font, &'static str>> = OnceLock::new();
    let face = FACE
        .get_or_init(|| Font::from_bytes(FONT, FontSettings::default()))
        .as_ref()
        .map_err(|e| anyhow!("title font: {e}"))?;
    let advance = |c: char| face.metrics(c, PX).advance_width.ceil() as usize;
    let width = |s: &[char]| s.iter().map(|&c| advance(c)).sum::<usize>();
    let mut lines: Vec<Vec<char>> = vec![Vec::new()];
    let mut words = title.split_whitespace().peekable();
    if words.peek().is_none() {
        return tiles("Untitled");
    }
    'words: for word in words {
        let word_width = word.chars().map(advance).take(160).sum::<usize>();
        let line = lines.last_mut().unwrap();
        if !line.is_empty() {
            if width(line) + advance(' ') + word_width <= 154 {
                line.push(' ');
            } else if lines.len() < 2 {
                lines.push(Vec::new());
            } else {
                ellipsis(lines.last_mut().unwrap(), &width);
                break;
            }
        }
        for c in word.chars() {
            let line = lines.last_mut().unwrap();
            if width(line) + advance(c) > 154 {
                if lines.len() == 2 {
                    ellipsis(lines.last_mut().unwrap(), &width);
                    break 'words;
                }
                lines.push(Vec::new());
            }
            lines.last_mut().unwrap().push(c);
        }
    }
    let mut tiles = [0; 1280];
    for (row, line) in lines.iter().enumerate() {
        let mut pen = ((WIDTH - width(line)) / 2) as i32;
        let baseline = (if lines.len() == 1 { 21 } else { 13 + 16 * row }) as i32;
        for &c in line {
            let (m, mask) = face.rasterize(c, PX);
            for y in 0..m.height {
                for x in 0..m.width {
                    let v = mask[y * m.width + x];
                    let px = pen + m.xmin + x as i32;
                    let py = baseline - m.ymin - m.height as i32 + y as i32;
                    if !(0..160).contains(&px) || !(0..32).contains(&py) {
                        continue;
                    }
                    let (x, y) = (px as usize, py as usize);
                    let idx = (y / 8 * 20 + x / 8) * 16 + (y % 8) * 2;
                    let bit = 0x80 >> (x % 8);
                    let shade = (u16::from(v) * 3 + 127) / 255;
                    if shade & 1 != 0 {
                        tiles[idx] |= bit;
                    }
                    if shade & 2 != 0 {
                        tiles[idx + 1] |= bit;
                    }
                }
            }
            pen += advance(c) as i32;
        }
    }
    Ok(tiles)
}

fn ellipsis(s: &mut Vec<char>, width: &impl Fn(&[char]) -> usize) {
    while width(s) + width(&['…']) > 154 {
        s.pop();
    }
    s.push('…');
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centered_title_is_not_an_ascii_placeholder() {
        let ja = tiles("剣の舞").unwrap();
        assert!(ja.iter().any(|&b| b != 0));
        assert_ne!(ja, tiles("???").unwrap());
        assert_ne!(ja, tiles("別の曲").unwrap());
        assert_eq!(tiles("  ").unwrap(), tiles("Untitled").unwrap());
        assert_ne!(tiles(&"あ".repeat(200)).unwrap(), ja);
    }
}
