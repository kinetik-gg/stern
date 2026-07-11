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
fn stage1_text_wrapper_matrix_exposes_semantic_contracts() {
    for case in [
        TextWrapperCase::TextField,
        TextWrapperCase::MultiLineTextField,
        TextWrapperCase::SearchField,
        TextWrapperCase::NumericInput,
    ] {
        let output = render_text_wrapper(case, false);
        let id = root_child(case.key());
        let node = output
            .semantics
            .get(id)
            .unwrap_or_else(|| panic!("{} semantic node", case.name()));

        assert_eq!(node.role, case.role(), "{}", case.name());
        assert_eq!(node.label.as_deref(), Some(case.label()), "{}", case.name());
        assert_eq!(node.bounds, case.rect(), "{}", case.name());
        assert!(node.focusable, "{}", case.name());
        assert!(!node.state.disabled, "{}", case.name());
        assert!(!node.state.focused, "{}", case.name());
        assert!(
            has_semantic_action(node, &SemanticActionKind::Focus),
            "{}",
            case.name()
        );
        assert!(
            has_semantic_action(node, &SemanticActionKind::SetText),
            "{}",
            case.name()
        );
        assert_eq!(
            node.state.value,
            Some(SemanticValue::Text(case.text().to_owned())),
            "{}",
            case.name()
        );
        assert_eq!(node.state.checked, None, "{}", case.name());
        assert!(!node.state.selected, "{}", case.name());

        let disabled_output = render_text_wrapper(case, true);
        let disabled_node = disabled_output
            .semantics
            .get(id)
            .unwrap_or_else(|| panic!("{} disabled semantic node", case.name()));
        assert_eq!(disabled_node.role, case.role(), "{}", case.name());
        assert_eq!(
            disabled_node.label.as_deref(),
            Some(case.label()),
            "{}",
            case.name()
        );
        assert!(disabled_node.state.disabled, "{}", case.name());
        assert!(!disabled_node.state.focused, "{}", case.name());
        assert!(!disabled_node.focusable, "{}", case.name());
        assert!(
            !has_semantic_action(disabled_node, &SemanticActionKind::Focus),
            "{}",
            case.name()
        );
        assert!(
            has_semantic_action(disabled_node, &SemanticActionKind::SetText),
            "{}",
            case.name()
        );
        assert_eq!(
            disabled_node.state.value,
            Some(SemanticValue::Text(case.text().to_owned())),
            "{}",
            case.name()
        );
    }
}

#[test]
fn focused_text_field_receives_text_and_unfocused_field_ignores_it() {
    let theme = default_dark_theme();
    let focused = WidgetId::from_key("focused");
    let unfocused = WidgetId::from_key("unfocused");
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit(" typed".to_owned())],
        ..UiInput::default()
    };
    let mut memory = UiMemory::new();
    memory.focus(focused);
    memory.set_text_input_owner(focused);
    let mut focused_state = TextEditState::new("focused");
    focused_state.set_caret(focused_state.text.len());
    let mut unfocused_state = TextEditState::new("unfocused");
    unfocused_state.set_caret(unfocused_state.text.len());

    let focused_output = text_field(
        focused,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut focused_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let unfocused_output = text_field(
        unfocused,
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(focused_output.changed);
    assert_eq!(focused_state.text, "focused typed");
    assert!(!unfocused_output.changed);
    assert_eq!(unfocused_state.text, "unfocused");
}

#[test]
fn path_field_preserves_text_input_and_emits_only_browse_or_open_intents() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("script-path");
    let text_id = id.child("text");
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let mut memory = UiMemory::new();
    memory.focus(text_id);
    memory.set_text_input_owner(text_id);
    let mut state = TextEditState::new("scripts/player.rs");
    state.set_caret(state.text.len());

    let typed = path_field(
        id,
        rect,
        "Script path",
        &mut state,
        PathFieldConfig::default().open(true),
        &UiInput {
            text_events: vec![TextInputEvent::Commit(".bak".to_owned())],
            ..UiInput::default()
        },
        &mut memory,
        &theme,
    );
    assert!(typed.changed);
    assert_eq!(state.text, "scripts/player.rs.bak");
    assert!(!typed.browse_requested);
    assert!(!typed.open_requested);
    assert!(
        typed
            .widget
            .semantics
            .iter()
            .any(|node| node.role == SemanticRole::TextField
                && node.label.as_deref() == Some("Script path")
                && node.state.value
                    == Some(SemanticValue::Text("scripts/player.rs.bak".to_owned())))
    );

    memory.begin_frame();
    let _ = path_field(
        id,
        rect,
        "Script path",
        &mut state,
        PathFieldConfig::default().open(true),
        &pressed_at(218.0, 8.0),
        &mut memory,
        &theme,
    );
    memory.begin_frame();
    let browse = path_field(
        id,
        rect,
        "Script path",
        &mut state,
        PathFieldConfig::default().open(true),
        &released_at(218.0, 8.0),
        &mut memory,
        &theme,
    );
    assert!(browse.browse_requested);
    assert!(!browse.open_requested);
    assert!(browse.browse_response.is_some());
    assert!(browse.widget.semantics.iter().any(|node| {
        node.role == SemanticRole::Button
            && node.label.as_deref() == Some("Browse Script path")
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Open)
    }));

    memory.begin_frame();
    let _ = path_field(
        id,
        rect,
        "Script path",
        &mut state,
        PathFieldConfig::default().open(true),
        &pressed_at(8.0, 8.0),
        &mut memory,
        &theme,
    );
    memory.begin_frame();
    let open = path_field(
        id,
        rect,
        "Script path",
        &mut state,
        PathFieldConfig::default().open(true),
        &double_released_at(8.0, 8.0),
        &mut memory,
        &theme,
    );
    assert!(open.open_requested);
    assert!(!open.browse_requested);
}

