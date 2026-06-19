//! DCC-style editor showcase surface.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use kinetik_ui::core::{
    ActionDescriptor, ActionSource, Axis, Brush, ClipId, Color, CornerRadius, ImageId, Key,
    KeyState, LinePrimitive, Modifiers, Point, Primitive, Rect, RectPrimitive, Shortcut, Size,
    Stroke, TextPrimitive, TextureId, Vec2,
};
use kinetik_ui::render::{
    ImageAtlasRegion, ImageResource, RenderImage, RenderImageSampling, RenderResources,
    TextureResource,
};
use kinetik_ui::text::TextEditState;
use kinetik_ui::widgets::{
    DockArea, DockNode, Frame, FrameId, GridColumns, GridLayout, Guide, ItemId, ListLayout, Menu,
    MenuItem, OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PanZoom, Panel,
    PanelId, PopoverPlacement, PopoverRequest, PropertyGridLayout, PropertyGridRow, TableColumn,
    TableLayout, TreeExpansion, TreeItem, TreeLayout, TreeModel, Ui, ViewportComposition,
    ViewportFit, ViewportSurface, frame_tabs, place_popover, solve_dock_layout,
    solve_dock_splitters,
};

/// Saves the current editor project.
pub const ACTION_SAVE: &str = "editor.save";
/// Toggles editor play mode.
pub const ACTION_PLAY: &str = "editor.play";
/// Stops editor play mode.
pub const ACTION_STOP: &str = "editor.stop";
/// Toggles viewport grid overlays.
pub const ACTION_GRID: &str = "editor.grid";
/// Queues a project build.
pub const ACTION_BUILD: &str = "editor.build";
/// Requests the command palette.
pub const ACTION_PALETTE: &str = "editor.palette";
const ACTION_TOOL_SELECT: &str = "editor.tool.select";
const ACTION_TOOL_MOVE: &str = "editor.tool.move";
const ACTION_TOOL_ROTATE: &str = "editor.tool.rotate";
const ACTION_TOOL_SCALE: &str = "editor.tool.scale";

const VIEWPORT_TEXTURE: TextureId = TextureId::from_raw(9_001);
const VIEWPORT_SIZE: Size = Size::new(1280.0, 720.0);

const ICON_ATLAS: ImageId = ImageId::from_raw(7_000);
const ICON_SIZE: u32 = 32;
const ICON_ATLAS_PADDING: u32 = 1;
const ICON_ATLAS_CELL_SIZE: u32 = ICON_SIZE + ICON_ATLAS_PADDING * 2;
const ICON_ATLAS_COLUMNS: u32 = 7;
const ICON_ATLAS_ROWS: u32 = 4;
const DENSE_ICON_SIZE: f32 = 16.0;
const ASSET_ICON_SIZE: f32 = 24.0;

const ICON_CURSOR: ImageId = ImageId::from_raw(7_001);
const ICON_MOVE: ImageId = ImageId::from_raw(7_002);
const ICON_TRANSFORM: ImageId = ImageId::from_raw(7_003);
const ICON_ROTATE: ImageId = ImageId::from_raw(7_004);
const ICON_CUBE: ImageId = ImageId::from_raw(7_005);
const ICON_PLAY: ImageId = ImageId::from_raw(7_006);
const ICON_PAUSE: ImageId = ImageId::from_raw(7_007);
const ICON_STOP: ImageId = ImageId::from_raw(7_008);
const ICON_PLUS: ImageId = ImageId::from_raw(7_009);
const ICON_SEARCH: ImageId = ImageId::from_raw(7_010);
const ICON_ARCHIVE: ImageId = ImageId::from_raw(7_011);
const ICON_FILE: ImageId = ImageId::from_raw(7_012);
const ICON_IMAGE: ImageId = ImageId::from_raw(7_013);
const ICON_GEAR: ImageId = ImageId::from_raw(7_014);
const ICON_GRID: ImageId = ImageId::from_raw(7_015);
const ICON_LAYERS: ImageId = ImageId::from_raw(7_016);
const ICON_CODE: ImageId = ImageId::from_raw(7_017);
const ICON_BOX: ImageId = ImageId::from_raw(7_018);
const ICON_ROCKET: ImageId = ImageId::from_raw(7_019);
const ICON_DOWNLOAD: ImageId = ImageId::from_raw(7_020);
const ICON_DOTS: ImageId = ImageId::from_raw(7_021);
const ICON_CHEVRON: ImageId = ImageId::from_raw(7_022);
const ICON_CARET: ImageId = ImageId::from_raw(7_023);
const ICON_RESET: ImageId = ImageId::from_raw(7_024);
const ICON_COMPONENT: ImageId = ImageId::from_raw(7_025);
const ICON_TOKENS: ImageId = ImageId::from_raw(7_026);
const ICON_EYE: ImageId = ImageId::from_raw(7_027);
const ICON_CROSSHAIR: ImageId = ImageId::from_raw(7_028);

const FRAME_SCENE: FrameId = FrameId::from_raw(1);
const FRAME_ASSETS: FrameId = FrameId::from_raw(2);
const FRAME_VIEWPORT: FrameId = FrameId::from_raw(3);
const FRAME_BOTTOM: FrameId = FrameId::from_raw(4);
const FRAME_INSPECTOR: FrameId = FrameId::from_raw(5);

const PANEL_SCENE: PanelId = PanelId::from_raw(1);
const PANEL_ASSETS: PanelId = PanelId::from_raw(2);
const PANEL_VIEWPORT: PanelId = PanelId::from_raw(3);
const PANEL_CONSOLE: PanelId = PanelId::from_raw(4);
const PANEL_JOBS: PanelId = PanelId::from_raw(5);
const PANEL_INSPECTOR: PanelId = PanelId::from_raw(6);

