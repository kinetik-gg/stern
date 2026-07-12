use std::time::Duration;

use kinetik_ui_core::{
    ComponentState, FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalKey,
    PhysicalSize, PlatformRequest, Point, Primitive, Rect, ScaleFactor, Size, TextInputEvent,
    TextInputOwnerMode, TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2, ViewportInfo,
};
use kinetik_ui_text::{
    ShapedTextNavigation, TextAffinity, TextCaret, TextComposition, TextEditState, TextLayoutKey,
    TextLayoutStore, TextSelection, TextStyle,
};
use kinetik_ui_widgets::{TextFieldAccess, Ui};

use super::{default_dark_theme, root_child};

const FIELD: Rect = Rect::new(10.0, 8.0, 260.0, 32.0);
const MIXED: &str = "abc אבג def";

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 360.0),
            PhysicalSize::ZERO,
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::ZERO, Duration::from_millis(16), 0),
    )
}

fn canonical(events: impl IntoIterator<Item = UiInputEvent>) -> UiInput {
    let mut input = UiInput::default();
    for event in events {
        input.push_event(event);
    }
    input
}

fn horizontal(direction: Key, modifiers: Modifiers) -> UiInputEvent {
    UiInputEvent::Key(KeyEvent::new(
        direction,
        KeyState::Pressed,
        modifiers,
        false,
    ))
}

fn focused_memory(access: TextFieldAccess) -> UiMemory {
    let id = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    match access {
        TextFieldAccess::Editable | TextFieldAccess::Disabled => memory.set_text_input_owner(id),
        TextFieldAccess::ReadOnly => {
            memory.set_text_input_owner_mode(id, TextInputOwnerMode::ReadOnly);
        }
    }
    memory
}

fn navigation(
    store: &mut TextLayoutStore,
    source: &str,
    rect: Rect,
    wrap: bool,
    focused: bool,
) -> (kinetik_ui_core::TextLayoutId, ShapedTextNavigation) {
    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: false,
        pressed: false,
        focused,
        disabled: false,
        selected: false,
    });
    let id = store.layout_id(TextLayoutKey::new(
        source,
        TextStyle::new(
            recipe.font.family,
            recipe.font.size,
            recipe.font.line_height,
        ),
        (rect.width - recipe.padding_x * 2.0).max(0.0),
        wrap,
    ));
    let layout = store.layout(id).expect("retained shaped layout");
    let navigation = layout.navigation(source).unwrap_or_else(|error| {
        panic!(
            "valid retained navigation: {error:?}; lines={:?}; runs={:?}",
            layout.lines, layout.runs
        )
    });
    (id, navigation)
}

fn render(
    input: UiInput,
    memory: &mut UiMemory,
    store: &mut TextLayoutStore,
    state: &mut TextEditState,
    access: TextFieldAccess,
    rect: Rect,
    multiline: bool,
) -> kinetik_ui_core::FrameOutput {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame_with_text_layouts(context(input), memory, &theme, store);
    if multiline {
        ui.multi_line_text_field_with_access("field", rect, state, access);
    } else {
        ui.text_field_with_access("field", rect, state, access);
    }
    ui.finish_output()
}

#[test]
fn retained_horizontal_navigation_matches_for_editable_and_read_only_and_disables_cleanly() {
    for access in [TextFieldAccess::Editable, TextFieldAccess::ReadOnly] {
        let mut store = TextLayoutStore::new();
        let (_, navigation) = navigation(&mut store, MIXED, FIELD, false, true);
        let mut expected = TextEditState::new(MIXED);
        expected.set_caret_position(TextCaret::new(8, TextAffinity::After));
        let event = KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        );
        let outcome = expected.apply_visual_navigation_key(&event, &navigation);
        assert!(outcome.is_some());

        let mut state = TextEditState::new(MIXED);
        state.set_caret_position(TextCaret::new(8, TextAffinity::After));
        let original_text = state.text.clone();
        let mut memory = focused_memory(access);
        let output = render(
            canonical([UiInputEvent::Key(event)]),
            &mut memory,
            &mut store,
            &mut state,
            access,
            FIELD,
            false,
        );
        assert_eq!(state.caret_position(), expected.caret_position());
        assert_eq!(state.selection, expected.selection);
        assert_eq!(state.text, original_text);
        if access == TextFieldAccess::ReadOnly {
            assert!(!output.platform_requests.iter().any(|request| matches!(
                request,
                PlatformRequest::StartTextInput { .. }
                    | PlatformRequest::UpdateTextInputRect { .. }
            )));
        }
    }

    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new(MIXED);
    state.set_caret_position(TextCaret::new(8, TextAffinity::After));
    let expected = state.clone();
    let mut memory = focused_memory(TextFieldAccess::Disabled);
    let output = render(
        canonical([horizontal(Key::ArrowRight, Modifiers::default())]),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::Disabled,
        FIELD,
        false,
    );
    assert_eq!(state, expected);
    assert!(!memory.is_focused(root_child("field")));
    assert!(!memory.owns_text_input(root_child("field")));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::UpdateTextInputRect { .. }
    )));
}

