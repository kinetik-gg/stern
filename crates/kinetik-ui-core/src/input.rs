//! Platform-independent input snapshots.

use crate::geometry::{Point, Vec2};

/// Mouse or pointer buttons recognized by the core input model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary activation button.
    Primary,
    /// Secondary/context button.
    Secondary,
    /// Middle mouse button.
    Middle,
    /// Additional pointer button.
    Other(u16),
}

/// Pressed/released state for a pointer button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PointerButtonState {
    /// Whether the button is currently down.
    pub down: bool,
    /// Whether the button was pressed during this frame.
    pub pressed: bool,
    /// Whether the button was released during this frame.
    pub released: bool,
}

impl PointerButtonState {
    /// Creates a pointer button state.
    #[must_use]
    pub const fn new(down: bool, pressed: bool, released: bool) -> Self {
        Self {
            down,
            pressed,
            released,
        }
    }
}

/// Pointer input normalized for the current frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PointerInput {
    /// Current pointer position in logical UI coordinates.
    pub position: Option<Point>,
    /// Pointer movement in logical UI units since the previous frame.
    pub delta: Vec2,
    /// Scroll wheel delta in logical units or lines as normalized by the platform adapter.
    pub wheel_delta: Vec2,
    /// Primary button state.
    pub primary: PointerButtonState,
    /// Secondary button state.
    pub secondary: PointerButtonState,
    /// Middle button state.
    pub middle: PointerButtonState,
    /// Number of consecutive clicks reported for the current activation.
    pub click_count: u8,
}

impl Default for PointerInput {
    fn default() -> Self {
        Self {
            position: None,
            delta: Vec2::ZERO,
            wheel_delta: Vec2::ZERO,
            primary: PointerButtonState::default(),
            secondary: PointerButtonState::default(),
            middle: PointerButtonState::default(),
            click_count: 0,
        }
    }
}

impl PointerInput {
    /// Returns the state for a mouse button.
    #[must_use]
    pub const fn button(&self, button: MouseButton) -> PointerButtonState {
        match button {
            MouseButton::Primary => self.primary,
            MouseButton::Secondary => self.secondary,
            MouseButton::Middle => self.middle,
            MouseButton::Other(_) => PointerButtonState::new(false, false, false),
        }
    }
}

/// Keyboard modifier state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[allow(clippy::struct_excessive_bools)]
pub struct Modifiers {
    /// Shift key.
    pub shift: bool,
    /// Control key.
    pub ctrl: bool,
    /// Alt/Option key.
    pub alt: bool,
    /// Super/Command/Windows key.
    pub super_key: bool,
}

impl Modifiers {
    /// Creates a modifier state.
    #[must_use]
    #[allow(clippy::fn_params_excessive_bools)]
    pub const fn new(shift: bool, ctrl: bool, alt: bool, super_key: bool) -> Self {
        Self {
            shift,
            ctrl,
            alt,
            super_key,
        }
    }

    /// Returns true when no modifiers are active.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.super_key
    }
}

/// Pressed/released keyboard state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// Key was pressed.
    Pressed,
    /// Key was released.
    Released,
}

/// Platform-independent key identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    /// A printable character key after keyboard layout resolution.
    Character(String),
    /// Enter/Return.
    Enter,
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Arrow left.
    ArrowLeft,
    /// Arrow right.
    ArrowRight,
    /// Arrow up.
    ArrowUp,
    /// Arrow down.
    ArrowDown,
    /// Space.
    Space,
    /// Function key by number.
    Function(u8),
    /// A key that has not been mapped yet.
    Unidentified,
}

/// A keyboard event received during the current frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// Key identity.
    pub key: Key,
    /// Pressed/released state.
    pub state: KeyState,
    /// Active modifiers at the time of the event.
    pub modifiers: Modifiers,
    /// Whether this event is an auto-repeat.
    pub repeat: bool,
}

impl KeyEvent {
    /// Creates a keyboard event.
    #[must_use]
    pub const fn new(key: Key, state: KeyState, modifiers: Modifiers, repeat: bool) -> Self {
        Self {
            key,
            state,
            modifiers,
            repeat,
        }
    }
}

/// Keyboard input normalized for the current frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyboardInput {
    /// Current modifier state.
    pub modifiers: Modifiers,
    /// Keyboard events that occurred this frame.
    pub events: Vec<KeyEvent>,
}

/// Text input event separated from shortcut-oriented keyboard events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
    /// Committed text input.
    Commit(String),
    /// Text composition update for IME-like input paths.
    Composition(String),
}

/// Complete normalized input snapshot for one UI frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiInput {
    /// Pointer input.
    pub pointer: PointerInput,
    /// Keyboard input.
    pub keyboard: KeyboardInput,
    /// Text input events.
    pub text_events: Vec<TextInputEvent>,
    /// Whether the window is focused.
    pub window_focused: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton, PointerButtonState,
        PointerInput, TextInputEvent, UiInput,
    };
    use crate::geometry::{Point, Vec2};

    #[test]
    fn default_pointer_input_is_idle() {
        let input = PointerInput::default();

        assert_eq!(input.position, None);
        assert_eq!(input.delta, Vec2::ZERO);
        assert_eq!(input.wheel_delta, Vec2::ZERO);
        assert_eq!(
            input.button(MouseButton::Primary),
            PointerButtonState::default()
        );
        assert_eq!(input.click_count, 0);
    }

    #[test]
    fn pointer_button_state_tracks_edges() {
        let state = PointerButtonState::new(true, true, false);

        assert!(state.down);
        assert!(state.pressed);
        assert!(!state.released);
    }

    #[test]
    fn pointer_input_returns_known_button_states() {
        let input = PointerInput {
            position: Some(Point::new(10.0, 20.0)),
            primary: PointerButtonState::new(true, false, false),
            click_count: 2,
            ..PointerInput::default()
        };

        assert_eq!(input.position, Some(Point::new(10.0, 20.0)));
        assert!(input.button(MouseButton::Primary).down);
        assert_eq!(input.click_count, 2);
    }

    #[test]
    fn modifiers_report_empty_state() {
        assert!(Modifiers::default().is_empty());
        assert!(!Modifiers::new(false, true, false, false).is_empty());
    }

    #[test]
    fn keyboard_events_keep_text_input_separate() {
        let input = UiInput {
            keyboard: KeyboardInput {
                modifiers: Modifiers::new(false, true, false, false),
                events: vec![KeyEvent::new(
                    Key::Character("s".to_owned()),
                    KeyState::Pressed,
                    Modifiers::new(false, true, false, false),
                    false,
                )],
            },
            text_events: vec![TextInputEvent::Commit("s".to_owned())],
            window_focused: true,
            ..UiInput::default()
        };

        assert_eq!(input.keyboard.events.len(), 1);
        assert_eq!(input.text_events.len(), 1);
        assert!(input.window_focused);
    }
}
