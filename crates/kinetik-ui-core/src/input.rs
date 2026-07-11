//! Platform-independent input snapshots.

use std::collections::HashSet;

use crate::WidgetId;
use crate::geometry::{Point, Vec2};

const RELEASE_ALL_CANCEL_BUTTON: u16 = u16::MAX;

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
            MouseButton::Other(RELEASE_ALL_CANCEL_BUTTON) => PointerButtonState::default(),
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
            MouseButton::Other(RELEASE_ALL_CANCEL_BUTTON) => {}
            MouseButton::Other(number) => self.other_button_mut(number).set_down(down),
        }
    }

    /// Clears frame-local pointer deltas and button edges.
    pub fn begin_frame(&mut self) {
        self.other_buttons
            .retain(|(number, _)| *number != RELEASE_ALL_CANCEL_BUTTON);
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

    /// Releases all buttons and marks the frame as a release-all cancellation.
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
        self.mark_release_all_cancelled();
    }

    pub(crate) fn release_all_cancelled(&self) -> bool {
        self.other_buttons
            .iter()
            .any(|(number, state)| *number == RELEASE_ALL_CANCEL_BUTTON && state.released)
    }

    pub(crate) fn record_button_edge(&mut self, button: MouseButton, down: bool) {
        let state = match button {
            MouseButton::Primary => &mut self.primary,
            MouseButton::Secondary => &mut self.secondary,
            MouseButton::Middle => &mut self.middle,
            MouseButton::Other(RELEASE_ALL_CANCEL_BUTTON) => return,
            MouseButton::Other(number) => self.other_button_mut(number),
        };
        if down {
            state.pressed = true;
        } else {
            state.released = true;
        }
    }

    pub(crate) fn mark_release_all_cancelled(&mut self) {
        if let Some((_, state)) = self
            .other_buttons
            .iter_mut()
            .find(|(number, _)| *number == RELEASE_ALL_CANCEL_BUTTON)
        {
            *state = PointerButtonState::new(false, false, true);
        } else {
            self.other_buttons.push((
                RELEASE_ALL_CANCEL_BUTTON,
                PointerButtonState::new(false, false, true),
            ));
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
    /// Text produced by this hardware key after keyboard-layout processing.
    ///
    /// IME commits use [`TextInputEvent::Commit`] instead. Platform adapters
    /// suppress this field while an IME preedit is active.
    pub text: Option<String>,
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
            text: None,
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
            text: None,
        }
    }

    /// Adds layout-produced hardware text to this keyboard event.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.text = (!text.is_empty()).then_some(text);
        self
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

/// Provenance-preserving scroll delta carried by an ordered input event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputWheelDelta {
    /// Device-independent wheel lines.
    Lines(Vec2),
    /// Logical pixel delta.
    Pixels(Vec2),
}

impl InputWheelDelta {
    /// Returns the underlying two-dimensional delta.
    #[must_use]
    pub const fn value(self) -> Vec2 {
        match self {
            Self::Lines(delta) | Self::Pixels(delta) => delta,
        }
    }
}

/// One normalized platform event in event-time order.
#[derive(Debug, Clone, PartialEq)]
pub enum UiInputEvent {
    /// Pointer moved to an event-time position.
    PointerMoved {
        /// Position in the input's current logical coordinate scope.
        position: Point,
        /// Movement since the preceding platform pointer position.
        delta: Vec2,
    },
    /// Pointer left the window or current input surface.
    PointerLeft,
    /// Pointer button transition at the event-time pointer position.
    PointerButton {
        /// Button that changed state.
        button: MouseButton,
        /// Whether the button became down.
        down: bool,
        /// Platform-provided consecutive click count.
        click_count: u8,
        /// Event-time pointer position, when known.
        position: Option<Point>,
    },
    /// Cancel every retained pointer button at an event-time position.
    PointerReleaseAll {
        /// Event-time pointer position, when known.
        position: Option<Point>,
    },
    /// Scroll event with retained line or pixel provenance.
    Wheel {
        /// Typed scroll delta.
        delta: InputWheelDelta,
        /// Event-time pointer position, when known.
        position: Option<Point>,
    },
    /// Keyboard modifiers changed.
    ModifiersChanged(Modifiers),
    /// Physical/logical keyboard event, including optional hardware text.
    Key(KeyEvent),
    /// IME or committed text event.
    Text(TextInputEvent),
    /// Targeted clipboard result.
    ClipboardText(ClipboardText),
    /// Platform IME availability changed.
    ImeEnabled(bool),
    /// Window focus changed.
    WindowFocusChanged(bool),
}

