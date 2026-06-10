//! Winit platform adapter for Kinetik UI.

use kinetik_ui_core::{
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, RepaintRequest, ScaleFactor, Size, TextInputEvent, UiInput, Vec2, ViewportInfo,
};
use winit::dpi::{PhysicalPosition, PhysicalSize as WinitPhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};
use winit::window::CursorIcon;

/// Cursor shape requested by UI code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorShape {
    /// Default arrow cursor.
    #[default]
    Default,
    /// Pointing hand cursor.
    Pointer,
    /// Text insertion cursor.
    Text,
    /// Grab cursor.
    Grab,
    /// Grabbing cursor.
    Grabbing,
}

impl CursorShape {
    /// Converts to a winit cursor icon.
    #[must_use]
    pub const fn to_winit(self) -> CursorIcon {
        match self {
            Self::Default => CursorIcon::Default,
            Self::Pointer => CursorIcon::Pointer,
            Self::Text => CursorIcon::Text,
            Self::Grab => CursorIcon::Grab,
            Self::Grabbing => CursorIcon::Grabbing,
        }
    }
}

/// Platform requests emitted by the adapter boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WinitPlatformRequests {
    /// Cursor shape to apply to the window.
    pub cursor: CursorShape,
    /// Redraw scheduling request.
    pub repaint: RepaintRequest,
}

