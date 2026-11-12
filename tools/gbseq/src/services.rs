//! File/export/audio services. No display server or audio device required.
use crate::model::{SequenceProject, STEPS_PER_MEASURE};
use anyhow::{ensure, Context, Result};
use gbdaw_export_kitaqgb::{export_project_to_dir, ExportPolicy, StreamMode};
use gbdaw_kokura::{render_project_to_samples, write_wav_i16_stereo};
use std::{fs, path::Path, thread};

pub fn project_frame_count(project: &SequenceProject) -> u32 {
    let frames = project
        .measures
        .saturating_mul(STEPS_PER_MEASURE)
        .saturating_mul(project.frames_per_step())
        .max(1);
    if project.loop_playback {
        frames
    } else {
        frames.saturating_add(60)
    }
}

pub fn load_sequence(path: &Path) -> Result<SequenceProject> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut project: SequenceProject =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    project.repair();
    project.validate()?;
    Ok(project)
}

pub fn save_sequence(project: &SequenceProject, path: &Path) -> Result<()> {
    project.validate()?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(project)?)
        .with_context(|| format!("write {}", path.display()))
}

pub fn export_sequence(project: &SequenceProject, out_dir: &Path) -> Result<usize> {
    export_player(project, out_dir, 0)
}

pub fn export_player(project: &SequenceProject, out_dir: &Path, player: usize) -> Result<usize> {
    let legacy = project.to_gbdaw_player(player)?;
    let manifest = export_project_to_dir(
        &legacy,
        out_dir,
        StreamMode::Extended,
        ExportPolicy::default(),
    )?;
    Ok(manifest.songs.first().map(|song| song.bytes).unwrap_or(0))
}

pub fn playback_projects(
    project: &SequenceProject,
    player: Option<usize>,
) -> Result<Vec<gbdaw_model::Project>> {
    project.validate()?;
    if let Some(player) = player {
        ensure!(
            player < project.player_count(),
            "player is outside the active link mode"
        );
    }
    let targets = player.map_or(0..project.player_count(), |p| p..p + 1);
    let mask = audible_mask(project);
    let mut source = project.clone();
    // The stream renderer consumes note events, not the GUI mixer metadata.
    // Apply mutes only to this snapshot; retain the transport and song length.
    for (i, track) in source
        .tracks
        .iter_mut()
        .take(project.link16_mode.track_count())
        .enumerate()
    {
        if mask & (1 << i) == 0 {
            track.notes.clear();
        }
    }
    targets.map(|p| source.to_gbdaw_player(p)).collect()
}

/// Mixer state is global among consoles in the active link mode. The lane
/// selected for editing is deliberately not part of the audio routing.
pub(crate) fn audible_mask(project: &SequenceProject) -> usize {
    let tracks = &project.tracks[..project.link16_mode.track_count()];
    let solo = tracks.iter().any(|t| t.solo);
    tracks.iter().enumerate().fold(0, |bits, (i, t)| {
        bits | (usize::from(!t.mute && (!solo || t.solo)) << i)
    })
}

pub fn render_mixed_projects(
    projects: &[gbdaw_model::Project],
    frames: u32,
) -> Result<(u32, Vec<i16>)> {
    render_mixed_with_strategy(projects, frames, true)
}

pub fn render_mixed_with_strategy(
    projects: &[gbdaw_model::Project],
    frames: u32,
    parallel: bool,
) -> Result<(u32, Vec<i16>)> {
    render_mixed_cancellable(projects, frames, parallel, None)
}

