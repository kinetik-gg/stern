//! Interactive showcase app state and rendering.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

mod runtime;
#[cfg(test)]
mod tests;

use kinetik_ui::core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionInvocation, ActionPriority, ActionQueue,
    ActionRouter, ActionRoutingContext, ActionSource, Axis, Brush, ClipId, Color, CornerRadius,
    FrameContext, FrameOutput, ImageId, Insets, Key, KeyEvent, KeyState, LayoutItem, LinePrimitive,
    Measurement, Modifiers, PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput,
    Primitive, Rect, RectPrimitive, RepaintRequest, ScaleFactor, SemanticNode, SemanticRole,
    Shortcut, Size, SizeRule, Stroke, TextInputEvent, TextPrimitive, TextureId, TexturePrimitive,
    TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, default_dark_theme, inspect_primitives,
    rect_from_size, split_leading,
};
use kinetik_ui::render::{
    ImageResource, RenderImage, RenderImageSampling, RenderResources, TextLayoutResourceSync,
    TextureResource,
};
use kinetik_ui::text::{TextEditState, TextLayoutStore};
use kinetik_ui::widgets::{
    CommandPaletteOverlay, Crosshair, Dock, DockDropTarget, DockNode, DockPlacement, Frame,
    FrameId, GridColumns, GridLayout, Guide, IconId, ItemId, ListLayout, Menu, MenuOverlay,
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PanZoom, Panel, PanelId,
    PopoverPlacement, PopoverRequest, TableColumn, TableLayout, Ui, ViewportComposition,
    ViewportSurface, frame_tabs, overlay_semantics, place_popover, solve_dock_layout,
    solve_dock_splitters,
};

use crate::editor::{self as editor_showcase, EditorShowcase};

const MIN_VIEWPORT_WIDTH: f32 = 1.0;
const MIN_VIEWPORT_HEIGHT: f32 = 1.0;
const ACTION_COMPONENTS_RUN: &str = "components.counter.increment";
const ACTION_SYSTEMS_DISPATCH: &str = "systems.dispatch.record";
const ACTION_WORKSPACE_SAVE: &str = "workspace.snapshot.capture";
const ACTION_COMMAND_PALETTE: &str = "command.palette.open";
const ACTION_VIEWPORT_GRID: &str = "viewport.grid.toggle";
const ACTION_EDITOR_DOCK_JOIN: &str = "editor.dock.join";
const ACTION_EDITOR_DOCK_SWAP: &str = "editor.dock.swap";
const EXPERIMENTAL_SUFFIX: &str = " (Experimental)";

/// Available showcase pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowcasePage {
    /// DCC/game-engine editor workbench.
    Editor,
    /// Component gallery and controls.
    Components,
    /// Layout, docking, and collection primitives.
    Layout,
    /// Viewport/media surface primitives.
    Viewport,
    /// Actions, overlays, diagnostics, and stress.
    Systems,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum DockSplitDemoState {
    #[default]
    Base,
    Inserted,
}

#[derive(Debug, Clone, PartialEq)]
struct ShowcaseWorkspaceSnapshot {
    page: ShowcasePage,
    selected_row: usize,
    selected_tab: usize,
    checkbox: bool,
    toggle: bool,
    radio: usize,
    strength: f32,
    dock_ratio: f32,
    dock_split_demo: DockSplitDemoState,
    zoom: f32,
    stress: usize,
    name: String,
    number: String,
    search: String,
    notes: String,
}

impl ShowcasePage {
    /// Every showcase page in canonical navigation and tooling order.
    pub const ALL: [Self; 5] = [
        Self::Editor,
        Self::Components,
        Self::Layout,
        Self::Viewport,
        Self::Systems,
    ];

