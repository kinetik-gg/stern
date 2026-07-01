//! DCC-style editor showcase surface.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::time::Duration;

use kinetik_ui::core::{
    ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionQueue,
    ActionSource, Axis, Brush, ClipId, Color, CornerRadius, CursorShape, ImagePrimitive, Key,
    KeyState, LinePrimitive, Modifiers, PlatformRequest, Point, Primitive, Rect, RectPrimitive,
    RepaintRequest, Response, SemanticNode, SemanticRole, Shortcut, Size, Stroke, TextPrimitive,
    TextureId, Theme, Vec2, WidgetId,
};
use kinetik_ui::render::{
    ImageAtlasRegion, ImageResource, RenderImage, RenderImageSampling, RenderResources,
    TextureResource,
};
use kinetik_ui::text::TextEditState;
use kinetik_ui::widgets::{
    AssetSlotAsset, AssetSlotConfig, DiagnosticSource, DiagnosticStrip, DiagnosticStripItem,
    DiagnosticStripItemId, DiagnosticStripSeverity, Dock, DockChromeStyle, DockDropTarget,
    DockDropZone, DockInteractionPolicy, DockNode, DockPlacement, DockSplitterContextActionKind,
    DockTabDrag, DropdownItem, DropdownItemId, DropdownModel, EdgeDescriptor, EdgeId,
    FeedbackAction, FeedbackDismiss, FeedbackId, FeedbackItem, FeedbackKind, FeedbackStack, Frame,
    FrameId, FrameLayout, FrameTab, GraphPoint, GraphRect, GraphVector, GridColumns, GridLayout,
    Guide, ItemId, JobCancel, JobList, JobPhase, JobProgress, JobRow, JobRowId, ListLayout, Menu,
    MenuBar, MenuBarMenu, MenuBarMenuId, MenuBarOverlayRequest, MenuItem, MenuOverlay, ModalAction,
    ModalActionRole, ModalDialog, ModalDialogOverlay, NodeDescriptor, NodeFrameDescriptor,
    NodeFrameId, NodeGraphDescriptor, NodeGraphEdgeRoutePoint, NodeGraphEmissionError,
    NodeGraphPanZoom, NodeGraphSelection, NodeGraphSelectionTarget, NodeGraphStaticOutput,
    NodeGraphStaticView, NodeGraphViewport, NodeGroupDescriptor, NodeGroupId, NodeId,
    NumericScrubInputConfig, OverlayDismissal, OverlayId, OverlayKind, OverlayStack, PanZoom,
    Panel, PanelId, PanelInstanceId, PanelInstancePolicy, PanelInstanceSnapshot,
    PanelOpenActionMetadata, PanelOpenDecision, PanelRegistry, PanelTypeCategory,
    PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext, PathFieldConfig, PopoverPlacement,
    PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId, PropertyGridAffordanceLayout,
    PropertyGridLayout, PropertyGridRow, PropertyGridRowStatus, RerouteDescriptor, RerouteId,
    SelectFieldConfig, StatusBar, StatusItem, StatusItemId, StatusItemKind, StatusProgress,
    TabStrip, TableColumn, TableLayout, Toolbar, ToolbarGroup, ToolbarGroupId, ToolbarItem,
    ToolbarItemPresentation, TreeExpansion, TreeItem, TreeLayout, TreeModel, Ui,
    VectorScrubInputConfig, ViewportComposition, ViewportFit, ViewportSurface, WorkspaceSnapshot,
    classify_numeric_input_draft, frame_tabs, icon_button_semantics,
    property_grid_row_affordance_rects, resolve_dock_splitter_context_actions_with_policy,
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
const ACTION_VIEWPORT_FOCUS_SELECTED: &str = "editor.viewport.focus-selected";
const ACTION_VIEWPORT_FIT_CONTENT: &str = "editor.viewport.fit-content";
const ACTION_VIEWPORT_FIT_SELECTION: &str = "editor.viewport.fit-selection";
const ACTION_VIEWPORT_ACTUAL_SIZE: &str = "editor.viewport.actual-size";
const ACTION_VIEWPORT_ZOOM_IN: &str = "editor.viewport.zoom-in";
const ACTION_VIEWPORT_ZOOM_OUT: &str = "editor.viewport.zoom-out";
const ACTION_VIEWPORT_PAN: &str = "editor.viewport.pan";
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
const ACTION_CANCEL_ACTIVE_FIXTURE_JOB: &str = "editor.jobs.cancel-active-fixture";
const ACTION_CANCEL_QUEUED_FIXTURE_JOB: &str = "editor.jobs.cancel-queued-fixture";
const ACTION_OPEN_FEEDBACK_REPORT: &str = "editor.feedback.open-report";
const ACTION_DISMISS_FEEDBACK_REPORT: &str = "editor.feedback.dismiss-report";

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

    const fn symbol(self) -> &'static str {
        match self {
            Self::Select => "cursor",
            Self::Move => "move",
            Self::Rotate => "rotate",
            Self::Scale => "transform",
            Self::Grid => "grid",
            Self::Crosshair => "crosshair",
            Self::Reset => "reset",
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Stop => "stop",
            Self::Rocket => "rocket",
            Self::Download => "download",
            Self::Plus => "plus",
            Self::Dots => "dots",
            Self::Search => "search",
            Self::Gear => "gear",
            Self::Layers => "layers",
            Self::Caret => "caret",
            Self::Eye => "eye",
            Self::Component => "component",
            Self::Cube => "cube",
            Self::Archive => "archive",
            Self::Image => "image",
            Self::Code => "code",
            Self::Tokens => "tokens",
            Self::Box => "box",
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

    const fn menu_bar_id(self) -> MenuBarMenuId {
        MenuBarMenuId::from_raw(self.raw())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorToolbarGroupKind {
    Tools,
    Viewport,
    Dock,
    Run,
}

impl EditorToolbarGroupKind {
    const fn id(self) -> ToolbarGroupId {
        ToolbarGroupId::from_raw(match self {
            Self::Tools => 1,
            Self::Viewport => 2,
            Self::Dock => 3,
            Self::Run => 4,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorStatusItemKind {
    Message,
    Actions,
    Snap,
    Backend,
    Jobs,
    Diagnostics,
    Feedback,
    Timeline,
}

impl EditorStatusItemKind {
    const fn id(self) -> StatusItemId {
        StatusItemId::from_raw(match self {
            Self::Message => 1,
            Self::Actions => 2,
            Self::Snap => 3,
            Self::Backend => 4,
            Self::Jobs => 5,
            Self::Diagnostics => 6,
            Self::Feedback => 7,
            Self::Timeline => 8,
        })
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
    position: [f32; 3],
    position_states: [TextEditState; 3],
    scale: TextEditState,
    mass: TextEditState,
    collider_kind: DropdownItemId,
    script_path: TextEditState,
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
            position: [12.0, 1.5, -6.0],
            position_states: [
                TextEditState::new("12.0"),
                TextEditState::new("1.5"),
                TextEditState::new("-6.0"),
            ],
            scale: TextEditState::new("1.0"),
            mass: TextEditState::new("84.0"),
            collider_kind: DropdownItemId::from_raw(2),
            script_path: TextEditState::new("scripts/hero_ctrl.rs"),
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
            ACTION_VIEWPORT_FOCUS_SELECTED => {
                "Viewport focus selected requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_FIT_CONTENT => {
                "Viewport fit content requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_FIT_SELECTION => {
                "Viewport fit selection requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_ACTUAL_SIZE => {
                "Viewport actual size requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_ZOOM_IN => {
                "Viewport zoom in requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_ZOOM_OUT => {
                "Viewport zoom out requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_PAN => {
                "Viewport pan mode requested".clone_into(&mut self.status);
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
        self.tool_bar(ui, viewport, &mut invocations);
        self.menu_bar(ui, viewport);
        self.workspace(ui, viewport);
        self.menu_overlay(ui, viewport, &mut invocations);
        let _modal_metadata = self.about_modal_overlay_model(viewport);
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

    fn menu_bar_model(&self) -> MenuBar {
        let mut menu_bar =
            MenuBar::from_menus(menu_header_rects().into_iter().map(|(kind, label, _)| {
                MenuBarMenu::new(kind.menu_bar_id(), label, self.menu_model(kind))
            }));
        if let Some(kind) = self.open_menu {
            menu_bar.open(kind.menu_bar_id());
        }
        menu_bar
    }

    fn toolbar_model(&self) -> Toolbar {
        let tool_items = EDITOR_TOOL_BUTTONS
            .into_iter()
            .map(|(tool, icon, label, action)| {
                ToolbarItem::new(toolbar_action(
                    action,
                    label,
                    icon,
                    Some(self.selected_tool == tool),
                    true,
                ))
                .with_presentation(ToolbarItemPresentation::IconOnly)
            });

        let viewport_items = [
            ToolbarItem::new(toolbar_action(
                ACTION_GRID,
                "Toggle grid",
                ToolbarIcon::Grid,
                Some(self.grid_visible),
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_VIEWPORT_FIT_SELECTION,
                "Frame selected",
                ToolbarIcon::Crosshair,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_VIEWPORT_FIT_CONTENT,
                "Reset view",
                ToolbarIcon::Reset,
                None,
                true,
            )),
        ];

        let dock_items = [
            ToolbarItem::new(toolbar_action(
                ACTION_DOCK_JOIN,
                "Join dock splitter",
                ToolbarIcon::Component,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_DOCK_SWAP,
                "Swap dock frames",
                ToolbarIcon::Layers,
                None,
                true,
            )),
        ];

        let run_items = [
            ToolbarItem::new(toolbar_action(
                ACTION_PLAY,
                "Play",
                ToolbarIcon::Play,
                Some(self.running),
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_PLAY,
                "Pause",
                ToolbarIcon::Pause,
                Some(!self.running),
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_STOP,
                "Stop",
                ToolbarIcon::Stop,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_BUILD,
                "Build",
                ToolbarIcon::Rocket,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_BUILD,
                "Export",
                ToolbarIcon::Download,
                None,
                true,
            )),
        ];

        Toolbar::from_groups([
            ToolbarGroup::new(EditorToolbarGroupKind::Tools.id(), "Tools", tool_items),
            ToolbarGroup::new(
                EditorToolbarGroupKind::Viewport.id(),
                "Viewport",
                viewport_items,
            ),
            ToolbarGroup::new(EditorToolbarGroupKind::Dock.id(), "Dock", dock_items),
            ToolbarGroup::new(EditorToolbarGroupKind::Run.id(), "Run", run_items),
        ])
    }

    fn status_bar_model(&self, action_count: u32) -> StatusBar {
        let jobs = Self::showcase_job_list();
        let job_summary = jobs.summary();
        let job_progress = jobs.active_status_progress();
        let diagnostics = Self::showcase_diagnostics();
        let diagnostic_summary = diagnostics.summary();
        let feedback = Self::showcase_feedback_stack();
        let active_feedback = feedback.active_items(showcase_feedback_now());

        StatusBar::from_items([
            StatusItem::new(
                EditorStatusItemKind::Message.id(),
                "Status",
                self.status.clone(),
                StatusItemKind::Message,
            ),
            StatusItem::new(
                EditorStatusItemKind::Actions.id(),
                "Actions",
                format!("Actions: {action_count}"),
                StatusItemKind::ActionCount,
            )
            .with_count(action_count),
            StatusItem::new(
                EditorStatusItemKind::Snap.id(),
                "Snap",
                if self.snap_enabled {
                    "Snap 1m"
                } else {
                    "Snap off"
                },
                if self.snap_enabled {
                    StatusItemKind::Ready
                } else {
                    StatusItemKind::Stale
                },
            ),
            StatusItem::new(
                EditorStatusItemKind::Backend.id(),
                "Backend",
                "Vello / winit",
                StatusItemKind::Ready,
            ),
            StatusItem::new(
                EditorStatusItemKind::Jobs.id(),
                "Jobs",
                format!(
                    "Jobs: {} active / {} total",
                    job_summary.active(),
                    job_summary.total()
                ),
                StatusItemKind::JobCount,
            )
            .with_count(job_summary.active())
            .with_progress(job_progress.unwrap_or_else(|| StatusProgress::new(0.0))),
            StatusItem::new(
                EditorStatusItemKind::Diagnostics.id(),
                "Diagnostics",
                format!(
                    "Diagnostics: {}E {}W {}I",
                    diagnostic_summary.errors, diagnostic_summary.warnings, diagnostic_summary.info
                ),
                if diagnostic_summary.errors > 0 {
                    StatusItemKind::Error
                } else if diagnostic_summary.warnings > 0 {
                    StatusItemKind::Stale
                } else {
                    StatusItemKind::Ready
                },
            )
            .with_count(diagnostic_summary.total()),
            StatusItem::new(
                EditorStatusItemKind::Feedback.id(),
                "Feedback",
                format!("Feedback: {}", active_feedback.len()),
                StatusItemKind::Message,
            )
            .with_count(active_feedback.len() as u32),
            StatusItem::new(
                EditorStatusItemKind::Timeline.id(),
                "Timeline",
                format!("Timeline: {:.0}%", self.timeline * 100.0),
                StatusItemKind::Progress,
            )
            .with_progress(StatusProgress::new(self.timeline))
            .with_visible(false),
        ])
    }

    fn about_modal_overlay_model(&self, viewport: Rect) -> ModalDialogOverlay {
        let _ = self;
        let dialog = ModalDialog::new(WidgetId::from_raw(40_001), "About Kinetik Forge")
            .with_body("Kinetik Forge editor showcase chrome is action-driven and data-only.")
            .with_actions([
                ModalAction::new(
                    modal_action(ACTION_PALETTE, "Open Docs", true),
                    ModalActionRole::Primary,
                ),
                ModalAction::new(
                    modal_action(ACTION_PALETTE, "Close", true),
                    ModalActionRole::Cancel,
                ),
            ]);
        let size = Size::new(360.0, 168.0);
        let rect = Rect::new(
            viewport.x + (viewport.width - size.width).max(0.0) * 0.5,
            viewport.y + (viewport.height - size.height).max(0.0) * 0.5,
            size.width,
            size.height,
        );
        ModalDialogOverlay::placed(
            OverlayId::from_raw(30_001),
            rect,
            dialog,
            OverlayDismissal::OutsideClickOrEscape,
            ActionContext::Editor,
        )
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
        let menu_bar = self.menu_bar_model();
        for ((kind, _label, rect), menu) in
            menu_header_rects().into_iter().zip(menu_bar.menus().iter())
        {
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
            debug_assert_eq!(menu.id, kind.menu_bar_id());
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
                &menu.title,
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
        let mut menu_bar = self.menu_bar_model();
        menu_bar.open(kind.menu_bar_id());
        menu_bar
            .active_overlay(MenuBarOverlayRequest {
                overlay_id: OverlayId::from_raw(10_000 + kind.raw()),
                kind: OverlayKind::Menu,
                anchor: menu_anchor(kind),
                size: menu_size(kind),
                placement: PopoverPlacement::Below,
                offset: 2.0,
                fit_viewport: true,
                viewport,
                dismissal: OverlayDismissal::OutsideClickOrEscape,
                source: ActionSource::Menu,
                context: ActionContext::Editor,
            })
            .expect("editor menu-bar active menu should convert to overlay")
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
                    ACTION_VIEWPORT_FOCUS_SELECTED,
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
                menu_action(ACTION_VIEWPORT_FIT_CONTENT, "Reset View", None, None, true),
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
        let toolbar = self.toolbar_model();
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        let mut x = 10.0;
        let tool_items = toolbar
            .group(EditorToolbarGroupKind::Tools.id())
            .expect("editor toolbar declares tool group")
            .visible_items();
        let mut tool_responses = Vec::new();
        for (visible_index, ((_, icon, _label, action), item)) in
            EDITOR_TOOL_BUTTONS.into_iter().zip(tool_items).enumerate()
        {
            let button = Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button);
            let id = ui.id(("editor.tool", action));
            let disabled = !item.enabled();
            let response = ui.pressable_with_id(id, button, disabled);
            if response.clicked {
                ui.request_repaint(RepaintRequest::NextFrame);
                let mut queue = ActionQueue::new();
                if toolbar.invoke_group_visible(
                    EditorToolbarGroupKind::Tools.id(),
                    visible_index,
                    &mut queue,
                    ActionContext::Editor,
                ) {
                    self.handle_action_queue(invocations, &mut queue);
                }
            }
            tool_responses.push((
                id,
                response,
                button,
                EDITOR_TOOL_BUTTONS[visible_index].0,
                icon,
                item.label(),
                disabled,
            ));
            x += chrome.toolbar_stride;
        }
        for (id, response, button, tool, icon, label, disabled) in tool_responses {
            paint_toolbar_icon_button_sized(
                ui,
                id,
                response,
                button,
                icon,
                label,
                self.selected_tool == tool,
                disabled,
                chrome.toolbar_icon,
            );
        }

        rect(
            ui,
            Rect::new(x + 4.0, TOOLBAR_Y + 3.0, 1.0, chrome.toolbar_button - 6.0),
            rgb(57, 60, 66),
            None,
        );
        x += 18.0;
        let viewport_items = toolbar
            .group(EditorToolbarGroupKind::Viewport.id())
            .expect("editor toolbar declares viewport group")
            .visible_items();
        for ((icon, _label, action), item) in [
            (ToolbarIcon::Grid, "Toggle grid", ACTION_GRID),
            (
                ToolbarIcon::Crosshair,
                "Frame selected",
                ACTION_VIEWPORT_FIT_SELECTION,
            ),
            (
                ToolbarIcon::Reset,
                "Reset view",
                ACTION_VIEWPORT_FIT_CONTENT,
            ),
        ]
        .into_iter()
        .zip(viewport_items)
        {
            let response = toolbar_icon_button(
                ui,
                ("editor.viewport-tool", action, icon.raw()),
                Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button),
                icon,
                item.label(),
                false,
                !item.enabled(),
            );
            if response.clicked && item.can_invoke() {
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
        let dock_items = toolbar
            .group(EditorToolbarGroupKind::Dock.id())
            .expect("editor toolbar declares dock group")
            .visible_items();
        for ((kind, icon, _label, action), item) in [
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
        ]
        .into_iter()
        .zip(dock_items)
        {
            let response = toolbar_icon_button(
                ui,
                ("editor.dock-action", action),
                Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button),
                icon,
                item.label(),
                false,
                !item.enabled(),
            );
            if response.clicked && item.can_invoke() {
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

        let run_items = toolbar
            .group(EditorToolbarGroupKind::Run.id())
            .expect("editor toolbar declares run group")
            .visible_items();
        for ((index, icon, _label, action, rect), item) in run_toolbar_buttons(viewport, chrome)
            .into_iter()
            .zip(run_items)
        {
            let response = toolbar_icon_button(
                ui,
                ("editor.run", action, index),
                rect,
                icon,
                item.label(),
                false,
                !item.enabled(),
            );
            if response.clicked {
                let mut queue = ActionQueue::new();
                if toolbar.invoke_group_visible(
                    EditorToolbarGroupKind::Run.id(),
                    index,
                    &mut queue,
                    ActionContext::Editor,
                ) {
                    self.handle_action_queue(invocations, &mut queue);
                }
            }
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
        let tab_strip = frame_tab_strip(frame);
        for (index, (tab, tab_rect)) in frame_tab_rects(frame, frame_rect, tab_height)
            .into_iter()
            .enumerate()
        {
            let response = ui.draggable(
                ("editor.frame-tab.drag", frame_id.raw(), tab.panel.raw()),
                tab_rect,
                false,
            );
            if let Some(target) = tab_strip.drag_target_by_index(index)
                && let Some(drag) = self.dock.begin_tab_drag(frame_id, target.panel)
            {
                tab_drags.push((response.id, drag));
            }
            if response.clicked
                && let Some(target) = tab_strip.activation_target_by_index(index)
            {
                self.dock.select_panel(frame_id, target.panel);
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
                            let model_row = &rows[row.index];
                            let status = model_row.state.status.presentation();
                            let label_color = match status.severity {
                                kinetik_ui::widgets::PropertyGridStatusSeverity::None => {
                                    rgb(154, 160, 168)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Info => {
                                    rgb(126, 179, 236)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Warning => {
                                    rgb(232, 179, 90)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Error => {
                                    rgb(236, 96, 96)
                                }
                            };
                            rect(ui, row.rect, rgb(24, 25, 27), Some(rgb(38, 40, 45)));
                            if status.accented {
                                rect(
                                    ui,
                                    Rect::new(row.rect.x, row.rect.y, 3.0, row.rect.height),
                                    label_color,
                                    None,
                                );
                                text(
                                    ui,
                                    row.label_rect.max_x() - 10.0,
                                    row.label_rect.y + 16.0,
                                    match status.severity {
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Info => {
                                            "i"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Warning => {
                                            "!"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Error => {
                                            "x"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::None => "",
                                    },
                                    9.0,
                                    label_color,
                                );
                            }
                            text(
                                ui,
                                row.label_rect.x + 6.0,
                                row.label_rect.y + 16.0,
                                &model_row.label,
                                11.0,
                                label_color,
                            );
                            let affordance_rects = property_grid_row_affordance_rects(
                                model_row,
                                row.value_rect.inset(2.0),
                                PropertyGridAffordanceLayout::default(),
                            );
                            self.inspector_value(ui, model_row, affordance_rects.value_rect);
                            let affordance = ui.property_grid_row_affordance_controls(
                                ("editor.inspector.affordance", row.id.raw()),
                                model_row,
                                affordance_rects,
                            );
                            if affordance.reset_requested {
                                self.status = format!("Reset requested for {}", model_row.label);
                            } else if affordance.keyframe_toggle_requested {
                                let state = if affordance.requested_keyed {
                                    "add"
                                } else {
                                    "remove"
                                };
                                self.status =
                                    format!("Keyframe {state} requested for {}", model_row.label);
                            }
                        }
                    }
                }
            },
        );
    }

    fn inspector_value(&mut self, ui: &mut Ui<'_>, row: &PropertyGridRow, rect_value: Rect) {
        let id = row.id;
        let disabled = row.state.disabled;
        let read_only = row.state.read_only;
        match id.raw() {
            2 => {
                ui.vector3_scrub_input(
                    "editor.inspector.position",
                    rect_value,
                    "Position",
                    &mut self.position,
                    &mut self.position_states,
                    VectorScrubInputConfig::new(
                        NumericScrubInputConfig::new(0.1).with_fine_step(0.01),
                    )
                    .disabled(disabled)
                    .read_only(read_only),
                );
            }
            5 => {
                inspector_numeric_scrub(
                    ui,
                    "editor.inspector.scale",
                    rect_value,
                    &mut self.scale,
                    NumericScrubInputConfig::new(0.01)
                        .with_fine_step(0.001)
                        .with_min(0.0)
                        .disabled(disabled)
                        .read_only(read_only),
                );
            }
            7 => {
                ui.slider(
                    "editor.inspector.exposure",
                    rect_value,
                    &mut self.exposure,
                    0.0..=1.0,
                    disabled || read_only,
                );
            }
            8 => {
                ui.slider(
                    "editor.inspector.roughness",
                    rect_value,
                    &mut self.roughness,
                    0.0..=1.0,
                    disabled || read_only,
                );
            }
            9 => {
                let asset = self.material_asset();
                let slot = ui.asset_slot_field(
                    "editor.inspector.material",
                    rect_value,
                    "Material",
                    Some(&asset),
                    AssetSlotConfig::new("Drop material")
                        .accepts_drop(true)
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if slot.drop_received {
                    "Material drop requested".clone_into(&mut self.status);
                } else if slot.open_requested {
                    self.status = format!("Open material asset: {}", asset.label);
                } else if slot.pick_requested {
                    "Material asset picker requested".clone_into(&mut self.status);
                }
            }
            11 => {
                ui.toggle_value(
                    "editor.inspector.snap",
                    Rect::new(rect_value.x, rect_value.y + 2.0, 42.0, 18.0),
                    &mut self.snap_enabled,
                    disabled || read_only,
                );
            }
            13 => {
                inspector_numeric_scrub(
                    ui,
                    "editor.inspector.mass",
                    rect_value,
                    &mut self.mass,
                    NumericScrubInputConfig::new(0.5)
                        .with_fine_step(0.1)
                        .with_min(0.0)
                        .disabled(disabled)
                        .read_only(read_only),
                );
            }
            14 => {
                let model = self.collider_model();
                let select = ui.select_field(
                    "editor.inspector.collider",
                    rect_value,
                    "Collider",
                    &model,
                    SelectFieldConfig::new("Choose collider")
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if select.open_requested {
                    "Collider choices requested".clone_into(&mut self.status);
                }
            }
            15 => {
                let path = ui.path_field(
                    "editor.inspector.script",
                    rect_value,
                    "Script path",
                    &mut self.script_path,
                    PathFieldConfig::default()
                        .open(true)
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if path.browse_requested {
                    "Script path browse requested".clone_into(&mut self.status);
                } else if path.open_requested {
                    self.status = format!("Open script path: {}", self.script_path.text);
                }
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

    fn material_asset(&self) -> AssetSlotAsset {
        let asset = &ASSETS[self.selected_asset.min(ASSETS.len().saturating_sub(1))];
        AssetSlotAsset::new(format!("asset://{}", asset.name), asset.name).with_kind(asset.kind)
    }

    fn collider_model(&self) -> DropdownModel {
        let mut model = DropdownModel::from_items([
            DropdownItem::new(DropdownItemId::from_raw(1), "Box"),
            DropdownItem::new(DropdownItemId::from_raw(2), "Capsule"),
            DropdownItem::new(DropdownItemId::from_raw(3), "Sphere"),
            DropdownItem::new(DropdownItemId::from_raw(4), "Mesh").with_enabled(false),
        ]);
        let _ = model.set_selected_id(self.collider_kind);
        model
    }

    fn showcase_job_list() -> JobList {
        JobList::from_rows([
            JobRow::new(job_row_id(1), "Active showcase job", JobPhase::Running)
                .with_progress(JobProgress::determinate(0.60))
                .with_detail("Deterministic fixture progress 3/5")
                .with_cancel(JobCancel::new(
                    ActionDescriptor::new(ACTION_CANCEL_ACTIVE_FIXTURE_JOB, "Cancel active job"),
                    ActionContext::Editor,
                )),
            JobRow::new(job_row_id(2), "Queued showcase job", JobPhase::Queued)
                .with_progress(JobProgress::determinate(0.20))
                .with_detail("Waiting in fixture queue")
                .with_cancel(JobCancel::new(
                    ActionDescriptor::new(ACTION_CANCEL_QUEUED_FIXTURE_JOB, "Cancel queued job"),
                    ActionContext::Editor,
                )),
            JobRow::new(job_row_id(3), "Completed showcase job", JobPhase::Succeeded)
                .with_progress(JobProgress::determinate(1.0))
                .with_detail("Finished fixture row"),
            JobRow::new(job_row_id(4), "Failed showcase job", JobPhase::Failed)
                .with_progress(JobProgress::determinate(0.80))
                .with_detail("Fixture failure for diagnostics presentation"),
        ])
    }

    fn showcase_diagnostics() -> DiagnosticStrip {
        DiagnosticStrip::from_items([
            DiagnosticStripItem::new(
                diagnostic_item_id(1),
                DiagnosticStripSeverity::Warning,
                "showcase.fixture.warning",
                "Fixture warning keeps diagnostics visible",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("panel", "Console"),
            DiagnosticStripItem::new(
                diagnostic_item_id(2),
                DiagnosticStripSeverity::Info,
                "showcase.fixture.info",
                "Fixture metadata is application-owned",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("state", "deterministic"),
            DiagnosticStripItem::new(
                diagnostic_item_id(3),
                DiagnosticStripSeverity::Error,
                "showcase.fixture.error",
                "Fixture error demonstrates summary counts",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("recoverable", "true"),
        ])
    }

    fn showcase_feedback_stack() -> FeedbackStack {
        FeedbackStack::from_items([
            FeedbackItem::timed(
                feedback_id(1),
                FeedbackKind::Success,
                "Saved",
                "Fixture save completed",
                Duration::from_secs(2),
                Duration::from_secs(8),
            )
            .with_dismiss(FeedbackDismiss::new(
                ActionDescriptor::new(ACTION_DISMISS_FEEDBACK_REPORT, "Dismiss feedback"),
                ActionContext::Editor,
            )),
            FeedbackItem::pinned(
                feedback_id(2),
                FeedbackKind::Warning,
                "Report",
                "Fixture report needs review",
            )
            .with_action(FeedbackAction::new(
                ActionDescriptor::new(ACTION_OPEN_FEEDBACK_REPORT, "Open report"),
                ActionContext::Editor,
            ))
            .with_dismiss(FeedbackDismiss::new(
                ActionDescriptor::new(ACTION_DISMISS_FEEDBACK_REPORT, "Dismiss report"),
                ActionContext::Editor,
            )),
            FeedbackItem::timed(
                feedback_id(3),
                FeedbackKind::Info,
                "Expired",
                "Expired fixture toast",
                Duration::from_secs(0),
                Duration::from_secs(2),
            ),
        ])
    }

    fn console_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let diagnostics = Self::showcase_diagnostics();
        let jobs = Self::showcase_job_list();
        let feedback = Self::showcase_feedback_stack();
        let summary = diagnostics.summary();
        let active_feedback = feedback.active_items(showcase_feedback_now());
        let diagnostics_header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 24.0);
        text(
            ui,
            diagnostics_header.x,
            diagnostics_header.y + 16.0,
            &format!(
                "Diagnostics: {} error, {} warning, {} info",
                summary.errors, summary.warnings, summary.info
            ),
            12.0,
            rgb(222, 225, 230),
        );

        let diagnostics_layout = ListLayout::new(22.0);
        let diagnostic_rows = diagnostics.ordered_items();
        let diagnostics_bounds = Rect::new(
            body.x + 8.0,
            body.y + 36.0,
            body.width - 16.0,
            (diagnostic_rows.len() as f32 * 22.0).min(72.0),
        );
        for item in diagnostics_layout.row_rects(
            diagnostics_bounds,
            diagnostic_rows.len(),
            0..diagnostic_rows.len(),
        ) {
            let diagnostic = diagnostic_rows[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 15.0,
                severity_label(diagnostic.severity),
                10.0,
                severity_color(diagnostic.severity),
            );
            text(
                ui,
                item.rect.x + 76.0,
                item.rect.y + 15.0,
                &diagnostic.code,
                10.0,
                rgb(178, 183, 190),
            );
            text(
                ui,
                item.rect.x + 216.0,
                item.rect.y + 15.0,
                &diagnostic.message,
                10.0,
                rgb(218, 221, 226),
            );
        }

        let jobs_y = diagnostics_bounds.max_y() + 12.0;
        let job_summary = jobs.summary();
        text(
            ui,
            body.x + 8.0,
            jobs_y + 16.0,
            &format!(
                "Jobs: {} active, {} complete, {} failed",
                job_summary.active(),
                job_summary.succeeded,
                job_summary.failed
            ),
            12.0,
            rgb(222, 225, 230),
        );
        let job_layout = ListLayout::new(24.0);
        let job_bounds = Rect::new(
            body.x + 8.0,
            jobs_y + 28.0,
            body.width - 16.0,
            (jobs.rows().len() as f32 * 24.0).min(96.0),
        );
        for item in job_layout.row_rects(job_bounds, jobs.rows().len(), 0..jobs.rows().len()) {
            let job = &jobs.rows()[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 16.0,
                job_phase_label(job.phase),
                10.0,
                job_phase_color(job.phase),
            );
            text(
                ui,
                item.rect.x + 86.0,
                item.rect.y + 16.0,
                &job.label,
                11.0,
                rgb(218, 221, 226),
            );
            if let Some(progress) = job.progress.status_progress() {
                let bar = Rect::new(item.rect.max_x() - 136.0, item.rect.y + 9.0, 72.0, 6.0);
                rect(ui, bar, rgb(39, 42, 47), Some(rgb(56, 59, 65)));
                rect(
                    ui,
                    Rect::new(bar.x, bar.y, bar.width * progress.value, bar.height),
                    rgb(69, 123, 220),
                    None,
                );
                text(
                    ui,
                    bar.max_x() + 8.0,
                    item.rect.y + 16.0,
                    &format!("{:.0}%", progress.value * 100.0),
                    10.0,
                    rgb(154, 160, 168),
                );
            }
        }

        let feedback_y = job_bounds.max_y() + 12.0;
        text(
            ui,
            body.x + 8.0,
            feedback_y + 16.0,
            &format!("Feedback: {} active toast(s)", active_feedback.len()),
            12.0,
            rgb(222, 225, 230),
        );
        for (index, item) in active_feedback.iter().enumerate() {
            let row = Rect::new(
                body.x + 8.0,
                feedback_y + 28.0 + index as f32 * 22.0,
                body.width - 16.0,
                20.0,
            );
            rect(ui, row, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                row.x + 8.0,
                row.y + 14.0,
                feedback_kind_label(item.kind),
                10.0,
                feedback_kind_color(item.kind),
            );
            text(
                ui,
                row.x + 78.0,
                row.y + 14.0,
                &item.text,
                10.0,
                rgb(218, 221, 226),
            );
        }

        let log_y = feedback_y + 84.0;
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
        let bounds = Rect::new(
            body.x + 8.0,
            log_y,
            body.width - 16.0,
            (body.max_y() - log_y - 8.0).max(0.0),
        );
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

        match Self::showcase_node_graph_output(
            ui.id("editor.node-graph.static-view"),
            Self::showcase_node_graph_viewport(body),
        ) {
            Ok(output) => {
                ui.extend(output.primitives);
                for semantic in output.semantics {
                    ui.push_semantic_node(semantic);
                }
            }
            Err(_) => {
                text(
                    ui,
                    body.x + 10.0,
                    body.y + 42.0,
                    "Node graph descriptor unavailable",
                    11.0,
                    rgb(236, 96, 96),
                );
            }
        }
    }

    fn showcase_node_graph_output(
        id: WidgetId,
        viewport: NodeGraphViewport,
    ) -> Result<NodeGraphStaticOutput, NodeGraphEmissionError> {
        let graph = Self::showcase_node_graph_descriptor();
        let selection = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(2)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(51)),
            NodeGraphSelectionTarget::Reroute(RerouteId::from_raw(1)),
        ]);

        NodeGraphStaticView::new(id, viewport, &graph)
            .with_selection(selection)
            .with_incompatible_ports([PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(2))])
            .emit()
    }

    fn showcase_node_graph_descriptor() -> NodeGraphDescriptor {
        const COLOR: PortTypeId = PortTypeId::from_raw(1);
        const MASK: PortTypeId = PortTypeId::from_raw(2);
        let frame = NodeFrameId::from_raw(1);
        let group = NodeGroupId::from_raw(1);

        NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Texture",
                    GraphRect::new(8.0, 28.0, 92.0, 64.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(1),
                    PortDirection::Output,
                    "Color",
                    COLOR,
                )])
                .with_frame(frame),
                NodeDescriptor::new(
                    NodeId::from_raw(2),
                    "Color Grade",
                    GraphRect::new(142.0, 54.0, 118.0, 76.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(PortId::from_raw(1), PortDirection::Input, "In", COLOR),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Output, "Out", COLOR),
                    PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "Mask", MASK)
                        .with_enabled(false),
                ])
                .with_frame(frame)
                .with_group(group)
                .with_label("Selected preview"),
                NodeDescriptor::new(
                    NodeId::from_raw(3),
                    "Output",
                    GraphRect::new(314.0, 36.0, 96.0, 68.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Input,
                        "Surface",
                        COLOR,
                    ),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "Mask", MASK),
                ])
                .with_bypassed(true),
            ],
            edges: vec![
                EdgeDescriptor::new(
                    EdgeId::from_raw(50),
                    PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(1)),
                ),
                EdgeDescriptor::new(
                    EdgeId::from_raw(51),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
                    PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(1)),
                )
                .with_route_points(vec![NodeGraphEdgeRoutePoint::reroute(
                    RerouteId::from_raw(1),
                )]),
            ],
            reroutes: vec![RerouteDescriptor::new(
                RerouteId::from_raw(1),
                "Route A",
                GraphPoint::new(284.0, 88.0),
            )],
            frames: vec![NodeFrameDescriptor::new(
                frame,
                "Preview Frame",
                GraphRect::new(-4.0, 14.0, 282.0, 132.0),
            )],
            groups: vec![
                NodeGroupDescriptor::new(
                    group,
                    "Look Dev",
                    GraphRect::new(132.0, 44.0, 140.0, 96.0),
                )
                .with_nodes(vec![NodeId::from_raw(2)]),
            ],
        }
    }

    fn showcase_node_graph_viewport(body: Rect) -> NodeGraphViewport {
        NodeGraphViewport::new(
            Rect::new(
                body.x + 8.0,
                body.y + 30.0,
                (body.width - 16.0).max(0.0),
                (body.height - 38.0).max(0.0),
            ),
            NodeGraphPanZoom::new(GraphVector::new(12.0, 8.0), 1.0),
        )
    }

    fn status_bar(&self, ui: &mut Ui<'_>, viewport: Rect, action_count: u32) {
        let status_bar = self.status_bar_model(action_count);
        let visible_items = status_bar.visible_items();
        let bar = Rect::new(0.0, viewport.max_y() - 24.0, viewport.width, 24.0);
        rect(ui, bar, rgb(27, 29, 32), Some(rgb(52, 55, 62)));
        let message = visible_items
            .iter()
            .find(|item| item.id == EditorStatusItemKind::Message.id())
            .expect("editor status bar exposes message item");
        text(
            ui,
            10.0,
            bar.y + 16.0,
            &message.text,
            11.0,
            rgb(198, 203, 211),
        );
        let mut x = viewport.max_x() - 92.0;
        for item in visible_items
            .iter()
            .filter(|item| item.id != EditorStatusItemKind::Message.id())
            .rev()
        {
            let width = status_item_text_width(&item.text);
            let color = match item.kind {
                StatusItemKind::Error => rgb(236, 96, 96),
                StatusItemKind::Stale => rgb(232, 179, 90),
                StatusItemKind::Ready
                | StatusItemKind::Pending
                | StatusItemKind::Message
                | StatusItemKind::ActionCount
                | StatusItemKind::JobCount
                | StatusItemKind::Progress => rgb(154, 160, 168),
            };
            text(ui, x, bar.y + 16.0, &item.text, 11.0, color);
            x -= width;
        }
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

fn toolbar_action(
    action_id: &'static str,
    label: &'static str,
    icon: ToolbarIcon,
    checked: Option<bool>,
    enabled: bool,
) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(action_id, label);
    action.icon = Some(ActionIcon::new(icon.symbol()));
    action.tooltip = Some(label.to_owned());
    action.keywords = vec!["editor".to_owned(), icon.symbol().to_owned()];
    action.state.checked = checked;
    action.state.enabled = enabled;
    action
}

fn modal_action(action_id: &'static str, label: &'static str, enabled: bool) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(action_id, label);
    action.keywords = vec!["editor".to_owned(), "modal".to_owned()];
    action.state.enabled = enabled;
    action
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
        PropertyGridRow::property(item_id(2), "Position", 0)
            .with_resettable(true, false)
            .with_keyframeable(true, true),
        PropertyGridRow::property(item_id(5), "Uniform Scale", 0)
            .with_resettable(true, true)
            .with_keyframeable(true, false),
        PropertyGridRow::section(item_id(6), "Rendering"),
        PropertyGridRow::property(item_id(7), "Exposure", 0)
            .with_status(PropertyGridRowStatus::warning("Preview range exceeded"))
            .with_resettable(true, false)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(8), "Roughness", 0)
            .with_resettable(true, true)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(9), "Material", 0)
            .with_status(PropertyGridRowStatus::info("Inherited material override"))
            .with_resettable(true, false),
        PropertyGridRow::section(item_id(10), "Editor"),
        PropertyGridRow::property(item_id(11), "Snap", 0).with_resettable(true, false),
        PropertyGridRow::section(item_id(12), "Physics"),
        PropertyGridRow::property(item_id(13), "Mass", 0)
            .with_status(PropertyGridRowStatus::error("Mass must be positive"))
            .with_resettable(true, false)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(14), "Collider", 0)
            .with_resettable(true, false)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(15), "Script", 0)
            .with_read_only(true)
            .with_status(PropertyGridRowStatus::info("Script comes from prefab"))
            .with_resettable(true, false)
            .with_keyframeable(true, false),
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
    frame_tab_strip(frame)
        .tabs()
        .iter()
        .cloned()
        .map(|tab| {
            let width = (tab.title.len() as f32 * 7.0 + 42.0).clamp(82.0, 146.0);
            let tab_rect = Rect::new(tab_x, frame_rect.y + 1.0, width, tab_height);
            tab_x += width + 1.0;
            (tab, tab_rect)
        })
        .collect()
}

fn frame_tab_strip(frame: &Frame) -> TabStrip {
    TabStrip::from_frame_tabs(frame_tabs(frame))
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
    use std::time::Duration;

    use super::{
        ACTION_GRID, ACTION_PLAY, ACTION_SAVE, ACTION_STOP, ACTION_VIEWPORT_ACTUAL_SIZE,
        ACTION_VIEWPORT_FIT_CONTENT, ACTION_VIEWPORT_FIT_SELECTION, ACTION_VIEWPORT_FOCUS_SELECTED,
        ACTION_VIEWPORT_PAN, ACTION_VIEWPORT_ZOOM_IN, ACTION_VIEWPORT_ZOOM_OUT,
        EditorChromeMetrics, EditorMenuKind, EditorShowcase, EditorStatusItemKind, EditorTool,
        EditorToolbarGroupKind, FRAME_BOTTOM, FRAME_INSPECTOR, FRAME_VIEWPORT, PANEL_TIMELINE,
        TOOLBAR_Y, VIEWPORT_SIZE, frame_tab_rects, frame_tab_strip, icon_atlas_image,
        inspector_label_width, item_id, phosphor_icons, register_resources, rgb, rgba,
    };
    use kinetik_ui::core::{
        ActionContext, ActionDescriptor, ActionId, ActionSource, Brush, CursorShape, FrameContext,
        PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect,
        RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, Size, TimeInfo, UiInput,
        UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
    };
    use kinetik_ui::render::RenderResources;
    use kinetik_ui::widgets::{
        DockSplitterContextActionKind, FeedbackKind, GraphVector, JobPhase, MenuItem,
        ModalActionRole, NodeFrameId, NodeGraphContextActionKind, NodeGraphContextTarget,
        NodeGraphHitTarget, NodeGraphLinkEditRequest, NodeGraphSelection, NodeGraphSelectionTarget,
        NodeId, OverlayDismissal, OverlayKind, PanZoom, PanelOpenDecision, PanelTypeCategory,
        PortEndpoint, PortId, StatusItemKind, TimelineDescriptor, TimelineFrameRate, TimelineId,
        TimelineItemDescriptor, TimelineItemId, TimelineKeyframeDescriptor, TimelineKeyframeId,
        TimelineLaneDescriptor, TimelineLaneId, TimelineLayout, TimelineMarkerDescriptor,
        TimelineMarkerId, TimelineRange, TimelineScale, TimelineSelection, TimelineSelectionTarget,
        TimelineSnapCandidate, TimelineSnapCandidateRequest, TimelineSnapSource, TimelineTime,
        TimelineTransportContext, TimelineViewportState, TimelineZoom, TransportActionRequest,
        TransportControlDescriptor, TransportControlId, TransportControlIntent, TransportControls,
        Ui, ViewportActionDescriptor, ViewportActionKind, ViewportActionRequest,
        ViewportActionTarget, ViewportCursorMetadata, ViewportCursorRequest,
        ViewportCursorRequestSource, ViewportCursorShape, ViewportOverlayDescriptor,
        ViewportOverlayId, ViewportOverlayKind, ViewportOverlaySpace, ViewportSelectionTargetId,
        ViewportSurface, ViewportToolDescriptor, ViewportToolId, hit_test_viewport_overlays,
        resolve_dock_splitter_context_actions_with_policy, solve_dock_layout,
        solve_dock_splitters_with_style, timeline_semantics, timeline_snap_candidates,
        viewport_action_requests, viewport_actions_semantics, viewport_cursor_request,
    };

    struct EditorTimelineFixture {
        descriptor: TimelineDescriptor,
        candidates: Vec<TimelineSnapCandidate>,
        transport_request: TransportActionRequest,
        state: TimelineViewportState,
        semantic_roles: Vec<SemanticRole>,
    }

    struct EditorViewportToolFixture {
        actions: Vec<ViewportActionDescriptor>,
        requests: Vec<ViewportActionRequest>,
        cursor_request: ViewportCursorRequest,
        semantic_roles: Vec<SemanticRole>,
    }

    fn editor_timeline_fixture() -> EditorTimelineFixture {
        let timeline = TimelineId::from_raw(9_000);
        let descriptor = TimelineDescriptor::new(
            [
                TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "Video"),
                TimelineLaneDescriptor::new(TimelineLaneId::from_raw(2), "Animation"),
            ],
            [
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(10),
                    TimelineLaneId::from_raw(1),
                    TimelineRange::seconds(0.0, 2.5),
                    "Intro clip",
                ),
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(11),
                    TimelineLaneId::from_raw(2),
                    TimelineRange::seconds(1.0, 3.0),
                    "Camera move",
                ),
            ],
            [TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(30),
                TimelineTime::from_seconds(1.5),
                "Beat",
            )],
            [TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(40),
                TimelineItemId::from_raw(11),
                TimelineTime::from_seconds(2.0),
            )],
        );
        let scale = TimelineScale::new(
            0.0,
            240.0,
            TimelineRange::seconds(0.0, 4.0),
            TimelineZoom::new(60.0),
            0.0,
        );
        let layout = TimelineLayout::new(24.0)
            .resolve(Rect::new(0.0, 0.0, 240.0, 48.0), scale, &descriptor, 0.0)
            .expect("editor timeline fixture resolves");
        let semantic_roles = timeline_semantics(
            WidgetId::from_key("editor.timeline.fixture"),
            layout.bounds,
            &layout,
            "Editor timeline",
        )
        .into_iter()
        .map(|node| node.role)
        .collect::<Vec<_>>();
        let candidates = timeline_snap_candidates(
            TimelineSnapCandidateRequest::new(
                timeline,
                scale.visible_range(),
                TimelineFrameRate::integer(24),
                &descriptor,
            )
            .with_selection_range(TimelineRange::seconds(0.5, 2.5))
            .with_playhead_time(TimelineTime::from_seconds(1.25)),
        );
        let selection = TimelineSelection::from_targets([TimelineSelectionTarget::Item(
            TimelineItemId::from_raw(11),
        )]);
        let state = TimelineViewportState::new(scale)
            .with_playhead_time(TimelineTime::from_seconds(1.25))
            .with_selection(selection)
            .with_selection_range(TimelineRange::seconds(0.5, 2.5));
        let transport = TransportControls::from_controls([
            TransportControlDescriptor::new(
                TransportControlId::from_raw(1),
                TransportControlIntent::PlayPause,
                ActionDescriptor::new(ACTION_PLAY, "Play"),
            ),
            TransportControlDescriptor::new(
                TransportControlId::from_raw(2),
                TransportControlIntent::Stop,
                ActionDescriptor::new(ACTION_STOP, "Stop"),
            ),
        ]);
        let transport_request = transport
            .request_for_visible(
                0,
                ActionSource::Button,
                Some(
                    TimelineTransportContext::new(timeline)
                        .with_playhead_time(TimelineTime::from_seconds(1.25))
                        .with_selection_range(TimelineRange::seconds(0.5, 2.5)),
                ),
            )
            .expect("editor transport fixture emits metadata");

        EditorTimelineFixture {
            descriptor,
            candidates,
            transport_request,
            state,
            semantic_roles,
        }
    }

    fn editor_viewport_tool_fixture() -> EditorViewportToolFixture {
        let viewport = WidgetId::from_key("editor.viewport.fixture");
        let selected = ViewportSelectionTargetId::from_raw(70);
        let overlay = ViewportOverlayId::from_raw(12);
        let select_tool = ViewportToolId::from_raw(1);
        let pan_tool = ViewportToolId::from_raw(2);
        let mut select_action = ActionDescriptor::new(super::ACTION_TOOL_SELECT, "Select");
        select_action.state.checked = Some(true);
        let mut pan_action = ActionDescriptor::new(ACTION_VIEWPORT_PAN, "Pan");
        pan_action.state.checked = Some(false);
        let mut grid_action = ActionDescriptor::new(ACTION_GRID, "Show Grid");
        grid_action.state.checked = Some(true);
        let actions = vec![
            ViewportActionDescriptor::new(
                select_action,
                ViewportActionKind::ActivateTool,
                ViewportActionTarget::new(viewport).with_tool(select_tool),
            ),
            ViewportActionDescriptor::new(
                ActionDescriptor::new(ACTION_VIEWPORT_FOCUS_SELECTED, "Focus Selected"),
                ViewportActionKind::FocusSelected,
                ViewportActionTarget::new(viewport).with_selection(selected),
            ),
            ViewportActionDescriptor::new(
                ActionDescriptor::new(ACTION_VIEWPORT_FIT_CONTENT, "Fit Content"),
                ViewportActionKind::FitContent,
                ViewportActionTarget::new(viewport),
            ),
            ViewportActionDescriptor::new(
                ActionDescriptor::new(ACTION_VIEWPORT_FIT_SELECTION, "Fit Selection"),
                ViewportActionKind::FitSelection,
                ViewportActionTarget::new(viewport).with_selection(selected),
            ),
            ViewportActionDescriptor::new(
                ActionDescriptor::new(ACTION_VIEWPORT_ACTUAL_SIZE, "Actual Size"),
                ViewportActionKind::ActualSize,
                ViewportActionTarget::new(viewport),
            ),
            ViewportActionDescriptor::new(
                ActionDescriptor::new(ACTION_VIEWPORT_ZOOM_IN, "Zoom In"),
                ViewportActionKind::ZoomIn,
                ViewportActionTarget::new(viewport),
            ),
            ViewportActionDescriptor::new(
                ActionDescriptor::new(ACTION_VIEWPORT_ZOOM_OUT, "Zoom Out"),
                ViewportActionKind::ZoomOut,
                ViewportActionTarget::new(viewport),
            ),
            ViewportActionDescriptor::new(
                pan_action,
                ViewportActionKind::PanMode,
                ViewportActionTarget::new(viewport).with_tool(pan_tool),
            ),
            ViewportActionDescriptor::new(
                grid_action,
                ViewportActionKind::ToggleOverlay,
                ViewportActionTarget::new(viewport).with_overlay(overlay),
            ),
        ];
        let requests = viewport_action_requests(
            &actions,
            ActionSource::Button,
            &ActionContext::Widget(viewport),
        );
        let semantic_roles = viewport_actions_semantics(
            viewport.child("actions"),
            Rect::new(0.0, 0.0, 280.0, 28.0),
            "Viewport tool actions",
            &actions,
            actions.iter().enumerate().map(|(index, action)| {
                (
                    action.action.id.clone(),
                    Rect::new(index as f32 * 28.0, 0.0, 24.0, 24.0),
                )
            }),
        )
        .into_iter()
        .map(|node| node.role)
        .collect::<Vec<_>>();
        let mut pan_zoom = PanZoom::default();
        pan_zoom.set_zoom(1.0);
        let surface = ViewportSurface {
            texture: super::VIEWPORT_TEXTURE,
            source_size: VIEWPORT_SIZE,
            bounds: Rect::new(0.0, 0.0, 320.0, 180.0),
            pan_zoom,
        };
        let overlay_hit = hit_test_viewport_overlays(
            surface,
            &[ViewportOverlayDescriptor::new(
                overlay,
                ViewportOverlayKind::ToolRegion,
                Rect::new(12.0, 12.0, 80.0, 40.0),
                ViewportOverlaySpace::Screen,
            )
            .with_tool(pan_tool)
            .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Crosshair))],
            Point::new(24.0, 20.0),
        )
        .expect("editor viewport fixture overlay hit");
        let tool = ViewportToolDescriptor::new(pan_tool, "Pan")
            .active(true)
            .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Grab));
        let cursor_request =
            viewport_cursor_request(viewport, None, None, Some(&overlay_hit), Some(&tool))
                .expect("editor viewport fixture cursor request");

        EditorViewportToolFixture {
            actions,
            requests,
            cursor_request,
            semantic_roles,
        }
    }

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
    fn editor_chrome_menu_bar_converts_active_menu_to_overlay_contract() {
        let mut editor = EditorShowcase::new();
        editor.open_menu = Some(EditorMenuKind::File);
        let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
        let menu_bar = editor.menu_bar_model();

        assert_eq!(menu_bar.menus().len(), 7);
        assert_eq!(
            menu_bar.active_id(),
            Some(EditorMenuKind::File.menu_bar_id())
        );
        assert_eq!(
            menu_bar.active_menu().expect("active file menu").title,
            "File"
        );

        let overlay = editor.menu_overlay_model(EditorMenuKind::File, viewport);

        assert_eq!(overlay.entry.kind, OverlayKind::Menu);
        assert_eq!(
            overlay.entry.dismissal,
            OverlayDismissal::OutsideClickOrEscape
        );
        assert_eq!(overlay.source, ActionSource::Menu);
        assert_eq!(overlay.context, ActionContext::Editor);
        assert!(overlay.entry.rect.y > super::menu_anchor(EditorMenuKind::File).max_y());
        assert!(overlay.visible_items().iter().any(|item| matches!(
            item,
            MenuItem::Action(action)
                if action.id.as_str() == ACTION_SAVE
                    && action.label == "Save Scene"
                    && action.can_invoke()
        )));
        assert!(overlay.visible_items().iter().any(|item| matches!(
            item,
            MenuItem::Action(action) if action.label == "Quit" && !action.can_invoke()
        )));
    }

    #[test]
    fn editor_chrome_toolbar_contract_tracks_checked_action_state() {
        let mut editor = EditorShowcase::new();
        let toolbar = editor.toolbar_model();
        let tools = toolbar
            .group(EditorToolbarGroupKind::Tools.id())
            .expect("tools group")
            .visible_items();

        assert_eq!(
            tools.iter().map(|item| item.label()).collect::<Vec<_>>(),
            ["Select", "Move", "Rotate", "Scale"]
        );
        assert_eq!(tools[1].action_id().as_str(), super::ACTION_TOOL_MOVE);
        assert_eq!(tools[1].checked(), Some(true));
        assert_eq!(tools[0].checked(), Some(false));
        assert_eq!(
            tools[1].icon().map(kinetik_ui::core::ActionIcon::as_str),
            Some("move")
        );

        let viewport_tools = toolbar
            .group(EditorToolbarGroupKind::Viewport.id())
            .expect("viewport group")
            .visible_items();
        assert_eq!(viewport_tools[0].action_id().as_str(), ACTION_GRID);
        assert_eq!(viewport_tools[0].checked(), Some(true));

        assert!(editor.apply_action(ACTION_PLAY));
        let toolbar = editor.toolbar_model();
        let run_items = toolbar
            .group(EditorToolbarGroupKind::Run.id())
            .expect("run group")
            .visible_items();
        assert_eq!(run_items[0].label(), "Play");
        assert_eq!(run_items[0].checked(), Some(true));
        assert_eq!(run_items[1].label(), "Pause");
        assert_eq!(run_items[1].checked(), Some(false));

        let invocation = toolbar
            .invocation_for_group_visible(
                EditorToolbarGroupKind::Run.id(),
                0,
                ActionContext::Editor,
            )
            .expect("run invocation");
        assert_eq!(invocation.action_id, ActionId::new(ACTION_PLAY));
        assert_eq!(invocation.source, ActionSource::Button);
        assert_eq!(invocation.context, ActionContext::Editor);
    }

    #[test]
    fn editor_chrome_status_bar_contract_preserves_order_counts_and_progress() {
        let mut editor = EditorShowcase::new();
        editor.status = "Busy".to_owned();
        editor.running = true;
        editor.timeline = 1.5;

        let status_bar = editor.status_bar_model(12);
        let visible = status_bar.visible_items();

        assert_eq!(
            visible
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            [
                "Busy",
                "Actions: 12",
                "Snap 1m",
                "Vello / winit",
                "Jobs: 2 active / 4 total",
                "Diagnostics: 1E 1W 1I",
                "Feedback: 2"
            ]
        );
        let actions = status_bar
            .item(EditorStatusItemKind::Actions.id())
            .expect("action count status");
        assert_eq!(actions.kind, StatusItemKind::ActionCount);
        assert_eq!(actions.count, Some(12));

        let jobs = status_bar
            .item(EditorStatusItemKind::Jobs.id())
            .expect("job status");
        assert_eq!(jobs.kind, StatusItemKind::JobCount);
        assert_eq!(jobs.count, Some(2));
        assert!((jobs.progress.expect("job progress").value - 0.4).abs() < f32::EPSILON);
        assert!(jobs.visible);

        let diagnostics = status_bar
            .item(EditorStatusItemKind::Diagnostics.id())
            .expect("diagnostics status");
        assert_eq!(diagnostics.kind, StatusItemKind::Error);
        assert_eq!(diagnostics.count, Some(3));

        let feedback = status_bar
            .item(EditorStatusItemKind::Feedback.id())
            .expect("feedback status");
        assert_eq!(feedback.kind, StatusItemKind::Message);
        assert_eq!(feedback.count, Some(2));

        let progress = status_bar
            .item(EditorStatusItemKind::Timeline.id())
            .expect("timeline progress status");
        assert_eq!(progress.kind, StatusItemKind::Progress);
        assert_eq!(progress.progress.expect("progress metadata").value, 1.0);
        assert!(!progress.visible);
    }

    #[test]
    fn editor_showcase_job_fixture_is_deterministic_and_app_owned() {
        let jobs = EditorShowcase::showcase_job_list();
        let summary = jobs.summary();
        let progress = jobs.active_progress().expect("active fixture jobs");

        assert_eq!(jobs.rows().len(), 4);
        assert_eq!(
            jobs.rows()
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            [
                "Active showcase job",
                "Queued showcase job",
                "Completed showcase job",
                "Failed showcase job"
            ]
        );
        assert_eq!(summary.running, 1);
        assert_eq!(summary.queued, 1);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.active(), 2);
        assert_eq!(progress.active, 2);
        assert_eq!(progress.determinate, 2);
        assert_eq!(progress.indeterminate, 0);
        assert!(
            (progress.status_progress().expect("status progress").value - 0.4).abs() < f32::EPSILON
        );
        assert_eq!(jobs.rows()[0].phase, JobPhase::Running);
        assert!(jobs.rows()[0].can_cancel());
        assert_eq!(
            jobs.cancel_request(super::job_row_id(1))
                .expect("cancel request")
                .invocation
                .action_id,
            ActionId::new(super::ACTION_CANCEL_ACTIVE_FIXTURE_JOB)
        );
    }

    #[test]
    fn editor_showcase_diagnostics_fixture_summarizes_ordered_app_metadata() {
        let diagnostics = EditorShowcase::showcase_diagnostics();
        let summary = diagnostics.summary();
        let ordered = diagnostics.ordered_items();

        assert_eq!(summary.errors, 1);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.info, 1);
        assert_eq!(summary.total(), 3);
        assert_eq!(
            ordered
                .iter()
                .map(|item| item.code.as_str())
                .collect::<Vec<_>>(),
            [
                "showcase.fixture.error",
                "showcase.fixture.warning",
                "showcase.fixture.info"
            ]
        );
        assert!(diagnostics.items().iter().all(|item| {
            item.source == Some(kinetik_ui::widgets::DiagnosticSource::Application)
        }));
    }

    #[test]
    fn editor_showcase_feedback_fixture_preserves_lifetime_action_and_dismiss_metadata() {
        let feedback = EditorShowcase::showcase_feedback_stack();
        let active = feedback.active_items(super::showcase_feedback_now());

        assert_eq!(feedback.items().len(), 3);
        assert_eq!(active.len(), 2);
        assert_eq!(
            active.iter().map(|item| item.kind).collect::<Vec<_>>(),
            [FeedbackKind::Success, FeedbackKind::Warning]
        );
        assert_eq!(
            feedback
                .item(super::feedback_id(1))
                .expect("timed feedback")
                .remaining_lifetime(super::showcase_feedback_now()),
            Some(Duration::from_secs(4))
        );
        assert_eq!(
            feedback
                .item(super::feedback_id(3))
                .expect("expired feedback")
                .remaining_lifetime(super::showcase_feedback_now()),
            None
        );
        assert_eq!(
            feedback
                .action_request(super::feedback_id(2), super::showcase_feedback_now())
                .expect("feedback action")
                .invocation
                .action_id,
            ActionId::new(super::ACTION_OPEN_FEEDBACK_REPORT)
        );
        assert_eq!(
            feedback
                .dismiss_request(super::feedback_id(2), super::showcase_feedback_now())
                .expect("feedback dismiss")
                .invocation
                .action_id,
            ActionId::new(super::ACTION_DISMISS_FEEDBACK_REPORT)
        );
    }

    #[test]
    fn editor_showcase_frame_emits_no_core_warnings() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let context = editor_test_context(UiInput::default());
        let mut ui = Ui::begin_frame(context, &mut memory, &theme);
        let mut editor = EditorShowcase::new();

        editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert!(
            output.diagnostics().is_empty(),
            "{:?}",
            output.diagnostics()
        );
    }

    #[test]
    fn editor_chrome_tab_strip_contract_preserves_frame_tab_targets() {
        let editor = EditorShowcase::new();
        let bottom = editor.dock.frame(FRAME_BOTTOM).expect("bottom frame");
        let strip = frame_tab_strip(bottom);
        let rects = frame_tab_rects(bottom, bottom_frame_rect(&editor), 26.0);

        assert_eq!(strip.len(), 3);
        assert_eq!(rects.len(), strip.len());
        assert_eq!(strip.tabs()[0].title, "Console");
        assert_eq!(strip.tabs()[1].title, "Timeline");
        assert_eq!(strip.active_panel(), Some(strip.tabs()[0].panel));
        assert_eq!(
            strip
                .activation_target_by_index(1)
                .expect("timeline target")
                .panel,
            PANEL_TIMELINE
        );
        assert_eq!(
            strip
                .drag_target_by_panel(PANEL_TIMELINE)
                .expect("timeline drag target")
                .index,
            1
        );
    }

    #[test]
    fn editor_timeline_fixture_exposes_data_only_semantics_snap_and_transport_requests() {
        let fixture = editor_timeline_fixture();

        fixture
            .descriptor
            .validate()
            .expect("editor-owned timeline descriptors validate");
        assert!(
            fixture
                .semantic_roles
                .iter()
                .any(|role| *role == SemanticRole::Custom("timeline".to_owned()))
        );
        assert!(
            fixture
                .semantic_roles
                .iter()
                .any(|role| *role == SemanticRole::Custom("timeline-item".to_owned()))
        );
        assert!(
            fixture
                .candidates
                .iter()
                .any(|candidate| candidate.source == TimelineSnapSource::Frame)
        );
        assert!(
            fixture
                .candidates
                .iter()
                .any(|candidate| candidate.source == TimelineSnapSource::Marker)
        );
        assert!(
            fixture
                .candidates
                .iter()
                .any(|candidate| candidate.source == TimelineSnapSource::Keyframe)
        );
        assert_eq!(
            fixture.transport_request.action_id,
            ActionId::new(ACTION_PLAY)
        );
        assert_eq!(fixture.transport_request.source, ActionSource::Button);
        assert_eq!(
            fixture
                .transport_request
                .timeline_context
                .expect("transport context")
                .timeline,
            TimelineId::from_raw(9_000)
        );
        assert!(
            fixture
                .state
                .selection
                .contains(TimelineSelectionTarget::Item(TimelineItemId::from_raw(11)))
        );
    }

    #[test]
    fn editor_viewport_tool_fixture_exercises_app_owned_action_routing() {
        let fixture = editor_viewport_tool_fixture();

        assert_eq!(fixture.actions.len(), 9);
        assert_eq!(
            fixture
                .requests
                .iter()
                .map(|request| request.kind)
                .collect::<Vec<_>>(),
            vec![
                ViewportActionKind::ActivateTool,
                ViewportActionKind::FocusSelected,
                ViewportActionKind::FitContent,
                ViewportActionKind::FitSelection,
                ViewportActionKind::ActualSize,
                ViewportActionKind::ZoomIn,
                ViewportActionKind::ZoomOut,
                ViewportActionKind::PanMode,
                ViewportActionKind::ToggleOverlay,
            ]
        );
        assert!(fixture.requests.iter().all(|request| {
            request.source == ActionSource::Button
                && matches!(request.context, ActionContext::Widget(_))
        }));
        assert_eq!(
            fixture.requests[0].action_id,
            ActionId::new(super::ACTION_TOOL_SELECT)
        );
        assert_eq!(fixture.requests[0].checked, Some(true));
        assert_eq!(
            fixture.requests[1].target.selection,
            Some(ViewportSelectionTargetId::from_raw(70))
        );
        assert_eq!(
            fixture.requests[8].target.overlay,
            Some(ViewportOverlayId::from_raw(12))
        );
        assert_eq!(fixture.requests[8].checked, Some(true));
        assert!(
            fixture
                .semantic_roles
                .iter()
                .any(|role| *role == SemanticRole::Custom("viewport-actions".to_owned()))
        );
        assert!(fixture.semantic_roles.contains(&SemanticRole::Toggle));
        assert!(fixture.semantic_roles.contains(&SemanticRole::Button));
        assert_eq!(
            fixture.cursor_request.source,
            ViewportCursorRequestSource::HoveredOverlay
        );
        assert_eq!(
            fixture.cursor_request.cursor.shape,
            ViewportCursorShape::Crosshair
        );
        assert_eq!(
            fixture.cursor_request.overlay,
            Some(ViewportOverlayId::from_raw(12))
        );
    }

    #[test]
    fn editor_chrome_modal_contract_exposes_data_only_action_metadata() {
        let editor = EditorShowcase::new();
        let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
        let before = editor.status.clone();
        let overlay = editor.about_modal_overlay_model(viewport);

        assert_eq!(overlay.entry.kind, OverlayKind::Modal);
        assert!(overlay.entry.modal);
        assert_eq!(
            overlay.entry.dismissal,
            OverlayDismissal::OutsideClickOrEscape
        );
        assert_eq!(overlay.context, ActionContext::Editor);
        assert_eq!(overlay.dialog.title, "About Kinetik Forge");
        assert_eq!(overlay.visible_actions().len(), 2);
        assert_eq!(
            overlay
                .visible_action_by_role(ModalActionRole::Cancel)
                .expect("cancel action")
                .action
                .label,
            "Close"
        );

        let invocation = overlay
            .invocation_for_role(ModalActionRole::Primary)
            .expect("primary modal action invocation");
        assert_eq!(invocation.action_id, ActionId::new(super::ACTION_PALETTE));
        assert_eq!(invocation.source, ActionSource::Button);
        assert_eq!(invocation.context, ActionContext::Editor);
        assert_eq!(editor.status, before);
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
    fn editor_node_graph_panel_exercises_stage9_contracts() {
        let mut editor = EditorShowcase::new();
        assert!(editor.open_or_focus_panel(super::PANEL_TYPE_NODE_GRAPH));

        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(editor_test_context(UiInput::default()), &mut memory, &theme);
        editor.render(&mut ui, 0);
        let frame = ui.finish_output();

        assert!(frame.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Custom("node-graph".to_owned())
                && node.label.as_deref() == Some("Node graph")
        }));

        let body = Rect::new(20.0, 40.0, 480.0, 180.0);
        let viewport = super::EditorShowcase::showcase_node_graph_viewport(body);
        let graph = super::EditorShowcase::showcase_node_graph_descriptor();
        graph.validate().expect("showcase graph validates");

        let output = super::EditorShowcase::showcase_node_graph_output(
            WidgetId::from_key("showcase-node-graph"),
            viewport,
        )
        .expect("showcase graph emits static output");
        assert!(matches!(
            output.primitives.first(),
            Some(Primitive::ClipBegin { .. })
        ));
        assert!(matches!(
            output.primitives.last(),
            Some(Primitive::ClipEnd { .. })
        ));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("node".to_owned())
                && node.label.as_deref() == Some("Color Grade")
                && node.state.selected
        }));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("edge".to_owned())
                && node.label.as_deref() == Some("Edge 51: Color Grade Out to Output Surface")
                && node.state.selected
        }));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("port".to_owned())
                && node.label.as_deref() == Some("Input Mask")
                && node.description.as_deref() == Some("Incompatible port")
        }));

        let color_grade_center = viewport.graph_rect_to_screen(graph.nodes[1].rect).center();
        assert_eq!(
            graph
                .hit_test(viewport, color_grade_center)
                .expect("node hit target"),
            NodeGraphHitTarget::NodeBody(NodeId::from_raw(2))
        );

        let selection =
            NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(2)));
        let context_actions = graph.context_actions(
            NodeGraphContextTarget::Node(NodeId::from_raw(2)),
            &selection,
        );
        assert!(
            context_actions.iter().any(|action| {
                action.kind == NodeGraphContextActionKind::Delete && action.enabled
            })
        );
        assert!(context_actions.iter().any(|action| {
            action.kind == NodeGraphContextActionKind::FrameSelection && action.enabled
        }));

        let link_request = graph
            .create_link_request(
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(1)),
            )
            .expect("link request metadata");
        assert!(matches!(
            link_request,
            NodeGraphLinkEditRequest::CreateLink(_)
        ));

        let move_request = graph
            .move_frame_request(
                viewport,
                NodeFrameId::from_raw(1),
                GraphVector::new(20.0, -10.0),
            )
            .expect("frame move metadata");
        assert_eq!(move_request.children.len(), 2);
        assert_eq!(move_request.graph_delta, GraphVector::new(20.0, -10.0));
    }

    #[test]
    fn inspector_snap_toggle_updates_status_same_frame() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(1290.0, 362.0, true, true, false)),
            &mut memory,
            &theme,
        );
        editor.render(&mut ui, 0);
        let _ = ui.finish_output();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(1290.0, 362.0, false, false, true)),
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
        let visible_tool_id =
            WidgetId::from_key("root").child(("editor.tool", super::ACTION_TOOL_SELECT));

        assert_eq!(memory.pressed(), Some(visible_tool_id));

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
    fn toolbar_run_click_invokes_through_visible_identity() {
        let mut editor = EditorShowcase::new();
        let mut memory = UiMemory::new();
        let theme = default_dark_theme();
        let chrome = EditorChromeMetrics::from_theme(&theme);
        let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
        let (index, _icon, _label, action, rect) = super::run_toolbar_buttons(viewport, chrome)[0];
        let point = rect.center();

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(point.x, point.y, true, true, false)),
            &mut memory,
            &theme,
        );
        let invocations = editor.render(&mut ui, 0);
        let _ = ui.finish_output();
        let visible_run_id = WidgetId::from_key("root").child(("editor.run", action, index));

        assert!(invocations.is_empty());
        assert_eq!(memory.pressed(), Some(visible_run_id));

        let mut ui = Ui::begin_frame(
            editor_test_context(pointer_input_at(point.x, point.y, false, false, true)),
            &mut memory,
            &theme,
        );
        let invocations = editor.render(&mut ui, 0);
        let output = ui.finish_output();

        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].action_id, ActionId::new(ACTION_PLAY));
        assert!(editor.running);
        assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Play mode running")
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

