//! Platform-independent input snapshots.

use crate::WidgetId;
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

    /// Records a press transition while preserving any release already seen this frame.
    pub fn press(&mut self) {
        self.down = true;
        self.pressed = true;
    }

    /// Records a release transition while preserving any press already seen this frame.
    pub fn release(&mut self) {
        self.down = false;
        self.released = true;
    }

    /// Applies a down/up transition.
    pub fn set_down(&mut self, down: bool) {
        if down {
            self.press();
        } else {
            self.release();
        }
    }

    /// Clears frame-local edge flags while preserving the current down state.
    pub fn clear_edges(&mut self) {
        self.pressed = false;
        self.released = false;
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
    /// Additional pointer button states keyed by platform-independent button number.
    pub other_buttons: Vec<(u16, PointerButtonState)>,
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
            other_buttons: Vec::new(),
            click_count: 0,
        }
    }
}

impl PointerInput {
    /// Returns the state for a mouse button.
    #[must_use]
    pub fn button(&self, button: MouseButton) -> PointerButtonState {
        match button {
            MouseButton::Primary => self.primary,
            MouseButton::Secondary => self.secondary,
            MouseButton::Middle => self.middle,
            MouseButton::Other(number) => self
                .other_buttons
                .iter()
                .find(|(candidate, _)| *candidate == number)
                .map_or_else(PointerButtonState::default, |(_, state)| *state),
        }
    }

    /// Applies a press/release transition for a pointer button.
    pub fn apply_button_transition(&mut self, button: MouseButton, down: bool) {
        match button {
            MouseButton::Primary => self.primary.set_down(down),
            MouseButton::Secondary => self.secondary.set_down(down),
            MouseButton::Middle => self.middle.set_down(down),
            MouseButton::Other(number) => self.other_button_mut(number).set_down(down),
        }
    }

    /// Clears frame-local pointer deltas and button edges.
    pub fn begin_frame(&mut self) {
        self.delta = Vec2::ZERO;
        self.wheel_delta = Vec2::ZERO;
        self.primary.clear_edges();
        self.secondary.clear_edges();
        self.middle.clear_edges();
        for (_, state) in &mut self.other_buttons {
            state.clear_edges();
        }
        self.click_count = 0;
    }

    /// Releases all buttons, preserving release edges for buttons that were down.
    pub fn release_all_buttons(&mut self) {
        if self.primary.down {
            self.primary.release();
        }
        if self.secondary.down {
            self.secondary.release();
        }
        if self.middle.down {
            self.middle.release();
        }
        for (_, state) in &mut self.other_buttons {
            if state.down {
                state.release();
            }
        }
    }

