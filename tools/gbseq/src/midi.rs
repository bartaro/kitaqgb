//! Musical interchange, not a serialization of Game Boy instruments.
use crate::{Link16Mode, MidiNote, NoteEffects, SequenceProject};
use anyhow::{bail, ensure, Context, Result};
use midly::{
    Format, Header, MetaMessage as Meta, MidiMessage as Msg, Smf, Timing, TrackEvent as Event,
    TrackEventKind as Kind,
};
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
};

pub const BASE: u8 = 36; // C2 (middle C = MIDI 60)
pub fn channel(lane: usize) -> u8 {
    match lane {
        3 => 9,
        9 => 3,
        n => n as u8,
    }
}
pub fn velocity(v: u8) -> u8 {
    ((v as u16 * 127 + 7) / 15) as u8
}
pub fn level(v: u8) -> u8 {
    ((v as u16 * 15 + 63) / 127).max(1) as u8
}
fn event<'a>(delta: u32, kind: Kind<'a>) -> Event<'a> {
    Event {
        delta: delta.into(),
        kind,
    }
}
fn note(ch: u8, pitch: u8, vel: u8) -> Kind<'static> {
    Kind::Midi {
        channel: ch.into(),
        message: if vel == 0 {
            Msg::NoteOff {
                key: pitch.into(),
                vel: 0.into(),
            }
        } else {
            Msg::NoteOn {
                key: pitch.into(),
                vel: vel.into(),
            }
        },
    }
}
pub fn export(project: &SequenceProject, path: &Path) -> Result<()> {
    project.validate()?;
    let end = project.total_steps() * 120;
    let tempo = project
        .frames_per_step_override
        .map_or(60_000_000 / u32::from(project.bpm), |f| {
            (u64::from(f) * 4 * 70_224 * 1_000_000 / 4_194_304) as u32
        });
    let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(480.into())));
    smf.tracks.push(vec![
        event(0, Kind::Meta(Meta::TrackName(project.title.as_bytes()))),
        event(0, Kind::Meta(Meta::Tempo(tempo.into()))),
        event(0, Kind::Meta(Meta::TimeSignature(4, 2, 24, 8))),
        event(end, Kind::Meta(Meta::EndOfTrack)),
    ]);
    let count = project.link16_mode.track_count();
    let names: Vec<_> = (0..count)
        .map(|i| format!("GBSEQ P{} CH{}", i / 4 + 1, i % 4 + 1))
        .collect();
    for (i, name) in names.iter().enumerate() {
        let mut events = Vec::new();
        for n in &project.tracks[i].notes {
            if n.velocity == 0 {
                continue;
            }
            events.push((n.start, true, n.pitch, velocity(n.velocity)));
            events.push((n.end(), false, n.pitch, 0));
        }
        events.sort_unstable(); // releases precede retriggers at a shared boundary
        let mut t = vec![event(0, Kind::Meta(Meta::TrackName(name.as_bytes())))];
        let mut last = 0;
        for (step, _, pitch, v) in events {
            let at = step * 120;
            t.push(event(at - last, note(channel(i), BASE + pitch, v)));
            last = at;
        }
        t.push(event(end - last, Kind::Meta(Meta::EndOfTrack)));
        smf.tracks.push(t);
    }
    smf.save(path)
        .with_context(|| format!("write MIDI {}", path.display()))
}

