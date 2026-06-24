//! Interactive showcase app state and rendering.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use kinetik_ui::core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionPriority, ActionQueue, ActionRouter,
    ActionRoutingContext, ActionSource, Axis, Brush, ClipId, Color, CornerRadius, FrameContext,
    FrameOutput, ImageId, Insets, Key, KeyEvent, KeyState, LayoutItem, LinePrimitive, Measurement,
    Modifiers, PhysicalSize, Point, PointerButtonState, PointerInput, Primitive, Rect,
    RectPrimitive, RepaintRequest, ScaleFactor, SemanticNode, SemanticRole, Shortcut, Size,
    SizeRule, Stroke, TextInputEvent, TextPrimitive, TextureId, TexturePrimitive, TimeInfo,
    UiInput, UiMemory, Vec2, ViewportInfo, column_layout, default_dark_theme, inspect_primitives,
    rect_from_size, row_layout, split_leading,
};
use kinetik_ui::render::{
    ImageResource, RenderImage, RenderImageSampling, RenderResources, TextureResource,
};
use kinetik_ui::text::{TextEditState, TextLayoutStore};
use kinetik_ui::widgets::{
    CommandPalette, Crosshair, Dock, DockDropTarget, DockNode, DockPlacement, Frame, FrameId,
    GridColumns, GridLayout, Guide, IconId, ItemId, ListLayout, Menu, OverlayDismissal,
    OverlayEntry, OverlayId, OverlayKind, OverlayStack, PanZoom, Panel, PanelId, PopoverPlacement,
    PopoverRequest, TableColumn, TableLayout, Ui, ViewportComposition, ViewportSurface, frame_tabs,
    overlay_semantics, place_popover, solve_dock_layout, solve_dock_splitters,
};

use crate::editor::{self as editor_showcase, EditorShowcase};

const MIN_VIEWPORT_WIDTH: f32 = 1.0;
const MIN_VIEWPORT_HEIGHT: f32 = 1.0;

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

impl ShowcasePage {
    fn label(self) -> &'static str {
        match self {
            Self::Editor => "Editor",
            Self::Components => "Components",
            Self::Layout => "Layout",
            Self::Viewport => "Viewport",
            Self::Systems => "Systems",
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
    output: FrameOutput,
    editor: EditorShowcase,
    static_resources: RenderResources,
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
            output: FrameOutput::new(),
            editor: EditorShowcase::new(),
            static_resources: static_render_resources(),
        };
        app.redraw_idle();
        app
    }
}

impl ShowcaseApp {
    /// Creates a showcase app.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current page.
    #[must_use]
    pub const fn page(&self) -> ShowcasePage {
        self.page
    }

    /// Selects a page and redraws without input.
    pub fn set_page(&mut self, page: ShowcasePage) {
        self.page = page;
        self.status = format!("Page: {}", page.label());
        self.redraw_idle();
    }

    /// Parses a page name used by showcase tooling.
    #[must_use]
    pub fn page_from_name(name: &str) -> Option<ShowcasePage> {
        match name.trim().to_ascii_lowercase().as_str() {
            "editor" | "engine" | "dcc" | "workbench" => Some(ShowcasePage::Editor),
            "components" | "component" => Some(ShowcasePage::Components),
            "layout" | "layouts" => Some(ShowcasePage::Layout),
            "viewport" | "viewports" => Some(ShowcasePage::Viewport),
            "systems" | "system" => Some(ShowcasePage::Systems),
            _ => None,
        }
    }

    /// Action invocation count.
    #[must_use]
    pub const fn action_count(&self) -> u32 {
        self.action_count
    }

    /// Slider value.
    #[must_use]
    pub const fn strength(&self) -> f32 {
        self.strength
    }

    /// Viewport zoom slider value.
    #[must_use]
    pub const fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Current viewport size.
    #[must_use]
    pub const fn viewport_size(&self) -> Size {
        self.viewport_size
    }

    /// Sets the logical viewport size used for layout.
    pub fn set_viewport_size(&mut self, size: Size) {
        let size = sanitize_viewport_size(size);
        if self.viewport_size == size {
            return;
        }
        self.viewport_size = size;
        self.previous_mouse = None;
        self.redraw_idle();
    }

    /// Current search query.
    #[must_use]
    pub fn search(&self) -> &str {
        &self.search.text
    }

    /// Current multi-line notes text.
    #[must_use]
    pub fn notes(&self) -> &str {
        &self.notes.text
    }

    /// Applies input and updates the cached primitive stream.
    pub fn update(&mut self, input: &ShowcaseInput) {
        let viewport_changed = input.viewport_size.is_some_and(|size| {
            let size = sanitize_viewport_size(size);
            size != self.viewport_size
        });
        if let Some(size) = input.viewport_size {
            self.viewport_size = sanitize_viewport_size(size);
        }

        let ui_input = self.to_ui_input(input, viewport_changed);
        let keyboard = ui_input.keyboard.clone();

        self.resolve_shortcuts(&keyboard);
        self.output = self.frame(frame_context(self.viewport_size, ui_input));
        self.previous_mouse_down = input.mouse_down;
        self.previous_mouse = input.mouse;
    }

    /// Applies a full toolkit frame context from a platform adapter.
    pub fn update_with_context(&mut self, context: FrameContext) {
        self.viewport_size = sanitize_viewport_size(context.viewport.logical_size);
        let keyboard = context.input.keyboard.clone();
        self.resolve_shortcuts(&keyboard);
        self.output = self.frame(context);
    }

    /// Builds the current primitive stream.
    #[must_use]
    pub fn primitives(&self) -> Vec<Primitive> {
        self.output.primitives.clone()
    }

    /// Returns the full toolkit frame output for diagnostics and integration.
    #[must_use]
    pub const fn output(&self) -> &FrameOutput {
        &self.output
    }

    /// Builds render resources referenced by the current showcase frame.
    #[must_use]
    pub fn render_resources(&self) -> RenderResources {
        let mut resources = self.static_resources.clone();
        resources.register_text_layouts(self.text_layouts.layouts());
        resources
    }

    fn redraw_idle(&mut self) {
        self.output = self.frame(frame_context(self.viewport_size, UiInput::default()));
    }

    fn frame(&mut self, context: FrameContext) -> FrameOutput {
        let theme = default_dark_theme();
        let mut memory = std::mem::take(&mut self.memory);
        let mut text_layouts = std::mem::take(&mut self.text_layouts);
        let mut editor_invocations = Vec::new();
        let mut output = {
            let mut ui =
                Ui::begin_frame_with_text_layouts(context, &mut memory, &theme, &mut text_layouts);

            if self.page == ShowcasePage::Editor {
                editor_invocations = self.editor.render(&mut ui, self.action_count);
            } else {
                Self::app_background(&mut ui);
                self.nav_interactions(&mut ui);
                self.page_content(&mut ui);
                self.chrome(&mut ui);
            }

            ui.finish_output()
        };
        let editor_invoked = !editor_invocations.is_empty();
        for invocation in editor_invocations {
            self.record_action(invocation.action_id, invocation.source);
        }
        if editor_invoked {
            output.request_repaint(RepaintRequest::NextFrame);
        }
        self.memory = memory;
        self.text_layouts = text_layouts;
        output
    }

    fn to_ui_input(&self, input: &ShowcaseInput, viewport_changed: bool) -> UiInput {
        let mouse = input.mouse;
        let pressed = input.mouse_down && !self.previous_mouse_down;
        let released = !input.mouse_down && self.previous_mouse_down;
        let delta = match (mouse, self.previous_mouse) {
            _ if viewport_changed => Vec2::ZERO,
            (Some(current), Some(previous)) => {
                Vec2::new(current.x - previous.x, current.y - previous.y)
            }
            _ => Vec2::ZERO,
        };
        let mut events = Vec::new();
        if input.backspace {
            events.push(KeyEvent::new(
                Key::Backspace,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            ));
        }
        if input.enter {
            events.push(KeyEvent::new(
                Key::Enter,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            ));
        }
        let committed = input.typed.iter().collect::<String>();
        let text_events = if committed.is_empty() {
            Vec::new()
        } else {
            vec![TextInputEvent::Commit(committed)]
        };

        UiInput {
            pointer: PointerInput {
                position: mouse,
                delta,
                primary: PointerButtonState::new(input.mouse_down, pressed, released),
                ..PointerInput::default()
            },
            keyboard: kinetik_ui::core::KeyboardInput {
                modifiers: Modifiers::default(),
                events,
            },
            text_events,
            clipboard_text: Vec::new(),
            window_focused: true,
        }
    }

    fn invoke_action(&mut self, id: &str, source: ActionSource) {
        self.editor.apply_action(id);
        self.record_action(id, source);
    }

    fn record_action(&mut self, action_id: &str, source: ActionSource) {
        self.action_count += 1;
        self.status = format!("{} via {:?} ({})", action_id, source, self.action_count);
    }

    fn resolve_shortcuts(&mut self, keyboard: &kinetik_ui::core::KeyboardInput) {
        let Some(invocation) =
            showcase_action_router().resolve_shortcut_in_context(keyboard, self.action_context())
        else {
            return;
        };
        self.invoke_action(invocation.action_id.as_str(), invocation.source);
    }

    fn action_context(&self) -> ActionRoutingContext {
        let Some(focused) = self.memory.focused() else {
            return ActionRoutingContext::new();
        };
        if self.memory.text_input_owner() == Some(focused) {
            ActionRoutingContext::new().with_text_input(focused)
        } else {
            ActionRoutingContext::new().with_focused_widget(focused)
        }
    }

    fn app_background(ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let (_, body) = split_leading(viewport, Axis::Vertical, 52.0);
        rect(ui, viewport, rgb(11, 12, 13), None);
        rect(
            ui,
            Rect::new(0.0, 52.0, viewport.width, 1.0),
            rgb(65, 72, 84),
            None,
        );
        let footer_height = body.height.min(140.0);
        rect(
            ui,
            Rect::new(
                0.0,
                viewport.max_y() - footer_height,
                viewport.width,
                footer_height,
            ),
            rgb(13, 16, 17),
            None,
        );
    }

    fn page_content(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let scroll_bounds = Rect::new(0.0, 52.0, viewport.width, (viewport.height - 52.0).max(1.0));
        let content_size = Size::new(viewport.width, self.page_content_height(viewport));
        ui.scroll_area(
            ("showcase.page-scroll", self.page as u8),
            scroll_bounds,
            content_size,
            false,
            |ui, _| match self.page {
                ShowcasePage::Editor => {
                    let _ = ui;
                }
                ShowcasePage::Components => self.components_page(ui),
                ShowcasePage::Layout => self.layout_page(ui),
                ShowcasePage::Viewport => self.viewport_page(ui),
                ShowcasePage::Systems => self.systems_page(ui),
            },
        );
    }

