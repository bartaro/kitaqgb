//! Window and event-loop layer for plita UI.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use plita_render::{ClearColor, GpuRenderer, RenderConfig, RenderError, RenderSize};
mod render_worker;
use plita_ui::{CreativeShell, DrawList, Point, PointerButton, Size, UiEvent, UiResponse};
use render_worker::WindowRenderer;
use thiserror::Error;
use tracing::{debug, error, info, trace, warn};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState},
    window::{Window, WindowAttributes, WindowId},
};

const CONTINUOUS_REDRAW_INTERVAL: Duration = Duration::from_nanos(16_666_667);
const MIN_CONTINUOUS_TICK_INTERVAL: Duration = Duration::from_micros(500);
const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_DISTANCE: f32 = 6.0;

#[derive(Clone, Copy, Debug)]
struct CompletedClick {
    at: Instant,
    position: Point,
    button: PointerButton,
}

#[derive(Clone, Copy, Debug, Default)]
struct DoubleClickDetector {
    pressed: Option<(Point, PointerButton)>,
    last_click: Option<CompletedClick>,
}

impl DoubleClickDetector {
    fn press(&mut self, position: Point, button: PointerButton) {
        self.pressed = Some((position, button));
    }

    fn release(&mut self, at: Instant, position: Point, button: PointerButton) -> bool {
        let completed_click = self
            .pressed
            .take()
            .is_some_and(|(pressed_at, pressed_button)| {
                pressed_button == button
                    && pointer_distance_squared(pressed_at, position)
                        <= DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE
            });
        if !completed_click {
            self.last_click = None;
            return false;
        }

        let is_double_click = self.last_click.is_some_and(|previous| {
            previous.button == button
                && at.saturating_duration_since(previous.at) <= DOUBLE_CLICK_INTERVAL
                && pointer_distance_squared(previous.position, position)
                    <= DOUBLE_CLICK_DISTANCE * DOUBLE_CLICK_DISTANCE
        });
        self.last_click = if is_double_click {
            None
        } else {
            Some(CompletedClick {
                at,
                position,
                button,
            })
        };
        is_double_click
    }

    fn clear(&mut self) {
        self.pressed = None;
        self.last_click = None;
    }
}

fn pointer_distance_squared(left: Point, right: Point) -> f32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

fn physical_position_to_ui(position: PhysicalPosition<f64>, scale_factor: f64) -> Point {
    let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    Point::new(
        (position.x / scale_factor) as f32,
        (position.y / scale_factor) as f32,
    )
}

/// Minimal app configuration needed by the Phase 1 window layer.
///
/// This mirrors the public `plita_app::AppConfig` shape without depending on that crate,
/// avoiding a circular dependency.
pub trait Phase0Config {
    /// Window title.
    fn title(&self) -> &str;
    /// Initial logical width.
    fn width(&self) -> u32;
    /// Initial logical height.
    fn height(&self) -> u32;
    /// Whether the initial main window is centered on its monitor.
    fn centered(&self) -> bool {
        false
    }
    /// Background clear color.
    fn clear_color(&self) -> ClearColor;
    /// Whether to continuously request redraws.
    fn continuous_redraw(&self) -> bool;
}

/// Concrete window configuration used internally by the window layer.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Initial logical width.
    pub width: u32,
    /// Initial logical height.
    pub height: u32,
    /// Whether the initial main window is centered on its monitor.
    pub centered: bool,
    /// Background clear color.
    pub clear_color: ClearColor,
    /// Whether to continuously request redraws.
    pub continuous_redraw: bool,
}

impl<T> From<T> for WindowConfig
where
    T: Phase0Config,
{
    fn from(value: T) -> Self {
        Self {
            title: value.title().to_string(),
            width: value.width().max(1),
            height: value.height().max(1),
            centered: value.centered(),
            clear_color: value.clear_color(),
            continuous_redraw: value.continuous_redraw(),
        }
    }
}

/// Window-layer errors.
#[derive(Debug, Error)]
pub enum WindowError {
    /// EventLoop creation or execution failed.
    #[error("winit event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    /// OS window creation failed.
    #[error("failed to create OS window: {0}")]
    Os(#[from] winit::error::OsError),

    /// Renderer initialization or frame rendering failed.
    #[error(transparent)]
    Render(#[from] RenderError),
}

/// Native auxiliary window requested by a `PlitaProgram`.
#[derive(Clone, Debug, PartialEq)]
pub struct AuxiliaryWindowDescriptor {
    /// Stable app-defined window id.
    pub id: u64,
    /// Native OS window title.
    pub title: String,
    /// Initial logical width.
    pub width: u32,
    /// Initial logical height.
    pub height: u32,
}

impl AuxiliaryWindowDescriptor {
    /// Create a descriptor with clamped nonzero size.
    pub fn new(id: u64, title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id,
            title: title.into(),
            width: width.max(1),
            height: height.max(1),
        }
    }
}

/// Trait implemented by custom plita applications.
///
/// The window layer owns the OS event loop and renderer. A program owns its
/// domain state, consumes normalized UI events, and emits a `DrawList`.
pub trait PlitaProgram {
    /// Minimum logical client size; None leaves resizing unconstrained.
    fn minimum_window_size(&self) -> Option<Size> {
        None
    }
    /// Some(true) requests borderless fullscreen; None leaves window state unmanaged.
    fn fullscreen_requested(&self) -> Option<bool> {
        None
    }
    /// Return false to cancel closing, for example when a document is unsaved.
    fn close_requested(&mut self) -> bool {
        true
    }

    /// Called when the window is resized.
    fn resize(&mut self, size: Size) -> UiResponse;

    /// Called for pointer, keyboard, IME, and file-drop events.
    fn handle_event(&mut self, event: UiEvent) -> UiResponse;

    /// Whether OS-generated repeated key-down events should reach the program.
    /// Programs emulating a physical keyboard should return false and derive
    /// repeat behavior from the guest while the original key remains held.
    fn accepts_key_repeat(&self) -> bool {
        true
    }

    /// Interval used to update continuous program state. This is independent
    /// from the 60 Hz draw cadence so emulators and audio tools can service
    /// input without rendering an extra frame for every update.
    fn continuous_tick_interval(&self) -> Duration {
        CONTINUOUS_REDRAW_INTERVAL
    }

    /// Opt in to coalesced worker-thread GPU rendering and presentation.
    /// Window/surface creation, input, ticks and draw-list construction stay on
    /// the event-loop thread; the program does not need to implement `Send`.
    /// The default is synchronous. This policy is read at renderer creation
    /// for both main and auxiliary windows, not as a runtime toggle.
    fn render_on_worker_thread(&self) -> bool {
        false
    }

    /// Whether a continuous program should redraw even when `tick` reports no
    /// visual change. Static tool surfaces can return false to avoid needless
    /// GPU work while still receiving their configured continuous ticks.
    fn redraw_when_unchanged(&self) -> bool {
        true
    }

    /// Called before a redraw. Continuous applications can update animations or
    /// emulation here and request a fresh draw list.
    fn tick(&mut self) -> UiResponse {
        UiResponse::none()
    }

    /// Build the draw list for the current state.
    fn draw_list(&self) -> DrawList;

    /// Auxiliary native windows requested by this program.
    fn auxiliary_windows(&self) -> Vec<AuxiliaryWindowDescriptor> {
        Vec::new()
    }

    /// Minimum logical client size for an auxiliary window.
    fn auxiliary_window_minimum_size(&self, _id: u64) -> Option<Size> {
        None
    }

    /// Change this token for each intentional restore/focus request.
    /// A stable token never repeatedly steals focus. None requests no activation.
    fn auxiliary_window_activation_request(&self, _id: u64) -> Option<u64> {
        None
    }

    /// Called when an auxiliary window is resized.
    fn resize_auxiliary_window(&mut self, _id: u64, _size: Size) -> UiResponse {
        UiResponse::none()
    }

    /// Called for events dispatched to an auxiliary window.
    fn handle_auxiliary_event(&mut self, _id: u64, _event: UiEvent) -> UiResponse {
        UiResponse::none()
    }

    /// Called when the user closes an auxiliary window.
    fn close_auxiliary_window(&mut self, _id: u64) -> UiResponse {
        UiResponse::none()
    }

    /// Called when an auxiliary renderer cannot be initialized.
    /// This is distinct from the user requesting that the window close.
    /// Existing programs retain their close behavior unless they override it.
    fn auxiliary_window_failed(&mut self, id: u64) -> UiResponse {
        self.close_auxiliary_window(id)
    }

    /// Build the draw list for an auxiliary window.
    fn auxiliary_draw_list(&self, _id: u64) -> Option<DrawList> {
        None
    }