pub fn import(path: &Path) -> Result<(SequenceProject, String)> {
    ensure!(
        std::fs::metadata(path)?.len() <= 8 * 1024 * 1024,
        "MIDI exceeds 8 MiB"
    );
    decode(
        &std::fs::read(path)?,
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("MIDI"),
    )
}
fn tagged(name: &[u8]) -> Option<usize> {
    (0..16).find(|&i| name == format!("GBSEQ P{} CH{}", i / 4 + 1, i % 4 + 1).as_bytes())
}
fn decode(bytes: &[u8], name: &str) -> Result<(SequenceProject, String)> {
    let smf = Smf::parse(bytes).context("invalid Standard MIDI File")?;
    ensure!(
        smf.header.format != Format::Sequential,
        "independent format-2 MIDI sequences are not supported"
    );
    let Timing::Metrical(q) = smf.header.timing else {
        bail!("SMPTE MIDI timing is not supported")
    };
    let q = u64::from(q.as_int());
    ensure!(q > 0, "invalid MIDI division");
    let step = |t: u64| (t * 4 + q / 2) / q;
    let mut project = SequenceProject {
        title: name.into(),
        bpm: 120,
        loop_playback: false,
        ..SequenceProject::empty()
    };
    let mut notes = Vec::new();
    let mut max_end = 1;
    let mut ports = [false; 16];
    let mut tempos = Vec::new();
    let mut non_four = false;
    for (ti, t) in smf.tracks.iter().enumerate() {
        let mut active: BTreeMap<(u8, u8), VecDeque<(u64, u8)>> = BTreeMap::new();
        let mut time = 0u64;
        let tag = t.iter().find_map(|e| {
            if let Kind::Meta(Meta::TrackName(n)) = e.kind {
                tagged(n)
            } else {
                None
            }
        });
        if let Some(l) = tag {
            ports[l] = true
        }
        for e in t {
            time += u64::from(e.delta.as_int());
            ensure!(step(time) <= 4096, "MIDI sequence exceeds 256 bars");
            match e.kind {
                Kind::Midi {
                    channel: ch,
                    message: Msg::NoteOn { key, vel },
                } if vel.as_int() != 0 => {
                    active
                        .entry((ch.as_int(), key.as_int()))
                        .or_default()
                        .push_back((time, vel.as_int()));
                }
                Kind::Midi {
                    channel: ch,
                    message: Msg::NoteOff { key, .. } | Msg::NoteOn { key, .. },
                } => {
                    let pair = (ch.as_int(), key.as_int());
                    if let Some((on, v)) = active.get_mut(&pair).and_then(VecDeque::pop_front) {
                        notes.push((
                            step(on),
                            step(time).max(step(on) + 1),
                            ti,
                            pair.0,
                            pair.1,
                            v,
                            tag,
                        ));
                    }
                }
                Kind::Meta(Meta::Tempo(us)) => {
                    ensure!(us.as_int() > 0, "zero MIDI tempo");
                    tempos.push((time, us.as_int()));
                }
                Kind::Meta(Meta::TrackName(s)) if ti == 0 && tag.is_none() => {
                    if !s.is_empty() {
                        project.title = String::from_utf8_lossy(s).into_owned();
                    }
                }
                Kind::Meta(Meta::TimeSignature(n, d, _, _)) => non_four |= (n, d) != (4, 2),
                _ => {}
            }
        }
        for ((ch, key), queue) in active {
            for (on, v) in queue {
                notes.push((step(on), step(time).max(step(on) + 1), ti, ch, key, v, tag));
            }
        }
        max_end = max_end.max(step(time));
    }
    tempos.sort_by_key(|x| x.0);
    let initial = tempos
        .iter()
        .take_while(|t| t.0 == 0)
        .last()
        .map_or(500000, |t| t.1);
    let bpm = (60_000_000 + initial / 2) / initial;
    project.bpm = bpm.clamp(30, 300) as u16;
    notes.sort_unstable();
    let mut end = [0; 16];
    let mut owners = [None; 16];
    let mut folded = 0;
    project.set_link16_mode(Link16Mode::Dmg07_16);
    let mut highest = ports.iter().rposition(|&b| b).unwrap_or(0);
    for (on, off, ti, ch, mut key, v, tag) in notes {
        ensure!(off <= 4096, "quantized note exceeds 256 bars");
        let owner = (ti, ch);
        let lane = if let Some(l) = tag {
            ensure!(
                end[l] <= on,
                "overlapping notes on tagged GBSEQ lane {}",
                l + 1
            );
            l
        } else {
            (0..16).filter(|&i|!ports[i] && end[i]<=on && (i%4==3)==(ch==9))
                .min_by_key(|&i|(if owners[i]==Some(owner){0}else if owners[i].is_none(){1}else{2},i))
                .context("too many simultaneous voices after sixteenth-note quantization (12 melodic / 4 noise maximum)")?
        };
        owners[lane] = Some(owner);
        end[lane] = off;
        highest = highest.max(lane);
        max_end = max_end.max(off);
        if !(BASE..BASE + 48).contains(&key) {
            folded += 1;
            while key < BASE {
                key += 12
            }
            while key >= BASE + 48 {
                key -= 12
            }
        }
        let track = &mut project.tracks[lane];
        track.notes.push(MidiNote {
            id: project.next_note_id,
            start: on as u32,
            length: (off - on) as u32,
            pitch: key - BASE,
            velocity: level(v),
            effects: NoteEffects::for_track(track.kind),
        });
        project.next_note_id += 1;
    }
    project.measures = max_end.div_ceil(16) as u32;
    project.set_link16_mode(if highest < 4 {
        Link16Mode::Local4
    } else if highest < 8 {
        Link16Mode::Cable8
    } else {
        Link16Mode::Dmg07_16
    });
    project.tracks.truncate(project.link16_mode.track_count());
    project.validate()?;
    Ok((project,format!("MIDI: 1/16 grid; octave-folded {folded}; tempo changes ignored: {}; tempo clamped: {}; non-4/4 mapped: {}",tempos.iter().any(|t|t.0>0 && t.1!=initial),bpm.clamp(30,300)!=bpm,non_four)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn type_zero_running_status_chord_drums_and_velocity_zero() {
        let data = [
            0, 0x90, 60, 100, 0, 64, 64, 0, 0x99, 36, 90, 96, 0x90, 60, 0, 0, 64, 0, 0, 0x89, 36,
            0, 0, 0xff, 0x2f, 0,
        ];
        let mut b = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk".to_vec();
        b.extend((data.len() as u32).to_be_bytes());
        b.extend(data);
        let (p, _) = decode(&b, "Chord").unwrap();
        assert_eq!(p.bpm, 120);
        assert_eq!(p.tracks[0].notes[0].pitch, 24);
        assert_eq!(p.tracks[1].notes[0].pitch, 28);
        assert_eq!(p.tracks[3].notes[0].length, 4);
        b[12] = 0xe7;
        assert!(decode(&b, "SMPTE").is_err());
    }
    #[test]
    fn all_16_lanes_roundtrip_even_with_empty_lanes_and_trailing_rests() {
        let mut p = SequenceProject::empty();
        p.set_link16_mode(Link16Mode::Dmg07_16);
        p.measures = 256;
        p.bpm = 139;
        for i in [0, 3, 5, 9, 11, 15] {
            p.insert_note_exact(i, 313, 3, 16 + i as u8, 1 + i as u8 % 15)
                .unwrap();
        }
        let f = std::env::temp_dir().join(format!("gbseq-midi-round-{}.mid", std::process::id()));
        export(&p, &f).unwrap();
        let (q, _) = import(&f).unwrap();
        std::fs::remove_file(f).unwrap();
        assert_eq!(
            (q.measures, q.bpm, q.link16_mode),
            (256, 139, Link16Mode::Dmg07_16)
        );
        for (a, b) in p.tracks.iter().zip(&q.tracks) {
            assert_eq!(
                a.notes
                    .iter()
                    .map(|n| (n.start, n.length, n.pitch, n.velocity))
                    .collect::<Vec<_>>(),
                b.notes
                    .iter()
                    .map(|n| (n.start, n.length, n.pitch, n.velocity))
                    .collect::<Vec<_>>()
            );
        }
    }
}