    fn other_button_mut(&mut self, number: u16) -> &mut PointerButtonState {
        if let Some(index) = self
            .other_buttons
            .iter()
            .position(|(candidate, _)| *candidate == number)
        {
            &mut self.other_buttons[index].1
        } else {
            let index = self.other_buttons.len();
            self.other_buttons
                .push((number, PointerButtonState::default()));
            &mut self.other_buttons[index].1
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
    /// Insert.
    Insert,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
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

/// Platform-independent physical key identity before keyboard-layout resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalKey {
    /// Letter key position A-Z.
    KeyA,
    /// Letter key position A-Z.
    KeyB,
    /// Letter key position A-Z.
    KeyC,
    /// Letter key position A-Z.
    KeyD,
    /// Letter key position A-Z.
    KeyE,
    /// Letter key position A-Z.
    KeyF,
    /// Letter key position A-Z.
    KeyG,
    /// Letter key position A-Z.
    KeyH,
    /// Letter key position A-Z.
    KeyI,
    /// Letter key position A-Z.
    KeyJ,
    /// Letter key position A-Z.
    KeyK,
    /// Letter key position A-Z.
    KeyL,
    /// Letter key position A-Z.
    KeyM,
    /// Letter key position A-Z.
    KeyN,
    /// Letter key position A-Z.
    KeyO,
    /// Letter key position A-Z.
    KeyP,
    /// Letter key position A-Z.
    KeyQ,
    /// Letter key position A-Z.
    KeyR,
    /// Letter key position A-Z.
    KeyS,
    /// Letter key position A-Z.
    KeyT,
    /// Letter key position A-Z.
    KeyU,
    /// Letter key position A-Z.
    KeyV,
    /// Letter key position A-Z.
    KeyW,
    /// Letter key position A-Z.
    KeyX,
    /// Letter key position A-Z.
    KeyY,
    /// Letter key position A-Z.
    KeyZ,
    /// Digit row key position 0-9.
    Digit(u8),
    /// Numpad digit key position 0-9.
    NumpadDigit(u8),
    /// Enter/Return key position.
    Enter,
    /// Numpad Enter key position.
    NumpadEnter,
    /// Escape key position.
    Escape,
    /// Tab key position.
    Tab,
    /// Space key position.
    Space,
    /// Backspace key position.
    Backspace,
    /// Delete key position.
    Delete,
    /// Insert key position.
    Insert,
    /// Home key position.
    Home,
    /// End key position.
    End,
    /// Page Up key position.
    PageUp,
    /// Page Down key position.
    PageDown,
    /// Arrow left key position.
    ArrowLeft,
    /// Arrow right key position.
    ArrowRight,
    /// Arrow up key position.
    ArrowUp,
    /// Arrow down key position.
    ArrowDown,
    /// Function key position by number.
    Function(u8),
    /// Minus key position.
    Minus,
    /// Equal key position.
    Equal,
    /// Left bracket key position.
    BracketLeft,
    /// Right bracket key position.
    BracketRight,
    /// Backslash key position.
    Backslash,
    /// Semicolon key position.
    Semicolon,
    /// Quote key position.
    Quote,
    /// Backquote key position.
    Backquote,
    /// Comma key position.
    Comma,
    /// Period key position.
    Period,
    /// Slash key position.
    Slash,
    /// Numpad add key position.
    NumpadAdd,
    /// Numpad subtract key position.
    NumpadSubtract,
    /// Numpad multiply key position.
    NumpadMultiply,
    /// Numpad divide key position.
    NumpadDivide,
    /// Numpad decimal key position.
    NumpadDecimal,
    /// Left Shift key position.
    ShiftLeft,
    /// Right Shift key position.
    ShiftRight,
    /// Left Control key position.
    ControlLeft,
    /// Right Control key position.
    ControlRight,
    /// Left Alt key position.
    AltLeft,
    /// Right Alt key position.
    AltRight,
    /// Left Super/Command/Windows key position.
    SuperLeft,
    /// Right Super/Command/Windows key position.
    SuperRight,
    /// A physical key that has not been mapped yet.
    Unidentified,
}

/// A keyboard event received during the current frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// Key identity.
    pub key: Key,
    /// Physical key identity before keyboard-layout resolution.
    pub physical_key: PhysicalKey,
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
            physical_key: PhysicalKey::Unidentified,
            state,
            modifiers,
            repeat,
        }
    }

    /// Creates a keyboard event with a physical key identity.
    #[must_use]
    pub const fn with_physical_key(
        key: Key,
        physical_key: PhysicalKey,
        state: KeyState,
        modifiers: Modifiers,
        repeat: bool,
    ) -> Self {
        Self {
            key,
            physical_key,
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

/// UTF-8 byte range used by text composition selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset.
    pub end: usize,
}

impl TextRange {
    /// Creates a text range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Text input event separated from shortcut-oriented keyboard events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
    /// Text composition/IME became active.
    CompositionStart,
    /// Text composition update for IME-like input paths.
    Composition {
        /// Current preedit text.
        text: String,
        /// Optional selected byte range inside the preedit text.
        selection: Option<TextRange>,
    },
    /// Committed text input.
    Commit(String),
    /// Text composition/IME ended.
    CompositionEnd,
}

/// Clipboard text returned by the platform for a specific text-editing widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardText {
    /// Text-input widget that requested the clipboard contents.
    pub target: WidgetId,
    /// Clipboard text snapshot.
    pub text: String,
}

impl ClipboardText {
    /// Creates targeted clipboard text input.
    #[must_use]
    pub fn new(target: WidgetId, text: impl Into<String>) -> Self {
        Self {
            target,
            text: text.into(),
        }
    }
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
    /// Clipboard text returned by a platform adapter.
    pub clipboard_text: Vec<ClipboardText>,
    /// Whether the window is focused.
    pub window_focused: bool,
}

