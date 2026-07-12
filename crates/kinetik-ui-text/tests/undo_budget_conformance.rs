//! Conformance for bounded and coalesced text-field-local undo history.

use kinetik_ui_core::{
    ClipboardText, Key, KeyEvent, KeyState, Modifiers, PhysicalKey, TextInputEvent, TextRange,
    UiInputEvent, WidgetId,
};
use kinetik_ui_text::{
    CosmicTextEngine, TextAffinity, TextCaret, TextComposition, TextEditMode, TextEditState,
    TextLayoutKey, TextNavigationOutcome, TextSelection, TextStyle,
};

const HISTORY_TEXT_BYTES: usize = 4 * 1024 * 1024;
const RUN_BYTES: usize = 4096;

fn target() -> WidgetId {
    WidgetId::from_key("field")
}

fn hardware(text: &str, repeat: bool) -> UiInputEvent {
    UiInputEvent::Key(
        KeyEvent::with_physical_key(
            Key::Character(text.to_owned()),
            PhysicalKey::Unidentified,
            KeyState::Pressed,
            Modifiers::default(),
            repeat,
        )
        .with_text(text),
    )
}

fn key(key: Key) -> UiInputEvent {
    UiInputEvent::Key(KeyEvent::new(
        key,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    ))
}

fn repeated_key(key: Key) -> UiInputEvent {
    UiInputEvent::Key(KeyEvent::new(
        key,
        KeyState::Pressed,
        Modifiers::default(),
        true,
    ))
}

fn modified_key(key: Key, modifiers: Modifiers) -> UiInputEvent {
    UiInputEvent::Key(KeyEvent::new(key, KeyState::Pressed, modifiers, false))
}

fn filtered_hardware(text: &str) -> UiInputEvent {
    UiInputEvent::Key(KeyEvent {
        key: Key::Character(text.to_owned()),
        physical_key: PhysicalKey::Unidentified,
        state: KeyState::Pressed,
        modifiers: Modifiers::default(),
        repeat: false,
        text: Some(text.to_owned()),
    })
}

fn apply(state: &mut TextEditState, event: &UiInputEvent, mode: TextEditMode) {
    let _ = state.apply_ordered_input(core::slice::from_ref(event), target(), mode);
}

fn type_repeated(state: &mut TextEditState, text: &str, fragments: usize) {
    if fragments == 0 {
        return;
    }
    apply(state, &hardware(text, false), TextEditMode::SingleLine);
    let repeated = hardware(text, true);
    for _ in 1..fragments {
        apply(state, &repeated, TextEditMode::SingleLine);
    }
}

#[test]
fn atomic_count_budget_and_oversized_barriers_are_literal() {
    let mut bounded = TextEditState::new("");
    for _ in 0..129 {
        bounded.insert_text("x");
    }
    assert_eq!(bounded.text.len(), 129);
    for expected in (1..129).rev() {
        assert!(bounded.undo());
        assert_eq!(bounded.text.len(), expected);
    }
    assert!(!bounded.undo());
    for expected in 2..=129 {
        assert!(bounded.redo());
        assert_eq!(bounded.text.len(), expected);
    }
    assert!(!bounded.redo());

    let exact_text = "x".repeat(HISTORY_TEXT_BYTES);
    let mut exact = TextEditState::new(exact_text.clone());
    exact.insert_text("y");
    assert!(exact.undo(), "an exact-4-MiB snapshot is retainable");
    assert_eq!(exact.text, exact_text);
    assert!(
        !exact.redo(),
        "the oversized current state is not retained as a reverse target"
    );

    let oversized_text = "z".repeat(HISTORY_TEXT_BYTES + 1);
    let mut barrier = TextEditState::new(oversized_text);
    barrier.set_selection(TextSelection::new(0, barrier.text.len()));
    barrier.insert_text("small");
    assert!(!barrier.undo(), "an oversized pre-edit state is a barrier");
    barrier.insert_text("!");
    assert!(barrier.undo());
    assert_eq!(barrier.text, "small");
    assert!(
        !barrier.undo(),
        "new history after the barrier cannot jump to the oversized state"
    );

    let mut branch = TextEditState::new("");
    branch.insert_text("a");
    branch.insert_text("b");
    branch.insert_text("c");
    assert!(branch.undo());
    assert!(branch.undo());
    assert_eq!(branch.text, "a");
    branch.insert_text("x");
    assert!(!branch.redo(), "a fresh mutation clears the redo branch");
    assert!(branch.undo());
    assert_eq!(branch.text, "a");
}

