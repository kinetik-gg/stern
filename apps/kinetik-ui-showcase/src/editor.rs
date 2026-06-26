//! DCC-style editor showcase surface.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use kinetik_ui::core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionQueue, ActionSource, Axis,
    Brush, ClipId, Color, CornerRadius, CursorShape, ImagePrimitive, Key, KeyState, LinePrimitive,
    Modifiers, PlatformRequest, Point, Primitive, Rect, RectPrimitive, RepaintRequest,
    SemanticNode, SemanticRole, Shortcut, Size, Stroke, TextPrimitive, TextureId, Theme, Vec2,
    WidgetId,
};
use kinetik_ui::render::{
    ImageAtlasRegion, ImageResource, RenderImage, RenderImageSampling, RenderResources,
    TextureResource,
};
use kinetik_ui::text::TextEditState;
use kinetik_ui::widgets::{
    Dock, DockChromeStyle, DockDropTarget, DockDropZone, DockInteractionPolicy, DockNode,
    DockPlacement, DockSplitterContextActionKind, DockTabDrag, Frame, FrameId, FrameLayout,
    FrameTab, GridColumns, GridLayout, Guide, ItemId, ListLayout, Menu, MenuItem, MenuOverlay,
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PanZoom, Panel, PanelId,
    PanelInstanceId, PanelInstancePolicy, PanelInstanceSnapshot, PanelOpenActionMetadata,
    PanelOpenDecision, PanelRegistry, PanelTypeCategory, PanelTypeDescriptor, PanelTypeId,
    PanelWorkspaceContext, PopoverPlacement, PopoverRequest, PropertyGridLayout, PropertyGridRow,
    TableColumn, TableLayout, TreeExpansion, TreeItem, TreeLayout, TreeModel, Ui,
    ViewportComposition, ViewportFit, ViewportSurface, WorkspaceSnapshot, frame_tabs,
    icon_button_semantics, place_popover, resolve_dock_splitter_context_actions_with_policy,
    resolve_frame_drop_zone_with_policy, solve_dock_layout, solve_dock_splitters_with_style,
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
const ACTION_DOCK_JOIN: &str = "editor.dock.join";
const ACTION_DOCK_SWAP: &str = "editor.dock.swap";
const ACTION_OPEN_VIEWPORT: &str = "editor.panel.open.viewport";
const ACTION_OPEN_EXPLORER: &str = "editor.panel.open.explorer";
const ACTION_OPEN_PROPERTIES: &str = "editor.panel.open.properties";
const ACTION_OPEN_ASSET_BROWSER: &str = "editor.panel.open.asset-browser";
const ACTION_OPEN_TIMELINE: &str = "editor.panel.open.timeline";
const ACTION_OPEN_CONSOLE: &str = "editor.panel.open.console";
const ACTION_OPEN_NODE_GRAPH: &str = "editor.panel.open.node-graph";

const VIEWPORT_TEXTURE: TextureId = TextureId::from_raw(9_001);
const VIEWPORT_SIZE: Size = Size::new(1280.0, 720.0);

#[allow(dead_code, missing_docs)]
pub(crate) mod phosphor_icons {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/icons/phosphor/phosphor_icons.rs"
    ));
}

use phosphor_icons::{DENSE_ICON_LOGICAL_SIZE, ICON_ATLASES, ICON_ENTRIES, PhosphorIcon};

const DENSE_ICON_SIZE: f32 = DENSE_ICON_LOGICAL_SIZE as f32;
const TOOLBAR_Y: f32 = 32.0;
const TOOLBAR_BOTTOM_PADDING: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct EditorChromeMetrics {
    toolbar_button: f32,
    toolbar_stride: f32,
    toolbar_icon: f32,
    dense_icon: f32,
    asset_icon: f32,
}

impl EditorChromeMetrics {
    fn from_theme(theme: &Theme) -> Self {
        let toolbar_button =
            (theme.controls.compact_control_height + theme.controls.padding_y).round();
        let toolbar_stride = (toolbar_button + theme.controls.padding_x * 0.5).round();
        Self {
            toolbar_button,
            toolbar_stride,
            toolbar_icon: theme.controls.icon_size,
            dense_icon: theme.controls.icon_size,
            asset_icon: theme.controls.icon_size,
        }
    }
}

fn workspace_top(theme: &Theme) -> f32 {
    TOOLBAR_Y + EditorChromeMetrics::from_theme(theme).toolbar_button + TOOLBAR_BOTTOM_PADDING
}

fn editor_dock_interaction_policy() -> DockInteractionPolicy {
    DockInteractionPolicy::default().with_drop_edge_fraction(0.25)
}

fn editor_dock_chrome_style() -> DockChromeStyle {
    DockChromeStyle::default().with_splitter_hit_thickness(4.0)
}

const FRAME_SCENE: FrameId = FrameId::from_raw(1);
const FRAME_ASSETS: FrameId = FrameId::from_raw(2);
const FRAME_VIEWPORT: FrameId = FrameId::from_raw(3);
const FRAME_BOTTOM: FrameId = FrameId::from_raw(4);
const FRAME_INSPECTOR: FrameId = FrameId::from_raw(5);

const PANEL_TYPE_SCENE: PanelTypeId = PanelTypeId::from_raw(1);
const PANEL_TYPE_ASSETS: PanelTypeId = PanelTypeId::from_raw(2);
const PANEL_TYPE_VIEWPORT: PanelTypeId = PanelTypeId::from_raw(3);
const PANEL_TYPE_CONSOLE: PanelTypeId = PanelTypeId::from_raw(4);
const PANEL_TYPE_TIMELINE: PanelTypeId = PanelTypeId::from_raw(5);
const PANEL_TYPE_INSPECTOR: PanelTypeId = PanelTypeId::from_raw(6);
const PANEL_TYPE_NODE_GRAPH: PanelTypeId = PanelTypeId::from_raw(7);

const PANEL_SCENE_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(1);
const PANEL_ASSETS_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(2);
const PANEL_VIEWPORT_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(3);
const PANEL_CONSOLE_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(4);
const PANEL_TIMELINE_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(5);
const PANEL_INSPECTOR_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(6);
const PANEL_NODE_GRAPH_INSTANCE: PanelInstanceId = PanelInstanceId::from_raw(7);

const PANEL_SCENE: PanelId = PanelId::from_instance_id(PANEL_SCENE_INSTANCE);
const PANEL_ASSETS: PanelId = PanelId::from_instance_id(PANEL_ASSETS_INSTANCE);
const PANEL_VIEWPORT: PanelId = PanelId::from_instance_id(PANEL_VIEWPORT_INSTANCE);
const PANEL_CONSOLE: PanelId = PanelId::from_instance_id(PANEL_CONSOLE_INSTANCE);
const PANEL_TIMELINE: PanelId = PanelId::from_instance_id(PANEL_TIMELINE_INSTANCE);
const PANEL_INSPECTOR: PanelId = PanelId::from_instance_id(PANEL_INSPECTOR_INSTANCE);
const PANEL_NODE_GRAPH: PanelId = PanelId::from_instance_id(PANEL_NODE_GRAPH_INSTANCE);
const FRAME_DRAG_INSERT_START: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EditorPanelInstanceSpec {
    id: PanelInstanceId,
    panel_type: PanelTypeId,
    title: &'static str,
    state_key: &'static str,
}

