//! Atomic editing operations shared by the GUI and headless CLI.
use crate::model::SequenceProject;
use anyhow::{bail, ensure, Result};

#[derive(Clone, Copy, Debug)]
pub enum NoteEdit {
    Move(i32),
    Transpose(i16),
    Velocity(u8),
    Quantize(u32),
    Delete,
    Duplicate,
}

/// Replace only the occupied portion of the destination lane. A surviving
/// right fragment needs a fresh ID when both ends of the old note survive.
pub fn input_note(
    p: &mut SequenceProject,
    lane: usize,
    start: u32,
    len: u32,
    pitch: u8,
    velocity: u8,
    overwrite: bool,
) -> Result<u64> {
    ensure!(
        lane < p.link16_mode.track_count()
            && len > 0
            && start.checked_add(len).is_some_and(|e| e <= p.total_steps()),
        "note lies outside song"
    );
    if overwrite {
        let end = start + len;
        let track = &mut p.tracks[lane];
        let mut tail = None;
        track.notes.retain_mut(|n| {
            let old_end = n.end();
            if old_end <= start || n.start >= end {
                return true;
            }
            if n.start < start {
                if old_end > end {
                    let mut split = n.clone();
                    split.id = p.next_note_id;
                    p.next_note_id += 1;
                    split.start = end;
                    split.length = old_end - end;
                    tail = Some(split);
                }
                n.length = start - n.start;
                true
            } else if old_end > end {
                n.start = end;
                n.length = old_end - end;
                true
            } else {
                false
            }
        });
        track.notes.extend(tail);
    }
    p.insert_note_exact(lane, start, len, pitch, velocity)
}

/// Transpose active pitched voices atomically; noise lanes and hidden players stay intact.
pub fn song_key(project: &mut SequenceProject, delta: i16) -> Result<()> {
    let count = project.link16_mode.track_count();
    for (_, t) in project.tracks[..count]
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 4 != 3)
    {
        ensure!(
            t.notes
                .iter()
                .all(|n| (0..=47).contains(&(i32::from(n.pitch) + i32::from(delta)))),
            "transpose exceeds C2..B5; song unchanged"
        );
    }
    for (_, t) in project.tracks[..count]
        .iter_mut()
        .enumerate()
        .filter(|(i, _)| i % 4 != 3)
    {
        for n in &mut t.notes {
            n.pitch = (i32::from(n.pitch) + i32::from(delta)) as u8;
        }
    }
    Ok(())
}

