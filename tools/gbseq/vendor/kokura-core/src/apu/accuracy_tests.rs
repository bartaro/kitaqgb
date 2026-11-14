use super::*;

fn pulse(channel: u8, sweep: u8, frequency: u16) -> Apu {
    let mut apu = Apu::default();
    apu.write(0xFF26, 0x80);
    apu.write(0xFF10, sweep);
    let base = if channel == 1 { 0xFF11 } else { 0xFF16 };
    apu.write(base, 0x80);
    apu.write(base + 1, 0xF0);
    apu.write(base + 2, frequency as u8);
    apu.write(base + 3, 0x80 | (frequency >> 8) as u8);
    apu
}

#[test]
fn retrigger_preserves_pulse_duty_phase_and_live_output() {
    for channel in [1, 2] {
        let mut apu = pulse(channel, 0, 1750);
        let index = usize::from(channel - 1);
        apu.channels[index].duty_position = 5;
        apu.channels[index].force_zero_until_advance = false;
        let previous = apu.channel_digital_output(channel);
        assert_eq!(previous, 15);
        apu.write(if channel == 1 { 0xFF14 } else { 0xFF19 }, 0x86);
        assert_eq!(apu.channels[index].duty_position, 5);
        assert_eq!(apu.channel_digital_output(channel), previous);
    }
}

#[test]
fn first_pulse_trigger_is_suppressed_until_its_first_duty_step() {
    let mut apu = pulse(1, 0, 2047);
    assert_eq!(apu.channel_digital_output(1), 0);
    apu.tick_fast(4);
    assert!(!apu.channels[0].force_zero_until_advance);
    assert_eq!(apu.channels[0].duty_position, 1);
    apu.write(0xFF26, 0);
    apu.write(0xFF26, 0x80);
    apu.write(0xFF11, 0x80);
    apu.write(0xFF12, 0xF0);
    apu.write(0xFF14, 0x80);
    assert_eq!(apu.channel_digital_output(1), 0);
}

#[test]
fn zero_sweep_pace_does_not_change_pitch_after_eight_ticks() {
    let mut apu = pulse(1, 0x01, 512);
    apu.tick_fast(8192 * 32);
    assert_eq!(apu.current_frequency(1), 512);
    assert_eq!(apu.channel_enable & 1, 1);
}

#[test]
fn period_register_writes_do_not_replace_the_sweep_shadow() {
    let mut apu = pulse(1, 0x12, 512);
    apu.write(0xFF13, 0x63);
    apu.write(0xFF14, 0x02);
    assert_eq!(apu.current_frequency(1), 0x263);
    assert_eq!(apu.channels[0].shadow_frequency, 512);
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 640);
}

#[test]
fn sweep_update_keeps_the_running_period_divider_phase() {
    let mut apu = pulse(1, 0x12, 512);
    apu.channels[0].freq_timer = 19;
    apu.clock_sweep(&mut Vec::new());
    assert_eq!(apu.current_frequency(1), 640);
    assert_eq!(apu.channels[0].freq_timer, 19);
}

#[test]
fn zero_shift_sweep_still_checks_overflow_without_writing_pitch() {
    for (frequency, active) in [(500, true), (1200, false)] {
        let mut apu = pulse(1, 0x10, frequency);
        assert!(apu.channels[0].enabled);
        apu.clock_sweep(&mut Vec::new());
        assert_eq!(apu.channels[0].enabled, active);
        assert_eq!(apu.current_frequency(1), frequency);
    }
}

#[test]
fn live_zero_pace_increasing_envelope_supports_zombie_volume_steps() {
    for (channel, envelope, trigger) in [
        (1u8, 0xFF12, 0xFF14),
        (2, 0xFF17, 0xFF19),
        (4, 0xFF21, 0xFF23),
    ] {
        let mut apu = Apu::default();
        apu.write(0xFF26, 0x80);
        apu.write(envelope, 0x58);
        apu.write(trigger, 0x80);
        for writes in 1..=16 {
            apu.write(envelope, 0x08);
            assert_eq!(
                apu.channels[usize::from(channel - 1)].envelope_volume,
                (5 + writes) & 15
            );
            assert!(apu.channels[usize::from(channel - 1)].dac_enabled);
        }
    }
}