const fn job_row_id(raw: u64) -> JobRowId {
    JobRowId::from_raw(raw)
}

const fn diagnostic_item_id(raw: u64) -> DiagnosticStripItemId {
    DiagnosticStripItemId::from_raw(raw)
}

const fn feedback_id(raw: u64) -> FeedbackId {
    FeedbackId::from_raw(raw)
}

fn showcase_feedback_now() -> Duration {
    Duration::from_secs(6)
}

fn status_item_text_width(text: &str) -> f32 {
    text.len() as f32 * 6.4 + 28.0
}

fn severity_label(severity: DiagnosticStripSeverity) -> &'static str {
    match severity {
        DiagnosticStripSeverity::Error => "Error",
        DiagnosticStripSeverity::Warning => "Warning",
        DiagnosticStripSeverity::Info => "Info",
    }
}

fn severity_color(severity: DiagnosticStripSeverity) -> Color {
    match severity {
        DiagnosticStripSeverity::Error => rgb(236, 96, 96),
        DiagnosticStripSeverity::Warning => rgb(232, 179, 90),
        DiagnosticStripSeverity::Info => rgb(135, 176, 236),
    }
}

fn job_phase_label(phase: JobPhase) -> &'static str {
    match phase {
        JobPhase::Queued => "Queued",
        JobPhase::Running => "Running",
        JobPhase::Cancelling => "Cancelling",
        JobPhase::Succeeded => "Done",
        JobPhase::Failed => "Failed",
    }
}

