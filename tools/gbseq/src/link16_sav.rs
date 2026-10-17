use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::model::{
    Link16Mode, PanPosition, SequenceProject, TrackKind, LINK16_STEP_COUNT, TRACK_COUNT,
};

pub const LINK16_SAV_BYTES: usize = 128 * 1024;
pub const LINK16_REGION_BYTES: usize = 32 * 1024;
pub const LINK16_HEADER_BYTES: usize = 16;
pub const LINK16_BYTES_PER_STEP: usize = 7;
pub const LINK16_FORMAT_VERSION: u8 = 4;
pub const LINK16_WAVE_BASE: usize =
    LINK16_HEADER_BYTES + LINK16_STEP_COUNT as usize * LINK16_BYTES_PER_STEP;
pub const LINK16_SETTINGS_BASE: usize = LINK16_WAVE_BASE + 33 + 64 * 16;
pub const LINK16_PLAY_START: usize = LINK16_SETTINGS_BASE;
pub const LINK16_PLAY_END: usize = LINK16_SETTINGS_BASE + 1;
pub const LINK16_BPM_LO: usize = LINK16_SETTINGS_BASE + 2;
pub const LINK16_BPM_HI: usize = LINK16_SETTINGS_BASE + 3;
pub const LINK16_LOOP_PLAYBACK: usize = LINK16_SETTINGS_BASE + 4;