const ICON_ASSETS: &[(ImageId, &[u8])] = &[
    (
        ICON_CURSOR,
        include_bytes!("../assets/icons/cursor-arrow.rgba"),
    ),
    (ICON_MOVE, include_bytes!("../assets/icons/move.rgba")),
    (
        ICON_TRANSFORM,
        include_bytes!("../assets/icons/transform.rgba"),
    ),
    (
        ICON_ROTATE,
        include_bytes!("../assets/icons/rotate-counter-clockwise.rgba"),
    ),
    (ICON_CUBE, include_bytes!("../assets/icons/cube.rgba")),
    (ICON_PLAY, include_bytes!("../assets/icons/play.rgba")),
    (ICON_PAUSE, include_bytes!("../assets/icons/pause.rgba")),
    (ICON_STOP, include_bytes!("../assets/icons/stop.rgba")),
    (ICON_PLUS, include_bytes!("../assets/icons/plus.rgba")),
    (
        ICON_SEARCH,
        include_bytes!("../assets/icons/magnifying-glass.rgba"),
    ),
    (ICON_ARCHIVE, include_bytes!("../assets/icons/archive.rgba")),
    (ICON_FILE, include_bytes!("../assets/icons/file.rgba")),
    (ICON_IMAGE, include_bytes!("../assets/icons/image.rgba")),
    (ICON_GEAR, include_bytes!("../assets/icons/gear.rgba")),
    (ICON_GRID, include_bytes!("../assets/icons/view-grid.rgba")),
    (ICON_LAYERS, include_bytes!("../assets/icons/layers.rgba")),
    (ICON_CODE, include_bytes!("../assets/icons/code.rgba")),
    (ICON_BOX, include_bytes!("../assets/icons/box.rgba")),
    (ICON_ROCKET, include_bytes!("../assets/icons/rocket.rgba")),
    (
        ICON_DOWNLOAD,
        include_bytes!("../assets/icons/download.rgba"),
    ),
    (
        ICON_DOTS,
        include_bytes!("../assets/icons/dots-horizontal.rgba"),
    ),
    (
        ICON_CHEVRON,
        include_bytes!("../assets/icons/chevron-right.rgba"),
    ),
    (
        ICON_CARET,
        include_bytes!("../assets/icons/caret-down.rgba"),
    ),
    (ICON_RESET, include_bytes!("../assets/icons/reset.rgba")),
    (
        ICON_COMPONENT,
        include_bytes!("../assets/icons/component-1.rgba"),
    ),
    (ICON_TOKENS, include_bytes!("../assets/icons/tokens.rgba")),
    (ICON_EYE, include_bytes!("../assets/icons/eye-open.rgba")),
    (
        ICON_CROSSHAIR,
        include_bytes!("../assets/icons/crosshair-2.rgba"),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTool {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EditorMenuKind {
    File,
    Edit,
    View,
    Project,
    Build,
    Window,
    Help,
}

impl EditorMenuKind {
    const fn raw(self) -> u64 {
        match self {
            Self::File => 1,
            Self::Edit => 2,
            Self::View => 3,
            Self::Project => 4,
            Self::Build => 5,
            Self::Window => 6,
            Self::Help => 7,
        }
    }
}

/// Action emitted by the editor UI to the application-owned action system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorInvocation {
    /// Application-owned action identity.
    pub action_id: &'static str,
    /// UI surface that requested the action.
    pub source: ActionSource,
}

/// Interactive DCC/editor showcase state.
pub struct EditorShowcase {
    dock: DockArea,
    scene_expansion: TreeExpansion,
    selected_node: ItemId,
    selected_asset: usize,
    selected_tool: EditorTool,
    running: bool,
    grid_visible: bool,
    snap_enabled: bool,
    viewport_pan_zoom: PanZoom,
    asset_filter: TextEditState,
    pos_x: TextEditState,
    pos_y: TextEditState,
    pos_z: TextEditState,
    scale: TextEditState,
    mass: TextEditState,
    exposure: f32,
    roughness: f32,
    timeline: f32,
    status: String,
    open_menu: Option<EditorMenuKind>,
}

impl Default for EditorShowcase {
    fn default() -> Self {
        let mut scene_expansion = TreeExpansion::new();
        scene_expansion.expand(item_id(1));
        scene_expansion.expand(item_id(2));
        scene_expansion.expand(item_id(6));
        Self {
            dock: default_dock(),
            scene_expansion,
            selected_node: item_id(7),
            selected_asset: 0,
            selected_tool: EditorTool::Move,
            running: false,
            grid_visible: true,
            snap_enabled: true,
            viewport_pan_zoom: PanZoom {
                fit: ViewportFit::Fit,
                zoom: 1.0,
                pan: Vec2::ZERO,
            },
            asset_filter: TextEditState::new("terrain"),
            pos_x: TextEditState::new("12.0"),
            pos_y: TextEditState::new("1.5"),
            pos_z: TextEditState::new("-6.0"),
            scale: TextEditState::new("1.0"),
            mass: TextEditState::new("84.0"),
            exposure: 0.58,
            roughness: 0.36,
            timeline: 0.41,
            status: "Editor ready".to_owned(),
            open_menu: None,
        }
    }
}

impl EditorShowcase {
    /// Creates an editor showcase.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies an editor-owned state transition for an action ID.
    pub fn apply_action(&mut self, action_id: &str) -> bool {
        match action_id {
            ACTION_SAVE => {
                "Saved project snapshot".clone_into(&mut self.status);
                true
            }
            ACTION_PLAY => {
                self.running = !self.running;
                let status = if self.running {
                    "Play mode running"
                } else {
                    "Play mode paused"
                };
                status.clone_into(&mut self.status);
                true
            }
            ACTION_STOP => {
                self.running = false;
                self.timeline = 0.0;
                "Play mode stopped".clone_into(&mut self.status);
                true
            }
            ACTION_GRID => {
                self.grid_visible = !self.grid_visible;
                let status = if self.grid_visible {
                    "Viewport grid enabled"
                } else {
                    "Viewport grid hidden"
                };
                status.clone_into(&mut self.status);
                true
            }
            ACTION_BUILD => {
                "Build queued for Windows x64".clone_into(&mut self.status);
                true
            }
            ACTION_PALETTE => {
                "Command palette requested".clone_into(&mut self.status);
                true
            }
            ACTION_TOOL_SELECT => self.select_tool(EditorTool::Select),
            ACTION_TOOL_MOVE => self.select_tool(EditorTool::Move),
            ACTION_TOOL_ROTATE => self.select_tool(EditorTool::Rotate),
            ACTION_TOOL_SCALE => self.select_tool(EditorTool::Scale),
            _ => false,
        }
    }

    /// Renders the editor and returns application action invocations.
    pub fn render(&mut self, ui: &mut Ui<'_>, action_count: u32) -> Vec<EditorInvocation> {
        let viewport = Rect::new(
            0.0,
            0.0,
            ui.viewport().logical_size.width,
            ui.viewport().logical_size.height,
        );
        let mut invocations = Vec::new();
        Self::background(ui, viewport);
        self.dismiss_menu_for_input(ui, viewport);
        self.menu_bar(ui, viewport);
        self.tool_bar(ui, viewport, &mut invocations);
        self.workspace(ui, viewport);
        self.menu_overlay(ui, viewport, &mut invocations);
        self.status_bar(ui, viewport, action_count + invocations.len() as u32);
        invocations
    }

    fn select_tool(&mut self, tool: EditorTool) -> bool {
        self.selected_tool = tool;
        let status = match tool {
            EditorTool::Select => "Select tool active",
            EditorTool::Move => "Move tool active",
            EditorTool::Rotate => "Rotate tool active",
            EditorTool::Scale => "Scale tool active",
        };
        status.clone_into(&mut self.status);
        true
    }

    fn trigger(
        &mut self,
        invocations: &mut Vec<EditorInvocation>,
        action_id: &'static str,
        source: ActionSource,
    ) {
        if self.apply_action(action_id) {
            invocations.push(EditorInvocation { action_id, source });
        }
    }

    fn background(ui: &mut Ui<'_>, viewport: Rect) {
        rect(ui, viewport, rgb(20, 21, 23), None);
        rect(
            ui,
            Rect::new(0.0, 0.0, viewport.width, 28.0),
            rgb(32, 34, 37),
            None,
        );
        rect(
            ui,
            Rect::new(0.0, 28.0, viewport.width, 36.0),
            rgb(25, 26, 29),
            None,
        );
        rect(
            ui,
            Rect::new(0.0, 63.0, viewport.width, 1.0),
            rgb(55, 58, 64),
            None,
        );
    }

    fn menu_bar(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
        text(ui, 12.0, 18.0, "Kinetik Forge", 13.0, rgb(226, 229, 234));
        for (kind, label, rect) in menu_header_rects() {
            let response = ui.pressable(("editor.menu-header", kind), rect, false);
            let active = self.open_menu == Some(kind);
            if active || response.state.hovered {
                rect_fill(
                    ui,
                    rect,
                    if active {
                        rgb(44, 47, 52)
                    } else {
                        rgb(34, 36, 40)
                    },
                    Some(rgb(66, 70, 78)),
                    CornerRadius::all(0.0),
                );
            }
            text(
                ui,
                rect.x + 10.0,
                17.0,
                label,
                11.0,
                if active {
                    rgb(238, 240, 244)
                } else {
                    rgb(196, 200, 207)
                },
            );
            if response.clicked {
                self.open_menu = if active { None } else { Some(kind) };
            } else if self.open_menu.is_some() && response.state.hovered {
                self.open_menu = Some(kind);
            }
        }

        let hint = if self.running {
            "Play Mode: Running"
        } else {
            "Play Mode: Edit"
        };
        text(
            ui,
            viewport.max_x() - 190.0,
            18.0,
            hint,
            11.0,
            if self.running {
                rgb(110, 205, 126)
            } else {
                rgb(170, 175, 182)
            },
        );
    }

    fn dismiss_menu_for_input(&mut self, ui: &Ui<'_>, viewport: Rect) {
        let Some(kind) = self.open_menu else {
            return;
        };
        let escape_pressed = ui
            .input()
            .keyboard
            .events
            .iter()
            .any(|event| event.state == KeyState::Pressed && matches!(event.key, Key::Escape));
        let outside_activation = ui.input().pointer.position.filter(|point| {
            ui.input().pointer.primary.released && !menu_bar_rect().contains_point(*point)
        });
        let mut stack = OverlayStack::new();
        stack.open(Self::menu_overlay_entry(kind, viewport));
        if !stack
            .dismissal_requests(outside_activation, escape_pressed)
            .is_empty()
        {
            self.open_menu = None;
        }
    }

    fn menu_overlay(
        &mut self,
        ui: &mut Ui<'_>,
        viewport: Rect,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        let Some(kind) = self.open_menu else {
            return;
        };
        let entry = Self::menu_overlay_entry(kind, viewport);
        let menu = self.menu_model(kind);
        let visible_items = menu.visible_items();
        rect_fill(
            ui,
            entry.rect.translate(Vec2::new(0.0, 2.0)),
            rgb(0, 0, 0),
            None,
            CornerRadius::all(0.0),
        );
        rect_fill(
            ui,
            entry.rect,
            rgb(28, 30, 33),
            Some(rgb(74, 78, 86)),
            CornerRadius::all(0.0),
        );

        let mut y = entry.rect.y + 6.0;
        for (index, item) in visible_items.into_iter().enumerate() {
            match item {
                MenuItem::Label(label) => {
                    text(
                        ui,
                        entry.rect.x + 10.0,
                        y + 15.0,
                        label,
                        10.0,
                        rgb(145, 150, 158),
                    );
                    y += 22.0;
                }
                MenuItem::Separator => {
                    rect(
                        ui,
                        Rect::new(entry.rect.x + 8.0, y + 4.0, entry.rect.width - 16.0, 1.0),
                        rgb(60, 63, 70),
                        None,
                    );
                    y += 9.0;
                }
                MenuItem::Action(action) => {
                    let row = Rect::new(entry.rect.x + 4.0, y, entry.rect.width - 8.0, 24.0);
                    let enabled = action.can_invoke();
                    let response = ui.pressable(
                        ("editor.menu-row", kind, index, action.id.as_str()),
                        row,
                        !enabled,
                    );
                    if response.state.hovered && enabled {
                        rect_fill(ui, row, rgb(43, 78, 132), None, CornerRadius::all(0.0));
                    }
                    if action.state.is_checked() {
                        rect(
                            ui,
                            Rect::new(row.x + 8.0, row.y + 8.0, 8.0, 8.0),
                            rgb(45, 110, 230),
                            None,
                        );
                    }
                    text(
                        ui,
                        row.x + 24.0,
                        row.y + 16.0,
                        &action.label,
                        11.0,
                        if enabled {
                            rgb(224, 227, 232)
                        } else {
                            rgb(112, 117, 126)
                        },
                    );
                    if let Some(shortcut) = action.shortcut.as_ref() {
                        let shortcut = shortcut_label(shortcut);
                        text(
                            ui,
                            row.max_x() - 74.0,
                            row.y + 16.0,
                            &shortcut,
                            10.0,
                            if enabled {
                                rgb(145, 151, 160)
                            } else {
                                rgb(86, 90, 98)
                            },
                        );
                    }
                    if response.clicked && enabled {
                        self.trigger_menu_action(invocations, action.id.as_str());
                        self.open_menu = None;
                    }
                    y += 24.0;
                }
            }
        }
    }

    fn menu_overlay_entry(kind: EditorMenuKind, viewport: Rect) -> OverlayEntry {
        let anchor = menu_anchor(kind);
        let size = menu_size(kind);
        let rect = place_popover(
            PopoverRequest {
                anchor,
                size,
                placement: PopoverPlacement::Below,
                offset: 2.0,
                fit_viewport: true,
            },
            viewport,
        );
        OverlayEntry::new(
            OverlayId::from_raw(10_000 + kind.raw()),
            OverlayKind::Menu,
            rect,
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape)
    }

    fn menu_model(&self, kind: EditorMenuKind) -> Menu {
        match kind {
            EditorMenuKind::File => menu([
                menu_action(
                    ACTION_PALETTE,
                    "New Scene",
                    Some(ctrl_char("n")),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_PALETTE,
                    "Open Project...",
                    Some(ctrl_char("o")),
                    None,
                    true,
                ),
                menu_action(ACTION_SAVE, "Save Scene", Some(ctrl_char("s")), None, true),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Import Asset...", None, None, true),
                menu_action(
                    ACTION_BUILD,
                    "Export Build",
                    Some(ctrl_char("b")),
                    None,
                    true,
                ),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Quit", None, None, false),
            ]),
            EditorMenuKind::Edit => menu([
                menu_action(ACTION_PALETTE, "Undo", Some(ctrl_char("z")), None, false),
                menu_action(ACTION_PALETTE, "Redo", Some(ctrl_char("y")), None, false),
                MenuItem::Separator,
                menu_action(
                    ACTION_PALETTE,
                    "Duplicate",
                    Some(ctrl_char("d")),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_PALETTE,
                    "Delete",
                    Some(shortcut(Key::Delete)),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_PALETTE,
                    "Preferences",
                    Some(ctrl_char(",")),
                    None,
                    true,
                ),
            ]),
            EditorMenuKind::View => menu([
                menu_action(ACTION_PALETTE, "Perspective View", None, Some(true), true),
                menu_action(
                    ACTION_PALETTE,
                    "Frame Selected",
                    Some(shortcut(Key::Character("f".to_owned()))),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_GRID,
                    "Show Grid",
                    Some(shortcut(Key::Character("g".to_owned()))),
                    Some(self.grid_visible),
                    true,
                ),
                menu_action(ACTION_PALETTE, "Show Overlays", None, Some(true), true),
                menu_action(ACTION_PALETTE, "Reset View", None, None, true),
            ]),
            EditorMenuKind::Project => menu([
                menu_action(
                    ACTION_PLAY,
                    "Play",
                    Some(shortcut(Key::Function(5))),
                    None,
                    true,
                ),
                menu_action(ACTION_STOP, "Stop", None, None, true),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Project Settings...", None, None, true),
            ]),
            EditorMenuKind::Build => menu([
                menu_action(
                    ACTION_BUILD,
                    "Build Project",
                    Some(ctrl_char("b")),
                    None,
                    true,
                ),
                menu_action(ACTION_BUILD, "Package Windows x64", None, None, true),
                menu_action(ACTION_PALETTE, "Run Profiler", None, None, false),
            ]),
            EditorMenuKind::Window => menu([
                menu_action(
                    ACTION_PALETTE,
                    "Command Palette",
                    Some(ctrl_char("p")),
                    None,
                    true,
                ),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Scene Graph", None, Some(true), true),
                menu_action(ACTION_PALETTE, "Inspector", None, Some(true), true),
                menu_action(ACTION_PALETTE, "Asset Browser", None, Some(true), true),
                menu_action(ACTION_PALETTE, "Console", None, Some(true), true),
            ]),
            EditorMenuKind::Help => menu([
                menu_action(
                    ACTION_PALETTE,
                    "Online Docs",
                    Some(shortcut(Key::Function(1))),
                    None,
                    true,
                ),
                menu_action(ACTION_PALETTE, "Keyboard Shortcuts", None, None, true),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "About Kinetik Forge", None, None, true),
            ]),
        }
    }

    fn trigger_menu_action(&mut self, invocations: &mut Vec<EditorInvocation>, action_id: &str) {
        let action_id = match action_id {
            ACTION_SAVE => ACTION_SAVE,
            ACTION_PLAY => ACTION_PLAY,
            ACTION_STOP => ACTION_STOP,
            ACTION_GRID => ACTION_GRID,
            ACTION_BUILD => ACTION_BUILD,
            _ => ACTION_PALETTE,
        };
        self.trigger(invocations, action_id, ActionSource::Menu);
    }

    fn tool_bar(
        &mut self,
        ui: &mut Ui<'_>,
        viewport: Rect,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        let mut x = 10.0;
        for (tool, icon, label, action) in [
            (
                EditorTool::Select,
                ICON_CURSOR,
                "Select",
                ACTION_TOOL_SELECT,
            ),
            (EditorTool::Move, ICON_MOVE, "Move", ACTION_TOOL_MOVE),
            (
                EditorTool::Rotate,
                ICON_ROTATE,
                "Rotate",
                ACTION_TOOL_ROTATE,
            ),
            (
                EditorTool::Scale,
                ICON_TRANSFORM,
                "Scale",
                ACTION_TOOL_SCALE,
            ),
        ] {
            let button = Rect::new(x, 33.0, 28.0, 26.0);
            let response = ui.image_icon_button_value(
                ("editor.tool", action),
                button,
                icon,
                label,
                &mut self.selected_tool,
                tool,
                false,
            );
            if response.clicked {
                let status = match tool {
                    EditorTool::Select => "Select tool active",
                    EditorTool::Move => "Move tool active",
                    EditorTool::Rotate => "Rotate tool active",
                    EditorTool::Scale => "Scale tool active",
                };
                status.clone_into(&mut self.status);
                invocations.push(EditorInvocation {
                    action_id: action,
                    source: ActionSource::Button,
                });
            }
            x += 32.0;
        }

        rect(
            ui,
            Rect::new(x + 4.0, 34.0, 1.0, 24.0),
            rgb(57, 60, 66),
            None,
        );
        x += 14.0;
        for (icon, label, action) in [
            (ICON_GRID, "Toggle grid", ACTION_GRID),
            (ICON_CROSSHAIR, "Frame selected", ACTION_PALETTE),
            (ICON_RESET, "Reset view", ACTION_PALETTE),
        ] {
            let response = ui.image_icon_button(
                ("editor.viewport-tool", action, icon.raw()),
                Rect::new(x, 33.0, 28.0, 26.0),
                icon,
                label,
                false,
            );
            if response.clicked {
                self.trigger(invocations, action, ActionSource::Button);
            }
            x += 32.0;
        }

        let right = viewport.max_x() - 196.0;
        for (index, (icon, label, action)) in [
            (ICON_PLAY, "Play", ACTION_PLAY),
            (ICON_PAUSE, "Pause", ACTION_PLAY),
            (ICON_STOP, "Stop", ACTION_STOP),
            (ICON_ROCKET, "Build", ACTION_BUILD),
            (ICON_DOWNLOAD, "Export", ACTION_BUILD),
        ]
        .into_iter()
        .enumerate()
        {
            let response = ui.image_icon_button(
                ("editor.run", action, index),
                Rect::new(right + index as f32 * 36.0, 33.0, 30.0, 26.0),
                icon,
                label,
                false,
            );
            if response.clicked {
                self.trigger(invocations, action, ActionSource::Button);
            }
        }
    }

    fn workspace(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
        let bottom_bar = 24.0;
        let bounds = Rect::new(
            4.0,
            68.0,
            (viewport.width - 8.0).max(1.0),
            (viewport.height - 68.0 - bottom_bar - 4.0).max(1.0),
        );
        let frame_layouts = solve_dock_layout(&self.dock, bounds);
        for layout in frame_layouts {
            self.editor_frame(ui, layout.frame, layout.rect.inset(2.0));
        }

        for splitter in solve_dock_splitters(&self.dock, bounds, 4.0) {
            let response = ui.draggable(
                ("editor.splitter", splitter.path.clone()),
                splitter.rect,
                false,
            );
            if response.dragged {
                self.dock
                    .resize_split(&splitter.path, bounds, response.drag_delta);
            }
            let color = if response.state.hovered || response.state.active {
                rgb(70, 116, 190)
            } else {
                rgb(38, 40, 45)
            };
            rect(ui, splitter.rect, color, None);
        }
    }

    fn editor_frame(&mut self, ui: &mut Ui<'_>, frame_id: FrameId, frame_rect: Rect) {
        if frame_rect.width <= 1.0 || frame_rect.height <= 1.0 {
            return;
        }
        rect(ui, frame_rect, rgb(28, 29, 32), Some(rgb(57, 59, 65)));

        let Some(frame_snapshot) = self.dock.frame(frame_id).cloned() else {
            return;
        };
        let tab_height = 26.0;
        let mut tab_x = frame_rect.x + 1.0;
        for tab in frame_tabs(&frame_snapshot) {
            let width = (tab.title.len() as f32 * 7.0 + 42.0).clamp(82.0, 146.0);
            let tab_rect = Rect::new(tab_x, frame_rect.y + 1.0, width, tab_height);
            let response = ui.tab_button(
                ("editor.frame-tab", frame_id.raw(), tab.panel.raw()),
                tab_rect,
                tab.title,
                tab.active,
                false,
            );
            if response.clicked {
                self.dock.select_panel(frame_id, tab.panel);
            }
            tab_x += width + 1.0;
        }
        rect(
            ui,
            Rect::new(
                frame_rect.x + 1.0,
                frame_rect.y + tab_height + 1.0,
                (frame_rect.width - 2.0).max(0.0),
                1.0,
            ),
            rgb(48, 50, 56),
            None,
        );

        let body = Rect::new(
            frame_rect.x + 1.0,
            frame_rect.y + tab_height + 2.0,
            (frame_rect.width - 2.0).max(0.0),
            (frame_rect.height - tab_height - 3.0).max(0.0),
        );
        let active_panel = self
            .dock
            .frame(frame_id)
            .and_then(Frame::active_panel)
            .map(|panel| panel.id);
        ui.clip_rect(
            ("editor.frame-body", frame_id.raw()),
            body,
            |ui| match active_panel {
                Some(PANEL_SCENE) => self.scene_graph(ui, body),
                Some(PANEL_ASSETS) => self.assets_browser(ui, body),
                Some(PANEL_VIEWPORT) => self.viewport_panel(ui, body),
                Some(PANEL_CONSOLE) => Self::console_panel(ui, body),
                Some(PANEL_JOBS) => Self::jobs_panel(ui, body),
                Some(PANEL_INSPECTOR) => self.inspector(ui, body),
                _ => {}
            },
        );
    }

    fn scene_graph(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 24.0);
        let add = ui.image_icon_button(
            "editor.scene.add",
            header.with_width(28.0),
            ICON_PLUS,
            "Add node",
            false,
        );
        if add.clicked {
            "Create node requested".clone_into(&mut self.status);
        }
        text(
            ui,
            header.x + 36.0,
            header.y + 17.0,
            "Scene",
            13.0,
            rgb(222, 225, 230),
        );
        icon(ui, header.right_strip(24.0), ICON_DOTS, DENSE_ICON_SIZE);

        let rows = scene_model().visible_rows(&self.scene_expansion);
        let layout = TreeLayout::new(22.0, 15.0);
        let content_height = layout.content_height(rows.len()) + 8.0;
        let scroll = Rect::new(
            body.x + 6.0,
            body.y + 38.0,
            body.width - 12.0,
            body.height - 44.0,
        );
        ui.scroll_area(
            "editor.scene.scroll",
            scroll,
            Size::new(scroll.width, content_height.max(scroll.height)),
            false,
            |ui, _| {
                for row_rect in layout.visible_row_rects(scroll, &rows, 0.0, 2) {
                    let row = row_rect.row;
                    let response = ui.list_row_value(
                        ("editor.scene.row", row.id.raw()),
                        row_rect.rect,
                        "",
                        &mut self.selected_node,
                        row.id,
                        false,
                    );
                    if response.clicked {
                        self.status = format!("Selected {}", scene_label(row.id));
                    }
                    let twisty = Rect::new(
                        row_rect.content_rect.x + 3.0,
                        row_rect.rect.y + 5.0,
                        12.0,
                        12.0,
                    );
                    if row.has_children {
                        let twist =
                            ui.pressable(("editor.scene.expand", row.id.raw()), twisty, false);
                        if twist.clicked {
                            self.scene_expansion.toggle(row.id);
                        }
                        text(
                            ui,
                            twisty.x + 2.0,
                            twisty.y + 10.0,
                            if row.expanded { "v" } else { ">" },
                            11.0,
                            rgb(176, 181, 188),
                        );
                    }
                    icon(
                        ui,
                        Rect::new(
                            row_rect.content_rect.x + 17.0,
                            row_rect.rect.y + 3.0,
                            18.0,
                            18.0,
                        ),
                        scene_icon(row.id),
                        DENSE_ICON_SIZE,
                    );
                    text(
                        ui,
                        row_rect.content_rect.x + 38.0,
                        row_rect.rect.y + 15.0,
                        scene_label(row.id),
                        12.0,
                        rgb(218, 221, 226),
                    );
                }
            },
        );
    }

    fn assets_browser(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let search = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 26.0);
        ui.search_field(
            "editor.assets.search",
            search,
            &mut self.asset_filter,
            false,
        );
        icon(
            ui,
            Rect::new(search.x + 5.0, search.y + 5.0, 18.0, 18.0),
            ICON_SEARCH,
            DENSE_ICON_SIZE,
        );

        let grid_bounds = Rect::new(
            body.x + 8.0,
            body.y + 44.0,
            body.width - 16.0,
            body.height - 50.0,
        );
        let layout = GridLayout {
            columns: GridColumns::Adaptive { min_width: 92.0 },
            item_size: Size::new(88.0, 74.0),
            gap: 6.0,
        };
        let content_rows = (ASSETS.len() as f32 / layout.column_count(grid_bounds) as f32).ceil();
        ui.scroll_area(
            "editor.assets.scroll",
            grid_bounds,
            Size::new(
                grid_bounds.width,
                (content_rows * 80.0).max(grid_bounds.height),
            ),
            false,
            |ui, _| {
                for item in layout.item_rects(grid_bounds, ASSETS.len(), 0..ASSETS.len()) {
                    let asset = &ASSETS[item.index];
                    let response = ui.selectable_value(
                        ("editor.asset", item.index),
                        item.rect,
                        &mut self.selected_asset,
                        item.index,
                        false,
                    );
                    let selected = response.state.selected;
                    rect(
                        ui,
                        item.rect,
                        if selected {
                            rgb(38, 74, 122)
                        } else if response.state.hovered {
                            rgb(38, 40, 44)
                        } else {
                            rgb(31, 32, 35)
                        },
                        Some(if selected {
                            rgb(82, 140, 220)
                        } else {
                            rgb(53, 55, 61)
                        }),
                    );
                    if response.clicked {
                        self.status = format!("Asset selected: {}", asset.name);
                    }
                    icon(
                        ui,
                        Rect::new(item.rect.x + 8.0, item.rect.y + 8.0, 24.0, 24.0),
                        asset.icon,
                        ASSET_ICON_SIZE,
                    );
                    text(
                        ui,
                        item.rect.x + 8.0,
                        item.rect.y + 48.0,
                        asset.name,
                        11.0,
                        rgb(224, 226, 230),
                    );
                    text(
                        ui,
                        item.rect.x + 8.0,
                        item.rect.y + 64.0,
                        asset.kind,
                        9.0,
                        rgb(144, 149, 156),
                    );
                }
            },
        );
    }

    fn viewport_panel(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(15, 16, 18), None);
        let toolbar = Rect::new(body.x, body.y, body.width, 28.0);
        rect(ui, toolbar, rgb(25, 26, 29), Some(rgb(44, 47, 53)));
        text(
            ui,
            toolbar.x + 10.0,
            toolbar.y + 18.0,
            "Perspective",
            12.0,
            rgb(220, 224, 230),
        );
        text(
            ui,
            toolbar.x + 94.0,
            toolbar.y + 18.0,
            "Lit",
            12.0,
            rgb(151, 158, 166),
        );
        text(
            ui,
            toolbar.x + 136.0,
            toolbar.y + 18.0,
            "1280 x 720",
            11.0,
            rgb(151, 158, 166),
        );

        let surface_bounds = Rect::new(
            body.x + 8.0,
            body.y + 36.0,
            (body.width - 16.0).max(1.0),
            (body.height - 66.0).max(1.0),
        );
        let drag = ui.draggable("editor.viewport.surface", surface_bounds, false);
        if drag.dragged {
            self.viewport_pan_zoom.pan_by(drag.drag_delta);
        }
        if drag.state.hovered {
            let wheel = ui.input().pointer.wheel_delta.y;
            if wheel.abs() > f32::EPSILON {
                let current = self.viewport_pan_zoom.content_zoom();
                let next = (current + (-wheel * 0.001)).clamp(0.25, 2.5);
                self.viewport_pan_zoom.set_zoom(next);
                self.status = format!("Viewport zoom {:.0}%", next * 100.0);
            }
        }
        let surface = ViewportSurface {
            texture: VIEWPORT_TEXTURE,
            source_size: VIEWPORT_SIZE,
            bounds: surface_bounds,
            pan_zoom: self.viewport_pan_zoom,
        };
        let mut guides = Vec::new();
        if self.grid_visible {
            guides.extend([
                Guide::Vertical(VIEWPORT_SIZE.width * 0.25),
                Guide::Vertical(VIEWPORT_SIZE.width * 0.5),
                Guide::Vertical(VIEWPORT_SIZE.width * 0.75),
                Guide::Horizontal(VIEWPORT_SIZE.height * 0.5),
            ]);
        }
        let composition = ViewportComposition {
            surface,
            guides,
            crosshair: None,
            clip: ClipId::from_raw(8_001),
        };
        ui.extend(composition.primitives());
        self.viewport_overlays(ui, surface, surface_bounds);

        let timeline = Rect::new(body.x + 10.0, body.max_y() - 24.0, body.width - 20.0, 14.0);
        ui.slider(
            "editor.timeline",
            timeline,
            &mut self.timeline,
            0.0..=1.0,
            false,
        );
    }

    fn viewport_overlays(&self, ui: &mut Ui<'_>, surface: ViewportSurface, bounds: Rect) {
        if self.grid_visible {
            let content = surface.content_rect();
            let step = (content.width / 8.0).max(1.0);
            for i in 1..8 {
                let x = content.x + step * i as f32;
                line(
                    ui,
                    Point::new(x, content.y),
                    Point::new(x, content.max_y()),
                    rgba(170, 190, 220, 0.20),
                    1.0,
                );
            }
            for i in 1..5 {
                let y = content.y + (content.height / 5.0) * i as f32;
                line(
                    ui,
                    Point::new(content.x, y),
                    Point::new(content.max_x(), y),
                    rgba(170, 190, 220, 0.18),
                    1.0,
                );
            }
        }

        if let Some(selection) =
            surface.content_rect_to_screen(Rect::new(720.0, 210.0, 210.0, 280.0))
        {
            rect(
                ui,
                selection,
                rgba(78, 142, 245, 0.12),
                Some(rgb(82, 148, 245)),
            );
            line(
                ui,
                Point::new(selection.x + selection.width * 0.5, selection.y),
                Point::new(selection.x + selection.width * 0.5, selection.max_y()),
                rgba(120, 210, 255, 0.75),
                1.0,
            );
            line(
                ui,
                Point::new(selection.x, selection.y + selection.height * 0.5),
                Point::new(selection.max_x(), selection.y + selection.height * 0.5),
                rgba(120, 210, 255, 0.75),
                1.0,
            );
        }
        let gizmo = Rect::new(bounds.x + 18.0, bounds.max_y() - 72.0, 62.0, 52.0);
        line(
            ui,
            Point::new(gizmo.x + 10.0, gizmo.max_y() - 10.0),
            Point::new(gizmo.x + 48.0, gizmo.max_y() - 10.0),
            rgb(236, 82, 82),
            2.0,
        );
        line(
            ui,
            Point::new(gizmo.x + 10.0, gizmo.max_y() - 10.0),
            Point::new(gizmo.x + 10.0, gizmo.y + 8.0),
            rgb(78, 205, 112),
            2.0,
        );
        line(
            ui,
            Point::new(gizmo.x + 10.0, gizmo.max_y() - 10.0),
            Point::new(gizmo.x + 42.0, gizmo.y + 20.0),
            rgb(90, 140, 245),
            2.0,
        );
        text(
            ui,
            bounds.x + 16.0,
            bounds.y + 24.0,
            "CameraPreview",
            11.0,
            rgb(238, 240, 244),
        );
        text(
            ui,
            bounds.max_x() - 160.0,
            bounds.y + 24.0,
            "Frame 124 / 300",
            11.0,
            rgb(238, 240, 244),
        );
    }

    fn inspector(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 34.0);
        rect(ui, header, rgb(37, 39, 43), Some(rgb(55, 58, 64)));
        icon(
            ui,
            Rect::new(header.x + 7.0, header.y + 7.0, 20.0, 20.0),
            scene_icon(self.selected_node),
            DENSE_ICON_SIZE,
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 22.0,
            scene_label(self.selected_node),
            13.0,
            rgb(231, 233, 237),
        );
        icon(
            ui,
            Rect::new(header.max_x() - 27.0, header.y + 7.0, 20.0, 20.0),
            ICON_GEAR,
            DENSE_ICON_SIZE,
        );

        let rows = inspector_rows();
        let grid = Rect::new(
            body.x + 8.0,
            body.y + 52.0,
            body.width - 16.0,
            body.height - 60.0,
        );
        let layout =
            PropertyGridLayout::new(24.0, 26.0, inspector_label_width(grid.width), 6.0, 12.0);
        ui.scroll_area(
            "editor.inspector.scroll",
            grid,
            Size::new(grid.width, layout.content_height(&rows).max(grid.height)),
            false,
            |ui, _| {
                for row in layout.visible_row_rects(grid, &rows, 0.0, 2) {
                    match row.kind {
                        kinetik_ui::widgets::PropertyGridRowKind::Section => {
                            rect(ui, row.rect, rgb(31, 33, 36), Some(rgb(46, 49, 55)));
                            text(
                                ui,
                                row.label_rect.x + 8.0,
                                row.label_rect.y + 17.0,
                                &rows[row.index].label,
                                12.0,
                                rgb(205, 209, 216),
                            );
                        }
                        kinetik_ui::widgets::PropertyGridRowKind::Property { .. } => {
                            rect(ui, row.rect, rgb(24, 25, 27), Some(rgb(38, 40, 45)));
                            text(
                                ui,
                                row.label_rect.x + 6.0,
                                row.label_rect.y + 16.0,
                                &rows[row.index].label,
                                11.0,
                                rgb(154, 160, 168),
                            );
                            self.inspector_value(ui, row.id, row.value_rect.inset(2.0));
                        }
                    }
                }
            },
        );
    }

    fn inspector_value(&mut self, ui: &mut Ui<'_>, id: ItemId, rect_value: Rect) {
        match id.raw() {
            2 => {
                ui.numeric_input("editor.inspector.pos-x", rect_value, &mut self.pos_x, false);
            }
            3 => {
                ui.numeric_input("editor.inspector.pos-y", rect_value, &mut self.pos_y, false);
            }
            4 => {
                ui.numeric_input("editor.inspector.pos-z", rect_value, &mut self.pos_z, false);
            }
            5 => {
                ui.numeric_input("editor.inspector.scale", rect_value, &mut self.scale, false);
            }
            7 => {
                ui.slider(
                    "editor.inspector.exposure",
                    rect_value,
                    &mut self.exposure,
                    0.0..=1.0,
                    false,
                );
            }
            8 => {
                ui.slider(
                    "editor.inspector.roughness",
                    rect_value,
                    &mut self.roughness,
                    0.0..=1.0,
                    false,
                );
            }
            11 => {
                ui.toggle_value(
                    "editor.inspector.snap",
                    Rect::new(rect_value.x, rect_value.y + 2.0, 42.0, 18.0),
                    &mut self.snap_enabled,
                    false,
                );
            }
            13 => {
                ui.numeric_input("editor.inspector.mass", rect_value, &mut self.mass, false);
            }
            _ => {
                text(
                    ui,
                    rect_value.x + 4.0,
                    rect_value.y + 15.0,
                    inspector_value_label(id),
                    11.0,
                    rgb(218, 221, 226),
                );
            }
        }
    }

    fn console_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: item_id(1),
                    header: "Time".to_owned(),
                    width: 74.0,
                },
                TableColumn {
                    id: item_id(2),
                    header: "Level".to_owned(),
                    width: 74.0,
                },
                TableColumn {
                    id: item_id(3),
                    header: "Message".to_owned(),
                    width: (body.width - 160.0).max(120.0),
                },
            ],
            header_height: 24.0,
            row_height: 24.0,
            sort: None,
        };
        let bounds = body.inset(8.0);
        for header in table.header_rects(bounds) {
            rect(ui, header.rect, rgb(31, 33, 36), Some(rgb(48, 50, 56)));
            text(
                ui,
                header.rect.x + 8.0,
                header.rect.y + 16.0,
                &table.columns[header.index].header,
                11.0,
                rgb(178, 183, 190),
            );
        }
        for cell in table.visible_body_cells(bounds, LOGS.len(), 0.0, 1) {
            let log = &LOGS[cell.row];
            let value = match cell.column {
                0 => log.time,
                1 => log.level,
                _ => log.message,
            };
            rect(ui, cell.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                cell.rect.x + 8.0,
                cell.rect.y + 16.0,
                value,
                11.0,
                log_color(log.level),
            );
        }
    }

    fn jobs_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let rows = [
            ("Bake global illumination", "Queued", 0.14),
            ("Build navigation mesh", "Ready", 0.62),
            ("Import character rig", "Running", 0.47),
            ("Package Windows x64", "Idle", 0.0),
        ];
        let layout = ListLayout::new(28.0);
        for item in layout.row_rects(body.inset(8.0), rows.len(), 0..rows.len()) {
            let (name, state, progress) = rows[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 18.0,
                name,
                11.0,
                rgb(222, 225, 230),
            );
            text(
                ui,
                item.rect.max_x() - 96.0,
                item.rect.y + 18.0,
                state,
                11.0,
                rgb(154, 160, 168),
            );
            let progress_rect = Rect::new(item.rect.max_x() - 220.0, item.rect.y + 10.0, 96.0, 6.0);
            rect(ui, progress_rect, rgb(39, 42, 47), Some(rgb(56, 59, 65)));
            rect(
                ui,
                Rect::new(
                    progress_rect.x,
                    progress_rect.y,
                    progress_rect.width * progress,
                    progress_rect.height,
                ),
                rgb(69, 123, 220),
                None,
            );
        }
    }

    fn status_bar(&self, ui: &mut Ui<'_>, viewport: Rect, action_count: u32) {
        let bar = Rect::new(0.0, viewport.max_y() - 24.0, viewport.width, 24.0);
        rect(ui, bar, rgb(27, 29, 32), Some(rgb(52, 55, 62)));
        text(
            ui,
            10.0,
            bar.y + 16.0,
            &self.status,
            11.0,
            rgb(198, 203, 211),
        );
        text(
            ui,
            viewport.max_x() - 330.0,
            bar.y + 16.0,
            &format!("Actions: {action_count}"),
            11.0,
            rgb(154, 160, 168),
        );
        text(
            ui,
            viewport.max_x() - 210.0,
            bar.y + 16.0,
            if self.snap_enabled {
                "Snap 1m"
            } else {
                "Snap off"
            },
            11.0,
            rgb(154, 160, 168),
        );
        text(
            ui,
            viewport.max_x() - 92.0,
            bar.y + 16.0,
            "Vello / winit",
            11.0,
            rgb(154, 160, 168),
        );
    }
}

