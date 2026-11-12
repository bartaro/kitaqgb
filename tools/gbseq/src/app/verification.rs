use super::*;

fn wave_fixture(size: Size) -> GbseqApp {
    let mut app = GbseqApp::new_with_audio(size, false);
    app.language = Language::Japanese;
    app.selected_track = 2;
    app.selected_note = Some(app.project.tracks[2].notes[0].id);
    app.detail_open = true;
    app.add_user_wave();
    app
}

/// Native event-loop/GPU smoke, not a hardware-input or sound-output test.
pub fn run_startup_smoke(receipt: PathBuf) -> plita::prelude::Result<()> {
    struct Probe {
        app: GbseqApp,
        ticks: u32,
        draws: std::cell::Cell<u32>,
        receipt: PathBuf,
        resize_origin: Option<Point>,
        monitor_path: Vec<(usize, Option<usize>, usize)>,
        resized_length: Option<u32>,
        performance_verified: bool,
    }
    impl PlitaProgram for Probe {
        fn resize(&mut self, size: Size) -> UiResponse {
            self.app.resize(size)
        }
        fn handle_event(&mut self, event: UiEvent) -> UiResponse {
            self.app.handle_event(event)
        }
        fn draw_list(&self) -> DrawList {
            self.draws.set(self.draws.get() + 1);
            self.app.draw_list()
        }
        fn tick(&mut self) -> UiResponse {
            self.ticks += 1;
            if self.ticks == 4 {
                self.app.apply_action(Action::Keyboard);
                self.app.apply_action(Action::ChordMix);
                self.app.apply_action(Action::ChordLane(0));
                self.app.menu = None;
                let r = self.app.panel_cell(3, 0, 3);
                let pt = Point::new(r.x + 5., r.y + 5.);
                self.app.handle_event(UiEvent::PointerDown {
                    position: pt,
                    button: PointerButton::Primary,
                });
                self.app.handle_event(UiEvent::PointerUp {
                    position: pt,
                    button: PointerButton::Primary,
                });
            }
            if self.ticks == 7 {
                self.app.handle_event(UiEvent::KeyDown {
                    key: "2|Code(Digit2)".into(),
                    ctrl: false,
                    shift: false,
                    alt: false,
                    meta: false,
                });
                self.performance_verified = self.app.project.tracks[self.app.selected_track]
                    .input_patch
                    .as_ref()
                    .is_some_and(|p| p.name.starts_with("002 "))
                    && self.app.chords.mix;
                self.app.apply_action(Action::Keyboard);
                self.app.apply_action(Action::ChordMix);
                self.app.menu = None;
            }
            if self.ticks == 12 {
                self.app.project.tracks[0].notes.clear();
                let id = self.app.project.insert_note_exact(0, 4, 2, 24, 15).unwrap();
                self.app.selected_note = Some(id);
                self.app.open_key_track(0);
            }
            if self.ticks == 14 {
                let r = self.app.note_rect(
                    0,
                    self.app
                        .project
                        .note(self.app.selected_note.unwrap())
                        .unwrap(),
                );
                let position = Point::new(r.right() + 2.0, r.y + r.height / 2.0);
                self.resize_origin = Some(position);
                self.app.handle_event(UiEvent::PointerDown {
                    position,
                    button: PointerButton::Primary,
                });
            }
            if self.ticks == 16 || self.ticks == 18 {
                let origin = self.resize_origin.unwrap();
                let delta = if self.ticks == 16 { 3.0 } else { 4.0 };
                let position = Point::new(origin.x + self.app.half_step_w * delta, origin.y);
                self.app.handle_event(if self.ticks == 16 {
                    UiEvent::PointerMove { position }
                } else {
                    UiEvent::PointerUp {
                        position,
                        button: PointerButton::Primary,
                    }
                });
            }
            if self.ticks == 19 {
                self.resized_length = self
                    .app
                    .selected_note
                    .and_then(|id| self.app.project.note(id))
                    .map(|n| n.length);
            }
            let action = match self.ticks {
                20 => Some(Action::SetLink16Mode(Link16Mode::Cable8)),
                21 => Some(Action::SetMonitor(Some(1))),
                22 => Some(Action::SetLink16Mode(Link16Mode::Dmg07_16)),
                23 => Some(Action::SetMonitor(Some(3))),
                _ => None,
            };
            if let Some(action) = action {
                let toolbar = self
                    .app
                    .link16_buttons()
                    .into_iter()
                    .find(|b| b.action == action);
                let spec = toolbar.or_else(|| {
                    // The monitor is in View when the native desktop is too narrow.
                    self.app.menu = Some(MenuKind::View);
                    self.app
                        .menu_buttons()
                        .into_iter()
                        .find(|b| b.action == action)
                });
                if let Some(spec) = spec {
                    let position = Point::new(spec.rect.x + 16.0, spec.rect.y + 12.0);
                    self.app.handle_event(UiEvent::PointerDown {
                        position,
                        button: PointerButton::Primary,
                    });
                    self.app.handle_event(UiEvent::PointerUp {
                        position,
                        button: PointerButton::Primary,
                    });
                }
                self.monitor_path.push((
                    self.app.project.player_count(),
                    self.app.monitor_player(),
                    self.app.active_player,
                ));
            }
            if self.ticks == 24 {
                self.app.menu = Some(MenuKind::Export);
            }
            if (25..34).contains(&self.ticks) {
                self.app.language = Language::ALL[(self.ticks - 25) as usize];
                self.app.menu = Some(MenuKind::Languages);
                let _ = self.app.draw_list();
            }
            if self.ticks >= 34 && self.draws.get() >= 4 {
                self.app.language = Language::Japanese;
                self.app.menu = Some(MenuKind::Export);
                let length = self.resized_length.unwrap_or(0);
                let player_export = self
                    .app
                    .menu_buttons()
                    .iter()
                    .any(|b| b.action == Action::ExportPlayerRoms);
                let passed = self.performance_verified
                    && length == 6
                    && self.monitor_path
                        == [(2, None, 0), (2, Some(1), 0), (4, None, 0), (4, Some(3), 0)]
                    && self.app.keyboard_rect().width == 88.0
                    && self.app.project.validate().is_ok()
                    && player_export;
                let report = serde_json::json!({
                    "passed": passed, "ticks": self.ticks, "draws": self.draws.get(),
                    "performance_preset_and_chord_ui":self.performance_verified,
                    "viewport": [self.app.size.width, self.app.size.height],
                    "language": "ja", "wave_graph": true, "key_editor": self.app.key_editor,
                    "language_menu_draws": Language::ALL.map(|l|l.code()),
                    "keyboard_width": self.app.keyboard_rect().width,
                    "note_grid_left": self.app.timeline_viewport().x,
                    "synthetic_resize_before": 2, "synthetic_resize_after": length,
                    "resize_input_source": "synthetic UiEvent in native event loop",
                    "monitor_path_count_solo0based_edit0based": self.monitor_path,
                    "font": active_font_source(), "physical_input_tested": false,
                    "audio_output_tested": false
                    ,"sram_free_player_export_menu": player_export
                    ,"gb_studio_export_menus": self.app.menu_buttons().iter().filter(|b| matches!(b.action, Action::ExportStudio(_))).count()
                    ,"file_dialog_tested": false
                });
                if let Some(parent) = self.receipt.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&self.receipt, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
                std::process::exit(if passed { 0 } else { 1 });
            }
            UiResponse::redraw()
        }
    }
    App::new()
        .title("GBSEQ - plita startup verification")
        .window_size(1280, 800)
        .continuous_redraw(true)
        .run_program(move |size| Probe {
            app: wave_fixture(size),
            ticks: 0,
            draws: std::cell::Cell::new(0),
            receipt,
            resize_origin: None,
            monitor_path: Vec::new(),
            resized_length: None,
            performance_verified: false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag(app: &mut GbseqApp, a: Point, b: Point) {
        app.handle_event(UiEvent::PointerDown {
            position: a,
            button: PointerButton::Primary,
        });
        // Deliberately only one move event: every skipped CH3 sample must be filled.
        app.handle_event(UiEvent::PointerMove { position: b });
        app.handle_event(UiEvent::PointerUp {
            position: b,
            button: PointerButton::Primary,
        });
    }

    #[test]
    fn wave_strokes_interpolate_both_directions_and_undo_as_one_edit() {
        let mut app = wave_fixture(Size::new(1280.0, 720.0));
        let before = app.project.clone();
        let undo = app.undo.len();
        let r = app.wave_editor_rect().unwrap();
        let a = Point::new(r.x, r.bottom() - 3.0);
        let b = Point::new(r.x + r.width * 31.0 / 32.0, r.y + 3.0);
        drag(&mut app, a, b);
        let ramp = app.project.user_waves[0].samples.clone();
        assert_eq!(app.undo.len(), undo + 1);
        assert_eq!((ramp[0], ramp[31]), (0, 15));
        for i in 0..32 {
            assert_eq!(ramp[i], (15.0 * i as f32 / 31.0).round() as u8);
        }
        app.undo();
        assert_eq!(app.project, before);
        app.redo();
        assert_eq!(app.project.user_waves[0].samples, ramp);
        // Right-to-left stroke, then pointer outside: values clamp without leaving holes.
        drag(&mut app, b, Point::new(r.x - 20.0, r.y - 20.0));
        assert!(app.project.user_waves[0].samples.iter().all(|v| *v == 15));
        let stored = app.project.clone();
        app.apply_action(Action::WaveCurve);
        app.draw_list();
        assert_eq!(app.project, stored, "curve must never alter stored samples");
        app.detail_open = false;
        assert!(
            app.wave_editor_rect().is_none(),
            "hidden graph cannot accept edits"
        );
    }

    #[test]
    fn drawn_ch3_wave_changes_rendered_audio_without_changing_timing() {
        let mut app = wave_fixture(Size::new(1280.0, 800.0));
        app.project.set_measures(1);
        for track in &mut app.project.tracks {
            track.notes.clear();
        }
        let id = app.project.insert_note_exact(2, 0, 16, 24, 15).unwrap();
        app.selected_note = Some(id);
        let render = |project: &SequenceProject| {
            render_project_to_samples(
                &project.to_gbdaw_player(0).unwrap(),
                StreamMode::Extended,
                30,
            )
            .unwrap()
        };
        let before = render(&app.project);
        let r = app.wave_editor_rect().unwrap();
        drag(
            &mut app,
            Point::new(r.x, r.y + 3.0),
            Point::new(r.right(), r.bottom() - 3.0),
        );
        let after = render(&app.project);
        assert_eq!(before.0, after.0);
        assert_eq!(before.1.len(), after.1.len());
        assert!(after.1.iter().any(|s| *s != 0));
        assert_ne!(before.1, after.1);
    }

    #[test]
    fn key_editor_event_path_add_move_resize_undo_and_link16_tracks() {
        let mut app = GbseqApp::new_with_audio(Size::new(1280.0, 800.0), false);
        app.project = SequenceProject::empty();
        app.project.set_link16_mode(Link16Mode::Dmg07_16);
        app.all_players_view = true;
        // Open P3 CH3 via the actual overview header double-click.
        let header = app.header_lane_rect(10);
        app.handle_event(UiEvent::PointerDoubleClick {
            position: Point::new(header.x + 30.0, header.y + header.height / 2.0),
            button: PointerButton::Primary,
        });
        assert!(app.key_editor);
        assert_eq!(
            (app.visible_track_start(), app.visible_track_count()),
            (10, 1)
        );
        assert!(
            app.all_players_visible(),
            "ALL16 overview is retained for the return from the key editor"
        );
        let point = Point::new(
            app.step_x(4),
            app.timeline_viewport().bottom()
                - (24 - app.key_low) as f32 * app.key_height()
                - app.key_height() / 2.0,
        );
        app.handle_event(UiEvent::PointerDoubleClick {
            position: point,
            button: PointerButton::Primary,
        });
        let id = app.selected_note.unwrap();
        assert_eq!(
            (
                app.project.note(id).unwrap().start,
                app.project.note(id).unwrap().pitch
            ),
            (4, 24)
        );
        let rect = app.note_rect(10, app.project.note(id).unwrap());
        let a = Point::new(rect.x + 3.0, rect.y + rect.height / 2.0);
        let b = Point::new(a.x + app.half_step_w * 2.0, a.y - app.key_height() * 3.0);
        drag(&mut app, a, b);
        assert_eq!(
            (
                app.project.note(id).unwrap().start,
                app.project.note(id).unwrap().pitch
            ),
            (6, 27)
        );
        let rect = app.note_rect(10, app.project.note(id).unwrap());
        let a = Point::new(rect.right() - 2.0, rect.y + 3.0);
        let b = Point::new(a.x + app.half_step_w * 2.0, a.y);
        drag(&mut app, a, b);
        assert_eq!(app.project.note(id).unwrap().length, 4);
        app.undo();
        assert_eq!(app.project.note(id).unwrap().length, 2);
        app.undo();
        assert_eq!(app.project.note(id).unwrap().pitch, 24);
        app.apply_action(Action::KeyTrack(1));
        assert_eq!(app.selected_track, 11);
        assert!(app.selected_note.is_none());
        app.key_low = 24;
        assert_eq!(app.key_pitch(app.timeline_viewport().y), 47);
        app.key_low = 0;
        assert_eq!(app.key_pitch(app.timeline_viewport().bottom()), 0);
        app.handle_key("E", false, false);
        assert!(!app.key_editor);
        assert_eq!(app.visible_track_count(), 16);
        app.project.validate().unwrap();
    }

    fn dump(app: &GbseqApp, path: &Path) {
        let mut commands = Vec::new();
        let mut clip_depth = 0;
        for c in app.draw_list().commands() {
            match c {
                DrawCommand::PushClip { .. } => clip_depth += 1,
                DrawCommand::PopClip => {
                    assert!(clip_depth > 0, "unbalanced GUI clip stack");
                    clip_depth -= 1;
                }
                _ => {}
            }
            let rect = |r: &Rect| serde_json::json!([r.x, r.y, r.width, r.height]);
            commands.push(match c {
                DrawCommand::FillRect { rect: r, color: c } => serde_json::json!({"kind":"rect", "rect":rect(r), "color":[c.r,c.g,c.b,c.a]}),
                DrawCommand::FillRectCorners { rect: r, top_left: a, bottom_left: b, .. } => serde_json::json!({"kind":"gradient","rect":rect(r),"top":[a.r,a.g,a.b,a.a],"bottom":[b.r,b.g,b.b,b.a]}),
                DrawCommand::FillTriangles { vertices } => serde_json::json!({"kind":"triangles", "vertices":vertices.iter().map(|v| [v.position.x,v.position.y,v.color.r,v.color.g,v.color.b,v.color.a]).collect::<Vec<_>>()}),
                DrawCommand::ImageRgba { rect: r, width, height, pixels } => serde_json::json!({"kind":"image","rect":rect(r),"width":width,"height":height,"pixels":pixels.to_vec()}),
                DrawCommand::PushClip { rect: r } => serde_json::json!({"kind":"clip","rect":rect(r)}),
                DrawCommand::PopClip => serde_json::json!({"kind":"pop"}),
                _ => continue,
            });
        }
        assert_eq!(clip_depth, 0);
        fs::write(path, serde_json::to_vec(&serde_json::json!({"width":app.size.width,"height":app.size.height,"commands":commands})).unwrap()).unwrap();
    }

    #[test]
    fn japanese_layout_wave_save_and_link16_export() {
        let out = std::env::var_os("GBSEQ_UI_EVIDENCE").map(PathBuf::from);
        if let Some(out) = &out {
            fs::create_dir_all(out).unwrap();
        }
        for (width, height) in [(1480, 900), (1280, 720)] {
            let mut app = wave_fixture(Size::new(width as f32, height as f32));
            let r = app.wave_editor_rect().unwrap();
            let buttons = app.inspector_buttons();
            for (index, button) in buttons.iter().enumerate() {
                assert!(
                    button.rect.bottom() < r.y - 22.0,
                    "{:?} overlaps wave title",
                    button.action
                );
                assert!(
                    buttons[index + 1..]
                        .iter()
                        .all(|other| !button.rect.intersects(other.rect)),
                    "overlapping inspector hit targets: {:?}",
                    button.action
                );
            }
            app.project.title = "日本語の曲名・波形テスト".into();
            let stored: SequenceProject =
                serde_json::from_str(&serde_json::to_string(&app.project).unwrap()).unwrap();
            assert_eq!(stored, app.project);
            app.language = Language::Japanese;
            if let Some(out) = &out {
                dump(&app, &out.join(format!("wave_ja_{width}.json")));
                app.wave_curve = false;
                dump(&app, &out.join(format!("wave_polyline_ja_{width}.json")));
                app.open_key_track(2);
                dump(&app, &out.join(format!("keys_ja_{width}.json")));
                app.language = Language::English;
                dump(&app, &out.join(format!("keys_en_{width}.json")));
                if width == 1280 {
                    let r = app.wave_editor_rect().unwrap();
                    drag(
                        &mut app,
                        Point::new(r.x, r.y + 3.0),
                        Point::new(r.right(), r.bottom() - 3.0),
                    );
                    app.save_path(&out.join("波形テスト.gbseq")).unwrap();
                    let reloaded =
                        crate::services::load_sequence(&out.join("波形テスト.gbseq")).unwrap();
                    assert_eq!(reloaded, app.project);
                    let manifest = export_link16_saves(&reloaded, &out.join("sav")).unwrap();
                    let sav = fs::read(&manifest.files[0].path).unwrap();
                    let base = crate::link16_sav::LINK16_WAVE_BASE + 33;
                    for i in 0..16 {
                        assert_eq!(
                            sav[base + i],
                            reloaded.user_waves[0].samples[i * 2] << 4
                                | reloaded.user_waves[0].samples[i * 2 + 1]
                        );
                    }
                    crate::services::render_sequence_wav(&reloaded, &out.join("wave_preview.wav"))
                        .unwrap();
                    let mut resize = GbseqApp::new_with_audio(Size::new(1280.0, 720.0), false);
                    resize.project = SequenceProject::empty();
                    resize.project.title = "右端ドラッグ検証".into();
                    resize.language = Language::Japanese;
                    let id = resize.project.insert_note_exact(0, 4, 2, 24, 15).unwrap();
                    resize.project.insert_note_exact(0, 14, 2, 28, 12).unwrap();
                    resize.selected_note = Some(id);
                    resize.detail_open = true;
                    resize.open_key_track(0);
                    dump(&resize, &out.join("resize_before.json"));
                    let r = resize.note_rect(0, resize.project.note(id).unwrap());
                    let p = Point::new(r.right() + 2.0, r.y + r.height / 2.0);
                    let q = Point::new(p.x + resize.half_step_w * 20.0, p.y);
                    drag(&mut resize, p, q);
                    assert_eq!(resize.project.note(id).unwrap().length, 10);
                    dump(&resize, &out.join("resize_after.json"));
                    resize.save_path(&out.join("resize_fixture.gbseq")).unwrap();
                }
            }
        }
    }
}