    fn page_content_height(&self, viewport: Rect) -> f32 {
        let page = page_rect(viewport);
        let height: f32 = match self.page {
            ShowcasePage::Editor => viewport.height,
            ShowcasePage::Components | ShowcasePage::Layout if page.width >= 1160.0 => 840.0,
            ShowcasePage::Components => 1320.0,
            ShowcasePage::Layout => 1180.0,
            ShowcasePage::Viewport if page.width >= 1160.0 => 780.0,
            ShowcasePage::Viewport => 1160.0,
            ShowcasePage::Systems if page.width >= 1220.0 => 780.0,
            ShowcasePage::Systems if page.width >= 820.0 => 1120.0,
            ShowcasePage::Systems => 1340.0,
        };
        height.max(viewport.height)
    }

    fn chrome(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        rect(
            ui,
            Rect::new(0.0, 0.0, viewport.width, 52.0),
            rgb(19, 21, 23),
            Some(rgb(58, 64, 72)),
        );
        rect(ui, Rect::new(0.0, 0.0, 6.0, 52.0), rgb(82, 150, 132), None);
        text(
            ui,
            20.0,
            24.0,
            "Kinetik UI Showcase",
            15.0,
            rgb(238, 238, 238),
        );
        text(ui, 20.0, 40.0, "Workbench", 10.0, rgb(150, 160, 164));
        for (page, item) in nav_items(viewport.width) {
            let response = ui.tab_button_value(
                ("nav", page as u8),
                item,
                page.label(),
                &mut self.page,
                page,
                false,
            );
            if response.clicked {
                self.status = format!("Page: {}", page.label());
            }
        }

        if viewport.width >= 1200.0 {
            Self::status_badge(
                ui,
                Rect::new(viewport.width - 434.0, 12.0, 128.0, 28.0),
                "Primitives",
                &self.output.primitives.len().to_string(),
                rgb(82, 150, 132),
            );
            Self::status_badge(
                ui,
                Rect::new(viewport.width - 294.0, 12.0, 108.0, 28.0),
                "Actions",
                &self.action_count.to_string(),
                rgb(144, 184, 255),
            );
            text(
                ui,
                viewport.width - 170.0,
                31.0,
                &self.status,
                10.0,
                rgb(178, 182, 188),
            );
        }
    }

