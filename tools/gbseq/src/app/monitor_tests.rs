use super::*;

fn fixture(mode: Link16Mode) -> GbseqApp {
    let mut app = GbseqApp::new_with_audio(Size::new(1280.0, 800.0), false);
    app.project = SequenceProject::empty();
    app.project.set_link16_mode(mode);
    app.project.set_measures(1);
    for player in 0..mode.player_count() {
        app.project
            .insert_note_exact(
                player * TRACK_COUNT,
                player as u32 * 2,
                5,
                18 + player as u8 * 5,
                15,
            )
            .unwrap();
    }
    app
}

fn click(app: &mut GbseqApp, action: Action) {
    let button = if app.menu.is_some() {
        app.menu_buttons()
    } else {
        app.all_buttons()
    }
    .into_iter()
    .find(|b| b.action == action)
    .expect("visible button");
    let position = Point::new(
        button.rect.x + button.rect.width / 2.0,
        button.rect.y + button.rect.height / 2.0,
    );
    assert_eq!(app.action_at(position), Some(action));
    app.handle_event(UiEvent::PointerDown {
        position,
        button: PointerButton::Primary,
    });
    app.handle_event(UiEvent::PointerUp {
        position,
        button: PointerButton::Primary,
    });
}

#[test]
fn monitor_switches_at_stereo_boundaries_without_moving_or_restarting() {
    let monitor = Arc::new(AtomicUsize::new(0));
    let mut pcm = SharedAudio {
        position: 0,
        samples: Arc::new(Stems::new(vec![
            vec![10, 20, 30, 40, 50, 60],
            vec![12, 22, 32, 42, 52, 62],
        ])),
        monitor: monitor.clone(),
        frame_monitor: 0,
        rate: 48000,
        looped: true,
    };
    assert_eq!(pcm.next(), Some(11));
    monitor.store(1, Ordering::Relaxed);
    assert_eq!(pcm.next(), Some(21)); // right half still belongs to the previous stereo frame
    assert_eq!([pcm.next(), pcm.next()], [Some(30), Some(40)]);
    monitor.store(2, Ordering::Relaxed);
    assert_eq!([pcm.next(), pcm.next()], [Some(52), Some(62)]);
    assert_eq!([pcm.next(), pcm.next()], [Some(12), Some(22)]); // loops with P2
    monitor.store(0, Ordering::Relaxed);
    assert_eq!([pcm.next(), pcm.next()], [Some(31), Some(41)]);
}

#[test]
fn monitor_is_independent_of_views_and_resets_on_mode_change() {
    let mut app = fixture(Link16Mode::Dmg07_16);
    let before = app.project.clone();
    click(&mut app, Action::SetPlayer(1));
    assert_eq!(app.monitor_player(), None);
    click(&mut app, Action::SetMonitor(Some(3)));
    click(&mut app, Action::ToggleAllPlayers);
    app.open_key_track(4);
    assert_eq!(app.monitor_player(), Some(3));
    assert_eq!(app.active_player, 1);
    assert_eq!(app.project, before);
    assert!(app.undo.is_empty());
    // Narrow layout: choose from View when there is no space for the buttons.
    app.resize(Size::new(900.0, 720.0));
    click(&mut app, Action::Menu(MenuKind::View));
    click(&mut app, Action::SetMonitor(Some(0)));
    assert_eq!(app.monitor_player(), Some(0));
    click(&mut app, Action::SetLink16Mode(Link16Mode::Cable8));
    assert_eq!(app.monitor_player(), None);
    assert_eq!(app.monitor_choices().len(), 3);
    app.apply_action(Action::SetMonitor(Some(2))); // not available for two consoles
    assert_eq!(app.monitor_player(), None);
    click(&mut app, Action::SetLink16Mode(Link16Mode::Local4));
    assert!(app.monitor_choices().is_empty());
    app.undo();
    assert_eq!(app.monitor_player(), None);
}