#[test]
fn retained_read_only_field_copies_without_mutation_or_native_ime() {
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new(MIXED);
    state.set_selection(TextSelection::new(0, 3));
    let expected = state.clone();
    let mut memory = focused_memory(TextFieldAccess::ReadOnly);
    let output = render(
        canonical([UiInputEvent::Key(KeyEvent::with_physical_key(
            Key::Character("ignored".to_owned()),
            PhysicalKey::KeyC,
            KeyState::Pressed,
            Modifiers::new(false, true, false, false),
            false,
        ))]),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::ReadOnly,
        FIELD,
        false,
    );

    assert_eq!(state, expected);
    assert!(
        output
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.layout.is_some()))
    );
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::CopyToClipboard("abc".to_owned()))
    );
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. }
            | PlatformRequest::UpdateTextInputRect { .. }
            | PlatformRequest::RequestClipboardText { .. }
    )));
}

#[test]
fn ordered_commit_then_arrow_uses_the_post_mutation_source() {
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new(MIXED);
    state.set_caret_position(TextCaret::new(8, TextAffinity::After));
    let mut expected = state.clone();
    expected.insert_text("X");
    let (_, fresh) = navigation(&mut store, &expected.text, FIELD, false, true);
    let arrow = KeyEvent::new(
        Key::ArrowRight,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    );
    assert!(
        expected
            .apply_visual_navigation_key(&arrow, &fresh)
            .is_some()
    );
    let after_insert = TextCaret::new(9, TextAffinity::Before);
    assert_ne!(expected.caret_position(), after_insert);

    let mut memory = focused_memory(TextFieldAccess::Editable);
    let output = render(
        canonical([
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            UiInputEvent::Key(arrow),
        ]),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::Editable,
        FIELD,
        false,
    );
    assert_eq!(state, expected);
    assert!(output.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Text(text) if text.text == expected.text && text.layout.is_some())
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn mutation_pointer_key_uses_frozen_entry_hit_then_fresh_navigation() {
    #[derive(Clone, Copy)]
    enum PointerCase {
        Press,
        ShiftPress,
        Drag,
    }

    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: true,
        pressed: true,
        focused: true,
        disabled: false,
        selected: false,
    });
    let prefix = "LONG ";
    let post_source = format!("{prefix}{MIXED}");

    for pointer_case in [
        PointerCase::Press,
        PointerCase::ShiftPress,
        PointerCase::Drag,
    ] {
        let mut store = TextLayoutStore::new();
        let (_, entry_navigation) = navigation(&mut store, MIXED, FIELD, false, true);
        let (_, post_navigation) = navigation(&mut store, &post_source, FIELD, false, true);
        let (layout_x, entry_hit, post_hit) = (0_u16..=1040)
            .map(|step| f32::from(step) * 0.25)
            .find_map(|x| {
                let entry_hit = entry_navigation.hit_test_caret(x, 0.0);
                let post_hit = post_navigation.hit_test_caret(x, 0.0);
                let fresh_target = post_navigation.visual_right(entry_hit);
                let stale_target = entry_navigation.visual_right(entry_hit);
                (entry_hit != post_hit && fresh_target != entry_hit && fresh_target != stale_target)
                    .then_some((x, entry_hit, post_hit))
            })
            .expect("bundled-font coordinate distinguishes entry and post-mutation hits");
        assert_ne!(entry_hit, post_hit);

        let witness = Point::new(
            FIELD.x + recipe.padding_x + layout_x,
            FIELD.y + recipe.padding_y + recipe.font.size,
        );
        let entry_anchor_rect = entry_navigation.caret_rect(TextCaret::new(0, TextAffinity::After));
        let entry_anchor = Point::new(
            FIELD.x + recipe.padding_x + entry_anchor_rect.x,
            FIELD.y + recipe.padding_y + recipe.font.size,
        );

        let mut expected = TextEditState::new(MIXED);
        expected.set_caret(0);
        expected.insert_text(prefix);
        match pointer_case {
            PointerCase::Press => expected.set_caret_position(entry_hit),
            PointerCase::ShiftPress => expected.set_selection_with_affinity(
                TextSelection::new(0, entry_hit.offset),
                entry_hit.affinity,
            ),
            PointerCase::Drag => {
                expected.set_caret_position(TextCaret::new(0, TextAffinity::After));
                expected.set_selection_with_affinity(
                    TextSelection::new(0, entry_hit.offset),
                    entry_hit.affinity,
                );
            }
        }
        let arrow = KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        );
        assert!(
            expected
                .apply_visual_navigation_key(&arrow, &post_navigation)
                .is_some()
        );

        let mut rehit = TextEditState::new(&post_source);
        match pointer_case {
            PointerCase::Press => rehit.set_caret_position(post_hit),
            PointerCase::ShiftPress | PointerCase::Drag => rehit.set_selection_with_affinity(
                TextSelection::new(0, post_hit.offset),
                post_hit.affinity,
            ),
        }
        let _ = rehit.apply_visual_navigation_key(&arrow, &post_navigation);
        assert_ne!(
            rehit.caret_position(),
            expected.caret_position(),
            "replay-time re-hit must be observable"
        );

        let mut events = vec![UiInputEvent::Text(TextInputEvent::Commit(
            prefix.to_owned(),
        ))];
        match pointer_case {
            PointerCase::Press => events.push(UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: true,
                click_count: 1,
                position: Some(witness),
            }),
            PointerCase::ShiftPress => {
                events.push(UiInputEvent::ModifiersChanged(Modifiers::new(
                    true, false, false, false,
                )));
                events.push(UiInputEvent::PointerButton {
                    button: MouseButton::Primary,
                    down: true,
                    click_count: 1,
                    position: Some(witness),
                });
                events.push(UiInputEvent::ModifiersChanged(Modifiers::default()));
            }
            PointerCase::Drag => {
                events.push(UiInputEvent::PointerButton {
                    button: MouseButton::Primary,
                    down: true,
                    click_count: 1,
                    position: Some(entry_anchor),
                });
                events.push(UiInputEvent::PointerMoved {
                    position: witness,
                    delta: Vec2::new(witness.x - entry_anchor.x, witness.y - entry_anchor.y),
                });
            }
        }
        events.push(UiInputEvent::Key(arrow));

        let mut state = TextEditState::new(MIXED);
        state.set_caret(0);
        let mut memory = focused_memory(TextFieldAccess::Editable);
        let _ = render(
            canonical(events),
            &mut memory,
            &mut store,
            &mut state,
            TextFieldAccess::Editable,
            FIELD,
            false,
        );
        assert_eq!(state, expected);
    }
}