    fn nav_interactions(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        for (page, item) in nav_items(viewport.width) {
            let response = ui.pressable(("nav.prepass", page as u8), item, false);
            if response.clicked {
                self.page = page;
                self.status = format!("Page: {}", page.label());
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    fn status_badge(ui: &mut Ui<'_>, rect_value: Rect, label: &str, value: &str, accent: Color) {
        rect(ui, rect_value, rgb(26, 28, 31), Some(rgb(62, 68, 76)));
        rect(
            ui,
            Rect::new(rect_value.x, rect_value.y, 3.0, rect_value.height),
            accent,
            None,
        );
        text(
            ui,
            rect_value.x + 10.0,
            rect_value.y + 11.0,
            label,
            8.0,
            rgb(142, 148, 156),
        );
        text(
            ui,
            rect_value.x + 76.0,
            rect_value.y + 19.0,
            value,
            11.0,
            rgb(232, 234, 238),
        );
    }

    fn components_page(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        section_title(ui, page.x, 86.0, "Component Gallery");

        if page.width >= 1160.0 {
            self.component_controls(ui, Rect::new(page.x, page.y, 620.0, 218.0));
            self.component_text_inputs(ui, Rect::new(page.x + 660.0, page.y, 500.0, 218.0));
            self.collection_preview(ui, Rect::new(page.x, page.y + 246.0, 560.0, 190.0));
            self.tabs_preview(ui, Rect::new(page.x + 600.0, page.y + 246.0, 560.0, 190.0));
            Self::primitive_preview(ui, Rect::new(page.x, page.y + 466.0, 1160.0, 230.0));
        } else {
            let width = page.width.min(900.0);
            self.component_controls(ui, Rect::new(page.x, page.y, width, 218.0));
            self.component_text_inputs(ui, Rect::new(page.x, page.y + 242.0, width, 218.0));
            self.collection_preview(ui, Rect::new(page.x, page.y + 484.0, width, 190.0));
            self.tabs_preview(ui, Rect::new(page.x, page.y + 698.0, width, 190.0));
            Self::primitive_preview(ui, Rect::new(page.x, page.y + 912.0, width, 230.0));
        }
    }

    fn component_controls(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Controls");
        let x = panel.x + 20.0;
        let y = panel.y + 40.0;
        let compact = panel.width < 560.0;
        let slider_x = if compact { x } else { x + 300.0 };
        let slider_y = if compact { y + 88.0 } else { y + 8.0 };
        let slider_width = if compact {
            (panel.width - 40.0).max(120.0)
        } else {
            (panel.max_x() - slider_x - 60.0).clamp(160.0, 240.0)
        };

        self.component_button_controls(ui, x, y);
        self.component_selection_controls(ui, x, y);
        self.component_slider_controls(ui, panel, slider_x, slider_y, slider_width);
        Self::state_strip(
            ui,
            Rect::new(
                x,
                panel.max_y() - 46.0,
                (panel.width - 40.0).max(120.0),
                24.0,
            ),
            &format!(
                "checkbox={} toggle={} radio={} selected_row={}",
                self.checkbox,
                self.toggle,
                self.radio + 1,
                self.selected_row + 1
            ),
        );
    }

    fn component_button_controls(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
        let run = ui.button(
            "components.run-action",
            Rect::new(x, y, 128.0, 30.0),
            "Run Action",
            false,
        );
        if run.clicked {
            self.invoke_action("components.run", ActionSource::Button);
        }

        let disabled = ui.button(
            "components.disabled",
            Rect::new(x + 144.0, y, 128.0, 30.0),
            "Disabled",
            true,
        );
        if disabled.clicked {
            "Disabled button should not invoke".clone_into(&mut self.status);
        }
    }

    fn component_selection_controls(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
        let checkbox = ui.checkbox_value(
            "components.checkbox",
            Rect::new(x, y + 48.0, 22.0, 22.0),
            &mut self.checkbox,
            false,
        );
        if checkbox.clicked {
            self.status = format!("Checkbox: {}", self.checkbox);
        }
        ui.label(Rect::new(x + 32.0, y + 46.0, 90.0, 20.0), "Checkbox");

        let toggle = ui.toggle_value(
            "components.toggle",
            Rect::new(x + 144.0, y + 48.0, 54.0, 24.0),
            &mut self.toggle,
            false,
        );
        if toggle.clicked {
            self.status = format!("Toggle: {}", self.toggle);
        }
        ui.label(Rect::new(x + 210.0, y + 46.0, 70.0, 20.0), "Toggle");

        for (index, radio_x, label) in [(0, x, "Radio A"), (1, x + 100.0, "Radio B")] {
            let response = ui.radio_button_value(
                ("components.radio", index),
                Rect::new(radio_x, y + 94.0, 20.0, 20.0),
                &mut self.radio,
                index,
                false,
            );
            if response.clicked {
                self.status = format!("Radio: {label}");
            }
            ui.label(Rect::new(radio_x + 30.0, y + 92.0, 70.0, 20.0), label);
        }
    }

    fn component_slider_controls(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        slider_x: f32,
        slider_y: f32,
        slider_width: f32,
    ) {
        let before = self.strength;
        ui.slider(
            "components.slider",
            Rect::new(slider_x, slider_y, slider_width, 16.0),
            &mut self.strength,
            0.0..=1.0,
            false,
        );
        text(
            ui,
            slider_x,
            slider_y - 10.0,
            &format!("Slider: {:.2}", self.strength),
            10.0,
            rgb(210, 210, 214),
        );
        if (before - self.strength).abs() > f32::EPSILON {
            self.status = format!("Slider: {:.2}", self.strength);
        }

        let icon_button_size = ui.theme().controls.control_height;
        ui.icon_button(
            "components.icon",
            Rect::new(
                slider_x,
                slider_y + 44.0,
                icon_button_size,
                icon_button_size,
            ),
            IconId::from_raw(1),
            false,
        );
        ui.label(
            Rect::new(slider_x + 44.0, slider_y + 50.0, 90.0, 20.0),
            "Icon button",
        );
        if panel.width >= 560.0 {
            ui.image(
                Rect::new(slider_x + 152.0, slider_y + 36.0, 64.0, 48.0),
                ImageId::from_raw(7),
            );
            ui.label(
                Rect::new(slider_x + 152.0, slider_y + 96.0, 120.0, 20.0),
                "Thumbnail",
            );
        }
    }

    fn component_text_inputs(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Text Input");
        let x = panel.x + 20.0;
        let y = panel.y + 46.0;
        let compact = panel.width < 460.0;
        let primary_width = if compact {
            (panel.width - 40.0).max(140.0)
        } else {
            (panel.width * 0.46).clamp(190.0, 260.0)
        };
        let secondary_x = if compact { x } else { x + primary_width + 50.0 };
        let secondary_y = y + 64.0;
        let secondary_width = if compact {
            primary_width.min(160.0)
        } else {
            (panel.max_x() - secondary_x - 20.0).clamp(100.0, 160.0)
        };

        text(ui, x, y - 8.0, "Search", 10.0, rgb(190, 190, 194));
        let search = ui.search_field(
            "components.search",
            Rect::new(x, y, primary_width, 30.0),
            &mut self.search,
            false,
        );
        if search.field.changed {
            self.status = format!("Search: {}", search.query);
        }

        text(ui, x, y + 56.0, "Text field", 10.0, rgb(190, 190, 194));
        let name = ui.text_field(
            "components.name",
            Rect::new(x, y + 64.0, primary_width.min(220.0), 30.0),
            &mut self.name,
            false,
        );
        if name.changed {
            self.status = format!("Name: {}", self.name.text);
        }

        text(
            ui,
            secondary_x,
            secondary_y - 8.0,
            "Numeric",
            10.0,
            rgb(190, 190, 194),
        );
        let number = ui.numeric_input(
            "components.number",
            Rect::new(secondary_x, secondary_y, secondary_width, 30.0),
            &mut self.number,
            false,
        );
        if number.field.changed {
            self.status = if number.valid {
                format!("Number: {}", self.number.text)
            } else {
                "Number field is invalid".to_owned()
            };
        }

        let notes_y = if compact { y + 118.0 } else { y + 120.0 };
        text(ui, x, notes_y - 8.0, "Multi-line", 10.0, rgb(190, 190, 194));
        let notes = ui.multi_line_text_field(
            "components.notes",
            Rect::new(x, notes_y, (panel.width - 80.0).max(160.0), 38.0),
            &mut self.notes,
            false,
        );
        if notes.changed {
            self.status = format!("Notes: {} lines", self.notes.text.lines().count());
        }

        text(
            ui,
            x + (panel.width - 160.0).max(0.0),
            notes_y + 30.0,
            "Undo stack",
            10.0,
            rgb(160, 160, 164),
        );
    }

    fn collection_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Lists, Grids, Tables");
        let x = panel.x + 20.0;
        let y = panel.y + 40.0;
        let list_width = (panel.width * 0.45).clamp(220.0, 260.0);
        let grid_x = if panel.width >= 520.0 {
            x + list_width + 50.0
        } else {
            x
        };
        let grid_y = if panel.width >= 520.0 { y } else { y + 124.0 };

        let list = ListLayout::new(28.0);
        let labels = [
            "Row: primary surface",
            "Row: selected state",
            "Row: cached resource",
            "Row: async result",
        ];
        for row in list.row_rects(Rect::new(x, y, list_width, 112.0), labels.len(), 0..4) {
            let response = ui.list_row_value(
                ("components.list-row", row.index),
                row.rect,
                labels[row.index],
                &mut self.selected_row,
                row.index,
                false,
            );
            if response.clicked {
                self.status = format!("Selected row {}", row.index + 1);
            }
        }

        let grid = GridLayout {
            columns: GridColumns::Fixed(4),
            item_size: Size::new(42.0, 30.0),
            gap: 12.0,
        };
        for item in grid.item_rects(
            Rect::new(
                grid_x,
                grid_y,
                (panel.max_x() - grid_x - 20.0).max(180.0),
                120.0,
            ),
            12,
            0..12,
        ) {
            rect(ui, item.rect, rgb(36, 38, 42), Some(rgb(70, 70, 74)));
        }
    }

    fn tabs_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Reusable Panel States");
        let x = panel.x + 20.0;
        let y = panel.y + 40.0;

        for (index, tab_x, label) in [
            (0, x, "Theme"),
            (1, x + 120.0, "State"),
            (2, x + 240.0, "Actions"),
        ] {
            let response = ui.tab_button_value(
                ("components.tab", index),
                Rect::new(tab_x, y, 108.0, 30.0),
                label,
                &mut self.selected_tab,
                index,
                false,
            );
            if response.clicked {
                self.status = format!("Tab: {label}");
            }
        }

        let body = match self.selected_tab {
            0 => "Palette: graphite, cyan, steel, signal blue.",
            1 => "State: focus, hover, active, selected, disabled.",
            _ => "Actions: toolbar, menu, palette, shortcut.",
        };
        rect(
            ui,
            Rect::new(x, y + 40.0, (panel.width - 40.0).max(140.0), 82.0),
            rgb(22, 22, 25),
            Some(rgb(62, 62, 66)),
        );
        text(ui, x + 20.0, y + 72.0, body, 11.0, rgb(224, 224, 226));
        text(
            ui,
            x + 20.0,
            y + 102.0,
            &format!("Actions: {}", self.action_count),
            12.0,
            rgb(144, 184, 255),
        );
    }

    fn primitive_preview(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Primitive Stream");
        let x = panel.x + 24.0;
        let y = panel.y + 48.0;
        rect(
            ui,
            Rect::new(x, y, 120.0, 72.0),
            rgb(46, 48, 54),
            Some(rgb(120, 120, 126)),
        );
        line(
            ui,
            Point::new(x + 146.0, y),
            Point::new(x + 266.0, y + 72.0),
            rgb(230, 230, 230),
            2.0,
        );
        ui.image(Rect::new(x + 296.0, y, 96.0, 72.0), ImageId::from_raw(11));
        text(ui, x + 436.0, y + 42.0, "Label", 13.0, rgb(238, 238, 238));
        rect(
            ui,
            Rect::new((x + 636.0).min(panel.max_x() - 160.0), y, 140.0, 72.0),
            rgb(12, 12, 13),
            Some(rgb(92, 132, 240)),
        );
        if panel.width >= 900.0 {
            ui.separator(Rect::new(x + 816.0, y + 32.0, 220.0, 12.0));
        }
    }

    fn state_strip(ui: &mut Ui<'_>, bounds: Rect, value: &str) {
        rect(ui, bounds, rgb(22, 22, 25), Some(rgb(58, 58, 62)));
        text(
            ui,
            bounds.x + 10.0,
            bounds.y + 16.0,
            value,
            10.0,
            rgb(190, 190, 194),
        );
    }

    fn layout_page(&mut self, ui: &mut Ui<'_>) {
        section_title(ui, 40.0, 86.0, "Layout, Docking, and Data Surfaces");
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        if page.width >= 1160.0 {
            Self::layout_solver_preview(ui, Rect::new(page.x, page.y, 560.0, 320.0));
            self.dock_preview(ui, Rect::new(page.x + 600.0, page.y, 560.0, 320.0));
            Self::table_preview(ui, Rect::new(page.x, page.y + 356.0, 1160.0, 300.0));
        } else {
            let width = page.width.min(760.0);
            Self::layout_solver_preview(ui, Rect::new(page.x, page.y, width, 320.0));
            self.dock_preview(ui, Rect::new(page.x, page.y + 356.0, width, 320.0));
            Self::table_preview(ui, Rect::new(page.x, page.y + 712.0, width, 300.0));
        }
    }

    fn layout_solver_preview(ui: &mut Ui<'_>, panel: Rect) {
        let body = panel_title_body(
            ui,
            panel,
            "Measurement-Aware Layout",
            Insets::new(20.0, 20.0, 46.0, 16.0),
        );

        ui.clip_rect("layout.measurement.body", body, |ui| {
            let row_bounds = Rect::new(body.x + 4.0, body.y, (body.width - 8.0).max(0.0), 42.0);
            let items = [
                LayoutItem::new(
                    SizeRule::Fixed(140.0),
                    SizeRule::Fixed(42.0),
                    Measurement::new(Size::new(140.0, 42.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(42.0),
                    Measurement::new(Size::new(180.0, 42.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fit,
                    SizeRule::Fixed(42.0),
                    Measurement::new(Size::new(96.0, 42.0)),
                ),
            ];
            for (index, rect_value) in row_layout(row_bounds, &items, 8.0).into_iter().enumerate() {
                rect(ui, rect_value, rgb(36, 42, 50), Some(rgb(90, 110, 140)));
                text(
                    ui,
                    rect_value.x + 12.0,
                    rect_value.y + 26.0,
                    &format!("Row {index}"),
                    11.0,
                    rgb(236, 236, 236),
                );
            }

            let column_items = [
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(34.0),
                    Measurement::new(Size::new(80.0, 34.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(54.0),
                    Measurement::new(Size::new(80.0, 54.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(34.0),
                    Measurement::new(Size::new(80.0, 34.0)),
                ),
            ];
            let column_bounds = Rect::new(body.x + 4.0, body.y + 70.0, 220.0, 122.0);
            for rect_value in column_layout(column_bounds, &column_items, 8.0) {
                rect(ui, rect_value, rgb(44, 38, 52), Some(rgb(120, 94, 150)));
            }

            let grid_x = (body.x + 280.0).min(body.max_x() - 220.0).max(body.x + 4.0);
            let adaptive = GridLayout {
                columns: GridColumns::Adaptive { min_width: 64.0 },
                item_size: Size::new(58.0, 32.0),
                gap: 8.0,
            };
            for item in
                adaptive.item_rects(Rect::new(grid_x, body.y + 70.0, 220.0, 120.0), 12, 0..12)
            {
                rect(ui, item.rect, rgb(38, 45, 44), Some(rgb(84, 122, 110)));
            }
        });
    }

    fn dock_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Interactive Dock Model");
        let dock_semantic_id = ui.id("layout.dock-preview.semantic");
        ui.push_semantic_node(
            SemanticNode::new(dock_semantic_id, SemanticRole::Dock, panel)
                .with_label("Interactive Dock Model"),
        );
        self.dock_preview_controls(ui, panel);
        let area = self.dock_model_preview();
        Self::draw_dock_preview(ui, &area, panel);
    }

    fn dock_model_preview(&self) -> Dock {
        let mut area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: self.dock_ratio,
            min_first: 140.0,
            min_second: 220.0,
            first: Box::new(DockNode::Frame(Frame::new(
                FrameId::from_raw(1),
                vec![
                    Panel::new(PanelId::from_raw(1), "Inspector"),
                    Panel::new(PanelId::from_raw(2), "Assets"),
                ],
            ))),
            second: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.62,
                min_first: 120.0,
                min_second: 80.0,
                first: Box::new(DockNode::Frame(Frame::new(
                    FrameId::from_raw(2),
                    vec![Panel::new(PanelId::from_raw(3), "Viewport")],
                ))),
                second: Box::new(DockNode::Frame(Frame::new(
                    FrameId::from_raw(3),
                    vec![
                        Panel::new(PanelId::from_raw(4), "Console"),
                        Panel::new(PanelId::from_raw(5), "Jobs"),
                    ],
                ))),
            }),
        });

        if self.dock_split_demo == DockSplitDemoState::Inserted {
            let drag = area
                .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
                .expect("demo panel exists");
            area.drop_tab(
                drag,
                DockDropTarget::split(
                    FrameId::from_raw(1),
                    DockPlacement::Bottom,
                    FrameId::from_raw(9),
                ),
            );
        }

        area
    }

    fn dock_preview_controls(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        let before = self.dock_ratio;
        ui.slider(
            "layout.dock-ratio",
            Rect::new(panel.x + 236.0, panel.y + 54.0, 170.0, 14.0),
            &mut self.dock_ratio,
            0.25..=0.75,
            false,
        );
        if (before - self.dock_ratio).abs() > f32::EPSILON {
            self.status = format!("Dock split: {:.0}%", self.dock_ratio * 100.0);
        }
        text(
            ui,
            panel.x + 420.0,
            panel.y + 64.0,
            &format!("{:.0}%", self.dock_ratio * 100.0),
            10.0,
            rgb(190, 190, 194),
        );

        let split = ui.button(
            "layout.split-demo",
            Rect::new(panel.x + 32.0, panel.y + 46.0, 132.0, 28.0),
            if self.dock_split_demo == DockSplitDemoState::Inserted {
                "Reset Dock"
            } else {
                "Split Tab"
            },
            false,
        );
        if split.clicked {
            self.dock_split_demo = match self.dock_split_demo {
                DockSplitDemoState::Base => DockSplitDemoState::Inserted,
                DockSplitDemoState::Inserted => DockSplitDemoState::Base,
            };
            self.status = match self.dock_split_demo {
                DockSplitDemoState::Base => "Dock split reset".to_owned(),
                DockSplitDemoState::Inserted => "Dock tab split inserted".to_owned(),
            };
        }
    }

    fn draw_dock_preview(ui: &mut Ui<'_>, area: &Dock, panel: Rect) {
        let dock_bounds = Rect::new(
            panel.x + 20.0,
            panel.y + 86.0,
            (panel.width - 60.0).max(0.0),
            (panel.height - 116.0).max(0.0),
        );
        for frame in solve_dock_layout(area, dock_bounds) {
            rect(ui, frame.rect, rgb(22, 22, 25), Some(rgb(70, 70, 76)));
            text(
                ui,
                frame.rect.x + 10.0,
                frame.rect.y + 24.0,
                &format!("Frame {}", frame.frame.raw()),
                10.0,
                rgb(180, 180, 184),
            );
        }
        for splitter in solve_dock_splitters(area, dock_bounds, 6.0) {
            rect(
                ui,
                splitter.rect,
                rgb(82, 94, 118),
                Some(rgb(116, 132, 160)),
            );
        }
        for frame in area.frames() {
            let tabs = frame_tabs(frame);
            let mut x = panel.x + 32.0;
            let y = panel.max_y() - 30.0 + frame.id.raw() as f32 * 0.0;
            for tab in tabs {
                let width = 74.0;
                rect(
                    ui,
                    Rect::new(x, y, width, 22.0),
                    if tab.active {
                        rgb(42, 96, 224)
                    } else {
                        rgb(30, 30, 33)
                    },
                    Some(rgb(72, 72, 76)),
                );
                text(ui, x + 8.0, y + 15.0, &tab.title, 9.0, rgb(236, 236, 238));
                x += width + 4.0;
            }
        }
        text(
            ui,
            panel.x + 32.0,
            panel.max_y() - 10.0,
            &format!("Frames: {} | Snapshot: valid", area.frames().len()),
            10.0,
            rgb(160, 160, 164),
        );
    }

    fn table_preview(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Virtualized Table Model");
        let table_semantic_id = ui.id("layout.table-preview.semantic");
        ui.push_semantic_node(
            SemanticNode::new(table_semantic_id, SemanticRole::Table, panel)
                .with_label("Virtualized Table Model"),
        );
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: ItemId::from_raw(1),
                    header: "Name".to_owned(),
                    width: 220.0,
                },
                TableColumn {
                    id: ItemId::from_raw(2),
                    header: "State".to_owned(),
                    width: 160.0,
                },
                TableColumn {
                    id: ItemId::from_raw(3),
                    header: "Latency".to_owned(),
                    width: 120.0,
                },
                TableColumn {
                    id: ItemId::from_raw(4),
                    header: "Owner".to_owned(),
                    width: 180.0,
                },
            ],
            header_height: 30.0,
            row_height: 28.0,
            sort: None,
        };
        let max_table_width = (panel.width - 48.0).max(0.0);
        let preferred_table_width = (panel.width * 0.62).clamp(0.0, 680.0);
        let table_width = if max_table_width < 420.0 {
            max_table_width
        } else {
            preferred_table_width.clamp(420.0, max_table_width)
        };
        let bounds = Rect::new(
            panel.x + 24.0,
            panel.y + 50.0,
            table_width,
            (panel.height - 90.0).max(120.0),
        );
        for header in table.header_rects(bounds) {
            rect(ui, header.rect, rgb(34, 34, 38), Some(rgb(72, 72, 76)));
            let column = &table.columns[header.index];
            text(
                ui,
                header.rect.x + 10.0,
                header.rect.y + 20.0,
                &column.header,
                10.0,
                rgb(236, 236, 238),
            );
        }
        for cell in table.cell_rects(bounds, 7, 0..7) {
            rect(ui, cell.rect, rgb(22, 22, 25), Some(rgb(52, 52, 58)));
            let row = cell.index / table.columns.len();
            let column = cell.index % table.columns.len();
            let value = match column {
                0 => format!("Item {row:02}"),
                1 => {
                    if row.is_multiple_of(2) {
                        "Ready".to_owned()
                    } else {
                        "Queued".to_owned()
                    }
                }
                2 => format!("{} ms", 12 + row * 7),
                _ => format!("Team {}", row % 3 + 1),
            };
            text(
                ui,
                cell.rect.x + 10.0,
                cell.rect.y + 18.0,
                &value,
                9.0,
                rgb(210, 210, 214),
            );
        }

        text(
            ui,
            bounds.max_x() + 56.0,
            bounds.y + 30.0,
            "Rows: 7 | Columns: 4 | Overscan: 0",
            11.0,
            rgb(190, 190, 194),
        );
    }

