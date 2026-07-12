use super::*;

#[test]
fn text_field_applies_input_while_focused() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("");
    let input = UiInput {
        text_events: vec![kinetik_ui_core::TextInputEvent::Commit("a".to_owned())],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "a");
}

#[test]
fn text_field_ignores_text_input_while_unfocused() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("base");
    let input = UiInput {
        text_events: vec![kinetik_ui_core::TextInputEvent::Commit(
            "ignored".to_owned(),
        )],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "base");
    assert!(output.widget.platform_requests.is_empty());
}

#[test]
fn text_field_applies_editing_shortcuts_only_while_focused() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abcd");
    let input = shortcut_input("a");

    let unfocused = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!unfocused.changed);
    assert_eq!(state.selection, TextSelection::new(4, 4));

    memory.focus(id);
    let focused = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!focused.changed);
    assert_eq!(state.selection, TextSelection::new(0, 4));
}

#[test]
fn text_field_single_line_input_drops_newlines_and_enter_key() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("");
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                Key::Enter,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        text_events: vec![kinetik_ui_core::TextInputEvent::Commit(
            "a\nb\r\nc".to_owned(),
        )],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "abc");
}

#[test]
fn text_field_copies_selected_text_through_platform_request() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));
    let input = shortcut_input("c");

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "abcd");
    assert!(output.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));
}

#[test]
fn text_field_cuts_selected_text_through_platform_request_and_undo() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));
    let input = shortcut_input("x");

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "ad");
    assert!(output.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));
    assert!(state.undo());
    assert_eq!(state.text, "abcd");
}

#[test]
fn text_field_requests_targeted_clipboard_text_on_paste() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("abcd");
    state.set_caret(2);
    let input = shortcut_input("v");

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "abcd");
    assert!(output.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::RequestClipboardText { target } if *target == id)
    }));
}

#[test]
fn text_field_switch_stops_previous_owner_before_starting_new_owner() {
    let theme = default_dark_theme();
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let mut first_state = TextEditState::new("one");
    let mut second_state = TextEditState::new("two");
    let mut memory = UiMemory::new();
    memory.focus(first);
    memory.set_text_input_owner(first);
    let mut input = input_at(4.0, 34.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);

    let first_output = text_field(
        first,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut first_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let second_output = text_field(
        second,
        Rect::new(0.0, 30.0, 80.0, 24.0),
        &mut second_state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(
        !first_output
            .widget
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );
    let stop_index = second_output
        .widget
        .platform_requests
        .iter()
        .position(|request| matches!(request, PlatformRequest::StopTextInput))
        .expect("previous text input stopped");
    let start_index = second_output
        .widget
        .platform_requests
        .iter()
        .position(|request| {
            matches!(request, PlatformRequest::StartTextInput { rect: Some(rect) } if *rect == Rect::new(0.0, 30.0, 80.0, 24.0))
    })
    .expect("new text input started");
    assert!(stop_index < start_index);
    assert_eq!(memory.text_input_owner(), Some(second));
}

#[test]
fn text_field_applies_only_targeted_clipboard_text() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let other = WidgetId::from_key("other");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("a");
    state.set_caret(1);
    let input = UiInput {
        clipboard_text: vec![
            ClipboardText::new(other, "wrong"),
            ClipboardText::new(id, "b\nc"),
        ],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "abc");
    assert!(state.undo());
    assert_eq!(state.text, "a");
}

#[test]
fn text_field_ignores_clipboard_text_for_other_target() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let other = WidgetId::from_key("other");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("a");
    state.set_caret(1);
    let input = UiInput {
        clipboard_text: vec![ClipboardText::new(other, "wrong")],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "a");
}