#[test]
fn combined_stack_budget_survives_128_fill_64_undo_bidirectional_transfer_and_branch() {
    let mut state = TextEditState::new("");
    for _ in 0..128 {
        state.insert_text("x");
    }
    assert_eq!(state.text.len(), 128);

    for expected in (64..128).rev() {
        assert!(state.undo());
        assert_eq!(state.text.len(), expected);
    }
    for expected in 65..=96 {
        assert!(state.redo());
        assert_eq!(state.text.len(), expected);
    }
    for expected in (64..96).rev() {
        assert!(state.undo());
        assert_eq!(state.text.len(), expected);
    }

    state.insert_text("b");
    assert_eq!(state.text.len(), 65);
    assert!(!state.redo());
    assert!(state.undo());
    assert_eq!(state.text.len(), 64);
}

#[test]
fn ordered_typing_chunks_at_4096_bytes_across_calls_and_long_sessions() {
    let mut ten_thousand = TextEditState::new("");
    type_repeated(&mut ten_thousand, "x", 10_000);
    assert_eq!(ten_thousand.text.len(), 10_000);
    for expected in [8192, 4096, 0] {
        assert!(ten_thousand.undo());
        assert_eq!(ten_thousand.text.len(), expected);
    }
    assert!(!ten_thousand.undo());
    for expected in [4096, 8192, 10_000] {
        assert!(ten_thousand.redo());
        assert_eq!(ten_thousand.text.len(), expected);
    }
    assert!(!ten_thousand.redo());

    let mut long = TextEditState::new("");
    type_repeated(&mut long, "x", 100_000);
    assert_eq!(long.text.len(), 100_000);
    let mut units = 0;
    while long.undo() {
        units += 1;
    }
    assert_eq!(units, 25);
    assert!(long.text.is_empty());

    let mut multibyte_crossing = TextEditState::new("");
    type_repeated(&mut multibyte_crossing, "x", RUN_BYTES - 1);
    apply(
        &mut multibyte_crossing,
        &hardware("é", true),
        TextEditMode::SingleLine,
    );
    assert_eq!(multibyte_crossing.text.len(), RUN_BYTES + 1);
    assert!(multibyte_crossing.undo());
    assert_eq!(multibyte_crossing.text.len(), RUN_BYTES - 1);
    assert!(multibyte_crossing.undo());
    assert!(multibyte_crossing.text.is_empty());

    let large_fragment = "q".repeat(RUN_BYTES + 1);
    let mut unsplit = TextEditState::new("");
    apply(
        &mut unsplit,
        &hardware(&large_fragment, false),
        TextEditMode::SingleLine,
    );
    apply(&mut unsplit, &hardware("z", true), TextEditMode::SingleLine);
    assert!(unsplit.undo());
    assert_eq!(unsplit.text, large_fragment);
    assert!(unsplit.undo());
    assert!(unsplit.text.is_empty());
}