fn job_phase_color(phase: JobPhase) -> Color {
    match phase {
        JobPhase::Queued => rgb(154, 160, 168),
        JobPhase::Running => rgb(135, 176, 236),
        JobPhase::Cancelling => rgb(232, 179, 90),
        JobPhase::Succeeded => rgb(114, 190, 145),
        JobPhase::Failed => rgb(236, 96, 96),
    }
}

fn feedback_kind_label(kind: FeedbackKind) -> &'static str {
    match kind {
        FeedbackKind::Info => "Info",
        FeedbackKind::Success => "Success",
        FeedbackKind::Warning => "Warning",
        FeedbackKind::Error => "Error",
    }
}

fn feedback_kind_color(kind: FeedbackKind) -> Color {
    match kind {
        FeedbackKind::Info => rgb(135, 176, 236),
        FeedbackKind::Success => rgb(114, 190, 145),
        FeedbackKind::Warning => rgb(232, 179, 90),
        FeedbackKind::Error => rgb(236, 96, 96),
    }
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

fn inspector_numeric_scrub(
    ui: &mut Ui<'_>,
    key: &'static str,
    rect: Rect,
    state: &mut TextEditState,
    config: NumericScrubInputConfig,
) {
    let mut value = classify_numeric_input_draft(&state.text)
        .value()
        .unwrap_or(0.0);
    ui.numeric_scrub_input(key, rect, &mut value, state, config);
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
    paint_toolbar_icon_button_sized(
        ui, id, response, rect, icon, label, selected, disabled, icon_size,
    );

    response
}

#[allow(clippy::too_many_arguments)]
fn paint_toolbar_icon_button_sized(
    ui: &mut Ui<'_>,
    id: WidgetId,
    response: Response,
    rect: Rect,
    icon: ToolbarIcon,
    label: &str,
    selected: bool,
    disabled: bool,
    icon_size: f32,
) {
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
