//! Windowless helpers for deterministic core runtime tests.

use std::time::Duration;

use crate::{
    ActionInvocation, ActionQueue, ActionRouter, ActionRoutingContext, ClipboardText, FrameContext,
    FrameOutput, FrameWarning, InputWheelDelta, Key, KeyEvent, KeyState, Modifiers, MouseButton,
    PhysicalKey, PlatformRequest, Point, Primitive, RepaintRequest, ScaleFactor, SemanticTree,
    Size, TextInputEvent, TextRange, TimeInfo, Ui, UiInput, UiInputEvent, UiMemory, Vec2,
    ViewportInfo, WidgetId,
};

/// Deterministic harness-visible phases recorded by frame trace helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessPhase {
    /// Typed scripted input was applied before the frame.
    ScriptedInputPrep,
    /// The harness began constructing a frame context and UI runtime.
    FrameBegin,
    /// The caller-provided frame build closure ran.
    Build,
    /// The runtime finalized frame output.
    FrameFinalization,
    /// Semantic output was inspected.
    InspectSemantics,
    /// Action output was inspected.
    InspectActions,
    /// Platform request output was inspected.
    InspectPlatformRequests,
    /// Repaint output was inspected.
    InspectRepaint,
    /// Warning output was inspected.
    InspectWarnings,
}

/// Ordered harness phase trace for one or more deterministic test frames.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTrace {
    phases: Vec<HarnessPhase>,
}

impl FrameTrace {
    /// Creates an empty phase trace.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns recorded phases in order.
    #[must_use]
    pub fn phases(&self) -> &[HarnessPhase] {
        &self.phases
    }

    fn push(&mut self, phase: HarnessPhase) {
        self.phases.push(phase);
    }

    fn extend(&mut self, other: Self) {
        self.phases.extend(other.phases);
    }
}

/// Deterministic reason a bounded settle run still has pending work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlePendingCause {
    /// One or more action invocations remain visible in the frame output.
    Actions,
    /// One or more platform requests remain visible in the frame output.
    PlatformRequests,
    /// One or more runtime warnings remain visible in the frame output.
    Warnings,
    /// The frame requested another repaint.
    Repaint(RepaintRequest),
}

/// Result of running bounded harness frames until idle or budget exhaustion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleResult {
    frames_run: usize,
    pending_cause: Option<SettlePendingCause>,
    trace: FrameTrace,
}

impl SettleResult {
    /// Creates a result for an idle settle run.
    #[must_use]
    pub fn idle(frames_run: usize, trace: FrameTrace) -> Self {
        Self {
            frames_run,
            pending_cause: None,
            trace,
        }
    }

    /// Creates a result for a settle run that exhausted its frame budget.
    #[must_use]
    pub fn pending(
        frames_run: usize,
        pending_cause: SettlePendingCause,
        trace: FrameTrace,
    ) -> Self {
        Self {
            frames_run,
            pending_cause: Some(pending_cause),
            trace,
        }
    }

