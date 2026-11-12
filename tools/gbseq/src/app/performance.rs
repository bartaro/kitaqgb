//! The input voice is independent of the song sink: playing a key never restarts transport.
use super::*;
use crate::{midi, model::PanPosition, NoteEffects};
use gbdaw_kokura::KokuraApuHost;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

const KEYS: [&str; 12] = ["A", "W", "S", "E", "D", "F", "T", "G", "Y", "H", "U", "J"];
struct Held {
    id: u16,
    pitch: u8,
    lane: usize,
    v: u8,
    fx: NoteEffects,
    start: Option<u32>,
}
type Message = (Instant, [u8; 3]);
type Input = (
    String,
    String,
    MidiInputConnection<()>,
    Receiver<Message>,
    Arc<AtomicBool>,
);
type Output = (String, String, MidiOutputConnection);
#[derive(Default)]
pub(super) struct Performance {
    pub open: bool,
    octave: u8,
    step: bool,
    initialized: bool,
    overwrite: bool,
    input: Option<Input>,
    output: Option<Output>,
    held: Option<Held>,
    down: BTreeSet<u16>,
    voice: Option<Arc<AtomicBool>>,
    pub take_before: Option<SequenceProject>,
    pub rejected: usize,
    events: Vec<(u32, bool, usize, u8, u8)>,
    next: usize,
    cycle: u128,
    sounding: [Option<u8>; 16],
    #[cfg(test)]
    sent: Vec<[u8; 3]>,
}
impl Performance {
    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }
    fn cycle_input(&mut self) -> Result<String> {
        let midi = MidiInput::new("GBSEQ input")?;
        let ports = midi.ports();
        let idx = self
            .input
            .as_ref()
            .and_then(|(id, name, ..)| {
                ports
                    .iter()
                    .position(|p| p.id() == *id && midi.port_name(p).as_ref().ok() == Some(name))
            })
            .map_or(0, |i| i + 1);
        self.input = None;
        let Some(p) = ports.get(idx) else {
            return Ok(if ports.is_empty() {
                "No MIDI input ports"
            } else {
                "MIDI IN: OFF"
            }
            .into());
        };
        let name = midi.port_name(p)?;
        let (tx, rx) = mpsc::sync_channel(512);
        let overflow = Arc::new(AtomicBool::new(false));
        let flag = overflow.clone();
        let c = midi
            .connect(
                p,
                "GBSEQ keyboard",
                move |_, b, _| {
                    if b.len() == 3
                        && matches!(b[0] & 0xf0, 0x80 | 0x90 | 0xb0)
                        && tx.try_send((Instant::now(), [b[0], b[1], b[2]])).is_err()
                    {
                        flag.store(true, Ordering::Release);
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.input = Some((p.id(), name.clone(), c, rx, overflow));
        Ok(format!("MIDI IN: {name}"))
    }
    fn cycle_output(&mut self) -> Result<String> {
        let midi = MidiOutput::new("GBSEQ output")?;
        let ports = midi.ports();
        let idx = self
            .output
            .as_ref()
            .and_then(|(id, name, ..)| {
                ports
                    .iter()
                    .position(|p| p.id() == *id && midi.port_name(p).as_ref().ok() == Some(name))
            })
            .map_or(0, |i| i + 1);
        self.silence();
        self.output = None;
        let Some(p) = ports.get(idx) else {
            return Ok(if ports.is_empty() {
                "No MIDI output ports"
            } else {
                "MIDI OUT: OFF"
            }
            .into());
        };
        let name = midi.port_name(p)?;
        let c = midi
            .connect(p, "GBSEQ song")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.output = Some((p.id(), name.clone(), c));
        Ok(format!("MIDI OUT: {name}"))
    }
    fn send(&mut self, b: [u8; 3]) -> Result<()> {
        #[cfg(test)]
        self.sent.push(b);
        if let Some((_, _, out)) = &mut self.output {
            out.send(&b)?;
        }
        Ok(())
    }
    fn release_lane(&mut self, i: usize) -> Result<()> {
        if let Some(p) = self.sounding[i] {
            self.send([0x80 | midi::channel(i), p, 0])?;
            self.sounding[i] = None;
        }
        Ok(())
    }
    pub fn silence(&mut self) {
        for i in 0..16 {
            let _ = self.release_lane(i);
        }
    }
    pub fn prepare_output(&mut self, p: &SequenceProject) {
        self.events.clear();
        self.next = 0;
        self.cycle = 0;
        for (lane, t) in p
            .tracks
            .iter()
            .take(p.link16_mode.track_count())
            .enumerate()
        {
            for n in &t.notes {
                if n.velocity > 0 {
                    self.events.push((
                        n.start,
                        true,
                        lane,
                        n.pitch + midi::BASE,
                        midi::velocity(n.velocity),
                    ));
                    self.events
                        .push((n.end(), false, lane, n.pitch + midi::BASE, 0));
                }
            }
        }
        self.events.sort_unstable();
    }
    fn advance(&mut self, step: u32, cycle: u128, mask: usize) -> Result<()> {
        if cycle != self.cycle {
            self.silence();
            self.next = 0;
            self.cycle = cycle;
        }
        for i in 0..16 {
            if mask & (1 << i) == 0 {
                self.release_lane(i)?;
            }
        }
        while let Some(&(at, on, i, p, v)) = self.events.get(self.next) {
            if at > step {
                break;
            }
            self.next += 1;
            if on && mask & (1 << i) != 0 {
                self.send([0x90 | midi::channel(i), p, v])?;
                self.sounding[i] = Some(p);
            } else if !on {
                self.release_lane(i)?;
            }
        }
        Ok(())
    }
    fn stop_voice(&mut self) {
        if let Some(flag) = self.voice.take() {
            flag.store(true, Ordering::Release);
        }
    }
}
impl Drop for Performance {
    fn drop(&mut self) {
        self.stop_voice();
        self.silence();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fresh() -> GbseqApp {
        let mut a = GbseqApp::new_with_audio(Size::new(950., 700.), false);
        a.project = SequenceProject::empty();
        a.toggle_keyboard();
        a
    }
    fn key(s: &str, down: bool) -> UiEvent {
        let key = format!("{s}|Code(Key{})", s.to_uppercase());
        if down {
            UiEvent::KeyDown {
                key,
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
            }
        } else {
            UiEvent::KeyUp {
                key,
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
            }
        }
    }
    #[test]
    fn pc_and_midi_input_protect_split_and_undo() {
        let mut a = fresh();
        let old = a.project.insert_note_exact(0, 0, 7, 11, 5).unwrap();
        a.live.step = true;
        a.cursor_step = 2;
        a.handle_event(key("a", true));
        a.handle_event(key("a", false));
        assert_eq!(a.project.tracks[0].notes.len(), 1);
        assert_eq!(a.cursor_step, 2);
        a.live.overwrite = true;
        a.handle_event(key("w", true));
        a.handle_event(key("w", true));
        a.handle_event(key("w", false));
        assert_eq!(a.cursor_step, 3);
        assert_eq!(a.project.note(old).unwrap().length, 2);
        assert_eq!(
            a.project.tracks[0]
                .notes
                .iter()
                .map(|n| (n.start, n.length, n.pitch))
                .collect::<Vec<_>>(),
            [(0, 2, 11), (2, 1, 25), (3, 4, 11)]
        );
        a.undo();
        assert_eq!(a.project.note(old).unwrap().length, 7);
        a.receive_key(Instant::now(), [0x95, 67, 80]);
        a.receive_key(Instant::now(), [0x95, 67, 0]);
        assert!(a.live.held.is_none());
        assert_eq!(a.project.tracks[0].notes[1].pitch, 31);
    }
    #[test]
    fn take_uses_transport_time_and_preserves_backing_with_one_undo() {
        let mut a = fresh();
        a.selected_track = 2;
        a.start_transport(true);
        let start = Instant::now();
        let dur =
            Duration::from_secs_f64(f64::from(a.project.frames_per_step()) / GB_FRAME_RATE_HZ);
        a.play_started = Some(start - dur * 5);
        a.receive_key(start - dur * 3, [0x91, 60, 110]);
        a.receive_key(start, [0x81, 60, 0]);
        assert!(a.play_started.is_some());
        a.stop_audio();
        let n = &a.project.tracks[2].notes[0];
        assert_eq!((n.start, n.length, n.pitch), (2, 3, 24));
        a.undo();
        assert!(a.project.tracks[2].notes.is_empty());
    }
    #[test]
    fn output_orders_releases_and_cleans_up_at_loops_and_stop() {
        let mut p = SequenceProject::empty();
        p.insert_note_exact(3, 0, 1, 1, 15).unwrap();
        p.insert_note_exact(3, 1, 1, 2, 8).unwrap();
        let mut l = Performance::default();
        l.prepare_output(&p);
        l.advance(0, 0, 65535).unwrap();
        l.advance(1, 0, 65535).unwrap();
        l.advance(0, 1, 65535).unwrap();
        l.silence();
        assert_eq!(
            l.sent,
            [
                [0x99, 37, 127],
                [0x89, 37, 0],
                [0x99, 38, 68],
                [0x89, 38, 0],
                [0x99, 37, 127],
                [0x89, 37, 0]
            ]
        );
    }
    #[test]
    #[ignore = "explicit native audio/MIDI device check"]
    fn native_keyboard_and_ports() {
        let mut a = GbseqApp::new(Size::new(950., 700.));
        assert!(a.audio.is_some());
        a.toggle_keyboard();
        a.handle_event(key("a", true));
        assert!(a.live.voice.is_some());
        thread::sleep(Duration::from_millis(80));
        a.handle_event(key("a", false));
        assert!(a.live.voice.is_none());
        eprintln!("{}", a.live.cycle_input().unwrap());
        eprintln!("{}", a.live.cycle_output().unwrap());
        if a.live.has_output() {
            a.live.prepare_output(&SequenceProject::demo());
            a.live.advance(0, 0, 65535).unwrap();
            thread::sleep(Duration::from_millis(80));
            a.live.silence();
        }
    }
    #[test]
    #[ignore = "requires a configured loopMIDI loopback and audio output"]
    fn native_midi_wire_records_playback_into_another_track() {
        let mut a = GbseqApp::new(Size::new(950., 700.));
        assert!(a.audio.is_some());
        a.project = SequenceProject::empty();
        a.project.measures = 1;
        a.project.insert_note_exact(0, 2, 3, 26, 13).unwrap();
        a.selected_track = 1;
        a.toggle_keyboard();
        let contains = |name: &str| name.to_ascii_lowercase().contains("loopmidi");
        let input = MidiInput::new("GBSEQ port inventory").unwrap().port_count();
        for _ in 0..input {
            a.live.cycle_input().unwrap();
            if a.live.input.as_ref().is_some_and(|i| contains(&i.1)) {
                break;
            }
        }
        let output = MidiOutput::new("GBSEQ port inventory")
            .unwrap()
            .port_count();
        for _ in 0..output {
            a.live.cycle_output().unwrap();
            if a.live.output.as_ref().is_some_and(|i| contains(&i.1)) {
                break;
            }
        }
        assert_eq!(
            a.live.input.as_ref().unwrap().1,
            a.live.output.as_ref().unwrap().1
        );
        a.start_transport(true);
        let limit = Instant::now() + Duration::from_secs(8);
        while (a.rendering || a.play_started.is_some()) && Instant::now() < limit {
            a.tick();
            thread::sleep(Duration::from_millis(2));
        }
        assert!(a.live.take_before.is_none());
        let notes = &a.project.tracks[1].notes;
        assert_eq!(notes.len(), 1);
        let n = &notes[0];
        assert_eq!((n.start, n.length, n.pitch, n.velocity), (2, 3, 26, 13));
        eprintln!(
            "GBSEQ OUT -> {} -> GBSEQ IN: step {}, length {}, pitch {}, velocity {}",
            a.live.input.as_ref().unwrap().1,
            n.start,
            n.length,
            n.pitch,
            n.velocity
        );
    }
}

fn monitor_voice(
    audio: &AudioPlayer,
    ch: u8,
    p: u8,
    v: u8,
    fx: &NoteEffects,
    wave: Option<[u8; 16]>,
) -> Result<Arc<AtomicBool>> {
    let sink = Sink::try_new(&audio.handle)?;
    let mut host = KokuraApuHost::default();
    let env = (v << 4) | (u8::from(fx.envelope_increase) << 3) | fx.envelope_period.min(7);
    host.set_inst(
        ch,
        fx.duty,
        if ch == 2 {
            fx.wave_level.clamp(1, 3) << 5
        } else {
            env
        },
    );
    host.set_ch1_sweep(fx.nr10());
    if let Some(w) = wave {
        host.load_custom_wave(w);
    }
    host.set_pan(
        ch,
        match fx.pan {
            PanPosition::Center => 0,
            PanPosition::Left => 1,
            PanPosition::Right => 2,
        },
    );
    host.note_on(ch, if ch == 3 { fx.nr43() } else { p });
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    thread::spawn(move || {
        let rate = host.audio_sample_rate();
        while !flag.load(Ordering::Acquire) {
            if sink.len() < 3 {
                host.advance_tcycles(16384);
                let pcm = host.drain_audio_frames_interleaved_i16(1024);
                sink.append(rodio::buffer::SamplesBuffer::new(2, rate, pcm));
            } else {
                thread::sleep(Duration::from_millis(1));
            }
        }
        sink.stop();
    });
    Ok(cancel)
}
impl GbseqApp {
    pub(super) fn toggle_keyboard(&mut self) {
        if self.live.open {
            self.patches.page = None;
            if self.live.take_before.is_some() {
                self.stop_audio();
            }
            self.live.step = false;
            self.release_key(None, Instant::now());
            self.live.down.clear();
        } else if !self.live.initialized {
            self.live.octave = 2;
            self.live.initialized = true;
            if !cfg!(test) {
                self.live.overwrite = i18n::preference_path()
                    .and_then(|p| fs::read_to_string(p.with_file_name("input-overwrite.txt")).ok())
                    .is_some_and(|v| v.trim() == "on");
            }
        }
        self.live.open = !self.live.open;
    }
    fn record_step(&self, now: Instant) -> Option<u32> {
        let timing = self.playback_timing?;
        let elapsed = self.transport_elapsed()?.saturating_sub(now.elapsed());
        Some(timing.step_at(elapsed).round() as u32)
    }
    fn store_key(&mut self, h: &Held, start: u32, len: u32) -> Result<()> {
        let id = crate::editing::input_note(
            &mut self.project,
            h.lane,
            start,
            len,
            h.pitch,
            h.v,
            self.live.overwrite,
        )?;
        self.project.note_mut(id).unwrap().effects = h.fx.clone();
        self.chords.clear();
        self.selected_note = Some(id);
        self.selected_notes.clear();
        Ok(())
    }
    fn press_key(&mut self, id: u16, pitch: u8, v: u8, now: Instant) {
        if !self.live.down.insert(id) {
            return;
        }
        self.release_key(None, now); // newest key owns this monophonic GB voice
        let lane = self.selected_track;
        let t = &self.project.tracks[lane];
        let fx = t
            .input_patch
            .as_ref()
            .map(|p| p.effects.clone())
            .or_else(|| {
                self.selected_note
                    .and_then(|id| self.project.note_location(id))
                    .filter(|(t, _)| *t == lane)
                    .map(|(_, n)| self.project.tracks[lane].notes[n].effects.clone())
            })
            .unwrap_or_else(|| NoteEffects::for_track(t.kind));
        let v = t
            .input_patch
            .as_ref()
            .map_or(v, |p| ((u16::from(v) * u16::from(p.volume) + 7) / 15) as u8);
        if let Some(a) = self.audio.as_ref() {
            let wave = if lane % 4 == 2 {
                self.project
                    .gbdaw_wave_definition(fx.wave_preset)
                    .ok()
                    .map(|w| w.bytes16)
            } else {
                None
            };
            match monitor_voice(a, (lane % 4) as u8, pitch, v, &fx, wave) {
                Ok(flag) => self.live.voice = Some(flag),
                Err(e) => self.status = format!("Monitor: {e}"),
            }
        }
        let start = if self.live.take_before.is_some() {
            self.record_step(now)
        } else {
            None
        };
        let h = Held {
            id,
            pitch,
            lane,
            v,
            fx,
            start,
        };
        if self.live.step && self.live.take_before.is_none() {
            let before = self.project.clone();
            let at = self.cursor_step;
            let len = self
                .snap_steps
                .min(self.project.total_steps().saturating_sub(at));
            match self.store_key(&h, at, len) {
                Ok(()) => {
                    self.push_undo(before);
                    self.cursor_step = (at + len).min(self.project.total_steps() - 1);
                }
                Err(e) => self.status = e.to_string(),
            }
        }
        self.live.held = Some(h);
    }
    fn release_key(&mut self, id: Option<u16>, now: Instant) {
        if let Some(id) = id {
            self.live.down.remove(&id);
            if self.live.held.as_ref().is_none_or(|h| h.id != id) {
                return;
            }
        }
        self.live.stop_voice();
        if let Some(h) = self.live.held.take() {
            if let Some(start) = h.start {
                let end = if self.play_started.is_some()
                    && self.audio.as_ref().is_some_and(AudioPlayer::finished)
                {
                    self.project.total_steps()
                } else {
                    self.record_step(now)
                        .unwrap_or(start + 1)
                        .max(start + 1)
                        .min(self.project.total_steps())
                };
                if end <= start
                    || self
                        .store_key(&h, start, end.saturating_sub(start))
                        .is_err()
                {
                    self.live.rejected += 1;
                    self.status = "REC: occupied or outside song; note not inserted".into();
                }
            }
        }
    }
    pub(super) fn finish_performance(&mut self) {
        self.release_key(None, Instant::now());
        self.live.down.clear();
        self.live.silence();
        if let Some(before) = self.live.take_before.take() {
            self.push_undo(before);
        }
    }
    fn receive_key(&mut self, now: Instant, b: [u8; 3]) {
        let id = u16::from(b[0] & 15) * 128 + u16::from(b[1]);
        match b[0] & 0xf0 {
            0x80 | 0x90 if b[2] == 0 || b[0] & 0xf0 == 0x80 => self.release_key(Some(id), now),
            0x90 if (midi::BASE..midi::BASE + 48).contains(&b[1]) => {
                self.press_key(id, b[1] - midi::BASE, midi::level(b[2]), now)
            }
            0xb0 if matches!(b[1], 120 | 123) => {
                self.release_key(None, now);
                self.live.down.clear();
            }
            _ => {}
        }
    }
    pub(super) fn poll_performance(&mut self) -> bool {
        let mut redraw = false;
        if self
            .live
            .input
            .as_ref()
            .is_some_and(|i| i.4.swap(false, Ordering::AcqRel))
        {
            if let Some(i) = &self.live.input {
                while i.3.try_recv().is_ok() {}
            }
            self.release_key(None, Instant::now());
            self.live.down.clear();
            self.status = "MIDI input overflow".into();
            redraw = true;
        }
        for _ in 0..512 {
            let Some((now, b)) = self.live.input.as_ref().and_then(|i| i.3.try_recv().ok()) else {
                break;
            };
            self.receive_key(now, b);
            redraw = true;
        }
        if self.live.has_output() {
            if let (Some(t), Some(pos)) = (self.playback_timing, self.transport_elapsed()) {
                let cycle = if t.looped {
                    pos.as_nanos() / t.duration.as_nanos().max(1)
                } else {
                    0
                };
                let mask = realtime::audible(&self.project);
                if let Err(e) = self.live.advance(t.step_at(pos) as u32, cycle, mask) {
                    self.live.silence();
                    self.live.output = None;
                    self.status = format!("MIDI OUT: {e}");
                    redraw = true;
                }
            }
        }
        redraw
    }
    pub(super) fn perf_rect(&self) -> Rect {
        let w = (self.size.width - 24.).clamp(200., 740.);
        Rect::new(
            (self.size.width - w) * 0.5,
            (self.size.height - 251.).max(0.),
            w,
            214.,
        )
    }
    pub(super) fn panel_cell(&self, row: usize, i: usize, n: usize) -> Rect {
        let r = self.perf_rect();
        let w = (r.width - 12.) / n as f32;
        Rect::new(
            r.x + 6. + w * i as f32,
            r.y + 5. + row as f32 * 29.,
            w - 2.,
            27.,
        )
    }
    fn perf_keys(&self) -> Vec<(usize, Rect, bool)> {
        let r = self.perf_rect();
        let w = (r.width - 12.) / 7.;
        let y = r.y + 122.;
        let mut keys = Vec::new();
        for (i, k) in [0, 2, 4, 5, 7, 9, 11].into_iter().enumerate() {
            keys.push((k, Rect::new(r.x + 6. + i as f32 * w, y, w - 2., 85.), false));
        }
        for (i, k) in [(1, 1), (2, 3), (4, 6), (5, 8), (6, 10)] {
            keys.push((
                k,
                Rect::new(r.x + 6. + (i as f32 - 0.28) * w, y, w * 0.56, 49.),
                true,
            ));
        }
        keys
    }
    pub(super) fn draw_performance(&self, list: &mut DrawList) {
        list.fill_rect(self.perf_rect(), HEADER);
        let mut cell = |r: Rect, label: &str, on: bool| {
            list.fill_rect(r, if on { ACCENT } else { PANEL_RAISED });
            self.text_in(list, r, label, 0.9, if on { BG } else { TEXT });
        };
        cell(
            self.panel_cell(0, 0, 2),
            &format!(
                "IN: {}",
                self.live.input.as_ref().map_or("OFF", |i| i.1.as_str())
            ),
            false,
        );
        cell(
            self.panel_cell(0, 1, 2),
            &format!(
                "OUT: {}",
                self.live.output.as_ref().map_or("OFF", |i| i.1.as_str())
            ),
            false,
        );
        for (i, s) in [
            "Listen",
            "Step input",
            "REC / STOP",
            "Rest",
            if self.live.overwrite {
                "Overwrite"
            } else {
                "Protect notes"
            },
            "Close",
        ]
        .iter()
        .enumerate()
        {
            cell(
                self.panel_cell(1, i, 6),
                s,
                match i {
                    0 => !self.live.step && self.live.take_before.is_none(),
                    1 => self.live.step,
                    2 => self.live.take_before.is_some(),
                    4 => self.live.overwrite,
                    _ => false,
                },
            );
        }
        let pos = if self.live.take_before.is_some() {
            self.record_step(Instant::now()).unwrap_or(0)
        } else {
            self.cursor_step
        };
        for (i, s) in [
            format!("Oct -  C{}", self.live.octave + 2),
            "Oct +".into(),
            format!("Grid: {}", self.snap_steps),
            format!(
                "P{} CH{}",
                self.selected_track / 4 + 1,
                self.selected_track % 4 + 1
            ),
            format!("Step {}", pos + 1),
        ]
        .iter()
        .enumerate()
        {
            cell(self.panel_cell(2, i, 5), s, false);
        }
        for (k, r, black) in self.perf_keys() {
            let sel =
                self.live.held.as_ref().is_some_and(|h| {
                    usize::from(h.pitch) == usize::from(self.live.octave) * 12 + k
                });
            list.fill_rect(
                r,
                if sel {
                    ACCENT
                } else if black {
                    BG
                } else {
                    TEXT
                },
            );
            self.text(
                list,
                r.x + r.width * 0.42,
                r.bottom() - 22.,
                KEYS[k],
                1.,
                if black && !sel { TEXT } else { BG },
            );
        }
        self.draw_patch_picker(list);
    }
    fn panel_press(&mut self, p: Point) {
        let result = if self.panel_cell(0, 0, 2).contains(p) {
            self.release_key(None, Instant::now());
            self.live.down.clear();
            Some(self.live.cycle_input())
        } else if self.panel_cell(0, 1, 2).contains(p) {
            Some(self.live.cycle_output())
        } else {
            None
        };
        if let Some(r) = result {
            self.status = r.unwrap_or_else(|e| format!("MIDI: {e}"));
            return;
        }
        for i in 0..6 {
            if self.panel_cell(1, i, 6).contains(p) {
                match i {
                    0 | 1 => {
                        if self.live.take_before.is_some() {
                            self.stop_audio();
                        }
                        self.live.step = i == 1;
                    }
                    2 => {
                        if self.live.take_before.is_some() {
                            self.stop_audio();
                        } else {
                            self.live.step = false;
                            self.live.rejected = 0;
                            self.start_transport(true);
                        }
                    }
                    3 => {
                        if self.live.step {
                            self.cursor_step = (self.cursor_step + self.snap_steps)
                                .min(self.project.total_steps() - 1);
                        }
                    }
                    4 => {
                        self.live.overwrite = !self.live.overwrite;
                        if !cfg!(test) {
                            if let Some(path) = i18n::preference_path()
                                .map(|p| p.with_file_name("input-overwrite.txt"))
                            {
                                if let Err(e) =
                                    fs::create_dir_all(path.parent().unwrap()).and_then(|_| {
                                        fs::write(
                                            path,
                                            if self.live.overwrite { "on" } else { "off" },
                                        )
                                    })
                                {
                                    self.status = format!("Preference: {e}");
                                }
                            }
                        }
                    }
                    _ => self.toggle_keyboard(),
                }
                return;
            }
        }
        for i in 0..3 {
            if self.panel_cell(2, i, 5).contains(p) {
                match i {
                    0 => self.live.octave = self.live.octave.saturating_sub(1),
                    1 => self.live.octave = (self.live.octave + 1).min(3),
                    _ => self.apply_action(Action::Snap),
                }
                return;
            }
        }
        if let Some((k, _, _)) = self
            .perf_keys()
            .into_iter()
            .rev()
            .find(|(_, r, _)| r.contains(p))
        {
            self.press_key(3000, self.live.octave * 12 + k as u8, 15, Instant::now());
        }
    }
    pub(super) fn performance_event(&mut self, e: &UiEvent) -> bool {
        if matches!(e, UiEvent::FocusChanged { focused: false }) {
            self.release_key(None, Instant::now());
            self.live.down.clear();
        }
        if !self.live.open {
            return false;
        }
        if let UiEvent::KeyUp { key, .. } = e {
            if let Some(i) = KEYS.iter().position(|k| *k == normalized_key(key)) {
                self.release_key(Some(2100 + i as u16), Instant::now());
                return true;
            }
        }
        if self.menu.is_none() && self.patch_event(e) {
            return true;
        }
        if let UiEvent::PointerUp { position, .. } = e {
            if self.live.down.contains(&3000) {
                self.release_key(Some(3000), Instant::now());
                return true;
            }
            if self.menu.is_none() && self.perf_rect().contains(*position) {
                return true;
            }
        }
        if self.menu.is_some() || self.measures_input.is_some() {
            return false;
        }
        match e {
            UiEvent::KeyDown {
                key, ctrl: false, ..
            } => {
                let key = normalized_key(key);
                if let Some(i) = KEYS.iter().position(|k| *k == key) {
                    self.press_key(
                        2100 + i as u16,
                        self.live.octave * 12 + i as u8,
                        15,
                        Instant::now(),
                    );
                    true
                } else if key == "Z" || key == "X" {
                    self.live.octave = if key == "Z" {
                        self.live.octave.saturating_sub(1)
                    } else {
                        (self.live.octave + 1).min(3)
                    };
                    true
                } else {
                    false
                }
            }
            UiEvent::PointerDown {
                position,
                button: PointerButton::Primary,
            } if self.perf_rect().contains(*position) => {
                self.panel_press(*position);
                true
            }
            UiEvent::PointerDoubleClick { position, .. }
                if self.perf_rect().contains(*position) =>
            {
                true
            }
            _ => false,
        }
    }
}
