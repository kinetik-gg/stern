//! Windowless helpers for deterministic core runtime tests.

use std::time::Duration;

use crate::{
    ActionInvocation, ActionQueue, ActionRouter, ActionRoutingContext, ClipboardText, FrameContext,
    FrameOutput, FrameWarning, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalKey,
    PlatformRequest, Point, Primitive, RepaintRequest, ScaleFactor, SemanticTree, Size,
    TextInputEvent, TextRange, TimeInfo, Ui, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId,
};

/// Typed test input operation applied to a [`UiTestHarness`] before a frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptedInput {
    /// Move the pointer to a logical UI position.
    PointerMove(Point),
    /// Press a pointer button.
    PointerDown(MouseButton),
    /// Release a pointer button.
    PointerUp(MouseButton),
    /// Add a scroll-wheel delta.
    Wheel(Vec2),
    /// Queue a keyboard event.
    Key(ScriptedKeyEvent),
    /// Commit text input.
    TextCommit(String),
    /// Start text composition.
    TextCompositionStart,
    /// Update text composition.
    TextComposition {
        /// Current preedit text.
        text: String,
        /// Optional selected byte range inside the preedit text.
        selection: Option<TextRange>,
    },
    /// End text composition.
    TextCompositionEnd,
    /// Advance controlled frame time.
    AdvanceFrame(Duration),
}

impl ScriptedInput {
    /// Creates a logical key press operation with explicit modifiers.
    #[must_use]
    pub fn key_press(key: Key, modifiers: Modifiers) -> Self {
        Self::Key(ScriptedKeyEvent::press(key, modifiers))
    }

    /// Creates a logical key release operation with explicit modifiers.
    #[must_use]
    pub fn key_release(key: Key, modifiers: Modifiers) -> Self {
        Self::Key(ScriptedKeyEvent::release(key, modifiers))
    }

    /// Creates a physical-key press operation with explicit modifiers.
    #[must_use]
    pub fn physical_key_press(key: Key, physical_key: PhysicalKey, modifiers: Modifiers) -> Self {
        Self::Key(ScriptedKeyEvent::physical_press(
            key,
            physical_key,
            modifiers,
        ))
    }

    /// Creates a physical-key release operation with explicit modifiers.
    #[must_use]
    pub fn physical_key_release(key: Key, physical_key: PhysicalKey, modifiers: Modifiers) -> Self {
        Self::Key(ScriptedKeyEvent::physical_release(
            key,
            physical_key,
            modifiers,
        ))
    }

    /// Creates a repeated logical key press operation.
    #[must_use]
    pub fn key_repeat(key: Key, modifiers: Modifiers) -> Self {
        Self::Key(ScriptedKeyEvent {
            key,
            physical_key: PhysicalKey::Unidentified,
            state: KeyState::Pressed,
            modifiers,
            repeat: true,
        })
    }
}

/// Typed keyboard operation used by [`ScriptedInput::Key`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedKeyEvent {
    /// Layout-resolved logical key.
    pub key: Key,
    /// Platform-independent physical key.
    pub physical_key: PhysicalKey,
    /// Pressed or released state.
    pub state: KeyState,
    /// Modifiers active for this event.
    pub modifiers: Modifiers,
    /// Whether this event is an auto-repeat.
    pub repeat: bool,
}

impl ScriptedKeyEvent {
    /// Creates a logical key press.
    #[must_use]
    pub const fn press(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            physical_key: PhysicalKey::Unidentified,
            state: KeyState::Pressed,
            modifiers,
            repeat: false,
        }
    }

    /// Creates a logical key release.
    #[must_use]
    pub const fn release(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            physical_key: PhysicalKey::Unidentified,
            state: KeyState::Released,
            modifiers,
            repeat: false,
        }
    }

    /// Creates a physical-key press.
    #[must_use]
    pub const fn physical_press(key: Key, physical_key: PhysicalKey, modifiers: Modifiers) -> Self {
        Self {
            key,
            physical_key,
            state: KeyState::Pressed,
            modifiers,
            repeat: false,
        }
    }

    /// Creates a physical-key release.
    #[must_use]
    pub const fn physical_release(
        key: Key,
        physical_key: PhysicalKey,
        modifiers: Modifiers,
    ) -> Self {
        Self {
            key,
            physical_key,
            state: KeyState::Released,
            modifiers,
            repeat: false,
        }
    }

    fn into_key_event(self) -> KeyEvent {
        KeyEvent::with_physical_key(
            self.key,
            self.physical_key,
            self.state,
            self.modifiers,
            self.repeat,
        )
    }
}

