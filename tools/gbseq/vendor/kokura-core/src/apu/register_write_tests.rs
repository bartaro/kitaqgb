use super::*;

const CHANNEL_REGS: [(u16, u16, u16); 4] = [
    (0xFF11, 0xFF12, 0xFF14),
    (0xFF16, 0xFF17, 0xFF19),
    (0xFF1B, 0xFF1A, 0xFF1E),
    (0xFF20, 0xFF21, 0xFF23),
];

fn active(index: usize, cgb: bool, envelope: u8) -> Apu {
    let mut apu = Apu::default();
    apu.set_cgb_mode(cgb);
    apu.write(0xFF26, 0x80);
    apu.write(
        CHANNEL_REGS[index].1,
        if index == 2 { 0x80 } else { envelope },
    );
    apu.write(CHANNEL_REGS[index].2, 0x80);
    apu
}

#[test]
fn length_enable_edges_follow_all_eight_sequencer_phases() {
    for cgb in [false, true] {
        for index in 0..4 {
            for phase in 0..8 {
                let mut apu = active(index, cgb, 0xF0);
                let control = CHANNEL_REGS[index].2;
                apu.frame_seq_step = phase;
                apu.channels[index].length_counter = 2;
                apu.write(control, 0x40);
                let expected = if phase & 1 == 0 { 1 } else { 2 };
                assert_eq!(apu.channels[index].length_counter, expected);
                apu.write(control, 0x40);
                apu.write(control, 0);
                assert_eq!(apu.channels[index].length_counter, expected);
            }
        }
    }
}

#[test]
fn length_enable_expiry_disables_but_trigger_reloads_without_false_expiry() {
    for index in 0..4 {
        for trigger in [false, true] {
            let mut apu = active(index, false, 0xF0);
            apu.frame_seq_step = 0;
            apu.channels[index].length_counter = 1;
            let trace = apu.write(CHANNEL_REGS[index].2, if trigger { 0xC0 } else { 0x40 });
            assert_eq!(apu.channels[index].enabled, trigger);
            assert_eq!(apu.channel_enable & (1 << index) != 0, trigger);
            assert_eq!(
                apu.channels[index].length_counter,
                if trigger {
                    max_length(index as u8 + 1) - 1
                } else {
                    0
                }
            );
            assert_eq!(
                trace
                    .iter()
                    .any(|event| matches!(event, ApuTraceEvent::ChannelLengthExpired { .. })),
                !trigger
            );
        }
    }
}