#[test]
fn powered_off_mixer_and_envelope_registers_ignore_writes() {
    for cgb in [false, true] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        apu.write(0xFF12, 0xF3);
        apu.write(0xFF24, 0x77);
        apu.write(0xFF25, 0xFF);
        apu.write(0xFF14, 0x80);
        apu.write(0xFF30, 0xAB);
        apu.write(0xFF26, 0x80);
        assert_eq!(apu.read(0xFF12), 0);
        assert_eq!(apu.read(0xFF24), 0);
        assert_eq!(apu.read(0xFF25), 0);
        assert_eq!(apu.read(0xFF30), 0xAB);
        assert_eq!(apu.channel_enable, 0);
    }
}

#[test]
fn only_dmg_accepts_length_writes_while_powered_off() {
    for cgb in [false, true] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        for (register, value, index) in [
            (0xFF11, 0xFF, 0),
            (0xFF16, 0xFF, 1),
            (0xFF1B, 0xFF, 2),
            (0xFF20, 0xFF, 3),
        ] {
            apu.write(register, value);
            assert_eq!(apu.channels[index].length_counter, if cgb { 0 } else { 1 });
        }
        assert_eq!(apu.read(0xFF11), 0x3F);
    }
}

#[test]
fn apu_registers_apply_hardware_read_masks() {
    let masks = [
        0x80, 0x3F, 0, 0xFF, 0xBF, 0xFF, 0x3F, 0, 0xFF, 0xBF, 0x7F, 0xFF, 0x9F, 0xFF, 0xBF, 0xFF,
        0xFF, 0, 0, 0xBF, 0, 0,
    ];
    let mut apu = Apu::default();
    apu.write(0xFF26, 0x80);
    for (index, mask) in masks.into_iter().enumerate() {
        let address = 0xFF10 + index as u16;
        apu.write(address, 0);
        assert_eq!(apu.read(address), mask, "register {address:04X}");
    }
}

#[test]
fn high_pass_coefficient_accounts_for_all_clocks_in_a_pcm_sample() {
    for (cgb, per_clock) in [(false, 0.999_958_f64), (true, 0.998_943_f64)] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        assert!((apu.hpf_charge_factor() - per_clock.powi(64)).abs() < 1e-12);
    }
}

#[test]
fn high_pass_rejects_dc_more_quickly_on_cgb_than_dmg() {
    let mut tail = Vec::new();
    for cgb in [false, true] {
        let mut apu = Apu::default();
        apu.set_cgb_mode(cgb);
        apu.write(0xFF26, 0x80);
        apu.channels[0].dac_enabled = true;
        let mut sample = Vec::new();
        for _ in 0..120 {
            apu.pcm_accum_left = 32 * i32::from(APU_PCM_SAMPLE_CYCLES);
            apu.pcm_accum_right = apu.pcm_accum_left;
            apu.pcm_accum_cycles = APU_PCM_SAMPLE_CYCLES;
            apu.enqueue_pcm_frame(&mut 0);
            sample = apu.drain_interleaved_i16(1);
        }
        tail.push(sample[0].abs());
    }
    assert!(tail[0] > 1000 && tail[0] < 1800);
    assert!(tail[1] < 2);
}

