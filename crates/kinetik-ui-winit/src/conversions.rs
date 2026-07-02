use kinetik_ui_core::{CursorShape, Key, Modifiers, PhysicalKey};
use winit::keyboard::{
    Key as WinitKey, KeyCode, ModifiersState, NamedKey, PhysicalKey as WinitPhysicalKey,
};
use winit::window::CursorIcon;
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