/// Small core-only harness for building deterministic UI frames in tests.
///
/// The harness owns the same state a platform adapter would provide to core:
/// viewport metadata, normalized input, retained UI memory, and controlled
/// frame time. It never creates a window, renderer, GPU resource, OS service,
/// or accessibility adapter.
#[derive(Debug, Clone)]
pub struct UiTestHarness {
    viewport: ViewportInfo,
    input: UiInput,
    memory: UiMemory,
    time: TimeInfo,
    last_output: Option<FrameOutput>,
}

impl Default for UiTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl UiTestHarness {
    /// Creates a harness with an `800x600` logical viewport at scale factor `1.0`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_viewport(Size::new(800.0, 600.0), ScaleFactor::ONE)
    }

    /// Creates a harness with the provided logical viewport size and scale factor.
    #[must_use]
    pub fn with_viewport(logical_size: Size, scale_factor: ScaleFactor) -> Self {
        let viewport = ViewportInfo::new(
            logical_size,
            scale_factor.logical_size_to_physical(logical_size),
            scale_factor,
        );
        let input = UiInput {
            window_focused: true,
            ..UiInput::default()
        };

        Self {
            viewport,
            input,
            memory: UiMemory::new(),
            time: TimeInfo::default(),
            last_output: None,
        }
    }

    /// Returns the current viewport metadata.
    #[must_use]
    pub const fn viewport(&self) -> ViewportInfo {
        self.viewport
    }

    /// Sets the logical viewport size and scale factor.
    pub fn set_viewport(&mut self, logical_size: Size, scale_factor: ScaleFactor) {
        self.viewport = ViewportInfo::new(
            logical_size,
            scale_factor.logical_size_to_physical(logical_size),
            scale_factor,
        );
    }

    /// Returns the retained UI memory.
    #[must_use]
    pub const fn memory(&self) -> &UiMemory {
        &self.memory
    }

    /// Returns mutable retained UI memory for test setup and assertions.
    pub fn memory_mut(&mut self) -> &mut UiMemory {
        &mut self.memory
    }

    /// Returns the pending input snapshot that will be used by the next frame.
    #[must_use]
    pub const fn input(&self) -> &UiInput {
        &self.input
    }

    /// Returns mutable input for cases that need direct access to core fields.
    pub fn input_mut(&mut self) -> &mut UiInput {
        &mut self.input
    }

    /// Returns the controlled time snapshot for the next frame.
    #[must_use]
    pub const fn time(&self) -> TimeInfo {
        self.time
    }

    /// Replaces the controlled time snapshot for the next frame.
    pub const fn set_time(&mut self, time: TimeInfo) {
        self.time = time;
    }

    /// Advances controlled time and increments the frame index by one.
    pub fn advance_frame(&mut self, delta: Duration) {
        self.time = TimeInfo::new(self.time.now + delta, delta, self.time.frame_index + 1);
    }

    /// Returns the most recent frame output, if a frame has been run.
    #[must_use]
    pub const fn last_output(&self) -> Option<&FrameOutput> {
        self.last_output.as_ref()
    }

    /// Returns the most recent render primitives, if a frame has been run.
    #[must_use]
    pub fn last_primitives(&self) -> Option<&[Primitive]> {
        self.last_output
            .as_ref()
            .map(|output| output.primitives.as_slice())
    }

    /// Returns the most recent semantic tree, if a frame has been run.
    #[must_use]
    pub fn last_semantics(&self) -> Option<&SemanticTree> {
        self.last_output.as_ref().map(|output| &output.semantics)
    }

    /// Returns the most recent action queue, if a frame has been run.
    #[must_use]
    pub fn last_actions(&self) -> Option<&ActionQueue> {
        self.last_output.as_ref().map(|output| &output.actions)
    }

    /// Returns the most recent platform requests, if a frame has been run.
    #[must_use]
    pub fn last_platform_requests(&self) -> Option<&[PlatformRequest]> {
        self.last_output
            .as_ref()
            .map(|output| output.platform_requests.as_slice())
    }

    /// Returns the most recent repaint request, if a frame has been run.
    #[must_use]
    pub fn last_repaint(&self) -> Option<RepaintRequest> {
        self.last_output.as_ref().map(|output| output.repaint)
    }

    /// Returns the most recent frame warnings, if a frame has been run.
    #[must_use]
    pub fn last_warnings(&self) -> Option<&[FrameWarning]> {
        self.last_output
            .as_ref()
            .map(|output| output.warnings.as_slice())
    }

    /// Runs one UI frame and returns the closure result plus finalized output.
    ///
    /// Frame-local input such as button edges, wheel delta, key events, text
    /// events, and clipboard text is cleared after the frame. Retained input
    /// state such as pointer position, pointer button down state, modifiers,
    /// and window focus remains available for later frames.
    #[must_use]
    pub fn run_frame<T>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> T) -> (T, FrameOutput) {
        let context = FrameContext::new(self.viewport, self.input.clone(), self.time);
        let mut ui = Ui::begin_frame(context, &mut self.memory);
        let result = build(&mut ui);
        let output = ui.end_frame();

        self.last_output = Some(output.clone());
        self.input.begin_frame();

        (result, output)
    }

    /// Runs one frame after applying typed scripted input operations.
    #[must_use]
    pub fn run_scripted_frame<T>(
        &mut self,
        script: impl IntoIterator<Item = ScriptedInput>,
        build: impl FnOnce(&mut Ui<'_>) -> T,
    ) -> (T, FrameOutput) {
        self.apply_script(script);
        self.run_frame(build)
    }

    /// Runs one frame and emits a routed shortcut invocation into frame output.
    ///
    /// The router only resolves shortcut intent. Applications remain
    /// responsible for executing the resulting action invocation.
    #[must_use]
    pub fn run_frame_with_action_router<T>(
        &mut self,
        router: &ActionRouter,
        routing: ActionRoutingContext,
        build: impl FnOnce(&mut Ui<'_>) -> T,
    ) -> (T, FrameOutput) {
        let shortcut_invocation = self.route_shortcut(router, routing);
        self.run_frame(|ui| {
            let result = build(ui);
            if let Some(invocation) = shortcut_invocation {
                ui.push_action(invocation);
            }
            result
        })
    }

    /// Runs one scripted frame and emits a routed shortcut invocation into frame output.
    #[must_use]
    pub fn run_scripted_frame_with_action_router<T>(
        &mut self,
        script: impl IntoIterator<Item = ScriptedInput>,
        router: &ActionRouter,
        routing: ActionRoutingContext,
        build: impl FnOnce(&mut Ui<'_>) -> T,
    ) -> (T, FrameOutput) {
        self.apply_script(script);
        self.run_frame_with_action_router(router, routing, build)
    }

    /// Resolves the pending keyboard input through an action router.
    #[must_use]
    pub fn route_shortcut(
        &self,
        router: &ActionRouter,
        routing: ActionRoutingContext,
    ) -> Option<ActionInvocation> {
        router.resolve_shortcut_in_context(&self.input.keyboard, routing)
    }

    /// Applies one typed scripted input operation to the pending frame.
    pub fn apply_scripted_input(&mut self, input: ScriptedInput) {
        match input {
            ScriptedInput::PointerMove(position) => self.set_pointer_position(position),
            ScriptedInput::PointerDown(button) => self.pointer_press(button),
            ScriptedInput::PointerUp(button) => self.pointer_release(button),
            ScriptedInput::Wheel(delta) => self.wheel(delta),
            ScriptedInput::Key(event) => {
                let modifiers = event.modifiers;
                self.input.keyboard.modifiers = modifiers;
                self.input.keyboard.events.push(event.into_key_event());
            }
            ScriptedInput::TextCommit(text) => self.text_commit(text),
            ScriptedInput::TextCompositionStart => self.text_composition_start(),
            ScriptedInput::TextComposition { text, selection } => {
                self.text_composition(text, selection);
            }
            ScriptedInput::TextCompositionEnd => self.text_composition_end(),
            ScriptedInput::AdvanceFrame(delta) => self.advance_frame(delta),
        }
    }

    /// Applies typed scripted input operations to the pending frame in order.
    pub fn apply_script(&mut self, script: impl IntoIterator<Item = ScriptedInput>) {
        for input in script {
            self.apply_scripted_input(input);
        }
    }

    /// Sets the pointer position in logical UI coordinates.
    ///
    /// When the previous position is known, pointer delta is updated by the
    /// movement between the previous and new positions.
    pub fn set_pointer_position(&mut self, position: Point) {
        self.input.pointer.delta = self.input.pointer.position.map_or(Vec2::ZERO, |previous| {
            Vec2::new(position.x - previous.x, position.y - previous.y)
        });
        self.input.pointer.position = Some(position);
    }

    /// Clears the pointer position for the next frame.
    pub fn clear_pointer_position(&mut self) {
        self.input.pointer.position = None;
        self.input.pointer.delta = Vec2::ZERO;
    }

    /// Queues a pointer button press for the next frame.
    pub fn pointer_press(&mut self, button: MouseButton) {
        self.input.pointer.apply_button_transition(button, true);
    }

    /// Queues a pointer button release for the next frame.
    pub fn pointer_release(&mut self, button: MouseButton) {
        self.input.pointer.apply_button_transition(button, false);
    }

    /// Adds a wheel delta to the next frame.
    pub fn wheel(&mut self, delta: Vec2) {
        self.input.pointer.wheel_delta = Vec2::new(
            self.input.pointer.wheel_delta.x + delta.x,
            self.input.pointer.wheel_delta.y + delta.y,
        );
    }

    /// Sets the click count reported for the next pointer activation.
    pub const fn set_click_count(&mut self, click_count: u8) {
        self.input.pointer.click_count = click_count;
    }

    /// Sets keyboard modifiers retained by the input snapshot.
    pub const fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.input.keyboard.modifiers = modifiers;
    }

    /// Queues a logical key press for the next frame.
    pub fn key_press(&mut self, key: Key) {
        self.key_event(key, PhysicalKey::Unidentified, KeyState::Pressed, false);
    }

    /// Queues a logical key release for the next frame.
    pub fn key_release(&mut self, key: Key) {
        self.key_event(key, PhysicalKey::Unidentified, KeyState::Released, false);
    }

    /// Queues a keyboard event with explicit physical-key and repeat data.
    pub fn key_event(
        &mut self,
        key: Key,
        physical_key: PhysicalKey,
        state: KeyState,
        repeat: bool,
    ) {
        self.input.keyboard.events.push(KeyEvent::with_physical_key(
            key,
            physical_key,
            state,
            self.input.keyboard.modifiers,
            repeat,
        ));
    }

    /// Queues a committed text input event for the next frame.
    pub fn text_commit(&mut self, text: impl Into<String>) {
        self.input
            .text_events
            .push(TextInputEvent::Commit(text.into()));
    }

    /// Queues a text composition start event for the next frame.
    pub fn text_composition_start(&mut self) {
        self.input
            .text_events
            .push(TextInputEvent::CompositionStart);
    }

    /// Queues a text composition update for the next frame.
    pub fn text_composition(&mut self, text: impl Into<String>, selection: Option<TextRange>) {
        self.input.text_events.push(TextInputEvent::Composition {
            text: text.into(),
            selection,
        });
    }

    /// Queues a text composition end event for the next frame.
    pub fn text_composition_end(&mut self) {
        self.input.text_events.push(TextInputEvent::CompositionEnd);
    }

    /// Queues clipboard text returned for a text-editing widget.
    pub fn clipboard_text(&mut self, target: WidgetId, text: impl Into<String>) {
        self.input
            .clipboard_text
            .push(ClipboardText::new(target, text));
    }

    /// Sets whether the synthetic window is focused.
    pub const fn set_window_focused(&mut self, focused: bool) {
        self.input.window_focused = focused;
    }
}
