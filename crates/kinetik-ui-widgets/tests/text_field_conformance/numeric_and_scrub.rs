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
use kinetik_ui_core::UiInputEvent;

#[test]
fn numeric_input_distinguishes_valid_invalid_and_empty_states() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();

    assert_eq!(
        classify_numeric_input_draft("42.5"),
        NumericInputDraft::Valid(42.5)
    );
    assert_eq!(
        classify_numeric_input_draft("42 px"),
        NumericInputDraft::Invalid
    );
    assert_eq!(classify_numeric_input_draft("  "), NumericInputDraft::Empty);

    let mut valid_state = TextEditState::new("42.5");
    let valid = numeric_input(
        WidgetId::from_key("valid"),
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut valid_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!(valid.valid);
    assert_eq!(valid.value, Some(42.5));
    assert_eq!(valid.policy.draft, NumericInputDraft::Valid(42.5));

    let mut invalid_state = TextEditState::new("42 px");
    let invalid = numeric_input(
        WidgetId::from_key("invalid"),
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut invalid_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!(!invalid.valid);
    assert_eq!(invalid.value, None);
    assert_eq!(invalid.policy.draft, NumericInputDraft::Invalid);

    let mut empty_state = TextEditState::new("  ");
    let empty = numeric_input(
        WidgetId::from_key("empty"),
        Rect::new(0.0, 64.0, 160.0, 24.0),
        &mut empty_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!(empty.valid);
    assert_eq!(empty.value, None);
    assert_eq!(empty.policy.draft, NumericInputDraft::Empty);
}

#[test]
fn focused_numeric_input_enter_requests_commit_for_valid_non_empty_draft() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("number");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("42.5");
    let input = UiInput {
        keyboard: key_input(Key::Enter, Modifiers::default()),
        ..UiInput::default()
    };

    let output = numeric_input(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert_eq!(output.policy.draft, NumericInputDraft::Valid(42.5));
    assert!(output.policy.commit_requested);
    assert!(!output.policy.revert_requested);
    assert_eq!(state.text, "42.5");
}

#[test]
fn focused_numeric_input_enter_ignores_invalid_and_empty_drafts() {
    let theme = default_dark_theme();
    let input = UiInput {
        keyboard: key_input(Key::Enter, Modifiers::default()),
        ..UiInput::default()
    };

    let invalid_id = WidgetId::from_key("invalid-number");
    let mut invalid_memory = UiMemory::new();
    invalid_memory.focus(invalid_id);
    invalid_memory.set_text_input_owner(invalid_id);
    let mut invalid_state = TextEditState::new("42 px");
    let invalid = numeric_input(
        invalid_id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut invalid_state,
        &input,
        &mut invalid_memory,
        &theme,
        false,
    );
    assert_eq!(invalid.policy.draft, NumericInputDraft::Invalid);
    assert!(!invalid.policy.commit_requested);
    assert!(!invalid.policy.revert_requested);

    let empty_id = WidgetId::from_key("empty-number");
    let mut empty_memory = UiMemory::new();
    empty_memory.focus(empty_id);
    empty_memory.set_text_input_owner(empty_id);
    let mut empty_state = TextEditState::new("  ");
    let empty = numeric_input(
        empty_id,
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut empty_state,
        &input,
        &mut empty_memory,
        &theme,
        false,
    );
    assert_eq!(empty.policy.draft, NumericInputDraft::Empty);
    assert!(!empty.policy.commit_requested);
    assert!(!empty.policy.revert_requested);
}

#[test]
fn focused_numeric_input_escape_requests_revert_and_helper_restores_baseline() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("number-revert");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("invalid draft");
    state.set_selection(TextSelection::new(0, state.text.len()));
    let input = UiInput {
        keyboard: key_input(Key::Escape, Modifiers::default()),
        ..UiInput::default()
    };

    let output = numeric_input(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert_eq!(output.policy.draft, NumericInputDraft::Invalid);
    assert!(!output.policy.commit_requested);
    assert!(output.policy.revert_requested);
    assert_eq!(state.text, "invalid draft");

    assert!(restore_text_draft(&mut state, "12.5"));
    assert_eq!(state.text, "12.5");
    assert_eq!(state.selection, TextSelection::new(4, 4));
    assert_eq!(
        classify_numeric_input_draft(&state.text),
        NumericInputDraft::Valid(12.5)
    );
}

#[test]
fn numeric_ordered_intent_is_emitted_only_by_the_single_claimed_pass() {
    let theme = default_dark_theme();
    let first = WidgetId::from_key("first-number");
    let second = WidgetId::from_key("second-number");
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Enter,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    let mut memory = UiMemory::new();
    memory.focus(first);
    memory.set_text_input_owner(first);
    let mut first_state = TextEditState::new("42");
    let mut second_state = TextEditState::new("7");

    let claimed = numeric_input(
        first,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut first_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let repeated = numeric_input(
        first,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut first_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    memory.focus(second);
    memory.set_text_input_owner(second);
    let handed_off = numeric_input(
        second,
        Rect::new(0.0, 30.0, 160.0, 24.0),
        &mut second_state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(claimed.policy.commit_requested);
    assert!(!repeated.policy.commit_requested);
    assert!(!handed_off.policy.commit_requested);

    let revert_id = WidgetId::from_key("revert-number");
    let mut revert_input = UiInput::default();
    revert_input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    let mut revert_memory = UiMemory::new();
    revert_memory.focus(revert_id);
    revert_memory.set_text_input_owner(revert_id);
    let mut revert_state = TextEditState::new("invalid draft");
    let reverted = numeric_input(
        revert_id,
        Rect::new(0.0, 60.0, 160.0, 24.0),
        &mut revert_state,
        &revert_input,
        &mut revert_memory,
        &theme,
        false,
    );
    assert!(reverted.policy.revert_requested);
}

#[test]
fn numeric_ordered_intent_rejects_focus_loss_repeat_modifiers_and_conflict() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("number-intent-guards");

    let mut after_focus_loss = UiInput::default();
    after_focus_loss.push_event(UiInputEvent::WindowFocusChanged(false));
    after_focus_loss.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Enter,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    after_focus_loss.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    let mut focus_memory = UiMemory::new();
    focus_memory.focus(id);
    focus_memory.set_text_input_owner(id);
    let mut focus_state = TextEditState::new("42");
    let focus_output = numeric_input(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut focus_state,
        &after_focus_loss,
        &mut focus_memory,
        &theme,
        false,
    );
    assert!(!focus_output.policy.commit_requested);
    assert!(!focus_output.policy.revert_requested);

    let mut filtered = UiInput::default();
    filtered.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Enter,
        KeyState::Pressed,
        Modifiers::default(),
        true,
    )));
    filtered.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::new(true, false, false, false),
        false,
    )));
    filtered.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Enter,
        KeyState::Released,
        Modifiers::default(),
        false,
    )));
    let mut filtered_memory = UiMemory::new();
    filtered_memory.focus(id);
    filtered_memory.set_text_input_owner(id);
    let mut filtered_state = TextEditState::new("42");
    let filtered_output = numeric_input(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut filtered_state,
        &filtered,
        &mut filtered_memory,
        &theme,
        false,
    );
    assert!(!filtered_output.policy.commit_requested);
    assert!(!filtered_output.policy.revert_requested);

    let mut conflicted = UiInput::default();
    conflicted.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Enter,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    conflicted.keyboard.events.push(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    ));
    let mut conflict_memory = UiMemory::new();
    conflict_memory.focus(id);
    conflict_memory.set_text_input_owner(id);
    let mut conflict_state = TextEditState::new("42");
    let conflict_output = numeric_input(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut conflict_state,
        &conflicted,
        &mut conflict_memory,
        &theme,
        false,
    );
    assert!(!conflict_output.policy.commit_requested);
    assert!(!conflict_output.policy.revert_requested);
}

