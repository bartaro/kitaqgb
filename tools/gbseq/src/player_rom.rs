//! Per-console playback ROMs: one local four-channel image, no cartridge RAM.
use crate::link16_rom::{DATA_OFFSET, MANIFEST_OFFSET, ROM_BYTES, SONG_TAG_OFFSET};
use crate::link16_sav::{build_player_images, region_checksum, LINK16_REGION_BYTES};
use crate::{Link16Mode, SequenceProject};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[path = "player_title.rs"]
mod player_title;
const CAPABILITY: &[u8] = b"L16PLAY2\x08\x09\x01\x04\x19\0\0\0";

#[derive(Debug, Serialize)]
pub struct PlayerRom {
    pub player: usize,
    pub role: &'static str,
    pub file: String,
    pub bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct PlayerRomPackage {
    pub format: &'static str,
    pub title: String,
    pub directory: PathBuf,
    pub song_id: String,
    pub players: usize,
    pub steps: u32,
    pub sram_bytes: usize,
    pub external_save_required: bool,
    pub files: Vec<PlayerRom>,
}

/// Each returned ROM contains only its own four-channel region, including its
/// envelopes, wave slots and user waves. The common ID identifies the set.
pub fn build_player_roms(project: &SequenceProject) -> Result<(String, Vec<Vec<u8>>)> {
    let (_, images) = build_player_images(project)?;
    let template: &[u8] = match project.link16_mode {
        Link16Mode::Local4 | Link16Mode::Cable8 => {
            include_bytes!("../assets/roms/link16_player_cable.gb")
        }
        Link16Mode::Dmg07_16 => include_bytes!("../assets/roms/link16_player_dmg07.gb"),
    };
    ensure!(
        template.len() == ROM_BYTES
            && template
                .windows(CAPABILITY.len())
                .filter(|w| *w == CAPABILITY)
                .count()
                == 1,
        "invalid playback ROM template; rebuild LINK16 player templates"
    );
    ensure!(
        template[0x147] == 0x19 && template[0x148] == 4 && template[0x149] == 0,
        "playback template must declare 512 KiB MBC5 ROM and no SRAM"
    );
    let tag = images[0].iter().fold(0x811c9dc5u32, |n, b| {
        (n ^ u32::from(*b)).wrapping_mul(0x01000193)
    });
    let mut identity = [0u8; 8];
    identity[..4].copy_from_slice(b"L16P");
    identity[4..].copy_from_slice(&tag.to_le_bytes());
    let mut roms = Vec::new();
    let title_tiles = player_title::tiles(&project.title)?;
    for (player, image) in images.iter().enumerate() {
        let mut rom = template.to_vec();
        // Never carry another player's data or editor-template garbage into
        // the reserved banks of this dedicated player ROM.
        rom[MANIFEST_OFFSET..].fill(0xff);
        rom[MANIFEST_OFFSET..MANIFEST_OFFSET + 4].copy_from_slice(b"L16P");
        rom[MANIFEST_OFFSET + 4] = 2;
        rom[MANIFEST_OFFSET + 5] = project.player_count() as u8;
        rom[MANIFEST_OFFSET + 6] = match project.link16_mode {
            Link16Mode::Local4 => 0,
            Link16Mode::Cable8 => 1,
            Link16Mode::Dmg07_16 => 2,
        };
        rom[MANIFEST_OFFSET + 7] = player as u8;
        rom[MANIFEST_OFFSET + 8..MANIFEST_OFFSET + 16].copy_from_slice(&identity);
        rom[MANIFEST_OFFSET + 16..MANIFEST_OFFSET + 16 + title_tiles.len()]
            .copy_from_slice(&title_tiles);
        let local = &mut rom[DATA_OFFSET..DATA_OFFSET + LINK16_REGION_BYTES];
        local.copy_from_slice(&image[..LINK16_REGION_BYTES]);
        local[SONG_TAG_OFFSET..SONG_TAG_OFFSET + 8].copy_from_slice(&identity);
        local[7] = region_checksum(local);
        let title = format!("L16P{tag:08X}{}", player + 1);
        rom[0x134..0x143].fill(0);
        rom[0x134..0x134 + title.len()].copy_from_slice(title.as_bytes());
        rom[0x14d] = rom[0x134..=0x14c]
            .iter()
            .fold(0u8, |n, b| n.wrapping_sub(*b).wrapping_sub(1));
        let sum = rom
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 0x14e && *i != 0x14f)
            .fold(0u16, |n, (_, b)| n.wrapping_add(u16::from(*b)));
        rom[0x14e..0x150].copy_from_slice(&sum.to_be_bytes());
        roms.push(rom);
    }
    Ok((format!("{tag:08X}"), roms))
}