    fn viewport_page(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        section_title(ui, page.x, 86.0, "Viewport, Texture, and Overlay Surface");

        if page.width >= 1120.0 {
            let main = Rect::new(page.x, page.y, 960.0, 620.0);
            let side_x = main.max_x() + 40.0;
            self.viewport_controls_panel(ui, Rect::new(side_x, page.y, 300.0, 250.0));
            self.viewport_surface_panel(ui, main);
            Self::video_boundary_panel(ui, Rect::new(side_x, page.y + 286.0, 300.0, 230.0));
        } else {
            let width = page.width.min(980.0);
            let surface_height = ((width - 80.0).max(220.0) * 9.0 / 16.0).clamp(180.0, 300.0);
            let main_height = surface_height + 132.0;
            let main = Rect::new(page.x, page.y, width, main_height);
            self.viewport_controls_panel(ui, Rect::new(page.x, main.max_y() + 24.0, width, 150.0));
            self.viewport_surface_panel(ui, main);
            Self::video_boundary_panel(ui, Rect::new(page.x, main.max_y() + 198.0, width, 190.0));
        }
    }

    fn viewport_surface_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Pan/Zoom Texture Surface");
        let max_surface_width = (panel.width - 80.0).max(160.0);
        let max_surface_height = (panel.height - 132.0).max(120.0);
        let surface_height = (max_surface_width * 9.0 / 16.0).min(max_surface_height);
        let surface_width = (surface_height * 16.0 / 9.0).min(max_surface_width);
        let surface = Rect::new(
            panel.x + (panel.width - surface_width) * 0.5,
            panel.y + 86.0,
            surface_width,
            surface_height,
        );
        let viewport_semantic_id = ui.id("viewport.surface.semantic");
        ui.push_semantic_node(
            SemanticNode::new(viewport_semantic_id, SemanticRole::Viewport, surface)
                .with_label("Pan/Zoom Texture Surface")
                .focusable(true),
        );

