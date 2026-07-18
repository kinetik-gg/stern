//! Pure, platform-selected shortcut presentation.

use crate::{Key, Modifiers, PhysicalKey, Shortcut};

/// Platform policy used to order and name shortcut tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutPlatform {
    /// Microsoft Windows presentation policy.
    Windows,
    /// Apple macOS presentation policy.
    MacOs,
    /// Linux desktop presentation policy.
    Linux,
}

/// Platform-independent modifier identity supplied to a label localizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutModifier {
    /// Control modifier.
    Control,
    /// Alt or Option modifier.
    Alt,
    /// Shift modifier.
    Shift,
    /// Super, Command, or Windows modifier.
    Super,
}

/// One borrowed shortcut token requiring a presentation label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutLabelToken<'a> {
    /// Active modifier identity.
    Modifier(ShortcutModifier),
    /// Layout-resolved logical key identity.
    LogicalKey(&'a Key),
    /// Layout-independent physical key identity.
    PhysicalKey(PhysicalKey),
}

/// Supplies localized labels and joining policy for shortcut tokens.
pub trait ShortcutLabelLocalizer {
    /// Returns the complete display label for one required token.
    fn token_label(
        &self,
        platform: ShortcutPlatform,
        token: ShortcutLabelToken<'_>,
    ) -> Option<String>;

    /// Returns the separator placed between complete token labels.
    fn separator(&self, platform: ShortcutPlatform) -> &str;
}

/// Deterministic English reference labels for shortcut presentation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnglishShortcutLabels;

impl ShortcutLabelLocalizer for EnglishShortcutLabels {
    fn token_label(
        &self,
        platform: ShortcutPlatform,
        token: ShortcutLabelToken<'_>,
    ) -> Option<String> {
        match token {
            ShortcutLabelToken::Modifier(modifier) => {
                Some(english_modifier_label(platform, modifier).to_owned())
            }
            ShortcutLabelToken::LogicalKey(key) => english_logical_key_label(platform, key),
            ShortcutLabelToken::PhysicalKey(key) => english_physical_key_label(platform, key),
        }
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn separator(&self, _platform: ShortcutPlatform) -> &str {
        "+"
    }
}

impl Shortcut {
    /// Formats this shortcut with an explicit platform and caller-supplied localizer.
    #[must_use]
    pub fn localized_label(
        &self,
        platform: ShortcutPlatform,
        localizer: &dyn ShortcutLabelLocalizer,
    ) -> Option<String> {
        let mut labels = Vec::with_capacity(5);
        append_modifiers(&mut labels, self.modifiers, platform, localizer)?;

        let key = if self.key == Key::Unidentified {
            match self.physical_key {
                Some(key) if key != PhysicalKey::Unidentified => {
                    ShortcutLabelToken::PhysicalKey(key)
                }
                Some(_) | None => return None,
            }
        } else {
            ShortcutLabelToken::LogicalKey(&self.key)
        };
        append_label(&mut labels, platform, key, localizer)?;

        Some(labels.join(localizer.separator(platform)))
    }

    /// Formats this shortcut with deterministic English reference labels.
    #[must_use]
    pub fn english_label(&self, platform: ShortcutPlatform) -> Option<String> {
        self.localized_label(platform, &EnglishShortcutLabels)
    }
}

fn append_modifiers(
    labels: &mut Vec<String>,
    modifiers: Modifiers,
    platform: ShortcutPlatform,
    localizer: &dyn ShortcutLabelLocalizer,
) -> Option<()> {
    for (active, modifier) in [
        (modifiers.ctrl, ShortcutModifier::Control),
        (modifiers.alt, ShortcutModifier::Alt),
        (modifiers.shift, ShortcutModifier::Shift),
        (modifiers.super_key, ShortcutModifier::Super),
    ] {
        if active {
            append_label(
                labels,
                platform,
                ShortcutLabelToken::Modifier(modifier),
                localizer,
            )?;
        }
    }
    Some(())
}

fn append_label(
    labels: &mut Vec<String>,
    platform: ShortcutPlatform,
    token: ShortcutLabelToken<'_>,
    localizer: &dyn ShortcutLabelLocalizer,
) -> Option<()> {
    let label = localizer.token_label(platform, token)?;
    if label.is_empty() {
        return None;
    }
    labels.push(label);
    Some(())
}

const fn english_modifier_label(
    platform: ShortcutPlatform,
    modifier: ShortcutModifier,
) -> &'static str {
    match (platform, modifier) {
        (ShortcutPlatform::Windows | ShortcutPlatform::Linux, ShortcutModifier::Control) => "Ctrl",
        (ShortcutPlatform::MacOs, ShortcutModifier::Control) => "Control",
        (ShortcutPlatform::Windows | ShortcutPlatform::Linux, ShortcutModifier::Alt) => "Alt",
        (ShortcutPlatform::MacOs, ShortcutModifier::Alt) => "Option",
        (_, ShortcutModifier::Shift) => "Shift",
        (ShortcutPlatform::Windows, ShortcutModifier::Super) => "Win",
        (ShortcutPlatform::MacOs, ShortcutModifier::Super) => "Command",
        (ShortcutPlatform::Linux, ShortcutModifier::Super) => "Super",
    }
}

