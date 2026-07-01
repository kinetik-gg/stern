use super::{
    ACTION_COMMAND_PALETTE, ACTION_COMPONENTS_RUN, ACTION_EDITOR_DOCK_JOIN,
    ACTION_EDITOR_DOCK_SWAP, ACTION_SYSTEMS_DISPATCH, ACTION_VIEWPORT_GRID, ACTION_WORKSPACE_SAVE,
    ActionContext, ActionDescriptor, ActionInvocation, ActionQueue, ActionRoutingContext,
    ActionSource, Axis, ClipId, Color, CommandPaletteOverlay, Crosshair, Dock, DockDropTarget,
    DockNode, DockPlacement, DockSplitDemoState, EditorShowcase, Frame, FrameContext, FrameId,
    FrameOutput, GridColumns, GridLayout, Guide, IconId, ImageId, Insets, ItemId, Key, KeyEvent,
    KeyState, LayoutItem, ListLayout, Measurement, Menu, MenuOverlay, Modifiers, OverlayDismissal,
    OverlayEntry, OverlayId, OverlayKind, OverlayStack, PanZoom, Panel, PanelId, Point,
    PointerButtonState, PointerInput, PopoverPlacement, PopoverRequest, Primitive, Rect,
    RenderResources, RepaintRequest, SemanticNode, SemanticRole, ShowcaseApp, ShowcaseInput,
    ShowcasePage, Size, SizeRule, TableColumn, TableLayout, TextInputEvent, TextureId,
    TexturePrimitive, Ui, UiInput, Vec2, ViewportComposition, ViewportSurface, column_layout,
    default_dark_theme, frame_context, frame_tabs, inspect_primitives, line, nav_items,
    overlay_semantics, page_rect, panel_title, panel_title_body, place_popover, rect,
    rect_from_size, rgb, row_layout, sanitize_viewport_size, section_title, showcase_action_router,
    showcase_actions, solve_dock_layout, solve_dock_splitters, split_leading, text,
};

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

    pub(super) fn redraw_idle(&mut self) {
        self.output = self.frame(frame_context(self.viewport_size, UiInput::default()));
    }

    pub(super) fn frame(&mut self, context: FrameContext) -> FrameOutput {
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
                self.chrome_nav(&mut ui);
                self.page_content(&mut ui);
                self.chrome_status(&mut ui);
            }

            ui.finish_output()
        };
        let mut editor_handled = false;
        for invocation in &editor_invocations {
            editor_handled |= self.handle_action_invocation(invocation);
        }
        if editor_handled {
            output.request_repaint(RepaintRequest::NextFrame);
        }
        self.memory = memory;
        self.text_layouts = text_layouts;
        output
    }

    pub(super) fn to_ui_input(&self, input: &ShowcaseInput, viewport_changed: bool) -> UiInput {
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

    pub(super) fn invoke_action(&mut self, id: &str, source: ActionSource) -> bool {
        let handled = self.editor.apply_action(id) || Self::is_showcase_action(id);
        self.finish_action_invocation(id, source, handled)
    }

    pub(super) fn handle_applied_action_invocation(
        &mut self,
        invocation: &ActionInvocation,
    ) -> bool {
        self.finish_action_invocation(
            invocation.action_id.as_str(),
            invocation.source,
            Self::can_handle_action_id(invocation.action_id.as_str()),
        )
    }

    pub(super) fn finish_action_invocation(
        &mut self,
        action_id: &str,
        source: ActionSource,
        handled: bool,
    ) -> bool {
        if handled {
            self.record_action(action_id, source);
            true
        } else {
            self.ignore_action(action_id, source);
            false
        }
    }

    pub(super) fn can_handle_action_id(action_id: &str) -> bool {
        let mut editor = EditorShowcase::new();
        editor.apply_action(action_id)
            || Self::is_showcase_action(action_id)
            || Self::is_editor_rendered_action(action_id)
    }

    pub(super) fn handle_action_invocation(&mut self, invocation: &ActionInvocation) -> bool {
        if invocation.context == ActionContext::Editor {
            self.handle_applied_action_invocation(invocation)
        } else {
            self.invoke_action(invocation.action_id.as_str(), invocation.source)
        }
    }

    pub(super) fn handle_action_queue(&mut self, queue: &mut ActionQueue) -> Vec<ActionInvocation> {
        let invocations = queue.drain().collect::<Vec<_>>();
        for invocation in &invocations {
            self.handle_action_invocation(invocation);
        }
        invocations
    }

    pub(super) fn record_action(&mut self, action_id: &str, source: ActionSource) {
        self.action_count += 1;
        self.status = format!("{} via {:?} ({})", action_id, source, self.action_count);
    }

    pub(super) fn ignore_action(&mut self, action_id: &str, source: ActionSource) {
        self.status = format!("Ignored unhandled action {action_id} via {source:?}");
    }

    pub(super) fn is_showcase_action(action_id: &str) -> bool {
        matches!(
            action_id,
            ACTION_COMPONENTS_RUN
                | ACTION_SYSTEMS_DISPATCH
                | ACTION_WORKSPACE_SAVE
                | ACTION_COMMAND_PALETTE
                | ACTION_VIEWPORT_GRID
        )
    }

    pub(super) fn is_editor_rendered_action(action_id: &str) -> bool {
        matches!(action_id, ACTION_EDITOR_DOCK_JOIN | ACTION_EDITOR_DOCK_SWAP)
    }

    pub(super) fn resolve_shortcuts(&mut self, keyboard: &kinetik_ui::core::KeyboardInput) {
        let Some(invocation) =
            showcase_action_router().resolve_shortcut_in_context(keyboard, self.action_context())
        else {
            return;
        };
        self.invoke_action(invocation.action_id.as_str(), invocation.source);
    }

    pub(super) fn action_context(&self) -> ActionRoutingContext {
        let Some(focused) = self.memory.focused() else {
            return ActionRoutingContext::new();
        };
        if self.memory.text_input_owner() == Some(focused) {
            ActionRoutingContext::new().with_text_input(focused)
        } else {
            ActionRoutingContext::new().with_focused_widget(focused)
        }
    }

    pub(super) fn app_background(ui: &mut Ui<'_>) {
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

    pub(super) fn page_content(&mut self, ui: &mut Ui<'_>) {
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

    pub(super) fn page_content_height(&self, viewport: Rect) -> f32 {
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

    pub(super) fn chrome_nav(&mut self, ui: &mut Ui<'_>) {
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
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    pub(super) fn chrome_status(&self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
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

    pub(super) fn status_badge(
        ui: &mut Ui<'_>,
        rect_value: Rect,
        label: &str,
        value: &str,
        accent: Color,
    ) {
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

    pub(super) fn components_page(&mut self, ui: &mut Ui<'_>) {
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

    pub(super) fn component_controls(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn component_button_controls(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
        let run = ui.button(
            "components.run-action",
            Rect::new(x, y, 128.0, 30.0),
            "Run Action",
            false,
        );
        if run.clicked {
            self.invoke_action(ACTION_COMPONENTS_RUN, ActionSource::Button);
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

    pub(super) fn component_selection_controls(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
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

    pub(super) fn component_slider_controls(
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
        let icon = ui.icon_button_with_label(
            "components.icon",
            Rect::new(
                slider_x,
                slider_y + 44.0,
                icon_button_size,
                icon_button_size,
            ),
            IconId::from_raw(1),
            "Icon button",
            false,
        );
        if icon.clicked {
            "Icon button".clone_into(&mut self.status);
        }
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

    pub(super) fn component_text_inputs(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn collection_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn tabs_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn primitive_preview(ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn state_strip(ui: &mut Ui<'_>, bounds: Rect, value: &str) {
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

    pub(super) fn layout_page(&mut self, ui: &mut Ui<'_>) {
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

    pub(super) fn layout_solver_preview(ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn dock_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn dock_model_preview(&self) -> Dock {
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

    pub(super) fn dock_preview_controls(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn draw_dock_preview(ui: &mut Ui<'_>, area: &Dock, panel: Rect) {
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

    pub(super) fn table_preview(ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn viewport_page(&mut self, ui: &mut Ui<'_>) {
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

    pub(super) fn viewport_surface_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn viewport_controls_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn video_boundary_panel(ui: &mut Ui<'_>, panel: Rect) {
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

    pub(super) fn systems_page(&mut self, ui: &mut Ui<'_>) {
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

    pub(super) fn systems_action_panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        actions: &[ActionDescriptor],
    ) {
        panel_title(ui, panel, "Action Router");
        let menu_overlay = MenuOverlay::new(
            OverlayEntry::new(
                OverlayId::from_raw(101),
                OverlayKind::Menu,
                Rect::new(panel.x + 20.0, panel.y + 88.0, 140.0, 28.0),
            )
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
            Menu::from_actions(actions.to_vec()),
            ActionSource::Menu,
            ActionContext::Global,
        );
        let mut queue = ActionQueue::new();
        let dispatch_action = ActionDescriptor::new(ACTION_SYSTEMS_DISPATCH, "Dispatch");
        let x = panel.x + 20.0;
        let y = panel.y + 46.0;
        let dispatch = ui.button(
            "systems.dispatch",
            Rect::new(x, y, 140.0, 30.0),
            dispatch_action.label.clone(),
            !dispatch_action.state.enabled,
        );
        if dispatch.clicked {
            queue.invoke(
                dispatch_action.id.clone(),
                ActionSource::Button,
                ActionContext::Global,
            );
        }
        let menu_item = ui.button(
            "systems.menu-save",
            Rect::new(x, y + 44.0, 140.0, 28.0),
            "Menu Save",
            false,
        );
        if menu_item.clicked {
            menu_overlay.invoke_visible(0, &mut queue);
        }
        let invocations = self.handle_action_queue(&mut queue);
        text(
            ui,
            x + 160.0,
            y + 20.0,
            &format!("Invocations: {}", self.action_count),
            11.0,
            rgb(144, 184, 255),
        );
        for invocation in invocations {
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

    pub(super) fn systems_overlay_panel(ui: &mut Ui<'_>, panel: Rect) {
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
        let stack = Self::systems_overlay_stack(panel, menu_rect, popover_rect, palette_rect);
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

    pub(super) fn systems_overlay_stack(
        panel: Rect,
        menu_rect: Rect,
        popover_rect: Rect,
        palette_rect: Rect,
    ) -> OverlayStack {
        let menu_overlay = MenuOverlay::new(
            OverlayEntry::new(OverlayId::from_raw(1), OverlayKind::Menu, menu_rect)
                .dismiss_on(OverlayDismissal::OutsideClick),
            Menu::new(),
            ActionSource::Menu,
            ActionContext::Global,
        );
        let palette_overlay = CommandPaletteOverlay::from_actions(
            OverlayEntry::new(
                OverlayId::from_raw(3),
                OverlayKind::CommandPalette,
                palette_rect,
            )
            .modal(true),
            &[],
            ActionContext::Global,
        );
        let dropdown_rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(menu_rect.max_x() - 52.0, menu_rect.y + 8.0, 42.0, 20.0),
                size: Size::new(130.0, 42.0),
                placement: PopoverPlacement::Right,
                offset: 6.0,
                fit_viewport: true,
            },
            panel,
        );
        let tooltip_rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(menu_rect.x + 8.0, menu_rect.y, 80.0, 18.0),
                size: Size::new(120.0, 24.0),
                placement: PopoverPlacement::Above,
                offset: 4.0,
                fit_viewport: true,
            },
            panel,
        );
        let mut stack = OverlayStack::new();
        menu_overlay.open_in(&mut stack);
        let _ = stack.open_child(
            menu_overlay.entry.id,
            OverlayEntry::new(OverlayId::from_raw(2), OverlayKind::Popover, popover_rect)
                .dismiss_on(OverlayDismissal::OutsideClick),
        );
        stack.open(
            OverlayEntry::new(OverlayId::from_raw(4), OverlayKind::Dropdown, dropdown_rect)
                .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        );
        stack.open(OverlayEntry::new(
            OverlayId::from_raw(5),
            OverlayKind::Tooltip,
            tooltip_rect,
        ));
        palette_overlay.open_in(&mut stack);
        stack
    }

    pub(super) fn systems_palette_panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        actions: &[ActionDescriptor],
    ) {
        panel_title(ui, panel, "Command Palette");
        let mut palette_overlay = CommandPaletteOverlay::from_actions(
            OverlayEntry::new(
                OverlayId::from_raw(201),
                OverlayKind::CommandPalette,
                Rect::new(
                    panel.x + 20.0,
                    panel.y + 42.0,
                    (panel.width - 40.0).max(160.0),
                    132.0,
                ),
            )
            .modal(true)
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
            actions,
            ActionContext::Global,
        );
        palette_overlay.palette.query = String::new();
        let x = panel.x + 20.0;
        let row_width = (panel.width - 40.0).max(160.0);
        let entries = palette_overlay
            .matches()
            .into_iter()
            .take(4)
            .map(|entry| entry.label.clone())
            .collect::<Vec<_>>();
        for (index, label) in entries.into_iter().enumerate() {
            let y = panel.y + 50.0 + index as f32 * 32.0;
            let response = ui.list_row(
                ("systems.palette", index),
                Rect::new(x, y, row_width, 28.0),
                &label,
                false,
                false,
            );
            if response.clicked {
                let mut queue = ActionQueue::new();
                palette_overlay.palette.selected = index;
                palette_overlay.invoke_selected(&mut queue);
                self.handle_action_queue(&mut queue);
            }
        }
    }

    pub(super) fn systems_stress_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Primitive Stress");
        let origin = Point::new(panel.x + 36.0, panel.y + 70.0);
        self.systems_stress_slider(ui, panel, origin);

        let wide = panel.width >= 820.0;
        let snapshot = Self::stress_snapshot_rect(panel, origin, wide);
        let tile_area = Self::stress_tile_area(panel, origin, snapshot, wide);
        Self::draw_stress_tiles(ui, tile_area, self.stress);
        self.draw_runtime_snapshot(ui, snapshot, wide);
    }

    pub(super) fn systems_stress_slider(&mut self, ui: &mut Ui<'_>, panel: Rect, origin: Point) {
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

    pub(super) fn stress_snapshot_rect(panel: Rect, origin: Point, wide: bool) -> Rect {
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

    pub(super) fn stress_tile_area(panel: Rect, origin: Point, snapshot: Rect, wide: bool) -> Rect {
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

    pub(super) fn draw_stress_tiles(ui: &mut Ui<'_>, tile_area: Rect, stress: usize) {
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

    pub(super) fn draw_runtime_snapshot(&self, ui: &mut Ui<'_>, snapshot: Rect, wide: bool) {
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
