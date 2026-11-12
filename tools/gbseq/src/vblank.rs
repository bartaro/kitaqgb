//! KITAQGB audio_vblank.c five-byte records (not the audio.c bytecode).
use crate::{
    model::{PanPosition, KITAQGB_BUILTIN_WAVES},
    SequenceProject,
};
use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};
const HOLD: u8 = 0xff;
const OFF: u8 = 0xfd;
const CTRL: u8 = 0xfc;
const ROWS_PER_BANK: usize = 2400;
type Row = [u8; 5];

#[derive(Serialize)]
pub struct Report {
    pub directory: PathBuf,
    pub players: usize,
    pub frames: u32,
    pub driver_source: String,
    pub records: Vec<usize>,
    pub warnings: Vec<String>,
}
fn time(p: &SequenceProject, s: u32) -> u32 {
    if let Some(f) = p.frames_per_step_override {
        return s * u32::from(f);
    }
    // Round absolute sixteenth-note boundaries at the native LCD refresh.
    let d = u64::from(p.bpm) * 70_224;
    ((u64::from(s) * 15 * 4_194_304 + d / 2) / d) as u32
}
fn wave_id(w: [u8; 16]) -> u8 {
    let distance = |i: usize| {
        w.iter()
            .zip(KITAQGB_BUILTIN_WAVES[i])
            .map(|(&a, b)| {
                let hi = i32::from(a >> 4) - i32::from(b >> 4);
                let lo = i32::from(a & 15) - i32::from(b & 15);
                hi * hi + lo * lo
            })
            .sum::<i32>()
    };
    u8::from(distance(1) < distance(0))
}
fn encode(
    p: &SequenceProject,
    player: usize,
    strict: bool,
) -> Result<(Vec<Row>, BTreeSet<String>)> {
    let mut warnings = BTreeSet::new();
    let total = p.total_steps();
    let mut positions = BTreeMap::from([(0u32, [HOLD; 4]), (total, [HOLD; 4])]);
    let mut on = BTreeMap::new();
    for (ch, t) in p.tracks[player * 4..player * 4 + 4].iter().enumerate() {
        for n in &t.notes {
            positions.entry(n.end()).or_insert([HOLD; 4])[ch] = OFF;
            positions.entry(n.start).or_insert([HOLD; 4]);
            on.insert((n.start, ch), n);
        }
    }
    let mut rows = Vec::new();
    let mut inst = [None; 4];
    let mut sweep = None;
    let mut wave = None;
    let mut route = None;
    let mut pan = 0xffu8;
    let positions = positions.into_iter().collect::<Vec<_>>();
    for pair in positions.windows(2) {
        let (step, mut notes) = pair[0];
        // A loop begins in a known state even if the previous iteration ended
        // with a held note. HOLD in this format is not a note-off.
        if step == 0 {
            notes = [OFF; 4];
            rows.push([CTRL, 0x17, 0x77, 0, 0]);
        }
        for ch in 0..4 {
            let Some(&n) = on.get(&(step, ch)) else {
                continue;
            };
            let fx = &n.effects;
            let env =
                (n.velocity << 4) | (u8::from(fx.envelope_increase) << 3) | fx.envelope_period;
            let ab = match ch {
                0 | 1 => (fx.duty << 6, env),
                2 => (0, fx.wave_level << 5),
                _ => (0, env),
            };
            if inst[ch] != Some(ab) {
                rows.push([CTRL, 0x11, ch as u8, ab.0, ab.1]);
                inst[ch] = Some(ab);
            }
            if ch == 0 && sweep != Some(fx.nr10()) {
                sweep = Some(fx.nr10());
                rows.push([CTRL, 0x13, fx.nr10(), 0, 0]);
            }
            if ch == 2 {
                let bytes = p.gbdaw_wave_definition(fx.wave_preset)?.bytes16;
                let id = wave_id(bytes);
                if bytes != KITAQGB_BUILTIN_WAVES[id as usize] {
                    let msg = format!(
                        "P{} CH3: wave slot {} -> {} (VBlank supports only triangle and saw)",
                        player + 1,
                        fx.wave_preset + 1,
                        ["triangle", "saw"][id as usize]
                    );
                    if strict {
                        bail!("{msg}");
                    }
                    warnings.insert(msg);
                }
                if wave != Some(id) {
                    rows.push([CTRL, 0x14, id, 0, 0]);
                    wave = Some(id);
                }
            }
            let lr = match fx.pan {
                PanPosition::Left => 0x10,
                PanPosition::Right => 1,
                PanPosition::Center => 0x11,
            };
            pan = (pan & !(0x11 << ch)) | (lr << ch);
            notes[ch] = if ch != 3 {
                n.pitch
            } else {
                let noise = fx.nr43();
                if matches!(noise, 0xfd | 0xff) {
                    let msg = format!(
                        "P{} CH4: NR43 {noise:#04x} is a VBlank marker; using {:#04x}",
                        player + 1,
                        noise - 1
                    );
                    if strict {
                        bail!("{msg}");
                    }
                    warnings.insert(msg);
                    noise - 1
                } else {
                    noise
                }
            };
        }
        if route != Some(pan) {
            rows.push([CTRL, 0x12, pan, 0, 0]);
            route = Some(pan);
        }
        let mut frames = time(p, pair[1].0) - time(p, step);
        while frames > 0 {
            let n = frames.min(251);
            rows.push([(n - 1) as u8, notes[0], notes[1], notes[2], notes[3]]);
            notes = [HOLD; 4];
            frames -= n;
        }
    }
    // Queue producers implement loops by rewinding their ROM cursor, not by
    // enqueuing 0xfe (only the pointer reader interprets a loop marker).
    if !p.loop_playback {
        rows.push([0xff, 0, 0, 0, 0]);
    }
    Ok((rows, warnings))
}
macro_rules! asset {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!("../assets/kitaqgb-vblank/", $name)).as_slice(),
        )
    };
}
const DRIVER: &[(&str, &[u8])] = &[
    asset!("audio.h"),
    asset!("audio.c"),
    asset!("audio_hwregs_gb.c"),
    asset!("audio_vblank.h"),
    asset!("audio_vblank.c"),
    asset!("audio_vblank_control.inc"),
    asset!("audio_vblank_queue.inc"),
    asset!("LICENSE"),
];