#[test]
fn filtered_pulse_tone_remains_audible_and_stereo_balanced() {
    let mut apu = pulse(1, 0, 1750);
    apu.set_cgb_mode(true);
    apu.write(0xFF24, 0x77);
    apu.write(0xFF25, 0x11);
    apu.tick_fast(4_194_304 / 8);
    let pcm = apu.drain_interleaved_i16(usize::MAX);
    let frames: Vec<_> = pcm.chunks_exact(2).skip(1024).collect();
    assert!(frames.iter().all(|frame| frame[0] == frame[1]));
    let rms = (frames
        .iter()
        .map(|frame| f64::from(frame[0]).powi(2))
        .sum::<f64>()
        / frames.len() as f64)
        .sqrt();
    assert!(rms > 1000. && rms < 15000., "pulse RMS {rms}");
}

#[test]
fn traced_and_fast_audio_paths_produce_identical_pcm() {
    let mut traced = pulse(1, 0x12, 512);
    traced.write(0xFF24, 0x77);
    traced.write(0xFF25, 0x11);
    let mut fast = traced.clone();
    traced.tick(123_456);
    fast.tick_fast(123_456);
    assert_eq!(
        traced.drain_interleaved_i16(usize::MAX),
        fast.drain_interleaved_i16(usize::MAX)
    );
}

fn noise(nr43: u8) -> Apu {
    let mut apu = Apu::default();
    apu.write(0xFF26, 0x80);
    apu.write(0xFF24, 0x77);
    apu.write(0xFF25, 0x88);
    apu.write(0xFF21, 0xF0);
    apu.write(0xFF22, nr43);
    apu.write(0xFF23, 0x80);
    apu
}

#[test]
fn noise_long_period_does_not_wrap_at_sixteen_bits() {
    let mut apu = noise(0xD7);
    // 112 * 2^13 T-cycles must elapse before the first shift.
    apu.tick_fast(917_503);
    assert_eq!(apu.channels[3].lfsr, 0x7FFF);
    apu.tick_fast(1);
    assert_eq!(apu.channels[3].lfsr, 0x3FFF);
}

#[test]
fn pcm_does_not_depend_on_cpu_tick_partitioning() {
    for mut apu in [pulse(1, 0, 2047), noise(0x00), noise(0x08)] {
        apu.write(0xFF25, 0xFF);
        apu.write(0xFF24, 0x77);
        let mut single_cycles = apu.clone();
        let mut instruction_cycles = apu.clone();
        apu.tick_fast(49_152);
        for _ in 0..49_152 {
            single_cycles.tick_fast(1);
        }
        for _ in 0..1024 {
            for cycles in [4, 8, 12, 24] {
                instruction_cycles.tick(cycles);
            }
        }
        let expected = single_cycles.drain_interleaved_i16(usize::MAX);
        assert_eq!(apu.drain_interleaved_i16(usize::MAX), expected);
        assert_eq!(
            instruction_cycles.drain_interleaved_i16(usize::MAX),
            expected
        );
    }
}

#[test]
fn powered_off_audio_preserves_sample_clock_and_queued_sound() {
    let mut apu = noise(0x03);
    apu.tick_fast(100);
    let queued = apu.pcm_buffer.clone();
    apu.write(0xFF26, 0);
    assert_eq!(apu.pcm_buffer, queued);
    apu.tick_fast(28 + 64 * 10);
    assert_eq!(apu.generated_frames(), 12);
    let pcm = apu.drain_interleaved_i16(usize::MAX);
    assert_eq!(pcm.len(), 24);
    assert!(pcm[4..].iter().all(|sample| *sample == 0));
    apu.write(0xFF26, 0x80);
    apu.tick(64);
    assert_eq!(apu.generated_frames(), 13);
}

#[test]
fn noise_lfsr_matches_documented_xnor_output_in_both_widths() {
    for short in [false, true] {
        let mut apu = noise(if short { 8 } else { 0 });
        let mut reference = 0u16;
        for _ in 0..32_767 {
            let feedback = (!(reference ^ (reference >> 1))) & 1;
            reference = (reference & 0x7FFF) | (feedback << 15);
            if short {
                reference = (reference & !0x80) | (feedback << 7);
            }
            reference >>= 1;
            apu.advance_noise(8);
            assert_eq!(apu.channel_digital_output(4), ((reference & 1) * 15) as u8);
        }
    }
}