/// Deterministic mismatch between the canonical stream and legacy projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputStreamConflict {
    /// Pointer movement, button edge, click, or wheel projection differs.
    Pointer,
    /// Keyboard-event projection differs.
    KeyboardEvents,
    /// Text-event projection differs.
    TextEvents,
    /// Targeted clipboard projection differs.
    ClipboardText,
    /// Retained keyboard modifier projection differs.
    Modifiers,
    /// Final focus state contradicts the ordered focus stream.
    WindowFocus,
}

/// Complete normalized input snapshot for one UI frame.
#[derive(Debug, Clone, PartialEq)]
pub struct UiInput {
    /// Authoritative ordered event stream for official input producers.
    pub events: Vec<UiInputEvent>,
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

impl Default for UiInput {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            pointer: PointerInput::default(),
            keyboard: KeyboardInput::default(),
            text_events: Vec::new(),
            clipboard_text: Vec::new(),
            window_focused: true,
        }
    }
}

impl UiInput {
    /// Appends one canonical event and updates its legacy snapshot projection.
    pub fn push_event(&mut self, event: UiInputEvent) {
        match &event {
            UiInputEvent::PointerMoved { position, delta } => {
                self.pointer.position = Some(*position);
                self.pointer.delta = add_vectors(self.pointer.delta, *delta);
            }
            UiInputEvent::PointerLeft => {
                self.pointer.position = None;
                self.pointer.delta = Vec2::ZERO;
            }
            UiInputEvent::PointerButton {
                button,
                down,
                click_count,
                position,
            } => {
                if let Some(position) = position {
                    self.pointer.position = Some(*position);
                }
                self.pointer.apply_button_transition(*button, *down);
                self.pointer.click_count = *click_count;
            }
            UiInputEvent::PointerReleaseAll { position } => {
                if let Some(position) = position {
                    self.pointer.position = Some(*position);
                }
                self.pointer.release_all_buttons();
            }
            UiInputEvent::Wheel { delta, position } => {
                if let Some(position) = position {
                    self.pointer.position = Some(*position);
                }
                self.pointer.wheel_delta = add_vectors(self.pointer.wheel_delta, delta.value());
            }
            UiInputEvent::ModifiersChanged(modifiers) => {
                self.keyboard.modifiers = *modifiers;
            }
            UiInputEvent::Key(event) => {
                self.keyboard.modifiers = event.modifiers;
                self.keyboard.events.push(event.clone());
            }
            UiInputEvent::Text(event) => self.text_events.push(event.clone()),
            UiInputEvent::ClipboardText(clipboard) => {
                self.clipboard_text.push(clipboard.clone());
            }
            UiInputEvent::ImeEnabled(_) => {}
            UiInputEvent::WindowFocusChanged(focused) => {
                self.window_focused = *focused;
            }
        }
        self.events.push(event);
    }

    /// Clears frame-local input while preserving retained down/focus state.
    pub fn begin_frame(&mut self) {
        self.events.clear();
        self.pointer.begin_frame();
        self.keyboard.events.clear();
        self.text_events.clear();
        self.clipboard_text.clear();
    }

    /// Releases all pointer buttons, intended for focus loss or input cancellation.
    pub fn release_pointer_buttons(&mut self) {
        self.push_event(UiInputEvent::PointerReleaseAll {
            position: self.pointer.position,
        });
    }

    /// Validates a non-empty canonical stream against all transient projections.
    ///
    /// Empty streams are the documented legacy snapshot compatibility path.
    ///
    /// # Errors
    ///
    /// Returns the first mismatch in a stable projection order.
    pub fn validate_event_stream(&self) -> Result<(), InputStreamConflict> {
        if self.events.is_empty() {
            return Ok(());
        }

        self.validate_text_event_stream()?;
        if !pointer_projection_matches(self) {
            return Err(InputStreamConflict::Pointer);
        }

        Ok(())
    }

