//! Exercise the exact audio host used by GBSEQ's keyboard and score renderers.
use gbdaw_kokura::KokuraApuHost;

#[test]
fn changing_the_render_block_size_does_not_change_the_pcm() {
    let mut block = KokuraApuHost::default();
    let mut small = block.clone();
    let mut total = 0usize;
    let mut samples = 0usize;
    for host in [&mut block, &mut small] {
        for (ch, pitch) in [(0, 45), (1, 47), (2, 44), (3, 0x08)] {
            host.note_on(ch, pitch);
        }
    }
    for (reg, val, cycles) in [
        (0xff25, 0xff, 70224),
        (0xff22, 0xd7, 925013),
        (0xff25, 0x52, 12139),
        (0xff26, 0, 70224),
        (0xff26, 0x80, 777),
    ] {
        block.write_apu_reg(reg, val);
        small.write_apu_reg(reg, val);
        block.advance_tcycles(cycles);
        let mut left = cycles;
        while left > 0 {
            let n = left.min(13);
            small.advance_tcycles(n);
            left -= n;
        }
        let pcm = block.drain_audio_frames_interleaved_i16(usize::MAX);
        assert_eq!(pcm, small.drain_audio_frames_interleaved_i16(usize::MAX));
        if total == 0 {
            assert!(pcm.iter().any(|&s| s != 0));
        }
        if reg == 0xff26 && val == 0 {
            assert!(pcm[2..].iter().all(|&s| s == 0));
        }
        samples += pcm.len();
        total += cycles as usize;
        assert_eq!(samples, total / 64 * 2);
    }
}

// A small original CGB program drives the CLI with LD A,d8 / LDH (a8),A.
// Replay the identical register writes against the sequencer host, with long
// host-side batches instead of emulated instructions. No retail ROM or logo.
#[test]
#[ignore = "set KOKURA_CLI to the separately built KOKURA CLI executable"]
fn same_register_writes_match_kokura_cli_wav_sample_for_sample() {
    use std::{env, fs, process::Command};
    let exe = env::var_os("KOKURA_CLI").expect("set KOKURA_CLI");
    let dir = env::temp_dir().join(format!("gbseq-apu-oracle-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let (rom, wav) = (dir.join("probe.gb"), dir.join("oracle.wav"));
    let mut cart = vec![0u8; 32768];
    cart[0x100..0x103].copy_from_slice(&[0xc3, 0x50, 0x01]); // JP $0150
    cart[0x134..0x13d].copy_from_slice(b"GBSEQ APU");
    cart[0x143] = 0x80;
    cart[0x14d] = cart[0x134..0x14d]
        .iter()
        .fold(0u8, |sum, b| sum.wrapping_sub(*b).wrapping_sub(1));
    let mut pc = 0x150;
    let mut host = KokuraApuHost::default();
    // Both initial states have all DACs off before our first NR52 reset,
    // except that the virtual keyboard pre-loads a wave with its DAC on.
    host.write_apu_reg(0xff1a, 0);
    host.advance_tcycles(16);
    let mut elapsed = 16usize;
    let mut write = |r: u8, v: u8, nops: usize| {
        cart[pc..pc + 4].copy_from_slice(&[0x3e, v, 0xe0, r]);
        pc += 4 + nops; // zero-filled NOPs
        // LD A,d8 takes 8 T-cycles. LDH writes on its third M-cycle
        // (8 T-cycles into LDH), then completes its remaining 4 T-cycles.
        // KOKURA now models the bus access instead of writing at opcode start.
        host.advance_tcycles(16);
        host.write_apu_reg(0xff00 | u16::from(r), v);
        host.advance_tcycles(4 + 4 * nops as u32);
        elapsed += 20 + 4 * nops;
    };
    for (r, v) in [
        (0x26, 0),
        (0x26, 0x80),
        (0x24, 0x77),
        (0x25, 0xff),
        (0x11, 0x80),
        (0x12, 0xd2),
        (0x13, 0xfe),
        (0x14, 0x87),
        (0x16, 0x40),
        (0x17, 0xb2),
        (0x18, 0xc7),
        (0x19, 0x87),
    ] {
        write(r, v, 0);
    }
    for i in 0..16u8 {
        write(0x30 + i, i * 13, 0);
    }
    for (r, v, n) in [
        (0x1a, 0x80, 0),
        (0x1c, 0x20, 0),
        (0x1d, 0xfb, 0),
        (0x1e, 0x87, 0),
        (0x21, 0xf0, 0),
        (0x22, 0x08, 0),
        (0x23, 0x80, 783),
        // Live writes/retriggers exercise the updated envelope, sweep, length
        // and CH3 sample-latch behavior through GBSEQ's actual host API.
        (0x17, 0xb0, 1973),
        (0x17, 0xa8, 220),
        (0x19, 0x87, 238),
        (0x1d, 0xf7, 127),
        (0x1e, 0xc7, 230),
        (0x30, 0xab, 367),
        (0x11, 0xbf, 0),
        (0x14, 0xc7, 257),
        (0x10, 0x21, 870),
        (0x14, 0x86, 1301),
        (0x12, 0xd8, 123),
        (0x10, 0, 411),
        (0x25, 0x5a, 997),
        (0x26, 0, 800),
        (0x26, 0x80, 0),
        (0x24, 0x77, 0),
        (0x25, 0x88, 0),
        (0x21, 0xf0, 0),
        (0x22, 0xd7, 0),
        (0x23, 0x80, 0),
    ] {
        write(r, v, n);
    }
    cart[pc..pc + 2].copy_from_slice(&[0x18, 0xfe]); // JR $ (no more writes)
    fs::write(&rom, cart).unwrap();
    let cli = Command::new(exe)
        .arg(&rom)
        .args(["--hardware", "cgb", "--run-frames", "38", "--record-wav"])
        .arg(&wav)
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let (rate, reference) = gbdaw_kokura::decode_wav_i16_stereo(&wav).unwrap();
    assert_eq!(rate, host.audio_sample_rate());
    assert!(reference.len() > 65536 && reference.iter().any(|&s| s != 0));
    let target = reference.len() / 2 * 64;
    host.advance_tcycles((target - elapsed) as u32);
    let actual = host.drain_audio_frames_interleaved_i16(usize::MAX);
    assert_eq!(actual.len(), reference.len());
    assert_eq!(
        actual.iter().zip(&reference).position(|(a, b)| a != b),
        None,
        "first differing i16 sample; CLI fixture retained at {}",
        dir.display()
    );
    println!(
        "KOKURA CLI vs GBSEQ host: {} identical stereo frames at {} Hz",
        reference.len() / 2,
        rate
    );
    fs::remove_file(rom).unwrap();
    fs::remove_file(wav).unwrap();
    fs::remove_dir(dir).unwrap();
}