#[test]
fn all_noise_register_settings_have_the_expected_clock_period() {
    for nr43 in 0..=255u8 {
        let mut apu = noise(nr43);
        let divisor = if nr43 & 7 == 0 {
            8
        } else {
            u32::from(nr43 & 7) * 16
        };
        let period = divisor << (nr43 >> 4);
        assert_eq!(apu.noise_timer(), period, "NR43={nr43:02X}");
        let mut remaining = period - 1;
        while remaining > 0 {
            let chunk = remaining.min(65_535) as u16;
            apu.advance_noise(chunk);
            remaining -= u32::from(chunk);
        }
        assert_eq!(apu.channels[3].lfsr, 0x7FFF, "NR43={nr43:02X}");
        apu.advance_noise(1);
        let expected = if nr43 >> 4 >= 14 {
            0x7FFF
        } else if nr43 & 8 != 0 {
            0x3FBF
        } else {
            0x3FFF
        };
        assert_eq!(apu.channels[3].lfsr, expected, "NR43={nr43:02X}");
    }
}

#[test]
fn noise_width_lockup_and_retrigger_follow_the_lfsr_state() {
    let mut apu = noise(0);
    apu.channels[3].lfsr = 0x7F00;
    apu.write(0xFF22, 8);
    apu.tick_fast(8 * 64);
    assert_eq!(apu.channels[3].lfsr & 0x7F, 0);
    assert_eq!(apu.channel_digital_output(4), 15);
    apu.write(0xFF23, 0x80);
    assert_eq!(apu.channels[3].lfsr, 0x7FFF);
    apu.tick_fast(8 * 7);
    assert_ne!(apu.channels[3].lfsr & 0x7F, 0);
}

#[test]
fn long_noise_divider_survives_existing_state_serialization() {
    let mut apu = noise(0xD7);
    apu.tick_fast(12_345);
    let bytes = bincode::serialize(&apu).unwrap();
    let mut restored: Apu = bincode::deserialize(&bytes).unwrap();
    assert_eq!(restored.noise_timer(), 917_504 - 12_345);
    for cycles in [65_535, 17, 640_000, 300_000] {
        apu.tick_fast(cycles);
        restored.tick(cycles);
        assert_eq!(restored.channels[3].lfsr, apu.channels[3].lfsr);
        assert_eq!(restored.noise_timer(), apu.noise_timer());
    }
}

#[test]
fn channel_areas_match_single_cycle_integration_with_register_changes() {
    let mut apu = noise(0);
    apu.write(0xFF11, 0x80);
    apu.write(0xFF12, 0xA3);
    apu.write(0xFF13, 0xFE);
    apu.write(0xFF14, 0x87);
    apu.write(0xFF16, 0x40);
    apu.write(0xFF17, 0xB2);
    apu.write(0xFF18, 0xF7);
    apu.write(0xFF19, 0x87);
    for i in 0..16 {
        apu.write(0xFF30 + i, (i * 13) as u8);
    }
    apu.write(0xFF1A, 0x80);
    apu.write(0xFF1C, 0x20);
    apu.write(0xFF1D, 0xFF);
    apu.write(0xFF1E, 0x87);
    apu.write(0xFF25, 0xFF);
    let mut reference = apu.clone();
    for (register, value, cycles) in [
        (0xFF25, 0xFF, 1003),
        (0xFF24, 0x23, 8193),
        (0xFF25, 0xA5, 131),
        (0xFF22, 0xD7, 917),
        (0xFF1C, 0x60, 65),
        (0xFF26, 0, 137),
        (0xFF26, 0x80, 64),
        (0xFF21, 0xF0, 64),
        (0xFF23, 0x80, 100_003),
    ] {
        apu.write(register, value);
        reference.write(register, value);
        apu.tick_fast(cycles);
        for _ in 0..cycles {
            reference.tick_fast(1);
        }
        assert_eq!(
            apu.drain_interleaved_i16(usize::MAX),
            reference.drain_interleaved_i16(usize::MAX)
        );
    }
}

