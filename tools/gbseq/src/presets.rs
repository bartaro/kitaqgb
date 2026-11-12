//! Original APU patches; GM program numbers are not Game Boy timbres.
use crate::{NoteEffects, SequenceProject, TrackKind, UserWave};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Patch {
    pub name: String,
    pub volume: u8,
    pub effects: NoteEffects,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Preset {
    pub format: String,
    pub channel: TrackKind,
    pub patch: Patch,
    pub wave: Option<[u8; 16]>,
}
const FMT: &str = "GBSEQ-VOICE-1";
const FAMILIES: [[&str; 16]; 4] = [
    [
        "Beacon",
        "Spark",
        "Pulse Harp",
        "Falling Star",
        "Copper Wire",
        "Comet",
        "Rocket",
        "Pinwheel",
        "Laser Bass",
        "Arc Lead",
        "Twinkle",
        "Echo Bell",
        "Glider",
        "Falcon",
        "Spiral",
        "Meteor",
    ],
    [
        "Needle",
        "Glass Pluck",
        "Reed",
        "Hollow Key",
        "Paper Bass",
        "Soft Reed",
        "Pizzicato",
        "Lantern",
        "Velvet",
        "Round Bass",
        "Organ",
        "Night Train",
        "Wire Bass",
        "Brass Key",
        "Tin Horn",
        "Frost",
    ],
    [
        "Flute",
        "Reed Pipe",
        "Mallet",
        "Glass",
        "Soft Brass",
        "Air Column",
        "Drawbar",
        "Hollow Tube",
        "Marimba",
        "Crystal",
        "Copper Bell",
        "Clavinet",
        "Reed Bass",
        "Silver String",
        "Chime",
        "Prism",
    ],
    [
        "Silk Hat",
        "Closed Hat",
        "Open Hat",
        "Shaker",
        "Sand",
        "Wire Brush",
        "Snare",
        "Rim",
        "Click",
        "Wood Tap",
        "Tom",
        "Grain",
        "Engine",
        "Static",
        "Crash",
        "Thunder",
    ],
];
pub fn factory(kind: TrackKind, id: u8) -> Preset {
    let ch = TrackKind::ALL.iter().position(|k| *k == kind).unwrap();
    let f = (id & 127) / 8;
    let v = id % 8;
    let mut e = NoteEffects::for_track(kind);
    e.pan = crate::model::PanPosition::Center;
    e.duty = f % 4;
    e.envelope_period = v;
    e.envelope_increase = false;
    let mut volume = 15 - f / 4 * 2;
    let mut wave = None;
    match kind {
        TrackKind::Pulse1 => {
            if f >= 4 {
                e.sweep_negate = true;
                e.sweep_period = 1 + f / 4;
                e.sweep_shift = 1 + f % 7;
            }
        }
        TrackKind::Pulse2 => {}
        TrackKind::Noise => {
            e.noise_shift = f / 2;
            e.noise_width7 = f % 2 == 0;
            e.noise_divisor = v;
            e.envelope_period = 1 + f % 5;
        }
        TrackKind::Wave => {
            volume = 15;
            e.wave_level = 1;
            let mut a = [0.; 32];
            for (i, s) in a.iter_mut().enumerate() {
                let t = i as f64 * std::f64::consts::TAU / 32.;
                let b = 2. + f64::from(f % 7);
                let c = 2. + f64::from(f / 4);
                *s = t.sin()
                    + (0.16 + f64::from(v) * 0.095) * (t * b + f64::from(f / 7) * 0.7).sin()
                    + (0.12 + f64::from(f) * 0.017) * (t * c + f64::from(v) * 0.41).cos();
            }
            let peak = a.iter().map(|x| x.abs()).fold(0., f64::max);
            let nib = |i: usize| ((a[i] / peak + 1.) * 7.5).round().clamp(0., 15.) as u8;
            wave = Some(std::array::from_fn(|i| (nib(i * 2) << 4) | nib(i * 2 + 1)));
        }
    }
    Preset {
        format: FMT.into(),
        channel: kind,
        patch: Patch {
            name: format!(
                "{:03} {} {}",
                id as u16 + 1,
                FAMILIES[ch][f as usize],
                v + 1
            ),
            volume,
            effects: e,
        },
        wave,
    }
}
impl Patch {
    pub fn validate(&self) -> Result<()> {
        let mut e = self.effects.clone();
        e.clamp();
        ensure!(
            !self.name.trim().is_empty()
                && self.name.len() <= 256
                && self.volume <= 15
                && e == self.effects,
            "invalid preset parameters"
        );
        Ok(())
    }
}
impl Preset {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.format == FMT && self.wave.is_some() == (self.channel == TrackKind::Wave),
            "invalid GBSEQ preset type"
        );
        self.patch.validate()
    }
    pub fn load(path: &Path) -> Result<Self> {
        ensure!(
            std::fs::metadata(path)?.len() <= 32768,
            "preset exceeds 32 KiB"
        );
        let p: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        p.validate()?;
        Ok(p)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
    pub fn capture(p: &SequenceProject, i: usize, selected: Option<u64>) -> Result<Self> {
        let t = &p.tracks[i];
        let mut patch = t.input_patch.clone().unwrap_or_else(|| Patch {
            name: t.name.clone(),
            volume: 15,
            effects: NoteEffects::for_track(t.kind),
        });
        if let Some(n) = selected.and_then(|id| t.notes.iter().find(|n| n.id == id)) {
            patch.volume = n.velocity;
            patch.effects = n.effects.clone();
        }
        let wave = if t.kind == TrackKind::Wave {
            Some(p.gbdaw_wave_definition(patch.effects.wave_preset)?.bytes16)
        } else {
            None
        };
        Ok(Self {
            format: FMT.into(),
            channel: t.kind,
            patch,
            wave,
        })
    }
    pub fn apply(&self, p: &mut SequenceProject, i: usize, ids: &[u64]) -> Result<()> {
        self.validate()?;
        ensure!(
            p.tracks.get(i).is_some_and(|t| t.kind == self.channel),
            "preset belongs to another channel"
        );
        let ids: BTreeSet<_> = ids.iter().copied().collect();
        let mut patch = self.patch.clone();
        if let Some(bytes) = self.wave {
            let level = patch.effects.wave_level;
            let found = (0..16).find(|&s| {
                p.wave_slots[s].level == level
                    && p.gbdaw_wave_definition(s as u8)
                        .is_ok_and(|w| w.bytes16 == bytes)
            });
            let slot = if let Some(s) = found {
                s
            } else {
                let mut used = [false; 16];
                for (j, t) in p
                    .tracks
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.kind == TrackKind::Wave)
                {
                    if j != i {
                        if let Some(prev) = &t.input_patch {
                            used[usize::from(prev.effects.wave_preset.min(15))] = true;
                        }
                    }
                    for n in &t.notes {
                        if j != i || !ids.contains(&n.id) {
                            used[usize::from(n.effects.wave_preset.min(15))] = true;
                        }
                    }
                }
                let s = used
                    .iter()
                    .rposition(|b| !*b)
                    .context("All 16 CH3 slots are used; existing sounds were preserved")?;
                let samples: Vec<_> = bytes.iter().flat_map(|b| [b >> 4, b & 15]).collect();
                let existing = p.user_waves.iter().position(|w| w.samples == samples);
                let reuse = p.user_waves.iter().enumerate().position(|(i, w)| {
                    w.preset_cache
                        && !p
                            .wave_slots
                            .iter()
                            .enumerate()
                            .any(|(j, v)| j != s && v.source == 0x80 | i as u8)
                });
                let source = if let Some(index) = existing {
                    index
                } else if let Some(index) = reuse {
                    p.user_waves[index] = UserWave {
                        name: patch.name.clone(),
                        samples,
                        preset_cache: true,
                    };
                    index
                } else {
                    ensure!(
                        p.user_waves.len() < 64,
                        "All 64 user waves are used; existing waves were preserved"
                    );
                    p.user_waves.push(UserWave {
                        name: patch.name.clone(),
                        samples,
                        preset_cache: true,
                    });
                    p.user_waves.len() - 1
                };
                p.wave_slots[s] = crate::WavePresetSlot {
                    source: 0x80 | source as u8,
                    level,
                };
                s
            };
            patch.effects.wave_preset = slot as u8;
        }
        for n in &mut p.tracks[i].notes {
            if ids.contains(&n.id) {
                n.velocity = patch.volume;
                n.effects = patch.effects.clone();
            }
        }
        p.tracks[i].input_patch = Some(patch);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_banks_have_distinct_sounding_parameters_and_travel_with_the_song() {
        for (lane, kind) in TrackKind::ALL.into_iter().enumerate() {
            let mut seen = BTreeSet::new();
            let mut p = SequenceProject::empty();
            for i in 0..128 {
                let v = factory(kind, i);
                let mut def = v.patch.clone();
                def.name.clear();
                assert!(seen.insert(serde_json::to_string(&(def, v.wave)).unwrap()));
                v.apply(&mut p, lane, &[]).unwrap();
                assert_eq!(Preset::capture(&p, lane, None).unwrap().wave, v.wave);
                assert_eq!(
                    p.tracks[lane].input_patch.as_ref().unwrap().volume,
                    v.patch.volume
                );
            }
            assert!(p.user_waves.len() <= 1);
            let id = p.add_note(lane, 0, 20).unwrap();
            assert_eq!(
                p.note(id).unwrap().effects,
                p.tracks[lane].input_patch.as_ref().unwrap().effects
            );
            let q: SequenceProject =
                serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
            q.validate().unwrap();
            assert_eq!(q, p);
        }
    }
    #[test]
    fn applying_wave_keeps_sounding_notes_and_other_player_defaults() {
        let mut p = SequenceProject::empty();
        p.set_link16_mode(crate::Link16Mode::Cable8);
        factory(TrackKind::Wave, 8).apply(&mut p, 2, &[]).unwrap();
        let a = p.add_note(2, 0, 20).unwrap();
        let old = Preset::capture(&p, 2, Some(a)).unwrap();
        factory(TrackKind::Wave, 27).apply(&mut p, 6, &[]).unwrap();
        factory(TrackKind::Wave, 66).apply(&mut p, 2, &[]).unwrap();
        assert_eq!(Preset::capture(&p, 2, Some(a)).unwrap().wave, old.wave);
        assert_eq!(
            Preset::capture(&p, 6, None).unwrap().wave,
            factory(TrackKind::Wave, 27).wave
        );
    }
}
