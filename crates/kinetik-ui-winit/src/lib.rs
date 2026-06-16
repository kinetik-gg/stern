//! Winit platform adapter for Kinetik UI.

use std::time::Duration;

use kinetik_ui_core::{
    AccessibilitySnapshot, ClipboardText, CursorShape, FrameContext, FrameOutput, Key, KeyEvent,
    KeyState, KeyboardInput, Modifiers, MouseButton as CoreMouseButton, PhysicalKey, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, PointerInput, Rect, RepaintRequest, ScaleFactor,
    SemanticTreeError, Size, TextInputEvent, TextRange, TimeInfo, UiInput, Vec2, ViewportInfo,
    WidgetId,
};
use winit::dpi::{
    LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize as WinitPhysicalSize,
};
use winit::event::{ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{
    Key as WinitKey, KeyCode, ModifiersState, NamedKey, PhysicalKey as WinitPhysicalKey,
};
use winit::window::{CursorIcon, Window};

/// Window operations used by the platform request applier.
pub trait WinitWindowOps {
    /// Requests a redraw from the host window.
    fn request_redraw(&mut self);
    /// Applies a cursor icon to the host window.
    fn set_cursor(&mut self, cursor: CursorIcon);
    /// Applies the host window title.
    fn set_title(&mut self, title: &str);
    /// Enables or disables IME/text input.
    fn set_ime_allowed(&mut self, allowed: bool);
    /// Sets the IME composition/caret area in logical window coordinates.
    fn set_ime_cursor_area(&mut self, rect: Rect);
}

impl WinitWindowOps for &Window {
    fn request_redraw(&mut self) {
        Window::request_redraw(self);
    }

    fn set_cursor(&mut self, cursor: CursorIcon) {
        Window::set_cursor(self, cursor);
    }

    fn set_title(&mut self, title: &str) {
        Window::set_title(self, title);
    }

    fn set_ime_allowed(&mut self, allowed: bool) {
        Window::set_ime_allowed(self, allowed);
    }

    fn set_ime_cursor_area(&mut self, rect: Rect) {
        let rect = sanitize_rect_for_platform(rect);
        Window::set_ime_cursor_area(
            self,
            LogicalPosition::new(f64::from(rect.x), f64::from(rect.y)),
            LogicalSize::new(f64::from(rect.width), f64::from(rect.height)),
        );
    }
}

/// Requests that remain application/shell responsibilities after applying
/// window-local effects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WinitShellRequests {
    /// Text to write to the platform clipboard.
    pub clipboard_text: Option<String>,
    /// Text-input widget that should receive clipboard text read by the shell.
    pub request_clipboard_text: Option<WidgetId>,
    /// URLs the shell should open.
    pub open_urls: Vec<String>,
    /// Delay after which the shell should request another redraw.
    pub repaint_after: Option<Duration>,
    /// Whether the shell should keep requesting redraws.
    pub continuous_repaint: bool,
}

/// Text input request translated for a winit application shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WinitTextInputRequest {
    /// Start text input/IME at an optional logical rectangle.
    Start {
        /// Logical text-editing rectangle.
        rect: Option<Rect>,
    },
    /// Stop text input/IME.
    Stop,
}

/// Platform requests emitted by the adapter boundary.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WinitPlatformRequests {
    /// Cursor shape to apply to the window.
    pub cursor: CursorShape,
    /// Redraw scheduling request.
    pub repaint: RepaintRequest,
    /// Text to write to the platform clipboard.
    pub clipboard_text: Option<String>,
    /// Text-input widget that should receive clipboard text read by the shell.
    pub request_clipboard_text: Option<WidgetId>,
    /// Text input/IME request.
    pub text_input: Option<WinitTextInputRequest>,
    /// Window title to apply.
    pub window_title: Option<String>,
    /// URLs the app shell should open.
    pub open_urls: Vec<String>,
}

impl WinitPlatformRequests {
    /// Translates core frame output into winit-facing platform requests.
    #[must_use]
    pub fn from_frame_output(output: &FrameOutput) -> Self {
        let mut requests = Self::default();
        requests.apply_frame_output(output);
        requests
    }

    /// Applies all requests from a core frame output.
    pub fn apply_frame_output(&mut self, output: &FrameOutput) {
        self.request_repaint(output.repaint);
        for request in &output.platform_requests {
            self.apply_platform_request(request);
        }
    }