    /// Opt in to retaining an auxiliary draw list until this revision changes.
    /// Return None for continuously animated/shared-image windows. Programs
    /// returning Some must change the revision and request redraw when content
    /// changes, including hover/pressed states. Native resize forces a rebuild.
    fn auxiliary_draw_revision(&self, _id: u64) -> Option<u64> {
        None
    }
}

impl PlitaProgram for CreativeShell {
    fn resize(&mut self, size: Size) -> UiResponse {
        CreativeShell::resize(self, size)
    }

    fn handle_event(&mut self, event: UiEvent) -> UiResponse {
        CreativeShell::handle_event(self, event)
    }

    fn draw_list(&self) -> DrawList {
        CreativeShell::draw_list(self)
    }

    fn auxiliary_windows(&self) -> Vec<AuxiliaryWindowDescriptor> {
        if !self.memory_window_open() {
            return Vec::new();
        }
        let size = self.memory_window_size();
        vec![AuxiliaryWindowDescriptor::new(
            plita_ui::CREATIVE_MEMORY_WINDOW_ID,
            "plita Memory Editor",
            size.width.round() as u32,
            size.height.round() as u32,
        )]
    }

    fn resize_auxiliary_window(&mut self, id: u64, size: Size) -> UiResponse {
        if id == plita_ui::CREATIVE_MEMORY_WINDOW_ID {
            self.resize_memory_window(size)
        } else {
            UiResponse::none()
        }
    }

    fn handle_auxiliary_event(&mut self, id: u64, event: UiEvent) -> UiResponse {
        if id == plita_ui::CREATIVE_MEMORY_WINDOW_ID {
            self.handle_memory_window_event(event)
        } else {
            UiResponse::none()
        }
    }

    fn close_auxiliary_window(&mut self, id: u64) -> UiResponse {
        if id == plita_ui::CREATIVE_MEMORY_WINDOW_ID {
            self.close_memory_window()
        } else {
            UiResponse::none()
        }
    }

    fn auxiliary_draw_list(&self, id: u64) -> Option<DrawList> {
        (id == plita_ui::CREATIVE_MEMORY_WINDOW_ID).then(|| self.memory_window_draw_list())
    }
}

/// Launch the Phase 1 windowed application.
pub fn run_phase1<C>(config: C) -> Result<(), WindowError>
where
    C: Phase0Config,
{
    run_program(config, CreativeShell::new)
}

/// Launch a custom plita program in the native window loop.
pub fn run_program<C, F, P>(config: C, factory: F) -> Result<(), WindowError>
where
    C: Phase0Config,
    F: FnOnce(Size) -> P + 'static,
    P: PlitaProgram + 'static,
{
    let config = WindowConfig::from(config);
    let event_loop = new_event_loop()?;

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = Phase1WindowApp::new(config, factory);
    event_loop.run_app(&mut app)?;

    if let Some(err) = app.take_startup_error() {
        return Err(err);
    }

    Ok(())
}

fn new_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    builder.build()
}

fn next_periodic_deadline(mut deadline: Instant, now: Instant, interval: Duration) -> Instant {
    let interval = interval.max(MIN_CONTINUOUS_TICK_INTERVAL);
    deadline += interval;
    while deadline <= now {
        deadline += interval;
    }
    deadline
}

fn next_continuous_redraw_deadline(deadline: Instant, now: Instant) -> Instant {
    next_periodic_deadline(deadline, now, CONTINUOUS_REDRAW_INTERVAL)
}

/// Backward-compatible Phase 0 entry point.
///
/// The implementation now runs the Phase 1 UI shell while keeping the old symbol
/// available for early examples and tests.
pub fn run_phase0<C>(config: C) -> Result<(), WindowError>
where
    C: Phase0Config,
{
    run_phase1(config)
}

struct Phase1WindowApp {
    config: WindowConfig,
    program_factory: Option<Box<dyn FnOnce(Size) -> Box<dyn PlitaProgram>>>,
    window: Option<Arc<Window>>,
    renderer: Option<WindowRenderer>,
    auxiliary_windows: Vec<AuxiliaryWindowState>,
    pending_auxiliary_windows: Vec<PendingAuxiliaryWindow>,
    failed_auxiliary_windows: Vec<(u64, Option<u64>)>,
    ui: Option<Box<dyn PlitaProgram>>,
    last_pointer_position: Point,
    double_click: DoubleClickDetector,
    modifiers: ModifiersState,
    ctrl_key_down: bool,
    shift_key_down: bool,
    alt_key_down: bool,
    meta_key_down: bool,
    next_continuous_tick_at: Instant,
    next_continuous_redraw_at: Instant,
    continuous_draw_dirty: bool,
    startup_error: Option<WindowError>,
}

struct AuxiliaryWindowState {
    id: u64,
    window: Arc<Window>,
    renderer: WindowRenderer,
    last_pointer_position: Point,
    double_click: DoubleClickDetector,
    activation_request: Option<u64>,
    draw_revision: Option<u64>,
    draw_dirty: bool,
}

fn auxiliary_draw_list_changed(previous: Option<u64>, current: Option<u64>) -> bool {
    current.is_none() || previous != current
}

fn take_auxiliary_activation(last: &mut Option<u64>, requested: Option<u64>) -> bool {
    let activate = requested.is_some() && requested != *last;
    *last = requested;
    activate
}

struct PendingAuxiliaryWindow {
    id: u64,
    window: Arc<Window>,
    renderer: std::sync::mpsc::Receiver<Result<GpuRenderer, RenderError>>,
}

impl Phase1WindowApp {
    fn new<F, P>(config: WindowConfig, factory: F) -> Self
    where
        F: FnOnce(Size) -> P + 'static,
        P: PlitaProgram + 'static,
    {
        Self {
            config,
            program_factory: Some(Box::new(move |size| Box::new(factory(size)))),
            window: None,
            renderer: None,
            auxiliary_windows: Vec::new(),
            pending_auxiliary_windows: Vec::new(),
            failed_auxiliary_windows: Vec::new(),
            ui: None,
            last_pointer_position: Point::new(0.0, 0.0),
            double_click: DoubleClickDetector::default(),
            modifiers: ModifiersState::empty(),
            ctrl_key_down: false,
            shift_key_down: false,
            alt_key_down: false,
            meta_key_down: false,
            next_continuous_tick_at: Instant::now(),
            next_continuous_redraw_at: Instant::now(),
            continuous_draw_dirty: false,
            startup_error: None,
        }
    }

    fn take_startup_error(&mut self) -> Option<WindowError> {
        self.startup_error.take()
    }

    fn reset_modifier_key_state(&mut self) {
        self.modifiers = ModifiersState::empty();
        self.ctrl_key_down = false;
        self.shift_key_down = false;
        self.alt_key_down = false;
        self.meta_key_down = false;
    }

    fn update_modifier_key_state(&mut self, event: &winit::event::KeyEvent) {
        let down = event.state == ElementState::Pressed;
        let logical = format!("{:?}", event.logical_key);
        let physical = format!("{:?}", event.physical_key);
        let marker = format!("{}|{}", logical, physical).to_ascii_uppercase();

        if marker.contains("CONTROL") {
            self.ctrl_key_down = down;
        }
        if marker.contains("SHIFT") {
            self.shift_key_down = down;
        }
        if marker.contains("ALT") {
            self.alt_key_down = down;
        }
        if marker.contains("SUPER") || marker.contains("META") {
            self.meta_key_down = down;
        }
    }

    fn effective_ctrl_key(&self) -> bool {
        self.modifiers.control_key() || self.ctrl_key_down
    }

    fn effective_shift_key(&self) -> bool {
        self.modifiers.shift_key() || self.shift_key_down
    }

    fn effective_alt_key(&self) -> bool {
        self.modifiers.alt_key() || self.alt_key_down
    }

    fn effective_meta_key(&self) -> bool {
        self.modifiers.super_key() || self.meta_key_down
    }

