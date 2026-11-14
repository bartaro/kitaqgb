//! Project replacement never waits in a plita input callback. The current
//! document remains authoritative until a requested save actually succeeds.
use super::*;
use std::{
    future::Future,
    pin::Pin,
    task::{Poll, Waker},
};

pub(super) enum Replace {
    New,
    Load(PathBuf),
}
enum After {
    Open,
    Save(Option<Replace>),
}
type Picker = Pin<Box<dyn Future<Output = Option<PathBuf>>>>;

#[derive(Default)]
pub(super) struct Files {
    confirm: Option<Replace>,
    picker: Option<(Picker, After)>,
    selected: usize,
    pressed: Option<usize>,
    swallow_double: bool,
}
impl Files {
    pub(super) fn active(&self) -> bool {
        self.confirm.is_some() || self.picker.is_some()
    }
}
impl GbseqApp {
    fn prepare_file_action(&mut self) {
        self.menu = None;
        self.buttons.clear();
        self.finish_measures_input(true);
        self.stop_audio(); // finalise a held recorded note before comparing/saving
        self.space_down = false;
        self.keyboard_shortcut_down = false;
        self.selection_modifier = false;
        self.shift_down = false;
        self.drag = None;
        self.wave_dragging = false;
        self.wave_drag_last = None;
        self.timeline_scroll.end_scrollbar_drag();
        if let Some(before) = self.drag_before.take().or(self.wave_drag_before.take()) {
            self.push_undo(before);
        }
    }
    pub(super) fn request_replace(&mut self, next: Replace) {
        self.prepare_file_action();
        if self.project == self.saved_project {
            self.replace_project(next);
        } else {
            self.files.confirm = Some(next);
            self.files.selected = 2; // Enter, like Escape, safely keeps the song.
            self.files.pressed = None;
        }
    }
    fn replace_project(&mut self, next: Replace) {
        if let Replace::Load(path) = next {
            if let Err(e) = self.load_path(&path) {
                self.status = format!("OPEN failed: {e:#}");
            }
            return;
        }
        self.project = SequenceProject::empty();
        self.chords.clear();
        self.reset_monitor();
        self.saved_project = self.project.clone();
        self.selected_notes.clear();
        self.current_path = None;
        self.selected_note = None;
        self.active_player = 0;
        self.all_players_view = false;
        self.selected_track = 0;
        self.cursor_step = 0;
        self.detail_open = false;
        self.undo.clear();
        self.redo.clear();
        self.timeline_scroll = ScrollAreaState::new();
        self.status = "NEW sequence".into();
    }
    pub(super) fn choose_project(&mut self, midi: bool) {
        self.prepare_file_action();
        let dlg = rfd::AsyncFileDialog::new();
        let dlg = if midi {
            dlg.add_filter("MIDI", &["mid", "midi"])
        } else {
            dlg.add_filter(self.language.translate("GBSEQ project"), &["gbseq", "json"])
        };
        self.files.picker = Some((
            Box::pin(async move { dlg.pick_file().await.map(|f| f.path().to_owned()) }),
            After::Open,
        ));
    }
    pub(super) fn request_save(&mut self, save_as: bool, after: Option<Replace>) {
        if self.live.take_before.is_some() {
            self.stop_audio();
        }
        if !save_as {
            if let Some(path) = self.current_path.clone() {
                self.complete_save(path, after);
                return;
            }
        }
        self.prepare_file_action();
        let dlg = rfd::AsyncFileDialog::new()
            .add_filter(self.language.translate("GBSEQ project"), &["gbseq"])
            .set_file_name(
                self.current_path
                    .as_ref()
                    .and_then(|p| p.file_name()?.to_str())
                    .unwrap_or("sequence.gbseq"),
            );
        self.files.picker = Some((
            Box::pin(async move { dlg.save_file().await.map(|f| f.path().to_owned()) }),
            After::Save(after),
        ));
    }
    fn complete_save(&mut self, path: PathBuf, after: Option<Replace>) {
        match self.save_path(&path) {
            Err(e) => self.status = format!("SAVE failed: {e:#}"),
            Ok(()) => {
                if let Some(next) = after {
                    self.replace_project(next);
                }
            }
        }
    }
    pub(super) fn poll_project_picker(&mut self) -> bool {
        let Some((f, _)) = self.files.picker.as_mut() else {
            return false;
        };
        let Poll::Ready(path) = f
            .as_mut()
            .poll(&mut std::task::Context::from_waker(Waker::noop()))
        else {
            return false;
        };
        let (_, after) = self.files.picker.take().unwrap();
        if let Some(path) = path {
            match after {
                After::Open => self.request_replace(Replace::Load(path)),
                After::Save(after) => self.complete_save(path, after),
            }
        }
        true
    }
    fn decide(&mut self, choice: usize) {
        let Some(next) = self.files.confirm.take() else {
            return;
        };
        self.files.pressed = None;
        match choice {
            0 => self.request_save(false, Some(next)),
            1 => self.replace_project(next),
            _ => {}
        }
    }
    fn prompt_rects(&self) -> (Rect, [Rect; 3]) {
        let w = (self.size.width - 16.).clamp(7., 510.);
        let panel = Rect::new(
            (self.size.width - w) / 2.,
            ((self.size.height - 148.) / 2.).max(0.),
            w,
            148.,
        );
        let bw = ((w - 32.) / 3.).max(1.);
        (
            panel,
            std::array::from_fn(|i| {
                Rect::new(panel.x + 10. + i as f32 * (bw + 6.), panel.y + 98., bw, 36.)
            }),
        )
    }
    pub(super) fn project_file_event(&mut self, e: &UiEvent) -> bool {
        if self.files.swallow_double && matches!(e, UiEvent::PointerDoubleClick { .. }) {
            self.files.swallow_double = false;
            return true;
        }
        if matches!(e, UiEvent::PointerDown { .. }) {
            self.files.swallow_double = false;
        }
        if !self.files.active() {
            return false;
        }
        if self.files.confirm.is_none() {
            return true;
        } // native picker owns input
        let (_, buttons) = self.prompt_rects();
        let hit = |p: Point| buttons.iter().position(|r| r.contains(p));
        match e {
            UiEvent::KeyDown { key, shift, .. } => match normalized_key(key).as_str() {
                "ESCAPE" => self.decide(2),
                "ENTER" | "RETURN" => self.decide(self.files.selected),
                "TAB" | "ARROWLEFT" | "ARROWRIGHT" => {
                    let back = *shift || normalized_key(key) == "ARROWLEFT";
                    self.files.selected = (self.files.selected + if back { 2 } else { 1 }) % 3;
                }
                _ => {}
            },
            UiEvent::PointerDown {
                position,
                button: PointerButton::Primary,
            } => {
                self.files.pressed = hit(*position);
                if let Some(i) = self.files.pressed {
                    self.files.selected = i;
                }
            }
            UiEvent::PointerUp {
                position,
                button: PointerButton::Primary,
            } => {
                if let Some(i) = self
                    .files
                    .pressed
                    .take()
                    .filter(|&i| Some(i) == hit(*position))
                {
                    self.files.swallow_double = true;
                    self.decide(i);
                }
            }
            UiEvent::FocusChanged { focused: false } => {
                self.files.pressed = None;
            }
            _ => {}
        }
        true
    }
    pub(super) fn draw_project_prompt(&self, list: &mut DrawList) {
        if !self.files.active() {
            return;
        }
        let (panel, buttons) = self.prompt_rects();
        list.fill_rect(self.root_rect(), Color::rgba8(0, 0, 0, 145));
        list.fill_rect(panel, PANEL);
        list.stroke_rect(panel, 1., ACCENT);
        let line = |y| Rect::new(panel.x + 14., panel.y + y, panel.width - 28., 30.);
        if self.files.confirm.is_none() {
            self.text_in(list, line(56.), "Waiting for the file dialog", 1.2, TEXT);
            return;
        }
        self.text_in(list, line(16.), "GBSEQ - Unsaved changes", 1.3, ACCENT);
        self.text_in(
            list,
            line(55.),
            "Save this sequence before replacing it?",
            1.1,
            TEXT,
        );
        for (i, (rect, label)) in buttons
            .into_iter()
            .zip(["SAVE", "Discard changes", "Cancel"])
            .enumerate()
        {
            let focused = self.files.selected == i;
            list.fill_rect(rect, if focused { ACCENT } else { PANEL_RAISED });
            self.text_in(
                list,
                Rect::new(rect.x + 7., rect.y + 8., rect.width - 14., 22.),
                label,
                1.1,
                if focused { BG } else { TEXT },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dirty() -> GbseqApp {
        let mut a = GbseqApp::new_with_audio(Size::new(1366., 700.), false);
        a.open_key_track(1);
        a.toggle_keyboard();
        a.apply_action(Action::ToggleLoop);
        a
    }
    fn click(a: &mut GbseqApp, r: Rect) {
        let position = Point::new(r.x + 4., r.y + 4.);
        a.handle_event(UiEvent::PointerDown {
            position,
            button: PointerButton::Primary,
        });
        a.handle_event(UiEvent::PointerUp {
            position,
            button: PointerButton::Primary,
        });
    }
    fn key(a: &mut GbseqApp, k: &str) {
        a.handle_event(UiEvent::KeyDown {
            key: k.into(),
            ctrl: false,
            shift: false,
            meta: false,
            alt: false,
        });
    }
    #[test]
    fn file_menu_new_with_performance_panel_dirty_cancel_and_discard() {
        let mut a = dirty();
        let before = a.project.clone();
        let undo = a.undo.len();
        a.apply_action(Action::Menu(MenuKind::File));
        let new = a
            .menu_buttons()
            .into_iter()
            .find(|s| s.action == Action::New)
            .unwrap();
        click(&mut a, new.rect);
        assert!(a.files.confirm.is_some());
        for _ in 0..4 {
            a.tick();
            a.draw_list();
        }
        key(&mut a, "a");
        assert!(a.files.confirm.is_some());
        key(&mut a, "Named(Escape)");
        assert!(!a.files.active() && a.live.open && a.key_editor);
        assert_eq!(a.project, before);
        assert_eq!(a.undo.len(), undo);
        a.apply_action(Action::New);
        key(&mut a, "Named(Enter)");
        assert_eq!(a.project, before);
        a.apply_action(Action::New);
        let discard = a.prompt_rects().1[1];
        click(&mut a, discard);
        assert!(!a.files.active());
        assert_eq!(a.project, SequenceProject::empty());
    }
    #[test]
    fn save_then_replace_requires_success_not_cancel_or_error() {
        let mut a = dirty();
        let before = a.project.clone();
        a.apply_action(Action::New);
        a.decide(0);
        assert!(a.files.picker.is_some() && a.files.confirm.is_none());
        // Replace the unpolled native future with a controlled file picker.
        let (picker, after) = a.files.picker.as_mut().unwrap();
        assert!(matches!(after, After::Save(Some(Replace::New))));
        *picker = Box::pin(std::future::pending());
        for _ in 0..3 {
            a.tick();
            a.draw_list();
        }
        assert!(a.files.active() && a.project == before);
        a.files.picker.as_mut().unwrap().0 = Box::pin(async { None });
        a.tick();
        assert!(!a.files.active() && a.project == before && a.project != a.saved_project);
        a.current_path = Some(std::env::temp_dir()); // a directory cannot be saved as a song
        a.apply_action(Action::New);
        a.decide(0);
        assert!(
            a.status.starts_with("SAVE failed:")
                && a.project == before
                && a.project != a.saved_project
        );
        let path = std::env::temp_dir().join(format!(
            "gbseq-replace-{}-{}.gbseq",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        a.current_path = Some(path.clone());
        a.apply_action(Action::New);
        a.decide(0);
        let saved: SequenceProject = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved, before);
        assert_eq!(a.project, SequenceProject::empty());
        fs::remove_file(path).unwrap();
    }
}
