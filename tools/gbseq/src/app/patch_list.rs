use super::*;
use crate::presets::{factory, Preset};
use std::{
    future::Future,
    pin::Pin,
    task::{Poll, Waker},
};

type FileChoice = Pin<Box<dyn Future<Output = Result<Option<(PathBuf, Preset)>>>>>;
#[derive(Default)]
pub(super) struct Browser {
    pub page: Option<usize>,
    users: Vec<(PathBuf, Preset)>,
    file: Option<FileChoice>,
    swallow_up: bool,
}
const PAGE: usize = 8;
fn library() -> Option<PathBuf> {
    i18n::preference_path().map(|p| p.with_file_name("presets"))
}
impl GbseqApp {
    fn remember_patch(&mut self, path: PathBuf, p: Preset) {
        self.patches.users.retain(|(k, _)| *k != path);
        self.patches.users.push((path, p));
        self.patches.users.sort_by(|a, b| a.0.cmp(&b.0));
    }
    fn open_patches(&mut self) {
        if !cfg!(test) {
            if let Some(folder) = library() {
                if let Ok(dir) = fs::read_dir(folder) {
                    for entry in dir.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("gbpreset") {
                            if let Ok(p) = Preset::load(&path) {
                                self.remember_patch(path, p);
                            }
                        }
                    }
                }
            }
        }
        let idx = self.project.tracks[self.selected_track]
            .input_patch
            .as_ref()
            .and_then(|p| p.name.get(..3))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        self.patches.page = Some(idx.saturating_sub(1).min(127) / PAGE);
    }
    fn choice_count(&self) -> usize {
        128 + self
            .patches
            .users
            .iter()
            .filter(|(_, p)| p.channel == self.project.tracks[self.selected_track].kind)
            .count()
    }
    fn choice(&self, n: usize) -> Option<Preset> {
        let k = self.project.tracks[self.selected_track].kind;
        if n < 128 {
            Some(factory(k, n as u8))
        } else {
            self.patches
                .users
                .iter()
                .filter(|(_, p)| p.channel == k)
                .nth(n - 128)
                .map(|(_, p)| p.clone())
        }
    }
    fn apply_patch(&mut self, p: Preset) {
        let before = self.project.clone();
        let ids = self.selection_ids();
        match p.apply(&mut self.project, self.selected_track, &ids) {
            Ok(()) => {
                self.push_undo(before);
                self.status = format!("Preset: {}", p.patch.name);
            }
            Err(e) => self.status = e.to_string(),
        }
        self.patches.page = None;
    }
    fn file_patch(&mut self, save: bool) {
        if self.patches.file.is_some() {
            return;
        }
        let selected = if save {
            match Preset::capture(&self.project, self.selected_track, self.selected_note) {
                Ok(p) => Some(p),
                Err(e) => {
                    self.status = e.to_string();
                    return;
                }
            }
        } else {
            None
        };
        self.patches.page = None;
        self.patches.file = Some(Box::pin(async move {
            let mut dlg = rfd::AsyncFileDialog::new().add_filter("GBSEQ instrument", &["gbpreset"]);
            if let Some(dir) = library() {
                fs::create_dir_all(&dir)?;
                dlg = dlg.set_directory(dir);
            }
            if let Some(mut p) = selected {
                let Some(file) = dlg
                    .set_file_name("My instrument.gbpreset")
                    .save_file()
                    .await
                else {
                    return Ok(None);
                };
                let path = file.path().to_path_buf();
                p.patch.name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .context("invalid preset name")?
                    .to_string();
                p.save(&path)?;
                Ok(Some((path, p)))
            } else {
                let Some(file) = dlg.pick_file().await else {
                    return Ok(None);
                };
                let path = file.path().to_path_buf();
                let p = Preset::load(&path)?;
                Ok(Some((path, p)))
            }
        }));
    }
    pub(super) fn poll_patch_file(&mut self) -> bool {
        let Some(f) = &mut self.patches.file else {
            return false;
        };
        let mut ctx = std::task::Context::from_waker(Waker::noop());
        let Poll::Ready(res) = f.as_mut().poll(&mut ctx) else {
            return false;
        };
        self.patches.file = None;
        match res {
            Ok(Some((path, p))) => {
                self.status = format!(
                    "User preset: {} / {}",
                    p.channel.short_label(),
                    p.patch.name
                );
                let k = p.channel;
                self.remember_patch(path, p);
                if self.live.open && self.project.tracks[self.selected_track].kind == k {
                    self.patches.page = Some(128 / PAGE);
                }
            }
            Err(e) => self.status = e.to_string(),
            _ => {}
        }
        true
    }
    fn patch_row(&self, i: usize) -> Rect {
        let r = self.panel_cell(3, 0, 3);
        let h = (r.y / (PAGE + 1) as f32).clamp(12., 26.);
        Rect::new(
            r.x,
            (r.y - h * (PAGE + 1) as f32).max(0.) + i as f32 * h,
            (self.perf_rect().width * 0.62).min(420.),
            h,
        )
    }
    pub(super) fn draw_patch_picker(&self, list: &mut DrawList) {
        let t = &self.project.tracks[self.selected_track];
        let name = t.input_patch.as_ref().map_or("Preset", |p| p.name.as_str());
        for (i, s) in [
            format!("{}  ▾", self.language.translate(name)),
            "Save user preset".into(),
            "Load user preset".into(),
        ]
        .iter()
        .enumerate()
        {
            let r = self.panel_cell(3, i, 3);
            list.fill_rect(r, PANEL_RAISED);
            self.text_in(list, r, s, 0.9, TEXT);
        }
        if let Some(page) = self.patches.page {
            for i in 0..PAGE {
                let r = self.patch_row(i);
                list.fill_rect(r, PANEL_ALT);
                if let Some(p) = self.choice(page * PAGE + i) {
                    self.text_in(list, r, &p.patch.name, 0.9, TEXT);
                }
            }
            let r = self.patch_row(PAGE);
            list.fill_rect(r, HEADER);
            self.text_in(
                list,
                r,
                &format!(
                    "<            {} / {}            >",
                    page + 1,
                    self.choice_count().div_ceil(PAGE)
                ),
                0.95,
                ACCENT,
            );
        }
    }
    pub(super) fn patch_event(&mut self, e: &UiEvent) -> bool {
        if matches!(e, UiEvent::PointerUp { .. }) && std::mem::take(&mut self.patches.swallow_up) {
            return true;
        }
        let count = self.choice_count().div_ceil(PAGE);
        if let Some(page) = self.patches.page {
            match e {
                UiEvent::PointerDown {
                    position,
                    button: PointerButton::Primary,
                } => {
                    self.patches.swallow_up = true;
                    for i in 0..PAGE {
                        if self.patch_row(i).contains(*position) {
                            if let Some(p) = self.choice(page * PAGE + i) {
                                self.apply_patch(p);
                            }
                            return true;
                        }
                    }
                    let r = self.patch_row(PAGE);
                    self.patches.page = if r.contains(*position) {
                        Some(if position.x < r.x + r.width / 2. {
                            page.saturating_sub(1)
                        } else {
                            (page + 1).min(count - 1)
                        })
                    } else {
                        None
                    };
                    return true;
                }
                UiEvent::PointerWheel { delta_y, .. } => {
                    self.patches.page = Some(if *delta_y > 0. {
                        page.saturating_sub(1)
                    } else {
                        (page + 1).min(count - 1)
                    });
                    return true;
                }
                UiEvent::KeyDown { key, .. } => {
                    match normalized_key(key).as_str() {
                        "ESCAPE" => self.patches.page = None,
                        "PAGEDOWN" | "ARROWRIGHT" => {
                            self.patches.page = Some((page + 1).min(count - 1))
                        }
                        "PAGEUP" | "ARROWLEFT" => self.patches.page = Some(page.saturating_sub(1)),
                        k => {
                            if let Some(n) =
                                k.parse::<usize>().ok().filter(|n| (1..=PAGE).contains(n))
                            {
                                if let Some(p) = self.choice(page * PAGE + n - 1) {
                                    self.apply_patch(p);
                                }
                            }
                        }
                    }
                    return true;
                }
                UiEvent::PointerDoubleClick { .. } => return true,
                _ => {}
            }
        }
        if let UiEvent::PointerDown {
            position,
            button: PointerButton::Primary,
        } = e
        {
            for i in 0..3 {
                if self.panel_cell(3, i, 3).contains(*position) {
                    self.patches.swallow_up = true;
                    if i == 0 {
                        self.open_patches();
                    } else {
                        self.file_patch(i == 1);
                    }
                    return true;
                }
            }
        }
        false
    }
}