#[test]
fn unfocused_and_disabled_numeric_inputs_ignore_commit_revert_keys() {
    let theme = default_dark_theme();
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![
                KeyEvent::new(Key::Enter, KeyState::Pressed, Modifiers::default(), false),
                KeyEvent::new(Key::Escape, KeyState::Pressed, Modifiers::default(), false),
            ],
        },
        ..UiInput::default()
    };

    let unfocused_id = WidgetId::from_key("unfocused-number");
    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("42");
    let unfocused = numeric_input(
        unfocused_id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert_eq!(unfocused.policy.draft, NumericInputDraft::Valid(42.0));
    assert!(!unfocused.policy.commit_requested);
    assert!(!unfocused.policy.revert_requested);

    let disabled_id = WidgetId::from_key("disabled-number");
    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(disabled_id);
    disabled_memory.set_text_input_owner(disabled_id);
    let mut disabled_state = TextEditState::new("42");
    let disabled = numeric_input(
        disabled_id,
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert_eq!(disabled.policy.draft, NumericInputDraft::Valid(42.0));
    assert!(!disabled.policy.commit_requested);
    assert!(!disabled.policy.revert_requested);
}

#[test]
fn numeric_scrub_input_maps_horizontal_drag_delta_to_value() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("scrub-number");
    let rect = Rect::new(0.0, 0.0, 160.0, 24.0);
    let config = NumericScrubInputConfig::new(0.5).with_range(0.0, 10.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("2");
    let mut value = 2.0;

    let _ = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &pressed_at(4.0, 4.0),
        &mut memory,
        &theme,
    );
    let output = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &scrub_drag_at(8.0, 4.0, 4.0, Modifiers::default()),
        &mut memory,
        &theme,
    );

    assert!(output.scrub_response.dragged);
    assert!(output.scrubbed);
    assert!(output.value_changed);
    assert!((value - 4.0).abs() < f32::EPSILON);
    assert_eq!(state.text, "4");
    assert_eq!(output.input.policy.draft, NumericInputDraft::Valid(4.0));

    let node = output
        .input
        .field
        .widget
        .semantics
        .iter()
        .find(|node| node.role == SemanticRole::TextField)
        .expect("numeric scrub text semantics");
    assert!(
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::SetText)
    );
    assert!(
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::SetValue)
    );
    assert_eq!(
        node.state.value,
        Some(SemanticValue::Number {
            current: 4.0,
            min: 0.0,
            max: 10.0,
        })
    );
}