        let mut pan_zoom = PanZoom::default();
        pan_zoom.set_zoom(0.25 + self.zoom * 3.75);
        let composition = ViewportComposition {
            surface: ViewportSurface {
                texture: TextureId::from_raw(99),
                source_size: Size::new(384.0, 216.0),
                bounds: surface,
                pan_zoom,
            },
            guides: vec![
                Guide::Horizontal(108.0),
                Guide::Vertical(192.0),
                Guide::Horizontal(72.0),
            ],
            crosshair: Some(Crosshair {
                visible: true,
                position: Point::new(192.0, 108.0),
                label: Some("192, 108".to_owned()),
                color: rgb(240, 240, 240),
            }),
            clip: ClipId::from_raw(99),
        };
        ui.extend(composition.primitives_at(ui.viewport().scale_factor));
        text(
            ui,
            surface.x,
            surface.max_y() + 36.0,
            "Surface: 384x216 | Guides: 3 | Crosshair: 192,108",
            11.0,
            rgb(190, 190, 194),
        );
    }

    fn viewport_controls_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Viewport Controls");
        let x = panel.x + 40.0;
        let y = panel.y + 84.0;
        let slider_width = (panel.width - 80.0).clamp(160.0, 220.0);
        let before = self.zoom;
        ui.slider(
            "viewport.zoom",
            Rect::new(x, y, slider_width, 16.0),
            &mut self.zoom,
            0.0..=1.0,
            false,
        );
        if (before - self.zoom).abs() > f32::EPSILON {
            self.status = format!("Viewport zoom {:.0}%", 25.0 + self.zoom * 375.0);
        }
        let fit = ui.button(
            "viewport.fit",
            Rect::new(x, y + 44.0, 90.0, 28.0),
            "Fit",
            false,
        );
        if fit.clicked {
            self.zoom = 0.0;
            "Viewport fit".clone_into(&mut self.status);
        }
        let actual = ui.button(
            "viewport.actual",
            Rect::new(x + 104.0, y + 44.0, 116.0, 28.0),
            "Actual Size",
            false,
        );
        if actual.clicked {
            self.zoom = 0.2;
            "Viewport actual size".clone_into(&mut self.status);
        }
        text(
            ui,
            x,
            y - 10.0,
            &format!("Zoom: {:.0}%", 25.0 + self.zoom * 375.0),
            11.0,
            rgb(220, 220, 224),
        );
    }

    fn video_boundary_panel(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "3D/Video Boundary");
        let texture_width = (panel.width - 80.0).clamp(180.0, 260.0);
        let texture_height = texture_width * 9.0 / 16.0;
        let texture = Rect::new(
            panel.x + 40.0,
            panel.y + 54.0,
            texture_width,
            texture_height,
        );
        ui.primitive(Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(101),
            rect: texture,
            source_size: Size::new(256.0, 144.0),
        }));
        text(
            ui,
            texture.x,
            texture.max_y() + 34.0,
            "Frame 256x144",
            11.0,
            rgb(220, 220, 224),
        );
    }

    fn systems_page(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        section_title(ui, page.x, 86.0, "Actions, Overlays, Diagnostics, Stress");

        let actions = showcase_actions();
        if page.width >= 1220.0 {
            self.systems_action_panel(ui, Rect::new(page.x, page.y, 360.0, 240.0), &actions);
            Self::systems_overlay_panel(ui, Rect::new(page.x + 400.0, page.y, 420.0, 240.0));
            self.systems_palette_panel(
                ui,
                Rect::new(page.x + 860.0, page.y, 360.0, 240.0),
                &actions,
            );
            self.systems_stress_panel(ui, Rect::new(page.x, page.y + 286.0, 1220.0, 330.0));
        } else if page.width >= 820.0 {
            let column = (page.width - 24.0) * 0.5;
            self.systems_action_panel(ui, Rect::new(page.x, page.y, column, 240.0), &actions);
            Self::systems_overlay_panel(
                ui,
                Rect::new(page.x + column + 24.0, page.y, column, 240.0),
            );
            self.systems_palette_panel(
                ui,
                Rect::new(page.x, page.y + 264.0, page.width, 210.0),
                &actions,
            );
            self.systems_stress_panel(ui, Rect::new(page.x, page.y + 498.0, page.width, 360.0));
        } else {
            self.systems_action_panel(ui, Rect::new(page.x, page.y, page.width, 220.0), &actions);
            Self::systems_overlay_panel(ui, Rect::new(page.x, page.y + 244.0, page.width, 240.0));
            self.systems_palette_panel(
                ui,
                Rect::new(page.x, page.y + 508.0, page.width, 210.0),
                &actions,
            );
            self.systems_stress_panel(ui, Rect::new(page.x, page.y + 742.0, page.width, 440.0));
        }
    }

    fn systems_action_panel(&mut self, ui: &mut Ui<'_>, panel: Rect, actions: &[ActionDescriptor]) {
        panel_title(ui, panel, "Action Router");
        let menu = Menu::from_actions(actions.to_vec());
        let mut queue = ActionQueue::new();
        let x = panel.x + 20.0;
        let y = panel.y + 46.0;
        let dispatch = ui.button(
            "systems.dispatch",
            Rect::new(x, y, 140.0, 30.0),
            "Dispatch",
            false,
        );
        if dispatch.clicked {
            self.invoke_action("systems.dispatch", ActionSource::Button);
        }
        let menu_item = ui.button(
            "systems.menu-save",
            Rect::new(x, y + 44.0, 140.0, 28.0),
            "Menu Save",
            false,
        );
        if menu_item.clicked && menu.invoke_visible(0, &mut queue, ActionContext::Global) {
            self.invoke_action("workspace.save", ActionSource::Menu);
        }
        text(
            ui,
            x + 160.0,
            y + 20.0,
            &format!("Invocations: {}", self.action_count),
            11.0,
            rgb(144, 184, 255),
        );
        for invocation in queue.drain() {
            text(
                ui,
                x,
                y + 112.0,
                invocation.action_id.as_str(),
                10.0,
                rgb(220, 220, 224),
            );
        }
    }

    fn systems_overlay_panel(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Overlay Stack");
        let menu_rect = Rect::new(
            panel.x + 30.0,
            panel.y + 62.0,
            (panel.width - 110.0).clamp(180.0, 260.0),
            54.0,
        );
        let popover_size = Size::new((panel.width - 100.0).clamp(190.0, 230.0), 58.0);
        let popover_rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(menu_rect.x + 30.0, menu_rect.y + 26.0, 120.0, 28.0),
                size: popover_size,
                placement: PopoverPlacement::Below,
                offset: 8.0,
                fit_viewport: true,
            },
            panel,
        );
        let palette_width = (panel.width - 80.0).clamp(220.0, 300.0);
        let palette_rect = Rect::new(
            (panel.x + panel.width * 0.28).min(panel.max_x() - palette_width - 30.0),
            panel.y + 132.0,
            palette_width,
            64.0,
        );
        let mut stack = OverlayStack::new();
        stack.open(OverlayEntry {
            id: OverlayId::from_raw(1),
            parent: None,
            kind: OverlayKind::Menu,
            rect: menu_rect,
            modal: false,
            dismissal: OverlayDismissal::OutsideClick,
        });
        stack.open(OverlayEntry {
            id: OverlayId::from_raw(2),
            parent: Some(OverlayId::from_raw(1)),
            kind: OverlayKind::Popover,
            rect: popover_rect,
            modal: false,
            dismissal: OverlayDismissal::OutsideClick,
        });
        stack.open(OverlayEntry {
            id: OverlayId::from_raw(3),
            parent: None,
            kind: OverlayKind::CommandPalette,
            rect: palette_rect,
            modal: true,
            dismissal: OverlayDismissal::Manual,
        });
        for (index, entry) in stack.entries().iter().enumerate() {
            let label = match entry.kind {
                OverlayKind::Popover => "Popover",
                OverlayKind::Dropdown => "Dropdown",
                OverlayKind::ContextMenu => "Context Menu",
                OverlayKind::Menu => "Menu",
                OverlayKind::CommandPalette => "Command Palette",
                OverlayKind::Tooltip => "Tooltip",
                OverlayKind::Modal => "Modal",
                OverlayKind::DragPreview => "Drag Preview",
            };
            ui.push_semantic_node(overlay_semantics(entry, label));
            rect(
                ui,
                entry.rect,
                rgb(30 + index as u8 * 10, 32, 38),
                Some(rgb(90, 90, 98)),
            );
            text(
                ui,
                entry.rect.x + 14.0,
                entry.rect.y + 32.0,
                label,
                11.0,
                rgb(236, 236, 238),
            );
        }
    }

    fn systems_palette_panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        actions: &[ActionDescriptor],
    ) {
        panel_title(ui, panel, "Command Palette");
        let mut palette = CommandPalette::from_actions(actions);
        palette.query = String::new();
        let x = panel.x + 20.0;
        let row_width = (panel.width - 40.0).max(160.0);
        for (index, entry) in palette.matches().into_iter().take(4).enumerate() {
            let y = panel.y + 50.0 + index as f32 * 32.0;
            let response = ui.list_row(
                ("systems.palette", index),
                Rect::new(x, y, row_width, 28.0),
                &entry.label,
                false,
                false,
            );
            if response.clicked {
                self.invoke_action(entry.action_id.as_str(), ActionSource::CommandPalette);
            }
        }
    }

    fn systems_stress_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Primitive Stress");
        let origin = Point::new(panel.x + 36.0, panel.y + 70.0);
        self.systems_stress_slider(ui, panel, origin);

        let wide = panel.width >= 820.0;
        let snapshot = Self::stress_snapshot_rect(panel, origin, wide);
        let tile_area = Self::stress_tile_area(panel, origin, snapshot, wide);
        Self::draw_stress_tiles(ui, tile_area, self.stress);
        self.draw_runtime_snapshot(ui, snapshot, wide);
    }

    fn systems_stress_slider(&mut self, ui: &mut Ui<'_>, panel: Rect, origin: Point) {
        let before = self.stress;
        let mut stress_value = (self.stress as f32 - 32.0) / 768.0;
        ui.slider(
            "systems.stress",
            Rect::new(
                origin.x,
                origin.y,
                (panel.width - 72.0).clamp(180.0, 260.0),
                16.0,
            ),
            &mut stress_value,
            0.0..=1.0,
            false,
        );
        self.stress = (32.0 + stress_value * 768.0).round() as usize;
        if before != self.stress {
            self.status = format!("Generated tiles: {}", self.stress);
        }
        text(
            ui,
            origin.x,
            origin.y - 12.0,
            &format!("Generated tiles: {}", self.stress),
            11.0,
            rgb(220, 220, 224),
        );
    }

    fn stress_snapshot_rect(panel: Rect, origin: Point, wide: bool) -> Rect {
        let snapshot_width = (panel.width - 72.0).clamp(220.0, 320.0);
        if wide {
            Rect::new(
                panel.max_x() - snapshot_width - 40.0,
                origin.y,
                snapshot_width,
                188.0,
            )
        } else {
            Rect::new(origin.x, panel.max_y() - 154.0, snapshot_width, 132.0)
        }
    }

    fn stress_tile_area(panel: Rect, origin: Point, snapshot: Rect, wide: bool) -> Rect {
        if wide {
            Rect::new(
                origin.x,
                origin.y + 56.0,
                (snapshot.x - origin.x - 40.0).max(120.0),
                panel.height - 102.0,
            )
        } else {
            Rect::new(
                origin.x,
                origin.y + 56.0,
                (panel.width - 72.0).max(120.0),
                (snapshot.y - origin.y - 76.0).max(40.0),
            )
        }
    }

    fn draw_stress_tiles(ui: &mut Ui<'_>, tile_area: Rect, stress: usize) {
        let cols = (tile_area.width / 21.0).floor().max(1.0) as usize;
        ui.clip_rect("systems.stress.tiles", tile_area, |ui| {
            for index in 0..stress {
                let col = index % cols;
                let row = index / cols;
                let tile_x = tile_area.x + col as f32 * 21.0;
                let tile_y = tile_area.y + row as f32 * 16.0;
                let shade = 30 + (index % 80) as u8;
                rect(
                    ui,
                    Rect::new(tile_x, tile_y, 16.0, 10.0),
                    rgb(shade, 48, 70),
                    Some(rgb(60, 70, 90)),
                );
            }
        });
    }

    fn draw_runtime_snapshot(&self, ui: &mut Ui<'_>, snapshot: Rect, wide: bool) {
        rect(ui, snapshot, rgb(18, 18, 20), Some(rgb(58, 58, 62)));
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 28.0,
            "Runtime Snapshot",
            13.0,
            rgb(238, 238, 240),
        );
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 58.0,
            &format!("Primitive count: {}", self.output.primitives.len()),
            10.0,
            rgb(190, 190, 194),
        );
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 78.0,
            &format!("Stress tiles: {}", self.stress),
            10.0,
            rgb(190, 190, 194),
        );
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 98.0,
            &format!("Action invocations: {}", self.action_count),
            10.0,
            rgb(190, 190, 194),
        );
        for (row, primitive) in inspect_primitives(&self.output.primitives)
            .into_iter()
            .take(if wide { 4 } else { 1 })
            .enumerate()
        {
            text(
                ui,
                snapshot.x + 20.0,
                snapshot.y + 132.0 + row as f32 * 18.0,
                &format!("#{} {:?}", primitive.index, primitive.kind),
                10.0,
                rgb(144, 184, 255),
            );
        }
    }
}

fn showcase_actions() -> Vec<ActionDescriptor> {
    let mut save = ActionDescriptor::new("workspace.save", "Save Workspace");
    save.keywords = vec!["write".to_owned(), "persist".to_owned()];
    let mut palette = ActionDescriptor::new("command.palette", "Open Command Palette");
    palette.keywords = vec!["search".to_owned(), "actions".to_owned()];
    let mut toggle_grid = ActionDescriptor::new("viewport.grid", "Toggle Viewport Grid");
    toggle_grid.keywords = vec!["guides".to_owned(), "overlay".to_owned()];
    vec![save, palette, toggle_grid]
}