#[test]
fn ordered_deletion_runs_are_directional_egc_safe_and_fenced_by_navigation() {
    let source = "A👩‍🚀B👍🏽C";
    let mut backward = TextEditState::new(source);
    let backspace = key(Key::Backspace);
    let repeated_backspace = repeated_key(Key::Backspace);
    apply(&mut backward, &backspace, TextEditMode::SingleLine);
    while !backward.text.is_empty() {
        apply(&mut backward, &repeated_backspace, TextEditMode::SingleLine);
    }
    assert!(backward.undo());
    assert_eq!(backward.text, source);
    assert!(!backward.undo());

    let mut forward = TextEditState::new(source);
    forward.set_caret(0);
    let delete = key(Key::Delete);
    let repeated_delete = repeated_key(Key::Delete);
    apply(&mut forward, &delete, TextEditMode::SingleLine);
    while !forward.text.is_empty() {
        apply(&mut forward, &repeated_delete, TextEditMode::SingleLine);
    }
    assert!(forward.undo());
    assert_eq!(forward.text, source);
    assert!(!forward.undo());

    let mut alternating = TextEditState::new("abc");
    alternating.set_caret(1);
    apply(&mut alternating, &backspace, TextEditMode::SingleLine);
    apply(&mut alternating, &delete, TextEditMode::SingleLine);
    assert_eq!(alternating.text, "c");
    assert!(alternating.undo());
    assert_eq!(alternating.text, "bc");
    assert!(alternating.undo());
    assert_eq!(alternating.text, "abc");

    let mut fenced = TextEditState::new("");
    type_repeated(&mut fenced, "a", 2);
    apply(&mut fenced, &key(Key::ArrowLeft), TextEditMode::SingleLine);
    apply(&mut fenced, &key(Key::ArrowRight), TextEditMode::SingleLine);
    apply(&mut fenced, &hardware("c", false), TextEditMode::SingleLine);
    assert_eq!(fenced.text, "aac");
    assert!(fenced.undo());
    assert_eq!(fenced.text, "aa");
    assert!(fenced.undo());
    assert!(fenced.text.is_empty());

    let mut no_op_preserves_redo = TextEditState::new("");
    no_op_preserves_redo.insert_text("a");
    no_op_preserves_redo.insert_text("b");
    assert!(no_op_preserves_redo.undo());
    no_op_preserves_redo.set_caret(0);
    apply(
        &mut no_op_preserves_redo,
        &backspace,
        TextEditMode::SingleLine,
    );
    assert!(no_op_preserves_redo.redo());
    assert_eq!(no_op_preserves_redo.text, "ab");
}