fn english_logical_key_label(platform: ShortcutPlatform, key: &Key) -> Option<String> {
    let label = match key {
        Key::Character(text) => {
            if text.trim().is_empty() {
                return None;
            }
            let mut chars = text.chars();
            let first = chars.next()?;
            if first.is_ascii_alphabetic() && chars.next().is_none() {
                first.to_ascii_uppercase().to_string()
            } else {
                text.clone()
            }
        }
        Key::Enter => platform_label(platform, "Enter", "Return", "Enter").to_owned(),
        Key::Escape => "Esc".to_owned(),
        Key::Tab => "Tab".to_owned(),
        Key::Backspace => platform_label(platform, "Backspace", "Delete", "Backspace").to_owned(),
        Key::Delete => platform_label(platform, "Delete", "Forward Delete", "Delete").to_owned(),
        Key::Insert => "Insert".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::End => "End".to_owned(),
        Key::PageUp => "Page Up".to_owned(),
        Key::PageDown => "Page Down".to_owned(),
        Key::ArrowLeft => "Left".to_owned(),
        Key::ArrowRight => "Right".to_owned(),
        Key::ArrowUp => "Up".to_owned(),
        Key::ArrowDown => "Down".to_owned(),
        Key::Space => "Space".to_owned(),
        Key::ContextMenu => "Context Menu".to_owned(),
        Key::Function(0) | Key::Unidentified => return None,
        Key::Function(number) => format!("F{number}"),
    };
    Some(label)
}