fn showcase_action_router() -> ActionRouter {
    let mut enter = ActionDescriptor::new("keyboard.enter", "Confirm Focused Command");
    enter.shortcut = Some(Shortcut::new(Modifiers::default(), Key::Enter));
    let mut save = ActionDescriptor::new(editor_showcase::ACTION_SAVE, "Save Project");
    save.shortcut = Some(Shortcut::new(
        Modifiers::new(false, true, false, false),
        Key::Character("s".to_owned()),
    ));
    let mut play = ActionDescriptor::new(editor_showcase::ACTION_PLAY, "Play");
    play.shortcut = Some(Shortcut::new(Modifiers::default(), Key::Function(5)));
    let mut grid = ActionDescriptor::new(editor_showcase::ACTION_GRID, "Toggle Grid");
    grid.shortcut = Some(Shortcut::new(
        Modifiers::default(),
        Key::Character("g".to_owned()),
    ));
    let mut build = ActionDescriptor::new(editor_showcase::ACTION_BUILD, "Build");
    build.shortcut = Some(Shortcut::new(
        Modifiers::new(false, true, false, false),
        Key::Character("b".to_owned()),
    ));
    let mut palette = ActionDescriptor::new(editor_showcase::ACTION_PALETTE, "Command Palette");
    palette.shortcut = Some(Shortcut::new(
        Modifiers::new(false, true, false, false),
        Key::Character("p".to_owned()),
    ));

    let mut router = ActionRouter::new();
    for action in [enter, save, play, grid, build, palette] {
        router.bind(ActionBinding::new(
            action,
            ActionContext::Global,
            ActionPriority::Global,
        ));
    }
    router
}