#[test]
fn modified_and_active_preedit_deletions_remain_atomic() {
    let modifiers = [
        Modifiers::new(true, false, false, false),
        Modifiers::new(false, true, true, false),
        Modifiers::new(false, false, false, true),
    ];

    for modifier in modifiers {
        let mut backward = TextEditState::new("abcd");
        let event = modified_key(Key::Backspace, modifier);
        apply(&mut backward, &event, TextEditMode::SingleLine);
        apply(&mut backward, &event, TextEditMode::SingleLine);
        assert_eq!(backward.text, "ab");
        assert!(backward.undo());
        assert_eq!(backward.text, "abc");
        assert!(backward.undo());
        assert_eq!(backward.text, "abcd");

        let mut forward = TextEditState::new("abcd");
        forward.set_caret(0);
        let event = modified_key(Key::Delete, modifier);
        apply(&mut forward, &event, TextEditMode::SingleLine);
        apply(&mut forward, &event, TextEditMode::SingleLine);
        assert_eq!(forward.text, "cd");
        assert!(forward.undo());
        assert_eq!(forward.text, "bcd");
        assert!(forward.undo());
        assert_eq!(forward.text, "abcd");
    }

    for deletion in [Key::Backspace, Key::Delete] {
        let mut state = TextEditState::new("abcd");
        if deletion == Key::Delete {
            state.set_caret(0);
        }
        state.composition = Some(TextComposition::new("候", None));
        let event = key(deletion);
        apply(&mut state, &event, TextEditMode::SingleLine);
        apply(&mut state, &event, TextEditMode::SingleLine);
        assert_eq!(state.text.len(), 2);
        assert!(state.undo());
        assert_eq!(state.text.len(), 3);
        assert!(state.composition.is_none());
        assert!(state.undo());
        assert_eq!(state.text, "abcd");
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the literal fence matrix keeps every boundary and its adjacent assertions together"
)]
fn semantic_fence_matrix_splits_runs_and_ignored_foreign_input_preserves_redo() {
    #[derive(Clone, Copy)]
    enum Fence {
        CaretSetter,
        SelectionSetter,
        Composition,
        FocusLoss,
        CopyShortcut,
        EdgeDelete,
        ShapedNavigation,
    }

    for fence in [
        Fence::CaretSetter,
        Fence::SelectionSetter,
        Fence::Composition,
        Fence::FocusLoss,
        Fence::CopyShortcut,
        Fence::EdgeDelete,
        Fence::ShapedNavigation,
    ] {
        let mut state = TextEditState::new("");
        type_repeated(&mut state, "x", 2);
        match fence {
            Fence::CaretSetter => state.set_caret(state.text.len()),
            Fence::SelectionSetter => {
                state.set_selection(TextSelection::new(state.text.len(), state.text.len()));
            }
            Fence::Composition => {
                let _ = state.apply_ordered_input(
                    &[
                        UiInputEvent::Text(TextInputEvent::CompositionStart),
                        UiInputEvent::Text(TextInputEvent::CompositionEnd),
                    ],
                    target(),
                    TextEditMode::SingleLine,
                );
            }
            Fence::FocusLoss => apply(
                &mut state,
                &UiInputEvent::WindowFocusChanged(false),
                TextEditMode::SingleLine,
            ),
            Fence::CopyShortcut => {
                let copy = UiInputEvent::Key(KeyEvent::with_physical_key(
                    Key::Character("ignored".to_owned()),
                    PhysicalKey::KeyC,
                    KeyState::Pressed,
                    Modifiers::new(false, true, false, false),
                    false,
                ));
                apply(&mut state, &copy, TextEditMode::SingleLine);
            }
            Fence::EdgeDelete => apply(&mut state, &key(Key::Delete), TextEditMode::SingleLine),
            Fence::ShapedNavigation => {
                let mut engine = CosmicTextEngine::new();
                let layout = engine.shape_text(&TextLayoutKey::new(
                    state.text.clone(),
                    TextStyle::new("Inter", 14.0, 20.0),
                    400.0,
                    false,
                ));
                let navigation = layout.navigation(&state.text).expect("matching map");
                let _ = state.move_visual_left(&navigation);
                let _ = state.move_visual_right(&navigation);
            }
        }
        apply(&mut state, &hardware("y", false), TextEditMode::SingleLine);
        assert!(state.undo());
        assert_eq!(state.text, "xx");
        assert!(state.undo());
        assert!(state.text.is_empty());
    }

    let mut foreign = TextEditState::new("");
    foreign.insert_text("a");
    foreign.insert_text("b");
    assert!(foreign.undo());
    let other = WidgetId::from_key("other");
    let _ = foreign.apply_ordered_input(
        &[UiInputEvent::ClipboardText(ClipboardText::new(
            other, "ignored",
        ))],
        target(),
        TextEditMode::SingleLine,
    );
    assert!(foreign.redo());
    assert_eq!(foreign.text, "ab");

    let filtered = UiInputEvent::ClipboardText(ClipboardText::new(target(), "\n\r\t"));
    let mut matching = TextEditState::new("");
    type_repeated(&mut matching, "x", 2);
    apply(&mut matching, &filtered, TextEditMode::SingleLine);
    apply(
        &mut matching,
        &hardware("y", false),
        TextEditMode::SingleLine,
    );
    assert_eq!(matching.text, "xxy");
    assert!(matching.undo());
    assert_eq!(matching.text, "xx");
    assert!(matching.undo());
    assert!(matching.text.is_empty());

    let mut filtered_preserves_redo = TextEditState::new("");
    filtered_preserves_redo.insert_text("a");
    filtered_preserves_redo.insert_text("b");
    assert!(filtered_preserves_redo.undo());
    apply(
        &mut filtered_preserves_redo,
        &filtered,
        TextEditMode::SingleLine,
    );
    assert!(filtered_preserves_redo.redo());
    assert_eq!(filtered_preserves_redo.text, "ab");

    let mut foreign_run = TextEditState::new("");
    type_repeated(&mut foreign_run, "x", 2);
    let wrong_target = UiInputEvent::ClipboardText(ClipboardText::new(other, "ignored"));
    apply(&mut foreign_run, &wrong_target, TextEditMode::SingleLine);
    apply(
        &mut foreign_run,
        &hardware("y", true),
        TextEditMode::SingleLine,
    );
    assert!(foreign_run.undo());
    assert!(foreign_run.text.is_empty());

    for filtered in ["", "\n\r\t"] {
        let mut filtered_state = TextEditState::new("");
        type_repeated(&mut filtered_state, "x", 2);
        apply(
            &mut filtered_state,
            &filtered_hardware(filtered),
            TextEditMode::SingleLine,
        );
        apply(
            &mut filtered_state,
            &hardware("y", false),
            TextEditMode::SingleLine,
        );
        assert_eq!(filtered_state.text, "xxy");
        assert!(filtered_state.undo());
        assert_eq!(filtered_state.text, "xx");
        assert!(filtered_state.undo());
        assert!(filtered_state.text.is_empty());
    }

    let mut filtered_hardware_redo = TextEditState::new("");
    filtered_hardware_redo.insert_text("a");
    filtered_hardware_redo.insert_text("b");
    assert!(filtered_hardware_redo.undo());
    apply(
        &mut filtered_hardware_redo,
        &hardware("\n", false),
        TextEditMode::SingleLine,
    );
    assert!(filtered_hardware_redo.redo());
    assert_eq!(filtered_hardware_redo.text, "ab");
}