#[test]
fn clipboard_text_targets_focused_requesting_field() {
    let theme = default_dark_theme();
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let mut memory = UiMemory::new();
    memory.focus(second);
    let mut first_state = TextEditState::new("one");
    let mut second_state = TextEditState::new("two");
    second_state.set_caret(3);
    let input = UiInput {
        clipboard_text: vec![
            ClipboardText::new(first, " wrong"),
            ClipboardText::new(second, " pasted"),
        ],
        ..UiInput::default()
    };

    let first_output = text_field(
        first,
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut first_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let second_output = text_field(
        second,
        Rect::new(0.0, 30.0, 80.0, 24.0),
        &mut second_state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!first_output.changed);
    assert_eq!(first_state.text, "one");
    assert!(second_output.changed);
    assert_eq!(second_state.text, "two pasted");
}

#[test]
fn ui_text_field_losing_focus_to_non_text_stops_platform_text_input() {
    let theme = default_dark_theme();
    let field = WidgetId::from_key("root").child("field");
    let other = WidgetId::from_key("root").child("other");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");
    let mut input = input_at(104.0, 4.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);

    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 80.0, 24.0), &mut state, false);
    ui.focusable("other", Rect::new(100.0, 0.0, 80.0, 24.0), false);
    let press_output = ui.finish_output();
    assert!(
        press_output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );

    let mut input = input_at(104.0, 4.0);
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 80.0, 24.0), &mut state, false);
    ui.focusable("other", Rect::new(100.0, 0.0, 80.0, 24.0), false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), Some(other));
    assert_eq!(memory.text_input_owner(), None);
    assert!(
        !output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn text_field_places_caret_from_pointer_press_with_shaped_layout() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("text");
    let rect = Rect::new(0.0, 0.0, 180.0, 28.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abcdef");
    let mut text_layouts = TextLayoutStore::new();
    let mut input = input_at(rect.max_x() - 4.0, 12.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);

    let output = text_field_with_text_layouts(
        id,
        rect,
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
        Some(&mut text_layouts),
    );

    assert_eq!(state.caret(), state.text.len());
    assert!(
        output
            .widget
            .response
            .as_ref()
            .expect("text field response")
            .state
            .focused
    );
    assert!(!text_layouts.is_empty());
}

#[test]
fn multi_line_text_field_preserves_targeted_clipboard_newlines() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multiline");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("first");
    state.set_caret(5);
    let input = UiInput {
        clipboard_text: vec![ClipboardText::new(id, "\r\nsecond\rthird")],
        ..UiInput::default()
    };

    let output = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "first\nsecond\nthird");
    assert_eq!(output.visible_lines, 3);
}

#[test]
fn multi_line_text_field_accepts_enter_while_focused() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multiline");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("first");
    let input = UiInput {
        keyboard: kinetik_ui_core::KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                Key::Enter,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    };

    let output = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert!(state.text.ends_with('\n'));
    assert!(
        output
            .widget
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::ClipBegin { .. }))
    );
}

#[test]
fn multi_line_text_field_places_caret_on_clicked_line() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multiline");
    let rect = Rect::new(0.0, 0.0, 180.0, 80.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("one\ntwo");
    let mut text_layouts = TextLayoutStore::new();
    let mut input = input_at(rect.max_x() - 4.0, 42.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);

    multi_line_text_field_with_text_layouts(
        id,
        rect,
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
        Some(&mut text_layouts),
    );

    assert_eq!(state.caret(), state.text.len());
}

#[test]
fn numeric_input_reports_parse_state() {
    let theme = default_dark_theme();
    let mut state = TextEditState::new("42");
    let output = numeric_input(
        WidgetId::from_key("number"),
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
        false,
    );

    assert!(output.valid);
    assert_eq!(output.value, Some(42.0));
}

#[test]
fn search_field_reports_query() {
    let theme = default_dark_theme();
    let mut state = TextEditState::new("media");
    let output = search_field(
        WidgetId::from_key("search"),
        Rect::new(0.0, 0.0, 80.0, 24.0),
        &mut state,
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
        false,
    );

    assert_eq!(output.query, "media");
    assert!(!output.empty);
}

#[test]
fn text_and_search_fields_expose_semantic_role_label_focus_and_value() {
    let theme = default_dark_theme();
    let field = WidgetId::from_key("field");
    let search = WidgetId::from_key("search");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut field_state = TextEditState::new("Project");
    let mut search_state = TextEditState::new("media");

    let field_output = text_field(
        field,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut field_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    let search_output = search_field(
        search,
        Rect::new(0.0, 30.0, 120.0, 24.0),
        &mut search_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );

    let field_node = &field_output.widget.semantics[0];
    assert_eq!(field_node.role, SemanticRole::TextField);
    assert_eq!(field_node.label.as_deref(), Some("Text field"));
    assert!(field_node.focusable);
    assert!(field_node.state.focused);
    assert!(
        matches!(field_node.state.value, Some(SemanticValue::Text(ref text)) if text == "Project")
    );

    let search_node = &search_output.field.widget.semantics[0];
    assert_eq!(search_node.role, SemanticRole::SearchField);
    assert_eq!(search_node.label.as_deref(), Some("Search"));
    assert!(search_node.focusable);
    assert!(!search_node.state.focused);
    assert!(
        matches!(search_node.state.value, Some(SemanticValue::Text(ref text)) if text == "media")
    );
}

#[test]
fn widget_semantics_map_roles_states_values_and_actions() {
    let button = button_semantics(
        WidgetId::from_key("button"),
        Rect::new(0.0, 0.0, 80.0, 24.0),
        "Analyze",
        false,
    );
    let checkbox = checkbox_semantics(
        WidgetId::from_key("checkbox"),
        Rect::new(0.0, 28.0, 20.0, 20.0),
        "Enabled",
        true,
        false,
    );
    let slider = slider_semantics(
        WidgetId::from_key("slider"),
        Rect::new(0.0, 56.0, 100.0, 12.0),
        "Strength",
        0.62,
        0.0..=1.0,
        false,
    );
    let field = text_field_semantics(
        WidgetId::from_key("field"),
        Rect::new(0.0, 72.0, 120.0, 24.0),
        "Name",
        "Project",
        false,
    );
    let search = search_field_semantics(
        WidgetId::from_key("search"),
        Rect::new(0.0, 100.0, 120.0, 24.0),
        "Search",
        "media",
        false,
    );
    let panel = panel_semantics(
        WidgetId::from_key("panel"),
        Rect::new(0.0, 0.0, 200.0, 200.0),
        "Inspector",
    );

    assert_eq!(button.role, SemanticRole::Button);
    assert!(button.focusable);
    assert!(
        button
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Invoke)
    );
    assert_eq!(checkbox.state.checked, Some(true));
    assert!(matches!(
        slider.state.value,
        Some(SemanticValue::Number { current, .. }) if (current - 0.62).abs() < f32::EPSILON
    ));
    assert!(matches!(field.state.value, Some(SemanticValue::Text(ref text)) if text == "Project"));
    assert_eq!(search.role, SemanticRole::SearchField);
    assert_eq!(panel.role, SemanticRole::Panel);
}

