use std::time::{Duration, Instant};

use kinetik_ui_core::{
    ClipboardText, InputWheelDelta, KeyEvent, KeyState, MouseButton as CoreMouseButton, Point,
    ScaleFactor, TextInputEvent, TextRange, UiInput, UiInputEvent, Vec2, WidgetId,
};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{Key as WinitKey, ModifiersState, PhysicalKey as WinitPhysicalKey};

use crate::conversions::{key_from_winit, modifiers_from_winit, physical_key_from_winit};
use crate::shell::{WinitShellFailure, WinitShellOutcome, WinitShellResult};
use crate::utils::{f64_to_f32, sanitize_scale_factor};

const MULTI_CLICK_MAX_DELAY: Duration = Duration::from_millis(500);
const MULTI_CLICK_MAX_DISTANCE_SQUARED: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClickAnchor {
    button: CoreMouseButton,
    count: u8,
    position: Point,
    pressed_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ActiveClick {
    button: CoreMouseButton,
    count: u8,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct AutomaticClickSequence {
    anchor: Option<ClickAnchor>,
    active: Option<ActiveClick>,
    last_event_at: Option<Instant>,
}

impl AutomaticClickSequence {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn transition(
        &mut self,
        button: CoreMouseButton,
        down: bool,
        position: Option<Point>,
        at: Instant,
    ) -> u8 {
        if down {
            self.press(button, position, at)
        } else {
            self.release(button, position, at)
        }
    }

    fn press(&mut self, button: CoreMouseButton, position: Option<Point>, at: Instant) -> u8 {
        let backwards = self.last_event_at.is_some_and(|last| at < last);
        if backwards || self.active.is_some() {
            self.reset();
        }

        let count = self
            .anchor
            .filter(|anchor| anchor.button == button)
            .filter(|anchor| {
                at.checked_duration_since(anchor.pressed_at)
                    .is_some_and(|elapsed| elapsed <= MULTI_CLICK_MAX_DELAY)
            })
            .filter(|anchor| {
                position.is_some_and(|position| within_click_distance(anchor.position, position))
            })
            .map_or(1, |anchor| anchor.count.saturating_add(1));

        self.anchor = position.map(|position| ClickAnchor {
            button,
            count,
            position,
            pressed_at: at,
        });
        self.active = Some(ActiveClick { button, count });
        self.last_event_at = Some(at);
        count
    }

    fn release(&mut self, button: CoreMouseButton, position: Option<Point>, at: Instant) -> u8 {
        let Some(active) = self.active.filter(|active| active.button == button) else {
            self.reset();
            return 0;
        };

        let backwards = self.last_event_at.is_some_and(|last| at < last);
        self.active = None;
        if backwards || position.is_none() {
            self.anchor = None;
        }
        self.last_event_at = Some(at);
        active.count
    }
}

fn within_click_distance(left: Point, right: Point) -> bool {
    let x = right.x - left.x;
    let y = right.y - left.y;
    let squared = x * x + y * y;
    squared.is_finite() && squared <= MULTI_CLICK_MAX_DISTANCE_SQUARED
}

/// Accumulates winit events into one Kinetik UI input frame.
#[derive(Debug, Clone, PartialEq)]
pub struct WinitInputAdapter {
    input: UiInput,
    last_pointer_position: Option<Point>,
    scale_factor: ScaleFactor,
    ime_enabled: bool,
    composition_active: bool,
    click_sequence: AutomaticClickSequence,
}

impl Default for WinitInputAdapter {
    fn default() -> Self {
        Self::new(ScaleFactor::ONE)
    }
}

impl WinitInputAdapter {
    /// Creates an input adapter.
    #[must_use]
    pub fn new(scale_factor: ScaleFactor) -> Self {
        Self {
            input: UiInput {
                window_focused: false,
                ..UiInput::default()
            },
            last_pointer_position: None,
            scale_factor: sanitize_scale_factor(scale_factor),
            ime_enabled: false,
            composition_active: false,
            click_sequence: AutomaticClickSequence::default(),
        }
    }

    /// Starts a new frame while preserving button-down state.
    pub fn begin_frame(&mut self) {
        self.input.begin_frame();
    }

    /// Returns the current input snapshot.
    #[must_use]
    pub fn input(&self) -> &UiInput {
        &self.input
    }

    /// Returns the most recently reported platform IME availability.
    #[must_use]
    pub const fn ime_enabled(&self) -> bool {
        self.ime_enabled
    }

    /// Consumes the adapter and returns the current input snapshot.
    #[must_use]
    pub fn into_input(self) -> UiInput {
        self.input
    }

    /// Updates the current scale factor.
    ///
    /// A real sanitized change invalidates automatic click history and records
    /// pointer leave so no projected position or delta survives from the old
    /// logical coordinate basis. An equal sanitized value has no input effect.
    pub fn set_scale_factor(&mut self, scale_factor: ScaleFactor) {
        let scale_factor = sanitize_scale_factor(scale_factor);
        if self.scale_factor != scale_factor {
            self.click_sequence.reset();
            self.last_pointer_position = None;
            self.input.push_event(UiInputEvent::PointerLeft);
            self.scale_factor = scale_factor;
        }
    }

    /// Updates window focus state.
    pub fn set_window_focused(&mut self, focused: bool) {
        if !focused {
            self.click_sequence.reset();
            self.end_composition();
            self.input.push_event(UiInputEvent::PointerReleaseAll {
                position: self.last_pointer_position,
            });
            self.input.push_event(UiInputEvent::PointerLeft);
            self.last_pointer_position = None;
        }
        self.input
            .push_event(UiInputEvent::WindowFocusChanged(focused));
    }

    /// Applies a pointer move event.
    pub fn pointer_moved(&mut self, position: PhysicalPosition<f64>) {
        let point = physical_position_to_logical(position, self.scale_factor);
        let delta = self.last_pointer_position.map_or(Vec2::ZERO, |last| {
            Vec2::new(point.x - last.x, point.y - last.y)
        });
        self.input.push_event(UiInputEvent::PointerMoved {
            position: point,
            delta,
        });
        self.last_pointer_position = Some(point);
    }

    /// Applies a pointer leave event.
    pub fn pointer_left(&mut self) {
        self.click_sequence.reset();
        self.input.push_event(UiInputEvent::PointerLeft);
        self.last_pointer_position = None;
    }

    /// Applies a mouse button event with an explicit click count.
    ///
    /// The supplied count is emitted unchanged. Calling this method clears all
    /// automatic click history, so a later [`Self::mouse_button_at`] transition
    /// starts a new sequence.
    pub fn mouse_button(&mut self, button: WinitMouseButton, state: ElementState, click_count: u8) {
        self.click_sequence.reset();
        self.push_mouse_button(button, state, click_count);
    }

    /// Applies a timestamped mouse button event with automatic click sequencing.
    ///
    /// Repeated presses of the same button within 500 milliseconds and four
    /// logical pixels increment the count with saturation. Matching releases
    /// carry the active press count without incrementing; unmatched or duplicate
    /// releases emit zero. Missing position, backwards time, overlapping or
    /// different-button transitions, pointer leave, focus loss, a real sanitized
    /// scale-factor change, or explicit-count input clears continuation. A scale
    /// change also invalidates logical pointer evidence until the next move.
    pub fn mouse_button_at(&mut self, button: WinitMouseButton, state: ElementState, at: Instant) {
        let core_button = mouse_button_from_winit(button);
        let click_count = self.click_sequence.transition(
            core_button,
            state == ElementState::Pressed,
            self.last_pointer_position,
            at,
        );
        self.push_mouse_button(button, state, click_count);
    }

    fn push_mouse_button(
        &mut self,
        button: WinitMouseButton,
        state: ElementState,
        click_count: u8,
    ) {
        self.input.push_event(UiInputEvent::PointerButton {
            button: mouse_button_from_winit(button),
            down: state == ElementState::Pressed,
            click_count,
            position: self.last_pointer_position,
        });
    }

    /// Applies a mouse wheel event.
    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => InputWheelDelta::Lines(Vec2::new(x, y)),
            MouseScrollDelta::PixelDelta(position) => InputWheelDelta::Pixels(Vec2::new(
                f64_to_f32(position.x / self.scale_factor.value()),
                f64_to_f32(position.y / self.scale_factor.value()),
            )),
        };
        self.input.push_event(UiInputEvent::Wheel {
            delta,
            position: self.last_pointer_position,
        });
    }

    /// Updates the current keyboard modifier state.
    pub fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.input
            .push_event(UiInputEvent::ModifiersChanged(modifiers_from_winit(
                modifiers,
            )));
    }

    /// Applies a keyboard event.
    pub fn keyboard_event(
        &mut self,
        key: &WinitKey,
        state: ElementState,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        self.keyboard_event_with_text(key, state, modifiers, repeat, None);
    }

    /// Applies a keyboard event with layout-produced hardware text.
    pub fn keyboard_event_with_text(
        &mut self,
        key: &WinitKey,
        state: ElementState,
        modifiers: ModifiersState,
        repeat: bool,
        text: Option<&str>,
    ) {
        self.push_key_event(
            KeyEvent::new(
                key_from_winit(key),
                key_state_from_winit(state),
                modifiers_from_winit(modifiers),
                repeat,
            ),
            text,
        );
    }

    /// Applies a keyboard event with physical key identity.
    pub fn keyboard_event_with_physical_key(
        &mut self,
        key: &WinitKey,
        physical_key: &WinitPhysicalKey,
        state: ElementState,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        self.keyboard_event_with_physical_key_and_text(
            key,
            physical_key,
            state,
            modifiers,
            repeat,
            None,
        );
    }

    /// Applies a physical keyboard event with layout-produced hardware text.
    #[allow(clippy::too_many_arguments)]
    pub fn keyboard_event_with_physical_key_and_text(
        &mut self,
        key: &WinitKey,
        physical_key: &WinitPhysicalKey,
        state: ElementState,
        modifiers: ModifiersState,
        repeat: bool,
        text: Option<&str>,
    ) {
        self.push_key_event(
            KeyEvent::with_physical_key(
                key_from_winit(key),
                physical_key_from_winit(physical_key),
                key_state_from_winit(state),
                modifiers_from_winit(modifiers),
                repeat,
            ),
            text,
        );
    }

    /// Applies committed text input.
    pub fn text_input(&mut self, text: impl Into<String>) {
        self.commit_text(text.into());
    }

    /// Applies clipboard text returned by the application shell for a text input.
    pub fn clipboard_text(&mut self, target: WidgetId, text: impl Into<String>) {
        self.input
            .push_event(UiInputEvent::ClipboardText(ClipboardText::new(
                target, text,
            )));
    }

    /// Appends targeted shell responses to the current ordered input frame and
    /// returns redacted failures for application diagnostics.
    pub fn apply_shell_outcome(&mut self, outcome: WinitShellOutcome) -> Vec<WinitShellFailure> {
        let mut failures = Vec::new();
        for result in outcome.into_results() {
            match result {
                WinitShellResult::ClipboardText(clipboard) => {
                    self.input
                        .push_event(UiInputEvent::ClipboardText(clipboard));
                }
                WinitShellResult::Failure(failure) => failures.push(failure),
            }
        }
        failures
    }

    /// Applies a winit IME event.
    pub fn ime_event(&mut self, event: Ime) {
        match event {
            Ime::Enabled => {
                self.ime_enabled = true;
                self.input.push_event(UiInputEvent::ImeEnabled(true));
            }
            Ime::Preedit(text, _) if text.is_empty() => self.end_composition(),
            Ime::Preedit(text, selection) => {
                if !self.composition_active {
                    self.composition_active = true;
                    self.input
                        .push_event(UiInputEvent::Text(TextInputEvent::CompositionStart));
                }
                self.input
                    .push_event(UiInputEvent::Text(TextInputEvent::Composition {
                        text,
                        selection: selection.map(|(start, end)| TextRange::new(start, end)),
                    }));
            }
            Ime::Commit(text) => self.commit_text(text),
            Ime::Disabled => {
                self.end_composition();
                self.ime_enabled = false;
                self.input.push_event(UiInputEvent::ImeEnabled(false));
            }
        }
    }

    fn push_key_event(&mut self, mut event: KeyEvent, text: Option<&str>) {
        if event.state == KeyState::Pressed && !self.composition_active {
            event.text = text.filter(|text| !text.is_empty()).map(str::to_owned);
        }
        self.input.push_event(UiInputEvent::Key(event));
    }

    fn commit_text(&mut self, text: String) {
        self.end_composition();
        self.input
            .push_event(UiInputEvent::Text(TextInputEvent::Commit(text)));
    }

    fn end_composition(&mut self) {
        if self.composition_active {
            self.composition_active = false;
            self.input
                .push_event(UiInputEvent::Text(TextInputEvent::CompositionEnd));
        }
    }
}

fn key_state_from_winit(state: ElementState) -> KeyState {
    match state {
        ElementState::Pressed => KeyState::Pressed,
        ElementState::Released => KeyState::Released,
    }
}

fn mouse_button_from_winit(button: WinitMouseButton) -> CoreMouseButton {
    match button {
        WinitMouseButton::Left => CoreMouseButton::Primary,
        WinitMouseButton::Right => CoreMouseButton::Secondary,
        WinitMouseButton::Middle => CoreMouseButton::Middle,
        WinitMouseButton::Back => CoreMouseButton::Other(4),
        WinitMouseButton::Forward => CoreMouseButton::Other(5),
        WinitMouseButton::Other(number) => CoreMouseButton::Other(number),
    }
}

fn physical_position_to_logical(
    position: PhysicalPosition<f64>,
    scale_factor: ScaleFactor,
) -> Point {
    let scale_factor = sanitize_scale_factor(scale_factor);
    Point::new(
        f64_to_f32(position.x / scale_factor.value()),
        f64_to_f32(position.y / scale_factor.value()),
    )
}