#[test]
fn shaped_pointer_fixtures_preserve_graphemes_affinity_and_registered_paint() {
    for source in ["Ae\u{301}B", "A👍🏽B", "A👩‍🚀B", "a->b", "אבג", MIXED] {
        let mut store = TextLayoutStore::new();
        let (layout_id, navigation) = navigation(&mut store, source, FIELD, false, true);
        let stop = navigation.caret_stops()[navigation.caret_stops().len() / 2];
        let caret_rect = navigation.caret_rect(stop.caret);
        let theme = default_dark_theme();
        let recipe = theme.text_field(ComponentState {
            hovered: true,
            pressed: true,
            focused: true,
            disabled: false,
            selected: false,
        });
        let point = Point::new(
            FIELD.x + recipe.padding_x + caret_rect.x,
            FIELD.y + recipe.padding_y + recipe.font.size + caret_rect.y + caret_rect.height * 0.5,
        );
        let expected =
            navigation.hit_test_caret(caret_rect.x, caret_rect.y + caret_rect.height * 0.5);
        let mut state = TextEditState::new(source);
        state.set_caret(0);
        let mut memory = focused_memory(TextFieldAccess::Editable);
        let output = render(
            canonical([UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: true,
                click_count: 1,
                position: Some(point),
            }]),
            &mut memory,
            &mut store,
            &mut state,
            TextFieldAccess::Editable,
            FIELD,
            false,
        );
        assert_eq!(state.caret_position(), expected, "source {source:?}");
        assert_eq!(
            TextSelection::new(expected.offset, expected.offset).clamp_to_text(source),
            TextSelection::new(expected.offset, expected.offset),
            "source {source:?}"
        );
        assert!(output.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Text(text) if text.layout == Some(layout_id)
        )));
    }
}