#[test]
fn path_field_text_editing_open_intent_uses_routed_text_response() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("captured-path");
    let text_id = id.child("text");
    let other = WidgetId::from_key("other-capture");
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let mut memory = UiMemory::new();
    memory.focus(text_id);
    memory.set_text_input_owner(text_id);
    memory.activate(other);
    memory.capture_pointer(other);
    let mut state = TextEditState::new("assets/texture.png");
    state.set_caret(state.text.len());
    let mut input = double_released_at(8.0, 8.0);
    input.text_events = vec![TextInputEvent::Commit(".bak".to_owned())];

    let output = path_field(
        id,
        rect,
        "Texture path",
        &mut state,
        PathFieldConfig::default().open(true),
        &input,
        &mut memory,
        &theme,
    );

    assert!(output.changed);
    assert_eq!(state.text, "assets/texture.png.bak");
    assert_eq!(
        output
            .field
            .widget
            .response
            .as_ref()
            .expect("text response")
            .id,
        text_id
    );
    assert!(!output.browse_requested);
    assert!(!output.open_requested);
}

#[test]
fn path_field_clipboard_routing_targets_composed_text_field_without_opening() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("clipboard-path");
    let text_id = id.child("text");
    let other = WidgetId::from_key("other-text");
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let mut memory = UiMemory::new();
    memory.focus(text_id);
    memory.set_text_input_owner(text_id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));

    let copy = path_field(
        id,
        rect,
        "Path",
        &mut state,
        PathFieldConfig::default().open(true),
        &UiInput {
            keyboard: KeyboardInput {
                modifiers: ctrl(),
                events: vec![shortcut_event("c")],
            },
            ..UiInput::default()
        },
        &mut memory,
        &theme,
    );
    assert!(!copy.changed);
    assert!(copy.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));
    assert!(!copy.browse_requested);
    assert!(!copy.open_requested);

    memory.begin_frame();
    let paste = path_field(
        id,
        rect,
        "Path",
        &mut state,
        PathFieldConfig::default().open(true),
        &UiInput {
            keyboard: KeyboardInput {
                modifiers: ctrl(),
                events: vec![shortcut_event("v")],
            },
            ..UiInput::default()
        },
        &mut memory,
        &theme,
    );
    assert!(paste.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::RequestClipboardText { target } if *target == text_id)
    }));
    assert!(!paste.browse_requested);
    assert!(!paste.open_requested);

    memory.begin_frame();
    let mut clipboard_input = double_released_at(8.0, 8.0);
    clipboard_input.clipboard_text = vec![
        kinetik_ui_core::ClipboardText::new(other, "wrong"),
        kinetik_ui_core::ClipboardText::new(text_id, "XY"),
    ];
    let applied = path_field(
        id,
        rect,
        "Path",
        &mut state,
        PathFieldConfig::default().open(true),
        &clipboard_input,
        &mut memory,
        &theme,
    );

    assert!(applied.changed);
    assert_eq!(state.text, "aXYd");
    assert!(!applied.browse_requested);
    assert!(!applied.open_requested);
}

#[test]
fn disabled_and_read_only_path_fields_do_not_edit_or_emit_browse_intents() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);

    for (key, config) in [
        ("disabled-path", PathFieldConfig::default().disabled(true)),
        ("read-only-path", PathFieldConfig::default().read_only(true)),
    ] {
        let id = WidgetId::from_key(key);
        let mut memory = UiMemory::new();
        memory.focus(id.child("text"));
        memory.set_text_input_owner(id.child("text"));
        let mut state = TextEditState::new("assets/file.png");
        state.set_caret(state.text.len());
        let output = path_field(
            id,
            rect,
            "Texture path",
            &mut state,
            config,
            &UiInput {
                text_events: vec![TextInputEvent::Commit("x".to_owned())],
                ..released_at(218.0, 8.0)
            },
            &mut memory,
            &theme,
        );

        assert_eq!(state.text, "assets/file.png");
        assert!(!output.changed);
        assert!(!output.browse_requested);
        assert!(!output.open_requested);
        assert!(
            output
                .field
                .widget
                .response
                .expect("text response")
                .state
                .disabled
        );
        assert!(
            output
                .browse_response
                .expect("browse response")
                .state
                .disabled
        );
    }
}
