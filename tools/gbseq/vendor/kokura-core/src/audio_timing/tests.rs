use crate::{types::ApuTraceEvent, types::HardwareMode, Machine};

fn machine(program: &[u8], cgb: bool) -> Box<Machine> {
    let mut rom = vec![0; 0x8000];
    rom[0x100..0x100 + program.len()].copy_from_slice(program);
    rom[0x143] = if cgb { 0x80 } else { 0 };
    let mut m = Box::new(Machine::new());
    m.load_rom(rom).unwrap();
    m.write8(0xFF04, 0);
    m.write8(0xFF26, 0);
    m.write8(0xFF26, 0x80);
    m
}

fn arm_length_one(m: &mut Machine) {
    m.write8(0xFF16, 63);
    m.write8(0xFF17, 0xF0);
    m.write8(0xFF19, 0xC0);
}

fn advance(m: &mut Machine, cycles: u32, fast: bool) {
    if fast {
        m.finish_step_fast(cycles);
    } else {
        m.finish_step(cycles);
    }
}

fn step(m: &mut Machine, fast: bool) {
    if fast {
        m.step_instruction_fast().unwrap();
    } else {
        m.step_instruction().unwrap();
    }
}

fn arm_wave(m: &mut Machine) {
    m.write8(0xFF30, 0xAB);
    m.write8(0xFF1A, 0x80);
    m.write8(0xFF1C, 0x20);
    m.write8(0xFF1D, 0xF8);
    m.write8(0xFF1E, 0x87);
}

#[test]
fn div_reset_clocks_apu_only_when_its_selected_bit_is_high() {
    for double in [false, true] {
        for fast in [false, true] {
            for high in [false, true] {
                let mut m = machine(&[0], double);
                m.cgb_double_speed = double;
                arm_length_one(&mut m);
                let rate = if double { 2 } else { 1 };
                advance(&mut m, (if high { 4096 } else { 2048 }) * rate, fast);
                m.write8(0xFF04, 0xA5);
                assert_eq!(m.peek8(0xFF26) & 2 == 0, high);
                assert_eq!(m.timer.divider_counter(), 0);
                if !high {
                    advance(&mut m, 8192 * rate - 4, fast);
                    assert_ne!(m.peek8(0xFF26) & 2, 0);
                    advance(&mut m, 4, fast);
                    assert_eq!(m.peek8(0xFF26) & 2, 0);
                }
            }
        }
    }
}