#[test]
fn canonical_replay_resolves_impossible_equal_ordinals_pointer_first() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        ResolvedTextPointerAction, TextPointerPhase, replay_text_field_events,
    };
    use kinetik_ui_core::{OrderedTextInputEvent, TextInputEvent, UiInputEvent};
    use kinetik_ui_text::TextEditMode;

    let mut state = TextEditState::new("ab");
    let result = replay_text_field_events(
        &mut state,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("field"),
        false,
        0,
        None,
        vec![ResolvedTextPointerAction {
            ordinal: Some(7),
            phase: TextPointerPhase::Press,
            model_caret: Some(kinetik_ui_text::TextCaret::at(1)),
            click_count: 1,
            modifiers: Modifiers::default(),
            release_clicked: false,
        }],
        vec![OrderedTextInputEvent {
            ordinal: Some(7),
            event: UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        }],
    );

    assert!(result.accepted_press);
    assert_eq!(state.text, "aXb");
    assert_eq!(state.selection, TextSelection::new(2, 2));
}

#[test]
fn canonical_replay_retains_one_outer_focus_loss_fence() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        ResolvedTextPointerAction, TextPointerPhase, replay_text_field_events,
    };
    use kinetik_ui_core::{OrderedTextInputEvent, TextInputEvent, UiInputEvent};
    use kinetik_ui_text::TextEditMode;

    let mut state = TextEditState::new("ab");
    let result = replay_text_field_events(
        &mut state,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("field"),
        true,
        2,
        None,
        vec![ResolvedTextPointerAction {
            ordinal: Some(3),
            phase: TextPointerPhase::Press,
            model_caret: Some(kinetik_ui_text::TextCaret::at(0)),
            click_count: 1,
            modifiers: Modifiers::default(),
            release_clicked: false,
        }],
        vec![
            OrderedTextInputEvent {
                ordinal: Some(1),
                event: UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            },
            OrderedTextInputEvent {
                ordinal: Some(2),
                event: UiInputEvent::WindowFocusChanged(false),
            },
            OrderedTextInputEvent {
                ordinal: Some(4),
                event: UiInputEvent::WindowFocusChanged(true),
            },
            OrderedTextInputEvent {
                ordinal: Some(5),
                event: UiInputEvent::Text(TextInputEvent::Commit("Y".to_owned())),
            },
        ],
    );

    assert!(result.focus_lost);
    assert!(!result.accepted_press);
    assert_eq!(state.text, "abX");
    assert_eq!(state.selection, TextSelection::new(3, 3));
}

#[test]
fn canonical_replay_retains_press_anchor_across_an_interleaved_edit() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        ResolvedTextPointerAction, TextPointerPhase, replay_text_field_events,
    };
    use kinetik_ui_core::{OrderedTextInputEvent, TextInputEvent, UiInputEvent};
    use kinetik_ui_text::TextEditMode;

    let mut state = TextEditState::new("abcd");
    let result = replay_text_field_events(
        &mut state,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("field"),
        false,
        4,
        None,
        vec![
            ResolvedTextPointerAction {
                ordinal: Some(1),
                phase: TextPointerPhase::Press,
                model_caret: Some(kinetik_ui_text::TextCaret::at(1)),
                click_count: 1,
                modifiers: Modifiers::default(),
                release_clicked: false,
            },
            ResolvedTextPointerAction {
                ordinal: Some(3),
                phase: TextPointerPhase::Move,
                model_caret: Some(kinetik_ui_text::TextCaret::at(4)),
                click_count: 0,
                modifiers: Modifiers::default(),
                release_clicked: false,
            },
        ],
        vec![OrderedTextInputEvent {
            ordinal: Some(2),
            event: UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        }],
    );

    assert!(result.accepted_press);
    assert_eq!(state.text, "aXbcd");
    assert_eq!(state.selection, TextSelection::new(1, 4));
}