#[allow(clippy::too_many_lines)]
fn english_physical_key_label(platform: ShortcutPlatform, key: PhysicalKey) -> Option<String> {
    let label = match key {
        PhysicalKey::KeyA => "A".to_owned(),
        PhysicalKey::KeyB => "B".to_owned(),
        PhysicalKey::KeyC => "C".to_owned(),
        PhysicalKey::KeyD => "D".to_owned(),
        PhysicalKey::KeyE => "E".to_owned(),
        PhysicalKey::KeyF => "F".to_owned(),
        PhysicalKey::KeyG => "G".to_owned(),
        PhysicalKey::KeyH => "H".to_owned(),
        PhysicalKey::KeyI => "I".to_owned(),
        PhysicalKey::KeyJ => "J".to_owned(),
        PhysicalKey::KeyK => "K".to_owned(),
        PhysicalKey::KeyL => "L".to_owned(),
        PhysicalKey::KeyM => "M".to_owned(),
        PhysicalKey::KeyN => "N".to_owned(),
        PhysicalKey::KeyO => "O".to_owned(),
        PhysicalKey::KeyP => "P".to_owned(),
        PhysicalKey::KeyQ => "Q".to_owned(),
        PhysicalKey::KeyR => "R".to_owned(),
        PhysicalKey::KeyS => "S".to_owned(),
        PhysicalKey::KeyT => "T".to_owned(),
        PhysicalKey::KeyU => "U".to_owned(),
        PhysicalKey::KeyV => "V".to_owned(),
        PhysicalKey::KeyW => "W".to_owned(),
        PhysicalKey::KeyX => "X".to_owned(),
        PhysicalKey::KeyY => "Y".to_owned(),
        PhysicalKey::KeyZ => "Z".to_owned(),
        PhysicalKey::Digit(number) if number <= 9 => number.to_string(),
        PhysicalKey::NumpadDigit(number) if number <= 9 => format!("Numpad {number}"),
        PhysicalKey::Digit(_)
        | PhysicalKey::NumpadDigit(_)
        | PhysicalKey::Function(0)
        | PhysicalKey::Unidentified => return None,
        PhysicalKey::Enter => platform_label(platform, "Enter", "Return", "Enter").to_owned(),
        PhysicalKey::NumpadEnter => {
            platform_label(platform, "Numpad Enter", "Numpad Return", "Numpad Enter").to_owned()
        }
        PhysicalKey::Escape => "Esc".to_owned(),
        PhysicalKey::Tab => "Tab".to_owned(),
        PhysicalKey::Space => "Space".to_owned(),
        PhysicalKey::Backspace => {
            platform_label(platform, "Backspace", "Delete", "Backspace").to_owned()
        }
        PhysicalKey::Delete => {
            platform_label(platform, "Delete", "Forward Delete", "Delete").to_owned()
        }
        PhysicalKey::Insert => "Insert".to_owned(),
        PhysicalKey::Home => "Home".to_owned(),
        PhysicalKey::End => "End".to_owned(),
        PhysicalKey::PageUp => "Page Up".to_owned(),
        PhysicalKey::PageDown => "Page Down".to_owned(),
        PhysicalKey::ArrowLeft => "Left".to_owned(),
        PhysicalKey::ArrowRight => "Right".to_owned(),
        PhysicalKey::ArrowUp => "Up".to_owned(),
        PhysicalKey::ArrowDown => "Down".to_owned(),
        PhysicalKey::Function(number) => format!("F{number}"),
        PhysicalKey::Minus => "-".to_owned(),
        PhysicalKey::Equal => "=".to_owned(),
        PhysicalKey::BracketLeft => "[".to_owned(),
        PhysicalKey::BracketRight => "]".to_owned(),
        PhysicalKey::Backslash => "\\".to_owned(),
        PhysicalKey::Semicolon => ";".to_owned(),
        PhysicalKey::Quote => "'".to_owned(),
        PhysicalKey::Backquote => "`".to_owned(),
        PhysicalKey::Comma => ",".to_owned(),
        PhysicalKey::Period => ".".to_owned(),
        PhysicalKey::Slash => "/".to_owned(),
        PhysicalKey::NumpadAdd => "Numpad +".to_owned(),
        PhysicalKey::NumpadSubtract => "Numpad -".to_owned(),
        PhysicalKey::NumpadMultiply => "Numpad *".to_owned(),
        PhysicalKey::NumpadDivide => "Numpad /".to_owned(),
        PhysicalKey::NumpadDecimal => "Numpad .".to_owned(),
        PhysicalKey::ShiftLeft => "Left Shift".to_owned(),
        PhysicalKey::ShiftRight => "Right Shift".to_owned(),
        PhysicalKey::ControlLeft => {
            platform_label(platform, "Left Ctrl", "Left Control", "Left Ctrl").to_owned()
        }
        PhysicalKey::ControlRight => {
            platform_label(platform, "Right Ctrl", "Right Control", "Right Ctrl").to_owned()
        }
        PhysicalKey::AltLeft => {
            platform_label(platform, "Left Alt", "Left Option", "Left Alt").to_owned()
        }
        PhysicalKey::AltRight => {
            platform_label(platform, "Right Alt", "Right Option", "Right Alt").to_owned()
        }
        PhysicalKey::SuperLeft => {
            platform_label(platform, "Left Win", "Left Command", "Left Super").to_owned()
        }
        PhysicalKey::SuperRight => {
            platform_label(platform, "Right Win", "Right Command", "Right Super").to_owned()
        }
    };
    Some(label)
}

const fn platform_label(
    platform: ShortcutPlatform,
    windows: &'static str,
    mac_os: &'static str,
    linux: &'static str,
) -> &'static str {
    match platform {
        ShortcutPlatform::Windows => windows,
        ShortcutPlatform::MacOs => mac_os,
        ShortcutPlatform::Linux => linux,
    }
}