#[test]
fn div_apu_rate_is_identical_at_both_cpu_speeds() {
    let mut normal = machine(&[0], true);
    let mut double = machine(&[0], true);
    double.cgb_double_speed = true;
    arm_wave(&mut normal);
    arm_wave(&mut double);
    let a = normal.finish_step(8192 * 8);
    let b = double.finish_step(8192 * 16);
    let steps = |trace: Vec<ApuTraceEvent>| {
        trace
            .into_iter()
            .filter_map(|event| match event {
                ApuTraceEvent::FrameSequencerStep { step } => Some(step),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(steps(a.apu_trace), (0..8).collect::<Vec<_>>());
    assert_eq!(steps(b.apu_trace), (0..8).collect::<Vec<_>>());
    assert_eq!(
        normal.drain_audio_frames_interleaved_i16(usize::MAX),
        double.drain_audio_frames_interleaved_i16(usize::MAX)
    );
}

#[test]
fn div_write_uses_the_store_bus_cycle_without_double_counting_time() {
    for fast in [false, true] {
        let mut m = machine(&[0xE0, 0x04], false);
        arm_length_one(&mut m);
        advance(&mut m, 4088, fast);
        let before = m.clocks.cycles;
        step(&mut m, fast);
        assert_eq!(m.clocks.cycles - before, 12);
        assert_eq!(m.timer.divider_counter(), 4);
        assert_eq!(m.peek8(0xFF26) & 2, 0);
    }
}

#[test]
fn length_enable_write_observes_sequencer_phase_at_the_cpu_bus_cycle() {
    for double in [false, true] {
        for fast in [false, true] {
            for before_edge in [false, true] {
                for (length, dac, control) in [
                    (0xFF11, 0xFF12, 0xFF14),
                    (0xFF16, 0xFF17, 0xFF19),
                    (0xFF1B, 0xFF1A, 0xFF1E),
                    (0xFF20, 0xFF21, 0xFF23),
                ] {
                    let mut m = machine(&[0xE0, control as u8], double);
                    m.cgb_double_speed = double;
                    m.write8(length, 0xFF);
                    m.write8(dac, if dac == 0xFF1A { 0x80 } else { 0xF0 });
                    m.write8(control, 0x80);
                    let mask = m.peek8(0xFF26) & 15;
                    assert_ne!(mask, 0);
                    // Step 1 does not clock length: enabling just before it
                    // adds a clock; enabling on/after it must not add one.
                    let edge = 16_384 * if double { 2 } else { 1 };
                    advance(&mut m, edge - if before_edge { 12 } else { 8 }, fast);
                    m.cpu.a = 0x40;
                    step(&mut m, fast);
                    assert_eq!(m.peek8(0xFF26) & mask == 0, before_edge);
                }
            }
        }
    }
}

#[test]
fn speed_switch_resets_div_using_the_old_speed_bit_and_freezes_phase() {
    for was_double in [false, true] {
        let mut m = machine(&[0x10, 0], true);
        m.cgb_double_speed = was_double;
        arm_length_one(&mut m);
        advance(&mut m, if was_double { 8192 } else { 4096 }, false);
        m.write8(0xFF4D, 1);
        let result = m.step_instruction().unwrap();
        assert_eq!(m.cgb_double_speed, !was_double);
        assert_eq!(m.timer.divider_counter(), 4);
        assert_eq!(m.peek8(0xFF26) & 2, 0);
        assert_eq!(
            result
                .apu_trace
                .iter()
                .filter(|e| matches!(e, ApuTraceEvent::FrameSequencerStep { .. }))
                .count(),
            1
        );
    }
}

#[test]
fn dmg_wave_ram_reads_and_writes_hit_the_instruction_bus_window() {
    for fast in [false, true] {
        for (program, offset, write) in [
            (&[0xF0, 0x3F][..], 8, false),
            (&[0xE0, 0x3F][..], 8, true),
            (&[0x7E][..], 4, false),
            (&[0x77][..], 4, true),
            (&[0xFA, 0x3F, 0xFF][..], 12, false),
            (&[0xEA, 0x3F, 0xFF][..], 12, true),
        ] {
            let mut m = machine(program, false);
            arm_wave(&mut m);
            advance(&mut m, 22 - offset, fast);
            m.cpu.a = 0xCD;
            m.cpu.set_hl(0xFF3F);
            let before = m.clocks.cycles;
            step(&mut m, fast);
            assert_eq!(m.clocks.cycles - before, u64::from(offset + 4));
            m.write8(0xFF1A, 0);
            if write {
                assert_eq!(m.peek8(0xFF30), 0xCD);
                assert_eq!(m.peek8(0xFF3F), 0);
            } else {
                assert_eq!(m.cpu.a, 0xAB);
            }
        }
    }
}

#[test]
fn wave_mmio_clock_splitting_preserves_frame_completion_and_trace() {
    let mut m = machine(&[0xE0, 0x25], false);
    m.clocks.frame_phase_ppu_cycles = m.ppu.frame_cycles() - 4;
    let start = m.clocks.frames;
    let result = m.step_instruction().unwrap();
    assert!(result.frame_completed);
    assert_eq!(result.cycles, 12);
    assert_eq!(m.clocks.frames, start + 1);
    assert_eq!(m.clocks.frame_phase_ppu_cycles, 8);
    assert!(result
        .apu_trace
        .iter()
        .any(|e| matches!(e, ApuTraceEvent::MixerControlWrite { .. })));
}

#[test]
fn cgb_pcm_read_observes_the_bus_cycle_at_both_speeds() {
    for double in [false, true] {
        for fast in [false, true] {
            let mut m = machine(&[0xF0, 0x77], true);
            m.cgb_double_speed = double;
            arm_wave(&mut m);
            let ratio = if double { 2 } else { 1 };
            advance(&mut m, 22 * ratio - 8, fast);
            assert_eq!(m.apu.pcm34() & 15, 0);
            let start = m.clocks.cycles;
            step(&mut m, fast);
            assert_eq!(m.cpu.a & 15, 11);
            assert_eq!(m.clocks.cycles - start, 12);
        }
    }
}

#[test]
fn serialized_resume_preserves_div_phase_and_wave_fetch_window() {
    for cgb in [false, true] {
        let mut m = machine(&[0], cgb);
        arm_wave(&mut m);
        advance(&mut m, 22, false);
        let bytes = m.save_state().to_bytes().unwrap();
        let state = crate::state::MachineState::from_bytes(&bytes).unwrap();
        let mut restored = machine(&[0], cgb);
        restored.load_state(&state);
        assert_eq!(
            restored.mode,
            if cgb {
                HardwareMode::Cgb
            } else {
                HardwareMode::Dmg
            }
        );
        assert_eq!(restored.peek8(0xFF3F), 0xAB);
        for ticks in [1, 1, 2, 4, 8192, 16384] {
            advance(&mut m, ticks, false);
            advance(&mut restored, ticks, true);
            assert_eq!(m.peek8(0xFF3F), restored.peek8(0xFF3F));
            assert_eq!(m.apu.pcm34(), restored.apu.pcm34());
            assert_eq!(
                m.apu.frame_sequencer_step(),
                restored.apu.frame_sequencer_step()
            );
            assert_eq!(m.timer.divider_counter(), restored.timer.divider_counter());
        }
    }
}

#[test]
fn wave_and_div_program_matches_traced_and_fast_execution() {
    let program = [
        0x3E, 0x87, 0xE0, 0x1E, 0xE0, 0x04, 0xF0, 0x3F, 0x21, 0x3F, 0xFF, 0xCB, 0xC6, 0x34, 0x00,
        0x18, 0xEF,
    ];
    let mut traced = machine(&program, false);
    let mut fast = machine(&program, false);
    arm_wave(&mut traced);
    arm_wave(&mut fast);
    for _ in 0..5000 {
        step(&mut traced, false);
        step(&mut fast, true);
    }
    assert_eq!(traced.clocks.cycles, fast.clocks.cycles);
    assert_eq!(traced.cpu.pc, fast.cpu.pc);
    assert_eq!(traced.timer.divider_counter(), fast.timer.divider_counter());
    assert_eq!(
        traced.drain_audio_frames_interleaved_i16(usize::MAX),
        fast.drain_audio_frames_interleaved_i16(usize::MAX)
    );
}
