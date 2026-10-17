use super::*;
use plita::prelude::*;
#[path = "bytecode_input.rs"]
mod bytecode_input;
use bytecode_input::BytecodeInput;
#[path = "workspace_ui.rs"]
mod workspace_ui;

impl PaletteColor {
    fn to_color(self) -> Color {
        Color::rgb8(self.r, self.g, self.b)
    }
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn project_default_filename(project_name: &str) -> String {
    let mut stem = project_name
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    stem = stem.trim().trim_matches('.').to_string();
    if stem.is_empty() {
        stem = "kitaqgb_assets".to_string();
    }
    format!("{stem}.gbh")
}

fn ensure_project_save_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("gbh");
    }
    path
}

#[cfg(not(test))]
fn pick_project_save_path(current_path: Option<&Path>, project_name: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save GBHUA project")
        .add_filter("GBHUA project", &["gbh", "json"])
        .add_filter("GBTD tiles", &["gbr", "gbtd", "gbtb"])
        .add_filter("GBMB map", &["gbm", "gbmb"])
        .add_filter("All files", &["*"]);

    if let Some(path) = current_path {
        if let Some(parent) = path.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(file_name) = path.file_name() {
            dialog = dialog.set_file_name(file_name.to_string_lossy());
        }
    } else {
        dialog = dialog.set_file_name(project_default_filename(project_name));
    }

    dialog.save_file().map(ensure_project_save_extension)
}

fn bytecode_text_from_key(key: &str) -> Option<String> {
    let value = normalize_key(key);
    if value.chars().count() == 1 {
        let ch = value.chars().next()?;
        if is_bytecode_input_char(ch) {
            return Some(ch.to_string());
        }
    }
    None
}

fn is_bytecode_input_char(ch: char) -> bool {
    ch.is_ascii_hexdigit() || matches!(ch, 'x' | 'X' | '$' | ',' | ';' | '\n' | '\r' | ' ' | '\t')
}

fn read_clipboard_text() -> std::result::Result<String, String> {
    #[cfg(test)]
    {
        if let Some(text) = TEST_CLIPBOARD_TEXT
            .lock()
            .map_err(|_| "test clipboard lock poisoned".to_string())?
            .take()
        {
            return Ok(text);
        }
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|err| err.to_string())?;
    clipboard.get_text().map_err(|err| err.to_string())
}

#[cfg(test)]
fn set_test_clipboard_text(text: &str) {
    *TEST_CLIPBOARD_TEXT.lock().expect("test clipboard lock") = Some(text.to_string());
}

#[cfg(test)]
fn test_export_path(name: &str) -> PathBuf {
    let thread = std::thread::current();
    let name_space: String = thread
        .name()
        .unwrap_or("unnamed")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let directory = std::env::temp_dir()
        .join(format!("gbhua-exports-{}", std::process::id()))
        .join(name_space);
    fs::create_dir_all(&directory).expect("test export directory");
    directory.join(name)
}

fn menu_entries(group: ActionGroup) -> Vec<(ButtonAction, &'static str)> {
    match group {
        ActionGroup::Tools => vec![
            (ButtonAction::Tool(StudioTool::Pixel), "PIXEL"),
            (ButtonAction::Tool(StudioTool::MetaSprite), "METASPRITE"),
            (ButtonAction::Tool(StudioTool::Map), "MAP"),
            (ButtonAction::Tool(StudioTool::Rect), "RECT"),
            (ButtonAction::Tool(StudioTool::Fill), "FILL"),
            (ButtonAction::Tool(StudioTool::Attr), "ATTR"),
            (ButtonAction::Tool(StudioTool::Collision), "COLL"),
            (ButtonAction::Tool(StudioTool::Event), "EVT"),
        ],
        ActionGroup::Edit => vec![
            (ButtonAction::Undo, "UNDO"),
            (ButtonAction::Redo, "REDO"),
            (ButtonAction::CopyGraphic, "COPY GRAPHIC"),
            (ButtonAction::PasteGraphic, "PASTE GRAPHIC"),
        ],
        ActionGroup::File => vec![
            (ButtonAction::LoadProject, "LOAD"),
            (ButtonAction::SaveProject, "SAVE"),
            (ButtonAction::ImportPng, "IMPORT PNG"),
        ],
        ActionGroup::Tiles => vec![
            (ButtonAction::AddTile, "ADD TILE"),
            (ButtonAction::FillTile, "FILL TILE"),
            (ButtonAction::FlipX, "FLIP X"),
            (ButtonAction::FlipY, "FLIP Y"),
            (ButtonAction::Rotate, "ROTATE"),
            (ButtonAction::Dedupe, "DEDUPE"),
            (ButtonAction::AutoCollision, "AUTO COLL"),
            (ButtonAction::AutoTile, "AUTOTILE"),
        ],
        ActionGroup::Canvas => GraphicSize::ALL
            .into_iter()
            .map(|size| (ButtonAction::SetGraphicSize(size), size.label()))
            .collect(),
        ActionGroup::Sprites => vec![
            (ButtonAction::NewMetaSprite, "NEW META"),
            (ButtonAction::FramePrev, "PREV FRAME"),
            (ButtonAction::FrameNext, "NEXT FRAME"),
            (ButtonAction::FrameAdd, "ADD FRAME"),
            (ButtonAction::FrameDuplicate, "DUP FRAME"),
            (ButtonAction::FrameClear, "CLEAR FRAME"),
        ],
        ActionGroup::Export => vec![
            (ButtonAction::ExportPng, "PNG OUT"),
            (ButtonAction::ExportC, "C OUT"),
            (ButtonAction::ExportGbr, "GBR OUT"),
            (ButtonAction::ExportGbm, "GBM OUT"),
        ],

        ActionGroup::View => vec![
            (ButtonAction::MapZoom(0), "FIT MAP"),
            (ButtonAction::MapZoom(1), "MAP 1X"),
            (ButtonAction::MapZoom(2), "MAP 2X"),
            (ButtonAction::MapZoom(4), "MAP 4X"),
            (ButtonAction::ToggleGrid, "GRID"),
            (ButtonAction::ToggleOverlays, "COLL / EVT"),
        ],
    }
}

#[cfg(test)]
fn action_group_for(action: ButtonAction) -> Option<ActionGroup> {
    match action {
        ButtonAction::Sidebar(_) | ButtonAction::ZoomIn | ButtonAction::ZoomOut => None,
        ButtonAction::Menu(_) => None,
        ButtonAction::Tool(_) => Some(ActionGroup::Tools),
        ButtonAction::Undo
        | ButtonAction::Redo
        | ButtonAction::CopyGraphic
        | ButtonAction::PasteGraphic => Some(ActionGroup::Edit),
        ButtonAction::MapZoom(_) | ButtonAction::ToggleGrid | ButtonAction::ToggleOverlays => {
            Some(ActionGroup::View)
        }
        ButtonAction::LoadProject | ButtonAction::SaveProject | ButtonAction::ImportPng => {
            Some(ActionGroup::File)
        }
        ButtonAction::AddTile
        | ButtonAction::FillTile
        | ButtonAction::FlipX
        | ButtonAction::FlipY
        | ButtonAction::Rotate
        | ButtonAction::Dedupe
        | ButtonAction::AutoCollision
        | ButtonAction::AutoTile => Some(ActionGroup::Tiles),
        ButtonAction::SetGraphicSize(_) => Some(ActionGroup::Canvas),
        ButtonAction::NewMetaSprite
        | ButtonAction::FramePrev
        | ButtonAction::FrameNext
        | ButtonAction::FrameAdd
        | ButtonAction::FrameDuplicate
        | ButtonAction::FrameClear => Some(ActionGroup::Sprites),
        ButtonAction::ExportC
        | ButtonAction::ExportGbr
        | ButtonAction::ExportGbm
        | ButtonAction::ExportPng => Some(ActionGroup::Export),
        ButtonAction::SetColorMode(_) => None,
        ButtonAction::BytecodeImport | ButtonAction::BytecodeExport => None,
    }
}

fn dropdown_label_width(label: &str) -> f32 {
    34.0 + label.len() as f32 * 8.0
}

pub fn run_gui() -> Result<()> {
    configure_bundled_fonts();
    App::new()
        .title(WINDOW_TITLE)
        .window_size(1360, 820)
        .clear_color(ClearColor::rgb(0.075, 0.082, 0.09))
        .continuous_redraw(false)
        .run_program(AssetStudioApp::new)
}

fn configure_bundled_fonts() {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let Some(directory) = executable.parent() else {
        return;
    };
    // Finder does not set the working directory to the application's resources.
    for root in [
        directory.join("assets/fonts"),
        directory.join("../Resources/assets/fonts"),
    ] {
        for (variable, file) in [
            ("PLITA_UI_FONT", "Roboto[wdth,wght].ttf"),
            ("PLITA_MEMORY_FONT", "RobotoMono-Regular.ttf"),
        ] {
            let path = root.join(file);
            if std::env::var_os(variable).is_none() && path.is_file() {
                std::env::set_var(variable, path);
            }
        }
    }
}

fn render_atlas_with_palette(
    project: &Project,
    columns: usize,
    palette: &[PaletteColor],
) -> Vec<u8> {
    let columns = columns.max(1);
    let rows = (project.tiles.len() + columns - 1) / columns;
    let width = columns * TILE_SIZE;
    let height = rows.max(1) * TILE_SIZE;
    let mut pixels = vec![0; width * height * 4];
    for (index, tile) in project.tiles.iter().enumerate() {
        let ox = (index % columns) * TILE_SIZE;
        let oy = (index / columns) * TILE_SIZE;
        blit_tile(&mut pixels, width, ox, oy, tile, palette, false, false);
    }
    pixels
}

#[cfg(test)]
fn render_map_with_mode(project: &Project, mode: ColorMode) -> Vec<u8> {
    render_map_with_overlays(project, mode, true)
}

