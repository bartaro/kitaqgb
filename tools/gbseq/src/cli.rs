use crate::{
    editing::{edit_notes, NoteEdit},
    model::{Link16Mode, SequenceProject, MAX_MEASURES},
    services::*,
};
use anyhow::{ensure, Result};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::{path::PathBuf, time::Instant};

#[derive(Parser)]
#[command(
    version,
    about = "GBSEQ headless sequencer: edit, validate, render and export"
)]
pub struct HeadlessCli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args)]
pub struct Output {
    /// Destination project (the input is never overwritten implicitly).
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
pub struct Selection {
    /// Note IDs, comma separated. Omit to select all notes in the chosen track(s).
    #[arg(long, value_delimiter = ',')]
    ids: Vec<u64>,
    /// One-based physical track number (1..16).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=16))]
    track: Option<u8>,
}

#[derive(Args)]
pub struct Players {
    /// One-based player number.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=4))]
    player: u8,
    /// Mix all active players, honoring global solo/mute.
    #[arg(long, conflicts_with = "player")]
    all_players: bool,
}
impl Players {
    fn selected(&self) -> Option<usize> {
        if self.all_players {
            None
        } else {
            Some(self.player as usize - 1)
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    Demo {
        out_path: PathBuf,
    },
    New {
        out_path: PathBuf,
        #[arg(long, default_value = "Untitled Sequence")]
        title: String,
        #[arg(long, default_value_t = 16)]
        bars: u32,
        #[arg(long, default_value_t = 150, value_parser = clap::value_parser!(u16).range(30..=300))]
        bpm: u16,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=4))]
        players: u8,
        #[arg(long)]
        no_loop: bool,
    },
    Validate {
        project: PathBuf,
    },
    Info {
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Notes {
        project: PathBuf,
        #[command(flatten)]
        selection: Selection,
    },
    AddNote {
        project: PathBuf,
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=16))]
        track: u8,
        /// Zero-based sixteenth-note step.
        #[arg(long)]
        start: u32,
        #[arg(long, default_value_t = 2)]
        length: u32,
        #[arg(long, default_value_t = 24, value_parser = clap::value_parser!(u8).range(0..=47))]
        pitch: u8,
        #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u8).range(0..=15))]
        velocity: u8,
        #[command(flatten)]
        output: Output,
    },
    Transform {
        project: PathBuf,
        #[command(flatten)]
        selection: Selection,
        #[arg(long, allow_hyphen_values = true)]
        transpose: Option<i16>,
        #[arg(long, allow_hyphen_values = true)]
        shift: Option<i32>,
        #[arg(long)]
        quantize: Option<u32>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=15))]
        velocity: Option<u8>,
        #[command(flatten)]
        output: Output,
    },
    DeleteNotes {
        project: PathBuf,
        #[command(flatten)]
        selection: Selection,
        #[command(flatten)]
        output: Output,
    },
    DuplicateNotes {
        project: PathBuf,
        #[command(flatten)]
        selection: Selection,
        #[command(flatten)]
        output: Output,
    },
    ExportKitaqgb {
        project: PathBuf,
        out_dir: PathBuf,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=4))]
        player: u8,
    },
    RenderWav {
        project: PathBuf,
        out_path: PathBuf,
        #[command(flatten)]
        players: Players,
    },
    ExportLink16Sav {
        project: PathBuf,
        out_dir: PathBuf,
    },
    BenchRender {
        project: PathBuf,
        #[command(flatten)]
        players: Players,
        #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=20))]
        runs: u8,
        /// Limit the benchmark to this many GB frames.
        #[arg(long)]
        frames: Option<u32>,
        /// Reference serial mixing instead of independent player workers.
        #[arg(long)]
        serial: bool,
    },
}

fn selected_ids(p: &SequenceProject, selection: &Selection) -> Result<Vec<u64>> {
    if let Some(track) = selection.track {
        ensure!(
            track as usize <= p.link16_mode.track_count(),
            "track is outside the active link mode"
        );
    }
    if !selection.ids.is_empty() {
        for id in &selection.ids {
            let (track, _) = p
                .note_location(*id)
                .ok_or_else(|| anyhow::anyhow!("note {id} does not exist"))?;
            ensure!(
                selection.track.is_none_or(|t| track + 1 == t as usize),
                "note {id} is not on the selected track"
            );
        }
        return Ok(selection.ids.clone());
    }
    Ok(p.tracks
        .iter()
        .take(p.link16_mode.track_count())
        .enumerate()
        .filter(|(i, _)| selection.track.is_none_or(|t| *i + 1 == t as usize))
        .flat_map(|(_, t)| t.notes.iter().map(|n| n.id))
        .collect())
}