    /// Stable lowercase page slug used by command-line tools and artifacts.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Editor => "editor",
            Self::Components => "components",
            Self::Layout => "layout",
            Self::Viewport => "viewport",
            Self::Systems => "systems",
        }
    }

    /// Human-readable page label used by showcase navigation.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Editor => "Editor",
            Self::Components => "Components",
            Self::Layout => "Layout",
            Self::Viewport => "Viewport",
            Self::Systems => "Systems",
        }
    }

    /// Parses a canonical slug or label, including compatibility aliases.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "editor" | "engine" | "dcc" | "workbench" => Some(Self::Editor),
            "components" | "component" => Some(Self::Components),
            "layout" | "layouts" => Some(Self::Layout),
            "viewport" | "viewports" => Some(Self::Viewport),
            "systems" | "system" => Some(Self::Systems),
            _ => None,
        }
    }
}

/// Window/input snapshot consumed by the showcase.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShowcaseInput {
    /// Mouse position in logical pixels.
    pub mouse: Option<Point>,
    /// Current window or render target size in physical pixels.
    pub viewport_size: Option<Size>,
    /// Whether the primary button is down.
    pub mouse_down: bool,
    /// Characters typed this frame.
    pub typed: Vec<char>,
    /// Backspace pressed this frame.
    pub backspace: bool,
    /// Enter pressed this frame.
    pub enter: bool,
}

/// Interactive showcase app.
pub struct ShowcaseApp {
    page: ShowcasePage,
    memory: UiMemory,
    text_layouts: TextLayoutStore,
    previous_mouse_down: bool,
    previous_mouse: Option<Point>,
    viewport_size: Size,
    action_count: u32,
    component_action_count: u32,
    systems_dispatch_count: u32,
    workspace_snapshot: Option<ShowcaseWorkspaceSnapshot>,
    selected_row: usize,
    selected_tab: usize,
    checkbox: bool,
    toggle: bool,
    radio: usize,
    strength: f32,
    dock_ratio: f32,
    dock_split_demo: DockSplitDemoState,
    zoom: f32,
    stress: usize,
    name: TextEditState,
    number: TextEditState,
    search: TextEditState,
    notes: TextEditState,
    status: String,
    pending_platform_requests: Vec<PlatformRequest>,
    output: FrameOutput,
    editor: EditorShowcase,
    render_resources: RenderResources,
    text_resource_sync: TextLayoutResourceSync,
}

impl Default for ShowcaseApp {
    fn default() -> Self {
        let mut app = Self {
            page: ShowcasePage::Editor,
            memory: UiMemory::new(),
            text_layouts: TextLayoutStore::new(),
            previous_mouse_down: false,
            previous_mouse: None,
            viewport_size: Size::new(1440.0, 900.0),
            action_count: 0,
            component_action_count: 0,
            systems_dispatch_count: 0,
            workspace_snapshot: None,
            selected_row: 1,
            selected_tab: 0,
            checkbox: true,
            toggle: false,
            radio: 0,
            strength: 0.62,
            dock_ratio: 0.42,
            dock_split_demo: DockSplitDemoState::Base,
            zoom: 0.48,
            stress: 128,
            name: TextEditState::new("Workspace"),
            number: TextEditState::new("42"),
            search: TextEditState::new("layout"),
            notes: TextEditState::new("First line\nSecond line"),
            status: "Ready".to_owned(),
            pending_platform_requests: Vec::new(),
            output: FrameOutput::new(),
            editor: EditorShowcase::new(),
            render_resources: static_render_resources(),
            text_resource_sync: TextLayoutResourceSync::new(),
        };
        app.redraw_idle();
        app
    }
}