#[test]
fn numeric_scrub_input_uses_fine_and_coarse_modifier_steps() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 160.0, 24.0);
    let config = NumericScrubInputConfig::new(1.0)
        .with_fine_step(0.25)
        .with_coarse_step(5.0);

    let fine_id = WidgetId::from_key("fine-scrub");
    let mut fine_memory = UiMemory::new();
    let mut fine_state = TextEditState::new("10");
    let mut fine_value = 10.0;
    let _ = numeric_scrub_input(
        fine_id,
        rect,
        &mut fine_value,
        &mut fine_state,
        config,
        &pressed_at(4.0, 4.0),
        &mut fine_memory,
        &theme,
    );
    let fine = numeric_scrub_input(
        fine_id,
        rect,
        &mut fine_value,
        &mut fine_state,
        config,
        &scrub_drag_at(8.0, 4.0, 8.0, shift()),
        &mut fine_memory,
        &theme,
    );
    assert!((fine.step - 0.25).abs() < f32::EPSILON);
    assert!((fine_value - 12.0).abs() < f32::EPSILON);

    let coarse_id = WidgetId::from_key("coarse-scrub");
    let mut coarse_memory = UiMemory::new();
    let mut coarse_state = TextEditState::new("10");
    let mut coarse_value = 10.0;
    let _ = numeric_scrub_input(
        coarse_id,
        rect,
        &mut coarse_value,
        &mut coarse_state,
        config,
        &pressed_at(4.0, 4.0),
        &mut coarse_memory,
        &theme,
    );
    let coarse = numeric_scrub_input(
        coarse_id,
        rect,
        &mut coarse_value,
        &mut coarse_state,
        config,
        &scrub_drag_at(8.0, 4.0, 3.0, ctrl()),
        &mut coarse_memory,
        &theme,
    );
    assert!((coarse.step - 5.0).abs() < f32::EPSILON);
    assert!((coarse_value - 25.0).abs() < f32::EPSILON);
}

