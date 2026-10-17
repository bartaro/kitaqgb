use gbdaw_export_kitaqgb::{export_project, ExportPolicy, StreamMode};
use gbdaw_kokura::ProjectAudioRenderer;
use gbseq::{model::PanPosition, Link16Mode, SequenceProject};

// Exercise GBSEQ conversion, command generation and the actual APU together.
// Equal-looking preview and ROM output is insufficient if both swap CH3/CH4.
#[test]
fn all_players_target_physical_channels_for_note_pan_and_stop() {
    for player in 0..4 {
        for channel in 0..4 {
            let mut project = SequenceProject::empty();
            project.set_link16_mode(Link16Mode::Dmg07_16);
            project.loop_playback = false;
            project.measures = 1;
            project.frames_per_step_override = Some(1);
            let id = project
                .insert_note_exact(player * 4 + channel, 0, 2, 24, 12)
                .unwrap();
            let note = project.note_mut(id).unwrap();
            note.effects.pan = PanPosition::Left;
            note.effects.envelope_period = 0;
            note.effects.wave_preset = 1;
            note.effects.noise_shift = 5;
            note.effects.noise_divisor = 1;
            let converted = project.to_gbdaw_player(player).unwrap();
            let exported =
                export_project(&converted, StreamMode::Extended, ExportPolicy::default()).unwrap();
            let channel_id = channel as u8;
            let value = if channel == 3 { 0x51 } else { 24 };
            for (cmd, frame, args) in [
                ("NOTE", 0, vec![channel_id, value]),
                ("SET_PAN", 0, vec![channel_id, 1]),
                ("STOP_CHANNEL", 2, vec![channel_id]),
            ] {
                assert!(
                    exported.preview.events.iter().any(|event| event.cmd == cmd
                        && event.frame == frame
                        && event.args == args),
                    "P{} CH{}: missing {cmd}",
                    player + 1,
                    channel + 1
                );
            }
            assert!(exported
                .bytes
                .windows(3)
                .any(|b| b == [0x10, channel_id, value]));
            if channel != 2 {
                assert!(exported
                    .preview
                    .events
                    .iter()
                    .any(|event| event.cmd == "SET_INST"
                        && event.args[0] == channel_id
                        && event.args[2] == 0xC0));
            }

            let mut renderer = ProjectAudioRenderer::new(&converted, StreamMode::Extended).unwrap();
            let samples = renderer.render_next_frames(1).unwrap();
            assert!(samples.chunks_exact(2).any(|s| s[0] != 0));
            // The host enables the CH3 DAC at reset; the analog mixer can
            // therefore emit a DC/HPF transient even on the unused side.
            // Check that audio is asymmetric, and verify the exact routing
            // bits below instead of requiring bit-perfect digital silence.
            assert!(samples.chunks_exact(2).any(|s| s[0] != s[1]));
            let active = renderer.snapshot();
            assert_eq!(active.nr52 & 15, 1 << channel);
            assert_eq!(active.nr51 & (0x11 << channel), 0x10 << channel);
            if channel == 3 {
                assert_eq!(active.nr41_to_nr44[2], 0x51);
            }
            renderer.render_next_frames(2).unwrap();
            let stopped = renderer.snapshot();
            assert_eq!(stopped.nr52 & 15, 0);
            assert!(renderer.is_finished());
            if channel == 2 {
                // Active wave RAM reads can expose the playback latch, not
                // the requested address. Read the stored bytes after STOP.
                assert_eq!(
                    stopped.wave_ram,
                    [
                        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC,
                        0xDD, 0xEE, 0xFF, 0,
                    ]
                );
            }
        }
    }
}