/// Validate every part before creating a new package. Never replace an older
/// export or a user's game source tree.
pub fn export(p: &SequenceProject, parent: &Path, strict: bool) -> Result<Report> {
    export_with_driver(p, parent, strict, None)
}
pub fn export_with_driver(
    p: &SequenceProject,
    parent: &Path,
    strict: bool,
    driver: Option<&Path>,
) -> Result<Report> {
    p.validate()?;
    let parts = (0..p.player_count())
        .map(|i| encode(p, i, strict))
        .collect::<Result<Vec<_>>>()?;
    let sdk = driver
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("GBSEQ_KITAQGB_LIB").map(PathBuf::from))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|dir| dir.join("kitaqgb-lib")))
                .filter(|d| d.is_dir())
        });
    let mut files = Vec::new();
    for &(name, bytes) in DRIVER {
        let data = if let Some(d) = &sdk {
            fs::read(d.join(name)).with_context(|| format!("read KITAQGB library file {name}"))?
        } else {
            bytes.to_vec()
        };
        files.push((name, data));
    }
    fs::create_dir_all(parent)?;
    let mut suffix = 0;
    let directory = loop {
        let path = parent.join(if suffix == 0 {
            "GBSEQ_VBLANK".into()
        } else {
            format!("GBSEQ_VBLANK_{suffix}")
        });
        match fs::create_dir(&path) {
            Ok(()) => break path,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => suffix += 1,
            Err(e) => return Err(e.into()),
        }
    };
    let drv = directory.join("driver");
    fs::create_dir(&drv)?;
    for (name, data) in files {
        fs::write(drv.join(name), data)?;
    }
    fs::write(
        drv.join("queue.c"),
        "#include \"audio_vblank.h\"\n#include \"audio_vblank_queue.inc\"\n",
    )?;
    let mut report = Report {
        directory,
        players: p.player_count(),
        frames: time(p, p.total_steps()),
        driver_source: if sdk.is_some() {
            "installed KITAQGB library"
        } else {
            "bundled published KITAQGB library"
        }
        .into(),
        records: vec![],
        warnings: vec![],
    };
    for (idx, (rows, warnings)) in parts.into_iter().enumerate() {
        let part = report.directory.join(format!("P{}", idx + 1));
        fs::create_dir(&part)?;
        write_part(&part, &rows, p.loop_playback).context("write VBlank player sources")?;
        report.records.push(rows.len());
        report.warnings.extend(warnings);
    }
    fs::write(
        report.directory.join("README.md"),
        include_str!("../docs/KITAQGB_VBLANK.md"),
    )?;
    fs::write(
        report.directory.join("manifest.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
fn write_part(dir: &Path, rows: &[Row], looped: bool) -> Result<()> {
    let mut decl = String::new();
    let mut cases = String::new();
    let mut files = String::new();
    let chunks = rows.chunks(ROWS_PER_BANK);
    let n = chunks.len();
    ensure!(n < 126, "song exceeds bank range");
    for (i, rs) in chunks.enumerate() {
        let bank = i + 2;
        let sym = format!("GbseqVB_Data{bank}");
        let file = format!("music_bank{bank:02}.c");
        let mut out = format!(
            "#pragma bank {bank}\n#pragma fixed_bank {bank}\n__prg_rom u8 {sym}[{}] = {{\n",
            rs.len() * 5
        );
        for row in rs {
            writeln!(
                &mut out,
                "  0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X},",
                row[0], row[1], row[2], row[3], row[4]
            )?;
        }
        out.push_str("};\n#pragma fixed_bank -1\n#pragma bank 0\n");
        fs::write(dir.join(&file), out)?;
        writeln!(decl, "extern __prg_rom u8 {sym}[];")?;
        writeln!(
            cases,
            "  if (vb_bank == {bank}) {{ src = {sym}; size = {}; }}",
            rs.len() * 5
        )?;
        write!(files, " {file}")?;
    }
    let producer = include_str!("vblank_player.c.in")
        .replace("@DECL@", &decl)
        .replace("@CASES@", &cases)
        .replace("@LAST@", &(n + 1).to_string())
        .replace(
            "@REPEAT@",
            if looped {
                "vb_bank = 2;"
            } else {
                "vb_done = 1;"
            },
        );
    fs::write(dir.join("music.c"), producer)?;
    fs::write(
        dir.join("music.h"),
        "#pragma once\nvoid GbseqVB_Start();\nvoid GbseqVB_Pump();\n",
    )?;
    fs::write(dir.join("example.c"), include_str!("vblank_example.c.in"))?;
    let size = (n + 2).next_power_of_two().max(4) * 16;
    let build=format!("kitaqgb compile ../driver/audio_hwregs_gb.c ../driver/audio.c ../driver/audio_vblank.c ../driver/queue.c{files} music.c example.c -I ../driver -o player.gb -O1 --stack-bank=fixed --stack-reserve=192 --rst-disable --no-disasm --romsize={size}k --cart=mbc5");
    fs::write(dir.join("build.txt"), format!("{build}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_ids_timing_and_one_shot_end() {
        let mut p = SequenceProject::empty();
        p.measures = 4;
        p.bpm = 120;
        p.loop_playback = false;
        for ch in 0..4 {
            p.insert_note_exact(ch, 0, 2, 12 + ch as u8, 10).unwrap();
        }
        p.tracks[2].notes[0].effects.wave_level = 2;
        let (rows, w) = encode(&p, 0, true).unwrap();
        assert!(w.is_empty());
        assert!(rows.contains(&[CTRL, 0x11, 2, 0, 0x40]));
        assert!(rows.contains(&[CTRL, 0x11, 3, 0, 0xa1]));
        let mut frame = 0;
        let mut timed = vec![];
        for r in &rows {
            if r[0] < 0xfb {
                timed.push((frame, &r[1..]));
                frame += 1 + u32::from(r[0]);
            }
        }
        assert_eq!(timed[0].1, &[12, 13, 14, 0x43]);
        assert_eq!(timed[1], (time(&p, 2), &[OFF; 4][..]));
        assert_eq!(frame, time(&p, 64));
        assert_eq!(rows.last().unwrap()[0], 0xff);
        p.frames_per_step_override = Some(1);
        p.loop_playback = true;
        let r = encode(&p, 0, true).unwrap().0;
        assert_eq!(
            r.iter()
                .filter(|r| r[0] < 0xfb)
                .map(|r| 1 + u32::from(r[0]))
                .sum::<u32>(),
            64
        );
        assert!(r.iter().all(|r| r[0] != 0xfe && r[0] != 0xff));
    }
    #[test]
    fn unsupported_timbres_are_reported_not_silently_lost() {
        let mut p = SequenceProject::empty();
        p.insert_note_exact(2, 0, 2, 28, 10).unwrap();
        p.tracks[2].notes[0].effects.wave_preset = 4;
        p.insert_note_exact(3, 0, 2, 0, 12).unwrap();
        let fx = &mut p.tracks[3].notes[0].effects;
        fx.noise_shift = 15;
        fx.noise_divisor = 7;
        fx.noise_width7 = true;
        assert!(encode(&p, 0, true).is_err());
        let (_, w) = encode(&p, 0, false).unwrap();
        assert_eq!(w.len(), 2);
    }
}
