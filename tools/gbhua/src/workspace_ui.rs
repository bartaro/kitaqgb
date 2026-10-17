use super::*;
use plita::DrawCommand;

#[cfg(test)]
#[path = "ui_snapshot.rs"]
mod snapshot;

#[cfg(test)]
mod tests {
    use super::*;

    fn click(app: &mut AssetStudioApp, action: ButtonAction) {
        let button = app
            .action_buttons()
            .into_iter()
            .find(|b| b.action == action)
            .unwrap();
        let point = Point::new(
            button.rect.x + button.rect.width * 0.5,
            button.rect.y + button.rect.height * 0.5,
        );
        app.handle_event(UiEvent::PointerDown {
            position: point,
            button: PointerButton::Primary,
        });
        app.handle_event(UiEvent::PointerUp {
            position: point,
            button: PointerButton::Primary,
        });
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    #[test]
    fn workspace_columns_and_controls_align_across_window_sizes() {
        for (w, h) in [
            (900.0, 600.0),
            (1024.0, 600.0),
            (1180.0, 640.0),
            (1366.0, 705.0),
            (1920.0, 1080.0),
        ] {
            let app = AssetStudioApp::new(Size::new(w, h));
            let graphic = app.graphic_panel_rect();
            assert_eq!(graphic.y, app.map_panel_rect().y);
            assert_eq!(graphic.y, app.atlas_panel_rect().y);
            assert_eq!(graphic.y, app.inspector_rect().y);
            let top = app.tile_editor_rect().y;
            assert_eq!(top, app.map_view_rect().y);
            assert_eq!(top, app.atlas_image_rect().y);
            assert_eq!(top, app.cgb_palette_grid_rect().y);
            assert!(app.bytecode_window_rect().bottom() < app.status_rect().y);
            let controls = app
                .toolbar_buttons()
                .into_iter()
                .chain(app.context_buttons())
                .collect::<Vec<_>>();
            for (i, a) in controls.iter().enumerate() {
                assert!(a.rect.x >= 0.0 && a.rect.right() <= w, "{w}: {a:?}");
                for b in controls.iter().skip(i + 1) {
                    assert!(!overlaps(a.rect, b.rect), "{w}: {a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn compact_sidebar_routes_clicks_only_to_the_visible_tab() {
        let mut app = AssetStudioApp::new(Size::new(1024.0, 600.0));
        let original_tile = app.editor.selected_tile;
        assert!(app.show_atlas() && !app.show_inspector());
        click(&mut app, ButtonAction::Sidebar(true));
        assert!(!app.show_atlas() && app.show_inspector());
        let grid = app.cgb_palette_grid_rect();
        let point = Point::new(grid.x + 45.0, grid.y + 6.5 * 24.0);
        assert!(!app.handle_atlas_hit(point));
        assert!(app.handle_cgb_palette_grid_hit(point));
        assert_eq!(app.editor.selected_palette, 6);
        assert_eq!(app.editor.selected_tile, original_tile);
        click(&mut app, ButtonAction::Sidebar(false));
        assert!(!app.handle_cgb_palette_grid_hit(point));
        assert!(app.handle_atlas_hit(point));
    }

    #[test]
    fn dismissing_a_menu_does_not_paint_the_underlying_canvas() {
        let mut app = AssetStudioApp::new(Size::new(1366.0, 705.0));
        let before = app.editor.project.clone();
        click(&mut app, ButtonAction::Menu(ActionGroup::File));
        let rect = app.tile_editor_rect();
        let point = Point::new(rect.x + 20.0, rect.y + 20.0);
        app.handle_pointer_down(point, PointerButton::Primary);
        app.handle_pointer_up(point, PointerButton::Primary);
        assert_eq!(app.open_group, None);
        assert_eq!(app.editor.project, before);
    }

    #[test]
    fn contextual_zoom_and_mode_controls_use_the_existing_editor_actions() {
        let mut app = AssetStudioApp::new(Size::new(1366.0, 705.0));
        let original = app.editor.project.clone();
        click(&mut app, ButtonAction::ZoomIn);
        assert!(app.map_zoom > 0);
        click(&mut app, ButtonAction::MapZoom(0));
        assert_eq!(app.map_zoom, 0);
        click(&mut app, ButtonAction::SetColorMode(ColorMode::Dmg));
        assert_eq!(app.editor.color_mode, ColorMode::Dmg);
        click(&mut app, ButtonAction::SetColorMode(ColorMode::Cgb));
        assert_eq!(app.editor.project, original);
        for button in app
            .toolbar_buttons()
            .into_iter()
            .chain(app.context_buttons())
        {
            if let Some(name) = icon(button.action) {
                assert!(lucide_icon_names().any(|available| available == name));
            }
        }
    }

    #[test]
    fn geometry_color_conversion_preserves_textures_and_alpha() {
        let color = Color::rgba8(128, 64, 32, 100);
        let mut list = DrawList::new(color);
        list.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), color);
        list.image_rgba8(Rect::new(0.0, 0.0, 1.0, 1.0), 1, 1, vec![128, 64, 32, 100]);
        let converted = geometry_colors(list.clone(), linear_color);
        assert!((converted.clear_color.r - 0.21586).abs() < 0.0001);
        assert_eq!(converted.clear_color.a, color.a);
        assert_eq!(converted.commands()[1], list.commands()[1]);
        let restored = geometry_colors(converted, srgb_color);
        assert!((restored.clear_color.r - color.r).abs() < 0.0001);
    }

    #[test]
    fn escape_and_focus_loss_cancel_transient_interactions() {
        let mut app = AssetStudioApp::new(Size::new(1366.0, 705.0));
        click(&mut app, ButtonAction::Menu(ActionGroup::File));
        app.handle_event(UiEvent::KeyDown {
            key: "Escape".into(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(app.open_group, None);
        let canvas = app.tile_editor_rect();
        app.handle_pointer_down(
            Point::new(canvas.x + 10.0, canvas.y + 10.0),
            PointerButton::Primary,
        );
        assert!(app.pointer_down);
        app.handle_event(UiEvent::FocusChanged { focused: false });
        assert!(!app.pointer_down);
        assert_eq!(app.drag_target, DragTarget::None);
        assert_eq!(app.buttons.pressed(), None);
        let before = app.editor.project.clone();
        app.handle_pointer_move(Point::new(canvas.right() - 10.0, canvas.bottom() - 10.0));
        assert_eq!(app.editor.project, before);
    }

    #[test]
    fn export_workspace_visual_regression_snapshots() {
        let Some(directory) = std::env::var_os("GBHUA_UI_SNAPSHOTS").map(PathBuf::from) else {
            return;
        };
        fs::create_dir_all(&directory).unwrap();
        for (name, w, h, mode, palette_tab, dropdown) in [
            ("wide-cgb", 1366.0, 705.0, ColorMode::Cgb, false, false),
            ("wide-dmg", 1366.0, 705.0, ColorMode::Dmg, false, false),
            ("compact-tiles", 1024.0, 600.0, ColorMode::Cgb, false, false),
            (
                "compact-palettes",
                900.0,
                600.0,
                ColorMode::Cgb,
                true,
                false,
            ),
            ("size-menu", 1366.0, 705.0, ColorMode::Cgb, false, true),
            ("large", 1920.0, 1080.0, ColorMode::Cgb, false, false),
        ] {
            let mut app = AssetStudioApp::new(Size::new(w, h));
            if let Some(path) = std::env::var_os("GBHUA_UI_PROJECT") {
                app.editor.load_project_path(PathBuf::from(path)).unwrap();
            }
            app.editor.color_mode = mode;
            app.palette_tab = palette_tab;
            app.editor.selected_tile = 60.min(app.editor.project.tiles.len() - 1);
            app.editor.refresh_bytecode_from_selected_tile();
            if dropdown {
                app.open_group = Some(ActionGroup::Canvas);
            }
            let list = geometry_colors(app.draw_list(), srgb_color);
            snapshot::save(&list, app.size, &directory.join(format!("{name}.png"))).unwrap();
        }
        let mut app = AssetStudioApp::new(Size::new(1366.0, 705.0));
        let save = app
            .toolbar_buttons()
            .into_iter()
            .find(|b| b.action == ButtonAction::SaveProject)
            .unwrap();
        app.handle_pointer_move(Point::new(save.rect.x + 10.0, save.rect.y + 10.0));
        snapshot::save(
            &geometry_colors(app.draw_list(), srgb_color),
            app.size,
            &directory.join("save-tooltip.png"),
        )
        .unwrap();
    }
}

pub(super) fn icon(action: ButtonAction) -> Option<&'static str> {
    Some(match action {
        ButtonAction::LoadProject => "folder-open",
        ButtonAction::SaveProject => "save",
        ButtonAction::Undo => "undo-2",
        ButtonAction::Redo => "redo-2",
        ButtonAction::CopyGraphic => "copy",
        ButtonAction::Rotate => "rotate-cw",
        ButtonAction::Tool(StudioTool::Pixel) => "file-pen-line",
        ButtonAction::MapZoom(0) => "maximize",
        ButtonAction::ZoomOut => "minus",
        ButtonAction::ZoomIn | ButtonAction::AddTile => "plus",
        _ => return None,
    })
}

impl AssetStudioApp {
    pub(super) fn action_active(&self, action: ButtonAction) -> bool {
        match action {
            ButtonAction::Menu(group) => self.open_group == Some(group),
            ButtonAction::Tool(tool) => self.editor.selected_tool == tool,
            ButtonAction::Sidebar(palette) => self.palette_tab == palette,
            ButtonAction::SetColorMode(mode) => self.editor.color_mode == mode,
            ButtonAction::SetGraphicSize(size) => self.editor.graphic_size == size,
            ButtonAction::MapZoom(zoom) => self.map_zoom == zoom,
            ButtonAction::ToggleGrid => self.grid_visible,
            ButtonAction::ToggleOverlays => self.overlays_visible,
            _ => false,
        }
    }

    pub(super) fn action_enabled(&self, action: ButtonAction) -> bool {
        match action {
            ButtonAction::Undo => !self.editor.undo_stack.is_empty(),
            ButtonAction::Redo => !self.editor.redo_stack.is_empty(),
            ButtonAction::PasteGraphic => self.copied_graphic.is_some(),
            _ => true,
        }
    }

    pub(super) fn draw_control(&self, list: &mut DrawList, button: ActionButton, menu: bool) {
        let theme = self.theme();
        let enabled = self.action_enabled(button.action);
        let state = if enabled {
            self.buttons
                .visual_state(&button.action, self.action_active(button.action))
        } else {
            ButtonVisualState::Normal
        };
        let fill = match state {
            ButtonVisualState::Normal => {
                if menu {
                    theme.top
                } else {
                    theme.panel_alt
                }
            }
            ButtonVisualState::Hovered => theme.button_hover,
            ButtonVisualState::Pressed => theme.button_pressed,
            ButtonVisualState::Active => theme.active,
        };
        let text = if enabled { theme.text } else { theme.muted };
        list.fill_rect(button.rect, fill);
        if state != ButtonVisualState::Normal {
            list.stroke_rect(button.rect, 1.0, theme.accent);
        }
        if button.rect.width == 32.0 {
            if let Some(name) = icon(button.action) {
                draw_lucide_icon(
                    list,
                    button.rect.inset(7.0),
                    name,
                    if enabled { text } else { theme.border },
                );
                return;
            }
        }
        if button.action == ButtonAction::Menu(ActionGroup::Canvas) {
            draw_text(
                list,
                button.rect.x + 10.0,
                button.rect.y + 10.0,
                button.label,
                1.1,
                text,
            );
            draw_lucide_icon(
                list,
                Rect::new(button.rect.right() - 24.0, button.rect.y + 8.0, 16.0, 16.0),
                "ellipsis",
                text,
            );
            return;
        }
        let text_width = measure_text_width(button.label, 1.1);
        draw_text(
            list,
            button.rect.x + ((button.rect.width - text_width) * 0.5).max(6.0),
            button.rect.y + 10.0,
            button.label,
            1.1,
            text,
        );
    }

    pub(super) fn draw_panel_header(
        &self,
        list: &mut DrawList,
        rect: Rect,
        title: &str,
        detail: &str,
    ) {
        let theme = self.theme();
        list.fill_rect(rect, theme.panel);
        list.fill_rect(
            Rect::new(rect.right() - 1.0, rect.y, 1.0, rect.height),
            theme.border,
        );
        draw_text(list, rect.x + 12.0, rect.y + 17.0, title, 1.3, theme.text);
        let width = measure_text_width(detail, 1.05);
        if width + measure_text_width(title, 1.3) + 40.0 < rect.width {
            draw_text(
                list,
                rect.right() - width - 12.0,
                rect.y + 19.0,
                detail,
                1.05,
                theme.muted,
            );
        }
    }

    pub(super) fn draw_context_controls(&self, list: &mut DrawList) {
        for button in self.context_buttons() {
            self.draw_control(list, button, false);
        }
    }

    pub(super) fn draw_tooltip(&self, list: &mut DrawList) {
        if self.open_group.is_some() || self.pointer_down {
            return;
        }
        let Some(point) = self.pointer else {
            return;
        };
        let Some(button) = self
            .toolbar_buttons()
            .into_iter()
            .chain(self.context_buttons())
            .find(|b| b.rect.width == 32.0 && b.rect.contains(point))
        else {
            return;
        };
        let label = match button.action {
            ButtonAction::Undo => "Undo  (Ctrl+Z)",
            ButtonAction::Redo => "Redo  (Ctrl+Y)",
            ButtonAction::SaveProject => "Save as...  (Ctrl+S)",
            ButtonAction::LoadProject => "Open project  (Ctrl+O)",
            ButtonAction::CopyGraphic => "Copy graphic  (Ctrl+C)",
            _ => button.label,
        };
        let theme = self.theme();
        let width = measure_text_width(label, 1.15) + 20.0;
        let x = button.rect.x.min((self.size.width - width - 8.0).max(0.0));
        let rect = Rect::new(x, button.rect.bottom() + 7.0, width, 30.0);
        list.fill_rect(rect, theme.top);
        list.stroke_rect(rect, 1.0, theme.border);
        draw_text(list, x + 10.0, rect.y + 9.0, label, 1.15, theme.text);
    }
}

// Image textures (including text) are sRGB; geometry is sent to a linear shader.
// Convert geometry only so all three editing views display identical colors.
pub(super) fn linear_color(mut color: Color) -> Color {
    let channel = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    color.r = channel(color.r);
    color.g = channel(color.g);
    color.b = channel(color.b);
    color
}

#[cfg(test)]
pub(super) fn srgb_color(mut color: Color) -> Color {
    let channel = |v: f32| {
        if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    };
    color.r = channel(color.r);
    color.g = channel(color.g);
    color.b = channel(color.b);
    color
}

pub(super) fn geometry_colors(source: DrawList, convert: fn(Color) -> Color) -> DrawList {
    let mut list = DrawList::new(convert(source.clear_color));
    for command in source.commands() {
        let mut command = command.clone();
        match &mut command {
            DrawCommand::FillRect { color, .. } => *color = convert(*color),
            DrawCommand::FillRectCorners {
                top_left,
                top_right,
                bottom_left,
                bottom_right,
                ..
            } => {
                for color in [top_left, top_right, bottom_left, bottom_right] {
                    *color = convert(*color);
                }
            }
            DrawCommand::FillTriangles { vertices } => {
                let updated: Vec<_> = vertices
                    .iter()
                    .map(|v| {
                        let mut v = *v;
                        v.color = convert(v.color);
                        v
                    })
                    .collect();
                *vertices = updated.into();
            }
            _ => {}
        }
        list.push(command);
    }
    list
}
