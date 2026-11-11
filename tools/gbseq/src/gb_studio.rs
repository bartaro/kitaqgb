//! Standalone writers for GB Studio. Order/row positions are shared by every
//! exported part; edit selection, monitor, mute and solo never discard tracks.
use crate::{model::PanPosition, MidiNote, SequenceProject};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

mod protracker;
mod uge;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Uge,
    Mod,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Uge => "uge",
            Self::Mod => "mod",
        }
    }
}
#[derive(Serialize)]
pub struct Report {
    pub files: Vec<PathBuf>,
    pub warnings: BTreeSet<String>,
}
type Warnings = BTreeSet<String>;
#[derive(Clone, Copy, Default)]
struct Cell {
    note: Option<u8>,
    inst: u8,
    fx: Option<(u8, u8)>,
}
type Row = [Cell; 4];

pub fn export(p: &SequenceProject, path: &Path, fmt: Format) -> Result<Report> {
    p.validate()?;
    let ext = fmt.extension();
    ensure!(
        path.extension().is_none_or(|e| e.eq_ignore_ascii_case(ext)),
        "Expected .{ext} output"
    );
    let mut report = Report {
        files: vec![],
        warnings: Warnings::new(),
    };
    let mut parts = vec![];
    for player in 0..p.player_count() {
        let mut dest = path.with_extension(ext);
        if p.player_count() > 1 {
            let mut name = path
                .file_stem()
                .context("Missing output filename")?
                .to_owned();
            name.push(format!("_P{}.{ext}", player + 1));
            dest.set_file_name(name);
            ensure!(
                !dest.exists(),
                "{} already exists; choose another base name",
                dest.display()
            );
        }
        let mut part = Part::new(p, player, fmt, &mut report.warnings)?;
        let bytes = match fmt {
            Format::Uge => uge::encode(&part),
            Format::Mod => protracker::encode(&mut part),
        };
        parts.push((dest, bytes));
    }
    // Resolve all parts, instrument limits and file names before writing any.
    for (dest, bytes) in parts {
        fs::write(&dest, bytes).with_context(|| format!("Write {}", dest.display()))?;
        report.files.push(dest);
    }
    Ok(report)
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Inst {
    volume: u8,
    period: u8,
    down: bool,
    duty: u8,
    sweep: u8,
    neg: bool,
    shift: u8,
    wave: u8,
    level: u8,
    narrow: bool,
}
struct Part {
    title: String,
    author: String,
    rows: Vec<Row>,
    banks: [Vec<Inst>; 3],
    waves: Vec<[u8; 32]>,
    speed: u8,
}
impl Part {
    fn new(p: &SequenceProject, player: usize, fmt: Format, w: &mut Warnings) -> Result<Self> {
        let steps = p.total_steps() as usize;
        let orders = steps.div_ceil(64);
        // Each channel only has one effect slot per row. Never sacrifice
        // volume/gates for tempo or flow; choose the same layout for all parts.
        let busy = |row: u32| {
            p.tracks[..p.player_count() * 4]
                .chunks_exact(4)
                .any(|tracks| {
                    tracks.iter().all(|track| {
                        let on = track.notes.iter().any(|n| n.start == row);
                        if on {
                            return fmt == Format::Mod;
                        }
                        track.notes.iter().any(|n| {
                            n.end() == row
                                || (row == 0 && p.loop_playback && n.end() == steps as u32)
                        })
                    })
                })
        };
        let setup = fmt == Format::Mod && busy(0);
        let flow = steps % 64 != 0 || (setup && p.loop_playback);
        let late = flow && busy(steps as u32 - 1);
        let tail_order = usize::from(steps % 64 == 0 && late);
        let silent = orders + tail_order;
        let song_orders = silent + usize::from(!p.loop_playback);
        let mut s = Self {
            title: if p.player_count() > 1 {
                format!("{} P{}", p.title, player + 1)
            } else {
                p.title.clone()
            },
            author: p.author.clone(),
            rows: vec![[Cell::default(); 4]; song_orders * 64],
            banks: Default::default(),
            waves: vec![],
            speed: 1,
        };
        let frames = p
            .frames_per_step_override
            .map(f64::from)
            .unwrap_or(900.0 / p.bpm as f64);
        // GBT converts the 50 Hz tracker speed into a whole number of GB ticks.
        s.speed = match fmt {
            Format::Uge => frames.round() as u8,
            Format::Mod => (1..=31u8)
                .min_by(|a, b| {
                    let err = |v: u8| ((v as f64 * 1.2).round() - frames).abs();
                    err(*a).total_cmp(&err(*b))
                })
                .unwrap(),
        }
        .max(1);
        let ticks = if fmt == Format::Uge {
            s.speed as f64
        } else {
            (s.speed as f64 * 1.2).round()
        };
        if (ticks - frames).abs() > 0.01 {
            w.insert("Tempo rounded to whole tracker ticks.".into());
        }
        let cut = Some(if fmt == Format::Uge { (14, 0) } else { (12, 0) });
        let mut pans = vec![[None; 4]; steps];
        for ch in 0..4 {
            let mut notes = p.tracks[player * 4 + ch].notes.iter().collect::<Vec<_>>();
            notes.sort_by_key(|n| n.start);
            for (i, n) in notes.iter().enumerate() {
                let at = n.start as usize;
                let (pitch, inst) = if fmt == Format::Uge {
                    s.uge_note(p, ch, n, w)?
                } else {
                    protracker::note(p, ch, n, w)
                };
                let fx = if fmt == Format::Mod {
                    let vol = if ch == 2 {
                        [0, 64, 32, 16][n.effects.wave_level.clamp(1, 3) as usize]
                    } else {
                        if n.velocity == 15 {
                            64
                        } else {
                            4 * n.velocity
                        }
                    };
                    Some((12, vol))
                } else {
                    None
                };
                s.rows[at][ch] = Cell {
                    note: Some(pitch),
                    inst,
                    fx,
                };
                pans[at][ch] = Some(n.effects.pan);
                let end = n.end() as usize;
                if end < steps && notes.get(i + 1).is_none_or(|next| next.start != n.end()) {
                    s.rows[end][ch].fx = cut;
                } else if end == steps && p.loop_playback && notes[0].start != 0 {
                    s.rows[0][ch].fx = cut;
                }
            }
        }
        if fmt == Format::Mod && !setup {
            s.rows[0].iter_mut().find(|c| c.fx.is_none()).unwrap().fx = Some((15, s.speed));
        }
        let order_base = u8::from(fmt == Format::Uge) + u8::from(setup);
        if flow {
            let target = if p.loop_playback { 0 } else { silent as u8 };
            let last = steps - 1 + usize::from(late);
            s.rows[last].iter_mut().find(|c| c.fx.is_none()).unwrap().fx =
                Some((11, target + order_base));
            if late {
                w.insert("The final pattern needs one extra row for its jump.".into());
            }
        }
        if !p.loop_playback {
            for c in &mut s.rows[silent * 64] {
                c.fx = cut;
            }
            s.rows[silent * 64 + 1][0].fx = Some((11, silent as u8 + order_base));
        }
        if setup {
            let mut init = vec![[Cell::default(); 4]; 64];
            init[0][0].fx = Some((15, s.speed));
            init[0][1].fx = Some((11, 1));
            init.append(&mut s.rows);
            s.rows = init;
            w.insert(
                "MOD uses one startup row to set tempo without losing a note or its volume.".into(),
            );
        }
        let mut mask = 0xffu8;
        if fmt == Format::Uge {
            let mut previous = 0xff;
            for (at, changes) in pans.iter().enumerate() {
                for (ch, &v) in changes.iter().enumerate() {
                    if let Some(pan) = v {
                        mask &= !(0x11 << ch);
                        mask |= (match pan {
                            PanPosition::Left => 0x10,
                            PanPosition::Right => 1,
                            PanPosition::Center => 0x11,
                        }) << ch;
                    }
                }
                if at == 0 || mask != previous {
                    if let Some(c) = s.rows[at].iter_mut().find(|c| c.fx.is_none()) {
                        c.fx = Some((8, mask));
                        previous = mask;
                    }
                }
            }
        }
        Ok(s)
    }
    fn uge_note(
        &mut self,
        p: &SequenceProject,
        ch: usize,
        n: &MidiNote,
        w: &mut Warnings,
    ) -> Result<(u8, u8)> {
        let e = &n.effects;
        let mut inst = Inst::default();
        let bank = [0, 0, 1, 2][ch];
        let mut pitch = n.pitch;
        if ch != 2 {
            inst.volume = n.velocity;
            inst.period = e.envelope_period;
            inst.down = !e.envelope_increase && inst.period != 0;
        }
        if ch < 2 {
            inst.duty = e.duty;
            if ch == 0 {
                inst.sweep = e.sweep_period;
                inst.neg = e.sweep_negate && e.sweep_shift > 0;
                inst.shift = e.sweep_shift;
            }
        }
        if ch == 2 {
            let packed = p.gbdaw_wave_definition(e.wave_preset)?.bytes16;
            let wave = std::array::from_fn(|i| {
                if i % 2 == 0 {
                    packed[i / 2] >> 4
                } else {
                    packed[i / 2] & 15
                }
            });
            inst.wave = self
                .waves
                .iter()
                .position(|v| v == &wave)
                .unwrap_or_else(|| {
                    self.waves.push(wave);
                    self.waves.len() - 1
                }) as u8;
            inst.level = e.wave_level.clamp(1, 3);
        }
        if ch == 3 {
            inst.narrow = e.noise_width7;
            let target = noise_rate(e.noise_shift << 4 | e.noise_divisor);
            let distance = |id: u8| {
                let k = 63 - id;
                let nr = if k < 7 {
                    k
                } else {
                    ((k / 4 - 1) << 4) | 4 | (k % 4)
                };
                (noise_rate(nr) - target).abs()
            };
            pitch = (0..64)
                .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
                .unwrap();
            if distance(pitch) > 0.001 {
                w.insert("Noise pitch approximated by the tracker noise table.".into());
            }
        }
        let slots = &mut self.banks[bank];
        let idx = if let Some(idx) = slots.iter().position(|v| *v == inst) {
            idx
        } else if slots.len() < 15 {
            slots.push(inst);
            slots.len() - 1
        } else {
            w.insert(
                "UGE exceeded 15 instruments of one type; the closest existing timbre was used."
                    .into(),
            );
            slots
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| {
                    1000 * u32::from(v.wave != inst.wave)
                        + 128 * u32::from(v.narrow != inst.narrow)
                        + 32 * u32::from(v.duty != inst.duty)
                        + 16 * u32::from(v.level.abs_diff(inst.level))
                        + 8 * u32::from(v.neg != inst.neg)
                        + 4 * u32::from(v.down != inst.down)
                        + u32::from(
                            v.period.abs_diff(inst.period)
                                + v.shift.abs_diff(inst.shift)
                                + v.sweep.abs_diff(inst.sweep)
                                + v.volume.abs_diff(inst.volume),
                        )
                })
                .unwrap()
                .0
        };
        Ok((pitch, idx as u8 + 1))
    }
}
fn noise_rate(nr: u8) -> f64 {
    let r = nr & 7;
    (if r == 0 { 0.5 } else { r as f64 }).log2() + (nr >> 4) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Link16Mode, UserWave};

    #[test]
    fn events_across_order_boundary_and_one_shot_silence() {
        let mut p = SequenceProject::empty();
        p.set_measures(5);
        p.loop_playback = false;
        p.insert_note_exact(0, 63, 3, 42, 13).unwrap();
        p.insert_note_exact(0, 66, 2, 24, 12).unwrap();
        for fmt in [Format::Uge, Format::Mod] {
            let s = Part::new(&p, 0, fmt, &mut Warnings::new()).unwrap();
            assert_eq!(s.rows.len(), 192);
            assert_eq!(s.rows[63][0].note, Some(42));
            assert_eq!(s.rows[64][0].note, None);
            assert_eq!(s.rows[66][0].note, Some(24));
            let cut = Some(if fmt == Format::Uge { (14, 0) } else { (12, 0) });
            assert_eq!(s.rows[68][0].fx, cut);
            let jump = Some((11, if fmt == Format::Uge { 3 } else { 2 }));
            assert!(s.rows[79].iter().any(|c| c.fx == jump));
            assert!(s.rows[128].iter().all(|c| c.fx == cut));
            assert_eq!(s.rows[129][0].fx, jump);
        }
    }

    #[test]
    fn exact_duty_noise_and_user_wave_are_preserved_in_uge() {
        let mut p = SequenceProject::empty();
        p.frames_per_step_override = Some(4);
        p.user_waves.push(UserWave {
            name: "Ramp".into(),
            samples: (0..32).map(|i| i % 16).collect(),
        });
        p.wave_slots[7].source = 0x80;
        for ch in 0..4 {
            p.insert_note_exact(ch, 0, 4, 12, 9).unwrap();
        }
        let s = Part::new(&p, 0, Format::Uge, &mut Warnings::new()).unwrap();
        assert_eq!(s.banks[0][0].duty, 2);
        assert_eq!(s.banks[0][0].volume, 9);
        assert_eq!(s.speed, 4);
        assert_eq!(s.waves[0].as_slice(), p.user_waves[0].samples);
        assert_eq!(s.rows[0][2].note, Some(12));
        assert!(s.rows[0].iter().any(|c| c.fx == Some((8, 0xbd))));
        let b = uge::encode(&s);
        assert_eq!(&b[..4], &6u32.to_le_bytes());
    }

    #[test]
    fn mod_has_real_samples_legal_slots_and_distinct_parts_even_when_muted() {
        let mut p = SequenceProject::empty();
        p.set_measures(256);
        p.set_link16_mode(Link16Mode::Dmg07_16);
        for ch in 0..16 {
            p.insert_note_exact(ch, 4095, 1, ch as u8, 15).unwrap();
            p.tracks[ch].mute = true;
        }
        for player in 0..4 {
            let mut s = Part::new(&p, player, Format::Mod, &mut Warnings::new()).unwrap();
            let last = &s.rows[64 + 4095]; // startup order protects all four wraparound gates
            assert_eq!(last[0].note, Some((4 * player) as u8));
            assert_eq!(last[0].inst, 2);
            assert_eq!(last[1].inst, 1);
            assert!((8..=15).contains(&last[2].inst));
            assert_eq!(last[3].note, Some(24));
            assert!((16..=31).contains(&last[3].inst));
            assert_eq!(s.rows[65 * 64][0].fx, Some((11, 1)));
            let b = protracker::encode(&mut s);
            assert_eq!(b[950], 66);
            assert_eq!(&b[1080..1084], b"M.K.");
            assert!(b[1084 + 66 * 1024..].iter().any(|&x| x > 127));
        }
    }
}
