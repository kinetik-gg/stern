#[allow(unused_imports)]
use super::{
    ActionBinding, ActionContext, ActionDescriptor, ActionPriority, ActionRouter,
    ActionRoutingContext, ComponentState, Duration, Key, KeyEvent, KeyState, KeyboardInput,
    Modifiers, NumericInputDraft, NumericScrubInputConfig, PathFieldConfig, PhysicalKey,
    PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect,
    RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue, Shortcut, Size,
    TextEditState, TextInputEvent, TextLayoutStore, TextSelection, TextWrapperCase, TimeInfo, Ui,
    UiInput, UiMemory, Vec2, VectorComponentLayout, VectorScrubInputConfig, ViewportInfo, WidgetId,
    action_descriptor, assert_f32_slice_eq, bind_global, classify_numeric_input_draft, ctrl,
    default_dark_theme, double_released_at, frame_context_at, has_selection_highlight,
    has_semantic_action, input_at, key_input, multi_line_text_field, numeric_input,
    numeric_scrub_input, path_field, physical_shortcut_event, pressed_at, released_at,
    render_text_wrapper, restore_text_draft, root_child, scrub_drag_at, shift, shortcut,
    shortcut_event, text_field, text_field_with_text_layouts, text_value, vector2_scrub_input,
    vector3_component_rects, vector3_scrub_input, vector4_scrub_input,
};

#[test]
fn ui_text_field_focus_handoff_stops_previous_owner_before_starting_new_owner() {
    let theme = default_dark_theme();
    let first = root_child("first");
    let second = root_child("second");
    let mut memory = UiMemory::new();
    memory.focus(first);
    memory.set_text_input_owner(first);
    let mut first_state = TextEditState::new("one");
    let mut second_state = TextEditState::new("two");

    let input = pressed_at(8.0, 40.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field(
        "first",
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut first_state,
        false,
    );
    ui.text_field(
        "second",
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut second_state,
        false,
    );
    let output = ui.finish_output();

    let stop_index = output
        .platform_requests
        .iter()
        .position(|request| matches!(request, PlatformRequest::StopTextInput))
        .expect("previous text input owner stopped");
    let second_start_index = output
        .platform_requests
        .iter()
        .position(|request| {
            matches!(
                request,
                PlatformRequest::StartTextInput { rect: Some(rect) }
                    if *rect == Rect::new(0.0, 32.0, 160.0, 24.0)
            )
        })
        .expect("new text input owner started");

    assert!(stop_index < second_start_index);
    assert_eq!(memory.focused(), Some(second));
    assert_eq!(memory.text_input_owner(), Some(second));
}

#[test]
fn ui_text_field_rerendering_current_owner_does_not_restart_platform_input() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 160.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), Some(field));
    assert_eq!(memory.text_input_owner(), Some(field));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
    )));
}

#[test]
fn ui_text_field_clicking_current_owner_does_not_restart_platform_input() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");

    let input = pressed_at(8.0, 8.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 160.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), Some(field));
    assert_eq!(memory.text_input_owner(), Some(field));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
    )));
}

#[test]
fn ui_text_field_dead_space_click_blurs_and_stops_input() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");

    let input = pressed_at(240.0, 120.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 160.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), None);
    assert_eq!(memory.text_input_owner(), None);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn ui_disabled_text_field_cannot_acquire_focus_or_text_ownership() {
    let theme = default_dark_theme();
    let active = root_child("active");
    let disabled = root_child("disabled");
    let mut memory = UiMemory::new();
    memory.focus(active);
    memory.set_text_input_owner(active);
    let mut active_state = TextEditState::new("active");
    let mut disabled_state = TextEditState::new("disabled");

    let input = pressed_at(8.0, 40.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field(
        "active",
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut active_state,
        false,
    );
    let disabled_output = ui.text_field(
        "disabled",
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut disabled_state,
        true,
    );
    let output = ui.finish_output();

    assert_eq!(
        disabled_output.widget.response.expect("response").id,
        disabled
    );
    assert_eq!(memory.focused(), Some(active));
    assert_eq!(memory.text_input_owner(), Some(active));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput {
            rect: Some(rect),
        } if *rect == Rect::new(0.0, 32.0, 160.0, 24.0)
    )));
    assert!(
        !output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn text_field_clipboard_requests_are_targeted_and_targeted_text_is_applied() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let other = WidgetId::from_key("other");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));
    let copy_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![shortcut_event("c")],
        },
        ..UiInput::default()
    };

    let copy = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &copy_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(copy.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));

    memory.begin_frame();
    let paste_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![shortcut_event("v")],
        },
        ..UiInput::default()
    };
    let paste = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &paste_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(paste.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::RequestClipboardText { target } if *target == id)
    }));

    memory.begin_frame();
    let clipboard_input = UiInput {
        clipboard_text: vec![
            kinetik_ui_core::ClipboardText::new(other, "wrong"),
            kinetik_ui_core::ClipboardText::new(id, "XY"),
        ],
        ..UiInput::default()
    };
    let applied = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &clipboard_input,
        &mut memory,
        &theme,
        false,
    );

    assert!(applied.changed);
    assert_eq!(state.text, "aXYd");
}