/// Registers editor media and icon resources.
pub fn register_resources(resources: &mut RenderResources) {
    if let Some(snapshot) = RenderImage::rgba8(
        1280,
        720,
        include_bytes!("../assets/viewport-1280x720.rgba").to_vec(),
    ) {
        resources.register_texture(TextureResource {
            id: VIEWPORT_TEXTURE,
            size: VIEWPORT_SIZE,
            sampling: RenderImageSampling::Smooth,
            snapshot: Some(snapshot),
        });
    }
    if let Some(atlas) = icon_atlas_image() {
        resources.register_image(ImageResource {
            id: ICON_ATLAS,
            size: Size::new(
                (ICON_ATLAS_COLUMNS * ICON_ATLAS_CELL_SIZE) as f32,
                (ICON_ATLAS_ROWS * ICON_ATLAS_CELL_SIZE) as f32,
            ),
            sampling: RenderImageSampling::UiIcon,
            pixels: Some(atlas),
            atlas_region: None,
        });
    }
    for (index, (id, _)) in ICON_ASSETS.iter().enumerate() {
        let column = index as u32 % ICON_ATLAS_COLUMNS;
        let row = index as u32 / ICON_ATLAS_COLUMNS;
        resources.register_image(ImageResource {
            id: *id,
            size: Size::new(ICON_SIZE as f32, ICON_SIZE as f32),
            sampling: RenderImageSampling::UiIcon,
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: ICON_ATLAS,
                source: Rect::new(
                    (column * ICON_ATLAS_CELL_SIZE + ICON_ATLAS_PADDING) as f32,
                    (row * ICON_ATLAS_CELL_SIZE + ICON_ATLAS_PADDING) as f32,
                    ICON_SIZE as f32,
                    ICON_SIZE as f32,
                ),
            }),
        });
    }
}

