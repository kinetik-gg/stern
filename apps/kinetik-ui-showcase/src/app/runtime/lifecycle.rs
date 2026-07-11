use super::super::{
    FrameContext, FrameOutput, Key, KeyEvent, KeyState, Modifiers, PointerButtonState,
    PointerInput, Primitive, RenderResources, RepaintRequest, ShowcaseApp, ShowcaseInput,
    ShowcasePage, Size, TextInputEvent, Ui, UiInput, Vec2, default_dark_theme, frame_context,
    sanitize_viewport_size,
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
        ShowcasePage::parse(name)
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

    pub(in crate::app) fn redraw_idle(&mut self) {
        self.output = self.frame(frame_context(self.viewport_size, UiInput::default()));
    }

    pub(in crate::app) fn frame(&mut self, context: FrameContext) -> FrameOutput {
        let theme = default_dark_theme();
        let mut memory = std::mem::take(&mut self.memory);
        let mut text_layouts = std::mem::take(&mut self.text_layouts);
        let mut editor_invocations = Vec::new();
        let mut output = {
            let mut ui =
                Ui::begin_frame_with_text_layouts(context, &mut memory, &theme, &mut text_layouts);

            if self.page == ShowcasePage::Editor {
                editor_invocations = self.editor.render(&mut ui, self.action_count);
                self.editor_nav(&mut ui);
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
        for request in self.pending_platform_requests.drain(..) {
            output.push_platform_request(request);
        }
        self.memory = memory;
        self.text_layouts = text_layouts;
        output
    }

    pub(in crate::app) fn to_ui_input(
        &self,
        input: &ShowcaseInput,
        viewport_changed: bool,
    ) -> UiInput {
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
            events: Vec::new(),
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
}
