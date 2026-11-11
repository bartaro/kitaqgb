//! Self-contained LINK16 song ROMs built from this project's own templates.
use crate::link16_sav::{build_player_images, region_checksum, LINK16_REGION_BYTES};
use crate::{Link16Mode, SequenceProject};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub const ROM_BYTES: usize = 512 * 1024;
pub const MANIFEST_OFFSET: usize = 8 * 16384;
pub const DATA_OFFSET: usize = 9 * 16384;
pub const SONG_TAG_OFFSET: usize = 32752;
const CAPABILITY: &[u8] = b"L16EXPT1\x08\x09\x04\x04\x1b\x04\0\0";

#[derive(Debug, Serialize)]
pub struct RomExport {
    pub format: &'static str,
    pub bytes: usize,
    pub players: usize,
    pub song_id: String,
    pub steps: u32,
    pub external_save_required: bool,
}

pub fn build_link16_rom(project: &SequenceProject) -> Result<(Vec<u8>, RomExport)> {
    let template: &[u8] = match project.link16_mode {
        Link16Mode::Local4 | Link16Mode::Cable8 => {
            include_bytes!("../assets/roms/link16_export_cable.gb")
        }
        Link16Mode::Dmg07_16 => include_bytes!("../assets/roms/link16_export_dmg07.gb"),
    };
    ensure!(
        template.len() == ROM_BYTES
            && template
                .windows(CAPABILITY.len())
                .filter(|w| *w == CAPABILITY)
                .count()
                == 1,
        "invalid embedded LINK16 ROM template; rebuild export templates"
    );
    let (_, images) = build_player_images(project)?;
    let mut canonical = images[0].clone();
    let tag = canonical.iter().fold(0x811c9dc5u32, |n, b| {
        (n ^ u32::from(*b)).wrapping_mul(0x01000193)
    });
    let mut identity = [0u8; 8];
    identity[..4].copy_from_slice(b"L16R");
    identity[4..].copy_from_slice(&tag.to_le_bytes());
    for p in 0..project.player_count() {
        let base = p * LINK16_REGION_BYTES;
        canonical[base + SONG_TAG_OFFSET..base + SONG_TAG_OFFSET + 8].copy_from_slice(&identity);
        canonical[base + 7] = region_checksum(&canonical[base..base + LINK16_REGION_BYTES]);
    }
    let mut rom = template.to_vec();
    rom[MANIFEST_OFFSET..MANIFEST_OFFSET + 4].copy_from_slice(b"L16R");
    rom[MANIFEST_OFFSET + 4] = 1;
    rom[MANIFEST_OFFSET + 5] = project.player_count() as u8;
    rom[MANIFEST_OFFSET + 6] = match project.link16_mode {
        Link16Mode::Local4 => 0,
        Link16Mode::Cable8 => 1,
        Link16Mode::Dmg07_16 => 2,
    };
    rom[MANIFEST_OFFSET + 7] = 0; // runtime consent flag is kept in WRAM
    rom[MANIFEST_OFFSET + 8..MANIFEST_OFFSET + 16].copy_from_slice(&identity);
    rom[DATA_OFFSET..DATA_OFFSET + canonical.len()].copy_from_slice(&canonical);
    // Never patch the executable's bank layout. Only the reserved song banks
    // and standard header checksums/title are changed.
    let title = format!("L16S{tag:08X}");
    rom[0x134..0x143].fill(0);
    rom[0x134..0x134 + title.len()].copy_from_slice(title.as_bytes());
    rom[0x148] = 4; // 512 KiB
    rom[0x14d] = rom[0x134..=0x14c]
        .iter()
        .fold(0u8, |n, b| n.wrapping_sub(*b).wrapping_sub(1));
    let sum = rom
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0x14e && *i != 0x14f)
        .fold(0u16, |n, (_, b)| n.wrapping_add(u16::from(*b)));
    rom[0x14e..0x150].copy_from_slice(&sum.to_be_bytes());
    Ok((
        rom,
        RomExport {
            format: "LINK16_EMBEDDED_ROM_V1",
            bytes: ROM_BYTES,
            players: project.player_count(),
            song_id: format!("{tag:08X}"),
            steps: project.total_steps(),
            external_save_required: false,
        },
    ))
}

pub fn export_link16_rom(project: &SequenceProject, path: &Path) -> Result<RomExport> {
    ensure!(
        path.extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("gb")),
        "ROM output must have a .gb extension"
    );
    let (rom, report) = build_link16_rom(project)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .context("ROM output already exists or cannot be created; choose a new filename")?;
    file.write_all(&rom)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_link_modes_embed_the_exact_song_and_valid_checksums() {
        for mode in [Link16Mode::Local4, Link16Mode::Cable8, Link16Mode::Dmg07_16] {
            let mut p = SequenceProject::empty();
            p.set_link16_mode(mode);
            p.measures = 256;
            for player in 0..p.player_count() {
                p.insert_note_exact(player * 4, 4095, 1, 24 + player as u8, 12)
                    .unwrap();
            }
            let (rom, report) = build_link16_rom(&p).unwrap();
            assert_eq!(rom.len(), ROM_BYTES);
            assert!(!report.external_save_required);
            assert_eq!(rom[MANIFEST_OFFSET + 5] as usize, p.player_count());
            assert_eq!(
                rom[0x14d],
                rom[0x134..=0x14c]
                    .iter()
                    .fold(0u8, |n, b| n.wrapping_sub(*b).wrapping_sub(1))
            );
            let sum = rom
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != 0x14e && *i != 0x14f)
                .fold(0u16, |n, (_, b)| n.wrapping_add(*b as u16));
            assert_eq!(u16::from_be_bytes([rom[0x14e], rom[0x14f]]), sum);
            for player in 0..p.player_count() {
                let r = &rom[DATA_OFFSET + player * LINK16_REGION_BYTES
                    ..DATA_OFFSET + (player + 1) * LINK16_REGION_BYTES];
                assert_eq!(r[5] as usize, player);
                assert_eq!(r[7], region_checksum(r));
                assert_eq!(
                    &r[SONG_TAG_OFFSET..SONG_TAG_OFFSET + 8],
                    &rom[MANIFEST_OFFSET + 8..MANIFEST_OFFSET + 16]
                );
                assert_eq!(r[16 + 4095 * 7], 24 + player as u8);
            }
            let (_, again) = build_link16_rom(&p).unwrap();
            assert_eq!(report.song_id, again.song_id);
            p.tracks[0].notes[0].pitch += 1;
            assert_ne!(report.song_id, build_link16_rom(&p).unwrap().1.song_id);
        }
    }
}
