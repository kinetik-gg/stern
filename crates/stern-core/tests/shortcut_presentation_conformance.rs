//! Deterministic conformance evidence for shortcut presentation policy.

use stern_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionPriority,
    ActionQueue, ActionRouter, ActionSource, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PhysicalKey, Shortcut, ShortcutLabelLocalizer, ShortcutLabelToken, ShortcutModifier,
    ShortcutPlatform,
};

const PLATFORMS: [ShortcutPlatform; 3] = [
    ShortcutPlatform::Windows,
    ShortcutPlatform::MacOs,
    ShortcutPlatform::Linux,
];

fn logical(key: Key) -> Shortcut {
    Shortcut::new(Modifiers::default(), key)
}

fn physical(key: PhysicalKey) -> Shortcut {
    Shortcut::physical(Modifiers::default(), key)
}

fn assert_labels(shortcut: &Shortcut, expected: [Option<&str>; 3]) {
    for (platform, expected) in PLATFORMS.into_iter().zip(expected) {
        assert_eq!(
            shortcut.english_label(platform),
            expected.map(str::to_owned),
            "unexpected {platform:?} label for {shortcut:?}"
        );
    }
}

#[derive(Clone, Copy)]
enum LocalizerFailure {
    None,
    RejectAlt,
    EmptyKey,
}

struct SymbolLocalizer {
    separator: String,
    failure: LocalizerFailure,
}

impl ShortcutLabelLocalizer for SymbolLocalizer {
    fn token_label(
        &self,
        _platform: ShortcutPlatform,
        token: ShortcutLabelToken<'_>,
    ) -> Option<String> {
        if matches!(
            (self.failure, token),
            (
                LocalizerFailure::RejectAlt,
                ShortcutLabelToken::Modifier(ShortcutModifier::Alt)
            )
        ) {
            return None;
        }
        if matches!(
            (self.failure, token),
            (
                LocalizerFailure::EmptyKey,
                ShortcutLabelToken::LogicalKey(_) | ShortcutLabelToken::PhysicalKey(_)
            )
        ) {
            return Some(String::new());
        }

        Some(match token {
            ShortcutLabelToken::Modifier(ShortcutModifier::Control) => "⌃".into(),
            ShortcutLabelToken::Modifier(ShortcutModifier::Alt) => "⌥".into(),
            ShortcutLabelToken::Modifier(ShortcutModifier::Shift) => "⇧".into(),
            ShortcutLabelToken::Modifier(ShortcutModifier::Super) => "⌘".into(),
            ShortcutLabelToken::LogicalKey(Key::Character(value)) => format!("key:{value}"),
            ShortcutLabelToken::LogicalKey(key) => format!("logical:{key:?}"),
            ShortcutLabelToken::PhysicalKey(key) => format!("physical:{key:?}"),
        })
    }

    fn separator(&self, _platform: ShortcutPlatform) -> &str {
        &self.separator
    }
}

#[test]
fn modifiers_use_exact_stable_platform_order_and_names() {
    let shortcut = Shortcut::new(
        Modifiers::new(true, true, true, true),
        Key::Character("k".into()),
    );

    assert_eq!(
        shortcut.english_label(ShortcutPlatform::Windows),
        Some("Ctrl+Alt+Shift+Win+K".into())
    );
    assert_eq!(
        shortcut.english_label(ShortcutPlatform::MacOs),
        Some("Control+Option+Shift+Command+K".into())
    );
    assert_eq!(
        shortcut.english_label(ShortcutPlatform::Linux),
        Some("Ctrl+Alt+Shift+Super+K".into())
    );
}

#[test]
fn presentation_includes_only_active_modifiers_once() {
    let shortcut = Shortcut::new(
        Modifiers::new(true, false, true, false),
        Key::Character("z".into()),
    );

    assert_eq!(
        shortcut.english_label(ShortcutPlatform::Windows),
        Some("Alt+Shift+Z".into())
    );
}