#[test]
fn pcm_averaging_preserves_fractional_mixer_levels() {
    let mut apu = noise(3);
    apu.pcm_accum_left = 1;
    apu.pcm_accum_right = -1;
    apu.pcm_accum_cycles = 64;
    apu.enqueue_pcm_frame(&mut 0);
    assert_eq!(apu.drain_interleaved_i16(1), vec![1, -1]);
}

fn wave(cgb: bool, frequency: u16) -> Apu {
    let mut apu = Apu::default();
    apu.set_cgb_mode(cgb);
    apu.write(0xFF26, 0x80);
    for i in 0..16u16 {
        apu.write(0xFF30 + i, (i as u8).wrapping_mul(17));
    }
    apu.write(0xFF1A, 0x80);
    apu.write(0xFF1C, 0x20);
    apu.write(0xFF1D, frequency as u8);
    apu.write(0xFF1E, 0x80 | (frequency >> 8) as u8);
    apu
}

#[test]
fn first_div_apu_edge_after_power_on_clocks_length() {
    let mut apu = pulse(2, 0, 1024);
    apu.write(0xFF16, 63);
    apu.write(0xFF19, 0xC4);
    apu.tick_fast(8191);
    assert_ne!(apu.read(0xFF26) & 2, 0);
    apu.tick_fast(1);
    assert_eq!(apu.read(0xFF26) & 2, 0);
}

#[test]
fn nr52_power_cycle_preserves_divider_phase() {
    let mut apu = Apu::default();
    apu.tick_fast(4096);
    apu.write(0xFF26, 0x80);
    apu.tick_fast(1024);
    apu.write(0xFF26, 0);
    apu.tick_fast(1024);
    apu.write(0xFF26, 0x80);
    apu.write(0xFF16, 63);
    apu.write(0xFF17, 0xF0);
    apu.write(0xFF19, 0xC0);
    apu.tick_fast(2047);
    assert_ne!(apu.read(0xFF26) & 2, 0);
    apu.tick_fast(1);
    assert_eq!(apu.read(0xFF26) & 2, 0);
}

#[test]
fn dmg_wave_ram_opens_only_during_the_fetch_window() {
    let mut apu = wave(false, 2040); // 16 clocks per sample, 6-clock trigger delay.
    assert_eq!(apu.read(0xFF30), 0xFF);
    apu.tick_fast(21);
    assert_eq!(apu.read(0xFF30), 0xFF);
    apu.tick_fast(1);
    assert_eq!(apu.read(0xFF3F), 0);
    apu.tick_fast(1);
    apu.write(0xFF3F, 0xAB);
    assert_eq!(apu.read(0xFF31), 0xAB);
    assert_eq!(apu.channel_digital_output(3), 0); // CPU writes do not refill the latch.
    apu.tick_fast(1);
    assert_eq!(apu.read(0xFF30), 0xFF);
    apu.write(0xFF30, 0xCD);
    apu.write(0xFF1A, 0);
    assert_eq!(apu.read(0xFF30), 0xAB);
    assert_eq!(apu.read(0xFF3F), 0xFF);
}

#[test]
fn wave_trigger_delay_and_previous_low_nibble_are_preserved() {
    for cgb in [false, true] {
        let mut apu = wave(cgb, 2040);
        apu.wave_ram[0] = 0xA3;
        apu.tick_fast(21);
        assert_eq!(apu.channel_digital_output(3), 0);
        apu.tick_fast(1);
        assert_eq!(apu.channel_digital_output(3), 3);
        apu.tick_fast(3); // Retrigger outside the RAM corruption window.
        apu.write(0xFF1E, 0x87);
        assert_eq!(apu.channel_digital_output(3), 3);
        apu.wave_ram[0] = 0xB7;
        apu.tick_fast(21);
        assert_eq!(apu.channel_digital_output(3), 3);
        apu.tick_fast(1);
        assert_eq!(apu.channel_digital_output(3), 7);
    }
}

