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
fn search_field_exposes_search_semantics_and_current_query() {
    let theme = default_dark_theme();
    let id = root_child("search");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("media");

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let search = ui.search_field(
        "search",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut state,
        false,
    );
    let output = ui.finish_output();

    assert_eq!(search.query, "media");
    assert!(!search.empty);
    assert_eq!(
        text_value(&output, &SemanticRole::SearchField).as_deref(),
        Some("media")
    );
    assert!(output.semantics.nodes().iter().any(|node| {
        node.id == id && node.role == SemanticRole::SearchField && node.state.focused
    }));
}

#[test]
fn ui_text_field_uses_layout_store_for_text_and_caret_blink_repaint() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abcdef");
    let mut text_layouts = TextLayoutStore::new();

    let mut ui = Ui::begin_frame_with_text_layouts(
        frame_context_at(Duration::from_millis(0), UiInput::default()),
        &mut memory,
        &theme,
        &mut text_layouts,
    );
    ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 28.0), &mut state, false);
    let output = ui.finish_output();

    assert!(!text_layouts.is_empty());
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "abcdef" && text.layout.is_some())
    }));
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect.width <= 1.0 && rect.rect.height > 1.0)
    }));
    assert_eq!(
        output.repaint,
        RepaintRequest::After(Duration::from_millis(500))
    );
}

#[test]
fn focused_text_field_extends_selection_with_shift_movement_without_changing_text() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("abcd");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
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

    assert!(!output.changed);
    assert_eq!(state.text, "abcd");
    assert_eq!(state.selection, TextSelection::new(4, 3));
}

#[test]
fn unfocused_and_disabled_text_fields_ignore_shift_movement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("abcd");
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
    assert_eq!(unfocused_state.selection, TextSelection::new(4, 4));

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("abcd");
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
    assert_eq!(disabled_state.selection, TextSelection::new(4, 4));
}

#[test]
fn ui_text_field_selection_only_movement_requests_repaint_and_highlight() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abcdef");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 24.0), &mut state, false);
    let frame = ui.finish_output();

    assert!(!output.changed);
    assert_eq!(state.text, "abcdef");
    assert_eq!(state.selection, TextSelection::new(6, 5));
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);
    assert!(has_selection_highlight(&frame, &theme));
}

#[test]
fn search_and_numeric_fields_extend_selection_through_widget_flow() {
    let theme = default_dark_theme();
    let search = root_child("search");
    let number = root_child("number");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let mut search_memory = UiMemory::new();
    search_memory.focus(search);
    search_memory.set_text_input_owner(search);
    let mut search_state = TextEditState::new("media");
    let mut ui = Ui::new(&input, &mut search_memory, &theme);
    let search_output = ui.search_field(
        "search",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut search_state,
        false,
    );
    let _ = ui.finish_output();
    assert!(!search_output.field.changed);
    assert_eq!(search_state.selection, TextSelection::new(5, 4));

    let mut numeric_memory = UiMemory::new();
    numeric_memory.focus(number);
    numeric_memory.set_text_input_owner(number);
    let mut numeric_state = TextEditState::new("42.5");
    let mut ui = Ui::new(&input, &mut numeric_memory, &theme);
    let numeric_output = ui.numeric_input(
        "number",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut numeric_state,
        false,
    );
    let _ = ui.finish_output();
    assert!(!numeric_output.field.changed);
    assert_eq!(numeric_state.selection, TextSelection::new(4, 3));
}