#[test]
fn logical_named_keys_have_exact_platform_labels() {
    let cases = [
        (Key::Enter, [Some("Enter"), Some("Return"), Some("Enter")]),
        (Key::Escape, [Some("Esc"); 3]),
        (Key::Tab, [Some("Tab"); 3]),
        (
            Key::Backspace,
            [Some("Backspace"), Some("Delete"), Some("Backspace")],
        ),
        (
            Key::Delete,
            [Some("Delete"), Some("Forward Delete"), Some("Delete")],
        ),
        (Key::Insert, [Some("Insert"); 3]),
        (Key::Home, [Some("Home"); 3]),
        (Key::End, [Some("End"); 3]),
        (Key::PageUp, [Some("Page Up"); 3]),
        (Key::PageDown, [Some("Page Down"); 3]),
        (Key::ArrowLeft, [Some("Left"); 3]),
        (Key::ArrowRight, [Some("Right"); 3]),
        (Key::ArrowUp, [Some("Up"); 3]),
        (Key::ArrowDown, [Some("Down"); 3]),
        (Key::Space, [Some("Space"); 3]),
        (Key::Function(1), [Some("F1"); 3]),
        (Key::Function(u8::MAX), [Some("F255"); 3]),
        (Key::Function(0), [None; 3]),
        (Key::Unidentified, [None; 3]),
    ];

    for (key, expected) in cases {
        assert_labels(&logical(key), expected);
    }
}

#[test]
fn logical_character_labels_normalize_only_one_ascii_letter() {
    let cases = [
        ("a", Some("A")),
        ("Z", Some("Z")),
        ("é", Some("é")),
        ("ßx", Some("ßx")),
        ("++", Some("++")),
        ("", None),
        (" \t", None),
    ];

    for (source, expected) in cases {
        assert_labels(&logical(Key::Character(source.into())), [expected; 3]);
    }
}

#[test]
fn physical_letters_and_bounded_numbers_have_exact_labels() {
    let letters = [
        PhysicalKey::KeyA,
        PhysicalKey::KeyB,
        PhysicalKey::KeyC,
        PhysicalKey::KeyD,
        PhysicalKey::KeyE,
        PhysicalKey::KeyF,
        PhysicalKey::KeyG,
        PhysicalKey::KeyH,
        PhysicalKey::KeyI,
        PhysicalKey::KeyJ,
        PhysicalKey::KeyK,
        PhysicalKey::KeyL,
        PhysicalKey::KeyM,
        PhysicalKey::KeyN,
        PhysicalKey::KeyO,
        PhysicalKey::KeyP,
        PhysicalKey::KeyQ,
        PhysicalKey::KeyR,
        PhysicalKey::KeyS,
        PhysicalKey::KeyT,
        PhysicalKey::KeyU,
        PhysicalKey::KeyV,
        PhysicalKey::KeyW,
        PhysicalKey::KeyX,
        PhysicalKey::KeyY,
        PhysicalKey::KeyZ,
    ];
    for (key, label) in letters.into_iter().zip('A'..='Z') {
        let expected = label.to_string();
        for platform in PLATFORMS {
            assert_eq!(
                physical(key).english_label(platform),
                Some(expected.clone())
            );
        }
    }

    for number in 0..=9 {
        for platform in PLATFORMS {
            assert_eq!(
                physical(PhysicalKey::Digit(number)).english_label(platform),
                Some(number.to_string())
            );
            assert_eq!(
                physical(PhysicalKey::NumpadDigit(number)).english_label(platform),
                Some(format!("Numpad {number}"))
            );
        }
    }
    for number in [10, u8::MAX] {
        assert_labels(&physical(PhysicalKey::Digit(number)), [None; 3]);
        assert_labels(&physical(PhysicalKey::NumpadDigit(number)), [None; 3]);
    }
}

