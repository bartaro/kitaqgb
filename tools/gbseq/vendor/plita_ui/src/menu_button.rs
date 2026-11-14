//! Menu-button presentation shared by applications with their own popup model.
use crate::{
    draw_panel, draw_text, finite_nonnegative, ButtonVisualState, Color, ColoredVertex, DrawList,
    PanelStyle, Point, Rect,
};

/// Appearance of a menu button, using the same surface as other plita controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuButtonStyle {
    /// Surface colors, border, radius and shadow for the current interaction state.
    pub panel: PanelStyle,
    /// Color shared by the caption and the dropdown indicator.
    pub foreground: Color,
    /// Caption scale, as used by `draw_text`.
    pub text_scale: f32,
}

impl MenuButtonStyle {
    /// Create a style. Hover/active colors can come from the application's theme.
    pub const fn new(panel: PanelStyle, foreground: Color, text_scale: f32) -> Self {
        Self {
            panel,
            foreground,
            text_scale,
        }
    }
}

/// Horizontal space reserved for the caption padding, gap and dropdown arrow.
/// Add this to `measure_text_width(label, scale)` when sizing a menu button.
pub const MENU_BUTTON_CHROME_WIDTH: f32 = 42.0;

/// Caption and arrow geometry inside a menu button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuButtonLayout {
    /// Button surface, including the small pressed-state offset.
    pub surface: Rect,
    /// Caption clip; long text cannot cover the arrow.
    pub caption: Rect,
    /// Right-aligned area containing the geometric chevron.
    pub indicator: Rect,
}

impl MenuButtonLayout {
    /// Lay out a button. Use the original rectangle for the whole-button hit target.
    pub fn new(rect: Rect, state: ButtonVisualState) -> Self {
        let width = finite_nonnegative(rect.width);
        let height = finite_nonnegative(rect.height);
        let offset = if state == ButtonVisualState::Pressed {
            1.0_f32.min(width).min(height)
        } else {
            0.0
        };
        let surface = Rect::new(
            rect.x + offset,
            rect.y + offset,
            width - offset,
            height - offset,
        );
        let padding = 10.0_f32.min(surface.width / 2.0);
        let indicator_width = 18.0_f32.min((surface.width - padding * 2.0).max(0.0));
        let indicator_height = 18.0_f32.min(surface.height);
        let indicator = Rect::new(
            surface.right() - padding - indicator_width,
            surface.y + (surface.height - indicator_height) / 2.0,
            indicator_width,
            indicator_height,
        );
        let caption = Rect::new(
            surface.x + padding,
            surface.y,
            (indicator.x - 4.0 - surface.x - padding).max(0.0),
            surface.height,
        );
        Self {
            surface,
            caption,
            indicator,
        }
    }
}

/// Draw a complete captioned menu button with a font-independent dropdown arrow.
///
/// `expanded` points the arrow upward while the popup is open. The application
/// owns popup contents and event routing; the caption and arrow share one hit
/// target, typically driven by `ButtonInteractionState`. Do not append an arrow
/// character to `label`. The caption is clipped to preserve the indicator space.
pub fn draw_menu_button(
    list: &mut DrawList,
    rect: Rect,
    label: &str,
    expanded: bool,
    state: ButtonVisualState,
    style: MenuButtonStyle,
) {
    if !rect.is_visible() || !rect.x.is_finite() || !rect.y.is_finite() {
        return;
    }
    let layout = MenuButtonLayout::new(rect, state);
    draw_panel(list, layout.surface, style.panel);
    let scale = crate::finite_at_least(style.text_scale, 0.1);
    if layout.caption.is_visible() {
        list.push_clip(layout.caption);
        draw_text(
            list,
            layout.caption.x,
            layout.surface.y + ((layout.surface.height - 14.0 * scale) / 2.0).max(0.0),
            label,
            scale,
            style.foreground,
        );
        list.pop_clip();
    }
    draw_chevron(list, layout.indicator, expanded, style.foreground);
}

fn draw_chevron(list: &mut DrawList, rect: Rect, expanded: bool, color: Color) {
    let scale = (rect.width / 18.0).min(rect.height / 18.0);
    if scale <= 0.0 {
        return;
    }
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let direction = if expanded { -1.0 } else { 1.0 };
    // A single, solid chevron mesh; independent of fonts and glyph availability.
    let points = [
        (-6.0, -2.0),
        (-4.5, -3.5),
        (0.0, 1.0),
        (4.5, -3.5),
        (6.0, -2.0),
        (0.0, 4.0),
    ];
    let vertices: Vec<_> = [0, 1, 2, 0, 2, 5, 2, 3, 4, 2, 4, 5]
        .into_iter()
        .map(|index| {
            let (x, y) = points[index];
            ColoredVertex::new(
                Point::new(cx + x * scale, cy + y * scale * direction),
                color,
            )
        })
        .collect();
    list.fill_triangle_list(vertices);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DrawCommand;

    #[test]
    fn caption_never_overlaps_arrow_even_for_tiny_or_pressed_buttons() {
        for width in [0.0, 1.0, 16.0, 42.0, 78.0, 320.0] {
            for state in [ButtonVisualState::Normal, ButtonVisualState::Pressed] {
                let layout = MenuButtonLayout::new(Rect::new(10.0, 20.0, width, 32.0), state);
                assert!(layout.caption.width >= 0.0);
                assert!(layout.caption.right() <= layout.indicator.x);
                assert!(layout.indicator.right() <= layout.surface.right());
                assert!(layout.indicator.bottom() <= layout.surface.bottom());
            }
        }
    }

    #[test]
    fn open_arrow_is_vertical_reflection_and_caption_is_clipped() {
        let rect = Rect::new(10.0, 20.0, 78.0, 32.0);
        let style = MenuButtonStyle::new(PanelStyle::dark(), Color::rgb8(230, 230, 230), 1.22);
        let mut closed = DrawList::new(Color::rgb8(0, 0, 0));
        let mut opened = closed.clone();
        let label = "CMT very-long-cassette-name-日本語.mzt";
        draw_menu_button(
            &mut closed,
            rect,
            label,
            false,
            ButtonVisualState::Normal,
            style,
        );
        draw_menu_button(
            &mut opened,
            rect,
            label,
            true,
            ButtonVisualState::Normal,
            style,
        );
        let layout = MenuButtonLayout::new(rect, ButtonVisualState::Normal);
        assert!(closed.commands().contains(&DrawCommand::PushClip {
            rect: layout.caption
        }));
        assert!(matches!(
            closed.commands()[closed.commands().len() - 2],
            DrawCommand::PopClip
        ));
        let DrawCommand::FillTriangles { vertices: down } = closed.commands().last().unwrap()
        else {
            panic!()
        };
        let DrawCommand::FillTriangles { vertices: up } = opened.commands().last().unwrap() else {
            panic!()
        };
        assert_eq!(down.len(), up.len());
        for (a, b) in down.iter().zip(up.iter()) {
            assert_eq!(a.position.x, b.position.x);
            assert_eq!(
                a.position.y + b.position.y,
                2.0 * (rect.y + rect.height / 2.0)
            );
            assert!(layout.indicator.contains(a.position));
            assert!(layout.indicator.contains(b.position));
        }
    }
}