    /// Updates the repaint request using core repaint priority rules.
    pub fn request_repaint(&mut self, repaint: RepaintRequest) {
        self.repaint = self.repaint.merge(repaint);
    }

    /// Applies one platform request.
    pub fn apply_platform_request(&mut self, request: &PlatformRequest) {
        match request {
            PlatformRequest::SetCursor(cursor) => {
                self.cursor = *cursor;
            }
            PlatformRequest::CopyToClipboard(text) => {
                self.clipboard_text = Some(text.clone());
            }
            PlatformRequest::RequestClipboardText { target } => {
                self.request_clipboard_text = Some(*target);
            }
            PlatformRequest::StartTextInput { rect } => {
                self.text_input = Some(WinitTextInputRequest::Start { rect: *rect });
            }
            PlatformRequest::StopTextInput => {
                self.text_input = Some(WinitTextInputRequest::Stop);
            }
            PlatformRequest::SetWindowTitle(title) => {
                self.window_title = Some(title.clone());
            }
            PlatformRequest::OpenUrl(url) => {
                self.open_urls.push(url.clone());
            }
        }
    }

    /// Applies window-local requests to a real winit window and returns shell work.
    #[must_use]
    pub fn apply_to_window(&self, window: &Window) -> WinitShellRequests {
        let mut window = window;
        self.apply_to_window_ops(&mut window)
    }

    /// Applies window-local requests to a target and returns shell work.
    #[must_use]
    pub fn apply_to_window_ops(&self, window: &mut impl WinitWindowOps) -> WinitShellRequests {
        window.set_cursor(cursor_to_winit(self.cursor));
        if let Some(title) = &self.window_title {
            window.set_title(title);
        }

        let mut shell = WinitShellRequests {
            clipboard_text: self.clipboard_text.clone(),
            request_clipboard_text: self.request_clipboard_text,
            open_urls: self.open_urls.clone(),
            ..WinitShellRequests::default()
        };

        match self.repaint {
            RepaintRequest::None => {}
            RepaintRequest::NextFrame => window.request_redraw(),
            RepaintRequest::After(delay) => {
                shell.repaint_after = Some(delay);
            }
            RepaintRequest::Continuous => {
                shell.continuous_repaint = true;
                window.request_redraw();
            }
        }

        match self.text_input {
            Some(WinitTextInputRequest::Start { rect }) => {
                window.set_ime_allowed(true);
                if let Some(rect) = rect {
                    window.set_ime_cursor_area(sanitize_rect_for_platform(rect));
                }
            }
            Some(WinitTextInputRequest::Stop) => {
                window.set_ime_allowed(false);
            }
            None => {}
        }

        shell
    }
}

/// Accessibility update ready for a winit-hosted platform adapter.
///
/// This type is intentionally free of OS accessibility APIs. Application shells
/// can translate the snapshot into Windows, macOS, Linux, or test adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct WinitAccessibilityUpdate {
    /// Validated accessibility snapshot exported from the core frame.
    pub snapshot: AccessibilitySnapshot,
}