#[derive(Clone, Debug, Serialize)]
pub struct Link16SavFile {
    pub player: usize,
    pub path: PathBuf,
    pub bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Link16SavManifest {
    pub format: &'static str,
    pub version: u8,
    pub mode: Link16Mode,
    pub measures: u32,
    pub steps: u32,
    pub step_frames: u8,
    pub requested_bpm: u16,
    pub effective_bpm: u16,
    pub playback_start_page: u8,
    pub playback_end_page: u8,
    pub loop_playback: bool,
    pub files: Vec<Link16SavFile>,
}

pub fn step_frames_for_bpm(bpm: u16) -> u8 {
    let denominator = u32::from(bpm) * 10;
    ((8959_u32 + denominator / 2) / denominator).clamp(4, 15) as u8
}

pub fn export_link16_saves(project: &SequenceProject, out_dir: &Path) -> Result<Link16SavManifest> {
    project.validate()?;
    if project.frames_per_step_override.is_some() {
        bail!(
            "LINK16 SAV export does not support frames_per_step_override; use GBSEQ playback, WAV, or KITAQGB export"
        );
    }
    if project.total_steps() > LINK16_STEP_COUNT {
        bail!(
            "project has {} steps; LINK16 supports {LINK16_STEP_COUNT}",
            project.total_steps()
        );
    }
    fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let effective_bpm = project.bpm.clamp(60, 225);
    let step_frames = step_frames_for_bpm(effective_bpm);
    let player_count = project.player_count();
    let mut player_images = Vec::with_capacity(player_count);
    for player in 0..player_count {
        let mut image = vec![0xFF; LINK16_SAV_BYTES];
        write_region(&mut image, 0, player as u8, project, player, step_frames)?;
        if player == 0 {
            for remote_player in 1..player_count {
                write_region(
                    &mut image,
                    remote_player,
                    remote_player as u8,
                    project,
                    remote_player,
                    step_frames,
                )?;
            }
        }
        player_images.push(image);
    }

    let names: Vec<String> = match project.link16_mode {
        Link16Mode::Local4 => vec!["link16_local.sav".to_string()],
        Link16Mode::Cable8 => (1..=2).map(|p| format!("link16_cable.p{p}.sav")).collect(),
        Link16Mode::Dmg07_16 => (1..=4).map(|p| format!("link16_dmg07.p{p}.sav")).collect(),
    };

    let mut files = Vec::new();
    for (player, (name, image)) in names.iter().zip(player_images.iter()).enumerate() {
        let path = out_dir.join(name);
        fs::write(&path, image).with_context(|| format!("write {}", path.display()))?;
        files.push(Link16SavFile {
            player: player + 1,
            path,
            bytes: image.len(),
        });
    }

    let manifest = Link16SavManifest {
        format: "LINK16_SRAM_V4",
        version: LINK16_FORMAT_VERSION,
        mode: project.link16_mode,
        measures: project.measures,
        steps: project.total_steps(),
        step_frames,
        requested_bpm: project.bpm,
        effective_bpm,
        playback_start_page: 0,
        playback_end_page: ((project.total_steps() - 1) / 16) as u8,
        loop_playback: project.loop_playback,
        files,
    };
    let manifest_path = out_dir.join("link16_sav_manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("write {}", manifest_path.display()))?;
    Ok(manifest)
}

fn write_region(
    image: &mut [u8],
    region: usize,
    logical_slot: u8,
    project: &SequenceProject,
    player: usize,
    step_frames: u8,
) -> Result<()> {
    if region >= 4 || player >= project.player_count() {
        bail!("invalid LINK16 player region");
    }
    let base = region * LINK16_REGION_BYTES;
    let data_start = base + LINK16_HEADER_BYTES;
    let steps = project.total_steps() as usize;
    let envelope_presets = envelope_presets(project, player);
    for step in 0..LINK16_STEP_COUNT as usize {
        let at = data_start + step * LINK16_BYTES_PER_STEP;
        image[at..at + 4].fill(0xFF);
        image[at + 4] = 0x76;
        image[at + 5] = 0x00;
        image[at + 6] = 0x04;
    }
    for local_track in 0..TRACK_COUNT {
        let track = &project.tracks[player * TRACK_COUNT + local_track];
        for note in &track.notes {
            let end = note.end().min(project.total_steps()) as usize;
            for step in note.start as usize..end {
                let at = data_start + step * LINK16_BYTES_PER_STEP;
                image[at + local_track] = match track.kind {
                    TrackKind::Noise => {
                        (note.effects.noise_shift.min(15) << 4)
                            | (u8::from(note.effects.noise_width7) << 3)
                            | note.effects.noise_divisor.min(7)
                    }
                    _ => note.pitch.min(47),
                };
                match track.kind {
                    TrackKind::Pulse1 => {
                        image[at + 4] = (image[at + 4] & 0xFC) | note.effects.duty.min(3)
                    }
                    TrackKind::Pulse2 => {
                        image[at + 4] = (image[at + 4] & 0xF3) | (note.effects.duty.min(3) << 2)
                    }
                    TrackKind::Wave => {
                        image[at + 4] =
                            (image[at + 4] & 0x0F) | (note.effects.wave_preset.min(15) << 4)
                    }
                    TrackKind::Noise => {}
                }
                let pan = match note.effects.pan {
                    PanPosition::Left => 1,
                    PanPosition::Right => 2,
                    PanPosition::Center => 0,
                };
                let pan_shift = local_track * 2;
                image[at + 5] = (image[at + 5] & !(3 << pan_shift)) | (pan << pan_shift);
                if local_track != 2 {
                    let envelope = (note.velocity.min(15) << 4)
                        | (u8::from(note.effects.envelope_increase) << 3)
                        | note.effects.envelope_period.min(7);
                    let selector = nearest_envelope(envelope, &envelope_presets);
                    // CH3 has no hardware volume envelope, so V4 packs CH4
                    // directly after CH2 and leaves bits 6-7 reserved.
                    let envelope_shift = if local_track == 3 { 4 } else { local_track * 2 };
                    image[at + 6] =
                        (image[at + 6] & !(3 << envelope_shift)) | (selector << envelope_shift);
                }
            }
        }
    }
    image[base..base + 4].copy_from_slice(b"L16X");
    image[base + 4] = LINK16_FORMAT_VERSION;
    image[base + 5] = logical_slot;
    image[base + 6] = step_frames;
    image[base + 8] = steps as u8;
    image[base + 9] = (steps >> 8) as u8;
    image[base + 10..base + 14].copy_from_slice(&envelope_presets);
    let wave_base = base + LINK16_WAVE_BASE;
    image[wave_base] = project.user_waves.len() as u8;
    for preset in 0..16 {
        image[wave_base + 1 + preset] = project.wave_slots[preset].source;
        image[wave_base + 17 + preset] = project.wave_slots[preset].level;
    }
    for (wave_index, wave) in project.user_waves.iter().enumerate() {
        for byte_index in 0..16 {
            image[wave_base + 33 + wave_index * 16 + byte_index] =
                (wave.samples[byte_index * 2].min(15) << 4)
                    | wave.samples[byte_index * 2 + 1].min(15);
        }
    }
    let bpm = project.bpm.clamp(60, 225);
    image[base + LINK16_PLAY_START] = 0;
    image[base + LINK16_PLAY_END] = ((project.total_steps() - 1) / 16) as u8;
    image[base + LINK16_BPM_LO] = bpm as u8;
    image[base + LINK16_BPM_HI] = (bpm >> 8) as u8;
    image[base + LINK16_LOOP_PLAYBACK] = u8::from(project.loop_playback);
    image[base + 7] = region_checksum(&image[base..base + LINK16_REGION_BYTES]);
    Ok(())
}

fn envelope_presets(project: &SequenceProject, player: usize) -> [u8; 4] {
    let mut counts = BTreeMap::<u8, usize>::new();
    for local_track in [0_usize, 1, 3] {
        for note in &project.tracks[player * TRACK_COUNT + local_track].notes {
            let value = (note.velocity.min(15) << 4)
                | (u8::from(note.effects.envelope_increase) << 3)
                | note.effects.envelope_period.min(7);
            *counts.entry(value).or_default() += note.length as usize;
        }
    }
    let mut ranked: Vec<(u8, usize)> = counts.into_iter().collect();
    ranked.sort_by_key(|(value, count)| (std::cmp::Reverse(*count), *value));
    let defaults = [0xA2, 0x82, 0xF1, 0x41];
    let mut result = defaults;
    for (index, (value, _)) in ranked.into_iter().take(4).enumerate() {
        result[index] = value;
    }
    result
}

fn nearest_envelope(value: u8, presets: &[u8; 4]) -> u8 {
    presets
        .iter()
        .enumerate()
        .min_by_key(|(_, preset)| value.abs_diff(**preset))
        .map(|(index, _)| index as u8)
        .unwrap_or(0)
}

pub fn region_checksum(region: &[u8]) -> u8 {
    let mut sum = 0x7B_u8
        .wrapping_add(region[5])
        .wrapping_add(region[6])
        .wrapping_add(region[8])
        .wrapping_add(region[9]);
    for (index, value) in region[LINK16_HEADER_BYTES..LINK16_REGION_BYTES]
        .iter()
        .enumerate()
    {
        sum = sum
            .wrapping_add(*value)
            .wrapping_add(index as u8)
            .wrapping_add((index >> 8) as u8);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Link16Mode, PanPosition, SequenceProject, UserWave, WAVE_SAMPLE_COUNT};

    #[test]
    fn dmg07_master_contains_four_valid_regions_and_peers_contain_local_region() {
        let mut project = SequenceProject::empty();
        project.set_link16_mode(Link16Mode::Dmg07_16);
        project.measures = 256;
        project.bpm = 120;
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        project.insert_note_exact(4, 4095, 1, 31, 15).unwrap();
        let dir = std::env::temp_dir().join(format!("gbseq_link16_sav_{}", std::process::id()));
        let manifest = export_link16_saves(&project, &dir).unwrap();
        assert_eq!(manifest.files.len(), 4);
        let p1 = fs::read(dir.join("link16_dmg07.p1.sav")).unwrap();
        assert_eq!(p1.len(), LINK16_SAV_BYTES);
        assert_eq!(manifest.requested_bpm, 120);
        assert_eq!(manifest.effective_bpm, 120);
        assert_eq!(manifest.playback_start_page, 0);
        assert_eq!(manifest.playback_end_page, 255);
        assert!(manifest.loop_playback);
        assert_eq!(p1[LINK16_PLAY_START], 0);
        assert_eq!(p1[LINK16_PLAY_END], 255);
        assert_eq!(p1[LINK16_LOOP_PLAYBACK], 1);
        assert_eq!(
            u16::from_le_bytes([p1[LINK16_BPM_LO], p1[LINK16_BPM_HI]]),
            120
        );
        for region in 0..4 {
            let base = region * LINK16_REGION_BYTES;
            assert_eq!(&p1[base..base + 4], b"L16X");
            assert_eq!(
                p1[base + 7],
                region_checksum(&p1[base..base + LINK16_REGION_BYTES])
            );
        }
        let p2 = fs::read(dir.join("link16_dmg07.p2.sav")).unwrap();
        assert_eq!(&p2[..4], b"L16X");
        assert_eq!(
            &p2[LINK16_REGION_BYTES..LINK16_REGION_BYTES + 4],
            &[0xFF; 4]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bpm_120_uses_fractional_runtime_metadata_with_compatible_header() {
        assert_eq!(step_frames_for_bpm(120), 7);
        assert_eq!(step_frames_for_bpm(60), 15);
        assert_eq!(step_frames_for_bpm(225), 4);
    }

    #[test]
    fn exact_frame_projects_are_rejected_by_link16_export() {
        let mut project = SequenceProject::empty();
        project.frames_per_step_override = Some(1);
        let dir = std::env::temp_dir().join(format!(
            "gbseq_link16_exact_frame_reject_{}",
            std::process::id()
        ));
        let error = export_link16_saves(&project, &dir).unwrap_err();
        assert!(error.to_string().contains("frames_per_step_override"));
        assert!(!dir.exists());
    }

    #[test]
    fn loop_off_is_written_to_every_save_region_and_manifest() {
        let mut project = SequenceProject::empty();
        project.set_link16_mode(Link16Mode::Dmg07_16);
        project.loop_playback = false;
        let dir =
            std::env::temp_dir().join(format!("gbseq_link16_loop_off_{}", std::process::id()));
        let manifest = export_link16_saves(&project, &dir).unwrap();
        assert!(!manifest.loop_playback);
        let p1 = fs::read(dir.join("link16_dmg07.p1.sav")).unwrap();
        let p4 = fs::read(dir.join("link16_dmg07.p4.sav")).unwrap();
        for region in 0..4 {
            assert_eq!(p1[region * LINK16_REGION_BYTES + LINK16_LOOP_PLAYBACK], 0);
        }
        assert_eq!(p4[LINK16_LOOP_PLAYBACK], 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cable_export_writes_numbered_saves_for_the_unified_rom() {
        let mut project = SequenceProject::empty();
        project.set_link16_mode(Link16Mode::Cable8);
        project.measures = 256;
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());
        project.insert_note_exact(0, 0, 1, 12, 15).unwrap();
        project.insert_note_exact(4, 4095, 1, 31, 15).unwrap();

        let dir =
            std::env::temp_dir().join(format!("gbseq_link16_cable_sav_{}", std::process::id()));
        let manifest = export_link16_saves(&project, &dir).unwrap();
        assert_eq!(manifest.steps, 4096);
        assert_eq!(manifest.files.len(), 2);

        let p1 = fs::read(dir.join("link16_cable.p1.sav")).unwrap();
        let p2 = fs::read(dir.join("link16_cable.p2.sav")).unwrap();
        assert_eq!(&p1[..4], b"L16X");
        assert_eq!(&p1[LINK16_REGION_BYTES..LINK16_REGION_BYTES + 4], b"L16X");
        assert_eq!(&p2[..4], b"L16X");
        assert_eq!(
            &p2[LINK16_REGION_BYTES..LINK16_REGION_BYTES + 4],
            &[0xFF; 4]
        );
        let last = LINK16_HEADER_BYTES + 4095 * LINK16_BYTES_PER_STEP;
        assert_eq!(p2[last], 31);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn last_step_automation_and_custom_wave_round_trip_to_player_four() {
        let mut project = SequenceProject::empty();
        project.set_link16_mode(Link16Mode::Dmg07_16);
        project.measures = 256;
        project
            .tracks
            .iter_mut()
            .for_each(|track| track.notes.clear());

        project.user_waves.push(UserWave {
            name: "Test Wave".to_string(),
            samples: (0..WAVE_SAMPLE_COUNT)
                .map(|sample| (sample as u8) & 15)
                .collect(),
        });
        project.wave_slots[15].source = 0x80;
        project.wave_slots[15].level = 3;

        let pulse_id = project.insert_note_exact(12, 4095, 1, 40, 11).unwrap();
        let (_, pulse_index) = project.note_location(pulse_id).unwrap();
        let pulse = &mut project.tracks[12].notes[pulse_index];
        pulse.effects.duty = 3;
        pulse.effects.pan = PanPosition::Left;
        pulse.effects.envelope_increase = true;
        pulse.effects.envelope_period = 5;

        let wave_id = project.insert_note_exact(14, 4095, 1, 23, 15).unwrap();
        let (_, wave_index) = project.note_location(wave_id).unwrap();
        let wave = &mut project.tracks[14].notes[wave_index];
        wave.effects.wave_preset = 15;
        wave.effects.pan = PanPosition::Right;

        let noise_id = project.insert_note_exact(15, 4094, 1, 0, 9).unwrap();
        let (_, noise_index) = project.note_location(noise_id).unwrap();
        let noise = &mut project.tracks[15].notes[noise_index];
        noise.effects.envelope_increase = true;
        noise.effects.envelope_period = 6;

        let dir =
            std::env::temp_dir().join(format!("gbseq_link16_custom_wave_{}", std::process::id()));
        export_link16_saves(&project, &dir).unwrap();
        let p1 = fs::read(dir.join("link16_dmg07.p1.sav")).unwrap();
        let p4 = fs::read(dir.join("link16_dmg07.p4.sav")).unwrap();
        let region3 = 3 * LINK16_REGION_BYTES;
        let last = region3 + LINK16_HEADER_BYTES + 4095 * LINK16_BYTES_PER_STEP;
        assert_eq!(p1[last], 40);
        assert_eq!(p1[last + 2], 23);
        assert_eq!(p1[last + 4], 0xF7);
        assert_eq!(p1[last + 5], 0x21);
        let noise_step = region3 + LINK16_HEADER_BYTES + 4094 * LINK16_BYTES_PER_STEP;
        let presets = [
            p1[region3 + 10],
            p1[region3 + 11],
            p1[region3 + 12],
            p1[region3 + 13],
        ];
        let expected_noise_envelope = (9 << 4) | (1 << 3) | 6;
        assert_eq!(
            (p1[noise_step + 6] >> 4) & 3,
            nearest_envelope(expected_noise_envelope, &presets)
        );
        assert_eq!(p1[noise_step + 6] & 0xC0, 0);
        assert_eq!(
            &p1[region3..region3 + LINK16_REGION_BYTES],
            &p4[..LINK16_REGION_BYTES]
        );

        let wave_base = region3 + LINK16_WAVE_BASE;
        assert_eq!(p1[wave_base], 1);
        assert_eq!(p1[wave_base + 1 + 15], 0x80);
        assert_eq!(p1[wave_base + 17 + 15], 3);
        assert_eq!(p1[wave_base + 33], 0x01);
        assert_eq!(p1[wave_base + 33 + 15], 0xEF);
        assert_eq!(
            p1[region3 + 7],
            region_checksum(&p1[region3..region3 + LINK16_REGION_BYTES])
        );
        let _ = fs::remove_dir_all(dir);
    }
}