#[test]
fn cut_word_delete_and_selection_replacement_are_atomic_between_typing_runs() {
    let ctrl = Modifiers::new(false, true, false, false);

    let mut cut = TextEditState::new("z");
    type_repeated(&mut cut, "a", 2);
    cut.selection = TextSelection::new(2, 3);
    apply(
        &mut cut,
        &modified_key(Key::Character("x".to_owned()), ctrl),
        TextEditMode::SingleLine,
    );
    apply(&mut cut, &hardware("b", false), TextEditMode::SingleLine);
    assert_eq!(cut.text, "zab");
    assert!(cut.undo());
    assert_eq!(cut.text, "za");
    assert!(cut.undo());
    assert_eq!(cut.text, "zaa");
    assert!(cut.undo());
    assert_eq!(cut.text, "z");

    let mut word_delete = TextEditState::new("");
    type_repeated(&mut word_delete, "w", 4);
    apply(
        &mut word_delete,
        &modified_key(Key::Backspace, ctrl),
        TextEditMode::SingleLine,
    );
    apply(
        &mut word_delete,
        &hardware("x", false),
        TextEditMode::SingleLine,
    );
    assert_eq!(word_delete.text, "x");
    assert!(word_delete.undo());
    assert!(word_delete.text.is_empty());
    assert!(word_delete.undo());
    assert_eq!(word_delete.text, "wwww");
    assert!(word_delete.undo());
    assert!(word_delete.text.is_empty());

    let mut replacement = TextEditState::new("");
    type_repeated(&mut replacement, "a", 2);
    replacement.selection = TextSelection::new(0, 1);
    apply(
        &mut replacement,
        &hardware("X", false),
        TextEditMode::SingleLine,
    );
    apply(
        &mut replacement,
        &hardware("y", false),
        TextEditMode::SingleLine,
    );
    assert_eq!(replacement.text, "Xya");
    assert!(replacement.undo());
    assert_eq!(replacement.text, "Xa");
    assert!(replacement.undo());
    assert_eq!(replacement.text, "aa");
    assert!(replacement.undo());
    assert!(replacement.text.is_empty());
}

#[test]
fn ime_paste_enter_and_selection_edits_are_atomic_ordered_units() {
    let mut state = TextEditState::new("");
    type_repeated(&mut state, "a", 2);
    let events = [
        UiInputEvent::Text(TextInputEvent::CompositionStart),
        UiInputEvent::Text(TextInputEvent::Composition {
            text: "候".to_owned(),
            selection: Some(TextRange::new(0, "候".len())),
        }),
        UiInputEvent::Text(TextInputEvent::Commit("候".to_owned())),
        UiInputEvent::ClipboardText(ClipboardText::new(target(), "P")),
        key(Key::Enter),
        key(Key::Backspace),
    ];
    let _ = state.apply_ordered_input(&events, target(), TextEditMode::MultiLine);
    assert_eq!(state.text, "aa候P");

    assert!(state.undo());
    assert_eq!(state.text, "aa候P\n");
    assert!(state.undo());
    assert_eq!(state.text, "aa候P");
    assert!(state.undo());
    assert_eq!(state.text, "aa候");
    assert!(state.undo());
    assert_eq!(state.text, "aa");
    assert!(state.undo());
    assert!(state.text.is_empty());

    for expected_selection in [TextSelection::new(4, 10), TextSelection::new(10, 4)] {
        let mut selection = TextEditState::new("abc אבג");
        selection.set_selection_with_affinity(expected_selection, TextAffinity::Before);
        let expected_affinity = selection.caret_position().affinity;
        selection.insert_text("X");
        selection.composition = Some(TextComposition::new("候", None));
        assert!(selection.undo());
        assert_eq!(selection.text, "abc אבג");
        assert_eq!(selection.selection, expected_selection);
        assert_eq!(selection.caret_position().affinity, expected_affinity);
        assert!(selection.composition.is_none());
        selection.composition = Some(TextComposition::new("候", None));
        assert!(selection.redo());
        assert_eq!(selection.text, "abc X");
        assert!(selection.composition.is_none());
    }
}