fn icon_atlas_image() -> Option<RenderImage> {
    let width = ICON_ATLAS_COLUMNS * ICON_ATLAS_CELL_SIZE;
    let height = ICON_ATLAS_ROWS * ICON_ATLAS_CELL_SIZE;
    let mut data = vec![0; (width * height * 4) as usize];
    for (index, (_, bytes)) in ICON_ASSETS.iter().enumerate() {
        if bytes.len() != (ICON_SIZE * ICON_SIZE * 4) as usize {
            return None;
        }
        let column = index as u32 % ICON_ATLAS_COLUMNS;
        let row = index as u32 / ICON_ATLAS_COLUMNS;
        let x0 = column * ICON_ATLAS_CELL_SIZE;
        let y0 = row * ICON_ATLAS_CELL_SIZE;
        for y in 0..ICON_ATLAS_CELL_SIZE {
            let source_y = y.saturating_sub(ICON_ATLAS_PADDING).min(ICON_SIZE - 1);
            for x in 0..ICON_ATLAS_CELL_SIZE {
                let source_x = x.saturating_sub(ICON_ATLAS_PADDING).min(ICON_SIZE - 1);
                let source_start = ((source_y * ICON_SIZE + source_x) * 4) as usize;
                let dest_start = (((y0 + y) * width + x0 + x) * 4) as usize;
                data[dest_start..dest_start + 4]
                    .copy_from_slice(&bytes[source_start..source_start + 4]);
            }
        }
    }
    RenderImage::rgba8(width, height, data)
}