    fn window_attributes(&self) -> WindowAttributes {
        Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(
                f64::from(self.config.width),
                f64::from(self.config.height),
            ))
            // Keep it hidden until GPU initialization is complete to avoid uninitialized contents.
            .with_visible(false)
    }

    fn auxiliary_window_attributes(
        &self,
        descriptor: &AuxiliaryWindowDescriptor,
    ) -> WindowAttributes {
        let mut attributes = Window::default_attributes()
            .with_title(descriptor.title.clone())
            .with_inner_size(LogicalSize::new(
                f64::from(descriptor.width),
                f64::from(descriptor.height),
            ))
            .with_visible(false);
        if let Some(size) = self
            .ui
            .as_ref()
            .and_then(|ui| ui.auxiliary_window_minimum_size(descriptor.id))
        {
            attributes = attributes.with_min_inner_size(LogicalSize::new(
                f64::from(size.width.max(1.0)),
                f64::from(size.height.max(1.0)),
            ));
        }
        attributes
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), WindowError> {
        if self.window.is_some() {
            return Ok(());
        }

        let window = Arc::new(event_loop.create_window(self.window_attributes())?);
        if self.config.centered {
            center_window_on_monitor(&window, event_loop);
        }
        // Keep IME available for the retained TextInput prototype. The UI layer
        // accepts IME commits only while its TextInput has focus, so enabling IME
        // here is safe and lets Japanese input work on Windows.
        window.set_ime_allowed(true);
        let scale_factor = window.scale_factor();
        let inner_size = window.inner_size();

        info!(
            title = %self.config.title,
            width = inner_size.width,
            height = inner_size.height,
            scale_factor,
            "window created"
        );

        let mut renderer = pollster::block_on(GpuRenderer::new(
            window.clone(),
            RenderConfig {
                clear_color: self.config.clear_color,
            },
        ))?;
        let factory = self
            .program_factory
            .take()
            .expect("program factory is used once");
        let ui = factory(renderer.ui_size());
        if let Some(size) = ui.minimum_window_size() {
            window.set_min_inner_size(Some(LogicalSize::new(
                f64::from(size.width.max(1.0)),
                f64::from(size.height.max(1.0)),
            )));
        }
        renderer.set_draw_list(ui.draw_list());

        info!(
            surface_format = ?renderer.surface_format(),
            surface_supported = renderer.is_surface_supported(),
            "renderer attached to window"
        );

        let renderer = WindowRenderer::new(renderer, ui.render_on_worker_thread(), scale_factor);

        window.set_visible(true);
        window.focus_window();
        window.request_redraw();

        self.ui = Some(ui);
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.sync_auxiliary_windows(event_loop)?;
        self.rebuild_all_draw_lists();
        self.request_redraw_all();
        Ok(())
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn request_redraw_all(&self) {
        self.request_redraw();
        for state in &self.auxiliary_windows {
            state.window.request_redraw();
        }
    }

    fn auxiliary_index_for_window(&self, id: WindowId) -> Option<usize> {
        self.auxiliary_windows
            .iter()
            .position(|state| state.window.id() == id)
    }

    fn fail_auxiliary_window(&mut self, id: u64) {
        if !self
            .failed_auxiliary_windows
            .iter()
            .any(|(failed, _)| *failed == id)
        {
            let request = self
                .ui
                .as_ref()
                .and_then(|ui| ui.auxiliary_window_activation_request(id));
            self.failed_auxiliary_windows.push((id, request));
            if let Some(ui) = self.ui.as_mut() {
                ui.auxiliary_window_failed(id);
            }
        }
    }

    fn discard_obsolete_auxiliary_failures(&mut self, descriptors: &[AuxiliaryWindowDescriptor]) {
        self.failed_auxiliary_windows.retain(|(id, request)| {
            descriptors.iter().any(|descriptor| descriptor.id == *id)
                && self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.auxiliary_window_activation_request(*id))
                    == *request
        });
    }

    fn sync_auxiliary_windows(&mut self, event_loop: &ActiveEventLoop) -> Result<(), WindowError> {
        if let (Some(window), Some(requested)) = (
            &self.window,
            self.ui.as_ref().and_then(|ui| ui.fullscreen_requested()),
        ) {
            if window.fullscreen().is_some() != requested {
                window
                    .set_fullscreen(requested.then(|| winit::window::Fullscreen::Borderless(None)));
            }
        }
        let descriptors = self
            .ui
            .as_ref()
            .map(|ui| ui.auxiliary_windows())
            .unwrap_or_default();
        // Retry only after withdrawal or an explicit new activation request.
        self.discard_obsolete_auxiliary_failures(&descriptors);
        self.auxiliary_windows.retain(|state| {
            descriptors
                .iter()
                .any(|descriptor| descriptor.id == state.id)
        });

        self.pending_auxiliary_windows
            .retain(|state| descriptors.iter().any(|d| d.id == state.id));
        let mut index = 0;
        while index < self.pending_auxiliary_windows.len() {
            let result = match self.pending_auxiliary_windows[index].renderer.try_recv() {
                Ok(result) => result,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    index += 1;
                    continue;
                }
                Err(_) => {
                    let state = self.pending_auxiliary_windows.remove(index);
                    error!(id = state.id, "auxiliary renderer worker disconnected");
                    self.fail_auxiliary_window(state.id);
                    return Ok(());
                }
            };
            let state = self.pending_auxiliary_windows.remove(index);
            let mut renderer = match result {
                Ok(renderer) => renderer,
                Err(err) => {
                    error!(id = state.id, error = %err, "auxiliary renderer initialization failed");
                    self.fail_auxiliary_window(state.id);
                    return Ok(());
                }
            };
            renderer.set_ui_scale_factor(state.window.scale_factor());
            renderer.resize(state.window.inner_size().into());
            if let Some(list) = self
                .ui
                .as_ref()
                .and_then(|ui| ui.auxiliary_draw_list(state.id))
            {
                renderer.set_draw_list(list);
            }
            info!(id = state.id, surface_format = ?renderer.surface_format(),
                "auxiliary renderer attached to window");
            let renderer = WindowRenderer::new(
                renderer,
                self.ui
                    .as_ref()
                    .is_some_and(|ui| ui.render_on_worker_thread()),
                state.window.scale_factor(),
            );
            state.window.set_visible(true);
            state.window.request_redraw();
            self.auxiliary_windows.push(AuxiliaryWindowState {
                id: state.id,
                window: state.window,
                renderer,
                last_pointer_position: Point::new(0., 0.),
                double_click: DoubleClickDetector::default(),
                activation_request: None,
                draw_revision: self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.auxiliary_draw_revision(state.id)),
                draw_dirty: false,
            });
        }

        for state in &mut self.auxiliary_windows {
            let request = self
                .ui
                .as_ref()
                .and_then(|ui| ui.auxiliary_window_activation_request(state.id));
            if take_auxiliary_activation(&mut state.activation_request, request) {
                state.window.set_visible(true);
                state.window.set_minimized(false);
                state.window.focus_window();
                state.window.request_redraw();
            }
        }

        for descriptor in descriptors {
            if self
                .failed_auxiliary_windows
                .iter()
                .any(|(id, _)| *id == descriptor.id)
                || self
                    .pending_auxiliary_windows
                    .iter()
                    .any(|s| s.id == descriptor.id)
                || self
                    .auxiliary_windows
                    .iter()
                    .any(|state| state.id == descriptor.id)
            {
                continue;
            }

            let window = match event_loop
                .create_window(self.auxiliary_window_attributes(&descriptor))
            {
                Ok(window) => Arc::new(window),
                Err(err) => {
                    error!(id = descriptor.id, error = %err, "auxiliary window creation failed");
                    self.fail_auxiliary_window(descriptor.id);
                    continue;
                }
            };
            window.set_ime_allowed(true);
            let prepared = match GpuRenderer::prepare_surface(Arc::clone(&window)) {
                Ok(prepared) => prepared,
                Err(err) => {
                    error!(id = descriptor.id, error = %err, "auxiliary surface creation failed");
                    self.fail_auxiliary_window(descriptor.id);
                    continue;
                }
            };
            let (tx, rx) = std::sync::mpsc::channel();
            let config = RenderConfig {
                clear_color: self.config.clear_color,
            };
            // Adapter/device and pipeline creation must not suspend the parent's audio producer.
            std::thread::spawn(move || {
                let _ = tx.send(pollster::block_on(GpuRenderer::from_prepared_surface(
                    prepared, config,
                )));
            });
            self.pending_auxiliary_windows.push(PendingAuxiliaryWindow {
                id: descriptor.id,
                window,
                renderer: rx,
            });
        }
        Ok(())
    }

    fn sync_auxiliary_windows_or_exit(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(err) = self.sync_auxiliary_windows(event_loop) {
            error!(error = %err, "auxiliary window synchronization failed");
            self.startup_error = Some(err);
            event_loop.exit();
        }
    }

    fn request_continuous_redraw_if_due(&mut self, event_loop: &ActiveEventLoop) {
        if !self.config.continuous_redraw {
            return;
        }

        let now = Instant::now();
        let tick_interval = self
            .ui
            .as_ref()
            .map(|ui| ui.continuous_tick_interval())
            .unwrap_or(CONTINUOUS_REDRAW_INTERVAL)
            .max(MIN_CONTINUOUS_TICK_INTERVAL);
        if now >= self.next_continuous_tick_at {
            let tick_dirty = self.tick_ui(event_loop);
            self.continuous_draw_dirty |= tick_dirty;
            self.next_continuous_tick_at =
                next_periodic_deadline(self.next_continuous_tick_at, now, tick_interval);
        }
        if now >= self.next_continuous_redraw_at {
            let should_redraw = self.continuous_draw_dirty
                || self
                    .ui
                    .as_ref()
                    .map(|ui| ui.redraw_when_unchanged())
                    .unwrap_or(true);
            if self.continuous_draw_dirty {
                self.rebuild_all_draw_lists();
                self.continuous_draw_dirty = false;
            }
            if should_redraw {
                self.request_redraw();
                for state in &mut self.auxiliary_windows {
                    if state.draw_dirty || state.draw_revision.is_none() {
                        state.window.request_redraw();
                        state.draw_dirty = false;
                    }
                }
            }
            self.next_continuous_redraw_at =
                next_continuous_redraw_deadline(self.next_continuous_redraw_at, now);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            self.next_continuous_tick_at
                .min(self.next_continuous_redraw_at),
        ));
    }

    fn wait_for_next_continuous_redraw(&self, event_loop: &ActiveEventLoop) {
        if self.config.continuous_redraw {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                self.next_continuous_tick_at
                    .min(self.next_continuous_redraw_at),
            ));
        }
    }

    fn dispatch_ui_event(&mut self, event_loop: &ActiveEventLoop, event: UiEvent) {
        let mut needs_redraw = false;
        if let Some(ui) = self.ui.as_mut() {
            let response = ui.handle_event(event);
            if response.needs_redraw {
                needs_redraw = true;
            }
        }
        self.sync_auxiliary_windows_or_exit(event_loop);

        if needs_redraw {
            self.rebuild_all_draw_lists();
            self.request_redraw_all();
        }
    }

    fn dispatch_auxiliary_ui_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: u64,
        event: UiEvent,
    ) {
        let mut needs_redraw = false;
        if let Some(ui) = self.ui.as_mut() {
            let response = ui.handle_auxiliary_event(id, event);
            if response.needs_redraw {
                needs_redraw = true;
            }
        }
        self.sync_auxiliary_windows_or_exit(event_loop);

        if needs_redraw {
            self.rebuild_all_draw_lists();
            self.request_redraw_all();
        }
    }

    fn rebuild_all_draw_lists(&mut self) {
        let Some(ui) = self.ui.as_ref() else {
            return;
        };
        let main_draw_list = ui.draw_list();
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_draw_list(main_draw_list);
        }
        for state in &mut self.auxiliary_windows {
            let revision = ui.auxiliary_draw_revision(state.id);
            if !auxiliary_draw_list_changed(state.draw_revision, revision) {
                continue;
            }
            if let Some(draw_list) = ui.auxiliary_draw_list(state.id) {
                state.renderer.set_draw_list(draw_list);
                state.draw_revision = revision;
                state.draw_dirty = true;
            }
        }
    }

    fn tick_ui(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let mut needs_redraw = false;
        if let Some(ui) = self.ui.as_mut() {
            let response = ui.tick();
            needs_redraw = response.needs_redraw;
            self.sync_auxiliary_windows_or_exit(event_loop);
        }
        needs_redraw
    }

    fn handle_auxiliary_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        index: usize,
        event: WindowEvent,
    ) {
        let auxiliary_id = self.auxiliary_windows[index].id;
        match event {
            WindowEvent::CloseRequested => {
                info!(id = auxiliary_id, "auxiliary close requested");
                if let Some(ui) = self.ui.as_mut() {
                    ui.close_auxiliary_window(auxiliary_id);
                }
                self.sync_auxiliary_windows_or_exit(event_loop);
                self.rebuild_all_draw_lists();
                self.request_redraw_all();
            }
            WindowEvent::RedrawRequested => {
                if self.auxiliary_windows[index].window.is_minimized() == Some(true) {
                    self.wait_for_next_continuous_redraw(event_loop);
                    return;
                }
                let render_result = {
                    let state = &mut self.auxiliary_windows[index];
                    state.renderer.render()
                };
                match render_result {
                    Ok(stats) if stats.frame_index == 1 => {
                        info!(
                            id = auxiliary_id,
                            width = stats.width,
                            height = stats.height,
                            draw_commands = stats.draw_commands,
                            "auxiliary first frame presented"
                        );
                    }
                    Ok(_) => {}
                    Err(err) => self.handle_render_error(event_loop, err),
                }
                self.wait_for_next_continuous_redraw(event_loop);
            }
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }
                debug!(
                    id = auxiliary_id,
                    width = size.width,
                    height = size.height,
                    "auxiliary window resized"
                );
                let ui_size = {
                    let state = &mut self.auxiliary_windows[index];
                    state.draw_revision = None;
                    state
                        .renderer
                        .set_ui_scale_factor(state.window.scale_factor());
                    state.renderer.resize(RenderSize::from(size));
                    state.renderer.ui_size()
                };
                if let Some(ui) = self.ui.as_mut() {
                    ui.resize_auxiliary_window(auxiliary_id, ui_size);
                }
                self.sync_auxiliary_windows_or_exit(event_loop);
                self.rebuild_all_draw_lists();
                self.request_redraw_all();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                debug!(
                    id = auxiliary_id,
                    scale_factor, "auxiliary scale factor changed"
                );
                let ui_size = {
                    let state = &mut self.auxiliary_windows[index];
                    state.draw_revision = None;
                    state.renderer.set_ui_scale_factor(scale_factor);
                    state.renderer.ui_size()
                };
                if let Some(ui) = self.ui.as_mut() {
                    ui.resize_auxiliary_window(auxiliary_id, ui_size);
                }
                self.rebuild_all_draw_lists();
                self.request_redraw_all();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let position = physical_position_to_ui(
                    position,
                    self.auxiliary_windows[index].window.scale_factor(),
                );
                if let Some(state) = self.auxiliary_windows.get_mut(index) {
                    state.last_pointer_position = position;
                }
                self.dispatch_auxiliary_ui_event(
                    event_loop,
                    auxiliary_id,
                    UiEvent::PointerMove { position },
                );
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let position = self.auxiliary_windows[index].last_pointer_position;
                let button = map_pointer_button(button);
                match state {
                    ElementState::Pressed => {
                        self.auxiliary_windows[index]
                            .double_click
                            .press(position, button);
                        self.dispatch_auxiliary_ui_event(
                            event_loop,
                            auxiliary_id,
                            UiEvent::PointerDown { position, button },
                        );
                    }
                    ElementState::Released => {
                        let is_double_click = self.auxiliary_windows[index].double_click.release(
                            Instant::now(),
                            position,
                            button,
                        );
                        self.dispatch_auxiliary_ui_event(
                            event_loop,
                            auxiliary_id,
                            UiEvent::PointerUp { position, button },
                        );
                        if is_double_click {
                            self.dispatch_auxiliary_ui_event(
                                event_loop,
                                auxiliary_id,
                                UiEvent::PointerDoubleClick { position, button },
                            );
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let (delta_x, delta_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        debug!(
                            id = auxiliary_id,
                            x,
                            y,
                            ?phase,
                            "auxiliary mouse wheel line delta"
                        );
                        (x * 32.0, y * 32.0)
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        debug!(
                            id = auxiliary_id,
                            x = pos.x,
                            y = pos.y,
                            ?phase,
                            "auxiliary mouse wheel pixel delta"
                        );
                        let scale_factor = self.auxiliary_windows[index].window.scale_factor();
                        let logical = physical_position_to_ui(pos, scale_factor);
                        (logical.x, logical.y)
                    }
                };
                self.dispatch_auxiliary_ui_event(
                    event_loop,
                    auxiliary_id,
                    UiEvent::PointerWheel {
                        position: self.auxiliary_windows[index].last_pointer_position,
                        delta_x,
                        delta_y,
                    },
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                log_keyboard_event(&event);
                self.update_modifier_key_state(&event);
                let ctrl = self.effective_ctrl_key();
                let shift = self.effective_shift_key();
                let alt = self.effective_alt_key();
                let meta = self.effective_meta_key();
                let dispatch_zero_release = should_dispatch_zero_shortcut_on_release(&event, ctrl);
                let accepts_repeat = self.ui.as_ref().is_none_or(|ui| ui.accepts_key_repeat());
                if should_dispatch_key_down(
                    event.state,
                    event.repeat,
                    dispatch_zero_release,
                    accepts_repeat,
                ) {
                    let key = ui_key_string(&event, ctrl, shift);
                    self.dispatch_auxiliary_ui_event(
                        event_loop,
                        auxiliary_id,
                        UiEvent::KeyDown {
                            key,
                            ctrl,
                            shift,
                            alt,
                            meta,
                        },
                    );
                } else if event.state == ElementState::Released {
                    let key = ui_key_string(&event, ctrl, shift);
                    self.dispatch_auxiliary_ui_event(
                        event_loop,
                        auxiliary_id,
                        UiEvent::KeyUp {
                            key,
                            ctrl,
                            shift,
                            alt,
                            meta,
                        },
                    );
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                trace!(
                    id = auxiliary_id,
                    ?modifiers,
                    state = ?self.modifiers,
                    "auxiliary modifiers changed"
                );
            }
            WindowEvent::Ime(ime) => {
                log_ime_event(&ime);
                match ime {
                    Ime::Preedit(text, _) => {
                        self.dispatch_auxiliary_ui_event(
                            event_loop,
                            auxiliary_id,
                            UiEvent::ImePreedit { text },
                        );
                    }
                    Ime::Commit(text) => {
                        self.dispatch_auxiliary_ui_event(
                            event_loop,
                            auxiliary_id,
                            UiEvent::ImeCommit { text },
                        );
                    }
                    Ime::Enabled | Ime::Disabled => {
                        self.request_redraw_all();
                    }
                }
            }
            WindowEvent::DroppedFile(path) => {
                info!(id = auxiliary_id, path = %path.display(), "auxiliary file dropped");
                self.dispatch_auxiliary_ui_event(
                    event_loop,
                    auxiliary_id,
                    UiEvent::FileDropped { path },
                );
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.reset_modifier_key_state();
                    self.auxiliary_windows[index].double_click.clear();
                }
                self.dispatch_auxiliary_ui_event(
                    event_loop,
                    auxiliary_id,
                    UiEvent::FocusChanged { focused },
                );
            }
            _ => {}
        }
    }

    fn handle_render_error(&mut self, event_loop: &ActiveEventLoop, err: RenderError) {
        if err.is_surface_timeout() {
            warn!("surface timeout; will retry on next redraw");
            self.request_redraw();
            return;
        }

        if err.is_surface_out_of_memory() {
            error!("GPU out of memory; exiting");
            event_loop.exit();
            return;
        }

        error!(error = %err, "rendering failed; exiting");
        self.startup_error = Some(WindowError::Render(err));
        event_loop.exit();
    }
}