    fn validate_text_event_stream(&self) -> Result<(), InputStreamConflict> {
        debug_assert!(!self.events.is_empty());

        let key_events = self
            .events
            .iter()
            .filter_map(|event| match event {
                UiInputEvent::Key(event) => Some(event.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if self.keyboard.events != key_events {
            return Err(InputStreamConflict::KeyboardEvents);
        }

        let text_events = self
            .events
            .iter()
            .filter_map(|event| match event {
                UiInputEvent::Text(event) => Some(event.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if self.text_events != text_events {
            return Err(InputStreamConflict::TextEvents);
        }

        let clipboard_text = self
            .events
            .iter()
            .filter_map(|event| match event {
                UiInputEvent::ClipboardText(clipboard) => Some(clipboard.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if self.clipboard_text != clipboard_text {
            return Err(InputStreamConflict::ClipboardText);
        }

        let mut projected_modifiers = None;
        let mut projected_focus = None;
        for event in &self.events {
            match event {
                UiInputEvent::ModifiersChanged(modifiers) => {
                    projected_modifiers = Some(*modifiers);
                }
                UiInputEvent::Key(event) => projected_modifiers = Some(event.modifiers),
                UiInputEvent::WindowFocusChanged(focused) => projected_focus = Some(*focused),
                _ => {}
            }
        }
        if projected_modifiers.is_some_and(|modifiers| modifiers != self.keyboard.modifiers) {
            return Err(InputStreamConflict::Modifiers);
        }
        if projected_focus.map_or(!self.window_focused, |focused| {
            focused != self.window_focused
        }) {
            return Err(InputStreamConflict::WindowFocus);
        }

        Ok(())
    }

    /// Returns the canonical stream or a deterministic legacy text-domain synthesis.
    ///
    /// Legacy synthesis preserves the pre-stream component order: focus-loss
    /// guard, clipboard shortcuts, text/IME events, targeted clipboard results,
    /// then remaining keyboard events. Pointer ordering cannot be recovered from
    /// an empty canonical stream.
    ///
    /// # Errors
    ///
    /// Returns a projection conflict for inconsistent mixed-mode input.
    pub fn effective_text_events(&self) -> Result<Vec<UiInputEvent>, InputStreamConflict> {
        if !self.events.is_empty() {
            self.validate_event_stream()?;
            return Ok(self.events.clone());
        }

        Ok(self.legacy_text_events())
    }

    pub(crate) fn effective_scoped_text_events(
        &self,
    ) -> Result<Vec<UiInputEvent>, InputStreamConflict> {
        if !self.events.is_empty() {
            self.validate_text_event_stream()?;
            return Ok(self.events.clone());
        }

        Ok(self.legacy_text_events())
    }

    fn legacy_text_events(&self) -> Vec<UiInputEvent> {
        let mut events = Vec::new();
        if !self.window_focused {
            events.push(UiInputEvent::WindowFocusChanged(false));
        }
        events.extend(
            self.keyboard
                .events
                .iter()
                .filter(|event| is_legacy_clipboard_shortcut(event))
                .cloned()
                .map(UiInputEvent::Key),
        );
        events.extend(self.text_events.iter().cloned().map(UiInputEvent::Text));
        events.extend(
            self.clipboard_text
                .iter()
                .cloned()
                .map(UiInputEvent::ClipboardText),
        );
        events.extend(
            self.keyboard
                .events
                .iter()
                .filter(|event| !is_legacy_clipboard_shortcut(event))
                .cloned()
                .map(UiInputEvent::Key),
        );
        events
    }
}

fn add_vectors(left: Vec2, right: Vec2) -> Vec2 {
    Vec2::new(left.x + right.x, left.y + right.y)
}

fn pointer_projection_matches(input: &UiInput) -> bool {
    let mut delta = Vec2::ZERO;
    let mut wheel = Vec2::ZERO;
    let mut click_count = 0;
    let mut saw_release_all = false;
    let mut buttons = HashSet::new();
    let mut position_evidence = None;

    for event in &input.events {
        match event {
            UiInputEvent::PointerMoved {
                position,
                delta: event_delta,
            } => {
                position_evidence = Some(Some(*position));
                delta = add_vectors(delta, *event_delta);
            }
            UiInputEvent::PointerLeft => {
                position_evidence = Some(None);
                delta = Vec2::ZERO;
            }
            UiInputEvent::PointerButton {
                button,
                click_count: event_click_count,
                position,
                ..
            } => {
                if let Some(position) = position {
                    position_evidence = Some(Some(*position));
                }
                buttons.insert(*button);
                click_count = *event_click_count;
            }
            UiInputEvent::PointerReleaseAll { position } => {
                if let Some(position) = position {
                    position_evidence = Some(Some(*position));
                }
                saw_release_all = true;
            }
            UiInputEvent::Wheel {
                delta: event_delta,
                position,
            } => {
                if let Some(position) = position {
                    position_evidence = Some(Some(*position));
                }
                wheel = add_vectors(wheel, event_delta.value());
            }
            _ => {}
        }
    }

    if input.pointer.delta != delta
        || input.pointer.wheel_delta != wheel
        || input.pointer.click_count != click_count
        || input.pointer.release_all_cancelled() != saw_release_all
        || position_evidence.is_some_and(|position| input.pointer.position != position)
    {
        return false;
    }

    buttons.extend(
        input
            .pointer
            .other_buttons
            .iter()
            .filter_map(|(number, _)| {
                (*number != RELEASE_ALL_CANCEL_BUTTON).then_some(MouseButton::Other(*number))
            }),
    );
    buttons.extend([
        MouseButton::Primary,
        MouseButton::Secondary,
        MouseButton::Middle,
    ]);
    buttons
        .into_iter()
        .all(|button| button_projection_matches(input, button))
}

fn button_projection_matches(input: &UiInput, button: MouseButton) -> bool {
    let state = input.pointer.button(button);
    let mut pressed = false;
    let mut released = false;
    let mut final_down = None;
    for event in &input.events {
        match event {
            UiInputEvent::PointerButton {
                button: event_button,
                down,
                ..
            } if *event_button == button => {
                pressed |= *down;
                released |= !*down;
                final_down = Some(*down);
            }
            UiInputEvent::PointerReleaseAll { .. } => {
                final_down = Some(false);
            }
            _ => {}
        }
    }

    state.pressed == pressed
        && (!state.released || released || input.pointer.release_all_cancelled())
        && (!released || state.released)
        && final_down.is_none_or(|down| state.down == down)
}

fn is_legacy_clipboard_shortcut(event: &KeyEvent) -> bool {
    if event.state != KeyState::Pressed
        || event.repeat
        || event.modifiers.alt
        || !(event.modifiers.ctrl || event.modifiers.super_key)
    {
        return false;
    }
    if let Key::Character(character) = &event.key
        && matches!(character.to_ascii_lowercase().as_str(), "c" | "x" | "v")
    {
        return true;
    }
    matches!(
        event.physical_key,
        PhysicalKey::KeyC | PhysicalKey::KeyX | PhysicalKey::KeyV
    )
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
            events: Vec::new(),
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

    #[test]
    fn ui_input_release_pointer_buttons_marks_release_all_cancellation() {
        let mut input = UiInput {
            pointer: PointerInput {
                primary: PointerButtonState::new(true, false, false),
                other_buttons: vec![(8, PointerButtonState::new(true, false, false))],
                ..PointerInput::default()
            },
            window_focused: true,
            ..UiInput::default()
        };

        input.release_pointer_buttons();

        assert!(!input.pointer.primary.down);
        assert!(input.pointer.primary.released);
        assert!(!input.pointer.secondary.down);
        assert!(!input.pointer.secondary.released);
        assert!(!input.pointer.middle.down);
        assert!(!input.pointer.middle.released);
        assert!(!input.pointer.button(MouseButton::Other(8)).down);
        assert!(input.pointer.button(MouseButton::Other(8)).released);
        assert_eq!(
            input
                .pointer
                .button(MouseButton::Other(super::RELEASE_ALL_CANCEL_BUTTON)),
            PointerButtonState::default()
        );
        assert!(input.pointer.release_all_cancelled());

        input.begin_frame();

        assert!(!input.pointer.primary.released);
        assert!(!input.pointer.release_all_cancelled());
    }
}