#[test]
fn every_other_physical_key_has_exact_platform_label_or_rejection() {
    let cases = [
        (
            PhysicalKey::Enter,
            [Some("Enter"), Some("Return"), Some("Enter")],
        ),
        (
            PhysicalKey::NumpadEnter,
            [
                Some("Numpad Enter"),
                Some("Numpad Return"),
                Some("Numpad Enter"),
            ],
        ),
        (PhysicalKey::Escape, [Some("Esc"); 3]),
        (PhysicalKey::Tab, [Some("Tab"); 3]),
        (PhysicalKey::Space, [Some("Space"); 3]),
        (
            PhysicalKey::Backspace,
            [Some("Backspace"), Some("Delete"), Some("Backspace")],
        ),
        (
            PhysicalKey::Delete,
            [Some("Delete"), Some("Forward Delete"), Some("Delete")],
        ),
        (PhysicalKey::Insert, [Some("Insert"); 3]),
        (PhysicalKey::Home, [Some("Home"); 3]),
        (PhysicalKey::End, [Some("End"); 3]),
        (PhysicalKey::PageUp, [Some("Page Up"); 3]),
        (PhysicalKey::PageDown, [Some("Page Down"); 3]),
        (PhysicalKey::ArrowLeft, [Some("Left"); 3]),
        (PhysicalKey::ArrowRight, [Some("Right"); 3]),
        (PhysicalKey::ArrowUp, [Some("Up"); 3]),
        (PhysicalKey::ArrowDown, [Some("Down"); 3]),
        (PhysicalKey::Function(1), [Some("F1"); 3]),
        (PhysicalKey::Function(u8::MAX), [Some("F255"); 3]),
        (PhysicalKey::Function(0), [None; 3]),
        (PhysicalKey::Minus, [Some("-"); 3]),
        (PhysicalKey::Equal, [Some("="); 3]),
        (PhysicalKey::BracketLeft, [Some("["); 3]),
        (PhysicalKey::BracketRight, [Some("]"); 3]),
        (PhysicalKey::Backslash, [Some("\\"); 3]),
        (PhysicalKey::Semicolon, [Some(";"); 3]),
        (PhysicalKey::Quote, [Some("'"); 3]),
        (PhysicalKey::Backquote, [Some("`"); 3]),
        (PhysicalKey::Comma, [Some(","); 3]),
        (PhysicalKey::Period, [Some("."); 3]),
        (PhysicalKey::Slash, [Some("/"); 3]),
        (PhysicalKey::NumpadAdd, [Some("Numpad +"); 3]),
        (PhysicalKey::NumpadSubtract, [Some("Numpad -"); 3]),
        (PhysicalKey::NumpadMultiply, [Some("Numpad *"); 3]),
        (PhysicalKey::NumpadDivide, [Some("Numpad /"); 3]),
        (PhysicalKey::NumpadDecimal, [Some("Numpad ."); 3]),
        (PhysicalKey::ShiftLeft, [Some("Left Shift"); 3]),
        (PhysicalKey::ShiftRight, [Some("Right Shift"); 3]),
        (
            PhysicalKey::ControlLeft,
            [Some("Left Ctrl"), Some("Left Control"), Some("Left Ctrl")],
        ),
        (
            PhysicalKey::ControlRight,
            [
                Some("Right Ctrl"),
                Some("Right Control"),
                Some("Right Ctrl"),
            ],
        ),
        (
            PhysicalKey::AltLeft,
            [Some("Left Alt"), Some("Left Option"), Some("Left Alt")],
        ),
        (
            PhysicalKey::AltRight,
            [Some("Right Alt"), Some("Right Option"), Some("Right Alt")],
        ),
        (
            PhysicalKey::SuperLeft,
            [Some("Left Win"), Some("Left Command"), Some("Left Super")],
        ),
        (
            PhysicalKey::SuperRight,
            [
                Some("Right Win"),
                Some("Right Command"),
                Some("Right Super"),
            ],
        ),
        (PhysicalKey::Unidentified, [None; 3]),
    ];

    for (key, expected) in cases {
        assert_labels(&physical(key), expected);
    }
}

