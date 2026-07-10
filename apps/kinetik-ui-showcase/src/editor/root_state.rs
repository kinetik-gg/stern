const ACTION_NEW_SCENE: &str = "editor.scene.new";
const ACTION_OPEN_PROJECT: &str = "editor.project.open";
/// Saves the current editor project once persistence is implemented.
pub const ACTION_SAVE: &str = "editor.save";
const ACTION_IMPORT_ASSET: &str = "editor.asset.import";
const ACTION_EXPORT: &str = "editor.export";
const ACTION_QUIT: &str = "editor.quit";
const ACTION_UNDO: &str = "editor.undo";
const ACTION_REDO: &str = "editor.redo";
const ACTION_DUPLICATE: &str = "editor.selection.duplicate";
const ACTION_DELETE: &str = "editor.selection.delete";
const ACTION_PREFERENCES: &str = "editor.preferences.open";
const ACTION_VIEW_PERSPECTIVE: &str = "editor.viewport.perspective";
const ACTION_SHOW_OVERLAYS: &str = "editor.viewport.overlays.toggle";
/// Starts editor play mode.
pub const ACTION_PLAY: &str = "editor.play";
/// Pauses editor play mode once a pause lifecycle is implemented.
pub const ACTION_PAUSE: &str = "editor.pause";
/// Stops editor play mode.
pub const ACTION_STOP: &str = "editor.stop";
/// Toggles viewport grid overlays.
pub const ACTION_GRID: &str = "editor.grid";
const ACTION_VIEWPORT_FOCUS_SELECTED: &str = "editor.viewport.focus-selected";
const ACTION_VIEWPORT_FIT_CONTENT: &str = "editor.viewport.fit-content";
const ACTION_VIEWPORT_FIT_SELECTION: &str = "editor.viewport.fit-selection";
#[cfg(test)]
const ACTION_VIEWPORT_ACTUAL_SIZE: &str = "editor.viewport.actual-size";
#[cfg(test)]
const ACTION_VIEWPORT_ZOOM_IN: &str = "editor.viewport.zoom-in";
#[cfg(test)]
const ACTION_VIEWPORT_ZOOM_OUT: &str = "editor.viewport.zoom-out";
#[cfg(test)]
const ACTION_VIEWPORT_PAN: &str = "editor.viewport.pan";
/// Builds the editor project once the build pipeline is implemented.
pub const ACTION_BUILD: &str = "editor.build";
const ACTION_PACKAGE_WINDOWS: &str = "editor.package.windows-x64";
const ACTION_RUN_PROFILER: &str = "editor.profiler.run";
const ACTION_PROJECT_SETTINGS: &str = "editor.project-settings.open";
/// Opens the editor command palette once the palette lifecycle is implemented.
pub const ACTION_PALETTE: &str = "editor.palette";
const ACTION_DOCS: &str = "editor.docs.open";
const ACTION_KEYBOARD_SHORTCUTS: &str = "editor.shortcuts.open";
const ACTION_ABOUT: &str = "editor.about.open";
const ACTION_ABOUT_CLOSE: &str = "editor.about.close";
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
