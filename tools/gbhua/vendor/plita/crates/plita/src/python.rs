//! Optional PyO3 helpers for PLITA applications.
//!
//! Enable the `python` feature on `plita` to reuse these bindings from an
//! application crate. Application-specific Python modules can import
//! `plita::python::prelude::*` instead of depending on PyO3 directly.

use crate::{AppBuilder, ButtonInteractionState, ClearColor, Color, Point, Rect, Size};

pub use pyo3;
use pyo3::{exceptions::PyRuntimeError, prelude::*, types::PyBytes};

/// Convenient imports for application-level Python modules.
pub mod prelude {
    pub use super::pyo3;
    pub use super::{
        add_plita_classes, app_error_to_py_err, py_bytes, run_app_builder, PyAppConfig,
        PyButtonInteraction, PyColor, PyPoint, PyRect, PySize,
    };
    pub use pyo3::prelude::*;
    pub use pyo3::types::PyBytes;
    pub use pyo3::{pyclass, pyfunction, pymethods, pymodule, wrap_pyfunction};
}

/// Convert a PLITA application error into a Python `RuntimeError`.
pub fn app_error_to_py_err(error: crate::AppError) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

/// Create a Python `bytes` object from a Rust byte slice.
pub fn py_bytes<'py>(py: Python<'py>, bytes: &[u8]) -> Bound<'py, PyBytes> {
    PyBytes::new_bound(py, bytes)
}

/// Run a PLITA app builder and convert errors into Python exceptions.
pub fn run_app_builder<F, P>(builder: AppBuilder, factory: F) -> PyResult<()>
where
    F: FnOnce(Size) -> P + 'static,
    P: crate::PlitaProgram + 'static,
{
    builder.run_program(factory).map_err(app_error_to_py_err)
}

