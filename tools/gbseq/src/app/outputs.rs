//! Never wait for an OS chooser, filesystem or renderer in a plita input callback.
use super::*;
use std::{
    future::Future,
    pin::Pin,
    task::{Poll, Waker},
};

pub(super) fn is_output(a: Action) -> bool {
    matches!(
        a,
        Action::RenderWav
            | Action::ExportMidi
            | Action::Export
            | Action::ExportLink16Sav
            | Action::ExportLink16Sd
            | Action::ExportRom
            | Action::ExportPlayerRoms
            | Action::ExportStudio(_)
    )
}

pub(super) type OutputDialog = Pin<Box<dyn Future<Output = Choice>>>;
pub(super) enum Choice {
    Selected(Job, PathBuf, Option<PathBuf>),
    Cancelled,
    Dismissed,
}
pub(super) struct OutputReport {
    status: String,
    details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooser_pending_keeps_gui_live_cancel_releases_all_exports() {
        let mut app = GbseqApp::new_with_audio(Size::new(800.0, 600.0), false);
        let ready = std::rc::Rc::new(std::cell::Cell::new(false));
        let signal = ready.clone();
        app.output_dialog = Some(Box::pin(std::future::poll_fn(move |_cx| {
            if signal.get() {
                Poll::Ready(Choice::Cancelled)
            } else {
                Poll::Pending
            }
        })));
        for a in [
            Action::RenderWav,
            Action::Export,
            Action::ExportLink16Sav,
            Action::ExportLink16Sd,
            Action::ExportRom,
            Action::ExportPlayerRoms,
            Action::ExportStudio(crate::gb_studio::Format::Uge),
        ] {
            assert!(!app.action_enabled(a));
            app.apply_action(a); // even programmatic repeats must not open a second picker
        }
        for _ in 0..3 {
            app.tick();
            app.draw_list();
        }
        app.apply_action(Action::Menu(MenuKind::View));
        assert_eq!(app.menu, Some(MenuKind::View));
        ready.set(true);
        app.tick();
        assert!(app.export_rx.is_none() && app.output_dialog.is_none());
        assert!(app.action_enabled(Action::ExportLink16Sav));
        assert_eq!(app.status, "Export cancelled");
    }