impl WinitAccessibilityUpdate {
    /// Translates core frame output into winit-facing accessibility data.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] when the frame's semantic tree is
    /// structurally invalid.
    pub fn from_frame_output(
        output: &FrameOutput,
        focused: Option<WidgetId>,
    ) -> Result<Self, SemanticTreeError> {
        output
            .accessibility_snapshot(focused)
            .map(|snapshot| Self { snapshot })
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
            self.last_pointer_position = None;
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

/// Converts a core cursor shape into a winit cursor icon.
#[must_use]
pub const fn cursor_to_winit(cursor: CursorShape) -> CursorIcon {
    match cursor {
        CursorShape::Default => CursorIcon::Default,
        CursorShape::Text => CursorIcon::Text,
        CursorShape::PointingHand => CursorIcon::Pointer,
        CursorShape::Crosshair => CursorIcon::Crosshair,
        CursorShape::Grab => CursorIcon::Grab,
        CursorShape::Grabbing => CursorIcon::Grabbing,
        CursorShape::ResizeHorizontal => CursorIcon::EwResize,
        CursorShape::ResizeVertical => CursorIcon::NsResize,
        CursorShape::ResizeTopLeftBottomRight => CursorIcon::NwseResize,
        CursorShape::ResizeTopRightBottomLeft => CursorIcon::NeswResize,
        CursorShape::NotAllowed => CursorIcon::NotAllowed,
    }
}

/// Converts a winit physical size and scale factor into viewport information.
#[must_use]
pub fn viewport_from_winit(size: WinitPhysicalSize<u32>, scale_factor: f64) -> ViewportInfo {
    let scale_factor = scale_factor_from_winit(scale_factor);
    let logical_width = f64::from(size.width) / scale_factor.value();
    let logical_height = f64::from(size.height) / scale_factor.value();

    ViewportInfo::new(
        Size::new(f64_to_f32(logical_width), f64_to_f32(logical_height)),
        PhysicalSize::new(size.width, size.height),
        scale_factor,
    )
}

/// Converts winit viewport data and a UI input snapshot into a full frame context.
#[must_use]
pub fn frame_context_from_winit(
    size: WinitPhysicalSize<u32>,
    scale_factor: f64,
    input: UiInput,
    time: TimeInfo,
) -> FrameContext {
    FrameContext::new(viewport_from_winit(size, scale_factor), input, time)
}

/// Converts a raw winit scale factor into a valid toolkit scale factor.
#[must_use]
pub fn scale_factor_from_winit(scale_factor: f64) -> ScaleFactor {
    sanitize_scale_factor(ScaleFactor::new(scale_factor))
}

/// Deterministic frame clock helper for winit application shells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WinitFrameClock {
    previous: Option<Duration>,
    frame_index: u64,
}

impl WinitFrameClock {
    /// Creates an empty frame clock.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            previous: None,
            frame_index: 0,
        }
    }

    /// Advances the clock and returns frame time information.
    ///
    /// The first frame reports a zero delta. If a later timestamp moves
    /// backwards, the delta is clamped to zero instead of underflowing.
    pub fn tick(&mut self, now: Duration) -> TimeInfo {
        let delta = self
            .previous
            .map_or(Duration::ZERO, |previous| now.saturating_sub(previous));
        let time = TimeInfo::new(now, delta, self.frame_index);
        self.previous = Some(now);
        self.frame_index = self.frame_index.saturating_add(1);
        time
    }

    /// Clears the previous timestamp and restarts frame numbering.
    pub fn reset(&mut self) {
        self.previous = None;
        self.frame_index = 0;
    }
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
        WinitKey::Named(NamedKey::Insert) => Key::Insert,
        WinitKey::Named(NamedKey::Home) => Key::Home,
        WinitKey::Named(NamedKey::End) => Key::End,
        WinitKey::Named(NamedKey::PageUp) => Key::PageUp,
        WinitKey::Named(NamedKey::PageDown) => Key::PageDown,
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