#[test]
fn single_line_search_and_numeric_navigation_remains_buffer_local() {
    let theme = default_dark_theme();
    let field = WidgetId::from_key("single-nav");
    let mut field_memory = UiMemory::new();
    field_memory.focus(field);
    field_memory.set_text_input_owner(field);
    let mut field_state = TextEditState::new("one\ntwo");
    field_state.set_caret(5);

    let home = text_field(
        field,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut field_state,
        &UiInput {
            keyboard: key_input(Key::Home, Modifiers::default()),
            ..UiInput::default()
        },
        &mut field_memory,
        &theme,
        false,
    );
    assert!(!home.changed);
    assert_eq!(field_state.caret(), 0);

    field_state.set_caret(5);
    let arrow_down = text_field(
        field,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut field_state,
        &UiInput {
            keyboard: key_input(Key::ArrowDown, Modifiers::default()),
            ..UiInput::default()
        },
        &mut field_memory,
        &theme,
        false,
    );
    assert!(!arrow_down.changed);
    assert_eq!(field_state.caret(), 5);

    let search = root_child("search-nav");
    let mut search_memory = UiMemory::new();
    search_memory.focus(search);
    search_memory.set_text_input_owner(search);
    let mut search_state = TextEditState::new("media");
    search_state.set_caret(2);
    let search_input = UiInput {
        keyboard: key_input(Key::End, Modifiers::default()),
        ..UiInput::default()
    };
    let mut ui = Ui::new(&search_input, &mut search_memory, &theme);
    let search_output = ui.search_field(
        "search-nav",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut search_state,
        false,
    );
    let _ = ui.finish_output();
    assert!(!search_output.field.changed);
    assert_eq!(search_state.caret(), 5);

    let number = WidgetId::from_key("number-nav");
    let mut number_memory = UiMemory::new();
    number_memory.focus(number);
    number_memory.set_text_input_owner(number);
    let mut number_state = TextEditState::new("42.5");
    number_state.set_caret(2);
    let number_output = numeric_input(
        number,
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut number_state,
        &UiInput {
            keyboard: key_input(Key::ArrowUp, Modifiers::default()),
            ..UiInput::default()
        },
        &mut number_memory,
        &theme,
        false,
    );
    assert!(!number_output.field.changed);
    assert_eq!(number_state.caret(), 2);
}

#[test]
fn multi_line_text_field_shift_end_extends_to_current_line_end() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo");
    state.set_caret(4);
    let input = UiInput {
        keyboard: key_input(Key::End, shift()),
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

    assert!(!output.changed);
    assert_eq!(state.text, "one\ntwo");
    assert_eq!(state.selection, TextSelection::new(4, 7));
}

#[test]
fn focused_text_field_reserves_typing_and_editing_shortcuts_from_global_actions() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit("x".to_owned())],
        ..UiInput::default()
    };

    let mut ui = Ui::new(&input, &mut memory, &theme);
    let text = ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 24.0), &mut state, false);
    let _ = ui.finish_output();
    assert!(text.changed);
    assert_eq!(memory.text_input_owner(), Some(field));

    let mut router = ActionRouter::new();
    bind_global(
        &mut router,
        "global.type.x",
        Shortcut::new(Modifiers::default(), Key::Character("x".to_owned())),
    );
    bind_global(&mut router, "global.select.all", shortcut("a"));
    bind_global(&mut router, "global.cut", shortcut("x"));
    bind_global(&mut router, "global.copy", shortcut("c"));
    bind_global(&mut router, "global.paste", shortcut("v"));

    let routing = ActionRoutingContext::new().with_text_input(field);
    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_input(Key::Character("x".to_owned()), Modifiers::default()),
            routing,
        ),
        None
    );
    for character in ["a", "c", "v", "x"] {
        assert_eq!(
            router.resolve_shortcut_in_context(
                &key_input(Key::Character(character.to_owned()), ctrl()),
                routing,
            ),
            None
        );
    }
}

#[test]
fn pointer_press_uses_shaped_layout_store_for_caret_hit_placement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("hit-field");
    let rect = Rect::new(0.0, 0.0, 220.0, 28.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abcdef");
    let mut text_layouts = TextLayoutStore::new();
    let input = pressed_at(rect.max_x() - 2.0, 14.0);

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
    assert_eq!(memory.focused(), Some(id));
    assert!(!text_layouts.is_empty());
    assert!(
        output
            .widget
            .response
            .as_ref()
            .is_some_and(|response| response.state.focused)
    );
}