#[test]
fn mixed_bidi_selection_paint_is_exactly_the_navigation_rectangles() {
    let mut store = TextLayoutStore::new();
    let (layout_id, navigation) = navigation(&mut store, MIXED, FIELD, false, true);
    let mut state = TextEditState::new(MIXED);
    state.set_selection_with_affinity(TextSelection::new(4, 12), TextAffinity::Before);
    let mut memory = focused_memory(TextFieldAccess::Editable);
    let output = render(
        UiInput::default(),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::Editable,
        FIELD,
        false,
    );
    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: false,
        pressed: false,
        focused: true,
        disabled: false,
        selected: false,
    });
    let expected = navigation
        .selection_rects(4..12)
        .into_iter()
        .map(|rect| {
            rect.translate(Vec2::new(
                FIELD.x + recipe.padding_x,
                FIELD.y + recipe.padding_y + recipe.font.size,
            ))
        })
        .collect::<Vec<_>>();
    let painted = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill == Some(recipe.selection) => Some(rect.rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(painted, expected);
    assert!(output.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Text(text) if text.layout == Some(layout_id)
    )));
}

#[test]
fn wrapped_horizontal_seam_is_reversible_with_exact_affinity() {
    let rect = Rect::new(10.0, 8.0, 78.0, 96.0);
    let source = "alpha אבג beta gamma delta";
    let mut store = TextLayoutStore::new();
    let (_, navigation) = navigation(&mut store, source, rect, true, true);
    let (from, to, forward, backward) = navigation
        .caret_stops()
        .iter()
        .flat_map(|left| {
            navigation
                .caret_stops()
                .iter()
                .map(move |right| (*left, *right))
        })
        .find_map(|(left, right)| {
            if left.visual_line == right.visual_line || left.caret.offset != right.caret.offset {
                return None;
            }
            if navigation.visual_right(left.caret) == right.caret
                && navigation.visual_left(right.caret) == left.caret
            {
                Some((left.caret, right.caret, Key::ArrowRight, Key::ArrowLeft))
            } else if navigation.visual_left(left.caret) == right.caret
                && navigation.visual_right(right.caret) == left.caret
            {
                Some((left.caret, right.caret, Key::ArrowLeft, Key::ArrowRight))
            } else {
                None
            }
        })
        .expect("wrapped layout exposes a reversible same-offset seam");

    let mut state = TextEditState::new(source);
    state.set_caret_position(from);
    let mut memory = focused_memory(TextFieldAccess::Editable);
    let _ = render(
        canonical([horizontal(forward, Modifiers::default())]),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::Editable,
        rect,
        true,
    );
    assert_eq!(state.caret_position(), to);

    let _ = render(
        canonical([horizontal(backward, Modifiers::default())]),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::Editable,
        rect,
        true,
    );
    assert_eq!(state.caret_position(), from);
}