pub fn edit_notes(project: &mut SequenceProject, ids: &[u64], edit: NoteEdit) -> Result<Vec<u64>> {
    ensure!(!ids.is_empty(), "select at least one note");
    let ids: std::collections::BTreeSet<_> = ids.iter().copied().collect();
    for id in &ids {
        ensure!(project.note(*id).is_some(), "note {id} does not exist");
    }
    let mut candidate = project.clone();
    let mut result: Vec<u64> = ids.iter().copied().collect();
    if matches!(edit, NoteEdit::Duplicate) {
        let start = ids
            .iter()
            .map(|id| project.note(*id).unwrap().start)
            .min()
            .unwrap();
        let end = ids
            .iter()
            .map(|id| project.note(*id).unwrap().end())
            .max()
            .unwrap();
        result.clear();
        for (track, source) in project.tracks.iter().enumerate() {
            for note in source.notes.iter().filter(|note| ids.contains(&note.id)) {
                let new_start = note
                    .start
                    .checked_add(end - start)
                    .ok_or_else(|| anyhow::anyhow!("note position overflow"))?;
                let id = candidate.insert_note_exact(
                    track,
                    new_start,
                    note.length,
                    note.pitch,
                    note.velocity,
                )?;
                candidate.note_mut(id).unwrap().effects = note.effects.clone();
                result.push(id);
            }
        }
    } else {
        for track in &mut candidate.tracks {
            if matches!(edit, NoteEdit::Delete) {
                track.notes.retain(|note| !ids.contains(&note.id));
                continue;
            }
            for note in track.notes.iter_mut().filter(|note| ids.contains(&note.id)) {
                match edit {
                    NoteEdit::Move(delta) => {
                        let start = i64::from(note.start) + i64::from(delta);
                        ensure!(
                            (0..=u32::MAX as i64).contains(&start),
                            "move exceeds song bounds"
                        );
                        note.start = start as u32;
                    }
                    NoteEdit::Transpose(delta) => {
                        let pitch = i32::from(note.pitch) + i32::from(delta);
                        ensure!(
                            (0..=47).contains(&pitch),
                            "transpose exceeds pitch range 0..47"
                        );
                        note.pitch = pitch as u8;
                    }
                    NoteEdit::Velocity(value) => {
                        ensure!(value <= 15, "velocity must be 0..15");
                        note.velocity = value;
                    }
                    NoteEdit::Quantize(grid) => {
                        ensure!(
                            grid > 0 && grid <= project.total_steps(),
                            "invalid quantize grid"
                        );
                        let nearest =
                            (u64::from(note.start) + u64::from(grid) / 2) / u64::from(grid);
                        let last_line = project.total_steps().saturating_sub(note.length) / grid;
                        note.start = nearest.min(u64::from(last_line)) as u32 * grid;
                    }
                    _ => bail!("unsupported edit"),
                }
            }
            track.notes.sort_by_key(|n| (n.start, n.pitch, n.id));
        }
        if matches!(edit, NoteEdit::Delete) {
            result.clear();
        }
    }
    // Validate before commit: overlapping monophonic notes must never be repaired away.
    candidate.validate()?;
    *project = candidate;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_move_is_atomic_and_preserves_effects() {
        let mut p = SequenceProject::empty();
        let a = p.insert_note_exact(0, 0, 2, 24, 12).unwrap();
        let b = p.insert_note_exact(0, 2, 2, 26, 13).unwrap();
        edit_notes(&mut p, &[a, b], NoteEdit::Move(2)).unwrap();
        assert_eq!(p.note(a).unwrap().start, 2);
        let before = p.clone();
        assert!(edit_notes(&mut p, &[a], NoteEdit::Move(2)).is_err());
        assert_eq!(before, p);
        edit_notes(&mut p, &[a, b], NoteEdit::Transpose(12)).unwrap();
        assert_eq!(p.note(b).unwrap().pitch, 38);
    }
    #[test]
    fn duplicate_preserves_spacing_and_rejects_collisions() {
        let mut p = SequenceProject::empty();
        let a = p.insert_note_exact(0, 0, 2, 24, 12).unwrap();
        let b = p.insert_note_exact(1, 4, 2, 12, 8).unwrap();
        let copied = edit_notes(&mut p, &[a, b], NoteEdit::Duplicate).unwrap();
        assert_eq!(p.note(copied[0]).unwrap().start, 6);
        assert_eq!(p.note(copied[1]).unwrap().start, 10);
        let before = p.clone();
        assert!(edit_notes(&mut p, &[a, b], NoteEdit::Duplicate).is_err());
        assert_eq!(p, before);
    }
    #[test]
    fn quantize_keeps_gate_near_song_end_and_collisions_are_atomic() {
        let mut p = SequenceProject::empty();
        p.measures = 1;
        let a = p.insert_note_exact(0, 3, 2, 20, 8).unwrap();
        let b = p.insert_note_exact(1, 15, 1, 22, 9).unwrap();
        edit_notes(&mut p, &[a, b], NoteEdit::Quantize(4)).unwrap();
        assert_eq!(
            (p.note(a).unwrap().start, p.note(a).unwrap().length),
            (4, 2)
        );
        assert_eq!(p.note(b).unwrap().start, 12);
        let c = p.insert_note_exact(0, 6, 1, 24, 10).unwrap();
        let snapshot = p.clone();
        assert!(edit_notes(&mut p, &[a, c, b], NoteEdit::Quantize(8)).is_err());
        assert_eq!(p, snapshot);
    }
    #[test]
    fn invalid_quantize_and_unknown_ids_leave_project_untouched() {
        let mut p = SequenceProject::demo();
        let before = p.clone();
        assert!(edit_notes(&mut p, &[1], NoteEdit::Quantize(0)).is_err());
        assert!(edit_notes(&mut p, &[u64::MAX], NoteEdit::Delete).is_err());
        assert_eq!(before, p);
    }
}