#[test]
fn numeric_scrub_input_sanitizes_steps_and_clamps_to_finite_bounds() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("clamped-scrub");
    let rect = Rect::new(0.0, 0.0, 160.0, 24.0);
    let config = NumericScrubInputConfig::new(-2.0)
        .with_fine_step(0.0)
        .with_coarse_step(f32::NAN)
        .with_range(10.0, 0.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("2");
    let mut value = 2.0;

    let _ = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &pressed_at(4.0, 4.0),
        &mut memory,
        &theme,
    );
    let output = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &scrub_drag_at(8.0, 4.0, 20.0, Modifiers::default()),
        &mut memory,
        &theme,
    );

    assert!((output.step - 1.0).abs() < f32::EPSILON);
    assert_eq!(output.min, Some(0.0));
    assert_eq!(output.max, Some(10.0));
    assert!((value - 10.0).abs() < f32::EPSILON);
    assert_eq!(state.text, "10");
}

#[test]
fn numeric_scrub_input_invalid_draft_does_not_silently_commit() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("invalid-scrub");
    let rect = Rect::new(0.0, 0.0, 160.0, 24.0);
    let config = NumericScrubInputConfig::new(1.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("12 px");
    let mut value = 12.0;

    let _ = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &pressed_at(4.0, 4.0),
        &mut memory,
        &theme,
    );
    let output = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &scrub_drag_at(8.0, 4.0, 8.0, Modifiers::default()),
        &mut memory,
        &theme,
    );

    assert!(output.scrub_response.dragged);
    assert!(!output.scrubbed);
    assert!(!output.value_changed);
    assert!((value - 12.0).abs() < f32::EPSILON);
    assert_eq!(state.text, "12 px");
    assert_eq!(output.input.policy.draft, NumericInputDraft::Invalid);
    assert!(!output.input.valid);
}

#[test]
fn disabled_and_read_only_numeric_scrub_inputs_do_not_mutate_or_take_ownership() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 160.0, 24.0);

    let disabled_id = WidgetId::from_key("disabled-scrub");
    let mut disabled_memory = UiMemory::new();
    let mut disabled_state = TextEditState::new("1");
    let mut disabled_value = 1.0;
    let disabled = numeric_scrub_input(
        disabled_id,
        rect,
        &mut disabled_value,
        &mut disabled_state,
        NumericScrubInputConfig::new(1.0).disabled(true),
        &UiInput {
            text_events: vec![TextInputEvent::Commit("9".to_owned())],
            ..scrub_drag_at(8.0, 4.0, 8.0, Modifiers::default())
        },
        &mut disabled_memory,
        &theme,
    );
    assert!(!disabled.scrub_response.dragged);
    assert!(!disabled.scrubbed);
    assert!((disabled_value - 1.0).abs() < f32::EPSILON);
    assert_eq!(disabled_state.text, "1");
    assert_eq!(disabled_memory.focused(), None);
    assert_eq!(disabled_memory.active(), None);
    assert_eq!(disabled_memory.text_input_owner(), None);
    assert!(disabled.input.field.widget.semantics[0].state.disabled);

    let read_only_id = WidgetId::from_key("read-only-scrub");
    let mut read_only_memory = UiMemory::new();
    let mut read_only_state = TextEditState::new("3");
    let mut read_only_value = 3.0;
    let read_only = numeric_scrub_input(
        read_only_id,
        rect,
        &mut read_only_value,
        &mut read_only_state,
        NumericScrubInputConfig::new(1.0).read_only(true),
        &pressed_at(4.0, 4.0),
        &mut read_only_memory,
        &theme,
    );
    assert!(!read_only.scrub_response.state.active);
    assert!(!read_only.scrubbed);
    assert!((read_only_value - 3.0).abs() < f32::EPSILON);
    assert_eq!(read_only_state.text, "3");
    assert_eq!(read_only_memory.focused(), None);
    assert_eq!(read_only_memory.text_input_owner(), None);
    assert!(read_only.input.field.widget.semantics[0].state.disabled);
    assert!(!read_only.input.field.widget.semantics[0].focusable);
}