fn menu_header_rects() -> [(EditorMenuKind, &'static str, Rect); 7] {
    let specs = [
        (EditorMenuKind::File, "File", 44.0),
        (EditorMenuKind::Edit, "Edit", 44.0),
        (EditorMenuKind::View, "View", 52.0),
        (EditorMenuKind::Project, "Project", 64.0),
        (EditorMenuKind::Build, "Build", 54.0),
        (EditorMenuKind::Window, "Window", 68.0),
        (EditorMenuKind::Help, "Help", 48.0),
    ];
    let mut x = 126.0;
    specs.map(|(kind, label, width)| {
        let rect = Rect::new(x, 3.0, width, 22.0);
        x += width + 4.0;
        (kind, label, rect)
    })
}

fn menu_bar_rect() -> Rect {
    Rect::new(0.0, 0.0, 760.0, 28.0)
}

fn menu_anchor(kind: EditorMenuKind) -> Rect {
    menu_header_rects()
        .into_iter()
        .find_map(|(candidate, _, rect)| (candidate == kind).then_some(rect))
        .unwrap_or(Rect::new(126.0, 3.0, 44.0, 22.0))
}

fn menu_size(kind: EditorMenuKind) -> Size {
    match kind {
        EditorMenuKind::File => Size::new(238.0, 188.0),
        EditorMenuKind::Edit => Size::new(226.0, 154.0),
        EditorMenuKind::View => Size::new(224.0, 136.0),
        EditorMenuKind::Project => Size::new(224.0, 106.0),
        EditorMenuKind::Build => Size::new(224.0, 82.0),
        EditorMenuKind::Window => Size::new(232.0, 154.0),
        EditorMenuKind::Help => Size::new(230.0, 88.0),
    }
}

fn menu<const N: usize>(items: [MenuItem; N]) -> Menu {
    let mut menu = Menu::new();
    for item in items {
        menu.push(item);
    }
    menu
}

fn menu_action(
    action_id: &'static str,
    label: &'static str,
    shortcut: Option<Shortcut>,
    checked: Option<bool>,
    enabled: bool,
) -> MenuItem {
    let mut action = ActionDescriptor::new(action_id, label);
    action.shortcut = shortcut;
    action.state.checked = checked;
    action.state.enabled = enabled;
    MenuItem::Action(action)
}

fn ctrl_char(character: &str) -> Shortcut {
    shortcut_with_modifiers(
        Key::Character(character.to_owned()),
        Modifiers::new(false, true, false, false),
    )
}

fn shortcut(key: Key) -> Shortcut {
    shortcut_with_modifiers(key, Modifiers::default())
}

fn shortcut_with_modifiers(key: Key, modifiers: Modifiers) -> Shortcut {
    Shortcut::new(modifiers, key)
}

fn shortcut_label(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    if shortcut.modifiers.ctrl {
        parts.push("Ctrl".to_owned());
    }
    if shortcut.modifiers.shift {
        parts.push("Shift".to_owned());
    }
    if shortcut.modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if shortcut.modifiers.super_key {
        parts.push("Super".to_owned());
    }
    parts.push(key_label(&shortcut.key));
    parts.join("+")
}

fn key_label(key: &Key) -> String {
    match key {
        Key::Character(character) => character.to_uppercase(),
        Key::Function(number) => format!("F{number}"),
        Key::Delete => "Del".to_owned(),
        Key::Escape => "Esc".to_owned(),
        Key::Enter => "Enter".to_owned(),
        Key::Tab => "Tab".to_owned(),
        Key::Space => "Space".to_owned(),
        Key::ArrowLeft => "Left".to_owned(),
        Key::ArrowRight => "Right".to_owned(),
        Key::ArrowUp => "Up".to_owned(),
        Key::ArrowDown => "Down".to_owned(),
        Key::Backspace => "Backspace".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::End => "End".to_owned(),
        Key::PageUp => "PageUp".to_owned(),
        Key::PageDown => "PageDown".to_owned(),
        Key::Insert => "Insert".to_owned(),
        Key::Unidentified => "?".to_owned(),
    }
}

fn default_dock() -> DockArea {
    DockArea::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.19,
        min_first: 220.0,
        min_second: 520.0,
        first: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.56,
            min_first: 180.0,
            min_second: 160.0,
            first: Box::new(DockNode::Frame(Frame::new(
                FRAME_SCENE,
                vec![Panel::new(PANEL_SCENE, "Scene")],
            ))),
            second: Box::new(DockNode::Frame(Frame::new(
                FRAME_ASSETS,
                vec![Panel::new(PANEL_ASSETS, "Assets")],
            ))),
        }),
        second: Box::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.76,
            min_first: 300.0,
            min_second: 180.0,
            first: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.74,
                min_first: 300.0,
                min_second: 140.0,
                first: Box::new(DockNode::Frame(Frame::new(
                    FRAME_VIEWPORT,
                    vec![Panel::new(PANEL_VIEWPORT, "Viewport")],
                ))),
                second: Box::new(DockNode::Frame(Frame::new(
                    FRAME_BOTTOM,
                    vec![
                        Panel::new(PANEL_CONSOLE, "Console"),
                        Panel::new(PANEL_JOBS, "Jobs"),
                    ],
                ))),
            }),
            second: Box::new(DockNode::Frame(Frame::new(
                FRAME_INSPECTOR,
                vec![Panel::new(PANEL_INSPECTOR, "Inspector")],
            ))),
        }),
    })
}