impl WinitPlatformRequests {
    /// Updates the repaint request using core repaint priority rules.
    pub fn request_repaint(&mut self, repaint: RepaintRequest) {
        self.repaint = self.repaint.merge(repaint);
    }
}

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
    pub const fn new(scale_factor: ScaleFactor) -> Self {
        Self {
            input: UiInput {
                pointer: PointerInput {
                    position: None,
                    delta: Vec2::ZERO,
                    wheel_delta: Vec2::ZERO,
                    primary: PointerButtonState::new(false, false, false),
                    secondary: PointerButtonState::new(false, false, false),
                    middle: PointerButtonState::new(false, false, false),
                    click_count: 0,
                },
                keyboard: KeyboardInput {
                    modifiers: Modifiers::new(false, false, false, false),
                    events: Vec::new(),
                },
                text_events: Vec::new(),
                window_focused: false,
            },
            last_pointer_position: None,
            scale_factor,
        }
    }

    /// Starts a new frame while preserving button-down state.
    pub fn begin_frame(&mut self) {
        self.input.pointer.delta = Vec2::ZERO;
        self.input.pointer.wheel_delta = Vec2::ZERO;
        self.input.pointer.primary.pressed = false;
        self.input.pointer.primary.released = false;
        self.input.pointer.secondary.pressed = false;
        self.input.pointer.secondary.released = false;
        self.input.pointer.middle.pressed = false;
        self.input.pointer.middle.released = false;
        self.input.pointer.click_count = 0;
        self.input.keyboard.events.clear();
        self.input.text_events.clear();
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
        self.scale_factor = scale_factor;
    }

    /// Updates window focus state.
    pub fn set_window_focused(&mut self, focused: bool) {
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

    /// Applies a mouse button event.
    pub fn mouse_button(&mut self, button: MouseButton, state: ElementState, click_count: u8) {
        let button_state = match state {
            ElementState::Pressed => PointerButtonState::new(true, true, false),
            ElementState::Released => PointerButtonState::new(false, false, true),
        };
        match button {
            MouseButton::Left => self.input.pointer.primary = button_state,
            MouseButton::Right => self.input.pointer.secondary = button_state,
            MouseButton::Middle => self.input.pointer.middle = button_state,
            MouseButton::Back | MouseButton::Forward | MouseButton::Other(_) => {}
        }
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

    /// Applies committed text input.
    pub fn text_input(&mut self, text: impl Into<String>) {
        self.input
            .text_events
            .push(TextInputEvent::Commit(text.into()));
    }
}

/// Converts a winit physical size and scale factor into viewport information.
#[must_use]
pub fn viewport_from_winit(size: WinitPhysicalSize<u32>, scale_factor: f64) -> ViewportInfo {
    let scale_factor = ScaleFactor::new(scale_factor);
    let logical_width = f64::from(size.width) / scale_factor.value();
    let logical_height = f64::from(size.height) / scale_factor.value();

    ViewportInfo::new(
        Size::new(f64_to_f32(logical_width), f64_to_f32(logical_height)),
        PhysicalSize::new(size.width, size.height),
        scale_factor,
    )
}

/// Converts winit modifiers into core modifiers.
#[must_use]
pub fn modifiers_from_winit(modifiers: ModifiersState) -> Modifiers {
    Modifiers::new(
        modifiers.shift_key(),
        modifiers.control_key(),
        modifiers.alt_key(),
        modifiers.super_key(),
    )
}

/// Converts a winit key into a core key.
#[must_use]
pub fn key_from_winit(key: &WinitKey) -> Key {
    match key {
        WinitKey::Character(character) => Key::Character(character.to_string()),
        WinitKey::Named(NamedKey::Enter) => Key::Enter,
        WinitKey::Named(NamedKey::Escape) => Key::Escape,
        WinitKey::Named(NamedKey::Tab) => Key::Tab,
        WinitKey::Named(NamedKey::Backspace) => Key::Backspace,
        WinitKey::Named(NamedKey::Delete) => Key::Delete,
        WinitKey::Named(NamedKey::ArrowLeft) => Key::ArrowLeft,
        WinitKey::Named(NamedKey::ArrowRight) => Key::ArrowRight,
        WinitKey::Named(NamedKey::ArrowUp) => Key::ArrowUp,
        WinitKey::Named(NamedKey::ArrowDown) => Key::ArrowDown,
        WinitKey::Named(NamedKey::Space) => Key::Space,
        WinitKey::Named(NamedKey::F1) => Key::Function(1),
        WinitKey::Named(NamedKey::F2) => Key::Function(2),
        WinitKey::Named(NamedKey::F3) => Key::Function(3),
        WinitKey::Named(NamedKey::F4) => Key::Function(4),
        WinitKey::Named(NamedKey::F5) => Key::Function(5),
        WinitKey::Named(NamedKey::F6) => Key::Function(6),
        WinitKey::Named(NamedKey::F7) => Key::Function(7),
        WinitKey::Named(NamedKey::F8) => Key::Function(8),
        WinitKey::Named(NamedKey::F9) => Key::Function(9),
        WinitKey::Named(NamedKey::F10) => Key::Function(10),
        WinitKey::Named(NamedKey::F11) => Key::Function(11),
        WinitKey::Named(NamedKey::F12) => Key::Function(12),
        _ => Key::Unidentified,
    }
}

fn physical_position_to_logical(
    position: PhysicalPosition<f64>,
    scale_factor: ScaleFactor,
) -> Point {
    Point::new(
        f64_to_f32(position.x / scale_factor.value()),
        f64_to_f32(position.y / scale_factor.value()),
    )
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{
        CursorShape, WinitInputAdapter, key_from_winit, modifiers_from_winit, viewport_from_winit,
    };
    use kinetik_ui_core::{Key, KeyState, Modifiers, RepaintRequest, ScaleFactor};
    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::event::{ElementState, MouseButton, MouseScrollDelta};
    use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};

    #[test]
    fn viewport_conversion_uses_logical_units() {
        let viewport = viewport_from_winit(PhysicalSize::new(1920, 1080), 2.0);

        assert_eq!(viewport.logical_size.width, 960.0);
        assert_eq!(viewport.logical_size.height, 540.0);
        assert_eq!(viewport.physical_size.width, 1920);
        assert_eq!(viewport.scale_factor, ScaleFactor::new(2.0));
    }

    #[test]
    fn pointer_conversion_tracks_position_delta_button_and_wheel() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::new(2.0));

        adapter.pointer_moved(PhysicalPosition::new(20.0, 10.0));
        adapter.pointer_moved(PhysicalPosition::new(24.0, 16.0));
        adapter.mouse_button(MouseButton::Left, ElementState::Pressed, 1);
        adapter.mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));

        let input = adapter.input();
        assert_eq!(input.pointer.position.expect("position").x, 12.0);
        assert_eq!(input.pointer.delta.x, 2.0);
        assert!(input.pointer.primary.down);
        assert!(input.pointer.primary.pressed);
        assert_eq!(input.pointer.wheel_delta.y, -1.0);
    }

    #[test]
    fn begin_frame_clears_transient_input() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
        adapter.mouse_button(MouseButton::Left, ElementState::Pressed, 1);
        adapter.text_input("a");

        adapter.begin_frame();

        assert!(adapter.input().pointer.primary.down);
        assert!(!adapter.input().pointer.primary.pressed);
        assert!(adapter.input().text_events.is_empty());
    }

    #[test]
    fn keyboard_conversion_maps_named_and_character_keys() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

        adapter.keyboard_event(
            &WinitKey::Named(NamedKey::Enter),
            ElementState::Pressed,
            ModifiersState::CONTROL,
            false,
        );
        adapter.keyboard_event(
            &WinitKey::Character("s".into()),
            ElementState::Pressed,
            ModifiersState::CONTROL,
            true,
        );

        assert_eq!(adapter.input().keyboard.events[0].key, Key::Enter);
        assert_eq!(adapter.input().keyboard.events[0].state, KeyState::Pressed);
        assert_eq!(
            adapter.input().keyboard.events[1].key,
            Key::Character("s".to_owned())
        );
        assert!(adapter.input().keyboard.events[1].repeat);
    }

    #[test]
    fn modifier_conversion_maps_control() {
        assert_eq!(
            modifiers_from_winit(ModifiersState::CONTROL),
            Modifiers::new(false, true, false, false)
        );
    }

    #[test]
    fn key_conversion_maps_arrows_and_functions() {
        assert_eq!(
            key_from_winit(&WinitKey::Named(NamedKey::ArrowLeft)),
            Key::ArrowLeft
        );
        assert_eq!(
            key_from_winit(&WinitKey::Named(NamedKey::F5)),
            Key::Function(5)
        );
    }

    #[test]
    fn cursor_and_redraw_requests_are_represented() {
        let mut requests = super::WinitPlatformRequests {
            cursor: CursorShape::Text,
            repaint: RepaintRequest::After(core::time::Duration::from_secs(5)),
        };

        requests.request_repaint(RepaintRequest::NextFrame);

        assert_eq!(requests.cursor.to_winit(), winit::window::CursorIcon::Text);
        assert_eq!(requests.repaint, RepaintRequest::NextFrame);
    }
}
