//! Application configuration and startup builder for PLITA UI.

use plita_render::ClearColor;
use thiserror::Error;
use tracing_subscriber::{fmt, EnvFilter};

/// Result type used by the public PLITA application API.
pub type Result<T> = std::result::Result<T, AppError>;

/// Top-level application error.
#[derive(Debug, Error)]
pub enum AppError {
    /// Window/event-loop layer error.
    #[error(transparent)]
    Window(#[from] plita_window::WindowError),
}

/// Phase 1 application configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    /// Initial window title.
    pub title: String,
    /// Initial logical width.
    pub width: u32,
    /// Initial logical height.
    pub height: u32,
    /// Center the initial main window on its monitor.
    pub centered: bool,
    /// GPU clear color used by the Phase 1 renderer.
    pub clear_color: ClearColor,
    /// When true, continuously requests redraws. When false, redraws only on demand.
    pub continuous_redraw: bool,
    /// Enable default tracing subscriber at application startup.
    pub install_tracing: bool,
    /// Optional versioned dock layout profile restored when the default shell starts.
    pub startup_layout_profile: Option<String>,
}

impl plita_window::Phase0Config for AppConfig {
    fn title(&self) -> &str {
        &self.title
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn centered(&self) -> bool {
        self.centered
    }

    fn clear_color(&self) -> ClearColor {
        self.clear_color
    }

    fn continuous_redraw(&self) -> bool {
        self.continuous_redraw
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "PLITA UI v1.0".to_string(),
            width: 1280,
            height: 800,
            centered: false,
            clear_color: ClearColor::rgb(0.105, 0.111, 0.125),
            continuous_redraw: false,
            install_tracing: true,
            startup_layout_profile: None,
        }
    }
}

/// Entry point type for PLITA applications.
pub struct App;

impl App {
    /// Create a new application builder.
    pub fn new() -> AppBuilder {
        AppBuilder::default()
    }
}

/// Builder for launching a PLITA application.
#[derive(Clone, Debug)]
pub struct AppBuilder {
    config: AppConfig,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
        }
    }
}

impl AppBuilder {
    /// Set the initial title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Set the initial logical window size.
    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self
    }

    /// Center the initial main window on its monitor.
    pub fn centered(mut self, centered: bool) -> Self {
        self.config.centered = centered;
        self
    }

    /// Set the GPU clear color.
    pub fn clear_color(mut self, color: ClearColor) -> Self {
        self.config.clear_color = color;
        self
    }

    /// Enable or disable continuous redraw.
    ///
    /// GUI applications should generally leave this disabled until animations or playback are added.
    pub fn continuous_redraw(mut self, enabled: bool) -> Self {
        self.config.continuous_redraw = enabled;
        self
    }

    /// Enable or disable automatic tracing subscriber installation.
    pub fn install_tracing(mut self, enabled: bool) -> Self {
        self.config.install_tracing = enabled;
        self
    }

    /// Restore a versioned dock layout profile when launching the default shell.
    pub fn startup_layout_profile(mut self, profile_text: impl Into<String>) -> Self {
        self.config.startup_layout_profile = Some(profile_text.into());
        self
    }

    /// Inspect the resolved configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Launch the application.
    pub fn run(self) -> Result<()> {
        if self.config.install_tracing {
            install_default_tracing();
        }

        let profile = self.config.startup_layout_profile.clone();
        plita_window::run_program(self.config, move |size| {
            let mut shell = plita_ui::CreativeShell::new(size);
            if let Some(profile) = profile.as_deref() {
                let _ = shell.load_dock_layout_profile_text(profile);
            }
            shell
        })
        .map_err(AppError::from)
    }

    /// Launch a custom PLITA program with the same native window/render loop.
    pub fn run_program<F, P>(self, factory: F) -> Result<()>
    where
        F: FnOnce(plita_ui::Size) -> P + 'static,
        P: plita_window::PlitaProgram + 'static,
    {
        if self.config.install_tracing {
            install_default_tracing();
        }

        plita_window::run_program(self.config, factory).map_err(AppError::from)
    }
}

fn install_default_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,plita_window=debug,plita_render=debug"));

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_desktop_sized() {
        let config = AppConfig::default();
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 800);
        assert!(!config.centered);
        assert!(!config.title.is_empty());
    }

    #[test]
    fn builder_clamps_zero_window_size() {
        let builder = App::new().window_size(0, 0);
        assert_eq!(builder.config().width, 1);
        assert_eq!(builder.config().height, 1);
    }

    #[test]
    fn builder_updates_title_and_redraw_mode() {
        let builder = App::new()
            .title("Demo")
            .centered(true)
            .continuous_redraw(true)
            .install_tracing(false);

        assert_eq!(builder.config().title, "Demo");
        assert!(builder.config().centered);
        assert!(builder.config().continuous_redraw);
        assert!(!builder.config().install_tracing);
    }

    #[test]
    fn builder_accepts_startup_layout_profile_text() {
        let builder = App::new().startup_layout_profile("PLITA_DOCK_LAYOUT_PROFILE\nversion=1\n{}");
        assert!(builder.config().startup_layout_profile.is_some());
    }
}