pub fn render_mixed_cancellable(
    projects: &[gbdaw_model::Project],
    frames: u32,
    parallel: bool,
    cancelled: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(u32, Vec<i16>)> {
    let rendered = render_player_stems(projects, frames, parallel, cancelled)?;
    let rate = rendered[0].0;
    if rendered.len() == 1 {
        return Ok(rendered.into_iter().next().unwrap());
    }
    let divisor = rendered.len() as i32;
    let len = rendered.iter().map(|(_, s)| s.len()).max().unwrap_or(0);
    let mut mix = vec![0i32; len];
    for (_, samples) in rendered {
        for (dest, sample) in mix.iter_mut().zip(samples) {
            *dest += i32::from(sample);
        }
    }
    Ok((
        rate,
        mix.into_iter().map(|s| (s / divisor) as i16).collect(),
    ))
}

/// Keep the console outputs separate so the GUI can change the monitor
/// without restarting KOKURA or moving the transport.
pub(crate) fn render_player_stems(
    projects: &[gbdaw_model::Project],
    frames: u32,
    parallel: bool,
    cancelled: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<(u32, Vec<i16>)>> {
    ensure!(
        !projects.is_empty() && projects.len() <= 4,
        "render requires 1..4 players"
    );
    let render = |project: &gbdaw_model::Project| {
        if let Some(cancelled) = cancelled {
            let mut renderer =
                gbdaw_kokura::ProjectAudioRenderer::new(project, StreamMode::Extended)?;
            let mut samples = Vec::new();
            let mut remaining = frames;
            while remaining > 0 && !renderer.is_finished() {
                ensure!(
                    !cancelled.load(std::sync::atomic::Ordering::Relaxed),
                    "render cancelled"
                );
                let count = remaining.min(16);
                samples.extend(renderer.render_next_frames(count)?);
                remaining -= count;
            }
            return Ok((gbdaw_kokura::default_output_sample_rate(), samples));
        }
        render_project_to_samples(project, StreamMode::Extended, frames)
            .map(|(rate, samples, _)| (rate, samples))
    };
    let rendered: Vec<(u32, Vec<i16>)> = if parallel && projects.len() > 1 {
        thread::scope(|scope| -> Result<_> {
            let workers: Vec<_> = projects
                .iter()
                .map(|p| scope.spawn(move || render(p)))
                .collect();
            workers
                .into_iter()
                .map(|w| {
                    w.join()
                        .map_err(|_| anyhow::anyhow!("audio worker panicked"))?
                })
                .collect()
        })?
    } else {
        projects.iter().map(render).collect::<Result<_>>()?
    };
    ensure!(
        rendered.iter().all(|(rate, _)| *rate == rendered[0].0),
        "player sample rates differ"
    );
    Ok(rendered)
}

pub fn render_sequence_wav(project: &SequenceProject, path: &Path) -> Result<()> {
    render_wav(project, path, None)
}

pub fn render_wav(project: &SequenceProject, path: &Path, player: Option<usize>) -> Result<()> {
    project.validate()?;
    let n = project.player_count();
    ensure!(
        player.is_none_or(|i| i < n),
        "player is outside the active link mode"
    );
    let mask = audible_mask(project);
    let limit = project_frame_count(project);
    let mut source = project.clone();
    for t in &mut source.tracks {
        t.mute = false;
        t.solo = false;
    }
    let mut end = 0;
    let rate = gbdaw_kokura::default_output_sample_rate();
    let mut players = Vec::new();
    for p in 0..n {
        let mut r = gbdaw_kokura::ProjectAudioRenderer::new(
            &source.to_gbdaw_player(p)?,
            StreamMode::Extended,
        )?;
        end = end.max(r.end_frame().unwrap_or(limit).min(limit));
        let ch = ((mask >> (4 * p)) & 15) as u8;
        if ch != 0 && player.is_none_or(|i| i == p) {
            r.set_monitor_mask(ch);
            players.push(r);
        }
    }
    // Use the complete score for duration, even for an isolated or silent
    // console. Mask the APU outputs, never the timed note events themselves.
    let samples_until = |f: u32| (u64::from(f) * 70_224 * u64::from(rate) / 4_194_304) as usize * 2;
    let mut pcm = Vec::with_capacity(samples_until(end));
    let divisor = if player.is_some() { 1 } else { n as i32 };
    for f in (0..end).step_by(32) {
        let count = 32.min(end - f);
        let mut mix = vec![0i32; samples_until(f + count) - samples_until(f)];
        for r in &mut players {
            for (dst, s) in mix.iter_mut().zip(r.render_next_frames(count)?) {
                *dst += i32::from(s);
            }
        }
        pcm.extend(mix.into_iter().map(|s| (s / divisor) as i16));
    }
    write_samples_wav(path, rate, &pcm)
}

fn write_samples_wav(path: &Path, rate: u32, samples: &[i16]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    write_wav_i16_stereo(path, rate, samples)?;
    Ok(())
}

pub fn inspect_sequence(project: &SequenceProject) -> Result<String> {
    project.validate()?;
    let legacy = project.to_gbdaw_project()?;
    Ok(format!(
        "title={} mode={} loop={} bpm={} frame_step={} measures={} steps={} tracks={} notes={} gbdaw_patterns={} ticks_per_row={} frames_per_tick={}",
        project.title,
        project.link16_mode.label(),
        project.loop_playback,
        project.bpm,
        project.frames_per_step(),
        project.measures,
        project.total_steps(),
        project.link16_mode.track_count(),
        project.tracks.iter().map(|t| t.notes.len()).sum::<usize>(),
        legacy.song.patterns.len(), legacy.song.ticks_per_row, legacy.song.frames_per_tick
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_mixer_preserves_timeline_and_only_routes_audible_lanes() {
        for mode in [crate::Link16Mode::Cable8, crate::Link16Mode::Dmg07_16] {
            let mut p = SequenceProject::empty();
            p.measures = 1;
            p.loop_playback = false;
            p.set_link16_mode(mode);
            let peer = (p.player_count() - 1) * 4;
            p.insert_note_exact(0, 0, 8, 20, 12).unwrap();
            p.insert_note_exact(peer, 0, 5, 31, 14).unwrap();
            p.insert_note_exact(peer + 1, 13, 3, 40, 10).unwrap();
            let path = std::env::temp_dir().join(format!(
                "gbseq-active-wav-{}-{}.wav",
                std::process::id(),
                peer
            ));
            let wav = |p: &SequenceProject, player| {
                render_wav(p, &path, player).unwrap();
                let bytes = fs::read(&path).unwrap();
                bytes[44..]
                    .chunks_exact(2)
                    .map(|v| i16::from_le_bytes(v.try_into().unwrap()))
                    .collect::<Vec<_>>()
            };
            let full = wav(&p, None);
            p.tracks[peer].solo = true;
            let isolated = wav(&p, Some(peer / 4));
            assert!(isolated.iter().any(|s| *s != 0));
            let solo = wav(&p, None);
            assert_ne!(full, solo);
            assert_eq!(full.len(), isolated.len());
            assert!(solo
                .iter()
                .zip(&isolated)
                .all(|(&mix, &one)| mix == one / p.player_count() as i16));
            let expected_muted = wav(&p, Some(0));
            assert_eq!(expected_muted, vec![0; full.len()]);
            p.tracks[peer].solo = false;
            for (i, t) in p.tracks.iter_mut().enumerate() {
                t.mute = i != peer;
            }
            assert_eq!(wav(&p, None), solo);
            p.tracks[peer].solo = true;
            p.tracks[peer].mute = true;
            assert_eq!(wav(&p, None), expected_muted);
            p.set_link16_mode(crate::Link16Mode::Local4);
            p.tracks[0].mute = false;
            assert_eq!(audible_mask(&p), 1);
            assert!(wav(&p, None).iter().any(|&s| s != 0));
            fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn cancellation_and_chunked_audio_match_the_export_path() {
        let p = SequenceProject::demo();
        let projects = playback_projects(&p, Some(0)).unwrap();
        let cancelled = std::sync::atomic::AtomicBool::new(true);
        assert!(render_mixed_cancellable(&projects, 32, true, Some(&cancelled)).is_err());
        cancelled.store(false, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            render_mixed_cancellable(&projects, 32, true, Some(&cancelled)).unwrap(),
            render_mixed_projects(&projects, 32).unwrap()
        );
    }
    #[test]
    fn parallel_mix_matches_serial_pcm() {
        let mut p = SequenceProject::demo();
        p.set_link16_mode(crate::Link16Mode::Dmg07_16);
        p.insert_note_exact(4, 0, 8, 28, 12).unwrap();
        let projects = playback_projects(&p, None).unwrap();
        let serial = render_mixed_with_strategy(&projects, 20, false).unwrap();
        let parallel = render_mixed_with_strategy(&projects, 20, true).unwrap();
        assert_eq!(serial, parallel);
        assert!(serial.1.iter().any(|s| *s != 0));
    }
}
