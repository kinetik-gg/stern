//! DCC-style editor showcase surface.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod showcase;
#[cfg(test)]
mod tests;

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