fn centered_axis_position(monitor_origin: i32, monitor_extent: u32, window_extent: u32) -> i32 {
    monitor_origin + i32::try_from(monitor_extent.saturating_sub(window_extent) / 2).unwrap_or(0)
}

fn center_window_on_monitor(window: &Window, event_loop: &ActiveEventLoop) {
    let Some(monitor) = window
        .current_monitor()
        .or_else(|| event_loop.primary_monitor())
    else {
        return;
    };
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size();
    window.set_outer_position(PhysicalPosition::new(
        centered_axis_position(monitor_position.x, monitor_size.width, window_size.width),
        centered_axis_position(monitor_position.y, monitor_size.height, window_size.height),
    ));
}

impl ApplicationHandler for Phase1WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(err) = self.initialize(event_loop) {
            error!(error = %err, "application initialization failed");
            self.startup_error = Some(err);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let is_main_window = self.window.as_ref().is_some_and(|window| id == window.id());
        if !is_main_window {
            if let Some(index) = self.auxiliary_index_for_window(id) {
                self.handle_auxiliary_window_event(event_loop, index, event);
            }
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                info!("close requested");
                if self.ui.as_mut().is_none_or(|ui| ui.close_requested()) {
                    event_loop.exit();
                } else {
                    self.rebuild_all_draw_lists();
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.config.continuous_redraw && self.tick_ui(event_loop) {
                    self.rebuild_all_draw_lists();
                }
                if self
                    .window
                    .as_ref()
                    .is_some_and(|window| window.is_minimized() == Some(true))
                {
                    self.wait_for_next_continuous_redraw(event_loop);
                    return;
                }
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render() {
                        Ok(stats) => {
                            trace!(
                                frame = stats.frame_index,
                                width = stats.width,
                                height = stats.height,
                                draw_commands = stats.draw_commands,
                                cpu_frame_ms = stats.cpu_frame_ms,
                                "redraw completed"
                            );
                        }
                        Err(err) => self.handle_render_error(event_loop, err),
                    }
                }

                self.wait_for_next_continuous_redraw(event_loop);
            }
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }
                debug!(width = size.width, height = size.height, "window resized");
                let mut ui_size = None;
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_ui_scale_factor(scale_factor);
                    renderer.resize(RenderSize::from(size));
                    ui_size = Some(renderer.ui_size());
                }
                if let (Some(ui), Some(size)) = (self.ui.as_mut(), ui_size) {
                    ui.resize(size);
                }
                self.sync_auxiliary_windows_or_exit(event_loop);
                self.rebuild_all_draw_lists();
                self.request_redraw_all();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                debug!(scale_factor, "scale factor changed");
                let mut ui_size = None;
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.set_ui_scale_factor(scale_factor);
                    ui_size = Some(renderer.ui_size());
                }
                if let (Some(ui), Some(size)) = (self.ui.as_mut(), ui_size) {
                    ui.resize(size);
                }
                self.sync_auxiliary_windows_or_exit(event_loop);
                self.rebuild_all_draw_lists();
                self.request_redraw_all();
            }
            WindowEvent::CursorMoved { position, .. } => {
                trace!(x = position.x, y = position.y, "cursor moved");
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);
                let position = physical_position_to_ui(position, scale_factor);
                self.last_pointer_position = position;
                self.dispatch_ui_event(event_loop, UiEvent::PointerMove { position });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                trace!(?state, ?button, "mouse input");
                let position = self.last_pointer_position;
                let button = map_pointer_button(button);
                match state {
                    ElementState::Pressed => {
                        self.double_click.press(position, button);
                        self.dispatch_ui_event(
                            event_loop,
                            UiEvent::PointerDown { position, button },
                        );
                    }
                    ElementState::Released => {
                        let is_double_click =
                            self.double_click.release(Instant::now(), position, button);
                        self.dispatch_ui_event(event_loop, UiEvent::PointerUp { position, button });
                        if is_double_click {
                            self.dispatch_ui_event(
                                event_loop,
                                UiEvent::PointerDoubleClick { position, button },
                            );
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let (delta_x, delta_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        debug!(x, y, ?phase, "mouse wheel line delta");
                        (x * 32.0, y * 32.0)
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        debug!(x = pos.x, y = pos.y, ?phase, "mouse wheel pixel delta");
                        let scale_factor = self
                            .window
                            .as_ref()
                            .map(|window| window.scale_factor())
                            .unwrap_or(1.0);
                        let logical = physical_position_to_ui(pos, scale_factor);
                        (logical.x, logical.y)
                    }
                };
                self.dispatch_ui_event(
                    event_loop,
                    UiEvent::PointerWheel {
                        position: self.last_pointer_position,
                        delta_x,
                        delta_y,
                    },
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                log_keyboard_event(&event);
                // Keep our own modifier key state in addition to winit's cached
                // ModifiersChanged state.  On some Windows/JIS paths, the
                // KeyboardInput for top-row Ctrl+Shift+0 can arrive while the
                // cached modifier state is incomplete; direct key-state tracking
                // keeps shortcut dispatch stable.
                self.update_modifier_key_state(&event);
                let ctrl = self.effective_ctrl_key();
                let shift = self.effective_shift_key();
                let alt = self.effective_alt_key();
                let meta = self.effective_meta_key();
                let dispatch_zero_release = should_dispatch_zero_shortcut_on_release(&event, ctrl);
                let accepts_repeat = self.ui.as_ref().is_none_or(|ui| ui.accepts_key_repeat());
                if should_dispatch_key_down(
                    event.state,
                    event.repeat,
                    dispatch_zero_release,
                    accepts_repeat,
                ) {
                    let key = ui_key_string(&event, ctrl, shift);
                    trace!(
                        key = %key,
                        ctrl,
                        shift,
                        alt,
                        meta,
                        dispatch_zero_release,
                        "dispatching ui keydown"
                    );
                    self.dispatch_ui_event(
                        event_loop,
                        UiEvent::KeyDown {
                            key,
                            ctrl,
                            shift,
                            alt,
                            meta,
                        },
                    );
                } else if event.state == ElementState::Released {
                    let key = ui_key_string(&event, ctrl, shift);
                    self.dispatch_ui_event(
                        event_loop,
                        UiEvent::KeyUp {
                            key,
                            ctrl,
                            shift,
                            alt,
                            meta,
                        },
                    );
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                trace!(?modifiers, state = ?self.modifiers, "modifiers changed");
            }
            WindowEvent::Ime(ime) => {
                log_ime_event(&ime);
                match ime {
                    Ime::Preedit(text, _) => {
                        self.dispatch_ui_event(event_loop, UiEvent::ImePreedit { text });
                    }
                    Ime::Commit(text) => {
                        self.dispatch_ui_event(event_loop, UiEvent::ImeCommit { text });
                    }
                    Ime::Enabled | Ime::Disabled => {
                        self.request_redraw_all();
                    }
                }
            }
            WindowEvent::DroppedFile(path) => {
                info!(path = %path.display(), "file dropped");
                self.dispatch_ui_event(event_loop, UiEvent::FileDropped { path });
            }
            WindowEvent::HoveredFile(path) => {
                trace!(path = %path.display(), "file hovered");
            }
            WindowEvent::HoveredFileCancelled => {
                trace!("file hover cancelled");
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.reset_modifier_key_state();
                    self.double_click.clear();
                }
                trace!(focused, "focus changed");
                self.dispatch_ui_event(event_loop, UiEvent::FocusChanged { focused });
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.pending_auxiliary_windows.is_empty() {
            self.sync_auxiliary_windows_or_exit(event_loop);
            if !self.config.continuous_redraw {
                event_loop.set_control_flow(if self.pending_auxiliary_windows.is_empty() {
                    ControlFlow::Wait
                } else {
                    ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(10))
                });
            }
        }
        self.request_continuous_redraw_if_due(event_loop);

        let mut pending = !self.pending_auxiliary_windows.is_empty();
        let mut errors = Vec::new();
        if let Some(renderer) = self.renderer.as_mut() {
            if let Err(error) = renderer.poll_completed() {
                errors.push(error);
            }
            pending |= renderer.frame_pending();
        }
        for state in &mut self.auxiliary_windows {
            if let Err(error) = state.renderer.poll_completed() {
                errors.push(error);
            }
            pending |= state.renderer.frame_pending();
        }
        for error in errors {
            self.handle_render_error(event_loop, error);
        }
        // Event-driven apps must also observe completion/errors without waiting
        // for another input event. No extra frames are submitted by this poll.
        if !self.config.continuous_redraw {
            event_loop.set_control_flow(if pending {
                ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(10))
            } else {
                ControlFlow::Wait
            });
        }
    }
}

