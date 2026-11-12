//! Bounded, demand-driven KOKURA synthesis. No emulation on the audio callback.
use super::*;
use gbdaw_kokura::ProjectAudioRenderer;
use std::sync::{mpsc::SyncSender, Mutex};

const BLOCK_FRAMES: u32 = 2;
const QUEUE_BLOCKS: usize = 4;
const PRIME_BLOCKS: usize = 2;

#[derive(Default)]
pub(super) struct Stats {
    pub generated_frames: AtomicUsize,
    pub consumed_samples: AtomicUsize,
    pub missing_samples: AtomicUsize,
    pub worker_finished: AtomicBool,
    error: Mutex<Option<String>>,
}

pub(super) struct Control {
    pub cancel: Arc<AtomicBool>,
    pub audible: Arc<AtomicUsize>,
    pub stats: Arc<Stats>,
}
impl Control {
    pub fn error(&self) -> Option<String> {
        self.stats.error.lock().unwrap().take()
    }
}

pub(super) fn audible(project: &SequenceProject) -> usize {
    let solo = project.tracks.iter().any(|t| t.solo);
    project.tracks.iter().enumerate().fold(0, |bits, (ch, t)| {
        bits | (usize::from(!t.mute && (!solo || t.solo)) << ch)
    })
}

pub(super) struct Ready {
    pub source: StreamSource,
    pub duration: Duration,
    pub rate: u32,
}

/// Compiled commands, not PCM, are retained for the duration of a session.
struct Generator {
    players: Vec<ProjectAudioRenderer>,
    frame: u32,
    end: u32,
    rate: u32,
    looped: bool,
    last_frames: u32,
}
impl Generator {
    fn new(project: &SequenceProject, cancel: &AtomicBool) -> Result<Self> {
        project.validate()?;
        let mut source = project.clone();
        for track in &mut source.tracks {
            track.mute = false;
            track.solo = false;
        }
        let mut players = Vec::new();
        for p in 0..source.player_count() {
            anyhow::ensure!(!cancel.load(Ordering::Relaxed), "render cancelled");
            players.push(ProjectAudioRenderer::new(
                &source.to_gbdaw_player(p)?,
                StreamMode::Extended,
            )?);
        }
        let limit = project_frame_count(project);
        let end = players
            .iter()
            .map(|r| r.end_frame().unwrap_or(limit).min(limit))
            .max()
            .unwrap();
        let rate = players[0].sample_rate();
        Ok(Self {
            players,
            frame: 0,
            end,
            rate,
            looped: project.loop_playback,
            last_frames: 0,
        })
    }
    fn samples_until(&self, frame: u32) -> usize {
        // The APU emits stereo at fixed T-cycle deadlines. Preserve rounding
        // across blocks (70224 is not a whole number of PCM sample periods).
        (u64::from(frame) * 70_224 * u64::from(self.rate) / 4_194_304) as usize * 2
    }
    fn duration(&self) -> Duration {
        Duration::from_secs_f64((self.samples_until(self.end) / 2) as f64 / f64::from(self.rate))
    }
    fn block(&mut self, mask: usize) -> Result<Option<Stems>> {
        if self.end == 0 {
            return Ok(None);
        }
        if self.frame == self.end {
            if !self.looped {
                return Ok(None);
            }
            for renderer in &mut self.players {
                renderer.reset();
            }
            self.frame = 0;
        }
        let count = BLOCK_FRAMES.min(self.end - self.frame);
        self.last_frames = count;
        let len = self.samples_until(self.frame + count) - self.samples_until(self.frame);
        let mut parts = Vec::with_capacity(self.players.len());
        for (p, renderer) in self.players.iter_mut().enumerate() {
            let ch_mask = ((mask >> (p * 4)) & 15) as u8;
            renderer.set_monitor_mask(ch_mask);
            let mut samples = renderer.render_next_frames(count)?;
            anyhow::ensure!(samples.len() <= len, "APU block exceeded its time interval");
            samples.resize(len, 0); // a console can end before the rest of the score
            if ch_mask == 0 {
                samples.fill(0);
            }
            parts.push(samples);
        }
        self.frame += count;
        Ok(Some(Stems::new(parts)))
    }
}