fn nav_items(viewport_width: f32) -> [(ShowcasePage, Rect); 4] {
    let (start, widths) = if viewport_width >= 940.0 {
        (360.0, [132.0, 92.0, 112.0, 112.0])
    } else {
        (180.0, [112.0, 82.0, 98.0, 90.0])
    };
    let gap = 10.0;
    let components = Rect::new(start, 12.0, widths[0], 28.0);
    let layout = Rect::new(components.max_x() + gap, 12.0, widths[1], 28.0);
    let viewport = Rect::new(layout.max_x() + gap, 12.0, widths[2], 28.0);
    let systems = Rect::new(viewport.max_x() + gap, 12.0, widths[3], 28.0);
    [
        (ShowcasePage::Components, components),
        (ShowcasePage::Layout, layout),
        (ShowcasePage::Viewport, viewport),
        (ShowcasePage::Systems, systems),
    ]
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

#[cfg(test)]
mod tests {
    use super::{ShowcaseApp, ShowcaseInput, ShowcasePage, frame_context, static_render_resources};
    use crate::editor::phosphor_icons;
    use kinetik_ui::{
        core::{
            ImageId, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize,
            PlatformRequest, Point, Primitive, Rect, RepaintRequest, ScaleFactor,
            SemanticActionKind, SemanticRole, SemanticValue, Size, TextureId, UiInput,
            ViewportInfo,
        },
        render::{RenderFrameInput, RenderImageSampling},
        render_vello::VelloRenderer,
    };

    fn click(app: &mut ShowcaseApp, point: Point) {
        app.update(&ShowcaseInput {
            mouse: Some(point),
            mouse_down: true,
            ..ShowcaseInput::default()
        });
        app.update(&ShowcaseInput {
            mouse: Some(point),
            mouse_down: false,
            ..ShowcaseInput::default()
        });
    }

    fn has_text(app: &ShowcaseApp, value: &str) -> bool {
        app.primitives()
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == value))
    }

    fn count_primitives(app: &ShowcaseApp, predicate: impl Fn(&Primitive) -> bool) -> usize {
        app.output()
            .primitives
            .iter()
            .filter(|primitive| predicate(primitive))
            .count()
    }

    fn count_semantic_role(app: &ShowcaseApp, role: &SemanticRole) -> usize {
        app.output()
            .semantics
            .nodes()
            .iter()
            .filter(|node| &node.role == role)
            .count()
    }

    fn semantic_node(app: &ShowcaseApp, role: &SemanticRole, label: &str) -> bool {
        app.output()
            .semantics
            .nodes()
            .iter()
            .any(|node| &node.role == role && node.label.as_deref() == Some(label))
    }

    fn semantic_role_has_action(
        app: &ShowcaseApp,
        role: &SemanticRole,
        action: &SemanticActionKind,
    ) -> bool {
        app.output()
            .semantics
            .nodes()
            .iter()
            .any(|node| &node.role == role && node.actions.iter().any(|item| &item.kind == action))
    }

    fn text_labels(app: &ShowcaseApp) -> Vec<&str> {
        app.output()
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn contains_text_in_order(app: &ShowcaseApp, expected: &[&str]) -> bool {
        let mut cursor = 0;
        for label in text_labels(app) {
            if expected
                .get(cursor)
                .is_some_and(|expected| *expected == label)
            {
                cursor += 1;
                if cursor == expected.len() {
                    return true;
                }
            }
        }
        false
    }

    fn viewport_texture_rect(app: &ShowcaseApp) -> Rect {
        app.primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Texture(texture) if texture.texture == TextureId::from_raw(99) => {
                    Some(texture.rect)
                }
                _ => None,
            })
            .expect("viewport texture")
    }

    #[test]
    fn clicking_button_changes_action_state() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        click(&mut app, Point::new(70.0, 154.0));

        assert_eq!(app.action_count(), 1);
    }

    #[test]
    fn components_page_structural_smoke_emits_controls_semantics_and_platform_requests() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        assert_eq!(app.output().warnings, Vec::new());
        assert!(app.output().primitives.len() > 120);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 40);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 25);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Image(_))) >= 2);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Line(_))) >= 1);
        assert!(contains_text_in_order(
            &app,
            &[
                "Component Gallery",
                "Controls",
                "Text Input",
                "Lists, Grids, Tables",
                "Reusable Panel States",
                "Primitive Stream",
            ]
        ));

        assert_eq!(count_semantic_role(&app, &SemanticRole::Button), 2);
        assert_eq!(count_semantic_role(&app, &SemanticRole::IconButton), 1);
        assert_eq!(count_semantic_role(&app, &SemanticRole::CheckBox), 1);
        assert_eq!(count_semantic_role(&app, &SemanticRole::Toggle), 1);
        assert_eq!(count_semantic_role(&app, &SemanticRole::RadioButton), 2);
        assert_eq!(count_semantic_role(&app, &SemanticRole::Slider), 1);
        assert_eq!(count_semantic_role(&app, &SemanticRole::SearchField), 1);
        assert!(count_semantic_role(&app, &SemanticRole::TextField) >= 3);
        assert!(count_semantic_role(&app, &SemanticRole::ListItem) >= 4);
        assert!(count_semantic_role(&app, &SemanticRole::Tab) >= 3);
        assert!(semantic_node(&app, &SemanticRole::Button, "Run Action"));
        assert!(semantic_node(&app, &SemanticRole::Button, "Disabled"));

        let slider = app
            .output()
            .semantics
            .nodes()
            .iter()
            .find(|node| node.role == SemanticRole::Slider)
            .expect("slider semantics");
        assert!(
            slider
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::SetValue)
        );
        assert!(matches!(
            slider.state.value,
            Some(SemanticValue::Number { current, min: 0.0, max: 1.0 })
                if (current - app.strength()).abs() < f32::EPSILON
        ));

        click(&mut app, Point::new(940.0, 160.0));

        assert!(app.output().platform_requests.iter().any(|request| {
            matches!(request, PlatformRequest::StartTextInput { rect: Some(rect) } if !rect.is_empty())
        }));
    }

    #[test]
    fn layout_page_structural_smoke_emits_layout_dock_table_and_actions() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Layout);

        assert_eq!(app.output().warnings, Vec::new());
        assert!(app.output().primitives.len() > 100);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 60);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 30);
        assert!(
            count_primitives(&app, |primitive| matches!(
                primitive,
                Primitive::ClipBegin { .. }
            )) >= 2
        );
        assert!(contains_text_in_order(
            &app,
            &[
                "Layout, Docking, and Data Surfaces",
                "Measurement-Aware Layout",
                "Interactive Dock Model",
                "Virtualized Table Model",
            ]
        ));
        assert!(has_text(&app, "Rows: 7 | Columns: 4 | Overscan: 0"));
        assert!(app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text.starts_with("Frames: "))
        }));

        assert!(semantic_node(
            &app,
            &SemanticRole::Dock,
            "Interactive Dock Model"
        ));
        assert!(semantic_node(
            &app,
            &SemanticRole::Table,
            "Virtualized Table Model"
        ));
        assert!(semantic_node(&app, &SemanticRole::Button, "Split Tab"));
        assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 4);
        assert!(semantic_role_has_action(
            &app,
            &SemanticRole::Slider,
            &SemanticActionKind::SetValue
        ));
        assert!(semantic_role_has_action(
            &app,
            &SemanticRole::Button,
            &SemanticActionKind::Invoke
        ));

        click(&mut app, Point::new(700.0, 162.0));

        assert!(has_text(&app, "Frame 9"));
    }

    #[test]
    fn component_status_reflects_toggle_click_same_frame() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        click(&mut app, Point::new(231.0, 204.0));

        assert!(app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Toggle: true")
        }));
        assert!(app.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Text(text)
                    if text.text == "checkbox=true toggle=true radio=1 selected_row=2"
            )
        }));
    }

    #[test]
    fn component_status_reflects_radio_click_same_frame() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        click(&mut app, Point::new(170.0, 252.0));

        assert_eq!(app.radio, 1);
        assert!(app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Radio: Radio B")
        }));
    }

    #[test]
    fn default_page_is_engine_editor_surface() {
        let app = ShowcaseApp::new();

        assert_eq!(app.page(), ShowcasePage::Editor);
        for label in ["Kinetik Forge", "Scene", "Viewport", "Inspector", "Console"] {
            assert!(
                app.primitives().iter().any(|primitive| {
                    matches!(primitive, Primitive::Text(text) if text.text == label)
                }),
                "{label}"
            );
        }
    }

    #[test]
    fn editor_file_menu_opens_dropdown_and_invokes_action() {
        let mut app = ShowcaseApp::new();

        click(&mut app, Point::new(145.0, 14.0));

        for label in ["New Scene", "Save Scene", "Export Build"] {
            assert!(
                app.primitives().iter().any(|primitive| {
                    matches!(primitive, Primitive::Text(text) if text.text == label)
                }),
                "{label}"
            );
        }

        click(&mut app, Point::new(170.0, 93.0));

        assert_eq!(app.action_count(), 1);
        assert_eq!(app.output().repaint, RepaintRequest::NextFrame);
        for label in ["Saved project snapshot", "Actions: 1"] {
            assert!(
                app.primitives().iter().any(|primitive| {
                    matches!(primitive, Primitive::Text(text) if text.text == label)
                }),
                "{label}"
            );
        }
        assert!(!app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Save Scene")
        }));
    }

    #[test]
    fn editor_shortcut_updates_visible_action_count_same_frame() {
        let mut app = ShowcaseApp::new();
        app.update_with_context(frame_context(
            Size::new(1440.0, 900.0),
            UiInput {
                keyboard: KeyboardInput {
                    modifiers: Modifiers::new(false, true, false, false),
                    events: vec![KeyEvent::new(
                        Key::Character("s".to_owned()),
                        KeyState::Pressed,
                        Modifiers::new(false, true, false, false),
                        false,
                    )],
                },
                ..UiInput::default()
            },
        ));

        assert_eq!(app.action_count(), 1);
        assert!(app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Actions: 1")
        }));
    }

    #[test]
    fn editor_grid_toolbar_updates_status_same_frame_and_requests_repaint() {
        let mut app = ShowcaseApp::new();

        click(&mut app, Point::new(161.0, 45.0));

        assert_eq!(app.action_count(), 1);
        assert_eq!(app.output().repaint, RepaintRequest::NextFrame);
        assert!(has_text(&app, "Viewport grid hidden"));
    }

    #[test]
    fn editor_play_toolbar_updates_hint_same_frame() {
        let mut app = ShowcaseApp::new();

        click(&mut app, Point::new(1307.0, 45.0));

        assert_eq!(app.action_count(), 1);
        assert!(has_text(&app, "Play Mode: Running"));
        assert!(has_text(&app, "Play mode running"));
    }

    #[test]
    fn editor_scene_add_requests_follow_up_repaint() {
        let mut app = ShowcaseApp::new();
        let add_node = app
            .output()
            .semantics
            .nodes()
            .iter()
            .find(|node| node.label.as_deref() == Some("Add node"))
            .expect("add node semantics")
            .bounds;

        click(
            &mut app,
            Point::new(
                add_node.x + add_node.width * 0.5,
                add_node.y + add_node.height * 0.5,
            ),
        );

        assert_eq!(app.output().repaint, RepaintRequest::NextFrame);
        assert!(app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Create node requested")
        }));
    }

    #[test]
    fn editor_resources_match_emitted_media_and_phosphor_atlas_icons() {
        let app = ShowcaseApp::new();
        let resources = app.render_resources();

        let primitives = app.primitives();
        let texture = app
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Texture(texture) => Some(texture.texture),
                _ => None,
            })
            .expect("editor emits viewport texture");

        assert!(resources.texture(texture).is_some());
        assert_eq!(
            resources.texture(texture).map(|resource| resource.sampling),
            Some(RenderImageSampling::Pixelated)
        );
        assert!(
            resources
                .texture(texture)
                .and_then(|resource| resource.snapshot.as_ref())
                .is_some_and(|snapshot| snapshot.width == 1280 && snapshot.height == 720)
        );
        let icon_images = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Image(image)
                    if phosphor_icons::ICON_ENTRIES
                        .iter()
                        .any(|entry| entry.image == image.image) =>
                {
                    Some(image)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(icon_images.len() >= 24);
        assert!(icon_images.iter().all(|image| image.tint.is_some()));
        assert!(
            icon_images
                .iter()
                .all(|image| resources.image(image.image).is_some())
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Line(_) | Primitive::Path(_)))
        );
        assert!(!resources.snapshot().images.is_empty());
    }

    #[test]
    fn editor_icons_are_registered_as_atlas_regions() {
        let app = ShowcaseApp::new();
        let resources = app.render_resources();

        for atlas in phosphor_icons::ICON_ATLASES {
            assert!(
                resources
                    .image(atlas.image)
                    .and_then(|resource| resource.pixels.as_ref())
                    .is_some_and(
                        |pixels| pixels.width == atlas.width && pixels.height == atlas.height
                    ),
                "missing atlas {}",
                atlas.physical_size
            );
        }
        let icon_regions = phosphor_icons::ICON_ENTRIES
            .iter()
            .filter_map(|entry| {
                resources
                    .image(entry.image)
                    .map(|resource| (entry, resource))
            })
            .filter(|(entry, resource)| {
                resource.pixels.is_none()
                    && resource
                        .atlas_region
                        .is_some_and(|region| region.atlas == entry.atlas)
            })
            .count();

        assert_eq!(icon_regions, phosphor_icons::ICON_ENTRIES.len());
    }

    #[test]
    fn generated_showcase_media_uses_intentional_sampling() {
        let resources = static_render_resources();

        for image in [ImageId::from_raw(7), ImageId::from_raw(11)] {
            assert_eq!(
                resources.image(image).map(|resource| resource.sampling),
                Some(RenderImageSampling::Pixelated),
                "{image:?}"
            );
        }

        for texture in [TextureId::from_raw(9_001), TextureId::from_raw(99)] {
            assert_eq!(
                resources.texture(texture).map(|resource| resource.sampling),
                Some(RenderImageSampling::Pixelated),
                "{texture:?}"
            );
        }

        assert_eq!(
            resources
                .texture(TextureId::from_raw(101))
                .map(|resource| resource.sampling),
            Some(RenderImageSampling::Smooth)
        );
    }

    #[test]
    fn component_thumbnail_uses_native_pixel_rect() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        let thumbnail = app
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Image(image) if image.image == ImageId::from_raw(7) => Some(image.rect),
                _ => None,
            })
            .expect("thumbnail image");
        let label = app
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Text(text) if text.text == "Thumbnail" => Some(text.origin),
                _ => None,
            })
            .expect("thumbnail label");

        assert!((thumbnail.width - 64.0).abs() < f32::EPSILON);
        assert!((thumbnail.height - 48.0).abs() < f32::EPSILON);
        assert!(label.y > thumbnail.max_y());
    }

    #[test]
    fn render_resources_reuse_static_media_and_append_text_layouts() {
        let app = ShowcaseApp::new();
        let static_snapshot = app.static_resources.snapshot();
        let fresh_static_snapshot = static_render_resources().snapshot();

        assert_eq!(static_snapshot, fresh_static_snapshot);
        assert!(!static_snapshot.images.is_empty());
        assert!(!static_snapshot.textures.is_empty());
        assert!(static_snapshot.text_layouts.is_empty());

        let frame_snapshot = app.render_resources().snapshot();
        assert_eq!(frame_snapshot.images, static_snapshot.images);
        assert_eq!(frame_snapshot.textures, static_snapshot.textures);
        assert!(!frame_snapshot.text_layouts.is_empty());
    }

    #[test]
    fn render_resources_share_cached_static_texture_payloads() {
        let app = ShowcaseApp::new();
        let resources = app.render_resources();
        let static_texture = app
            .static_resources
            .texture(TextureId::from_raw(9_001))
            .and_then(|resource| resource.snapshot.as_ref())
            .expect("static editor texture");
        let frame_texture = resources
            .texture(TextureId::from_raw(9_001))
            .and_then(|resource| resource.snapshot.as_ref())
            .expect("frame editor texture");

        assert!(std::sync::Arc::ptr_eq(
            &static_texture.data,
            &frame_texture.data
        ));
    }

    #[test]
    fn clicking_navigation_changes_page() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        click(&mut app, Point::new(620.0, 20.0));

        assert_eq!(app.page(), ShowcasePage::Viewport);
        assert!(has_text(&app, "Viewport, Texture, and Overlay Surface"));
        assert!(has_text(&app, "Page: Viewport"));
        assert!(!has_text(&app, "Component Gallery"));
    }

    #[test]
    fn viewport_size_sets_logical_frame_context() {
        let mut app = ShowcaseApp::new();

        app.set_viewport_size(Size::new(720.0, 450.0));

        assert_eq!(app.viewport_size(), Size::new(720.0, 450.0));
        assert_eq!(app.output().warnings, Vec::new());
        assert!(app.primitives().iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect) if rect.rect == Rect::new(0.0, 0.0, 720.0, 450.0)
        )));
    }

    #[test]
    fn resized_hit_testing_uses_logical_coordinates() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);
        app.set_viewport_size(Size::new(720.0, 450.0));

        click(&mut app, Point::new(35.0, 77.0));
        assert_eq!(app.action_count(), 0);

        click(&mut app, Point::new(70.0, 154.0));

        assert_eq!(app.action_count(), 1);
    }

    #[test]
    fn page_names_are_parseable_for_render_tools() {
        assert_eq!(
            ShowcaseApp::page_from_name("layout"),
            Some(ShowcasePage::Layout)
        );
        assert_eq!(ShowcaseApp::page_from_name("unknown"), None);
    }

    #[test]
    fn slider_drag_updates_value() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        app.update(&ShowcaseInput {
            mouse: Some(Point::new(360.0, 160.0)),
            mouse_down: true,
            ..ShowcaseInput::default()
        });
        app.update(&ShowcaseInput {
            mouse: Some(Point::new(600.0, 160.0)),
            mouse_down: true,
            ..ShowcaseInput::default()
        });

        assert!(app.strength() > 0.95);
    }

    #[test]
    fn focused_search_accepts_keyboard_input() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        click(&mut app, Point::new(940.0, 160.0));
        app.update(&ShowcaseInput {
            typed: vec!['x'],
            ..ShowcaseInput::default()
        });

        assert!(app.search().ends_with('x'));
    }

    #[test]
    fn focused_multi_line_field_accepts_text_and_enter() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Components);

        click(&mut app, Point::new(1070.0, 306.0));
        app.update(&ShowcaseInput {
            typed: vec!['x'],
            ..ShowcaseInput::default()
        });
        let actions_before_enter = app.action_count();
        app.update(&ShowcaseInput {
            enter: true,
            ..ShowcaseInput::default()
        });

        assert!(app.notes().contains('x'));
        assert!(app.notes().ends_with('\n'));
        assert_eq!(app.action_count(), actions_before_enter);
    }

    #[test]
    fn viewport_buttons_change_zoom_state() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Viewport);

        click(&mut app, Point::new(1090.0, 240.0));
        assert!(app.zoom().abs() < f32::EPSILON);
        assert!(has_text(&app, "Zoom: 25%"));
        assert!((viewport_texture_rect(&app).width - 96.0).abs() < f32::EPSILON);

        click(&mut app, Point::new(1200.0, 240.0));
        assert!((app.zoom() - 0.2).abs() < f32::EPSILON);
        assert!(has_text(&app, "Zoom: 100%"));
        assert!((viewport_texture_rect(&app).width - 384.0).abs() < f32::EPSILON);
    }

    #[test]
    fn viewport_page_structural_smoke_emits_texture_viewport_semantics_and_platform_requests() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Viewport);

        assert_eq!(app.output().warnings, Vec::new());
        assert!(app.output().primitives.len() > 60);
        assert_eq!(
            count_primitives(&app, |primitive| matches!(primitive, Primitive::Texture(_))),
            2
        );
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Line(_))) >= 5);
        assert!(
            count_primitives(&app, |primitive| matches!(
                primitive,
                Primitive::ClipBegin { .. }
            )) >= 2
        );
        assert!(contains_text_in_order(
            &app,
            &[
                "Viewport, Texture, and Overlay Surface",
                "Viewport Controls",
                "Pan/Zoom Texture Surface",
                "3D/Video Boundary",
            ]
        ));
        assert!(has_text(
            &app,
            "Surface: 384x216 | Guides: 3 | Crosshair: 192,108"
        ));

        assert!(semantic_node(
            &app,
            &SemanticRole::Viewport,
            "Pan/Zoom Texture Surface"
        ));
        assert!(semantic_node(&app, &SemanticRole::Button, "Fit"));
        assert!(semantic_node(&app, &SemanticRole::Button, "Actual Size"));
        assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 3);
        assert!(semantic_role_has_action(
            &app,
            &SemanticRole::Viewport,
            &SemanticActionKind::Focus
        ));
        assert!(semantic_role_has_action(
            &app,
            &SemanticRole::Slider,
            &SemanticActionKind::SetValue
        ));
        assert!(semantic_role_has_action(
            &app,
            &SemanticRole::Button,
            &SemanticActionKind::Invoke
        ));

        let resources = app.render_resources();
        assert!(resources.texture(TextureId::from_raw(99)).is_some());
        assert!(resources.texture(TextureId::from_raw(101)).is_some());

        app.update(&ShowcaseInput {
            mouse: Some(Point::new(1090.0, 240.0)),
            ..ShowcaseInput::default()
        });

        assert!(
            app.output()
                .platform_requests
                .contains(&PlatformRequest::SetCursor(
                    kinetik_ui::core::CursorShape::PointingHand
                ))
        );
    }

    #[test]
    fn layout_page_split_demo_changes_dock_preview() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Layout);

        click(&mut app, Point::new(700.0, 162.0));

        assert!(app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Frame 9")
        }));
    }

    #[test]
    fn systems_palette_invokes_actions() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Systems);

        click(&mut app, Point::new(930.0, 160.0));

        assert_eq!(app.action_count(), 1);
    }

    #[test]
    fn systems_page_exposes_runtime_diagnostics() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Systems);

        let has_snapshot = app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Runtime Snapshot")
        });

        assert!(has_snapshot);
    }

    #[test]
    fn systems_page_structural_smoke_emits_actions_overlays_palette_and_stress() {
        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Systems);

        assert_eq!(app.output().warnings, Vec::new());
        assert!(app.output().primitives.len() > 180);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 130);
        assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 20);
        assert!(
            count_primitives(&app, |primitive| matches!(
                primitive,
                Primitive::ClipBegin { .. }
            )) >= 1
        );
        assert!(contains_text_in_order(
            &app,
            &[
                "Actions, Overlays, Diagnostics, Stress",
                "Action Router",
                "Overlay Stack",
                "Command Palette",
                "Primitive Stress",
                "Runtime Snapshot",
            ]
        ));

        assert!(semantic_node(&app, &SemanticRole::Button, "Dispatch"));
        assert!(semantic_node(&app, &SemanticRole::Button, "Menu Save"));
        assert!(semantic_node(&app, &SemanticRole::Menu, "Menu"));
        assert!(semantic_node(
            &app,
            &SemanticRole::CommandPalette,
            "Command Palette"
        ));
        assert!(semantic_node(
            &app,
            &SemanticRole::Custom("popover".to_owned()),
            "Popover"
        ));
        assert!(count_semantic_role(&app, &SemanticRole::ListItem) >= 3);
        assert!(app.output().semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Menu
                && node
                    .actions
                    .iter()
                    .any(|action| action.kind == SemanticActionKind::Dismiss)
        }));

        click(&mut app, Point::new(100.0, 210.0));

        assert_eq!(app.action_count(), 1);
        assert!(has_text(&app, "workspace.save via Menu (1)"));

        let mut app = ShowcaseApp::new();
        app.set_page(ShowcasePage::Systems);

        click(&mut app, Point::new(930.0, 160.0));

        assert_eq!(app.action_count(), 1);
        assert!(has_text(&app, "workspace.save via CommandPalette (1)"));
    }

    #[test]
    fn state_changes_produce_different_frames() {
        let mut app = ShowcaseApp::new();
        let before = crate::raster::rasterize(&app.primitives(), 1440, 900);

        click(&mut app, Point::new(70.0, 154.0));
        let after = crate::raster::rasterize(&app.primitives(), 1440, 900);

        assert_ne!(before.pixels, after.pixels);
    }

    #[test]
    fn showcase_uses_widget_generated_primitives() {
        let app = ShowcaseApp::new();
        let primitives = app.primitives();

        assert!(
            primitives
                .iter()
                .any(|item| matches!(item, Primitive::Texture(_)))
        );
        assert!(
            primitives
                .iter()
                .any(|item| matches!(item, Primitive::Line(_) | Primitive::Path(_)))
        );
        assert!(
            primitives
                .iter()
                .filter(|item| matches!(item, Primitive::Rect(_)))
                .count()
                > 20
        );
    }

    #[test]
    fn showcase_app_does_not_define_fake_control_helpers() {
        let source = include_str!("app.rs");

        for marker in [
            ["fn ", "button", "("].concat(),
            ["fn ", "slider", "("].concat(),
            ["fn ", "input_box", "("].concat(),
        ] {
            assert!(!source.contains(&marker), "{marker}");
        }
    }

    #[test]
    fn showcase_text_primitives_have_registered_layouts() {
        for page in [
            ShowcasePage::Editor,
            ShowcasePage::Components,
            ShowcasePage::Layout,
            ShowcasePage::Viewport,
            ShowcasePage::Systems,
        ] {
            let mut app = ShowcaseApp::new();
            app.set_page(page);
            let resources = app.render_resources();
            let mut text_count = 0;

            for primitive in app.primitives() {
                let Primitive::Text(text) = primitive else {
                    continue;
                };
                text_count += 1;
                let layout = text
                    .layout
                    .unwrap_or_else(|| panic!("{page:?} text {:?} missing layout", text.text));
                assert!(
                    resources.has_text_layout(layout),
                    "{page:?} text {:?} references missing layout {layout:?}",
                    text.text
                );
            }

            assert!(text_count > 0, "{page:?} emitted no text primitives");
        }
    }

    #[test]
    fn showcase_pages_translate_to_vello_without_renderer_diagnostics() {
        for size in [Size::new(1440.0, 900.0), Size::new(820.0, 640.0)] {
            for page in [
                ShowcasePage::Editor,
                ShowcasePage::Components,
                ShowcasePage::Layout,
                ShowcasePage::Viewport,
                ShowcasePage::Systems,
            ] {
                let mut app = ShowcaseApp::new();
                app.set_viewport_size(size);
                app.set_page(page);
                let resources = app.render_resources();
                let mut renderer = VelloRenderer::new();
                let output = renderer.submit_frame(RenderFrameInput {
                    viewport: test_viewport(size),
                    primitives: &app.output().primitives,
                    resources: &resources,
                });

                assert!(
                    output.diagnostics.is_empty(),
                    "{page:?} at {size:?}: {:?}",
                    output.diagnostics
                );
            }
        }
    }

    #[test]
    fn showcase_pages_snap_text_origins_and_baselines_at_fractional_dpi() {
        for (size, scale_factor) in [
            (Size::new(1151.2, 719.2), 1.25),
            (Size::new(960.7, 602.0), 1.5),
        ] {
            for page in [
                ShowcasePage::Editor,
                ShowcasePage::Components,
                ShowcasePage::Layout,
                ShowcasePage::Viewport,
                ShowcasePage::Systems,
            ] {
                let mut app = ShowcaseApp::new();
                app.set_viewport_size(size);
                app.set_page(page);
                let resources = app.render_resources();
                let mut renderer = VelloRenderer::new();
                let output = renderer.submit_frame(RenderFrameInput {
                    viewport: test_viewport_scaled(size, scale_factor),
                    primitives: &app.output().primitives,
                    resources: &resources,
                });
                let encoding = renderer.scene().encoding();
                let glyphs = &encoding.resources.glyphs;

                assert!(
                    output.diagnostics.is_empty(),
                    "{page:?} at {size:?}: {:?}",
                    output.diagnostics
                );
                assert!(
                    !glyphs.is_empty(),
                    "{page:?} at {size:?} emitted no glyphs at fractional DPI"
                );
                assert!(
                    glyphs
                        .iter()
                        .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
                    "{page:?} at {size:?} emitted fractional glyph x positions"
                );
                assert!(
                    glyphs
                        .iter()
                        .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
                    "{page:?} at {size:?} emitted fractional glyph baselines"
                );
            }
        }
    }

    #[test]
    fn editor_open_menu_translates_to_vello_without_renderer_diagnostics() {
        let mut app = ShowcaseApp::new();
        click(&mut app, Point::new(145.0, 14.0));
        let resources = app.render_resources();
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: test_viewport(Size::new(1440.0, 900.0)),
            primitives: &app.output().primitives,
            resources: &resources,
        });

        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    }

    fn test_viewport(size: Size) -> ViewportInfo {
        test_viewport_scaled(size, 1.0)
    }

    fn test_viewport_scaled(size: Size, scale_factor: f64) -> ViewportInfo {
        ViewportInfo::new(
            size,
            PhysicalSize::new(
                (f64::from(size.width) * scale_factor).round().max(1.0) as u32,
                (f64::from(size.height) * scale_factor).round().max(1.0) as u32,
            ),
            ScaleFactor::new(scale_factor),
        )
    }
}