#[test]
fn focused_text_field_handles_physical_clipboard_shortcuts_with_mismatched_logical_keys() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);

    let mut copy_state = TextEditState::new("abcd");
    copy_state.set_selection(TextSelection::new(1, 3));
    let copy_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![physical_shortcut_event("j", PhysicalKey::KeyC)],
        },
        ..UiInput::default()
    };

    let copy = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut copy_state,
        &copy_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!copy.changed);
    assert_eq!(copy_state.text, "abcd");
    assert!(copy.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));

    memory.begin_frame();
    let mut cut_state = TextEditState::new("abcd");
    cut_state.set_selection(TextSelection::new(1, 3));
    let cut_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![physical_shortcut_event("q", PhysicalKey::KeyX)],
        },
        ..UiInput::default()
    };

    let cut = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut cut_state,
        &cut_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(cut.changed);
    assert_eq!(cut_state.text, "ad");
    assert!(cut.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));
    assert!(cut_state.undo());
    assert_eq!(cut_state.text, "abcd");

    memory.begin_frame();
    let mut paste_state = TextEditState::new("abcd");
    paste_state.set_caret(2);
    let paste_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![physical_shortcut_event("m", PhysicalKey::KeyV)],
        },
        ..UiInput::default()
    };

    let paste = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut paste_state,
        &paste_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!paste.changed);
    assert_eq!(paste_state.text, "abcd");
    assert!(paste.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::RequestClipboardText { target } if *target == id)
    }));
}

#[test]
fn unfocused_and_disabled_text_fields_ignore_clipboard_shortcuts_and_targeted_text() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![
                physical_shortcut_event("j", PhysicalKey::KeyC),
                physical_shortcut_event("q", PhysicalKey::KeyX),
                physical_shortcut_event("m", PhysicalKey::KeyV),
            ],
        },
        clipboard_text: vec![kinetik_ui_core::ClipboardText::new(id, "XY")],
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("abcd");
    unfocused_state.set_selection(TextSelection::new(1, 3));
    let unfocused = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.text, "abcd");
    assert!(
        !unfocused
            .widget
            .platform_requests
            .iter()
            .any(is_clipboard_platform_request)
    );

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("abcd");
    disabled_state.set_selection(TextSelection::new(1, 3));
    let disabled = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.text, "abcd");
    assert!(
        !disabled
            .widget
            .platform_requests
            .iter()
            .any(is_clipboard_platform_request)
    );
}

fn is_clipboard_platform_request(request: &PlatformRequest) -> bool {
    matches!(
        request,
        PlatformRequest::CopyToClipboard(_) | PlatformRequest::RequestClipboardText { .. }
    )
}

#[test]
fn focused_text_field_handles_backspace_delete_and_replacement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);

    let mut backspace_state = TextEditState::new("aéz");
    backspace_state.set_caret("aé".len());
    let backspace_input = UiInput {
        keyboard: key_input(Key::Backspace, Modifiers::default()),
        ..UiInput::default()
    };
    let backspace = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut backspace_state,
        &backspace_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(backspace.changed);
    assert_eq!(backspace_state.text, "az");
    assert_eq!(backspace_state.caret(), 1);

    memory.begin_frame();
    let mut delete_state = TextEditState::new("aéz");
    delete_state.set_caret(1);
    let delete_input = UiInput {
        keyboard: key_input(Key::Delete, Modifiers::default()),
        ..UiInput::default()
    };
    let delete = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut delete_state,
        &delete_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(delete.changed);
    assert_eq!(delete_state.text, "az");
    assert_eq!(delete_state.caret(), 1);

    memory.begin_frame();
    let mut replace_state = TextEditState::new("abcd");
    replace_state.set_selection(TextSelection::new(3, 1));
    let replace_input = UiInput {
        text_events: vec![TextInputEvent::Commit("XY".to_owned())],
        ..UiInput::default()
    };
    let replace = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut replace_state,
        &replace_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(replace.changed);
    assert_eq!(replace_state.text, "aXYd");
    assert_eq!(replace_state.caret(), 3);
}

#[test]
fn unfocused_and_disabled_text_fields_ignore_deletion_and_replacement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit("XY".to_owned())],
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![
                KeyEvent::new(
                    Key::Backspace,
                    KeyState::Pressed,
                    Modifiers::default(),
                    false,
                ),
                KeyEvent::new(Key::Delete, KeyState::Pressed, Modifiers::default(), false),
            ],
        },
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("abcd");
    unfocused_state.set_selection(TextSelection::new(1, 3));
    let unfocused = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.text, "abcd");
    assert_eq!(unfocused_state.selection, TextSelection::new(1, 3));

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("abcd");
    disabled_state.set_selection(TextSelection::new(1, 3));
    let disabled = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.text, "abcd");
    assert_eq!(disabled_state.selection, TextSelection::new(1, 3));
}

#[test]
fn targeted_clipboard_text_replaces_existing_selection_with_undo() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let other = WidgetId::from_key("other");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(3, 1));
    let input = UiInput {
        clipboard_text: vec![
            kinetik_ui_core::ClipboardText::new(other, "wrong"),
            kinetik_ui_core::ClipboardText::new(id, "é"),
        ],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "aéd");
    assert_eq!(state.caret(), "aé".len());
    assert!(state.undo());
    assert_eq!(state.text, "abcd");
    assert_eq!(state.selection, TextSelection::new(3, 1));
}