    /// Returns true when the settle run reached idle output.
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        self.pending_cause.is_none()
    }

    /// Returns how many frames were run.
    #[must_use]
    pub const fn frames_run(&self) -> usize {
        self.frames_run
    }

    /// Returns the pending cause when the frame budget was exhausted.
    #[must_use]
    pub const fn pending_cause(&self) -> Option<SettlePendingCause> {
        self.pending_cause
    }

    /// Returns the collected phase trace for all frames run.
    #[must_use]
    pub const fn trace(&self) -> &FrameTrace {
        &self.trace
    }
}

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
    /// Add a line-wheel delta with explicit provenance.
    WheelLines(Vec2),
    /// Add a logical pixel-wheel delta with explicit provenance.
    WheelPixels(Vec2),
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
        self.run_frame_observed(build, None)
    }

    /// Runs one UI frame and returns a deterministic trace of harness-visible phases.
    #[must_use]
    pub fn run_frame_with_trace<T>(
        &mut self,
        build: impl FnOnce(&mut Ui<'_>) -> T,
    ) -> (T, FrameOutput, FrameTrace) {
        let mut trace = FrameTrace::new();
        let (result, output) = self.run_frame_observed(build, Some(&mut trace));
        (result, output, trace)
    }

    fn run_frame_observed<T>(
        &mut self,
        build: impl FnOnce(&mut Ui<'_>) -> T,
        mut trace: Option<&mut FrameTrace>,
    ) -> (T, FrameOutput) {
        if let Some(trace) = &mut trace {
            trace.push(HarnessPhase::FrameBegin);
        }
        let context = FrameContext::new(self.viewport, self.input.clone(), self.time);
        let mut ui = Ui::begin_frame(context, &mut self.memory);
        if let Some(trace) = &mut trace {
            trace.push(HarnessPhase::Build);
        }
        let result = build(&mut ui);
        if let Some(trace) = &mut trace {
            trace.push(HarnessPhase::FrameFinalization);
        }
        let output = ui.end_frame();
        if let Some(trace) = &mut trace {
            inspect_frame_output(trace, &output);
        }

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

    /// Runs one traced frame after applying typed scripted input operations.
    #[must_use]
    pub fn run_scripted_frame_with_trace<T>(
        &mut self,
        script: impl IntoIterator<Item = ScriptedInput>,
        build: impl FnOnce(&mut Ui<'_>) -> T,
    ) -> (T, FrameOutput, FrameTrace) {
        let mut trace = FrameTrace::new();
        trace.push(HarnessPhase::ScriptedInputPrep);
        self.apply_script(script);
        let (result, output) = self.run_frame_observed(build, Some(&mut trace));
        (result, output, trace)
    }

    /// Runs frames until output is idle or the explicit frame budget is exhausted.
    #[must_use]
    pub fn settle_frames<T>(
        &mut self,
        max_frames: usize,
        mut build: impl FnMut(&mut Ui<'_>) -> T,
    ) -> SettleResult {
        self.settle_frames_inner(
            max_frames,
            None::<std::iter::Empty<ScriptedInput>>,
            &mut build,
        )
    }

    /// Applies scripted input once, then runs frames until idle or budget exhaustion.
    #[must_use]
    pub fn settle_scripted_frames<T>(
        &mut self,
        script: impl IntoIterator<Item = ScriptedInput>,
        max_frames: usize,
        mut build: impl FnMut(&mut Ui<'_>) -> T,
    ) -> SettleResult {
        self.settle_frames_inner(max_frames, Some(script), &mut build)
    }

    fn settle_frames_inner<T>(
        &mut self,
        max_frames: usize,
        script: Option<impl IntoIterator<Item = ScriptedInput>>,
        build: &mut impl FnMut(&mut Ui<'_>) -> T,
    ) -> SettleResult {
        let mut trace = FrameTrace::new();
        if let Some(script) = script {
            trace.push(HarnessPhase::ScriptedInputPrep);
            self.apply_script(script);
        }

        let mut pending_cause = None;
        let mut frames_run = 0;
        for _ in 0..max_frames {
            let mut frame_trace = FrameTrace::new();
            let (_, output) = self.run_frame_observed(|ui| build(ui), Some(&mut frame_trace));
            trace.extend(frame_trace);
            frames_run += 1;

            pending_cause = pending_cause_for_output(&output);
            if pending_cause.is_none() {
                return SettleResult::idle(frames_run, trace);
            }
        }

        match pending_cause {
            Some(cause) => SettleResult::pending(frames_run, cause, trace),
            None => SettleResult::idle(frames_run, trace),
        }
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
        let mut shortcut_invocations = self.route_shortcuts(router, routing);
        self.run_frame(|ui| {
            let result = build(ui);
            for invocation in shortcut_invocations.drain() {
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
        self.route_shortcuts(router, routing).pop_front()
    }

    /// Resolves all pending keyboard shortcuts through an action router.
    #[must_use]
    pub fn route_shortcuts(
        &self,
        router: &ActionRouter,
        routing: ActionRoutingContext,
    ) -> ActionQueue {
        router.resolve_shortcuts_in_context(&self.input.keyboard, routing)
    }

    /// Applies one typed scripted input operation to the pending frame.
    pub fn apply_scripted_input(&mut self, input: ScriptedInput) {
        match input {
            ScriptedInput::PointerMove(position) => self.set_pointer_position(position),
            ScriptedInput::PointerDown(button) => self.pointer_press(button),
            ScriptedInput::PointerUp(button) => self.pointer_release(button),
            ScriptedInput::Wheel(delta) | ScriptedInput::WheelLines(delta) => {
                self.wheel_lines(delta);
            }
            ScriptedInput::WheelPixels(delta) => self.wheel_pixels(delta),
            ScriptedInput::Key(event) => {
                self.input
                    .push_event(UiInputEvent::Key(event.into_key_event()));
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
        let delta = self.input.pointer.position.map_or(Vec2::ZERO, |previous| {
            Vec2::new(position.x - previous.x, position.y - previous.y)
        });
        self.input
            .push_event(UiInputEvent::PointerMoved { position, delta });
    }

    /// Clears the pointer position for the next frame.
    pub fn clear_pointer_position(&mut self) {
        self.input.push_event(UiInputEvent::PointerLeft);
    }

    /// Queues a pointer button press for the next frame.
    pub fn pointer_press(&mut self, button: MouseButton) {
        self.input.push_event(UiInputEvent::PointerButton {
            button,
            down: true,
            click_count: self.input.pointer.click_count,
            position: self.input.pointer.position,
        });
    }

    /// Queues a pointer button release for the next frame.
    pub fn pointer_release(&mut self, button: MouseButton) {
        self.input.push_event(UiInputEvent::PointerButton {
            button,
            down: false,
            click_count: self.input.pointer.click_count,
            position: self.input.pointer.position,
        });
    }

    /// Adds a wheel delta to the next frame.
    pub fn wheel(&mut self, delta: Vec2) {
        self.wheel_lines(delta);
    }

    /// Adds a line-wheel delta to the next frame.
    pub fn wheel_lines(&mut self, delta: Vec2) {
        self.input.push_event(UiInputEvent::Wheel {
            delta: InputWheelDelta::Lines(delta),
            position: self.input.pointer.position,
        });
    }

    /// Adds a logical pixel-wheel delta to the next frame.
    pub fn wheel_pixels(&mut self, delta: Vec2) {
        self.input.push_event(UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(delta),
            position: self.input.pointer.position,
        });
    }

    /// Sets the click count reported for the next pointer activation.
    pub fn set_click_count(&mut self, click_count: u8) {
        self.input.pointer.click_count = click_count;
        if let Some(UiInputEvent::PointerButton {
            click_count: event_click_count,
            ..
        }) = self
            .input
            .events
            .iter_mut()
            .rev()
            .find(|event| matches!(event, UiInputEvent::PointerButton { .. }))
        {
            *event_click_count = click_count;
        }
    }

    /// Sets keyboard modifiers retained by the input snapshot.
    pub fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.input
            .push_event(UiInputEvent::ModifiersChanged(modifiers));
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
        self.input
            .push_event(UiInputEvent::Key(KeyEvent::with_physical_key(
                key,
                physical_key,
                state,
                self.input.keyboard.modifiers,
                repeat,
            )));
    }

    /// Queues a keyboard event with layout-produced hardware text.
    pub fn key_event_with_text(
        &mut self,
        key: Key,
        physical_key: PhysicalKey,
        state: KeyState,
        repeat: bool,
        text: impl Into<String>,
    ) {
        let mut event = KeyEvent::with_physical_key(
            key,
            physical_key,
            state,
            self.input.keyboard.modifiers,
            repeat,
        );
        if state == KeyState::Pressed {
            event = event.with_text(text);
        }
        self.input.push_event(UiInputEvent::Key(event));
    }

    /// Queues a committed text input event for the next frame.
    pub fn text_commit(&mut self, text: impl Into<String>) {
        self.input
            .push_event(UiInputEvent::Text(TextInputEvent::Commit(text.into())));
    }

    /// Queues a text composition start event for the next frame.
    pub fn text_composition_start(&mut self) {
        self.input
            .push_event(UiInputEvent::Text(TextInputEvent::CompositionStart));
    }

    /// Queues a text composition update for the next frame.
    pub fn text_composition(&mut self, text: impl Into<String>, selection: Option<TextRange>) {
        self.input
            .push_event(UiInputEvent::Text(TextInputEvent::Composition {
                text: text.into(),
                selection,
            }));
    }

    /// Queues a text composition end event for the next frame.
    pub fn text_composition_end(&mut self) {
        self.input
            .push_event(UiInputEvent::Text(TextInputEvent::CompositionEnd));
    }

    /// Queues clipboard text returned for a text-editing widget.
    pub fn clipboard_text(&mut self, target: WidgetId, text: impl Into<String>) {
        self.input
            .push_event(UiInputEvent::ClipboardText(ClipboardText::new(
                target, text,
            )));
    }

    /// Sets whether the synthetic window is focused.
    pub fn set_window_focused(&mut self, focused: bool) {
        if !focused {
            self.input.release_pointer_buttons();
            self.input.push_event(UiInputEvent::PointerLeft);
        }
        self.input
            .push_event(UiInputEvent::WindowFocusChanged(focused));
    }
}

fn inspect_frame_output(trace: &mut FrameTrace, output: &FrameOutput) {
    trace.push(HarnessPhase::InspectSemantics);
    let _ = &output.semantics;
    trace.push(HarnessPhase::InspectActions);
    let _ = &output.actions;
    trace.push(HarnessPhase::InspectPlatformRequests);
    let _ = &output.platform_requests;
    trace.push(HarnessPhase::InspectRepaint);
    let _ = output.repaint;
    trace.push(HarnessPhase::InspectWarnings);
    let _ = &output.warnings;
}

fn pending_cause_for_output(output: &FrameOutput) -> Option<SettlePendingCause> {
    if !output.actions.is_empty() {
        return Some(SettlePendingCause::Actions);
    }
    if !output.platform_requests.is_empty() {
        return Some(SettlePendingCause::PlatformRequests);
    }
    if !output.warnings.is_empty() {
        return Some(SettlePendingCause::Warnings);
    }
    match output.repaint {
        RepaintRequest::None => None,
        request => Some(SettlePendingCause::Repaint(request)),
    }
}