fn scene_model() -> TreeModel {
    TreeModel::new(vec![
        tree_item(1, None, true),
        tree_item(2, Some(1), true),
        tree_item(3, Some(2), false),
        tree_item(4, Some(2), false),
        tree_item(5, Some(2), false),
        tree_item(6, Some(1), true),
        tree_item(7, Some(6), false),
        tree_item(8, Some(6), false),
        tree_item(9, Some(1), false),
        tree_item(10, Some(1), false),
        tree_item(11, Some(1), false),
    ])
}

fn inspector_rows() -> Vec<PropertyGridRow> {
    vec![
        PropertyGridRow::section(item_id(1), "Transform"),
        PropertyGridRow::property(item_id(2), "Position X", 0),
        PropertyGridRow::property(item_id(3), "Position Y", 0),
        PropertyGridRow::property(item_id(4), "Position Z", 0),
        PropertyGridRow::property(item_id(5), "Uniform Scale", 0),
        PropertyGridRow::section(item_id(6), "Rendering"),
        PropertyGridRow::property(item_id(7), "Exposure", 0),
        PropertyGridRow::property(item_id(8), "Roughness", 0),
        PropertyGridRow::property(item_id(9), "Material", 0),
        PropertyGridRow::section(item_id(10), "Editor"),
        PropertyGridRow::property(item_id(11), "Snap", 0),
        PropertyGridRow::section(item_id(12), "Physics"),
        PropertyGridRow::property(item_id(13), "Mass", 0),
        PropertyGridRow::property(item_id(14), "Collider", 0),
        PropertyGridRow::property(item_id(15), "Script", 0),
    ]
}

