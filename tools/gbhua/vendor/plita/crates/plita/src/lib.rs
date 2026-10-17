//! PLITA UI top-level crate.
//!
//! Phase 1 v2 exposes a small application builder, GPU-backed window loop,
//! retained creative shell, widget tree, and event-routing primitives. Later
//! phases will add docking, canvas editing, command history, and the final text
//! system.

pub use plita_app::{App, AppBuilder, AppConfig, AppError, Result};
pub use plita_render::{ClearColor, RenderConfig, RenderError, RenderSize};
pub use plita_ui::{
    active_font_family, active_font_source, active_memory_font_family, active_text_backend,
    default_font_family, draw_box_shadow, draw_lucide_icon, draw_memory_editor,
    draw_memory_editor_scaled, draw_memory_viewer, draw_panel, draw_text, lucide_icon_names,
    measure_text_width, memory_viewer_address_at_point, phase1_demo_draw_list, BoxShadow,
    ButtonInteractionState, ButtonRelease, ButtonVisualState, CanvasDocument, Color, ColoredVertex,
    CommandDescriptor, CommandHistory, CommandId, CommandRegistry, Constraints, CreativeLayout,
    CreativeShell, DockAreaKind, DockLayoutGeometry, DockLayoutProfile, DockLayoutState,
    DockNodeGeometry, DockNodeGeometryKind, DockPanelKind, DrawCommand, DrawList, LayerNode,
    MemoryEditorState, MemoryViewerPalette, PanelRadialHighlight, PanelStyle, Point, PointerButton,
    Rect, ScrollAreaState, ScrollbarStyle, Size, SplitterKind, TextInputState, ThemeTokens,
    ToolKind, UiEvent, UiResponse, Widget, WidgetId, WidgetNode, WidgetRole, WidgetTree,
    WidgetVisualState, CREATIVE_MEMORY_WINDOW_ID, DEFAULT_FONT_FAMILY,
};
pub use plita_window::{AuxiliaryWindowDescriptor, PlitaProgram};

#[cfg(feature = "python")]
pub mod python;

/// Convenient imports for examples and applications.
pub mod prelude {
    pub use crate::{
        active_font_family, active_font_source, active_memory_font_family, active_text_backend,
        default_font_family, draw_box_shadow, draw_lucide_icon, draw_memory_editor,
        draw_memory_editor_scaled, draw_memory_viewer, draw_panel, draw_text, lucide_icon_names,
        measure_text_width, memory_viewer_address_at_point, phase1_demo_draw_list, App, AppBuilder,
        AppConfig, AuxiliaryWindowDescriptor, BoxShadow, ButtonInteractionState, ButtonRelease,
        ButtonVisualState, CanvasDocument, ClearColor, Color, ColoredVertex, CommandDescriptor,
        CommandHistory, CommandId, CommandRegistry, Constraints, CreativeLayout, CreativeShell,
        DockAreaKind, DockLayoutGeometry, DockLayoutProfile, DockLayoutState, DockNodeGeometry,
        DockNodeGeometryKind, DockPanelKind, DrawCommand, DrawList, LayerNode, MemoryEditorState,
        MemoryViewerPalette, PanelRadialHighlight, PanelStyle, PlitaProgram, Point, PointerButton,
        Rect, RenderConfig, RenderSize, Result, ScrollAreaState, ScrollbarStyle, Size,
        SplitterKind, TextInputState, ThemeTokens, ToolKind, UiEvent, UiResponse, Widget, WidgetId,
        WidgetNode, WidgetRole, WidgetTree, WidgetVisualState, CREATIVE_MEMORY_WINDOW_ID,
        DEFAULT_FONT_FAMILY,
    };

    #[cfg(feature = "python")]
    pub use crate::python;
}