impl HeadlessCli {
    pub fn run(self) -> Result<()> {
        self.command.run()
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    #[test]
    fn all_players_and_signed_transforms_parse() {
        assert!(HeadlessCli::try_parse_from([
            "gbseq-cli",
            "render-wav",
            "input.gbseq",
            "output.wav",
            "--all-players"
        ])
        .is_ok());
        assert!(HeadlessCli::try_parse_from([
            "gbseq-cli",
            "transform",
            "input.gbseq",
            "--transpose",
            "-12",
            "--out",
            "output.gbseq"
        ])
        .is_ok());
    }
}
impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Demo { out_path } => {
                save_sequence(&SequenceProject::demo(), &out_path)?;
                println!("saved={}", out_path.display());
            }
            Self::New {
                out_path,
                title,
                bars,
                bpm,
                players,
                no_loop,
            } => {
                ensure!(
                    (1..=MAX_MEASURES).contains(&bars),
                    "bars must be 1..{MAX_MEASURES}"
                );
                ensure!(players != 3, "supported player counts: 1, 2, 4");
                let mut p = SequenceProject {
                    title,
                    measures: bars,
                    bpm,
                    loop_playback: !no_loop,
                    ..SequenceProject::empty()
                };
                p.set_link16_mode(match players {
                    1 => Link16Mode::Local4,
                    2 => Link16Mode::Cable8,
                    _ => Link16Mode::Dmg07_16,
                });
                save_sequence(&p, &out_path)?;
                println!("saved={}", out_path.display());
            }
            Self::Validate { project } => {
                println!("OK {}", inspect_sequence(&load_sequence(&project)?)?)
            }
            Self::Info {
                project,
                json: as_json,
            } => {
                let p = load_sequence(&project)?;
                if as_json {
                    println!("{}", serde_json::to_string_pretty(&p)?);
                } else {
                    println!("{}", inspect_sequence(&p)?);
                }
            }
            Self::Notes { project, selection } => {
                let p = load_sequence(&project)?;
                let rows: Vec<_> = selected_ids(&p, &selection)?
                    .iter()
                    .map(|id| {
                        let (track, _) = p.note_location(*id).unwrap();
                        json!({"track":track+1,"note":p.note(*id).unwrap()})
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&rows)?);
            }
            Self::AddNote {
                project,
                track,
                start,
                length,
                pitch,
                velocity,
                output,
            } => {
                let mut p = load_sequence(&project)?;
                ensure!(
                    track as usize <= p.link16_mode.track_count(),
                    "track is outside the active link mode"
                );
                let id = p.insert_note_exact(track as usize - 1, start, length, pitch, velocity)?;
                save_sequence(&p, &output.out)?;
                println!("note_id={id} saved={}", output.out.display());
            }
            Self::Transform {
                project,
                selection,
                transpose,
                shift,
                quantize,
                velocity,
                output,
            } => {
                ensure!(
                    transpose.is_some()
                        || shift.is_some()
                        || quantize.is_some()
                        || velocity.is_some(),
                    "specify a transform"
                );
                let mut p = load_sequence(&project)?;
                let ids = selected_ids(&p, &selection)?;
                for edit in [
                    shift.map(NoteEdit::Move),
                    transpose.map(NoteEdit::Transpose),
                    quantize.map(NoteEdit::Quantize),
                    velocity.map(NoteEdit::Velocity),
                ]
                .into_iter()
                .flatten()
                {
                    edit_notes(&mut p, &ids, edit)?;
                }
                save_sequence(&p, &output.out)?;
                println!("edited={} saved={}", ids.len(), output.out.display());
            }
            Self::DeleteNotes {
                project,
                selection,
                output,
            } => {
                let mut p = load_sequence(&project)?;
                let ids = selected_ids(&p, &selection)?;
                edit_notes(&mut p, &ids, NoteEdit::Delete)?;
                save_sequence(&p, &output.out)?;
                println!("deleted={} saved={}", ids.len(), output.out.display());
            }
            Self::DuplicateNotes {
                project,
                selection,
                output,
            } => {
                let mut p = load_sequence(&project)?;
                let ids = selected_ids(&p, &selection)?;
                let added = edit_notes(&mut p, &ids, NoteEdit::Duplicate)?;
                save_sequence(&p, &output.out)?;
                println!("{}", serde_json::to_string(&added)?);
            }
            Self::ExportKitaqgb {
                project,
                out_dir,
                player,
            } => {
                let p = load_sequence(&project)?;
                ensure!(
                    player as usize <= p.player_count(),
                    "player is outside the active link mode"
                );
                println!(
                    "exported_bytes={}",
                    export_player(&p, &out_dir, player as usize - 1)?
                );
            }
            Self::RenderWav {
                project,
                out_path,
                players,
            } => {
                let start = Instant::now();
                render_wav(&load_sequence(&project)?, &out_path, players.selected())?;
                println!(
                    "rendered={} elapsed_ms={:.3}",
                    out_path.display(),
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
            Self::ExportLink16Sav { project, out_dir } => {
                let m = crate::export_link16_saves(&load_sequence(&project)?, &out_dir)?;
                println!("{}", serde_json::to_string_pretty(&m)?);
            }
            Self::BenchRender {
                project,
                players,
                runs,
                frames,
                serial,
            } => {
                let p = load_sequence(&project)?;
                let frames = frames.unwrap_or_else(|| project_frame_count(&p));
                ensure!(
                    frames > 0 && frames <= project_frame_count(&p),
                    "frames must be within the song"
                );
                let projects = playback_projects(&p, players.selected())?;
                let mut times = Vec::new();
                let mut checksum = 0;
                let mut count = 0;
                let mut rate = 0;
                for _ in 0..runs {
                    let start = Instant::now();
                    let (hz, samples) = render_mixed_with_strategy(&projects, frames, !serial)?;
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                    let hash = samples
                        .iter()
                        .flat_map(|s| s.to_le_bytes())
                        .fold(0xcbf29ce484222325u64, |h, b| {
                            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
                        });
                    ensure!(count == 0 || checksum == hash, "non-deterministic PCM");
                    checksum = hash;
                    count = samples.len();
                    rate = hz;
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({"elapsed_ms":times,"frames":frames,"players":projects.len(),"parallel":!serial,"sample_rate":rate,"samples":count,"pcm_fnv1a64":format!("{checksum:016x}")})
                    )?
                );
            }
        }
        Ok(())
    }
}