#[test]
fn vector3_scrub_input_updates_components_independently() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("position");
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let component_rects = vector3_component_rects(rect, VectorComponentLayout::default());
    let y_center = component_rects[1].value_rect.center();
    let config =
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(0.5).with_range(-10.0, 10.0));
    let mut values = [1.0, 2.0, 3.0];
    let mut states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let mut memory = UiMemory::new();

    let _ = vector3_scrub_input(
        id,
        rect,
        "Position",
        &mut values,
        &mut states,
        config,
        &pressed_at(y_center.x, y_center.y),
        &mut memory,
        &theme,
    );
    let output = vector3_scrub_input(
        id,
        rect,
        "Position",
        &mut values,
        &mut states,
        config,
        &scrub_drag_at(y_center.x + 8.0, y_center.y, 4.0, Modifiers::default()),
        &mut memory,
        &theme,
    );

    assert!(output.scrubbed);
    assert!(output.value_changed);
    assert_eq!(output.components.len(), 3);
    assert_f32_slice_eq(&values, &[1.0, 4.0, 3.0]);
    assert_eq!(states[0].text, "1");
    assert_eq!(states[1].text, "4");
    assert_eq!(states[2].text, "3");
    assert_eq!(
        output
            .widget
            .semantics
            .iter()
            .filter_map(|node| node.label.as_deref())
            .collect::<Vec<_>>(),
        vec!["Position X", "Position Y", "Position Z"]
    );
    assert_eq!(
        output.widget.semantics[1].state.value,
        Some(SemanticValue::Number {
            current: 4.0,
            min: -10.0,
            max: 10.0,
        })
    );
}

#[test]
fn disabled_and_read_only_vector_scrub_inputs_propagate_to_all_components() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);

    let mut disabled_values = [1.0, 2.0];
    let mut disabled_states = [TextEditState::new("1"), TextEditState::new("2")];
    let disabled = vector2_scrub_input(
        WidgetId::from_key("disabled-vector"),
        rect,
        "Offset",
        &mut disabled_values,
        &mut disabled_states,
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(1.0)).disabled(true),
        &scrub_drag_at(8.0, 4.0, 8.0, Modifiers::default()),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(!disabled.scrubbed);
    assert_f32_slice_eq(&disabled_values, &[1.0, 2.0]);
    assert!(
        disabled.widget.semantics.iter().all(|node| {
            node.state.disabled && !node.focusable && node.label.as_deref().is_some()
        })
    );

    let mut read_only_values = [1.0, 2.0, 3.0, 4.0];
    let mut read_only_states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
        TextEditState::new("4"),
    ];
    let read_only = vector4_scrub_input(
        WidgetId::from_key("read-only-vector"),
        rect,
        "Color",
        &mut read_only_values,
        &mut read_only_states,
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(1.0)).read_only(true),
        &pressed_at(8.0, 4.0),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(!read_only.scrubbed);
    assert_f32_slice_eq(&read_only_values, &[1.0, 2.0, 3.0, 4.0]);
    assert!(
        read_only.widget.semantics.iter().all(|node| {
            node.state.disabled && !node.focusable && node.label.as_deref().is_some()
        })
    );
}