#[test]
fn gui_mixes_all_players_from_a_single_edit_view_and_exports_current_monitor() {
    for mode in [Link16Mode::Cable8, Link16Mode::Dmg07_16] {
        let mut app = fixture(mode);
        let n = mode.player_count();
        // Also cover a one-shot with different stem lengths and trailing silence.
        app.project.loop_playback = n == 4;
        click(&mut app, Action::SetPlayer(n - 1));
        assert_eq!(app.monitor_player(), None);
        click(&mut app, Action::Play);
        let key = app.render_key.clone();
        click(&mut app, Action::SetMonitor(Some(0)));
        assert_eq!(app.render_key, key); // selection is live even during background rendering
        let deadline = Instant::now() + Duration::from_secs(45);
        while app.render_rx.is_some() {
            assert!(Instant::now() < deadline);
            app.poll_audio_render();
            thread::sleep(Duration::from_millis(2));
        }
        let rate = app.headless_stream.as_ref().unwrap().sample_rate();
        // Reference: independently render each console in isolation, including one
        // that is never the displayed console while Play is rendering.
        let parts = (0..n)
            .map(|p| {
                let project = app.project.to_gbdaw_player(p).unwrap();
                render_project_to_samples(
                    &project,
                    StreamMode::Extended,
                    project_frame_count(&app.project),
                )
                .unwrap()
                .1
            })
            .collect::<Vec<_>>();
        for (i, part) in parts.iter().enumerate() {
            assert!(part.iter().any(|v| *v != 0));
            assert!(parts[..i].iter().all(|other| *other != *part));
        }
        let stems = Arc::new(Stems::new(parts.clone()));
        click(&mut app, Action::SetMonitor(None));
        let expected: Vec<_> = (0..stems.len).map(|i| stems.sample(i, 0)).collect();
        let streamed = app
            .headless_stream
            .as_mut()
            .unwrap()
            .collect_wait(stems.len);
        assert_eq!(
            streamed, expected,
            "{mode:?}: streamed PCM differs from offline mix"
        );
        assert_eq!(
            app.stream_control
                .as_ref()
                .unwrap()
                .stats
                .missing_samples
                .load(Ordering::Relaxed),
            0
        );
        for player in std::iter::once(None).chain((0..n).map(Some)) {
            click(&mut app, Action::SetMonitor(player));
            let expected = (0..stems.len)
                .map(|i| {
                    player.map_or_else(
                        || {
                            (parts
                                .iter()
                                .map(|p| i32::from(p.get(i).copied().unwrap_or(0)))
                                .sum::<i32>()
                                / n as i32) as i16
                        },
                        |p| parts[p].get(i).copied().unwrap_or(0),
                    )
                })
                .collect::<Vec<_>>();
            let output = SharedAudio {
                samples: stems.clone(),
                monitor: app.monitor.clone(),
                frame_monitor: 0,
                position: 0,
                rate,
                looped: false,
            }
            .collect::<Vec<_>>();
            assert_eq!(output, expected);
            let path = std::env::temp_dir().join(format!(
                "gbseq-monitor-{}-{n}-{}.wav",
                std::process::id(),
                player.map_or(0, |p| p + 1)
            ));
            app.start_wav_export(path.clone());
            click(&mut app, Action::SetMonitor(None)); // export must retain its initial selection
            app.export_rx
                .take()
                .unwrap()
                .recv_timeout(Duration::from_secs(45))
                .unwrap()
                .unwrap();
            let wav = fs::read(&path).unwrap();
            fs::remove_file(path).unwrap();
            assert_eq!(&wav[36..40], b"data");
            assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), rate);
            let pcm = wav[44..]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]));
            assert_eq!(
                pcm.len(),
                expected.len(),
                "{mode:?}, {player:?}: WAV duration"
            );
            assert!(
                pcm.eq(expected.iter().copied()),
                "{mode:?}, {player:?}: WAV samples"
            );
        }
    }
}