    #[test]
    fn accepted_folder_writes_snapshot_in_worker_and_reports_completion() {
        let mut app = GbseqApp::new_with_audio(Size::new(800.0, 600.0), false);
        let dir = std::env::temp_dir().join(format!(
            "gbseq-output-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let job = app.output_job(Action::ExportLink16Sav);
        let original_bpm = job.project.bpm;
        let path = dir.clone();
        app.output_dialog = Some(Box::pin(async move { Choice::Selected(job, path, None) }));
        app.project.bpm = original_bpm + 1;
        app.tick();
        assert!(!app.action_enabled(Action::Export));
        let limit = Instant::now() + Duration::from_secs(5);
        while app.export_rx.is_some() {
            assert!(Instant::now() < limit);
            app.tick();
            thread::yield_now();
        }
        assert!(app.status.starts_with("LINK16 "), "{}", app.status);
        let save = fs::read(dir.join("link16_local.sav")).unwrap();
        assert_eq!(save.len(), crate::link16_sav::LINK16_SAV_BYTES);
        let at = crate::link16_sav::LINK16_BPM_LO;
        assert_eq!(u16::from_le_bytes([save[at], save[at + 1]]), original_bpm);
        for entry in fs::read_dir(&dir).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
}
pub(super) struct Job {
    action: Action,
    project: SequenceProject,
    edit_player: usize,
    monitor: Option<usize>,
    language: Language,
}

impl Job {
    fn write(self, path: &Path, input_rom: Option<&Path>) -> Result<OutputReport> {
        let p = &self.project;
        let mut details = None;
        let status = match self.action {
            Action::ExportMidi => {
                crate::midi::export(p, path)?;
                format!("MIDI -> {}", path.display())
            }
            Action::RenderWav => {
                crate::services::render_wav(p, path, self.monitor)?;
                format!("WAV rendered -> {}", path.display())
            }
            Action::Export => {
                let m = export_project_to_dir(
                    &p.to_gbdaw_player(self.edit_player)?,
                    path,
                    StreamMode::Extended,
                    ExportPolicy::default(),
                )?;
                format!(
                    "KITAQGB export {} bytes -> {}",
                    m.songs.first().map_or(0, |s| s.bytes),
                    path.display()
                )
            }
            Action::ExportLink16Sav => {
                let m = export_link16_saves(p, path)?;
                format!(
                    "LINK16 {} players / {} steps / {} BPM -> {}",
                    p.player_count(),
                    m.steps,
                    m.effective_bpm,
                    path.display()
                )
            }
            Action::ExportLink16Sd => {
                let m = crate::link16_sd::export_link16_sd_package(
                    p,
                    input_rom.context("Missing LINK16 V4 ROM")?,
                    path,
                )?;
                format!(
                    "SD package {} players -> {}",
                    m.players.len(),
                    m.directory.display()
                )
            }
            Action::ExportRom => {
                let m = crate::link16_rom::export_link16_rom(p, path)?;
                format!(
                    "ROM {} players / {} steps -> {}",
                    m.players,
                    m.steps,
                    path.display()
                )
            }
            Action::ExportPlayerRoms => {
                let m = crate::player_rom::export_player_roms(p, path)?;
                format!(
                    "{} playback ROMs / 512KB each / no SRAM -> {}",
                    m.players,
                    m.directory.display()
                )
            }
            Action::ExportStudio(fmt) => {
                let m = crate::gb_studio::export(p, path, fmt)?;
                let mut lines = m
                    .files
                    .iter()
                    .map(|f| f.display().to_string())
                    .collect::<Vec<_>>();
                lines.extend(
                    m.warnings
                        .iter()
                        .map(|w| self.language.translate(w).into_owned()),
                );
                details = Some(lines.join("\n"));
                format!("GB Studio: {} -> {}", m.files.len(), m.files[0].display())
            }
            _ => anyhow::bail!("Not an export command"),
        };
        Ok(OutputReport { status, details })
    }

    async fn choose(self) -> Choice {
        use rfd::AsyncFileDialog as Picker;
        let tr = |s: &'static str| self.language.translate(s).into_owned();
        let mut rom = None;
        let pick = match self.action {
            Action::ExportMidi => {
                Picker::new()
                    .add_filter("MIDI", &["mid"])
                    .set_file_name("song.mid")
                    .save_file()
                    .await
            }
            Action::RenderWav => {
                Picker::new()
                    .add_filter(tr("Wave audio"), &["wav"])
                    .set_file_name("gbseq_mix.wav")
                    .save_file()
                    .await
            }
            Action::ExportRom => {
                Picker::new()
                    .set_title(tr("Export song as Game Boy ROM"))
                    .add_filter("Game Boy ROM", &["gb"])
                    .set_file_name("link16_song.gb")
                    .save_file()
                    .await
            }
            Action::ExportStudio(f) => {
                Picker::new()
                    .set_title(tr("Export GB Studio music"))
                    .add_filter("GB Studio", &[f.extension()])
                    .set_file_name(format!("song.{}", f.extension()))
                    .save_file()
                    .await
            }
            Action::ExportLink16Sd => {
                let Some(input) = Picker::new()
                    .set_title(tr("Choose matching LINK16 V4 ROM"))
                    .add_filter("Game Boy ROM", &["gb"])
                    .pick_file()
                    .await
                else {
                    return Choice::Cancelled;
                };
                rom = Some(input.path().to_owned());
                Picker::new()
                    .set_title(tr("Choose SD package output"))
                    .pick_folder()
                    .await
            }
            Action::ExportPlayerRoms => {
                Picker::new()
                    .set_title(tr("Export playback ROMs (no SRAM)"))
                    .pick_folder()
                    .await
            }
            Action::ExportLink16Sav | Action::Export => {
                Picker::new()
                    .set_title(tr("Choose export folder"))
                    .pick_folder()
                    .await
            }
            _ => unreachable!(),
        };
        match pick {
            Some(f) => Choice::Selected(self, f.path().to_owned(), rom),
            None => Choice::Cancelled,
        }
    }
}

impl GbseqApp {
    fn output_job(&self, action: Action) -> Job {
        Job {
            action,
            project: self.project.clone(),
            edit_player: self.active_player,
            monitor: self.monitor_player(),
            language: self.language,
        }
    }
    pub(super) fn choose_export(&mut self, action: Action) {
        if self.output_dialog.is_some() || self.export_rx.is_some() {
            return;
        }
        self.output_dialog = Some(Box::pin(self.output_job(action).choose()));
        self.status = "Choose export destination...".into();
    }
    #[cfg(test)]
    pub(super) fn start_wav_export(&mut self, path: PathBuf) {
        self.start_export_writer(self.output_job(Action::RenderWav), path, None);
    }
    fn start_export_writer(&mut self, job: Job, path: PathBuf, rom: Option<PathBuf>) {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(
                job.write(&path, rom.as_deref())
                    .map_err(|e| format!("EXPORT failed: {e:#}")),
            );
        });
        self.export_rx = Some(rx);
        self.status = "Writing export in background...".into();
    }
    pub(super) fn poll_export(&mut self) -> bool {
        if let Some(dialog) = &mut self.output_dialog {
            // The plita event loop ticks continuously. Only poll once per tick;
            // never block_on or join while the native panel is open.
            let mut ctx = std::task::Context::from_waker(Waker::noop());
            match dialog.as_mut().poll(&mut ctx) {
                Poll::Pending => return false,
                Poll::Ready(choice) => {
                    self.output_dialog = None;
                    match choice {
                        Choice::Selected(job, path, rom) => {
                            self.start_export_writer(job, path, rom)
                        }
                        Choice::Cancelled => self.status = "Export cancelled".into(),
                        Choice::Dismissed => {}
                    }
                    return true;
                }
            }
        }
        let Some(rx) = &self.export_rx else {
            return false;
        };
        let completed = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("Export worker disconnected".to_string()),
        };
        self.export_rx = None;
        match completed {
            Ok(report) => {
                self.status = report.status;
                if let Some(text) = report.details {
                    self.output_dialog = Some(Box::pin(async move {
                        rfd::AsyncMessageDialog::new()
                            .set_title("GB Studio")
                            .set_description(text)
                            .show()
                            .await;
                        Choice::Dismissed
                    }));
                }
            }
            Err(e) => self.status = e,
        }
        true
    }
}