fn showcase_actions() -> Vec<ActionDescriptor> {
    let mut save = ActionDescriptor::new(ACTION_WORKSPACE_SAVE, "Save Workspace");
    save.keywords = vec!["write".to_owned(), "persist".to_owned()];
    let mut palette = ActionDescriptor::new(
        ACTION_COMMAND_PALETTE,
        format!("Open Command Palette{EXPERIMENTAL_SUFFIX}"),
    );
    palette.keywords = vec!["search".to_owned(), "actions".to_owned()];
    palette.state.enabled = false;
    let mut toggle_grid = ActionDescriptor::new(
        ACTION_VIEWPORT_GRID,
        format!("Toggle Viewport Grid{EXPERIMENTAL_SUFFIX}"),
    );
    toggle_grid.keywords = vec!["guides".to_owned(), "overlay".to_owned()];
    toggle_grid.state.enabled = false;
    vec![save, palette, toggle_grid]
}

fn showcase_action_router(play_enabled: bool) -> ActionRouter {
    let mut play = ActionDescriptor::new(editor_showcase::ACTION_PLAY, "Play");
    play.shortcut = Some(Shortcut::new(Modifiers::default(), Key::Function(5)));
    play.state.enabled = play_enabled;
    let mut grid = ActionDescriptor::new(editor_showcase::ACTION_GRID, "Toggle Grid");
    grid.shortcut = Some(Shortcut::new(
        Modifiers::default(),
        Key::Character("g".to_owned()),
    ));

    let mut router = ActionRouter::new();
    let mut documentation =
        ActionDescriptor::new(editor_showcase::ACTION_DOCS, "Open Online Documentation");
    documentation.shortcut = Some(Shortcut::new(Modifiers::default(), Key::Function(1)));

    for action in [play, grid, documentation] {
        router.bind(ActionBinding::new(
            action,
            ActionContext::Global,
            ActionPriority::Global,
        ));
    }
    router
}

fn nav_items(viewport_width: f32) -> [(ShowcasePage, Rect); 5] {
    let (start, widths, gap) = if viewport_width >= 940.0 {
        (300.0, [72.0, 122.0, 82.0, 104.0, 92.0], 10.0)
    } else {
        (170.0, [60.0, 96.0, 66.0, 84.0, 72.0], 8.0)
    };
    let mut x = start;
    std::array::from_fn(|index| {
        let item = (
            ShowcasePage::ALL[index],
            Rect::new(x, 12.0, widths[index], 28.0),
        );
        x += widths[index] + gap;
        item
    })
}

fn editor_nav_bounds(viewport: Rect) -> Rect {
    let width = viewport.width.min(408.0);
    let height = viewport.height.min(24.0);
    Rect::new(
        viewport.x + (viewport.width - width).max(0.0) * 0.5,
        viewport.max_y() - height,
        width,
        height,
    )
}

fn editor_nav_items(viewport: Rect) -> [(ShowcasePage, Rect); 5] {
    let bounds = editor_nav_bounds(viewport);
    let scale = bounds.width / 408.0;
    let widths = [56.0, 104.0, 64.0, 84.0, 76.0];
    let gap = 4.0 * scale;
    let horizontal_padding = 4.0 * scale;
    let vertical_padding = (bounds.height / 12.0).min(2.0);
    let item_height = (bounds.height - vertical_padding * 2.0).max(0.0);
    let mut x = bounds.x + horizontal_padding;
    std::array::from_fn(|index| {
        let width = widths[index] * scale;
        let item = (
            ShowcasePage::ALL[index],
            Rect::new(x, bounds.y + vertical_padding, width, item_height),
        );
        x += width + gap;
        item
    })
}