/// Register PLITA's common Python helper classes on a module.
pub fn add_plita_classes(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyPoint>()?;
    module.add_class::<PySize>()?;
    module.add_class::<PyRect>()?;
    module.add_class::<PyColor>()?;
    module.add_class::<PyButtonInteraction>()?;
    module.add_class::<PyAppConfig>()?;
    module.add("__plita_version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

/// Python-facing two-dimensional point.
#[pyclass(name = "Point")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PyPoint {
    #[pyo3(get, set)]
    pub x: f32,
    #[pyo3(get, set)]
    pub y: f32,
}

#[pymethods]
impl PyPoint {
    #[new]
    #[pyo3(signature = (x = 0.0, y = 0.0))]
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn __repr__(&self) -> String {
        format!("Point(x={}, y={})", self.x, self.y)
    }
}

impl From<Point> for PyPoint {
    fn from(value: Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<PyPoint> for Point {
    fn from(value: PyPoint) -> Self {
        Point::new(value.x, value.y)
    }
}

/// Python-facing two-dimensional size.
#[pyclass(name = "Size")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PySize {
    #[pyo3(get, set)]
    pub width: f32,
    #[pyo3(get, set)]
    pub height: f32,
}

#[pymethods]
impl PySize {
    #[new]
    #[pyo3(signature = (width = 0.0, height = 0.0))]
    fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    fn clamped(&self) -> Self {
        Size::from(*self).clamped().into()
    }

    fn __repr__(&self) -> String {
        format!("Size(width={}, height={})", self.width, self.height)
    }
}

impl From<Size> for PySize {
    fn from(value: Size) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<PySize> for Size {
    fn from(value: PySize) -> Self {
        Size::new(value.width, value.height)
    }
}

/// Python-facing rectangle.
#[pyclass(name = "Rect")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PyRect {
    #[pyo3(get, set)]
    pub x: f32,
    #[pyo3(get, set)]
    pub y: f32,
    #[pyo3(get, set)]
    pub width: f32,
    #[pyo3(get, set)]
    pub height: f32,
}

#[pymethods]
impl PyRect {
    #[new]
    #[pyo3(signature = (x = 0.0, y = 0.0, width = 0.0, height = 0.0))]
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn right(&self) -> f32 {
        Rect::from(*self).right()
    }

    fn bottom(&self) -> f32 {
        Rect::from(*self).bottom()
    }

    fn contains(&self, point: &PyPoint) -> bool {
        Rect::from(*self).contains(Point::from(*point))
    }

    #[pyo3(signature = (amount))]
    fn inset(&self, amount: f32) -> Self {
        Rect::from(*self).inset(amount).into()
    }

    fn __repr__(&self) -> String {
        format!(
            "Rect(x={}, y={}, width={}, height={})",
            self.x, self.y, self.width, self.height
        )
    }
}

impl From<Rect> for PyRect {
    fn from(value: Rect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

impl From<PyRect> for Rect {
    fn from(value: PyRect) -> Self {
        Rect::new(value.x, value.y, value.width, value.height)
    }
}

/// Python-facing RGBA color.
#[pyclass(name = "Color")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PyColor {
    #[pyo3(get, set)]
    pub r: f32,
    #[pyo3(get, set)]
    pub g: f32,
    #[pyo3(get, set)]
    pub b: f32,
    #[pyo3(get, set)]
    pub a: f32,
}

#[pymethods]
impl PyColor {
    #[new]
    #[pyo3(signature = (r = 0.0, g = 0.0, b = 0.0, a = 1.0))]
    fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[staticmethod]
    fn rgb(r: f32, g: f32, b: f32) -> Self {
        Color::rgb(r, g, b).into()
    }

    #[staticmethod]
    fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Color::rgb8(r, g, b).into()
    }

    #[staticmethod]
    fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::rgba8(r, g, b, a).into()
    }

    fn __repr__(&self) -> String {
        format!(
            "Color(r={}, g={}, b={}, a={})",
            self.r, self.g, self.b, self.a
        )
    }
}

impl From<Color> for PyColor {
    fn from(value: Color) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

impl From<PyColor> for Color {
    fn from(value: PyColor) -> Self {
        Color {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

impl From<PyColor> for ClearColor {
    fn from(value: PyColor) -> Self {
        ClearColor {
            r: f64::from(value.r),
            g: f64::from(value.g),
            b: f64::from(value.b),
            a: f64::from(value.a),
        }
    }
}

/// Python-facing button hover/press interaction state.
#[pyclass(name = "ButtonInteraction")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PyButtonInteraction {
    state: ButtonInteractionState<String>,
}

#[pymethods]
impl PyButtonInteraction {
    #[new]
    fn new() -> Self {
        Self {
            state: ButtonInteractionState::new(),
        }
    }

    fn hovered(&self) -> Option<String> {
        self.state.hovered().cloned()
    }

    fn pressed(&self) -> Option<String> {
        self.state.pressed().cloned()
    }

    fn clear(&mut self) -> bool {
        self.state.clear()
    }

    fn hover(&mut self, name: &str) -> String {
        let id = name.to_string();
        self.state.hover(id.clone());
        self.state.visual_state(&id, false).label().to_string()
    }

    fn press(&mut self, name: &str) -> String {
        let id = name.to_string();
        self.state.press(id.clone());
        self.state.visual_state(&id, false).label().to_string()
    }

    #[pyo3(signature = (name = None, activate = true))]
    fn release(&mut self, name: Option<String>, activate: bool) -> Option<String> {
        let release = self.state.release_over(name);
        if activate {
            release.activated
        } else {
            None
        }
    }

    fn click(&mut self, name: &str) -> Option<String> {
        self.state.click(name.to_string()).activated
    }

    #[pyo3(signature = (name, active = false))]
    fn visual_state(&self, name: &str, active: bool) -> String {
        self.state
            .visual_state(&name.to_string(), active)
            .label()
            .to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "ButtonInteraction(hovered={:?}, pressed={:?})",
            self.hovered(),
            self.pressed()
        )
    }
}

/// Python-facing app configuration used by binding crates.
#[pyclass(name = "AppConfig")]
#[derive(Clone, Debug, PartialEq)]
pub struct PyAppConfig {
    #[pyo3(get, set)]
    pub title: String,
    #[pyo3(get, set)]
    pub width: u32,
    #[pyo3(get, set)]
    pub height: u32,
    #[pyo3(get, set)]
    pub clear_color: PyColor,
    #[pyo3(get, set)]
    pub continuous_redraw: bool,
    #[pyo3(get, set)]
    pub install_tracing: bool,
}

impl Default for PyAppConfig {
    fn default() -> Self {
        let config = crate::AppConfig::default();
        Self {
            title: config.title,
            width: config.width,
            height: config.height,
            clear_color: PyColor {
                r: config.clear_color.r as f32,
                g: config.clear_color.g as f32,
                b: config.clear_color.b as f32,
                a: config.clear_color.a as f32,
            },
            continuous_redraw: config.continuous_redraw,
            install_tracing: config.install_tracing,
        }
    }
}

#[pymethods]
impl PyAppConfig {
    #[new]
    #[pyo3(signature = (
        title = None,
        width = None,
        height = None,
        clear_color = None,
        continuous_redraw = None,
        install_tracing = None
    ))]
    fn new(
        title: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        clear_color: Option<PyColor>,
        continuous_redraw: Option<bool>,
        install_tracing: Option<bool>,
    ) -> Self {
        let mut config = Self::default();
        if let Some(title) = title {
            config.title = title;
        }
        if let Some(width) = width {
            config.width = width.max(1);
        }
        if let Some(height) = height {
            config.height = height.max(1);
        }
        if let Some(clear_color) = clear_color {
            config.clear_color = clear_color;
        }
        if let Some(continuous_redraw) = continuous_redraw {
            config.continuous_redraw = continuous_redraw;
        }
        if let Some(install_tracing) = install_tracing {
            config.install_tracing = install_tracing;
        }
        config
    }

    fn to_builder_summary(&self) -> String {
        format!(
            "{} {}x{} continuous_redraw={} install_tracing={}",
            self.title, self.width, self.height, self.continuous_redraw, self.install_tracing
        )
    }
}

