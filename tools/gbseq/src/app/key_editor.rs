use super::*;

pub(super) const KEYBOARD_WIDTH: f32 = 88.0;

impl GbseqApp {
    pub(super) fn timeline_left(&self) -> f32 {
        if self.key_editor {
            16.0 + KEYBOARD_WIDTH + 12.0
        } else {
            TRACK_HEADER_W
        }
    }

    pub(super) fn open_key_track(&mut self, track: usize) {
        self.key_editor = true;
        self.selected_track = track.min(self.project.tracks.len() - 1);
        // Keep the all-player playback choice when opening a track from ALL16.
        self.active_player = self.selected_track / TRACK_COUNT;
        self.selected_note = self.selected_note.filter(|id| {
            self.project
                .note_location(*id)
                .is_some_and(|(ch, _)| ch == self.selected_track)
        });
        self.selected_notes.retain(|id| {
            self.project
                .note_location(*id)
                .is_some_and(|(ch, _)| ch == self.selected_track)
        });
        self.detail_open &= self.selected_note.is_some();
        self.center_key_range();
        self.clamp_scroll();
        self.status =
            "KEY EDITOR: drag notes / wheel: pitch / Shift+wheel: time / E: all tracks".into();
    }

    pub(super) fn center_key_range(&mut self) {
        let center = self
            .selected_note
            .and_then(|id| self.project.note(id))
            .or_else(|| self.project.tracks[self.selected_track].notes.first())
            .map_or(24, |note| note.pitch);
        self.key_low = center.saturating_sub(12).min(24);
    }

    pub(super) fn key_height(&self) -> f32 {
        self.timeline_viewport().height / 24.0
    }

    pub(super) fn key_pitch(&self, y: f32) -> u8 {
        (self.key_low as f32
            + ((self.timeline_viewport().bottom() - y) / self.key_height()).floor())
        .clamp(self.key_low as f32, (self.key_low + 23) as f32) as u8
    }

    pub(super) fn keyboard_rect(&self) -> Rect {
        let r = self.timeline_viewport();
        Rect::new(16.0, r.y, KEYBOARD_WIDTH, r.height)
    }

    pub(super) fn key_note_rect(&self, note: &MidiNote) -> Rect {
        let r = self.timeline_viewport();
        Rect::new(
            self.step_x(note.start),
            r.bottom() - (note.pitch as f32 - self.key_low as f32 + 1.0) * self.key_height() + 1.0,
            (note.length as f32 * self.half_step_w - 3.0).max(12.0),
            (self.key_height() - 2.0).max(3.0),
        )
    }

    pub(super) fn audition_key(&mut self, pitch: u8) {
        let track = self.selected_track;
        let mut note = self
            .selected_note
            .and_then(|id| self.project.note(id))
            .cloned()
            .unwrap_or(MidiNote {
                id: 1,
                start: 0,
                length: 2,
                pitch,
                velocity: 15,
                effects: crate::model::NoteEffects::for_track(self.project.tracks[track].kind),
            });
        note.pitch = pitch;
        self.preview_note(track, note);
    }