#[test]
fn caller_localizer_can_supply_symbols_and_an_empty_separator() {
    let localizer = SymbolLocalizer {
        separator: String::new(),
        failure: LocalizerFailure::None,
    };
    let shortcut = Shortcut::new(
        Modifiers::new(true, true, true, true),
        Key::Character("k".into()),
    );

    assert_eq!(
        shortcut.localized_label(ShortcutPlatform::MacOs, &localizer),
        Some("⌃⌥⇧⌘key:k".into())
    );
}

#[test]
fn logical_display_wins_and_unidentified_shortcuts_fail_closed() {
    let localizer = SymbolLocalizer {
        separator: "/".into(),
        failure: LocalizerFailure::None,
    };
    let logical_and_physical =
        logical(Key::Character("q".into())).with_physical_key(PhysicalKey::KeyA);

    assert_eq!(
        logical_and_physical.localized_label(ShortcutPlatform::Linux, &localizer),
        Some("key:q".into())
    );
    assert_eq!(
        physical(PhysicalKey::KeyB).localized_label(ShortcutPlatform::Linux, &localizer),
        Some("physical:KeyB".into())
    );
    assert_eq!(
        logical(Key::Unidentified).localized_label(ShortcutPlatform::Linux, &localizer),
        None
    );
    assert_eq!(
        physical(PhysicalKey::Unidentified).localized_label(ShortcutPlatform::Linux, &localizer),
        None
    );
}

#[test]
fn a_missing_or_empty_required_token_rejects_the_complete_label() {
    let shortcut = Shortcut::new(
        Modifiers::new(false, false, true, false),
        Key::Character("x".into()),
    );
    for failure in [LocalizerFailure::RejectAlt, LocalizerFailure::EmptyKey] {
        let localizer = SymbolLocalizer {
            separator: "+".into(),
            failure,
        };
        assert_eq!(
            shortcut.localized_label(ShortcutPlatform::Windows, &localizer),
            None
        );
    }
}

#[test]
fn repeated_presentation_preserves_shortcut_routing_and_action_state() {
    let modifiers = Modifiers::new(false, true, false, false);
    let shortcut =
        Shortcut::new(modifiers, Key::Character("k".into())).with_physical_key(PhysicalKey::KeyK);
    let original_shortcut = shortcut.clone();
    let mut descriptor = ActionDescriptor::new("edit.keep", "Keep");
    descriptor.shortcut = Some(shortcut.clone());
    let original_descriptor = descriptor.clone();
    let mut router = ActionRouter::new();
    router.bind(ActionBinding::new(
        descriptor.clone(),
        ActionContext::Global,
        ActionPriority::Global,
    ));
    let input = KeyboardInput {
        modifiers,
        events: vec![KeyEvent::with_physical_key(
            Key::Character("л".into()),
            PhysicalKey::KeyK,
            KeyState::Pressed,
            modifiers,
            false,
        )],
    };
    let before_route = router.resolve_shortcut(&input);
    let mut queue = ActionQueue::new();
    queue.push(ActionInvocation::new(
        ActionId::new("existing"),
        ActionSource::Programmatic,
        ActionContext::Global,
    ));
    let before_queue = queue.clone();

    assert_labels(
        &shortcut,
        [Some("Ctrl+K"), Some("Control+K"), Some("Ctrl+K")],
    );
    let owned_label: String = shortcut
        .english_label(ShortcutPlatform::Windows)
        .expect("complete owned presentation label");
    assert_eq!(owned_label, "Ctrl+K");

    assert_eq!(shortcut, original_shortcut);
    assert_eq!(descriptor, original_descriptor);
    assert_eq!(router.resolve_shortcut(&input), before_route);
    assert_eq!(queue, before_queue);
}
