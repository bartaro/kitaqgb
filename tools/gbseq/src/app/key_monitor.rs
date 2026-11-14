//! One monophonic, continuously clocked source for mouse, PC and MIDI audition.
//! Unlike a queue of short buffers this cannot underrun while waiting for a UI
//! timer/feeder, and changing keys never mixes two full-volume sinks together.
use super::*;
use gbdaw_kokura::KokuraApuHost;
use std::sync::Mutex;

const FRAMES: usize = 128; // ~2 ms at KOKURA's native 65536 Hz.
enum Gate {
    On(KokuraApuHost),
    Off,
}
type Mailbox = Arc<Mutex<Option<Gate>>>;

pub(super) struct KeyMonitor {
    mailbox: Mailbox,
    _sink: Sink,
    playing: bool,
    #[cfg(test)]
    pub(super) active_blocks: Arc<AtomicUsize>,
}
impl KeyMonitor {
    pub(super) fn new(handle: &OutputStreamHandle, rate: u32) -> Result<Self> {
        let sink = Sink::try_new(handle)?;
        let mailbox = Mailbox::default();
        let source = KeySource::new(mailbox.clone(), rate);
        #[cfg(test)]
        let active_blocks = source.active_blocks.clone();
        sink.append(source);
        Ok(Self {
            mailbox,
            _sink: sink,
            playing: false,
            #[cfg(test)]
            active_blocks,
        })
    }
    pub(super) fn play(&mut self, host: KokuraApuHost) {
        *self.mailbox.lock().unwrap() = Some(Gate::On(host));
        self.playing = true;
    }
    pub(super) fn release(&mut self) {
        if self.playing {
            *self.mailbox.lock().unwrap() = Some(Gate::Off);
            self.playing = false;
        }
    }
    #[cfg(test)]
    pub(super) fn is_playing(&self) -> bool {
        self.playing
    }
}
struct KeySource {
    mailbox: Mailbox,
    host: Option<KokuraApuHost>,
    rate: u32,
    pcm: [i16; FRAMES * 2],
    index: usize,
    #[cfg(test)]
    active_blocks: Arc<AtomicUsize>,
}
impl KeySource {
    fn new(mailbox: Mailbox, rate: u32) -> Self {
        Self {
            mailbox,
            host: None,
            rate,
            pcm: [0; FRAMES * 2],
            index: FRAMES * 2,
            #[cfg(test)]
            active_blocks: Default::default(),
        }
    }
    fn refill(&mut self) {
        let last = [self.pcm[FRAMES * 2 - 2], self.pcm[FRAMES * 2 - 1]];
        let change = self.mailbox.try_lock().ok().and_then(|mut m| m.take());
        let ramp = change.is_some();
        if let Some(change) = change {
            self.host = match change {
                Gate::Off => None,
                Gate::On(host) => Some(host),
            };
        }
        if let Some(host) = &mut self.host {
            host.advance_tcycles((4_194_304 * FRAMES as u32) / self.rate);
            let samples = host.drain_audio_frames_interleaved_i16(FRAMES);
            self.pcm.copy_from_slice(&samples);
            #[cfg(test)]
            if self.pcm.iter().any(|&x| x != 0) {
                self.active_blocks.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            self.pcm.fill(0);
        }
        if ramp {
            // Blend from the last audible stereo sample, not from zero on a
            // retrigger. Complementary weights guarantee no gain overshoot.
            for (i, frame) in self.pcm.chunks_exact_mut(2).enumerate() {
                for (x, from) in frame.iter_mut().zip(last) {
                    *x = ((i32::from(from) * (FRAMES - 1 - i) as i32 + i32::from(*x) * i as i32)
                        / (FRAMES - 1) as i32) as i16;
                }
            }
        }
        self.index = 0;
    }
}
impl Iterator for KeySource {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.index == self.pcm.len() {
            self.refill();
        }
        self.index += 1;
        Some(self.pcm[self.index - 1])
    }
}
impl Source for KeySource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_four_channels_match_unbroken_pcm_and_release_to_exact_silence() {
        for ch in 0..4 {
            let fx = crate::NoteEffects::default();
            let host = super::super::performance::voice_host(ch, 26, 15, &fx, None);
            let mut reference = host.clone();
            let mailbox = Mailbox::default();
            let mut source = KeySource::new(mailbox.clone(), host.audio_sample_rate());
            *mailbox.lock().unwrap() = Some(Gate::On(host));
            let blocks = 96;
            reference.advance_tcycles((FRAMES * blocks * 64) as u32);
            let expected = reference.drain_audio_frames_interleaved_i16(FRAMES * blocks);
            let pcm: Vec<_> = source.by_ref().take(expected.len()).collect();
            assert!(expected.iter().any(|&s| s != 0));
            assert_eq!(pcm[..2], [0, 0]);
            assert_eq!(pcm[FRAMES * 2..], expected[FRAMES * 2..], "CH{}", ch + 1);
            *mailbox.lock().unwrap() = Some(Gate::Off);
            let tail: Vec<_> = source.by_ref().take(FRAMES * 4).collect();
            assert_eq!(&tail[..2], &pcm[pcm.len() - 2..]);
            assert!(tail[FRAMES * 2 - 2..].iter().all(|&s| s == 0));
        }
    }
    #[test]
    fn rapid_retrigger_uses_latest_voice_and_never_sums_notes() {
        let host = |note| {
            super::super::performance::voice_host(1, note, 15, &crate::NoteEffects::default(), None)
        };
        let m = Mailbox::default();
        let mut s = KeySource::new(m.clone(), host(7).audio_sample_rate());
        *m.lock().unwrap() = Some(Gate::On(host(7)));
        let previous = s.by_ref().take(FRAMES * 4).collect::<Vec<_>>();
        *m.lock().unwrap() = Some(Gate::Off);
        *m.lock().unwrap() = Some(Gate::On(host(40)));
        let retrigger = s.by_ref().take(FRAMES * 4).collect::<Vec<_>>();
        assert_eq!(retrigger[..2], previous[previous.len() - 2..]);
        let mut isolated = host(40);
        isolated.advance_tcycles((2 * FRAMES * 64) as u32);
        let expected = isolated.drain_audio_frames_interleaved_i16(2 * FRAMES);
        assert_eq!(retrigger[FRAMES * 2..], expected[FRAMES * 2..]);
        let peak = expected
            .iter()
            .chain(&previous[previous.len() - 2..])
            .map(|s| i32::from(*s).abs())
            .max()
            .unwrap();
        assert!(retrigger.iter().all(|s| i32::from(*s).abs() <= peak));
    }
}