fn inspector_label_width(grid_width: f32) -> f32 {
    if !grid_width.is_finite() {
        return 72.0;
    }

    (grid_width * 0.42).clamp(52.0, 96.0)
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::items_after_test_module)]
mod tests {
    use super::{
        EditorShowcase, EditorTool, ICON_ASSETS, ICON_ATLAS, ICON_ATLAS_CELL_SIZE,
        ICON_ATLAS_PADDING, ICON_CROSSHAIR, ICON_SIZE, icon_atlas_image, inspector_label_width,
        register_resources,
    };
    use kinetik_ui::core::{
        FrameContext, PhysicalSize, Point, PointerButtonState, PointerInput, Primitive, Rect,
        ScaleFactor, Size, TimeInfo, UiInput, UiMemory, ViewportInfo, default_dark_theme,
    };
    use kinetik_ui::render::RenderResources;
    use kinetik_ui::widgets::Ui;

    #[test]
    fn inspector_label_width_preserves_value_space_at_narrow_widths() {
        assert_eq!(inspector_label_width(120.0), 52.0);
        assert!((inspector_label_width(180.0) - 75.6).abs() < f32::EPSILON);
        assert_eq!(inspector_label_width(400.0), 96.0);
        assert_eq!(inspector_label_width(f32::NAN), 72.0);
    }