fn static_render_resources() -> RenderResources {
    let mut resources = RenderResources::new();
    editor_showcase::register_resources(&mut resources);
    resources.register_image(ImageResource {
        id: ImageId::from_raw(7),
        size: Size::new(64.0, 48.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: Some(thumbnail_image()),
        atlas_region: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(11),
        size: Size::new(96.0, 72.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: Some(primitive_image()),
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(99),
        size: Size::new(384.0, 216.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: Some(viewport_texture()),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(101),
        size: Size::new(256.0, 144.0),
        sampling: RenderImageSampling::Smooth,
        snapshot: Some(video_texture()),
    });
    resources
}

fn frame_context(size: Size, input: UiInput) -> FrameContext {
    let width = physical_dimension(size.width);
    let height = physical_dimension(size.height);
    FrameContext::new(
        ViewportInfo::new(size, PhysicalSize::new(width, height), ScaleFactor::ONE),
        input,
        TimeInfo::default(),
    )
}

fn page_rect(viewport: Rect) -> Rect {
    kinetik_ui::core::pad_rect(viewport, Insets::new(40.0, 40.0, 104.0, 40.0))
}

fn physical_dimension(value: f32) -> u32 {
    if value.is_finite() {
        value.round().max(1.0).min(u32::MAX as f32) as u32
    } else {
        1
    }
}

fn thumbnail_image() -> RenderImage {
    rgba_image(64, 48, |x, y| {
        let active = x > 8 && x < 56 && y > 8 && y < 40;
        if active {
            [77, 83, 93, 255]
        } else {
            [36, 39, 45, 255]
        }
    })
}

fn primitive_image() -> RenderImage {
    rgba_image(96, 72, |x, y| {
        let highlight = y == 8 && (12..84).contains(&x);
        if highlight {
            [136, 140, 148, 255]
        } else if y > 14 && x > 8 && x < 88 {
            [78, 80, 86, 255]
        } else {
            [44, 46, 52, 255]
        }
    })
}

fn viewport_texture() -> RenderImage {
    rgba_image(384, 216, |x, y| {
        let stripe = (x / 48) % 2 == 0;
        let guide = x == 192 || y == 108 || y == 72;
        if guide {
            [180, 205, 232, 255]
        } else if stripe {
            [28, 35, 44, 255]
        } else {
            [23, 29, 37, 255]
        }
    })
}

fn video_texture() -> RenderImage {
    rgba_image(256, 144, |x, y| {
        let checker = ((x / 20) + (y / 20)) % 2 == 0;
        let guide = x == 128 || y == 72;
        if guide {
            [96, 138, 184, 255]
        } else if checker {
            [29, 36, 46, 255]
        } else {
            [22, 27, 35, 255]
        }
    })
}

fn rgba_image(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> RenderImage {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            data.extend_from_slice(&pixel(x, y));
        }
    }
    RenderImage::rgba8(width, height, data).expect("generated RGBA image has matching byte length")
}

fn section_title(ui: &mut Ui<'_>, x: f32, baseline: f32, value: &str) {
    text(ui, x, baseline, value, 18.0, rgb(242, 242, 244));
}

fn panel_title(ui: &mut Ui<'_>, rect_value: Rect, value: &str) {
    let _ = panel_title_body(ui, rect_value, value, Insets::new(20.0, 20.0, 46.0, 18.0));
}

fn panel_title_body(ui: &mut Ui<'_>, rect_value: Rect, value: &str, body_insets: Insets) -> Rect {
    let frame = ui.panel_frame(rect_value, body_insets);
    text(
        ui,
        rect_value.x + 20.0,
        rect_value.y + 30.0,
        value,
        14.0,
        rgb(238, 238, 240),
    );
    frame.body
}

fn rect(ui: &mut Ui<'_>, rect: Rect, fill: Color, stroke: Option<Color>) {
    ui.primitive(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(fill)),
        stroke: stroke.map(|stroke| Stroke::new(1.0, Brush::Solid(stroke))),
        radius: CornerRadius::all(0.0),
    }));
}

fn line(ui: &mut Ui<'_>, from: Point, to: Point, color: Color, width: f32) {
    ui.primitive(Primitive::Line(LinePrimitive {
        from,
        to,
        stroke: Stroke::new(width, Brush::Solid(color)),
    }));
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

fn sanitize_viewport_size(size: Size) -> Size {
    Size::new(
        size.width.max(MIN_VIEWPORT_WIDTH),
        size.height.max(MIN_VIEWPORT_HEIGHT),
    )
}