const EDITOR_PANEL_INSTANCES: &[EditorPanelInstanceSpec] = &[
    EditorPanelInstanceSpec {
        id: PANEL_SCENE_INSTANCE,
        panel_type: PANEL_TYPE_SCENE,
        title: "Explorer",
        state_key: "editor.explorer",
    },
    EditorPanelInstanceSpec {
        id: PANEL_ASSETS_INSTANCE,
        panel_type: PANEL_TYPE_ASSETS,
        title: "Asset Browser",
        state_key: "editor.asset-browser",
    },
    EditorPanelInstanceSpec {
        id: PANEL_VIEWPORT_INSTANCE,
        panel_type: PANEL_TYPE_VIEWPORT,
        title: "Viewport",
        state_key: "editor.viewport",
    },
    EditorPanelInstanceSpec {
        id: PANEL_CONSOLE_INSTANCE,
        panel_type: PANEL_TYPE_CONSOLE,
        title: "Console",
        state_key: "editor.console",
    },
    EditorPanelInstanceSpec {
        id: PANEL_TIMELINE_INSTANCE,
        panel_type: PANEL_TYPE_TIMELINE,
        title: "Timeline",
        state_key: "editor.timeline",
    },
    EditorPanelInstanceSpec {
        id: PANEL_INSPECTOR_INSTANCE,
        panel_type: PANEL_TYPE_INSPECTOR,
        title: "Properties",
        state_key: "editor.properties",
    },
    EditorPanelInstanceSpec {
        id: PANEL_NODE_GRAPH_INSTANCE,
        panel_type: PANEL_TYPE_NODE_GRAPH,
        title: "Node Graph",
        state_key: "editor.node-graph",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTool {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolbarIcon {
    Select,
    Move,
    Rotate,
    Scale,
    Grid,
    Crosshair,
    Reset,
    Play,
    Pause,
    Stop,
    Rocket,
    Download,
    Plus,
    Dots,
    Search,
    Gear,
    Layers,
    Caret,
    Eye,
    Component,
    Cube,
    Archive,
    Image,
    Code,
    Tokens,
    Box,
}

const EDITOR_TOOL_BUTTONS: [(EditorTool, ToolbarIcon, &str, &str); 4] = [
    (
        EditorTool::Select,
        ToolbarIcon::Select,
        "Select",
        ACTION_TOOL_SELECT,
    ),
    (
        EditorTool::Move,
        ToolbarIcon::Move,
        "Move",
        ACTION_TOOL_MOVE,
    ),
    (
        EditorTool::Rotate,
        ToolbarIcon::Rotate,
        "Rotate",
        ACTION_TOOL_ROTATE,
    ),
    (
        EditorTool::Scale,
        ToolbarIcon::Scale,
        "Scale",
        ACTION_TOOL_SCALE,
    ),
];

impl ToolbarIcon {
    const fn raw(self) -> u64 {
        match self {
            Self::Select => 1,
            Self::Move => 2,
            Self::Rotate => 3,
            Self::Scale => 4,
            Self::Grid => 5,
            Self::Crosshair => 6,
            Self::Reset => 7,
            Self::Play => 8,
            Self::Pause => 9,
            Self::Stop => 10,
            Self::Rocket => 11,
            Self::Download => 12,
            Self::Plus => 13,
            Self::Dots => 14,
            Self::Search => 15,
            Self::Gear => 16,
            Self::Layers => 17,
            Self::Caret => 18,
            Self::Eye => 19,
            Self::Component => 20,
            Self::Cube => 21,
            Self::Archive => 22,
            Self::Image => 23,
            Self::Code => 24,
            Self::Tokens => 25,
            Self::Box => 26,
        }
    }

    const fn phosphor(self) -> PhosphorIcon {
        match self {
            Self::Select => PhosphorIcon::Cursor,
            Self::Move => PhosphorIcon::Move,
            Self::Rotate => PhosphorIcon::Rotate,
            Self::Scale => PhosphorIcon::Transform,
            Self::Grid => PhosphorIcon::Grid,
            Self::Crosshair => PhosphorIcon::Crosshair,
            Self::Reset => PhosphorIcon::Reset,
            Self::Play => PhosphorIcon::Play,
            Self::Pause => PhosphorIcon::Pause,
            Self::Stop => PhosphorIcon::Stop,
            Self::Rocket => PhosphorIcon::Rocket,
            Self::Download => PhosphorIcon::Download,
            Self::Plus => PhosphorIcon::Plus,
            Self::Dots => PhosphorIcon::Dots,
            Self::Search => PhosphorIcon::Search,
            Self::Gear => PhosphorIcon::Gear,
            Self::Layers => PhosphorIcon::Layers,
            Self::Caret => PhosphorIcon::Caret,
            Self::Eye => PhosphorIcon::Eye,
            Self::Component => PhosphorIcon::Component,
            Self::Cube => PhosphorIcon::Cube,
            Self::Archive => PhosphorIcon::Archive,
            Self::Image => PhosphorIcon::Image,
            Self::Code => PhosphorIcon::Code,
            Self::Tokens => PhosphorIcon::Tokens,
            Self::Box => PhosphorIcon::Box,
        }
    }
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
pub type EditorInvocation = ActionInvocation;

/// Interactive DCC/editor showcase state.
pub struct EditorShowcase {
    dock: Dock,
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
    next_drop_frame: u64,
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
            next_drop_frame: FRAME_DRAG_INSERT_START,
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
            _ => panel_type_for_open_action(action_id)
                .is_some_and(|panel_type| self.open_or_focus_panel(panel_type)),
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
        self.tool_bar_run_interactions(ui, viewport, &mut invocations);
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
            invocations.push(ActionInvocation::new(
                ActionId::new(action_id),
                source,
                ActionContext::Editor,
            ));
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
            let was_active = self.open_menu == Some(kind);
            if response.clicked {
                self.open_menu = if was_active { None } else { Some(kind) };
                ui.request_repaint(RepaintRequest::NextFrame);
            } else if self.open_menu.is_some()
                && response.state.hovered
                && self.open_menu != Some(kind)
            {
                self.open_menu = Some(kind);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
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

    fn dismiss_menu_for_input(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
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
        let overlay = self.menu_overlay_model(kind, viewport);
        let mut stack = OverlayStack::new();
        overlay.open_in(&mut stack);
        if !stack
            .dismissal_requests(outside_activation, escape_pressed)
            .is_empty()
        {
            self.open_menu = None;
            ui.request_repaint(RepaintRequest::NextFrame);
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
        let overlay = self.menu_overlay_model(kind, viewport);
        if self.menu_overlay_interactions(ui, kind, &overlay, invocations) {
            return;
        }
        let visible_items = overlay.visible_items();
        rect_fill(
            ui,
            overlay.entry.rect.translate(Vec2::new(0.0, 2.0)),
            rgb(0, 0, 0),
            None,
            CornerRadius::all(0.0),
        );
        rect_fill(
            ui,
            overlay.entry.rect,
            rgb(28, 30, 33),
            Some(rgb(74, 78, 86)),
            CornerRadius::all(0.0),
        );

        let mut y = overlay.entry.rect.y + 6.0;
        for (index, item) in visible_items.into_iter().enumerate() {
            match item {
                MenuItem::Label(label) => {
                    text(
                        ui,
                        overlay.entry.rect.x + 10.0,
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
                        Rect::new(
                            overlay.entry.rect.x + 8.0,
                            y + 4.0,
                            overlay.entry.rect.width - 16.0,
                            1.0,
                        ),
                        rgb(60, 63, 70),
                        None,
                    );
                    y += 9.0;
                }
                MenuItem::Action(action) => {
                    let row = Rect::new(
                        overlay.entry.rect.x + 4.0,
                        y,
                        overlay.entry.rect.width - 8.0,
                        24.0,
                    );
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
                    y += 24.0;
                }
            }
        }
    }

    fn menu_overlay_interactions(
        &mut self,
        ui: &mut Ui<'_>,
        kind: EditorMenuKind,
        overlay: &MenuOverlay,
        invocations: &mut Vec<EditorInvocation>,
    ) -> bool {
        let mut y = overlay.entry.rect.y + 6.0;
        for (index, item) in overlay.visible_items().into_iter().enumerate() {
            match item {
                MenuItem::Label(_) => {
                    y += 22.0;
                }
                MenuItem::Separator => {
                    y += 9.0;
                }
                MenuItem::Action(action) => {
                    let row = Rect::new(
                        overlay.entry.rect.x + 4.0,
                        y,
                        overlay.entry.rect.width - 8.0,
                        24.0,
                    );
                    let enabled = action.can_invoke();
                    let response = ui.pressable(
                        ("editor.menu-row.prepass", kind, index, action.id.as_str()),
                        row,
                        !enabled,
                    );
                    if response.clicked && enabled {
                        let mut queue = ActionQueue::new();
                        if overlay.invoke_visible(index, &mut queue) {
                            self.handle_action_queue(invocations, &mut queue);
                            self.open_menu = None;
                            ui.request_repaint(RepaintRequest::NextFrame);
                            return true;
                        }
                    }
                    y += 24.0;
                }
            }
        }
        false
    }

    fn menu_overlay_model(&self, kind: EditorMenuKind, viewport: Rect) -> MenuOverlay {
        MenuOverlay::new(
            Self::menu_overlay_entry(kind, viewport),
            self.menu_model(kind),
            ActionSource::Menu,
            ActionContext::Editor,
        )
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
            EditorMenuKind::Window => self.window_menu_model(),
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

    fn window_menu_model(&self) -> Menu {
        let registry = editor_panel_registry();
        let open_metadata = editor_open_panel_metadata();
        let mut menu = Menu::new();
        menu.push(menu_action(
            ACTION_PALETTE,
            "Command Palette",
            Some(ctrl_char("p")),
            None,
            true,
        ));
        menu.push(MenuItem::Separator);

        for category in registry.categories() {
            menu.push(MenuItem::Label(panel_category_label(category).to_owned()));
            for metadata in open_metadata
                .iter()
                .filter(|metadata| &metadata.category == category)
            {
                menu.push(menu_action_from_panel_metadata(
                    metadata,
                    self.is_panel_type_open(metadata.panel_type),
                ));
            }
        }

        menu
    }

    fn is_panel_type_open(&self, panel_type: PanelTypeId) -> bool {
        let instances = editor_panel_instances();
        instances.iter().any(|instance| {
            let panel = PanelId::from_instance_id(instance.id);
            instance.panel_type == panel_type
                && self
                    .dock
                    .frames()
                    .iter()
                    .any(|frame| frame.panels.iter().any(|item| item.id == panel))
        })
    }

    fn open_or_focus_panel(&mut self, panel_type: PanelTypeId) -> bool {
        let registry = editor_panel_registry();
        let instances = editor_panel_instances();
        let Some(decision) = registry.resolve_open_decision(
            panel_type,
            &instances,
            &self.dock,
            PanelWorkspaceContext::Docked,
        ) else {
            "Panel open request unavailable".clone_into(&mut self.status);
            return false;
        };

        match decision {
            PanelOpenDecision::FocusExisting(request) => {
                if self
                    .dock
                    .select_panel(request.target.frame, request.target.panel)
                {
                    self.status = format!("Focused {}", request.metadata.title);
                    true
                } else {
                    self.status = format!("Could not focus {}", request.metadata.title);
                    false
                }
            }
            PanelOpenDecision::OpenNew(request) => {
                self.status = format!("Open {} requested", request.metadata.title);
                true
            }
        }
    }

    fn handle_action_queue(
        &mut self,
        invocations: &mut Vec<EditorInvocation>,
        queue: &mut ActionQueue,
    ) {
        for invocation in queue.drain() {
            if self.apply_action(invocation.action_id.as_str()) {
                invocations.push(invocation);
            }
        }
    }

    fn tool_bar(
        &mut self,
        ui: &mut Ui<'_>,
        viewport: Rect,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        self.tool_bar_tool_interactions(ui, invocations);
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        let mut x = 10.0;
        for (tool, icon, label, action) in EDITOR_TOOL_BUTTONS {
            let button = Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button);
            toolbar_icon_button(
                ui,
                ("editor.tool", action),
                button,
                icon,
                label,
                self.selected_tool == tool,
                false,
            );
            x += chrome.toolbar_stride;
        }

        rect(
            ui,
            Rect::new(x + 4.0, TOOLBAR_Y + 3.0, 1.0, chrome.toolbar_button - 6.0),
            rgb(57, 60, 66),
            None,
        );
        x += 18.0;
        for (icon, label, action) in [
            (ToolbarIcon::Grid, "Toggle grid", ACTION_GRID),
            (ToolbarIcon::Crosshair, "Frame selected", ACTION_PALETTE),
            (ToolbarIcon::Reset, "Reset view", ACTION_PALETTE),
        ] {
            let response = toolbar_icon_button(
                ui,
                ("editor.viewport-tool", action, icon.raw()),
                Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button),
                icon,
                label,
                false,
                false,
            );
            if response.clicked {
                self.trigger(invocations, action, ActionSource::Button);
            }
            x += chrome.toolbar_stride;
        }

        rect(
            ui,
            Rect::new(x + 4.0, TOOLBAR_Y + 3.0, 1.0, chrome.toolbar_button - 6.0),
            rgb(57, 60, 66),
            None,
        );
        x += 18.0;
        for (kind, icon, label, action) in [
            (
                DockSplitterContextActionKind::Join,
                ToolbarIcon::Component,
                "Join dock splitter",
                ACTION_DOCK_JOIN,
            ),
            (
                DockSplitterContextActionKind::Swap,
                ToolbarIcon::Layers,
                "Swap dock frames",
                ACTION_DOCK_SWAP,
            ),
        ] {
            let response = toolbar_icon_button(
                ui,
                ("editor.dock-action", action),
                Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button),
                icon,
                label,
                false,
                false,
            );
            if response.clicked {
                let bounds = editor_workspace_rect(ui.theme(), viewport);
                if self.apply_splitter_context_action(bounds, kind) {
                    invocations.push(ActionInvocation::new(
                        ActionId::new(action),
                        ActionSource::Button,
                        ActionContext::Editor,
                    ));
                }
                ui.request_repaint(RepaintRequest::NextFrame);
            }
            x += chrome.toolbar_stride;
        }

        for (index, icon, label, action, rect) in run_toolbar_buttons(viewport, chrome) {
            toolbar_icon_button(
                ui,
                ("editor.run", action, index),
                rect,
                icon,
                label,
                false,
                false,
            );
        }
    }

    fn apply_splitter_context_action(
        &mut self,
        bounds: Rect,
        kind: DockSplitterContextActionKind,
    ) -> bool {
        let frame_layouts = solve_dock_layout(&self.dock, bounds);
        let Some(splitter) =
            solve_dock_splitters_with_style(&self.dock, bounds, editor_dock_chrome_style())
                .into_iter()
                .next()
        else {
            "No dock splitter action available".clone_into(&mut self.status);
            return false;
        };
        let policy = editor_dock_interaction_policy();
        let actions = resolve_dock_splitter_context_actions_with_policy(
            &self.dock,
            &frame_layouts,
            &splitter,
            policy,
        );
        let Some(action) = actions
            .into_iter()
            .find(|action| action.kind == kind && action.enabled)
        else {
            match kind {
                DockSplitterContextActionKind::Join => "No dock join action available",
                DockSplitterContextActionKind::Swap => "No dock swap action available",
            }
            .clone_into(&mut self.status);
            return false;
        };

        match kind {
            DockSplitterContextActionKind::Join => {
                let Some(request) = action.join_request() else {
                    "No dock join action available".clone_into(&mut self.status);
                    return false;
                };
                let source = request.source_frame();
                let target = request.target_frame();
                if self
                    .dock
                    .apply_join_request_with_policy(bounds, request, policy)
                {
                    self.status = format!(
                        "Dock splitter joined frame {} into frame {}",
                        source.raw(),
                        target.raw()
                    );
                    true
                } else {
                    "Dock join request rejected".clone_into(&mut self.status);
                    false
                }
            }
            DockSplitterContextActionKind::Swap => {
                let Some(request) = action.swap_request() else {
                    "No dock swap action available".clone_into(&mut self.status);
                    return false;
                };
                let source = request.source_frame();
                let target = request.target_frame();
                if self
                    .dock
                    .apply_swap_request_with_policy(bounds, request, policy)
                {
                    self.status = format!(
                        "Dock splitter swapped frame {} with frame {}",
                        source.raw(),
                        target.raw()
                    );
                    true
                } else {
                    "Dock swap request rejected".clone_into(&mut self.status);
                    false
                }
            }
        }
    }

    fn tool_bar_run_interactions(
        &mut self,
        ui: &mut Ui<'_>,
        viewport: Rect,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        for (index, _icon, _label, action, rect) in run_toolbar_buttons(viewport, chrome) {
            let response = ui.pressable(("editor.run.prepass", action, index), rect, false);
            if response.clicked {
                self.trigger(invocations, action, ActionSource::Button);
            }
        }
    }

    fn tool_bar_tool_interactions(
        &mut self,
        ui: &mut Ui<'_>,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        let mut x = 10.0;
        for (_tool, _icon, _label, action) in EDITOR_TOOL_BUTTONS {
            let button = Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button);
            let response = ui.pressable(("editor.tool.prepass", action), button, false);
            if response.clicked {
                ui.request_repaint(RepaintRequest::NextFrame);
                self.trigger(invocations, action, ActionSource::Button);
            }
            x += chrome.toolbar_stride;
        }
    }

    fn workspace(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
        let bounds = editor_workspace_rect(ui.theme(), viewport);
        let dock_semantic_id = ui.id("editor.dock.semantic");
        ui.push_semantic_node(
            SemanticNode::new(dock_semantic_id, SemanticRole::Dock, bounds)
                .with_label("Editor Dock"),
        );
        let frame_layouts = solve_dock_layout(&self.dock, bounds);
        let mut tab_drags = Vec::new();
        for layout in &frame_layouts {
            let frame_rect = layout.rect.inset(2.0);
            let Some(frame_snapshot) = self.dock.frame(layout.frame).cloned() else {
                continue;
            };
            self.frame_tab_interactions(
                ui,
                layout.frame,
                frame_rect,
                26.0,
                &frame_snapshot,
                &mut tab_drags,
            );
        }
        for layout in &frame_layouts {
            self.editor_frame(ui, layout.frame, layout.rect.inset(2.0));
        }
        self.frame_drop_targets(ui, &frame_layouts, &tab_drags);

        let chrome_style = editor_dock_chrome_style();
        let interaction_policy = editor_dock_interaction_policy();
        for splitter in solve_dock_splitters_with_style(&self.dock, bounds, chrome_style) {
            let response = ui.draggable(
                ("editor.splitter", splitter.path.clone()),
                splitter.rect,
                false,
            );
            if response.dragged {
                self.dock.resize_split_with_policy(
                    &splitter.path,
                    bounds,
                    response.drag_delta,
                    interaction_policy,
                );
                ui.request_repaint(RepaintRequest::NextFrame);
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

        let tab_height = 26.0;
        let active_frame = self.dock.active_frame() == Some(frame_id);
        rect(
            ui,
            frame_rect,
            if active_frame {
                rgb(30, 33, 39)
            } else {
                rgb(28, 29, 32)
            },
            Some(if active_frame {
                rgb(78, 128, 210)
            } else {
                rgb(57, 59, 65)
            }),
        );
        let frame_semantic_id = ui.id(("editor.frame.semantic", frame_id.raw()));
        let mut frame_semantics =
            SemanticNode::new(frame_semantic_id, SemanticRole::Frame, frame_rect)
                .with_label(format!("Frame {}", frame_id.raw()))
                .focusable(true);
        frame_semantics.state.focused = active_frame;
        ui.push_semantic_node(frame_semantics);
        let Some(frame_snapshot) = self.dock.frame(frame_id).cloned() else {
            return;
        };
        for (tab, tab_rect) in frame_tab_rects(&frame_snapshot, frame_rect, tab_height) {
            ui.tab_button(
                ("editor.frame-tab", frame_id.raw(), tab.panel.raw()),
                tab_rect,
                tab.title,
                tab.active,
                false,
            );
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
            .cloned();
        if let Some(panel) = active_panel.as_ref() {
            let panel_semantic_id =
                ui.id(("editor.panel.semantic", frame_id.raw(), panel.id.raw()));
            ui.push_semantic_node(
                SemanticNode::new(panel_semantic_id, SemanticRole::Panel, body)
                    .with_label(panel.title.clone()),
            );
        }
        ui.clip_rect(
            ("editor.frame-body", frame_id.raw()),
            body,
            |ui| match active_panel.as_ref().map(|panel| panel.id) {
                Some(PANEL_SCENE) => self.scene_graph(ui, body),
                Some(PANEL_ASSETS) => self.assets_browser(ui, body),
                Some(PANEL_VIEWPORT) => self.viewport_panel(ui, body),
                Some(PANEL_CONSOLE) => Self::console_panel(ui, body),
                Some(PANEL_TIMELINE) => Self::timeline_panel(ui, body),
                Some(PANEL_INSPECTOR) => self.inspector(ui, body),
                Some(PANEL_NODE_GRAPH) => Self::node_graph_panel(ui, body),
                _ => {}
            },
        );
    }

    fn frame_tab_interactions(
        &mut self,
        ui: &mut Ui<'_>,
        frame_id: FrameId,
        frame_rect: Rect,
        tab_height: f32,
        frame: &Frame,
        tab_drags: &mut Vec<(WidgetId, DockTabDrag)>,
    ) {
        for (tab, tab_rect) in frame_tab_rects(frame, frame_rect, tab_height) {
            let response = ui.draggable(
                ("editor.frame-tab.drag", frame_id.raw(), tab.panel.raw()),
                tab_rect,
                false,
            );
            if let Some(drag) = self.dock.begin_tab_drag(frame_id, tab.panel) {
                tab_drags.push((response.id, drag));
            }
            if response.clicked {
                self.dock.select_panel(frame_id, tab.panel);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
            if response.dragged && tab.draggable {
                self.status = format!("Dragging {} tab", tab.title);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    fn frame_drop_targets(
        &mut self,
        ui: &mut Ui<'_>,
        frame_layouts: &[FrameLayout],
        tab_drags: &[(WidgetId, DockTabDrag)],
    ) {
        let Some(pointer) = ui.input().pointer.position else {
            return;
        };
        for layout in frame_layouts {
            let frame_rect = layout.rect.inset(2.0);
            let drop = ui.drop_target(
                ("editor.frame.drop-target", layout.frame.raw()),
                frame_rect,
                false,
            );
            let Some(source) = drop.source else {
                continue;
            };
            let Some((_, drag)) = tab_drags.iter().find(|(drag_id, _)| *drag_id == source) else {
                continue;
            };
            let Some(target) = self.dock_drop_target(layout.frame, frame_rect, pointer) else {
                continue;
            };

            if drop.dropped {
                if self.dock.drop_tab(*drag, target) {
                    if matches!(target, DockDropTarget::Split { .. }) {
                        self.next_drop_frame += 1;
                    }
                    self.status = dock_drop_status(target);
                    ui.request_repaint(RepaintRequest::NextFrame);
                }
                return;
            }

            if drop.response.state.hovered {
                draw_dock_drop_affordance(ui, frame_rect, target);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    fn dock_drop_target(
        &self,
        frame: FrameId,
        frame_rect: Rect,
        pointer: Point,
    ) -> Option<DockDropTarget> {
        let zone = resolve_frame_drop_zone_with_policy(
            frame_rect,
            pointer,
            editor_dock_interaction_policy(),
        )?;
        Some(match zone {
            DockDropZone::Center => DockDropTarget::tab(frame),
            DockDropZone::Left => DockDropTarget::split(
                frame,
                DockPlacement::Left,
                FrameId::from_raw(self.next_drop_frame),
            ),
            DockDropZone::Right => DockDropTarget::split(
                frame,
                DockPlacement::Right,
                FrameId::from_raw(self.next_drop_frame),
            ),
            DockDropZone::Top => DockDropTarget::split(
                frame,
                DockPlacement::Top,
                FrameId::from_raw(self.next_drop_frame),
            ),
            DockDropZone::Bottom => DockDropTarget::split(
                frame,
                DockPlacement::Bottom,
                FrameId::from_raw(self.next_drop_frame),
            ),
        })
    }

    fn scene_graph(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 24.0);
        let add = toolbar_icon_button_sized(
            ui,
            "editor.scene.add",
            header.with_width(28.0),
            ToolbarIcon::Plus,
            "Add node",
            false,
            false,
            DENSE_ICON_SIZE,
        );
        if add.clicked {
            "Create node requested".clone_into(&mut self.status);
            ui.request_repaint(RepaintRequest::NextFrame);
        }
        text(
            ui,
            header.x + 36.0,
            header.y + 17.0,
            "Scene",
            13.0,
            rgb(222, 225, 230),
        );
        draw_icon(
            ui,
            header.right_strip(24.0),
            ToolbarIcon::Dots,
            DENSE_ICON_SIZE,
        );

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
                    let twisty = Rect::new(
                        row_rect.content_rect.x + 3.0,
                        row_rect.rect.y + 5.0,
                        12.0,
                        12.0,
                    );
                    let twist = row.has_children.then(|| {
                        ui.pressable(("editor.scene.expand", row.id.raw()), twisty, false)
                    });
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
                    if let Some(twist) = twist {
                        let mut expanded = row.expanded;
                        if twist.clicked {
                            self.scene_expansion.toggle(row.id);
                            expanded = !expanded;
                            ui.request_repaint(RepaintRequest::NextFrame);
                        }
                        text(
                            ui,
                            twisty.x + 2.0,
                            twisty.y + 10.0,
                            if expanded { "v" } else { ">" },
                            11.0,
                            rgb(176, 181, 188),
                        );
                    }
                    draw_icon(
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
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        rect(ui, body, rgb(24, 25, 27), None);
        let search = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 26.0);
        ui.search_field(
            "editor.assets.search",
            search,
            &mut self.asset_filter,
            false,
        );
        draw_icon(
            ui,
            Rect::new(search.x + 5.0, search.y + 5.0, 18.0, 18.0),
            ToolbarIcon::Search,
            chrome.dense_icon,
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
                        ui.request_repaint(RepaintRequest::NextFrame);
                    }
                    draw_icon(
                        ui,
                        Rect::new(
                            item.rect.x + 8.0,
                            item.rect.y + 8.0,
                            chrome.asset_icon,
                            chrome.asset_icon,
                        ),
                        asset.icon,
                        chrome.asset_icon,
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
        let viewport_semantic_id = ui.id("editor.viewport.surface.semantic");
        ui.push_semantic_node(
            SemanticNode::new(viewport_semantic_id, SemanticRole::Viewport, surface_bounds)
                .with_label("Viewport Surface")
                .focusable(true),
        );
        let drag = ui.draggable("editor.viewport.surface", surface_bounds, false);
        if drag.dragged {
            self.viewport_pan_zoom.pan_by(drag.drag_delta);
            ui.request_repaint(RepaintRequest::NextFrame);
        }
        if drag.state.hovered {
            let wheel = ui.input().pointer.wheel_delta.y;
            if wheel.abs() > f32::EPSILON {
                let current = self.viewport_pan_zoom.content_zoom();
                let next = (current + (-wheel * 0.001)).clamp(0.25, 2.5);
                self.viewport_pan_zoom.set_zoom(next);
                self.status = format!("Viewport zoom {:.0}%", next * 100.0);
                ui.request_repaint(RepaintRequest::NextFrame);
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
        ui.extend(composition.primitives_at(ui.viewport().scale_factor));
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
            let content = surface.content_rect_at(ui.viewport().scale_factor);
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

        if let Some(selection) = surface.content_rect_to_screen_at(
            Rect::new(720.0, 210.0, 210.0, 280.0),
            ui.viewport().scale_factor,
        ) {
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
        draw_icon(
            ui,
            Rect::new(header.x + 7.0, header.y + 7.0, 20.0, 20.0),
            scene_icon(self.selected_node),
            DENSE_ICON_SIZE,
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 12.0,
            "Inspector",
            9.0,
            rgb(151, 158, 166),
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 27.0,
            scene_label(self.selected_node),
            12.0,
            rgb(231, 233, 237),
        );
        draw_icon(
            ui,
            Rect::new(header.max_x() - 27.0, header.y + 7.0, 20.0, 20.0),
            ToolbarIcon::Gear,
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

    fn timeline_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let rows = [
            ("Intro camera pan", "Frame 001-072", 0.24),
            ("Character pickup", "Frame 073-144", 0.48),
            ("Vehicle reveal", "Frame 145-216", 0.72),
            ("Cut to gameplay", "Frame 217-300", 0.88),
        ];
        let layout = ListLayout::new(28.0);
        for item in layout.row_rects(body.inset(8.0), rows.len(), 0..rows.len()) {
            let (name, range, progress) = rows[item.index];
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
                range,
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

    fn node_graph_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        text(
            ui,
            body.x + 10.0,
            body.y + 20.0,
            "Compositor Graph",
            12.0,
            rgb(218, 222, 228),
        );

        let nodes = [
            (
                "Texture",
                Rect::new(body.x + 18.0, body.y + 42.0, 92.0, 44.0),
            ),
            (
                "Color Grade",
                Rect::new(body.x + 150.0, body.y + 66.0, 112.0, 44.0),
            ),
            (
                "Output",
                Rect::new(body.x + 300.0, body.y + 48.0, 92.0, 44.0),
            ),
        ];
        for window in nodes.windows(2) {
            let from = window[0].1;
            let to = window[1].1;
            line(
                ui,
                Point::new(from.max_x(), from.y + from.height * 0.5),
                Point::new(to.x, to.y + to.height * 0.5),
                rgb(83, 137, 230),
                2.0,
            );
        }
        for (label, rect_bounds) in nodes {
            rect_fill(
                ui,
                rect_bounds,
                rgb(31, 33, 37),
                Some(rgb(66, 71, 80)),
                CornerRadius::all(4.0),
            );
            text(
                ui,
                rect_bounds.x + 10.0,
                rect_bounds.y + 27.0,
                label,
                11.0,
                rgb(228, 231, 236),
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
            sampling: RenderImageSampling::Pixelated,
            snapshot: Some(snapshot),
        });
    }
    for atlas in ICON_ATLASES {
        if let Some(snapshot) = RenderImage::rgba8(atlas.width, atlas.height, atlas.bytes.to_vec())
        {
            resources.register_image(ImageResource {
                id: atlas.image,
                size: Size::new(atlas.width as f32, atlas.height as f32),
                sampling: RenderImageSampling::UiIcon,
                pixels: Some(snapshot),
                atlas_region: None,
            });
        }
    }
    for entry in ICON_ENTRIES {
        resources.register_image(ImageResource {
            id: entry.image,
            size: Size::new(entry.logical_size as f32, entry.logical_size as f32),
            sampling: RenderImageSampling::UiIcon,
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: entry.atlas,
                source: entry.source,
            }),
        });
    }
}

#[cfg(test)]
fn icon_atlas_image(physical_size: u32) -> Option<RenderImage> {
    ICON_ATLASES
        .iter()
        .find(|atlas| atlas.physical_size == physical_size)
        .and_then(|atlas| RenderImage::rgba8(atlas.width, atlas.height, atlas.bytes.to_vec()))
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

fn editor_workspace_rect(theme: &Theme, viewport: Rect) -> Rect {
    let bottom_bar = 24.0;
    let workspace_top = workspace_top(theme);
    Rect::new(
        4.0,
        workspace_top,
        (viewport.width - 8.0).max(1.0),
        (viewport.height - workspace_top - bottom_bar - 4.0).max(1.0),
    )
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
        EditorMenuKind::Window => Size::new(232.0, 340.0),
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

fn menu_action_from_panel_metadata(metadata: &PanelOpenActionMetadata, checked: bool) -> MenuItem {
    let action_id = metadata
        .default_open_action
        .as_ref()
        .expect("showcase panel descriptors declare default open actions");
    let mut action = ActionDescriptor::new(action_id.as_str(), metadata.title.clone());
    action.state.checked = Some(checked);
    MenuItem::Action(action)
}

fn panel_type_for_open_action(action_id: &str) -> Option<PanelTypeId> {
    match action_id {
        ACTION_OPEN_VIEWPORT => Some(PANEL_TYPE_VIEWPORT),
        ACTION_OPEN_EXPLORER => Some(PANEL_TYPE_SCENE),
        ACTION_OPEN_PROPERTIES => Some(PANEL_TYPE_INSPECTOR),
        ACTION_OPEN_ASSET_BROWSER => Some(PANEL_TYPE_ASSETS),
        ACTION_OPEN_TIMELINE => Some(PANEL_TYPE_TIMELINE),
        ACTION_OPEN_CONSOLE => Some(PANEL_TYPE_CONSOLE),
        ACTION_OPEN_NODE_GRAPH => Some(PANEL_TYPE_NODE_GRAPH),
        _ => None,
    }
}

fn panel_category_label(category: &PanelTypeCategory) -> &str {
    match category {
        PanelTypeCategory::General => "General",
        PanelTypeCategory::Hierarchy => "Hierarchy",
        PanelTypeCategory::Inspector => "Inspector",
        PanelTypeCategory::Viewport => "Viewport",
        PanelTypeCategory::Assets => "Assets",
        PanelTypeCategory::Timeline => "Timeline",
        PanelTypeCategory::Diagnostics => "Diagnostics",
        PanelTypeCategory::Custom(label) => label.as_str(),
    }
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

fn editor_panel_type_descriptors() -> Vec<PanelTypeDescriptor> {
    vec![
        PanelTypeDescriptor::new(PANEL_TYPE_VIEWPORT, "Viewport")
            .with_category(PanelTypeCategory::Viewport)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(760.0, 520.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_VIEWPORT)),
        PanelTypeDescriptor::new(PANEL_TYPE_SCENE, "Explorer")
            .with_category(PanelTypeCategory::Hierarchy)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(300.0, 420.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_EXPLORER)),
        PanelTypeDescriptor::new(PANEL_TYPE_INSPECTOR, "Properties")
            .with_category(PanelTypeCategory::Inspector)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(280.0, 520.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_PROPERTIES)),
        PanelTypeDescriptor::new(PANEL_TYPE_ASSETS, "Asset Browser")
            .with_category(PanelTypeCategory::Assets)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(300.0, 260.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_ASSET_BROWSER)),
        PanelTypeDescriptor::new(PANEL_TYPE_TIMELINE, "Timeline")
            .with_category(PanelTypeCategory::Timeline)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(640.0, 180.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_TIMELINE)),
        PanelTypeDescriptor::new(PANEL_TYPE_CONSOLE, "Console")
            .with_category(PanelTypeCategory::Diagnostics)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(640.0, 180.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_CONSOLE)),
        PanelTypeDescriptor::new(PANEL_TYPE_NODE_GRAPH, "Node Graph")
            .with_category(PanelTypeCategory::Timeline)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(520.0, 220.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_NODE_GRAPH)),
    ]
}

fn editor_panel_registry() -> PanelRegistry {
    PanelRegistry::from_descriptors(editor_panel_type_descriptors())
        .expect("showcase panel descriptors must be unique")
}

fn editor_open_panel_metadata() -> Vec<PanelOpenActionMetadata> {
    editor_panel_registry().open_actions().collect()
}

fn editor_panel_instances() -> Vec<PanelInstanceSnapshot> {
    EDITOR_PANEL_INSTANCES
        .iter()
        .map(|spec| {
            PanelInstanceSnapshot::new(spec.id, spec.panel_type, spec.title)
                .with_state_key(spec.state_key)
        })
        .collect()
}

fn default_workspace_snapshot() -> WorkspaceSnapshot {
    default_dock_layout().workspace_snapshot(editor_panel_instances())
}

fn default_dock() -> Dock {
    let registry = editor_panel_registry();
    Dock::restore_workspace(default_workspace_snapshot(), registry.descriptors())
        .expect("default editor workspace snapshot should restore")
}

fn default_dock_layout() -> Dock {
    Dock::new(DockNode::Split {
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
                vec![editor_panel(PANEL_SCENE_INSTANCE)],
            ))),
            second: Box::new(DockNode::Frame(Frame::new(
                FRAME_ASSETS,
                vec![editor_panel(PANEL_ASSETS_INSTANCE)],
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
                    vec![editor_panel(PANEL_VIEWPORT_INSTANCE)],
                ))),
                second: Box::new(DockNode::Frame(Frame::new(
                    FRAME_BOTTOM,
                    vec![
                        editor_panel(PANEL_CONSOLE_INSTANCE),
                        editor_panel(PANEL_TIMELINE_INSTANCE),
                        editor_panel(PANEL_NODE_GRAPH_INSTANCE),
                    ],
                ))),
            }),
            second: Box::new(DockNode::Frame(Frame::new(
                FRAME_INSPECTOR,
                vec![editor_panel(PANEL_INSPECTOR_INSTANCE)],
            ))),
        }),
    })
}

fn editor_panel(instance: PanelInstanceId) -> Panel {
    let spec = EDITOR_PANEL_INSTANCES
        .iter()
        .find(|spec| spec.id == instance)
        .expect("editor panel instance is declared");
    Panel::from_instance_id(spec.id, spec.title)
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

fn frame_tab_rects(frame: &Frame, frame_rect: Rect, tab_height: f32) -> Vec<(FrameTab, Rect)> {
    let mut tab_x = frame_rect.x + 1.0;
    frame_tabs(frame)
        .into_iter()
        .map(|tab| {
            let width = (tab.title.len() as f32 * 7.0 + 42.0).clamp(82.0, 146.0);
            let tab_rect = Rect::new(tab_x, frame_rect.y + 1.0, width, tab_height);
            tab_x += width + 1.0;
            (tab, tab_rect)
        })
        .collect()
}

fn dock_drop_status(target: DockDropTarget) -> String {
    match target {
        DockDropTarget::Tab { frame } => {
            format!("Dock tab merged into frame {}", frame.raw())
        }
        DockDropTarget::Split {
            frame, placement, ..
        } => {
            let placement = match placement {
                DockPlacement::Left => "left of",
                DockPlacement::Right => "right of",
                DockPlacement::Top => "above",
                DockPlacement::Bottom => "below",
            };
            format!("Dock tab split {placement} frame {}", frame.raw())
        }
    }
}

fn draw_dock_drop_affordance(ui: &mut Ui<'_>, frame_rect: Rect, target: DockDropTarget) {
    let preview = match target {
        DockDropTarget::Tab { .. } => frame_rect.inset(24.0),
        DockDropTarget::Split {
            placement: DockPlacement::Left,
            ..
        } => Rect::new(
            frame_rect.x + 6.0,
            frame_rect.y + 6.0,
            frame_rect.width * 0.35,
            frame_rect.height - 12.0,
        ),
        DockDropTarget::Split {
            placement: DockPlacement::Right,
            ..
        } => Rect::new(
            frame_rect.max_x() - frame_rect.width * 0.35 - 6.0,
            frame_rect.y + 6.0,
            frame_rect.width * 0.35,
            frame_rect.height - 12.0,
        ),
        DockDropTarget::Split {
            placement: DockPlacement::Top,
            ..
        } => Rect::new(
            frame_rect.x + 6.0,
            frame_rect.y + 6.0,
            frame_rect.width - 12.0,
            frame_rect.height * 0.35,
        ),
        DockDropTarget::Split {
            placement: DockPlacement::Bottom,
            ..
        } => Rect::new(
            frame_rect.x + 6.0,
            frame_rect.max_y() - frame_rect.height * 0.35 - 6.0,
            frame_rect.width - 12.0,
            frame_rect.height * 0.35,
        ),
    };
    rect_fill(
        ui,
        preview,
        rgba(78, 142, 245, 0.18),
        Some(rgb(86, 151, 245)),
        CornerRadius::all(3.0),
    );
}

fn run_toolbar_buttons(
    viewport: Rect,
    chrome: EditorChromeMetrics,
) -> [(usize, ToolbarIcon, &'static str, &'static str, Rect); 5] {
    let right = viewport.max_x() - (chrome.toolbar_stride * 4.0 + chrome.toolbar_button);
    [
        (
            0,
            ToolbarIcon::Play,
            "Play",
            ACTION_PLAY,
            Rect::new(
                right,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            1,
            ToolbarIcon::Pause,
            "Pause",
            ACTION_PLAY,
            Rect::new(
                right + chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            2,
            ToolbarIcon::Stop,
            "Stop",
            ACTION_STOP,
            Rect::new(
                right + 2.0 * chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            3,
            ToolbarIcon::Rocket,
            "Build",
            ACTION_BUILD,
            Rect::new(
                right + 3.0 * chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            4,
            ToolbarIcon::Download,
            "Export",
            ACTION_BUILD,
            Rect::new(
                right + 4.0 * chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
    ]
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::items_after_test_module)]
mod tests {
    use super::{
        EditorChromeMetrics, EditorMenuKind, EditorShowcase, EditorTool, FRAME_BOTTOM,
        FRAME_INSPECTOR, FRAME_VIEWPORT, PANEL_TIMELINE, TOOLBAR_Y, VIEWPORT_SIZE, frame_tab_rects,
        icon_atlas_image, inspector_label_width, item_id, phosphor_icons, register_resources, rgb,
        rgba,
    };
    use kinetik_ui::core::{
        Brush, CursorShape, FrameContext, PhysicalSize, PlatformRequest, Point, PointerButtonState,
        PointerInput, Primitive, Rect, RepaintRequest, ScaleFactor, SemanticActionKind,
        SemanticRole, Size, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, default_dark_theme,
    };
    use kinetik_ui::render::RenderResources;
    use kinetik_ui::widgets::{
        DockSplitterContextActionKind, PanelOpenDecision, PanelTypeCategory, Ui, ViewportSurface,
        resolve_dock_splitter_context_actions_with_policy, solve_dock_layout,
        solve_dock_splitters_with_style,
    };

    #[test]
    fn inspector_label_width_preserves_value_space_at_narrow_widths() {
        assert_eq!(inspector_label_width(120.0), 52.0);
        assert!((inspector_label_width(180.0) - 75.6).abs() < f32::EPSILON);
        assert_eq!(inspector_label_width(400.0), 96.0);
        assert_eq!(inspector_label_width(f32::NAN), 72.0);
    }

    #[test]
    fn editor_chrome_metrics_follow_theme_controls() {
        let theme = default_dark_theme();
        let chrome = EditorChromeMetrics::from_theme(&theme);

        assert_eq!(
            chrome.toolbar_button,
            theme.controls.compact_control_height + theme.controls.padding_y
        );
        assert_eq!(
            chrome.toolbar_stride,
            chrome.toolbar_button + theme.controls.padding_x * 0.5
        );
        assert_eq!(chrome.toolbar_icon, theme.controls.icon_size);
        assert_eq!(chrome.asset_icon, theme.controls.icon_size);
        assert_eq!(chrome.toolbar_button, 26.0);
        assert_eq!(chrome.toolbar_stride, 30.0);
        assert_eq!(chrome.toolbar_icon, 16.0);
        assert_eq!(super::workspace_top(&theme), 68.0);
    }

    #[test]
    fn default_workspace_snapshot_validates_against_showcase_panel_descriptors() {
        let registry = super::editor_panel_registry();
        let snapshot = super::default_workspace_snapshot();
        let diagnostics = snapshot.diagnostics(registry.descriptors());

        assert!(diagnostics.is_valid(), "{diagnostics:?}");
        assert!(diagnostics.dock.diagnostics.is_empty(), "{diagnostics:?}");
        assert!(diagnostics.workspace.is_empty(), "{diagnostics:?}");
        snapshot
            .validate(registry.descriptors())
            .expect("workspace validates");
        assert_eq!(
            snapshot.panel_instances,
            super::editor_panel_instances(),
            "default workspace instances should be deterministic"
        );
    }

    #[test]
    fn default_workspace_snapshot_round_trips_through_workspace_restore() {
        let registry = super::editor_panel_registry();
        let snapshot = super::default_workspace_snapshot();
        let restored = super::Dock::restore_workspace(snapshot.clone(), registry.descriptors())
            .expect("restore");

        assert_eq!(restored.snapshot(), snapshot.dock);
        assert_eq!(
            restored.workspace_snapshot(super::editor_panel_instances()),
            snapshot
        );
    }

    #[test]
    fn editor_panel_registry_builds_unique_showcase_descriptors() {
        let registry = super::editor_panel_registry();

        assert_eq!(registry.descriptors().len(), 7);
        assert_eq!(
            registry.descriptors(),
            super::editor_panel_type_descriptors().as_slice()
        );
        assert_eq!(
            registry
                .descriptor(super::PANEL_TYPE_NODE_GRAPH)
                .expect("node graph descriptor")
                .title,
            "Node Graph"
        );
    }

    #[test]
    fn registry_open_metadata_exposes_editor_vocabulary_in_stable_order() {
        let registry = super::editor_panel_registry();
        let metadata = super::editor_open_panel_metadata();
        let titles = metadata
            .iter()
            .map(|metadata| metadata.title.as_str())
            .collect::<Vec<_>>();
        let action_ids = metadata
            .iter()
            .map(|metadata| {
                metadata
                    .default_open_action
                    .as_ref()
                    .expect("open action")
                    .as_str()
            })
            .collect::<Vec<_>>();
        let categories = registry
            .categories()
            .into_iter()
            .map(super::panel_category_label)
            .collect::<Vec<_>>();

        assert_eq!(
            titles,
            [
                "Viewport",
                "Explorer",
                "Properties",
                "Asset Browser",
                "Timeline",
                "Console",
                "Node Graph",
            ]
        );
        assert_eq!(
            action_ids,
            [
                super::ACTION_OPEN_VIEWPORT,
                super::ACTION_OPEN_EXPLORER,
                super::ACTION_OPEN_PROPERTIES,
                super::ACTION_OPEN_ASSET_BROWSER,
                super::ACTION_OPEN_TIMELINE,
                super::ACTION_OPEN_CONSOLE,
                super::ACTION_OPEN_NODE_GRAPH,
            ]
        );
        assert_eq!(
            metadata
                .iter()
                .map(|metadata| metadata.category.clone())
                .collect::<Vec<_>>(),
            [
                PanelTypeCategory::Viewport,
                PanelTypeCategory::Hierarchy,
                PanelTypeCategory::Inspector,
                PanelTypeCategory::Assets,
                PanelTypeCategory::Timeline,
                PanelTypeCategory::Diagnostics,
                PanelTypeCategory::Timeline,
            ]
        );
        assert_eq!(
            categories,
            [
                "Viewport",
                "Hierarchy",
                "Inspector",
                "Assets",
                "Timeline",
                "Diagnostics",
            ]
        );
    }

    #[test]
    fn default_workspace_snapshot_contains_roblox_blender_style_vocabulary() {
        let snapshot = super::default_workspace_snapshot();
        let titles = snapshot
            .panel_instances
            .iter()
            .map(|instance| instance.title.as_str())
            .collect::<Vec<_>>();
        let state_keys = snapshot
            .panel_instances
            .iter()
            .map(|instance| instance.state_key.as_deref().expect("state key"))
            .collect::<Vec<_>>();

        assert_eq!(
            titles,
            [
                "Explorer",
                "Asset Browser",
                "Viewport",
                "Console",
                "Timeline",
                "Properties",
                "Node Graph",
            ]
        );
        assert_eq!(
            state_keys,
            [
                "editor.explorer",
                "editor.asset-browser",
                "editor.viewport",
                "editor.console",
                "editor.timeline",
                "editor.properties",
                "editor.node-graph",
            ]
        );
    }

    #[test]
    fn registry_open_or_focus_workflow_is_app_owned_and_deterministic() {
        let mut editor = EditorShowcase::new();
        let registry = super::editor_panel_registry();
        let instances = super::editor_panel_instances();
        let decision = registry
            .resolve_open_decision(
                super::PANEL_TYPE_NODE_GRAPH,
                &instances,
                &editor.dock,
                super::PanelWorkspaceContext::Docked,
            )
            .expect("open decision");

        assert!(matches!(decision, PanelOpenDecision::FocusExisting(_)));
        assert!(editor.open_or_focus_panel(super::PANEL_TYPE_NODE_GRAPH));
        assert_eq!(editor.status, "Focused Node Graph");
        assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
        assert_eq!(
            editor
                .dock
                .frame(FRAME_BOTTOM)
                .and_then(|frame| frame.active_panel())
                .map(|panel| panel.id),
            Some(super::PANEL_NODE_GRAPH)
        );

        assert!(editor.apply_action(super::ACTION_OPEN_PROPERTIES));
        assert_eq!(editor.status, "Focused Properties");
        assert_eq!(editor.dock.active_frame(), Some(FRAME_INSPECTOR));
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
    fn toolbar_tool_click_has_single_same_frame_selection_visual() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let chrome = EditorChromeMetrics::from_theme(&theme);
        let rotate = Point::new(
            10.0 + 2.0 * chrome.toolbar_stride + chrome.toolbar_button * 0.5,
            TOOLBAR_Y + chrome.toolbar_button * 0.5,
        );

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(rotate.x, rotate.y, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(rotate.x, rotate.y, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let selected_fill = rgb(39, 69, 122);
        let selected_toolbar_buttons = output
            .primitives
            .iter()
            .filter(|primitive| match primitive {
                Primitive::Rect(rect) => {
                    rect.rect.y == TOOLBAR_Y
                        && rect.rect.width == chrome.toolbar_button
                        && rect.rect.height == chrome.toolbar_button
                        && matches!(&rect.fill, Some(Brush::Solid(color)) if *color == selected_fill)
                }
                _ => false,
            })
            .count();

        assert_eq!(editor.selected_tool, EditorTool::Rotate);
        assert_eq!(selected_toolbar_buttons, 1);
    }

    #[test]
    fn frame_tab_click_updates_body_same_frame() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let bottom = bottom_frame_rect(&editor);
        let timeline = editor
            .dock
            .frame(FRAME_BOTTOM)
            .and_then(|frame| {
                frame_tab_rects(frame, bottom, 26.0)
                    .into_iter()
                    .find(|(tab, _rect)| tab.panel == PANEL_TIMELINE)
                    .map(|(_tab, rect)| rect)
            })
            .expect("timeline tab");
        let point = Point::new(
            timeline.x + timeline.width * 0.5,
            timeline.y + timeline.height * 0.5,
        );

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(point.x, point.y, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(point.x, point.y, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
        assert_eq!(focused_frame_semantic_count(&output), 1);
        assert!(output.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Frame
                && node.label.as_deref() == Some("Frame 4")
                && node.state.focused
        }));
        assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Intro camera pan")
        }));
        assert!(!output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Message")
        }));
    }

    #[test]
    fn splitter_drag_routes_through_dock_resize_path() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let bounds = editor_workspace_bounds();
        let splitter = solve_dock_splitters_with_style(
            &editor.dock,
            bounds,
            super::editor_dock_chrome_style(),
        )
        .into_iter()
        .next()
        .expect("root splitter");
        let before = splitter.ratio;
        let press = splitter.rect.center();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(press.x, press.y, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at_with_delta(
                press.x + 48.0,
                press.y,
                true,
                false,
                false,
                Vec2::new(48.0, 0.0),
            )),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let after = solve_dock_splitters_with_style(
            &editor.dock,
            bounds,
            super::editor_dock_chrome_style(),
        )
        .into_iter()
        .next()
        .expect("root splitter")
        .ratio;

        assert!(after > before, "{after} should be greater than {before}");
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
    }

    #[test]
    fn editor_splitter_join_action_uses_context_metadata_and_apply_request() {
        let mut editor = EditorShowcase::new();
        let bounds = editor_workspace_bounds();
        let layout = solve_dock_layout(&editor.dock, bounds);
        let splitter = solve_dock_splitters_with_style(
            &editor.dock,
            bounds,
            super::editor_dock_chrome_style(),
        )
        .into_iter()
        .next()
        .expect("root splitter");
        let action = resolve_dock_splitter_context_actions_with_policy(
            &editor.dock,
            &layout,
            &splitter,
            super::editor_dock_interaction_policy(),
        )
        .into_iter()
        .find(|action| action.kind == DockSplitterContextActionKind::Join && action.enabled)
        .expect("enabled join action");
        let request = action.join_request().expect("join request");
        let source = request.source_frame();
        let target = request.target_frame();

        assert!(editor.apply_splitter_context_action(bounds, DockSplitterContextActionKind::Join));

        assert!(editor.dock.frame(source).is_none());
        assert!(editor.dock.frame(target).is_some());
        assert_eq!(editor.dock.active_frame(), Some(target));
        assert_eq!(
            editor.status,
            format!(
                "Dock splitter joined frame {} into frame {}",
                source.raw(),
                target.raw()
            )
        );
    }

    #[test]
    fn editor_splitter_swap_action_uses_context_metadata_and_apply_request() {
        let mut editor = EditorShowcase::new();
        let bounds = editor_workspace_bounds();
        let layout = solve_dock_layout(&editor.dock, bounds);
        let splitter = solve_dock_splitters_with_style(
            &editor.dock,
            bounds,
            super::editor_dock_chrome_style(),
        )
        .into_iter()
        .next()
        .expect("root splitter");
        let action = resolve_dock_splitter_context_actions_with_policy(
            &editor.dock,
            &layout,
            &splitter,
            super::editor_dock_interaction_policy(),
        )
        .into_iter()
        .find(|action| action.kind == DockSplitterContextActionKind::Swap && action.enabled)
        .expect("enabled swap action");
        let request = action.swap_request().expect("swap request");
        let source = request.source_frame();
        let target = request.target_frame();
        let source_before = editor_frame_rect(&editor, source);
        let target_before = editor_frame_rect(&editor, target);

        assert!(editor.apply_splitter_context_action(bounds, DockSplitterContextActionKind::Swap));

        assert_eq!(editor_frame_rect(&editor, source), target_before);
        assert_eq!(editor_frame_rect(&editor, target), source_before);
        assert_eq!(
            editor.status,
            format!(
                "Dock splitter swapped frame {} with frame {}",
                source.raw(),
                target.raw()
            )
        );
    }

    #[test]
    fn tab_drag_drop_uses_dock_drag_and_target_without_panel_metadata_mutation() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let bottom = bottom_frame_rect(&editor);
        let inspector = editor_frame_rect(&editor, FRAME_INSPECTOR);
        let timeline = editor
            .dock
            .frame(FRAME_BOTTOM)
            .and_then(|frame| {
                frame_tab_rects(frame, bottom, 26.0)
                    .into_iter()
                    .find(|(tab, _rect)| tab.panel == PANEL_TIMELINE)
                    .map(|(_tab, rect)| rect)
            })
            .expect("timeline tab");
        let start = timeline.center();
        let target = inspector.center();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(start.x, start.y, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let drag_delta = Vec2::new(target.x - start.x, target.y - start.y);
        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at_with_delta(
                target.x, target.y, true, false, false, drag_delta,
            )),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let dragging_output = ui.finish_output();

        assert_eq!(editor.status, "Dragging Timeline tab");
        assert!(dragging_output.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect)
                    if matches!(&rect.fill, Some(Brush::Solid(color)) if *color == rgba(78, 142, 245, 0.18))
                        && inspector.contains_point(rect.rect.center())
            )
        }));

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(target.x, target.y, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let inspector_frame = editor.dock.frame(FRAME_INSPECTOR).expect("inspector frame");
        let timeline_panel = inspector_frame
            .panels
            .iter()
            .find(|panel| panel.id == PANEL_TIMELINE)
            .expect("moved timeline panel");

        assert_eq!(timeline_panel.title, "Timeline");
        assert_eq!(
            inspector_frame.active_panel().map(|panel| panel.id),
            Some(PANEL_TIMELINE)
        );
        assert_eq!(editor.dock.active_frame(), Some(FRAME_INSPECTOR));
        assert!(
            !editor
                .dock
                .frame(FRAME_BOTTOM)
                .expect("bottom frame")
                .panels
                .iter()
                .any(|panel| panel.id == PANEL_TIMELINE)
        );
        assert!(editor.status.contains("Dock tab merged into frame"));
        assert_eq!(output.repaint, RepaintRequest::NextFrame);

        let registry = super::editor_panel_registry();
        let moved_workspace = editor
            .dock
            .workspace_snapshot(super::editor_panel_instances());
        moved_workspace
            .validate(registry.descriptors())
            .expect("moved workspace validates");
        let moved_timeline = moved_workspace
            .panel_instances
            .iter()
            .find(|instance| instance.id == PANEL_TIMELINE.instance_id())
            .expect("timeline instance metadata");
        assert_eq!(moved_timeline.panel_type, super::PANEL_TYPE_TIMELINE);
        assert_eq!(moved_timeline.title, "Timeline");
        assert_eq!(moved_timeline.state_key.as_deref(), Some("editor.timeline"));
        let restored =
            super::Dock::restore_workspace(moved_workspace.clone(), registry.descriptors())
                .expect("moved workspace restores");
        assert_eq!(restored.snapshot(), moved_workspace.dock);
    }

    #[test]
    fn viewport_selection_overlay_uses_scaled_content_mapping() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let viewport_frame = editor_frame_rect(&editor, FRAME_VIEWPORT);
        let viewport_body = frame_body_rect(viewport_frame);
        let surface_bounds = Rect::new(
            viewport_body.x + 8.0,
            viewport_body.y + 36.0,
            (viewport_body.width - 16.0).max(1.0),
            (viewport_body.height - 66.0).max(1.0),
        );
        let surface = ViewportSurface {
            texture: super::VIEWPORT_TEXTURE,
            source_size: VIEWPORT_SIZE,
            bounds: surface_bounds,
            pan_zoom: editor.viewport_pan_zoom,
        };
        let scale = ScaleFactor::new(1.25);
        let expected = surface
            .content_rect_to_screen_at(Rect::new(720.0, 210.0, 210.0, 280.0), scale)
            .expect("selection rect");

        let mut ui = Ui::begin_frame(
            editor_test_context_scaled(UiInput::default(), scale),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let selection_fill = rgba(78, 142, 245, 0.12);
        let selection = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect)
                    if matches!(&rect.fill, Some(Brush::Solid(color)) if *color == selection_fill) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("selection overlay rect");

        assert_eq!(selection, expected);
        let physical_x = f64::from(selection.x) * scale.value();
        let physical_width = f64::from(selection.width) * scale.value();
        assert!((physical_x - physical_x.round()).abs() < 0.001);
        assert!((physical_width - physical_width.round()).abs() < 0.001);
    }

    #[test]
    fn scene_expander_flips_arrow_and_requests_repaint_same_frame() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let expander = Point::new(38.0, super::workspace_top(&theme) + 100.0);

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(expander.x, expander.y, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(expander.x, expander.y, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert_eq!(output.repaint, RepaintRequest::NextFrame);
        assert!(!editor.scene_expansion.is_expanded(item_id(2)));
        assert!(
            output.primitives.iter().any(|primitive| {
                matches!(primitive, Primitive::Text(text) if text.text == ">")
            })
        );
    }

    #[test]
    fn outside_click_dismisses_menu_and_requests_repaint() {
        let mut editor = EditorShowcase::new();
        editor.open_menu = Some(EditorMenuKind::File);
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(900.0, 700.0, false, false, true)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert_eq!(editor.open_menu, None);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
    }

    #[test]
    fn icon_atlas_duplicates_edge_pixels_into_gutters() {
        let first = phosphor_icons::ICON_ENTRIES
            .iter()
            .find(|entry| entry.logical_size == phosphor_icons::STANDARD_ICON_LOGICAL_SIZE)
            .expect("standard icon entry");
        let atlas = icon_atlas_image(first.physical_size).expect("atlas");
        let source = first.source;
        let left_gutter = atlas_pixel(
            &atlas.data,
            atlas.width,
            source.x as u32 - phosphor_icons::ICON_ATLAS_PADDING,
            source.y as u32,
        );
        let first_inner = atlas_pixel(&atlas.data, atlas.width, source.x as u32, source.y as u32);
        let bottom_gutter = atlas_pixel(
            &atlas.data,
            atlas.width,
            source.max_x() as u32,
            source.max_y() as u32,
        );
        let bottom_inner = atlas_pixel(
            &atlas.data,
            atlas.width,
            source.max_x() as u32 - 1,
            source.max_y() as u32 - 1,
        );
        let atlas_entry = phosphor_icons::ICON_ATLASES
            .iter()
            .find(|atlas| atlas.image == first.atlas)
            .expect("atlas entry");

        assert_eq!(atlas.width, atlas_entry.width);
        assert_eq!(atlas.height, atlas_entry.height);
        assert_eq!(left_gutter, first_inner);
        assert_eq!(bottom_gutter, bottom_inner);
    }

    #[test]
    fn icon_manifest_entries_register_as_atlas_regions() {
        let mut resources = RenderResources::new();

        register_resources(&mut resources);

        for entry in phosphor_icons::ICON_ENTRIES {
            let resource = resources.image(entry.image).expect(entry.symbol);
            let region = resource.atlas_region.expect("icon atlas region");

            assert_eq!(
                resource.size,
                Size::new(entry.logical_size as f32, entry.logical_size as f32)
            );
            assert_eq!(
                resource.sampling,
                kinetik_ui::render::RenderImageSampling::UiIcon
            );
            assert_eq!(region.atlas, entry.atlas);
            assert_eq!(region.source, entry.source, "{}", entry.source_name);
        }
    }

    #[test]
    fn icon_atlas_regions_target_inner_unpadded_cells() {
        let mut resources = RenderResources::new();

        register_resources(&mut resources);

        let entry = phosphor_icons::ICON_ENTRIES
            .iter()
            .find(|entry| {
                entry.icon == phosphor_icons::PhosphorIcon::Crosshair
                    && entry.logical_size == phosphor_icons::STANDARD_ICON_LOGICAL_SIZE
                    && entry.physical_size == 24
            })
            .expect("crosshair entry");
        let region = resources
            .image(entry.image)
            .and_then(|resource| resource.atlas_region)
            .expect("icon region");

        assert_eq!(region.source.width, entry.physical_size as f32);
        assert_eq!(region.source.height, entry.physical_size as f32);
        assert_eq!(region.source, entry.source);
        assert_eq!(entry.source_name, "crosshair");
    }

    #[test]
    fn editor_structural_smoke_emits_dock_frame_panel_viewport_and_action_categories() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        let invocations = editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert!(invocations.is_empty());
        assert_eq!(output.warnings, Vec::new());
        assert!(output.primitives.len() > 200);
        assert!(
            count_primitives(&output.primitives, |primitive| matches!(
                primitive,
                Primitive::Rect(_)
            )) > 100
        );
        assert!(
            count_primitives(&output.primitives, |primitive| matches!(
                primitive,
                Primitive::Text(_)
            )) > 50
        );
        assert!(
            count_primitives(&output.primitives, |primitive| matches!(
                primitive,
                Primitive::Image(_)
            )) >= 24
        );
        assert!(
            count_primitives(&output.primitives, |primitive| matches!(
                primitive,
                Primitive::Texture(_)
            )) >= 1
        );
        assert!(
            count_primitives(&output.primitives, |primitive| matches!(
                primitive,
                Primitive::Line(_)
            )) >= 8
        );
        assert!(
            count_primitives(&output.primitives, |primitive| matches!(
                primitive,
                Primitive::ClipBegin { .. }
            )) >= 2
        );
        assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Texture(texture) if texture.texture == super::VIEWPORT_TEXTURE)
        }));
        assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "CameraPreview")
        }));

        assert_eq!(count_semantic_role(&output, &SemanticRole::Dock), 1);
        assert!(count_semantic_role(&output, &SemanticRole::Frame) >= 5);
        assert!(count_semantic_role(&output, &SemanticRole::Panel) >= 5);
        assert!(count_semantic_role(&output, &SemanticRole::Viewport) >= 1);
        assert!(count_semantic_role(&output, &SemanticRole::Tab) >= 6);
        assert!(count_semantic_role(&output, &SemanticRole::IconButton) >= 12);
        assert!(count_semantic_role(&output, &SemanticRole::Slider) >= 1);
        assert!(output.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::IconButton
                && node.label.as_deref() == Some("Play")
                && node
                    .actions
                    .iter()
                    .any(|action| action.kind == SemanticActionKind::Invoke)
        }));
        assert!(output.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Slider
                && node
                    .actions
                    .iter()
                    .any(|action| action.kind == SemanticActionKind::SetValue)
        }));
    }

    #[test]
    fn editor_uses_phosphor_atlas_primitives_for_visible_editor_icons() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let atlas_icon_count = output
            .primitives
            .iter()
            .filter(|primitive| {
                matches!(primitive, Primitive::Image(image) if is_editor_icon(image.image))
            })
            .count();

        assert!(
            atlas_icon_count >= 24,
            "visible Phosphor icon count was {atlas_icon_count}"
        );
    }

    #[test]
    fn editor_toolbar_icons_use_tinted_bitmap_atlas() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let toolbar_bitmap_icons = output
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Image(image)
                    if is_editor_icon(image.image) && point_is_in_toolbar(image.rect.center()) =>
                {
                    Some(image)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(toolbar_bitmap_icons.len() >= 12);
        assert!(
            toolbar_bitmap_icons
                .iter()
                .all(|image| image.tint.is_some())
        );
    }

    #[test]
    fn editor_toolbar_atlas_icons_use_integer_logical_destinations() {
        let theme = default_dark_theme();
        let chrome = EditorChromeMetrics::from_theme(&theme);
        let mut memory = UiMemory::new();
        let context = editor_test_context_scaled(UiInput::default(), ScaleFactor::new(1.25));
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let mut checked = 0;

        for primitive in &output.primitives {
            let Primitive::Image(image) = primitive else {
                continue;
            };
            if !is_editor_icon(image.image) || !point_is_in_toolbar(image.rect.center()) {
                continue;
            }
            assert_eq!(image.rect.x, image.rect.x.round());
            assert_eq!(image.rect.y, image.rect.y.round());
            assert_eq!(image.rect.width, chrome.toolbar_icon);
            assert_eq!(image.rect.height, chrome.toolbar_icon);
            checked += 1;
        }

        assert!(checked >= 12);
    }

    #[test]
    fn editor_icons_pick_exact_physical_atlas_for_dpi_scale() {
        let theme = default_dark_theme();
        let chrome = EditorChromeMetrics::from_theme(&theme);
        let dense = phosphor_icons::icon_image(
            phosphor_icons::PhosphorIcon::Search,
            super::DENSE_ICON_SIZE,
            1.25,
        );
        let toolbar = phosphor_icons::icon_image(
            phosphor_icons::PhosphorIcon::Cursor,
            chrome.toolbar_icon,
            1.5,
        );
        let dense_entry = icon_entry(dense);
        let toolbar_entry = icon_entry(toolbar);

        assert_eq!(dense_entry.logical_size, 16);
        assert_eq!(dense_entry.physical_size, 20);
        assert_eq!(toolbar_entry.logical_size, 16);
        assert_eq!(toolbar_entry.physical_size, 24);

        let fallback = phosphor_icons::icon_image(
            phosphor_icons::PhosphorIcon::Search,
            super::DENSE_ICON_SIZE,
            1.33,
        );
        assert_eq!(icon_entry(fallback).physical_size, 24);
    }

    #[test]
    fn toolbar_icon_size_leaves_padding_inside_button() {
        let theme = default_dark_theme();
        let chrome = EditorChromeMetrics::from_theme(&theme);
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();
        let first_button = Rect::new(
            10.0,
            TOOLBAR_Y,
            chrome.toolbar_button,
            chrome.toolbar_button,
        );

        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let first_icon = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Image(image)
                    if is_editor_icon(image.image)
                        && first_button.contains_point(image.rect.center()) =>
                {
                    Some(image)
                }
                _ => None,
            })
            .expect("first toolbar icon");

        assert!(first_icon.rect.x >= first_button.x + 4.0);
        assert!(first_icon.rect.max_x() <= first_button.max_x() - 4.0);
        assert!(first_icon.rect.y >= first_button.y + 4.0);
        assert!(first_icon.rect.max_y() <= first_button.max_y() - 4.0);
    }
    #[test]
    fn editor_toolbar_atlas_icons_preserve_icon_button_semantics() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();
        let toolbar_labels = [
            "Select",
            "Move",
            "Rotate",
            "Scale",
            "Toggle grid",
            "Frame selected",
            "Reset view",
            "Play",
            "Pause",
            "Stop",
            "Build",
            "Export",
        ];

        for label in toolbar_labels {
            assert!(
                output.semantics.nodes().iter().any(|node| {
                    node.role == SemanticRole::IconButton
                        && node.label.as_deref() == Some(label)
                        && node.focusable
                }),
                "missing toolbar icon semantics for {label}"
            );
        }
    }

    #[test]
    fn editor_toolbar_atlas_icons_request_hover_cursor() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(pointer_input_at(20.0, 44.0, false, false, false));
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert!(
            output
                .platform_requests
                .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
        );
    }

    fn editor_test_context(input: UiInput) -> FrameContext {
        editor_test_context_scaled(input, ScaleFactor::ONE)
    }

    fn editor_test_context_scaled(input: UiInput, scale_factor: ScaleFactor) -> FrameContext {
        FrameContext::new(
            ViewportInfo::new(
                Size::new(1440.0, 900.0),
                PhysicalSize::new(
                    (1440.0 * scale_factor.value()).round() as u32,
                    (900.0 * scale_factor.value()).round() as u32,
                ),
                scale_factor,
            ),
            input,
            TimeInfo::default(),
        )
    }

    fn bottom_frame_rect(editor: &EditorShowcase) -> Rect {
        editor_frame_rect(editor, FRAME_BOTTOM)
    }

    fn editor_frame_rect(editor: &EditorShowcase, frame: super::FrameId) -> Rect {
        solve_dock_layout(&editor.dock, editor_workspace_bounds())
            .into_iter()
            .find(|layout| layout.frame == frame)
            .map(|layout| layout.rect.inset(2.0))
            .expect("editor frame")
    }

    fn editor_workspace_bounds() -> Rect {
        let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
        let theme = default_dark_theme();
        let workspace_top = super::workspace_top(&theme);
        Rect::new(
            4.0,
            workspace_top,
            viewport.width - 8.0,
            viewport.height - workspace_top - 28.0,
        )
    }

    fn frame_body_rect(frame_rect: Rect) -> Rect {
        let tab_height = 26.0;
        Rect::new(
            frame_rect.x + 1.0,
            frame_rect.y + tab_height + 2.0,
            (frame_rect.width - 2.0).max(0.0),
            (frame_rect.height - tab_height - 3.0).max(0.0),
        )
    }

    fn point_is_in_toolbar(point: Point) -> bool {
        let chrome = EditorChromeMetrics::from_theme(&default_dark_theme());
        point.y >= TOOLBAR_Y && point.y <= TOOLBAR_Y + chrome.toolbar_button
    }

    fn count_primitives(primitives: &[Primitive], predicate: impl Fn(&Primitive) -> bool) -> usize {
        primitives
            .iter()
            .filter(|primitive| predicate(primitive))
            .count()
    }

    fn count_semantic_role(output: &kinetik_ui::core::FrameOutput, role: &SemanticRole) -> usize {
        output
            .semantics
            .nodes()
            .iter()
            .filter(|node| &node.role == role)
            .count()
    }

    fn focused_frame_semantic_count(output: &kinetik_ui::core::FrameOutput) -> usize {
        output
            .semantics
            .nodes()
            .iter()
            .filter(|node| node.role == SemanticRole::Frame && node.state.focused)
            .count()
    }

    fn pointer_input_at(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
        pointer_input_at_with_delta(x, y, down, pressed, released, Vec2::ZERO)
    }

    fn pointer_input_at_with_delta(
        x: f32,
        y: f32,
        down: bool,
        pressed: bool,
        released: bool,
        delta: Vec2,
    ) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                delta,
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
        phosphor_icons::ICON_ENTRIES
            .iter()
            .any(|entry| entry.image == image)
    }

    fn icon_entry(image: kinetik_ui::core::ImageId) -> &'static phosphor_icons::PhosphorIconEntry {
        phosphor_icons::ICON_ENTRIES
            .iter()
            .find(|entry| entry.image == image)
            .expect("icon entry")
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
    icon: ToolbarIcon,
}