#[test]
fn monitor_respects_global_track_solo_without_altering_notes() {
    let mut app = fixture(Link16Mode::Cable8);
    app.project.tracks[4].solo = true;
    let before = app.project.clone();
    let mut silent = before.clone();
    for track in &mut silent.tracks {
        track.notes.clear();
    }
    let render = |p: &gbdaw_model::Project| {
        render_project_to_samples(p, StreamMode::Extended, 32)
            .unwrap()
            .1
    };
    let silence = render(&silent.to_gbdaw_player(0).unwrap());
    let sources = crate::services::playback_projects(&app.project, None).unwrap();
    assert_eq!(render(&sources[0]), silence);
    assert_eq!(
        render(&sources[1]),
        render(&before.to_gbdaw_player(1).unwrap())
    );
    assert_ne!(render(&sources[1]), silence);
    assert_eq!(app.project, before);
    app.project.tracks[4].mute = true;
    let sources = crate::services::playback_projects(&app.project, None).unwrap();
    assert!(sources.iter().all(|s| render(s) == silence));
}

#[test]
#[ignore = "requires a native audio output device"]
fn live_output_keeps_the_transport_while_switching_consoles() {
    for mode in [Link16Mode::Cable8, Link16Mode::Dmg07_16] {
        let mut app = fixture(mode);
        app.project.loop_playback = true;
        app.audio = Some(AudioPlayer::new().expect("native output required for this test"));
        click(&mut app, Action::Play);
        let deadline = Instant::now() + Duration::from_secs(45);
        while app.play_started.is_none() {
            assert!(Instant::now() < deadline, "{}", app.status);
            app.poll_audio_render();
            thread::sleep(Duration::from_millis(5));
        }
        let started = app.play_started;
        for p in (0..app.project.player_count())
            .map(Some)
            .chain(std::iter::once(None))
        {
            let pos = app.audio.as_ref().unwrap().position().unwrap();
            click(&mut app, Action::SetMonitor(p));
            thread::sleep(Duration::from_millis(85));
            assert_eq!(app.play_started, started);
            assert!(app.render_rx.is_none());
            let end = app.audio.as_ref().unwrap().position().unwrap();
            assert!(end > pos, "audio sink did not advance");
            let timing = app.playback_timing.unwrap();
            let (shown, total) = app.transport_times();
            let after = app.audio.as_ref().unwrap().position().unwrap();
            assert_eq!(total, timing.duration);
            let lo = timing.position_at(end);
            let hi = timing.position_at(after);
            assert!(if hi >= lo {
                lo <= shown && shown <= hi
            } else {
                shown >= lo || shown <= hi
            });
            println!("{mode:?} monitor {p:?}: {pos:?} -> {end:?}");
        }
        assert_eq!(
            app.stream_control
                .as_ref()
                .unwrap()
                .stats
                .missing_samples
                .load(Ordering::Relaxed),
            0
        );
        app.stop_audio();
    }
}

#[test]
fn mute_solo_and_undo_update_live_control_without_restarting() {
    let mut app = fixture(Link16Mode::Cable8);
    app.start_playback();
    let cancel = app.render_cancel.as_ref().unwrap().clone();
    app.apply_action(Action::ToggleSolo(4));
    assert_eq!(
        app.stream_control
            .as_ref()
            .unwrap()
            .audible
            .load(Ordering::Relaxed),
        1 << 4
    );
    app.apply_action(Action::ToggleSolo(0));
    assert_eq!(
        app.stream_control
            .as_ref()
            .unwrap()
            .audible
            .load(Ordering::Relaxed),
        17
    );
    app.apply_action(Action::ToggleMute(4));
    assert_eq!(
        app.stream_control
            .as_ref()
            .unwrap()
            .audible
            .load(Ordering::Relaxed),
        1
    );
    app.undo();
    assert_eq!(
        app.stream_control
            .as_ref()
            .unwrap()
            .audible
            .load(Ordering::Relaxed),
        17
    );
    app.redo();
    assert_eq!(
        app.stream_control
            .as_ref()
            .unwrap()
            .audible
            .load(Ordering::Relaxed),
        1
    );
    assert!(!cancel.load(Ordering::Relaxed));
    app.stop_audio();
    assert!(cancel.load(Ordering::Relaxed));
}

