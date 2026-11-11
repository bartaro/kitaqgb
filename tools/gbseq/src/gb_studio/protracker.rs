use super::{noise_rate, MidiNote, Part, SequenceProject, Warnings};
use crate::model::PanPosition;

// Periods recognized by the GBT converter (not a continuously rounded equal-tempered table).
const PERIOD: [u16; 48] = [
    1712, 1616, 1524, 1440, 1356, 1280, 1208, 1140, 1076, 1016, 960, 907, 856, 808, 762, 720, 678,
    640, 604, 570, 538, 508, 480, 453, 428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, 226,
    214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113,
];
const NOISE: [u8; 16] = [
    0x5f, 0x4e, 0x3e, 0x2f, 0x2e, 0x2c, 0x1f, 0x0f, 0x64, 0x54, 0x44, 0x24, 0x00, 0x67, 0x56, 0x46,
];
const DUTY: [u8; 4] = [4, 1, 2, 3];
pub(super) fn note(p: &SequenceProject, ch: usize, n: &MidiNote, w: &mut Warnings) -> (u8, u8) {
    let e = &n.effects;
    if e.pan != PanPosition::Center
        || (ch != 2 && e.envelope_period != 0)
        || (ch == 0 && e.sweep_shift != 0)
    {
        w.insert(
            "MOD uses note volume; pan, envelopes and hardware sweep are not exported.".into(),
        );
    }
    let inst = match ch {
        0 | 1 => DUTY[e.duty.min(3) as usize],
        2 => {
            w.insert(
                "MOD replaces CH3 waves with fixed GBT instruments. Use UGE to keep user waves."
                    .into(),
            );
            match p.wave_slots[e.wave_preset as usize].source {
                0 => 12,
                1 => 13,
                2 => 8,
                3 => 10,
                4 => 15,
                5 => 9,
                6 => 11,
                _ => 15,
            }
        }
        _ => {
            w.insert("MOD percussion uses the nearest fixed GBT noise instrument.".into());
            let target = noise_rate(e.noise_shift << 4 | e.noise_divisor);
            let i = NOISE
                .iter()
                .enumerate()
                .filter(|(_, nr)| (**nr & 8 != 0) == e.noise_width7)
                .min_by(|(_, a), (_, b)| {
                    (noise_rate(**a) - target)
                        .abs()
                        .total_cmp(&(noise_rate(**b) - target).abs())
                })
                .unwrap()
                .0;
            return (24, 16 + i as u8);
        }
    };
    (n.pitch, inst)
}
fn ascii(out: &mut Vec<u8>, text: &str, n: usize) {
    let start = out.len();
    out.extend(text.chars().take(n).map(|c| {
        if c.is_ascii_graphic() || c == ' ' {
            c as u8
        } else {
            b'?'
        }
    }));
    out.resize(start + n, 0);
}
// Original, synthetic PCM for ordinary MOD players. GBT instead interprets the slot number.
fn pcm(slot: u8) -> Vec<u8> {
    let n = if slot >= 16 { 1024 } else { 32 };
    let mut reg = 0x7fffu16;
    (0..n)
        .map(|i| {
            let x = i as f64 / 32.0;
            let a = match slot {
                1..=4 => {
                    if i < [8, 16, 24, 4][(slot - 1) as usize] {
                        1.0
                    } else {
                        -1.0
                    }
                }
                8..=15 => match slot {
                    13 => 1.0 - 2.0 * x,
                    14 => {
                        if i < 16 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    15 => (x * std::f64::consts::TAU).sin(),
                    _ => {
                        ((x * std::f64::consts::TAU).sin()
                            + 0.35 * (x * (slot - 6) as f64 * std::f64::consts::TAU).sin())
                            / 1.35
                    }
                },
                16..=31 => {
                    let nr = NOISE[(slot - 16) as usize];
                    if i % (1 + ((nr >> 4) as usize / 2)) == 0 {
                        let fb = (reg ^ (reg >> 1)) & 1;
                        reg = (reg >> 1) | (fb << 14);
                        if nr & 8 != 0 {
                            reg = (reg & !64) | (fb << 6);
                        }
                    }
                    if reg & 1 == 0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                _ => 0.0,
            };
            (a * 100.0) as i8 as u8
        })
        .collect()
}
pub(super) fn encode(s: &mut Part) -> Vec<u8> {
    let mut out = Vec::new();
    ascii(&mut out, &s.title, 20);
    let samples: Vec<_> = (1..=31).map(pcm).collect();
    for (i, sample) in samples.iter().enumerate() {
        ascii(&mut out, &format!("GBSEQ {:02}", i + 1), 22);
        let words = (sample.len() as u16 / 2).to_be_bytes();
        out.extend(words);
        out.extend([0, 64, 0, 0]);
        out.extend(words);
    }
    let count = s.rows.len() / 64;
    out.extend([count as u8, 0x7f]);
    out.extend((0..128).map(|i| if i < count { i as u8 } else { 0 }));
    out.extend(b"M.K.");
    for row in &s.rows {
        for c in row {
            let period = c.note.map_or(0, |n| PERIOD[n as usize]);
            let (fx, param) = c.fx.unwrap_or_default();
            out.extend([
                (c.inst & 0xf0) | (period >> 8) as u8,
                period as u8,
                (c.inst << 4) | fx,
                param,
            ]);
        }
    }
    for data in samples {
        out.extend(data)
    }
    out
}
