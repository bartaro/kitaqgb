use super::{Cell, Part};

fn word(out: &mut Vec<u8>, v: u32) {
    out.extend(v.to_le_bytes())
}
// Pascal short strings have a BYTE length and exactly 255 payload bytes.
fn text(out: &mut Vec<u8>, s: &str) {
    let mut n = s.len().min(255);
    while !s.is_char_boundary(n) {
        n -= 1
    }
    out.push(n as u8);
    out.extend(&s.as_bytes()[..n]);
    out.resize(out.len() + 255 - n, 0);
}
fn cell(out: &mut Vec<u8>, c: Cell) {
    let (effect, param) = c.fx.unwrap_or_default();
    for v in [c.note.unwrap_or(90), c.inst, 0, effect] {
        word(out, v as u32)
    }
    out.push(param);
}
pub(super) fn encode(s: &Part) -> Vec<u8> {
    let mut out = Vec::new();
    word(&mut out, 6);
    text(&mut out, &s.title);
    text(&mut out, &s.author);
    text(&mut out, "GBSEQ");
    for (kind, bank) in s.banks.iter().enumerate() {
        for i in 0..15 {
            let a = bank.get(i).copied().unwrap_or_default();
            word(&mut out, kind as u32);
            text(
                &mut out,
                &if i < bank.len() {
                    format!("{} {:02}", ["Pulse", "Wave", "Noise"][kind], i + 1)
                } else {
                    String::new()
                },
            );
            word(&mut out, 0);
            out.extend([0, a.volume]);
            word(&mut out, a.down as u32);
            out.push(a.period);
            for v in [a.sweep, a.neg as u8, a.shift] {
                word(&mut out, v as u32)
            }
            out.push(a.duty);
            for v in [a.level, a.wave, a.narrow as u8] {
                word(&mut out, v as u32)
            }
            out.push(0); // disabled instrument subpattern
            for _ in 0..64 {
                cell(&mut out, Cell::default())
            }
        }
    }
    for i in 0..16 {
        out.extend(s.waves.get(i).unwrap_or(&[0; 32]))
    }
    word(&mut out, s.speed as u32);
    out.push(0);
    word(&mut out, 0);
    let orders = s.rows.len() / 64;
    word(&mut out, (orders * 4) as u32);
    for (i, pattern) in s.rows.chunks_exact(64).enumerate() {
        for ch in 0..4 {
            word(&mut out, (4 * i + ch) as u32);
            for row in pattern {
                cell(&mut out, row[ch]);
            }
        }
    }
    for ch in 0..4 {
        word(&mut out, (orders + 1) as u32);
        for i in 0..orders {
            word(&mut out, (4 * i + ch) as u32)
        }
        word(&mut out, 0); // hUGETracker order-list sentinel
    }
    for _ in 0..16 {
        word(&mut out, 0)
    } // empty routines
    out
}