#[test]
fn wave_live_frequency_write_waits_for_current_sample() {
    let mut apu = wave(true, 2040);
    apu.tick_fast(22);
    apu.tick_fast(5);
    apu.write(0xFF1D, 0xFF);
    apu.tick_fast(10);
    assert_eq!(apu.channels[2].wave_position, 1);
    apu.tick_fast(1);
    assert_eq!(apu.channels[2].wave_position, 2);
    apu.tick_fast(2);
    assert_eq!(apu.channels[2].wave_position, 3);
}

#[test]
fn dmg_retrigger_corrupts_only_the_selected_wave_ram_group() {
    for cgb in [false, true] {
        for byte in 0..16usize {
            let mut apu = wave(cgb, 2040);
            let original = apu.wave_ram;
            // Retrigger as the next byte is about to be fetched.
            let sample = if byte == 0 { 1 } else { byte * 2 };
            apu.tick_fast(22 + 16 * (sample as u32 - 1) - 2);
            apu.write(0xFF1E, 0x87);
            let mut expected = original;
            if !cgb {
                if byte < 4 {
                    expected[0] = original[byte];
                } else {
                    let start = byte & !3;
                    expected[..4].copy_from_slice(&original[start..start + 4]);
                }
            }
            assert_eq!(apu.wave_ram, expected, "cgb={cgb}, byte={byte}");
        }
    }
}

#[test]
fn wave_retrigger_does_not_corrupt_ram_outside_the_collision_phase() {
    for remaining in [1, 3, 4, 8] {
        let mut apu = wave(false, 2040);
        apu.tick_fast(22 + 16 * 17 - remaining);
        let original = apu.wave_ram;
        apu.write(0xFF1E, 0x87);
        assert_eq!(apu.wave_ram, original, "remaining={remaining}");
    }
    let mut apu = wave(false, 2040);
    apu.tick_fast(22 + 16 * 17 - 2);
    apu.write(0xFF1A, 0);
    let original = apu.wave_ram;
    apu.write(0xFF1A, 0x80);
    apu.write(0xFF1E, 0x87);
    assert_eq!(apu.wave_ram, original);
}

#[test]
fn all_active_wave_addresses_alias_the_fetched_byte_on_cgb() {
    let mut apu = wave(true, 2040);
    apu.tick_fast(22 + 16 * 17 + 5); // Sample 18, byte 9, outside the DMG window.
    for offset in 0..16 {
        assert_eq!(apu.read(0xFF30 + offset), 0x99);
    }
    apu.write(0xFF3F, 0xA5);
    assert_eq!(apu.wave_ram[9], 0xA5);
    assert_eq!(apu.channel_digital_output(3), 9);
    apu.write(0xFF1A, 0);
    for offset in 0..16u16 {
        assert_eq!(
            apu.read(0xFF30 + offset),
            if offset == 9 { 0xA5 } else { offset as u8 * 17 }
        );
    }
}

#[test]
fn legacy_wave_state_uses_its_position_until_the_next_latch() {
    let mut apu = wave(true, 2040);
    apu.channels[2].wave_position = 3;
    apu.channels[2].wave_sample_buffer = 0xA7;
    apu.channels[2].duty_position = 0; // Version 7 snapshots predating nibble selectors.
    assert_eq!(apu.channel_digital_output(3), 7);
    apu.write(0xFF1E, 0x87);
    assert_eq!(apu.channel_digital_output(3), 7);
    apu.tick_fast(22);
    assert_eq!(apu.channel_digital_output(3), 0);
}