fn map_pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Other(value) => PointerButton::Other(value),
        _ => PointerButton::Other(0),
    }
}

fn logical_key_string(event: &winit::event::KeyEvent) -> String {
    match &event.logical_key {
        Key::Character(value) => value.as_str().to_string(),
        other => format!("{other:?}"),
    }
}

fn ui_key_string(event: &winit::event::KeyEvent, _ctrl: bool, _shift: bool) -> String {
    let logical = logical_key_string(event);
    // Keep the physical identity present for every key event. Logical values can
    // change between press and release as Shift/IME state changes, while the
    // physical code remains stable. Consumers that only need text continue to
    // use the portion before `|`.
    ui_key_string_parts(&logical, &format!("{:?}", event.physical_key))
}

fn ui_key_string_parts(logical: &str, physical: &str) -> String {
    format!("{logical}|{physical}")
}

fn compact_key_debug_text(value: &str) -> String {
    value
        .to_ascii_uppercase()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_')
        .collect()
}

fn is_zero_shortcut_key_debug_text(value: &str) -> bool {
    let compact = compact_key_debug_text(value);

    compact.contains("DIGIT0")
        || compact.contains("KEY0")
        || compact.contains("NUMPAD0")
        || compact.contains("CODE(0)")
        || compact.contains("WINDOWS(48)")
        || compact.contains("WINDOWS(0X30)")
        || compact.contains("WINDOWS(0X0030)")
        || compact.contains("WINDOWS(11)")
        || compact.contains("WINDOWS(0XB)")
        || compact.contains("WINDOWS(0X000B)")
        || compact.contains("NATIVEKEYCODE(48)")
        || compact.contains("NATIVEKEYCODE(0X30)")
        || compact.contains("NATIVEKEYCODE(0X0030)")
        || compact.contains("NATIVEKEYCODE(11)")
        || compact.contains("NATIVEKEYCODE(0XB)")
        || compact.contains("NATIVEKEYCODE(0X000B)")
        || compact.contains("SCANCODE(11)")
        || compact.contains("SCANCODE(0XB)")
        || compact.contains("SCANCODE(0X000B)")
}