fn render_map_with_overlays(project: &Project, mode: ColorMode, overlays: bool) -> Vec<u8> {
    let width = project.map_width as usize * TILE_SIZE;
    let height = project.map_height as usize * TILE_SIZE;
    let mut pixels = vec![0; width * height * 4];
    for y in 0..project.map_height as usize {
        for x in 0..project.map_width as usize {
            let cell = project.map[y * project.map_width as usize + x];
            let tile = &project.tiles[cell.tile as usize];
            let palette = match mode {
                ColorMode::Dmg => &DMG_PALETTE,
                ColorMode::Cgb => project.palette(cell.palette as usize),
            };
            blit_tile(
                &mut pixels,
                width,
                x * TILE_SIZE,
                y * TILE_SIZE,
                tile,
                palette,
                cell.flip_x,
                cell.flip_y,
            );
            if overlays && cell.collision != 0 {
                tint_tile(
                    &mut pixels,
                    width,
                    x * TILE_SIZE,
                    y * TILE_SIZE,
                    [210, 67, 67, 72],
                );
            }
            if overlays && cell.event != 0 {
                tint_tile(
                    &mut pixels,
                    width,
                    x * TILE_SIZE,
                    y * TILE_SIZE,
                    [64, 188, 214, 72],
                );
            }
        }
    }
    pixels
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StudioTool {
    Pixel,
    MetaSprite,
    Map,
    Rect,
    Fill,
    Attr,
    Collision,
    Event,
}

impl StudioTool {
    fn label(self) -> &'static str {
        match self {
            Self::Pixel => "PIXEL",
            Self::MetaSprite => "META",
            Self::Map => "MAP",
            Self::Rect => "RECT",
            Self::Fill => "FILL",
            Self::Attr => "ATTR",
            Self::Collision => "COLL",
            Self::Event => "EVT",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ColorMode {
    Dmg,
    Cgb,
}

impl ColorMode {
    fn label(self) -> &'static str {
        match self {
            Self::Dmg => "DMG",
            Self::Cgb => "CGB",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActionGroup {
    Tools,
    Edit,
    File,
    Tiles,
    Canvas,
    Sprites,
    Export,
    View,
}

impl ActionGroup {
    fn label(self) -> &'static str {
        match self {
            Self::Tools => "TOOLS",
            Self::Edit => "EDIT",
            Self::File => "FILE",
            Self::Tiles => "TILES",
            Self::Canvas => "SIZE",
            Self::Sprites => "SPRITES",
            Self::Export => "EXPORT",
            Self::View => "VIEW",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ButtonAction {
    Sidebar(bool),
    ZoomIn,
    ZoomOut,
    Menu(ActionGroup),
    Tool(StudioTool),
    AddTile,
    FillTile,
    FlipX,
    FlipY,
    Rotate,
    Dedupe,
    AutoCollision,
    AutoTile,
    NewMetaSprite,
    FramePrev,
    FrameNext,
    FrameAdd,
    FrameDuplicate,
    FrameClear,
    Undo,
    Redo,
    CopyGraphic,
    PasteGraphic,
    MapZoom(u8),
    ToggleGrid,
    ToggleOverlays,
    LoadProject,
    SaveProject,
    ImportPng,
    ExportPng,
    ExportC,
    ExportGbr,
    ExportGbm,
    SetColorMode(ColorMode),
    SetGraphicSize(GraphicSize),
    BytecodeImport,
    BytecodeExport,
}

#[derive(Clone, Copy, Debug)]
struct ActionButton {
    action: ButtonAction,
    rect: Rect,
    label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DragTarget {
    None,
    TilePixels,
    MetaSpriteParts,
    MapCells,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScrollTarget {
    Atlas,
    Inspector,
    Map,
}

struct EditorSnapshot {
    project: Project,
    graphic_size: GraphicSize,
    selected_tile: usize,
    selected_palette: u8,
    color_mode: ColorMode,
    current_path: Option<PathBuf>,
}

struct EditorState {
    project: Project,
    selected_tile: usize,
    selected_color: u8,
    selected_palette: u8,
    selected_collision: u8,
    selected_event: u8,
    selected_metasprite: usize,
    selected_metasprite_frame: usize,
    selected_tool: StudioTool,
    color_mode: ColorMode,
    graphic_size: GraphicSize,
    map_cursor: Option<(usize, usize)>,
    status: String,
    current_path: Option<PathBuf>,
    bytecode: BytecodeInput,
    saved_project: Option<Project>,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
}

impl EditorState {
    fn new() -> Self {
        let mut state = Self {
            project: Project::default(),
            selected_tile: 4,
            selected_color: 3,
            selected_palette: 0,
            selected_collision: 1,
            selected_event: 1,
            selected_metasprite: 0,
            selected_metasprite_frame: 0,
            selected_tool: StudioTool::Map,
            color_mode: ColorMode::Cgb,
            graphic_size: GraphicSize::Size8x8,
            map_cursor: None,
            status: "Ready".to_string(),
            current_path: None,
            bytecode: BytecodeInput::new(),
            saved_project: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        // An untouched new document is clean even before its first Save As.
        state.saved_project = Some(state.project.clone());
        state.refresh_bytecode_from_selected_tile();
        state
    }

    fn load_project_path(&mut self, path: impl AsRef<Path>) -> StudioResult<()> {
        let path = path.as_ref();
        let project = Project::load_project_file(path)?;
        self.capture_undo();
        self.project = project;
        self.saved_project = Some(self.project.clone());
        self.current_path = Some(path.to_path_buf());
        self.clamp_selection();
        self.refresh_bytecode_from_selected_tile();
        self.status = format!("Loaded {}", path_label(path));
        Ok(())
    }

    fn save_project_to_path(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        match self.project.save_project_file(&path) {
            Ok(()) => {
                self.current_path = Some(path.to_path_buf());
                if matches!(asset_file_kind(path), AssetFileKind::Json) {
                    self.saved_project = Some(self.project.clone());
                }
                self.status = format!("Saved {}", path_label(&path));
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    fn capture_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = self.snapshot();
            self.restore(previous);
            self.redo_stack.push(current);
            self.clamp_selection();
            self.refresh_bytecode_from_selected_tile();
            self.status = "Undo".to_string();
        } else {
            self.status = "Nothing to undo".to_string();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = self.snapshot();
            self.restore(next);
            self.undo_stack.push(current);
            self.clamp_selection();
            self.refresh_bytecode_from_selected_tile();
            self.status = "Redo".to_string();
        } else {
            self.status = "Nothing to redo".to_string();
        }
    }

    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            project: self.project.clone(),
            graphic_size: self.graphic_size,
            selected_tile: self.selected_tile,
            selected_palette: self.selected_palette,
            color_mode: self.color_mode,
            current_path: self.current_path.clone(),
        }
    }

    fn restore(&mut self, snapshot: EditorSnapshot) {
        self.project = snapshot.project;
        self.graphic_size = snapshot.graphic_size;
        self.selected_tile = snapshot.selected_tile;
        self.selected_palette = snapshot.selected_palette;
        self.color_mode = snapshot.color_mode;
        self.current_path = snapshot.current_path;
        self.bytecode.input.focused = false;
    }

    fn is_dirty(&self) -> bool {
        self.saved_project.as_ref() != Some(&self.project)
    }

    fn image_mode(&self) -> image_io::ImageMode {
        match self.color_mode {
            ColorMode::Dmg => image_io::ImageMode::Dmg,
            ColorMode::Cgb => image_io::ImageMode::Cgb,
        }
    }

    fn import_png_path(&mut self, path: &Path) -> StudioResult<()> {
        let (project, report) = image_io::import_png(
            path,
            &image_io::ImportOptions {
                mode: self.image_mode(),
                resize: None,
            },
        )?;
        self.capture_undo();
        self.project = project;
        self.current_path = None;
        self.saved_project = None;
        self.selected_tile = 0;
        self.selected_palette = 0;
        self.graphic_size = GraphicSize::Size8x8;
        self.bytecode.input.focused = false;
        self.clamp_selection();
        self.refresh_bytecode_from_selected_tile();
        self.status = format!(
            "Imported {}: {} tiles, {} palettes{}",
            path_label(path),
            report.tile_count,
            report.palette_count,
            if report.warnings.is_empty() {
                String::new()
            } else {
                format!(" | {}", report.warnings.join("; "))
            }
        );
        Ok(())
    }

    fn load_asset_path(&mut self, path: &Path) -> StudioResult<()> {
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        {
            self.import_png_path(path)
        } else {
            self.load_project_path(path)
        }
    }

    fn clamp_selection(&mut self) {
        self.selected_tile = self
            .selected_tile
            .min(self.project.tiles.len().saturating_sub(1));
        self.selected_palette &= 0x07;
        self.selected_color &= 0x03;
        self.selected_metasprite = self
            .selected_metasprite
            .min(self.project.metasprites.len().saturating_sub(1));
        let frame_count = self
            .project
            .metasprites
            .get(self.selected_metasprite)
            .map(|metasprite| metasprite.frames.len())
            .unwrap_or(0);
        self.selected_metasprite_frame = self
            .selected_metasprite_frame
            .min(frame_count.saturating_sub(1));
    }

    fn refresh_bytecode_from_selected_tile(&mut self) {
        if let Ok(tile) = self.project.tile(self.selected_tile) {
            self.bytecode
                .set_text(format_tile_bytecode(&tile.encoded_2bpp()));
        }
    }

    fn refresh_bytecode_if_unfocused(&mut self) {
        if !self.bytecode.input.focused {
            self.refresh_bytecode_from_selected_tile();
        }
    }
}

pub struct AssetStudioApp {
    size: Size,
    palette_tab: bool,
    pointer: Option<Point>,
    editor: EditorState,
    buttons: ButtonInteractionState<ButtonAction>,
    open_group: Option<ActionGroup>,
    atlas_scroll: ScrollAreaState,
    inspector_scroll: ScrollAreaState,
    map_scroll: ScrollAreaState,
    map_zoom: u8,
    grid_visible: bool,
    overlays_visible: bool,
    copied_graphic: Option<(GraphicSize, Vec<u8>)>,
    last_pixel: Option<(usize, usize)>,
    scroll_drag: Option<ScrollTarget>,
    pointer_down: bool,
    drag_target: DragTarget,
    #[cfg(test)]
    save_dialog_path: Option<PathBuf>,
    #[cfg(test)]
    discard_confirmed: bool,
}

impl AssetStudioApp {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            palette_tab: false,
            pointer: None,
            editor: EditorState::new(),
            buttons: ButtonInteractionState::new(),
            open_group: None,
            atlas_scroll: ScrollAreaState::new(),
            inspector_scroll: ScrollAreaState::new(),
            map_scroll: ScrollAreaState::new(),
            map_zoom: 0,
            grid_visible: true,
            overlays_visible: true,
            copied_graphic: None,
            last_pixel: None,
            scroll_drag: None,
            pointer_down: false,
            drag_target: DragTarget::None,
            #[cfg(test)]
            save_dialog_path: None,
            #[cfg(test)]
            discard_confirmed: true,
        }
    }

    fn theme(&self) -> StudioTheme {
        StudioTheme {
            bg: Color::rgb8(23, 24, 26),
            top: Color::rgb8(35, 36, 39),
            panel: Color::rgb8(28, 29, 32),
            panel_alt: Color::rgb8(38, 40, 43),
            border: Color::rgb8(62, 65, 69),
            text: Color::rgb8(235, 236, 239),
            muted: Color::rgb8(157, 163, 171),
            accent: Color::rgb8(86, 180, 170),
            accent2: Color::rgb8(221, 174, 84),
            active: Color::rgb8(41, 79, 73),
            button_hover: Color::rgb8(56, 59, 64),
            button_pressed: Color::rgb8(57, 99, 91),
            danger: Color::rgb8(195, 83, 82),
        }
    }

    fn scrollbar_style(&self) -> ScrollbarStyle {
        ScrollbarStyle {
            track: Color::rgba8(10, 13, 16, 170),
            thumb: Color::rgba8(112, 146, 146, 180),
            thumb_hovered: Color::rgba8(136, 173, 169, 220),
            thumb_dragged: Color::rgba8(166, 196, 188, 235),
            corner: Color::rgba8(10, 13, 16, 190),
            ..ScrollbarStyle::default()
        }
    }

    fn compact(&self) -> bool {
        self.size.width < 1180.0
    }

    fn show_atlas(&self) -> bool {
        !self.compact() || !self.palette_tab
    }

    fn show_inspector(&self) -> bool {
        !self.compact() || self.palette_tab
    }

    fn top_bar_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.size.width, 44.0)
    }

    fn status_rect(&self) -> Rect {
        Rect::new(
            0.0,
            (self.size.height - 28.0).max(0.0),
            self.size.width,
            28.0,
        )
    }

    fn atlas_panel_rect(&self) -> Rect {
        Rect::new(0.0, 44.0, 216.0, (self.size.height - 72.0).max(0.0))
    }

    fn inspector_rect(&self) -> Rect {
        if self.compact() {
            return self.atlas_panel_rect();
        }
        Rect::new(
            self.size.width - 244.0,
            44.0,
            244.0,
            self.size.height - 72.0,
        )
    }

    fn center_rect(&self) -> Rect {
        let left = self.atlas_panel_rect().right();
        let right = if self.compact() {
            self.size.width
        } else {
            self.inspector_rect().x
        };
        Rect::new(left, 44.0, (right - left).max(0.0), self.size.height - 72.0)
    }

    fn graphic_panel_rect(&self) -> Rect {
        let center = self.center_rect();
        let width = (center.width * 0.44).clamp(248.0, 360.0).min(center.width);
        Rect::new(center.x, center.y, width, center.height)
    }

    fn content_top(&self) -> f32 {
        self.top_bar_rect().bottom() + 88.0
    }

    fn tile_editor_area_rect(&self) -> Rect {
        let panel = self.graphic_panel_rect();
        let side = (panel.width - 24.0)
            .min(self.status_rect().y - self.content_top() - 214.0)
            .max(64.0);
        Rect::new(
            panel.x + (panel.width - side) * 0.5,
            self.content_top(),
            side,
            side,
        )
    }

    fn tile_editor_rect(&self) -> Rect {
        let area = self.tile_editor_area_rect();
        if self.editor.selected_tool == StudioTool::MetaSprite {
            return area;
        }
        let (width, height) = self.editor.graphic_size.dimensions();
        let scale = (area.width / width as f32).min(area.height / height as f32);
        Rect::new(
            area.x + (area.width - width as f32 * scale) * 0.5,
            area.y,
            width as f32 * scale,
            height as f32 * scale,
        )
    }

    fn palette_rect(&self) -> Rect {
        let area = self.tile_editor_area_rect();
        Rect::new(area.x, area.bottom() + 12.0, area.width, 34.0)
    }

    fn bytecode_window_rect(&self) -> Rect {
        let palette = self.palette_rect();
        let panel = self.graphic_panel_rect();
        Rect::new(
            panel.x + 12.0,
            palette.bottom() + 16.0,
            panel.width - 24.0,
            140.0,
        )
    }

    fn bytecode_input_rect(&self) -> Rect {
        let window = self.bytecode_window_rect();
        Rect::new(window.x, window.y + 30.0, window.width, 64.0)
    }

    fn bytecode_buttons(&self) -> Vec<ActionButton> {
        let window = self.bytecode_window_rect();
        let width = (window.width - 8.0) * 0.5;
        vec![
            ActionButton {
                action: ButtonAction::BytecodeImport,
                rect: Rect::new(window.x, window.y + 104.0, width, 30.0),
                label: "Import",
            },
            ActionButton {
                action: ButtonAction::BytecodeExport,
                rect: Rect::new(window.x + width + 8.0, window.y + 104.0, width, 30.0),
                label: "Export",
            },
        ]
    }

    fn atlas_viewport_rect(&self) -> Rect {
        let panel = self.atlas_panel_rect();
        Rect::new(
            panel.x + 8.0,
            self.content_top() - 4.0,
            panel.width - 16.0,
            (panel.bottom() - self.content_top() - 8.0).max(1.0),
        )
    }

    fn atlas_content_size(&self) -> Size {
        let rows = ((self.editor.project.tiles.len() + 7) / 8).max(1) as f32;
        Size::new(200.0, rows * 24.0 + 8.0)
    }

    fn inspector_viewport_rect(&self) -> Rect {
        let panel = self.inspector_rect();
        Rect::new(
            panel.x + 12.0,
            self.content_top() - 4.0,
            panel.width - 24.0,
            (panel.bottom() - self.content_top() - 8.0).max(1.0),
        )
    }

    fn inspector_content_size(&self) -> Size {
        Size::new(self.inspector_viewport_rect().width, 638.0)
    }

    fn inspector_content_origin(&self) -> Point {
        self.inspector_scroll.content_origin(
            self.inspector_viewport_rect(),
            self.inspector_content_size(),
        )
    }

    fn cgb_palette_grid_rect(&self) -> Rect {
        let origin = self.inspector_content_origin();
        Rect::new(
            origin.x,
            origin.y + 4.0,
            self.inspector_viewport_rect().width - 12.0,
            192.0,
        )
    }

    fn map_panel_rect(&self) -> Rect {
        let center = self.center_rect();
        let graphic = self.graphic_panel_rect();
        Rect::new(
            graphic.right(),
            center.y,
            (center.right() - graphic.right()).max(0.0),
            center.height,
        )
    }

    fn map_viewport_rect(&self) -> Rect {
        let panel = self.map_panel_rect();
        Rect::new(
            panel.x + 12.0,
            self.content_top(),
            (panel.width - 24.0).max(1.0),
            (panel.bottom() - self.content_top() - 12.0).max(1.0),
        )
    }

    fn map_content_size(&self) -> Size {
        let area = self.map_viewport_rect();
        let image_w = self.editor.project.map_width as f32 * TILE_SIZE as f32;
        let image_h = self.editor.project.map_height as f32 * TILE_SIZE as f32;
        let scale = if self.map_zoom == 0 {
            (area.width / image_w).min(area.height / image_h)
        } else {
            self.map_zoom as f32
        };
        Size::new(image_w * scale, image_h * scale)
    }

    fn map_view_rect(&self) -> Rect {
        let area = self.map_viewport_rect();
        let content = self.map_content_size();
        let origin = self.map_scroll.content_origin(area, content);
        Rect::new(
            origin.x + (area.width - content.width).max(0.0) * 0.5,
            origin.y,
            content.width,
            content.height,
        )
    }

    fn atlas_image_rect(&self) -> Rect {
        let origin = self
            .atlas_scroll
            .content_origin(self.atlas_viewport_rect(), self.atlas_content_size());
        let rows = ((self.editor.project.tiles.len() + 7) / 8).max(1) as f32;
        Rect::new(origin.x + 4.0, origin.y + 4.0, 192.0, rows * 24.0)
    }

    fn active_palette(&self) -> [PaletteColor; 4] {
        match self.editor.color_mode {
            ColorMode::Dmg => DMG_PALETTE,
            ColorMode::Cgb => {
                let palette = self
                    .editor
                    .project
                    .palette(self.editor.selected_palette as usize);
                [palette[0], palette[1], palette[2], palette[3]]
            }
        }
    }

    fn toolbar_buttons(&self) -> Vec<ActionButton> {
        let mut buttons = Vec::new();
        let mut x = 96.0;
        for (group, width) in [
            (ActionGroup::File, 48.0),
            (ActionGroup::Edit, 48.0),
            (ActionGroup::Tools, 56.0),
            (ActionGroup::Tiles, 54.0),
            (ActionGroup::Sprites, 70.0),
            (ActionGroup::Export, 66.0),
            (ActionGroup::View, 52.0),
        ] {
            buttons.push(ActionButton {
                action: ButtonAction::Menu(group),
                rect: Rect::new(x, 6.0, width, 32.0),
                label: group.label(),
            });
            x += width + 2.0;
        }
        x += 14.0;
        for (action, label) in [
            (ButtonAction::LoadProject, "Open project"),
            (ButtonAction::SaveProject, "Save as..."),
            (ButtonAction::Undo, "Undo"),
            (ButtonAction::Redo, "Redo"),
        ] {
            if x + 32.0 < self.size.width - 128.0 {
                buttons.push(ActionButton {
                    action,
                    rect: Rect::new(x, 6.0, 32.0, 32.0),
                    label,
                });
            }
            x += 36.0;
        }
        for (i, mode) in [ColorMode::Dmg, ColorMode::Cgb].into_iter().enumerate() {
            buttons.push(ActionButton {
                action: ButtonAction::SetColorMode(mode),
                rect: Rect::new(self.size.width - 116.0 + i as f32 * 52.0, 7.0, 50.0, 30.0),
                label: mode.label(),
            });
        }
        buttons
    }

    fn context_buttons(&self) -> Vec<ActionButton> {
        let panel = self.graphic_panel_rect();
        let y = panel.y + 44.0;
        let mut buttons = Vec::new();
        let mut x = panel.x + 12.0;
        for (action, label, width) in [
            (ButtonAction::Tool(StudioTool::Pixel), "Pencil", 32.0),
            (ButtonAction::Tool(StudioTool::Fill), "Fill", 40.0),
            (ButtonAction::CopyGraphic, "Copy graphic", 32.0),
            (ButtonAction::Rotate, "Rotate clockwise", 32.0),
        ] {
            buttons.push(ActionButton {
                action,
                rect: Rect::new(x, y, width, 32.0),
                label,
            });
            x += width + 4.0;
        }
        buttons.push(ActionButton {
            action: ButtonAction::Menu(ActionGroup::Canvas),
            rect: Rect::new(panel.right() - 84.0, y, 72.0, 32.0),
            label: self.editor.graphic_size.label(),
        });
        let map = self.map_panel_rect();
        let mut x = map.x + 12.0;
        for (action, label, width) in [
            (ButtonAction::Tool(StudioTool::Map), "Stamp", 62.0),
            (ButtonAction::MapZoom(0), "Fit map", 32.0),
            (ButtonAction::ZoomOut, "Zoom out", 32.0),
            (ButtonAction::ZoomIn, "Zoom in", 32.0),
            (ButtonAction::ToggleGrid, "Grid", 48.0),
        ] {
            buttons.push(ActionButton {
                action,
                rect: Rect::new(x, y, width, 32.0),
                label,
            });
            x += width + 4.0;
        }
        if self.compact() {
            for (i, label) in ["Tiles", "Palettes"].into_iter().enumerate() {
                buttons.push(ActionButton {
                    action: ButtonAction::Sidebar(i == 1),
                    rect: Rect::new(12.0 + i as f32 * 98.0, 54.0, 94.0, 30.0),
                    label,
                });
            }
        }
        if self.show_atlas() {
            buttons.push(ActionButton {
                action: ButtonAction::AddTile,
                rect: Rect::new(172.0, y, 32.0, 32.0),
                label: "Add tile",
            });
        }
        buttons
    }

    fn dropdown_buttons(&self) -> Vec<ActionButton> {
        let Some(group) = self.open_group else {
            return Vec::new();
        };
        let Some(parent) = self
            .toolbar_buttons()
            .into_iter()
            .chain(self.context_buttons())
            .find(|button| button.action == ButtonAction::Menu(group))
        else {
            return Vec::new();
        };

        let entries = menu_entries(group);
        let width = entries
            .iter()
            .map(|(_, label)| dropdown_label_width(label))
            .fold(parent.rect.width, f32::max)
            .max(112.0);
        let mut buttons = Vec::new();
        let mut y = parent.rect.bottom() + 4.0;
        for (action, label) in entries {
            buttons.push(ActionButton {
                action,
                rect: Rect::new(
                    parent.rect.x.min((self.size.width - width - 8.0).max(0.0)),
                    y,
                    width,
                    30.0,
                ),
                label,
            });
            y += 32.0;
        }
        buttons
    }

    fn action_buttons(&self) -> Vec<ActionButton> {
        self.dropdown_buttons()
            .into_iter()
            .chain(self.toolbar_buttons())
            .chain(self.context_buttons())
            .collect()
    }

    fn inspector_buttons(&self) -> Vec<ActionButton> {
        Vec::new()
    }

    fn handle_pointer_down(&mut self, position: Point, button: PointerButton) -> UiResponse {
        self.pointer = Some(position);
        if button == PointerButton::Primary {
            if let Some(action) = self.button_at(position) {
                self.buttons.handle_pointer_down(Some(action));
                return UiResponse::redraw();
            }
        }
        if button != PointerButton::Primary && button != PointerButton::Secondary {
            return UiResponse::none();
        }
        let menu_closed = self.open_group.take().is_some();
        if menu_closed {
            self.buttons.set_hovered(None);
            return UiResponse::redraw();
        }
        if button == PointerButton::Primary && self.bytecode_input_rect().contains(position) {
            let rect = self.bytecode_input_rect();
            self.editor.bytecode.focus_at(rect, position);
            self.pointer_down = false;
            self.drag_target = DragTarget::None;
            self.buttons.set_hovered(None);
            return UiResponse::redraw();
        }
        let bytecode_focus_changed = if self.editor.bytecode.input.focused {
            self.editor.bytecode.input.focused = false;
            true
        } else {
            false
        };
        let button_changed = self.buttons.set_hovered(None);
        self.pointer_down = button == PointerButton::Primary;
        if button == PointerButton::Primary && self.begin_scrollbar_drag(position) {
            return UiResponse::redraw();
        }
        if self.handle_palette_hit(position) {
            return UiResponse::redraw();
        }
        if self.handle_cgb_palette_grid_hit(position) {
            return UiResponse::redraw();
        }
        if self.handle_atlas_hit(position) {
            return UiResponse::redraw();
        }
        if self.tile_editor_rect().contains(position) {
            if button == PointerButton::Secondary
                && self.editor.selected_tool != StudioTool::MetaSprite
            {
                if let Some((x, y)) = self.tile_pixel_at(position) {
                    let pixels = read_graphic_pixels(
                        &self.editor.project,
                        self.editor.selected_tile,
                        self.editor.graphic_size,
                    );
                    self.editor.selected_color =
                        pixels[y * self.editor.graphic_size.dimensions().0 + x];
                    self.editor.status = format!("Picked color {}", self.editor.selected_color);
                }
                return UiResponse::redraw();
            }
            self.editor.capture_undo();
            self.last_pixel = None;
            if self.editor.selected_tool == StudioTool::MetaSprite {
                self.drag_target = DragTarget::MetaSpriteParts;
                self.paint_metasprite_at(position, button == PointerButton::Secondary);
            } else {
                self.drag_target = DragTarget::TilePixels;
                self.paint_tile_at(position);
            }
            return UiResponse::redraw();
        }
        if self.map_cell_at(position).is_some() {
            if button == PointerButton::Primary {
                self.editor.capture_undo();
            }
            self.drag_target = DragTarget::MapCells;
            self.paint_map_at(position, button == PointerButton::Secondary);
            return UiResponse::redraw();
        }
        if button_changed || menu_closed || bytecode_focus_changed {
            UiResponse::redraw()
        } else {
            UiResponse::none()
        }
    }

    fn handle_pointer_move(&mut self, position: Point) -> UiResponse {
        self.pointer = Some(position);
        if self.scroll_drag.is_some() {
            return if self.drag_scrollbar(position) {
                UiResponse::redraw()
            } else {
                UiResponse::none()
            };
        }

        let hit = self.button_at(position);
        let button_changed = self.buttons.handle_pointer_move(hit);
        if !self.pointer_down {
            let mut changed = button_changed;
            if let Some(cell) = self.map_cell_at(position) {
                if self.editor.map_cursor != Some(cell) {
                    self.editor.map_cursor = Some(cell);
                    changed = true;
                }
            }
            if changed {
                return UiResponse::redraw();
            }
            return UiResponse::none();
        }
        match self.drag_target {
            DragTarget::TilePixels => {
                self.paint_tile_at(position);
                UiResponse::redraw()
            }
            DragTarget::MetaSpriteParts => {
                self.paint_metasprite_at(position, false);
                UiResponse::redraw()
            }
            DragTarget::MapCells => {
                self.paint_map_at(position, false);
                UiResponse::redraw()
            }
            DragTarget::None => UiResponse::none(),
        }
    }

    fn handle_pointer_up(&mut self, position: Point, button: PointerButton) -> UiResponse {
        let was_dragging = self.pointer_down || self.drag_target != DragTarget::None;
        self.pointer_down = false;
        self.drag_target = DragTarget::None;
        self.last_pixel = None;
        let scroll_changed = if button == PointerButton::Primary {
            self.end_scrollbar_drag()
        } else {
            false
        };

        if button == PointerButton::Primary {
            let hit = self.button_at(position);
            let release = self.buttons.handle_pointer_up(hit);
            if let Some(action) = release.activated {
                self.apply_action(action);
            }
            if release.changed || was_dragging || scroll_changed {
                return UiResponse::redraw();
            }
        }

        if was_dragging || scroll_changed {
            UiResponse::redraw()
        } else {
            UiResponse::none()
        }
    }

    fn handle_pointer_wheel(&mut self, position: Point, delta_x: f32, delta_y: f32) -> UiResponse {
        if self.map_viewport_rect().contains(position) {
            let viewport = self.map_viewport_rect();
            let content = self.map_content_size();
            if self.map_scroll.wheel(delta_x, delta_y, viewport, content) {
                return UiResponse::redraw();
            }
        }
        if self.show_atlas() && self.atlas_panel_rect().contains(position) {
            let viewport = self.atlas_viewport_rect();
            let content = self.atlas_content_size();
            if self.atlas_scroll.wheel(delta_x, delta_y, viewport, content) {
                return UiResponse::redraw();
            }
        }
        if self.show_inspector() && self.inspector_rect().contains(position) {
            let viewport = self.inspector_viewport_rect();
            let content = self.inspector_content_size();
            if self
                .inspector_scroll
                .wheel(delta_x, delta_y, viewport, content)
            {
                return UiResponse::redraw();
            }
        }
        UiResponse::none()
    }

    fn begin_scrollbar_drag(&mut self, position: Point) -> bool {
        let style = self.scrollbar_style();
        let map_viewport = self.map_viewport_rect();
        let map_content = self.map_content_size();
        if self
            .map_scroll
            .begin_scrollbar_drag(map_viewport, map_content, position, style)
            .is_some()
        {
            self.scroll_drag = self.map_scroll.drag().map(|_| ScrollTarget::Map);
            return true;
        }
        let atlas_viewport = self.atlas_viewport_rect();
        let atlas_content = self.atlas_content_size();
        if self.show_atlas()
            && self
                .atlas_scroll
                .begin_scrollbar_drag(atlas_viewport, atlas_content, position, style)
                .is_some()
        {
            self.scroll_drag = self.atlas_scroll.drag().map(|_| ScrollTarget::Atlas);
            return true;
        }

        let inspector_viewport = self.inspector_viewport_rect();
        let inspector_content = self.inspector_content_size();
        if self.show_inspector()
            && self
                .inspector_scroll
                .begin_scrollbar_drag(inspector_viewport, inspector_content, position, style)
                .is_some()
        {
            self.scroll_drag = self
                .inspector_scroll
                .drag()
                .map(|_| ScrollTarget::Inspector);
            return true;
        }
        false
    }

    fn drag_scrollbar(&mut self, position: Point) -> bool {
        let style = self.scrollbar_style();
        match self.scroll_drag {
            Some(ScrollTarget::Map) => {
                let viewport = self.map_viewport_rect();
                let content = self.map_content_size();
                self.map_scroll
                    .drag_scrollbar(viewport, content, position, style)
            }
            Some(ScrollTarget::Atlas) => {
                let viewport = self.atlas_viewport_rect();
                let content = self.atlas_content_size();
                self.atlas_scroll
                    .drag_scrollbar(viewport, content, position, style)
            }
            Some(ScrollTarget::Inspector) => {
                let viewport = self.inspector_viewport_rect();
                let content = self.inspector_content_size();
                self.inspector_scroll
                    .drag_scrollbar(viewport, content, position, style)
            }
            None => false,
        }
    }

    fn end_scrollbar_drag(&mut self) -> bool {
        let changed = match self.scroll_drag {
            Some(ScrollTarget::Map) => self.map_scroll.end_scrollbar_drag(),
            Some(ScrollTarget::Atlas) => self.atlas_scroll.end_scrollbar_drag(),
            Some(ScrollTarget::Inspector) => self.inspector_scroll.end_scrollbar_drag(),
            None => false,
        };
        self.scroll_drag = None;
        changed
    }

    fn button_at(&self, position: Point) -> Option<ButtonAction> {
        for button in self
            .action_buttons()
            .into_iter()
            .chain(self.inspector_buttons())
            .chain(self.bytecode_buttons())
        {
            if button.rect.contains(position) {
                return Some(button.action);
            }
        }
        None
    }

    fn handle_palette_hit(&mut self, position: Point) -> bool {
        let palette = self.palette_rect();
        if !palette.contains(position) {
            return false;
        }
        let swatch_w = palette.width / 4.0;
        let index = ((position.x - palette.x) / swatch_w).floor() as i32;
        if (0..4).contains(&index) {
            self.editor.selected_color = index as u8;
            self.editor.status = format!("Color {}", self.editor.selected_color);
            return true;
        }
        false
    }

    fn handle_cgb_palette_grid_hit(&mut self, position: Point) -> bool {
        if !self.show_inspector() || !self.inspector_viewport_rect().contains(position) {
            return false;
        }
        let grid = self.cgb_palette_grid_rect();
        if !grid.contains(position) {
            return false;
        }

        let label_w = 32.0;
        let swatch_w = (grid.width - label_w).max(4.0) / 4.0;
        let color = ((position.x - grid.x - label_w) / swatch_w).floor() as i32;
        match self.editor.color_mode {
            ColorMode::Dmg => {
                if position.y > grid.y + 32.0 {
                    return false;
                }
                if (0..4).contains(&color) {
                    self.editor.selected_color = color as u8;
                    self.editor.status = format!("DMG shade {}", self.editor.selected_color);
                    return true;
                }
            }
            ColorMode::Cgb => {
                let row_h = grid.height / 8.0;
                let palette = ((position.y - grid.y) / row_h).floor() as i32;
                if (0..8).contains(&palette) {
                    self.editor.selected_palette = palette as u8;
                    if (0..4).contains(&color) {
                        self.editor.selected_color = color as u8;
                    }
                    self.editor.status = format!(
                        "Palette {} color {}",
                        self.editor.selected_palette, self.editor.selected_color
                    );
                    return true;
                }
            }
        }
        false
    }

    fn handle_atlas_hit(&mut self, position: Point) -> bool {
        if !self.show_atlas() || !self.atlas_viewport_rect().contains(position) {
            return false;
        }
        let atlas = self.atlas_image_rect();
        if !atlas.contains(position) {
            return false;
        }
        let col = ((position.x - atlas.x) / 24.0).floor() as usize;
        let row = ((position.y - atlas.y) / 24.0).floor() as usize;
        let index = row * 8 + col;
        if index < self.editor.project.tiles.len() {
            self.editor.selected_tile = index;
            self.editor.refresh_bytecode_from_selected_tile();
            self.editor.status = format!("Tile {}", index);
            return true;
        }
        false
    }

    fn paint_tile_at(&mut self, position: Point) {
        let Some((x, y)) = self.tile_pixel_at(position) else {
            self.last_pixel = None;
            return;
        };
        let size = self.editor.graphic_size;
        let (width, height) = size.dimensions();
        let mut pixels = read_graphic_pixels(&self.editor.project, self.editor.selected_tile, size);
        if self.editor.selected_tool == StudioTool::Fill {
            let before = pixels[y * width + x];
            let color = self.editor.selected_color;
            if before == color {
                return;
            }
            let mut stack = vec![(x, y)];
            while let Some((px, py)) = stack.pop() {
                if pixels[py * width + px] != before {
                    continue;
                }
                pixels[py * width + px] = color;
                if px > 0 {
                    stack.push((px - 1, py));
                }
                if py > 0 {
                    stack.push((px, py - 1));
                }
                if px + 1 < width {
                    stack.push((px + 1, py));
                }
                if py + 1 < height {
                    stack.push((px, py + 1));
                }
            }
        } else {
            let (sx, sy) = self.last_pixel.unwrap_or((x, y));
            let steps = sx.abs_diff(x).max(sy.abs_diff(y)).max(1);
            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let px = (sx as f32 + (x as f32 - sx as f32) * t).round() as usize;
                let py = (sy as f32 + (y as f32 - sy as f32) * t).round() as usize;
                pixels[py * width + px] = self.editor.selected_color;
            }
        }
        match write_graphic_pixels(
            &mut self.editor.project,
            self.editor.selected_tile,
            size,
            &pixels,
        ) {
            Ok(_) => {
                self.last_pixel = Some((x, y));
                self.editor.refresh_bytecode_if_unfocused();
                self.editor.status = format!("Pixel {x},{y} = {}", self.editor.selected_color);
            }
            Err(err) => self.editor.status = err.to_string(),
        }
    }

    fn tile_pixel_at(&self, position: Point) -> Option<(usize, usize)> {
        let rect = self.tile_editor_rect();
        if !rect.contains(position) {
            return None;
        }
        let (width, height) = self.editor.graphic_size.dimensions();
        let cell_width = rect.width / width as f32;
        let cell_height = rect.height / height as f32;
        let x = ((position.x - rect.x) / cell_width).floor() as usize;
        let y = ((position.y - rect.y) / cell_height).floor() as usize;
        (x < width && y < height).then_some((x, y))
    }

    fn paint_metasprite_at(&mut self, position: Point, remove: bool) {
        let rect = self.tile_editor_rect();
        if !rect.contains(position) {
            return;
        }
        if self.editor.project.metasprites.is_empty() {
            self.editor.selected_metasprite = self.editor.project.add_metasprite("metasprite_0");
            self.editor.selected_metasprite_frame = 0;
        }
        let logical_x = (position.x - rect.x) * 80.0 / rect.width;
        let logical_y = (position.y - rect.y) * 80.0 / rect.height;
        let dx = (((logical_x - 40.0) / 8.0).floor() * 8.0).clamp(-40.0, 32.0) as i8;
        let dy = (((logical_y - 40.0) / 8.0).floor() * 8.0).clamp(-40.0, 32.0) as i8;
        let selected_tile = self.editor.selected_tile as u16;
        let selected_flags = self.editor.selected_palette & 0x07;
        let frame = match self.editor.project.metasprite_frame_mut(
            self.editor.selected_metasprite,
            self.editor.selected_metasprite_frame,
        ) {
            Ok(frame) => frame,
            Err(err) => {
                self.editor.status = err.to_string();
                return;
            }
        };
        if let Some(index) = frame
            .parts
            .iter()
            .position(|part| part.dx == dx && part.dy == dy)
        {
            if remove {
                frame.parts.remove(index);
                self.editor.status = format!("Removed metasprite part at {dx},{dy}");
            } else {
                frame.parts[index].tile = selected_tile;
                frame.parts[index].flags = selected_flags;
                self.editor.status = format!("Updated metasprite part at {dx},{dy}");
            }
        } else if !remove {
            frame.parts.push(MetaSpritePart {
                dx,
                dy,
                tile: selected_tile,
                flags: selected_flags,
            });
            self.editor.status = format!("Added metasprite part at {dx},{dy}");
        }
    }

    fn paint_map_at(&mut self, position: Point, pick: bool) {
        let Some((x, y)) = self.map_cell_at(position) else {
            return;
        };
        if pick {
            if let Ok(cell) = self.editor.project.map_cell(x, y) {
                self.editor.selected_tile = cell.tile as usize;
                self.editor.selected_palette = cell.palette;
                self.editor.refresh_bytecode_from_selected_tile();
                self.editor.status = format!("Picked {},{}", x, y);
            }
            return;
        }
        if self.editor.selected_tool == StudioTool::Map {
            self.stamp_graphic_map_at(x, y);
            self.editor.map_cursor = Some((x, y));
            return;
        }
        let mut cell = match self.editor.project.map_cell(x, y) {
            Ok(cell) => cell,
            Err(err) => {
                self.editor.status = err.to_string();
                return;
            }
        };
        match self.editor.selected_tool {
            StudioTool::Map | StudioTool::Rect | StudioTool::Fill => {
                cell.tile = self.editor.selected_tile as u16;
                cell.palette = self.editor.selected_palette;
            }
            StudioTool::Attr => {
                cell.palette = self.editor.selected_palette;
                cell.flip_x = !cell.flip_x;
            }
            StudioTool::Collision => {
                cell.collision = u8::from(cell.collision == 0);
            }
            StudioTool::Event => {
                cell.event = if cell.event == 0 {
                    self.editor.selected_event
                } else {
                    0
                };
            }
            StudioTool::Pixel => {
                cell.tile = self.editor.selected_tile as u16;
            }
            StudioTool::MetaSprite => return,
        }
        let result = if self.editor.selected_tool == StudioTool::Fill {
            self.editor.project.flood_fill_map(x, y, cell).map(|count| {
                self.editor.status = format!("Filled {} cells", count);
            })
        } else if self.editor.selected_tool == StudioTool::Rect {
            let count = self.editor.project.paint_rect(x, y, 3, 3, cell);
            self.editor.status = format!("Rect paint {} cells", count);
            Ok(())
        } else {
            self.editor.project.set_map_cell(x, y, cell).map(|_| {
                self.editor.status = format!("Map {},{} tile {}", x, y, cell.tile);
            })
        };
        if let Err(err) = result {
            self.editor.status = err.to_string();
        }
        self.editor.map_cursor = Some((x, y));
    }

    fn stamp_graphic_map_at(&mut self, x: usize, y: usize) {
        match stamp_graphic_map(
            &mut self.editor.project,
            self.editor.selected_tile,
            self.editor.graphic_size,
            x,
            y,
            self.editor.selected_palette,
        ) {
            Ok(changed) => {
                self.editor.status = format!(
                    "Stamped {} graphic at {},{} ({} tiles)",
                    self.editor.graphic_size.label(),
                    x,
                    y,
                    changed
                );
            }
            Err(err) => self.editor.status = err.to_string(),
        }
    }

    fn map_cell_at(&self, position: Point) -> Option<(usize, usize)> {
        let rect = self.map_view_rect();
        if !rect.contains(position) || !self.map_viewport_rect().contains(position) {
            return None;
        }
        let x = ((position.x - rect.x) / rect.width * self.editor.project.map_width as f32).floor();
        let y =
            ((position.y - rect.y) / rect.height * self.editor.project.map_height as f32).floor();
        let x = x as usize;
        let y = y as usize;
        if x < self.editor.project.map_width as usize && y < self.editor.project.map_height as usize
        {
            Some((x, y))
        } else {
            None
        }
    }

    fn apply_action(&mut self, action: ButtonAction) {
        if !self.action_enabled(action) {
            self.open_group = None;
            return;
        }
        match action {
            ButtonAction::Sidebar(palette) => {
                self.palette_tab = palette;
                self.open_group = None;
            }
            ButtonAction::ZoomIn | ButtonAction::ZoomOut => {
                let current = self.map_content_size().width
                    / (self.editor.project.map_width as f32 * TILE_SIZE as f32);
                let levels = [1u8, 2, 4, 8];
                let zoom = if action == ButtonAction::ZoomIn {
                    levels
                        .into_iter()
                        .find(|z| *z as f32 > current + 0.01)
                        .unwrap_or(8)
                } else {
                    levels
                        .into_iter()
                        .rev()
                        .find(|z| (*z as f32) < current - 0.01)
                        .unwrap_or(1)
                };
                self.apply_action(ButtonAction::MapZoom(zoom));
            }
            ButtonAction::MapZoom(zoom) => {
                self.open_group = None;
                self.map_zoom = zoom;
                self.map_scroll = ScrollAreaState::new();
                self.editor.status = if zoom == 0 {
                    "Map fit".into()
                } else {
                    format!("Map {zoom}x")
                };
            }
            ButtonAction::ToggleGrid => {
                self.open_group = None;
                self.grid_visible = !self.grid_visible;
            }
            ButtonAction::ToggleOverlays => {
                self.open_group = None;
                self.overlays_visible = !self.overlays_visible;
            }
            ButtonAction::CopyGraphic => {
                self.open_group = None;
                self.copied_graphic = Some((
                    self.editor.graphic_size,
                    read_graphic_pixels(
                        &self.editor.project,
                        self.editor.selected_tile,
                        self.editor.graphic_size,
                    ),
                ));
                self.editor.status = format!("Copied {}", self.editor.graphic_size.label());
            }
            ButtonAction::PasteGraphic => {
                self.open_group = None;
                if let Some((size, pixels)) = self.copied_graphic.clone() {
                    if self.editor.selected_tile + size.tile_count() > MAX_TILE_ID + 1 {
                        self.editor.status = "Paste exceeds tile limit".into();
                        return;
                    }
                    self.editor.capture_undo();
                    match write_graphic_pixels(
                        &mut self.editor.project,
                        self.editor.selected_tile,
                        size,
                        &pixels,
                    ) {
                        Ok(_) => {
                            self.editor.graphic_size = size;
                            self.editor.refresh_bytecode_from_selected_tile();
                            self.editor.status = format!("Pasted {}", size.label());
                        }
                        Err(err) => self.editor.status = err.to_string(),
                    }
                } else {
                    self.editor.status = "No graphic copied".into();
                }
            }
            ButtonAction::Menu(group) => {
                self.open_group = if self.open_group == Some(group) {
                    None
                } else {
                    Some(group)
                };
                self.editor.status = format!("{} menu", group.label());
            }
            ButtonAction::Tool(tool) => {
                self.open_group = None;
                self.editor.selected_tool = tool;
                self.editor.status = format!("Tool {}", tool.label());
            }
            ButtonAction::Undo => {
                self.open_group = None;
                self.editor.undo();
            }
            ButtonAction::Redo => {
                self.open_group = None;
                self.editor.redo();
            }
            ButtonAction::AddTile => {
                self.open_group = None;
                if self.editor.project.tiles.len() >= MAX_TILE_ID + 1 {
                    self.editor.status = "Tile limit reached (256)".into();
                    return;
                }
                self.editor.capture_undo();
                self.editor.selected_tile = self.editor.project.add_tile(Tile::blank()) as usize;
                self.editor.refresh_bytecode_from_selected_tile();
                self.editor.status = format!("Added tile {}", self.editor.selected_tile);
            }
            ButtonAction::FillTile => {
                self.open_group = None;
                self.editor.capture_undo();
                let size = self.editor.graphic_size;
                let (width, height) = size.dimensions();
                let pixels = vec![self.editor.selected_color; width * height];
                match write_graphic_pixels(
                    &mut self.editor.project,
                    self.editor.selected_tile,
                    size,
                    &pixels,
                ) {
                    Ok(_) => {
                        self.editor.refresh_bytecode_if_unfocused();
                        self.editor.status = format!(
                            "Filled {} graphic at tile {}",
                            size.label(),
                            self.editor.selected_tile
                        );
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::FlipX => {
                self.open_group = None;
                self.editor.capture_undo();
                let size = self.editor.graphic_size;
                let (width, height) = size.dimensions();
                let before =
                    read_graphic_pixels(&self.editor.project, self.editor.selected_tile, size);
                let after = flip_graphic_x(&before, width, height);
                match write_graphic_pixels(
                    &mut self.editor.project,
                    self.editor.selected_tile,
                    size,
                    &after,
                ) {
                    Ok(_) => {
                        self.editor.refresh_bytecode_if_unfocused();
                        self.editor.status = format!(
                            "Flipped {} graphic at tile {} X",
                            size.label(),
                            self.editor.selected_tile
                        );
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::FlipY => {
                self.open_group = None;
                self.editor.capture_undo();
                let size = self.editor.graphic_size;
                let (width, height) = size.dimensions();
                let before =
                    read_graphic_pixels(&self.editor.project, self.editor.selected_tile, size);
                let after = flip_graphic_y(&before, width, height);
                match write_graphic_pixels(
                    &mut self.editor.project,
                    self.editor.selected_tile,
                    size,
                    &after,
                ) {
                    Ok(_) => {
                        self.editor.refresh_bytecode_if_unfocused();
                        self.editor.status = format!(
                            "Flipped {} graphic at tile {} Y",
                            size.label(),
                            self.editor.selected_tile
                        );
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::Rotate => {
                self.open_group = None;
                self.editor.capture_undo();
                let size = self.editor.graphic_size;
                let rotated_size = size.rotated();
                let (width, height) = size.dimensions();
                let before =
                    read_graphic_pixels(&self.editor.project, self.editor.selected_tile, size);
                let after = rotate_graphic_right(&before, width, height);
                match write_graphic_pixels(
                    &mut self.editor.project,
                    self.editor.selected_tile,
                    rotated_size,
                    &after,
                ) {
                    Ok(_) => {
                        self.editor.graphic_size = rotated_size;
                        self.editor.refresh_bytecode_if_unfocused();
                        self.editor.status = format!(
                            "Rotated {} graphic to {} at tile {}",
                            size.label(),
                            rotated_size.label(),
                            self.editor.selected_tile
                        );
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::Dedupe => {
                self.open_group = None;
                self.editor.capture_undo();
                let report = self.editor.project.dedupe_tiles();
                self.editor.clamp_selection();
                self.editor.refresh_bytecode_if_unfocused();
                self.editor.status = format!(
                    "Dedupe {} -> {} removed {}",
                    report.before, report.after, report.removed
                );
            }
            ButtonAction::AutoCollision => {
                self.open_group = None;
                self.editor.capture_undo();
                let count = self.editor.project.auto_tag_collision_by_density(0.35);
                self.editor.status = format!("Auto collision {} cells", count);
            }
            ButtonAction::AutoTile => {
                self.open_group = None;
                self.editor.capture_undo();
                let table: Vec<u16> = (0..16)
                    .map(|index| {
                        (index + 16).min(self.editor.project.tiles.len().saturating_sub(1)) as u16
                    })
                    .collect();
                match self.editor.project.apply_neighbor_autotile(&table) {
                    Ok(count) => self.editor.status = format!("Autotile {} cells", count),
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::NewMetaSprite => {
                self.open_group = None;
                self.editor.capture_undo();
                let index = self.editor.project.metasprites.len();
                self.editor.selected_metasprite = self
                    .editor
                    .project
                    .add_metasprite(format!("metasprite_{index}"));
                self.editor.selected_metasprite_frame = 0;
                self.editor.selected_tool = StudioTool::MetaSprite;
                self.editor.status = format!("Added metasprite {index}");
            }
            ButtonAction::FramePrev => {
                self.open_group = None;
                self.editor.selected_metasprite_frame =
                    self.editor.selected_metasprite_frame.saturating_sub(1);
                self.editor.selected_tool = StudioTool::MetaSprite;
                self.editor.status =
                    format!("Metasprite frame {}", self.editor.selected_metasprite_frame);
            }
            ButtonAction::FrameNext => {
                self.open_group = None;
                let frame_count = self
                    .editor
                    .project
                    .metasprites
                    .get(self.editor.selected_metasprite)
                    .map(|metasprite| metasprite.frames.len())
                    .unwrap_or(0);
                self.editor.selected_metasprite_frame =
                    (self.editor.selected_metasprite_frame + 1).min(frame_count.saturating_sub(1));
                self.editor.selected_tool = StudioTool::MetaSprite;
                self.editor.status =
                    format!("Metasprite frame {}", self.editor.selected_metasprite_frame);
            }
            ButtonAction::FrameAdd => {
                self.open_group = None;
                self.editor.capture_undo();
                if self.editor.project.metasprites.is_empty() {
                    self.editor.selected_metasprite =
                        self.editor.project.add_metasprite("metasprite_0");
                    self.editor.selected_metasprite_frame = 0;
                } else {
                    let frame_index = self.editor.project.metasprites
                        [self.editor.selected_metasprite]
                        .frames
                        .len();
                    match self.editor.project.add_metasprite_frame(
                        self.editor.selected_metasprite,
                        format!("frame_{frame_index:02}"),
                    ) {
                        Ok(index) => self.editor.selected_metasprite_frame = index,
                        Err(err) => self.editor.status = err.to_string(),
                    }
                }
                self.editor.selected_tool = StudioTool::MetaSprite;
                self.editor.status =
                    format!("Added frame {}", self.editor.selected_metasprite_frame);
            }
            ButtonAction::FrameDuplicate => {
                self.open_group = None;
                self.editor.capture_undo();
                let current = self
                    .editor
                    .project
                    .metasprite_frame(
                        self.editor.selected_metasprite,
                        self.editor.selected_metasprite_frame,
                    )
                    .cloned();
                match current {
                    Ok(mut frame) => {
                        let frames = &mut self.editor.project.metasprites
                            [self.editor.selected_metasprite]
                            .frames;
                        frame.name = format!("frame_{:02}", frames.len());
                        frames.push(frame);
                        self.editor.selected_metasprite_frame = frames.len() - 1;
                        self.editor.selected_tool = StudioTool::MetaSprite;
                        self.editor.status =
                            format!("Duplicated frame {}", self.editor.selected_metasprite_frame);
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::FrameClear => {
                self.open_group = None;
                self.editor.capture_undo();
                match self.editor.project.metasprite_frame_mut(
                    self.editor.selected_metasprite,
                    self.editor.selected_metasprite_frame,
                ) {
                    Ok(frame) => {
                        frame.parts.clear();
                        self.editor.selected_tool = StudioTool::MetaSprite;
                        self.editor.status = "Cleared metasprite frame".to_string();
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::LoadProject => {
                self.open_group = None;
                self.open_project_dialog();
            }
            ButtonAction::SaveProject => {
                self.open_group = None;
                self.save_project_dialog();
            }
            ButtonAction::ImportPng => {
                self.open_group = None;
                if !self.confirm_replace() {
                    return;
                }
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Import PNG as a new project")
                    .add_filter("PNG image", &["png"])
                    .pick_file()
                {
                    if let Err(error) = self.editor.import_png_path(&path) {
                        self.editor.status = error.to_string();
                    }
                } else {
                    self.editor.status = "Import cancelled".into();
                }
            }
            ButtonAction::ExportPng
            | ButtonAction::ExportC
            | ButtonAction::ExportGbr
            | ButtonAction::ExportGbm => self.export_dialog(action),
            ButtonAction::SetColorMode(mode) => {
                self.open_group = None;
                self.editor.color_mode = mode;
                self.editor.status = format!("Mode {}", mode.label());
            }
            ButtonAction::SetGraphicSize(size) => {
                self.open_group = None;
                self.editor.graphic_size = size;
                self.editor.status = format!(
                    "Graphic size {} ({} tiles)",
                    size.label(),
                    size.tile_count()
                );
            }
            ButtonAction::BytecodeImport => {
                self.open_group = None;
                self.editor.bytecode.input.focused = false;
                match parse_tile_bytecode(&self.editor.bytecode.input.text) {
                    Ok(bytes) => {
                        self.editor.capture_undo();
                        match self
                            .editor
                            .project
                            .set_tile_from_2bpp_bytes(self.editor.selected_tile, &bytes)
                        {
                            Ok(()) => {
                                self.editor.refresh_bytecode_from_selected_tile();
                                self.editor.status =
                                    format!("Imported bytecode tile {}", self.editor.selected_tile);
                            }
                            Err(err) => self.editor.status = err.to_string(),
                        }
                    }
                    Err(err) => self.editor.status = err.to_string(),
                }
            }
            ButtonAction::BytecodeExport => {
                self.open_group = None;
                self.editor.bytecode.input.focused = false;
                self.editor.refresh_bytecode_from_selected_tile();
                self.editor.status =
                    format!("Exported bytecode tile {}", self.editor.selected_tile);
            }
        }
    }

    fn open_project_dialog(&mut self) {
        if !self.confirm_replace() {
            return;
        }
        let mut dialog = rfd::FileDialog::new()
            .set_title("Open GBHUA project")
            .add_filter(
                "KITAQGB assets",
                &["gbh", "json", "gbr", "gbtd", "gbtb", "gbm", "gbmb"],
            )
            .add_filter("PNG image", &["png"])
            .add_filter("All files", &["*"]);
        if let Some(parent) = self
            .editor
            .current_path
            .as_ref()
            .and_then(|path| path.parent())
        {
            dialog = dialog.set_directory(parent);
        }

        if let Some(path) = dialog.pick_file() {
            if let Err(err) = self.editor.load_asset_path(&path) {
                self.editor.status = err.to_string();
            }
        } else {
            self.editor.status = "Open cancelled".to_string();
        }
    }

    fn export_dialog(&mut self, action: ButtonAction) {
        self.open_group = None;
        let (extension, default_name) = match action {
            ButtonAction::ExportC => ("c", AUTOEXPORT_C),
            ButtonAction::ExportGbr => ("gbr", AUTOEXPORT_GBR),
            ButtonAction::ExportGbm => ("gbm", AUTOEXPORT_GBM),
            _ => ("png", "preview.png"),
        };
        #[cfg(not(test))]
        let path = rfd::FileDialog::new()
            .set_title("Export KITAQGB asset")
            .add_filter(extension, &[extension])
            .set_file_name(default_name)
            .save_file();
        #[cfg(test)]
        let path = Some(
            self.save_dialog_path
                .take()
                .unwrap_or_else(|| test_export_path(default_name)),
        );
        let Some(mut path) = path else {
            self.editor.status = "Export cancelled".into();
            return;
        };
        if path.extension().is_none() {
            path.set_extension(extension);
        }
        #[cfg(not(test))]
        if action == ButtonAction::ExportGbm && path.with_extension("gbr").exists() {
            let answer = rfd::MessageDialog::new()
                .set_title("Replace companion tiles?")
                .set_description(format!(
                    "Also replace {}?",
                    path.with_extension("gbr").display()
                ))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if answer != rfd::MessageDialogResult::Yes {
                self.editor.status = "Export cancelled".into();
                return;
            }
        }
        let result = match action {
            ButtonAction::ExportC => {
                self.editor
                    .project
                    .export_kitaqgb_c("kq_asset")
                    .and_then(|text| {
                        fs::write(&path, text)?;
                        Ok(())
                    })
            }
            ButtonAction::ExportGbr => self.editor.project.save_gbtd_file(&path),
            ButtonAction::ExportGbm => self.editor.project.save_gbmb_file(&path),
            _ => image_io::preview_png(&self.editor.project, &path, self.editor.image_mode(), 1),
        };
        self.editor.status = match result {
            Ok(()) => format!("Exported {}", path.display()),
            Err(error) => error.to_string(),
        };
    }

    fn save_project_dialog(&mut self) {
        #[cfg(not(test))]
        let Some(path) = pick_project_save_path(
            self.editor.current_path.as_deref(),
            &self.editor.project.name,
        ) else {
            self.editor.status = "Save cancelled".to_string();
            return;
        };
        #[cfg(test)]
        let Some(path) = self
            .save_dialog_path
            .take()
            .map(ensure_project_save_extension)
        else {
            self.editor.status = "Save cancelled".to_string();
            return;
        };
        self.editor.save_project_to_path(path);
    }

    fn confirm_replace(&mut self) -> bool {
        if !self.editor.is_dirty() {
            return true;
        }
        #[cfg(test)]
        {
            self.discard_confirmed
        }
        #[cfg(not(test))]
        {
            match rfd::MessageDialog::new()
                .set_title("GBHUA - Unsaved changes")
                .set_description("Save changes before continuing?\nYes: save   No: discard   Cancel: keep editing")
                .set_buttons(rfd::MessageButtons::YesNoCancel)
                .show()
            {
                rfd::MessageDialogResult::Yes => {
                    self.save_project_dialog();
                    !self.editor.is_dirty()
                }
                rfd::MessageDialogResult::No => true,
                _ => false,
            }
        }
    }

    fn handle_key(&mut self, key: &str, ctrl: bool) -> UiResponse {
        if self.editor.bytecode.input.focused {
            return self.handle_bytecode_key(key, ctrl);
        }
        let key = normalize_key(key);
        if ctrl && key == "O" {
            self.apply_action(ButtonAction::LoadProject);
            return UiResponse::redraw();
        }
        if ctrl && key == "S" {
            self.apply_action(ButtonAction::SaveProject);
            return UiResponse::redraw();
        }
        if ctrl && key == "E" {
            self.apply_action(ButtonAction::ExportC);
            return UiResponse::redraw();
        }
        if ctrl && key == "G" {
            self.apply_action(ButtonAction::ExportGbr);
            return UiResponse::redraw();
        }
        if ctrl && key == "B" {
            self.apply_action(ButtonAction::ExportGbm);
            return UiResponse::redraw();
        }
        if ctrl && key == "Z" {
            self.apply_action(ButtonAction::Undo);
            return UiResponse::redraw();
        }
        if ctrl && key == "Y" {
            self.apply_action(ButtonAction::Redo);
            return UiResponse::redraw();
        }
        if ctrl && key == "C" {
            self.apply_action(ButtonAction::CopyGraphic);
            return UiResponse::redraw();
        }
        if ctrl && key == "V" {
            self.apply_action(ButtonAction::PasteGraphic);
            return UiResponse::redraw();
        }
        match key.as_str() {
            "1" => self.editor.selected_color = 0,
            "2" => self.editor.selected_color = 1,
            "3" => self.editor.selected_color = 2,
            "4" => self.editor.selected_color = 3,
            "P" => self.editor.selected_tool = StudioTool::Pixel,
            "V" => self.editor.selected_tool = StudioTool::MetaSprite,
            "M" => self.editor.selected_tool = StudioTool::Map,
            "F" => self.editor.selected_tool = StudioTool::Fill,
            "C" => self.editor.selected_tool = StudioTool::Collision,
            "E" => self.editor.selected_tool = StudioTool::Event,
            "A" => {
                self.editor.selected_palette = (self.editor.selected_palette + 1) & 0x07;
            }
            _ => return UiResponse::none(),
        }
        self.editor.status = format!(
            "Tool {} tile {} color {} pal {}",
            self.editor.selected_tool.label(),
            self.editor.selected_tile,
            self.editor.selected_color,
            self.editor.selected_palette
        );
        UiResponse::redraw()
    }

    fn handle_bytecode_key(&mut self, raw_key: &str, ctrl: bool) -> UiResponse {
        self.handle_bytecode_key_modifiers(raw_key, ctrl, false)
    }

    fn handle_bytecode_key_modifiers(
        &mut self,
        raw_key: &str,
        ctrl: bool,
        shift: bool,
    ) -> UiResponse {
        let key = normalize_key(raw_key);
        if ctrl && key == "V" {
            return self.paste_bytecode_clipboard();
        }
        if ctrl && key == "A" {
            self.editor.bytecode.select_all();
            return UiResponse::redraw();
        }
        if ctrl && matches!(key.as_str(), "C" | "X") {
            if let Some(text) = self.editor.bytecode.selected_text() {
                match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
                    Ok(()) => {
                        if key == "X" {
                            self.editor.bytecode.delete_selection();
                        }
                    }
                    Err(err) => self.editor.status = format!("Copy failed: {err}"),
                }
            }
            return UiResponse::redraw();
        }
        if ctrl {
            return UiResponse::none();
        }
        match key.as_str() {
            "BACKSPACE" => {
                if !self.editor.bytecode.delete_selection() {
                    self.editor.bytecode.input.backspace();
                }
                UiResponse::redraw()
            }
            "DELETE" => {
                if !self.editor.bytecode.delete_selection() {
                    self.editor.bytecode.input.delete_forward();
                }
                UiResponse::redraw()
            }
            "ENTER" => self.insert_bytecode_text("\n"),
            "ARROWLEFT" | "ARROWRIGHT" | "HOME" | "END" => {
                self.editor.bytecode.move_cursor(&key, shift);
                UiResponse::redraw()
            }
            "ESCAPE" => {
                self.editor.bytecode.input.focused = false;
                UiResponse::redraw()
            }
            "SPACE" => self.insert_bytecode_text(" "),
            _ => {
                if let Some(text) = bytecode_text_from_key(raw_key) {
                    self.insert_bytecode_text(&text)
                } else {
                    UiResponse::none()
                }
            }
        }
    }

    fn handle_text_input(&mut self, text: &str) -> UiResponse {
        if !self.editor.bytecode.input.focused {
            return UiResponse::none();
        }
        self.insert_bytecode_text(text)
    }

    fn insert_bytecode_text(&mut self, text: &str) -> UiResponse {
        let filtered: String = text
            .chars()
            .filter(|ch| is_bytecode_input_char(*ch))
            .collect();
        if !filtered.is_empty() {
            self.editor.bytecode.insert(&filtered);
            UiResponse::redraw()
        } else {
            UiResponse::none()
        }
    }

    fn paste_bytecode_clipboard(&mut self) -> UiResponse {
        match read_clipboard_text() {
            Ok(text) => {
                let _ = self.insert_bytecode_text(&text);
                self.editor.status = "Pasted bytecode".to_string();
                UiResponse::redraw()
            }
            Err(err) => {
                self.editor.status = format!("Paste failed: {err}");
                UiResponse::redraw()
            }
        }
    }

    fn draw_toolbar(&self, list: &mut DrawList, theme: StudioTheme) {
        let rect = self.top_bar_rect();
        list.fill_rect(rect, theme.top);
        list.fill_rect(
            Rect::new(0.0, rect.bottom() - 1.0, rect.width, 1.0),
            theme.border,
        );
        draw_text(list, 16.0, 14.0, SCREEN_TITLE, 1.6, theme.text);
        for button in self.toolbar_buttons() {
            self.draw_control(list, button, true);
        }
    }

    fn draw_dropdown_menu(&self, list: &mut DrawList, theme: StudioTheme) {
        let buttons = self.dropdown_buttons();
        if buttons.is_empty() {
            return;
        }
        let mut bounds = buttons[0].rect;
        for button in &buttons[1..] {
            bounds = Rect::new(
                bounds.x.min(button.rect.x),
                bounds.y.min(button.rect.y),
                bounds.right().max(button.rect.right()) - bounds.x.min(button.rect.x),
                bounds.bottom().max(button.rect.bottom()) - bounds.y.min(button.rect.y),
            );
        }
        list.fill_rect(bounds.inset(-4.0), Color::rgba8(18, 22, 27, 242));
        list.stroke_rect(bounds.inset(-4.0), 1.0, theme.border);
        for button in buttons {
            let active = matches!(button.action, ButtonAction::Tool(tool) if tool == self.editor.selected_tool)
                || matches!(button.action, ButtonAction::SetColorMode(mode) if mode == self.editor.color_mode)
                || matches!(button.action, ButtonAction::SetGraphicSize(size) if size == self.editor.graphic_size)
                || matches!(button.action, ButtonAction::MapZoom(zoom) if zoom == self.map_zoom)
                || button.action == ButtonAction::ToggleGrid && self.grid_visible
                || button.action == ButtonAction::ToggleOverlays && self.overlays_visible;
            let state = self.buttons.visual_state(&button.action, active);
            draw_button(list, button.rect, button.label, state, theme);
        }
    }

    fn draw_atlas(&self, list: &mut DrawList, theme: StudioTheme) {
        if !self.show_atlas() {
            return;
        }
        let panel = self.atlas_panel_rect();
        let title = if self.compact() { "" } else { "Tiles" };
        self.draw_panel_header(list, panel, title, "");
        draw_text(
            list,
            panel.x + 12.0,
            panel.y + 55.0,
            &format!(
                "Tile {:03} / {}",
                self.editor.selected_tile,
                self.editor.project.tiles.len()
            ),
            1.1,
            theme.muted,
        );
        let viewport = self.atlas_viewport_rect();
        let content_size = self.atlas_content_size();
        list.fill_rect(viewport, theme.panel);
        self.atlas_scroll.push_clip(list, viewport, content_size);
        let atlas = self.atlas_image_rect();
        let rows = ((self.editor.project.tiles.len() + 7) / 8).max(1);
        let palette = self.active_palette();
        let pixels = render_atlas_with_palette(&self.editor.project, 8, &palette);
        list.fill_rect(atlas.inset(-4.0), theme.panel_alt);
        list.image_rgba8(
            atlas,
            (8 * TILE_SIZE) as u32,
            (rows * TILE_SIZE) as u32,
            pixels,
        );
        let graphic_end = self
            .editor
            .selected_tile
            .saturating_add(self.editor.graphic_size.tile_count());
        for index in 0..self.editor.project.tiles.len() {
            let x = atlas.x + (index % 8) as f32 * 24.0;
            let y = atlas.y + (index / 8) as f32 * 24.0;
            let rect = Rect::new(x, y, 24.0, 24.0);
            if index == self.editor.selected_tile {
                list.stroke_rect(rect.inset(-2.0), 2.0, theme.accent2);
            } else if index < graphic_end && index > self.editor.selected_tile {
                list.stroke_rect(rect.inset(-1.0), 2.0, theme.accent);
            } else {
                list.stroke_rect(rect, 1.0, Color::rgba8(0, 0, 0, 90));
            }
        }
        list.pop_clip();
        self.atlas_scroll
            .draw_scrollbars(list, viewport, content_size, self.scrollbar_style());
    }

    fn draw_tile_editor(&self, list: &mut DrawList, theme: StudioTheme) {
        let panel = self.graphic_panel_rect();
        let title = if self.editor.selected_tool == StudioTool::MetaSprite {
            "Metasprite"
        } else {
            "Graphic"
        };
        self.draw_panel_header(
            list,
            panel,
            title,
            &format!("Tile {:03}", self.editor.selected_tile),
        );
        let tile_area = self.tile_editor_area_rect();
        let tile_rect = self.tile_editor_rect();
        list.fill_rect(tile_area, theme.bg);
        if self.editor.selected_tool == StudioTool::MetaSprite {
            for y in 0..10 {
                for x in 0..10 {
                    let cell = tile_rect.width / 10.0;
                    let rect = Rect::new(
                        tile_rect.x + x as f32 * cell,
                        tile_rect.y + y as f32 * cell,
                        cell,
                        cell,
                    );
                    list.fill_rect(
                        rect,
                        if (x + y) & 1 == 0 {
                            Color::rgb8(36, 42, 47)
                        } else {
                            Color::rgb8(29, 34, 39)
                        },
                    );
                    list.stroke_rect(rect, 1.0, Color::rgba8(0, 0, 0, 55));
                }
            }
            if let Ok(pixels) = self.editor.project.render_metasprite_rgba(
                self.editor.selected_metasprite,
                self.editor.selected_metasprite_frame,
                80,
                80,
                40,
                40,
            ) {
                list.image_rgba8(tile_rect, 80, 80, pixels);
            }
            let axis = tile_rect.width * 0.5;
            list.fill_rect(
                Rect::new(tile_rect.x + axis, tile_rect.y, 1.0, tile_rect.height),
                theme.accent2,
            );
            list.fill_rect(
                Rect::new(tile_rect.x, tile_rect.y + axis, tile_rect.width, 1.0),
                theme.accent2,
            );
            if let Some(metasprite) = self
                .editor
                .project
                .metasprites
                .get(self.editor.selected_metasprite)
            {
                let frame_count = metasprite.frames.len();
                let part_count = metasprite
                    .frames
                    .get(self.editor.selected_metasprite_frame)
                    .map(|frame| frame.parts.len())
                    .unwrap_or(0);
                draw_text(
                    list,
                    tile_rect.x + 8.0,
                    tile_rect.y + 12.0,
                    &format!(
                        "{}  FRAME {}/{}  PARTS {}",
                        metasprite.name,
                        self.editor.selected_metasprite_frame + 1,
                        frame_count,
                        part_count
                    ),
                    0.8,
                    theme.text,
                );
            }
        } else {
            let size = self.editor.graphic_size;
            let (width, height) = size.dimensions();
            let cell_width = tile_rect.width / width as f32;
            let cell_height = tile_rect.height / height as f32;
            let palette = self.active_palette();
            let pixels = read_graphic_pixels(&self.editor.project, self.editor.selected_tile, size);
            for y in 0..height {
                for x in 0..width {
                    let value = pixels[y * width + x] as usize;
                    let color = palette[value].to_color();
                    let rect = Rect::new(
                        tile_rect.x + x as f32 * cell_width,
                        tile_rect.y + y as f32 * cell_height,
                        cell_width,
                        cell_height,
                    );
                    list.fill_rect(rect, color);
                    if self.grid_visible {
                        list.stroke_rect(rect, 1.0, Color::rgba8(0, 0, 0, 80));
                    }
                }
            }
            let (tile_columns, tile_rows) = size.tile_dimensions();
            for tile_x in 1..tile_columns {
                let x = tile_rect.x + tile_x as f32 * TILE_SIZE as f32 * cell_width;
                list.fill_rect(
                    Rect::new(x - 1.0, tile_rect.y, 2.0, tile_rect.height),
                    theme.accent,
                );
            }
            for tile_y in 1..tile_rows {
                let y = tile_rect.y + tile_y as f32 * TILE_SIZE as f32 * cell_height;
                list.fill_rect(
                    Rect::new(tile_rect.x, y - 1.0, tile_rect.width, 2.0),
                    theme.accent,
                );
            }
        }
        list.stroke_rect(tile_rect, 2.0, theme.accent);
        let palette = self.palette_rect();
        let swatch_w = palette.width / 4.0;
        for index in 0..4 {
            let rect = Rect::new(
                palette.x + index as f32 * swatch_w,
                palette.y,
                swatch_w - 6.0,
                palette.height,
            );
            let palette_colors = self.active_palette();
            let color = palette_colors[index].to_color();
            list.fill_rect(rect, color);
            let stroke = if index as u8 == self.editor.selected_color {
                theme.accent2
            } else {
                theme.border
            };
            list.stroke_rect(rect, 2.0, stroke);
            draw_text(
                list,
                rect.x + 10.0,
                rect.y + 13.0,
                &index.to_string(),
                0.9,
                if color.r * 0.299 + color.g * 0.587 + color.b * 0.114 > 0.5 {
                    Color::rgb8(8, 10, 12)
                } else {
                    Color::rgb8(245, 245, 245)
                },
            );
        }
    }

    fn draw_bytecode_window(&self, list: &mut DrawList, theme: StudioTheme) {
        let window = self.bytecode_window_rect();
        list.fill_rect(
            Rect::new(window.x, window.y - 2.0, window.width, 1.0),
            theme.border,
        );
        draw_text(
            list,
            window.x,
            window.y + 10.0,
            &format!("Bytecode  /  tile {:03}", self.editor.selected_tile),
            1.1,
            theme.text,
        );

        let input = self.bytecode_input_rect();
        list.fill_rect(input, Color::rgb8(13, 16, 19));
        list.stroke_rect(
            input,
            if self.editor.bytecode.input.focused {
                2.0
            } else {
                1.0
            },
            if self.editor.bytecode.input.focused {
                theme.accent2
            } else {
                theme.border
            },
        );
        self.editor
            .bytecode
            .draw(list, input, theme.text, theme.accent2);

        for button in self.bytecode_buttons() {
            let state = self.buttons.visual_state(&button.action, false);
            draw_button(list, button.rect, button.label, state, theme);
        }
    }

    fn draw_cgb_palette_grid(&self, list: &mut DrawList, theme: StudioTheme) {
        let grid = self.cgb_palette_grid_rect();
        let label_w = 32.0;
        let swatch_w = (grid.width - label_w).max(4.0) / 4.0;

        list.fill_rect(grid, theme.panel);

        if self.editor.color_mode == ColorMode::Dmg {
            draw_text(list, grid.x + 4.0, grid.y + 12.0, "GB", 0.72, theme.muted);
            for color_index in 0..4 {
                let rect = Rect::new(
                    grid.x + label_w + color_index as f32 * swatch_w,
                    grid.y + 4.0,
                    swatch_w - 3.0,
                    28.0,
                );
                list.fill_rect(rect, DMG_PALETTE[color_index].to_color());
                let stroke = if color_index as u8 == self.editor.selected_color {
                    theme.accent2
                } else {
                    Color::rgba8(0, 0, 0, 120)
                };
                list.stroke_rect(rect, 1.5, stroke);
            }
            return;
        }

        let row_h = grid.height / 8.0;
        for palette_index in 0..8 {
            let y = grid.y + palette_index as f32 * row_h;
            let row = Rect::new(grid.x, y, grid.width, row_h);
            if palette_index as u8 == self.editor.selected_palette {
                list.fill_rect(row, Color::rgba8(86, 180, 170, 42));
                list.stroke_rect(row.inset(-1.0), 1.5, theme.accent2);
            }

            draw_text(
                list,
                row.x + 4.0,
                row.y + 7.0,
                &format!("P{}", palette_index),
                1.05,
                theme.muted,
            );

            for color_index in 0..4 {
                let rect = Rect::new(
                    grid.x + label_w + color_index as f32 * swatch_w,
                    y + 3.0,
                    swatch_w - 3.0,
                    row_h - 6.0,
                );
                let color = self.editor.project.palette(palette_index)[color_index].to_color();
                list.fill_rect(rect, color);
                let stroke = if palette_index as u8 == self.editor.selected_palette
                    && color_index as u8 == self.editor.selected_color
                {
                    theme.accent2
                } else {
                    Color::rgba8(0, 0, 0, 120)
                };
                list.stroke_rect(rect, 1.5, stroke);
            }
        }
    }

    fn draw_map_editor(&self, list: &mut DrawList, theme: StudioTheme) {
        let panel = self.map_panel_rect();
        let zoom = if self.map_zoom == 0 {
            "Fit".into()
        } else {
            format!("{}%", self.map_zoom as u32 * 100)
        };
        self.draw_panel_header(
            list,
            panel,
            "Map",
            &format!(
                "{} x {}  /  {}",
                self.editor.project.map_width, self.editor.project.map_height, zoom
            ),
        );
        let view = self.map_view_rect();
        let viewport = self.map_viewport_rect();
        list.push_clip(viewport);
        let width = self.editor.project.map_width as usize * TILE_SIZE;
        let height = self.editor.project.map_height as usize * TILE_SIZE;
        list.fill_rect(view.inset(-8.0), Color::rgb8(13, 16, 19));
        list.image_rgba8(
            view,
            width as u32,
            height as u32,
            render_map_with_overlays(
                &self.editor.project,
                self.editor.color_mode,
                self.overlays_visible,
            ),
        );
        let cell_w = view.width / self.editor.project.map_width as f32;
        let cell_h = view.height / self.editor.project.map_height as f32;
        if self.grid_visible && cell_w >= 8.0 && cell_h >= 8.0 {
            for x in 0..=self.editor.project.map_width as usize {
                let px = view.x + x as f32 * cell_w;
                list.fill_rect(
                    Rect::new(px, view.y, 1.0, view.height),
                    Color::rgba8(0, 0, 0, 80),
                );
            }
            for y in 0..=self.editor.project.map_height as usize {
                let py = view.y + y as f32 * cell_h;
                list.fill_rect(
                    Rect::new(view.x, py, view.width, 1.0),
                    Color::rgba8(0, 0, 0, 80),
                );
            }
        }
        if let Some((x, y)) = self.editor.map_cursor {
            let rect = Rect::new(
                view.x + x as f32 * cell_w,
                view.y + y as f32 * cell_h,
                cell_w,
                cell_h,
            );
            list.stroke_rect(rect.inset(-1.0), 2.0, theme.accent2);
        }
        list.stroke_rect(view, 2.0, theme.accent);
        list.pop_clip();
        self.map_scroll.draw_scrollbars(
            list,
            viewport,
            self.map_content_size(),
            self.scrollbar_style(),
        );
    }

    fn draw_inspector(&self, list: &mut DrawList, theme: StudioTheme) {
        if !self.show_inspector() {
            return;
        }
        let panel = self.inspector_rect();
        self.draw_panel_header(
            list,
            panel,
            if self.compact() { "" } else { "Palettes" },
            "",
        );
        draw_text(
            list,
            panel.x + 12.0,
            panel.y + 55.0,
            if self.editor.color_mode == ColorMode::Dmg {
                "DMG / 4 shades"
            } else {
                "CGB / 8 palettes"
            },
            1.1,
            theme.muted,
        );
        let viewport = self.inspector_viewport_rect();
        let content_size = self.inspector_content_size();
        let origin = self
            .inspector_scroll
            .push_clip(list, viewport, content_size);
        self.draw_cgb_palette_grid(list, theme);
        let budget = self.editor.project.budget_report();
        let x = origin.x;
        let mut y = origin.y + 224.0;
        list.fill_rect(
            Rect::new(x, y - 12.0, viewport.width - 12.0, 1.0),
            theme.border,
        );
        draw_text(list, x, y, "Selection", 1.25, theme.text);
        y += 28.0;
        for (key, value) in [
            ("Tool", self.editor.selected_tool.label().to_string()),
            (
                "Palette / Color",
                format!(
                    "P{} / {}",
                    self.editor.selected_palette, self.editor.selected_color
                ),
            ),
            (
                "Collision / Event",
                format!(
                    "{} / {}",
                    self.editor.selected_collision, self.editor.selected_event
                ),
            ),
        ] {
            draw_text(list, x, y, key, 1.05, theme.muted);
            let width = measure_text_width(&value, 1.05);
            draw_text(
                list,
                x + viewport.width - width - 12.0,
                y,
                &value,
                1.05,
                theme.text,
            );
            y += 24.0;
        }
        y += 18.0;
        list.fill_rect(
            Rect::new(x, y - 12.0, viewport.width - 12.0, 1.0),
            theme.border,
        );
        draw_text(list, x, y, "Memory", 1.25, theme.text);
        y += 26.0;
        let value = format!("{} / 256 tiles", budget.tile_count);
        draw_text(list, x, y, &value, 1.1, theme.text);
        y += 22.0;
        list.fill_rect(Rect::new(x, y, viewport.width - 12.0, 4.0), theme.border);
        list.fill_rect(
            Rect::new(
                x,
                y,
                (viewport.width - 12.0) * (budget.tile_count as f32 / 256.0).min(1.0),
                4.0,
            ),
            if budget.warnings.is_empty() {
                theme.accent
            } else {
                theme.danger
            },
        );
        y += 18.0;
        for (key, value) in [
            ("Unique tiles", budget.unique_tiles.to_string()),
            (
                "2bpp / Map",
                format!("{} / {} B", budget.tile_bytes, budget.map_bytes),
            ),
            ("Attributes", format!("{} B", budget.attr_bytes)),
            (
                "Collision / Events",
                format!("{} / {} B", budget.collision_bytes, budget.event_bytes),
            ),
            ("RLE map", format!("{} B", budget.rle_map_bytes)),
            ("Highest tile ID", budget.max_tile_id.to_string()),
            (
                "Metasprites",
                self.editor.project.metasprites.len().to_string(),
            ),
        ] {
            draw_text(list, x, y, key, 1.0, theme.muted);
            let width = measure_text_width(&value, 1.0);
            draw_text(
                list,
                x + viewport.width - width - 12.0,
                y,
                &value,
                1.0,
                theme.text,
            );
            y += 22.0;
        }
        draw_text(
            list,
            x,
            y + 6.0,
            &if budget.warnings.is_empty() {
                "Budget OK".into()
            } else {
                format!("{} warnings", budget.warnings.len())
            },
            1.1,
            if budget.warnings.is_empty() {
                theme.accent
            } else {
                theme.danger
            },
        );
        list.pop_clip();
        self.inspector_scroll
            .draw_scrollbars(list, viewport, content_size, self.scrollbar_style());
    }
}

impl PlitaProgram for AssetStudioApp {
    fn close_requested(&mut self) -> bool {
        self.confirm_replace()
    }
    fn resize(&mut self, size: Size) -> UiResponse {
        self.size = size.clamped();
        UiResponse::redraw()
    }

    fn handle_event(&mut self, event: UiEvent) -> UiResponse {
        match event {
            UiEvent::PointerDown { position, button } => self.handle_pointer_down(position, button),
            UiEvent::PointerMove { position } => self.handle_pointer_move(position),
            UiEvent::PointerUp { position, button } => self.handle_pointer_up(position, button),
            UiEvent::PointerWheel {
                position,
                delta_x,
                delta_y,
            } => self.handle_pointer_wheel(position, delta_x, delta_y),
            UiEvent::KeyDown {
                key,
                ctrl,
                shift,
                meta,
                ..
            } => {
                if normalize_key(&key) == "ESCAPE" {
                    self.open_group = None;
                    self.editor.bytecode.input.focused = false;
                    self.buttons = ButtonInteractionState::new();
                    return UiResponse::redraw();
                }
                if self.editor.bytecode.input.focused {
                    self.handle_bytecode_key_modifiers(&key, ctrl || meta, shift)
                } else {
                    self.handle_key(&key, ctrl || meta)
                }
            }
            UiEvent::TextInput { text } | UiEvent::ImeCommit { text } => {
                self.handle_text_input(&text)
            }
            UiEvent::FileDropped { path } => {
                if !self.confirm_replace() {
                    return UiResponse::redraw();
                }
                match self.editor.load_asset_path(&path) {
                    Ok(()) => UiResponse::redraw(),
                    Err(err) => {
                        self.editor.status = err.to_string();
                        UiResponse::redraw()
                    }
                }
            }
            UiEvent::FocusChanged { focused: false } => {
                self.open_group = None;
                self.pointer = None;
                self.pointer_down = false;
                self.last_pixel = None;
                self.drag_target = DragTarget::None;
                self.end_scrollbar_drag();
                self.buttons = ButtonInteractionState::new();
                UiResponse::redraw()
            }
            _ => UiResponse::none(),
        }
    }

    fn draw_list(&self) -> DrawList {
        let theme = self.theme();
        let mut list = DrawList::new(theme.bg);
        list.fill_rect(
            Rect::new(0.0, 0.0, self.size.width, self.size.height),
            theme.bg,
        );
        self.draw_toolbar(&mut list, theme);
        self.draw_atlas(&mut list, theme);
        self.draw_tile_editor(&mut list, theme);
        self.draw_bytecode_window(&mut list, theme);
        self.draw_map_editor(&mut list, theme);
        self.draw_inspector(&mut list, theme);
        self.draw_context_controls(&mut list);
        self.draw_dropdown_menu(&mut list, theme);
        let status = self.status_rect();
        list.fill_rect(status, theme.top);
        list.stroke_rect(status, 1.0, theme.border);
        let filename = self
            .editor
            .current_path
            .as_deref()
            .map(path_label)
            .unwrap_or_else(|| "Untitled".into());
        let status_text = format!(
            "{}{} | {}",
            if self.editor.is_dirty() { "* " } else { "" },
            filename,
            self.editor.status
        );
        list.push_clip(status);
        draw_text(
            &mut list,
            status.x + 14.0,
            status.y + 8.0,
            &status_text,
            1.0,
            theme.text,
        );
        list.pop_clip();
        self.draw_tooltip(&mut list);
        workspace_ui::geometry_colors(list, workspace_ui::linear_color)
    }
}

#[derive(Clone, Copy)]
struct StudioTheme {
    bg: Color,
    top: Color,
    panel: Color,
    panel_alt: Color,
    border: Color,
    text: Color,
    muted: Color,
    accent: Color,
    accent2: Color,
    active: Color,
    button_hover: Color,
    button_pressed: Color,
    danger: Color,
}

fn draw_button(
    list: &mut DrawList,
    rect: Rect,
    label: &str,
    state: ButtonVisualState,
    theme: StudioTheme,
) {
    let fill = match state {
        ButtonVisualState::Pressed => theme.button_pressed,
        ButtonVisualState::Active => theme.active,
        ButtonVisualState::Hovered => theme.button_hover,
        ButtonVisualState::Normal => theme.panel_alt,
    };
    let draw_rect = if state == ButtonVisualState::Pressed {
        Rect::new(
            rect.x + 1.0,
            rect.y + 1.0,
            rect.width - 1.0,
            rect.height - 1.0,
        )
    } else {
        rect
    };
    let stroke = match state {
        ButtonVisualState::Normal => theme.border,
        _ => theme.accent2,
    };
    let text = if state == ButtonVisualState::Pressed {
        Color::rgb8(255, 255, 255)
    } else {
        theme.text
    };
    list.fill_rect(draw_rect, fill);
    list.stroke_rect(draw_rect, 1.0, stroke);
    draw_text(list, draw_rect.x + 8.0, draw_rect.y + 9.0, label, 1.1, text);
}

fn normalize_key(key: &str) -> String {
    let logical = key.split('|').next().unwrap_or(key);
    let mut value = logical.trim().to_ascii_uppercase();
    for prefix in ["CHARACTER(\"", "NAMED("] {
        if value.starts_with(prefix) {
            value = value
                .trim_start_matches(prefix)
                .trim_end_matches("\")")
                .trim_end_matches(')')
                .to_string();
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use plita::DrawCommand;

    #[test]
    fn brush_interpolates_picks_and_copies_without_damaging_other_tiles() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor.selected_tool = StudioTool::Pixel;
        app.editor.project.tile_mut(4).unwrap().fill(0);
        app.editor.selected_color = 3;
        let rect = app.tile_editor_rect();
        let a = Point::new(rect.x + rect.width / 16.0, rect.y + rect.height / 16.0);
        let b = Point::new(
            rect.right() - rect.width / 16.0,
            rect.bottom() - rect.height / 16.0,
        );
        app.handle_pointer_down(a, PointerButton::Primary);
        app.handle_pointer_move(b);
        app.handle_pointer_up(b, PointerButton::Primary);
        let pixels = read_graphic_pixels(&app.editor.project, 4, GraphicSize::Size8x8);
        for index in 0..8 {
            assert_eq!(pixels[index * 8 + index], 3);
        }
        let undo_count = app.editor.undo_stack.len();
        app.editor.selected_color = 1;
        app.handle_pointer_down(a, PointerButton::Secondary);
        assert_eq!(app.editor.selected_color, 3);
        assert_eq!(app.editor.undo_stack.len(), undo_count);
        app.apply_action(ButtonAction::CopyGraphic);
        app.editor.selected_tile = 20;
        app.apply_action(ButtonAction::PasteGraphic);
        assert_eq!(
            read_graphic_pixels(&app.editor.project, 20, GraphicSize::Size8x8),
            pixels
        );
        assert_eq!(app.editor.project.tiles.len(), 64);
    }

    #[test]
    fn tile_bucket_respects_connected_region_and_undo() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        let mut pixels = vec![0; 64];
        for y in 0..8 {
            pixels[y * 8 + 4] = 1;
        }
        write_graphic_pixels(&mut app.editor.project, 4, GraphicSize::Size8x8, &pixels).unwrap();
        app.editor.selected_tool = StudioTool::Fill;
        app.editor.selected_color = 2;
        let rect = app.tile_editor_rect();
        let p = Point::new(rect.x + 1.0, rect.y + 1.0);
        app.handle_pointer_down(p, PointerButton::Primary);
        app.handle_pointer_up(p, PointerButton::Primary);
        let filled = read_graphic_pixels(&app.editor.project, 4, GraphicSize::Size8x8);
        assert_eq!(&filled[..8], &[2, 2, 2, 2, 1, 0, 0, 0]);
        app.apply_action(ButtonAction::Undo);
        assert_eq!(
            read_graphic_pixels(&app.editor.project, 4, GraphicSize::Size8x8),
            pixels
        );
    }

    #[test]
    fn map_zoom_scroll_and_hit_testing_are_clipped() {
        let mut app = AssetStudioApp::new(Size::new(1024.0, 640.0));
        app.apply_action(ButtonAction::MapZoom(4));
        let viewport = app.map_viewport_rect();
        assert!(app.map_view_rect().width > viewport.width);
        assert_eq!(
            app.map_cell_at(Point::new(viewport.x + 1.0, viewport.y + 1.0)),
            Some((0, 0))
        );
        let before = app.map_view_rect();
        app.handle_pointer_wheel(center(viewport), 0.0, -120.0);
        let after = app.map_view_rect();
        assert_ne!(before.y, after.y);
        assert_eq!(
            app.map_cell_at(Point::new(viewport.x - 1.0, viewport.y)),
            None
        );
        app.apply_action(ButtonAction::MapZoom(0));
        assert!(app.map_view_rect().width <= viewport.width + 0.1);
        assert!(app.map_view_rect().height <= viewport.height + 0.1);
    }

    #[test]
    fn layout_is_aligned_and_bytecode_controls_do_not_overlap() {
        for (width, height) in [
            (1024.0, 600.0),
            (1280.0, 720.0),
            (1360.0, 820.0),
            (1920.0, 1080.0),
        ] {
            let app = AssetStudioApp::new(Size::new(width, height));
            assert_eq!(app.map_panel_rect().y, app.graphic_panel_rect().y);
            assert_eq!(app.map_viewport_rect().y, app.tile_editor_area_rect().y);
            assert!(
                app.map_panel_rect().right()
                    <= if app.compact() {
                        width
                    } else {
                        app.inspector_rect().x
                    }
            );
            assert!(app.bytecode_window_rect().bottom() < app.status_rect().y);
            assert!(app.bytecode_input_rect().bottom() < app.bytecode_buttons()[0].rect.y);
            assert!(app.toolbar_buttons().last().unwrap().rect.right() < width);
        }
    }

    #[test]
    fn cancelled_discard_preserves_project_on_drop_and_close() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.discard_confirmed = false;
        app.apply_action(ButtonAction::AddTile);
        let before = app.editor.project.clone();
        app.handle_event(UiEvent::FileDropped {
            path: PathBuf::from("does-not-exist.gbh"),
        });
        assert_eq!(app.editor.project, before);
        assert!(!app.close_requested());
        app.editor.saved_project = Some(before);
        assert!(app.close_requested());
    }

    #[test]
    fn untouched_startup_allows_load_save_and_close_without_discard_confirmation() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.discard_confirmed = false;
        assert!(!app.editor.is_dirty());
        assert!(app.confirm_replace());
        assert!(app.close_requested());

        app.apply_action(ButtonAction::SaveProject);
        assert_eq!(app.editor.status, "Save cancelled");
        assert!(!app.editor.is_dirty());
        let path = test_export_path("untouched.gbh");
        app.save_dialog_path = Some(path.clone());
        app.apply_action(ButtonAction::SaveProject);
        assert_eq!(
            Project::load_project_file(&path).unwrap(),
            app.editor.project
        );
        assert!(!app.editor.is_dirty());

        let mut fresh = AssetStudioApp::new(Size::new(1360.0, 820.0));
        fresh.discard_confirmed = false;
        fresh.handle_event(UiEvent::FileDropped { path: path.clone() });
        assert_eq!(fresh.editor.current_path, Some(path.clone()));
        assert!(fresh.editor.status.starts_with("Loaded "));
        assert!(!fresh.editor.is_dirty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn initial_document_edit_undo_redo_and_view_settings_track_real_changes() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.discard_confirmed = false;
        app.apply_action(ButtonAction::Tool(StudioTool::Pixel));
        app.apply_action(ButtonAction::SetColorMode(ColorMode::Dmg));
        app.apply_action(ButtonAction::MapZoom(2));
        app.apply_action(ButtonAction::SetGraphicSize(GraphicSize::Size16x16));
        assert!(!app.editor.is_dirty());
        assert!(app.confirm_replace());
        app.apply_action(ButtonAction::AddTile);
        assert!(app.editor.is_dirty());
        assert!(!app.confirm_replace());
        app.apply_action(ButtonAction::Undo);
        assert!(!app.editor.is_dirty());
        assert!(app.confirm_replace());
        app.apply_action(ButtonAction::Redo);
        assert!(app.editor.is_dirty());
        app.apply_action(ButtonAction::SaveProject);
        assert!(
            app.editor.is_dirty(),
            "cancelled save must preserve pending changes"
        );
        assert!(!app.confirm_replace());
    }

    #[test]
    fn editing_a_graphic_keeps_all_later_tiles_and_map_references() {
        let mut project = Project::default();
        project.map[0].tile = 63;
        let tail = project.tiles[20..].to_vec();
        write_graphic_pixels(&mut project, 4, GraphicSize::Size16x16, &[3; 256]).unwrap();
        assert_eq!(project.tiles.len(), 64);
        assert_eq!(project.tiles[20..], tail);
        assert_eq!(project.map[0].tile, 63);
        assert_eq!(project.render_map_art_rgba().len(), 256 * 256 * 4);
    }

    #[test]
    fn undo_restores_rectangular_dimensions_and_selection() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor.graphic_size = GraphicSize::Size8x16;
        app.editor.selected_tile = 4;
        let original = app.editor.project.clone();
        app.apply_action(ButtonAction::Rotate);
        assert_eq!(app.editor.graphic_size, GraphicSize::Size16x8);
        app.editor.selected_tile = 30;
        app.apply_action(ButtonAction::Undo);
        assert_eq!(app.editor.graphic_size, GraphicSize::Size8x16);
        assert_eq!(app.editor.selected_tile, 4);
        assert_eq!(app.editor.project, original);
        app.apply_action(ButtonAction::Redo);
        assert_eq!(app.editor.graphic_size, GraphicSize::Size16x8);
    }

    #[test]
    fn png_drop_is_undoable_and_export_uses_selected_path() {
        let dir = std::env::temp_dir().join(format!("gbhua-gui-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.png");
        image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]))
            .save(&input)
            .unwrap();
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        let before = app.editor.project.clone();
        app.handle_event(UiEvent::FileDropped {
            path: input.clone(),
        });
        assert_eq!(app.editor.project.map_width, 2);
        assert!(app.editor.is_dirty());
        let project_path = dir.join("named.gbh");
        app.save_dialog_path = Some(project_path);
        app.apply_action(ButtonAction::SaveProject);
        assert!(!app.editor.is_dirty());
        let png = dir.join("preview.png");
        app.save_dialog_path = Some(png.clone());
        app.apply_action(ButtonAction::ExportPng);
        assert_eq!(image::open(&png).unwrap().width(), 16);
        let c_path = dir.join("named.c");
        app.save_dialog_path = Some(c_path.clone());
        app.apply_action(ButtonAction::ExportC);
        assert!(fs::read_to_string(c_path)
            .unwrap()
            .contains("kq_asset_load"));
        app.editor.undo();
        assert_eq!(app.editor.project, before);
        assert!(app.editor.is_dirty());
        let _ = fs::remove_dir_all(dir);
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }

    fn click(app: &mut AssetStudioApp, position: Point) {
        app.handle_event(UiEvent::PointerDown {
            position,
            button: PointerButton::Primary,
        });
        app.handle_event(UiEvent::PointerUp {
            position,
            button: PointerButton::Primary,
        });
    }

    fn right_click(app: &mut AssetStudioApp, position: Point) {
        app.handle_event(UiEvent::PointerDown {
            position,
            button: PointerButton::Secondary,
        });
        app.handle_event(UiEvent::PointerUp {
            position,
            button: PointerButton::Secondary,
        });
    }

    fn action_button(app: &AssetStudioApp, action: ButtonAction) -> ActionButton {
        app.action_buttons()
            .into_iter()
            .chain(app.inspector_buttons())
            .chain(app.bytecode_buttons())
            .find(|button| button.action == action)
            .expect("button exists")
    }

    fn click_action(app: &mut AssetStudioApp, action: ButtonAction) {
        if !app
            .action_buttons()
            .iter()
            .any(|button| button.action == action)
        {
            if let Some(group) = action_group_for(action) {
                let button = action_button(app, ButtonAction::Menu(group));
                click(app, center(button.rect));
            }
        }
        let button = action_button(app, action);
        click(app, center(button.rect));
    }

    fn action_fill_color(app: &AssetStudioApp, action: ButtonAction) -> Color {
        let button = action_button(app, action);
        app.draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color }
                    if *rect == button.rect || *rect == button.rect.inset(1.0) =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .expect("button fill color")
    }

    fn tile_point(app: &AssetStudioApp, x: usize, y: usize) -> Point {
        let rect = app.tile_editor_rect();
        let (width, height) = app.editor.graphic_size.dimensions();
        let step_x = rect.width / width as f32;
        let step_y = rect.height / height as f32;
        Point::new(
            rect.x + x as f32 * step_x + step_x * 0.5,
            rect.y + y as f32 * step_y + step_y * 0.5,
        )
    }

    fn map_point(app: &AssetStudioApp, x: usize, y: usize) -> Point {
        let rect = app.map_view_rect();
        Point::new(
            rect.x + (x as f32 + 0.5) * rect.width / app.editor.project.map_width as f32,
            rect.y + (y as f32 + 0.5) * rect.height / app.editor.project.map_height as f32,
        )
    }

    fn palette_point(app: &AssetStudioApp, index: usize) -> Point {
        let rect = app.palette_rect();
        Point::new(
            rect.x + (index as f32 + 0.5) * rect.width / 4.0,
            rect.y + rect.height * 0.5,
        )
    }

    fn cgb_palette_point(app: &AssetStudioApp, palette: usize, color: usize) -> Point {
        let rect = app.cgb_palette_grid_rect();
        let row_h = rect.height / 8.0;
        let label_w = 32.0;
        let swatch_w = (rect.width - label_w).max(4.0) / 4.0;
        Point::new(
            rect.x + label_w + (color as f32 + 0.5) * swatch_w,
            rect.y + (palette as f32 + 0.5) * row_h,
        )
    }

    fn atlas_point(app: &AssetStudioApp, index: usize) -> Point {
        let rect = app.atlas_image_rect();
        Point::new(
            rect.x + (index % 8) as f32 * 24.0 + 12.0,
            rect.y + (index / 8) as f32 * 24.0 + 12.0,
        )
    }

    fn atlas_pixels(app: &AssetStudioApp) -> Vec<u8> {
        let rows = ((app.editor.project.tiles.len() + 7) / 8).max(1) as u32;
        app.draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::ImageRgba {
                    width,
                    height,
                    pixels,
                    ..
                } if *width == (8 * TILE_SIZE) as u32 && *height == rows * TILE_SIZE as u32 => {
                    Some(pixels.to_vec())
                }
                _ => None,
            })
            .expect("atlas image")
    }

    #[test]
    fn tile_encodes_game_boy_2bpp_rows() {
        let mut tile = Tile::blank();
        for x in 0..TILE_SIZE {
            tile.set_pixel(x, 0, if x % 2 == 0 { 1 } else { 2 })
                .expect("pixel");
        }

        let bytes = tile.encoded_2bpp();

        assert_eq!(bytes.len(), TILE_BYTES_2BPP);
        assert_eq!(bytes[0], 0b1010_1010);
        assert_eq!(bytes[1], 0b0101_0101);
    }

    #[test]
    fn bytecode_parser_roundtrips_game_boy_2bpp_tile() {
        let text =
            "0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,\r\n0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00";
        let bytes = parse_tile_bytecode(text).expect("parse bytecode");
        assert_eq!(bytes.len(), TILE_BYTES_2BPP);
        assert_eq!(bytes[0], 0x78);
        assert_eq!(bytes[15], 0x00);

        let tile = Tile::decode_2bpp(&bytes).expect("decode tile");
        assert_eq!(tile.encoded_2bpp(), bytes);
        assert_eq!(format_tile_bytecode(&bytes).lines().count(), 2);
    }

    #[test]
    fn project_sets_selected_tile_from_bytecode_bytes() {
        let bytes = parse_tile_bytecode(
            "0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00",
        )
        .expect("bytes");
        let mut project = Project::new(2, 2, 4).expect("project");
        project
            .set_tile_from_2bpp_bytes(2, &bytes)
            .expect("set tile");

        assert_eq!(project.tile(2).expect("tile").encoded_2bpp(), bytes);
    }

    #[test]
    fn save_dialog_defaults_to_project_name_and_adds_extension() {
        assert_eq!(project_default_filename("room: boss/1"), "room_ boss_1.gbh");
        assert_eq!(
            ensure_project_save_extension(PathBuf::from("room")),
            PathBuf::from("room.gbh")
        );
        assert_eq!(
            ensure_project_save_extension(PathBuf::from("room.gbr")),
            PathBuf::from("room.gbr")
        );
    }

    #[test]
    fn graphic_sizes_map_to_requested_pixel_and_tile_dimensions() {
        let expected = [
            (GraphicSize::Size8x8, (8, 8), (1, 1)),
            (GraphicSize::Size16x16, (16, 16), (2, 2)),
            (GraphicSize::Size32x32, (32, 32), (4, 4)),
            (GraphicSize::Size8x16, (8, 16), (1, 2)),
            (GraphicSize::Size16x8, (16, 8), (2, 1)),
            (GraphicSize::Size16x32, (16, 32), (2, 4)),
            (GraphicSize::Size32x16, (32, 16), (4, 2)),
        ];
        for (size, pixels, tiles) in expected {
            assert_eq!(size.dimensions(), pixels);
            assert_eq!(size.tile_dimensions(), tiles);
            assert_eq!(
                GraphicSize::from_dimensions(pixels.0, pixels.1).expect("supported size"),
                size
            );
        }
    }

    #[test]
    fn large_graphic_editor_paints_across_tiles_and_stamps_map_block() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor.selected_tile = 8;
        app.editor.selected_color = 3;
        click_action(
            &mut app,
            ButtonAction::SetGraphicSize(GraphicSize::Size16x16),
        );

        let graphic_point = tile_point(&app, 9, 10);
        click(&mut app, graphic_point);
        assert_eq!(
            app.editor
                .project
                .tile(11)
                .expect("bottom-right tile")
                .pixel(1, 2)
                .expect("painted pixel"),
            3
        );

        click_action(&mut app, ButtonAction::Tool(StudioTool::Map));
        let map_position = map_point(&app, 4, 5);
        click(&mut app, map_position);
        assert_eq!(app.editor.project.map_cell(4, 5).expect("top left").tile, 8);
        assert_eq!(
            app.editor.project.map_cell(5, 5).expect("top right").tile,
            9
        );
        assert_eq!(
            app.editor.project.map_cell(4, 6).expect("bottom left").tile,
            10
        );
        assert_eq!(
            app.editor
                .project
                .map_cell(5, 6)
                .expect("bottom right")
                .tile,
            11
        );
    }

    #[test]
    fn rectangular_graphic_rotation_swaps_canvas_dimensions() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor.selected_tile = 12;
        app.editor.graphic_size = GraphicSize::Size8x16;
        let mut pixels = vec![0; 8 * 16];
        pixels[15 * 8] = 2;
        write_graphic_pixels(
            &mut app.editor.project,
            app.editor.selected_tile,
            app.editor.graphic_size,
            &pixels,
        )
        .expect("seed graphic");

        click_action(&mut app, ButtonAction::Rotate);

        assert_eq!(app.editor.graphic_size, GraphicSize::Size16x8);
        let rotated = read_graphic_pixels(
            &app.editor.project,
            app.editor.selected_tile,
            app.editor.graphic_size,
        );
        assert_eq!(rotated[0], 2);
    }

    #[test]
    fn atlas_draw_data_changes_immediately_after_tile_edit() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor.selected_tile = 4;
        app.editor.selected_color = 3;
        let before = atlas_pixels(&app);

        let point = tile_point(&app, 0, 0);
        click(&mut app, point);
        let after = atlas_pixels(&app);

        let source_x = (app.editor.selected_tile % 8) * TILE_SIZE;
        let source_y = (app.editor.selected_tile / 8) * TILE_SIZE;
        let offset = (source_y * 8 * TILE_SIZE + source_x) * 4;
        assert_ne!(&before[offset..offset + 4], &after[offset..offset + 4]);
    }

    #[test]
    fn dedupe_updates_map_references() {
        let mut project = Project::new(4, 4, 4).expect("project");
        project.tiles[2] = project.tiles[1].clone();
        project.set_map_tile(1, 1, 2).expect("map");

        let report = project.dedupe_tiles();

        assert_eq!(report.removed, 1);
        assert_eq!(project.map_cell(1, 1).expect("cell").tile, 1);
    }

    #[test]
    fn export_contains_kitaqgb_map_runtime_surface() {
        let mut project = Project::new(4, 4, 8).expect("project");
        project
            .set_map_cell(
                1,
                1,
                MapCell {
                    tile: 4,
                    palette: 2,
                    collision: 1,
                    event: 7,
                    ..MapCell::default()
                },
            )
            .expect("cell");

        let c = project.export_kitaqgb_c("room-1").expect("export");

        assert!(c.contains("#include \"rpg.h\""));
        assert!(c.contains("__prg_rom u8 room_1_tile_data"));
        assert!(c.contains("map_t room_1_map"));
        assert!(c.contains("room_1_map.width = 4"));
        assert!(c.contains("__settileattr_bulk_fast"));
    }

    #[test]
    fn gbtd_roundtrip_preserves_tiles_and_palettes() {
        let mut project = Project::new(4, 3, 6).expect("project");
        project.name = "roundtrip_tiles".to_string();
        project.set_tile_pixel(2, 1, 0, 1).expect("first pixel");
        project.set_tile_pixel(2, 2, 0, 2).expect("second pixel");
        project.set_tile_pixel(2, 3, 0, 3).expect("third pixel");
        project.palettes[4][2] = PaletteColor::new(12, 34, 56);

        let bytes = project.to_gbtd_bytes().expect("gbtd");
        assert!(bytes.starts_with(b"GBO0"));
        let loaded = Project::from_gbtd_bytes(&bytes).expect("load gbtd");

        assert_eq!(loaded.name, "roundtrip_tiles");
        assert_eq!(loaded.tiles.len(), project.tiles.len());
        assert_eq!(
            loaded.tile(2).expect("loaded").pixels(),
            project.tile(2).expect("original").pixels()
        );
        assert_eq!(loaded.palettes[4][2], PaletteColor::new(12, 34, 56));
    }

    #[test]
    fn gbmb_roundtrip_preserves_map_records_and_bundle_tiles() {
        let base = std::env::temp_dir().join(format!("gbhua_gbmb_{}", std::process::id()));
        let gbm_path = base.with_extension("gbm");
        let gbr_path = base.with_extension("gbr");
        let _ = std::fs::remove_file(&gbm_path);
        let _ = std::fs::remove_file(&gbr_path);

        let mut project = Project::new(5, 4, 8).expect("project");
        project.name = "roundtrip_map".to_string();
        project.set_tile_pixel(6, 0, 0, 3).expect("tile pixel");
        project
            .set_map_cell(
                3,
                2,
                MapCell {
                    tile: 6,
                    palette: 5,
                    flip_x: true,
                    flip_y: true,
                    ..MapCell::default()
                },
            )
            .expect("map cell");

        let bytes = project.to_gbmb_bytes("roundtrip_map.gbr").expect("gbmb");
        assert!(bytes.starts_with(b"GBO1"));
        let loaded = Project::from_gbmb_bytes(&bytes).expect("load gbmb");
        assert_eq!(loaded.map_width, 5);
        assert_eq!(loaded.map_height, 4);
        let cell = loaded.map_cell(3, 2).expect("cell");
        assert_eq!(cell.tile, 6);
        assert_eq!(cell.palette, 5);
        assert!(cell.flip_x);
        assert!(cell.flip_y);

        project.save_gbmb_file(&gbm_path).expect("bundle save");
        assert!(gbm_path.exists());
        assert!(gbr_path.exists());
        let loaded = Project::load_gbmb_file(&gbm_path).expect("bundle load");
        assert_eq!(
            loaded.tile(6).expect("loaded tile").pixels(),
            project.tile(6).expect("original tile").pixels()
        );

        let _ = std::fs::remove_file(&gbm_path);
        let _ = std::fs::remove_file(&gbr_path);
    }

    #[test]
    fn screen_copy_omits_plita_and_uses_roboto_default() {
        assert_eq!(default_font_family(), "Roboto");

        let app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        let mut labels = vec![
            WINDOW_TITLE.to_string(),
            SCREEN_TITLE.to_string(),
            app.editor.status.clone(),
        ];
        labels.extend(
            app.action_buttons()
                .into_iter()
                .chain(app.inspector_buttons())
                .chain(app.bytecode_buttons())
                .map(|button| button.label.to_string()),
        );
        labels.push("BYTECODE".to_string());
        for group in [
            ActionGroup::Tools,
            ActionGroup::Edit,
            ActionGroup::File,
            ActionGroup::Tiles,
            ActionGroup::Canvas,
            ActionGroup::Sprites,
            ActionGroup::Export,
        ] {
            labels.extend(
                menu_entries(group)
                    .into_iter()
                    .map(|(_, label)| label.to_string()),
            );
        }

        for label in labels {
            assert!(
                !label.to_ascii_lowercase().contains("plita"),
                "screen label contains forbidden text: {label}"
            );
        }
    }

    #[test]
    fn dropdown_groups_drive_save_and_c_export_actions() {
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_C));
        let save_base =
            std::env::temp_dir().join(format!("gbhua_dropdown_save_{}", std::process::id()));
        let save_path = save_base.with_extension("gbh");
        let _ = std::fs::remove_file(&save_path);
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));

        assert!(!app
            .dropdown_buttons()
            .iter()
            .any(|button| button.action == ButtonAction::SaveProject));
        click_action(&mut app, ButtonAction::Menu(ActionGroup::File));
        assert!(app
            .action_buttons()
            .iter()
            .any(|button| button.action == ButtonAction::SaveProject));
        app.save_dialog_path = Some(save_base);
        click_action(&mut app, ButtonAction::SaveProject);
        assert!(save_path.exists());
        assert_eq!(
            app.editor.current_path.as_deref(),
            Some(save_path.as_path())
        );

        click_action(&mut app, ButtonAction::Menu(ActionGroup::Export));
        assert!(app
            .action_buttons()
            .iter()
            .any(|button| button.action == ButtonAction::ExportC));
        click_action(&mut app, ButtonAction::ExportC);
        assert!(std::fs::read_to_string(test_export_path(AUTOEXPORT_C))
            .expect("export c")
            .contains("kq_asset_load"));

        let _ = std::fs::remove_file(save_path);
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_C));
    }

    #[test]
    fn bytecode_window_imports_and_exports_selected_tile() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor.selected_tile = 2;
        app.editor.refresh_bytecode_from_selected_tile();
        let text =
            "0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,\n0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00";

        let input = center(app.bytecode_input_rect());
        click(&mut app, input);
        assert!(app.editor.bytecode.input.focused);
        app.handle_bytecode_key("A", true);
        app.handle_event(UiEvent::KeyDown {
            key: "Named(Delete)".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        for key in ["0", "x", "7", "8", ","] {
            app.handle_event(UiEvent::KeyDown {
                key: key.to_string(),
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
            });
        }
        assert_eq!(app.editor.bytecode.input.text, "0X78,");
        app.handle_bytecode_key("A", true);
        app.handle_event(UiEvent::KeyDown {
            key: "Named(Delete)".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        set_test_clipboard_text(text);
        app.handle_event(UiEvent::KeyDown {
            key: "V".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(app.editor.bytecode.input.text.contains("0x78"));
        click_action(&mut app, ButtonAction::BytecodeImport);

        let expected = parse_tile_bytecode(text).expect("expected");
        assert_eq!(
            app.editor
                .project
                .tile(app.editor.selected_tile)
                .expect("tile")
                .encoded_2bpp(),
            expected
        );
        assert!(app.editor.status.contains("Imported bytecode"));

        app.editor.project.tile_mut(2).expect("tile").fill(0);
        click_action(&mut app, ButtonAction::BytecodeExport);
        assert_eq!(
            parse_tile_bytecode(&app.editor.bytecode.input.text).expect("exported")[0],
            0
        );
        assert!(app.editor.status.contains("Exported bytecode"));
    }

    #[test]
    fn dmg_and_cgb_modes_switch_preview_palette_without_changing_data() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        app.editor
            .project
            .set_tile_pixel(0, 0, 0, 1)
            .expect("pixel");
        app.editor.project.palettes[0][1] = PaletteColor::new(255, 0, 0);

        let cgb = render_map_with_mode(&app.editor.project, ColorMode::Cgb);
        assert_eq!(&cgb[0..4], &[255, 0, 0, 255]);

        click_action(&mut app, ButtonAction::SetColorMode(ColorMode::Dmg));
        assert_eq!(app.editor.color_mode, ColorMode::Dmg);
        let dmg = render_map_with_mode(&app.editor.project, app.editor.color_mode);
        assert_eq!(
            &dmg[0..4],
            &[DMG_PALETTE[1].r, DMG_PALETTE[1].g, DMG_PALETTE[1].b, 255]
        );
        assert_eq!(
            app.editor.project.palettes[0][1],
            PaletteColor::new(255, 0, 0)
        );

        click_action(&mut app, ButtonAction::SetColorMode(ColorMode::Cgb));
        assert_eq!(app.editor.color_mode, ColorMode::Cgb);
    }

    #[test]
    fn atlas_and_inspector_wheel_scroll_use_shared_scrollbar_state() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 220.0));
        for _ in 0..160 {
            app.editor.project.add_tile(Tile::blank());
        }

        let atlas = center(app.atlas_viewport_rect());
        app.handle_event(UiEvent::PointerWheel {
            position: atlas,
            delta_x: 0.0,
            delta_y: -96.0,
        });
        assert!(app.atlas_scroll.offset().y > 0.0);

        let inspector = center(app.inspector_viewport_rect());
        app.handle_event(UiEvent::PointerWheel {
            position: inspector,
            delta_x: 0.0,
            delta_y: -96.0,
        });
        assert!(app.inspector_scroll.offset().y > 0.0);
    }

    #[test]
    fn map_panel_top_aligns_with_tile_editor_top() {
        let app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        assert_eq!(app.map_panel_rect().y, app.graphic_panel_rect().y);
        assert_eq!(app.map_viewport_rect().y, app.tile_editor_rect().y);
    }

    #[test]
    fn autotile_uses_neighbor_collision_mask() {
        let mut project = Project::new(3, 3, 32).expect("project");
        for y in 0..3 {
            for x in 0..3 {
                let mut cell = MapCell::with_tile(1);
                cell.collision = 1;
                project.set_map_cell(x, y, cell).expect("cell");
            }
        }
        let tiles: Vec<u16> = (0..16).collect();

        let changed = project.apply_neighbor_autotile(&tiles).expect("autotile");

        assert_eq!(changed, 9);
        assert_eq!(project.map_cell(1, 1).expect("center").tile, 15);
        assert_eq!(project.map_cell(0, 0).expect("corner").tile, 0b0110);
    }

    #[test]
    fn gui_pointer_paths_operate_editor_tools() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        let draw = app.draw_list();
        assert!(draw.len() > 20);

        for tool in [
            StudioTool::Pixel,
            StudioTool::Map,
            StudioTool::Rect,
            StudioTool::Fill,
            StudioTool::Attr,
            StudioTool::Collision,
            StudioTool::Event,
        ] {
            click_action(&mut app, ButtonAction::Tool(tool));
            assert_eq!(app.editor.selected_tool, tool);
        }

        let point = palette_point(&app, 2);
        click(&mut app, point);
        assert_eq!(app.editor.selected_color, 2);

        let point = cgb_palette_point(&app, 5, 1);
        click(&mut app, point);
        assert_eq!(app.editor.selected_palette, 5);
        assert_eq!(app.editor.selected_color, 1);

        let point = palette_point(&app, 2);
        click(&mut app, point);
        assert_eq!(app.editor.selected_color, 2);

        let point = atlas_point(&app, 5);
        click(&mut app, point);
        assert_eq!(app.editor.selected_tile, 5);

        let point = tile_point(&app, 2, 3);
        click(&mut app, point);
        assert_eq!(
            app.editor
                .project
                .tile(5)
                .expect("tile")
                .pixel(2, 3)
                .expect("pixel"),
            2
        );

        app.editor.selected_tool = StudioTool::Map;
        app.editor.selected_palette = 2;
        let point = map_point(&app, 4, 6);
        click(&mut app, point);
        let painted = app.editor.project.map_cell(4, 6).expect("painted");
        assert_eq!(painted.tile, 5);
        assert_eq!(painted.palette, 2);

        app.editor.selected_tile = 1;
        app.editor.selected_palette = 0;
        let point = map_point(&app, 4, 6);
        right_click(&mut app, point);
        assert_eq!(app.editor.selected_tile, 5);
        assert_eq!(app.editor.selected_palette, 2);

        click_action(&mut app, ButtonAction::Tool(StudioTool::Fill));
        app.editor.selected_tile = 4;
        app.editor.selected_palette = 1;
        let point = map_point(&app, 10, 10);
        click(&mut app, point);
        assert_eq!(app.editor.project.map_cell(10, 10).expect("fill").tile, 4);
        assert!(app.editor.status.starts_with("Filled "));

        click_action(&mut app, ButtonAction::Tool(StudioTool::Collision));
        let point = map_point(&app, 1, 1);
        click(&mut app, point);
        assert_eq!(
            app.editor
                .project
                .map_cell(1, 1)
                .expect("collision")
                .collision,
            1
        );

        click_action(&mut app, ButtonAction::Tool(StudioTool::Event));
        let point = map_point(&app, 2, 1);
        click(&mut app, point);
        assert_eq!(app.editor.project.map_cell(2, 1).expect("event").event, 1);

        click_action(&mut app, ButtonAction::Tool(StudioTool::Attr));
        app.editor.selected_palette = 3;
        let point = map_point(&app, 3, 1);
        click(&mut app, point);
        let attr = app.editor.project.map_cell(3, 1).expect("attr");
        assert_eq!(attr.palette, 3);
        assert!(attr.flip_x);

        click_action(&mut app, ButtonAction::Tool(StudioTool::Rect));
        app.editor.selected_tile = 3;
        let point = map_point(&app, 20, 20);
        click(&mut app, point);
        assert_eq!(app.editor.project.map_cell(22, 22).expect("rect").tile, 3);

        let hover = map_point(&app, 5, 5);
        app.handle_event(UiEvent::PointerMove { position: hover });
        assert_eq!(app.editor.map_cursor, Some((5, 5)));
    }

    #[test]
    fn gui_buttons_use_plita_common_hover_press_release_state() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        click_action(&mut app, ButtonAction::Menu(ActionGroup::Tools));
        let action = ButtonAction::Tool(StudioTool::Pixel);
        let button = action_button(&app, action);
        let point = center(button.rect);
        let theme = app.theme();

        assert_eq!(
            app.buttons.visual_state(&action, false),
            ButtonVisualState::Normal
        );
        assert_eq!(
            action_fill_color(&app, action),
            workspace_ui::linear_color(theme.panel_alt)
        );

        app.handle_event(UiEvent::PointerMove { position: point });
        assert_eq!(app.buttons.hovered().copied(), Some(action));
        assert_eq!(
            app.buttons.visual_state(&action, false),
            ButtonVisualState::Hovered
        );
        assert_eq!(
            action_fill_color(&app, action),
            workspace_ui::linear_color(theme.button_hover)
        );

        app.handle_event(UiEvent::PointerDown {
            position: point,
            button: PointerButton::Primary,
        });
        assert_eq!(app.buttons.pressed().copied(), Some(action));
        assert_eq!(
            app.buttons.visual_state(&action, false),
            ButtonVisualState::Pressed
        );
        assert_eq!(app.editor.selected_tool, StudioTool::Map);

        app.handle_event(UiEvent::PointerUp {
            position: point,
            button: PointerButton::Primary,
        });
        assert_eq!(app.editor.selected_tool, StudioTool::Pixel);
        assert_eq!(app.buttons.pressed(), None);
        assert_eq!(app.buttons.hovered().copied(), Some(action));
        assert_eq!(
            app.buttons.visual_state(&action, true),
            ButtonVisualState::Active
        );
    }

    #[test]
    fn gui_button_release_elsewhere_does_not_activate_action() {
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));
        click_action(&mut app, ButtonAction::Menu(ActionGroup::Tools));
        let action = ButtonAction::Tool(StudioTool::Pixel);
        let point = center(action_button(&app, action).rect);

        app.handle_event(UiEvent::PointerDown {
            position: point,
            button: PointerButton::Primary,
        });
        app.handle_event(UiEvent::PointerUp {
            position: Point::new(4.0, 4.0),
            button: PointerButton::Primary,
        });

        assert_eq!(app.editor.selected_tool, StudioTool::Map);
        assert_eq!(app.buttons.pressed(), None);
        assert_eq!(app.buttons.hovered(), None);
    }

    #[test]
    fn atlas_preview_uses_requested_cgb_palette() {
        let mut project = Project::new(2, 2, 1).expect("project");
        project
            .set_tile_pixel(0, 0, 0, 1)
            .expect("first atlas pixel");
        project.palettes[0][1] = PaletteColor::new(255, 0, 0);
        project.palettes[3][1] = PaletteColor::new(0, 0, 255);

        let palette_0 = project.render_atlas_rgba_with_palette(1, 0);
        let palette_3 = project.render_atlas_rgba_with_palette(1, 3);

        assert_eq!(&palette_0[0..4], &[255, 0, 0, 255]);
        assert_eq!(&palette_3[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn gui_buttons_operate_asset_commands_save_export_and_file_drop() {
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_C));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBR));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBM));
        let save_path =
            std::env::temp_dir().join(format!("gbhua_commands_save_{}.gbh", std::process::id()));
        let save_as_path =
            std::env::temp_dir().join(format!("gbhua_commands_save_as_{}.gbh", std::process::id()));
        let _ = std::fs::remove_file(&save_path);
        let _ = std::fs::remove_file(&save_as_path);
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));

        let start_tiles = app.editor.project.tiles.len();
        click_action(&mut app, ButtonAction::AddTile);
        assert_eq!(app.editor.project.tiles.len(), start_tiles + 1);
        click_action(&mut app, ButtonAction::Undo);
        assert_eq!(app.editor.project.tiles.len(), start_tiles);
        click_action(&mut app, ButtonAction::Redo);
        assert_eq!(app.editor.project.tiles.len(), start_tiles + 1);

        app.editor.selected_color = 3;
        let selected = app.editor.selected_tile;
        click_action(&mut app, ButtonAction::FillTile);
        assert!(app
            .editor
            .project
            .tile(selected)
            .expect("tile")
            .pixels()
            .iter()
            .all(|value| *value == 3));

        app.editor.project.tile_mut(selected).expect("tile").fill(0);
        app.editor
            .project
            .set_tile_pixel(selected, 0, 1, 1)
            .expect("pixel");
        click_action(&mut app, ButtonAction::FlipX);
        assert_eq!(
            app.editor
                .project
                .tile(selected)
                .expect("tile")
                .pixel(7, 1)
                .expect("pixel"),
            1
        );
        click_action(&mut app, ButtonAction::FlipY);
        assert_eq!(
            app.editor
                .project
                .tile(selected)
                .expect("tile")
                .pixel(7, 6)
                .expect("pixel"),
            1
        );
        let before_rotate = app.editor.project.tile(selected).expect("tile").clone();
        click_action(&mut app, ButtonAction::Rotate);
        assert_ne!(
            &before_rotate,
            app.editor.project.tile(selected).expect("tile")
        );

        for y in 0..3 {
            for x in 0..3 {
                let mut cell = MapCell::with_tile(1);
                cell.collision = 1;
                app.editor.project.set_map_cell(x, y, cell).expect("cell");
            }
        }
        click_action(&mut app, ButtonAction::AutoCollision);
        assert_eq!(
            app.editor
                .project
                .map_cell(0, 0)
                .expect("auto collision")
                .collision,
            1
        );
        click_action(&mut app, ButtonAction::AutoTile);
        assert_eq!(
            app.editor.project.map_cell(1, 1).expect("autotile").tile,
            31
        );

        app.editor.project.tiles[10] = app.editor.project.tiles[1].clone();
        app.editor.project.set_map_tile(4, 4, 10).expect("ref");
        let before_dedupe = app.editor.project.tiles.len();
        click_action(&mut app, ButtonAction::Dedupe);
        assert!(app.editor.project.tiles.len() < before_dedupe);
        assert_ne!(app.editor.project.map_cell(4, 4).expect("remap").tile, 10);

        app.save_dialog_path = Some(save_path.clone());
        click_action(&mut app, ButtonAction::SaveProject);
        assert!(save_path.exists());
        click_action(&mut app, ButtonAction::ExportC);
        assert!(std::fs::read_to_string(test_export_path(AUTOEXPORT_C))
            .expect("export c")
            .contains("kq_asset_load"));
        click_action(&mut app, ButtonAction::ExportGbr);
        assert!(test_export_path(AUTOEXPORT_GBR).exists());
        click_action(&mut app, ButtonAction::ExportGbm);
        assert!(test_export_path(AUTOEXPORT_GBM).exists());
        assert!(test_export_path(AUTOEXPORT_GBR).exists());

        let drop_path = std::env::temp_dir().join(format!("gbhua_drop_{}.gbh", std::process::id()));
        let mut dropped = Project::new(6, 5, 8).expect("dropped project");
        dropped.name = "dropped_room".to_string();
        dropped.save_json_file(&drop_path).expect("save dropped");
        app.handle_event(UiEvent::FileDropped {
            path: drop_path.clone(),
        });
        assert_eq!(app.editor.project.map_width, 6);
        assert_eq!(app.editor.project.name, "dropped_room");
        assert_eq!(
            app.editor.current_path.as_deref(),
            Some(drop_path.as_path())
        );

        app.editor.project.name = "saved_drop".to_string();
        app.save_dialog_path = Some(save_as_path.clone());
        click_action(&mut app, ButtonAction::SaveProject);
        let saved = Project::load_project_file(&save_as_path).expect("load saved as");
        assert_eq!(saved.name, "saved_drop");
        assert_eq!(
            app.editor.current_path.as_deref(),
            Some(save_as_path.as_path())
        );

        let _ = std::fs::remove_file(save_path);
        let _ = std::fs::remove_file(save_as_path);
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_C));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBR));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBM));
        let _ = std::fs::remove_file(drop_path);
    }

    #[test]
    fn keyboard_shortcuts_operate_tools_and_quick_commands() {
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_C));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBR));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBM));
        let save_path =
            std::env::temp_dir().join(format!("gbhua_shortcut_save_{}.gbh", std::process::id()));
        let _ = std::fs::remove_file(&save_path);
        let mut app = AssetStudioApp::new(Size::new(1360.0, 820.0));

        for (key, color) in [("1", 0), ("2", 1), ("3", 2), ("4", 3)] {
            app.handle_event(UiEvent::KeyDown {
                key: key.to_string(),
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
            });
            assert_eq!(app.editor.selected_color, color);
        }
        for (key, tool) in [
            ("P", StudioTool::Pixel),
            ("M", StudioTool::Map),
            ("F", StudioTool::Fill),
            ("C", StudioTool::Collision),
            ("E", StudioTool::Event),
        ] {
            app.handle_event(UiEvent::KeyDown {
                key: key.to_string(),
                ctrl: false,
                shift: false,
                alt: false,
                meta: false,
            });
            assert_eq!(app.editor.selected_tool, tool);
        }

        let palette = app.editor.selected_palette;
        app.handle_event(UiEvent::KeyDown {
            key: "A".to_string(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(app.editor.selected_palette, (palette + 1) & 0x07);

        click_action(&mut app, ButtonAction::AddTile);
        let after_add = app.editor.project.tiles.len();
        app.handle_event(UiEvent::KeyDown {
            key: "Z".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(app.editor.project.tiles.len(), after_add - 1);
        app.handle_event(UiEvent::KeyDown {
            key: "Y".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert_eq!(app.editor.project.tiles.len(), after_add);

        app.save_dialog_path = Some(save_path.clone());
        app.handle_event(UiEvent::KeyDown {
            key: "S".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(save_path.exists());
        app.handle_event(UiEvent::KeyDown {
            key: "E".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(test_export_path(AUTOEXPORT_C).exists());
        app.handle_event(UiEvent::KeyDown {
            key: "G".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(test_export_path(AUTOEXPORT_GBR).exists());
        app.handle_event(UiEvent::KeyDown {
            key: "B".to_string(),
            ctrl: true,
            shift: false,
            alt: false,
            meta: false,
        });
        assert!(test_export_path(AUTOEXPORT_GBM).exists());

        let _ = std::fs::remove_file(save_path);
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_C));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBR));
        let _ = std::fs::remove_file(test_export_path(AUTOEXPORT_GBM));
    }

    #[test]
    fn project_roundtrips_typed_collision_and_metasprite_frames() {
        let mut project = Project::new(20, 36, 4).expect("project");
        project.map[21].collision = 12;
        let metasprite = project.add_metasprite("flipper");
        project.metasprites[metasprite].frames[0]
            .parts
            .push(MetaSpritePart {
                dx: -8,
                dy: 0,
                tile: 2,
                flags: 1,
            });

        let json = project.to_json_pretty().expect("json");
        let restored = Project::from_json_str(&json).expect("restore");

        assert_eq!(restored.map[21].collision, 12);
        assert_eq!(restored.metasprites[0].frames[0].parts[0].dx, -8);
    }

    #[test]
    fn metasprite_export_contains_frame_tables_and_preview_pixels() {
        let mut project = Project::new(4, 4, 3).expect("project");
        project.set_tile_pixel(2, 0, 0, 3).expect("pixel");
        let metasprite = project.add_metasprite("flipper");
        project.metasprites[metasprite].frames[0]
            .parts
            .push(MetaSpritePart {
                dx: 0,
                dy: 0,
                tile: 2,
                flags: 0,
            });

        let export = project.export_kitaqgb_c("demo").expect("export");
        let preview = project
            .render_metasprite_rgba(0, 0, 16, 16, 4, 4)
            .expect("preview");

        assert!(export.contains("DEMO_FLIPPER_0_FRAME_COUNT 1"));
        assert!(export.contains("demo_flipper_0_parts"));
        assert!(preview.chunks_exact(4).any(|pixel| pixel[3] == 255));
    }
}
