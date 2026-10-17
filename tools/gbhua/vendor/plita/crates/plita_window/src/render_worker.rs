use std::sync::{mpsc, Arc, Condvar, Mutex};

use plita_render::{FrameStats, GpuRenderer, RenderError, RenderSize};
use plita_ui::{DrawList, Size};

#[derive(Default)]
struct RenderWork {
    draw_list: Option<DrawList>,
    size: Option<RenderSize>,
    scale: Option<f64>,
    render: Option<u64>,
    shutdown: bool,
}

#[derive(Clone, Default)]
struct RenderMailbox(Arc<(Mutex<RenderWork>, Condvar)>);

impl RenderMailbox {
    fn update(&self, update: impl FnOnce(&mut RenderWork)) {
        let (lock, changed) = &*self.0;
        let mut work = lock.lock().unwrap_or_else(|error| error.into_inner());
        if !work.shutdown {
            update(&mut work);
        }
        changed.notify_one();
    }

    fn take(&self) -> Option<RenderWork> {
        let (lock, changed) = &*self.0;
        let mut work = lock.lock().unwrap_or_else(|error| error.into_inner());
        while !work.shutdown
            && work.render.is_none()
            && work.draw_list.is_none()
            && work.size.is_none()
            && work.scale.is_none()
        {
            work = changed
                .wait(work)
                .unwrap_or_else(|error| error.into_inner());
        }
        if work.shutdown {
            None
        } else {
            Some(std::mem::take(&mut *work))
        }
    }

    fn shutdown(&self) {
        self.update(|work| work.shutdown = true);
    }
}

pub(super) enum WindowRenderer {
    Direct(GpuRenderer),
    Threaded(RenderWorker),
}

impl WindowRenderer {
    pub(super) fn new(renderer: GpuRenderer, threaded: bool, scale: f64) -> Self {
        if threaded {
            Self::Threaded(RenderWorker::new(renderer, scale))
        } else {
            Self::Direct(renderer)
        }
    }

    pub(super) fn set_draw_list(&mut self, list: DrawList) {
        match self {
            Self::Direct(renderer) => renderer.set_draw_list(list),
            Self::Threaded(worker) => worker.mailbox.update(|work| work.draw_list = Some(list)),
        }
    }

    pub(super) fn render(&mut self) -> Result<FrameStats, RenderError> {
        match self {
            Self::Direct(renderer) => renderer.render(),
            Self::Threaded(worker) => worker.request_frame(),
        }
    }

    pub(super) fn poll_completed(&mut self) -> Result<(), RenderError> {
        match self {
            Self::Direct(_) => Ok(()),
            Self::Threaded(worker) => worker.poll_completed(),
        }
    }

    pub(super) fn frame_pending(&self) -> bool {
        matches!(self, Self::Threaded(worker) if worker.submitted > worker.finished)
    }

    pub(super) fn resize(&mut self, size: RenderSize) {
        match self {
            Self::Direct(renderer) => renderer.resize(size),
            Self::Threaded(worker) => {
                worker.size = size;
                worker.mailbox.update(|work| work.size = Some(size));
            }
        }
    }

