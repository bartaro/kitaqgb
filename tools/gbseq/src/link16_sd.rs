//! LINK16 SD import layout. No firmware or SD card is modified.
use crate::link16_sav::{
    build_player_images, region_checksum, LINK16_REGION_BYTES, LINK16_SAV_BYTES,
};
use crate::{Link16Mode, SequenceProject};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
pub struct ImportPlayer {
    pub player: usize,
    pub sd_root: PathBuf,
    pub rom: String,
    pub srm: String,
    pub srm_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct ImportPackage {
    pub format: &'static str,
    pub directory: PathBuf,
    pub title: String,
    pub steps: u32,
    pub players: Vec<ImportPlayer>,
}

pub fn validate_image(image: &[u8], player: usize, player_count: usize) -> Result<()> {
    ensure!(
        matches!(player_count, 1 | 2 | 4) && player < player_count,
        "invalid active player"
    );
    ensure!(
        image.len() == LINK16_SAV_BYTES,
        "LINK16 V4 requires exactly 131072 bytes; never truncate to 32 KiB"
    );
    let regions = if player == 0 { player_count } else { 1 };
    for region in 0..regions {
        let r = &image[region * LINK16_REGION_BYTES..(region + 1) * LINK16_REGION_BYTES];
        ensure!(
            &r[..5] == b"L16X\x04",
            "region {region}: not a LINK16 V4 save"
        );
        let slot = if region == 0 { player } else { region };
        ensure!(
            usize::from(r[5]) == slot,
            "region {region}: wrong player slot"
        );
        ensure!(
            r[7] == region_checksum(r),
            "region {region}: checksum mismatch"
        );
        ensure!(
            (4..=15).contains(&r[6]),
            "region {region}: invalid step frames"
        );
        ensure!(
            (1..=4096).contains(&u16::from_le_bytes([r[8], r[9]])),
            "region {region}: invalid song length"
        );
    }
    Ok(())
}

pub fn export_link16_sd_package(
    project: &SequenceProject,
    rom_path: &Path,
    out_dir: &Path,
) -> Result<ImportPackage> {
    let rom = fs::read(rom_path).with_context(|| format!("read {}", rom_path.display()))?;
    let title: &[u8] = match project.link16_mode {
        Link16Mode::Cable8 => b"L16CABLE",
        Link16Mode::Dmg07_16 => b"L16DMG07",
        Link16Mode::Local4 => anyhow::bail!(
            "Hardware SD package requires CABLE 8CH or DMG07 16CH and its matching current V4 ROM"
        ),
    };
    ensure!(
        rom.len() >= 0x150 && &rom[0x134..0x134 + title.len()] == title,
        "select the matching LINK16 ROM for this link mode"
    );
    ensure!(
        rom[0x147] == 0x1b && rom[0x149] == 4,
        "LINK16 ROM must declare MBC5 + battery and 128 KiB SRAM"
    );
    let header = rom[0x134..=0x14c]
        .iter()
        .fold(0u8, |n, b| n.wrapping_sub(*b).wrapping_sub(1));
    ensure!(rom[0x14d] == header, "ROM header checksum mismatch");
    ensure!(
        rom.len() == (32768usize << rom[0x148].min(16)),
        "ROM size/header mismatch"
    );
    let (_, images) = build_player_images(project)?;
    for (p, image) in images.iter().enumerate() {
        validate_image(image, p, images.len())?;
    }
    // An ASCII 8.3 basename derived from ROM+song bytes reduces stale save-name
    // reuse. This is a naming fingerprint, not a security/integrity checksum.
    let tag = rom
        .iter()
        .chain(images.iter().flatten())
        .fold(0x811c9dc5u32, |n, b| {
            (n ^ u32::from(*b)).wrapping_mul(0x01000193)
        });
    let directory = out_dir.join(format!("LINK16_SD_{tag:08X}"));
    fs::create_dir_all(out_dir)?;
    fs::create_dir(&directory).context("package already exists or cannot be created; choose a new output folder (existing saves are never replaced)")?;
    let mut players = Vec::new();
    for (p, image) in images.iter().enumerate() {
        let sd_root = directory.join(format!("P{}_SD_ROOT", p + 1));
        let name = format!("L{:06X}{}", tag & 0xffffff, p + 1);
        let rom_name = format!("{name}.gb");
        let srm_name = format!("EDGB/SAVE/{name}.srm");
        fs::create_dir_all(sd_root.join("EDGB/SAVE"))?;
        fs::write(sd_root.join(&rom_name), &rom)?;
        fs::write(sd_root.join(&srm_name), image)?;
        players.push(ImportPlayer {
            player: p + 1,
            sd_root,
            rom: rom_name,
            srm: srm_name,
            srm_bytes: image.len(),
        });
    }
    let package = ImportPackage {
        format: "LINK16_RAW_SRM_PACKAGE_V1",
        directory,
        title: project.title.clone(),
        steps: project.total_steps(),
        players,
    };
    fs::write(
        package.directory.join("README.ja.txt"),
        include_str!("../docs/LINK16_SD_IMPORT.ja.txt"),
    )?;
    fs::write(
        package.directory.join("README.en.txt"),
        include_str!("../docs/LINK16_SD_IMPORT.en.txt"),
    )?;
    fs::write(
        package.directory.join("manifest.json"),
        serde_json::to_string_pretty(&package)?,
    )?;
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_corrupt_truncated_and_wrong_player_images() {
        let mut p = SequenceProject::empty();
        p.set_link16_mode(Link16Mode::Cable8);
        let (_, mut images) = build_player_images(&p).unwrap();
        assert!(validate_image(&images[0], 0, 2).is_ok());
        assert!(validate_image(&images[1], 1, 2).is_ok());
        assert!(validate_image(&images[0][..32768], 0, 2).is_err());
        assert!(validate_image(&images[1], 0, 2).is_err());
        images[0][8192] ^= 1;
        assert!(validate_image(&images[0], 0, 2).is_err());
    }
    #[test]
    fn package_keeps_rom_and_save_bytes_and_never_overwrites() {
        for mode in [Link16Mode::Cable8, Link16Mode::Dmg07_16] {
            let mut p = SequenceProject::empty();
            p.set_link16_mode(mode);
            let tmp = std::env::temp_dir()
                .join(format!("gbseq-link16-sd-{}-{mode:?}", std::process::id()));
            fs::create_dir_all(&tmp).unwrap();
            let rom_path = tmp.join("template.gb");
            // Synthetic header-only fixture, not a playable or distributed ROM.
            let mut rom = vec![0u8; 32768];
            let title: &[u8] = if mode == Link16Mode::Cable8 {
                b"L16CABLE"
            } else {
                b"L16DMG07"
            };
            rom[0x134..0x134 + title.len()].copy_from_slice(title);
            rom[0x147] = 0x1b;
            rom[0x149] = 4;
            rom[0x14d] = rom[0x134..=0x14c]
                .iter()
                .fold(0u8, |n, b| n.wrapping_sub(*b).wrapping_sub(1));
            fs::write(&rom_path, &rom).unwrap();
            let pack = export_link16_sd_package(&p, &rom_path, &tmp).unwrap();
            let (_, images) = build_player_images(&p).unwrap();
            assert_eq!(pack.players.len(), p.player_count());
            for player in &pack.players {
                assert_eq!(Path::new(&player.rom).file_stem().unwrap().len(), 8);
                assert_eq!(
                    Path::new(&player.rom).file_stem(),
                    Path::new(&player.srm).file_stem()
                );
                assert_eq!(fs::read(player.sd_root.join(&player.rom)).unwrap(), rom);
                assert_eq!(
                    fs::read(player.sd_root.join(&player.srm)).unwrap(),
                    images[player.player - 1]
                );
            }
            let original = pack
                .players
                .iter()
                .map(|v| fs::read(v.sd_root.join(&v.srm)).unwrap())
                .collect::<Vec<_>>();
            assert!(export_link16_sd_package(&p, &rom_path, &tmp).is_err());
            for (entry, expected) in pack.players.iter().zip(original) {
                assert_eq!(fs::read(entry.sd_root.join(&entry.srm)).unwrap(), expected);
            }
            let wrong = tmp.join("wrong.gb");
            let mut bad = rom.clone();
            bad[0x149] = 3;
            fs::write(&wrong, bad).unwrap();
            assert!(export_link16_sd_package(&p, &wrong, &tmp.join("must_not_exist")).is_err());
            assert!(!tmp.join("must_not_exist").exists());
            p.tracks[0].notes.clear();
            p.insert_note_exact(0, 0, 1, 12, 10).unwrap();
            let other = export_link16_sd_package(&p, &rom_path, &tmp).unwrap();
            assert_ne!(pack.directory, other.directory);
            fs::remove_dir_all(tmp).unwrap(); // only this test's unique temporary fixtures
        }
    }
}