#[test]
fn zero_length_trigger_reload_obeys_phase_and_enable_without_double_clock() {
    for cgb in [false, true] {
        for index in 0..4 {
            for phase in 0..8 {
                for enabled in [false, true] {
                    for old_enabled in [false, true] {
                        let mut apu = active(index, cgb, 0xF0);
                        let control = CHANNEL_REGS[index].2;
                        apu.write(control, if old_enabled { 0x40 } else { 0 });
                        apu.frame_seq_step = phase;
                        apu.channels[index].length_counter = 0;
                        apu.write(control, if enabled { 0xC0 } else { 0x80 });
                        let extra = u16::from(enabled && phase & 1 == 0);
                        assert_eq!(
                            apu.channels[index].length_counter,
                            max_length(index as u8 + 1) - extra
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn disabled_channels_still_reload_and_clock_enabled_length() {
    for index in 0..4 {
        let mut apu = active(index, false, 0xF0);
        let (_, dac, control) = CHANNEL_REGS[index];
        apu.write(dac, 0);
        apu.channels[index].length_counter = 0;
        apu.write(control, 0xC0);
        let length = max_length(index as u8 + 1);
        assert_eq!(apu.channels[index].length_counter, length);
        apu.clock_length(&mut Vec::new());
        assert_eq!(apu.channels[index].length_counter, length - 1);
        assert!(!apu.channels[index].enabled);
    }
}

#[test]
fn live_envelope_write_does_not_reload_initial_volume_or_countdown() {
    for cgb in [false, true] {
        for index in [0, 1, 3] {
            let mut apu = active(index, cgb, 0x59);
            apu.channels[index].envelope_counter = 3;
            apu.write(CHANNEL_REGS[index].1, 0xDB);
            assert_eq!(apu.channels[index].envelope_volume, 5);
            assert_eq!(apu.channels[index].envelope_counter, 3);
            apu.clock_envelopes(&mut Vec::new());
            apu.clock_envelopes(&mut Vec::new());
            assert_eq!(apu.channels[index].envelope_volume, 5);
            apu.clock_envelopes(&mut Vec::new());
            assert_eq!(apu.channels[index].envelope_volume, 6);
            apu.write(CHANNEL_REGS[index].2, 0x80);
            assert_eq!(apu.channels[index].envelope_volume, 13);
        }
    }
}

#[test]
fn envelope_trigger_before_envelope_tick_adds_one_to_reload() {
    for index in [0, 1, 3] {
        for phase in 0..8 {
            for period in 0..8 {
                let mut apu = active(index, false, 0x58 | period);
                apu.frame_seq_step = phase;
                apu.write(CHANNEL_REGS[index].2, 0x80);
                let expected = if period == 0 { 8 } else { period } + u8::from(phase == 6);
                assert_eq!(apu.channels[index].envelope_counter, expected);
            }
        }
    }
}

#[test]
fn zero_envelope_pace_counts_as_eight_without_automatic_volume_change() {
    let mut apu = active(1, true, 0x58);
    for ticks in 1..=8 {
        apu.clock_envelopes(&mut Vec::new());
        assert_eq!(
            apu.channels[1].envelope_counter,
            if ticks == 8 { 8 } else { 8 - ticks }
        );
        assert_eq!(apu.channels[1].envelope_volume, 5);
    }
}

#[test]
fn cgb_live_envelope_quirks_use_old_pace_direction_and_running_state() {
    for index in [0, 1, 3] {
        for (old, new, expected) in [
            (0x58, 0x08, 6),
            (0x50, 0x08, 10),
            (0x51, 0x09, 9),
            (0x51, 0x11, 7),
            (0x59, 0x11, 11),
            (0x59, 0xE9, 5),
        ] {
            let mut apu = active(index, true, old);
            apu.write(CHANNEL_REGS[index].1, new);
            assert_eq!(
                apu.channels[index].envelope_volume,
                expected,
                "channel {} old {old:02X} new {new:02X}",
                index + 1
            );
        }
    }
}

#[test]
fn exhausted_envelope_stays_stopped_after_live_write_until_retrigger() {
    for index in [0, 1, 3] {
        let mut apu = active(index, true, 0xE9);
        apu.clock_envelopes(&mut Vec::new());
        assert_eq!(apu.channels[index].envelope_volume, 15);
        apu.clock_envelopes(&mut Vec::new());
        apu.write(CHANNEL_REGS[index].1, 0xA1);
        assert_eq!(apu.channels[index].envelope_volume, 1);
        for _ in 0..4 {
            apu.clock_envelopes(&mut Vec::new());
        }
        assert_eq!(apu.channels[index].envelope_volume, 1);
        apu.write(CHANNEL_REGS[index].2, 0x80);
        apu.clock_envelopes(&mut Vec::new());
        assert_eq!(apu.channels[index].envelope_volume, 9);
    }
}

#[test]
fn envelope_dac_off_immediately_disables_and_write_does_not_reenable() {
    for index in [0, 1, 3] {
        let mut apu = active(index, true, 0x59);
        apu.write(CHANNEL_REGS[index].1, 7);
        assert!(!apu.channels[index].enabled);
        apu.write(CHANNEL_REGS[index].1, 0xF1);
        assert!(!apu.channels[index].enabled);
        assert_eq!(apu.channel_enable & (1 << index), 0);
    }
}

#[test]
fn sweep_cannot_be_enabled_by_nr10_without_trigger() {
    let mut apu = active(0, false, 0xF0);
    apu.write(0xFF14, 0x82);
    apu.write(0xFF10, 0x12);
    for _ in 0..16 {
        apu.clock_sweep(&mut Vec::new());
    }
    assert_eq!(apu.current_frequency(1), 512);
    assert!(apu.channels[0].enabled);
    apu.write(0xFF14, 0x82);
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 640);
}

#[test]
fn sweep_pace_write_preserves_countdown_then_reloads_new_pace() {
    let mut apu = active(0, false, 0xF0);
    apu.write(0xFF10, 0x32);
    apu.write(0xFF14, 0x82);
    apu.clock_sweep(&mut Vec::new());
    apu.write(0xFF10, 0x12);
    assert_eq!(apu.channels[0].sweep_counter, 2);
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 512);
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 640);
    assert_eq!(apu.channels[0].sweep_counter, 1);
}

#[test]
fn pulse_retrigger_preserves_low_two_period_timer_bits() {
    for index in [0, 1] {
        for low_bits in 0..4 {
            let mut apu = active(index, false, 0xF0);
            apu.channels[index].freq_timer = 16 + low_bits;
            apu.write(CHANNEL_REGS[index].2, 0x87);
            assert_eq!(apu.channels[index].freq_timer, 1024 + low_bits);
        }
    }
}

#[test]
fn zero_sweep_pace_pauses_changes_without_clearing_trigger_latch() {
    let mut apu = active(0, false, 0xF0);
    apu.write(0xFF10, 0x12);
    apu.write(0xFF14, 0x82);
    apu.write(0xFF10, 0);
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.channels[0].sweep_counter, 8);
    assert_eq!(apu.current_frequency(1), 512);
    apu.write(0xFF10, 0x12);
    for _ in 0..8 {
        apu.clock_sweep(&mut Vec::new());
    }
    assert_eq!(apu.current_frequency(1), 640);
}

#[test]
fn stopped_cgb_envelope_does_not_use_running_zero_pace_zombie_rule() {
    let mut apu = active(1, true, 0x11);
    apu.clock_envelopes(&mut Vec::new());
    apu.clock_envelopes(&mut Vec::new());
    apu.write(0xFF17, 0x10);
    assert_eq!(apu.channels[1].envelope_volume, 2);
    apu.write(0xFF17, 0x18);
    assert_eq!(apu.channels[1].envelope_volume, 12);
}

#[test]
fn control_latches_survive_state_roundtrip_without_changing_layout() {
    let mut apu = active(0, true, 0xF9);
    apu.clock_envelopes(&mut Vec::new());
    apu.write(0xFF10, 0x12);
    apu.write(0xFF14, 2);
    apu.write(0xFF12, 0xA1);
    let encoded = bincode::serialize(&apu).unwrap();
    let mut restored: Apu = bincode::deserialize(&encoded).unwrap();
    for _ in 0..16 {
        restored.clock_envelopes(&mut Vec::new());
        restored.clock_sweep(&mut Vec::new());
    }
    assert_eq!(restored.channels[0].envelope_volume, 1);
    assert_eq!(restored.current_frequency(1), 512);
    assert!(restored.channels[0].enabled);
    let mut legacy = apu.clone();
    legacy.channels[0].wave_sample_buffer = 0;
    assert_eq!(encoded.len(), bincode::serialize(&legacy).unwrap().len());
    restored.write(0xFF26, 0);
    assert_eq!(restored.channels[0].wave_sample_buffer, 0);
}

#[test]
fn legacy_control_latches_are_inferred_before_live_nr10_mutation() {
    for originally_enabled in [false, true] {
        let mut apu = active(0, false, 0x59);
        apu.write(0xFF10, if originally_enabled { 0x12 } else { 0 });
        apu.write(0xFF14, 0x82);
        apu.channels[0].wave_sample_buffer = 0;
        let bytes = bincode::serialize(&apu).unwrap();
        let mut restored: Apu = bincode::deserialize(&bytes).unwrap();
        restored.write(0xFF10, 0x12);
        restored.clock_envelopes(&mut Vec::new());
        assert_eq!(restored.channels[0].envelope_volume, 6);
        for _ in 0..8 {
            restored.clock_sweep(&mut Vec::new());
        }
        assert_eq!(restored.current_frequency(1) != 512, originally_enabled);
    }
}

#[test]
fn live_control_writes_keep_fast_and_traced_pcm_identical() {
    let mut fast = active(0, true, 0x59);
    fast.write(0xFF24, 0x77);
    fast.write(0xFF25, 0xFF);
    let mut traced = fast.clone();
    for (address, value, cycles) in [
        (0xFF10, 0x12, 8192),
        (0xFF11, 0xBF, 7),
        (0xFF14, 0xC2, 8192 * 6),
        (0xFF12, 0x51, 8192 * 8),
        (0xFF14, 0x82, 31),
        (0xFF12, 0x08, 8192 * 8),
        (0xFF10, 0x19, 8192 * 8),
        (0xFF10, 0x11, 127),
    ] {
        fast.write(address, value);
        traced.write(address, value);
        fast.tick_fast(cycles);
        traced.tick(cycles);
        assert_eq!(
            fast.drain_interleaved_i16(usize::MAX),
            traced.drain_interleaved_i16(usize::MAX)
        );
        assert_eq!(
            bincode::serialize(&fast).unwrap(),
            bincode::serialize(&traced).unwrap()
        );
    }
}