    pub(super) fn draw_key_editor(&self, list: &mut DrawList) {
        let r = self.timeline_viewport();
        let h = self.key_height();
        let keys = self.keyboard_rect();
        let black_width = keys.width * 0.66;
        list.fill_rect(Rect::new(0.0, r.y, self.timeline_left(), r.height), PANEL);
        let noise = self.selected_track % TRACK_COUNT == 3;
        for pitch in self.key_low..self.key_low + 24 {
            let y = r.bottom() - (pitch - self.key_low + 1) as f32 * h;
            let black = !noise && matches!(pitch % 12, 1 | 3 | 6 | 8 | 10);
            list.fill_rect(
                Rect::new(r.x, y, r.width, h),
                if black {
                    Color::rgb8(17, 23, 30)
                } else {
                    Color::rgb8(25, 33, 42)
                },
            );
            list.fill_rect(
                Rect::new(r.x, y + h - 1.0, r.width, 1.0),
                if pitch % 12 == 0 {
                    GRID_MEASURE
                } else {
                    BORDER.with_alpha(0.35)
                },
            );
            list.fill_rect(
                Rect::new(keys.x, y, keys.width, h - 1.0),
                Color::rgb8(214, 223, 228),
            );
            if black {
                list.fill_rect(
                    Rect::new(keys.x, y, black_width, h - 1.0),
                    Color::rgb8(35, 42, 51),
                );
            }
            let label = if noise {
                format!("NOISE {pitch:02}")
            } else {
                note_label(pitch)
            };
            self.text_in(
                list,
                Rect::new(
                    if black || noise {
                        keys.x + 6.0
                    } else {
                        keys.right() - measure_text_width(&label, 0.95) - 6.0
                    },
                    y + (h - 13.0) / 2.0,
                    if black {
                        black_width - 12.0
                    } else {
                        keys.width - 12.0
                    },
                    h,
                ),
                &label,
                0.95,
                if black { TEXT } else { BG },
            );
        }
        for spec in self.track_buttons() {
            let active = match spec.action {
                Action::ToggleMute(index) => self.project.tracks[index].mute,
                Action::ToggleSolo(index) => self.project.tracks[index].solo,
                _ => false,
            };
            self.draw_button(list, &spec, active);
        }
        list.push_clip(r);
        let first = (self.scroll_x() / self.half_step_w).floor().max(0.0) as u32;
        let last = ((self.scroll_x() + r.width) / self.half_step_w)
            .ceil()
            .min(self.project.total_steps() as f32) as u32;
        for step in first..=last {
            let (width, color) = if step % STEPS_PER_MEASURE == 0 {
                (2.0, GRID_MEASURE)
            } else if step % 4 == 0 {
                (1.0, GRID_BEAT)
            } else {
                (1.0, GRID_HALF)
            };
            list.fill_rect(Rect::new(self.step_x(step), r.y, width, r.height), color);
        }
        list.fill_rect(
            Rect::new(self.step_x(self.cursor_step), r.y, 1.0, r.height),
            ACCENT.with_alpha(0.72),
        );
        for note in &self.project.tracks[self.selected_track].notes {
            let rect = self.key_note_rect(note);
            if !rect.intersects(r) {
                continue;
            }
            let selected =
                self.selected_note == Some(note.id) || self.selected_notes.contains(&note.id);
            let color = track_color(self.selected_track);
            list.fill_vertical_gradient_rect(rect, color, color.with_alpha(0.7));
            list.stroke_rect(
                rect,
                if selected { 2.0 } else { 1.0 },
                if selected { TEXT } else { color },
            );
            list.fill_rect(
                Rect::new(
                    rect.right() - NOTE_HANDLE_W,
                    rect.y,
                    NOTE_HANDLE_W,
                    rect.height,
                ),
                if selected {
                    TEXT.with_alpha(0.9)
                } else {
                    BG.with_alpha(0.72)
                },
            );
            let label = if noise {
                format!("NOISE {:02}", note.pitch)
            } else {
                note_label(note.pitch)
            };
            if rect.width > 42.0 {
                self.text_in(
                    list,
                    Rect::new(
                        rect.x + 4.0,
                        rect.y + 3.0,
                        rect.width - NOTE_HANDLE_W - 6.0,
                        rect.height - 3.0,
                    ),
                    &label,
                    0.85,
                    BG,
                );
            }
        }
        if let Some(step) = self.playhead_step() {
            list.fill_rect(
                Rect::new(self.step_x(0) + step * self.half_step_w, r.y, 2.0, r.height),
                PLAYHEAD,
            );
        }
        list.pop_clip();
        self.timeline_scroll.draw_scrollbars(
            list,
            self.scroll_area_rect(),
            self.timeline_content_size(),
            ScrollbarStyle {
                thickness: SCROLLBAR_H,
                ..ScrollbarStyle::default()
            },
        );
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    fn editor() -> (GbseqApp, u64) {
        let mut app = GbseqApp::new_with_audio(Size::new(1280.0, 720.0), false);
        app.project = SequenceProject::empty();
        let id = app.project.insert_note_exact(0, 4, 2, 24, 15).unwrap();
        app.open_key_track(0);
        (app, id)
    }

    fn down(app: &mut GbseqApp, position: Point) {
        app.handle_event(UiEvent::PointerDown {
            position,
            button: PointerButton::Primary,
        });
    }

    fn up(app: &mut GbseqApp, position: Point) {
        app.handle_event(UiEvent::PointerUp {
            position,
            button: PointerButton::Primary,
        });
    }

    #[test]
    fn keyboard_matches_current_fcseq_and_freed_space_accepts_notes() {
        let (mut app, _) = editor();
        assert_eq!(app.keyboard_rect().width, 88.0);
        assert_eq!(app.keyboard_rect().x, 16.0);
        assert_eq!(app.timeline_viewport().x, 116.0);
        let p = Point::new(140.0, app.timeline_viewport().y + app.key_height() * 2.5);
        assert!(!app.keyboard_rect().contains(p));
        assert!(app.timeline_viewport().contains(p));
        let count = app.project.tracks[0].notes.len();
        app.handle_event(UiEvent::PointerDoubleClick {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(app.project.tracks[0].notes.len(), count + 1);
        assert!(app
            .track_buttons()
            .iter()
            .all(|b| b.rect.right() <= app.timeline_viewport().x));
        app.handle_key("E", false, false);
        assert!(!app.key_editor);
        assert_eq!(app.timeline_viewport().x, TRACK_HEADER_W);
    }

    #[test]
    fn right_edge_release_position_is_applied_even_without_a_move_event() {
        let (mut app, id) = editor();
        let before = app.project.clone();
        let r = app.note_rect(0, app.project.note(id).unwrap());
        let p = Point::new(r.right() - 2.0, r.y + r.height / 2.0);
        let q = Point::new(p.x + app.half_step_w * 3.0, p.y);
        down(&mut app, p);
        up(&mut app, q);
        assert_eq!(app.project.note(id).unwrap().length, 5);
        assert_eq!(app.project.note(id).unwrap().start, 4);
        assert_eq!(app.undo.len(), 1);
        app.undo();
        assert_eq!(app.project, before);
        app.redo();
        assert_eq!(app.project.note(id).unwrap().length, 5);
    }

    #[test]
    fn right_edge_can_be_grabbed_from_the_border_and_outside_padding() {
        for padding in [0.0, 2.0, 4.0] {
            let (mut app, id) = editor();
            let r = app.note_rect(0, app.project.note(id).unwrap());
            let p = Point::new(r.right() + padding, r.y + r.height / 2.0);
            let q = Point::new(p.x + app.half_step_w * 2.0, p.y);
            down(&mut app, p);
            assert!(matches!(app.drag, Some(DragKind::Resize { id: hit, .. }) if hit == id));
            app.handle_event(UiEvent::PointerMove { position: q });
            up(&mut app, q);
            assert_eq!(app.project.note(id).unwrap().length, 4);
        }
    }

    #[test]
    fn dragging_beyond_the_next_note_fills_the_gap_and_preserves_other_notes() {
        let (mut app, id) = editor();
        let other = app.project.insert_note_exact(0, 10, 2, 28, 12).unwrap();
        let other_before = app.project.note(other).unwrap().clone();
        let r = app.note_rect(0, app.project.note(id).unwrap());
        let p = Point::new(r.right() - 2.0, r.y + r.height / 2.0);
        let q = Point::new(p.x + app.half_step_w * 10.0, p.y);
        down(&mut app, p);
        app.handle_event(UiEvent::PointerMove { position: q });
        up(&mut app, q);
        assert_eq!(app.project.note(id).unwrap().end(), 10);
        assert_eq!(app.project.note(other).unwrap(), &other_before);
        assert_eq!(app.undo.len(), 1);
        app.project.validate().unwrap();
    }

    #[test]
    fn selected_handle_remains_grabbable_in_minimum_width_overlaps() {
        let (mut app, id) = editor();
        // At 1 px/step, the 12 px minimum visual bars overlap even when the
        // stored notes do not. Click the selected bar's visible right handle.
        app.half_step_w = 1.0;
        let other = app.project.insert_note_exact(0, 12, 2, 24, 12).unwrap();
        app.selected_note = Some(id);
        let r = app.note_rect(0, app.project.note(id).unwrap());
        let p = Point::new(r.right() - 2.0, r.y + r.height / 2.0);
        let q = Point::new(p.x + 3.0, p.y);
        down(&mut app, p);
        assert!(matches!(app.drag, Some(DragKind::Resize { id: hit, .. }) if hit == id));
        app.handle_event(UiEvent::PointerMove { position: q });
        up(&mut app, q);
        assert_eq!(app.project.note(id).unwrap().length, 5);
        assert_eq!(app.project.note(other).unwrap().start, 12);
    }

    #[test]
    fn resize_stops_at_song_end_and_can_shrink_back_during_the_same_stroke() {
        let (mut app, id) = editor();
        let r = app.note_rect(0, app.project.note(id).unwrap());
        let p = Point::new(r.right() - 2.0, r.y + r.height / 2.0);
        down(&mut app, p);
        app.handle_event(UiEvent::PointerMove {
            position: Point::new(
                p.x + app.half_step_w * app.project.total_steps() as f32,
                p.y,
            ),
        });
        assert_eq!(
            app.project.note(id).unwrap().end(),
            app.project.total_steps()
        );
        let q = Point::new(p.x - app.half_step_w, p.y);
        up(&mut app, q);
        assert_eq!(app.project.note(id).unwrap().length, 1);
        assert_eq!(app.undo.len(), 1);
        app.project.validate().unwrap();
    }
}
