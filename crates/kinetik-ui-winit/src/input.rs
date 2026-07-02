use kinetik_ui_core::{
    ClipboardText, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton as CoreMouseButton,
    Point, PointerButtonState, PointerInput, ScaleFactor, TextInputEvent, TextRange, UiInput, Vec2,
    WidgetId,
};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{Key as WinitKey, ModifiersState, PhysicalKey as WinitPhysicalKey};

use crate::conversions::{key_from_winit, modifiers_from_winit, physical_key_from_winit};
use crate::utils::{f64_to_f32, sanitize_scale_factor};
/// Accumulates winit events into one Kinetik UI input frame.
#[derive(Debug, Clone, PartialEq)]
pub struct WinitInputAdapter {
    input: UiInput,
    last_pointer_position: Option<Point>,
    scale_factor: ScaleFactor,
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
                pointer: PointerInput {
                    position: None,
                    delta: Vec2::ZERO,
                    wheel_delta: Vec2::ZERO,
                    primary: PointerButtonState::new(false, false, false),
                    secondary: PointerButtonState::new(false, false, false),
                    middle: PointerButtonState::new(false, false, false),
                    other_buttons: Vec::new(),
                    click_count: 0,
                },
                keyboard: KeyboardInput {
                    modifiers: Modifiers::new(false, false, false, false),
                    events: Vec::new(),
                },
                text_events: Vec::new(),
                clipboard_text: Vec::new(),
                window_focused: false,
            },
            last_pointer_position: None,
            scale_factor: sanitize_scale_factor(scale_factor),
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

    /// Consumes the adapter and returns the current input snapshot.
    #[must_use]
    pub fn into_input(self) -> UiInput {
        self.input
    }

    /// Updates the current scale factor.
    pub fn set_scale_factor(&mut self, scale_factor: ScaleFactor) {
        self.scale_factor = sanitize_scale_factor(scale_factor);
    }

    /// Updates window focus state.
    pub fn set_window_focused(&mut self, focused: bool) {
        if !focused {
            self.input.release_pointer_buttons();
            self.clear_pointer_position();
        }
        self.input.window_focused = focused;
    }

    /// Applies a pointer move event.
    pub fn pointer_moved(&mut self, position: PhysicalPosition<f64>) {
        let point = physical_position_to_logical(position, self.scale_factor);
        let delta = self.last_pointer_position.map_or(Vec2::ZERO, |last| {
            Vec2::new(point.x - last.x, point.y - last.y)
        });
        self.input.pointer.position = Some(point);
        self.input.pointer.delta = Vec2::new(
            self.input.pointer.delta.x + delta.x,
            self.input.pointer.delta.y + delta.y,
        );
        self.last_pointer_position = Some(point);
    }

    /// Applies a pointer leave event.
    pub fn pointer_left(&mut self) {
        self.clear_pointer_position();
    }

    /// Applies a mouse button event.
    pub fn mouse_button(&mut self, button: WinitMouseButton, state: ElementState, click_count: u8) {
        self.input.pointer.apply_button_transition(
            mouse_button_from_winit(button),
            state == ElementState::Pressed,
        );
        self.input.pointer.click_count = click_count;
    }

    /// Applies a mouse wheel event.
    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
            MouseScrollDelta::PixelDelta(position) => Vec2::new(
                f64_to_f32(position.x / self.scale_factor.value()),
                f64_to_f32(position.y / self.scale_factor.value()),
            ),
        };
        self.input.pointer.wheel_delta = Vec2::new(
            self.input.pointer.wheel_delta.x + delta.x,
            self.input.pointer.wheel_delta.y + delta.y,
        );
    }

    /// Updates the current keyboard modifier state.
    pub fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.input.keyboard.modifiers = modifiers_from_winit(modifiers);
    }

    fn clear_pointer_position(&mut self) {
        self.input.pointer.position = None;
        self.input.pointer.delta = Vec2::ZERO;
        self.last_pointer_position = None;
    }

    /// Applies a keyboard event.
    pub fn keyboard_event(
        &mut self,
        key: &WinitKey,
        state: ElementState,
        modifiers: ModifiersState,
        repeat: bool,
    ) {
        let key_state = match state {
            ElementState::Pressed => KeyState::Pressed,
            ElementState::Released => KeyState::Released,
        };
        let modifiers = modifiers_from_winit(modifiers);
        self.input.keyboard.modifiers = modifiers;
        self.input.keyboard.events.push(KeyEvent::new(
            key_from_winit(key),
            key_state,
            modifiers,
            repeat,
        ));
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
        let key_state = match state {
            ElementState::Pressed => KeyState::Pressed,
            ElementState::Released => KeyState::Released,
        };
        let modifiers = modifiers_from_winit(modifiers);
        self.input.keyboard.modifiers = modifiers;
        self.input.keyboard.events.push(KeyEvent::with_physical_key(
            key_from_winit(key),
            physical_key_from_winit(physical_key),
            key_state,
            modifiers,
            repeat,
        ));
    }

    /// Applies committed text input.
    pub fn text_input(&mut self, text: impl Into<String>) {
        self.input
            .text_events
            .push(TextInputEvent::Commit(text.into()));
    }

    /// Applies clipboard text returned by the application shell for a text input.
    pub fn clipboard_text(&mut self, target: WidgetId, text: impl Into<String>) {
        self.input
            .clipboard_text
            .push(ClipboardText::new(target, text));
    }

    /// Applies a winit IME event.
    pub fn ime_event(&mut self, event: Ime) {
        let event = match event {
            Ime::Enabled => TextInputEvent::CompositionStart,
            Ime::Preedit(text, selection) => TextInputEvent::Composition {
                text,
                selection: selection.map(|(start, end)| TextRange::new(start, end)),
            },
            Ime::Commit(text) => TextInputEvent::Commit(text),
            Ime::Disabled => TextInputEvent::CompositionEnd,
        };
        self.input.text_events.push(event);
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