#[test]
fn canonical_place_caret_is_release_ordered_pointer_first_without_press_activation() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        ResolvedTextPointerAction, TextPointerPhase, replay_text_field_events,
    };
    use kinetik_ui_core::{OrderedTextInputEvent, TextInputEvent, UiInputEvent};
    use kinetik_ui_text::TextEditMode;

    let mut state = TextEditState::new("ab");
    let result = replay_text_field_events(
        &mut state,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("field"),
        false,
        0,
        None,
        vec![
            ResolvedTextPointerAction {
                ordinal: Some(1),
                phase: TextPointerPhase::OwnershipPress,
                model_caret: Some(kinetik_ui_text::TextCaret::at(0)),
                click_count: 1,
                modifiers: Modifiers::default(),
                release_clicked: false,
            },
            ResolvedTextPointerAction {
                ordinal: Some(7),
                phase: TextPointerPhase::PlaceCaret,
                model_caret: Some(kinetik_ui_text::TextCaret::at(1)),
                click_count: 1,
                modifiers: Modifiers::default(),
                release_clicked: true,
            },
        ],
        vec![
            OrderedTextInputEvent {
                ordinal: Some(2),
                event: UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            },
            OrderedTextInputEvent {
                ordinal: Some(7),
                event: UiInputEvent::Text(TextInputEvent::Commit("Y".to_owned())),
            },
        ],
    );

    assert!(!result.accepted_press);
    assert_eq!(state.text, "aYb");
    assert_eq!(state.selection, TextSelection::new(2, 2));
}

#[test]
fn focus_loss_fences_place_caret_and_release_metadata_drives_selection() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        ResolvedTextPointerAction, TextPointerPhase, replay_text_field_events,
    };
    use kinetik_ui_core::{OrderedTextInputEvent, TextInputEvent, UiInputEvent};
    use kinetik_ui_text::TextEditMode;

    let mut fenced = TextEditState::new("alpha beta");
    let result = replay_text_field_events(
        &mut fenced,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("fenced"),
        false,
        0,
        None,
        vec![ResolvedTextPointerAction {
            ordinal: Some(2),
            phase: TextPointerPhase::PlaceCaret,
            model_caret: Some(kinetik_ui_text::TextCaret::at(7)),
            click_count: 2,
            modifiers: Modifiers::new(true, false, false, false),
            release_clicked: true,
        }],
        vec![
            OrderedTextInputEvent {
                ordinal: Some(1),
                event: UiInputEvent::WindowFocusChanged(false),
            },
            OrderedTextInputEvent {
                ordinal: Some(3),
                event: UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            },
        ],
    );
    assert!(result.focus_lost);
    assert_eq!(fenced.text, "alpha beta");
    assert_eq!(fenced.selection, TextSelection::new(10, 10));

    let mut selected = TextEditState::new("alpha beta");
    let result = replay_text_field_events(
        &mut selected,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("selected"),
        false,
        0,
        None,
        vec![ResolvedTextPointerAction {
            ordinal: Some(2),
            phase: TextPointerPhase::PlaceCaret,
            model_caret: Some(kinetik_ui_text::TextCaret::at(7)),
            click_count: 2,
            modifiers: Modifiers::new(true, false, false, false),
            release_clicked: true,
        }],
        Vec::new(),
    );
    assert!(!result.accepted_press);
    assert_eq!(selected.selected_text(), Some("beta"));

    let mut shifted = TextEditState::new("alpha beta");
    shifted.set_caret(0);
    let _ = replay_text_field_events(
        &mut shifted,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("shifted"),
        false,
        0,
        None,
        vec![ResolvedTextPointerAction {
            ordinal: Some(2),
            phase: TextPointerPhase::PlaceCaret,
            model_caret: Some(kinetik_ui_text::TextCaret::at(5)),
            click_count: 1,
            modifiers: Modifiers::new(true, false, false, false),
            release_clicked: true,
        }],
        Vec::new(),
    );
    assert_eq!(shifted.selection, TextSelection::new(0, 5));
}

#[test]
fn shaped_replay_validation_failure_consumes_without_scalar_fallback() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        TextNavigationResolution, replay_text_field_events_with_navigation,
    };
    use kinetik_ui_core::{Key, KeyEvent, KeyState, OrderedTextInputEvent, UiInputEvent};
    use kinetik_ui_text::TextEditMode;

    let mut state = TextEditState::new("abc");
    let expected = state.clone();
    let mut resolver_calls = 0;
    let result = replay_text_field_events_with_navigation(
        &mut state,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("field"),
        true,
        3,
        None,
        Vec::new(),
        vec![OrderedTextInputEvent {
            ordinal: Some(0),
            event: UiInputEvent::Key(KeyEvent::new(
                Key::ArrowLeft,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )),
        }],
        true,
        |_| {
            resolver_calls += 1;
            TextNavigationResolution::Invalid
        },
    );

    assert_eq!(resolver_calls, 1);
    assert_eq!(state, expected);
    assert_eq!(
        result,
        crate::components::text_interaction::TextReplayResult::default()
    );
}