#[test]
#[ignore = "requires a native audio output device"]
fn live_long_16ch_stays_bounded_during_editor_and_monitor_actions() {
    let mut app = fixture(Link16Mode::Dmg07_16);
    app.project.measures = 256;
    app.project.loop_playback = true;
    for ch in 0..16 {
        app.project.tracks[ch].notes.clear();
        for step in (0..4096).step_by(4) {
            let id = app
                .project
                .insert_note_exact(ch, step, 3, (ch * 3 % 48) as u8, 11)
                .unwrap();
            let fx = &mut app.project.note_mut(id).unwrap().effects;
            fx.noise_shift = 1;
            fx.noise_divisor = 1;
            fx.wave_preset = 1;
        }
    }
    app.audio = Some(AudioPlayer::new().expect("native output required"));
    let begin = Instant::now();
    app.start_playback();
    let until = begin + Duration::from_secs(20);
    while app.play_started.is_none() {
        app.tick();
        assert!(Instant::now() < until, "{}", app.status);
        thread::sleep(Duration::from_millis(2));
    }
    let startup = begin.elapsed();
    let started = app.play_started;
    for step in 0..100 {
        if step % 10 == 0 {
            app.apply_action(Action::SetMonitor(Some(step / 10 % 4)));
        }
        if step % 15 == 0 {
            app.apply_action(Action::ToggleMute(5));
        }
        if step % 17 == 0 {
            app.apply_action(Action::ToggleSolo(9));
        }
        if step % 23 == 0 {
            app.undo();
            app.redo();
        }
        app.draw_list();
        app.tick();
        assert_eq!(app.play_started, started);
        assert!(app.render_rx.is_none());
        thread::sleep(Duration::from_millis(10));
    }
    let stats = app.stream_control.as_ref().unwrap().stats.clone();
    let played = stats.consumed_samples.load(Ordering::Relaxed);
    let generated = stats.generated_frames.load(Ordering::Relaxed);
    let consumed_frames = played as f64 / (2.0 * 65536.0) * GB_FRAME_RATE_HZ;
    let ahead = generated as f64 - consumed_frames;
    let missing = stats.missing_samples.load(Ordering::Relaxed);
    println!("16CH / 256 bars / 16384 notes: startup={startup:?}, generated={generated} GB frames, consumed={played} samples, ahead={ahead:.2} frames, missing={missing}");
    assert!(ahead <= 13.0, "unbounded synthesis lead");
    assert_eq!(missing, 0, "native output starved");
    assert!(app.transport_times().0 > Duration::from_millis(500));
    app.stop_audio();
    let until = Instant::now() + Duration::from_secs(2);
    while !stats.worker_finished.load(Ordering::Acquire) {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
#[ignore = "requires a native audio output device"]
fn live_clock_finishes_at_pcm_length_and_restarts_a_fresh_stream() {
    let mut app = fixture(Link16Mode::Local4);
    app.project.loop_playback = false;
    app.project.frames_per_step_override = Some(3);
    app.audio = Some(AudioPlayer::new().expect("native output required for this test"));
    for pass in 0..2 {
        app.start_playback();
        let deadline = Instant::now() + Duration::from_secs(12);
        let mut total = Duration::ZERO;
        let mut advanced = false;
        loop {
            app.tick();
            if app.finished_duration.is_some() {
                break;
            }
            if app.play_started.is_some() {
                let (now, end) = app.transport_times();
                total = end;
                advanced |= now >= Duration::from_millis(100);
                assert!(now <= end);
            }
            assert!(Instant::now() < deadline, "{}", app.status);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(advanced);
        assert_eq!(app.transport_times(), (total, total));
        println!(
            "pass {pass}: final clock {} / {}",
            transport_stamp(total),
            transport_stamp(total)
        );
    }
}