pub(super) struct StreamSource {
    rx: Receiver<Stems>,
    block: Option<Stems>,
    cursor: usize,
    rate: u32,
    duration: Option<Duration>,
    monitor: Arc<AtomicUsize>,
    frame_monitor: usize,
    silent_right: bool,
    cancel: Arc<AtomicBool>,
    stats: Arc<Stats>,
}
impl Iterator for StreamSource {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.cancel.load(Ordering::Relaxed) {
            return None;
        }
        if self.silent_right {
            self.silent_right = false;
            self.stats.missing_samples.fetch_add(1, Ordering::Relaxed);
            return Some(0);
        }
        if self.block.as_ref().is_none_or(|b| self.cursor >= b.len) {
            match self.rx.try_recv() {
                Ok(block) => {
                    self.block = Some(block);
                    self.cursor = 0;
                }
                Err(TryRecvError::Disconnected) => return None,
                Err(TryRecvError::Empty) => {
                    // Never wait, allocate, emulate or lock in the callback.
                    // Retain the musical cursor; report inserted silence.
                    self.silent_right = true;
                    self.stats.missing_samples.fetch_add(1, Ordering::Relaxed);
                    return Some(0);
                }
            }
        }
        if self.cursor % 2 == 0 {
            self.frame_monitor = self.monitor.load(Ordering::Relaxed);
        }
        let value = self
            .block
            .as_ref()
            .unwrap()
            .sample(self.cursor, self.frame_monitor);
        self.cursor += 1;
        self.stats.consumed_samples.fetch_add(1, Ordering::Relaxed);
        Some(value)
    }
}
impl Source for StreamSource {
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
        self.duration
    }
}
impl Drop for StreamSource {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
impl StreamSource {
    pub(super) fn collect_wait(&mut self, len: usize) -> Vec<i16> {
        let mut pcm = Vec::with_capacity(len);
        while pcm.len() < len {
            if self.block.as_ref().is_none_or(|b| self.cursor >= b.len) {
                match self.rx.recv_timeout(Duration::from_secs(20)) {
                    Ok(b) => {
                        self.block = Some(b);
                        self.cursor = 0;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(e) => panic!("stream stalled: {e}"),
                }
            }
            match self.next() {
                Some(s) => pcm.push(s),
                None => break,
            }
        }
        pcm
    }
}

fn send(mut block: Stems, tx: &SyncSender<Stems>, control: &Control) -> bool {
    loop {
        if control.cancel.load(Ordering::Relaxed) {
            return false;
        }
        match tx.try_send(block) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
            Err(mpsc::TrySendError::Full(pending)) => {
                block = pending;
                thread::sleep(Duration::from_millis(2));
            }
        }
    }
}

pub(super) fn start(
    project: SequenceProject,
    monitor: Arc<AtomicUsize>,
) -> (Control, Receiver<std::result::Result<Ready, String>>) {
    start_transport(project, monitor, false)
}
pub(super) fn start_transport(
    project: SequenceProject,
    monitor: Arc<AtomicUsize>,
    record: bool,
) -> (Control, Receiver<std::result::Result<Ready, String>>) {
    let control = Control {
        cancel: Arc::new(AtomicBool::new(false)),
        audible: Arc::new(AtomicUsize::new(audible(&project))),
        stats: Arc::new(Stats::default()),
    };
    let worker = Control {
        cancel: control.cancel.clone(),
        audible: control.audible.clone(),
        stats: control.stats.clone(),
    };
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        struct Finished(Arc<Stats>);
        impl Drop for Finished {
            fn drop(&mut self) {
                self.0.worker_finished.store(true, Ordering::Release);
            }
        }
        let _finished = Finished(worker.stats.clone());
        let prepared = (|| -> Result<_> {
            let mut generator = Generator::new(&project, &worker.cancel)?;
            if record {
                generator.end = project.total_steps() * project.frames_per_step();
            }
            let (tx, rx) = mpsc::sync_channel(QUEUE_BLOCKS);
            let duration = generator.duration();
            for _ in 0..PRIME_BLOCKS {
                if let Some(block) = generator.block(worker.audible.load(Ordering::Relaxed))? {
                    worker
                        .stats
                        .generated_frames
                        .fetch_add(generator.last_frames as usize, Ordering::Relaxed);
                    anyhow::ensure!(send(block, &tx, &worker), "render cancelled");
                } else {
                    break;
                }
            }
            let source = StreamSource {
                rx,
                block: None,
                cursor: 0,
                rate: generator.rate,
                duration: (!project.loop_playback).then_some(duration),
                monitor,
                frame_monitor: 0,
                silent_right: false,
                cancel: worker.cancel.clone(),
                stats: worker.stats.clone(),
            };
            Ok((
                generator,
                tx,
                Ready {
                    source,
                    duration,
                    rate: gbdaw_kokura::default_output_sample_rate(),
                },
            ))
        })();
        let (mut generator, tx, ready) = match prepared {
            Ok(value) => value,
            Err(e) => {
                let _ = ready_tx.send(Err(format!("{e:#}")));
                return;
            }
        };
        if ready_tx.send(Ok(ready)).is_err() {
            return;
        }
        while !worker.cancel.load(Ordering::Relaxed) {
            match generator.block(worker.audible.load(Ordering::Relaxed)) {
                Ok(Some(block)) => {
                    worker
                        .stats
                        .generated_frames
                        .fetch_add(generator.last_frames as usize, Ordering::Relaxed);
                    if !send(block, &tx, &worker) {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    *worker.stats.error.lock().unwrap() = Some(format!("{e:#}"));
                    break;
                }
            }
        }
    });
    (control, ready_rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(mode: Link16Mode) -> SequenceProject {
        let mut p = SequenceProject::empty();
        p.set_link16_mode(mode);
        p.measures = 1;
        for ch in 0..p.tracks.len() {
            let id = p.insert_note_exact(ch, 0, 12, 17 + ch as u8, 14).unwrap();
            let fx = &mut p.note_mut(id).unwrap().effects;
            fx.envelope_period = 2;
            fx.wave_preset = 1;
            fx.noise_shift = 1;
            fx.noise_divisor = 1;
            fx.pan = if ch % 2 == 0 {
                crate::model::PanPosition::Left
            } else {
                crate::model::PanPosition::Right
            };
        }
        p
    }
    fn ready(p: SequenceProject, monitor: Arc<AtomicUsize>) -> (Control, Ready) {
        let (control, rx) = start(p, monitor);
        let ready = rx.recv_timeout(Duration::from_secs(20)).unwrap().unwrap();
        (control, ready)
    }
    fn stopped(c: &Control) {
        c.cancel.store(true, Ordering::Relaxed);
        let until = Instant::now() + Duration::from_secs(2);
        while !c.stats.worker_finished.load(Ordering::Acquire) {
            assert!(Instant::now() < until, "cancelled worker did not exit");
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn short_blocks_match_offline_pcm_and_loop_boundaries_for_every_mode() {
        for mode in [Link16Mode::Local4, Link16Mode::Cable8, Link16Mode::Dmg07_16] {
            for looped in [false, true] {
                let mut p = score(mode);
                p.loop_playback = looped;
                let projects = crate::services::playback_projects(&p, None).unwrap();
                let (rate, expected) =
                    crate::services::render_mixed_projects(&projects, project_frame_count(&p))
                        .unwrap();
                let (control, mut live) = ready(p, Arc::new(AtomicUsize::new(0)));
                assert_eq!(live.rate, rate);
                assert_eq!(
                    live.duration,
                    Duration::from_secs_f64(expected.len() as f64 / (2.0 * f64::from(rate)))
                );
                let len = expected.len() * if looped { 3 } else { 1 };
                assert_eq!(
                    live.source.collect_wait(len),
                    expected.repeat(if looped { 3 } else { 1 })
                );
                if !looped {
                    assert_eq!(live.source.collect_wait(2), Vec::<i16>::new());
                }
                assert_eq!(control.stats.missing_samples.load(Ordering::Relaxed), 0);
                stopped(&control);
            }
        }
    }

    #[test]
    fn longest_song_starts_with_bounded_pcm_and_cancel_releases_backpressure() {
        let mut p = score(Link16Mode::Dmg07_16);
        p.measures = 256;
        p.insert_note_exact(0, 4095, 1, 24, 12).unwrap();
        let (control, ready) = ready(p, Arc::new(AtomicUsize::new(0)));
        thread::sleep(Duration::from_millis(80));
        let ahead = control.stats.generated_frames.load(Ordering::Relaxed);
        // Four queued blocks and at most one pending producer block, no whole-song PCM.
        assert!(
            ahead <= (QUEUE_BLOCKS + 1) * BLOCK_FRAMES as usize,
            "{ahead}"
        );
        assert!(ready.duration > Duration::from_secs(200));
        stopped(&control);
        drop(ready);
    }

    #[test]
    fn monitoring_stays_stereo_aligned_without_resetting_stream() {
        for mode in [Link16Mode::Cable8, Link16Mode::Dmg07_16] {
            let p = score(mode);
            let (rate, parts) = {
                let projects = crate::services::playback_projects(&p, None).unwrap();
                let parts =
                    crate::services::render_player_stems(&projects, 2, false, None).unwrap();
                (parts[0].0, parts.into_iter().map(|(_, s)| s).collect())
            };
            let expected = Stems::new(parts);
            let monitor = Arc::new(AtomicUsize::new(0));
            let (control, mut live) = ready(p, monitor.clone());
            assert_eq!(live.rate, rate);
            assert_eq!(live.source.next(), Some(expected.sample(0, 0)));
            monitor.store(1, Ordering::Relaxed);
            assert_eq!(live.source.next(), Some(expected.sample(1, 0)));
            let mut cursor = 2;
            for player in 1..=mode.player_count() {
                monitor.store(player, Ordering::Relaxed);
                assert_eq!(live.source.next(), Some(expected.sample(cursor, player)));
                assert_eq!(
                    live.source.next(),
                    Some(expected.sample(cursor + 1, player))
                );
                cursor += 2;
            }
            assert_eq!(
                control.stats.consumed_samples.load(Ordering::Relaxed),
                cursor
            );
            stopped(&control);
        }
    }

    #[test]
    fn live_mute_and_unmute_preserve_held_oscillators_and_score_pan() {
        let p = score(Link16Mode::Cable8);
        let cancel = AtomicBool::new(false);
        let mut reference = Generator::new(&p, &cancel).unwrap();
        let mut monitored = Generator::new(&p, &cancel).unwrap();
        for mask in [255, 0, 1, 128, 255] {
            reference.block(255).unwrap();
            let pcm = monitored.block(mask).unwrap().unwrap();
            if mask == 0 {
                assert!((0..pcm.len).all(|i| pcm.sample(i, 0) == 0));
            }
            for (a, b) in reference.players.iter().zip(&monitored.players) {
                let a = a.snapshot();
                let b = b.snapshot();
                assert_eq!(a.cycle_counter, b.cycle_counter);
                assert_eq!(a.nr10_to_nr14, b.nr10_to_nr14);
                assert_eq!(a.nr30_to_nr34, b.nr30_to_nr34);
                assert_eq!(a.nr52, b.nr52);
                if mask == 255 {
                    assert_eq!(a.nr51, b.nr51);
                }
            }
        }
    }

    #[test]
    fn starved_callback_returns_stereo_silence_without_moving_musical_cursor() {
        let (tx, rx) = mpsc::sync_channel(1);
        let stats = Arc::new(Stats::default());
        let mut source = StreamSource {
            rx,
            block: None,
            cursor: 0,
            rate: 65536,
            duration: None,
            monitor: Arc::new(AtomicUsize::new(0)),
            frame_monitor: 0,
            silent_right: false,
            cancel: Arc::new(AtomicBool::new(false)),
            stats: stats.clone(),
        };
        assert_eq!(source.next(), Some(0));
        tx.send(Stems::new(vec![vec![111, 222]])).unwrap();
        assert_eq!(source.next(), Some(0)); // complete the silent stereo frame first
        assert_eq!([source.next(), source.next()], [Some(111), Some(222)]);
        assert_eq!(stats.missing_samples.load(Ordering::Relaxed), 2);
        assert_eq!(stats.consumed_samples.load(Ordering::Relaxed), 2);
        drop(tx);
        assert_eq!(source.next(), None);
    }
}