const ASSETS: &[Asset] = &[
    Asset {
        name: "camp_scene",
        kind: "scene",
        icon: ToolbarIcon::Cube,
    },
    Asset {
        name: "terrain_forest",
        kind: "mesh",
        icon: ToolbarIcon::Box,
    },
    Asset {
        name: "van_body",
        kind: "mesh",
        icon: ToolbarIcon::Component,
    },
    Asset {
        name: "campfire",
        kind: "prefab",
        icon: ToolbarIcon::Tokens,
    },
    Asset {
        name: "night_sky",
        kind: "texture",
        icon: ToolbarIcon::Image,
    },
    Asset {
        name: "hero_ctrl",
        kind: "script",
        icon: ToolbarIcon::Code,
    },
    Asset {
        name: "audio_loop",
        kind: "asset",
        icon: ToolbarIcon::Archive,
    },
    Asset {
        name: "lighting_lut",
        kind: "texture",
        icon: ToolbarIcon::Image,
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
        message: "Registered 28 Phosphor toolbar icons",
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

fn scene_icon(id: ItemId) -> ToolbarIcon {
    match id.raw() {
        1 => ToolbarIcon::Layers,
        2 | 6 => ToolbarIcon::Caret,
        3 => ToolbarIcon::Eye,
        4 => ToolbarIcon::Crosshair,
        5 => ToolbarIcon::Grid,
        7 => ToolbarIcon::Component,
        8 | 9 => ToolbarIcon::Cube,
        10 => ToolbarIcon::Rocket,
        11 => ToolbarIcon::Archive,
        _ => ToolbarIcon::Box,
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

fn toolbar_icon_button(
    ui: &mut Ui<'_>,
    key: impl std::hash::Hash,
    rect: Rect,
    icon: ToolbarIcon,
    label: &str,
    selected: bool,
    disabled: bool,
) -> kinetik_ui::core::Response {
    toolbar_icon_button_sized(
        ui,
        key,
        rect,
        icon,
        label,
        selected,
        disabled,
        EditorChromeMetrics::from_theme(ui.theme()).toolbar_icon,
    )
}

#[allow(clippy::too_many_arguments)]
fn toolbar_icon_button_sized(
    ui: &mut Ui<'_>,
    key: impl std::hash::Hash,
    rect: Rect,
    icon: ToolbarIcon,
    label: &str,
    selected: bool,
    disabled: bool,
    icon_size: f32,
) -> kinetik_ui::core::Response {
    let id = ui.id(key);
    let response = ui.pressable_with_id(id, rect, disabled);
    let visual_selected = selected || response.clicked;
    let fill = if disabled {
        rgb(24, 25, 28)
    } else if visual_selected {
        rgb(39, 69, 122)
    } else if response.state.pressed {
        rgb(35, 37, 42)
    } else if response.state.hovered {
        rgb(31, 33, 38)
    } else {
        rgb(24, 25, 28)
    };
    let stroke = if visual_selected {
        rgb(83, 137, 230)
    } else {
        rgb(58, 61, 68)
    };
    let color = if disabled {
        rgb(112, 118, 128)
    } else if visual_selected {
        rgb(246, 248, 252)
    } else {
        rgb(218, 223, 232)
    };

    rect_fill(ui, rect, fill, Some(stroke), CornerRadius::all(4.0));
    let icon_size = clamped_icon_size(icon_size, rect);
    draw_tinted_icon(ui, rect, icon, icon_size, color);

    let mut semantics = icon_button_semantics(id, rect, label, disabled);
    semantics.state.focused = response.state.focused;
    semantics.state.pressed = response.state.pressed;
    semantics.state.selected = visual_selected;
    ui.push_semantic_node(semantics);
    if response.state.hovered && !disabled {
        ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
    }

    response
}

fn draw_icon(ui: &mut Ui<'_>, bounds: Rect, icon: ToolbarIcon, size: f32) {
    draw_tinted_icon(ui, bounds, icon, size, rgb(205, 212, 222));
}

fn draw_tinted_icon(ui: &mut Ui<'_>, bounds: Rect, icon: ToolbarIcon, size: f32, color: Color) {
    let size = clamped_icon_size(size, bounds);
    let rect = Rect::new(
        bounds.x + (bounds.width - size) * 0.5,
        bounds.y + (bounds.height - size) * 0.5,
        size,
        size,
    );
    ui.primitive(Primitive::Image(ImagePrimitive {
        image: phosphor_icons::icon_image(
            icon.phosphor(),
            size,
            ui.viewport().scale_factor.value(),
        ),
        rect,
        tint: Some(color),
    }));
}

fn clamped_icon_size(size: f32, bounds: Rect) -> f32 {
    let requested = if size.is_finite() && size > 0.0 {
        size
    } else {
        DENSE_ICON_SIZE
    };
    let available = bounds.width.min(bounds.height).max(1.0);
    requested.min(available)
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
