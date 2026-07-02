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
fn single_line_drops_committed_newlines_while_multi_line_preserves_them() {
    let theme = default_dark_theme();
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit("a\r\nb\nc".to_owned())],
        ..UiInput::default()
    };

    let single = WidgetId::from_key("single");
    let mut single_memory = UiMemory::new();
    single_memory.focus(single);
    single_memory.set_text_input_owner(single);
    let mut single_state = TextEditState::new("");
    let single_output = text_field(
        single,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut single_state,
        &input,
        &mut single_memory,
        &theme,
        false,
    );

    let multi = WidgetId::from_key("multi");
    let mut multi_memory = UiMemory::new();
    multi_memory.focus(multi);
    multi_memory.set_text_input_owner(multi);
    let mut multi_state = TextEditState::new("");
    let multi_output = multi_line_text_field(
        multi,
        Rect::new(0.0, 0.0, 160.0, 80.0),
        &mut multi_state,
        &input,
        &mut multi_memory,
        &theme,
        false,
    );

    assert!(single_output.changed);
    assert_eq!(single_state.text, "abc");
    assert!(multi_output.changed);
    assert_eq!(multi_state.text, "a\r\nb\nc");
    assert_eq!(multi_output.visible_lines, 3);
}

#[test]
fn multi_line_enter_inserts_newline_but_single_line_enter_does_not() {
    let theme = default_dark_theme();
    let input = UiInput {
        keyboard: key_input(Key::Enter, Modifiers::default()),
        ..UiInput::default()
    };

    let single = WidgetId::from_key("single-enter");
    let mut single_memory = UiMemory::new();
    single_memory.focus(single);
    single_memory.set_text_input_owner(single);
    let mut single_state = TextEditState::new("one");
    let single_output = text_field(
        single,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut single_state,
        &input,
        &mut single_memory,
        &theme,
        false,
    );

    let multi = WidgetId::from_key("multi-enter");
    let mut multi_memory = UiMemory::new();
    multi_memory.focus(multi);
    multi_memory.set_text_input_owner(multi);
    let mut multi_state = TextEditState::new("one");
    multi_state.set_caret(multi_state.text.len());
    let multi_output = multi_line_text_field(
        multi,
        Rect::new(0.0, 0.0, 160.0, 80.0),
        &mut multi_state,
        &input,
        &mut multi_memory,
        &theme,
        false,
    );

    assert!(!single_output.changed);
    assert_eq!(single_state.text, "one");
    assert!(multi_output.changed);
    assert_eq!(multi_state.text, "one\n");
}

#[test]
fn multi_line_text_field_moves_vertically_between_explicit_lines() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-nav");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(6);

    let up_input = UiInput {
        keyboard: key_input(Key::ArrowUp, Modifiers::default()),
        ..UiInput::default()
    };
    let up = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &up_input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!up.changed);
    assert_eq!(state.text, "one\ntwo\nthree");
    assert_eq!(state.caret(), 2);

    let down_input = UiInput {
        keyboard: key_input(Key::ArrowDown, Modifiers::default()),
        ..UiInput::default()
    };
    let down = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &down_input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!down.changed);
    assert_eq!(state.caret(), 6);
}

#[test]
fn multi_line_text_field_extends_selection_with_shift_vertical_navigation() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-shift-nav");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(6);
    let input = UiInput {
        keyboard: key_input(Key::ArrowDown, shift()),
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
    assert_eq!(state.text, "one\ntwo\nthree");
    assert_eq!(state.selection, TextSelection::new(6, 10));
}

#[test]
fn multi_line_text_field_home_end_are_line_local() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-home-end");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(5);

    let home_input = UiInput {
        keyboard: key_input(Key::Home, Modifiers::default()),
        ..UiInput::default()
    };
    let home = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &home_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!home.changed);
    assert_eq!(state.caret(), 4);

    state.set_caret(5);
    let end_input = UiInput {
        keyboard: key_input(Key::End, Modifiers::default()),
        ..UiInput::default()
    };
    let end = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &end_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!end.changed);
    assert_eq!(state.caret(), 7);
}

#[test]
fn multi_line_text_field_shift_home_end_extend_to_current_line_edges() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-shift-home-end");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(5);

    let shift_home_input = UiInput {
        keyboard: key_input(Key::Home, shift()),
        ..UiInput::default()
    };
    let shift_home = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &shift_home_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!shift_home.changed);
    assert_eq!(state.selection, TextSelection::new(5, 4));

    state.set_caret(5);
    let shift_end_input = UiInput {
        keyboard: key_input(Key::End, shift()),
        ..UiInput::default()
    };
    let shift_end = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &shift_end_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!shift_end.changed);
    assert_eq!(state.selection, TextSelection::new(5, 7));
}

#[test]
fn unfocused_and_disabled_multi_line_text_fields_ignore_navigation() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-disabled-nav");
    let input = UiInput {
        keyboard: key_input(Key::ArrowDown, shift()),
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("one\ntwo");
    unfocused_state.set_caret(1);
    let unfocused = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.selection, TextSelection::new(1, 1));

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("one\ntwo");
    disabled_state.set_caret(1);
    let disabled = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.selection, TextSelection::new(1, 1));
}
