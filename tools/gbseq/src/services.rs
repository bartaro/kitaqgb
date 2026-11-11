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
    prepare_playback_projects(project, player, player.is_none())
}

fn prepare_playback_projects(
    project: &SequenceProject,
    player: Option<usize>,
    global_solo: bool,
) -> Result<Vec<gbdaw_model::Project>> {
    project.validate()?;
    if let Some(player) = player {
        ensure!(
            player < project.player_count(),
            "player is outside the active link mode"
        );
    }
    let targets = player.map_or(0..project.player_count(), |p| p..p + 1);
    let solo_tracks = if global_solo {
        0..project.link16_mode.track_count()
    } else {
        targets.start * crate::model::TRACK_COUNT..targets.end * crate::model::TRACK_COUNT
    };
    let solo = project.tracks[solo_tracks].iter().any(|t| t.solo);
    let mut source = project.clone();
    // The stream renderer consumes note events, not the GUI mixer metadata.
    // Apply mutes only to this snapshot; retain the transport and song length.
    for track in &mut source.tracks[..project.link16_mode.track_count()] {
        if track.mute || (solo && !track.solo) {
            track.notes.clear();
        }
    }
    targets.map(|p| source.to_gbdaw_player(p)).collect()
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
    let projects = playback_projects(project, player)?;
    write_projects_wav(&projects, path, project_frame_count(project))
}

/// A monitor isolates a console *after* the global track mute/solo rules.
/// A stream may end early even with the project loop flag. A solo WAV must
/// use the mix duration regardless of the selected console's last event.
#[cfg(feature = "gui")]
pub(crate) fn render_monitor_wav(
    project: &SequenceProject,
    path: &Path,
    player: Option<usize>,
) -> Result<()> {
    if let Some(p) = player {
        let projects = prepare_playback_projects(project, None, true)?;
        ensure!(p < projects.len(), "invalid monitor player");
        let mut stems = render_player_stems(&projects, project_frame_count(project), true, None)?;
        let len = stems.iter().map(|(_, s)| s.len()).max().unwrap_or(0);
        let (rate, mut solo) = stems.swap_remove(p);
        solo.resize(len, 0);
        return write_samples_wav(path, rate, &solo);
    }
    let projects = prepare_playback_projects(project, player, true)?;
    write_projects_wav(&projects, path, project_frame_count(project))
}

fn write_projects_wav(projects: &[gbdaw_model::Project], path: &Path, frames: u32) -> Result<()> {
    let (rate, samples) = render_mixed_projects(projects, frames)?;
    write_samples_wav(path, rate, &samples)
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