    pub(super) fn set_ui_scale_factor(&mut self, scale: f64) {
        match self {
            Self::Direct(renderer) => renderer.set_ui_scale_factor(scale),
            Self::Threaded(worker) => {
                worker.scale = scale;
                worker.mailbox.update(|work| work.scale = Some(scale));
            }
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn size(&self) -> RenderSize {
        match self {
            Self::Direct(renderer) => renderer.size(),
            Self::Threaded(worker) => worker.size,
        }
    }

    pub(super) fn ui_size(&self) -> Size {
        match self {
            Self::Direct(renderer) => renderer.ui_size(),
            Self::Threaded(worker) => worker.size.to_ui_size_with_scale_factor(worker.scale),
        }
    }
}

pub(super) struct RenderWorker {
    mailbox: RenderMailbox,
    completed: mpsc::Receiver<(u64, Result<FrameStats, RenderError>)>,
    size: RenderSize,
    scale: f64,
    stats: FrameStats,
    submitted: u64,
    finished: u64,
}

impl RenderWorker {
    fn new(mut renderer: GpuRenderer, scale: f64) -> Self {
        let size = renderer.size();
        let mailbox = RenderMailbox::default();
        let receiver = mailbox.clone();
        let (completed, responses) = mpsc::channel();
        // The surface was created on the window thread; this worker never asks
        // winit for a native handle, and never holds the mailbox during GPU work.
        std::thread::spawn(move || {
            while let Some(work) = receiver.take() {
                if let Some(scale) = work.scale {
                    renderer.set_ui_scale_factor(scale);
                }
                if let Some(size) = work.size {
                    renderer.resize(size);
                }
                if let Some(list) = work.draw_list {
                    renderer.set_draw_list(list);
                }
                if let Some(request) = work.render {
                    if completed.send((request, renderer.render())).is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            mailbox,
            completed: responses,
            size,
            scale,
            submitted: 0,
            finished: 0,
            stats: FrameStats {
                frame_index: 0,
                width: size.width,
                height: size.height,
                draw_commands: 0,
                cpu_frame_ms: 0.0,
            },
        }
    }

    fn poll_completed(&mut self) -> Result<(), RenderError> {
        loop {
            match self.completed.try_recv() {
                Ok((request, result)) => {
                    self.finished = request;
                    self.stats = result?;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(RenderError::WorkerDisconnected)
                }
            }
        }
        Ok(())
    }

    fn request_frame(&mut self) -> Result<FrameStats, RenderError> {
        self.poll_completed()?;
        self.submitted += 1;
        let request = self.submitted;
        self.mailbox.update(|work| work.render = Some(request));
        Ok(self.stats)
    }
}

impl Drop for RenderWorker {
    fn drop(&mut self) {
        self.mailbox.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn mailbox_coalesces_frames_resize_and_scale_to_the_latest_state() {
        let mailbox = RenderMailbox::default();
        for index in 1..1000 {
            mailbox.update(|work| {
                work.draw_list = Some(DrawList::new(plita_ui::Color::rgb(0.0, 0.0, 0.0)));
                work.size = Some(RenderSize::clamped(index, 400));
                work.scale = Some(index as f64);
                work.render = Some(index as u64);
            });
        }
        let work = mailbox.take().unwrap();
        assert_eq!(work.size, Some(RenderSize::clamped(999, 400)));
        assert_eq!(work.scale, Some(999.0));
        assert_eq!(work.render, Some(999));
        assert!(work.draw_list.is_some());
        mailbox.shutdown();
        assert!(mailbox.take().is_none());
    }

    #[test]
    fn slow_presentation_does_not_lock_the_ui_and_shutdown_wakes_the_worker() {
        let mailbox = RenderMailbox::default();
        let worker_mailbox = mailbox.clone();
        let (started, rendering) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let (done, finished) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            assert!(worker_mailbox.take().unwrap().render.is_some());
            started.send(()).unwrap();
            wait.recv().unwrap();
            let latest = worker_mailbox.take().unwrap();
            assert_eq!(latest.size, Some(RenderSize::clamped(640, 400)));
            done.send(()).unwrap();
            assert!(worker_mailbox.take().is_none());
        });
        mailbox.update(|work| work.render = Some(1));
        rendering.recv_timeout(Duration::from_secs(2)).unwrap();
        for _ in 0..120 {
            mailbox.update(|work| work.size = Some(RenderSize::clamped(640, 400)));
        }
        release.send(()).unwrap();
        finished.recv_timeout(Duration::from_secs(2)).unwrap();
        mailbox.shutdown();
        worker.join().unwrap();
    }

    #[test]
    fn shutdown_discards_pending_work_and_rejects_further_requests() {
        let mailbox = RenderMailbox::default();
        mailbox.update(|work| work.render = Some(1));
        mailbox.shutdown();
        mailbox.update(|work| panic!("shutdown must reject new work: {:?}", work.render));
        assert!(mailbox.take().is_none());
    }

    #[test]
    fn completions_and_disconnect_are_reported_without_submitting_another_frame() {
        let (sender, completed) = mpsc::channel();
        let size = RenderSize::clamped(320, 240);
        let stats = FrameStats {
            frame_index: 1,
            width: 320,
            height: 240,
            draw_commands: 2,
            cpu_frame_ms: 1.0,
        };
        let mut worker = RenderWorker {
            mailbox: RenderMailbox::default(),
            completed,
            size,
            scale: 1.0,
            stats,
            submitted: 3,
            finished: 0,
        };
        sender.send((3, Ok(stats))).unwrap();
        worker.poll_completed().unwrap();
        assert_eq!(worker.finished, worker.submitted);
        sender
            .send((4, Err(RenderError::WorkerDisconnected)))
            .unwrap();
        assert!(matches!(
            worker.poll_completed(),
            Err(RenderError::WorkerDisconnected)
        ));
        drop(sender);
        assert!(matches!(
            worker.poll_completed(),
            Err(RenderError::WorkerDisconnected)
        ));
    }
}