#[test]
fn configured_unavailable_navigation_is_fail_closed_but_no_store_keeps_legacy_preedit() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        TextNavigationResolution, replay_text_field_events,
        replay_text_field_events_with_navigation,
    };
    use kinetik_ui_core::{Key, KeyEvent, KeyState, OrderedTextInputEvent, UiInputEvent};
    use kinetik_ui_text::{TextComposition, TextEditMode};

    let event = OrderedTextInputEvent {
        ordinal: Some(0),
        event: UiInputEvent::Key(KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )),
    };

    let mut unavailable = TextEditState::new("abc");
    let unavailable_expected = unavailable.clone();
    let _ = replay_text_field_events_with_navigation(
        &mut unavailable,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("configured"),
        true,
        3,
        None,
        Vec::new(),
        vec![event.clone()],
        true,
        |_| TextNavigationResolution::Unavailable,
    );
    assert_eq!(unavailable, unavailable_expected);

    let mut legacy = TextEditState::new("abc");
    legacy.composition = Some(TextComposition::new("候", None));
    let _ = replay_text_field_events(
        &mut legacy,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("legacy"),
        true,
        3,
        None,
        Vec::new(),
        vec![event],
    );
    assert_eq!(legacy.caret(), 2);
    assert!(legacy.composition.is_some());
}

