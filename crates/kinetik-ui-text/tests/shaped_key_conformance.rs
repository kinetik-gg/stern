//! Conformance coverage for the shaped horizontal-key dispatch seam.

use kinetik_ui_core::{Key, KeyEvent, KeyState, Modifiers, TextRange};
use kinetik_ui_text::{
    CosmicTextEngine, ShapedTextNavigation, TextAffinity, TextCaret, TextComposition,
    TextEditState, TextLayoutKey, TextNavigationOutcome, TextSelection, TextStyle,
};

const MIXED: &str = "abc אבג def";
const WORDS: &str = "café אבג crème";
const COMBINING: &str = "Ae\u{301}B";

fn navigation(source: &str) -> ShapedTextNavigation {
    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&TextLayoutKey::new(
        source,
        TextStyle::new("Inter", 18.0, 24.0),
        400.0,
        false,
    ));
    layout.navigation(source).expect("valid shaped navigation")
}

const fn caret(offset: usize, affinity: TextAffinity) -> TextCaret {
    TextCaret::new(offset, affinity)
}

fn key(key: Key, modifiers: Modifiers, repeat: bool) -> KeyEvent {
    KeyEvent::new(key, KeyState::Pressed, modifiers, repeat)
}

#[test]
#[allow(clippy::too_many_lines)]
fn mixed_bidi_key_matrix_is_literal() {
    struct Case {
        source: &'static str,
        selection: TextSelection,
        affinity: TextAffinity,
        key: Key,
        modifiers: Modifiers,
        expected_selection: TextSelection,
        expected_caret: TextCaret,
    }

    let cases = [
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 8),
            affinity: TextAffinity::After,
            key: Key::ArrowRight,
            modifiers: Modifiers::default(),
            expected_selection: TextSelection::new(6, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(6, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowLeft,
            modifiers: Modifiers::default(),
            expected_selection: TextSelection::new(8, 8),
            expected_caret: caret(8, TextAffinity::After),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 8),
            affinity: TextAffinity::After,
            key: Key::ArrowRight,
            modifiers: Modifiers::new(true, false, false, false),
            expected_selection: TextSelection::new(8, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowLeft,
            modifiers: Modifiers::new(true, false, false, false),
            expected_selection: TextSelection::new(8, 8),
            expected_caret: caret(8, TextAffinity::After),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowLeft,
            modifiers: Modifiers::default(),
            expected_selection: TextSelection::new(8, 8),
            expected_caret: caret(8, TextAffinity::After),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowRight,
            modifiers: Modifiers::default(),
            expected_selection: TextSelection::new(6, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: WORDS,
            selection: TextSelection::new(0, 0),
            affinity: TextAffinity::After,
            key: Key::ArrowRight,
            modifiers: Modifiers::new(false, true, false, false),
            expected_selection: TextSelection::new(6, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: WORDS,
            selection: TextSelection::new(19, 19),
            affinity: TextAffinity::Before,
            key: Key::ArrowLeft,
            modifiers: Modifiers::new(false, false, true, false),
            expected_selection: TextSelection::new(13, 13),
            expected_caret: caret(13, TextAffinity::After),
        },
        Case {
            source: WORDS,
            selection: TextSelection::new(0, 0),
            affinity: TextAffinity::After,
            key: Key::ArrowRight,
            modifiers: Modifiers::new(true, true, false, false),
            expected_selection: TextSelection::new(0, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: WORDS,
            selection: TextSelection::new(0, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowLeft,
            modifiers: Modifiers::new(true, false, true, false),
            expected_selection: TextSelection::new(0, 0),
            expected_caret: caret(0, TextAffinity::After),
        },
        Case {
            source: WORDS,
            selection: TextSelection::new(0, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowLeft,
            modifiers: Modifiers::new(false, true, false, false),
            expected_selection: TextSelection::new(0, 0),
            expected_caret: caret(0, TextAffinity::After),
        },
        Case {
            source: WORDS,
            selection: TextSelection::new(0, 6),
            affinity: TextAffinity::Before,
            key: Key::ArrowRight,
            modifiers: Modifiers::new(false, true, false, false),
            expected_selection: TextSelection::new(6, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 8),
            affinity: TextAffinity::After,
            key: Key::ArrowRight,
            modifiers: Modifiers::new(false, true, true, false),
            expected_selection: TextSelection::new(6, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
        Case {
            source: MIXED,
            selection: TextSelection::new(8, 8),
            affinity: TextAffinity::After,
            key: Key::ArrowRight,
            modifiers: Modifiers::new(false, true, false, true),
            expected_selection: TextSelection::new(6, 6),
            expected_caret: caret(6, TextAffinity::Before),
        },
    ];

    for case in cases {
        let navigation = navigation(case.source);
        let mut state = TextEditState::new(case.source);
        state.set_selection_with_affinity(case.selection, case.affinity);
        assert_eq!(
            state.apply_visual_navigation_key(&key(case.key, case.modifiers, false), &navigation),
            Some(TextNavigationOutcome::Moved)
        );
        assert_eq!(state.selection, case.expected_selection);
        assert_eq!(state.caret_position(), case.expected_caret);
    }
}

#[test]
fn repeat_release_unrelated_and_boundary_outcomes_are_distinct() {
    let mixed = navigation(MIXED);
    let mut repeated = TextEditState::new(MIXED);
    repeated.set_caret_position(caret(8, TextAffinity::After));
    assert_eq!(
        repeated.apply_visual_navigation_key(
            &key(Key::ArrowRight, Modifiers::default(), true),
            &mixed,
        ),
        Some(TextNavigationOutcome::Moved)
    );
    assert_eq!(repeated.caret_position(), caret(6, TextAffinity::Before));

    let mut unchanged = TextEditState::new(MIXED);
    let expected = unchanged.clone();
    let released = KeyEvent::new(
        Key::ArrowRight,
        KeyState::Released,
        Modifiers::default(),
        false,
    );
    assert_eq!(
        unchanged.apply_visual_navigation_key(&released, &mixed),
        None
    );
    assert_eq!(unchanged, expected);
    assert_eq!(
        unchanged
            .apply_visual_navigation_key(&key(Key::ArrowUp, Modifiers::default(), false), &mixed,),
        None
    );
    assert_eq!(unchanged, expected);

    let rtl_source = "אבג";
    let rtl = navigation(rtl_source);
    let mut edge = TextEditState::new(rtl_source);
    let edge_expected = edge.clone();
    assert_eq!(
        edge.apply_visual_navigation_key(&key(Key::ArrowLeft, Modifiers::default(), false), &rtl,),
        Some(TextNavigationOutcome::Unchanged)
    );
    assert_eq!(edge, edge_expected);
}

#[test]
fn stale_navigation_is_transactional_for_every_dispatch_branch() {
    let stale = navigation("Xe\u{301}B");
    let mut expected = TextEditState::new(COMBINING);
    expected.insert_text("!");
    assert!(expected.undo());
    expected.set_caret_position(caret(4, TextAffinity::Before));
    expected.selection.anchor = 2;

    let modifiers = [
        Modifiers::default(),
        Modifiers::new(true, false, false, false),
        Modifiers::new(false, true, false, false),
        Modifiers::new(true, true, false, false),
    ];
    for direction in [Key::ArrowLeft, Key::ArrowRight] {
        for modifiers in modifiers {
            let mut actual = expected.clone();
            assert_eq!(
                actual.apply_visual_navigation_key(
                    &key(direction.clone(), modifiers, false),
                    &stale,
                ),
                Some(TextNavigationOutcome::SourceMismatch)
            );
            assert_eq!(actual, expected);
            let mut actual_redo = actual;
            let mut expected_redo = expected.clone();
            assert_eq!(actual_redo.redo(), expected_redo.redo());
            assert_eq!(actual_redo, expected_redo);
        }
    }
}

#[test]
fn active_composition_precedes_matching_and_stale_sources() {
    let matching = navigation(MIXED);
    let stale = navigation("xbc אבג def");
    let mut expected = TextEditState::new(MIXED);
    expected.insert_text("!");
    assert!(expected.undo());
    expected.set_caret_position(caret(8, TextAffinity::After));
    expected.selection.anchor = 7;
    expected.composition = Some(TextComposition::new(
        "候補",
        Some(TextRange::new(0, "候".len())),
    ));

    let modifiers = [
        Modifiers::default(),
        Modifiers::new(true, false, false, false),
        Modifiers::new(false, true, false, false),
        Modifiers::new(false, false, true, false),
        Modifiers::new(false, true, true, false),
        Modifiers::new(false, true, false, true),
        Modifiers::new(true, true, false, false),
    ];
    for navigation in [&matching, &stale] {
        for direction in [Key::ArrowLeft, Key::ArrowRight] {
            for modifiers in modifiers {
                let mut actual = expected.clone();
                assert_eq!(
                    actual.apply_visual_navigation_key(
                        &key(direction.clone(), modifiers, true),
                        navigation,
                    ),
                    Some(TextNavigationOutcome::Unchanged)
                );
                assert_eq!(actual, expected);
            }
        }
    }
}