fn should_dispatch_zero_shortcut_on_release(event: &winit::event::KeyEvent, ctrl: bool) -> bool {
    if event.state != ElementState::Released || !ctrl {
        return false;
    }

    // On Windows/JIS keyboards, Ctrl+Shift+top-row-0 has been observed to skip
    // the Pressed event and only report a Released event such as
    // `physical_key=Code(Digit0), logical_key=Unidentified(Windows(0x0030))`.
    // Treat that release as a shortcut trigger so the UI can close floating
    // docks even when the OS/input stack suppresses the key-down notification.
    let debug_text = format!("{:?}|{:?}", event.logical_key, event.physical_key);
    is_zero_shortcut_key_debug_text(&debug_text)
}

fn should_dispatch_key_down(
    state: ElementState,
    repeat: bool,
    dispatch_release_shortcut: bool,
    accepts_repeat: bool,
) -> bool {
    dispatch_release_shortcut || (state == ElementState::Pressed && (!repeat || accepts_repeat))
}

fn log_keyboard_event(event: &winit::event::KeyEvent) {
    let logical_key = logical_key_string(event);

    trace!(
        state = ?event.state,
        physical_key = ?event.physical_key,
        logical_key = %logical_key,
        repeat = event.repeat,
        "keyboard input"
    );
}