impl PyAppConfig {
    /// Convert into a PLITA app builder.
    pub fn to_builder(&self) -> AppBuilder {
        AppBuilder::default()
            .title(self.title.clone())
            .window_size(self.width, self.height)
            .clear_color(ClearColor::from(self.clear_color))
            .continuous_redraw(self.continuous_redraw)
            .install_tracing(self.install_tracing)
    }
}

#[pymodule]
fn plita(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    add_plita_classes(module)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_round_trips_through_python_wrapper() {
        let size = Size::new(320.0, 240.0);
        let py_size = PySize::from(size);

        assert_eq!(Size::from(py_size), size);
    }

    #[test]
    fn app_config_builds_plita_builder() {
        let config = PyAppConfig::new(
            Some("Demo".to_string()),
            Some(0),
            Some(240),
            Some(PyColor::rgb8(1, 2, 3)),
            Some(true),
            Some(false),
        );
        let builder = config.to_builder();

        assert_eq!(builder.config().title, "Demo");
        assert_eq!(builder.config().width, 1);
        assert_eq!(builder.config().height, 240);
        assert!(builder.config().continuous_redraw);
        assert!(!builder.config().install_tracing);
    }

    #[test]
    fn button_interaction_wrapper_reports_hover_press_and_click() {
        let mut buttons = PyButtonInteraction::new();

        assert_eq!(buttons.visual_state("run", false), "normal");
        assert_eq!(buttons.hover("run"), "hovered");
        assert_eq!(buttons.hovered(), Some("run".to_string()));
        assert_eq!(buttons.press("run"), "pressed");
        assert_eq!(buttons.pressed(), Some("run".to_string()));
        assert_eq!(
            buttons.release(Some("run".to_string()), true),
            Some("run".to_string())
        );
        assert_eq!(buttons.pressed(), None);
        assert_eq!(buttons.click("step"), Some("step".to_string()));
    }
}