impl UiInput {
    /// Clears frame-local input while preserving retained down/focus state.
    pub fn begin_frame(&mut self) {
        self.pointer.begin_frame();
        self.keyboard.events.clear();
        self.text_events.clear();
        self.clipboard_text.clear();
    }

    /// Releases pressed pointer buttons, intended for window focus loss.
    pub fn release_pointer_buttons(&mut self) {
        self.pointer.release_all_buttons();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClipboardText, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton, PhysicalKey,
        PointerButtonState, PointerInput, TextInputEvent, TextRange, UiInput,
    };
    use crate::WidgetId;
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
        assert!(input.other_buttons.is_empty());
        assert_eq!(input.click_count, 0);
    }

    #[test]
    fn pointer_button_state_tracks_edges() {
        let mut state = PointerButtonState::new(false, false, false);

        state.press();
        assert!(state.down);
        assert!(state.pressed);
        assert!(!state.released);

        state.release();
        assert!(!state.down);
        assert!(state.pressed);
        assert!(state.released);

        state.clear_edges();
        assert!(!state.pressed);
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
    fn pointer_input_preserves_same_frame_button_edges() {
        let mut input = PointerInput::default();

        input.apply_button_transition(MouseButton::Primary, true);
        input.apply_button_transition(MouseButton::Primary, false);
        input.apply_button_transition(MouseButton::Other(8), true);

        assert!(!input.primary.down);
        assert!(input.primary.pressed);
        assert!(input.primary.released);
        assert!(input.button(MouseButton::Other(8)).down);

        input.begin_frame();

        assert!(!input.primary.pressed);
        assert!(!input.primary.released);
        assert!(input.button(MouseButton::Other(8)).down);
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
            clipboard_text: vec![ClipboardText::new(WidgetId::from_key("field"), "clip")],
            window_focused: true,
            ..UiInput::default()
        };

        assert_eq!(input.keyboard.events.len(), 1);
        assert_eq!(
            input.keyboard.events[0].physical_key,
            PhysicalKey::Unidentified
        );
        assert_eq!(input.text_events.len(), 1);
        assert_eq!(input.clipboard_text.len(), 1);
        assert!(input.window_focused);
    }

    #[test]
    fn keyboard_event_can_carry_physical_key_identity() {
        let event = KeyEvent::with_physical_key(
            Key::Character("z".to_owned()),
            PhysicalKey::KeyY,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        );

        assert_eq!(event.key, Key::Character("z".to_owned()));
        assert_eq!(event.physical_key, PhysicalKey::KeyY);
    }

    #[test]
    fn text_events_express_composition_lifecycle() {
        let events = [
            TextInputEvent::CompositionStart,
            TextInputEvent::Composition {
                text: "kana".to_owned(),
                selection: Some(TextRange::new(1, 3)),
            },
            TextInputEvent::Commit("かな".to_owned()),
            TextInputEvent::CompositionEnd,
        ];

        assert_eq!(events.len(), 4);
        assert!(matches!(
            &events[1],
            TextInputEvent::Composition {
                selection: Some(TextRange { start: 1, end: 3 }),
                ..
            }
        ));
    }

    #[test]
    fn ui_input_begin_frame_clears_transient_events() {
        let mut input = UiInput {
            pointer: PointerInput {
                position: Some(Point::new(1.0, 1.0)),
                delta: Vec2::new(2.0, 3.0),
                primary: PointerButtonState::new(true, true, false),
                ..PointerInput::default()
            },
            keyboard: KeyboardInput {
                modifiers: Modifiers::default(),
                events: vec![KeyEvent::new(
                    Key::Enter,
                    KeyState::Pressed,
                    Modifiers::default(),
                    false,
                )],
            },
            text_events: vec![TextInputEvent::Commit("x".to_owned())],
            clipboard_text: vec![ClipboardText::new(WidgetId::from_key("field"), "paste")],
            window_focused: true,
        };

        input.begin_frame();

        assert_eq!(input.pointer.position, Some(Point::new(1.0, 1.0)));
        assert!(input.pointer.primary.down);
        assert_eq!(input.pointer.delta, Vec2::ZERO);
        assert!(input.keyboard.events.is_empty());
        assert!(input.text_events.is_empty());
        assert!(input.clipboard_text.is_empty());
    }
}