#[test]
#[allow(clippy::too_many_lines)]
fn shaped_pointer_hit_applies_retained_horizontal_and_vertical_offsets_once() {
    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: true,
        pressed: true,
        focused: true,
        disabled: false,
        selected: false,
    });

    let single_rect = Rect::new(10.0, 8.0, 92.0, 32.0);
    let single_source = "prefix abc אבג def suffix";
    let horizontal_offset = 40.0;
    let mut single_store = TextLayoutStore::new();
    let (_, single_navigation) =
        navigation(&mut single_store, single_source, single_rect, false, true);
    let single_content_width = single_rect.width - recipe.padding_x * 2.0;
    let (single_caret, single_layout_x) = single_navigation
        .caret_stops()
        .iter()
        .find_map(|stop| {
            let rect = single_navigation.caret_rect(stop.caret);
            let screen_x = rect.x - horizontal_offset;
            if !(4.0..single_content_width - 4.0).contains(&screen_x) {
                return None;
            }
            let correct = single_navigation.hit_test_caret(rect.x, rect.y + rect.height * 0.5);
            let zero = single_navigation
                .hit_test_caret(rect.x - horizontal_offset, rect.y + rect.height * 0.5);
            let twice = single_navigation
                .hit_test_caret(rect.x + horizontal_offset, rect.y + rect.height * 0.5);
            (correct != zero && correct != twice).then_some((correct, rect.x))
        })
        .expect("horizontal offset witness");
    let single_point = Point::new(
        single_rect.x + recipe.padding_x + single_layout_x - horizontal_offset,
        single_rect.y + recipe.padding_y + recipe.font.size,
    );
    let mut single_state = TextEditState::new(single_source);
    single_state.set_caret(0);
    let mut single_memory = focused_memory(TextFieldAccess::Editable);
    single_memory.set_scroll_offset(root_child("field"), Vec2::new(horizontal_offset, 0.0));
    let _ = render(
        canonical([UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(single_point),
        }]),
        &mut single_memory,
        &mut single_store,
        &mut single_state,
        TextFieldAccess::Editable,
        single_rect,
        false,
    );
    assert_eq!(single_state.caret_position(), single_caret);

    let multi_rect = Rect::new(10.0, 8.0, 92.0, 48.0);
    let multi_source = "alpha beta gamma delta אבג epsilon zeta eta theta";
    let vertical_offset = recipe.font.line_height;
    let mut multi_store = TextLayoutStore::new();
    let (_, multi_navigation) = navigation(&mut multi_store, multi_source, multi_rect, true, true);
    let content_height = multi_rect.height - recipe.padding_y * 2.0;
    let (multi_caret, multi_rect_caret) = multi_navigation
        .caret_stops()
        .iter()
        .find_map(|stop| {
            let rect = multi_navigation.caret_rect(stop.caret);
            let screen_y = recipe.font.size + rect.y - vertical_offset;
            if !(2.0..content_height - 2.0).contains(&screen_y) {
                return None;
            }
            let y = rect.y + rect.height * 0.5;
            let correct = multi_navigation.hit_test_caret(rect.x, y);
            let zero = multi_navigation.hit_test_caret(rect.x, y - vertical_offset);
            let twice = multi_navigation.hit_test_caret(rect.x, y + vertical_offset);
            (correct != zero && correct != twice).then_some((correct, rect))
        })
        .expect("vertical offset witness");
    let multi_point = Point::new(
        multi_rect.x + recipe.padding_x + multi_rect_caret.x,
        multi_rect.y + recipe.padding_y + recipe.font.size + multi_rect_caret.y - vertical_offset
            + multi_rect_caret.height * 0.5,
    );
    let mut multi_state = TextEditState::new(multi_source);
    multi_state.set_caret(0);
    let mut multi_memory = focused_memory(TextFieldAccess::Editable);
    multi_memory.set_scroll_offset(root_child("field"), Vec2::new(0.0, vertical_offset));
    let _ = render(
        canonical([UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(multi_point),
        }]),
        &mut multi_memory,
        &mut multi_store,
        &mut multi_state,
        TextFieldAccess::Editable,
        multi_rect,
        true,
    );
    assert_eq!(multi_state.caret_position(), multi_caret);
}

#[test]
fn retained_preedit_uses_display_navigation_and_suppresses_model_arrows() {
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new("ab");
    state.set_caret_position(TextCaret::new(1, TextAffinity::After));
    state.composition = Some(TextComposition {
        text: "e\u{301}o\u{301}".to_owned(),
        selection: Some(kinetik_ui_core::TextRange::new(2, 5)),
    });
    let expected_state = state.clone();
    let display = "ae\u{301}o\u{301}b";
    let (layout_id, navigation) = navigation(&mut store, display, FIELD, false, true);
    let display_caret = TextCaret::new(4, TextAffinity::After);
    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: false,
        pressed: false,
        focused: true,
        disabled: false,
        selected: false,
    });
    let expected_caret = navigation.caret_rect(display_caret).translate(Vec2::new(
        FIELD.x + recipe.padding_x,
        FIELD.y + recipe.padding_y + recipe.font.size,
    ));
    let content = Rect::new(
        FIELD.x + recipe.padding_x,
        FIELD.y + recipe.padding_y,
        FIELD.width - recipe.padding_x * 2.0,
        FIELD.height - recipe.padding_y * 2.0,
    );

    let mut memory = focused_memory(TextFieldAccess::Editable);
    let output = render(
        canonical([horizontal(Key::ArrowLeft, Modifiers::default())]),
        &mut memory,
        &mut store,
        &mut state,
        TextFieldAccess::Editable,
        FIELD,
        false,
    );
    assert_eq!(state, expected_state);
    assert!(output.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Text(text) if text.layout == Some(layout_id) && text.text == display
    )));
    let native = output
        .platform_requests
        .iter()
        .find_map(|request| match request {
            PlatformRequest::UpdateTextInputRect { rect }
            | PlatformRequest::StartTextInput { rect: Some(rect) } => Some(*rect),
            _ => None,
        });
    assert_eq!(native, content.intersection(expected_caret));
}