#[test]
fn active_preedit_horizontal_replay_never_invokes_the_navigation_resolver() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        TextNavigationResolution, replay_text_field_events_with_navigation,
    };
    use kinetik_ui_core::{Key, KeyEvent, KeyState, OrderedTextInputEvent, UiInputEvent};
    use kinetik_ui_text::{TextComposition, TextEditMode};

    let mut state = TextEditState::new("abc");
    state.set_caret(1);
    state.composition = Some(TextComposition::new("候補", None));
    let expected = state.clone();
    let mut resolver_calls = 0;
    let events = [
        (Key::ArrowLeft, Modifiers::new(true, true, false, false)),
        (Key::ArrowRight, Modifiers::new(false, false, true, false)),
    ]
    .into_iter()
    .enumerate()
    .map(|(ordinal, (key, modifiers))| OrderedTextInputEvent {
        ordinal: Some(ordinal),
        event: UiInputEvent::Key(KeyEvent::new(key, KeyState::Pressed, modifiers, false)),
    })
    .collect();

    let result = replay_text_field_events_with_navigation(
        &mut state,
        TextFieldAccess::Editable,
        TextEditMode::SingleLine,
        WidgetId::from_key("field"),
        true,
        1,
        None,
        Vec::new(),
        events,
        true,
        |_| {
            resolver_calls += 1;
            TextNavigationResolution::Invalid
        },
    );

    assert_eq!(resolver_calls, 0);
    assert_eq!(state, expected);
    assert_eq!(
        result,
        crate::components::text_interaction::TextReplayResult::default()
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn shaped_replay_reresolves_after_every_mutation_class() {
    use crate::TextFieldAccess;
    use crate::components::text_interaction::{
        TextNavigationResolution, replay_text_field_events_with_navigation,
    };
    use kinetik_ui_core::{
        ClipboardText, Key, KeyEvent, KeyState, OrderedTextInputEvent, PhysicalKey, TextInputEvent,
        UiInputEvent,
    };
    use kinetik_ui_text::{
        CosmicTextEngine, TextAffinity, TextCaret, TextEditMode, TextLayoutKey, TextStyle,
    };

    struct Case {
        name: &'static str,
        initial: TextEditState,
        mutation: Vec<UiInputEvent>,
    }

    let target = WidgetId::from_key("field");
    let source = "abc אבג def";
    let at_bidi = || {
        let mut state = TextEditState::new(source);
        state.set_caret_position(TextCaret::new(8, TextAffinity::After));
        state
    };
    let hardware = UiInputEvent::Key(
        KeyEvent::with_physical_key(
            Key::Character("X".to_owned()),
            PhysicalKey::Unidentified,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )
        .with_text("X"),
    );
    let mut undo = at_bidi();
    undo.insert_text("X");
    let exposes_shaped_difference = |state: &TextEditState| {
        let mut engine = CosmicTextEngine::new();
        let layout = engine.shape_text(&TextLayoutKey::new(
            state.text.clone(),
            TextStyle::new("Inter", 14.0, 20.0),
            400.0,
            false,
        ));
        let navigation = layout.navigation(&state.text).expect("redo navigation");
        [
            (Key::ArrowLeft, Modifiers::default(), false),
            (Key::ArrowRight, Modifiers::default(), false),
            (
                Key::ArrowLeft,
                Modifiers::new(false, true, false, false),
                true,
            ),
            (
                Key::ArrowRight,
                Modifiers::new(false, true, false, false),
                true,
            ),
        ]
        .into_iter()
        .any(|(direction, modifiers, word)| {
            let arrow = KeyEvent::new(direction.clone(), KeyState::Pressed, modifiers, false);
            let mut visual = state.clone();
            let _ = visual.apply_visual_navigation_key(&arrow, &navigation);
            let mut scalar = state.clone();
            match (direction, word) {
                (Key::ArrowLeft, false) => scalar.move_left(),
                (Key::ArrowRight, false) => scalar.move_right(),
                (Key::ArrowLeft, true) => scalar.move_word_left(),
                (Key::ArrowRight, true) => scalar.move_word_right(),
                _ => unreachable!(),
            }
            visual.caret_position() != scalar.caret_position()
        })
    };
    let mut base_engine = CosmicTextEngine::new();
    let base_layout = base_engine.shape_text(&TextLayoutKey::new(
        source,
        TextStyle::new("Inter", 14.0, 20.0),
        400.0,
        false,
    ));
    let base_navigation = base_layout.navigation(source).expect("base navigation");
    let redo = base_navigation
        .caret_stops()
        .iter()
        .find_map(|stop| {
            let mut inserted = TextEditState::new(source);
            inserted.set_caret_position(stop.caret);
            inserted.insert_text("X");
            if !exposes_shaped_difference(&inserted) {
                return None;
            }
            let mut redo = inserted;
            assert!(redo.undo());
            Some(redo)
        })
        .expect("redo restoration exposes shaped/scalar difference");
    let cases = vec![
        Case {
            name: "hardware text",
            initial: at_bidi(),
            mutation: vec![hardware],
        },
        Case {
            name: "committed text",
            initial: at_bidi(),
            mutation: vec![UiInputEvent::Text(TextInputEvent::Commit("X".to_owned()))],
        },
        Case {
            name: "IME commit",
            initial: at_bidi(),
            mutation: vec![
                UiInputEvent::Text(TextInputEvent::CompositionStart),
                UiInputEvent::Text(TextInputEvent::Composition {
                    text: "候".to_owned(),
                    selection: None,
                }),
                UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            ],
        },
        Case {
            name: "Backspace",
            initial: at_bidi(),
            mutation: vec![UiInputEvent::Key(KeyEvent::new(
                Key::Backspace,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            ))],
        },
        Case {
            name: "Delete",
            initial: at_bidi(),
            mutation: vec![UiInputEvent::Key(KeyEvent::new(
                Key::Delete,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            ))],
        },
        Case {
            name: "targeted paste",
            initial: at_bidi(),
            mutation: vec![UiInputEvent::ClipboardText(ClipboardText::new(target, "X"))],
        },
        Case {
            name: "undo",
            initial: undo,
            mutation: vec![UiInputEvent::Key(KeyEvent::new(
                Key::Character("z".to_owned()),
                KeyState::Pressed,
                Modifiers::new(false, true, false, false),
                false,
            ))],
        },
        Case {
            name: "redo",
            initial: redo,
            mutation: vec![UiInputEvent::Key(KeyEvent::new(
                Key::Character("y".to_owned()),
                KeyState::Pressed,
                Modifiers::new(false, true, false, false),
                false,
            ))],
        },
    ];

    for case in cases {
        let mut candidate_engine = CosmicTextEngine::new();
        let candidate_layout = candidate_engine.shape_text(&TextLayoutKey::new(
            case.initial.text.clone(),
            TextStyle::new("Inter", 14.0, 20.0),
            400.0,
            false,
        ));
        let candidate_navigation = candidate_layout
            .navigation(&case.initial.text)
            .expect("initial navigation");
        let mut candidate_initials = vec![case.initial.clone()];
        candidate_initials.extend(candidate_navigation.caret_stops().iter().map(|stop| {
            let mut candidate = case.initial.clone();
            candidate.set_caret_position(stop.caret);
            candidate
        }));

        let (initial, post_mutation, expected, arrow) = candidate_initials
            .into_iter()
            .find_map(|initial| {
                let mut post_mutation = initial.clone();
                let _ = post_mutation.apply_ordered_input_with_result(
                    &case.mutation,
                    target,
                    TextEditMode::SingleLine,
                );
                let mut engine = CosmicTextEngine::new();
                let layout = engine.shape_text(&TextLayoutKey::new(
                    post_mutation.text.clone(),
                    TextStyle::new("Inter", 14.0, 20.0),
                    400.0,
                    false,
                ));
                let navigation = layout
                    .navigation(&post_mutation.text)
                    .expect("post-mutation navigation");
                [
                    (Key::ArrowLeft, Modifiers::default(), false),
                    (Key::ArrowRight, Modifiers::default(), false),
                    (
                        Key::ArrowLeft,
                        Modifiers::new(false, true, false, false),
                        true,
                    ),
                    (
                        Key::ArrowRight,
                        Modifiers::new(false, true, false, false),
                        true,
                    ),
                ]
                .into_iter()
                .find_map(|(direction, modifiers, word)| {
                    let arrow =
                        KeyEvent::new(direction.clone(), KeyState::Pressed, modifiers, false);
                    let mut visual = post_mutation.clone();
                    let _ = visual.apply_visual_navigation_key(&arrow, &navigation);
                    let mut scalar = post_mutation.clone();
                    match (direction, word) {
                        (Key::ArrowLeft, false) => scalar.move_left(),
                        (Key::ArrowRight, false) => scalar.move_right(),
                        (Key::ArrowLeft, true) => scalar.move_word_left(),
                        (Key::ArrowRight, true) => scalar.move_word_right(),
                        _ => unreachable!(),
                    }
                    (visual.caret_position() != scalar.caret_position()).then_some((
                        initial.clone(),
                        post_mutation.clone(),
                        visual,
                        arrow,
                    ))
                })
            })
            .unwrap_or_else(|| {
                panic!(
                    "{} must expose a shaped/scalar directional witness",
                    case.name
                )
            });

        let mut ordered = case
            .mutation
            .into_iter()
            .enumerate()
            .map(|(ordinal, event)| OrderedTextInputEvent {
                ordinal: Some(ordinal),
                event,
            })
            .collect::<Vec<_>>();
        ordered.push(OrderedTextInputEvent {
            ordinal: Some(ordered.len()),
            event: UiInputEvent::Key(arrow),
        });
        let mut actual = initial;
        let entry_anchor = actual.selection.anchor;
        let mut resolved_sources = Vec::new();
        let _ = replay_text_field_events_with_navigation(
            &mut actual,
            TextFieldAccess::Editable,
            TextEditMode::SingleLine,
            target,
            true,
            entry_anchor,
            None,
            Vec::new(),
            ordered,
            true,
            |state| {
                resolved_sources.push(state.text.clone());
                let mut engine = CosmicTextEngine::new();
                let layout = engine.shape_text(&TextLayoutKey::new(
                    state.text.clone(),
                    TextStyle::new("Inter", 14.0, 20.0),
                    400.0,
                    false,
                ));
                layout.navigation(&state.text).map_or(
                    TextNavigationResolution::Invalid,
                    TextNavigationResolution::Ready,
                )
            },
        );
        assert_eq!(resolved_sources, [post_mutation.text], "{}", case.name);
        assert_eq!(actual, expected, "{}", case.name);
    }
}

#[test]
fn fallback_wrapped_geometry_shares_rows_for_paint_hit_selection_caret_and_extent() {
    use crate::components::text_geometry::{TextFieldGeometry, TextFieldKind};
    use kinetik_ui_core::{Brush, ComponentState};

    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: false,
        pressed: false,
        focused: true,
        disabled: false,
        selected: false,
    });
    let line_height = recipe.font.line_height.max(1.0);
    let rect = Rect::new(
        0.0,
        0.0,
        recipe.padding_x * 2.0 + 9.0,
        recipe.padding_y * 2.0 + line_height * 2.0,
    );
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));
    let geometry = TextFieldGeometry::build(
        rect,
        &state,
        &recipe,
        TextFieldKind::WrappedMultiLine,
        kinetik_ui_core::Vec2::ZERO,
        None,
    );
    let primitives = geometry.primitives(WidgetId::from_key("field"), true, true, true);
    let fallback_rows = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) if text.layout.is_none() => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let selection_rows = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect) if rect.fill == Some(recipe.selection)
            )
        })
        .count();
    let painted_caret = primitives.iter().find_map(|primitive| match primitive {
        Primitive::Rect(rect) if rect.fill == Some(Brush::Solid(recipe.caret)) => Some(rect.rect),
        _ => None,
    });
    let third_row_hit = geometry.model_caret_at(Point::new(
        rect.x + recipe.padding_x + 1.0,
        rect.y + recipe.padding_y + line_height * 2.0 + 1.0,
    ));

    assert_eq!(fallback_rows, ["a", "b", "c", "d"]);
    assert_eq!(third_row_hit.offset, 2);
    assert_eq!(selection_rows, 2);
    assert_approx(geometry.caret_content_rect().y, line_height * 3.0);
    assert_eq!(
        painted_caret,
        Some(
            geometry
                .caret_content_rect()
                .translate(kinetik_ui_core::Vec2::new(
                    recipe.padding_x,
                    recipe.padding_y,
                ))
        )
    );
    assert_approx(geometry.viewport().content_size().height, line_height * 4.0);
    assert!(geometry.viewport().content_size().height > geometry.viewport().viewport_size().height);
}