    #[test]
    fn inspector_snap_toggle_updates_status_same_frame() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(1290.0, 410.0, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(1290.0, 410.0, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert!(!editor.snap_enabled);
        assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Snap off")
        }));
    }

    #[test]
    fn toolbar_tool_selection_updates_status_same_frame() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(14.0, 40.0, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(14.0, 40.0, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert_eq!(editor.selected_tool, EditorTool::Select);
        assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Select tool active")
        }));
    }

    #[test]
    fn icon_atlas_duplicates_edge_pixels_into_gutters() {
        let atlas = icon_atlas_image().expect("atlas");
        let first_icon = ICON_ASSETS[0].1;
        let top_left = atlas_pixel(&atlas.data, atlas.width, 0, 0);
        let inset_top_left = atlas_pixel(
            &atlas.data,
            atlas.width,
            ICON_ATLAS_PADDING,
            ICON_ATLAS_PADDING,
        );
        let bottom_right = atlas_pixel(
            &atlas.data,
            atlas.width,
            ICON_ATLAS_CELL_SIZE - 1,
            ICON_ATLAS_CELL_SIZE - 1,
        );

        assert_eq!(atlas.width, 238);
        assert_eq!(atlas.height, 136);
        assert_eq!(top_left, &first_icon[0..4]);
        assert_eq!(inset_top_left, &first_icon[0..4]);
        assert_eq!(
            bottom_right,
            &first_icon[(((ICON_SIZE - 1) * ICON_SIZE + ICON_SIZE - 1) * 4) as usize..][..4]
        );
    }

    #[test]
    fn icon_atlas_regions_target_inner_unpadded_cells() {
        let mut resources = RenderResources::new();

        register_resources(&mut resources);

        let region = resources
            .image(ICON_CROSSHAIR)
            .and_then(|resource| resource.atlas_region)
            .expect("icon region");

        assert_eq!(region.source.width, ICON_SIZE as f32);
        assert_eq!(region.source.height, ICON_SIZE as f32);
        assert_eq!(
            region.source,
            Rect::new(
                (6 * ICON_ATLAS_CELL_SIZE + ICON_ATLAS_PADDING) as f32,
                (3 * ICON_ATLAS_CELL_SIZE + ICON_ATLAS_PADDING) as f32,
                ICON_SIZE as f32,
                ICON_SIZE as f32,
            )
        );
    }

    #[test]
    fn editor_icon_destinations_land_on_common_physical_pixels() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let icon_rects: Vec<_> = output
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Image(image) if is_editor_icon(image.image) => Some(image.rect),
                _ => None,
            })
            .collect();

        assert!(!icon_rects.is_empty());
        for rect in icon_rects {
            assert_eq!(rect.width, rect.height);
            for scale in [1.0_f32, 1.25, 1.5, 2.0] {
                let physical = rect.width * scale;
                assert_eq!(
                    physical,
                    physical.round(),
                    "icon rect {rect:?} is fractional at {scale}x"
                );
            }
        }
    }

    fn editor_test_context(input: UiInput) -> FrameContext {
        FrameContext::new(
            ViewportInfo::new(
                Size::new(1440.0, 900.0),
                PhysicalSize::new(1440, 900),
                ScaleFactor::ONE,
            ),
            input,
            TimeInfo::default(),
        )
    }

    fn pointer_input_at(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                primary: PointerButtonState::new(down, pressed, released),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    fn atlas_pixel(data: &[u8], width: u32, x: u32, y: u32) -> &[u8] {
        let start = ((y * width + x) * 4) as usize;
        &data[start..start + 4]
    }

    fn is_editor_icon(image: kinetik_ui::core::ImageId) -> bool {
        image.raw() > ICON_ATLAS.raw() && image.raw() <= ICON_CROSSHAIR.raw()
    }
}

fn tree_item(raw: u64, parent: Option<u64>, has_children: bool) -> TreeItem {
    TreeItem {
        id: item_id(raw),
        parent: parent.map(item_id),
        has_children,
    }
}

const fn item_id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

struct Asset {
    name: &'static str,
    kind: &'static str,
    icon: ImageId,
}

const ASSETS: &[Asset] = &[
    Asset {
        name: "camp_scene",
        kind: "scene",
        icon: ICON_CUBE,
    },
    Asset {
        name: "terrain_forest",
        kind: "mesh",
        icon: ICON_BOX,
    },
    Asset {
        name: "van_body",
        kind: "mesh",
        icon: ICON_COMPONENT,
    },
    Asset {
        name: "campfire",
        kind: "prefab",
        icon: ICON_TOKENS,
    },
    Asset {
        name: "night_sky",
        kind: "texture",
        icon: ICON_IMAGE,
    },
    Asset {
        name: "hero_ctrl",
        kind: "script",
        icon: ICON_CODE,
    },
    Asset {
        name: "audio_loop",
        kind: "asset",
        icon: ICON_ARCHIVE,
    },
    Asset {
        name: "lighting_lut",
        kind: "texture",
        icon: ICON_IMAGE,
    },
];

struct LogRow {
    time: &'static str,
    level: &'static str,
    message: &'static str,
}

const LOGS: &[LogRow] = &[
    LogRow {
        time: "00:00.1",
        level: "Info",
        message: "Loaded project campfire_adventure.kforge",
    },
    LogRow {
        time: "00:00.3",
        level: "Info",
        message: "Registered 28 Radix toolbar icons",
    },
    LogRow {
        time: "00:01.2",
        level: "Warn",
        message: "Light probe bake uses preview samples",
    },
    LogRow {
        time: "00:02.6",
        level: "Info",
        message: "Viewport texture uploaded through TextureResource",
    },
    LogRow {
        time: "00:03.1",
        level: "Info",
        message: "Scene graph visible range solved deterministically",
    },
];

fn scene_label(id: ItemId) -> &'static str {
    match id.raw() {
        1 => "CampfireAdventure",
        2 => "World",
        3 => "DirectionalLight",
        4 => "MainCamera",
        5 => "ReflectionProbe",
        6 => "Actors",
        7 => "Player",
        8 => "Van",
        9 => "Terrain",
        10 => "CampfireFX",
        11 => "AudioBus",
        _ => "Node",
    }
}

fn scene_icon(id: ItemId) -> ImageId {
    match id.raw() {
        1 => ICON_LAYERS,
        2 | 6 => ICON_CARET,
        3 => ICON_EYE,
        4 => ICON_CROSSHAIR,
        5 => ICON_GRID,
        7 => ICON_COMPONENT,
        8 | 9 => ICON_CUBE,
        10 => ICON_ROCKET,
        11 => ICON_ARCHIVE,
        _ => ICON_BOX,
    }
}

fn inspector_value_label(id: ItemId) -> &'static str {
    match id.raw() {
        9 => "M_AdventureNight",
        14 => "Capsule",
        15 => "player_controller.lua",
        _ => "-",
    }
}

fn log_color(level: &str) -> Color {
    match level {
        "Warn" => rgb(232, 179, 90),
        "Error" => rgb(236, 96, 96),
        _ => rgb(190, 197, 205),
    }
}

trait RectExt {
    fn with_width(self, width: f32) -> Self;
    fn right_strip(self, width: f32) -> Self;
}

impl RectExt for Rect {
    fn with_width(self, width: f32) -> Self {
        Rect::new(self.x, self.y, width.max(0.0), self.height)
    }

    fn right_strip(self, width: f32) -> Self {
        let width = width.max(0.0).min(self.width.max(0.0));
        Rect::new(self.max_x() - width, self.y, width, self.height)
    }
}

trait PanZoomExt {
    fn content_zoom(self) -> f32;
}

impl PanZoomExt for PanZoom {
    fn content_zoom(self) -> f32 {
        match self.fit {
            ViewportFit::Zoom => self.zoom,
            _ => 1.0,
        }
    }
}

fn rect(ui: &mut Ui<'_>, rect: Rect, fill: Color, stroke: Option<Color>) {
    rect_fill(ui, rect, fill, stroke, CornerRadius::all(0.0));
}

fn rect_fill(
    ui: &mut Ui<'_>,
    rect: Rect,
    fill: Color,
    stroke: Option<Color>,
    radius: CornerRadius,
) {
    ui.primitive(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(fill)),
        stroke: stroke.map(|stroke| Stroke::new(1.0, Brush::Solid(stroke))),
        radius,
    }));
}

fn line(ui: &mut Ui<'_>, from: Point, to: Point, color: Color, width: f32) {
    ui.primitive(Primitive::Line(LinePrimitive {
        from,
        to,
        stroke: Stroke::new(width, Brush::Solid(color)),
    }));
}

fn icon(ui: &mut Ui<'_>, bounds: Rect, image: ImageId, size: f32) {
    let size = if size.is_finite() && size > 0.0 {
        size
    } else {
        DENSE_ICON_SIZE
    };
    let rect = Rect::new(
        bounds.x + (bounds.width - size) * 0.5,
        bounds.y + (bounds.height - size) * 0.5,
        size,
        size,
    );
    ui.image(rect, image);
}

fn text(ui: &mut Ui<'_>, x: f32, baseline: f32, value: &str, size: f32, fill: Color) {
    ui.primitive(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(x, baseline),
        text: value.to_owned(),
        family: "sans-serif".to_owned(),
        size,
        line_height: size + 5.0,
        brush: Brush::Solid(fill),
    }));
}

fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::rgb(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
    )
}

fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> Color {
    Color::rgba(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        alpha,
    )
}