/// Converts a winit physical key into a core physical key.
#[must_use]
pub const fn physical_key_from_winit(physical_key: &WinitPhysicalKey) -> PhysicalKey {
    match physical_key {
        WinitPhysicalKey::Code(code) => physical_key_code_from_winit(*code),
        WinitPhysicalKey::Unidentified(_) => PhysicalKey::Unidentified,
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

#[allow(clippy::too_many_lines)]
const fn physical_key_code_from_winit(code: KeyCode) -> PhysicalKey {
    match code {
        KeyCode::KeyA => PhysicalKey::KeyA,
        KeyCode::KeyB => PhysicalKey::KeyB,
        KeyCode::KeyC => PhysicalKey::KeyC,
        KeyCode::KeyD => PhysicalKey::KeyD,
        KeyCode::KeyE => PhysicalKey::KeyE,
        KeyCode::KeyF => PhysicalKey::KeyF,
        KeyCode::KeyG => PhysicalKey::KeyG,
        KeyCode::KeyH => PhysicalKey::KeyH,
        KeyCode::KeyI => PhysicalKey::KeyI,
        KeyCode::KeyJ => PhysicalKey::KeyJ,
        KeyCode::KeyK => PhysicalKey::KeyK,
        KeyCode::KeyL => PhysicalKey::KeyL,
        KeyCode::KeyM => PhysicalKey::KeyM,
        KeyCode::KeyN => PhysicalKey::KeyN,
        KeyCode::KeyO => PhysicalKey::KeyO,
        KeyCode::KeyP => PhysicalKey::KeyP,
        KeyCode::KeyQ => PhysicalKey::KeyQ,
        KeyCode::KeyR => PhysicalKey::KeyR,
        KeyCode::KeyS => PhysicalKey::KeyS,
        KeyCode::KeyT => PhysicalKey::KeyT,
        KeyCode::KeyU => PhysicalKey::KeyU,
        KeyCode::KeyV => PhysicalKey::KeyV,
        KeyCode::KeyW => PhysicalKey::KeyW,
        KeyCode::KeyX => PhysicalKey::KeyX,
        KeyCode::KeyY => PhysicalKey::KeyY,
        KeyCode::KeyZ => PhysicalKey::KeyZ,
        KeyCode::Digit0 => PhysicalKey::Digit(0),
        KeyCode::Digit1 => PhysicalKey::Digit(1),
        KeyCode::Digit2 => PhysicalKey::Digit(2),
        KeyCode::Digit3 => PhysicalKey::Digit(3),
        KeyCode::Digit4 => PhysicalKey::Digit(4),
        KeyCode::Digit5 => PhysicalKey::Digit(5),
        KeyCode::Digit6 => PhysicalKey::Digit(6),
        KeyCode::Digit7 => PhysicalKey::Digit(7),
        KeyCode::Digit8 => PhysicalKey::Digit(8),
        KeyCode::Digit9 => PhysicalKey::Digit(9),
        KeyCode::Numpad0 => PhysicalKey::NumpadDigit(0),
        KeyCode::Numpad1 => PhysicalKey::NumpadDigit(1),
        KeyCode::Numpad2 => PhysicalKey::NumpadDigit(2),
        KeyCode::Numpad3 => PhysicalKey::NumpadDigit(3),
        KeyCode::Numpad4 => PhysicalKey::NumpadDigit(4),
        KeyCode::Numpad5 => PhysicalKey::NumpadDigit(5),
        KeyCode::Numpad6 => PhysicalKey::NumpadDigit(6),
        KeyCode::Numpad7 => PhysicalKey::NumpadDigit(7),
        KeyCode::Numpad8 => PhysicalKey::NumpadDigit(8),
        KeyCode::Numpad9 => PhysicalKey::NumpadDigit(9),
        KeyCode::Enter => PhysicalKey::Enter,
        KeyCode::NumpadEnter => PhysicalKey::NumpadEnter,
        KeyCode::Escape => PhysicalKey::Escape,
        KeyCode::Tab => PhysicalKey::Tab,
        KeyCode::Space => PhysicalKey::Space,
        KeyCode::Backspace => PhysicalKey::Backspace,
        KeyCode::Delete => PhysicalKey::Delete,
        KeyCode::Insert => PhysicalKey::Insert,
        KeyCode::Home => PhysicalKey::Home,
        KeyCode::End => PhysicalKey::End,
        KeyCode::PageUp => PhysicalKey::PageUp,
        KeyCode::PageDown => PhysicalKey::PageDown,
        KeyCode::ArrowLeft => PhysicalKey::ArrowLeft,
        KeyCode::ArrowRight => PhysicalKey::ArrowRight,
        KeyCode::ArrowUp => PhysicalKey::ArrowUp,
        KeyCode::ArrowDown => PhysicalKey::ArrowDown,
        KeyCode::F1 => PhysicalKey::Function(1),
        KeyCode::F2 => PhysicalKey::Function(2),
        KeyCode::F3 => PhysicalKey::Function(3),
        KeyCode::F4 => PhysicalKey::Function(4),
        KeyCode::F5 => PhysicalKey::Function(5),
        KeyCode::F6 => PhysicalKey::Function(6),
        KeyCode::F7 => PhysicalKey::Function(7),
        KeyCode::F8 => PhysicalKey::Function(8),
        KeyCode::F9 => PhysicalKey::Function(9),
        KeyCode::F10 => PhysicalKey::Function(10),
        KeyCode::F11 => PhysicalKey::Function(11),
        KeyCode::F12 => PhysicalKey::Function(12),
        KeyCode::Minus => PhysicalKey::Minus,
        KeyCode::Equal => PhysicalKey::Equal,
        KeyCode::BracketLeft => PhysicalKey::BracketLeft,
        KeyCode::BracketRight => PhysicalKey::BracketRight,
        KeyCode::Backslash => PhysicalKey::Backslash,
        KeyCode::Semicolon => PhysicalKey::Semicolon,
        KeyCode::Quote => PhysicalKey::Quote,
        KeyCode::Backquote => PhysicalKey::Backquote,
        KeyCode::Comma => PhysicalKey::Comma,
        KeyCode::Period => PhysicalKey::Period,
        KeyCode::Slash => PhysicalKey::Slash,
        KeyCode::NumpadAdd => PhysicalKey::NumpadAdd,
        KeyCode::NumpadSubtract => PhysicalKey::NumpadSubtract,
        KeyCode::NumpadMultiply => PhysicalKey::NumpadMultiply,
        KeyCode::NumpadDivide => PhysicalKey::NumpadDivide,
        KeyCode::NumpadDecimal => PhysicalKey::NumpadDecimal,
        KeyCode::ShiftLeft => PhysicalKey::ShiftLeft,
        KeyCode::ShiftRight => PhysicalKey::ShiftRight,
        KeyCode::ControlLeft => PhysicalKey::ControlLeft,
        KeyCode::ControlRight => PhysicalKey::ControlRight,
        KeyCode::AltLeft => PhysicalKey::AltLeft,
        KeyCode::AltRight => PhysicalKey::AltRight,
        KeyCode::SuperLeft => PhysicalKey::SuperLeft,
        KeyCode::SuperRight => PhysicalKey::SuperRight,
        _ => PhysicalKey::Unidentified,
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

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
}

fn f32_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_scale_factor(scale_factor: ScaleFactor) -> ScaleFactor {
    if scale_factor.is_valid() {
        scale_factor
    } else {
        ScaleFactor::ONE
    }
}

fn sanitize_rect_for_platform(rect: Rect) -> Rect {
    Rect::new(
        f32_or_zero(rect.x),
        f32_or_zero(rect.y),
        if rect.width.is_finite() {
            rect.width.max(0.0)
        } else {
            0.0
        },
        if rect.height.is_finite() {
            rect.height.max(0.0)
        } else {
            0.0
        },
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{
        WinitAccessibilityUpdate, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
        WinitTextInputRequest, WinitWindowOps, cursor_to_winit, frame_context_from_winit,
        key_from_winit, modifiers_from_winit, physical_key_from_winit, scale_factor_from_winit,
        viewport_from_winit,
    };
    use kinetik_ui_core::{
        ClipboardText, CursorShape, FrameOutput, Key, KeyState, Modifiers,
        MouseButton as CoreMouseButton, PhysicalKey, PlatformRequest, Rect, RepaintRequest,
        ScaleFactor, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
        SemanticTreeError, SemanticValue, TextInputEvent, TextRange, TimeInfo, UiInput, WidgetId,
    };
    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::event::{ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta};
    use winit::keyboard::{
        Key as WinitKey, KeyCode, ModifiersState, NamedKey, PhysicalKey as WinitPhysicalKey,
    };
    use winit::window::CursorIcon;

    #[derive(Debug, Default, PartialEq)]
    struct FakeWindow {
        redraws: usize,
        cursor: Option<CursorIcon>,
        title: Option<String>,
        ime_allowed: Option<bool>,
        ime_rect: Option<Rect>,
    }

    impl WinitWindowOps for FakeWindow {
        fn request_redraw(&mut self) {
            self.redraws += 1;
        }

        fn set_cursor(&mut self, cursor: CursorIcon) {
            self.cursor = Some(cursor);
        }

        fn set_title(&mut self, title: &str) {
            self.title = Some(title.to_owned());
        }

        fn set_ime_allowed(&mut self, allowed: bool) {
            self.ime_allowed = Some(allowed);
        }

        fn set_ime_cursor_area(&mut self, rect: Rect) {
            self.ime_rect = Some(rect);
        }
    }

    #[test]
    fn viewport_conversion_uses_logical_units() {
        let viewport = viewport_from_winit(PhysicalSize::new(1920, 1080), 2.0);

        assert_eq!(viewport.logical_size.width, 960.0);
        assert_eq!(viewport.logical_size.height, 540.0);
        assert_eq!(viewport.physical_size.width, 1920);
        assert_eq!(viewport.scale_factor, ScaleFactor::new(2.0));
    }

    #[test]
    fn viewport_conversion_sanitizes_invalid_scale_factor() {
        let viewport = viewport_from_winit(PhysicalSize::new(1920, 1080), f64::NAN);

        assert_eq!(viewport.logical_size.width, 1920.0);
        assert_eq!(viewport.logical_size.height, 1080.0);
        assert_eq!(viewport.scale_factor, ScaleFactor::ONE);
        assert_eq!(scale_factor_from_winit(0.0), ScaleFactor::ONE);
    }

    #[test]
    fn frame_context_from_winit_combines_viewport_input_and_time() {
        let input = UiInput {
            window_focused: true,
            ..UiInput::default()
        };
        let time = TimeInfo::new(
            core::time::Duration::from_millis(32),
            core::time::Duration::from_millis(16),
            2,
        );

        let context = frame_context_from_winit(PhysicalSize::new(1280, 720), 2.0, input, time);

        assert_eq!(context.viewport.logical_size.width, 640.0);
        assert!(context.input.window_focused);
        assert_eq!(context.time.frame_index, 2);
    }

    #[test]
    fn frame_clock_reports_delta_and_clamps_backwards_time() {
        let mut clock = WinitFrameClock::new();

        let first = clock.tick(core::time::Duration::from_millis(20));
        let second = clock.tick(core::time::Duration::from_millis(36));
        let backwards = clock.tick(core::time::Duration::from_millis(30));

        assert_eq!(first.delta, core::time::Duration::ZERO);
        assert_eq!(first.frame_index, 0);
        assert_eq!(second.delta, core::time::Duration::from_millis(16));
        assert_eq!(second.frame_index, 1);
        assert_eq!(backwards.delta, core::time::Duration::ZERO);
        assert_eq!(backwards.frame_index, 2);

        clock.reset();
        assert_eq!(
            clock.tick(core::time::Duration::from_millis(1)).frame_index,
            0
        );
    }

    #[test]
    fn pointer_conversion_tracks_position_delta_button_and_wheel() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::new(2.0));

        adapter.pointer_moved(PhysicalPosition::new(20.0, 10.0));
        adapter.pointer_moved(PhysicalPosition::new(24.0, 16.0));
        adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
        adapter.mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));

        let input = adapter.input();
        assert_eq!(input.pointer.position.expect("position").x, 12.0);
        assert_eq!(input.pointer.delta.x, 2.0);
        assert!(input.pointer.primary.down);
        assert!(input.pointer.primary.pressed);
        assert_eq!(input.pointer.wheel_delta.y, -1.0);
    }

    #[test]
    fn pointer_and_wheel_conversion_sanitize_non_finite_platform_values() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::new(f64::NAN));

        adapter.pointer_moved(PhysicalPosition::new(f64::INFINITY, f64::NAN));
        adapter.mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            f64::NAN,
            f64::INFINITY,
        )));

        let input = adapter.input();
        assert_eq!(input.pointer.position.expect("position").x, 0.0);
        assert_eq!(input.pointer.position.expect("position").y, 0.0);
        assert_eq!(input.pointer.wheel_delta.x, 0.0);
        assert_eq!(input.pointer.wheel_delta.y, 0.0);
    }

    #[test]
    fn begin_frame_clears_transient_input() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
        adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
        adapter.text_input("a");

        adapter.begin_frame();

        assert!(adapter.input().pointer.primary.down);
        assert!(!adapter.input().pointer.primary.pressed);
        assert!(adapter.input().text_events.is_empty());
    }

    #[test]
    fn mouse_button_transitions_preserve_same_frame_edges_and_other_buttons() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

        adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
        adapter.mouse_button(WinitMouseButton::Left, ElementState::Released, 1);
        adapter.mouse_button(WinitMouseButton::Other(8), ElementState::Pressed, 1);

        assert!(!adapter.input().pointer.primary.down);
        assert!(adapter.input().pointer.primary.pressed);
        assert!(adapter.input().pointer.primary.released);
        assert!(
            adapter
                .input()
                .pointer
                .button(CoreMouseButton::Other(8))
                .down
        );
    }

    #[test]
    fn losing_window_focus_releases_pressed_buttons() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

        adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
        adapter.begin_frame();
        adapter.set_window_focused(false);

        assert!(!adapter.input().pointer.primary.down);
        assert!(adapter.input().pointer.primary.released);
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
    fn keyboard_conversion_preserves_physical_key() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

        adapter.keyboard_event_with_physical_key(
            &WinitKey::Character("z".into()),
            &WinitPhysicalKey::Code(KeyCode::KeyY),
            ElementState::Pressed,
            ModifiersState::empty(),
            false,
        );

        let event = &adapter.input().keyboard.events[0];
        assert_eq!(event.key, Key::Character("z".to_owned()));
        assert_eq!(event.physical_key, PhysicalKey::KeyY);
    }

    #[test]
    fn ime_events_preserve_lifecycle_and_selection() {
        let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

        adapter.ime_event(Ime::Enabled);
        adapter.ime_event(Ime::Preedit("compose".to_owned(), Some((1, 4))));
        adapter.ime_event(Ime::Commit("done".to_owned()));
        adapter.ime_event(Ime::Disabled);

        assert_eq!(
            adapter.input().text_events[0],
            TextInputEvent::CompositionStart
        );
        assert_eq!(
            adapter.input().text_events[1],
            TextInputEvent::Composition {
                text: "compose".to_owned(),
                selection: Some(TextRange::new(1, 4)),
            }
        );
        assert_eq!(
            adapter.input().text_events[2],
            TextInputEvent::Commit("done".to_owned())
        );
        assert_eq!(
            adapter.input().text_events[3],
            TextInputEvent::CompositionEnd
        );
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
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::Home)), Key::Home);
        assert_eq!(
            physical_key_from_winit(&WinitPhysicalKey::Code(KeyCode::Digit7)),
            PhysicalKey::Digit(7)
        );
    }

    #[test]
    fn cursor_and_redraw_requests_are_represented() {
        let mut requests = WinitPlatformRequests {
            cursor: CursorShape::Text,
            repaint: RepaintRequest::After(core::time::Duration::from_secs(5)),
            ..WinitPlatformRequests::default()
        };

        requests.request_repaint(RepaintRequest::NextFrame);

        assert_eq!(
            cursor_to_winit(requests.cursor),
            winit::window::CursorIcon::Text
        );
        assert_eq!(
            cursor_to_winit(CursorShape::PointingHand),
            winit::window::CursorIcon::Pointer
        );
        assert_eq!(requests.repaint, RepaintRequest::NextFrame);
    }

    #[test]
    fn frame_output_platform_requests_translate_to_winit_request_data() {
        let mut output = FrameOutput::new();
        let text_rect = Rect::new(10.0, 20.0, 100.0, 24.0);
        let text_target = WidgetId::from_key("field");
        output.request_repaint(RepaintRequest::After(core::time::Duration::from_millis(20)));
        output.push_platform_request(PlatformRequest::SetCursor(CursorShape::Text));
        output.push_platform_request(PlatformRequest::CopyToClipboard("copy".to_owned()));
        output.push_platform_request(PlatformRequest::RequestClipboardText {
            target: text_target,
        });
        output.push_platform_request(PlatformRequest::StartTextInput {
            rect: Some(text_rect),
        });
        output.push_platform_request(PlatformRequest::SetWindowTitle("Kinetik".to_owned()));
        output.push_platform_request(PlatformRequest::OpenUrl("https://example.com".to_owned()));

        let requests = WinitPlatformRequests::from_frame_output(&output);

        assert_eq!(requests.cursor, CursorShape::Text);
        assert_eq!(
            requests.repaint,
            RepaintRequest::After(core::time::Duration::from_millis(20))
        );
        assert_eq!(requests.clipboard_text, Some("copy".to_owned()));
        assert_eq!(requests.request_clipboard_text, Some(text_target));
        assert_eq!(
            requests.text_input,
            Some(WinitTextInputRequest::Start {
                rect: Some(text_rect)
            })
        );
        assert_eq!(requests.window_title, Some("Kinetik".to_owned()));
        assert_eq!(requests.open_urls, vec!["https://example.com".to_owned()]);
    }

    #[test]
    fn frame_output_accessibility_update_preserves_semantic_data() {
        let mut output = FrameOutput::new();
        let root = WidgetId::from_key("root");
        let button = WidgetId::from_key("button");
        let slider = WidgetId::from_key("slider");
        output.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([button, slider]),
        );
        output.push_semantic_node(
            SemanticNode::new(
                button,
                SemanticRole::Button,
                Rect::new(0.0, 0.0, 80.0, 28.0),
            )
            .focusable(true)
            .with_label("Run")
            .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Run")),
        );
        let mut slider_node = SemanticNode::new(
            slider,
            SemanticRole::Slider,
            Rect::new(0.0, 32.0, 120.0, 18.0),
        )
        .focusable(true)
        .with_label("Opacity")
        .with_action(SemanticAction::new(
            SemanticActionKind::Increment,
            "Increase",
        ));
        slider_node.state.value = Some(SemanticValue::Number {
            current: 0.5,
            min: 0.0,
            max: 1.0,
        });
        output.push_semantic_node(slider_node);

        let update =
            WinitAccessibilityUpdate::from_frame_output(&output, Some(button)).expect("update");
        let snapshot = update.snapshot;

        assert_eq!(snapshot.root, Some(root));
        assert_eq!(
            snapshot
                .nodes
                .iter()
                .map(|node| node.id)
                .collect::<Vec<_>>(),
            vec![root, button, slider]
        );
        assert_eq!(snapshot.focus_order, vec![button, slider]);
        assert_eq!(snapshot.focused, Some(button));
        assert_eq!(
            snapshot.node(button).expect("button").label.as_deref(),
            Some("Run")
        );
        assert_eq!(
            snapshot.node(slider).expect("slider").state.value,
            Some(SemanticValue::Number {
                current: 0.5,
                min: 0.0,
                max: 1.0,
            })
        );
        assert!(
            snapshot
                .node(slider)
                .expect("slider")
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Increment)
        );
    }

    #[test]
    fn frame_output_accessibility_update_reports_invalid_semantics_without_os_services() {
        let mut output = FrameOutput::new();
        let root = WidgetId::from_key("root");
        let missing = WidgetId::from_key("missing");
        output.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]),
        );

        assert_eq!(
            WinitAccessibilityUpdate::from_frame_output(&output, None).expect_err("error"),
            SemanticTreeError::UnknownChild {
                parent: root,
                child: missing,
            }
        );
    }

    #[test]
    fn stop_text_input_overrides_start_request() {
        let mut output = FrameOutput::new();
        output.push_platform_request(PlatformRequest::StartTextInput { rect: None });
        output.push_platform_request(PlatformRequest::StopTextInput);

        let requests = WinitPlatformRequests::from_frame_output(&output);

        assert_eq!(requests.text_input, Some(WinitTextInputRequest::Stop));
    }

    #[test]
    fn platform_requests_apply_window_effects_and_return_shell_work() {
        let text_rect = Rect::new(10.0, 20.0, 100.0, 24.0);
        let text_target = WidgetId::from_key("field");
        let requests = WinitPlatformRequests {
            cursor: CursorShape::Text,
            repaint: RepaintRequest::Continuous,
            clipboard_text: Some("copy".to_owned()),
            request_clipboard_text: Some(text_target),
            text_input: Some(WinitTextInputRequest::Start {
                rect: Some(text_rect),
            }),
            window_title: Some("Kinetik".to_owned()),
            open_urls: vec!["https://example.com".to_owned()],
        };
        let mut window = FakeWindow::default();

        let shell = requests.apply_to_window_ops(&mut window);

        assert_eq!(window.redraws, 1);
        assert_eq!(window.cursor, Some(CursorIcon::Text));
        assert_eq!(window.title, Some("Kinetik".to_owned()));
        assert_eq!(window.ime_allowed, Some(true));
        assert_eq!(window.ime_rect, Some(text_rect));
        assert_eq!(shell.clipboard_text, Some("copy".to_owned()));
        assert_eq!(shell.request_clipboard_text, Some(text_target));
        assert_eq!(shell.open_urls, vec!["https://example.com".to_owned()]);
        assert!(shell.continuous_repaint);
    }

    #[test]
    fn adapter_feeds_targeted_clipboard_text_into_input() {
        let target = WidgetId::from_key("field");
        let mut adapter = WinitInputAdapter::default();

        adapter.clipboard_text(target, "pasted");
        assert_eq!(
            adapter.input().clipboard_text,
            &[ClipboardText::new(target, "pasted")]
        );

        adapter.begin_frame();
        assert!(adapter.input().clipboard_text.is_empty());
    }

    #[test]
    fn platform_text_input_rects_are_sanitized_for_window_ops() {
        let requests = WinitPlatformRequests {
            text_input: Some(WinitTextInputRequest::Start {
                rect: Some(Rect::new(f32::NAN, f32::INFINITY, -10.0, f32::NAN)),
            }),
            ..WinitPlatformRequests::default()
        };
        let mut window = FakeWindow::default();

        let _ = requests.apply_to_window_ops(&mut window);

        assert_eq!(window.ime_allowed, Some(true));
        assert_eq!(window.ime_rect, Some(Rect::new(0.0, 0.0, 0.0, 0.0)));
    }

    #[test]
    fn delayed_repaint_is_returned_to_shell_without_immediate_redraw() {
        let requests = WinitPlatformRequests {
            repaint: RepaintRequest::After(core::time::Duration::from_millis(15)),
            ..WinitPlatformRequests::default()
        };
        let mut window = FakeWindow::default();

        let shell = requests.apply_to_window_ops(&mut window);

        assert_eq!(window.redraws, 0);
        assert_eq!(
            shell.repaint_after,
            Some(core::time::Duration::from_millis(15))
        );
    }
}