#[test]
fn preedit_model_selection_keeps_insertion_leading_affinity() {
    use crate::components::text_geometry::{TextFieldGeometry, TextFieldKind};
    use kinetik_ui_core::ComponentState;
    use kinetik_ui_text::TextComposition;

    let theme = default_dark_theme();
    let recipe = theme.text_field(ComponentState {
        hovered: false,
        pressed: false,
        focused: true,
        disabled: false,
        selected: false,
    });
    let rect = Rect::new(0.0, 0.0, 160.0, 24.0);
    let selection_width = |selection| {
        let mut state = TextEditState::new("ab");
        state.set_selection(selection);
        state.composition = Some(TextComposition::new("XY", None));
        TextFieldGeometry::build(
            rect,
            &state,
            &recipe,
            TextFieldKind::SingleLine,
            kinetik_ui_core::Vec2::ZERO,
            None,
        )
        .primitives(WidgetId::from_key("field"), true, true, true)
        .into_iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill == Some(recipe.selection) => Some(rect.rect.width),
            _ => None,
        })
        .sum::<f32>()
    };
    let char_width = (recipe.font.size * 0.55).max(1.0);

    assert_approx(selection_width(TextSelection::new(0, 1)), char_width);
    assert_approx(selection_width(TextSelection::new(2, 1)), char_width * 3.0);
}

