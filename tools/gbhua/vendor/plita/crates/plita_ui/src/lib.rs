//! Basic UI model for plita UI Phase 2.
//!
//! Phase 2 v3 begins the retained Dock Layout State foundation. It keeps the
//! lightweight rectangle-rendered shell and v12 floating dock behavior stable,
//! then adds Dock Tree, tab-group, floating-panel, and JSON-seed state models
//! that can grow into full layout persistence and attach/detach docking.
//!
//! Phase 2 v4.0 starts Dock Tab Group hardening. It keeps the v25 live split
//! ratio behavior stable, then adds retained tab visibility, tab persistence
//! seed data, restore-all-tab operations, and layout-memory slots that prepare
//! plita for true attach/detach docking.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// Two-dimensional point in logical or physical UI pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

impl Point {
    /// Create a point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Two-dimensional size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Size {
    /// Create a size.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Return a size with both dimensions clamped to at least one pixel.
    pub fn clamped(self) -> Self {
        Self {
            width: finite_at_least(self.width, 1.0),
            height: finite_at_least(self.height, 1.0),
        }
    }
}

/// Return a finite value that is at least `min`.
fn finite_at_least(value: f32, min: f32) -> f32 {
    if value.is_finite() {
        value.max(min)
    } else {
        min
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Clamp a panel dimension without panicking when the window becomes tiny.
///
/// `f32::clamp` panics if `min > max`. During a Windows resize or minimize
/// operation, the reported drawable size can briefly become very small. In that
/// case the available maximum side-panel width can be smaller than the normal
/// minimum side-panel width. The bootstrap layout should collapse the panel to
/// the available maximum instead of crashing the application.
fn clamp_panel_extent(value: f32, min: f32, max: f32) -> f32 {
    let min = if min.is_finite() { min.max(0.0) } else { 0.0 };
    let max = if max.is_finite() { max.max(0.0) } else { min };
    let value = if value.is_finite() { value } else { min };

    if max <= min {
        max
    } else {
        value.clamp(min, max)
    }
}

fn clamp_dock_split_ratio(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.05, 0.95)
    } else {
        0.78
    }
}

fn scroll_point(current: Point, delta: Point, max_x: f32, max_y: f32) -> Point {
    Point::new(
        (current.x + delta.x).clamp(0.0, max_x.max(0.0)),
        (current.y + delta.y).clamp(0.0, max_y.max(0.0)),
    )
}

/// Rectangle in UI coordinate space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left coordinate.
    pub x: f32,
    /// Top coordinate.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    /// Create a rectangle.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge coordinate.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge coordinate.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// Whether the rectangle has positive visible area.
    pub fn is_visible(self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }

    /// Whether the rectangle contains a point.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }

    /// Whether two rectangles overlap.
    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Inset the rectangle by the given amount on all sides.
    pub fn inset(self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: (self.width - amount * 2.0).max(0.0),
            height: (self.height - amount * 2.0).max(0.0),
        }
    }
}

/// RGBA color in linear floating point range.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

impl Color {
    /// Create an opaque RGB color.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create an RGBA color.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create an RGB color from 8-bit channel values.
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Create an RGBA color from 8-bit channel values.
    pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Return this color with a replaced alpha channel.
    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Return this color with its alpha multiplied by `factor`.
    pub fn scale_alpha(self, factor: f32) -> Self {
        Self {
            a: (self.a * factor).clamp(0.0, 1.0),
            ..self
        }
    }

    /// Linearly interpolate between two colors.
    pub fn lerp(self, other: Self, amount: f32) -> Self {
        let t = amount.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

pub mod icons;
pub use icons::{draw_lucide_icon, lucide_icon_names};

/// Colored vertex used by mesh-backed plita UI drawing commands.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColoredVertex {
    /// Vertex position in UI coordinates.
    pub position: Point,
    /// Vertex color.
    pub color: Color,
}

impl ColoredVertex {
    /// Create a colored UI vertex.
    pub const fn new(position: Point, color: Color) -> Self {
        Self { position, color }
    }
}

/// Draw command emitted by Phase 1 UI layout.
#[derive(Clone, Debug)]
pub enum DrawCommand {
    /// Fill a rectangle with a solid color.
    FillRect { rect: Rect, color: Color },
    /// Fill a rectangle with colors assigned to each corner.
    FillRectCorners {
        /// Destination rectangle.
        rect: Rect,
        /// Color at the top-left corner.
        top_left: Color,
        /// Color at the top-right corner.
        top_right: Color,
        /// Color at the bottom-right corner.
        bottom_right: Color,
        /// Color at the bottom-left corner.
        bottom_left: Color,
    },
    /// Fill arbitrary triangles with per-vertex colors.
    FillTriangles {
        /// Vertices interpreted as a triangle list.
        vertices: Arc<[ColoredVertex]>,
    },
    /// Draw an RGBA8 image stretched into a rectangle.
    ImageRgba {
        /// Destination rectangle.
        rect: Rect,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Pixel data in tightly packed RGBA8 row-major order.
        pixels: Arc<[u8]>,
    },
    /// Draw an RGBA8 image backed by a shared mutable buffer.
    ///
    /// This is intended for high-frequency video surfaces: the draw list can
    /// stay stable while the producer updates the shared pixels between frames.
    ImageRgbaShared {
        /// Destination rectangle.
        rect: Rect,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Pixel data in tightly packed RGBA8 row-major order.
        pixels: Arc<Mutex<Vec<u8>>>,
    },
    /// Restrict following draw commands to this rectangle until `PopClip`.
    PushClip { rect: Rect },
    /// Restore the previous clip rectangle.
    PopClip,
}

impl PartialEq for DrawCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::FillRect {
                    rect: left_rect,
                    color: left_color,
                },
                Self::FillRect {
                    rect: right_rect,
                    color: right_color,
                },
            ) => left_rect == right_rect && left_color == right_color,
            (
                Self::FillRectCorners {
                    rect: left_rect,
                    top_left: left_top_left,
                    top_right: left_top_right,
                    bottom_right: left_bottom_right,
                    bottom_left: left_bottom_left,
                },
                Self::FillRectCorners {
                    rect: right_rect,
                    top_left: right_top_left,
                    top_right: right_top_right,
                    bottom_right: right_bottom_right,
                    bottom_left: right_bottom_left,
                },
            ) => {
                left_rect == right_rect
                    && left_top_left == right_top_left
                    && left_top_right == right_top_right
                    && left_bottom_right == right_bottom_right
                    && left_bottom_left == right_bottom_left
            }
            (Self::FillTriangles { vertices: left }, Self::FillTriangles { vertices: right }) => {
                left == right
            }
            (
                Self::ImageRgba {
                    rect: left_rect,
                    width: left_width,
                    height: left_height,
                    pixels: left_pixels,
                },
                Self::ImageRgba {
                    rect: right_rect,
                    width: right_width,
                    height: right_height,
                    pixels: right_pixels,
                },
            ) => {
                left_rect == right_rect
                    && left_width == right_width
                    && left_height == right_height
                    && left_pixels == right_pixels
            }
            (
                Self::ImageRgbaShared {
                    rect: left_rect,
                    width: left_width,
                    height: left_height,
                    pixels: left_pixels,
                },
                Self::ImageRgbaShared {
                    rect: right_rect,
                    width: right_width,
                    height: right_height,
                    pixels: right_pixels,
                },
            ) => {
                left_rect == right_rect
                    && left_width == right_width
                    && left_height == right_height
                    && Arc::ptr_eq(left_pixels, right_pixels)
            }
            (Self::PushClip { rect: left }, Self::PushClip { rect: right }) => left == right,
            (Self::PopClip, Self::PopClip) => true,
            _ => false,
        }
    }
}

/// Flat list of UI draw commands.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawList {
    /// Clear/background color for the frame.
    pub clear_color: Color,
    commands: Vec<DrawCommand>,
}

impl DrawList {
    /// Create an empty draw list.
    pub fn new(clear_color: Color) -> Self {
        Self {
            clear_color,
            commands: Vec::new(),
        }
    }

    /// Add a filled rectangle if it has visible area.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.is_visible() {
            self.commands.push(DrawCommand::FillRect { rect, color });
        }
    }

    /// Add a filled rectangle whose four corners can carry distinct colors.
    pub fn fill_rect_corners(
        &mut self,
        rect: Rect,
        top_left: Color,
        top_right: Color,
        bottom_right: Color,
        bottom_left: Color,
    ) {
        if rect.is_visible() {
            self.commands.push(DrawCommand::FillRectCorners {
                rect,
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            });
        }
    }

    /// Add a vertical two-color gradient rectangle.
    pub fn fill_vertical_gradient_rect(&mut self, rect: Rect, top: Color, bottom: Color) {
        self.fill_rect_corners(rect, top, top, bottom, bottom);
    }

    /// Add a triangle list with per-vertex colors.
    pub fn fill_triangle_list<V>(&mut self, vertices: V)
    where
        V: Into<Arc<[ColoredVertex]>>,
    {
        let vertices = vertices.into();
        if vertices.len() >= 3 && vertices.len() % 3 == 0 {
            self.commands.push(DrawCommand::FillTriangles { vertices });
        }
    }

    /// Add a filled rounded rectangle.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let radius = rounded_rect_radius(rect, radius);
        if radius <= 0.0 {
            self.fill_rect(rect, color);
            return;
        }

        let vertices = rounded_rect_triangle_vertices(rect, radius, |_| color);
        self.fill_triangle_list(vertices);
    }

    /// Add a rounded rectangle with a vertical two-color gradient.
    pub fn fill_rounded_vertical_gradient_rect(
        &mut self,
        rect: Rect,
        radius: f32,
        top: Color,
        bottom: Color,
    ) {
        let radius = rounded_rect_radius(rect, radius);
        if radius <= 0.0 {
            self.fill_vertical_gradient_rect(rect, top, bottom);
            return;
        }

        let vertices = rounded_rect_triangle_vertices(rect, radius, |point| {
            vertical_gradient_color(rect, point.y, top, bottom)
        });
        self.fill_triangle_list(vertices);
    }

    /// Add a rectangle filled with a radial gradient.
    pub fn fill_radial_gradient_rect(
        &mut self,
        rect: Rect,
        center: Point,
        radius: f32,
        inner: Color,
        outer: Color,
    ) {
        self.fill_radial_gradient_rect_steps(
            rect,
            center,
            radius,
            inner,
            outer,
            DEFAULT_RADIAL_GRADIENT_STEPS,
        );
    }

    /// Add a rectangle filled with a radial gradient using an explicit grid density.
    pub fn fill_radial_gradient_rect_steps(
        &mut self,
        rect: Rect,
        center: Point,
        radius: f32,
        inner: Color,
        outer: Color,
        steps: u32,
    ) {
        let vertices = radial_gradient_rect_vertices(rect, center, radius, inner, outer, steps);
        self.fill_triangle_list(vertices);
    }

    /// Add a rounded rectangle filled with a radial gradient.
    pub fn fill_rounded_radial_gradient_rect(
        &mut self,
        rect: Rect,
        corner_radius: f32,
        center: Point,
        gradient_radius: f32,
        inner: Color,
        outer: Color,
    ) {
        let corner_radius = rounded_rect_radius(rect, corner_radius);
        if corner_radius <= 0.0 {
            self.fill_radial_gradient_rect(rect, center, gradient_radius, inner, outer);
            return;
        }

        let vertices = rounded_rect_triangle_vertices(rect, corner_radius, |point| {
            radial_gradient_color(point, center, gradient_radius, inner, outer)
        });
        self.fill_triangle_list(vertices);
    }

    /// Add a tightly packed RGBA8 image if its destination and payload are valid.
    pub fn image_rgba8<P>(&mut self, rect: Rect, width: u32, height: u32, pixels: P)
    where
        P: Into<Arc<[u8]>>,
    {
        if !rect.is_visible() || width == 0 || height == 0 {
            return;
        }
        let pixels = pixels.into();
        let expected_len = width as usize * height as usize * 4;
        if pixels.len() == expected_len {
            self.commands.push(DrawCommand::ImageRgba {
                rect,
                width,
                height,
                pixels,
            });
        }
    }

    /// Add a shared RGBA8 image if its destination is visible.
    pub fn image_rgba8_shared(
        &mut self,
        rect: Rect,
        width: u32,
        height: u32,
        pixels: Arc<Mutex<Vec<u8>>>,
    ) {
        if !rect.is_visible() || width == 0 || height == 0 {
            return;
        }
        self.commands.push(DrawCommand::ImageRgbaShared {
            rect,
            width,
            height,
            pixels,
        });
    }

    /// Push a clipping rectangle for subsequent commands.
    pub fn push_clip(&mut self, rect: Rect) {
        if rect.is_visible() {
            self.commands.push(DrawCommand::PushClip { rect });
        }
    }

    /// Pop the most recent clipping rectangle.
    pub fn pop_clip(&mut self) {
        self.commands.push(DrawCommand::PopClip);
    }

    /// Add a one-pixel-ish border around a rectangle.
    pub fn stroke_rect(&mut self, rect: Rect, thickness: f32, color: Color) {
        if !rect.is_visible() || thickness <= 0.0 {
            return;
        }
        let t = thickness.min(rect.width / 2.0).min(rect.height / 2.0);
        self.fill_rect(Rect::new(rect.x, rect.y, rect.width, t), color);
        self.fill_rect(Rect::new(rect.x, rect.bottom() - t, rect.width, t), color);
        self.fill_rect(Rect::new(rect.x, rect.y, t, rect.height), color);
        self.fill_rect(Rect::new(rect.right() - t, rect.y, t, rect.height), color);
    }

    /// Append a command directly.
    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    /// Borrow all commands.
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Number of commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// True when no commands are present.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

const DEFAULT_RADIAL_GRADIENT_STEPS: u32 = 8;
const MAX_RADIAL_GRADIENT_STEPS: u32 = 24;
const MIN_ROUNDED_RECT_SEGMENTS: u32 = 4;
const MAX_ROUNDED_RECT_SEGMENTS: u32 = 18;

/// Approximate CSS-style soft shadow for a UI surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    /// Shadow offset in UI pixels.
    pub offset: Point,
    /// Approximate blur radius in UI pixels.
    pub blur: f32,
    /// Shadow spread in UI pixels.
    pub spread: f32,
    /// Shadow color.
    pub color: Color,
    /// Corner radius used by the shadow outline.
    pub corner_radius: f32,
    /// Number of translucent layers used to approximate the blur.
    pub steps: u32,
}

impl BoxShadow {
    /// Create a soft shadow.
    pub const fn new(offset: Point, blur: f32, spread: f32, color: Color) -> Self {
        Self {
            offset,
            blur,
            spread,
            color,
            corner_radius: 0.0,
            steps: 8,
        }
    }
}

/// Relative radial highlight used by `PanelStyle`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelRadialHighlight {
    /// Relative center where `(0, 0)` is the panel top-left and `(1, 1)` is bottom-right.
    pub center: Point,
    /// Radius as a multiplier of the larger panel dimension.
    pub radius_scale: f32,
    /// Highlight color at the center. The outer edge fades to transparent.
    pub color: Color,
}

impl PanelRadialHighlight {
    /// Create a relative radial panel highlight.
    pub const fn new(center: Point, radius_scale: f32, color: Color) -> Self {
        Self {
            center,
            radius_scale,
            color,
        }
    }
}

/// Shared panel surface style for lightweight plita tool panels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelStyle {
    /// Background color at the top edge.
    pub background_top: Color,
    /// Background color at the bottom edge.
    pub background_bottom: Color,
    /// Border color.
    pub border: Color,
    /// Subtle top-edge highlight.
    pub highlight: Color,
    /// Subtle bottom/right-edge shadow.
    pub shadow: Color,
    /// Border thickness in pixels.
    pub border_thickness: f32,
    /// Top highlight thickness in pixels.
    pub highlight_thickness: f32,
    /// Bottom/right shadow thickness in pixels.
    pub shadow_thickness: f32,
    /// Rounded corner radius in UI pixels.
    pub corner_radius: f32,
    /// Optional CSS-style soft box shadow approximation.
    pub box_shadow: Option<BoxShadow>,
    /// Optional radial highlight over the panel background.
    pub radial_highlight: Option<PanelRadialHighlight>,
}

impl PanelStyle {
    /// Create a panel style with a subtle default highlight and shadow.
    pub const fn new(background_top: Color, background_bottom: Color, border: Color) -> Self {
        Self {
            background_top,
            background_bottom,
            border,
            highlight: Color::rgba(1.0, 1.0, 1.0, 0.08),
            shadow: Color::rgba(0.0, 0.0, 0.0, 0.22),
            border_thickness: 1.0,
            highlight_thickness: 1.0,
            shadow_thickness: 1.0,
            corner_radius: 0.0,
            box_shadow: None,
            radial_highlight: None,
        }
    }

    /// Create a flat panel style with no highlight or shadow.
    pub const fn flat(background: Color, border: Color) -> Self {
        Self {
            background_top: background,
            background_bottom: background,
            border,
            highlight: Color::rgba(0.0, 0.0, 0.0, 0.0),
            shadow: Color::rgba(0.0, 0.0, 0.0, 0.0),
            border_thickness: 1.0,
            highlight_thickness: 0.0,
            shadow_thickness: 0.0,
            corner_radius: 0.0,
            box_shadow: None,
            radial_highlight: None,
        }
    }

    /// Default dark tool-panel style.
    pub const fn dark() -> Self {
        Self {
            background_top: Color::rgb8(35, 43, 55),
            background_bottom: Color::rgb8(20, 27, 36),
            border: Color::rgb8(84, 102, 122),
            highlight: Color::rgba(1.0, 1.0, 1.0, 0.08),
            shadow: Color::rgba(0.0, 0.0, 0.0, 0.22),
            border_thickness: 1.0,
            highlight_thickness: 1.0,
            shadow_thickness: 1.0,
            corner_radius: 6.0,
            box_shadow: Some(BoxShadow {
                offset: Point::new(0.0, 8.0),
                blur: 18.0,
                spread: 0.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.24),
                corner_radius: 6.0,
                steps: 8,
            }),
            radial_highlight: Some(PanelRadialHighlight {
                center: Point::new(0.35, 0.0),
                radius_scale: 0.9,
                color: Color::rgba(1.0, 1.0, 1.0, 0.055),
            }),
        }
    }
}

/// Draw a CSS-style soft box shadow approximation.
pub fn draw_box_shadow(list: &mut DrawList, rect: Rect, shadow: BoxShadow) {
    if !rect.is_visible() || shadow.color.a <= 0.0 {
        return;
    }

    let blur = finite_nonnegative(shadow.blur);
    let spread = finite_nonnegative(shadow.spread);
    let steps = shadow.steps.clamp(1, 24);
    if blur <= 0.0 {
        let shadow_rect = offset_rect(rect, shadow.offset).inset(-spread);
        list.fill_rounded_rect(shadow_rect, shadow.corner_radius + spread, shadow.color);
        return;
    }

    for index in (0..steps).rev() {
        let t = if steps <= 1 {
            0.0
        } else {
            index as f32 / (steps - 1) as f32
        };
        let expansion = spread + blur * t;
        let falloff = (1.0 - t).powf(1.6);
        let alpha_scale = (0.18 + falloff * 0.82) / steps as f32 * 1.35;
        let color = shadow.color.scale_alpha(alpha_scale);
        if color.a <= 0.0 {
            continue;
        }
        let shadow_rect = offset_rect(rect, shadow.offset).inset(-expansion);
        list.fill_rounded_rect(shadow_rect, shadow.corner_radius + expansion, color);
    }
}

/// Draw a shared plita panel surface with gradient, edge light, shadow, and border.
pub fn draw_panel(list: &mut DrawList, rect: Rect, style: PanelStyle) {
    if !rect.is_visible() {
        return;
    }

    let border = finite_nonnegative(style.border_thickness)
        .min(rect.width * 0.5)
        .min(rect.height * 0.5);
    let corner_radius = rounded_rect_radius(rect, style.corner_radius);

    if let Some(mut box_shadow) = style.box_shadow {
        if box_shadow.corner_radius <= 0.0 {
            box_shadow.corner_radius = corner_radius;
        }
        draw_box_shadow(list, rect, box_shadow);
    }

    let inner = if border > 0.0 {
        rect.inset(border)
    } else {
        rect
    };
    let background_rect = if corner_radius > 0.0 && border > 0.0 && style.border.a > 0.0 {
        list.fill_rounded_rect(rect, corner_radius, style.border);
        inner
    } else {
        rect
    };

    let inner_radius = rounded_rect_radius(inner, corner_radius - border);
    let background_radius = if background_rect == inner {
        inner_radius
    } else {
        corner_radius
    };
    if background_radius > 0.0 {
        list.fill_rounded_vertical_gradient_rect(
            background_rect,
            background_radius,
            style.background_top,
            style.background_bottom,
        );
    } else {
        list.fill_vertical_gradient_rect(
            background_rect,
            style.background_top,
            style.background_bottom,
        );
    }

    if let Some(highlight) = style.radial_highlight {
        draw_panel_radial_highlight(list, inner, inner_radius, highlight);
    }

    if inner.is_visible() {
        let highlight = finite_nonnegative(style.highlight_thickness).min(inner.height);
        if highlight > 0.0 && style.highlight.a > 0.0 {
            let x_inset = inner_radius.min(inner.width * 0.5);
            list.fill_rect(
                Rect::new(
                    inner.x + x_inset,
                    inner.y,
                    (inner.width - x_inset * 2.0).max(0.0),
                    highlight,
                ),
                style.highlight,
            );
        }

        let shadow = finite_nonnegative(style.shadow_thickness)
            .min(inner.width * 0.5)
            .min(inner.height * 0.5);
        if shadow > 0.0 && style.shadow.a > 0.0 {
            let x_inset = inner_radius.min(inner.width * 0.5);
            let y_inset = inner_radius.min(inner.height * 0.5);
            list.fill_rect(
                Rect::new(
                    inner.x + x_inset,
                    inner.bottom() - shadow,
                    (inner.width - x_inset * 2.0).max(0.0),
                    shadow,
                ),
                style.shadow,
            );
            list.fill_rect(
                Rect::new(
                    inner.right() - shadow,
                    inner.y + y_inset,
                    shadow,
                    (inner.height - y_inset * 2.0).max(0.0),
                ),
                style.shadow,
            );
        }
    }

    if corner_radius <= 0.0 && border > 0.0 && style.border.a > 0.0 {
        list.stroke_rect(rect, border, style.border);
    }
}

fn offset_rect(rect: Rect, offset: Point) -> Rect {
    Rect::new(
        rect.x + offset.x,
        rect.y + offset.y,
        rect.width,
        rect.height,
    )
}

fn draw_panel_radial_highlight(
    list: &mut DrawList,
    rect: Rect,
    corner_radius: f32,
    highlight: PanelRadialHighlight,
) {
    if !rect.is_visible() || highlight.color.a <= 0.0 {
        return;
    }

    let center = Point::new(
        rect.x + rect.width * highlight.center.x,
        rect.y + rect.height * highlight.center.y,
    );
    let radius = rect.width.max(rect.height) * finite_nonnegative(highlight.radius_scale);
    if radius <= 0.0 {
        return;
    }
    let outer = highlight.color.with_alpha(0.0);
    if corner_radius > 0.0 {
        list.fill_rounded_radial_gradient_rect(
            rect,
            corner_radius,
            center,
            radius,
            highlight.color,
            outer,
        );
    } else {
        list.fill_radial_gradient_rect(rect, center, radius, highlight.color, outer);
    }
}

fn rounded_rect_radius(rect: Rect, radius: f32) -> f32 {
    if !rect.is_visible() || !radius.is_finite() {
        return 0.0;
    }
    radius.max(0.0).min(rect.width * 0.5).min(rect.height * 0.5)
}

fn rounded_rect_segments(radius: f32) -> u32 {
    ((radius / 5.0).ceil() as u32).clamp(MIN_ROUNDED_RECT_SEGMENTS, MAX_ROUNDED_RECT_SEGMENTS)
}

fn rounded_rect_triangle_vertices<F>(rect: Rect, radius: f32, mut color_at: F) -> Vec<ColoredVertex>
where
    F: FnMut(Point) -> Color,
{
    let radius = rounded_rect_radius(rect, radius);
    if radius <= 0.0 {
        return Vec::new();
    }

    let segments = rounded_rect_segments(radius);
    let mut boundary = Vec::with_capacity((segments as usize + 1) * 4);
    push_arc_points(
        &mut boundary,
        Point::new(rect.right() - radius, rect.y + radius),
        radius,
        -std::f32::consts::FRAC_PI_2,
        0.0,
        segments,
    );
    push_arc_points(
        &mut boundary,
        Point::new(rect.right() - radius, rect.bottom() - radius),
        radius,
        0.0,
        std::f32::consts::FRAC_PI_2,
        segments,
    );
    push_arc_points(
        &mut boundary,
        Point::new(rect.x + radius, rect.bottom() - radius),
        radius,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        segments,
    );
    push_arc_points(
        &mut boundary,
        Point::new(rect.x + radius, rect.y + radius),
        radius,
        std::f32::consts::PI,
        std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
        segments,
    );

    if boundary.len() < 3 {
        return Vec::new();
    }

    let center = Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    let center_vertex = ColoredVertex::new(center, color_at(center));
    let mut vertices = Vec::with_capacity(boundary.len() * 3);
    for index in 0..boundary.len() {
        let a = boundary[index];
        let b = boundary[(index + 1) % boundary.len()];
        vertices.push(center_vertex);
        vertices.push(ColoredVertex::new(a, color_at(a)));
        vertices.push(ColoredVertex::new(b, color_at(b)));
    }
    vertices
}

fn push_arc_points(
    points: &mut Vec<Point>,
    center: Point,
    radius: f32,
    start: f32,
    end: f32,
    segments: u32,
) {
    let segments = segments.max(1);
    for index in 0..=segments {
        let t = index as f32 / segments as f32;
        let angle = start + (end - start) * t;
        points.push(Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        ));
    }
}

fn radial_gradient_rect_vertices(
    rect: Rect,
    center: Point,
    radius: f32,
    inner: Color,
    outer: Color,
    steps: u32,
) -> Vec<ColoredVertex> {
    if !rect.is_visible() || !radius.is_finite() || radius <= 0.0 {
        return Vec::new();
    }
    let steps = steps.clamp(1, MAX_RADIAL_GRADIENT_STEPS);
    let mut vertices = Vec::with_capacity((steps as usize * steps as usize) * 6);
    for y in 0..steps {
        let y0 = rect.y + rect.height * y as f32 / steps as f32;
        let y1 = rect.y + rect.height * (y + 1) as f32 / steps as f32;
        for x in 0..steps {
            let x0 = rect.x + rect.width * x as f32 / steps as f32;
            let x1 = rect.x + rect.width * (x + 1) as f32 / steps as f32;
            let p00 = Point::new(x0, y0);
            let p10 = Point::new(x1, y0);
            let p11 = Point::new(x1, y1);
            let p01 = Point::new(x0, y1);
            push_gradient_vertex(&mut vertices, p00, center, radius, inner, outer);
            push_gradient_vertex(&mut vertices, p10, center, radius, inner, outer);
            push_gradient_vertex(&mut vertices, p11, center, radius, inner, outer);
            push_gradient_vertex(&mut vertices, p00, center, radius, inner, outer);
            push_gradient_vertex(&mut vertices, p11, center, radius, inner, outer);
            push_gradient_vertex(&mut vertices, p01, center, radius, inner, outer);
        }
    }
    vertices
}

fn push_gradient_vertex(
    vertices: &mut Vec<ColoredVertex>,
    point: Point,
    center: Point,
    radius: f32,
    inner: Color,
    outer: Color,
) {
    vertices.push(ColoredVertex::new(
        point,
        radial_gradient_color(point, center, radius, inner, outer),
    ));
}

fn vertical_gradient_color(rect: Rect, y: f32, top: Color, bottom: Color) -> Color {
    if rect.height <= 0.0 {
        return top;
    }
    top.lerp(bottom, (y - rect.y) / rect.height)
}

fn radial_gradient_color(
    point: Point,
    center: Point,
    radius: f32,
    inner: Color,
    outer: Color,
) -> Color {
    if radius <= 0.0 {
        return outer;
    }
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    inner.lerp(outer, (dx * dx + dy * dy).sqrt() / radius)
}

/// Orientation of a scrollbar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollbarAxis {
    /// Left/right scrolling.
    Horizontal,
    /// Up/down scrolling.
    Vertical,
}

/// Part of a scrollbar hit by the pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollbarPart {
    /// The draggable thumb.
    Thumb,
    /// Track before the thumb.
    BackTrack,
    /// Track after the thumb.
    ForwardTrack,
}

/// Hit-test result for a scrollbar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollbarHit {
    /// Hit scrollbar axis.
    pub axis: ScrollbarAxis,
    /// Hit part of the scrollbar.
    pub part: ScrollbarPart,
}

/// Visual style for scrollbars drawn by plita.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarStyle {
    /// Thickness of each scrollbar track.
    pub thickness: f32,
    /// Inset from the viewport edge.
    pub margin: f32,
    /// Minimum thumb length.
    pub min_thumb_len: f32,
    /// Track color.
    pub track: Color,
    /// Normal thumb color.
    pub thumb: Color,
    /// Thumb color used by apps for hover feedback.
    pub thumb_hovered: Color,
    /// Thumb color used by apps while dragging.
    pub thumb_dragged: Color,
    /// Corner color when both axes are visible.
    pub corner: Color,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            thickness: 7.0,
            margin: 4.0,
            min_thumb_len: 18.0,
            track: Color::rgba8(18, 21, 26, 150),
            thumb: Color::rgba8(106, 158, 255, 170),
            thumb_hovered: Color::rgba8(134, 178, 255, 210),
            thumb_dragged: Color::rgba8(157, 196, 255, 230),
            corner: Color::rgba8(18, 21, 26, 180),
        }
    }
}

impl ScrollbarStyle {
    fn normalized(self) -> Self {
        Self {
            thickness: finite_at_least(self.thickness, 1.0),
            margin: finite_nonnegative(self.margin),
            min_thumb_len: finite_at_least(self.min_thumb_len, 1.0),
            ..self
        }
    }
}

/// Computed geometry for one visible scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarMetrics {
    /// Axis represented by this scrollbar.
    pub axis: ScrollbarAxis,
    /// Track rectangle in viewport coordinates.
    pub track: Rect,
    /// Thumb rectangle in viewport coordinates.
    pub thumb: Rect,
    /// Maximum offset on this axis.
    pub max_offset: f32,
    /// Visible viewport length on this axis.
    pub viewport_len: f32,
    /// Scrollable content length on this axis.
    pub content_len: f32,
}

impl ScrollbarMetrics {
    fn contains(self, point: Point) -> bool {
        self.track.contains(point)
    }

    fn point_value(self, point: Point) -> f32 {
        match self.axis {
            ScrollbarAxis::Horizontal => point.x,
            ScrollbarAxis::Vertical => point.y,
        }
    }

    fn track_start(self) -> f32 {
        match self.axis {
            ScrollbarAxis::Horizontal => self.track.x,
            ScrollbarAxis::Vertical => self.track.y,
        }
    }

    fn track_len(self) -> f32 {
        match self.axis {
            ScrollbarAxis::Horizontal => self.track.width,
            ScrollbarAxis::Vertical => self.track.height,
        }
    }

    fn thumb_start(self) -> f32 {
        match self.axis {
            ScrollbarAxis::Horizontal => self.thumb.x,
            ScrollbarAxis::Vertical => self.thumb.y,
        }
    }

    fn thumb_len(self) -> f32 {
        match self.axis {
            ScrollbarAxis::Horizontal => self.thumb.width,
            ScrollbarAxis::Vertical => self.thumb.height,
        }
    }

    fn scroll_for_thumb_delta(self, pointer_delta: f32) -> f32 {
        let travel = (self.track_len() - self.thumb_len()).max(0.0);
        if travel <= 0.0 || self.max_offset <= 0.0 {
            0.0
        } else {
            pointer_delta / travel * self.max_offset
        }
    }

    fn offset_for_track_point(self, point: Point) -> f32 {
        let travel = (self.track_len() - self.thumb_len()).max(0.0);
        if travel <= 0.0 || self.max_offset <= 0.0 {
            return 0.0;
        }
        let thumb_center = self.point_value(point) - self.thumb_len() * 0.5;
        ((thumb_center - self.track_start()) / travel * self.max_offset).clamp(0.0, self.max_offset)
    }
}

/// Computed scroll area geometry for drawing and hit testing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollAreaGeometry {
    /// Viewport rectangle in screen coordinates.
    pub viewport: Rect,
    /// Full content size.
    pub content_size: Size,
    /// Current clamped scroll offset.
    pub offset: Point,
    /// Maximum scroll offset.
    pub max_offset: Point,
    /// Vertical scrollbar metrics, when the content is taller than the viewport.
    pub vertical: Option<ScrollbarMetrics>,
    /// Horizontal scrollbar metrics, when the content is wider than the viewport.
    pub horizontal: Option<ScrollbarMetrics>,
    /// Corner filler when both axes are visible.
    pub corner: Option<Rect>,
}

impl ScrollAreaGeometry {
    /// Origin where content should be drawn after applying the scroll offset.
    pub fn content_origin(self) -> Point {
        Point::new(
            self.viewport.x - self.offset.x,
            self.viewport.y - self.offset.y,
        )
    }

    /// Content rectangle after applying the scroll offset.
    pub fn content_rect(self) -> Rect {
        let origin = self.content_origin();
        Rect::new(
            origin.x,
            origin.y,
            self.content_size.width,
            self.content_size.height,
        )
    }

    /// Hit-test visible scrollbars.
    pub fn hit_test(self, point: Point) -> Option<ScrollbarHit> {
        for metrics in [self.vertical, self.horizontal].into_iter().flatten() {
            if !metrics.contains(point) {
                continue;
            }
            let part = if metrics.thumb.contains(point) {
                ScrollbarPart::Thumb
            } else if metrics.point_value(point) < metrics.thumb_start() {
                ScrollbarPart::BackTrack
            } else {
                ScrollbarPart::ForwardTrack
            };
            return Some(ScrollbarHit {
                axis: metrics.axis,
                part,
            });
        }
        None
    }

    fn metrics(self, axis: ScrollbarAxis) -> Option<ScrollbarMetrics> {
        match axis {
            ScrollbarAxis::Horizontal => self.horizontal,
            ScrollbarAxis::Vertical => self.vertical,
        }
    }
}

/// Active drag data for a plita scroll area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarDrag {
    /// Axis being dragged.
    pub axis: ScrollbarAxis,
    /// Pointer position at drag start.
    pub start_pointer: Point,
    /// Scroll offset at drag start.
    pub start_offset: Point,
}

/// Retained state for a scrollable rectangular area.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollAreaState {
    offset: Point,
    drag: Option<ScrollbarDrag>,
}

impl ScrollAreaState {
    /// Create a scroll area at offset zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a scroll area from an existing offset.
    pub fn from_offset(offset: Point) -> Self {
        Self { offset, drag: None }
    }

    /// Current scroll offset.
    pub const fn offset(&self) -> Point {
        self.offset
    }

    /// Active scrollbar drag, if any.
    pub const fn drag(&self) -> Option<ScrollbarDrag> {
        self.drag
    }

    /// Return the maximum offset for the given viewport and content.
    pub fn max_offset(viewport: Rect, content_size: Size) -> Point {
        Point::new(
            (finite_nonnegative(content_size.width) - finite_nonnegative(viewport.width)).max(0.0),
            (finite_nonnegative(content_size.height) - finite_nonnegative(viewport.height))
                .max(0.0),
        )
    }

    /// Set and clamp the offset. Returns true when the offset changed.
    pub fn set_offset(&mut self, offset: Point, viewport: Rect, content_size: Size) -> bool {
        let max = Self::max_offset(viewport, content_size);
        let next = scroll_point(offset, Point::new(0.0, 0.0), max.x, max.y);
        let changed = self.offset != next;
        self.offset = next;
        changed
    }

    /// Scroll by a delta. Positive delta values move the visible content right/down.
    pub fn scroll_by(&mut self, delta: Point, viewport: Rect, content_size: Size) -> bool {
        let max = Self::max_offset(viewport, content_size);
        let next = scroll_point(self.offset, delta, max.x, max.y);
        let changed = self.offset != next;
        self.offset = next;
        changed
    }

    /// Handle a pointer wheel event using plita's normalized wheel deltas.
    pub fn wheel(
        &mut self,
        delta_x: f32,
        delta_y: f32,
        viewport: Rect,
        content_size: Size,
    ) -> bool {
        self.scroll_by(Point::new(-delta_x, -delta_y), viewport, content_size)
    }

    /// Compute draw and hit-test geometry.
    pub fn geometry(
        &self,
        viewport: Rect,
        content_size: Size,
        style: ScrollbarStyle,
    ) -> ScrollAreaGeometry {
        scroll_area_geometry(viewport, content_size, self.offset, style)
    }

    /// Content origin after applying the current scroll offset.
    pub fn content_origin(&self, viewport: Rect, content_size: Size) -> Point {
        self.geometry(viewport, content_size, ScrollbarStyle::default())
            .content_origin()
    }

    /// Push the viewport clip and return the scrolled content origin.
    pub fn push_clip(&self, list: &mut DrawList, viewport: Rect, content_size: Size) -> Point {
        list.push_clip(viewport);
        self.content_origin(viewport, content_size)
    }

    /// Draw visible horizontal and vertical scrollbars.
    pub fn draw_scrollbars(
        &self,
        list: &mut DrawList,
        viewport: Rect,
        content_size: Size,
        style: ScrollbarStyle,
    ) {
        let style = style.normalized();
        let geometry = self.geometry(viewport, content_size, style);
        draw_scrollbar_geometry(list, geometry, style, self.drag.map(|drag| drag.axis));
    }

    /// Hit-test this scroll area's scrollbars.
    pub fn hit_test_scrollbar(
        &self,
        viewport: Rect,
        content_size: Size,
        point: Point,
        style: ScrollbarStyle,
    ) -> Option<ScrollbarHit> {
        self.geometry(viewport, content_size, style).hit_test(point)
    }

    /// Begin a scrollbar interaction. Track clicks page the view immediately.
    pub fn begin_scrollbar_drag(
        &mut self,
        viewport: Rect,
        content_size: Size,
        point: Point,
        style: ScrollbarStyle,
    ) -> Option<ScrollbarHit> {
        let geometry = self.geometry(viewport, content_size, style);
        let hit = geometry.hit_test(point)?;
        match hit.part {
            ScrollbarPart::Thumb => {
                self.drag = Some(ScrollbarDrag {
                    axis: hit.axis,
                    start_pointer: point,
                    start_offset: self.offset,
                });
            }
            ScrollbarPart::BackTrack | ScrollbarPart::ForwardTrack => {
                if let Some(metrics) = geometry.metrics(hit.axis) {
                    let mut next = self.offset;
                    let target = metrics.offset_for_track_point(point);
                    match hit.axis {
                        ScrollbarAxis::Horizontal => next.x = target,
                        ScrollbarAxis::Vertical => next.y = target,
                    }
                    self.set_offset(next, viewport, content_size);
                }
                self.drag = None;
            }
        }
        Some(hit)
    }

    /// Continue an active scrollbar drag.
    pub fn drag_scrollbar(
        &mut self,
        viewport: Rect,
        content_size: Size,
        point: Point,
        style: ScrollbarStyle,
    ) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let geometry = self.geometry(viewport, content_size, style);
        let Some(metrics) = geometry.metrics(drag.axis) else {
            self.drag = None;
            return false;
        };
        let delta = match drag.axis {
            ScrollbarAxis::Horizontal => point.x - drag.start_pointer.x,
            ScrollbarAxis::Vertical => point.y - drag.start_pointer.y,
        };
        let mut next = drag.start_offset;
        let axis_delta = metrics.scroll_for_thumb_delta(delta);
        match drag.axis {
            ScrollbarAxis::Horizontal => next.x += axis_delta,
            ScrollbarAxis::Vertical => next.y += axis_delta,
        }
        self.set_offset(next, viewport, content_size)
    }

    /// End an active scrollbar drag.
    pub fn end_scrollbar_drag(&mut self) -> bool {
        let changed = self.drag.is_some();
        self.drag = None;
        changed
    }
}

fn scroll_area_geometry(
    viewport: Rect,
    content_size: Size,
    offset: Point,
    style: ScrollbarStyle,
) -> ScrollAreaGeometry {
    let style = style.normalized();
    let content_size = Size::new(
        finite_nonnegative(content_size.width),
        finite_nonnegative(content_size.height),
    );
    let max_offset = ScrollAreaState::max_offset(viewport, content_size);
    let offset = scroll_point(offset, Point::new(0.0, 0.0), max_offset.x, max_offset.y);
    let show_vertical = max_offset.y > 0.0 && viewport.height > style.thickness * 2.0;
    let show_horizontal = max_offset.x > 0.0 && viewport.width > style.thickness * 2.0;

    let vertical = if show_vertical {
        let reserved = if show_horizontal {
            style.thickness + style.margin
        } else {
            0.0
        };
        let track = Rect::new(
            viewport.right() - style.margin - style.thickness,
            viewport.y + style.margin,
            style.thickness,
            (viewport.height - style.margin * 2.0 - reserved).max(0.0),
        );
        make_scrollbar_metrics(
            ScrollbarAxis::Vertical,
            track,
            viewport.height,
            content_size.height,
            offset.y,
            style,
        )
    } else {
        None
    };

    let horizontal = if show_horizontal {
        let reserved = if show_vertical {
            style.thickness + style.margin
        } else {
            0.0
        };
        let track = Rect::new(
            viewport.x + style.margin,
            viewport.bottom() - style.margin - style.thickness,
            (viewport.width - style.margin * 2.0 - reserved).max(0.0),
            style.thickness,
        );
        make_scrollbar_metrics(
            ScrollbarAxis::Horizontal,
            track,
            viewport.width,
            content_size.width,
            offset.x,
            style,
        )
    } else {
        None
    };

    let corner = if show_vertical && show_horizontal {
        Some(Rect::new(
            viewport.right() - style.margin - style.thickness,
            viewport.bottom() - style.margin - style.thickness,
            style.thickness,
            style.thickness,
        ))
    } else {
        None
    };

    ScrollAreaGeometry {
        viewport,
        content_size,
        offset,
        max_offset,
        vertical,
        horizontal,
        corner,
    }
}

fn make_scrollbar_metrics(
    axis: ScrollbarAxis,
    track: Rect,
    viewport_len: f32,
    content_len: f32,
    offset: f32,
    style: ScrollbarStyle,
) -> Option<ScrollbarMetrics> {
    if !track.is_visible() || content_len <= viewport_len {
        return None;
    }
    let track_len = match axis {
        ScrollbarAxis::Horizontal => track.width,
        ScrollbarAxis::Vertical => track.height,
    };
    if track_len <= 0.0 {
        return None;
    }

    let max_offset = (content_len - viewport_len).max(0.0);
    let visible_ratio = (viewport_len / content_len).clamp(0.02, 1.0);
    let thumb_len = (track_len * visible_ratio)
        .max(style.min_thumb_len.min(track_len))
        .min(track_len);
    let travel = (track_len - thumb_len).max(0.0);
    let t = if max_offset > 0.0 {
        (offset / max_offset).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let thumb = match axis {
        ScrollbarAxis::Horizontal => {
            Rect::new(track.x + travel * t, track.y, thumb_len, track.height)
        }
        ScrollbarAxis::Vertical => Rect::new(track.x, track.y + travel * t, track.width, thumb_len),
    };

    Some(ScrollbarMetrics {
        axis,
        track,
        thumb,
        max_offset,
        viewport_len,
        content_len,
    })
}

fn draw_scrollbar_geometry(
    list: &mut DrawList,
    geometry: ScrollAreaGeometry,
    style: ScrollbarStyle,
    active_axis: Option<ScrollbarAxis>,
) {
    if let Some(corner) = geometry.corner {
        list.fill_rect(corner, style.corner);
    }
    for metrics in [geometry.vertical, geometry.horizontal]
        .into_iter()
        .flatten()
    {
        list.fill_rect(metrics.track, style.track);
        let thumb_color = if active_axis == Some(metrics.axis) {
            style.thumb_dragged
        } else {
            style.thumb
        };
        list.fill_rect(metrics.thumb, thumb_color);
    }
}

/// Layout constraints for future retained widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Minimum allowed size.
    pub min: Size,
    /// Maximum allowed size.
    pub max: Size,
}

impl Constraints {
    /// Create new constraints.
    pub const fn new(min: Size, max: Size) -> Self {
        Self { min, max }
    }
}

/// Stable widget identifier used by event routing, status text, and tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    /// Create a widget id from a stable integer.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Raw numeric id for diagnostics.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable canvas object identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ObjectId(u64);

impl ObjectId {
    /// Create an object id from a stable integer.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Raw numeric id for diagnostics.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Simple editable rectangle object used by the Phase 1 canvas prototype.
#[derive(Clone, Debug, PartialEq)]
pub struct EditableRect {
    /// Object id.
    pub id: ObjectId,
    /// Rectangle in canvas world coordinates.
    pub rect: Rect,
    /// Debug label.
    pub label: &'static str,
}

impl EditableRect {
    /// Create a new editable rectangle.
    pub const fn new(id: ObjectId, rect: Rect, label: &'static str) -> Self {
        Self { id, rect, label }
    }
}

/// Lightweight canvas document model exposed to applications and tests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasDocument {
    /// Canvas objects in paint order.
    pub objects: Vec<EditableRect>,
    /// Multi-selection set.
    pub selected: Vec<ObjectId>,
}

/// Layer-tree row derived from a canvas document.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerNode {
    /// Object id represented by this row.
    pub object_id: ObjectId,
    /// Debug/user-visible label.
    pub label: &'static str,
    /// Object bounds in world coordinates.
    pub bounds: Rect,
    /// Whether the row is selected.
    pub selected: bool,
}

impl CanvasDocument {
    /// Build a layer tree in front-to-back order.
    pub fn layer_tree(&self) -> Vec<LayerNode> {
        self.objects
            .iter()
            .rev()
            .map(|object| LayerNode {
                object_id: object.id,
                label: object.label,
                bounds: object.rect,
                selected: self.selected.contains(&object.id),
            })
            .collect()
    }
}

/// Minimal undoable command set for the Phase 1 canvas prototype.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasCommand {
    /// Create a canvas rectangle.
    CreateRect { object: EditableRect },
    /// Move an existing canvas rectangle.
    MoveRect { id: ObjectId, from: Rect, to: Rect },
    /// Resize an existing canvas rectangle.
    ResizeRect { id: ObjectId, from: Rect, to: Rect },
    /// Delete an existing canvas rectangle.
    DeleteRect { object: EditableRect },
}

/// Undo/redo stacks for Phase 1 canvas commands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandHistory {
    undo_stack: Vec<CanvasCommand>,
    redo_stack: Vec<CanvasCommand>,
}

impl CommandHistory {
    /// Create an empty command history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a command that has already been applied by an interactive gesture.
    pub fn record_done(&mut self, command: CanvasCommand) {
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    /// Number of commands that can be undone.
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of commands that can be redone.
    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }
}

/// Stable command identity used by menus, shortcuts, and app integrations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CommandId {
    /// Create a new demo document.
    NewDemo,
    /// Reset/open the demo document.
    OpenReset,
    /// Save a diagnostic snapshot.
    SaveSnapshot,
    /// Undo the last command.
    Undo,
    /// Redo the last undone command.
    Redo,
    /// Copy the current selection.
    Copy,
    /// Paste from the current clipboard.
    Paste,
    /// Fit the canvas view.
    FitView,
    /// Show all dock tabs.
    RestoreDockTabs,
    /// Detach the active right dock tab.
    DetachRight,
    /// Detach the active bottom dock tab.
    DetachBottom,
    /// Attach all floating dock panels.
    AttachFloating,
}

/// User-visible command metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandDescriptor {
    /// Stable command id.
    pub id: CommandId,
    /// Compact label suitable for menus and tooltips.
    pub label: &'static str,
    /// Optional keyboard shortcut shown in diagnostics.
    pub shortcut: Option<&'static str>,
}

impl CommandDescriptor {
    /// Create command metadata.
    pub const fn new(id: CommandId, label: &'static str, shortcut: Option<&'static str>) -> Self {
        Self {
            id,
            label,
            shortcut,
        }
    }
}

/// Small command registry for applications that want menu/shortcut parity.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandRegistry {
    commands: Vec<CommandDescriptor>,
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the default CreativeShell command registry.
    pub fn creative_shell_defaults() -> Self {
        let mut registry = Self::new();
        for command in [
            CommandDescriptor::new(CommandId::NewDemo, "NEW DEMO", None),
            CommandDescriptor::new(CommandId::OpenReset, "OPEN RESET", Some("Ctrl+O")),
            CommandDescriptor::new(CommandId::SaveSnapshot, "SAVE SNAPSHOT", Some("Ctrl+S")),
            CommandDescriptor::new(CommandId::Undo, "UNDO", Some("Ctrl+Z")),
            CommandDescriptor::new(CommandId::Redo, "REDO", Some("Ctrl+Y")),
            CommandDescriptor::new(CommandId::Copy, "COPY", Some("Ctrl+C")),
            CommandDescriptor::new(CommandId::Paste, "PASTE", Some("Ctrl+V")),
            CommandDescriptor::new(CommandId::FitView, "FIT VIEW", Some("F")),
            CommandDescriptor::new(CommandId::RestoreDockTabs, "RESTORE DOCK TABS", None),
            CommandDescriptor::new(CommandId::DetachRight, "DETACH RIGHT TAB", None),
            CommandDescriptor::new(CommandId::DetachBottom, "DETACH BOTTOM TAB", None),
            CommandDescriptor::new(CommandId::AttachFloating, "ATTACH FLOATING", None),
        ] {
            registry.register(command);
        }
        registry
    }

    /// Register or replace a command descriptor.
    pub fn register(&mut self, descriptor: CommandDescriptor) {
        if let Some(existing) = self
            .commands
            .iter_mut()
            .find(|item| item.id == descriptor.id)
        {
            *existing = descriptor;
        } else {
            self.commands.push(descriptor);
        }
    }

    /// Lookup a command descriptor.
    pub fn get(&self, id: CommandId) -> Option<&CommandDescriptor> {
        self.commands.iter().find(|item| item.id == id)
    }

    /// Borrow registered commands in insertion order.
    pub fn commands(&self) -> &[CommandDescriptor] {
        &self.commands
    }
}

/// Configurable grid input area for pixel, step, and waveform style editing.
#[derive(Clone, Debug, PartialEq)]
pub struct GridInputConfig {
    /// Number of columns.
    pub columns: usize,
    /// Number of rows.
    pub rows: usize,
    /// Cell size in canvas world units.
    pub cell_size: f32,
}

impl GridInputConfig {
    /// Create a grid input configuration.
    pub fn new(columns: usize, rows: usize, cell_size: f32) -> Self {
        Self {
            columns: columns.max(1),
            rows: rows.max(1),
            cell_size: finite_at_least(cell_size, 1.0),
        }
    }
}

impl Default for GridInputConfig {
    fn default() -> Self {
        Self::new(16, 8, 12.0)
    }
}

/// Editable binary grid used for dots, pattern steps, and coarse waveform input.
#[derive(Clone, Debug, PartialEq)]
pub struct GridInput {
    /// Grid configuration.
    pub config: GridInputConfig,
    /// Cell values in row-major order. A value of 0 is off; non-zero is on.
    pub cells: Vec<u8>,
}

impl GridInput {
    /// Create a new grid input from a configuration.
    pub fn new(config: GridInputConfig) -> Self {
        let len = config.columns.saturating_mul(config.rows).max(1);
        Self {
            config,
            cells: vec![0; len],
        }
    }

    /// Number of active cells.
    pub fn active_count(&self) -> usize {
        self.cells.iter().filter(|value| **value != 0).count()
    }

    /// Return a cell index for a column and row.
    pub fn index(&self, column: usize, row: usize) -> Option<usize> {
        if column < self.config.columns && row < self.config.rows {
            Some(row * self.config.columns + column)
        } else {
            None
        }
    }

    /// Set a cell value.
    pub fn set(&mut self, column: usize, row: usize, value: bool) {
        if let Some(index) = self.index(column, row) {
            if let Some(cell) = self.cells.get_mut(index) {
                *cell = u8::from(value);
            }
        }
    }

    /// Toggle a cell and return the new value.
    pub fn toggle_index(&mut self, index: usize) -> Option<u8> {
        let cell = self.cells.get_mut(index)?;
        *cell = if *cell == 0 { 1 } else { 0 };
        Some(*cell)
    }

    /// Set a cell by flat index.
    pub fn set_index(&mut self, index: usize, value: u8) {
        if let Some(cell) = self.cells.get_mut(index) {
            *cell = value;
        }
    }
}

/// Bootstrap floating window rectangle and visibility.
#[derive(Clone, Debug, PartialEq)]
pub struct FloatingWindowState {
    /// Window rectangle in screen coordinates.
    pub rect: Rect,
    /// Whether the window is visible.
    pub open: bool,
}

/// Simple memory-viewer model for debugger-style windows.
#[derive(Clone, Debug, PartialEq)]
pub struct MemoryViewer {
    /// First visible address.
    pub base_address: u16,
    /// Bytes shown by the viewer.
    pub bytes: Vec<u8>,
}

impl MemoryViewer {
    /// Create deterministic demo memory.
    pub fn demo() -> Self {
        let bytes = (0..256)
            .map(|index| ((index * 17 + 3) & 0xff) as u8)
            .collect();
        Self {
            base_address: 0x8000,
            bytes,
        }
    }

    /// Replace the memory contents.
    pub fn set_bytes(&mut self, base_address: u16, bytes: Vec<u8>) {
        self.base_address = base_address;
        self.bytes = bytes;
    }
}

/// Result of editing one memory byte through a retained memory editor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryEdit {
    /// Edited byte offset from the viewer base address.
    pub offset: usize,
    /// Byte value before the edit.
    pub old_value: u8,
    /// Byte value after the edit.
    pub new_value: u8,
}

/// Retained editable hex memory-view state.
#[derive(Clone, Debug, PartialEq)]
pub struct MemoryEditorState {
    /// Selected byte offset from the viewer base address.
    pub selected_offset: usize,
    /// Whether the next hex digit edits the high nibble.
    pub high_nibble: bool,
    /// Whether the memory editor currently owns keyboard focus.
    pub focused: bool,
}

impl Default for MemoryEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryEditorState {
    /// Number of bytes shown per row by plita's built-in memory editor.
    pub const BYTES_PER_ROW: usize = 8;
    /// Height of the header strip.
    pub const HEADER_HEIGHT: f32 = 42.0;
    /// Height of one byte row.
    pub const ROW_HEIGHT: f32 = 18.0;
    /// X offset for the first hex byte.
    pub const HEX_X: f32 = 76.0;
    /// Width of one hex byte cell.
    pub const HEX_CELL_W: f32 = 25.0;

    /// Create a memory editor at the first byte.
    pub const fn new() -> Self {
        Self {
            selected_offset: 0,
            high_nibble: true,
            focused: false,
        }
    }

    /// Clamp selection to the available byte length.
    pub fn clamp_to_len(&mut self, len: usize) {
        if len == 0 {
            self.selected_offset = 0;
            self.high_nibble = true;
        } else if self.selected_offset >= len {
            self.selected_offset = len - 1;
            self.high_nibble = true;
        }
    }

    /// Select a byte offset and focus the editor.
    pub fn select_offset(&mut self, offset: usize, len: usize) {
        if len == 0 {
            self.selected_offset = 0;
        } else {
            self.selected_offset = offset.min(len - 1);
        }
        self.high_nibble = true;
        self.focused = true;
    }

    /// Move selected byte by a signed delta.
    pub fn move_by(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.selected_offset = 0;
            self.high_nibble = true;
            return;
        }
        let next = self
            .selected_offset
            .saturating_add_signed(delta)
            .min(len - 1);
        self.selected_offset = next;
        self.high_nibble = true;
    }

    /// Move to the first byte in the current row.
    pub fn move_row_home(&mut self) {
        self.selected_offset = self.selected_offset / Self::BYTES_PER_ROW * Self::BYTES_PER_ROW;
        self.high_nibble = true;
    }

    /// Move to the last byte in the current row.
    pub fn move_row_end(&mut self, len: usize) {
        if len == 0 {
            self.selected_offset = 0;
            self.high_nibble = true;
            return;
        }
        let row_start = self.selected_offset / Self::BYTES_PER_ROW * Self::BYTES_PER_ROW;
        self.selected_offset = (row_start + Self::BYTES_PER_ROW - 1).min(len - 1);
        self.high_nibble = true;
    }

    /// Edit the selected byte using a single hex digit.
    pub fn edit_hex_digit(&mut self, bytes: &mut [u8], digit: u8) -> Option<MemoryEdit> {
        if digit > 0x0f || bytes.is_empty() {
            return None;
        }
        self.clamp_to_len(bytes.len());
        let offset = self.selected_offset;
        let old_value = bytes[offset];
        let new_value = if self.high_nibble {
            (old_value & 0x0f) | (digit << 4)
        } else {
            (old_value & 0xf0) | digit
        };
        bytes[offset] = new_value;
        if self.high_nibble {
            self.high_nibble = false;
        } else {
            self.high_nibble = true;
            if offset + 1 < bytes.len() {
                self.selected_offset = offset + 1;
            }
        }
        Some(MemoryEdit {
            offset,
            old_value,
            new_value,
        })
    }

    /// Rectangle for a byte cell in a memory-editor drawing rectangle.
    pub fn byte_rect(rect: Rect, offset: usize) -> Rect {
        Self::byte_rect_scaled(rect, offset, 1.0)
    }

    /// Rectangle for a byte cell in a scaled memory-editor drawing rectangle.
    pub fn byte_rect_scaled(rect: Rect, offset: usize, font_scale: f32) -> Rect {
        let metrics = MemoryEditorMetrics::new(font_scale);
        let row = offset / Self::BYTES_PER_ROW;
        let col = offset % Self::BYTES_PER_ROW;
        Rect::new(
            rect.x + metrics.hex_x + col as f32 * metrics.hex_cell_w - 3.0 * metrics.scale,
            rect.y + metrics.header_height + 5.0 * metrics.scale + row as f32 * metrics.row_height,
            metrics.hex_cell_w - 4.0 * metrics.scale,
            metrics.row_height - 3.0 * metrics.scale,
        )
    }

    /// Hit-test a byte cell in a memory-editor drawing rectangle.
    pub fn hit_offset(rect: Rect, point: Point, len: usize) -> Option<usize> {
        Self::hit_offset_scaled(rect, point, len, 1.0)
    }

    /// Hit-test a byte cell in a scaled memory-editor drawing rectangle.
    pub fn hit_offset_scaled(
        rect: Rect,
        point: Point,
        len: usize,
        font_scale: f32,
    ) -> Option<usize> {
        let metrics = MemoryEditorMetrics::new(font_scale);
        if len == 0 || !rect.contains(point) {
            return None;
        }
        let y = point.y - (rect.y + metrics.header_height + 5.0 * metrics.scale);
        if y < 0.0 {
            return None;
        }
        let row = (y / metrics.row_height).floor() as usize;
        let x = point.x - (rect.x + metrics.hex_x - 3.0 * metrics.scale);
        if x < 0.0 {
            return None;
        }
        let col = (x / metrics.hex_cell_w).floor() as usize;
        if col >= Self::BYTES_PER_ROW {
            return None;
        }
        let offset = row * Self::BYTES_PER_ROW + col;
        (offset < len).then_some(offset)
    }

    /// Visible byte rows for a scaled memory-editor drawing rectangle.
    pub fn rows_for_rect_scaled(rect: Rect, font_scale: f32) -> usize {
        let metrics = MemoryEditorMetrics::new(font_scale);
        ((rect.height - metrics.header_height - 8.0 * metrics.scale) / metrics.row_height)
            .floor()
            .max(0.0) as usize
    }
}

#[derive(Clone, Copy, Debug)]
struct MemoryEditorMetrics {
    scale: f32,
    header_height: f32,
    row_height: f32,
    hex_x: f32,
    hex_cell_w: f32,
    title_scale: f32,
    header_scale: f32,
    body_scale: f32,
}

impl MemoryEditorMetrics {
    fn new(font_scale: f32) -> Self {
        let scale = if font_scale.is_finite() {
            font_scale.clamp(0.75, 2.25)
        } else {
            1.0
        };
        Self {
            scale,
            header_height: MemoryEditorState::HEADER_HEIGHT * scale,
            row_height: MemoryEditorState::ROW_HEIGHT * scale,
            hex_x: MemoryEditorState::HEX_X * scale,
            hex_cell_w: MemoryEditorState::HEX_CELL_W * scale,
            title_scale: 0.95 * scale,
            header_scale: 0.62 * scale,
            body_scale: 0.68 * scale,
        }
    }
}

/// Color tokens for debugger-style hex memory views.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemoryViewerPalette {
    /// Viewer background.
    pub background: Color,
    /// Header strip background.
    pub header: Color,
    /// Border and separator color.
    pub border: Color,
    /// Primary text color.
    pub text: Color,
    /// Muted address/header text color.
    pub muted: Color,
    /// Accent color for the current base address.
    pub accent: Color,
}

impl MemoryViewerPalette {
    /// Default dark debugger palette.
    pub const fn dark() -> Self {
        Self {
            background: Color::rgb8(14, 16, 20),
            header: Color::rgb8(24, 28, 34),
            border: Color::rgb8(66, 74, 88),
            text: Color::rgb8(225, 231, 238),
            muted: Color::rgb8(138, 149, 166),
            accent: Color::rgb8(132, 190, 255),
        }
    }
}

/// Draw a compact Mesen-style hex memory viewer.
///
/// Rows are eight bytes wide so the view remains legible in narrow inspector
/// panels. The `bytes` slice is rendered from `base_address`.
pub fn draw_memory_viewer(
    list: &mut DrawList,
    rect: Rect,
    title: &str,
    base_address: u16,
    bytes: &[u8],
    palette: MemoryViewerPalette,
) {
    const BYTES_PER_ROW: usize = MEMORY_VIEWER_BYTES_PER_ROW;
    const HEADER_HEIGHT: f32 = MEMORY_VIEWER_HEADER_HEIGHT;
    const ROW_HEIGHT: f32 = MEMORY_VIEWER_ROW_HEIGHT;

    if !rect.is_visible() {
        return;
    }

    list.fill_rect(rect, palette.background);
    list.stroke_rect(rect, 1.0, palette.border);
    let header = Rect::new(rect.x, rect.y, rect.width, HEADER_HEIGHT);
    list.fill_rect(header, palette.header);
    list.stroke_rect(header, 1.0, palette.border);
    draw_text(list, rect.x + 10.0, rect.y + 9.0, title, 0.95, palette.text);
    draw_bitmap_text(
        list,
        rect.x + 10.0,
        rect.y + 28.0,
        "ADDR",
        0.62,
        palette.muted,
    );
    for col in 0..BYTES_PER_ROW {
        let cell = memory_viewer_byte_rect(rect, 0, col);
        draw_bitmap_text(
            list,
            cell.x,
            rect.y + 28.0,
            &format!("{col:02X}"),
            0.62,
            palette.muted,
        );
    }
    let ascii_x = memory_viewer_byte_rect(rect, 0, BYTES_PER_ROW).x
        + BITMAP_SPACE_ADVANCE * MEMORY_VIEWER_TEXT_SCALE * BITMAP_SCALE_FACTOR;
    draw_bitmap_text(list, ascii_x, rect.y + 28.0, "ASCII", 0.62, palette.muted);

    list.push_clip(rect.inset(1.0));
    let rows = ((rect.height - HEADER_HEIGHT - 8.0) / ROW_HEIGHT)
        .floor()
        .max(0.0) as usize;
    let mut y = rect.y + HEADER_HEIGHT + 7.0;
    for row in 0..rows {
        let offset = row * BYTES_PER_ROW;
        if offset >= bytes.len() {
            break;
        }
        let row_bytes = &bytes[offset..bytes.len().min(offset + BYTES_PER_ROW)];
        let address = base_address.wrapping_add(offset as u16);
        let ascii: String = row_bytes
            .iter()
            .map(|byte| {
                if byte.is_ascii_graphic() {
                    char::from(*byte)
                } else {
                    '.'
                }
            })
            .collect();
        let color = if row == 0 {
            palette.accent
        } else {
            palette.text
        };
        draw_bitmap_text(
            list,
            rect.x + 10.0,
            y,
            &format!("{address:04X}:"),
            MEMORY_VIEWER_TEXT_SCALE,
            color,
        );
        for (col, byte) in row_bytes.iter().enumerate() {
            let cell = memory_viewer_byte_rect(rect, row, col);
            draw_bitmap_text(
                list,
                cell.x,
                cell.y,
                &format!("{byte:02X}"),
                MEMORY_VIEWER_TEXT_SCALE,
                color,
            );
        }
        draw_bitmap_text(list, ascii_x, y, &ascii, MEMORY_VIEWER_TEXT_SCALE, color);
        y += ROW_HEIGHT;
    }
    list.pop_clip();
}

const MEMORY_VIEWER_BYTES_PER_ROW: usize = 8;
const MEMORY_VIEWER_HEADER_HEIGHT: f32 = 42.0;
const MEMORY_VIEWER_ROW_HEIGHT: f32 = 14.0;
const MEMORY_VIEWER_TEXT_SCALE: f32 = 0.68;

fn memory_viewer_byte_rect(rect: Rect, row: usize, col: usize) -> Rect {
    let scale = MEMORY_VIEWER_TEXT_SCALE * BITMAP_SCALE_FACTOR;
    let pitch = BITMAP_SPACE_ADVANCE + 2.0 * BITMAP_GLYPH_ADVANCE;
    Rect::new(
        rect.x
            + 10.0
            + (5.0 * BITMAP_GLYPH_ADVANCE + BITMAP_SPACE_ADVANCE + col as f32 * pitch) * scale,
        rect.y + MEMORY_VIEWER_HEADER_HEIGHT + 7.0 + row as f32 * MEMORY_VIEWER_ROW_HEIGHT,
        2.0 * BITMAP_GLYPH_ADVANCE * scale,
        MEMORY_VIEWER_ROW_HEIGHT,
    )
}

/// Hit-test only the visible hex bytes drawn by `draw_memory_viewer`.
/// Address labels, ASCII text, inter-byte gaps and unused cells are not editable hits.
pub fn memory_viewer_address_at_point(
    rect: Rect,
    base_address: u16,
    byte_count: usize,
    position: Point,
) -> Option<u16> {
    if !rect.inset(1.0).contains(position) {
        return None;
    }
    let rows = ((rect.height - MEMORY_VIEWER_HEADER_HEIGHT - 8.0) / MEMORY_VIEWER_ROW_HEIGHT)
        .floor()
        .max(0.0) as usize;
    let row_y = position.y - rect.y - MEMORY_VIEWER_HEADER_HEIGHT - 7.0;
    if row_y < 0.0 {
        return None;
    }
    let row = (row_y / MEMORY_VIEWER_ROW_HEIGHT).floor() as usize;
    if row >= rows {
        return None;
    }
    for col in 0..MEMORY_VIEWER_BYTES_PER_ROW {
        let index = row * MEMORY_VIEWER_BYTES_PER_ROW + col;
        if index < byte_count && memory_viewer_byte_rect(rect, row, col).contains(position) {
            return Some(base_address.wrapping_add(index as u16));
        }
    }
    None
}

/// Draw an editable compact hex memory view.
pub fn draw_memory_editor(
    list: &mut DrawList,
    rect: Rect,
    title: &str,
    base_address: u16,
    bytes: &[u8],
    editor: &MemoryEditorState,
    palette: MemoryViewerPalette,
) {
    draw_memory_editor_scaled(list, rect, title, base_address, bytes, editor, palette, 1.0);
}

/// Draw an editable hex memory view with scaled text and hit-test-compatible layout.
#[allow(clippy::too_many_arguments)]
pub fn draw_memory_editor_scaled(
    list: &mut DrawList,
    rect: Rect,
    title: &str,
    base_address: u16,
    bytes: &[u8],
    editor: &MemoryEditorState,
    palette: MemoryViewerPalette,
    font_scale: f32,
) {
    if !rect.is_visible() {
        return;
    }
    let metrics = MemoryEditorMetrics::new(font_scale);

    list.fill_rect(rect, palette.background);
    list.stroke_rect(rect, 1.0, palette.border);
    let header = Rect::new(rect.x, rect.y, rect.width, metrics.header_height);
    list.fill_rect(header, palette.header);
    list.stroke_rect(header, 1.0, palette.border);
    draw_text(
        list,
        rect.x + 10.0 * metrics.scale,
        rect.y + 9.0 * metrics.scale,
        title,
        metrics.title_scale,
        palette.text,
    );
    draw_memory_text(
        list,
        rect.x + 10.0 * metrics.scale,
        rect.y + 28.0 * metrics.scale,
        "ADDR    00 01 02 03 04 05 06 07   ASCII",
        metrics.header_scale,
        palette.muted,
    );

    let body = Rect::new(
        rect.x + 1.0,
        rect.y + metrics.header_height,
        rect.width - 2.0,
        rect.height - metrics.header_height - 1.0,
    );
    list.push_clip(body);

    let rows = MemoryEditorState::rows_for_rect_scaled(rect, metrics.scale);
    let ascii_x = rect.x + metrics.hex_x + metrics.hex_cell_w * 8.0 + 9.0 * metrics.scale;
    for row in 0..rows {
        let offset = row * MemoryEditorState::BYTES_PER_ROW;
        if offset >= bytes.len() {
            break;
        }
        let y =
            rect.y + metrics.header_height + 8.0 * metrics.scale + row as f32 * metrics.row_height;
        let address = base_address.wrapping_add(offset as u16);
        draw_memory_text(
            list,
            rect.x + 10.0 * metrics.scale,
            y,
            &format!("{address:04X}:"),
            metrics.body_scale,
            palette.muted,
        );

        let row_end = (offset + MemoryEditorState::BYTES_PER_ROW).min(bytes.len());
        let row_bytes = &bytes[offset..row_end];
        for (col, byte) in row_bytes.iter().enumerate() {
            let byte_offset = offset + col;
            let cell = MemoryEditorState::byte_rect_scaled(rect, byte_offset, metrics.scale);
            if editor.selected_offset == byte_offset {
                list.fill_rect(
                    cell,
                    if editor.focused {
                        Color::rgba8(73, 123, 201, 210)
                    } else {
                        Color::rgba8(66, 74, 88, 190)
                    },
                );
                let nibble_x = if editor.high_nibble {
                    cell.x + 3.0 * metrics.scale
                } else {
                    cell.x + 10.0 * metrics.scale
                };
                list.fill_rect(
                    Rect::new(
                        nibble_x,
                        cell.bottom() - 2.0 * metrics.scale,
                        6.0 * metrics.scale,
                        2.0 * metrics.scale,
                    ),
                    palette.accent,
                );
            }
            draw_memory_text(
                list,
                cell.x + 3.0 * metrics.scale,
                y,
                &format!("{:02X}", byte),
                metrics.body_scale,
                palette.text,
            );
        }

        let ascii: String = row_bytes
            .iter()
            .map(|byte| {
                if byte.is_ascii_graphic() {
                    char::from(*byte)
                } else {
                    '.'
                }
            })
            .collect();
        draw_memory_text(list, ascii_x, y, &ascii, metrics.body_scale, palette.muted);
    }
    list.pop_clip();
}

/// Retained single-line text input state used by the Phase 1 inspector prototype.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputState {
    /// Current text.
    pub text: String,
    /// Byte cursor position. The prototype normalizes this to a UTF-8 character boundary.
    pub cursor: usize,
    /// Whether this input currently owns keyboard focus.
    pub focused: bool,
    /// Current IME preedit/composition text, if any.
    pub ime_preedit: String,
}

impl TextInputState {
    /// Create a text input with an initial value.
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self {
            text,
            cursor,
            focused: false,
            ime_preedit: String::new(),
        }
    }

    /// Move cursor to a valid UTF-8 character boundary no larger than `cursor`.
    pub fn normalize_cursor(&mut self) {
        if self.cursor > self.text.len() {
            self.cursor = self.text.len();
        }
        while self.cursor > 0 && !self.text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
    }

    /// Insert text at the current cursor.
    pub fn insert_text(&mut self, value: &str) {
        self.normalize_cursor();
        self.text.insert_str(self.cursor, value);
        self.cursor += value.len();
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        self.normalize_cursor();
        if self.cursor == 0 {
            return;
        }
        let mut start = self.cursor - 1;
        while start > 0 && !self.text.is_char_boundary(start) {
            start -= 1;
        }
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    /// Delete the character after the cursor.
    pub fn delete_forward(&mut self) {
        self.normalize_cursor();
        if self.cursor >= self.text.len() {
            return;
        }
        let mut end = self.cursor + 1;
        while end < self.text.len() && !self.text.is_char_boundary(end) {
            end += 1;
        }
        self.text.replace_range(self.cursor..end, "");
    }

    /// Move cursor left by one Unicode scalar value.
    pub fn move_left(&mut self) {
        self.normalize_cursor();
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        while self.cursor > 0 && !self.text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
    }

    /// Move cursor right by one Unicode scalar value.
    pub fn move_right(&mut self) {
        self.normalize_cursor();
        if self.cursor >= self.text.len() {
            return;
        }
        self.cursor += 1;
        while self.cursor < self.text.len() && !self.text.is_char_boundary(self.cursor) {
            self.cursor += 1;
        }
    }

    /// Move cursor to the start of the line.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end of the line.
    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
        self.normalize_cursor();
    }

    /// Return the text before the cursor.
    pub fn text_before_cursor(&self) -> &str {
        let mut cursor = self.cursor.min(self.text.len());
        while cursor > 0 && !self.text.is_char_boundary(cursor) {
            cursor -= 1;
        }
        &self.text[..cursor]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextAction {
    AddRect,
    ToggleGridCell,
    MemoryViewer,
    Close,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuCommand {
    NewDemo,
    OpenReset,
    SaveSnapshot,
    Undo,
    Redo,
    Copy,
    Paste,
    FitView,
    ZoomIn,
    ZoomOut,
    ToggleMemory,
    ShowProperties,
    ShowLayers,
    ShowMemoryPanel,
    ShowTimeline,
    ShowLog,
    ShowDiagnostics,
    ShowDockLayoutInfo,
    ResetDockLayout,
    SaveDockLayout,
    LoadDockLayout,
    RestoreDockTabs,
    SaveDockLayoutSlotA,
    LoadDockLayoutSlotA,
    SaveDockLayoutSlotB,
    LoadDockLayoutSlotB,
    FloatProperties,
    FloatLayers,
    FloatMemory,
    FloatTimeline,
    FloatLog,
    FloatDiagnostics,
    DetachActiveRight,
    DetachActiveBottom,
    DockAllFloating,
    About,
}

const FILE_MENU_COMMANDS: [MenuCommand; 4] = [
    MenuCommand::NewDemo,
    MenuCommand::OpenReset,
    MenuCommand::SaveSnapshot,
    MenuCommand::About,
];
const EDIT_MENU_COMMANDS: [MenuCommand; 4] = [
    MenuCommand::Undo,
    MenuCommand::Redo,
    MenuCommand::Copy,
    MenuCommand::Paste,
];
const VIEW_MENU_COMMANDS: [MenuCommand; 4] = [
    MenuCommand::FitView,
    MenuCommand::ZoomIn,
    MenuCommand::ZoomOut,
    MenuCommand::ToggleMemory,
];
const WINDOW_MENU_COMMANDS: [MenuCommand; 24] = [
    MenuCommand::ShowProperties,
    MenuCommand::ShowLayers,
    MenuCommand::ShowMemoryPanel,
    MenuCommand::ShowTimeline,
    MenuCommand::ShowLog,
    MenuCommand::ShowDiagnostics,
    MenuCommand::FloatProperties,
    MenuCommand::FloatLayers,
    MenuCommand::FloatMemory,
    MenuCommand::FloatTimeline,
    MenuCommand::FloatLog,
    MenuCommand::FloatDiagnostics,
    MenuCommand::DetachActiveRight,
    MenuCommand::DetachActiveBottom,
    MenuCommand::DockAllFloating,
    MenuCommand::ShowDockLayoutInfo,
    MenuCommand::SaveDockLayout,
    MenuCommand::LoadDockLayout,
    MenuCommand::SaveDockLayoutSlotA,
    MenuCommand::LoadDockLayoutSlotA,
    MenuCommand::SaveDockLayoutSlotB,
    MenuCommand::LoadDockLayoutSlotB,
    MenuCommand::RestoreDockTabs,
    MenuCommand::ResetDockLayout,
];
const HELP_MENU_COMMANDS: [MenuCommand; 4] = [
    MenuCommand::About,
    MenuCommand::SaveSnapshot,
    MenuCommand::FloatTimeline,
    MenuCommand::ShowDiagnostics,
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct ContextMenuState {
    position: Point,
}

const RICH_APPLY_ID: WidgetId = WidgetId::new(300);
const RICH_RESET_ID: WidgetId = WidgetId::new(301);
const SLIDER_ID: WidgetId = WidgetId::new(310);
const TEXT_INPUT_ID: WidgetId = WidgetId::new(500);
const BOTTOM_SPLITTER_ID: WidgetId = WidgetId::new(10);

/// Right-side dock panel shown inside the inspector region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectorPanelKind {
    /// Object/property inspector.
    Properties,
    /// Layer/object list.
    Layers,
    /// Debugger-oriented memory summary.
    Memory,
}

impl InspectorPanelKind {
    /// User-visible tab label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Properties => "PROP",
            Self::Layers => "LAYERS",
            Self::Memory => "MEM",
        }
    }

    /// Stable widget id for the tab.
    pub const fn widget_id(self) -> WidgetId {
        match self {
            Self::Properties => WidgetId::new(600),
            Self::Layers => WidgetId::new(601),
            Self::Memory => WidgetId::new(602),
        }
    }
}

/// Bottom dock panel shown above the status bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BottomPanelKind {
    /// DAW-style timeline panel.
    Timeline,
    /// Text/log style debugger panel.
    Log,
    /// Compact diagnostic dashboard.
    Diagnostics,
}

impl BottomPanelKind {
    /// User-visible tab label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Timeline => "TIMELINE",
            Self::Log => "LOG",
            Self::Diagnostics => "DIAG",
        }
    }

    /// Stable widget id for the tab.
    pub const fn widget_id(self) -> WidgetId {
        match self {
            Self::Timeline => WidgetId::new(620),
            Self::Log => WidgetId::new(621),
            Self::Diagnostics => WidgetId::new(622),
        }
    }
}

/// Floating dock-panel kind used by Phase 2 v2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloatingDockKind {
    /// Floating copy of an inspector-side panel.
    Inspector(InspectorPanelKind),
    /// Floating copy of a bottom-dock panel.
    Bottom(BottomPanelKind),
}

impl FloatingDockKind {
    /// User-visible label for the floating dock title bar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inspector(InspectorPanelKind::Properties) => "FLOAT PROP",
            Self::Inspector(InspectorPanelKind::Layers) => "FLOAT LAYERS",
            Self::Inspector(InspectorPanelKind::Memory) => "FLOAT MEMORY",
            Self::Bottom(BottomPanelKind::Timeline) => "FLOAT TIMELINE",
            Self::Bottom(BottomPanelKind::Log) => "FLOAT LOG",
            Self::Bottom(BottomPanelKind::Diagnostics) => "FLOAT DIAG",
        }
    }

    /// Stable title-bar widget id.
    pub const fn title_widget_id(self) -> WidgetId {
        match self {
            Self::Inspector(InspectorPanelKind::Properties) => WidgetId::new(700),
            Self::Inspector(InspectorPanelKind::Layers) => WidgetId::new(701),
            Self::Inspector(InspectorPanelKind::Memory) => WidgetId::new(702),
            Self::Bottom(BottomPanelKind::Timeline) => WidgetId::new(703),
            Self::Bottom(BottomPanelKind::Log) => WidgetId::new(704),
            Self::Bottom(BottomPanelKind::Diagnostics) => WidgetId::new(705),
        }
    }

    /// Stable close-button widget id.
    pub const fn close_widget_id(self) -> WidgetId {
        match self {
            Self::Inspector(InspectorPanelKind::Properties) => WidgetId::new(710),
            Self::Inspector(InspectorPanelKind::Layers) => WidgetId::new(711),
            Self::Inspector(InspectorPanelKind::Memory) => WidgetId::new(712),
            Self::Bottom(BottomPanelKind::Timeline) => WidgetId::new(713),
            Self::Bottom(BottomPanelKind::Log) => WidgetId::new(714),
            Self::Bottom(BottomPanelKind::Diagnostics) => WidgetId::new(715),
        }
    }
}

/// Retained floating dock-panel state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingDockPanel {
    /// Which retained panel is shown in the floating surface.
    pub kind: FloatingDockKind,
    /// Screen-space rectangle of the floating panel.
    pub rect: Rect,
    /// Whether the floating panel is currently visible.
    pub open: bool,
}

impl FloatingDockPanel {
    /// Create a floating dock panel.
    pub const fn new(kind: FloatingDockKind, rect: Rect) -> Self {
        Self {
            kind,
            rect,
            open: true,
        }
    }
}

/// Unified retained dock panel identity used by the Dock Tree model.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DockPanelKind {
    /// Object/property inspector panel.
    Properties,
    /// Layer/object list panel.
    Layers,
    /// Memory/debug summary panel.
    Memory,
    /// Timeline panel.
    Timeline,
    /// Log panel.
    Log,
    /// Diagnostics panel.
    Diagnostics,
}

impl DockPanelKind {
    /// Stable label used for diagnostics, JSON seeds, and future tab titles.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Properties => "properties",
            Self::Layers => "layers",
            Self::Memory => "memory",
            Self::Timeline => "timeline",
            Self::Log => "log",
            Self::Diagnostics => "diagnostics",
        }
    }

    /// Parse a stable layout label back into a unified panel id.
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim() {
            "properties" | "PROP" => Some(Self::Properties),
            "layers" | "LAYERS" => Some(Self::Layers),
            "memory" | "MEM" => Some(Self::Memory),
            "timeline" | "TIMELINE" => Some(Self::Timeline),
            "log" | "LOG" => Some(Self::Log),
            "diagnostics" | "diag" | "DIAG" => Some(Self::Diagnostics),
            _ => None,
        }
    }

    /// Short UI label matching the current Phase 2 tabs.
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Properties => "PROP",
            Self::Layers => "LAYERS",
            Self::Memory => "MEM",
            Self::Timeline => "TIMELINE",
            Self::Log => "LOG",
            Self::Diagnostics => "DIAG",
        }
    }

    /// Stable widget id for a close affordance inside a fixed dock tab.
    pub const fn tab_close_widget_id(self) -> WidgetId {
        match self {
            Self::Properties => WidgetId::new(760),
            Self::Layers => WidgetId::new(761),
            Self::Memory => WidgetId::new(762),
            Self::Timeline => WidgetId::new(763),
            Self::Log => WidgetId::new(764),
            Self::Diagnostics => WidgetId::new(765),
        }
    }

    /// Default dock area for the retained panel.
    pub const fn default_area(self) -> DockAreaKind {
        match self {
            Self::Properties | Self::Layers | Self::Memory => DockAreaKind::Right,
            Self::Timeline | Self::Log | Self::Diagnostics => DockAreaKind::Bottom,
        }
    }

    /// Convert an existing inspector panel kind into the unified panel id.
    pub const fn from_inspector(panel: InspectorPanelKind) -> Self {
        match panel {
            InspectorPanelKind::Properties => Self::Properties,
            InspectorPanelKind::Layers => Self::Layers,
            InspectorPanelKind::Memory => Self::Memory,
        }
    }

    /// Convert an existing bottom panel kind into the unified panel id.
    pub const fn from_bottom(panel: BottomPanelKind) -> Self {
        match panel {
            BottomPanelKind::Timeline => Self::Timeline,
            BottomPanelKind::Log => Self::Log,
            BottomPanelKind::Diagnostics => Self::Diagnostics,
        }
    }

    /// Convert to the current inspector panel enum when possible.
    pub const fn inspector_panel(self) -> Option<InspectorPanelKind> {
        match self {
            Self::Properties => Some(InspectorPanelKind::Properties),
            Self::Layers => Some(InspectorPanelKind::Layers),
            Self::Memory => Some(InspectorPanelKind::Memory),
            Self::Timeline | Self::Log | Self::Diagnostics => None,
        }
    }

    /// Convert to the current bottom panel enum when possible.
    pub const fn bottom_panel(self) -> Option<BottomPanelKind> {
        match self {
            Self::Timeline => Some(BottomPanelKind::Timeline),
            Self::Log => Some(BottomPanelKind::Log),
            Self::Diagnostics => Some(BottomPanelKind::Diagnostics),
            Self::Properties | Self::Layers | Self::Memory => None,
        }
    }

    /// Convert a floating dock panel kind into the unified panel id.
    pub const fn from_floating(kind: FloatingDockKind) -> Self {
        match kind {
            FloatingDockKind::Inspector(panel) => Self::from_inspector(panel),
            FloatingDockKind::Bottom(panel) => Self::from_bottom(panel),
        }
    }

    /// Convert into the existing floating dock identity.
    pub const fn floating_kind(self) -> FloatingDockKind {
        match self {
            Self::Properties => FloatingDockKind::Inspector(InspectorPanelKind::Properties),
            Self::Layers => FloatingDockKind::Inspector(InspectorPanelKind::Layers),
            Self::Memory => FloatingDockKind::Inspector(InspectorPanelKind::Memory),
            Self::Timeline => FloatingDockKind::Bottom(BottomPanelKind::Timeline),
            Self::Log => FloatingDockKind::Bottom(BottomPanelKind::Log),
            Self::Diagnostics => FloatingDockKind::Bottom(BottomPanelKind::Diagnostics),
        }
    }

    /// Stable panel order used by tab visibility, floating state, and JSON seeds.
    pub const fn all() -> [Self; 6] {
        [
            Self::Properties,
            Self::Layers,
            Self::Memory,
            Self::Timeline,
            Self::Log,
            Self::Diagnostics,
        ]
    }
}

/// Dock area used by the retained Dock Tree foundation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DockAreaKind {
    /// Central canvas/editor well.
    Center,
    /// Right-side inspector area.
    Right,
    /// Bottom timeline/log area.
    Bottom,
    /// Detached pseudo-floating area.
    Floating,
}

impl DockAreaKind {
    /// Stable name used by layout persistence seeds.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Floating => "floating",
        }
    }
}

/// Stable Dock Tree node id.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DockNodeId(u64);

impl DockNodeId {
    /// Create a dock node id from a stable integer.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Raw numeric id for diagnostics and future persistence.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Split direction for future resizable dock nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DockSplitAxis {
    /// Left/right split.
    Horizontal,
    /// Top/bottom split.
    Vertical,
}

impl DockSplitAxis {
    /// Stable name used by layout persistence seeds.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// Tab entry inside a Dock Tree tab group.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DockTabState {
    /// Panel represented by this tab.
    pub panel: DockPanelKind,
    /// Whether the tab is logically present in the dock group.
    pub open: bool,
}

impl DockTabState {
    /// Create an open dock tab.
    pub const fn new(panel: DockPanelKind) -> Self {
        Self { panel, open: true }
    }
}

/// Retained Dock Tree node.
#[derive(Clone, Debug, PartialEq)]
pub enum DockNode {
    /// Split node with two child nodes.
    Split {
        /// Stable node id.
        id: DockNodeId,
        /// Split direction.
        axis: DockSplitAxis,
        /// Fraction assigned to the first child.
        ratio: f32,
        /// First child.
        first: Box<DockNode>,
        /// Second child.
        second: Box<DockNode>,
    },
    /// Tab group node.
    Tabs {
        /// Stable node id.
        id: DockNodeId,
        /// Area represented by this tab group.
        area: DockAreaKind,
        /// Active panel in this group.
        active: Option<DockPanelKind>,
        /// Ordered tabs.
        tabs: Vec<DockTabState>,
    },
    /// Placeholder node for canvas/central content before full docking lands.
    Empty {
        /// Stable node id.
        id: DockNodeId,
        /// Area represented by the placeholder.
        area: DockAreaKind,
    },
}

impl DockNode {
    /// Create a split node.
    pub fn split(
        id: DockNodeId,
        axis: DockSplitAxis,
        ratio: f32,
        first: DockNode,
        second: DockNode,
    ) -> Self {
        Self::Split {
            id,
            axis,
            ratio: clamp_dock_split_ratio(ratio),
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Create a tab group node.
    pub fn tabs(
        id: DockNodeId,
        area: DockAreaKind,
        tabs: Vec<DockTabState>,
        active: Option<DockPanelKind>,
    ) -> Self {
        Self::Tabs {
            id,
            area,
            active,
            tabs,
        }
    }

    /// Create an empty placeholder node.
    pub const fn empty(id: DockNodeId, area: DockAreaKind) -> Self {
        Self::Empty { id, area }
    }

    /// Return this node id.
    pub fn id(&self) -> DockNodeId {
        match self {
            Self::Split { id, .. } | Self::Tabs { id, .. } | Self::Empty { id, .. } => *id,
        }
    }

    /// Find a tab group by area.
    pub fn find_tabs(&self, target: DockAreaKind) -> Option<&[DockTabState]> {
        match self {
            Self::Tabs { area, tabs, .. } if *area == target => Some(tabs),
            Self::Split { first, second, .. } => {
                first.find_tabs(target).or_else(|| second.find_tabs(target))
            }
            _ => None,
        }
    }

    /// Return active panel in a tab group by area.
    pub fn active_panel(&self, target: DockAreaKind) -> Option<DockPanelKind> {
        match self {
            Self::Tabs { area, active, .. } if *area == target => *active,
            Self::Split { first, second, .. } => first
                .active_panel(target)
                .or_else(|| second.active_panel(target)),
            _ => None,
        }
    }

    fn find_tabs_mut(
        &mut self,
        target: DockAreaKind,
    ) -> Option<(&mut Option<DockPanelKind>, &mut Vec<DockTabState>)> {
        match self {
            Self::Tabs {
                area, active, tabs, ..
            } if *area == target => Some((active, tabs)),
            Self::Split { first, second, .. } => {
                if let Some(found) = first.find_tabs_mut(target) {
                    Some(found)
                } else {
                    second.find_tabs_mut(target)
                }
            }
            _ => None,
        }
    }

    fn set_active_panel(&mut self, panel: DockPanelKind) -> bool {
        let area = panel.default_area();
        let Some((active, tabs)) = self.find_tabs_mut(area) else {
            return false;
        };
        if let Some(tab) = tabs.iter_mut().find(|tab| tab.panel == panel) {
            tab.open = true;
        } else {
            tabs.push(DockTabState::new(panel));
        }
        *active = Some(panel);
        true
    }

    fn set_tab_open(&mut self, panel: DockPanelKind, open: bool) -> bool {
        match self {
            Self::Tabs { active, tabs, .. } => {
                let Some(index) = tabs.iter().position(|tab| tab.panel == panel) else {
                    return false;
                };
                tabs[index].open = open;
                if open {
                    if active.is_none() {
                        *active = Some(panel);
                    }
                } else if *active == Some(panel) {
                    *active = tabs.iter().find(|tab| tab.open).map(|tab| tab.panel);
                }
                true
            }
            Self::Split { first, second, .. } => {
                first.set_tab_open(panel, open) || second.set_tab_open(panel, open)
            }
            Self::Empty { .. } => false,
        }
    }

    fn is_tab_open(&self, panel: DockPanelKind) -> Option<bool> {
        match self {
            Self::Tabs { tabs, .. } => tabs
                .iter()
                .find(|tab| tab.panel == panel)
                .map(|tab| tab.open),
            Self::Split { first, second, .. } => first
                .is_tab_open(panel)
                .or_else(|| second.is_tab_open(panel)),
            Self::Empty { .. } => None,
        }
    }

    fn restore_all_tabs(&mut self) {
        match self {
            Self::Tabs { active, tabs, .. } => {
                for tab in tabs.iter_mut() {
                    tab.open = true;
                }
                if active.is_none() {
                    *active = tabs.first().map(|tab| tab.panel);
                }
            }
            Self::Split { first, second, .. } => {
                first.restore_all_tabs();
                second.restore_all_tabs();
            }
            Self::Empty { .. } => {}
        }
    }

    fn split_ratio(&self, target: DockNodeId) -> Option<f32> {
        match self {
            Self::Split { id, ratio, .. } if *id == target => Some(*ratio),
            Self::Split { first, second, .. } => first
                .split_ratio(target)
                .or_else(|| second.split_ratio(target)),
            _ => None,
        }
    }

    fn set_split_ratio(&mut self, target: DockNodeId, ratio: f32) -> bool {
        match self {
            Self::Split {
                id, ratio: stored, ..
            } if *id == target => {
                *stored = clamp_dock_split_ratio(ratio);
                true
            }
            Self::Split { first, second, .. } => {
                first.set_split_ratio(target, ratio) || second.set_split_ratio(target, ratio)
            }
            _ => false,
        }
    }

    fn allocate(&self, rect: Rect, out: &mut Vec<DockNodeGeometry>) {
        match self {
            Self::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                out.push(DockNodeGeometry {
                    id: *id,
                    area: DockAreaKind::Center,
                    rect,
                    kind: DockNodeGeometryKind::Split {
                        axis: *axis,
                        ratio: *ratio,
                    },
                });
                let ratio = clamp_dock_split_ratio(*ratio);
                match axis {
                    DockSplitAxis::Horizontal => {
                        let first_w = (rect.width * ratio).round().clamp(0.0, rect.width);
                        let first_rect = Rect::new(rect.x, rect.y, first_w, rect.height);
                        let second_rect = Rect::new(
                            rect.x + first_w,
                            rect.y,
                            (rect.width - first_w).max(0.0),
                            rect.height,
                        );
                        first.allocate(first_rect, out);
                        second.allocate(second_rect, out);
                    }
                    DockSplitAxis::Vertical => {
                        let first_h = (rect.height * ratio).round().clamp(0.0, rect.height);
                        let first_rect = Rect::new(rect.x, rect.y, rect.width, first_h);
                        let second_rect = Rect::new(
                            rect.x,
                            rect.y + first_h,
                            rect.width,
                            (rect.height - first_h).max(0.0),
                        );
                        first.allocate(first_rect, out);
                        second.allocate(second_rect, out);
                    }
                }
            }
            Self::Tabs {
                id,
                area,
                active,
                tabs,
            } => {
                out.push(DockNodeGeometry {
                    id: *id,
                    area: *area,
                    rect,
                    kind: DockNodeGeometryKind::Tabs {
                        active: *active,
                        visible_tabs: tabs
                            .iter()
                            .filter(|tab| tab.open)
                            .map(|tab| tab.panel)
                            .collect(),
                    },
                });
            }
            Self::Empty { id, area } => {
                out.push(DockNodeGeometry {
                    id: *id,
                    area: *area,
                    rect,
                    kind: DockNodeGeometryKind::Empty,
                });
            }
        }
    }
}

/// Floating dock entry in the retained Dock Layout model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockFloatingPanelState {
    /// Unified panel id.
    pub panel: DockPanelKind,
    /// Screen-space rectangle.
    pub rect: Rect,
    /// Whether the pseudo-floating panel is open.
    pub open: bool,
}

impl DockFloatingPanelState {
    /// Create a floating dock entry.
    pub const fn new(panel: DockPanelKind, rect: Rect, open: bool) -> Self {
        Self { panel, rect, open }
    }
}

/// Resolved geometry for one node in the retained Dock Tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DockNodeGeometry {
    /// Stable node id.
    pub id: DockNodeId,
    /// Dock area represented by the node.
    pub area: DockAreaKind,
    /// Allocated screen rectangle.
    pub rect: Rect,
    /// Node kind plus active/tab metadata.
    pub kind: DockNodeGeometryKind,
}

/// Kind-specific geometry metadata.
#[derive(Clone, Debug, PartialEq)]
pub enum DockNodeGeometryKind {
    /// Split node allocation.
    Split { axis: DockSplitAxis, ratio: f32 },
    /// Tab group allocation.
    Tabs {
        /// Active panel in this group.
        active: Option<DockPanelKind>,
        /// Visible panels in tab order.
        visible_tabs: Vec<DockPanelKind>,
    },
    /// Empty center/editor allocation.
    Empty,
}

/// Full retained Dock Tree allocation result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockLayoutGeometry {
    nodes: Vec<DockNodeGeometry>,
}

impl DockLayoutGeometry {
    /// Borrow allocated nodes in tree order.
    pub fn nodes(&self) -> &[DockNodeGeometry] {
        &self.nodes
    }

    /// Find the first rectangle for a dock area.
    pub fn rect_for_area(&self, area: DockAreaKind) -> Option<Rect> {
        self.nodes
            .iter()
            .find(|node| {
                node.area == area && !matches!(node.kind, DockNodeGeometryKind::Split { .. })
            })
            .map(|node| node.rect)
    }
}

/// Versioned layout profile text used by file-backed persistence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DockLayoutProfile {
    /// Profile schema version.
    pub version: u32,
    /// Deterministic Dock Layout JSON seed.
    pub seed_json: String,
}

impl DockLayoutProfile {
    /// Current profile format version.
    pub const VERSION: u32 = 1;
    const HEADER: &'static str = "PLITA_DOCK_LAYOUT_PROFILE";

    /// Create a profile from a layout state.
    pub fn from_layout(layout: &DockLayoutState) -> Self {
        Self {
            version: Self::VERSION,
            seed_json: layout.to_json_pretty(),
        }
    }

    /// Parse a profile text payload.
    pub fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let header = lines.next()?.trim();
        if header != Self::HEADER {
            return None;
        }
        let version_line = lines.next()?.trim();
        let version = version_line.strip_prefix("version=")?.parse::<u32>().ok()?;
        if version != Self::VERSION {
            return None;
        }
        let seed_json = lines.collect::<Vec<_>>().join("\n");
        if !seed_json.contains("\"version\"") {
            return None;
        }
        Some(Self { version, seed_json })
    }

    /// Serialize to a compact text payload.
    pub fn to_text(&self) -> String {
        format!(
            "{}\nversion={}\n{}",
            Self::HEADER,
            self.version,
            self.seed_json
        )
    }

    /// Convert this profile into a normalized layout state for a surface size.
    pub fn into_layout(self, size: Size) -> Option<DockLayoutState> {
        DockLayoutState::from_json_seed(&self.seed_json, size)
    }
}

/// Retained layout state that will back full Dock Tree docking and persistence.
#[derive(Clone, Debug, PartialEq)]
pub struct DockLayoutState {
    /// Layout schema version. v1 is the first Dock Tree foundation format.
    pub version: u32,
    /// Root Dock Tree node.
    pub root: DockNode,
    /// Detached/floating dock panel states.
    pub floating_panels: Vec<DockFloatingPanelState>,
}

impl DockLayoutState {
    /// Create the default Phase 2 v3 layout state for a surface size.
    pub fn default_for_size(size: Size) -> Self {
        let right_tabs = DockNode::tabs(
            DockNodeId::new(3),
            DockAreaKind::Right,
            vec![
                DockTabState::new(DockPanelKind::Properties),
                DockTabState::new(DockPanelKind::Layers),
                DockTabState::new(DockPanelKind::Memory),
            ],
            Some(DockPanelKind::Properties),
        );
        let bottom_tabs = DockNode::tabs(
            DockNodeId::new(4),
            DockAreaKind::Bottom,
            vec![
                DockTabState::new(DockPanelKind::Timeline),
                DockTabState::new(DockPanelKind::Log),
                DockTabState::new(DockPanelKind::Diagnostics),
            ],
            Some(DockPanelKind::Timeline),
        );
        let center = DockNode::empty(DockNodeId::new(5), DockAreaKind::Center);
        let main = DockNode::split(
            DockNodeId::new(2),
            DockSplitAxis::Vertical,
            0.78,
            center,
            bottom_tabs,
        );
        let root = DockNode::split(
            DockNodeId::new(1),
            DockSplitAxis::Horizontal,
            0.78,
            main,
            right_tabs,
        );
        let floating_panels = DockPanelKind::all()
            .iter()
            .map(|panel| {
                DockFloatingPanelState::new(
                    *panel,
                    Self::default_floating_rect(size, *panel),
                    false,
                )
            })
            .collect();

        Self {
            version: 1,
            root,
            floating_panels,
        }
    }

    /// Default floating rectangle for a panel. Kept in the state model so future
    /// persistence does not depend on `CreativeShell` internals.
    pub fn default_floating_rect(size: Size, panel: DockPanelKind) -> Rect {
        let offset = match panel {
            DockPanelKind::Properties => 0.0,
            DockPanelKind::Layers => 28.0,
            DockPanelKind::Memory => 56.0,
            DockPanelKind::Timeline => 84.0,
            DockPanelKind::Log => 112.0,
            DockPanelKind::Diagnostics => 140.0,
        };
        Self::clamp_floating_rect_for_size(
            size,
            Rect::new(
                (size.width - 430.0 - offset).max(24.0),
                120.0 + offset * 0.35,
                340.0,
                214.0,
            ),
        )
    }

    /// Clamp a pseudo-floating dock rectangle so its title bar remains reachable
    /// on the current surface.
    pub fn clamp_floating_rect_for_size(size: Size, rect: Rect) -> Rect {
        let size = size.clamped();
        let margin = 12.0_f32;
        let min_w = 80.0_f32;
        let min_h = 60.0_f32;
        let max_w = (size.width - margin * 2.0).max(min_w);
        let max_h = (size.height - margin * 2.0).max(min_h);
        let raw_w = if rect.width.is_finite() {
            rect.width
        } else {
            340.0
        };
        let raw_h = if rect.height.is_finite() {
            rect.height
        } else {
            214.0
        };
        let width = raw_w.clamp(min_w, max_w);
        let height = raw_h.clamp(min_h, max_h);
        let max_x = (size.width - width - margin).max(margin);
        let max_y = (size.height - 28.0 - margin).max(margin);
        let raw_x = if rect.x.is_finite() { rect.x } else { margin };
        let raw_y = if rect.y.is_finite() { rect.y } else { margin };
        let x = raw_x.clamp(margin, max_x);
        let y = raw_y.clamp(margin, max_y);
        Rect::new(x, y, width, height)
    }

    /// Normalize retained layout state after loading, resizing, or accepting
    /// external seed data.
    pub fn normalize_for_size(&mut self, size: Size) {
        let all_panels = DockPanelKind::all();

        let mut normalized = Vec::with_capacity(all_panels.len());
        for panel in all_panels {
            let fallback = Self::default_floating_rect(size, panel);
            let existing = self
                .floating_panels
                .iter()
                .find(|item| item.panel == panel)
                .copied();
            let rect = existing.map(|item| item.rect).unwrap_or(fallback);
            let open = existing.map(|item| item.open).unwrap_or(false);
            normalized.push(DockFloatingPanelState::new(
                panel,
                Self::clamp_floating_rect_for_size(size, rect),
                open,
            ));
        }
        self.floating_panels = normalized;

        let inspector_active = DockPanelKind::from_inspector(self.active_inspector_panel());
        if !self
            .panels_for_area(DockAreaKind::Right)
            .contains(&inspector_active)
        {
            self.set_active_inspector_panel(InspectorPanelKind::Properties);
        }
        let bottom_active = DockPanelKind::from_bottom(self.active_bottom_panel());
        if !self
            .panels_for_area(DockAreaKind::Bottom)
            .contains(&bottom_active)
        {
            self.set_active_bottom_panel(BottomPanelKind::Timeline);
        }
    }

    /// Allocate screen rectangles directly from the retained Dock Tree.
    pub fn allocate_geometry(&self, rect: Rect) -> DockLayoutGeometry {
        let mut geometry = DockLayoutGeometry::default();
        self.root.allocate(rect, &mut geometry.nodes);
        geometry
    }

    /// Ratio of the root horizontal split. The first child is the main editor area;
    /// the second child is the right inspector tab group.
    pub fn root_split_ratio(&self) -> f32 {
        self.root.split_ratio(DockNodeId::new(1)).unwrap_or(0.78)
    }

    /// Ratio of the main vertical split. The first child is the canvas/center
    /// placeholder; the second child is the bottom dock tab group.
    pub fn main_split_ratio(&self) -> f32 {
        self.root.split_ratio(DockNodeId::new(2)).unwrap_or(0.78)
    }

    /// Update the retained root split ratio. Since v23, this ratio is synced
    /// from the visible right-side splitter and can be applied back to the
    /// legacy inspector width when a layout seed is loaded.
    pub fn set_root_split_ratio(&mut self, ratio: f32) {
        let _ = self.root.set_split_ratio(DockNodeId::new(1), ratio);
    }

    /// Update the retained main split ratio without applying it to legacy layout
    /// geometry yet. This keeps v21 behavior stable while preparing full split
    /// layout persistence.
    pub fn set_main_split_ratio(&mut self, ratio: f32) {
        let _ = self.root.set_split_ratio(DockNodeId::new(2), ratio);
    }

    /// Compact one-line summary of retained Dock Tree split ratios.
    pub fn split_summary_line(&self) -> String {
        format!(
            "SPLIT ROOT {}%  MAIN {}%",
            (self.root_split_ratio() * 100.0).round() as i32,
            (self.main_split_ratio() * 100.0).round() as i32
        )
    }

    /// Return whether a retained dock tab is currently visible in its tab group.
    pub fn is_panel_tab_open(&self, panel: DockPanelKind) -> bool {
        self.root.is_tab_open(panel).unwrap_or(true)
    }

    /// Show or hide a retained dock tab. At least one tab per fixed dock area is kept open.
    pub fn set_panel_tab_open(&mut self, panel: DockPanelKind, open: bool) -> bool {
        if !open {
            let area = panel.default_area();
            if matches!(area, DockAreaKind::Right | DockAreaKind::Bottom)
                && self.panels_for_area(area).len() <= 1
                && self.is_panel_tab_open(panel)
            {
                return false;
            }
        }
        self.root.set_tab_open(panel, open)
    }

    /// Restore all retained dock tabs to visible state.
    pub fn restore_all_tabs(&mut self) {
        self.root.restore_all_tabs();
    }

    /// Short labels for currently hidden fixed dock tabs.
    pub fn hidden_panel_labels(&self) -> Vec<&'static str> {
        DockPanelKind::all()
            .iter()
            .copied()
            .filter(|panel| !self.is_panel_tab_open(*panel))
            .map(DockPanelKind::tab_label)
            .collect()
    }

    /// Compact one-line tab visibility summary for diagnostics.
    pub fn tab_summary_line(&self) -> String {
        let hidden = self.hidden_panel_labels();
        if hidden.is_empty() {
            "TABS HIDDEN NONE".to_string()
        } else {
            format!("TABS HIDDEN {}", hidden.join(","))
        }
    }

    /// Active right-side inspector panel.
    pub fn active_inspector_panel(&self) -> InspectorPanelKind {
        self.root
            .active_panel(DockAreaKind::Right)
            .and_then(DockPanelKind::inspector_panel)
            .unwrap_or(InspectorPanelKind::Properties)
    }

    /// Active bottom dock panel.
    pub fn active_bottom_panel(&self) -> BottomPanelKind {
        self.root
            .active_panel(DockAreaKind::Bottom)
            .and_then(DockPanelKind::bottom_panel)
            .unwrap_or(BottomPanelKind::Timeline)
    }

    /// Ordered open panel ids for a dock area, as defined by the Dock Tree root.
    pub fn panels_for_area(&self, area: DockAreaKind) -> Vec<DockPanelKind> {
        self.root
            .find_tabs(area)
            .map(|tabs| {
                tabs.iter()
                    .filter(|tab| tab.open)
                    .map(|tab| tab.panel)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Ordered inspector tabs derived from the retained Dock Tree.
    pub fn inspector_tabs(&self) -> Vec<InspectorPanelKind> {
        let tabs: Vec<InspectorPanelKind> = self
            .panels_for_area(DockAreaKind::Right)
            .into_iter()
            .filter_map(|panel| panel.inspector_panel())
            .collect();
        if tabs.is_empty() {
            vec![
                InspectorPanelKind::Properties,
                InspectorPanelKind::Layers,
                InspectorPanelKind::Memory,
            ]
        } else {
            tabs
        }
    }

    /// Ordered bottom dock tabs derived from the retained Dock Tree.
    pub fn bottom_tabs(&self) -> Vec<BottomPanelKind> {
        let tabs: Vec<BottomPanelKind> = self
            .panels_for_area(DockAreaKind::Bottom)
            .into_iter()
            .filter_map(|panel| panel.bottom_panel())
            .collect();
        if tabs.is_empty() {
            vec![
                BottomPanelKind::Timeline,
                BottomPanelKind::Log,
                BottomPanelKind::Diagnostics,
            ]
        } else {
            tabs
        }
    }

    /// Set the active right-side inspector panel.
    pub fn set_active_inspector_panel(&mut self, panel: InspectorPanelKind) {
        let _ = self
            .root
            .set_active_panel(DockPanelKind::from_inspector(panel));
    }

    /// Set the active bottom dock panel.
    pub fn set_active_bottom_panel(&mut self, panel: BottomPanelKind) {
        let _ = self
            .root
            .set_active_panel(DockPanelKind::from_bottom(panel));
    }

    /// Borrow floating panel entries.
    pub fn floating_panels(&self) -> &[DockFloatingPanelState] {
        &self.floating_panels
    }

    /// Count open floating panels.
    pub fn floating_dock_count(&self) -> usize {
        self.floating_panels
            .iter()
            .filter(|panel| panel.open)
            .count()
    }

    /// Return short tab labels for currently open floating panels.
    pub fn open_floating_panel_labels(&self) -> Vec<&'static str> {
        self.floating_panels
            .iter()
            .filter(|panel| panel.open)
            .map(|panel| panel.panel.tab_label())
            .collect()
    }

    /// Compact one-line retained layout summary for diagnostics/status UI.
    pub fn summary_line(&self, saved_seed: bool) -> String {
        let right = DockPanelKind::from_inspector(self.active_inspector_panel()).tab_label();
        let bottom = DockPanelKind::from_bottom(self.active_bottom_panel()).tab_label();
        let floating = self.open_floating_panel_labels();
        let floating_text = if floating.is_empty() {
            "NONE".to_string()
        } else {
            floating.join(",")
        };
        format!(
            "RIGHT {}  BOTTOM {}  FLOAT {} [{}]  SAVED {}",
            right,
            bottom,
            self.floating_dock_count(),
            floating_text,
            if saved_seed { "YES" } else { "NO" }
        )
    }

    /// Return a floating panel entry by unified panel id.
    pub fn floating_panel(&self, panel: DockPanelKind) -> Option<DockFloatingPanelState> {
        self.floating_panels
            .iter()
            .find(|item| item.panel == panel)
            .copied()
    }

    /// Update or create a floating panel entry.
    pub fn set_floating_panel(&mut self, panel: DockPanelKind, rect: Rect, open: bool) {
        if let Some(item) = self
            .floating_panels
            .iter_mut()
            .find(|item| item.panel == panel)
        {
            item.rect = rect;
            item.open = open;
        } else {
            self.floating_panels
                .push(DockFloatingPanelState::new(panel, rect, open));
        }
    }

    /// Mark a floating panel as open.
    ///
    /// Opening a floating panel must not change the active fixed dock tab.
    /// Ctrl+Shift+1..6 is the floating-dock shortcut family, while Ctrl+1..6
    /// remains the fixed dock tab shortcut family.  Keeping those side effects
    /// separate prevents a floating shortcut from also moving INSPECTOR DOCK or
    /// BOTTOM DOCK selection.
    pub fn open_floating_panel(&mut self, panel: DockPanelKind, rect: Rect) {
        self.set_floating_panel(panel, rect, true);
    }

    /// Detach a fixed dock tab into a floating panel.
    pub fn detach_panel(&mut self, panel: DockPanelKind, size: Size) -> bool {
        let was_open = self.is_panel_tab_open(panel);
        if !was_open {
            let rect = self
                .floating_panel(panel)
                .map(|state| state.rect)
                .unwrap_or_else(|| Self::default_floating_rect(size, panel));
            self.open_floating_panel(panel, Self::clamp_floating_rect_for_size(size, rect));
            return true;
        }

        if !self.set_panel_tab_open(panel, false) {
            return false;
        }
        let rect = self
            .floating_panel(panel)
            .map(|state| state.rect)
            .unwrap_or_else(|| Self::default_floating_rect(size, panel));
        self.open_floating_panel(panel, Self::clamp_floating_rect_for_size(size, rect));
        self.normalize_for_size(size);
        true
    }

    /// Attach a floating panel back to its default fixed dock area.
    pub fn attach_panel(&mut self, panel: DockPanelKind) -> bool {
        let changed = self.set_panel_tab_open(panel, true);
        self.close_floating_panel(panel);
        match panel.default_area() {
            DockAreaKind::Right => {
                if let Some(panel) = panel.inspector_panel() {
                    self.set_active_inspector_panel(panel);
                }
            }
            DockAreaKind::Bottom => {
                if let Some(panel) = panel.bottom_panel() {
                    self.set_active_bottom_panel(panel);
                }
            }
            DockAreaKind::Center | DockAreaKind::Floating => {}
        }
        changed
    }

    /// Attach every currently floating panel back to its fixed dock area.
    pub fn attach_all_floating_panels(&mut self) -> usize {
        let floating: Vec<DockPanelKind> = self
            .floating_panels
            .iter()
            .filter(|panel| panel.open)
            .map(|panel| panel.panel)
            .collect();
        let count = floating.len();
        for panel in floating {
            self.attach_panel(panel);
        }
        count
    }

    /// Mark a floating panel as closed.
    pub fn close_floating_panel(&mut self, panel: DockPanelKind) {
        if let Some(item) = self
            .floating_panels
            .iter_mut()
            .find(|item| item.panel == panel)
        {
            item.open = false;
        }
    }

    /// Mark all floating panels as closed.
    pub fn close_all_floating_panels(&mut self) {
        for panel in &mut self.floating_panels {
            panel.open = false;
        }
    }

    /// Rebuild floating entries from the legacy Phase 2 floating dock vector.
    pub fn sync_floating_from_legacy(&mut self, panels: &[FloatingDockPanel], size: Size) {
        for panel in DockPanelKind::all() {
            let fallback = Self::default_floating_rect(size, panel);
            let current = self.floating_panel(panel);
            let legacy = panels
                .iter()
                .find(|item| DockPanelKind::from_floating(item.kind) == panel);
            let rect = legacy
                .map(|item| item.rect)
                .or_else(|| current.map(|item| item.rect))
                .unwrap_or(fallback);
            let open = legacy
                .map(|item| item.open)
                .or_else(|| current.map(|item| item.open))
                .unwrap_or(false);
            self.set_floating_panel(panel, Self::clamp_floating_rect_for_size(size, rect), open);
        }
        self.normalize_for_size(size);
    }

    /// Import the deterministic Phase 2 v3 layout JSON seed.
    ///
    /// This is intentionally a tiny hand-rolled reader for the JSON emitted by
    /// `to_json_pretty()`. It keeps v15 dependency-free while giving menu-level
    /// save/load a real state round-trip. A serde-backed project layout format
    /// can replace this once the Dock Tree format settles.
    pub fn from_json_seed(text: &str, size: Size) -> Option<Self> {
        if !text.contains("\"version\"") || !text.contains("\"floating\"") {
            return None;
        }
        let mut layout = Self::default_for_size(size);
        for panel in DockPanelKind::all() {
            if let Some(segment) = json_object_with_tab(text, panel) {
                let open = json_bool_value(segment, "open").unwrap_or(true);
                let _ = layout.set_panel_tab_open(panel, open);
            }
        }
        if let Some(active_right) = json_string_value(text, "active_right")
            .and_then(|value| DockPanelKind::from_label(&value))
        {
            if let Some(panel) = active_right.inspector_panel() {
                layout.set_active_inspector_panel(panel);
            }
        }
        if let Some(active_bottom) = json_string_value(text, "active_bottom")
            .and_then(|value| DockPanelKind::from_label(&value))
        {
            if let Some(panel) = active_bottom.bottom_panel() {
                layout.set_active_bottom_panel(panel);
            }
        }
        if let Some(root_ratio) = json_f32_value(text, "root_ratio") {
            layout.set_root_split_ratio(root_ratio);
        }
        if let Some(main_ratio) = json_f32_value(text, "main_ratio") {
            layout.set_main_split_ratio(main_ratio);
        }
        for panel in DockPanelKind::all() {
            if let Some(segment) = json_object_with_panel(text, panel) {
                let fallback = Self::default_floating_rect(size, panel);
                let open = json_bool_value(segment, "open").unwrap_or(false);
                let x = json_f32_value(segment, "x").unwrap_or(fallback.x);
                let y = json_f32_value(segment, "y").unwrap_or(fallback.y);
                let w = json_f32_value(segment, "w")
                    .unwrap_or(fallback.width)
                    .max(80.0);
                let h = json_f32_value(segment, "h")
                    .unwrap_or(fallback.height)
                    .max(60.0);
                layout.set_floating_panel(panel, Rect::new(x, y, w, h), open);
            }
        }
        layout.normalize_for_size(size);
        Some(layout)
    }

    /// Export a small deterministic JSON seed for future layout persistence.
    /// This intentionally avoids adding serde before the format settles.
    pub fn to_json_pretty(&self) -> String {
        let active_right = DockPanelKind::from_inspector(self.active_inspector_panel());
        let active_bottom = DockPanelKind::from_bottom(self.active_bottom_panel());
        let mut text = String::new();
        text.push_str("{\n");
        text.push_str(&format!("  \"version\": {},\n", self.version));
        text.push_str(
            "  \"root\": { \"kind\": \"split\", \"axis\": \"horizontal\", \"id\": 1 },\n",
        );
        text.push_str(&format!(
            "  \"root_ratio\": {:.3},\n",
            self.root_split_ratio()
        ));
        text.push_str(&format!(
            "  \"main_ratio\": {:.3},\n",
            self.main_split_ratio()
        ));
        text.push_str(&format!(
            "  \"active_right\": \"{}\",\n",
            active_right.label()
        ));
        text.push_str(&format!(
            "  \"active_bottom\": \"{}\",\n",
            active_bottom.label()
        ));
        text.push_str("  \"dock_tabs\": [\n");
        let all_tabs = DockPanelKind::all();
        for (index, panel) in all_tabs.iter().enumerate() {
            let comma = if index + 1 == all_tabs.len() { "" } else { "," };
            text.push_str(&format!(
                "    {{ \"tab\": \"{}\", \"open\": {} }}{}\n",
                panel.label(),
                self.is_panel_tab_open(*panel),
                comma
            ));
        }
        text.push_str("  ],\n");
        text.push_str("  \"floating\": [\n");
        for (index, panel) in self.floating_panels.iter().enumerate() {
            let comma = if index + 1 == self.floating_panels.len() {
                ""
            } else {
                ","
            };
            text.push_str(&format!(
                "    {{ \"panel\": \"{}\", \"open\": {}, \"x\": {}, \"y\": {}, \"w\": {}, \"h\": {} }}{}\n",
                panel.panel.label(),
                panel.open,
                panel.rect.x.round() as i32,
                panel.rect.y.round() as i32,
                panel.rect.width.round() as i32,
                panel.rect.height.round() as i32,
                comma
            ));
        }
        text.push_str("  ]\n");
        text.push_str("}\n");
        text
    }

    /// Export a versioned text profile around the deterministic layout seed.
    pub fn to_profile_text(&self) -> String {
        DockLayoutProfile::from_layout(self).to_text()
    }

    /// Import a versioned layout profile.
    pub fn from_profile_text(text: &str, size: Size) -> Option<Self> {
        DockLayoutProfile::from_text(text)?.into_layout(size)
    }
}

fn json_string_value(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\"", key);
    let start = text.find(&marker)?;
    let after_key = &text[start + marker.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let after_quote = after_colon.strip_prefix('\"')?;
    let end = after_quote.find('\"')?;
    Some(after_quote[..end].to_string())
}

fn json_bool_value(text: &str, key: &str) -> Option<bool> {
    let marker = format!("\"{}\"", key);
    let start = text.find(&marker)?;
    let after_key = &text[start + marker.len()..];
    let colon = after_key.find(':')?;
    let value = after_key[colon + 1..].trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_f32_value(text: &str, key: &str) -> Option<f32> {
    let marker = format!("\"{}\"", key);
    let start = text.find(&marker)?;
    let after_key = &text[start + marker.len()..];
    let colon = after_key.find(':')?;
    let value = after_key[colon + 1..].trim_start();
    let end = value
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.'))
        .unwrap_or(value.len());
    value[..end].parse::<f32>().ok()
}

fn json_object_with_panel(text: &str, panel: DockPanelKind) -> Option<&str> {
    let marker = format!("\"panel\": \"{}\"", panel.label());
    let panel_pos = text.find(&marker)?;
    let start = text[..panel_pos].rfind('{')?;
    let end = text[panel_pos..].find('}')? + panel_pos;
    Some(&text[start..=end])
}

fn json_object_with_tab(text: &str, panel: DockPanelKind) -> Option<&str> {
    let marker = format!("\"tab\": \"{}\"", panel.label());
    let panel_pos = text.find(&marker)?;
    let start = text[..panel_pos].rfind('{')?;
    let end = text[panel_pos..].find('}')? + panel_pos;
    Some(&text[start..=end])
}

impl Default for DockLayoutState {
    fn default() -> Self {
        Self::default_for_size(Size::new(1280.0, 800.0))
    }
}

/// Widget role used by the retained tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetRole {
    /// Root window content.
    Root,
    /// Top menu bar.
    MenuBar,
    /// Clickable menu-bar item.
    MenuItem(MenuKind),
    /// Toolbar container.
    ToolBar,
    /// Clickable tool button.
    ToolButton(ToolKind),
    /// Main canvas.
    Canvas,
    /// Inspector side panel.
    Inspector,
    /// Tab inside the inspector dock area.
    InspectorTab(InspectorPanelKind),
    /// Close affordance inside a fixed dock tab.
    DockTabClose(DockPanelKind),
    /// Timeline area.
    Timeline,
    /// Tab inside the bottom dock area.
    BottomTab(BottomPanelKind),
    /// Status bar.
    StatusBar,
    /// Draggable splitter.
    Splitter(SplitterKind),
    /// Rich, skinnable push button.
    RichButton,
    /// Rich, skinnable horizontal slider.
    Slider,
    /// Single-line retained text input.
    TextInput,
    /// Editable debugger memory hex view.
    MemoryEditor,
    /// Floating panel title bar.
    FloatingTitle,
    /// Floating panel close button.
    FloatingClose,
    /// Floating dock panel title bar.
    FloatingDockTitle(FloatingDockKind),
    /// Floating dock panel close button.
    FloatingDockClose(FloatingDockKind),
}

/// Splitter orientation/semantic side.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitterKind {
    /// Splitter between toolbar and canvas.
    Left,
    /// Splitter between canvas and inspector.
    Right,
    /// Splitter between the main editor area and the bottom dock.
    Bottom,
}

/// Visual state for a retained widget node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetVisualState {
    /// Normal resting state.
    Normal,
    /// Pointer is hovering this widget.
    Hovered,
    /// Pointer is pressing/dragging this widget.
    Active,
    /// Widget is logically selected.
    Selected,
}

/// Visual state for a command button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonVisualState {
    /// Normal resting state.
    Normal,
    /// Pointer is hovering the button.
    Hovered,
    /// Pointer is pressing the button.
    Pressed,
    /// Button represents an active/toggled command.
    Active,
}

impl ButtonVisualState {
    /// Stable lowercase label for diagnostics and Python bindings.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Hovered => "hovered",
            Self::Pressed => "pressed",
            Self::Active => "active",
        }
    }
}

/// Result returned when a pressed button is released.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonRelease<ButtonId> {
    /// Button that was pressed before release, if any.
    pub pressed: Option<ButtonId>,
    /// Button under the pointer at release time, if any.
    pub released: Option<ButtonId>,
    /// Button that should be activated because press and release matched.
    pub activated: Option<ButtonId>,
    /// Whether hover or press state changed and a redraw is useful.
    pub changed: bool,
}

impl<ButtonId> ButtonRelease<ButtonId> {
    /// Whether the release produced an activation.
    pub fn did_activate(&self) -> bool {
        self.activated.is_some()
    }
}

/// Reusable hover/press state machine for command buttons.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonInteractionState<ButtonId> {
    hovered: Option<ButtonId>,
    pressed: Option<ButtonId>,
}

impl<ButtonId> Default for ButtonInteractionState<ButtonId> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ButtonId> ButtonInteractionState<ButtonId> {
    /// Create an empty button interaction state.
    pub const fn new() -> Self {
        Self {
            hovered: None,
            pressed: None,
        }
    }

    /// Currently hovered button id.
    pub fn hovered(&self) -> Option<&ButtonId> {
        self.hovered.as_ref()
    }

    /// Currently pressed button id.
    pub fn pressed(&self) -> Option<&ButtonId> {
        self.pressed.as_ref()
    }

    /// Clear hover and press state.
    pub fn clear(&mut self) -> bool {
        let changed = self.hovered.is_some() || self.pressed.is_some();
        self.hovered = None;
        self.pressed = None;
        changed
    }
}

impl<ButtonId> ButtonInteractionState<ButtonId>
where
    ButtonId: Clone + Eq,
{
    /// Set the hovered button.
    pub fn set_hovered(&mut self, hovered: Option<ButtonId>) -> bool {
        let changed = self.hovered != hovered;
        self.hovered = hovered;
        changed
    }

    /// Mark a button as hovered.
    pub fn hover(&mut self, id: ButtonId) -> bool {
        self.set_hovered(Some(id))
    }

    /// Mark a button as pressed and hovered.
    pub fn press(&mut self, id: ButtonId) -> bool {
        let hovered = Some(id.clone());
        let pressed = Some(id);
        let changed = self.hovered != hovered || self.pressed != pressed;
        self.hovered = hovered;
        self.pressed = pressed;
        changed
    }

    /// Release over an optional button and report whether it activates.
    pub fn release_over(&mut self, released: Option<ButtonId>) -> ButtonRelease<ButtonId> {
        let pressed = self.pressed.take();
        let hover_changed = self.hovered != released;
        self.hovered = released.clone();
        let activated = match (&pressed, &released) {
            (Some(pressed), Some(released)) if pressed == released => Some(released.clone()),
            _ => None,
        };
        let changed = pressed.is_some() || hover_changed;
        ButtonRelease {
            pressed,
            released,
            activated,
            changed,
        }
    }

    /// Run a complete click interaction for a button.
    pub fn click(&mut self, id: ButtonId) -> ButtonRelease<ButtonId> {
        self.press(id.clone());
        self.release_over(Some(id))
    }

    /// Return the visual state for a button.
    pub fn visual_state(&self, id: &ButtonId, active: bool) -> ButtonVisualState {
        if self.pressed.as_ref() == Some(id) {
            ButtonVisualState::Pressed
        } else if active {
            ButtonVisualState::Active
        } else if self.hovered.as_ref() == Some(id) {
            ButtonVisualState::Hovered
        } else {
            ButtonVisualState::Normal
        }
    }

    /// Convenience helper for pointer-move hit testing.
    pub fn handle_pointer_move(&mut self, hit: Option<ButtonId>) -> bool {
        self.set_hovered(hit)
    }

    /// Convenience helper for pointer-down hit testing.
    pub fn handle_pointer_down(&mut self, hit: Option<ButtonId>) -> bool {
        match hit {
            Some(id) => self.press(id),
            None => false,
        }
    }

    /// Convenience helper for pointer-up hit testing.
    pub fn handle_pointer_up(&mut self, hit: Option<ButtonId>) -> ButtonRelease<ButtonId> {
        self.release_over(hit)
    }
}

/// Public style tokens shared by custom plita apps and built-in widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeTokens {
    /// App background.
    pub background: Color,
    /// Primary panel surface.
    pub panel: Color,
    /// Secondary panel surface.
    pub panel_alt: Color,
    /// Border color.
    pub border: Color,
    /// Accent color.
    pub accent: Color,
    /// Primary text color.
    pub text: Color,
    /// Muted text color.
    pub text_muted: Color,
    /// Button surface.
    pub button: Color,
    /// Hovered button surface.
    pub button_hover: Color,
    /// Active button surface.
    pub button_active: Color,
}

impl ThemeTokens {
    /// Default dark production-tool palette.
    pub const fn dark() -> Self {
        Self {
            background: Color::rgb8(14, 18, 25),
            panel: Color::rgb8(26, 35, 45),
            panel_alt: Color::rgb8(20, 28, 37),
            border: Color::rgb8(84, 102, 122),
            accent: Color::rgb8(106, 158, 255),
            text: Color::rgb8(246, 249, 255),
            text_muted: Color::rgb8(202, 215, 232),
            button: Color::rgb8(38, 52, 68),
            button_hover: Color::rgb8(54, 72, 94),
            button_active: Color::rgb8(74, 98, 128),
        }
    }

    /// Shared panel style derived from these tokens.
    pub const fn panel_style(self) -> PanelStyle {
        PanelStyle::new(self.panel, self.panel_alt, self.border)
    }
}

/// A lightweight retained widget node.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetNode {
    /// Stable widget id.
    pub id: WidgetId,
    /// Semantic role.
    pub role: WidgetRole,
    /// Assigned rectangle.
    pub rect: Rect,
    /// Debug/user-visible label.
    pub label: &'static str,
    /// Visual state.
    pub state: WidgetVisualState,
}

impl WidgetNode {
    /// Create a node.
    pub const fn new(
        id: WidgetId,
        role: WidgetRole,
        rect: Rect,
        label: &'static str,
        state: WidgetVisualState,
    ) -> Self {
        Self {
            id,
            role,
            rect,
            label,
            state,
        }
    }
}

/// Minimal trait representing retained widgets.
pub trait Widget {
    /// Stable id.
    fn id(&self) -> WidgetId;
    /// Layout rect.
    fn rect(&self) -> Rect;
    /// Semantic role.
    fn role(&self) -> WidgetRole;
}

impl Widget for WidgetNode {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn role(&self) -> WidgetRole {
        self.role
    }
}

/// Retained widget tree in paint order. Later nodes are considered above earlier ones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WidgetTree {
    nodes: Vec<WidgetNode>,
}

impl WidgetTree {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Add a node.
    pub fn push(&mut self, node: WidgetNode) {
        self.nodes.push(node);
    }

    /// Borrow all nodes in paint order.
    pub fn nodes(&self) -> &[WidgetNode] {
        &self.nodes
    }

    /// Hit test from top-most node to bottom-most node.
    pub fn hit_test(&self, point: Point) -> Option<&WidgetNode> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.rect.contains(point))
    }
}

/// Mouse/pointer button used by the UI event router.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    /// Main/left button.
    Primary,
    /// Secondary/right button.
    Secondary,
    /// Middle button.
    Middle,
    /// Other platform-specific button.
    Other(u16),
}

/// Events consumed by the retained UI shell.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    /// Pointer moved to a new position.
    PointerMove { position: Point },
    /// Pointer button pressed.
    PointerDown {
        position: Point,
        button: PointerButton,
    },
    /// Pointer button released.
    PointerUp {
        position: Point,
        button: PointerButton,
    },
    /// Two completed clicks with the same button occurred within the
    /// platform-independent time and position tolerance.
    PointerDoubleClick {
        position: Point,
        button: PointerButton,
    },
    /// Wheel scrolled.
    PointerWheel {
        position: Point,
        delta_x: f32,
        delta_y: f32,
    },
    /// Key pressed, represented as a diagnostic string plus modifier state for Phase 1.
    KeyDown {
        /// Logical key string.
        key: String,
        /// Control key is held.
        ctrl: bool,
        /// Shift key is held.
        shift: bool,
        /// Alt key is held.
        alt: bool,
        /// Platform/super key is held.
        meta: bool,
    },
    /// Key released, represented with the same diagnostic string shape as `KeyDown`.
    KeyUp {
        /// Logical key string.
        key: String,
        /// Control key is held.
        ctrl: bool,
        /// Shift key is held.
        shift: bool,
        /// Alt key is held.
        alt: bool,
        /// Platform/super key is held.
        meta: bool,
    },
    /// Window focus changed.
    FocusChanged { focused: bool },
    /// Text committed by the platform text/IME pipeline.
    TextInput { text: String },
    /// IME composition/preedit text changed.
    ImePreedit { text: String },
    /// IME composition was committed.
    ImeCommit { text: String },
    /// A file was dropped onto the application window.
    FileDropped { path: PathBuf },
}

/// Structured keyboard input consumed by the shortcut resolver.
///
/// Phase 2 v2.1 keeps the public `UiEvent::KeyDown` shape for compatibility,
/// but immediately normalizes the legacy diagnostic string into this type.
/// This keeps ordinary logical shortcuts such as Ctrl+C/Ctrl+V separate from
/// physical-key recovery used by Ctrl+Shift+number on Windows/JIS keyboards.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyInput {
    /// Original diagnostic key string received from the window layer.
    pub raw: String,
    /// Logical key portion before the optional `|physical` suffix.
    pub logical: String,
    /// Optional physical-key/debug suffix appended by the window layer.
    pub physical: Option<String>,
    /// Control key is held.
    pub ctrl: bool,
    /// Shift key is held.
    pub shift: bool,
    /// Alt key is held.
    pub alt: bool,
    /// Platform/super key is held.
    pub meta: bool,
}

impl KeyInput {
    /// Build a normalized key input from the legacy Phase 1/2 key string.
    pub fn from_legacy(key: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> Self {
        let mut parts = key.splitn(2, '|');
        let logical = parts.next().unwrap_or(key).trim().to_string();
        let physical = parts
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        Self {
            raw: key.to_string(),
            logical,
            physical,
            ctrl,
            shift,
            alt,
            meta,
        }
    }

    /// Logical key in uppercase form.  Ordinary commands must use this only.
    pub fn logical_upper(&self) -> String {
        self.logical.to_ascii_uppercase()
    }

    /// Full diagnostic key in uppercase form, including physical metadata.
    pub fn raw_upper(&self) -> String {
        self.raw.to_ascii_uppercase()
    }

    /// Recover a shortcut digit from logical and physical/native diagnostics.
    pub fn shortcut_digit(&self) -> Option<u8> {
        shortcut_digit_from_key(&self.raw_upper())
            .or_else(|| shortcut_digit_from_key(&self.logical_upper()))
    }

    /// Returns true when no command modifier is held.
    pub fn text_editing_mode(&self) -> bool {
        !self.ctrl && !self.alt && !self.meta
    }
}

/// Result from dispatching an event into the UI tree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiResponse {
    /// Whether the caller should rebuild the draw list and request redraw.
    pub needs_redraw: bool,
    /// Whether the status text changed.
    pub status_changed: bool,
}

impl UiResponse {
    /// Request a redraw and report that visible state changed.
    pub fn redraw() -> Self {
        Self {
            needs_redraw: true,
            status_changed: true,
        }
    }

    /// Report that no redraw is needed.
    pub fn none() -> Self {
        Self::default()
    }
}

/// Phase 1 creative-application demo layout rectangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreativeLayout {
    /// Full window area.
    pub root: Rect,
    /// Top menu bar area.
    pub menu_bar: Rect,
    /// Left tool strip.
    pub tool_bar: Rect,
    /// Splitter between toolbar and canvas.
    pub left_splitter: Rect,
    /// Central editor/canvas area.
    pub canvas: Rect,
    /// Splitter between canvas and inspector.
    pub right_splitter: Rect,
    /// Right inspector area.
    pub inspector: Rect,
    /// Bottom status bar.
    pub status_bar: Rect,
    /// Timeline strip above the status bar.
    pub timeline: Rect,
}

impl CreativeLayout {
    /// Compute a deterministic creative-app layout for the given surface size.
    pub fn compute(size: Size) -> Self {
        Self::compute_with_panel_sizes(size, None, None, None)
    }

    /// Compute a creative-app layout using optional retained side-panel widths.
    pub fn compute_with_side_widths(
        size: Size,
        left_width: Option<f32>,
        right_width: Option<f32>,
    ) -> Self {
        Self::compute_with_panel_sizes(size, left_width, right_width, None)
    }

    /// Compute a creative-app layout using optional retained side and bottom dock extents.
    pub fn compute_with_panel_sizes(
        size: Size,
        left_width: Option<f32>,
        right_width: Option<f32>,
        bottom_height: Option<f32>,
    ) -> Self {
        let size = size.clamped();
        let root = Rect::new(0.0, 0.0, size.width, size.height);

        let menu_h = 34.0_f32.min(size.height * 0.25_f32).max(24.0_f32);
        let status_h = 28.0_f32.min(size.height * 0.20_f32).max(20.0_f32);
        let default_timeline_h: f32 = if size.height >= 360.0_f32 {
            108.0_f32
        } else {
            72.0_f32
        };
        let default_left_w = if size.width >= 900.0_f32 {
            76.0_f32
        } else {
            56.0_f32
        };
        let default_right_w = if size.width >= 1000.0_f32 {
            276.0_f32
        } else {
            (size.width * 0.26_f32).clamp(140.0_f32, 240.0_f32)
        };
        let splitter_w = 5.0_f32;

        let mut left_w = clamp_panel_extent(
            left_width.unwrap_or(default_left_w),
            48.0_f32,
            size.width * 0.40_f32,
        );
        let mut right_w = clamp_panel_extent(
            right_width.unwrap_or(default_right_w),
            120.0_f32,
            size.width * 0.50_f32,
        );
        let available_w = (size.width - splitter_w * 2.0_f32).max(1.0_f32);
        let min_canvas_w = available_w.clamp(1.0_f32, 160.0_f32);
        let side_budget = (available_w - min_canvas_w).max(1.0_f32);
        if left_w + right_w > side_budget {
            let ratio = side_budget / (left_w + right_w);
            left_w *= ratio;
            right_w *= ratio;
        }

        let content_y = menu_h;
        let content_h = (size.height - menu_h - status_h).max(1.0_f32);
        let bottom_max = (content_h * 0.55_f32).max(48.0_f32);
        let timeline_h = clamp_panel_extent(
            bottom_height.unwrap_or(default_timeline_h),
            48.0_f32,
            bottom_max,
        );
        let main_h = (content_h - timeline_h).max(1.0_f32);
        let center_w = (size.width - left_w - right_w - splitter_w * 2.0_f32).max(1.0_f32);

        let menu_bar = Rect::new(0.0, 0.0, size.width, menu_h);
        let tool_bar = Rect::new(0.0, content_y, left_w, main_h);
        let left_splitter = Rect::new(tool_bar.right(), content_y, splitter_w, main_h);
        let canvas = Rect::new(left_splitter.right(), content_y, center_w, main_h);
        let right_splitter = Rect::new(canvas.right(), content_y, splitter_w, main_h);
        let inspector = Rect::new(right_splitter.right(), content_y, right_w, main_h);
        let timeline = Rect::new(0.0, content_y + main_h, size.width, timeline_h);
        let status_bar = Rect::new(0.0, size.height - status_h, size.width, status_h);

        Self {
            root,
            menu_bar,
            tool_bar,
            left_splitter,
            canvas,
            right_splitter,
            inspector,
            status_bar,
            timeline,
        }
    }
}

fn dock_root_split_ratio_from_layout(layout: CreativeLayout) -> f32 {
    if layout.root.width <= 0.0 {
        0.78
    } else {
        clamp_dock_split_ratio(layout.inspector.x / layout.root.width)
    }
}

fn dock_main_split_ratio_from_layout(layout: CreativeLayout) -> f32 {
    let combined = layout.canvas.height + layout.timeline.height;
    if combined <= 0.0 {
        0.78
    } else {
        clamp_dock_split_ratio(layout.canvas.height / combined)
    }
}

fn dock_right_width_from_root_split_ratio(size: Size, ratio: f32) -> f32 {
    let size = size.clamped();
    let raw = size.width * (1.0 - clamp_dock_split_ratio(ratio));
    clamp_panel_extent(raw, 120.0_f32, size.width * 0.55_f32)
}

fn dock_bottom_height_from_main_split_ratio(size: Size, ratio: f32) -> f32 {
    let size = size.clamped();
    let menu_h = 34.0_f32.min(size.height * 0.25_f32).max(24.0_f32);
    let status_h = 28.0_f32.min(size.height * 0.20_f32).max(20.0_f32);
    let content_h = (size.height - menu_h - status_h).max(1.0_f32);
    let raw = content_h * (1.0 - clamp_dock_split_ratio(ratio));
    clamp_panel_extent(raw, 48.0_f32, (content_h * 0.55_f32).max(48.0_f32))
}

fn bottom_splitter_rect(layout: CreativeLayout) -> Rect {
    let h = 6.0_f32.min(layout.timeline.height.max(1.0_f32));
    Rect::new(
        layout.root.x,
        layout.timeline.y - h * 0.5_f32,
        layout.root.width,
        h,
    )
}

/// Menu-bar item kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuKind {
    /// File menu.
    File,
    /// Edit menu.
    Edit,
    /// View menu.
    View,
    /// Window menu.
    Window,
    /// Help menu.
    Help,
}

impl MenuKind {
    /// Label shown in the menu bar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::File => "FILE",
            Self::Edit => "EDIT",
            Self::View => "VIEW",
            Self::Window => "WINDOW",
            Self::Help => "HELP",
        }
    }

    /// Stable widget id for this menu item.
    pub const fn widget_id(self) -> WidgetId {
        match self {
            Self::File => WidgetId::new(20),
            Self::Edit => WidgetId::new(21),
            Self::View => WidgetId::new(22),
            Self::Window => WidgetId::new(23),
            Self::Help => WidgetId::new(24),
        }
    }

    fn from_widget_id(id: WidgetId) -> Option<Self> {
        match id.raw() {
            20 => Some(Self::File),
            21 => Some(Self::Edit),
            22 => Some(Self::View),
            23 => Some(Self::Window),
            24 => Some(Self::Help),
            _ => None,
        }
    }
}

/// Toolbar tool kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolKind {
    /// Select/move objects.
    Select,
    /// Pan the canvas.
    Pan,
    /// Create rectangles.
    Box,
    /// Create/edit text.
    Text,
    /// Paint a configurable grid input.
    Grid,
}

impl ToolKind {
    /// Label shown in the toolbar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Select => "SEL",
            Self::Pan => "PAN",
            Self::Box => "BOX",
            Self::Text => "TXT",
            Self::Grid => "GRD",
        }
    }

    /// Stable widget id for this tool button.
    pub const fn widget_id(self) -> WidgetId {
        match self {
            Self::Select => WidgetId::new(100),
            Self::Pan => WidgetId::new(101),
            Self::Box => WidgetId::new(102),
            Self::Text => WidgetId::new(103),
            Self::Grid => WidgetId::new(104),
        }
    }

    fn from_widget_id(id: WidgetId) -> Option<Self> {
        match id.raw() {
            100 => Some(Self::Select),
            101 => Some(Self::Pan),
            102 => Some(Self::Box),
            103 => Some(Self::Text),
            104 => Some(Self::Grid),
            _ => None,
        }
    }
}

/// Resize handle used by the Phase 1 canvas prototype.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResizeHandle {
    /// Top-left handle.
    TopLeft,
    /// Top-right handle.
    TopRight,
    /// Bottom-left handle.
    BottomLeft,
    /// Bottom-right handle.
    BottomRight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShellScrollArea {
    Inspector,
    Bottom,
    FloatingMemory,
    FloatingDock(FloatingDockKind),
}

const INSPECTOR_SCROLL_EXTRA: Point = Point { x: 640.0, y: 900.0 };
const BOTTOM_SCROLL_EXTRA: Point = Point { x: 900.0, y: 720.0 };
const FLOATING_MEMORY_SCROLL_EXTRA: Point = Point { x: 620.0, y: 620.0 };
const FLOATING_DOCK_SCROLL_EXTRA: Point = Point { x: 760.0, y: 760.0 };
/// Stable auxiliary-window id used by the native window layer for CreativeShell memory editing.
pub const CREATIVE_MEMORY_WINDOW_ID: u64 = 0x504c_4954_414d_454d;
const DETACHED_MEMORY_WINDOW_MIN_SIZE: Size = Size {
    width: 360.0,
    height: 240.0,
};
const DETACHED_MEMORY_WINDOW_DEFAULT_SIZE: Size = Size {
    width: 500.0,
    height: 360.0,
};
const DETACHED_MEMORY_FONT_SCALE: f32 = 1.12;

impl ShellScrollArea {
    fn label(self) -> &'static str {
        match self {
            Self::Inspector => "INSPECTOR",
            Self::Bottom => "BOTTOM",
            Self::FloatingMemory => "MEMORY",
            Self::FloatingDock(kind) => kind.label(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ActiveDrag {
    LeftSplitter {
        start_x: f32,
        start_left_width: f32,
    },
    RightSplitter {
        start_x: f32,
        start_right_width: f32,
    },
    BottomSplitter {
        start_y: f32,
        start_bottom_height: f32,
    },
    CanvasPan {
        start: Point,
        start_pan: Point,
    },
    CanvasObjectMove {
        id: ObjectId,
        start: Point,
        start_rect: Rect,
    },
    CanvasObjectResize {
        id: ObjectId,
        handle: ResizeHandle,
        start: Point,
        start_rect: Rect,
    },
    GridPaint {
        last_index: Option<usize>,
        value: u8,
    },
    Slider {
        start_x: f32,
        start_value: f32,
    },
    FloatingWindow {
        start: Point,
        start_rect: Rect,
    },
    FloatingDockWindow {
        kind: FloatingDockKind,
        start: Point,
        start_rect: Rect,
    },
    Scrollbar {
        area: ShellScrollArea,
        axis: ScrollbarAxis,
        start: Point,
        start_scroll: Point,
    },
}

/// Retained Phase 1 shell state.
#[derive(Clone, Debug, PartialEq)]
pub struct CreativeShell {
    size: Size,
    selected_tool: ToolKind,
    active_menu: Option<MenuKind>,
    hovered_widget: Option<WidgetId>,
    active_widget: Option<WidgetId>,
    captured_widget: Option<WidgetId>,
    drag: Option<ActiveDrag>,
    left_width: f32,
    right_width: f32,
    bottom_height: f32,
    canvas_pan: Point,
    canvas_zoom: f32,
    objects: Vec<EditableRect>,
    selected_object: Option<ObjectId>,
    selected_objects: Vec<ObjectId>,
    next_object_id: u64,
    command_history: CommandHistory,
    clipboard: Vec<EditableRect>,
    grid_input: GridInput,
    snap_enabled: bool,
    snap_step: f32,
    slider_value: f32,
    context_menu: Option<ContextMenuState>,
    floating_debug: FloatingWindowState,
    floating_debug_scroll: Point,
    floating_docks: Vec<FloatingDockPanel>,
    floating_dock_scrolls: Vec<(FloatingDockKind, Point)>,
    dock_layout: DockLayoutState,
    saved_dock_layout_json: Option<String>,
    dock_layout_slots: [Option<String>; 2],
    memory_viewer: MemoryViewer,
    memory_editor: MemoryEditorState,
    active_inspector_panel: InspectorPanelKind,
    active_bottom_panel: BottomPanelKind,
    inspector_scroll: Point,
    bottom_scroll: Point,
    text_input: TextInputState,
    last_pointer: Point,
    status_text: String,
}

impl CreativeShell {
    /// Create a shell state for a surface size.
    pub fn new(size: Size) -> Self {
        let size = size.clamped();
        let layout = CreativeLayout::compute(size);
        let mut dock_layout = DockLayoutState::default_for_size(size);
        dock_layout.set_root_split_ratio(dock_root_split_ratio_from_layout(layout));
        dock_layout.set_main_split_ratio(dock_main_split_ratio_from_layout(layout));
        let active_inspector_panel = dock_layout.active_inspector_panel();
        let active_bottom_panel = dock_layout.active_bottom_panel();
        Self {
            size,
            selected_tool: ToolKind::Select,
            active_menu: None,
            hovered_widget: None,
            active_widget: None,
            captured_widget: None,
            drag: None,
            left_width: layout.tool_bar.width,
            right_width: layout.inspector.width,
            bottom_height: layout.timeline.height,
            canvas_pan: Point::new(0.0, 0.0),
            canvas_zoom: 1.0,
            objects: vec![EditableRect::new(
                ObjectId::new(1),
                Rect::new(60.0, 50.0, 112.0, 72.0),
                "RECT1",
            )],
            selected_object: None,
            selected_objects: Vec::new(),
            next_object_id: 2,
            command_history: CommandHistory::new(),
            clipboard: Vec::new(),
            grid_input: GridInput::new(GridInputConfig::default()),
            snap_enabled: true,
            snap_step: 8.0,
            slider_value: 0.65,
            context_menu: None,
            floating_debug: FloatingWindowState {
                rect: Rect::new((size.width - 390.0).max(40.0), 96.0, 360.0, 238.0),
                open: true,
            },
            floating_debug_scroll: Point::new(0.0, 0.0),
            floating_docks: Vec::new(),
            floating_dock_scrolls: Vec::new(),
            dock_layout,
            saved_dock_layout_json: None,
            dock_layout_slots: [None, None],
            memory_viewer: MemoryViewer::demo(),
            memory_editor: MemoryEditorState::new(),
            active_inspector_panel,
            active_bottom_panel,
            inspector_scroll: Point::new(0.0, 0.0),
            bottom_scroll: Point::new(0.0, 0.0),
            text_input: TextInputState::new("RECT1"),
            last_pointer: Point::new(0.0, 0.0),
            status_text: "STATUS READY TOOL SEL".to_string(),
        }
    }

    /// Current selected tool.
    pub const fn selected_tool(&self) -> ToolKind {
        self.selected_tool
    }

    /// Current active/open menu.
    pub const fn active_menu(&self) -> Option<MenuKind> {
        self.active_menu
    }

    /// Current canvas zoom factor.
    pub const fn canvas_zoom(&self) -> f32 {
        self.canvas_zoom
    }

    /// Current canvas pan offset.
    pub const fn canvas_pan(&self) -> Point {
        self.canvas_pan
    }

    /// Current hovered widget id.
    pub const fn hovered_widget(&self) -> Option<WidgetId> {
        self.hovered_widget
    }

    /// Current active widget id.
    pub const fn active_widget(&self) -> Option<WidgetId> {
        self.active_widget
    }

    /// Current status string.
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// Borrow the retained text input state.
    pub fn text_input(&self) -> &TextInputState {
        &self.text_input
    }

    /// Borrow the retained memory editor state.
    pub fn memory_editor(&self) -> &MemoryEditorState {
        &self.memory_editor
    }

    /// Whether the detached native memory editor window should be open.
    pub const fn memory_window_open(&self) -> bool {
        self.floating_debug.open
    }

    /// Current detached memory editor logical size.
    pub fn memory_window_size(&self) -> Size {
        Size::new(
            self.floating_debug
                .rect
                .width
                .max(DETACHED_MEMORY_WINDOW_DEFAULT_SIZE.width),
            self.floating_debug
                .rect
                .height
                .max(DETACHED_MEMORY_WINDOW_DEFAULT_SIZE.height),
        )
    }

    /// Resize the detached memory editor window surface.
    pub fn resize_memory_window(&mut self, size: Size) -> UiResponse {
        let size = Size::new(
            size.width.max(DETACHED_MEMORY_WINDOW_MIN_SIZE.width),
            size.height.max(DETACHED_MEMORY_WINDOW_MIN_SIZE.height),
        );
        self.floating_debug.rect.width = size.width;
        self.floating_debug.rect.height = size.height;
        self.clamp_detached_memory_scroll();
        UiResponse::redraw()
    }

    /// Close the detached memory editor window without discarding the memory buffer.
    pub fn close_memory_window(&mut self) -> UiResponse {
        self.floating_debug.open = false;
        self.memory_editor.focused = false;
        if matches!(
            self.drag,
            Some(ActiveDrag::Scrollbar {
                area: ShellScrollArea::FloatingMemory,
                ..
            })
        ) {
            self.drag = None;
        }
        self.set_status("MEMORY WINDOW CLOSED".to_string());
        UiResponse::redraw()
    }

    /// Dispatch an event from the detached memory editor window.
    pub fn handle_memory_window_event(&mut self, event: UiEvent) -> UiResponse {
        match event {
            UiEvent::PointerMove { position } => {
                self.last_pointer = position;
                if let Some(ActiveDrag::Scrollbar {
                    area: ShellScrollArea::FloatingMemory,
                    axis,
                    start,
                    start_scroll,
                }) = self.drag
                {
                    self.drag_detached_memory_scrollbar_to(axis, start, start_scroll, position);
                    return UiResponse::redraw();
                }
                UiResponse::none()
            }
            UiEvent::PointerDown { position, button } => {
                self.last_pointer = position;
                if button != PointerButton::Primary {
                    return UiResponse::none();
                }
                self.active_menu = None;
                self.text_input.focused = false;
                self.text_input.ime_preedit.clear();
                if self.begin_detached_memory_scrollbar_drag_at(position) {
                    return UiResponse::redraw();
                }
                if let Some(offset) = self.detached_memory_editor_offset_at(position) {
                    self.memory_editor
                        .select_offset(offset, self.memory_viewer.bytes.len());
                    self.set_status(format!(
                        "MEM EDIT FOCUS {:04X}",
                        self.memory_selected_address()
                    ));
                    UiResponse::redraw()
                } else {
                    self.memory_editor.focused = false;
                    UiResponse::redraw()
                }
            }
            UiEvent::PointerUp { .. } => {
                if matches!(
                    self.drag,
                    Some(ActiveDrag::Scrollbar {
                        area: ShellScrollArea::FloatingMemory,
                        ..
                    })
                ) {
                    self.drag = None;
                    return UiResponse::redraw();
                }
                UiResponse::none()
            }
            UiEvent::PointerWheel {
                delta_x, delta_y, ..
            } => {
                if self.scroll_detached_memory_window(delta_x, delta_y) {
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::KeyDown {
                key,
                ctrl,
                shift,
                alt,
                meta,
            } => {
                let input = KeyInput::from_legacy(&key, ctrl, shift, alt, meta);
                if self.memory_editor.focused && input.text_editing_mode() {
                    if let Some(response) = self
                        .handle_memory_editor_key(input.logical.as_str(), &input.logical_upper())
                    {
                        return response;
                    }
                    self.set_status(format!("MEM EDIT KEY {}", input.logical_upper()));
                    return UiResponse::redraw();
                }
                UiResponse::none()
            }
            UiEvent::TextInput { text } | UiEvent::ImeCommit { text } => {
                if self.memory_editor.focused && self.apply_memory_editor_text(&text) {
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::FocusChanged { focused } => {
                if !focused {
                    self.memory_editor.focused = false;
                    self.drag = None;
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::PointerDoubleClick { .. }
            | UiEvent::KeyUp { .. }
            | UiEvent::ImePreedit { .. }
            | UiEvent::FileDropped { .. } => UiResponse::none(),
        }
    }

    /// Build the draw list for the detached memory editor window.
    pub fn memory_window_draw_list(&self) -> DrawList {
        let theme = Phase1Theme::dark();
        let mut list = DrawList::new(MemoryViewerPalette::dark().background);
        let viewport = self.detached_memory_viewport();
        let content_size = self.detached_memory_content_size();
        list.fill_rect(viewport, MemoryViewerPalette::dark().background);
        list.push_clip(viewport);
        draw_memory_editor_scaled(
            &mut list,
            self.detached_memory_editor_rect(),
            "MEMORY EDITOR",
            self.memory_viewer.base_address,
            &self.memory_viewer.bytes,
            &self.memory_editor,
            MemoryViewerPalette::dark(),
            DETACHED_MEMORY_FONT_SCALE,
        );
        list.pop_clip();
        ScrollAreaState::from_offset(self.floating_debug_scroll).draw_scrollbars(
            &mut list,
            viewport,
            content_size,
            phase1_scrollbar_style(&theme),
        );
        list
    }

    /// Replace the inspector text input value.
    pub fn set_text_input_value(&mut self, value: impl Into<String>) {
        self.text_input = TextInputState::new(value);
        self.set_status(format!("TEXT INPUT SET {}", self.text_input.text));
    }

    /// Current selected canvas object.
    pub const fn selected_object(&self) -> Option<ObjectId> {
        self.selected_object
    }

    /// Current selected canvas object rectangle, if any.
    pub fn selected_object_rect(&self) -> Option<Rect> {
        self.selected_object.and_then(|id| self.object_rect(id))
    }

    /// Borrow the current multi-selection set.
    pub fn selected_objects(&self) -> &[ObjectId] {
        &self.selected_objects
    }

    /// Select all objects whose world rectangles intersect `world_rect`.
    pub fn select_objects_in_rect(&mut self, world_rect: Rect) -> usize {
        self.selected_objects = self
            .objects
            .iter()
            .filter(|object| object.rect.intersects(world_rect))
            .map(|object| object.id)
            .collect();
        self.selected_object = self.selected_objects.last().copied();
        self.set_status(format!("SELECTION MARQUEE {}", self.selected_objects.len()));
        self.selected_objects.len()
    }

    /// Enable or disable world-coordinate grid snapping.
    pub fn set_snap_to_grid(&mut self, enabled: bool, step: f32) {
        self.snap_enabled = enabled;
        self.snap_step = finite_at_least(step, 1.0);
        self.set_status(format!(
            "SNAP {} STEP {}",
            if self.snap_enabled { "ON" } else { "OFF" },
            self.snap_step.round() as i32
        ));
    }

    /// Snap a point to the configured world-coordinate grid.
    pub fn snap_point_to_grid(&self, point: Point) -> Point {
        if !self.snap_enabled {
            return point;
        }
        let step = self.snap_step.max(1.0);
        Point::new(
            (point.x / step).round() * step,
            (point.y / step).round() * step,
        )
    }

    /// Snap a rectangle origin to the configured world-coordinate grid.
    pub fn snap_rect_to_grid(&self, rect: Rect) -> Rect {
        let origin = self.snap_point_to_grid(Point::new(rect.x, rect.y));
        Rect::new(origin.x, origin.y, rect.width, rect.height)
    }

    /// Borrow the current canvas objects.
    pub fn objects(&self) -> &[EditableRect] {
        &self.objects
    }

    /// Export the current canvas document model.
    pub fn document_model(&self) -> CanvasDocument {
        CanvasDocument {
            objects: self.objects.clone(),
            selected: self.selected_objects.clone(),
        }
    }

    /// Number of undoable commands.
    pub fn undo_len(&self) -> usize {
        self.command_history.undo_len()
    }

    /// Number of redoable commands.
    pub fn redo_len(&self) -> usize {
        self.command_history.redo_len()
    }

    /// Number of objects currently stored in the in-memory clipboard.
    pub fn clipboard_len(&self) -> usize {
        self.clipboard.len()
    }

    /// Borrow the current configurable grid input.
    pub fn grid_input(&self) -> &GridInput {
        &self.grid_input
    }

    /// Replace the grid input configuration. Existing cell contents are reset.
    pub fn set_grid_input_config(&mut self, config: GridInputConfig) {
        self.grid_input = GridInput::new(config);
        self.set_status(format!(
            "GRID CONFIG C {} R {} CELL {}",
            self.grid_input.config.columns,
            self.grid_input.config.rows,
            self.grid_input.config.cell_size.round() as i32
        ));
    }

    /// Current slider value used by the Phase 1 rich slider prototype.
    pub const fn slider_value(&self) -> f32 {
        self.slider_value
    }

    /// Whether the debugger floating window is open.
    pub const fn floating_debug_open(&self) -> bool {
        self.floating_debug.open
    }

    /// Borrow the currently open floating dock panels.
    pub fn floating_docks(&self) -> &[FloatingDockPanel] {
        &self.floating_docks
    }

    /// Borrow the retained Dock Layout State foundation.
    pub fn dock_layout(&self) -> &DockLayoutState {
        &self.dock_layout
    }

    /// Export the current Dock Layout State as a deterministic JSON seed.
    pub fn dock_layout_json(&self) -> String {
        self.dock_layout.to_json_pretty()
    }

    /// Borrow the last menu-saved Dock Layout JSON seed, if any.
    pub fn saved_dock_layout_json(&self) -> Option<&str> {
        self.saved_dock_layout_json.as_deref()
    }

    /// Number of bytes in the last menu-saved Dock Layout JSON seed.
    pub fn saved_dock_layout_len(&self) -> usize {
        self.saved_dock_layout_json
            .as_ref()
            .map(|text| text.len())
            .unwrap_or(0)
    }

    /// Compact Dock Layout State summary for diagnostics/status UI.
    pub fn dock_layout_summary_text(&self) -> String {
        self.dock_layout
            .summary_line(self.saved_dock_layout_json.is_some())
    }

    /// Compact Dock Tree split-ratio summary for diagnostics/status UI.
    pub fn dock_split_summary_text(&self) -> String {
        self.dock_layout.split_summary_line()
    }

    /// Compact retained dock tab visibility summary for diagnostics/status UI.
    pub fn dock_tab_summary_text(&self) -> String {
        self.dock_layout.tab_summary_line()
    }

    /// Hide or show one fixed dock tab in the retained Dock Tree.
    pub fn set_dock_panel_tab_open(&mut self, panel: DockPanelKind, open: bool) -> bool {
        let changed = self.dock_layout.set_panel_tab_open(panel, open);
        if changed {
            self.active_inspector_panel = self.dock_layout.active_inspector_panel();
            self.active_bottom_panel = self.dock_layout.active_bottom_panel();
            self.set_status(format!(
                "DOCK TAB {} {}",
                panel.tab_label(),
                if open { "OPEN" } else { "HIDDEN" }
            ));
        } else {
            self.set_status(format!("DOCK TAB {} KEEP", panel.tab_label()));
        }
        changed
    }

    /// Restore every retained fixed dock tab to visible state.
    pub fn restore_dock_tabs(&mut self) {
        self.dock_layout.restore_all_tabs();
        self.active_inspector_panel = self.dock_layout.active_inspector_panel();
        self.active_bottom_panel = self.dock_layout.active_bottom_panel();
        self.set_status("DOCK TABS RESTORED".to_string());
    }

    /// Detach a fixed dock panel into the floating dock layer.
    pub fn detach_dock_panel(&mut self, panel: DockPanelKind) -> bool {
        let changed = self.dock_layout.detach_panel(panel, self.size);
        if changed {
            self.apply_dock_layout_state(self.dock_layout.clone());
            self.set_status(format!("DOCK PANEL DETACHED {}", panel.tab_label()));
        } else {
            self.set_status(format!("DOCK PANEL KEEP {}", panel.tab_label()));
        }
        changed
    }

    /// Attach a floating dock panel back to its default fixed area.
    pub fn attach_dock_panel(&mut self, panel: DockPanelKind) -> bool {
        let changed = self.dock_layout.attach_panel(panel);
        self.apply_dock_layout_state(self.dock_layout.clone());
        self.set_status(format!("DOCK PANEL ATTACHED {}", panel.tab_label()));
        changed
    }

    /// Attach all open floating dock panels.
    pub fn attach_all_floating_dock_panels(&mut self) -> usize {
        let count = self.dock_layout.attach_all_floating_panels();
        self.apply_dock_layout_state(self.dock_layout.clone());
        self.set_status(format!("DOCK FLOATING ATTACHED {}", count));
        count
    }

    /// Allocate current Dock Tree geometry inside the editor content area.
    pub fn dock_layout_geometry(&self) -> DockLayoutGeometry {
        let layout = self.layout();
        let dock_rect = Rect::new(
            layout.canvas.x,
            layout.canvas.y,
            layout.inspector.right() - layout.canvas.x,
            layout.timeline.bottom() - layout.canvas.y,
        );
        self.dock_layout.allocate_geometry(dock_rect)
    }

    /// Save the current layout seed into an in-memory slot for quick A/B checks.
    pub fn save_dock_layout_slot(&mut self, slot: usize) -> bool {
        if slot >= self.dock_layout_slots.len() {
            self.set_status(format!("DOCK LAYOUT SLOT {} NG", slot + 1));
            return false;
        }
        let json = self.save_dock_layout_seed();
        let bytes = json.len();
        self.dock_layout_slots[slot] = Some(json);
        self.set_status(format!(
            "DOCK LAYOUT SLOT {} SAVED {} BYTES",
            slot + 1,
            bytes
        ));
        true
    }

    /// Load a layout seed from an in-memory slot.
    pub fn load_dock_layout_slot(&mut self, slot: usize) -> bool {
        if slot >= self.dock_layout_slots.len() {
            self.set_status(format!("DOCK LAYOUT SLOT {} NG", slot + 1));
            return false;
        }
        let Some(json) = self.dock_layout_slots[slot].clone() else {
            self.set_status(format!("DOCK LAYOUT SLOT {} EMPTY", slot + 1));
            return false;
        };
        let loaded = self.load_dock_layout_json(&json);
        if loaded {
            self.set_status(format!("DOCK LAYOUT SLOT {} LOADED", slot + 1));
        }
        loaded
    }

    /// Compact memory-slot summary for diagnostics.
    pub fn dock_layout_slot_summary_text(&self) -> String {
        let a = self.dock_layout_slots[0]
            .as_ref()
            .map(|text| text.len())
            .unwrap_or(0);
        let b = self.dock_layout_slots[1]
            .as_ref()
            .map(|text| text.len())
            .unwrap_or(0);
        format!("SLOTS A {}B  B {}B", a, b)
    }

    /// Save the current dock layout as a versioned profile payload.
    pub fn save_dock_layout_profile_text(&mut self) -> String {
        self.sync_dock_layout_from_shell();
        let profile = self.dock_layout.to_profile_text();
        self.set_status(format!("DOCK PROFILE SAVED {} BYTES", profile.len()));
        profile
    }

    /// Load a versioned dock layout profile payload.
    pub fn load_dock_layout_profile_text(&mut self, text: &str) -> bool {
        let Some(layout) = DockLayoutState::from_profile_text(text, self.size) else {
            self.set_status("DOCK PROFILE LOAD NG".to_string());
            return false;
        };
        self.apply_dock_layout_state(layout);
        self.set_status("DOCK PROFILE LOADED".to_string());
        true
    }

    fn sync_root_split_ratio_from_legacy_layout(&mut self) {
        let layout = self.layout();
        self.dock_layout
            .set_root_split_ratio(dock_root_split_ratio_from_layout(layout));
    }

    fn sync_split_ratios_from_legacy_layout(&mut self) {
        let layout = self.layout();
        self.dock_layout
            .set_root_split_ratio(dock_root_split_ratio_from_layout(layout));
        self.dock_layout
            .set_main_split_ratio(dock_main_split_ratio_from_layout(layout));
    }

    fn sync_main_split_ratio_from_legacy_layout(&mut self) {
        let layout = self.layout();
        self.dock_layout
            .set_main_split_ratio(dock_main_split_ratio_from_layout(layout));
    }

    fn apply_root_split_ratio_to_legacy_layout(&mut self) {
        self.right_width =
            dock_right_width_from_root_split_ratio(self.size, self.dock_layout.root_split_ratio());
        let layout = self.layout();
        self.right_width = layout.inspector.width;
        self.sync_root_split_ratio_from_legacy_layout();
    }

    fn apply_main_split_ratio_to_legacy_layout(&mut self) {
        self.bottom_height = dock_bottom_height_from_main_split_ratio(
            self.size,
            self.dock_layout.main_split_ratio(),
        );
        let layout = self.layout();
        self.bottom_height = layout.timeline.height;
        self.sync_main_split_ratio_from_legacy_layout();
    }

    fn apply_split_ratios_to_legacy_layout(&mut self) {
        self.apply_root_split_ratio_to_legacy_layout();
        self.apply_main_split_ratio_to_legacy_layout();
    }

    /// Save the current retained Dock Layout State into the in-memory v15 seed slot.
    pub fn save_dock_layout_seed(&mut self) -> String {
        self.sync_dock_layout_from_shell();
        let json = self.dock_layout.to_json_pretty();
        let bytes = json.len();
        self.saved_dock_layout_json = Some(json.clone());
        self.set_status(format!("DOCK LAYOUT SAVED {} BYTES", bytes));
        json
    }

    /// Load the last in-memory Dock Layout seed produced by `save_dock_layout_seed()`.
    pub fn load_saved_dock_layout_seed(&mut self) -> bool {
        let Some(json) = self.saved_dock_layout_json.clone() else {
            self.set_status("DOCK LAYOUT LOAD EMPTY".to_string());
            return false;
        };
        self.load_dock_layout_json(&json)
    }

    /// Load a Dock Layout JSON seed and apply it to the current shell.
    pub fn load_dock_layout_json(&mut self, json: &str) -> bool {
        let Some(layout) = DockLayoutState::from_json_seed(json, self.size) else {
            self.set_status("DOCK LAYOUT LOAD NG".to_string());
            return false;
        };
        self.apply_dock_layout_state(layout);
        self.set_status("DOCK LAYOUT LOADED".to_string());
        true
    }

    fn apply_dock_layout_state(&mut self, mut layout: DockLayoutState) {
        layout.normalize_for_size(self.size);
        self.dock_layout = layout;
        self.apply_split_ratios_to_legacy_layout();
        self.active_inspector_panel = self.dock_layout.active_inspector_panel();
        self.active_bottom_panel = self.dock_layout.active_bottom_panel();
        self.floating_docks = self
            .dock_layout
            .floating_panels()
            .iter()
            .filter(|panel| panel.open)
            .map(|panel| FloatingDockPanel::new(panel.panel.floating_kind(), panel.rect))
            .collect();
        self.drag = None;
    }

    /// Number of currently open floating dock panels.
    pub fn floating_dock_count(&self) -> usize {
        self.floating_docks
            .iter()
            .filter(|panel| panel.open)
            .count()
    }

    /// Borrow the memory viewer model.
    pub fn memory_viewer(&self) -> &MemoryViewer {
        &self.memory_viewer
    }

    /// Active right dock panel.
    pub const fn active_inspector_panel(&self) -> InspectorPanelKind {
        self.active_inspector_panel
    }

    /// Active bottom dock panel.
    pub const fn active_bottom_panel(&self) -> BottomPanelKind {
        self.active_bottom_panel
    }

    /// Current scroll offset of the inspector dock content.
    pub const fn inspector_scroll(&self) -> Point {
        self.inspector_scroll
    }

    /// Current scroll offset of the bottom dock content.
    pub const fn bottom_scroll(&self) -> Point {
        self.bottom_scroll
    }

    fn set_active_inspector_panel(&mut self, panel: InspectorPanelKind) {
        self.active_inspector_panel = panel;
        self.dock_layout.set_active_inspector_panel(panel);
    }

    fn set_active_bottom_panel(&mut self, panel: BottomPanelKind) {
        self.active_bottom_panel = panel;
        self.dock_layout.set_active_bottom_panel(panel);
    }

    fn sync_dock_layout_from_shell(&mut self) {
        self.sync_split_ratios_from_legacy_layout();
        self.dock_layout
            .set_active_inspector_panel(self.active_inspector_panel);
        self.dock_layout
            .set_active_bottom_panel(self.active_bottom_panel);
        self.dock_layout
            .sync_floating_from_legacy(&self.floating_docks, self.size);
        self.floating_docks = self
            .dock_layout
            .floating_panels()
            .iter()
            .filter(|panel| panel.open)
            .map(|panel| FloatingDockPanel::new(panel.panel.floating_kind(), panel.rect))
            .collect();
    }

    /// Reset the retained dock layout model while preserving the rest of the document.
    pub fn reset_dock_layout(&mut self) {
        let default_layout = CreativeLayout::compute(self.size);
        self.left_width = default_layout.tool_bar.width;
        self.right_width = default_layout.inspector.width;
        self.bottom_height = default_layout.timeline.height;
        self.dock_layout = DockLayoutState::default_for_size(self.size);
        self.sync_split_ratios_from_legacy_layout();
        self.active_inspector_panel = self.dock_layout.active_inspector_panel();
        self.active_bottom_panel = self.dock_layout.active_bottom_panel();
        self.floating_docks.clear();
        self.floating_dock_scrolls.clear();
        self.inspector_scroll = Point::new(0.0, 0.0);
        self.bottom_scroll = Point::new(0.0, 0.0);
        self.floating_debug_scroll = Point::new(0.0, 0.0);
        self.drag = None;
        self.set_status("DOCK LAYOUT RESET".to_string());
    }

    /// Replace the bytes shown in the debugger memory viewer.
    pub fn set_memory_viewer_bytes(&mut self, base_address: u16, bytes: Vec<u8>) {
        self.memory_viewer.set_bytes(base_address, bytes);
        self.memory_editor
            .clamp_to_len(self.memory_viewer.bytes.len());
        self.floating_debug.open = true;
        self.set_status(format!("MEMORY VIEW BASE {:04X}", base_address));
    }

    /// Export a tiny line-oriented text snapshot for Phase 1 diagnostics.
    ///
    /// This is intentionally not the final project file format. It gives tests,
    /// menus, and future file dialogs a deterministic document payload before
    /// serde-backed persistence is introduced.
    pub fn snapshot_text(&self) -> String {
        let mut text = format!(
            "PLITA_PHASE2_V2\nTEXT {}\nSIZE {} {}\nPAN {} {}\nZOOM {}\nOBJECTS {}\nFLOATING_DOCKS {}\n",
            self.text_input.text,
            self.size.width.round() as i32,
            self.size.height.round() as i32,
            self.canvas_pan.x.round() as i32,
            self.canvas_pan.y.round() as i32,
            (self.canvas_zoom * 100.0).round() as i32,
            self.objects.len(),
            self.floating_dock_count()
        );
        for object in &self.objects {
            text.push_str(&format!(
                "RECT {} {} {} {} {} {}\n",
                object.id.raw(),
                object.rect.x.round() as i32,
                object.rect.y.round() as i32,
                object.rect.width.round() as i32,
                object.rect.height.round() as i32,
                object.label
            ));
        }
        text
    }

    /// Reset the prototype document to the deterministic demo state.
    pub fn reset_demo_document(&mut self) {
        self.objects = vec![EditableRect::new(
            ObjectId::new(1),
            Rect::new(60.0, 50.0, 112.0, 72.0),
            "RECT1",
        )];
        self.clear_object_selection();
        self.next_object_id = 2;
        self.command_history = CommandHistory::new();
        self.clipboard.clear();
        self.grid_input = GridInput::new(GridInputConfig::default());
        self.context_menu = None;
        self.floating_docks.clear();
        self.floating_dock_scrolls.clear();
        self.slider_value = 0.65;
        self.text_input = TextInputState::new("RECT1");
        self.memory_editor = MemoryEditorState::new();
        self.set_active_inspector_panel(InspectorPanelKind::Properties);
        self.set_active_bottom_panel(BottomPanelKind::Timeline);
        self.sync_dock_layout_from_shell();
        self.canvas_pan = Point::new(0.0, 0.0);
        self.canvas_zoom = 1.0;
        self.inspector_scroll = Point::new(0.0, 0.0);
        self.bottom_scroll = Point::new(0.0, 0.0);
        self.floating_debug_scroll = Point::new(0.0, 0.0);
        self.set_status("COMMAND OPEN RESET DEMO".to_string());
    }

    /// Resize the retained shell while preserving side-panel widths where possible.
    pub fn resize(&mut self, size: Size) -> UiResponse {
        self.size = size.clamped();
        let layout = self.layout();
        self.left_width = layout.tool_bar.width;
        self.right_width = layout.inspector.width;
        self.bottom_height = layout.timeline.height;
        self.sync_dock_layout_from_shell();
        self.set_status(format!(
            "STATUS RESIZE W {} H {} TOOL {}",
            self.size.width.round() as i32,
            self.size.height.round() as i32,
            self.selected_tool.label()
        ));
        UiResponse::redraw()
    }

    /// Compute the current layout.
    pub fn layout(&self) -> CreativeLayout {
        CreativeLayout::compute_with_panel_sizes(
            self.size,
            Some(self.left_width),
            Some(self.right_width),
            Some(self.bottom_height),
        )
    }

    /// Build the retained widget tree for current layout/state.
    pub fn widget_tree(&self) -> WidgetTree {
        let layout = self.layout();
        let mut tree = WidgetTree::new();
        tree.push(WidgetNode::new(
            WidgetId::new(1),
            WidgetRole::Root,
            layout.root,
            "ROOT",
            WidgetVisualState::Normal,
        ));
        tree.push(WidgetNode::new(
            WidgetId::new(2),
            WidgetRole::MenuBar,
            layout.menu_bar,
            "MENU",
            self.visual_state_for(WidgetId::new(2)),
        ));

        for (menu, rect) in self.menu_item_rects(layout) {
            tree.push(WidgetNode::new(
                menu.widget_id(),
                WidgetRole::MenuItem(menu),
                rect,
                menu.label(),
                self.visual_state_for_menu(menu),
            ));
        }

        tree.push(WidgetNode::new(
            WidgetId::new(3),
            WidgetRole::ToolBar,
            layout.tool_bar,
            "TOOLBAR",
            self.visual_state_for(WidgetId::new(3)),
        ));
        tree.push(WidgetNode::new(
            WidgetId::new(4),
            WidgetRole::Canvas,
            layout.canvas,
            "CANVAS",
            self.visual_state_for(WidgetId::new(4)),
        ));
        tree.push(WidgetNode::new(
            WidgetId::new(5),
            WidgetRole::Inspector,
            layout.inspector,
            "INSPECTOR",
            self.visual_state_for(WidgetId::new(5)),
        ));

        for (panel, rect) in self.inspector_tab_rects(layout) {
            tree.push(WidgetNode::new(
                panel.widget_id(),
                WidgetRole::InspectorTab(panel),
                rect,
                panel.label(),
                self.visual_state_for_inspector_panel(panel),
            ));
            let dock_panel = DockPanelKind::from_inspector(panel);
            tree.push(WidgetNode::new(
                dock_panel.tab_close_widget_id(),
                WidgetRole::DockTabClose(dock_panel),
                dock_tab_close_rect(rect),
                "DOCK TAB CLOSE",
                self.visual_state_for(dock_panel.tab_close_widget_id()),
            ));
        }
        tree.push(WidgetNode::new(
            WidgetId::new(6),
            WidgetRole::Timeline,
            layout.timeline,
            "TIMELINE",
            self.visual_state_for(WidgetId::new(6)),
        ));

        for (panel, rect) in self.bottom_tab_rects(layout) {
            tree.push(WidgetNode::new(
                panel.widget_id(),
                WidgetRole::BottomTab(panel),
                rect,
                panel.label(),
                self.visual_state_for_bottom_panel(panel),
            ));
            let dock_panel = DockPanelKind::from_bottom(panel);
            tree.push(WidgetNode::new(
                dock_panel.tab_close_widget_id(),
                WidgetRole::DockTabClose(dock_panel),
                dock_tab_close_rect(rect),
                "DOCK TAB CLOSE",
                self.visual_state_for(dock_panel.tab_close_widget_id()),
            ));
        }
        tree.push(WidgetNode::new(
            WidgetId::new(7),
            WidgetRole::StatusBar,
            layout.status_bar,
            "STATUS",
            self.visual_state_for(WidgetId::new(7)),
        ));
        tree.push(WidgetNode::new(
            WidgetId::new(8),
            WidgetRole::Splitter(SplitterKind::Left),
            layout.left_splitter,
            "LEFT SPLITTER",
            self.visual_state_for(WidgetId::new(8)),
        ));
        tree.push(WidgetNode::new(
            WidgetId::new(9),
            WidgetRole::Splitter(SplitterKind::Right),
            layout.right_splitter,
            "RIGHT SPLITTER",
            self.visual_state_for(WidgetId::new(9)),
        ));
        tree.push(WidgetNode::new(
            BOTTOM_SPLITTER_ID,
            WidgetRole::Splitter(SplitterKind::Bottom),
            bottom_splitter_rect(layout),
            "BOTTOM SPLITTER",
            self.visual_state_for(BOTTOM_SPLITTER_ID),
        ));

        for (tool, rect) in self.tool_button_rects(layout) {
            tree.push(WidgetNode::new(
                tool.widget_id(),
                WidgetRole::ToolButton(tool),
                rect,
                tool.label(),
                self.visual_state_for_tool(tool),
            ));
        }

        for (id, label, rect) in self.rich_button_rects(layout) {
            tree.push(WidgetNode::new(
                id,
                WidgetRole::RichButton,
                rect,
                label,
                self.visual_state_for(id),
            ));
        }

        tree.push(WidgetNode::new(
            SLIDER_ID,
            WidgetRole::Slider,
            self.slider_rect(layout),
            "OPACITY SLIDER",
            self.visual_state_for(SLIDER_ID),
        ));

        tree.push(WidgetNode::new(
            TEXT_INPUT_ID,
            WidgetRole::TextInput,
            self.text_input_rect(layout),
            "NAME TEXT INPUT",
            self.visual_state_for(TEXT_INPUT_ID),
        ));

        for panel in self.floating_docks.iter().filter(|panel| panel.open) {
            tree.push(WidgetNode::new(
                panel.kind.title_widget_id(),
                WidgetRole::FloatingDockTitle(panel.kind),
                floating_dock_title_rect(panel.rect),
                panel.kind.label(),
                self.visual_state_for(panel.kind.title_widget_id()),
            ));
            tree.push(WidgetNode::new(
                panel.kind.close_widget_id(),
                WidgetRole::FloatingDockClose(panel.kind),
                floating_dock_close_rect(panel.rect),
                "FLOAT DOCK CLOSE",
                self.visual_state_for(panel.kind.close_widget_id()),
            ));
        }

        tree
    }

    /// Dispatch an event to the retained widget tree.
    pub fn handle_event(&mut self, event: UiEvent) -> UiResponse {
        match event {
            UiEvent::PointerMove { position } => self.handle_pointer_move(position),
            UiEvent::PointerDown { position, button } => self.handle_pointer_down(position, button),
            UiEvent::PointerUp { position, button } => self.handle_pointer_up(position, button),
            UiEvent::PointerDoubleClick { .. } => UiResponse::none(),
            UiEvent::PointerWheel {
                position,
                delta_x,
                delta_y,
            } => {
                self.last_pointer = position;
                if self.scroll_window_at(position, delta_x, delta_y) {
                    return UiResponse::redraw();
                }
                let hit_role = self.widget_tree().hit_test(position).map(|node| node.role);
                if matches!(hit_role, Some(WidgetRole::Canvas)) {
                    let zoom_delta = if delta_y > 0.0 {
                        1.10_f32
                    } else if delta_y < 0.0 {
                        0.90_f32
                    } else {
                        1.0_f32
                    };
                    self.canvas_zoom = (self.canvas_zoom * zoom_delta).clamp(0.25_f32, 4.0_f32);
                    self.set_status(format!(
                        "STATUS CANVAS ZOOM {} DX {} DY {}",
                        (self.canvas_zoom * 100.0).round() as i32,
                        delta_x.round() as i32,
                        delta_y.round() as i32
                    ));
                } else {
                    self.set_status(format!(
                        "STATUS WHEEL X {} Y {} DX {} DY {} TOOL {}",
                        position.x.round() as i32,
                        position.y.round() as i32,
                        delta_x.round() as i32,
                        delta_y.round() as i32,
                        self.selected_tool.label()
                    ));
                }
                UiResponse::redraw()
            }
            UiEvent::KeyDown {
                key,
                ctrl,
                shift,
                alt,
                meta,
            } => self.handle_key_down(&key, ctrl, shift, alt, meta),
            UiEvent::KeyUp { key, .. } => {
                self.set_status(format!("KEY UP {}", key));
                UiResponse::redraw()
            }
            UiEvent::FocusChanged { focused } => {
                self.set_status(format!("FOCUS {}", focused));
                UiResponse::redraw()
            }
            UiEvent::TextInput { text } | UiEvent::ImeCommit { text } => {
                if self.memory_editor.focused {
                    if self.apply_memory_editor_text(&text) {
                        UiResponse::redraw()
                    } else {
                        UiResponse::none()
                    }
                } else if self.text_input.focused {
                    self.text_input.insert_text(&text);
                    self.text_input.ime_preedit.clear();
                    self.set_status(format!("TEXT INPUT {}", self.text_input.text));
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::ImePreedit { text } => {
                if self.text_input.focused {
                    self.text_input.ime_preedit = text;
                    self.set_status(format!("IME PREEDIT {}", self.text_input.ime_preedit));
                    UiResponse::redraw()
                } else {
                    UiResponse::none()
                }
            }
            UiEvent::FileDropped { path } => {
                self.set_status(format!("FILE DROPPED {}", path.display()));
                UiResponse::redraw()
            }
        }
    }

    fn scroll_window_at(&mut self, position: Point, delta_x: f32, delta_y: f32) -> bool {
        let Some(area) = self.scroll_area_at(position) else {
            return false;
        };
        let Some((viewport, content_size, offset)) = self.scroll_area_spec(area) else {
            return false;
        };
        let mut state = ScrollAreaState::from_offset(offset);
        let changed = state.wheel(delta_x, delta_y, viewport, content_size);
        if changed {
            let next = state.offset();
            self.set_scroll_area_offset(area, next);
            self.set_status(format!(
                "SCROLL {} X {} Y {}",
                area.label(),
                next.x.round() as i32,
                next.y.round() as i32
            ));
        }
        changed
    }

    fn scroll_area_at(&self, position: Point) -> Option<ShellScrollArea> {
        let layout = self.layout();
        if layout.inspector.contains(position) {
            return Some(ShellScrollArea::Inspector);
        }
        if layout.timeline.contains(position) {
            return Some(ShellScrollArea::Bottom);
        }
        self.floating_docks
            .iter()
            .find(|panel| panel.open && panel.rect.contains(position))
            .map(|panel| ShellScrollArea::FloatingDock(panel.kind))
    }

    fn scrollbar_area_at(&self, position: Point) -> Option<ShellScrollArea> {
        for panel in self.floating_docks.iter().rev() {
            if panel.open && panel.rect.contains(position) {
                return Some(ShellScrollArea::FloatingDock(panel.kind));
            }
        }
        let layout = self.layout();
        if layout.inspector.contains(position) {
            Some(ShellScrollArea::Inspector)
        } else if layout.timeline.contains(position) {
            Some(ShellScrollArea::Bottom)
        } else {
            None
        }
    }

    fn scroll_area_spec(&self, area: ShellScrollArea) -> Option<(Rect, Size, Point)> {
        let viewport = self.scroll_area_viewport(area)?;
        let extra = match area {
            ShellScrollArea::Inspector => INSPECTOR_SCROLL_EXTRA,
            ShellScrollArea::Bottom => BOTTOM_SCROLL_EXTRA,
            ShellScrollArea::FloatingMemory => FLOATING_MEMORY_SCROLL_EXTRA,
            ShellScrollArea::FloatingDock(_) => FLOATING_DOCK_SCROLL_EXTRA,
        };
        let content_size = Size::new(
            finite_nonnegative(viewport.width) + extra.x,
            finite_nonnegative(viewport.height) + extra.y,
        );
        Some((viewport, content_size, self.scroll_area_offset(area)))
    }

    fn scroll_area_viewport(&self, area: ShellScrollArea) -> Option<Rect> {
        match area {
            ShellScrollArea::Inspector => {
                let rect = self.layout().inspector;
                Some(Rect::new(
                    rect.x + 1.0,
                    rect.y + 72.0,
                    rect.width - 2.0,
                    rect.height - 74.0,
                ))
            }
            ShellScrollArea::Bottom => {
                let rect = self.layout().timeline;
                Some(Rect::new(
                    rect.x + 1.0,
                    rect.y + 31.0,
                    rect.width - 2.0,
                    rect.height - 32.0,
                ))
            }
            ShellScrollArea::FloatingMemory => {
                if !self.floating_debug.open {
                    return None;
                }
                Some(self.floating_memory_body_rect())
            }
            ShellScrollArea::FloatingDock(kind) => {
                let rect = self
                    .floating_docks
                    .iter()
                    .find(|panel| panel.open && panel.kind == kind)
                    .map(|panel| panel.rect)?;
                Some(Rect::new(
                    rect.x + 10.0,
                    rect.y + 40.0,
                    rect.width - 20.0,
                    rect.height - 52.0,
                ))
            }
        }
    }

    fn scroll_area_offset(&self, area: ShellScrollArea) -> Point {
        match area {
            ShellScrollArea::Inspector => self.inspector_scroll,
            ShellScrollArea::Bottom => self.bottom_scroll,
            ShellScrollArea::FloatingMemory => self.floating_debug_scroll,
            ShellScrollArea::FloatingDock(kind) => self.floating_dock_scroll(kind),
        }
    }

    fn set_scroll_area_offset(&mut self, area: ShellScrollArea, offset: Point) {
        match area {
            ShellScrollArea::Inspector => self.inspector_scroll = offset,
            ShellScrollArea::Bottom => self.bottom_scroll = offset,
            ShellScrollArea::FloatingMemory => self.floating_debug_scroll = offset,
            ShellScrollArea::FloatingDock(kind) => self.set_floating_dock_scroll(kind, offset),
        }
    }

    fn begin_scrollbar_drag_at(&mut self, position: Point) -> bool {
        let Some(area) = self.scrollbar_area_at(position) else {
            return false;
        };
        let Some((viewport, content_size, offset)) = self.scroll_area_spec(area) else {
            return false;
        };
        let mut state = ScrollAreaState::from_offset(offset);
        let Some(hit) = state.begin_scrollbar_drag(
            viewport,
            content_size,
            position,
            phase1_scrollbar_input_style(),
        ) else {
            return false;
        };
        let next = state.offset();
        self.set_scroll_area_offset(area, next);
        if let Some(drag) = state.drag() {
            self.drag = Some(ActiveDrag::Scrollbar {
                area,
                axis: drag.axis,
                start: drag.start_pointer,
                start_scroll: drag.start_offset,
            });
        }
        self.active_menu = None;
        self.active_widget = None;
        self.captured_widget = None;
        self.set_status(format!(
            "SCROLLBAR {:?} {} X {} Y {}",
            hit.axis,
            area.label(),
            next.x.round() as i32,
            next.y.round() as i32
        ));
        true
    }

    fn drag_scrollbar_to(
        &mut self,
        area: ShellScrollArea,
        axis: ScrollbarAxis,
        start: Point,
        start_scroll: Point,
        position: Point,
    ) -> bool {
        let Some((viewport, content_size, _)) = self.scroll_area_spec(area) else {
            return false;
        };
        let mut state = ScrollAreaState {
            offset: start_scroll,
            drag: Some(ScrollbarDrag {
                axis,
                start_pointer: start,
                start_offset: start_scroll,
            }),
        };
        let changed = state.drag_scrollbar(
            viewport,
            content_size,
            position,
            phase1_scrollbar_input_style(),
        );
        let next = state.offset();
        self.set_scroll_area_offset(area, next);
        self.set_status(format!(
            "SCROLLBAR DRAG {} X {} Y {}",
            area.label(),
            next.x.round() as i32,
            next.y.round() as i32
        ));
        changed
    }

    fn detached_memory_viewport(&self) -> Rect {
        let size = self.memory_window_size();
        Rect::new(0.0, 0.0, size.width, size.height)
    }

    fn detached_memory_content_size(&self) -> Size {
        let viewport = self.detached_memory_viewport();
        let rows = self
            .memory_viewer
            .bytes
            .len()
            .div_ceil(MemoryEditorState::BYTES_PER_ROW)
            .max(1);
        let metrics = MemoryEditorMetrics::new(DETACHED_MEMORY_FONT_SCALE);
        let content_height =
            metrics.header_height + rows as f32 * metrics.row_height + 18.0 * metrics.scale;
        Size::new(viewport.width, content_height.max(viewport.height))
    }

    fn detached_memory_editor_rect(&self) -> Rect {
        let viewport = self.detached_memory_viewport();
        let content_size = self.detached_memory_content_size();
        Rect::new(
            viewport.x - self.floating_debug_scroll.x,
            viewport.y - self.floating_debug_scroll.y,
            content_size.width,
            content_size.height,
        )
    }

    fn detached_memory_editor_offset_at(&self, position: Point) -> Option<usize> {
        if !self.detached_memory_viewport().contains(position) {
            return None;
        }
        MemoryEditorState::hit_offset_scaled(
            self.detached_memory_editor_rect(),
            position,
            self.memory_viewer.bytes.len(),
            DETACHED_MEMORY_FONT_SCALE,
        )
    }

    fn scroll_detached_memory_window(&mut self, delta_x: f32, delta_y: f32) -> bool {
        let viewport = self.detached_memory_viewport();
        let content_size = self.detached_memory_content_size();
        let mut state = ScrollAreaState::from_offset(self.floating_debug_scroll);
        let changed = state.wheel(delta_x, delta_y, viewport, content_size);
        if changed {
            self.floating_debug_scroll = state.offset();
            self.set_status(format!(
                "MEMORY SCROLL Y {}",
                self.floating_debug_scroll.y.round() as i32
            ));
        }
        changed
    }

    fn clamp_detached_memory_scroll(&mut self) {
        let viewport = self.detached_memory_viewport();
        let content_size = self.detached_memory_content_size();
        let mut state = ScrollAreaState::from_offset(self.floating_debug_scroll);
        state.set_offset(self.floating_debug_scroll, viewport, content_size);
        self.floating_debug_scroll = state.offset();
    }

    fn begin_detached_memory_scrollbar_drag_at(&mut self, position: Point) -> bool {
        let viewport = self.detached_memory_viewport();
        let content_size = self.detached_memory_content_size();
        let mut state = ScrollAreaState::from_offset(self.floating_debug_scroll);
        let Some(hit) = state.begin_scrollbar_drag(
            viewport,
            content_size,
            position,
            phase1_scrollbar_input_style(),
        ) else {
            return false;
        };
        self.floating_debug_scroll = state.offset();
        if let Some(drag) = state.drag() {
            self.drag = Some(ActiveDrag::Scrollbar {
                area: ShellScrollArea::FloatingMemory,
                axis: drag.axis,
                start: drag.start_pointer,
                start_scroll: drag.start_offset,
            });
        }
        self.set_status(format!(
            "MEMORY SCROLLBAR {:?} Y {}",
            hit.axis,
            self.floating_debug_scroll.y.round() as i32
        ));
        true
    }

    fn drag_detached_memory_scrollbar_to(
        &mut self,
        axis: ScrollbarAxis,
        start: Point,
        start_scroll: Point,
        position: Point,
    ) -> bool {
        let viewport = self.detached_memory_viewport();
        let content_size = self.detached_memory_content_size();
        let mut state = ScrollAreaState {
            offset: start_scroll,
            drag: Some(ScrollbarDrag {
                axis,
                start_pointer: start,
                start_offset: start_scroll,
            }),
        };
        let changed = state.drag_scrollbar(
            viewport,
            content_size,
            position,
            phase1_scrollbar_input_style(),
        );
        self.floating_debug_scroll = state.offset();
        self.set_status(format!(
            "MEMORY SCROLLBAR DRAG Y {}",
            self.floating_debug_scroll.y.round() as i32
        ));
        changed
    }

    fn floating_dock_scroll(&self, kind: FloatingDockKind) -> Point {
        self.floating_dock_scrolls
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map(|(_, scroll)| *scroll)
            .unwrap_or_default()
    }

    fn set_floating_dock_scroll(&mut self, kind: FloatingDockKind, scroll: Point) {
        if let Some((_, existing)) = self
            .floating_dock_scrolls
            .iter_mut()
            .find(|(candidate, _)| *candidate == kind)
        {
            *existing = scroll;
        } else {
            self.floating_dock_scrolls.push((kind, scroll));
        }
    }

    /// Build the current draw list.
    pub fn draw_list(&self) -> DrawList {
        let layout = self.layout();
        let theme = Phase1Theme::dark();
        let mut list = DrawList::new(theme.background);

        list.fill_rect(layout.root, theme.background);
        list.fill_radial_gradient_rect(
            layout.root,
            Point::new(layout.root.x + layout.root.width * 0.52, layout.root.y),
            layout.root.width.max(layout.root.height) * 0.82,
            Color::rgba8(78, 108, 156, 58),
            Color::rgba8(78, 108, 156, 0),
        );
        draw_panel(
            &mut list,
            layout.menu_bar,
            theme.panel_style(theme.menu, theme.status),
        );

        draw_panel(
            &mut list,
            layout.tool_bar,
            theme.panel_style(theme.panel, theme.status),
        );

        let left_splitter_color = self.splitter_color(WidgetId::new(8), &theme);
        let right_splitter_color = self.splitter_color(WidgetId::new(9), &theme);
        let bottom_splitter_color = self.splitter_color(BOTTOM_SPLITTER_ID, &theme);
        list.fill_rect(layout.left_splitter, left_splitter_color);
        list.fill_rect(layout.right_splitter, right_splitter_color);

        list.fill_rect(layout.canvas, theme.canvas);
        draw_canvas_grid(
            &mut list,
            layout.canvas,
            theme.grid_minor,
            theme.grid_major,
            self.canvas_pan,
            self.canvas_zoom,
        );
        draw_canvas_objects(&mut list, layout.canvas, &theme, self);
        draw_grid_input(&mut list, layout.canvas, &theme, self);
        list.stroke_rect(layout.canvas, 1.0, theme.border);

        draw_panel(
            &mut list,
            layout.inspector,
            theme.panel_style(theme.panel, theme.background),
        );

        draw_panel(
            &mut list,
            layout.timeline,
            theme.panel_style(theme.timeline, theme.status),
        );
        draw_timeline(&mut list, layout.timeline, &theme, self);
        list.fill_rect(bottom_splitter_rect(layout), bottom_splitter_color);

        draw_panel(
            &mut list,
            layout.status_bar,
            theme.panel_style(theme.status, theme.background),
        );

        draw_toolbar(&mut list, layout.tool_bar, &theme, self);
        draw_inspector(&mut list, layout.inspector, &theme, self);
        draw_rich_controls(&mut list, layout, &theme, self);
        draw_status(&mut list, layout.status_bar, &theme, &self.status_text);
        draw_canvas_label(&mut list, layout.canvas, &theme, self);
        draw_menu(&mut list, layout.menu_bar, &theme, self);
        draw_context_menu(&mut list, &theme, self);
        draw_floating_dock_windows(&mut list, &theme, self);

        list
    }

    fn handle_pointer_move(&mut self, position: Point) -> UiResponse {
        self.last_pointer = position;

        if let Some(drag) = self.drag {
            match drag {
                ActiveDrag::LeftSplitter {
                    start_x,
                    start_left_width,
                } => {
                    let delta = position.x - start_x;
                    self.left_width = clamp_panel_extent(
                        start_left_width + delta,
                        48.0_f32,
                        self.size.width * 0.45_f32,
                    );
                    self.set_status(format!(
                        "STATUS DRAG LEFT {} TOOL {}",
                        self.left_width.round() as i32,
                        self.selected_tool.label()
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::RightSplitter {
                    start_x,
                    start_right_width,
                } => {
                    let delta = position.x - start_x;
                    self.right_width = clamp_panel_extent(
                        start_right_width - delta,
                        120.0_f32,
                        self.size.width * 0.55_f32,
                    );
                    self.sync_root_split_ratio_from_legacy_layout();
                    self.set_status(format!(
                        "STATUS DRAG RIGHT {} ROOT {} TOOL {}",
                        self.right_width.round() as i32,
                        (self.dock_layout.root_split_ratio() * 100.0).round() as i32,
                        self.selected_tool.label()
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::BottomSplitter {
                    start_y,
                    start_bottom_height,
                } => {
                    let delta = position.y - start_y;
                    let layout = self.layout();
                    let content_h = layout.canvas.height + layout.timeline.height;
                    self.bottom_height = clamp_panel_extent(
                        start_bottom_height - delta,
                        48.0_f32,
                        (content_h * 0.55_f32).max(48.0_f32),
                    );
                    self.sync_main_split_ratio_from_legacy_layout();
                    self.set_status(format!(
                        "STATUS DRAG BOTTOM {} MAIN {} TOOL {}",
                        self.bottom_height.round() as i32,
                        (self.dock_layout.main_split_ratio() * 100.0).round() as i32,
                        self.selected_tool.label()
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::CanvasPan { start, start_pan } => {
                    let delta_x = position.x - start.x;
                    let delta_y = position.y - start.y;
                    self.canvas_pan = Point::new(start_pan.x + delta_x, start_pan.y + delta_y);
                    self.set_status(format!(
                        "STATUS PAN X {} Y {} Z {}",
                        self.canvas_pan.x.round() as i32,
                        self.canvas_pan.y.round() as i32,
                        (self.canvas_zoom * 100.0).round() as i32
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::CanvasObjectMove {
                    id,
                    start,
                    start_rect,
                } => {
                    let dx = (position.x - start.x) / self.canvas_zoom.max(0.01_f32);
                    let dy = (position.y - start.y) / self.canvas_zoom.max(0.01_f32);
                    if let Some(object) = self.object_mut(id) {
                        object.rect = Rect::new(
                            start_rect.x + dx,
                            start_rect.y + dy,
                            start_rect.width,
                            start_rect.height,
                        );
                    }
                    self.set_status(format!(
                        "STATUS MOVE OBJECT {} DX {} DY {}",
                        id.raw(),
                        dx.round() as i32,
                        dy.round() as i32
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::CanvasObjectResize {
                    id,
                    handle,
                    start,
                    start_rect,
                } => {
                    let dx = (position.x - start.x) / self.canvas_zoom.max(0.01_f32);
                    let dy = (position.y - start.y) / self.canvas_zoom.max(0.01_f32);
                    let resized = resize_rect_from_handle(start_rect, handle, dx, dy);
                    if let Some(object) = self.object_mut(id) {
                        object.rect = resized;
                    }
                    self.set_status(format!(
                        "STATUS RESIZE OBJECT {} W {} H {}",
                        id.raw(),
                        resized.width.round() as i32,
                        resized.height.round() as i32
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::GridPaint { last_index, value } => {
                    if let Some(index) = self.grid_cell_at_screen(position) {
                        if last_index != Some(index) {
                            self.grid_input.set_index(index, value);
                            self.drag = Some(ActiveDrag::GridPaint {
                                last_index: Some(index),
                                value,
                            });
                            self.set_status(format!(
                                "GRID PAINT INDEX {} VALUE {} ACTIVE {}",
                                index,
                                value,
                                self.grid_input.active_count()
                            ));
                            return UiResponse::redraw();
                        }
                    }
                    return UiResponse::none();
                }
                ActiveDrag::Slider {
                    start_x,
                    start_value,
                } => {
                    let slider = self.slider_rect(self.layout());
                    let delta = if slider.width > 0.0 {
                        (position.x - start_x) / slider.width
                    } else {
                        0.0
                    };
                    self.slider_value = (start_value + delta).clamp(0.0, 1.0);
                    self.set_status(format!(
                        "SLIDER OPACITY {}",
                        (self.slider_value * 100.0).round() as i32
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::FloatingWindow { start, start_rect } => {
                    let dx = position.x - start.x;
                    let dy = position.y - start.y;
                    self.floating_debug.rect = Rect::new(
                        (start_rect.x + dx).max(0.0),
                        (start_rect.y + dy).max(0.0),
                        start_rect.width,
                        start_rect.height,
                    );
                    self.set_status(format!(
                        "FLOAT MEMORY X {} Y {}",
                        self.floating_debug.rect.x.round() as i32,
                        self.floating_debug.rect.y.round() as i32
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::FloatingDockWindow {
                    kind,
                    start,
                    start_rect,
                } => {
                    let dx = position.x - start.x;
                    let dy = position.y - start.y;
                    let new_rect = Rect::new(
                        (start_rect.x + dx).max(0.0),
                        (start_rect.y + dy).max(0.0),
                        start_rect.width,
                        start_rect.height,
                    );
                    self.set_floating_dock_rect(kind, new_rect);
                    self.set_status(format!(
                        "FLOAT DOCK {} X {} Y {}",
                        kind.label(),
                        new_rect.x.round() as i32,
                        new_rect.y.round() as i32
                    ));
                    return UiResponse::redraw();
                }
                ActiveDrag::Scrollbar {
                    area,
                    axis,
                    start,
                    start_scroll,
                } => {
                    self.drag_scrollbar_to(area, axis, start, start_scroll, position);
                    return UiResponse::redraw();
                }
            }
        }

        let hit = self
            .widget_tree()
            .hit_test(position)
            .map(|node| (node.id, node.label));
        let next_hover = hit.map(|(id, _)| id);
        let hover_changed = self.hovered_widget != next_hover;
        self.hovered_widget = next_hover;

        let label = hit.map(|(_, label)| label).unwrap_or("NONE");
        self.set_status(format!(
            "STATUS HOVER {} X {} Y {} TOOL {}",
            label,
            position.x.round() as i32,
            position.y.round() as i32,
            self.selected_tool.label()
        ));

        if hover_changed {
            UiResponse::redraw()
        } else {
            UiResponse {
                needs_redraw: true,
                status_changed: true,
            }
        }
    }

    fn handle_pointer_down(&mut self, position: Point, button: PointerButton) -> UiResponse {
        self.last_pointer = position;
        if button == PointerButton::Secondary {
            self.context_menu = Some(ContextMenuState { position });
            self.active_menu = None;
            self.set_status(format!(
                "CONTEXT MENU X {} Y {}",
                position.x.round() as i32,
                position.y.round() as i32
            ));
            return UiResponse::redraw();
        }
        if button != PointerButton::Primary {
            self.set_status(format!(
                "STATUS POINTER OTHER TOOL {}",
                self.selected_tool.label()
            ));
            return UiResponse::redraw();
        }

        if let Some(action) = self.context_action_at(position) {
            self.invoke_context_action(action, position);
            return UiResponse::redraw();
        } else if self.context_menu.is_some() {
            self.context_menu = None;
        }

        if let Some(command) = self.menu_command_at(position) {
            self.invoke_menu_command(command);
            return UiResponse::redraw();
        }

        if self.begin_scrollbar_drag_at(position) {
            return UiResponse::redraw();
        }

        let hit = self
            .widget_tree()
            .hit_test(position)
            .map(|node| (node.id, node.role, node.label));
        let Some((id, role, label)) = hit else {
            self.active_widget = None;
            self.captured_widget = None;
            return UiResponse::none();
        };

        self.active_widget = Some(id);
        self.captured_widget = Some(id);
        if !matches!(role, WidgetRole::TextInput) {
            self.text_input.focused = false;
            self.text_input.ime_preedit.clear();
        }
        if !matches!(role, WidgetRole::MemoryEditor) {
            self.memory_editor.focused = false;
        }
        match role {
            WidgetRole::DockTabClose(panel) => {
                self.set_dock_panel_tab_open(panel, false);
                self.active_menu = None;
            }
            WidgetRole::InspectorTab(panel) => {
                self.set_active_inspector_panel(panel);
                self.active_menu = None;
                self.set_status(format!("DOCK INSPECTOR TAB {}", panel.label()));
            }
            WidgetRole::BottomTab(panel) => {
                self.set_active_bottom_panel(panel);
                self.active_menu = None;
                self.set_status(format!("DOCK BOTTOM TAB {}", panel.label()));
            }
            WidgetRole::TextInput => {
                self.text_input.focused = true;
                self.memory_editor.focused = false;
                self.text_input.cursor = self.text_input.text.len();
                self.text_input.normalize_cursor();
                self.set_status(format!("TEXT INPUT FOCUS {}", self.text_input.text));
            }
            WidgetRole::MemoryEditor => {
                self.active_menu = None;
                self.memory_editor.focused = true;
                if let Some(offset) = self.memory_editor_offset_at(position) {
                    self.memory_editor
                        .select_offset(offset, self.memory_viewer.bytes.len());
                }
                self.set_status(format!(
                    "MEM EDIT FOCUS {:04X}",
                    self.memory_selected_address()
                ));
            }
            WidgetRole::FloatingClose => {
                self.floating_debug.open = false;
                self.set_status("FLOAT MEMORY CLOSED".to_string());
            }
            WidgetRole::FloatingTitle => {
                self.drag = Some(ActiveDrag::FloatingWindow {
                    start: position,
                    start_rect: self.floating_debug.rect,
                });
                self.set_status("FLOAT MEMORY DRAG".to_string());
            }
            WidgetRole::FloatingDockClose(kind) => {
                self.close_floating_dock(kind);
            }
            WidgetRole::FloatingDockTitle(kind) => {
                if let Some(rect) = self.floating_dock_rect(kind) {
                    self.drag = Some(ActiveDrag::FloatingDockWindow {
                        kind,
                        start: position,
                        start_rect: rect,
                    });
                    self.set_status(format!("FLOAT DOCK DRAG {}", kind.label()));
                }
            }
            WidgetRole::Slider => {
                self.drag = Some(ActiveDrag::Slider {
                    start_x: position.x,
                    start_value: self.slider_value,
                });
                self.update_slider_from_position(position);
            }
            WidgetRole::RichButton => {
                if id == RICH_APPLY_ID {
                    self.floating_debug.open = true;
                    self.set_status(format!(
                        "RICH BUTTON APPLY GRID {}",
                        self.grid_input.active_count()
                    ));
                } else if id == RICH_RESET_ID {
                    self.grid_input = GridInput::new(self.grid_input.config.clone());
                    self.set_status("RICH BUTTON RESET GRID".to_string());
                }
            }
            WidgetRole::MenuItem(menu) => {
                self.set_status(format!("STATUS PRESS MENU {}", menu.label()));
            }
            WidgetRole::Splitter(SplitterKind::Left) => {
                self.drag = Some(ActiveDrag::LeftSplitter {
                    start_x: position.x,
                    start_left_width: self.left_width,
                });
                self.set_status("STATUS CAPTURE LEFT SPLITTER".to_string());
            }
            WidgetRole::Splitter(SplitterKind::Right) => {
                self.drag = Some(ActiveDrag::RightSplitter {
                    start_x: position.x,
                    start_right_width: self.right_width,
                });
                self.set_status("STATUS CAPTURE RIGHT SPLITTER".to_string());
            }
            WidgetRole::Splitter(SplitterKind::Bottom) => {
                self.drag = Some(ActiveDrag::BottomSplitter {
                    start_y: position.y,
                    start_bottom_height: self.bottom_height,
                });
                self.set_status("STATUS CAPTURE BOTTOM SPLITTER".to_string());
            }
            WidgetRole::ToolButton(tool) => {
                self.active_menu = None;
                self.set_status(format!("STATUS PRESS TOOL {}", tool.label()));
            }
            WidgetRole::Canvas if self.selected_tool == ToolKind::Select => {
                self.active_menu = None;
                if let Some((object_id, handle)) = self.hit_test_selected_handle(position) {
                    self.select_only_object(object_id);
                    if let Some(start_rect) = self.object_rect(object_id) {
                        self.drag = Some(ActiveDrag::CanvasObjectResize {
                            id: object_id,
                            handle,
                            start: position,
                            start_rect,
                        });
                    }
                    self.set_status(format!(
                        "STATUS RESIZE HANDLE {:?} OBJECT {}",
                        handle,
                        object_id.raw()
                    ));
                } else if let Some(object_id) = self.hit_test_object(position) {
                    self.select_only_object(object_id);
                    if let Some(start_rect) = self.object_rect(object_id) {
                        self.drag = Some(ActiveDrag::CanvasObjectMove {
                            id: object_id,
                            start: position,
                            start_rect,
                        });
                    }
                    self.set_status(format!("STATUS SELECT OBJECT {}", object_id.raw()));
                } else {
                    self.clear_object_selection();
                    self.set_status("STATUS CANVAS CLEAR SELECTION".to_string());
                }
            }
            WidgetRole::Canvas if self.selected_tool == ToolKind::Box => {
                self.active_menu = None;
                let world = self.screen_to_world(position);
                let id = ObjectId::new(self.next_object_id);
                self.next_object_id = self.next_object_id.saturating_add(1);
                let object = EditableRect::new(
                    id,
                    Rect::new(world.x - 56.0, world.y - 36.0, 112.0, 72.0),
                    "RECT",
                );
                self.apply_command(CanvasCommand::CreateRect { object });
                self.select_only_object(id);
                self.set_status(format!("STATUS CREATE RECT {}", id.raw()));
            }
            WidgetRole::Canvas if self.selected_tool == ToolKind::Grid => {
                self.active_menu = None;
                if let Some(index) = self.grid_cell_at_screen(position) {
                    let new_value = self.grid_input.toggle_index(index).unwrap_or(0);
                    self.drag = Some(ActiveDrag::GridPaint {
                        last_index: Some(index),
                        value: new_value,
                    });
                    self.set_status(format!(
                        "GRID TOGGLE INDEX {} VALUE {} ACTIVE {}",
                        index,
                        new_value,
                        self.grid_input.active_count()
                    ));
                } else {
                    self.set_status("GRID CLICK OUTSIDE INPUT".to_string());
                }
            }
            WidgetRole::Canvas if self.selected_tool == ToolKind::Pan => {
                self.active_menu = None;
                self.drag = Some(ActiveDrag::CanvasPan {
                    start: position,
                    start_pan: self.canvas_pan,
                });
                self.set_status("STATUS CAPTURE CANVAS PAN".to_string());
            }
            _ => {
                self.active_menu = None;
                self.set_status(format!(
                    "STATUS PRESS {} TOOL {}",
                    label,
                    self.selected_tool.label()
                ));
            }
        }
        UiResponse::redraw()
    }

    fn handle_pointer_up(&mut self, position: Point, button: PointerButton) -> UiResponse {
        self.last_pointer = position;
        if button != PointerButton::Primary {
            return UiResponse::none();
        }

        if let Some(ActiveDrag::Scrollbar { area, .. }) = self.drag {
            let scroll = self.scroll_area_offset(area);
            self.set_status(format!(
                "SCROLLBAR RELEASE {} X {} Y {}",
                area.label(),
                scroll.x.round() as i32,
                scroll.y.round() as i32
            ));
            self.active_widget = None;
            self.captured_widget = None;
            self.drag = None;
            return UiResponse::redraw();
        }

        let captured = self.captured_widget;
        let hit_id = self.widget_tree().hit_test(position).map(|node| node.id);
        if let Some(id) = captured {
            if let Some(tool) = ToolKind::from_widget_id(id) {
                if hit_id == Some(id) {
                    self.selected_tool = tool;
                    self.active_menu = None;
                    self.set_status(format!("STATUS SELECT TOOL {}", tool.label()));
                }
            } else if let Some(menu) = MenuKind::from_widget_id(id) {
                if hit_id == Some(id) {
                    self.active_menu = if self.active_menu == Some(menu) {
                        None
                    } else {
                        Some(menu)
                    };
                    self.set_status(format!("STATUS MENU {}", menu.label()));
                }
            } else if id == WidgetId::new(8) || id == WidgetId::new(9) || id == BOTTOM_SPLITTER_ID {
                self.set_status(format!(
                    "STATUS SPLITTER RELEASE ROOT {} MAIN {} TOOL {}",
                    (self.dock_layout.root_split_ratio() * 100.0).round() as i32,
                    (self.dock_layout.main_split_ratio() * 100.0).round() as i32,
                    self.selected_tool.label()
                ));
            } else if id == SLIDER_ID {
                self.set_status(format!(
                    "SLIDER OPACITY {}",
                    (self.slider_value * 100.0).round() as i32
                ));
            } else if let Some(kind) = self.floating_dock_kind_from_title_id(id) {
                self.set_status(format!("FLOAT DOCK RELEASE {}", kind.label()));
            } else if id == WidgetId::new(4) {
                self.set_status(format!(
                    "STATUS CANVAS RELEASE Z {}",
                    (self.canvas_zoom * 100.0).round() as i32
                ));
            }
        }

        if let Some(ActiveDrag::CanvasObjectMove { id, start_rect, .. }) = self.drag {
            if let Some(to) = self.object_rect(id) {
                if to != start_rect {
                    self.command_history.record_done(CanvasCommand::MoveRect {
                        id,
                        from: start_rect,
                        to,
                    });
                    self.set_status(format!(
                        "STATUS MOVE COMMIT {} UNDO {}",
                        id.raw(),
                        self.undo_len()
                    ));
                }
            }
        }

        if let Some(ActiveDrag::CanvasObjectResize { id, start_rect, .. }) = self.drag {
            if let Some(to) = self.object_rect(id) {
                if to != start_rect {
                    self.command_history.record_done(CanvasCommand::ResizeRect {
                        id,
                        from: start_rect,
                        to,
                    });
                    self.set_status(format!(
                        "STATUS RESIZE COMMIT {} UNDO {}",
                        id.raw(),
                        self.undo_len()
                    ));
                }
            }
        }

        self.active_widget = None;

        self.captured_widget = None;
        self.drag = None;
        UiResponse::redraw()
    }

    fn apply_command(&mut self, command: CanvasCommand) {
        self.apply_command_effect(&command, true);
        self.command_history.record_done(command);
    }

    fn undo_last_command(&mut self) {
        let Some(command) = self.command_history.undo_stack.pop() else {
            self.set_status("COMMAND UNDO EMPTY".to_string());
            return;
        };
        self.apply_command_effect(&command, false);
        self.command_history.redo_stack.push(command);
        self.set_status(format!(
            "COMMAND UNDO OK UNDO {} REDO {}",
            self.undo_len(),
            self.redo_len()
        ));
    }

    fn redo_last_command(&mut self) {
        let Some(command) = self.command_history.redo_stack.pop() else {
            self.set_status("COMMAND REDO EMPTY".to_string());
            return;
        };
        self.apply_command_effect(&command, true);
        self.command_history.undo_stack.push(command);
        self.set_status(format!(
            "COMMAND REDO OK UNDO {} REDO {}",
            self.undo_len(),
            self.redo_len()
        ));
    }

    fn apply_command_effect(&mut self, command: &CanvasCommand, forward: bool) {
        match (command, forward) {
            (CanvasCommand::CreateRect { object }, true) => {
                self.objects.retain(|item| item.id != object.id);
                self.objects.push(object.clone());
                self.select_only_object(object.id);
            }
            (CanvasCommand::CreateRect { object }, false) => {
                self.objects.retain(|item| item.id != object.id);
                if self.selected_object == Some(object.id) {
                    self.clear_object_selection();
                }
            }
            (CanvasCommand::DeleteRect { object }, true) => {
                self.objects.retain(|item| item.id != object.id);
                if self.selected_object == Some(object.id) {
                    self.clear_object_selection();
                }
            }
            (CanvasCommand::DeleteRect { object }, false) => {
                self.objects.retain(|item| item.id != object.id);
                self.objects.push(object.clone());
                self.select_only_object(object.id);
            }
            (CanvasCommand::MoveRect { id, from: _, to }, true)
            | (CanvasCommand::ResizeRect { id, from: _, to }, true) => {
                if let Some(object) = self.object_mut(*id) {
                    object.rect = *to;
                }
                self.select_only_object(*id);
            }
            (CanvasCommand::MoveRect { id, from, to: _ }, false)
            | (CanvasCommand::ResizeRect { id, from, to: _ }, false) => {
                if let Some(object) = self.object_mut(*id) {
                    object.rect = *from;
                }
                self.select_only_object(*id);
            }
        }
    }

    fn object_mut(&mut self, id: ObjectId) -> Option<&mut EditableRect> {
        self.objects.iter_mut().find(|object| object.id == id)
    }

    fn object_rect(&self, id: ObjectId) -> Option<Rect> {
        self.objects
            .iter()
            .find(|object| object.id == id)
            .map(|object| object.rect)
    }

    fn delete_selected_object(&mut self) {
        let Some(id) = self.selected_object else {
            self.set_status("COMMAND DELETE EMPTY".to_string());
            return;
        };
        let Some(object) = self.objects.iter().find(|object| object.id == id).cloned() else {
            self.clear_object_selection();
            self.set_status("COMMAND DELETE MISSING".to_string());
            return;
        };
        self.apply_command(CanvasCommand::DeleteRect { object });
        self.set_status(format!(
            "COMMAND DELETE OBJECT {} UNDO {}",
            id.raw(),
            self.undo_len()
        ));
    }

    fn copy_selected_object(&mut self) {
        let Some(id) = self.selected_object else {
            self.clipboard.clear();
            self.set_status("COMMAND COPY EMPTY".to_string());
            return;
        };
        let Some(object) = self.objects.iter().find(|object| object.id == id).cloned() else {
            self.clear_object_selection();
            self.clipboard.clear();
            self.set_status("COMMAND COPY MISSING".to_string());
            return;
        };
        self.clipboard.clear();
        self.clipboard.push(object);
        self.set_status(format!(
            "COMMAND COPY OBJECT {} CLIP {}",
            id.raw(),
            self.clipboard_len()
        ));
    }

    fn cut_selected_object(&mut self) {
        let Some(id) = self.selected_object else {
            self.clipboard.clear();
            self.set_status("COMMAND CUT EMPTY".to_string());
            return;
        };
        let Some(object) = self.objects.iter().find(|object| object.id == id).cloned() else {
            self.clear_object_selection();
            self.clipboard.clear();
            self.set_status("COMMAND CUT MISSING".to_string());
            return;
        };
        self.clipboard.clear();
        self.clipboard.push(object.clone());
        self.apply_command(CanvasCommand::DeleteRect { object });
        self.set_status(format!(
            "COMMAND CUT OBJECT {} CLIP {} UNDO {}",
            id.raw(),
            self.clipboard_len(),
            self.undo_len()
        ));
    }

    fn paste_clipboard(&mut self) {
        let Some(source) = self.clipboard.first().cloned() else {
            self.set_status("COMMAND PASTE EMPTY".to_string());
            return;
        };
        let new_id = ObjectId::new(self.next_object_id);
        self.next_object_id = self.next_object_id.saturating_add(1);
        let mut object = source;
        object.id = new_id;
        object.rect = Rect::new(
            object.rect.x + 32.0,
            object.rect.y + 32.0,
            object.rect.width,
            object.rect.height,
        );
        object.label = "RECT";
        self.apply_command(CanvasCommand::CreateRect { object });
        self.select_only_object(new_id);
        self.set_status(format!(
            "COMMAND PASTE OBJECT {} CLIP {} UNDO {}",
            new_id.raw(),
            self.clipboard_len(),
            self.undo_len()
        ));
    }

    fn duplicate_selected_object(&mut self) {
        let Some(id) = self.selected_object else {
            self.set_status("COMMAND DUPLICATE EMPTY".to_string());
            return;
        };
        let Some(source) = self.objects.iter().find(|object| object.id == id).cloned() else {
            self.clear_object_selection();
            self.set_status("COMMAND DUPLICATE MISSING".to_string());
            return;
        };
        let new_id = ObjectId::new(self.next_object_id);
        self.next_object_id = self.next_object_id.saturating_add(1);
        let mut object = source;
        object.id = new_id;
        object.rect = Rect::new(
            object.rect.x + 24.0,
            object.rect.y + 24.0,
            object.rect.width,
            object.rect.height,
        );
        object.label = "RECT";
        self.apply_command(CanvasCommand::CreateRect { object });
        self.select_only_object(new_id);
        self.set_status(format!(
            "COMMAND DUPLICATE OBJECT {} UNDO {}",
            new_id.raw(),
            self.undo_len()
        ));
    }

    fn nudge_selected_object(&mut self, dx: f32, dy: f32) {
        let Some(id) = self.selected_object else {
            self.set_status("COMMAND NUDGE EMPTY".to_string());
            return;
        };
        let Some(from) = self.object_rect(id) else {
            self.clear_object_selection();
            self.set_status("COMMAND NUDGE MISSING".to_string());
            return;
        };
        let to = Rect::new(from.x + dx, from.y + dy, from.width, from.height);
        self.apply_command_effect(&CanvasCommand::MoveRect { id, from, to }, true);
        self.command_history
            .record_done(CanvasCommand::MoveRect { id, from, to });
        self.set_status(format!(
            "COMMAND NUDGE OBJECT {} X {} Y {}",
            id.raw(),
            to.x.round() as i32,
            to.y.round() as i32
        ));
    }

    fn hit_test_selected_handle(&self, screen_point: Point) -> Option<(ObjectId, ResizeHandle)> {
        let id = self.selected_object?;
        let rect = self.object_rect(id)?;
        let screen = self.world_rect_to_screen(rect);
        for (handle, handle_rect) in selection_handle_rects(screen) {
            if handle_rect.contains(screen_point) {
                return Some((id, handle));
            }
        }
        None
    }

    fn hit_test_object(&self, screen_point: Point) -> Option<ObjectId> {
        let layout = self.layout();
        if !layout.canvas.contains(screen_point) {
            return None;
        }
        self.objects
            .iter()
            .rev()
            .find(|object| {
                self.world_rect_to_screen(object.rect)
                    .contains(screen_point)
            })
            .map(|object| object.id)
    }

    fn screen_to_world(&self, point: Point) -> Point {
        let layout = self.layout();
        let origin = self.canvas_world_origin_screen(layout.canvas);
        Point::new(
            (point.x - origin.x) / self.canvas_zoom.max(0.01_f32),
            (point.y - origin.y) / self.canvas_zoom.max(0.01_f32),
        )
    }

    fn canvas_world_origin_screen(&self, canvas: Rect) -> Point {
        Point::new(
            canvas.x + canvas.width * 0.18_f32 + self.canvas_pan.x,
            canvas.y + canvas.height * 0.14_f32 + self.canvas_pan.y,
        )
    }

    fn world_rect_to_screen(&self, rect: Rect) -> Rect {
        let layout = self.layout();
        let origin = self.canvas_world_origin_screen(layout.canvas);
        Rect::new(
            origin.x + rect.x * self.canvas_zoom,
            origin.y + rect.y * self.canvas_zoom,
            rect.width * self.canvas_zoom,
            rect.height * self.canvas_zoom,
        )
    }

    fn visual_state_for(&self, id: WidgetId) -> WidgetVisualState {
        if self.active_widget == Some(id) || self.captured_widget == Some(id) {
            WidgetVisualState::Active
        } else if self.hovered_widget == Some(id) {
            WidgetVisualState::Hovered
        } else {
            WidgetVisualState::Normal
        }
    }

    fn visual_state_for_tool(&self, tool: ToolKind) -> WidgetVisualState {
        let id = tool.widget_id();
        if self.active_widget == Some(id) || self.captured_widget == Some(id) {
            WidgetVisualState::Active
        } else if self.selected_tool == tool {
            WidgetVisualState::Selected
        } else if self.hovered_widget == Some(id) {
            WidgetVisualState::Hovered
        } else {
            WidgetVisualState::Normal
        }
    }

    fn visual_state_for_menu(&self, menu: MenuKind) -> WidgetVisualState {
        let id = menu.widget_id();
        if self.active_widget == Some(id) || self.captured_widget == Some(id) {
            WidgetVisualState::Active
        } else if self.active_menu == Some(menu) {
            WidgetVisualState::Selected
        } else if self.hovered_widget == Some(id) {
            WidgetVisualState::Hovered
        } else {
            WidgetVisualState::Normal
        }
    }

    fn visual_state_for_inspector_panel(&self, panel: InspectorPanelKind) -> WidgetVisualState {
        let id = panel.widget_id();
        if self.active_widget == Some(id) || self.captured_widget == Some(id) {
            WidgetVisualState::Active
        } else if self.active_inspector_panel == panel {
            WidgetVisualState::Selected
        } else if self.hovered_widget == Some(id) {
            WidgetVisualState::Hovered
        } else {
            WidgetVisualState::Normal
        }
    }

    fn visual_state_for_bottom_panel(&self, panel: BottomPanelKind) -> WidgetVisualState {
        let id = panel.widget_id();
        if self.active_widget == Some(id) || self.captured_widget == Some(id) {
            WidgetVisualState::Active
        } else if self.active_bottom_panel == panel {
            WidgetVisualState::Selected
        } else if self.hovered_widget == Some(id) {
            WidgetVisualState::Hovered
        } else {
            WidgetVisualState::Normal
        }
    }

    fn inspector_tab_rects(&self, layout: CreativeLayout) -> Vec<(InspectorPanelKind, Rect)> {
        let panels = self.dock_layout.inspector_tabs();
        let x = layout.inspector.x + 10.0;
        let y = layout.inspector.y + 42.0;
        let gap = 4.0;
        let count = panels.len().max(1) as f32;
        let total_gap = gap * (count - 1.0).max(0.0);
        let w = ((layout.inspector.width - 20.0 - total_gap) / count).max(42.0);
        panels
            .into_iter()
            .enumerate()
            .map(|(index, panel)| (panel, Rect::new(x + (w + gap) * index as f32, y, w, 26.0)))
            .collect()
    }

    fn bottom_tab_rects(&self, layout: CreativeLayout) -> Vec<(BottomPanelKind, Rect)> {
        let panels = self.dock_layout.bottom_tabs();
        let mut x = layout.timeline.x + 8.0;
        let y = layout.timeline.y + 4.0;
        let gap = 4.0;
        panels
            .into_iter()
            .map(|panel| {
                let w = match panel {
                    BottomPanelKind::Timeline => 118.0,
                    BottomPanelKind::Log => 70.0,
                    BottomPanelKind::Diagnostics => 76.0,
                };
                let rect = Rect::new(x, y, w, 22.0);
                x += w + gap;
                (panel, rect)
            })
            .collect()
    }

    fn menu_item_rects(&self, layout: CreativeLayout) -> [(MenuKind, Rect); 5] {
        let y = layout.menu_bar.y;
        let h = layout.menu_bar.height;
        [
            (
                MenuKind::File,
                Rect::new(layout.menu_bar.x + 10.0, y, 54.0, h),
            ),
            (
                MenuKind::Edit,
                Rect::new(layout.menu_bar.x + 66.0, y, 54.0, h),
            ),
            (
                MenuKind::View,
                Rect::new(layout.menu_bar.x + 122.0, y, 62.0, h),
            ),
            (
                MenuKind::Window,
                Rect::new(layout.menu_bar.x + 186.0, y, 88.0, h),
            ),
            (
                MenuKind::Help,
                Rect::new(layout.menu_bar.x + 276.0, y, 58.0, h),
            ),
        ]
    }

    fn handle_key_down(
        &mut self,
        key: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) -> UiResponse {
        let input = KeyInput::from_legacy(key, ctrl, shift, alt, meta);
        self.handle_key_input(&input)
    }

    fn handle_key_input(&mut self, input: &KeyInput) -> UiResponse {
        // Shortcut cleanup Phase 2 v2.1:
        // - ordinary commands use only the logical key, so Ctrl+C/Ctrl+V are not
        //   affected by physical-key metadata;
        // - Ctrl+Shift+number uses the normalized digit resolver, so Windows/JIS
        //   shifted symbols, native codes, scan codes, and release-fallback zero
        //   all go through one path.
        let upper = input.logical_upper();
        let logical_key = input.logical.as_str();
        let shortcut_digit = input.shortcut_digit();
        let ctrl = input.ctrl;
        let shift = input.shift;
        let alt = input.alt;
        let meta = input.meta;
        if self.memory_editor.focused && input.text_editing_mode() {
            if let Some(response) = self.handle_memory_editor_key(logical_key, &upper) {
                return response;
            }
            self.set_status(format!("MEM EDIT KEY {}", upper));
            return UiResponse::redraw();
        }
        if self.text_input.focused && input.text_editing_mode() {
            if let Some(response) = self.handle_text_input_key(logical_key, &upper, shift) {
                return response;
            }
        }
        if upper == "ESCAPE" {
            self.active_widget = None;
            self.captured_widget = None;
            self.active_menu = None;
            self.drag = None;
            self.set_status(format!("STATUS ESC TOOL {}", self.selected_tool.label()));
            return UiResponse::redraw();
        }

        if ctrl && upper == "S" {
            let snapshot_len = self.snapshot_text().len();
            self.set_status(format!(
                "COMMAND SAVE SNAPSHOT {} BYTES OBJ {}",
                snapshot_len,
                self.objects.len()
            ));
            return UiResponse::redraw();
        }
        if ctrl && upper == "O" {
            self.reset_demo_document();
            return UiResponse::redraw();
        }
        if ctrl && upper == "C" {
            self.copy_selected_object();
            return UiResponse::redraw();
        }
        if ctrl && upper == "X" {
            self.cut_selected_object();
            return UiResponse::redraw();
        }
        if ctrl && upper == "V" {
            self.paste_clipboard();
            return UiResponse::redraw();
        }
        if ctrl && upper == "D" {
            self.duplicate_selected_object();
            return UiResponse::redraw();
        }
        if ctrl && upper == "A" {
            self.selected_objects = self.objects.iter().map(|object| object.id).collect();
            self.selected_object = self.selected_objects.last().copied();
            self.set_status(format!(
                "COMMAND SELECT ALL {}",
                self.selected_objects.len()
            ));
            return UiResponse::redraw();
        }
        if ctrl && shift && shortcut_digit == Some(1) {
            self.open_floating_dock(FloatingDockKind::Inspector(InspectorPanelKind::Properties));
            return UiResponse::redraw();
        }
        if ctrl && shift && shortcut_digit == Some(2) {
            self.open_floating_dock(FloatingDockKind::Inspector(InspectorPanelKind::Layers));
            return UiResponse::redraw();
        }
        if ctrl && shift && shortcut_digit == Some(3) {
            self.open_floating_dock(FloatingDockKind::Inspector(InspectorPanelKind::Memory));
            return UiResponse::redraw();
        }
        if ctrl && shift && shortcut_digit == Some(4) {
            self.open_floating_dock(FloatingDockKind::Bottom(BottomPanelKind::Timeline));
            return UiResponse::redraw();
        }
        if ctrl && shift && shortcut_digit == Some(5) {
            self.open_floating_dock(FloatingDockKind::Bottom(BottomPanelKind::Log));
            return UiResponse::redraw();
        }
        if ctrl && shift && shortcut_digit == Some(6) {
            self.open_floating_dock(FloatingDockKind::Bottom(BottomPanelKind::Diagnostics));
            return UiResponse::redraw();
        }
        if ctrl && shortcut_digit == Some(0) {
            // Ctrl+Shift+0 can arrive as `)`, `0`, an unidentified logical key with
            // `PhysicalKey::Code(Digit0)`, or even with an unreliable shift flag on
            // some layouts.  Ctrl+0 is otherwise unused in plita, so we reserve it
            // as the robust close-all-floating-panels shortcut.
            self.close_all_floating_docks();
            return UiResponse::redraw();
        }
        if ctrl && !shift && shortcut_digit == Some(1) {
            self.set_active_inspector_panel(InspectorPanelKind::Properties);
            self.set_status("DOCK SHORTCUT INSPECTOR PROP".to_string());
            return UiResponse::redraw();
        }
        if ctrl && !shift && shortcut_digit == Some(2) {
            self.set_active_inspector_panel(InspectorPanelKind::Layers);
            self.set_status("DOCK SHORTCUT INSPECTOR LAYERS".to_string());
            return UiResponse::redraw();
        }
        if ctrl && !shift && shortcut_digit == Some(3) {
            self.set_active_inspector_panel(InspectorPanelKind::Memory);
            self.set_status("DOCK SHORTCUT INSPECTOR MEMORY".to_string());
            return UiResponse::redraw();
        }
        if ctrl && !shift && shortcut_digit == Some(4) {
            self.set_active_bottom_panel(BottomPanelKind::Timeline);
            self.set_status("DOCK SHORTCUT BOTTOM TIMELINE".to_string());
            return UiResponse::redraw();
        }
        if ctrl && !shift && shortcut_digit == Some(5) {
            self.set_active_bottom_panel(BottomPanelKind::Log);
            self.set_status("DOCK SHORTCUT BOTTOM LOG".to_string());
            return UiResponse::redraw();
        }
        if ctrl && !shift && shortcut_digit == Some(6) {
            self.set_active_bottom_panel(BottomPanelKind::Diagnostics);
            self.set_status("DOCK SHORTCUT BOTTOM DIAG".to_string());
            return UiResponse::redraw();
        }
        if ctrl && upper == "Z" && shift {
            self.redo_last_command();
            return UiResponse::redraw();
        }
        if ctrl && upper == "Z" {
            self.undo_last_command();
            return UiResponse::redraw();
        }
        if ctrl && upper == "Y" {
            self.redo_last_command();
            return UiResponse::redraw();
        }

        if upper.contains("DELETE") || upper.contains("BACKSPACE") {
            self.delete_selected_object();
            return UiResponse::redraw();
        }

        let nudge_amount = if shift { 10.0_f32 } else { 1.0_f32 };
        if upper.contains("ARROWLEFT") || upper == "LEFT" {
            self.nudge_selected_object(-nudge_amount, 0.0);
            return UiResponse::redraw();
        }
        if upper.contains("ARROWRIGHT") || upper == "RIGHT" {
            self.nudge_selected_object(nudge_amount, 0.0);
            return UiResponse::redraw();
        }
        if upper.contains("ARROWUP") || upper == "UP" {
            self.nudge_selected_object(0.0, -nudge_amount);
            return UiResponse::redraw();
        }
        if upper.contains("ARROWDOWN") || upper == "DOWN" {
            self.nudge_selected_object(0.0, nudge_amount);
            return UiResponse::redraw();
        }

        let tool = match upper.as_str() {
            "1" | "V" => Some(ToolKind::Select),
            "2" | "H" => Some(ToolKind::Pan),
            "3" | "R" => Some(ToolKind::Box),
            "4" | "T" => Some(ToolKind::Text),
            "5" | "G" => Some(ToolKind::Grid),
            _ => None,
        };
        if let Some(tool) = tool {
            self.selected_tool = tool;
            self.active_menu = None;
            self.set_status(format!("STATUS SHORTCUT TOOL {}", tool.label()));
            return UiResponse::redraw();
        }

        if upper == "M" {
            self.floating_debug.open = !self.floating_debug.open;
            self.set_status(format!(
                "MEMORY VIEWER OPEN {}",
                self.floating_debug.open as u8
            ));
            return UiResponse::redraw();
        }

        if upper == "F" {
            self.canvas_pan = Point::new(0.0, 0.0);
            self.canvas_zoom = 1.0;
            self.set_status("STATUS FIT VIEW Z 100".to_string());
            return UiResponse::redraw();
        }

        self.set_status(format!(
            "STATUS KEY {} CTRL {} SHIFT {} ALT {} META {} TOOL {}",
            upper,
            ctrl as u8,
            shift as u8,
            alt as u8,
            meta as u8,
            self.selected_tool.label()
        ));
        UiResponse::redraw()
    }

    fn handle_text_input_key(
        &mut self,
        key: &str,
        upper: &str,
        _shift: bool,
    ) -> Option<UiResponse> {
        if upper == "ENTER" || upper == "NAMED(ENTER)" {
            self.text_input.focused = false;
            self.text_input.ime_preedit.clear();
            self.set_status(format!("TEXT INPUT COMMIT {}", self.text_input.text));
            return Some(UiResponse::redraw());
        }
        if upper == "ESCAPE" {
            self.text_input.focused = false;
            self.text_input.ime_preedit.clear();
            self.set_status("TEXT INPUT CANCEL".to_string());
            return Some(UiResponse::redraw());
        }
        if upper.contains("BACKSPACE") {
            self.text_input.backspace();
            self.set_status(format!("TEXT INPUT {}", self.text_input.text));
            return Some(UiResponse::redraw());
        }
        if upper.contains("DELETE") {
            self.text_input.delete_forward();
            self.set_status(format!("TEXT INPUT {}", self.text_input.text));
            return Some(UiResponse::redraw());
        }
        if upper.contains("ARROWLEFT") || upper == "LEFT" {
            self.text_input.move_left();
            self.set_status(format!("TEXT CURSOR {}", self.text_input.cursor));
            return Some(UiResponse::redraw());
        }
        if upper.contains("ARROWRIGHT") || upper == "RIGHT" {
            self.text_input.move_right();
            self.set_status(format!("TEXT CURSOR {}", self.text_input.cursor));
            return Some(UiResponse::redraw());
        }
        if upper.contains("HOME") {
            self.text_input.move_home();
            self.set_status("TEXT CURSOR 0".to_string());
            return Some(UiResponse::redraw());
        }
        if upper.contains("END") {
            self.text_input.move_end();
            self.set_status(format!("TEXT CURSOR {}", self.text_input.cursor));
            return Some(UiResponse::redraw());
        }
        if key.chars().count() == 1 && !key.chars().any(|ch| ch.is_control()) {
            self.text_input.insert_text(key);
            self.set_status(format!("TEXT INPUT {}", self.text_input.text));
            return Some(UiResponse::redraw());
        }
        None
    }

    fn handle_memory_editor_key(&mut self, key: &str, upper: &str) -> Option<UiResponse> {
        if upper == "ESCAPE" {
            self.memory_editor.focused = false;
            self.set_status("MEM EDIT BLUR".to_string());
            return Some(UiResponse::redraw());
        }
        if upper.contains("ARROWLEFT") || upper == "LEFT" {
            self.memory_editor
                .move_by(-1, self.memory_viewer.bytes.len());
            self.set_status(format!(
                "MEM EDIT SELECT {:04X}",
                self.memory_selected_address()
            ));
            return Some(UiResponse::redraw());
        }
        if upper.contains("ARROWRIGHT") || upper == "RIGHT" {
            self.memory_editor
                .move_by(1, self.memory_viewer.bytes.len());
            self.set_status(format!(
                "MEM EDIT SELECT {:04X}",
                self.memory_selected_address()
            ));
            return Some(UiResponse::redraw());
        }
        if upper.contains("ARROWUP") || upper == "UP" {
            self.memory_editor.move_by(
                -(MemoryEditorState::BYTES_PER_ROW as isize),
                self.memory_viewer.bytes.len(),
            );
            self.set_status(format!(
                "MEM EDIT SELECT {:04X}",
                self.memory_selected_address()
            ));
            return Some(UiResponse::redraw());
        }
        if upper.contains("ARROWDOWN") || upper == "DOWN" {
            self.memory_editor.move_by(
                MemoryEditorState::BYTES_PER_ROW as isize,
                self.memory_viewer.bytes.len(),
            );
            self.set_status(format!(
                "MEM EDIT SELECT {:04X}",
                self.memory_selected_address()
            ));
            return Some(UiResponse::redraw());
        }
        if upper.contains("HOME") {
            self.memory_editor.move_row_home();
            self.set_status(format!(
                "MEM EDIT SELECT {:04X}",
                self.memory_selected_address()
            ));
            return Some(UiResponse::redraw());
        }
        if upper.contains("END") {
            self.memory_editor
                .move_row_end(self.memory_viewer.bytes.len());
            self.set_status(format!(
                "MEM EDIT SELECT {:04X}",
                self.memory_selected_address()
            ));
            return Some(UiResponse::redraw());
        }
        if upper.contains("DELETE") {
            if let Some(byte) = self
                .memory_viewer
                .bytes
                .get_mut(self.memory_editor.selected_offset)
            {
                *byte = 0;
                self.memory_editor.high_nibble = true;
                self.set_status(format!(
                    "MEM EDIT {:04X} = 00",
                    self.memory_selected_address()
                ));
                return Some(UiResponse::redraw());
            }
        }
        if let Some(digit) = hex_digit_from_key(key, upper) {
            return Some(self.apply_memory_editor_hex_digit(digit));
        }
        None
    }

    fn apply_memory_editor_text(&mut self, text: &str) -> bool {
        let mut changed = false;
        for ch in text.chars() {
            let upper = ch.to_ascii_uppercase().to_string();
            if let Some(digit) = hex_digit_from_key(&upper, &upper) {
                self.apply_memory_editor_hex_digit(digit);
                changed = true;
            }
        }
        changed
    }

    fn apply_memory_editor_hex_digit(&mut self, digit: u8) -> UiResponse {
        let edit = self
            .memory_editor
            .edit_hex_digit(&mut self.memory_viewer.bytes, digit);
        if let Some(edit) = edit {
            self.set_status(format!(
                "MEM EDIT {:04X} {:02X}->{:02X}",
                self.memory_viewer
                    .base_address
                    .wrapping_add(edit.offset as u16),
                edit.old_value,
                edit.new_value
            ));
        } else {
            self.set_status("MEM EDIT EMPTY".to_string());
        }
        UiResponse::redraw()
    }

    fn memory_selected_address(&self) -> u16 {
        self.memory_viewer
            .base_address
            .wrapping_add(self.memory_editor.selected_offset as u16)
    }

    fn grid_world_rect(&self) -> Rect {
        Rect::new(
            230.0,
            54.0,
            self.grid_input.config.columns as f32 * self.grid_input.config.cell_size,
            self.grid_input.config.rows as f32 * self.grid_input.config.cell_size,
        )
    }

    fn grid_cell_at_screen(&self, point: Point) -> Option<usize> {
        let world = self.screen_to_world(point);
        let rect = self.grid_world_rect();
        if !rect.contains(world) {
            return None;
        }
        let cell = self.grid_input.config.cell_size.max(1.0);
        let column = ((world.x - rect.x) / cell).floor() as usize;
        let row = ((world.y - rect.y) / cell).floor() as usize;
        self.grid_input.index(column, row)
    }

    fn slider_rect(&self, layout: CreativeLayout) -> Rect {
        let width = (layout.inspector.width - 34.0).max(48.0);
        Rect::new(
            layout.inspector.x + 17.0,
            layout.inspector.bottom() - 90.0,
            width,
            18.0,
        )
    }

    fn text_input_rect(&self, layout: CreativeLayout) -> Rect {
        let width = (layout.inspector.width - 28.0).max(64.0);
        Rect::new(
            layout.inspector.x + 14.0,
            layout.inspector.bottom() - 138.0,
            width,
            30.0,
        )
    }

    fn rich_button_rects(&self, layout: CreativeLayout) -> [(WidgetId, &'static str, Rect); 2] {
        let y = layout.inspector.bottom() - 54.0;
        let button_w = ((layout.inspector.width - 42.0) / 2.0).max(52.0);
        [
            (
                RICH_APPLY_ID,
                "APPLY",
                Rect::new(layout.inspector.x + 14.0, y, button_w, 32.0),
            ),
            (
                RICH_RESET_ID,
                "RESET",
                Rect::new(layout.inspector.x + 24.0 + button_w, y, button_w, 32.0),
            ),
        ]
    }

    fn floating_memory_body_rect(&self) -> Rect {
        let rect = self.floating_debug.rect;
        Rect::new(
            rect.x + 1.0,
            rect.y + 32.0,
            rect.width - 2.0,
            rect.height - 34.0,
        )
    }

    fn floating_memory_editor_rect(&self) -> Rect {
        let body = self.floating_memory_body_rect();
        Rect::new(
            body.x - self.floating_debug_scroll.x,
            body.y - self.floating_debug_scroll.y,
            body.width + FLOATING_MEMORY_SCROLL_EXTRA.x,
            body.height + FLOATING_MEMORY_SCROLL_EXTRA.y,
        )
    }

    fn memory_editor_offset_at(&self, position: Point) -> Option<usize> {
        if !self.floating_debug.open || !self.floating_memory_body_rect().contains(position) {
            return None;
        }
        MemoryEditorState::hit_offset(
            self.floating_memory_editor_rect(),
            position,
            self.memory_viewer.bytes.len(),
        )
    }

    fn default_floating_dock_rect(&self, kind: FloatingDockKind) -> Rect {
        DockLayoutState::default_floating_rect(self.size, DockPanelKind::from_floating(kind))
    }

    fn floating_dock_rect(&self, kind: FloatingDockKind) -> Option<Rect> {
        self.floating_docks
            .iter()
            .find(|panel| panel.kind == kind && panel.open)
            .map(|panel| panel.rect)
    }

    fn set_floating_dock_rect(&mut self, kind: FloatingDockKind, rect: Rect) {
        let rect = DockLayoutState::clamp_floating_rect_for_size(self.size, rect);
        if let Some(panel) = self
            .floating_docks
            .iter_mut()
            .find(|panel| panel.kind == kind)
        {
            panel.rect = rect;
            panel.open = true;
        }
        self.dock_layout
            .set_floating_panel(DockPanelKind::from_floating(kind), rect, true);
        self.dock_layout.normalize_for_size(self.size);
    }

    fn open_floating_dock(&mut self, kind: FloatingDockKind) {
        let panel = DockPanelKind::from_floating(kind);
        if let Some(index) = self
            .floating_docks
            .iter()
            .position(|panel| panel.kind == kind)
        {
            self.floating_docks[index].rect = DockLayoutState::clamp_floating_rect_for_size(
                self.size,
                self.floating_docks[index].rect,
            );
            self.floating_docks[index].open = true;
        } else {
            let rect = self
                .dock_layout
                .floating_panel(panel)
                .map(|state| DockLayoutState::clamp_floating_rect_for_size(self.size, state.rect))
                .unwrap_or_else(|| self.default_floating_dock_rect(kind));
            self.floating_docks.push(FloatingDockPanel::new(kind, rect));
        }
        if let Some(rect) = self.floating_dock_rect(kind) {
            self.dock_layout.open_floating_panel(panel, rect);
        }
        self.set_status(format!(
            "FLOAT DOCK OPEN {} COUNT {}",
            kind.label(),
            self.floating_dock_count()
        ));
    }

    fn close_floating_dock(&mut self, kind: FloatingDockKind) {
        if let Some(panel) = self
            .floating_docks
            .iter_mut()
            .find(|panel| panel.kind == kind)
        {
            panel.open = false;
        }
        self.dock_layout
            .close_floating_panel(DockPanelKind::from_floating(kind));
        self.set_status(format!(
            "FLOAT DOCK CLOSE {} COUNT {}",
            kind.label(),
            self.floating_dock_count()
        ));
    }

    fn close_all_floating_docks(&mut self) {
        for panel in &mut self.floating_docks {
            panel.open = false;
        }
        self.dock_layout.close_all_floating_panels();
        self.set_status("FLOAT DOCK ALL CLOSED".to_string());
    }

    fn floating_dock_kind_from_title_id(&self, id: WidgetId) -> Option<FloatingDockKind> {
        self.floating_docks
            .iter()
            .find(|panel| panel.open && panel.kind.title_widget_id() == id)
            .map(|panel| panel.kind)
    }

    fn update_slider_from_position(&mut self, position: Point) {
        let slider = self.slider_rect(self.layout());
        let value = if slider.width > 0.0 {
            (position.x - slider.x) / slider.width
        } else {
            self.slider_value
        };
        self.slider_value = value.clamp(0.0, 1.0);
        self.set_status(format!(
            "SLIDER OPACITY {}",
            (self.slider_value * 100.0).round() as i32
        ));
    }

    fn context_menu_rect(&self) -> Option<Rect> {
        let menu = self.context_menu?;
        Some(Rect::new(menu.position.x, menu.position.y, 168.0, 106.0))
    }

    fn context_action_rects(&self) -> Option<[(ContextAction, Rect); 4]> {
        let popup = self.context_menu_rect()?;
        Some([
            (
                ContextAction::AddRect,
                Rect::new(popup.x, popup.y, popup.width, 26.0),
            ),
            (
                ContextAction::ToggleGridCell,
                Rect::new(popup.x, popup.y + 26.0, popup.width, 26.0),
            ),
            (
                ContextAction::MemoryViewer,
                Rect::new(popup.x, popup.y + 52.0, popup.width, 26.0),
            ),
            (
                ContextAction::Close,
                Rect::new(popup.x, popup.y + 78.0, popup.width, 26.0),
            ),
        ])
    }

    fn context_action_at(&self, point: Point) -> Option<ContextAction> {
        self.context_action_rects()?
            .iter()
            .find(|(_, rect)| (*rect).contains(point))
            .map(|(action, _)| *action)
    }

    fn menu_popup_rect(&self) -> Option<Rect> {
        let menu = self.active_menu?;
        let layout = self.layout();
        let item_rect = self
            .menu_item_rects(layout)
            .iter()
            .find(|(kind, _)| *kind == menu)
            .map(|(_, item_rect)| *item_rect)
            .unwrap_or(Rect::new(
                layout.menu_bar.x + 10.0,
                layout.menu_bar.bottom(),
                110.0,
                100.0,
            ));
        let rows = Self::menu_commands_for(menu).len() as f32;
        let row_height = Self::menu_row_height_for(menu);
        Some(Rect::new(
            item_rect.x,
            layout.menu_bar.bottom() - 1.0,
            220.0,
            rows * row_height + 4.0,
        ))
    }

    fn menu_row_height_for(menu: MenuKind) -> f32 {
        match menu {
            MenuKind::Window => 13.0,
            _ => 30.0,
        }
    }

    fn menu_commands_for(menu: MenuKind) -> &'static [MenuCommand] {
        match menu {
            MenuKind::File => &FILE_MENU_COMMANDS,
            MenuKind::Edit => &EDIT_MENU_COMMANDS,
            MenuKind::View => &VIEW_MENU_COMMANDS,
            MenuKind::Window => &WINDOW_MENU_COMMANDS,
            MenuKind::Help => &HELP_MENU_COMMANDS,
        }
    }

    fn menu_command_rects(&self) -> Option<Vec<(MenuCommand, Rect)>> {
        let menu = self.active_menu?;
        let popup = self.menu_popup_rect()?;
        let row_height = Self::menu_row_height_for(menu);
        Some(
            Self::menu_commands_for(menu)
                .iter()
                .enumerate()
                .map(|(index, command)| {
                    (
                        *command,
                        Rect::new(
                            popup.x,
                            popup.y + index as f32 * row_height,
                            popup.width,
                            row_height,
                        ),
                    )
                })
                .collect(),
        )
    }

    fn menu_command_at(&self, point: Point) -> Option<MenuCommand> {
        self.menu_command_rects()?
            .iter()
            .find(|(_, rect)| (*rect).contains(point))
            .map(|(command, _)| *command)
    }

    fn menu_command_label(command: MenuCommand) -> &'static str {
        match command {
            MenuCommand::NewDemo => "NEW DEMO",
            MenuCommand::OpenReset => "OPEN RESET",
            MenuCommand::SaveSnapshot => "SAVE SNAPSHOT",
            MenuCommand::Undo => "UNDO",
            MenuCommand::Redo => "REDO",
            MenuCommand::Copy => "COPY",
            MenuCommand::Paste => "PASTE",
            MenuCommand::FitView => "FIT VIEW",
            MenuCommand::ZoomIn => "ZOOM IN",
            MenuCommand::ZoomOut => "ZOOM OUT",
            MenuCommand::ToggleMemory => "MEMORY VIEW",
            MenuCommand::ShowProperties => "PANEL PROPERTIES",
            MenuCommand::ShowLayers => "PANEL LAYERS",
            MenuCommand::ShowMemoryPanel => "PANEL MEMORY",
            MenuCommand::ShowTimeline => "BOTTOM TIMELINE",
            MenuCommand::ShowLog => "BOTTOM LOG",
            MenuCommand::ShowDiagnostics => "BOTTOM DIAG",
            MenuCommand::ShowDockLayoutInfo => "DOCK LAYOUT INFO",
            MenuCommand::ResetDockLayout => "RESET DOCK LAYOUT",
            MenuCommand::SaveDockLayout => "SAVE DOCK LAYOUT",
            MenuCommand::LoadDockLayout => "LOAD DOCK LAYOUT",
            MenuCommand::RestoreDockTabs => "RESTORE DOCK TABS",
            MenuCommand::SaveDockLayoutSlotA => "SAVE LAYOUT SLOT A",
            MenuCommand::LoadDockLayoutSlotA => "LOAD LAYOUT SLOT A",
            MenuCommand::SaveDockLayoutSlotB => "SAVE LAYOUT SLOT B",
            MenuCommand::LoadDockLayoutSlotB => "LOAD LAYOUT SLOT B",
            MenuCommand::FloatProperties => "FLOAT PROP",
            MenuCommand::FloatLayers => "FLOAT LAYERS",
            MenuCommand::FloatMemory => "FLOAT MEMORY",
            MenuCommand::FloatTimeline => "FLOAT TIMELINE",
            MenuCommand::FloatLog => "FLOAT LOG",
            MenuCommand::FloatDiagnostics => "FLOAT DIAG",
            MenuCommand::DetachActiveRight => "DETACH RIGHT TAB",
            MenuCommand::DetachActiveBottom => "DETACH BOTTOM TAB",
            MenuCommand::DockAllFloating => "ATTACH FLOATING",
            MenuCommand::About => "ABOUT plita",
        }
    }

    fn invoke_menu_command(&mut self, command: MenuCommand) {
        self.active_menu = None;
        match command {
            MenuCommand::NewDemo | MenuCommand::OpenReset => self.reset_demo_document(),
            MenuCommand::SaveSnapshot => {
                let snapshot_len = self.snapshot_text().len();
                self.set_status(format!("MENU SAVE SNAPSHOT {} BYTES", snapshot_len));
            }
            MenuCommand::Undo => self.undo_last_command(),
            MenuCommand::Redo => self.redo_last_command(),
            MenuCommand::Copy => self.copy_selected_object(),
            MenuCommand::Paste => self.paste_clipboard(),
            MenuCommand::FitView => {
                self.canvas_pan = Point::new(0.0, 0.0);
                self.canvas_zoom = 1.0;
                self.set_status("MENU FIT VIEW".to_string());
            }
            MenuCommand::ZoomIn => {
                self.canvas_zoom = (self.canvas_zoom * 1.25).clamp(0.25_f32, 4.0_f32);
                self.set_status(format!(
                    "MENU ZOOM {}",
                    (self.canvas_zoom * 100.0).round() as i32
                ));
            }
            MenuCommand::ZoomOut => {
                self.canvas_zoom = (self.canvas_zoom * 0.80).clamp(0.25_f32, 4.0_f32);
                self.set_status(format!(
                    "MENU ZOOM {}",
                    (self.canvas_zoom * 100.0).round() as i32
                ));
            }
            MenuCommand::ToggleMemory => {
                self.floating_debug.open = !self.floating_debug.open;
                self.set_status(format!(
                    "MENU MEMORY VIEW {}",
                    self.floating_debug.open as u8
                ));
            }
            MenuCommand::ShowProperties => {
                self.set_active_inspector_panel(InspectorPanelKind::Properties);
                self.set_status("MENU PANEL PROPERTIES".to_string());
            }
            MenuCommand::ShowLayers => {
                self.set_active_inspector_panel(InspectorPanelKind::Layers);
                self.set_status("MENU PANEL LAYERS".to_string());
            }
            MenuCommand::ShowMemoryPanel => {
                self.set_active_inspector_panel(InspectorPanelKind::Memory);
                self.floating_debug.open = true;
                self.set_status("MENU PANEL MEMORY".to_string());
            }
            MenuCommand::ShowTimeline => {
                self.set_active_bottom_panel(BottomPanelKind::Timeline);
                self.set_status("MENU BOTTOM TIMELINE".to_string());
            }
            MenuCommand::ShowLog => {
                self.set_active_bottom_panel(BottomPanelKind::Log);
                self.set_status("MENU BOTTOM LOG".to_string());
            }
            MenuCommand::ShowDiagnostics => {
                self.set_active_bottom_panel(BottomPanelKind::Diagnostics);
                self.set_status("MENU BOTTOM DIAG".to_string());
            }
            MenuCommand::ShowDockLayoutInfo => {
                self.set_active_bottom_panel(BottomPanelKind::Diagnostics);
                self.set_status(format!(
                    "MENU DOCK LAYOUT INFO {}",
                    self.dock_layout_summary_text()
                ));
            }
            MenuCommand::ResetDockLayout => {
                self.reset_dock_layout();
                self.set_status("MENU RESET DOCK LAYOUT".to_string());
            }
            MenuCommand::SaveDockLayout => {
                let json = self.save_dock_layout_seed();
                self.set_status(format!("MENU SAVE DOCK LAYOUT {} BYTES", json.len()));
            }
            MenuCommand::LoadDockLayout => {
                if self.load_saved_dock_layout_seed() {
                    self.set_status("MENU LOAD DOCK LAYOUT".to_string());
                }
            }
            MenuCommand::RestoreDockTabs => {
                self.restore_dock_tabs();
                self.set_status("MENU RESTORE DOCK TABS".to_string());
            }
            MenuCommand::SaveDockLayoutSlotA => {
                self.save_dock_layout_slot(0);
            }
            MenuCommand::LoadDockLayoutSlotA => {
                self.load_dock_layout_slot(0);
            }
            MenuCommand::SaveDockLayoutSlotB => {
                self.save_dock_layout_slot(1);
            }
            MenuCommand::LoadDockLayoutSlotB => {
                self.load_dock_layout_slot(1);
            }
            MenuCommand::FloatProperties => {
                self.open_floating_dock(FloatingDockKind::Inspector(
                    InspectorPanelKind::Properties,
                ));
            }
            MenuCommand::FloatLayers => {
                self.open_floating_dock(FloatingDockKind::Inspector(InspectorPanelKind::Layers));
            }
            MenuCommand::FloatMemory => {
                self.open_floating_dock(FloatingDockKind::Inspector(InspectorPanelKind::Memory));
            }
            MenuCommand::FloatTimeline => {
                self.open_floating_dock(FloatingDockKind::Bottom(BottomPanelKind::Timeline));
            }
            MenuCommand::FloatLog => {
                self.open_floating_dock(FloatingDockKind::Bottom(BottomPanelKind::Log));
            }
            MenuCommand::FloatDiagnostics => {
                self.open_floating_dock(FloatingDockKind::Bottom(BottomPanelKind::Diagnostics));
            }
            MenuCommand::DetachActiveRight => {
                self.detach_dock_panel(DockPanelKind::from_inspector(self.active_inspector_panel));
            }
            MenuCommand::DetachActiveBottom => {
                self.detach_dock_panel(DockPanelKind::from_bottom(self.active_bottom_panel));
            }
            MenuCommand::DockAllFloating => {
                self.attach_all_floating_dock_panels();
            }
            MenuCommand::About => {
                self.floating_debug.open = true;
                self.set_status("MENU ABOUT plita UI MIT CLEAN ROOM".to_string());
            }
        }
    }

    fn invoke_context_action(&mut self, action: ContextAction, position: Point) {
        self.context_menu = None;
        match action {
            ContextAction::AddRect => {
                let world = self.screen_to_world(position);
                let id = ObjectId::new(self.next_object_id);
                self.next_object_id = self.next_object_id.saturating_add(1);
                let object = EditableRect::new(
                    id,
                    Rect::new(world.x - 40.0, world.y - 24.0, 80.0, 48.0),
                    "RECT",
                );
                self.apply_command(CanvasCommand::CreateRect { object });
                self.select_only_object(id);
                self.set_status(format!("CONTEXT ADD RECT {}", id.raw()));
            }
            ContextAction::ToggleGridCell => {
                if let Some(index) = self.grid_cell_at_screen(position) {
                    let value = self.grid_input.toggle_index(index).unwrap_or(0);
                    self.set_status(format!("CONTEXT GRID INDEX {} VALUE {}", index, value));
                } else {
                    self.selected_tool = ToolKind::Grid;
                    self.set_status("CONTEXT SELECT GRID TOOL".to_string());
                }
            }
            ContextAction::MemoryViewer => {
                self.floating_debug.open = true;
                self.set_status("CONTEXT MEMORY VIEWER OPEN".to_string());
            }
            ContextAction::Close => {
                self.set_status("CONTEXT CLOSED".to_string());
            }
        }
    }

    fn tool_button_rects(&self, layout: CreativeLayout) -> [(ToolKind, Rect); 5] {
        let button_w = (layout.tool_bar.width - 18.0).max(32.0);
        let x = layout.tool_bar.x + 9.0;
        let y = layout.tool_bar.y + 44.0;
        [
            (ToolKind::Select, Rect::new(x, y, button_w, 36.0)),
            (ToolKind::Pan, Rect::new(x, y + 46.0, button_w, 36.0)),
            (ToolKind::Box, Rect::new(x, y + 92.0, button_w, 36.0)),
            (ToolKind::Text, Rect::new(x, y + 138.0, button_w, 36.0)),
            (ToolKind::Grid, Rect::new(x, y + 184.0, button_w, 36.0)),
        ]
    }

    fn splitter_color(&self, id: WidgetId, theme: &Phase1Theme) -> Color {
        match self.visual_state_for(id) {
            WidgetVisualState::Active => theme.accent,
            WidgetVisualState::Hovered => theme.button_hover,
            _ => theme.border,
        }
    }

    fn select_only_object(&mut self, id: ObjectId) {
        self.selected_object = Some(id);
        self.selected_objects.clear();
        self.selected_objects.push(id);
    }

    fn clear_object_selection(&mut self) {
        self.selected_object = None;
        self.selected_objects.clear();
    }

    fn set_status(&mut self, text: String) {
        self.status_text = text;
    }
}

/// Generate the Phase 1 v1-compatible demo draw list.
pub fn phase1_demo_draw_list(size: Size) -> DrawList {
    CreativeShell::new(size).draw_list()
}

#[derive(Clone, Copy)]
struct Phase1Theme {
    background: Color,
    menu: Color,
    panel: Color,
    canvas: Color,
    timeline: Color,
    status: Color,
    border: Color,
    grid_minor: Color,
    grid_major: Color,
    accent: Color,
    text: Color,
    text_muted: Color,
    button: Color,
    button_hover: Color,
    button_active: Color,
    button_selected: Color,
}

impl Phase1Theme {
    fn dark() -> Self {
        Self {
            background: Color::rgb8(28, 30, 34),
            menu: Color::rgb8(36, 39, 44),
            panel: Color::rgb8(45, 48, 54),
            canvas: Color::rgb8(31, 34, 39),
            timeline: Color::rgb8(38, 41, 47),
            status: Color::rgb8(32, 35, 40),
            border: Color::rgb8(71, 76, 86),
            grid_minor: Color::rgba8(66, 72, 82, 95),
            grid_major: Color::rgba8(96, 108, 128, 140),
            accent: Color::rgb8(106, 158, 255),
            text: Color::rgb8(220, 224, 232),
            text_muted: Color::rgb8(151, 158, 170),
            button: Color::rgb8(56, 60, 68),
            button_hover: Color::rgb8(70, 76, 88),
            button_active: Color::rgb8(86, 94, 110),
            button_selected: Color::rgb8(63, 91, 142),
        }
    }

    fn panel_style(&self, top: Color, bottom: Color) -> PanelStyle {
        let mut style = PanelStyle::new(top, bottom, self.border);
        style.highlight = Color::rgba8(255, 255, 255, 20);
        style.shadow = Color::rgba8(0, 0, 0, 58);
        style.corner_radius = 5.0;
        style.box_shadow = Some(BoxShadow {
            offset: Point::new(0.0, 7.0),
            blur: 16.0,
            spread: 0.0,
            color: Color::rgba8(0, 0, 0, 58),
            corner_radius: 5.0,
            steps: 7,
        });
        style.radial_highlight = Some(PanelRadialHighlight::new(
            Point::new(0.35, 0.0),
            0.95,
            Color::rgba8(255, 255, 255, 13),
        ));
        style
    }
}

fn draw_menu(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    for (menu, item_rect) in shell.menu_item_rects(shell.layout()) {
        let state = shell.visual_state_for_menu(menu);
        let bg = match state {
            WidgetVisualState::Active => theme.button_active,
            WidgetVisualState::Selected => theme.button_selected,
            WidgetVisualState::Hovered => theme.button_hover,
            WidgetVisualState::Normal => theme.menu,
        };
        list.fill_rect(item_rect.inset(3.0), bg);
        draw_text(
            list,
            item_rect.x + 6.0,
            item_rect.y + 11.0,
            menu.label(),
            2.0,
            theme.text,
        );
    }
    draw_text(
        list,
        rect.right() - 270.0,
        rect.y + 11.0,
        "plita UI PHASE2 V4.0",
        2.0,
        theme.text_muted,
    );

    if shell.active_menu.is_some() {
        let Some(popup) = shell.menu_popup_rect() else {
            return;
        };
        list.fill_rect(popup, theme.panel);
        list.stroke_rect(popup, 1.0, theme.accent);
        if let Some(items) = shell.menu_command_rects() {
            for (command, item_rect) in items {
                let hovered = item_rect.contains(shell.last_pointer);
                list.fill_rect(
                    item_rect.inset(2.0),
                    if hovered {
                        theme.button_hover
                    } else {
                        theme.panel
                    },
                );
                let compact = item_rect.height <= 22.0;
                let text_y = if compact {
                    item_rect.y + 7.0
                } else {
                    item_rect.y + 10.0
                };
                let scale = if compact { 0.9 } else { 1.25 };
                draw_text(
                    list,
                    item_rect.x + 10.0,
                    text_y,
                    CreativeShell::menu_command_label(command),
                    scale,
                    theme.text_muted,
                );
            }
        }
    }
}

fn draw_toolbar(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    draw_text(
        list,
        rect.x + 10.0,
        rect.y + 8.0,
        "TOOLS",
        1.2,
        theme.text_muted,
    );
    let layout = shell.layout();
    for (tool, button) in shell.tool_button_rects(layout) {
        let state = shell.visual_state_for_tool(tool);
        let color = match state {
            WidgetVisualState::Active => theme.button_active,
            WidgetVisualState::Selected => theme.button_selected,
            WidgetVisualState::Hovered => theme.button_hover,
            WidgetVisualState::Normal => theme.button,
        };
        list.fill_rect(button, color);
        list.stroke_rect(
            button,
            1.0,
            if state == WidgetVisualState::Selected {
                theme.accent
            } else {
                theme.border
            },
        );
        draw_text(
            list,
            button.x + 9.0,
            button.y + 13.0,
            tool.label(),
            1.5,
            theme.text,
        );
    }
}

fn draw_dock_tab_close(
    list: &mut DrawList,
    rect: Rect,
    theme: &Phase1Theme,
    shell: &CreativeShell,
    panel: DockPanelKind,
) {
    let state = shell.visual_state_for(panel.tab_close_widget_id());
    let fill = match state {
        WidgetVisualState::Active => theme.button_active,
        WidgetVisualState::Hovered => theme.button_hover,
        WidgetVisualState::Selected | WidgetVisualState::Normal => Color::rgba8(20, 23, 28, 210),
    };
    list.fill_rect(rect, fill);
    list.stroke_rect(rect, 1.0, theme.border);
    draw_text(
        list,
        rect.x + 3.0,
        rect.y + 3.0,
        "X",
        0.72,
        theme.text_muted,
    );
}

fn draw_inspector(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    draw_text(
        list,
        rect.x + 14.0,
        rect.y + 12.0,
        "INSPECTOR DOCK",
        1.8,
        theme.text,
    );
    draw_text(
        list,
        rect.x + 14.0,
        rect.y + 30.0,
        &format!(
            "TOOL {}  PANEL {}",
            shell.selected_tool.label(),
            shell.active_inspector_panel.label()
        ),
        1.15,
        theme.accent,
    );

    for (panel, tab) in shell.inspector_tab_rects(shell.layout()) {
        let state = shell.visual_state_for_inspector_panel(panel);
        let color = match state {
            WidgetVisualState::Active => theme.button_active,
            WidgetVisualState::Selected => theme.button_selected,
            WidgetVisualState::Hovered => theme.button_hover,
            WidgetVisualState::Normal => theme.button,
        };
        list.fill_rect(tab, color);
        list.stroke_rect(
            tab,
            1.0,
            if state == WidgetVisualState::Selected {
                theme.accent
            } else {
                theme.border
            },
        );
        draw_text(
            list,
            tab.x + 8.0,
            tab.y + 8.0,
            panel.label(),
            1.05,
            theme.text,
        );
        draw_dock_tab_close(
            list,
            dock_tab_close_rect(tab),
            theme,
            shell,
            DockPanelKind::from_inspector(panel),
        );
    }

    let content_clip = Rect::new(
        rect.x + 1.0,
        rect.y + 72.0,
        rect.width - 2.0,
        rect.height - 74.0,
    );
    list.push_clip(content_clip);
    let rect = Rect::new(
        rect.x - shell.inspector_scroll.x,
        rect.y - shell.inspector_scroll.y,
        rect.width,
        rect.height,
    );
    let mut y = rect.y + 78.0;
    match shell.active_inspector_panel {
        InspectorPanelKind::Properties => {
            draw_text(
                list,
                rect.x + 14.0,
                y,
                &format!(
                    "OBJECTS {} CLIP {} GRID {} UNDO {} REDO {}",
                    shell.objects.len(),
                    shell.clipboard_len(),
                    shell.grid_input.active_count(),
                    shell.undo_len(),
                    shell.redo_len()
                ),
                1.05,
                theme.text_muted,
            );
            y += 20.0;
            draw_text(
                list,
                rect.x + 14.0,
                y,
                &format!("FONT {} {}", active_font_family(), active_text_backend()),
                1.0,
                theme.text_muted,
            );
            y += 24.0;
            if let Some(selected) = shell.selected_object_rect() {
                let selected_lines = [
                    format!(
                        "SELECTED X {} Y {}",
                        selected.x.round() as i32,
                        selected.y.round() as i32
                    ),
                    format!(
                        "SIZE W {} H {}",
                        selected.width.round() as i32,
                        selected.height.round() as i32
                    ),
                ];
                for label in selected_lines {
                    let row = Rect::new(rect.x + 12.0, y, rect.width - 24.0, 28.0);
                    list.fill_rect(row, Color::rgba8(63, 91, 142, 180));
                    list.stroke_rect(row, 1.0, theme.accent);
                    draw_text(list, row.x + 10.0, row.y + 10.0, &label, 1.15, theme.text);
                    y += 34.0;
                }
                y += 4.0;
            }
            for label in ["POSITION", "SIZE", "COLOR", "LAYER"] {
                let row = Rect::new(rect.x + 12.0, y, rect.width - 24.0, 31.0);
                list.fill_rect(row, theme.button);
                list.stroke_rect(row, 1.0, theme.border);
                draw_text(
                    list,
                    row.x + 10.0,
                    row.y + 11.0,
                    label,
                    1.3,
                    theme.text_muted,
                );
                y += 38.0;
            }
        }
        InspectorPanelKind::Layers => {
            let header = Rect::new(rect.x + 12.0, y, rect.width - 24.0, 28.0);
            list.fill_rect(header, Color::rgba8(23, 26, 31, 220));
            list.stroke_rect(header, 1.0, theme.border);
            draw_text(
                list,
                header.x + 10.0,
                header.y + 10.0,
                "LAYER STACK",
                1.2,
                theme.text,
            );
            y += 34.0;
            for object in shell.objects().iter().rev().take(8) {
                let row = Rect::new(rect.x + 12.0, y, rect.width - 24.0, 26.0);
                let selected = shell.selected_object == Some(object.id)
                    || shell.selected_objects.contains(&object.id);
                list.fill_rect(
                    row,
                    if selected {
                        Color::rgba8(63, 91, 142, 200)
                    } else {
                        theme.button
                    },
                );
                list.stroke_rect(row, 1.0, if selected { theme.accent } else { theme.border });
                draw_text(
                    list,
                    row.x + 10.0,
                    row.y + 9.0,
                    &format!(
                        "{}  ID {}  {}x{}",
                        object.label,
                        object.id.raw(),
                        object.rect.width.round() as i32,
                        object.rect.height.round() as i32
                    ),
                    1.05,
                    if selected {
                        theme.text
                    } else {
                        theme.text_muted
                    },
                );
                y += 30.0;
            }
            if shell.objects().is_empty() {
                draw_text(
                    list,
                    rect.x + 16.0,
                    y + 8.0,
                    "NO OBJECTS",
                    1.2,
                    theme.text_muted,
                );
            }
        }
        InspectorPanelKind::Memory => {
            let header = Rect::new(rect.x + 12.0, y, rect.width - 24.0, 28.0);
            list.fill_rect(header, Color::rgba8(23, 26, 31, 220));
            list.stroke_rect(header, 1.0, theme.border);
            draw_text(
                list,
                header.x + 10.0,
                header.y + 10.0,
                "DEBUG MEMORY SUMMARY",
                1.15,
                theme.text,
            );
            y += 38.0;
            draw_text(
                list,
                rect.x + 14.0,
                y,
                &format!("BASE {:04X}", shell.memory_viewer.base_address),
                1.3,
                theme.accent,
            );
            y += 24.0;
            let bytes = &shell.memory_viewer.bytes;
            for row in 0..6 {
                let offset = row * 8;
                if offset >= bytes.len() {
                    break;
                }
                let mut line = format!(
                    "{:04X}:",
                    shell.memory_viewer.base_address.wrapping_add(offset as u16)
                );
                for byte in bytes.iter().skip(offset).take(8) {
                    line.push_str(&format!(" {:02X}", byte));
                }
                draw_text(list, rect.x + 14.0, y, &line, 1.05, theme.text_muted);
                y += 18.0;
            }
            y += 6.0;
            draw_text(
                list,
                rect.x + 14.0,
                y,
                "M TO TOGGLE FLOATING VIEW",
                1.05,
                theme.text_muted,
            );
        }
    }
    list.pop_clip();
    draw_scrollbars(
        list,
        content_clip,
        shell.inspector_scroll,
        640.0,
        900.0,
        theme,
    );
}

fn draw_status(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, status: &str) {
    draw_text(
        list,
        rect.x + 12.0,
        rect.y + 9.0,
        status,
        1.5,
        theme.text_muted,
    );
}

fn draw_canvas_label(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    let panel = Rect::new(rect.x + 18.0, rect.y + 18.0, 198.0, 54.0);
    list.fill_rect(panel, Color::rgba8(20, 23, 28, 220));
    list.stroke_rect(panel, 1.0, theme.border);
    draw_text(
        list,
        panel.x + 12.0,
        panel.y + 13.0,
        "CANVAS AREA",
        1.5,
        theme.text_muted,
    );
    draw_text(
        list,
        panel.x + 12.0,
        panel.y + 31.0,
        &format!(
            "TOOL {} ZOOM {} OBJ {} GRID {}",
            shell.selected_tool.label(),
            (shell.canvas_zoom * 100.0).round() as i32,
            shell.objects.len(),
            shell.grid_input.active_count()
        ),
        1.2,
        theme.accent,
    );
}

fn draw_canvas_objects(
    list: &mut DrawList,
    canvas: Rect,
    theme: &Phase1Theme,
    shell: &CreativeShell,
) {
    let origin = shell.canvas_world_origin_screen(canvas);
    let artboard = Rect::new(
        origin.x,
        origin.y,
        420.0 * shell.canvas_zoom,
        300.0 * shell.canvas_zoom,
    );
    list.fill_rect(artboard, Color::rgb8(245, 246, 250));
    list.stroke_rect(artboard, 2.0, Color::rgb8(95, 106, 128));

    for object in shell.objects() {
        let screen = shell.world_rect_to_screen(object.rect);
        let is_selected =
            shell.selected_object == Some(object.id) || shell.selected_objects.contains(&object.id);
        let fill = if is_selected {
            Color::rgba8(106, 158, 255, 225)
        } else {
            Color::rgba8(130, 175, 245, 190)
        };
        let border = if is_selected {
            theme.accent
        } else {
            Color::rgb8(45, 84, 164)
        };
        list.fill_rect(screen, fill);
        list.stroke_rect(screen, if is_selected { 3.0 } else { 2.0 }, border);
        if is_selected {
            draw_selection_handles(list, screen, theme.accent);
        }
    }
}

fn draw_grid_input(list: &mut DrawList, canvas: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    let world = shell.grid_world_rect();
    let screen = shell.world_rect_to_screen(world);
    if !canvas.contains(Point::new(screen.x, screen.y))
        && !canvas.contains(Point::new(screen.right(), screen.bottom()))
    {
        // Still draw when partially outside; this guard only avoids wildly misplaced grids.
    }
    let title = Rect::new(screen.x, screen.y - 22.0, screen.width.max(160.0), 20.0);
    list.fill_rect(title, Color::rgba8(20, 23, 28, 220));
    list.stroke_rect(
        title,
        1.0,
        if shell.selected_tool == ToolKind::Grid {
            theme.accent
        } else {
            theme.border
        },
    );
    draw_text(
        list,
        title.x + 6.0,
        title.y + 7.0,
        &format!(
            "GRID {}X{} CELL {} ON {}",
            shell.grid_input.config.columns,
            shell.grid_input.config.rows,
            shell.grid_input.config.cell_size.round() as i32,
            shell.grid_input.active_count()
        ),
        1.0,
        theme.text_muted,
    );

    let cell = shell.grid_input.config.cell_size * shell.canvas_zoom;
    let gap = 1.0_f32.min((cell * 0.18).max(0.0));
    for row in 0..shell.grid_input.config.rows {
        for col in 0..shell.grid_input.config.columns {
            let index = row * shell.grid_input.config.columns + col;
            let active = shell.grid_input.cells.get(index).copied().unwrap_or(0) != 0;
            let x = screen.x + col as f32 * cell;
            let y = screen.y + row as f32 * cell;
            let rect = Rect::new(
                x + gap,
                y + gap,
                (cell - gap * 2.0).max(1.0),
                (cell - gap * 2.0).max(1.0),
            );
            list.fill_rect(
                rect,
                if active {
                    Color::rgba8(106, 158, 255, 230)
                } else {
                    Color::rgba8(25, 28, 33, 220)
                },
            );
            list.stroke_rect(rect, 1.0, if active { theme.accent } else { theme.border });
        }
    }
}

fn draw_text_input(
    list: &mut DrawList,
    layout: CreativeLayout,
    theme: &Phase1Theme,
    shell: &CreativeShell,
) {
    let input = shell.text_input_rect(layout);
    let state = shell.visual_state_for(TEXT_INPUT_ID);
    draw_text(
        list,
        input.x,
        input.y - 16.0,
        "NAME TEXT INPUT",
        1.1,
        theme.text_muted,
    );
    let border = if shell.text_input.focused {
        theme.accent
    } else if state == WidgetVisualState::Hovered {
        theme.button_hover
    } else {
        theme.border
    };
    list.fill_rect(input, Color::rgba8(21, 24, 30, 245));
    list.stroke_rect(input, 1.0, border);
    let display = if shell.text_input.ime_preedit.is_empty() {
        shell.text_input.text.clone()
    } else {
        format!(
            "{}[{}]",
            shell.text_input.text, shell.text_input.ime_preedit
        )
    };
    draw_text(
        list,
        input.x + 8.0,
        input.y + 10.0,
        &display,
        1.15,
        theme.text,
    );
    if shell.text_input.focused {
        let text_before_cursor = shell.text_input.text_before_cursor();
        let caret_offset =
            measure_text_width(text_before_cursor, 1.15).min((input.width - 18.0).max(1.0));
        let caret_x = input.x + 8.0 + caret_offset;
        list.fill_rect(
            Rect::new(caret_x, input.y + 7.0, 2.0, input.height - 14.0),
            theme.accent,
        );
    }
}

fn draw_rich_controls(
    list: &mut DrawList,
    layout: CreativeLayout,
    theme: &Phase1Theme,
    shell: &CreativeShell,
) {
    draw_text_input(list, layout, theme, shell);

    let slider = shell.slider_rect(layout);
    draw_text(
        list,
        slider.x,
        slider.y - 16.0,
        "OPACITY SLIDER",
        1.1,
        theme.text_muted,
    );
    list.fill_rect(slider, Color::rgba8(22, 25, 30, 230));
    list.stroke_rect(slider, 1.0, theme.border);
    let filled = Rect::new(
        slider.x,
        slider.y,
        slider.width * shell.slider_value.clamp(0.0, 1.0),
        slider.height,
    );
    list.fill_rect(filled, Color::rgba8(106, 158, 255, 170));
    let knob_x = slider.x + slider.width * shell.slider_value.clamp(0.0, 1.0) - 5.0;
    let knob = Rect::new(knob_x, slider.y - 4.0, 10.0, slider.height + 8.0);
    list.fill_rect(knob, theme.button_selected);
    list.stroke_rect(knob, 1.0, theme.accent);

    for (id, label, rect) in shell.rich_button_rects(layout) {
        let state = shell.visual_state_for(id);
        let color = match state {
            WidgetVisualState::Active => theme.button_active,
            WidgetVisualState::Hovered => theme.button_hover,
            WidgetVisualState::Selected => theme.button_selected,
            WidgetVisualState::Normal => theme.button,
        };
        // Layered rectangles approximate a richer rounded/raised button without adding a path renderer yet.
        list.fill_rect(rect, Color::rgba8(18, 21, 26, 180));
        list.fill_rect(rect.inset(2.0), color);
        list.fill_rect(
            Rect::new(rect.x + 3.0, rect.y + 3.0, rect.width - 6.0, 4.0),
            Color::rgba8(255, 255, 255, 28),
        );
        list.stroke_rect(
            rect,
            1.0,
            if state == WidgetVisualState::Hovered {
                theme.accent
            } else {
                theme.border
            },
        );
        draw_text(list, rect.x + 12.0, rect.y + 12.0, label, 1.25, theme.text);
    }
}

fn draw_context_menu(list: &mut DrawList, theme: &Phase1Theme, shell: &CreativeShell) {
    let Some(rect) = shell.context_menu_rect() else {
        return;
    };
    list.fill_rect(rect, Color::rgba8(35, 38, 44, 245));
    list.stroke_rect(rect, 1.0, theme.accent);
    let Some(items) = shell.context_action_rects() else {
        return;
    };
    let labels = ["ADD RECT", "GRID CELL", "MEMORY VIEW", "CLOSE"];
    for (index, (_, item_rect)) in items.iter().enumerate() {
        list.fill_rect(
            item_rect.inset(2.0),
            if (*item_rect).contains(shell.last_pointer) {
                theme.button_hover
            } else {
                Color::rgba8(35, 38, 44, 245)
            },
        );
        draw_text(
            list,
            item_rect.x + 10.0,
            item_rect.y + 9.0,
            labels[index],
            1.2,
            theme.text,
        );
    }
}

fn floating_dock_title_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, 28.0)
}

fn floating_dock_close_rect(rect: Rect) -> Rect {
    Rect::new(rect.right() - 28.0, rect.y + 4.0, 20.0, 20.0)
}

fn dock_tab_close_rect(rect: Rect) -> Rect {
    Rect::new(rect.right() - 17.0, rect.y + 5.0, 12.0, 12.0)
}

fn draw_floating_dock_windows(list: &mut DrawList, theme: &Phase1Theme, shell: &CreativeShell) {
    for panel in shell.floating_docks.iter().filter(|panel| panel.open) {
        draw_floating_dock_window(list, theme, shell, *panel);
    }
}

fn draw_floating_dock_window(
    list: &mut DrawList,
    theme: &Phase1Theme,
    shell: &CreativeShell,
    panel: FloatingDockPanel,
) {
    let rect = panel.rect;
    list.fill_rect(rect, Color::rgba8(30, 34, 40, 244));
    list.stroke_rect(rect, 1.0, theme.accent);

    let title = floating_dock_title_rect(rect);
    list.fill_rect(title, Color::rgba8(72, 87, 112, 238));
    draw_text(
        list,
        title.x + 10.0,
        title.y + 10.0,
        panel.kind.label(),
        1.15,
        theme.text,
    );

    let close = floating_dock_close_rect(rect);
    list.fill_rect(close, theme.button);
    list.stroke_rect(close, 1.0, theme.border);
    draw_text(list, close.x + 6.0, close.y + 6.0, "X", 1.05, theme.text);

    let body = Rect::new(
        rect.x + 10.0,
        rect.y + 40.0,
        rect.width - 20.0,
        rect.height - 52.0,
    );
    list.stroke_rect(body, 1.0, theme.border);

    let body_clip = body;
    let scroll = shell.floating_dock_scroll(panel.kind);
    list.push_clip(body_clip);
    let body = Rect::new(
        body.x - scroll.x,
        body.y - scroll.y,
        body.width + 760.0,
        body.height + 760.0,
    );
    match panel.kind {
        FloatingDockKind::Inspector(InspectorPanelKind::Properties) => {
            draw_text(
                list,
                body.x + 10.0,
                body.y + 12.0,
                "PROPERTIES PANEL",
                1.1,
                theme.text,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 38.0,
                &format!("OBJECTS {}", shell.objects.len()),
                1.0,
                theme.text_muted,
            );
            if let Some(rect) = shell.selected_object_rect() {
                draw_text(
                    list,
                    body.x + 10.0,
                    body.y + 62.0,
                    &format!("X {} Y {}", rect.x.round() as i32, rect.y.round() as i32),
                    1.0,
                    theme.text_muted,
                );
                draw_text(
                    list,
                    body.x + 10.0,
                    body.y + 86.0,
                    &format!(
                        "W {} H {}",
                        rect.width.round() as i32,
                        rect.height.round() as i32
                    ),
                    1.0,
                    theme.text_muted,
                );
            } else {
                draw_text(
                    list,
                    body.x + 10.0,
                    body.y + 62.0,
                    "NO SELECTION",
                    1.0,
                    theme.text_muted,
                );
            }
        }
        FloatingDockKind::Inspector(InspectorPanelKind::Layers) => {
            draw_text(
                list,
                body.x + 10.0,
                body.y + 12.0,
                "LAYERS PANEL",
                1.1,
                theme.text,
            );
            let mut y = body.y + 38.0;
            for object in shell.objects.iter().take(6) {
                let selected = shell.selected_object == Some(object.id)
                    || shell.selected_objects.contains(&object.id);
                let row = Rect::new(body.x + 8.0, y - 4.0, body.width - 16.0, 20.0);
                if selected {
                    list.fill_rect(row, theme.button_selected);
                }
                draw_text(
                    list,
                    body.x + 12.0,
                    y,
                    &format!("{} {}", object.id.raw(), object.label),
                    1.0,
                    theme.text_muted,
                );
                y += 22.0;
            }
        }
        FloatingDockKind::Inspector(InspectorPanelKind::Memory) => {
            draw_text(
                list,
                body.x + 10.0,
                body.y + 12.0,
                "MEMORY SUMMARY",
                1.1,
                theme.text,
            );
            for row in 0..4 {
                let addr = shell
                    .memory_viewer
                    .base_address
                    .wrapping_add((row * 8) as u16);
                let mut line = format!("{:04X} ", addr);
                for col in 0..8 {
                    let index = row * 8 + col;
                    let byte = shell.memory_viewer.bytes.get(index).copied().unwrap_or(0);
                    line.push_str(&format!("{:02X} ", byte));
                }
                draw_text(
                    list,
                    body.x + 10.0,
                    body.y + 40.0 + row as f32 * 22.0,
                    &line,
                    0.95,
                    theme.text_muted,
                );
            }
        }
        FloatingDockKind::Bottom(BottomPanelKind::Timeline) => {
            draw_text(
                list,
                body.x + 10.0,
                body.y + 12.0,
                "TIMELINE PANEL",
                1.1,
                theme.text,
            );
            draw_timeline_tracks(
                list,
                Rect::new(
                    body.x + 6.0,
                    body.y + 36.0,
                    body.width - 12.0,
                    body.height - 42.0,
                ),
                theme.grid_minor,
                theme.accent,
            );
        }
        FloatingDockKind::Bottom(BottomPanelKind::Log) => {
            draw_text(
                list,
                body.x + 10.0,
                body.y + 12.0,
                "LOG PANEL",
                1.1,
                theme.text,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 40.0,
                "INFO  FLOATING DOCK OPEN",
                0.95,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 62.0,
                &format!("STATUS {}", shell.status_text()),
                0.95,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 84.0,
                "CTRL+SHIFT+0 DOCK ALL",
                0.95,
                theme.accent,
            );
        }
        FloatingDockKind::Bottom(BottomPanelKind::Diagnostics) => {
            draw_text(
                list,
                body.x + 10.0,
                body.y + 12.0,
                "DIAGNOSTICS PANEL",
                1.1,
                theme.text,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 40.0,
                &format!("ZOOM {}%", (shell.canvas_zoom * 100.0).round() as i32),
                0.95,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 62.0,
                &format!("GRID ACTIVE {}", shell.grid_input.active_count()),
                0.95,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 84.0,
                &shell.dock_layout_summary_text(),
                0.82,
                theme.accent,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 106.0,
                &shell.dock_split_summary_text(),
                0.9,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 128.0,
                &shell.dock_tab_summary_text(),
                0.9,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 150.0,
                &shell.dock_layout_slot_summary_text(),
                0.9,
                theme.text_muted,
            );
            draw_text(
                list,
                body.x + 10.0,
                body.y + 172.0,
                &format!("LAYOUT SEED {} BYTES", shell.saved_dock_layout_len()),
                0.9,
                theme.text_muted,
            );
        }
    }
    list.pop_clip();
    draw_scrollbars(list, body_clip, scroll, 760.0, 760.0, theme);
}

fn draw_selection_handles(list: &mut DrawList, rect: Rect, color: Color) {
    for (_, handle_rect) in selection_handle_rects(rect) {
        list.fill_rect(handle_rect, Color::rgb8(245, 246, 250));
        list.stroke_rect(handle_rect, 1.0, color);
    }
}

fn selection_handle_rects(rect: Rect) -> [(ResizeHandle, Rect); 4] {
    let handle = 9.0_f32;
    let half = handle / 2.0;
    [
        (
            ResizeHandle::TopLeft,
            Rect::new(rect.x - half, rect.y - half, handle, handle),
        ),
        (
            ResizeHandle::TopRight,
            Rect::new(rect.right() - half, rect.y - half, handle, handle),
        ),
        (
            ResizeHandle::BottomLeft,
            Rect::new(rect.x - half, rect.bottom() - half, handle, handle),
        ),
        (
            ResizeHandle::BottomRight,
            Rect::new(rect.right() - half, rect.bottom() - half, handle, handle),
        ),
    ]
}

fn resize_rect_from_handle(rect: Rect, handle: ResizeHandle, dx: f32, dy: f32) -> Rect {
    let min_size = 16.0_f32;
    match handle {
        ResizeHandle::TopLeft => {
            let new_x = (rect.x + dx).min(rect.right() - min_size);
            let new_y = (rect.y + dy).min(rect.bottom() - min_size);
            Rect::new(new_x, new_y, rect.right() - new_x, rect.bottom() - new_y)
        }
        ResizeHandle::TopRight => {
            let new_right = (rect.right() + dx).max(rect.x + min_size);
            let new_y = (rect.y + dy).min(rect.bottom() - min_size);
            Rect::new(rect.x, new_y, new_right - rect.x, rect.bottom() - new_y)
        }
        ResizeHandle::BottomLeft => {
            let new_x = (rect.x + dx).min(rect.right() - min_size);
            let new_bottom = (rect.bottom() + dy).max(rect.y + min_size);
            Rect::new(new_x, rect.y, rect.right() - new_x, new_bottom - rect.y)
        }
        ResizeHandle::BottomRight => {
            let new_right = (rect.right() + dx).max(rect.x + min_size);
            let new_bottom = (rect.bottom() + dy).max(rect.y + min_size);
            Rect::new(rect.x, rect.y, new_right - rect.x, new_bottom - rect.y)
        }
    }
}

fn draw_canvas_grid(
    list: &mut DrawList,
    rect: Rect,
    minor: Color,
    major: Color,
    pan: Point,
    zoom: f32,
) {
    let step = (32.0_f32 * zoom).clamp(8.0_f32, 128.0_f32);
    let major_step = step * 4.0;
    let x_start = rect.x + pan.x.rem_euclid(step);
    let y_start = rect.y + pan.y.rem_euclid(step);

    let mut x = x_start;
    let mut i = 1;
    while x < rect.right() {
        let color = if i % 4 == 0 { major } else { minor };
        list.fill_rect(Rect::new(x, rect.y, 1.0, rect.height), color);
        x += step;
        i += 1;
    }

    let mut y = y_start;
    i = 1;
    while y < rect.bottom() {
        let color = if i % 4 == 0 { major } else { minor };
        list.fill_rect(Rect::new(rect.x, y, rect.width, 1.0), color);
        y += step;
        i += 1;
    }

    let ruler_x = Rect::new(rect.x, rect.y, rect.width, 18.0);
    let ruler_y = Rect::new(rect.x, rect.y, 18.0, rect.height);
    list.fill_rect(ruler_x, Color::rgba8(22, 25, 30, 220));
    list.fill_rect(ruler_y, Color::rgba8(22, 25, 30, 220));

    let mut tick = rect.x + major_step;
    while tick < rect.right() {
        list.fill_rect(Rect::new(tick, rect.y, 1.0, 18.0), major);
        tick += major_step;
    }

    tick = rect.y + major_step;
    while tick < rect.bottom() {
        list.fill_rect(Rect::new(rect.x, tick, 18.0, 1.0), major);
        tick += major_step;
    }
}

fn draw_timeline(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    let header = Rect::new(rect.x, rect.y, rect.width, 30.0);
    list.fill_rect(header, Color::rgba8(23, 26, 31, 230));

    for (panel, tab) in shell.bottom_tab_rects(shell.layout()) {
        let state = shell.visual_state_for_bottom_panel(panel);
        let color = match state {
            WidgetVisualState::Active => theme.button_active,
            WidgetVisualState::Selected => theme.button_selected,
            WidgetVisualState::Hovered => theme.button_hover,
            WidgetVisualState::Normal => theme.button,
        };
        list.fill_rect(tab, color);
        list.stroke_rect(
            tab,
            1.0,
            if state == WidgetVisualState::Selected {
                theme.accent
            } else {
                theme.border
            },
        );
        draw_text(
            list,
            tab.x + 8.0,
            tab.y + 8.0,
            panel.label(),
            1.05,
            theme.text,
        );
        draw_dock_tab_close(
            list,
            dock_tab_close_rect(tab),
            theme,
            shell,
            DockPanelKind::from_bottom(panel),
        );
    }

    let content_clip = Rect::new(
        rect.x + 1.0,
        rect.y + 31.0,
        rect.width - 2.0,
        rect.height - 32.0,
    );
    list.push_clip(content_clip);
    let scrolled = Rect::new(
        rect.x - shell.bottom_scroll.x,
        rect.y - shell.bottom_scroll.y,
        rect.width + 900.0,
        rect.height + 720.0,
    );
    match shell.active_bottom_panel {
        BottomPanelKind::Timeline => {
            draw_timeline_tracks(list, scrolled, theme.grid_minor, theme.accent)
        }
        BottomPanelKind::Log => draw_log_panel(list, scrolled, theme, shell),
        BottomPanelKind::Diagnostics => draw_diagnostics_panel(list, scrolled, theme, shell),
    }
    list.pop_clip();
    draw_scrollbars(list, content_clip, shell.bottom_scroll, 900.0, 720.0, theme);
}

fn draw_timeline_tracks(list: &mut DrawList, rect: Rect, grid: Color, accent: Color) {
    let bounds = rect;
    let track_h = ((rect.height - 30.0) / 3.0).max(16.0);
    for row in 0..3 {
        let y = rect.y + 30.0 + track_h * row as f32;
        if let Some(line) = clip_rect_to_bounds(Rect::new(rect.x, y, rect.width, 1.0), bounds) {
            list.fill_rect(line, grid);
        }
        if y + 18.0 < rect.bottom() {
            draw_text(
                list,
                rect.x + 12.0,
                y + 8.0,
                &format!("TRACK{}", row + 1),
                1.5,
                Color::rgb8(151, 158, 170),
            );
        }
    }

    let mut x = rect.x + 110.0;
    let mut tick = 1;
    while x < rect.right() {
        let color = if tick % 4 == 0 { accent } else { grid };
        if let Some(line) =
            clip_rect_to_bounds(Rect::new(x, rect.y + 30.0, 1.0, rect.height - 30.0), bounds)
        {
            list.fill_rect(line, color);
        }
        if tick % 4 == 0 && x + 24.0 < rect.right() {
            draw_text(
                list,
                x + 5.0,
                rect.y + 36.0,
                &format!("{}", tick / 4),
                1.2,
                Color::rgb8(151, 158, 170),
            );
        }
        x += 28.0;
        tick += 1;
    }

    for (row, offset) in [0.0_f32, 76.0_f32, 160.0_f32].iter().enumerate() {
        let y = rect.y + 37.0 + track_h * row as f32;
        let clip = Rect::new(rect.x + 124.0 + *offset, y, 88.0 + row as f32 * 36.0, 20.0);
        if let Some(clipped) = clip_rect_to_bounds(clip, bounds.inset(1.0)) {
            list.fill_rect(clipped, Color::rgba8(106, 158, 255, 200));
            list.stroke_rect(clipped, 1.0, Color::rgb8(45, 84, 164));
        }
    }
}

fn clip_rect_to_bounds(rect: Rect, bounds: Rect) -> Option<Rect> {
    let x0 = rect.x.max(bounds.x);
    let y0 = rect.y.max(bounds.y);
    let x1 = rect.right().min(bounds.right());
    let y1 = rect.bottom().min(bounds.bottom());
    if x1 <= x0 || y1 <= y0 {
        None
    } else {
        Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

fn draw_scrollbars(
    list: &mut DrawList,
    viewport: Rect,
    scroll: Point,
    max_x: f32,
    max_y: f32,
    theme: &Phase1Theme,
) {
    let state = ScrollAreaState::from_offset(scroll);
    let content_size = Size::new(
        finite_nonnegative(viewport.width) + max_x,
        finite_nonnegative(viewport.height) + max_y,
    );
    state.draw_scrollbars(list, viewport, content_size, phase1_scrollbar_style(theme));
}

fn phase1_scrollbar_style(theme: &Phase1Theme) -> ScrollbarStyle {
    ScrollbarStyle {
        thumb_hovered: theme.accent,
        thumb_dragged: theme.accent,
        ..phase1_scrollbar_input_style()
    }
}

fn phase1_scrollbar_input_style() -> ScrollbarStyle {
    ScrollbarStyle {
        thickness: 4.0,
        margin: 4.0,
        min_thumb_len: 18.0,
        track: Color::rgba8(18, 21, 26, 150),
        thumb: Color::rgba8(106, 158, 255, 170),
        thumb_hovered: Color::rgba8(134, 178, 255, 210),
        thumb_dragged: Color::rgba8(157, 196, 255, 230),
        corner: Color::rgba8(18, 21, 26, 180),
    }
}

fn draw_log_panel(list: &mut DrawList, rect: Rect, theme: &Phase1Theme, shell: &CreativeShell) {
    let mut y = rect.y + 40.0;
    for line in [
        "[dock] Phase2 v4.0 tab visibility / layout slots",
        "[input] Ctrl+1..3 fixed tabs / Ctrl+Shift+1..6 floating",
        "[layout]",
    ] {
        draw_text(list, rect.x + 14.0, y, line, 1.25, theme.text_muted);
        y += 19.0;
    }
    draw_text(
        list,
        rect.x + 14.0,
        y,
        &shell.dock_layout_summary_text(),
        1.05,
        theme.accent,
    );
    y += 19.0;
    draw_text(
        list,
        rect.x + 14.0,
        y,
        shell.status_text(),
        1.15,
        theme.text_muted,
    );
}

fn bottom_diagnostics_lines(shell: &CreativeShell) -> [String; 8] {
    [
        "DOCK LAYOUT".to_string(),
        shell.dock_layout_summary_text(),
        shell.dock_split_summary_text(),
        shell.dock_tab_summary_text(),
        shell.dock_layout_slot_summary_text(),
        format!("LAYOUT JSON SEED {} BYTES", shell.saved_dock_layout_len()),
        format!(
            "OBJECTS {}  GRID {}  UNDO {}  REDO {}",
            shell.objects.len(),
            shell.grid_input.active_count(),
            shell.undo_len(),
            shell.redo_len()
        ),
        format!(
            "ZOOM {}  PAN {},{}",
            (shell.canvas_zoom * 100.0).round() as i32,
            shell.canvas_pan.x.round() as i32,
            shell.canvas_pan.y.round() as i32
        ),
    ]
}

fn bottom_diagnostics_line_positions(rect: Rect) -> [f32; 8] {
    if rect.height <= 80.0 {
        // Compact windows can make the bottom dock only ~72px tall. Keep the
        // Dock Layout title, active/floating summary, and split-ratio summary
        // inside the visible content area. Tab/slot/seed lines are useful but
        // secondary, so they may be clipped first on very small windows.
        [
            rect.y + 34.0,
            rect.y + 44.0,
            rect.y + 54.0,
            rect.y + 64.0,
            rect.y + 74.0,
            rect.y + 84.0,
            rect.y + 94.0,
            rect.y + 104.0,
        ]
    } else {
        [
            rect.y + 38.0,
            rect.y + 56.0,
            rect.y + 74.0,
            rect.y + 92.0,
            rect.y + 110.0,
            rect.y + 130.0,
            rect.y + 148.0,
            rect.y + 166.0,
        ]
    }
}

fn draw_diagnostics_panel(
    list: &mut DrawList,
    rect: Rect,
    theme: &Phase1Theme,
    shell: &CreativeShell,
) {
    let content = Rect::new(
        rect.x + 8.0,
        rect.y + 34.0,
        rect.width - 16.0,
        (rect.height - 40.0).max(0.0),
    );
    list.fill_rect(content, Color::rgba8(36, 45, 60, 155));
    list.stroke_rect(content, 1.0, theme.border);

    let lines = bottom_diagnostics_lines(shell);
    let ys = bottom_diagnostics_line_positions(rect);
    let bottom_limit = rect.bottom() - 8.0;

    for (index, (line, y)) in lines.iter().zip(ys.iter()).enumerate() {
        if *y + 10.0 > bottom_limit {
            break;
        }
        let color = match index {
            0 => theme.text,
            1 => theme.accent,
            _ => theme.text_muted,
        };
        let scale = if index == 1 { 0.98 } else { 1.02 };
        draw_text(list, rect.x + 14.0, *y, line, scale, color);
    }
}

/// plita's bundled default UI font family.
pub const DEFAULT_FONT_FAMILY: &str = "Roboto";

/// Return plita's default UI font family.
pub fn default_font_family() -> &'static str {
    DEFAULT_FONT_FAMILY
}

/// Return the currently active text backend.
///
/// Phase 1 v8 keeps the rectangle-only renderer, but it can now rasterize a real
/// local TTF/OTF/TTC font into alpha rectangles. Roboto is bundled as the
/// default; set `PLITA_UI_FONT` or place another font in `assets/fonts/` to
/// override it.
pub fn active_text_backend() -> &'static str {
    if rich_font_rasterizer().is_some() {
        "LOCAL_FONT"
    } else {
        "BITMAP_FALLBACK"
    }
}

/// Return the active local font source path when a rich local font was loaded.
pub fn active_font_source() -> Option<&'static str> {
    rich_font_rasterizer().map(|rasterizer| rasterizer.source_name.as_str())
}

/// Return a compact family label for the active text renderer.
pub fn active_font_family() -> &'static str {
    active_font_source()
        .map(font_family_label_for_source)
        .unwrap_or("Bitmap Fallback")
}

/// Return a compact family label for the memory-editor numeric text renderer.
pub fn active_memory_font_family() -> &'static str {
    memory_font_rasterizer()
        .map(|rasterizer| memory_font_family_label_for_source(&rasterizer.source_name))
        .unwrap_or("Bitmap Fallback")
}

fn font_family_label_for_source(source: &str) -> &'static str {
    if source.to_ascii_lowercase().contains("roboto") {
        DEFAULT_FONT_FAMILY
    } else {
        "Local Font"
    }
}

fn memory_font_family_label_for_source(source: &str) -> &'static str {
    if source.to_ascii_lowercase().contains("robotomono")
        || source.to_ascii_lowercase().contains("roboto mono")
    {
        "Roboto Mono"
    } else {
        "Local Memory Font"
    }
}

struct RichFontRasterizer {
    font: fontdue::Font,
    source_name: String,
    fallback_font: Option<fontdue::Font>,
}

static RICH_FONT: OnceLock<Option<RichFontRasterizer>> = OnceLock::new();
static MEMORY_FONT: OnceLock<Option<RichFontRasterizer>> = OnceLock::new();
const TEXT_IMAGE_CACHE_LIMIT: usize = 768;
static TEXT_IMAGE_CACHE: OnceLock<Mutex<HashMap<TextImageKey, TextImage>>> = OnceLock::new();

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TextImageKey {
    text: String,
    size_tenths: u16,
    rgba: [u8; 4],
}

#[derive(Clone, Debug, PartialEq)]
struct TextImage {
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
    pixels: Arc<[u8]>,
}

fn rich_font_rasterizer() -> Option<&'static RichFontRasterizer> {
    RICH_FONT.get_or_init(load_rich_font).as_ref()
}

fn memory_font_rasterizer() -> Option<&'static RichFontRasterizer> {
    MEMORY_FONT.get_or_init(load_memory_font).as_ref()
}

fn load_rich_font() -> Option<RichFontRasterizer> {
    for path in candidate_font_paths() {
        let Some(font) = load_font_path(&path) else {
            continue;
        };
        let fallback_font = (font.lookup_glyph_index('\u{4e00}') == 0)
            .then(load_cjk_fallback_font)
            .flatten();
        return Some(RichFontRasterizer {
            font,
            source_name: path.display().to_string(),
            fallback_font,
        });
    }
    None
}

fn load_font_path(path: &PathBuf) -> Option<fontdue::Font> {
    let bytes = std::fs::read(path).ok()?;
    // Try a few collection indexes so common Windows .ttc fonts work.
    (0..8_u32).find_map(|collection_index| {
        fontdue::Font::from_bytes(
            bytes.clone(),
            fontdue::FontSettings {
                collection_index,
                ..fontdue::FontSettings::default()
            },
        )
        .ok()
    })
}

fn load_cjk_fallback_font() -> Option<fontdue::Font> {
    candidate_cjk_font_paths()
        .into_iter()
        .filter_map(|path| load_font_path(&path))
        .find(|font| font.lookup_glyph_index('\u{4e00}') != 0)
}

fn load_memory_font() -> Option<RichFontRasterizer> {
    for path in candidate_memory_font_paths() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };

        if let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
            return Some(RichFontRasterizer {
                font,
                source_name: path.display().to_string(),
                fallback_font: None,
            });
        }
    }
    None
}

fn candidate_font_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(path) = std::env::var_os("PLITA_UI_FONT") {
        paths.push(PathBuf::from(path));
    }

    paths.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("fonts")
            .join("Roboto[wdth,wght].ttf"),
    );

    let local_candidates = [
        "assets/fonts/Roboto[wdth,wght].ttf",
        "assets/fonts/Roboto-Regular.ttf",
        "assets/fonts/Inter-Regular.ttf",
        "assets/fonts/NotoSansJP-Regular.ttf",
        "assets/fonts/NotoSansJP-Regular.otf",
        "assets/fonts/NotoSansJP[wght].ttf",
        "assets/fonts/NotoSansCJKjp-Regular.otf",
    ];
    for candidate in local_candidates {
        paths.push(PathBuf::from(candidate));
    }

    if cfg!(target_os = "windows") {
        let windows_candidates = [
            r"C:\Windows\Fonts\NotoSansJP-Regular.ttf",
            r"C:\Windows\Fonts\meiryo.ttc",
            r"C:\Windows\Fonts\meiryob.ttc",
            r"C:\Windows\Fonts\YuGothR.ttc",
            r"C:\Windows\Fonts\YuGothM.ttc",
            r"C:\Windows\Fonts\YuGothB.ttc",
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\seguisb.ttf",
        ];
        for candidate in windows_candidates {
            paths.push(PathBuf::from(candidate));
        }
    }

    let unix_candidates = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/Library/Fonts/NotoSansJP-Regular.otf",
        "/System/Library/Fonts/Helvetica.ttc",
    ];
    for candidate in unix_candidates {
        paths.push(PathBuf::from(candidate));
    }

    paths
}

fn candidate_cjk_font_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = std::env::var_os("PLITA_UI_FONT_FALLBACK") {
        paths.push(PathBuf::from(path));
    }

    for candidate in [
        "assets/fonts/NotoSansJP-Regular.ttf",
        "assets/fonts/NotoSansJP-Regular.otf",
        "assets/fonts/NotoSansJP[wght].ttf",
        "assets/fonts/NotoSansCJKjp-Regular.otf",
    ] {
        paths.push(PathBuf::from(candidate));
    }
    if cfg!(target_os = "windows") {
        for candidate in [
            r"C:\Windows\Fonts\NotoSansJP-Regular.ttf",
            r"C:\Windows\Fonts\meiryo.ttc",
            r"C:\Windows\Fonts\YuGothR.ttc",
            r"C:\Windows\Fonts\YuGothM.ttc",
        ] {
            paths.push(PathBuf::from(candidate));
        }
    }
    for candidate in [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.ttf",
        "/Library/Fonts/NotoSansJP-Regular.otf",
    ] {
        paths.push(PathBuf::from(candidate));
    }
    paths
}

fn candidate_memory_font_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(path) = std::env::var_os("PLITA_MEMORY_FONT") {
        paths.push(PathBuf::from(path));
    }

    paths.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("fonts")
            .join("RobotoMono-Regular.ttf"),
    );
    paths.push(PathBuf::from("assets/fonts/RobotoMono-Regular.ttf"));
    paths
}

fn draw_memory_text(list: &mut DrawList, x: f32, y: f32, text: &str, scale: f32, color: Color) {
    if memory_font_rasterizer()
        .is_some_and(|rasterizer| rasterizer.draw_text(list, x, y, text, scale, color))
    {
        return;
    }
    draw_bitmap_text(list, x, y, text, scale, color);
}

impl RichFontRasterizer {
    fn font_for_char(&self, ch: char) -> &fontdue::Font {
        if self.font.lookup_glyph_index(ch) != 0 {
            &self.font
        } else {
            self.fallback_font
                .as_ref()
                .filter(|font| font.lookup_glyph_index(ch) != 0)
                .unwrap_or(&self.font)
        }
    }

    fn size_px(scale: f32) -> f32 {
        (scale.max(0.5) * 10.0_f32).clamp(10.0_f32, 36.0_f32)
    }

    fn advance_width(value: f32) -> f32 {
        if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        }
    }

    fn measure_text_width(&self, text: &str, scale: f32) -> f32 {
        let size_px = Self::size_px(scale);
        let mut width = 0.0_f32;
        let mut line_width = 0.0_f32;
        for ch in text.chars() {
            match ch {
                '\n' => {
                    width = width.max(line_width);
                    line_width = 0.0;
                }
                '\r' => {}
                '\t' => line_width += size_px * 1.6_f32,
                ' ' => line_width += size_px * 0.38_f32,
                _ => {
                    let metrics = self.font_for_char(ch).metrics(ch, size_px);
                    line_width += Self::advance_width(metrics.advance_width);
                }
            }
        }
        width.max(line_width)
    }

    fn draw_text(
        &self,
        list: &mut DrawList,
        x: f32,
        y: f32,
        text: &str,
        scale: f32,
        color: Color,
    ) -> bool {
        if text.is_empty() {
            return true;
        }

        let image = self.text_image(text, scale, color);
        if image.width == 0 || image.height == 0 {
            return false;
        }
        list.image_rgba8(
            Rect::new(
                x + image.offset_x,
                y + image.offset_y,
                image.width as f32,
                image.height as f32,
            ),
            image.width,
            image.height,
            Arc::clone(&image.pixels),
        );
        true
    }

    fn text_image(&self, text: &str, scale: f32, color: Color) -> TextImage {
        let size_px = Self::size_px(scale);
        let key = TextImageKey {
            text: text.to_string(),
            size_tenths: (size_px * 10.0).round() as u16,
            rgba: color_to_rgba8(color),
        };
        let cache = TEXT_IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(cache) = cache.lock() {
            if let Some(image) = cache.get(&key) {
                return image.clone();
            }
        }

        let image = self.rasterize_text_image(text, size_px, key.rgba);
        if let Ok(mut cache) = cache.lock() {
            if cache.len() >= TEXT_IMAGE_CACHE_LIMIT {
                cache.clear();
            }
            cache.insert(key, image.clone());
        }
        image
    }

    fn rasterize_text_image(&self, text: &str, size_px: f32, rgba: [u8; 4]) -> TextImage {
        struct GlyphBitmap {
            x: f32,
            y: f32,
            width: usize,
            height: usize,
            bitmap: Vec<u8>,
        }

        let line_height = size_px * 1.35_f32;
        let baseline = size_px * 0.92_f32;
        let mut cursor_x = 0.0_f32;
        let mut cursor_y = 0.0_f32;
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut glyphs = Vec::new();

        for ch in text.chars() {
            match ch {
                '\n' => {
                    cursor_x = 0.0;
                    cursor_y += line_height;
                    continue;
                }
                '\r' => continue,
                '\t' => {
                    cursor_x += size_px * 1.6_f32;
                    continue;
                }
                ' ' => {
                    cursor_x += size_px * 0.38_f32;
                    continue;
                }
                _ => {}
            }

            let (metrics, bitmap) = self.font_for_char(ch).rasterize(ch, size_px);
            if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
                cursor_x += Self::advance_width(metrics.advance_width);
                continue;
            }

            let glyph_x = cursor_x + metrics.xmin as f32;
            let glyph_y = cursor_y + baseline - (metrics.ymin as f32 + metrics.height as f32);
            min_x = min_x.min(glyph_x);
            min_y = min_y.min(glyph_y);
            max_x = max_x.max(glyph_x + metrics.width as f32);
            max_y = max_y.max(glyph_y + metrics.height as f32);
            glyphs.push(GlyphBitmap {
                x: glyph_x,
                y: glyph_y,
                width: metrics.width,
                height: metrics.height,
                bitmap,
            });
            cursor_x += Self::advance_width(metrics.advance_width);
        }

        if glyphs.is_empty() || !min_x.is_finite() || !min_y.is_finite() {
            return TextImage {
                width: 0,
                height: 0,
                offset_x: 0.0,
                offset_y: 0.0,
                pixels: Arc::<[u8]>::from(Vec::<u8>::new()),
            };
        }

        let offset_x = min_x.floor();
        let offset_y = min_y.floor();
        let width = (max_x.ceil() - offset_x).max(1.0) as u32;
        let height = (max_y.ceil() - offset_y).max(1.0) as u32;
        let mut pixels = vec![0_u8; width as usize * height as usize * 4];
        for glyph in glyphs {
            let base_x = (glyph.x - offset_x).round() as i32;
            let base_y = (glyph.y - offset_y).round() as i32;
            for row in 0..glyph.height {
                for col in 0..glyph.width {
                    let coverage = glyph.bitmap[row * glyph.width + col];
                    if coverage <= 10 {
                        continue;
                    }
                    let px = base_x + col as i32;
                    let py = base_y + row as i32;
                    if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                        continue;
                    }
                    let dst = (py as usize * width as usize + px as usize) * 4;
                    let alpha = ((u16::from(rgba[3]) * u16::from(coverage)) / 255) as u8;
                    pixels[dst] = rgba[0];
                    pixels[dst + 1] = rgba[1];
                    pixels[dst + 2] = rgba[2];
                    pixels[dst + 3] = alpha;
                }
            }
        }
        TextImage {
            width,
            height,
            offset_x,
            offset_y,
            pixels: Arc::<[u8]>::from(pixels),
        }
    }
}

fn color_to_rgba8(color: Color) -> [u8; 4] {
    [
        (color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// Measure text width using the same lightweight metrics as `draw_text`.
///
/// This is intentionally approximate for Phase 1, but it keeps the TextInput
/// caret next to the last visible glyph instead of using a fixed byte-count offset.
pub fn measure_text_width(text: &str, scale: f32) -> f32 {
    if let Some(rasterizer) = rich_font_rasterizer() {
        return rasterizer.measure_text_width(text, scale);
    }
    measure_bitmap_text_width(text, scale)
}

fn measure_bitmap_text_width(text: &str, scale: f32) -> f32 {
    let scale = scale.max(0.5);
    let mut width = 0.0_f32;
    let mut line_width = 0.0_f32;
    for ch in text.chars() {
        match ch {
            '\n' => {
                width = width.max(line_width);
                line_width = 0.0;
            }
            '\r' => {}
            ' ' => line_width += 4.0 * scale,
            _ => line_width += 6.0 * scale,
        }
    }
    width.max(line_width)
}

/// Draw text using the best available Phase 1 backend.
///
/// Order:
/// 1. Bundled Roboto, or another local TTF/OTF/TTC font from `PLITA_UI_FONT`,
///    `assets/fonts/`, or common system font locations.
/// 2. Clean-room built-in 5x7 bitmap fallback.
///
/// `PLITA_UI_FONT` still takes precedence for apps that need a different UI
/// typeface.
pub fn draw_text(list: &mut DrawList, x: f32, y: f32, text: &str, scale: f32, color: Color) {
    if let Some(rasterizer) = rich_font_rasterizer() {
        if rasterizer.draw_text(list, x, y, text, scale, color) {
            return;
        }
    }
    draw_bitmap_text(list, x, y, text, scale, color);
}

/// Draw ASCII text using a tiny built-in 5x7 bitmap font.
///
/// This is the fallback renderer. It is clean-room and intentionally tiny.
pub fn draw_bitmap_text(list: &mut DrawList, x: f32, y: f32, text: &str, scale: f32, color: Color) {
    let scale = (scale * BITMAP_SCALE_FACTOR).max(0.75);
    let mut cursor_x = x;
    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += BITMAP_SPACE_ADVANCE * scale;
            continue;
        }
        if let Some(rows) = glyph_5x7(ch) {
            for (row_index, row_bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if (row_bits >> (4 - col)) & 1 == 1 {
                        list.fill_rect(
                            Rect::new(
                                cursor_x + col as f32 * scale,
                                y + row_index as f32 * scale,
                                scale,
                                scale,
                            ),
                            color,
                        );
                    }
                }
            }
        }
        cursor_x += BITMAP_GLYPH_ADVANCE * scale;
    }
}

const BITMAP_SCALE_FACTOR: f32 = 1.25;
const BITMAP_SPACE_ADVANCE: f32 = 4.0;
const BITMAP_GLYPH_ADVANCE: f32 = 6.0;

#[cfg(test)]
mod memory_viewer_hit_tests {
    use super::*;

    #[test]
    fn visible_hex_cells_map_to_their_drawn_byte_addresses() {
        let rect = Rect::new(16., 134., 480., 356.);
        for row in 0..21 {
            for col in 0..8 {
                let cell = memory_viewer_byte_rect(rect, row, col);
                let point = Point::new(cell.x + cell.width / 2., cell.y + 3.);
                assert_eq!(
                    memory_viewer_address_at_point(rect, 0xc000, 168, point),
                    Some(0xc000 + (row * 8 + col) as u16)
                );
            }
        }
    }

    #[test]
    fn labels_ascii_gaps_and_missing_or_hidden_bytes_are_not_editable() {
        let rect = Rect::new(16., 134., 480., 356.);
        let first = memory_viewer_byte_rect(rect, 0, 0);
        for point in [
            Point::new(rect.x + 12., first.y + 3.),
            Point::new(first.x - 1., first.y + 3.),
            Point::new(first.x, rect.y + 28.),
            Point::new(rect.x + 260., first.y + 3.),
            Point::new(first.x, first.y + 21. * 14. + 3.),
        ] {
            assert_eq!(
                memory_viewer_address_at_point(rect, 0xc000, 176, point),
                None
            );
        }
        let missing = memory_viewer_byte_rect(rect, 1, 1);
        assert_eq!(
            memory_viewer_address_at_point(
                rect,
                0xc000,
                9,
                Point::new(missing.x + 2., missing.y + 3.)
            ),
            None
        );
    }
}

fn glyph_5x7(ch: char) -> Option<[u8; 7]> {
    let c = ch.to_ascii_uppercase();
    let rows = match c {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '|' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        _ => return None,
    };
    Some(rows)
}

/// Return the ordinary logical key portion before any diagnostic physical-key suffix.
///
/// The window layer may pass values such as `!|Code(Digit1)` so the UI can
/// recognize top-row number keys across keyboard layouts.  Letter shortcuts and
/// text-editing keys must ignore the suffix.
fn logical_key_part(key: &str) -> &str {
    key.split('|').next().unwrap_or(key).trim()
}

/// Return a numeric shortcut digit from a logical-key diagnostic string.
///
/// On Windows/winit, Ctrl+Shift+number can arrive as the shifted character
/// (`!`, `\"`, `#`, `$`, `%`, `^`, `&`, `)`) instead of the unshifted digit,
/// depending on keyboard layout. Floating dock shortcuts must therefore be
/// parsed semantically rather than by comparing only to "1".."6".
fn shortcut_digit_from_key(upper: &str) -> Option<u8> {
    // Accept both logical text (`1`, `!`) and physical-key debug text such as
    // `PhysicalKey::Code(Digit1)`, `Code(Digit1)`, or `Code(Key1)`.  Some
    // Windows/Japanese layouts also expose top-row number keys as native codes,
    // so common ASCII/native/scancode diagnostics are accepted as a fallback.
    let compact = upper.replace(' ', "");
    let logical = logical_key_part(&compact);

    let logical_digit = match logical {
        "0" | ")" | "NUMPAD0" | "NUM0" => Some(0),
        "1" | "!" | "NUMPAD1" | "NUM1" => Some(1),
        "2" | "@" | "\"" | "NUMPAD2" | "NUM2" => Some(2),
        "3" | "#" | "NUMPAD3" | "NUM3" => Some(3),
        "4" | "$" | "NUMPAD4" | "NUM4" => Some(4),
        "5" | "%" | "NUMPAD5" | "NUM5" => Some(5),
        "6" | "^" | "&" | "NUMPAD6" | "NUM6" => Some(6),
        _ => None,
    };
    if logical_digit.is_some() {
        return logical_digit;
    }

    (0_u8..=6).find(|&digit| compact_contains_digit_key(&compact, digit))
}

fn hex_digit_from_key(key: &str, upper: &str) -> Option<u8> {
    let compact = upper.replace(' ', "");
    let logical = logical_key_part(&compact);
    let direct = match logical {
        "0" | ")" | "NUMPAD0" | "NUM0" => Some(0),
        "1" | "!" | "NUMPAD1" | "NUM1" => Some(1),
        "2" | "@" | "\"" | "NUMPAD2" | "NUM2" => Some(2),
        "3" | "#" | "NUMPAD3" | "NUM3" => Some(3),
        "4" | "$" | "NUMPAD4" | "NUM4" => Some(4),
        "5" | "%" | "NUMPAD5" | "NUM5" => Some(5),
        "6" | "^" | "&" | "NUMPAD6" | "NUM6" => Some(6),
        "7" | "'" | "NUMPAD7" | "NUM7" => Some(7),
        "8" | "(" | "NUMPAD8" | "NUM8" => Some(8),
        "9" | "NUMPAD9" | "NUM9" => Some(9),
        "A" => Some(10),
        "B" => Some(11),
        "C" => Some(12),
        "D" => Some(13),
        "E" => Some(14),
        "F" => Some(15),
        _ => None,
    };
    if direct.is_some() {
        return direct;
    }

    let mut chars = key.chars();
    let ch = chars.next()?;
    if chars.next().is_none() {
        ch.to_digit(16).map(|digit| digit as u8)
    } else {
        None
    }
}

fn compact_contains_digit_key(compact: &str, digit: u8) -> bool {
    let ascii = 48_u8 + digit;
    let pc_scan = if digit == 0 { 11 } else { digit + 1 };

    let basic_match = compact.contains(&format!("DIGIT{}", digit))
        || compact.contains(&format!("KEY{}", digit))
        || compact.contains(&format!("NUMPAD{}", digit))
        || compact.contains(&format!("CODE({})", digit))
        // Winit/Windows may expose top-row digits as either virtual-key-like
        // native codes (`0` => 48 / 0x30) or PC set-1 style scan codes
        // (`0` => 11, `1` => 2, ...).  Accept both NativeKeyCode(...) and
        // Windows(...) wrappers because actual debug strings differ between
        // winit/platform paths.
        || compact.contains(&format!("NATIVEKEYCODE({})", ascii))
        || compact.contains(&format!("NATIVEKEYCODE(0X{:X})", ascii))
        || compact.contains(&format!("WINDOWS({})", ascii))
        || compact.contains(&format!("WINDOWS(0X{:X})", ascii))
        || compact.contains(&format!("NATIVEKEYCODE({})", pc_scan))
        || compact.contains(&format!("NATIVEKEYCODE(0X{:X})", pc_scan))
        || compact.contains(&format!("WINDOWS({})", pc_scan))
        || compact.contains(&format!("WINDOWS(0X{:X})", pc_scan))
        || compact.contains(&format!("SCANCODE({})", pc_scan))
        || compact.contains(&format!("SCANCODE(0X{:X})", pc_scan));

    if basic_match {
        return true;
    }

    // JIS/Windows Ctrl+Shift+0 has been observed to reach the UI as an
    // unidentified zero-like native key, while Ctrl+0 still arrives normally.
    // These zero-only fallbacks are deliberately not applied to 1..6 so Ctrl+1
    // tab shortcuts remain conservative.
    digit == 0
        && (compact.contains("NATIVEKEYCODE(0)")
            || compact.contains("WINDOWS(0)")
            || compact.contains("VIRTUALKEYCODE(0)")
            || compact.contains("KEYCODE::DIGIT0")
            || compact.contains("PHYSICALKEY::CODE(DIGIT0)")
            || compact.contains("CODE(DIGIT0)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_points() {
        let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert!(rect.contains(Point::new(10.0, 20.0)));
        assert!(rect.contains(Point::new(40.0, 60.0)));
        assert!(!rect.contains(Point::new(41.0, 60.0)));
        assert!(!rect.contains(Point::new(40.0, 61.0)));
    }

    #[test]
    fn layout_keeps_canvas_visible() {
        let layout = CreativeLayout::compute(Size::new(1280.0, 800.0));
        assert!(layout.canvas.width > 700.0);
        assert!(layout.canvas.height > 500.0);
        assert_eq!(layout.menu_bar.y, 0.0);
        assert!(layout.status_bar.y > layout.timeline.y);
        assert!(layout.left_splitter.width > 0.0);
        assert!(layout.right_splitter.width > 0.0);
    }

    #[test]
    fn demo_draw_list_contains_commands() {
        let list = phase1_demo_draw_list(Size::new(1280.0, 800.0));
        assert!(list.len() > 100);
    }

    #[test]
    fn draw_list_accepts_valid_rgba_image_payload() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.image_rgba8(Rect::new(4.0, 8.0, 32.0, 16.0), 2, 2, vec![255; 16]);

        assert!(matches!(
            list.commands(),
            [DrawCommand::ImageRgba {
                rect,
                width: 2,
                height: 2,
                pixels
            }] if *rect == Rect::new(4.0, 8.0, 32.0, 16.0) && pixels.len() == 16
        ));
    }

    #[test]
    fn draw_list_accepts_shared_rgba_image_payload() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let pixels = Arc::new(Mutex::new(vec![255; 16]));
        list.image_rgba8_shared(Rect::new(4.0, 8.0, 32.0, 16.0), 2, 2, Arc::clone(&pixels));

        assert!(matches!(
            list.commands(),
            [DrawCommand::ImageRgbaShared {
                rect,
                width: 2,
                height: 2,
                pixels: command_pixels
            }] if *rect == Rect::new(4.0, 8.0, 32.0, 16.0)
                && Arc::ptr_eq(command_pixels, &pixels)
        ));
    }

    #[test]
    fn draw_list_accepts_vertical_gradient_rect() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let rect = Rect::new(4.0, 8.0, 32.0, 16.0);
        let top = Color::rgb8(16, 24, 32);
        let bottom = Color::rgb8(48, 56, 64);
        list.fill_vertical_gradient_rect(rect, top, bottom);

        assert!(matches!(
            list.commands(),
            [DrawCommand::FillRectCorners {
                rect: command_rect,
                top_left,
                top_right,
                bottom_right,
                bottom_left
            }] if *command_rect == rect
                && *top_left == top
                && *top_right == top
                && *bottom_right == bottom
                && *bottom_left == bottom
        ));
    }

    #[test]
    fn draw_list_accepts_triangle_mesh() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let vertices = vec![
            ColoredVertex::new(Point::new(0.0, 0.0), Color::rgb8(255, 0, 0)),
            ColoredVertex::new(Point::new(8.0, 0.0), Color::rgb8(0, 255, 0)),
            ColoredVertex::new(Point::new(0.0, 8.0), Color::rgb8(0, 0, 255)),
        ];
        list.fill_triangle_list(vertices.clone());

        assert!(matches!(
            list.commands(),
            [DrawCommand::FillTriangles { vertices: command_vertices }]
                if command_vertices.as_ref() == vertices.as_slice()
        ));
    }

    #[test]
    fn draw_list_accepts_rounded_rect_mesh() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.fill_rounded_vertical_gradient_rect(
            Rect::new(4.0, 8.0, 32.0, 16.0),
            6.0,
            Color::rgb8(16, 24, 32),
            Color::rgb8(48, 56, 64),
        );

        assert!(matches!(
            list.commands(),
            [DrawCommand::FillTriangles { vertices }] if vertices.len() >= 12
        ));
    }

    #[test]
    fn draw_list_accepts_radial_gradient_mesh() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.fill_radial_gradient_rect_steps(
            Rect::new(0.0, 0.0, 16.0, 16.0),
            Point::new(8.0, 0.0),
            18.0,
            Color::rgba8(255, 255, 255, 96),
            Color::rgba8(255, 255, 255, 0),
            4,
        );

        assert!(matches!(
            list.commands(),
            [DrawCommand::FillTriangles { vertices }] if vertices.len() == 4 * 4 * 6
        ));
    }

    #[test]
    fn draw_panel_emits_gradient_edges_and_border() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let rect = Rect::new(4.0, 8.0, 32.0, 16.0);
        let mut style = PanelStyle::new(
            Color::rgb8(20, 30, 40),
            Color::rgb8(10, 15, 20),
            Color::rgb8(80, 90, 100),
        );
        style.highlight = Color::rgba8(255, 255, 255, 32);
        style.shadow = Color::rgba8(0, 0, 0, 64);
        style.border_thickness = 2.0;
        style.highlight_thickness = 2.0;
        style.shadow_thickness = 3.0;

        draw_panel(&mut list, rect, style);

        assert_eq!(list.len(), 8);
        assert!(matches!(
            list.commands().first(),
            Some(DrawCommand::FillRectCorners { rect: command_rect, .. })
                if *command_rect == rect
        ));
        assert!(list.commands().iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color }
                if *rect == Rect::new(6.0, 10.0, 28.0, 2.0) && *color == style.highlight
        )));
        assert!(list.commands().iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color }
                if *rect == Rect::new(6.0, 19.0, 28.0, 3.0) && *color == style.shadow
        )));
        assert!(list.commands().iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color }
                if *rect == Rect::new(4.0, 8.0, 32.0, 2.0) && *color == style.border
        )));
    }

    #[test]
    fn draw_panel_can_emit_rounded_shadowed_surface() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let mut style = PanelStyle::new(
            Color::rgb8(42, 52, 66),
            Color::rgb8(24, 32, 42),
            Color::rgb8(96, 116, 140),
        );
        style.corner_radius = 6.0;
        style.box_shadow = Some(BoxShadow {
            offset: Point::new(0.0, 4.0),
            blur: 8.0,
            spread: 1.0,
            color: Color::rgba8(0, 0, 0, 96),
            corner_radius: 6.0,
            steps: 4,
        });
        style.radial_highlight = Some(PanelRadialHighlight::new(
            Point::new(0.5, 0.0),
            0.9,
            Color::rgba8(255, 255, 255, 24),
        ));

        draw_panel(&mut list, Rect::new(10.0, 12.0, 80.0, 44.0), style);

        let mesh_count = list
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::FillTriangles { .. }))
            .count();
        assert!(mesh_count >= 4);
    }

    #[test]
    fn memory_viewer_draws_compact_hex_commands() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        draw_memory_viewer(
            &mut list,
            Rect::new(10.0, 20.0, 220.0, 160.0),
            "MEM VRAM $8000",
            0x8000,
            &(0..32).collect::<Vec<u8>>(),
            MemoryViewerPalette::dark(),
        );

        assert!(list.len() > 10);
        assert!(list
            .commands()
            .iter()
            .any(|command| matches!(command, DrawCommand::PushClip { .. })));
    }

    #[test]
    fn memory_editor_hex_digit_edits_high_then_low_nibble() {
        let mut editor = MemoryEditorState::new();
        let mut bytes = vec![0x03, 0x44];

        let high = editor
            .edit_hex_digit(&mut bytes, 0x0a)
            .expect("high nibble edit should apply");
        assert_eq!(
            high,
            MemoryEdit {
                offset: 0,
                old_value: 0x03,
                new_value: 0xa3,
            }
        );
        assert_eq!(bytes[0], 0xa3);
        assert_eq!(editor.selected_offset, 0);
        assert!(!editor.high_nibble);

        let low = editor
            .edit_hex_digit(&mut bytes, 0x0f)
            .expect("low nibble edit should apply");
        assert_eq!(
            low,
            MemoryEdit {
                offset: 0,
                old_value: 0xa3,
                new_value: 0xaf,
            }
        );
        assert_eq!(bytes[0], 0xaf);
        assert_eq!(editor.selected_offset, 1);
        assert!(editor.high_nibble);
    }

    #[test]
    fn memory_editor_hit_tests_hex_cell() {
        let rect = Rect::new(10.0, 20.0, 360.0, 220.0);
        let cell = MemoryEditorState::byte_rect(rect, 5);
        let center = Point::new(cell.x + cell.width * 0.5, cell.y + cell.height * 0.5);

        assert_eq!(MemoryEditorState::hit_offset(rect, center, 16), Some(5));
    }

    #[test]
    fn creative_shell_memory_editor_edits_memory_bytes() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.set_memory_viewer_bytes(0xc000, vec![0x03, 0x44]);
        shell.resize_memory_window(Size::new(500.0, 360.0));

        let editor_rect = shell.detached_memory_editor_rect();
        let cell = MemoryEditorState::byte_rect_scaled(editor_rect, 0, DETACHED_MEMORY_FONT_SCALE);
        let center = Point::new(cell.x + cell.width * 0.5, cell.y + cell.height * 0.5);
        shell.handle_memory_window_event(UiEvent::PointerDown {
            position: center,
            button: PointerButton::Primary,
        });
        shell.handle_memory_window_event(UiEvent::KeyDown {
            key: "A".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        shell.handle_memory_window_event(UiEvent::KeyDown {
            key: "F".to_string(),
            ctrl: false,
            shift: true,
            alt: false,
            meta: false,
        });

        assert_eq!(shell.memory_viewer().bytes[0], 0xaf);
        assert_eq!(shell.memory_editor().selected_offset, 1);
        assert!(shell.memory_editor().high_nibble);
    }

    #[test]
    fn memory_editor_uses_roboto_mono_numeric_font() {
        assert_eq!(active_memory_font_family(), "Roboto Mono");
    }

    #[test]
    fn detached_memory_window_edits_parent_memory_bytes() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.set_memory_viewer_bytes(0xc000, vec![0x03, 0x44]);
        shell.resize_memory_window(Size::new(500.0, 360.0));

        let cell = MemoryEditorState::byte_rect_scaled(
            shell.detached_memory_editor_rect(),
            0,
            DETACHED_MEMORY_FONT_SCALE,
        );
        let center = Point::new(cell.x + cell.width * 0.5, cell.y + cell.height * 0.5);
        shell.handle_memory_window_event(UiEvent::PointerDown {
            position: center,
            button: PointerButton::Primary,
        });
        shell.handle_memory_window_event(UiEvent::KeyDown {
            key: "A".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        shell.handle_memory_window_event(UiEvent::KeyDown {
            key: "F".to_string(),
            ctrl: false,
            shift: true,
            alt: false,
            meta: false,
        });

        assert_eq!(shell.memory_viewer().bytes[0], 0xaf);
        assert_eq!(shell.memory_editor().selected_offset, 1);
        assert!(shell.memory_editor().high_nibble);
    }

    #[test]
    fn detached_memory_window_draws_vertical_scrollbar_for_long_memory() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.set_memory_viewer_bytes(0x8000, (0..=255).cycle().take(512).collect());
        shell.resize_memory_window(Size::new(420.0, 220.0));
        let list = shell.memory_window_draw_list();

        assert!(list.commands().iter().any(|command| {
            matches!(
                command,
                DrawCommand::FillRect { rect, .. }
                    if rect.x >= 410.0 && rect.width <= 5.0 && rect.height > 100.0
            )
        }));
    }

    #[test]
    fn draw_list_rejects_mismatched_rgba_image_payload() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.image_rgba8(Rect::new(4.0, 8.0, 32.0, 16.0), 2, 2, vec![255; 15]);
        list.image_rgba8(Rect::new(4.0, 8.0, 0.0, 16.0), 2, 2, vec![255; 16]);

        assert!(list.is_empty());
    }

    #[test]
    fn draw_list_accepts_clip_commands() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        let clip = Rect::new(4.0, 8.0, 32.0, 16.0);
        list.push_clip(clip);
        list.fill_rect(Rect::new(6.0, 10.0, 8.0, 8.0), Color::rgb8(255, 255, 255));
        list.pop_clip();

        assert!(matches!(
            list.commands(),
            [
                DrawCommand::PushClip { rect },
                DrawCommand::FillRect { .. },
                DrawCommand::PopClip
            ] if *rect == clip
        ));
    }

    #[test]
    fn text_backend_emits_commands() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        draw_text(&mut list, 0.0, 0.0, "A1", 1.0, Color::rgb8(255, 255, 255));
        assert!(!list.is_empty());
    }

    #[test]
    fn bitmap_text_fallback_emits_rectangles() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        draw_bitmap_text(&mut list, 0.0, 0.0, "A1", 1.0, Color::rgb8(255, 255, 255));
        assert!(!list.is_empty());
    }

    #[test]
    fn widget_tree_hit_tests_tool_button() {
        let shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let tree = shell.widget_tree();
        let select_button = tree
            .nodes()
            .iter()
            .find(|node| node.role == WidgetRole::ToolButton(ToolKind::Select))
            .expect("select tool button node should exist");
        let p = Point::new(select_button.rect.x + 4.0, select_button.rect.y + 4.0);
        let hit = tree.hit_test(p).expect("select tool button should be hit");
        assert_eq!(hit.role, WidgetRole::ToolButton(ToolKind::Select));
    }

    #[test]
    fn clicking_box_tool_changes_selected_tool() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let box_button = shell
            .widget_tree()
            .nodes()
            .iter()
            .find(|node| node.role == WidgetRole::ToolButton(ToolKind::Box))
            .unwrap()
            .rect;
        let p = Point::new(box_button.x + 4.0, box_button.y + 4.0);
        shell.handle_event(UiEvent::PointerMove { position: p });
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.selected_tool(), ToolKind::Box);
    }

    #[test]
    fn left_splitter_drag_changes_left_width() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let start_width = shell.layout().tool_bar.width;
        let splitter = shell.layout().left_splitter;
        let p0 = Point::new(splitter.x + 2.0, splitter.y + 20.0);
        let p1 = Point::new(p0.x + 40.0, p0.y);
        shell.handle_event(UiEvent::PointerDown {
            position: p0,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove { position: p1 });
        shell.handle_event(UiEvent::PointerUp {
            position: p1,
            button: PointerButton::Primary,
        });
        assert!(shell.layout().tool_bar.width > start_width);
    }

    #[test]
    fn keyboard_shortcut_selects_pan_tool() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "2".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.selected_tool(), ToolKind::Pan);
    }

    #[test]
    fn canvas_wheel_changes_zoom() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let canvas = shell.layout().canvas;
        let before = shell.canvas_zoom();
        shell.handle_event(UiEvent::PointerWheel {
            position: Point::new(canvas.x + 40.0, canvas.y + 40.0),
            delta_x: 0.0,
            delta_y: 32.0,
        });
        assert!(shell.canvas_zoom() > before);
    }

    #[test]
    fn inspector_and_bottom_wheel_scroll_both_axes() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let inspector = shell.layout().inspector;
        let response = shell.handle_event(UiEvent::PointerWheel {
            position: Point::new(inspector.x + 24.0, inspector.y + 96.0),
            delta_x: -48.0,
            delta_y: -96.0,
        });
        assert!(response.needs_redraw);
        assert!(shell.inspector_scroll().x > 0.0);
        assert!(shell.inspector_scroll().y > 0.0);

        let timeline = shell.layout().timeline;
        let response = shell.handle_event(UiEvent::PointerWheel {
            position: Point::new(timeline.x + 120.0, timeline.y + 80.0),
            delta_x: -64.0,
            delta_y: -112.0,
        });
        assert!(response.needs_redraw);
        assert!(shell.bottom_scroll().x > 0.0);
        assert!(shell.bottom_scroll().y > 0.0);
    }

    #[test]
    fn menu_click_opens_active_menu() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let file_menu = shell
            .widget_tree()
            .nodes()
            .iter()
            .find(|node| node.role == WidgetRole::MenuItem(MenuKind::File))
            .unwrap()
            .rect;
        let p = Point::new(file_menu.x + 5.0, file_menu.y + 5.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.active_menu(), Some(MenuKind::File));
    }

    #[test]
    fn select_tool_can_select_existing_canvas_object() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let object_id = shell.objects()[0].id;
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p = Point::new(screen_rect.x + 4.0, screen_rect.y + 4.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.selected_object(), Some(object_id));
    }

    #[test]
    fn box_tool_creates_object_and_undo_removes_it() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        let canvas = shell.layout().canvas;
        let before = shell.objects().len();
        let p = Point::new(canvas.x + 240.0, canvas.y + 180.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.objects().len(), before + 1);
        assert_eq!(shell.undo_len(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "z".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), before);
        assert_eq!(shell.redo_len(), 1);
    }

    #[test]
    fn dragging_object_records_move_command() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let start_rect = shell.objects()[0].rect;
        let screen_rect = shell.world_rect_to_screen(start_rect);
        let p0 = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        let p1 = Point::new(p0.x + 50.0, p0.y + 20.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p0,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove { position: p1 });
        shell.handle_event(UiEvent::PointerUp {
            position: p1,
            button: PointerButton::Primary,
        });
        assert_ne!(shell.objects()[0].rect, start_rect);
        assert_eq!(shell.undo_len(), 1);
    }

    #[test]
    fn resizing_selected_object_records_resize_command() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let object_id = shell.objects()[0].id;
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let select_p = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: select_p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: select_p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.selected_object(), Some(object_id));

        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p0 = Point::new(screen_rect.right(), screen_rect.bottom());
        let p1 = Point::new(p0.x + 30.0, p0.y + 20.0);
        let before = shell.objects()[0].rect;
        shell.handle_event(UiEvent::PointerDown {
            position: p0,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove { position: p1 });
        shell.handle_event(UiEvent::PointerUp {
            position: p1,
            button: PointerButton::Primary,
        });
        assert!(shell.objects()[0].rect.width > before.width);
        assert!(shell.objects()[0].rect.height > before.height);
        assert_eq!(shell.undo_len(), 1);
    }

    #[test]
    fn delete_selected_object_can_be_undone() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let object_id = shell.objects()[0].id;
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "Delete".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(shell.objects().iter().all(|object| object.id != object_id));
        shell.handle_event(UiEvent::KeyDown {
            key: "z".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(shell.objects().iter().any(|object| object.id == object_id));
    }

    #[test]
    fn duplicate_and_nudge_selected_object() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        let before_len = shell.objects().len();
        shell.handle_event(UiEvent::KeyDown {
            key: "d".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), before_len + 1);
        let selected = shell
            .selected_object()
            .expect("duplicated object should be selected");
        let before = shell.object_rect(selected).unwrap();
        shell.handle_event(UiEvent::KeyDown {
            key: "ArrowRight".to_string(),
            ctrl: false,
            shift: true,
            alt: false,
            meta: false,
        });
        let after = shell.object_rect(selected).unwrap();
        assert_eq!(after.x, before.x + 10.0);
    }

    #[test]
    fn copy_paste_selected_object_creates_undoable_rect() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "c".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.clipboard_len(), 1);
        let before_len = shell.objects().len();
        shell.handle_event(UiEvent::KeyDown {
            key: "v".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), before_len + 1);
        assert_eq!(shell.undo_len(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "z".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), before_len);
    }

    #[test]
    fn ctrl_copy_paste_accepts_physical_metadata_suffix() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "c|Code(KeyC)".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.clipboard_len(), 1);
        let before_len = shell.objects().len();
        shell.handle_event(UiEvent::KeyDown {
            key: "v|Code(KeyV)".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), before_len + 1);
    }

    #[test]
    fn ctrl_shift_z_accepts_physical_metadata_suffix() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let canvas = shell.layout().canvas;
        let p = Point::new(canvas.x + 260.0, canvas.y + 200.0);
        shell.selected_tool = ToolKind::Box;
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        let after_create = shell.objects().len();
        shell.handle_event(UiEvent::KeyDown {
            key: "z|Code(KeyZ)".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), after_create - 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "Z|Code(KeyZ)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), after_create);
    }

    #[test]
    fn cut_selected_object_copies_and_deletes_with_undo() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let object_id = shell.objects()[0].id;
        let screen_rect = shell.world_rect_to_screen(shell.objects()[0].rect);
        let p = Point::new(screen_rect.x + 10.0, screen_rect.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "x".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.clipboard_len(), 1);
        assert!(shell.objects().iter().all(|object| object.id != object_id));
        shell.handle_event(UiEvent::KeyDown {
            key: "z".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(shell.objects().iter().any(|object| object.id == object_id));
    }

    #[test]
    fn snapshot_and_open_reset_are_deterministic() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let snapshot = shell.snapshot_text();
        assert!(snapshot.contains("PLITA_PHASE2_V2"));
        assert!(snapshot.contains("OBJECTS 1"));
        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        let canvas = shell.layout().canvas;
        let p = Point::new(canvas.x + 260.0, canvas.y + 200.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        assert!(shell.objects().len() > 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "o".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.objects().len(), 1);
        assert_eq!(shell.undo_len(), 0);
        assert_eq!(shell.redo_len(), 0);
    }

    #[test]
    fn text_input_accepts_keyboard_and_backspace() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let input = shell.text_input_rect(shell.layout());
        let p = Point::new(input.x + 10.0, input.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert!(shell.text_input().focused);
        shell.handle_event(UiEvent::KeyDown {
            key: "A".to_string(),
            ctrl: false,
            shift: true,
            alt: false,
            meta: false,
        });
        assert!(shell.text_input().text.ends_with('A'));
        shell.handle_event(UiEvent::KeyDown {
            key: "Backspace".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(!shell.text_input().text.ends_with('A'));
    }

    #[test]
    fn text_input_accepts_ime_commit() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let input = shell.text_input_rect(shell.layout());
        let p = Point::new(input.x + 10.0, input.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::ImePreedit {
            text: "あ".to_string(),
        });
        assert_eq!(shell.text_input().ime_preedit, "あ");
        shell.handle_event(UiEvent::ImeCommit {
            text: "やはた".to_string(),
        });
        assert!(shell.text_input().text.contains("やはた"));
        assert!(shell.text_input().ime_preedit.is_empty());
    }

    #[test]
    fn active_menu_popup_command_can_toggle_memory_viewer() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let menu_rect = shell.menu_item_rects(shell.layout())[2].1;
        let p = Point::new(menu_rect.x + 8.0, menu_rect.y + 8.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.active_menu(), Some(MenuKind::View));
        let items = shell.menu_command_rects().expect("view menu commands");
        let memory_item = items[3].1;
        let before = shell.floating_debug_open();
        let cmd_p = Point::new(memory_item.x + 8.0, memory_item.y + 8.0);
        shell.handle_event(UiEvent::PointerDown {
            position: cmd_p,
            button: PointerButton::Primary,
        });
        assert_ne!(shell.floating_debug_open(), before);
    }

    #[test]
    fn text_input_home_end_named_keys_move_cursor() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let input = shell.text_input_rect(shell.layout());
        let p = Point::new(input.x + 10.0, input.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "Named(Home)".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.text_input().cursor, 0);
        shell.handle_event(UiEvent::KeyDown {
            key: "Named(End)".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.text_input().cursor, shell.text_input().text.len());
    }

    #[test]
    fn text_width_measurement_tracks_visible_text() {
        assert!(measure_text_width("aasfa", 1.15) > 0.0);
        assert!(measure_text_width("やはた", 1.15) > 0.0);
    }

    #[test]
    fn rich_font_advance_width_does_not_force_minimum_spacing() {
        assert_eq!(RichFontRasterizer::advance_width(1.25), 1.25);
        assert_eq!(RichFontRasterizer::advance_width(0.0), 0.0);
        assert_eq!(RichFontRasterizer::advance_width(-1.0), 0.0);
    }

    #[test]
    fn default_font_family_is_roboto() {
        assert_eq!(default_font_family(), "Roboto");
        assert_eq!(
            font_family_label_for_source(r"C:\project\assets\fonts\Roboto[wdth,wght].ttf"),
            "Roboto"
        );
    }

    #[test]
    fn bundled_roboto_precedes_other_local_font_candidates() {
        let candidates = candidate_font_paths();
        let absolute_roboto = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("fonts")
            .join("Roboto[wdth,wght].ttf");
        let absolute = candidates
            .iter()
            .position(|path| path == &absolute_roboto)
            .expect("absolute bundled Roboto should be a font candidate");
        let roboto = candidates
            .iter()
            .position(|path| path == &PathBuf::from("assets/fonts/Roboto[wdth,wght].ttf"))
            .expect("bundled Roboto should be a font candidate");
        let inter = candidates
            .iter()
            .position(|path| path == &PathBuf::from("assets/fonts/Inter-Regular.ttf"))
            .expect("Inter candidate should remain available");
        let noto = candidates
            .iter()
            .position(|path| path == &PathBuf::from("assets/fonts/NotoSansJP-Regular.ttf"))
            .expect("Noto Sans JP candidate should remain available");

        assert!(absolute_roboto.exists());
        assert!(absolute < roboto);
        assert!(roboto < inter);
        assert!(roboto < noto);
    }

    #[test]
    fn cjk_fallback_candidates_remain_separate_from_the_primary_font_order() {
        let candidates = candidate_cjk_font_paths();
        assert!(candidates.contains(&PathBuf::from("assets/fonts/NotoSansJP-Regular.ttf")));
        if cfg!(target_os = "windows") {
            assert!(candidates.contains(&PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc")));
        }
    }

    #[test]
    fn rich_rasterizer_uses_an_available_fallback_for_missing_cjk_glyphs() {
        let Some(primary) = load_font_path(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("assets")
                .join("fonts")
                .join("Roboto[wdth,wght].ttf"),
        ) else {
            return;
        };
        let Some(fallback_font) = load_cjk_fallback_font() else {
            return;
        };
        let rasterizer = RichFontRasterizer {
            font: primary,
            source_name: "clean-room primary font fixture".to_owned(),
            fallback_font: Some(fallback_font),
        };
        let cjk = '\u{4e00}';
        assert_eq!(rasterizer.font.lookup_glyph_index(cjk), 0);
        assert_ne!(rasterizer.font_for_char(cjk).lookup_glyph_index(cjk), 0);
    }

    #[test]
    fn inspector_dock_tabs_switch_active_panel() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert_eq!(
            shell.active_inspector_panel(),
            InspectorPanelKind::Properties
        );
        let tab = shell.inspector_tab_rects(shell.layout())[1].1;
        let p = Point::new(tab.x + 8.0, tab.y + 8.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
    }

    #[test]
    fn bottom_dock_tabs_and_shortcuts_switch_active_panel() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Timeline);
        let tab = shell.bottom_tab_rects(shell.layout())[2].1;
        let p = Point::new(tab.x + 8.0, tab.y + 8.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Diagnostics);
        shell.handle_event(UiEvent::KeyDown {
            key: "5".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Log);
    }

    #[test]
    fn floating_dock_shortcuts_open_and_close_panels() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert_eq!(shell.floating_dock_count(), 0);
        shell.set_active_inspector_panel(InspectorPanelKind::Memory);
        shell.set_active_bottom_panel(BottomPanelKind::Diagnostics);
        shell.handle_event(UiEvent::KeyDown {
            key: "1".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Memory);
        shell.handle_event(UiEvent::KeyDown {
            key: "4".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 2);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Diagnostics);
        shell.handle_event(UiEvent::KeyDown {
            key: "0".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);
    }

    #[test]
    fn ctrl_shift_inspector_floating_shortcuts_do_not_switch_fixed_inspector_tabs() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "2".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);

        shell.handle_event(UiEvent::KeyDown {
            key: "1".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
        assert_eq!(
            shell.dock_layout().active_inspector_panel(),
            InspectorPanelKind::Layers
        );

        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 2);
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
        assert_eq!(
            shell.dock_layout().active_inspector_panel(),
            InspectorPanelKind::Layers
        );
    }

    #[test]
    fn ctrl_shift_bottom_floating_shortcuts_do_not_switch_fixed_bottom_tabs() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "5".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Log);

        shell.handle_event(UiEvent::KeyDown {
            key: "4".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Log);
        assert_eq!(
            shell.dock_layout().active_bottom_panel(),
            BottomPanelKind::Log
        );

        shell.handle_event(UiEvent::KeyDown {
            key: "6".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 2);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Log);
        assert_eq!(
            shell.dock_layout().active_bottom_panel(),
            BottomPanelKind::Log
        );
    }

    #[test]
    fn floating_dock_title_drag_updates_position() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let kind = FloatingDockKind::Inspector(InspectorPanelKind::Layers);
        shell.handle_event(UiEvent::KeyDown {
            key: "2".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        let before = shell.floating_dock_rect(kind).expect("floating layers");
        let title = floating_dock_title_rect(before);
        let start = Point::new(title.x + 10.0, title.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: start,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove {
            position: Point::new(start.x + 30.0, start.y + 20.0),
        });
        shell.handle_event(UiEvent::PointerUp {
            position: Point::new(start.x + 30.0, start.y + 20.0),
            button: PointerButton::Primary,
        });
        let after = shell
            .floating_dock_rect(kind)
            .expect("floating layers after");
        assert!(after.x > before.x);
        assert!(after.y > before.y);
    }

    #[test]
    fn shifted_symbol_shortcuts_open_floating_docks() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert_eq!(shell.floating_dock_count(), 0);
        shell.handle_event(UiEvent::KeyDown {
            key: "!".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "$".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 2);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Timeline);
    }

    #[test]
    fn shifted_symbol_shortcut_closes_all_floating_docks() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "!".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "@".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 2);
        shell.handle_event(UiEvent::KeyDown {
            key: ")".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);
    }

    #[test]
    fn floating_dock_close_all_accepts_ctrl_zero_variants() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 720.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "!|PhysicalKey::Code(Digit1)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "@|PhysicalKey::Code(Digit2)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 2);

        shell.handle_event(UiEvent::KeyDown {
            key: ")|PhysicalKey::Code(Digit0)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);

        shell.handle_event(UiEvent::KeyDown {
            key: "1".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "Unidentified(NativeKeyCode(0))|PhysicalKey::Code(Digit0)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);

        shell.handle_event(UiEvent::KeyDown {
            key: "1".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "0".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);
    }

    #[test]
    fn top_row_zero_native_key_variants_close_floating_docks() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 720.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "!|PhysicalKey::Code(Digit1)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "Unidentified(NativeKeyCode(48))|PhysicalKey::Code(Key0)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);

        shell.handle_event(UiEvent::KeyDown {
            key: "!|PhysicalKey::Code(Digit1)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "Unidentified(NativeKeyCode(11))".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);
    }

    #[test]
    fn ctrl_shift_zero_runtime_variants_close_floating_docks() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 720.0));

        shell.handle_event(UiEvent::KeyDown {
            key: "!|PhysicalKey::Code(Digit1)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        shell.handle_event(UiEvent::KeyDown {
            key: "Unidentified(Windows(0))|PhysicalKey::Code(Digit0)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);

        shell.handle_event(UiEvent::KeyDown {
            key: "!|PhysicalKey::Code(Digit1)".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        // Simulate a Windows event path where the logical key is unidentified
        // and the cached Shift modifier is stale, but the Ctrl-combination now
        // still carries physical-key metadata from plita_window.
        shell.handle_event(UiEvent::KeyDown {
            key: "Unidentified(NativeKeyCode(0))|PhysicalKey::Code(Digit0)".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);
    }

    #[test]
    fn key_input_splits_logical_and_physical_metadata() {
        let input = KeyInput::from_legacy("c|PhysicalKey::Code(KeyC)", true, false, false, false);
        assert_eq!(input.logical, "c");
        assert_eq!(input.physical.as_deref(), Some("PhysicalKey::Code(KeyC)"));
        assert_eq!(input.logical_upper(), "C");
        assert!(input.ctrl);
        assert!(!input.shift);
    }

    #[test]
    fn key_input_digit_resolver_accepts_windows_jis_zero_variants() {
        let shifted = KeyInput::from_legacy(
            "Unidentified(Windows(0x0030))|PhysicalKey::Code(Digit0)",
            true,
            true,
            false,
            false,
        );
        assert_eq!(shifted.shortcut_digit(), Some(0));

        let release_fallback = KeyInput::from_legacy(
            "Unidentified(Windows(0x0030))|Code(Digit0)",
            true,
            false,
            false,
            false,
        );
        assert_eq!(release_fallback.shortcut_digit(), Some(0));

        let one = KeyInput::from_legacy("!|PhysicalKey::Code(Digit1)", true, true, false, false);
        assert_eq!(one.shortcut_digit(), Some(1));
    }

    #[test]
    fn dock_layout_state_default_tree_has_expected_tabs() {
        let layout = DockLayoutState::default_for_size(Size::new(1280.0, 800.0));
        assert_eq!(layout.version, 1);
        assert_eq!(
            layout.active_inspector_panel(),
            InspectorPanelKind::Properties
        );
        assert_eq!(layout.active_bottom_panel(), BottomPanelKind::Timeline);
        assert_eq!(
            layout.root.active_panel(DockAreaKind::Right),
            Some(DockPanelKind::Properties)
        );
        assert_eq!(
            layout.root.active_panel(DockAreaKind::Bottom),
            Some(DockPanelKind::Timeline)
        );
        assert_eq!(
            layout
                .root
                .find_tabs(DockAreaKind::Right)
                .expect("right tabs")
                .len(),
            3
        );
        assert_eq!(
            layout
                .root
                .find_tabs(DockAreaKind::Bottom)
                .expect("bottom tabs")
                .len(),
            3
        );
        assert_eq!(layout.floating_panels().len(), 6);
        assert_eq!(layout.floating_dock_count(), 0);
    }

    #[test]
    fn dock_layout_state_syncs_active_tabs_with_shell_shortcuts() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "2".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "6".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Diagnostics);
        assert_eq!(
            shell.dock_layout().active_inspector_panel(),
            InspectorPanelKind::Layers
        );
        assert_eq!(
            shell.dock_layout().active_bottom_panel(),
            BottomPanelKind::Diagnostics
        );
    }

    #[test]
    fn dock_layout_state_tracks_floating_dock_lifecycle_and_drag() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let kind = FloatingDockKind::Inspector(InspectorPanelKind::Layers);
        shell.handle_event(UiEvent::KeyDown {
            key: "2".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 1);
        assert_eq!(shell.dock_layout().floating_dock_count(), 1);
        assert!(
            shell
                .dock_layout()
                .floating_panel(DockPanelKind::Layers)
                .expect("layers state")
                .open
        );

        let before = shell.floating_dock_rect(kind).expect("floating layers");
        let title = floating_dock_title_rect(before);
        let start = Point::new(title.x + 10.0, title.y + 10.0);
        shell.handle_event(UiEvent::PointerDown {
            position: start,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove {
            position: Point::new(start.x + 44.0, start.y + 18.0),
        });
        shell.handle_event(UiEvent::PointerUp {
            position: Point::new(start.x + 44.0, start.y + 18.0),
            button: PointerButton::Primary,
        });
        let after = shell
            .dock_layout()
            .floating_panel(DockPanelKind::Layers)
            .expect("layers state after")
            .rect;
        assert!(after.x > before.x);
        assert!(after.y > before.y);

        shell.handle_event(UiEvent::KeyDown {
            key: "0".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.floating_dock_count(), 0);
        assert_eq!(shell.dock_layout().floating_dock_count(), 0);
    }

    #[test]
    fn dock_layout_reset_and_json_seed_are_deterministic() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "5".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(shell.dock_layout().floating_dock_count(), 1);
        assert_eq!(
            shell.dock_layout().active_bottom_panel(),
            BottomPanelKind::Log
        );

        shell.reset_dock_layout();
        assert_eq!(
            shell.active_inspector_panel(),
            InspectorPanelKind::Properties
        );
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Timeline);
        assert_eq!(shell.floating_dock_count(), 0);
        assert_eq!(shell.dock_layout().floating_dock_count(), 0);
        let json = shell.dock_layout_json();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"active_right\": \"properties\""));
        assert!(json.contains("\"active_bottom\": \"timeline\""));
        assert!(json.contains("\"panel\": \"diagnostics\""));
    }

    #[test]
    fn dock_layout_tabs_drive_tab_rect_order() {
        let shell = CreativeShell::new(Size::new(1280.0, 800.0));

        let inspector_from_layout = shell.dock_layout().inspector_tabs();
        let inspector_from_rects: Vec<InspectorPanelKind> = shell
            .inspector_tab_rects(shell.layout())
            .into_iter()
            .map(|(panel, _)| panel)
            .collect();
        assert_eq!(inspector_from_rects, inspector_from_layout);

        let bottom_from_layout = shell.dock_layout().bottom_tabs();
        let bottom_from_rects: Vec<BottomPanelKind> = shell
            .bottom_tab_rects(shell.layout())
            .into_iter()
            .map(|(panel, _)| panel)
            .collect();
        assert_eq!(bottom_from_rects, bottom_from_layout);
    }

    #[test]
    fn dock_layout_tabs_drive_widget_tree_nodes() {
        let shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let tree = shell.widget_tree();

        let inspector_nodes: Vec<InspectorPanelKind> = tree
            .nodes()
            .iter()
            .filter_map(|node| match node.role {
                WidgetRole::InspectorTab(panel) => Some(panel),
                _ => None,
            })
            .collect();
        assert_eq!(inspector_nodes, shell.dock_layout().inspector_tabs());

        let bottom_nodes: Vec<BottomPanelKind> = tree
            .nodes()
            .iter()
            .filter_map(|node| match node.role {
                WidgetRole::BottomTab(panel) => Some(panel),
                _ => None,
            })
            .collect();
        assert_eq!(bottom_nodes, shell.dock_layout().bottom_tabs());
    }

    #[test]
    fn dock_layout_json_seed_round_trips_active_tabs_and_floating_panels() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "2".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "5".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        let json = shell.save_dock_layout_seed();
        assert!(shell.saved_dock_layout_json().is_some());
        assert!(json.contains("\"active_right\": \"layers\""));
        assert!(json.contains("\"active_bottom\": \"log\""));
        assert!(json.contains("\"panel\": \"memory\", \"open\": true"));

        shell.reset_dock_layout();
        assert_eq!(
            shell.active_inspector_panel(),
            InspectorPanelKind::Properties
        );
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Timeline);
        assert_eq!(shell.floating_dock_count(), 0);

        assert!(shell.load_dock_layout_json(&json));
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Log);
        assert_eq!(shell.floating_dock_count(), 1);
        assert!(
            shell
                .dock_layout()
                .floating_panel(DockPanelKind::Memory)
                .expect("memory float")
                .open
        );
    }

    #[test]
    fn dock_layout_normalizes_missing_duplicate_and_offscreen_floating_state() {
        let mut layout = DockLayoutState::default_for_size(Size::new(1280.0, 800.0));
        layout.floating_panels = vec![
            DockFloatingPanelState::new(
                DockPanelKind::Memory,
                Rect::new(5000.0, 4000.0, 9999.0, 9999.0),
                true,
            ),
            DockFloatingPanelState::new(
                DockPanelKind::Memory,
                Rect::new(1.0, 1.0, 100.0, 80.0),
                false,
            ),
        ];

        layout.normalize_for_size(Size::new(320.0, 240.0));

        assert_eq!(layout.floating_panels().len(), 6);
        assert_eq!(layout.floating_dock_count(), 1);
        let memory = layout
            .floating_panel(DockPanelKind::Memory)
            .expect("memory panel");
        assert!(memory.open);
        assert!(memory.rect.x >= 12.0);
        assert!(memory.rect.y >= 12.0);
        assert!(memory.rect.x <= 320.0 - memory.rect.width - 12.0 || memory.rect.x == 12.0);
        assert!(memory.rect.y <= 240.0 - 28.0 - 12.0 || memory.rect.y == 12.0);
        assert!(memory.rect.width <= 320.0 - 24.0 || memory.rect.width == 80.0);
        assert!(memory.rect.height <= 240.0 - 24.0 || memory.rect.height == 60.0);
    }

    #[test]
    fn dock_layout_json_seed_load_clamps_offscreen_floating_panels() {
        let json = r#"{
  "version": 1,
  "root": { "kind": "split", "axis": "horizontal", "id": 1 },
  "active_right": "memory",
  "active_bottom": "diagnostics",
  "floating": [
    { "panel": "layers", "open": true, "x": 9000, "y": 8000, "w": 9000, "h": 8000 }
  ]
}
"#;
        let layout =
            DockLayoutState::from_json_seed(json, Size::new(300.0, 220.0)).expect("layout seed");
        assert_eq!(layout.active_inspector_panel(), InspectorPanelKind::Memory);
        assert_eq!(layout.active_bottom_panel(), BottomPanelKind::Diagnostics);
        assert_eq!(layout.floating_panels().len(), 6);
        assert_eq!(layout.floating_dock_count(), 1);
        let layers = layout
            .floating_panel(DockPanelKind::Layers)
            .expect("layers panel");
        assert!(layers.open);
        assert!(layers.rect.x >= 12.0);
        assert!(layers.rect.y >= 12.0);
        assert!(layers.rect.x <= 300.0 - layers.rect.width - 12.0 || layers.rect.x == 12.0);
        assert!(layers.rect.y <= 220.0 - 28.0 - 12.0 || layers.rect.y == 12.0);
        assert!(layers.rect.width <= 300.0 - 24.0 || layers.rect.width == 80.0);
        assert!(layers.rect.height <= 220.0 - 24.0 || layers.rect.height == 60.0);
    }

    #[test]
    fn shell_resize_keeps_open_floating_docks_reachable() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let json = r#"{
  "version": 1,
  "root": { "kind": "split", "axis": "horizontal", "id": 1 },
  "active_right": "properties",
  "active_bottom": "timeline",
  "floating": [
    { "panel": "properties", "open": true, "x": 9000, "y": 8000, "w": 400, "h": 240 }
  ]
}
"#;
        assert!(shell.load_dock_layout_json(json));
        shell.resize(Size::new(360.0, 240.0));
        let rect = shell
            .floating_dock_rect(FloatingDockKind::Inspector(InspectorPanelKind::Properties))
            .expect("properties floating dock");
        assert!(rect.x >= 12.0);
        assert!(rect.y >= 12.0);
        assert!(rect.x <= 360.0 - rect.width - 12.0 || rect.x == 12.0);
        assert!(rect.y <= 240.0 - 28.0 - 12.0 || rect.y == 12.0);
    }

    #[test]
    fn expanded_window_menu_uses_compact_rows_for_small_windows() {
        let mut shell = CreativeShell::new(Size::new(743.0, 354.0));
        let menu_rect = shell.menu_item_rects(shell.layout())[3].1;
        let p = Point::new(menu_rect.x + 8.0, menu_rect.y + 8.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        let popup = shell.menu_popup_rect().expect("window popup");
        assert_eq!(shell.active_menu(), Some(MenuKind::Window));
        assert!(popup.bottom() <= shell.size.height);
        let items = shell.menu_command_rects().expect("window menu commands");
        assert!(items.iter().all(|(_, rect)| rect.height <= 18.0));
        assert!(items
            .iter()
            .any(|(command, _)| *command == MenuCommand::LoadDockLayoutSlotB));
    }

    #[test]
    fn window_menu_exposes_layout_save_load_reset_commands() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let menu_rect = shell.menu_item_rects(shell.layout())[3].1;
        let p = Point::new(menu_rect.x + 8.0, menu_rect.y + 8.0);
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerUp {
            position: p,
            button: PointerButton::Primary,
        });
        assert_eq!(shell.active_menu(), Some(MenuKind::Window));
        let commands: Vec<MenuCommand> = shell
            .menu_command_rects()
            .expect("window menu commands")
            .into_iter()
            .map(|(command, _)| command)
            .collect();
        assert!(commands.contains(&MenuCommand::SaveDockLayout));
        assert!(commands.contains(&MenuCommand::LoadDockLayout));
        assert!(commands.contains(&MenuCommand::FloatProperties));
        assert!(commands.contains(&MenuCommand::DockAllFloating));
        assert!(commands.contains(&MenuCommand::SaveDockLayoutSlotA));
        assert!(commands.contains(&MenuCommand::LoadDockLayoutSlotA));
        assert!(commands.contains(&MenuCommand::RestoreDockTabs));
        assert!(commands.contains(&MenuCommand::ResetDockLayout));
    }

    #[test]
    fn dock_layout_summary_reports_active_tabs_floating_and_saved_seed() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let initial = shell.dock_layout_summary_text();
        assert!(initial.contains("RIGHT PROP"));
        assert!(initial.contains("BOTTOM TIMELINE"));
        assert!(initial.contains("FLOAT 0 [NONE]"));
        assert!(initial.contains("SAVED NO"));

        shell.handle_event(UiEvent::KeyDown {
            key: "6".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        shell.save_dock_layout_seed();
        let after = shell.dock_layout_summary_text();
        assert!(after.contains("RIGHT MEM"));
        assert!(after.contains("BOTTOM TIMELINE"));
        assert!(after.contains("FLOAT 1 [DIAG]"));
        assert!(after.contains("SAVED YES"));
        assert!(shell.saved_dock_layout_len() > 0);
    }

    #[test]
    fn window_menu_exposes_dock_layout_info_command() {
        let commands = CreativeShell::menu_commands_for(MenuKind::Window);
        assert!(commands.contains(&MenuCommand::ShowDockLayoutInfo));
        assert!(commands.contains(&MenuCommand::SaveDockLayout));
        assert!(commands.contains(&MenuCommand::LoadDockLayout));
        assert!(commands.contains(&MenuCommand::FloatProperties));
        assert!(commands.contains(&MenuCommand::DockAllFloating));
        assert!(commands.contains(&MenuCommand::SaveDockLayoutSlotA));
        assert!(commands.contains(&MenuCommand::LoadDockLayoutSlotA));
        assert!(commands.contains(&MenuCommand::SaveDockLayoutSlotB));
        assert!(commands.contains(&MenuCommand::LoadDockLayoutSlotB));
        assert!(commands.contains(&MenuCommand::RestoreDockTabs));
        assert!(commands.contains(&MenuCommand::ResetDockLayout));
    }

    #[test]
    fn dock_layout_info_command_switches_to_diagnostics_without_opening_floating() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.set_active_inspector_panel(InspectorPanelKind::Layers);
        shell.invoke_menu_command(MenuCommand::ShowDockLayoutInfo);
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Diagnostics);
        assert_eq!(shell.floating_dock_count(), 0);
        assert!(shell.status_text().contains("MENU DOCK LAYOUT INFO"));
        assert!(shell.status_text().contains("RIGHT LAYERS"));
        assert!(shell.status_text().contains("BOTTOM DIAG"));
    }

    #[test]
    fn bottom_diag_lines_include_layout_summary_near_top() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "6".to_string(),
            ctrl: true,
            shift: true,
            alt: false,
            meta: false,
        });
        shell.invoke_menu_command(MenuCommand::ShowDockLayoutInfo);
        shell.save_dock_layout_seed();

        let lines = bottom_diagnostics_lines(&shell);
        assert_eq!(lines[0], "DOCK LAYOUT");
        assert!(lines[1].contains("RIGHT PROP"));
        assert!(lines[1].contains("BOTTOM DIAG"));
        assert!(lines[1].contains("FLOAT 1 [DIAG]"));
        assert!(lines[1].contains("SAVED YES"));
        assert!(lines[2].contains("SPLIT ROOT"));
        assert!(lines[3].contains("TABS HIDDEN"));
        assert!(lines[5].contains("LAYOUT JSON SEED"));
    }

    #[test]
    fn bottom_diag_layout_lines_fit_in_compact_bottom_dock() {
        let shell = CreativeShell::new(Size::new(743.0, 354.0));
        let rect = shell.layout().timeline;
        let positions = bottom_diagnostics_line_positions(rect);

        let lines = bottom_diagnostics_lines(&shell);
        assert_eq!(lines[0], "DOCK LAYOUT");
        assert!(lines[2].contains("SPLIT ROOT"));

        assert!(positions[0] + 10.0 <= rect.bottom() - 8.0);
        assert!(positions[1] + 10.0 <= rect.bottom() - 8.0);
        assert!(positions[2] + 10.0 <= rect.bottom() - 8.0);
        assert!(positions[1] < rect.y + 70.0);
    }

    #[test]
    fn dock_split_ratios_round_trip_through_json_seed() {
        let mut layout = DockLayoutState::default_for_size(Size::new(1280.0, 800.0));
        layout.set_root_split_ratio(0.64);
        layout.set_main_split_ratio(0.71);

        let json = layout.to_json_pretty();
        assert!(json.contains("\"root_ratio\": 0.640"));
        assert!(json.contains("\"main_ratio\": 0.710"));

        let restored =
            DockLayoutState::from_json_seed(&json, Size::new(1280.0, 800.0)).expect("layout seed");
        assert!((restored.root_split_ratio() - 0.64).abs() < 0.001);
        assert!((restored.main_split_ratio() - 0.71).abs() < 0.001);
    }

    #[test]
    fn dock_split_ratios_are_clamped_and_reported_in_diagnostics() {
        let mut layout = DockLayoutState::default_for_size(Size::new(1280.0, 800.0));
        layout.set_root_split_ratio(2.0);
        layout.set_main_split_ratio(-1.0);
        assert!((layout.root_split_ratio() - 0.95).abs() < 0.001);
        assert!((layout.main_split_ratio() - 0.05).abs() < 0.001);
        assert_eq!(layout.split_summary_line(), "SPLIT ROOT 95%  MAIN 5%");

        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.dock_layout.set_root_split_ratio(0.64);
        shell.dock_layout.set_main_split_ratio(0.71);
        let lines = bottom_diagnostics_lines(&shell);
        assert_eq!(lines[2], "SPLIT ROOT 64%  MAIN 71%");
    }

    #[test]
    fn right_splitter_drag_updates_root_ratio_and_saved_seed() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let before = shell.dock_layout().root_split_ratio();
        let splitter = shell.layout().right_splitter;
        let p0 = Point::new(splitter.x + 2.0, splitter.y + 20.0);
        let p1 = Point::new(p0.x - 90.0, p0.y);
        shell.handle_event(UiEvent::PointerDown {
            position: p0,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove { position: p1 });
        shell.handle_event(UiEvent::PointerUp {
            position: p1,
            button: PointerButton::Primary,
        });

        let after = shell.dock_layout().root_split_ratio();
        assert!(
            after < before,
            "dragging the right splitter left should increase inspector width and lower root ratio"
        );
        assert!(
            shell.status_text().contains("ROOT")
                || shell.dock_split_summary_text().contains("SPLIT ROOT")
        );

        let json = shell.save_dock_layout_seed();
        assert!(json.contains("\"root_ratio\""));
        assert!(json.contains(&format!("\"root_ratio\": {:.3}", after)));
    }

    #[test]
    fn loading_root_ratio_applies_visible_right_panel_width() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let before_width = shell.layout().inspector.width;
        let json = r#"{
  "version": 1,
  "root": { "kind": "split", "axis": "horizontal", "id": 1 },
  "root_ratio": 0.650,
  "main_ratio": 0.780,
  "active_right": "properties",
  "active_bottom": "timeline",
  "floating": []
}
"#;

        assert!(shell.load_dock_layout_json(json));
        let after_width = shell.layout().inspector.width;
        assert!(
            after_width > before_width,
            "lower root ratio should restore a wider inspector panel"
        );
        assert!((shell.dock_layout().root_split_ratio() - 0.650).abs() < 0.01);
        assert!(shell.dock_split_summary_text().contains("SPLIT ROOT 65%"));
    }

    #[test]
    fn bottom_splitter_drag_updates_main_ratio_and_saved_seed() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let before_ratio = shell.dock_layout().main_split_ratio();
        let before_height = shell.layout().timeline.height;
        let splitter = bottom_splitter_rect(shell.layout());
        let p0 = Point::new(splitter.x + 200.0, splitter.y + splitter.height * 0.5);
        let p1 = Point::new(p0.x, p0.y - 80.0);

        shell.handle_event(UiEvent::PointerDown {
            position: p0,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove { position: p1 });
        shell.handle_event(UiEvent::PointerUp {
            position: p1,
            button: PointerButton::Primary,
        });

        let after_ratio = shell.dock_layout().main_split_ratio();
        let after_height = shell.layout().timeline.height;
        assert!(
            after_height > before_height,
            "dragging the bottom splitter upward should expand the bottom dock"
        );
        assert!(
            after_ratio < before_ratio,
            "expanding the bottom dock should lower the main split ratio"
        );
        assert!(
            shell.status_text().contains("MAIN")
                || shell.dock_split_summary_text().contains("MAIN")
        );

        let json = shell.save_dock_layout_seed();
        assert!(json.contains("\"main_ratio\""));
        assert!(json.contains(&format!("\"main_ratio\": {:.3}", after_ratio)));
    }

    #[test]
    fn loading_main_ratio_applies_visible_bottom_dock_height() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let before_height = shell.layout().timeline.height;
        let json = r#"{
  "version": 1,
  "root": { "kind": "split", "axis": "horizontal", "id": 1 },
  "root_ratio": 0.780,
  "main_ratio": 0.650,
  "active_right": "properties",
  "active_bottom": "diagnostics",
  "floating": []
}
"#;

        assert!(shell.load_dock_layout_json(json));
        let after_height = shell.layout().timeline.height;
        assert!(
            after_height > before_height,
            "lower main ratio should restore a taller bottom dock"
        );
        assert_eq!(shell.active_bottom_panel(), BottomPanelKind::Diagnostics);
        assert!((shell.dock_layout().main_split_ratio() - 0.650).abs() < 0.01);
        assert!(shell.dock_split_summary_text().contains("MAIN 65%"));
    }

    #[test]
    fn widget_tree_exposes_bottom_splitter_hit_area() {
        let shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let splitter = bottom_splitter_rect(shell.layout());
        let point = Point::new(
            splitter.x + splitter.width * 0.5,
            splitter.y + splitter.height * 0.5,
        );
        let tree = shell.widget_tree();
        let hit = tree.hit_test(point).expect("bottom splitter hit");
        assert_eq!(hit.role, WidgetRole::Splitter(SplitterKind::Bottom));
    }

    #[test]
    fn dock_tab_visibility_hides_tabs_and_restores_them() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert!(shell.dock_layout().is_panel_tab_open(DockPanelKind::Memory));
        assert!(shell.set_dock_panel_tab_open(DockPanelKind::Memory, false));
        assert!(!shell.dock_layout().is_panel_tab_open(DockPanelKind::Memory));
        assert!(shell.dock_tab_summary_text().contains("MEM"));
        assert!(!shell
            .inspector_tab_rects(shell.layout())
            .iter()
            .any(|(panel, _)| *panel == InspectorPanelKind::Memory));

        shell.restore_dock_tabs();
        assert!(shell.dock_layout().is_panel_tab_open(DockPanelKind::Memory));
        assert!(shell.dock_tab_summary_text().contains("NONE"));
        assert!(shell
            .inspector_tab_rects(shell.layout())
            .iter()
            .any(|(panel, _)| *panel == InspectorPanelKind::Memory));
    }

    #[test]
    fn dock_tab_visibility_round_trips_through_json_seed() {
        let mut layout = DockLayoutState::default_for_size(Size::new(1280.0, 800.0));
        assert!(layout.set_panel_tab_open(DockPanelKind::Memory, false));
        assert!(layout.set_panel_tab_open(DockPanelKind::Log, false));
        let json = layout.to_json_pretty();
        assert!(json.contains("\"dock_tabs\""));
        assert!(json.contains("\"tab\": \"memory\""));
        let restored =
            DockLayoutState::from_json_seed(&json, Size::new(1280.0, 800.0)).expect("layout seed");
        assert!(!restored.is_panel_tab_open(DockPanelKind::Memory));
        assert!(!restored.is_panel_tab_open(DockPanelKind::Log));
        assert!(restored.is_panel_tab_open(DockPanelKind::Properties));
        assert!(restored.tab_summary_line().contains("MEM"));
        assert!(restored.tab_summary_line().contains("LOG"));
    }

    #[test]
    fn dock_layout_slots_save_and_restore_independent_layouts() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.set_active_inspector_panel(InspectorPanelKind::Layers);
        assert!(shell.save_dock_layout_slot(0));

        shell.set_active_inspector_panel(InspectorPanelKind::Memory);
        assert!(shell.set_dock_panel_tab_open(DockPanelKind::Log, false));
        assert!(shell.save_dock_layout_slot(1));

        shell.set_active_inspector_panel(InspectorPanelKind::Properties);
        assert!(shell.load_dock_layout_slot(0));
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
        assert!(shell.dock_layout().is_panel_tab_open(DockPanelKind::Log));

        assert!(shell.load_dock_layout_slot(1));
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Memory);
        assert!(!shell.dock_layout().is_panel_tab_open(DockPanelKind::Log));
        assert!(shell.dock_layout_slot_summary_text().contains("SLOTS A"));
    }

    #[test]
    fn dock_tree_geometry_allocates_center_right_and_bottom_from_ratios() {
        let mut layout = DockLayoutState::default_for_size(Size::new(1280.0, 800.0));
        layout.set_root_split_ratio(0.75);
        layout.set_main_split_ratio(0.70);

        let geometry = layout.allocate_geometry(Rect::new(0.0, 0.0, 1000.0, 800.0));
        let center = geometry
            .rect_for_area(DockAreaKind::Center)
            .expect("center rect");
        let right = geometry
            .rect_for_area(DockAreaKind::Right)
            .expect("right rect");
        let bottom = geometry
            .rect_for_area(DockAreaKind::Bottom)
            .expect("bottom rect");

        assert_eq!(center, Rect::new(0.0, 0.0, 750.0, 560.0));
        assert_eq!(right, Rect::new(750.0, 0.0, 250.0, 800.0));
        assert_eq!(bottom, Rect::new(0.0, 560.0, 750.0, 240.0));
        assert!(matches!(
            geometry.nodes()[0].kind,
            DockNodeGeometryKind::Split {
                axis: DockSplitAxis::Horizontal,
                ..
            }
        ));
    }

    #[test]
    fn fixed_tab_close_hides_tab_and_ctrl_shortcut_reopens_it() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let tab = shell
            .inspector_tab_rects(shell.layout())
            .into_iter()
            .find(|(panel, _)| *panel == InspectorPanelKind::Memory)
            .expect("memory tab")
            .1;
        let close = dock_tab_close_rect(tab);
        let p = Point::new(close.x + 2.0, close.y + 2.0);

        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });
        assert!(!shell.dock_layout().is_panel_tab_open(DockPanelKind::Memory));

        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(shell.dock_layout().is_panel_tab_open(DockPanelKind::Memory));
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Memory);
    }

    #[test]
    fn detach_and_attach_floating_panel_preserves_fixed_tab_state() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert!(shell.detach_dock_panel(DockPanelKind::Layers));
        assert!(!shell.dock_layout().is_panel_tab_open(DockPanelKind::Layers));
        assert!(
            shell
                .dock_layout()
                .floating_panel(DockPanelKind::Layers)
                .expect("layers floating")
                .open
        );
        assert_eq!(
            shell.active_inspector_panel(),
            InspectorPanelKind::Properties
        );

        assert!(shell.attach_dock_panel(DockPanelKind::Layers));
        assert!(shell.dock_layout().is_panel_tab_open(DockPanelKind::Layers));
        assert!(
            !shell
                .dock_layout()
                .floating_panel(DockPanelKind::Layers)
                .expect("layers floating")
                .open
        );
        assert_eq!(shell.active_inspector_panel(), InspectorPanelKind::Layers);
    }

    #[test]
    fn layout_profile_round_trips_and_rejects_corrupt_payload() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        assert!(shell.set_dock_panel_tab_open(DockPanelKind::Log, false));
        shell.detach_dock_panel(DockPanelKind::Memory);
        let profile = shell.save_dock_layout_profile_text();

        let mut restored = CreativeShell::new(Size::new(1280.0, 800.0));
        assert!(restored.load_dock_layout_profile_text(&profile));
        assert!(!restored.dock_layout().is_panel_tab_open(DockPanelKind::Log));
        assert!(!restored
            .dock_layout()
            .is_panel_tab_open(DockPanelKind::Memory));
        assert!(
            restored
                .dock_layout()
                .floating_panel(DockPanelKind::Memory)
                .expect("memory floating")
                .open
        );

        assert!(
            DockLayoutState::from_profile_text("not a profile", Size::new(1280.0, 800.0)).is_none()
        );
    }

    #[test]
    fn command_registry_exposes_default_shortcut_metadata() {
        let registry = CommandRegistry::creative_shell_defaults();
        assert_eq!(
            registry.get(CommandId::Copy).and_then(|cmd| cmd.shortcut),
            Some("Ctrl+C")
        );
        assert_eq!(
            registry.get(CommandId::AttachFloating).map(|cmd| cmd.label),
            Some("ATTACH FLOATING")
        );
        assert!(registry.commands().len() >= 10);
    }

    #[test]
    fn document_model_layer_tree_multi_select_and_snap_are_available() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::KeyDown {
            key: "3".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        let p = Point::new(
            shell.layout().canvas.x + 260.0,
            shell.layout().canvas.y + 170.0,
        );
        shell.handle_event(UiEvent::PointerDown {
            position: p,
            button: PointerButton::Primary,
        });

        let count = shell.select_objects_in_rect(Rect::new(0.0, 0.0, 400.0, 300.0));
        assert!(count >= 2);
        let document = shell.document_model();
        let layers = document.layer_tree();
        assert_eq!(document.objects.len(), shell.objects().len());
        assert!(layers.iter().any(|node| node.selected));

        shell.set_snap_to_grid(true, 16.0);
        let snapped = shell.snap_rect_to_grid(Rect::new(17.0, 31.0, 20.0, 20.0));
        assert_eq!(snapped.x, 16.0);
        assert_eq!(snapped.y, 32.0);
    }

    #[test]
    fn theme_tokens_expose_stable_dark_palette() {
        let tokens = ThemeTokens::dark();
        assert_eq!(tokens.accent, Color::rgb8(106, 158, 255));
        assert!(tokens.background != tokens.panel);
    }

    #[test]
    fn scroll_area_state_clamps_offsets_and_reports_content_origin() {
        let viewport = Rect::new(10.0, 20.0, 100.0, 80.0);
        let content = Size::new(260.0, 240.0);
        let mut scroll = ScrollAreaState::new();

        assert!(scroll.scroll_by(Point::new(500.0, 500.0), viewport, content));
        assert_eq!(scroll.offset(), Point::new(160.0, 160.0));
        assert_eq!(
            scroll.content_origin(viewport, content),
            Point::new(-150.0, -140.0)
        );

        assert!(scroll.scroll_by(Point::new(-999.0, -999.0), viewport, content));
        assert_eq!(scroll.offset(), Point::new(0.0, 0.0));
    }

    #[test]
    fn scroll_area_draws_vertical_and_horizontal_scrollbars() {
        let viewport = Rect::new(0.0, 0.0, 120.0, 90.0);
        let content = Size::new(300.0, 260.0);
        let scroll = ScrollAreaState::from_offset(Point::new(20.0, 40.0));
        let style = ScrollbarStyle::default();
        let geometry = scroll.geometry(viewport, content, style);
        let vertical_thumb = geometry.vertical.expect("vertical scrollbar").thumb;
        let horizontal_thumb = geometry.horizontal.expect("horizontal scrollbar").thumb;

        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        scroll.draw_scrollbars(&mut list, viewport, content, style);

        assert!(list
            .commands()
            .iter()
            .any(|command| matches!(command, DrawCommand::FillRect { rect, .. } if *rect == vertical_thumb)));
        assert!(list
            .commands()
            .iter()
            .any(|command| matches!(command, DrawCommand::FillRect { rect, .. } if *rect == horizontal_thumb)));
    }

    #[test]
    fn scroll_area_vertical_thumb_drag_updates_offset() {
        let viewport = Rect::new(0.0, 0.0, 120.0, 90.0);
        let content = Size::new(180.0, 360.0);
        let style = ScrollbarStyle::default();
        let mut scroll = ScrollAreaState::new();
        let thumb = scroll
            .geometry(viewport, content, style)
            .vertical
            .expect("vertical scrollbar")
            .thumb;
        let start = Point::new(thumb.x + thumb.width * 0.5, thumb.y + thumb.height * 0.5);

        let hit = scroll
            .begin_scrollbar_drag(viewport, content, start, style)
            .expect("thumb hit");
        assert_eq!(hit.part, ScrollbarPart::Thumb);
        assert!(scroll.drag().is_some());
        assert!(scroll.drag_scrollbar(
            viewport,
            content,
            Point::new(start.x, start.y + 40.0),
            style
        ));
        assert!(scroll.offset().y > 0.0);
        assert!(scroll.end_scrollbar_drag());
        assert_eq!(scroll.drag(), None);
    }

    #[test]
    fn creative_shell_inspector_scrollbar_drag_updates_scroll() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        let (viewport, content, offset) = shell
            .scroll_area_spec(ShellScrollArea::Inspector)
            .expect("inspector scroll area");
        let thumb = ScrollAreaState::from_offset(offset)
            .geometry(viewport, content, phase1_scrollbar_input_style())
            .vertical
            .expect("vertical scrollbar")
            .thumb;
        let start = Point::new(thumb.x + thumb.width * 0.5, thumb.y + thumb.height * 0.5);
        let end = Point::new(start.x, start.y + 90.0);

        shell.handle_event(UiEvent::PointerDown {
            position: start,
            button: PointerButton::Primary,
        });
        shell.handle_event(UiEvent::PointerMove { position: end });
        shell.handle_event(UiEvent::PointerUp {
            position: end,
            button: PointerButton::Primary,
        });

        assert!(shell.inspector_scroll().y > 0.0);
    }

    #[test]
    fn button_interaction_tracks_hover_press_and_release() {
        let mut buttons = ButtonInteractionState::new();

        assert_eq!(
            buttons.visual_state(&"run", false),
            ButtonVisualState::Normal
        );
        assert!(buttons.handle_pointer_move(Some("run")));
        assert_eq!(
            buttons.visual_state(&"run", false),
            ButtonVisualState::Hovered
        );
        assert!(buttons.handle_pointer_down(Some("run")));
        assert_eq!(
            buttons.visual_state(&"run", true),
            ButtonVisualState::Pressed
        );

        let release = buttons.handle_pointer_up(Some("run"));
        assert_eq!(release.activated, Some("run"));
        assert!(release.changed);
        assert_eq!(buttons.pressed(), None);
        assert_eq!(buttons.hovered(), Some(&"run"));
    }

    #[test]
    fn button_interaction_does_not_activate_when_released_elsewhere() {
        let mut buttons = ButtonInteractionState::new();

        buttons.press("run");
        let release = buttons.release_over(Some("step"));

        assert_eq!(release.pressed, Some("run"));
        assert_eq!(release.released, Some("step"));
        assert_eq!(release.activated, None);
        assert_eq!(
            buttons.visual_state(&"run", true),
            ButtonVisualState::Active
        );
    }

    #[test]
    fn file_drop_event_updates_status_without_panicking() {
        let mut shell = CreativeShell::new(Size::new(1280.0, 800.0));
        shell.handle_event(UiEvent::FileDropped {
            path: PathBuf::from("demo.gb"),
        });
        assert!(shell.status_text().contains("demo.gb"));
    }
}