#[test]
fn mixed_ordered_stream_preserves_mutation_and_history_event_order() {
    let ctrl = Modifiers::new(false, true, false, false);
    let events = [
        hardware("a", false),
        hardware("b", true),
        UiInputEvent::Text(TextInputEvent::CompositionStart),
        UiInputEvent::Text(TextInputEvent::Commit("候".to_owned())),
        UiInputEvent::ClipboardText(ClipboardText::new(target(), "P")),
        key(Key::Backspace),
        modified_key(Key::Character("z".to_owned()), ctrl),
        modified_key(Key::Character("y".to_owned()), ctrl),
        key(Key::ArrowLeft),
        key(Key::Delete),
        modified_key(Key::Character("z".to_owned()), ctrl),
        modified_key(Key::Character("y".to_owned()), ctrl),
    ];
    let mut state = TextEditState::new("");
    let _ = state.apply_ordered_input(&events, target(), TextEditMode::SingleLine);
    assert_eq!(state.text, "ab");

    for expected in ["ab候", "ab候P", "ab候", "ab", ""] {
        assert!(state.undo());
        assert_eq!(state.text, expected);
    }
    assert!(!state.undo());
}

#[test]
fn stale_and_active_preedit_visual_keys_preserve_an_in_progress_run() {
    let mut state = TextEditState::new("");
    type_repeated(&mut state, "a", 2);

    let mut engine = CosmicTextEngine::new();
    let stale_source = "different";
    let stale_layout = engine.shape_text(&TextLayoutKey::new(
        stale_source,
        TextStyle::new("Inter", 14.0, 20.0),
        400.0,
        false,
    ));
    let stale_navigation = stale_layout.navigation(stale_source).expect("stale map");
    let expected = state.clone();
    assert_eq!(
        state.move_visual_left(&stale_navigation),
        TextNavigationOutcome::SourceMismatch
    );
    assert_eq!(state, expected);

    let source = state.text.clone();
    let matching_layout = engine.shape_text(&TextLayoutKey::new(
        source.clone(),
        TextStyle::new("Inter", 14.0, 20.0),
        400.0,
        false,
    ));
    let matching = matching_layout.navigation(&source).expect("matching map");
    state.composition = Some(TextComposition::new("候", None));
    let expected = state.clone();
    let arrow = KeyEvent::new(
        Key::ArrowLeft,
        KeyState::Pressed,
        Modifiers::new(true, true, false, false),
        false,
    );
    assert_eq!(
        state.apply_visual_navigation_key(&arrow, &matching),
        Some(TextNavigationOutcome::Unchanged)
    );
    assert_eq!(state, expected);

    state.composition = None;
    assert!(state.undo());
    assert!(
        state.text.is_empty(),
        "the preserved typing run undoes once"
    );
}

#[test]
fn direct_edit_calls_remain_atomic_and_clone_preserves_future_behavior() {
    let mut direct = TextEditState::new("");
    direct.insert_text("a");
    direct.insert_text("b");
    assert!(direct.undo());
    assert_eq!(direct.text, "a");
    assert!(direct.undo());
    assert!(direct.text.is_empty());

    let mut in_progress = TextEditState::new("");
    type_repeated(&mut in_progress, "x", 17);
    let mut cloned = in_progress.clone();
    apply(
        &mut in_progress,
        &hardware("y", true),
        TextEditMode::SingleLine,
    );
    apply(&mut cloned, &hardware("y", true), TextEditMode::SingleLine);
    assert_eq!(in_progress, cloned);
    assert!(in_progress.undo());
    assert!(cloned.undo());
    assert_eq!(in_progress, cloned);
    assert!(in_progress.text.is_empty());

    let caret = TextCaret::new(0, TextAffinity::After);
    assert_eq!(in_progress.caret_position(), caret);
}

#[test]
fn long_alternating_atomic_replacements_keep_only_the_nearest_128_units() {
    let mut state = TextEditState::new("a");
    for edit in 0..10_000 {
        state.set_selection(TextSelection::new(0, state.text.len()));
        state.insert_text(if edit % 2 == 0 { "b" } else { "a" });
    }
    assert_eq!(state.text, "a");

    for traversal in 1..=128 {
        assert!(state.undo());
        assert_eq!(state.text, if traversal % 2 == 0 { "a" } else { "b" });
    }
    assert!(!state.undo());
}