pub fn export_player_roms(
    project: &SequenceProject,
    output_root: &Path,
) -> Result<PlayerRomPackage> {
    let (song_id, roms) = build_player_roms(project)?;
    fs::create_dir_all(output_root)?;
    let directory = output_root.join(format!("LINK16_PLAYER_{song_id}"));
    // Creating the package directory exclusively is the transaction boundary:
    // pre-existing ROMs/saves/packages are never opened for replacement.
    fs::create_dir(&directory)
        .context("playback ROM package already exists; choose another output folder")?;
    let mut files = Vec::new();
    for (player, rom) in roms.iter().enumerate() {
        let role = if project.player_count() == 1 {
            "SOLO"
        } else {
            "SELECTABLE"
        };
        let file = format!("P{}.gb", player + 1);
        fs::write(directory.join(&file), rom)?;
        files.push(PlayerRom {
            player: player + 1,
            role,
            file,
            bytes: rom.len(),
        });
    }
    let report = PlayerRomPackage {
        format: "LINK16_PLAYER_ROM_V2",
        title: project.title.clone(),
        directory,
        song_id,
        players: project.player_count(),
        steps: project.total_steps(),
        sram_bytes: 0,
        external_save_required: false,
        files,
    };
    fs::write(
        report.directory.join("manifest.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    fs::write(report.directory.join("README.ja.txt"), format!(
        "LINK16 ミュージックプレイヤー（GB/GBC共通）\n曲: {}\n{}台 / {}ステップ / 各ROM 512KB / SRAM不要\n\nP番号は楽曲の担当パートです。起動直後は全ROMともSOLO。ケーブルなしでSTARTを押すと自分の4CHを再生・停止できます。\n上部に曲名とパート、中央に4CHの出力レベルの目安を表示します。\n2台・4台で同期するには、同じセットの異なるパートを各台に用意します。全台の停止中にSELECT、1台でA（MASTER）、他はB（PEER）を選択し、MASTERのSTARTで再生／停止。\n4人用アダプターはMASTERをポート1、PEERをポート2〜4へ。ROMの番号とポート番号は独立です。\n停止後、SELECTでメニュー、もう一度SELECTでSOLOへ戻ります。通信は音声を混合しません。編集はGBSEQで行ってください。\n",
        project.title, report.players, report.steps))?;
    fs::write(report.directory.join("README.txt"),
        "LINK16 Music Player for GB/GBC. 512 KiB per ROM, no SRAM or SAV/SRM.\nP1..P4 identify musical parts, not link roles. Each boots in SOLO: START plays/stops its four voices without a link.\nFor sync, use distinct parts from one set. Press SELECT while stopped on each console. A: MASTER on one; B: PEER on the others. START on the master plays/stops everyone.\nDMG-07: plug the selected MASTER into port 1, PEERs into ports 2..4, regardless of part number. SELECT in the menu returns to SOLO.\nThe four meters show relative channel activity, not a frequency analysis. The cable does not carry/mix audio. Edit in GBSEQ.\n")?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dedicated_roms_only_contain_their_own_part_and_need_no_sram() {
        for mode in [Link16Mode::Local4, Link16Mode::Cable8, Link16Mode::Dmg07_16] {
            let mut p = SequenceProject::empty();
            p.set_link16_mode(mode);
            p.measures = 256;
            for t in &mut p.tracks {
                t.notes.clear();
            }
            for player in 0..p.player_count() {
                p.insert_note_exact(player * 4, 4095, 1, 24 + player as u8, 12)
                    .unwrap();
                p.insert_note_exact(player * 4 + 2, 2338, 2, 10 + player as u8, 12)
                    .unwrap();
            }
            let (_, roms) = build_player_roms(&p).unwrap();
            assert_eq!(roms.len(), p.player_count());
            for (player, rom) in roms.iter().enumerate() {
                assert_eq!(rom.len(), 512 * 1024);
                assert_eq!(&rom[0x147..=0x149], &[0x19, 4, 0]);
                assert_eq!(rom[MANIFEST_OFFSET + 7], player as u8);
                assert_eq!(rom[DATA_OFFSET + 5], player as u8);
                assert_eq!(rom[DATA_OFFSET + 16 + 4095 * 7], 24 + player as u8);
                assert_eq!(
                    rom[DATA_OFFSET + 7],
                    region_checksum(&rom[DATA_OFFSET..DATA_OFFSET + LINK16_REGION_BYTES])
                );
                assert!(rom[DATA_OFFSET + LINK16_REGION_BYTES..]
                    .iter()
                    .all(|b| *b == 255));
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
                    .fold(0u16, |n, (_, b)| n.wrapping_add(u16::from(*b)));
                assert_eq!(u16::from_be_bytes([rom[0x14e], rom[0x14f]]), sum);
            }
            if roms.len() > 1 {
                assert_ne!(roms[0], roms[1]);
            }
        }
    }

    #[test]
    fn package_export_refuses_existing_set_without_touching_files() {
        let p = SequenceProject::empty();
        let root = std::env::temp_dir().join(format!("gbseq_player_rom_{}", std::process::id()));
        let package = export_player_roms(&p, &root).unwrap();
        let file = package.directory.join(&package.files[0].file);
        fs::write(&file, b"keep user file").unwrap();
        assert!(export_player_roms(&p, &root).is_err());
        assert_eq!(fs::read(file).unwrap(), b"keep user file");
    }
}
