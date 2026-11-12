//! Estimated chord symbols on a sixteenth-note grid. Noise has no harmonic pitch.
use crate::SequenceProject;
const NAMES: [&str; 12] = [
    "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];
const SHAPES: &[(&str, &[usize])] = &[
    ("", &[0, 4, 7]),
    ("m", &[0, 3, 7]),
    ("dim", &[0, 3, 6]),
    ("aug", &[0, 4, 8]),
    ("sus2", &[0, 2, 7]),
    ("sus4", &[0, 5, 7]),
    ("7", &[0, 4, 7, 10]),
    ("maj7", &[0, 4, 7, 11]),
    ("m7", &[0, 3, 7, 10]),
    ("m7b5", &[0, 3, 6, 10]),
    ("dim7", &[0, 3, 6, 9]),
    ("mMaj7", &[0, 3, 7, 11]),
    ("6", &[0, 4, 7, 9]),
    ("m6", &[0, 3, 7, 9]),
    ("add9", &[0, 2, 4, 7]),
];
fn window(
    p: &SequenceProject,
    lane: Option<usize>,
    start: u32,
    len: u32,
) -> ([u32; 12], Option<u8>) {
    let mut hist = [0; 12];
    let mut bass = None;
    for (i, t) in p
        .tracks
        .iter()
        .take(p.link16_mode.track_count())
        .enumerate()
    {
        if i % 4 == 3 || lane.is_some_and(|x| x != i) {
            continue;
        }
        let ix = t.notes.partition_point(|n| n.end() <= start);
        for n in t.notes[ix..].iter().take_while(|n| n.start < start + len) {
            if n.velocity == 0 {
                continue;
            }
            let duration = n.end().min(start + len) - n.start.max(start);
            hist[n.pitch as usize % 12] += duration;
            if n.start <= start {
                bass = Some(bass.map_or(n.pitch, |b: u8| b.min(n.pitch)));
            }
        }
    }
    (hist, bass.map(|b| b % 12))
}
fn classify(h: &[u32; 12], bass: Option<u8>, inversions: bool) -> Option<String> {
    let total: u32 = h.iter().sum();
    let mut best = None;
    let mut high = i64::MIN;
    for root in 0..12 {
        for &(suffix, shape) in SHAPES {
            let mut inside = 0;
            let mut weak = u32::MAX;
            for &interval in shape {
                let wt = h[(root + interval) % 12];
                inside += wt;
                weak = weak.min(wt);
            }
            if weak == 0 || inside * 5 < total * 4 {
                continue;
            }
            let score = i64::from(inside) * 120 - i64::from(total - inside) * 150
                + i64::from(weak) * 7
                - shape.len() as i64 * 3
                + if bass == Some(root as u8) { 12 } else { 0 };
            if score > high {
                high = score;
                best = Some((root, suffix, shape));
            }
        }
    }
    best.map(|(r, s, shape)| {
        let mut label = format!("{}{s}", NAMES[r]);
        if let Some(b) = bass.filter(|&b| {
            inversions && b as usize != r && shape.contains(&((b as usize + 12 - r) % 12))
        }) {
            label.push('/');
            label.push_str(NAMES[b as usize]);
        }
        label
    })
}
pub fn chord(p: &SequenceProject, lane: Option<usize>, beat: u32) -> Option<String> {
    if beat >= p.total_steps() / 4 {
        return None;
    }
    let start = beat * 4;
    let (h, b) = window(p, lane, start, 4);
    if h.iter().all(|v| *v == 0) {
        return None;
    }
    if let Some(c) = classify(&h, b, lane.is_none()) {
        return Some(c);
    }
    if h.iter().filter(|&&n| n > 0).count() >= 3 {
        return None;
    }
    for len in [8, 16] {
        let (w, b) = window(p, lane, start / len * len, len);
        if let Some(c) = classify(&w, b, false) {
            return Some(c);
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn walks_progression_detects_inversions_and_ignores_drums() {
        let mut p = SequenceProject::empty();
        p.measures = 2;
        for (i, n) in [12, 16, 19].into_iter().enumerate() {
            p.insert_note_exact(i, 0, 4, n, 15).unwrap();
        }
        for (i, n) in [11, 14, 19].into_iter().enumerate() {
            p.insert_note_exact(i, 4, 4, n, 15).unwrap();
        }
        p.insert_note_exact(3, 0, 8, 13, 15).unwrap();
        assert_eq!(chord(&p, None, 0).as_deref(), Some("C"));
        assert_eq!(chord(&p, None, 1).as_deref(), Some("G/B"));
        assert_eq!(chord(&p, Some(0), 0), None);
        assert_eq!(chord(&p, Some(3), 1), None);
        assert_eq!(chord(&p, None, 2), None);
        for (i, n) in [21, 24, 28, 31].into_iter().enumerate() {
            p.insert_note_exact(0, 16 + i as u32, 1, n, 15).unwrap();
        }
        assert_eq!(chord(&p, Some(0), 4).as_deref(), Some("Am7"));
    }
}