fn log_ime_event(ime: &Ime) {
    match ime {
        Ime::Enabled => debug!("IME enabled"),
        Ime::Preedit(text, cursor) => debug!(text = %text, ?cursor, "IME preedit"),
        Ime::Commit(text) => debug!(text = %text, "IME commit"),
        Ime::Disabled => debug!("IME disabled"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_auxiliary_views_ignore_main_frames_but_refresh_after_invalidation() {
        let mut last_revision = None;
        let mut rebuilds = 0;
        for revision in std::iter::repeat_n(Some(7), 120).chain([Some(8), Some(8)]) {
            if auxiliary_draw_list_changed(last_revision, revision) {
                rebuilds += 1;
                last_revision = revision;
            }
        }
        assert_eq!(rebuilds, 2);
        // Native resizing invalidates the retained revision.
        last_revision = None;
        assert!(auxiliary_draw_list_changed(last_revision, Some(8)));
        // Animated views preserve the default refresh behavior.
        assert!(auxiliary_draw_list_changed(Some(8), None));
        assert!(auxiliary_draw_list_changed(None, None));
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a Windows desktop and a working WGPU adapter"]
    fn native_auxiliary_renderer_initializes_resizes_and_reopens() {
        native_auxiliary_renderer_probe(false);
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires a Windows desktop and a working WGPU adapter"]
    fn native_worker_renderer_initializes_resizes_and_reopens() {
        native_auxiliary_renderer_probe(true);
    }

    #[cfg(target_os = "windows")]
    fn native_auxiliary_renderer_probe(threaded: bool) {
        use std::{cell::Cell, rc::Rc};
        use winit::{dpi::PhysicalSize, platform::windows::EventLoopBuilderExtWindows};

        struct Probe(Rc<Cell<bool>>, bool, Rc<Cell<u64>>);
        impl PlitaProgram for Probe {
            fn render_on_worker_thread(&self) -> bool {
                self.1
            }
            fn auxiliary_draw_revision(&self, _id: u64) -> Option<u64> {
                Some(0)
            }
            fn auxiliary_window_activation_request(&self, _id: u64) -> Option<u64> {
                Some(self.2.get())
            }
            fn resize(&mut self, _size: Size) -> UiResponse {
                UiResponse::none()
            }
            fn handle_event(&mut self, _event: UiEvent) -> UiResponse {
                UiResponse::none()
            }
            fn draw_list(&self) -> DrawList {
                DrawList::new(plita_ui::Color::rgb(0.8, 0.8, 0.8))
            }
            fn auxiliary_windows(&self) -> Vec<AuxiliaryWindowDescriptor> {
                if self.0.get() {
                    vec![AuxiliaryWindowDescriptor::new(
                        1297827672,
                        "plita GX surface probe",
                        224,
                        256,
                    )]
                } else {
                    Vec::new()
                }
            }
            fn auxiliary_draw_list(&self, _id: u64) -> Option<DrawList> {
                let mut list = DrawList::new(plita_ui::Color::rgb(0.0, 0.0, 0.0));
                list.image_rgba8_shared(
                    plita_ui::Rect::new(0.0, 0.0, 160.0, 200.0),
                    2,
                    2,
                    Arc::new(std::sync::Mutex::new(vec![
                        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                    ])),
                );
                Some(list)
            }
        }

        struct Harness {
            app: Phase1WindowApp,
            requested: Rc<Cell<bool>>,
            activation: Rc<Cell<u64>>,
            deadline: Instant,
            phase: u8,
            first_window: Option<WindowId>,
            passed: bool,
        }
        impl ApplicationHandler for Harness {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                self.app.resumed(event_loop);
            }
            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                id: WindowId,
                event: WindowEvent,
            ) {
                self.app.window_event(event_loop, id, event);
            }
            fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
                if self.passed
                    || Instant::now() >= self.deadline
                    || self.app.startup_error.is_some()
                {
                    event_loop.exit();
                    return;
                }
                self.app.about_to_wait(event_loop);
                // The probe submits frames after the app's pending-work poll.
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + Duration::from_millis(10),
                ));
                if let Some(state) = self.app.auxiliary_windows.first_mut() {
                    match self.phase {
                        0 => {
                            self.first_window = Some(state.window.id());
                            let stats = state.renderer.render().expect("first auxiliary present");
                            if stats.draw_commands == 0 {
                                return;
                            }
                            state.window.set_minimized(true);
                            self.phase = 1;
                            eprintln!(
                                "native auxiliary ready: image submitted, minimize requested"
                            );
                        }
                        1 if state.window.is_minimized() == Some(true) => {
                            // A stable request must not undo the user's minimize action.
                            assert_eq!(state.activation_request, Some(self.activation.get()));
                            self.activation.set(self.activation.get() + 1);
                            self.phase = 2;
                        }
                        2 if state.window.is_minimized() == Some(false) => {
                            assert_eq!(Some(state.window.id()), self.first_window);
                            assert_eq!(state.activation_request, Some(self.activation.get()));
                            let _ = state.window.request_inner_size(PhysicalSize::new(200, 320));
                            self.phase = 3;
                            eprintln!("native auxiliary restored: same window, resize requested");
                        }
                        3 if state.renderer.size() == RenderSize::clamped(200, 320) => {
                            let stats = state.renderer.render().expect("resized auxiliary present");
                            if (stats.width, stats.height) != (200, 320) {
                                return;
                            }
                            assert_eq!((stats.width, stats.height), (200, 320));
                            self.requested.set(false);
                            self.phase = 4;
                            eprintln!("native auxiliary resized: 200x320 image submitted");
                        }
                        5 => {
                            assert_ne!(Some(state.window.id()), self.first_window);
                            let stats =
                                state.renderer.render().expect("reopened auxiliary present");
                            if stats.draw_commands == 0 {
                                return;
                            }
                            self.passed = true;
                            eprintln!(
                                "native auxiliary reopened: new window, WGPU image submitted"
                            );
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
                if self.phase == 2 {
                    self.app
                        .sync_auxiliary_windows(event_loop)
                        .expect("dispatch explicit restore request");
                }
                if self.phase == 4 {
                    self.app
                        .sync_auxiliary_windows(event_loop)
                        .expect("close auxiliary");
                    assert!(self.app.auxiliary_windows.is_empty());
                    self.requested.set(true);
                    self.app.fail_auxiliary_window(1297827672);
                    for _ in 0..32 {
                        self.app
                            .sync_auxiliary_windows(event_loop)
                            .expect("suppress failed request");
                        assert!(self.app.pending_auxiliary_windows.is_empty());
                        assert!(self.app.auxiliary_windows.is_empty());
                    }
                    self.requested.set(false);
                    self.app
                        .sync_auxiliary_windows(event_loop)
                        .expect("withdraw failed request");
                    assert!(self.app.failed_auxiliary_windows.is_empty());
                    self.requested.set(true);
                    self.app
                        .sync_auxiliary_windows(event_loop)
                        .expect("reopen auxiliary");
                    self.phase = 5;
                }
            }
        }

        let requested = Rc::new(Cell::new(true));
        let program_requested = requested.clone();
        let activation = Rc::new(Cell::new(0));
        let program_activation = activation.clone();
        let app = Phase1WindowApp::new(
            WindowConfig {
                title: "plita main surface probe".to_string(),
                width: 320,
                height: 240,
                centered: false,
                clear_color: ClearColor::rgb(0.8, 0.8, 0.8),
                continuous_redraw: false,
            },
            move |_| Probe(program_requested, threaded, program_activation),
        );
        let mut harness = Harness {
            app,
            requested,
            activation,
            deadline: Instant::now() + Duration::from_secs(60),
            phase: 0,
            first_window: None,
            passed: false,
        };
        let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
        event_loop.run_app(&mut harness).unwrap();
        assert!(
            harness.passed,
            "native auxiliary GPU initialization/present did not complete (phase {})",
            harness.phase
        );
    }

    #[derive(Clone)]
    struct DummyConfig;

    #[test]
    fn failed_auxiliary_request_notifies_once_per_id() {
        use std::{cell::Cell, rc::Rc};
        struct Probe(Rc<Cell<usize>>);
        impl PlitaProgram for Probe {
            fn resize(&mut self, _size: Size) -> UiResponse {
                UiResponse::none()
            }
            fn handle_event(&mut self, _event: UiEvent) -> UiResponse {
                UiResponse::none()
            }
            fn draw_list(&self) -> DrawList {
                DrawList::new(plita_ui::Color::rgb(0.0, 0.0, 0.0))
            }
            fn close_auxiliary_window(&mut self, _id: u64) -> UiResponse {
                self.0.set(self.0.get() + 1);
                UiResponse::redraw()
            }
        }
        let closed = Rc::new(Cell::new(0));
        let factory_closed = closed.clone();
        let mut app = Phase1WindowApp::new(WindowConfig::from(DummyConfig), move |_| {
            Probe(factory_closed)
        });
        app.ui = Some(Box::new(Probe(closed.clone())));
        app.fail_auxiliary_window(7);
        app.fail_auxiliary_window(7);
        app.fail_auxiliary_window(8);
        assert_eq!(closed.get(), 2);
        assert_eq!(app.failed_auxiliary_windows, vec![(7, None), (8, None)]);
        assert!(
            app.startup_error.is_none(),
            "auxiliary failures must not exit the main window"
        );
    }

    impl Phase0Config for DummyConfig {
        fn title(&self) -> &str {
            "Dummy"
        }

        fn width(&self) -> u32 {
            0
        }

        fn height(&self) -> u32 {
            0
        }

        fn clear_color(&self) -> ClearColor {
            ClearColor::rgb(0.0, 0.0, 0.0)
        }

        fn continuous_redraw(&self) -> bool {
            false
        }
    }

    #[test]
    fn window_config_clamps_size() {
        let config = WindowConfig::from(DummyConfig);
        assert_eq!(config.width, 1);
        assert_eq!(config.height, 1);
        assert!(!config.centered);
    }

    #[test]
    fn centered_axis_position_handles_smaller_and_oversized_windows() {
        assert_eq!(centered_axis_position(100, 1000, 600), 300);
        assert_eq!(centered_axis_position(-1920, 1920, 2560), -1920);
    }

    #[test]
    fn physical_pointer_position_converts_to_logical_ui_units() {
        assert_eq!(
            physical_position_to_ui(PhysicalPosition::new(1_560.0, 1_080.0), 1.5),
            Point::new(1_040.0, 720.0)
        );
        assert_eq!(
            physical_position_to_ui(PhysicalPosition::new(640.0, 480.0), 0.0),
            Point::new(640.0, 480.0)
        );
    }

    #[test]
    fn double_click_requires_two_nearby_completed_clicks() {
        let mut detector = DoubleClickDetector::default();
        let first_at = Instant::now();
        let first = Point::new(120.0, 80.0);
        let second = Point::new(124.0, 83.0);

        detector.press(first, PointerButton::Primary);
        assert!(!detector.release(first_at, first, PointerButton::Primary));
        detector.press(second, PointerButton::Primary);
        assert!(detector.release(
            first_at + Duration::from_millis(350),
            second,
            PointerButton::Primary
        ));
    }

    #[test]
    fn double_click_rejects_slow_distant_or_different_button_clicks() {
        let first_at = Instant::now();
        let point = Point::new(40.0, 50.0);

        let mut slow = DoubleClickDetector::default();
        slow.press(point, PointerButton::Primary);
        assert!(!slow.release(first_at, point, PointerButton::Primary));
        slow.press(point, PointerButton::Primary);
        assert!(!slow.release(
            first_at + DOUBLE_CLICK_INTERVAL + Duration::from_millis(1),
            point,
            PointerButton::Primary
        ));

        let mut distant = DoubleClickDetector::default();
        distant.press(point, PointerButton::Primary);
        assert!(!distant.release(first_at, point, PointerButton::Primary));
        let far = Point::new(point.x + DOUBLE_CLICK_DISTANCE + 1.0, point.y);
        distant.press(far, PointerButton::Primary);
        assert!(!distant.release(
            first_at + Duration::from_millis(100),
            far,
            PointerButton::Primary
        ));

        let mut different_button = DoubleClickDetector::default();
        different_button.press(point, PointerButton::Primary);
        assert!(!different_button.release(first_at, point, PointerButton::Primary));
        different_button.press(point, PointerButton::Secondary);
        assert!(!different_button.release(
            first_at + Duration::from_millis(100),
            point,
            PointerButton::Secondary
        ));
    }

    #[test]
    fn drag_release_does_not_count_as_a_click() {
        let mut detector = DoubleClickDetector::default();
        let at = Instant::now();
        let start = Point::new(10.0, 10.0);
        let end = Point::new(30.0, 10.0);

        detector.press(start, PointerButton::Primary);
        assert!(!detector.release(at, end, PointerButton::Primary));
        detector.press(end, PointerButton::Primary);
        assert!(!detector.release(at + Duration::from_millis(100), end, PointerButton::Primary));
    }

    #[test]
    fn zero_shortcut_debug_text_accepts_observed_windows_jis_release() {
        assert!(is_zero_shortcut_key_debug_text(
            "Unidentified(Windows(0x0030))|Code(Digit0)"
        ));
        assert!(is_zero_shortcut_key_debug_text(
            "Unidentified(NativeKeyCode(11))|PhysicalKey::Code(Key0)"
        ));
        assert!(is_zero_shortcut_key_debug_text("Code(Numpad0)"));
        assert!(!is_zero_shortcut_key_debug_text("Code(Digit6)"));
    }

    #[test]
    fn key_down_dispatch_honors_the_program_repeat_policy() {
        assert!(should_dispatch_key_down(
            ElementState::Pressed,
            false,
            false,
            false
        ));
        assert!(!should_dispatch_key_down(
            ElementState::Pressed,
            true,
            false,
            false
        ));
        assert!(should_dispatch_key_down(
            ElementState::Pressed,
            true,
            false,
            true
        ));
        assert!(should_dispatch_key_down(
            ElementState::Released,
            false,
            true,
            false
        ));
    }

    #[test]
    fn ui_key_strings_always_keep_stable_physical_identity() {
        assert_eq!(ui_key_string_parts("8", "Code(Digit8)"), "8|Code(Digit8)");
        assert_eq!(ui_key_string_parts("@", "Code(Digit2)"), "@|Code(Digit2)");
        assert_eq!(
            ui_key_string_parts("Named(Shift)", "Code(ShiftLeft)"),
            "Named(Shift)|Code(ShiftLeft)"
        );
    }

    #[test]
    fn continuous_redraw_deadline_does_not_wait_after_render_cost() {
        let base = Instant::now();
        let first_deadline = base + CONTINUOUS_REDRAW_INTERVAL;
        let after_render = first_deadline + Duration::from_millis(6);
        let next = next_continuous_redraw_deadline(first_deadline, after_render);

        assert_eq!(next, base + CONTINUOUS_REDRAW_INTERVAL * 2);
    }

    #[test]
    fn continuous_redraw_deadline_skips_missed_intervals() {
        let base = Instant::now();
        let first_deadline = base + CONTINUOUS_REDRAW_INTERVAL;
        let late = first_deadline + CONTINUOUS_REDRAW_INTERVAL * 3 + Duration::from_millis(1);
        let next = next_continuous_redraw_deadline(first_deadline, late);

        assert!(next > late);
        assert_eq!(next, base + CONTINUOUS_REDRAW_INTERVAL * 5);
    }

    #[test]
    fn periodic_deadline_supports_faster_program_ticks_than_redraws() {
        let base = Instant::now();
        let interval = Duration::from_nanos(4_166_667);
        let first_deadline = base + interval;
        let late = first_deadline + interval * 2 + Duration::from_micros(1);
        let next = next_periodic_deadline(first_deadline, late, interval);

        assert!(next > late);
        assert_eq!(next, base + interval * 4);
        assert!(next < base + CONTINUOUS_REDRAW_INTERVAL * 2);
    }
}

#[cfg(test)]
mod auxiliary_failure_dispatch_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn activation_is_only_consumed_when_the_request_changes() {
        let mut last = None;
        assert!(!take_auxiliary_activation(&mut last, None));
        assert!(take_auxiliary_activation(&mut last, Some(1)));
        assert!(!take_auxiliary_activation(&mut last, Some(1)));
        assert!(take_auxiliary_activation(&mut last, Some(2)));
        assert!(!take_auxiliary_activation(&mut last, Some(2)));
        assert!(!take_auxiliary_activation(&mut last, None));
        assert!(take_auxiliary_activation(&mut last, Some(0)));
    }

    #[test]
    fn explicit_retry_clears_failure_without_recreating_the_main_window() {
        struct Probe(Rc<Cell<u64>>);
        impl PlitaProgram for Probe {
            fn resize(&mut self, _: Size) -> UiResponse {
                UiResponse::none()
            }
            fn handle_event(&mut self, _: UiEvent) -> UiResponse {
                UiResponse::none()
            }
            fn draw_list(&self) -> DrawList {
                DrawList::new(plita_ui::Color::rgb(0., 0., 0.))
            }
            fn auxiliary_window_activation_request(&self, _: u64) -> Option<u64> {
                Some(self.0.get())
            }
        }
        let token = Rc::new(Cell::new(1));
        let factory_token = token.clone();
        let config = WindowConfig {
            title: "retry".into(),
            width: 320,
            height: 240,
            centered: false,
            clear_color: ClearColor::rgb(0., 0., 0.),
            continuous_redraw: false,
        };
        let mut app = Phase1WindowApp::new(config, move |_| Probe(factory_token));
        app.ui = Some(Box::new(Probe(token.clone())));
        let descriptors = [AuxiliaryWindowDescriptor::new(7, "test", 320, 240)];
        app.fail_auxiliary_window(7);
        app.discard_obsolete_auxiliary_failures(&descriptors);
        assert_eq!(app.failed_auxiliary_windows, vec![(7, Some(1))]);
        token.set(2);
        app.discard_obsolete_auxiliary_failures(&descriptors);
        assert!(app.failed_auxiliary_windows.is_empty());
        app.fail_auxiliary_window(7);
        app.discard_obsolete_auxiliary_failures(&[]);
        assert!(app.failed_auxiliary_windows.is_empty());
        assert!(app.startup_error.is_none());
    }

    #[test]
    fn renderer_failure_is_not_dispatched_as_a_user_close() {
        struct Probe(Rc<Cell<usize>>, Rc<Cell<usize>>);
        impl PlitaProgram for Probe {
            fn resize(&mut self, _: Size) -> UiResponse {
                UiResponse::none()
            }
            fn handle_event(&mut self, _: UiEvent) -> UiResponse {
                UiResponse::none()
            }
            fn draw_list(&self) -> DrawList {
                DrawList::new(plita_ui::Color::rgb(0.0, 0.0, 0.0))
            }
            fn close_auxiliary_window(&mut self, _: u64) -> UiResponse {
                self.0.set(self.0.get() + 1);
                UiResponse::redraw()
            }
            fn auxiliary_window_failed(&mut self, _: u64) -> UiResponse {
                self.1.set(self.1.get() + 1);
                UiResponse::redraw()
            }
        }
        let closes = Rc::new(Cell::new(0));
        let failures = Rc::new(Cell::new(0));
        let factory_closes = closes.clone();
        let factory_failures = failures.clone();
        let config = WindowConfig {
            title: "failure dispatch".into(),
            width: 320,
            height: 240,
            centered: false,
            clear_color: ClearColor::rgb(0.0, 0.0, 0.0),
            continuous_redraw: false,
        };
        let mut app =
            Phase1WindowApp::new(config, move |_| Probe(factory_closes, factory_failures));
        app.ui = Some(Box::new(Probe(closes.clone(), failures.clone())));
        app.fail_auxiliary_window(7);
        app.fail_auxiliary_window(7);
        app.fail_auxiliary_window(8);
        assert_eq!(failures.get(), 2);
        assert_eq!(closes.get(), 0);
        assert_eq!(app.failed_auxiliary_windows, vec![(7, None), (8, None)]);
        assert!(app.startup_error.is_none());
    }
}