#[test]
fn vector_runtime_final_press_is_independent_of_component_helper_order() {
    use crate::{
        VectorComponentLayout, VectorScrubInputConfig, vector_scrub_input_with_runtime,
        vector2_component_rects,
    };
    use kinetik_ui_core::{
        FrameContext, MouseButton, PhysicalSize, ScaleFactor, Size, TextInputEvent, TimeInfo,
        Ui as CoreUi, UiInputEvent, ViewportInfo,
    };

    fn run(reversed: bool) -> ([f32; 2], [TextEditState; 2], Option<WidgetId>, usize, bool) {
        let theme = default_dark_theme();
        let rect = Rect::new(0.0, 0.0, 220.0, 24.0);
        let mut component_rects = vector2_component_rects(rect, VectorComponentLayout::default());
        let x = component_rects[0].value_rect.center();
        let y = component_rects[1].value_rect.center();
        if reversed {
            component_rects.reverse();
        }
        let mut input = UiInput::default();
        for event in [
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: true,
                click_count: 1,
                position: Some(x),
            },
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                click_count: 1,
                position: Some(Point::new(x.x + 1.0, x.y)),
            },
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: true,
                click_count: 1,
                position: Some(y),
            },
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                click_count: 1,
                position: Some(Point::new(y.x + 1.0, y.y)),
            },
            UiInputEvent::Text(TextInputEvent::Commit("Z".to_owned())),
        ] {
            input.push_event(event);
        }
        let context = FrameContext::new(
            ViewportInfo::new(
                Size::new(220.0, 24.0),
                PhysicalSize::new(220, 24),
                ScaleFactor::ONE,
            ),
            input,
            TimeInfo::default(),
        );
        let mut memory = UiMemory::new();
        let mut values = [1.0, 2.0];
        let mut states = [TextEditState::new("1"), TextEditState::new("2")];
        let vector_id = WidgetId::from_key("vector");
        let y_id = vector_id.child("Y");
        let mut runtime = CoreUi::begin_frame(context, &mut memory);
        let output = vector_scrub_input_with_runtime(
            &mut runtime,
            vector_id,
            "Offset",
            &mut values,
            &mut states,
            VectorScrubInputConfig::default(),
            &theme,
            None,
            true,
            component_rects,
        );
        assert_eq!(output.components.len(), 2);
        let frame = runtime.end_frame();
        let starts = frame
            .platform_requests
            .iter()
            .filter(|request| matches!(request, PlatformRequest::StartTextInput { rect: Some(_) }))
            .count();
        let claim_consumed = !memory.claim_text_input_events(y_id);
        (values, states, memory.focused(), starts, claim_consumed)
    }

    let normal = run(false);
    let reversed = run(true);
    for index in 0..2 {
        assert_approx(normal.0[index], reversed.0[index]);
    }
    assert_eq!(normal.1, reversed.1);
    assert_eq!(normal.2, reversed.2);
    assert_eq!(normal.3, reversed.3);
    assert_eq!(normal.4, reversed.4);
    assert_approx(normal.0[0], 1.0);
    assert_approx(normal.0[1], 2.0);
    assert_eq!(normal.1[0].text, "1");
    assert!(normal.1[1].text.contains('Z'));
    assert_eq!(normal.2, Some(WidgetId::from_key("vector").child("Y")));
    assert_eq!(normal.3, 1);
    assert!(normal.4);
}
