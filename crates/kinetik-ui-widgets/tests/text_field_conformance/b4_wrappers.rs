#![allow(clippy::float_cmp)]

use kinetik_ui_core::{
    ClipboardText, FrameOutput, InputWheelDelta, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    MouseButton, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect,
    SemanticActionKind, SemanticValue, TextInputEvent, UiInput, UiInputEvent, UiMemory, Vec2,
};
use kinetik_ui_text::{TextEditState, TextSelection};
use kinetik_ui_widgets::{
    NumericScrubInputConfig, PathFieldConfig, Ui, VectorComponentLayout, VectorScrubInputConfig,
    vector2_component_rects, vector3_component_rects,
};

use super::{default_dark_theme, root_child};

const FIELD_RECT: Rect = Rect::new(0.0, 0.0, 160.0, 24.0);

fn press(x: f32, y: f32, click_count: u8) -> UiInputEvent {
    UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count,
        position: Some(Point::new(x, y)),
    }
}

fn release(x: f32, y: f32, click_count: u8) -> UiInputEvent {
    UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count,
        position: Some(Point::new(x, y)),
    }
}

fn moved(x: f32, y: f32, delta_x: f32) -> UiInputEvent {
    UiInputEvent::PointerMoved {
        position: Point::new(x, y),
        delta: Vec2::new(delta_x, 0.0),
    }
}

fn canonical(events: impl IntoIterator<Item = UiInputEvent>) -> UiInput {
    let mut input = UiInput::default();
    for event in events {
        input.push_event(event);
    }
    input
}

fn ctrl() -> Modifiers {
    Modifiers::new(false, true, false, false)
}

fn shift() -> Modifiers {
    Modifiers::new(true, false, false, false)
}

fn copy_input() -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![KeyEvent::new(
                Key::Character("c".to_owned()),
                KeyState::Pressed,
                ctrl(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn has_action(node: &kinetik_ui_core::SemanticNode, action: &SemanticActionKind) -> bool {
    node.actions
        .iter()
        .any(|candidate| candidate.kind == *action)
}

fn assert_caret_start(frame: &FrameOutput, field_rect: Rect) {
    let caret = frame
        .platform_requests
        .iter()
        .find_map(|request| match request {
            PlatformRequest::StartTextInput { rect: Some(rect) } => Some(*rect),
            _ => None,
        })
        .expect("wrapper starts text input with caret geometry");
    assert_eq!(caret.width, 1.0);
    assert!(caret.height > 0.0);
    assert_ne!(caret, field_rect);
    assert!(field_rect.intersection(caret).is_some());
}

#[test]
fn clicked_scrub_places_caret_at_release_and_only_replays_later_text() {
    let theme = default_dark_theme();
    let input = canonical([
        press(8.0, 8.0, 1),
        UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        release(9.0, 8.0, 1),
        UiInputEvent::Text(TextInputEvent::Commit("Y".to_owned())),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("12");
    let mut value = 12.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let frame = ui.finish_output();

    assert!(!output.scrubbed);
    assert_eq!(value, 12.0);
    assert!(!state.text.contains('X'));
    assert!(state.text.contains('Y'));
    assert_eq!(memory.focused(), Some(root_child("number")));
    assert_eq!(memory.text_input_owner(), Some(root_child("number")));
    assert_caret_start(&frame, FIELD_RECT);
}

#[test]
fn clicked_scrub_rejects_preplacement_text_and_supports_multiframe_snapshots() {
    let theme = default_dark_theme();
    for events in [
        vec![
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            press(8.0, 8.0, 1),
            release(9.0, 8.0, 1),
        ],
        vec![
            press(8.0, 8.0, 1),
            release(9.0, 8.0, 1),
            UiInputEvent::Text(TextInputEvent::Commit("Y".to_owned())),
        ],
    ] {
        let input = canonical(events);
        let has_post_text = input.events.iter().any(
            |event| matches!(event, UiInputEvent::Text(TextInputEvent::Commit(text)) if text == "Y"),
        );
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("12");
        let mut value = 12.0;
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.numeric_scrub_input(
            "number",
            FIELD_RECT,
            &mut value,
            &mut state,
            NumericScrubInputConfig::new(1.0),
        );
        let _ = ui.finish_output();
        assert!(!state.text.contains('X'));
        assert_eq!(state.text.contains('Y'), has_post_text);
    }

    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("12");
    let mut value = 12.0;
    let pressed = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(8.0, 8.0)),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut ui = Ui::new(&pressed, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    assert_eq!(memory.focused(), None);

    let released = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(9.0, 8.0)),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut ui = Ui::new(&released, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let frame = ui.finish_output();
    assert_eq!(memory.focused(), Some(root_child("number")));
    assert_caret_start(&frame, FIELD_RECT);
}

#[test]
fn real_scrub_uses_causal_move_and_never_activates_unfocused_text() {
    let theme = default_dark_theme();
    let input = canonical([
        press(8.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(shift()),
        moved(16.0, 8.0, 8.0),
        UiInputEvent::ModifiersChanged(ctrl()),
        UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        release(16.0, 8.0, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("2");
    let mut value = 2.0;
    let config = NumericScrubInputConfig::new(1.0)
        .with_fine_step(0.25)
        .with_coarse_step(5.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let frame = ui.finish_output();

    assert!(output.scrub_response.dragged);
    assert!(output.scrubbed);
    assert_eq!(output.step, 0.25);
    assert_eq!(value, 4.0);
    assert_eq!(state.text, "4");
    assert_eq!(memory.focused(), None);
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );
}

#[test]
fn multiple_moves_use_only_the_last_causal_move_modifiers() {
    let theme = default_dark_theme();
    let input = canonical([
        press(8.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(shift()),
        moved(14.0, 8.0, 6.0),
        UiInputEvent::ModifiersChanged(ctrl()),
        moved(16.0, 8.0, 2.0),
        UiInputEvent::ModifiersChanged(shift()),
        release(16.0, 8.0, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("2");
    let mut value = 2.0;
    let config = NumericScrubInputConfig::new(1.0)
        .with_fine_step(0.25)
        .with_coarse_step(5.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let _ = ui.finish_output();

    assert!(output.scrubbed);
    assert_eq!(output.step, 5.0);
    assert_eq!(value, 42.0);
    assert_eq!(state.text, "42");
    assert_eq!(memory.focused(), None);
}

#[test]
fn release_only_threshold_crossing_uses_release_modifiers_not_final_snapshot() {
    let theme = default_dark_theme();
    let input = canonical([
        press(8.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(shift()),
        release(13.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(ctrl()),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("10");
    let mut value = 10.0;
    let config = NumericScrubInputConfig::new(1.0)
        .with_fine_step(0.25)
        .with_coarse_step(5.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let _ = ui.finish_output();

    assert!(output.scrubbed);
    assert_eq!(output.step, 0.25);
    assert_eq!(value, 11.25);
    assert_eq!(state.text, "11.25");
    assert_eq!(memory.focused(), None);
}

#[test]
fn multiple_domain_transactions_never_borrow_aggregate_drag_evidence() {
    let theme = default_dark_theme();
    let config = NumericScrubInputConfig::new(1.0)
        .with_fine_step(0.25)
        .with_coarse_step(5.0);

    let crossed_then_clicked = canonical([
        press(8.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(shift()),
        moved(14.0, 8.0, 6.0),
        release(14.0, 8.0, 1),
        press(8.0, 8.0, 1),
        release(9.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(ctrl()),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("3");
    let mut value = 3.0;
    let mut ui = Ui::new(&crossed_then_clicked, &mut memory, &theme);
    let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let _ = ui.finish_output();
    assert!(output.scrub_response.dragged);
    assert!(!output.scrubbed);
    assert_eq!(output.step, 5.0);
    assert_eq!(value, 3.0);
    assert_eq!(state.text, "3");
    assert_eq!(memory.focused(), Some(root_child("number")));

    let clicked_then_crossed = canonical([
        press(8.0, 8.0, 1),
        moved(9.0, 8.0, 1.0),
        release(9.0, 8.0, 1),
        press(8.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(ctrl()),
        moved(14.0, 8.0, 6.0),
        release(14.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(shift()),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("3");
    let mut value = 3.0;
    let mut ui = Ui::new(&clicked_then_crossed, &mut memory, &theme);
    let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let _ = ui.finish_output();
    assert!(output.scrub_response.dragged);
    assert!(output.scrubbed);
    assert_eq!(output.step, 5.0);
    assert_eq!(value, 33.0);
    assert_eq!(state.text, "33");
    assert_eq!(memory.focused(), None);
}

#[test]
fn legacy_crossed_drag_snapshot_uses_its_retained_event_time_modifiers() {
    let theme = default_dark_theme();
    let config = NumericScrubInputConfig::new(1.0)
        .with_fine_step(0.25)
        .with_coarse_step(5.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("1");
    let mut value = 1.0;

    let pressed = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(8.0, 8.0)),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut ui = Ui::new(&pressed, &mut memory, &theme);
    ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let _ = ui.finish_output();

    let crossed = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(14.0, 8.0)),
            delta: Vec2::new(6.0, 0.0),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        keyboard: KeyboardInput {
            modifiers: shift(),
            events: Vec::new(),
        },
        ..UiInput::default()
    };
    let mut ui = Ui::new(&crossed, &mut memory, &theme);
    let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let frame = ui.finish_output();

    assert!(output.scrubbed);
    assert_eq!(output.step, 0.25);
    assert_eq!(value, 2.5);
    assert_eq!(state.text, "2.5");
    assert_eq!(memory.focused(), None);
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );

    let released = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(14.0, 8.0)),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        text_events: vec![TextInputEvent::Commit("X".to_owned())],
        ..UiInput::default()
    };
    let mut ui = Ui::new(&released, &mut memory, &theme);
    let released_output =
        ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
    let frame = ui.finish_output();
    assert!(!released_output.scrubbed);
    assert_eq!(value, 2.5);
    assert_eq!(state.text, "2.5");
    assert_eq!(memory.focused(), None);
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );
}

#[test]
fn later_press_outside_scrub_revokes_earlier_drag_authority() {
    let theme = default_dark_theme();
    let input = canonical([
        press(8.0, 8.0, 1),
        moved(14.0, 8.0, 6.0),
        release(14.0, 8.0, 1),
        press(220.0, 8.0, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("3");
    let mut value = 3.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();

    assert!(output.scrub_response.dragged);
    assert!(!output.scrubbed);
    assert_eq!(value, 3.0);
    assert_eq!(state.text, "3");
    assert_eq!(memory.focused(), None);
}

#[test]
fn read_only_and_disabled_scrubs_have_exact_access_semantics() {
    let theme = default_dark_theme();
    let id = root_child("number");

    let mut read_only_memory = UiMemory::new();
    read_only_memory.focus(id);
    let mut read_only_state = TextEditState::new("42");
    read_only_state.set_selection(TextSelection::new(0, 2));
    let mut read_only_value = 42.0;
    let input = copy_input();
    let mut ui = Ui::new(&input, &mut read_only_memory, &theme);
    let read_only = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut read_only_value,
        &mut read_only_state,
        NumericScrubInputConfig::new(1.0).read_only(true),
    );
    let frame = ui.finish_output();
    let node = frame.semantics.get(id).expect("read-only numeric node");

    assert!(read_only.read_only);
    assert_eq!(read_only_value, 42.0);
    assert_eq!(read_only_state.text, "42");
    assert!(node.focusable);
    assert!(!node.state.disabled);
    assert_eq!(
        node.state.value,
        Some(SemanticValue::Number {
            current: 42.0,
            min: 42.0,
            max: 42.0,
        })
    );
    assert!(!has_action(node, &SemanticActionKind::SetText));
    assert!(!has_action(node, &SemanticActionKind::SetValue));
    assert!(
        frame
            .platform_requests
            .contains(&PlatformRequest::CopyToClipboard("42".to_owned()))
    );
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("7");
    let mut disabled_value = 7.0;
    let disabled_input = UiInput::default();
    let mut ui = Ui::new(&disabled_input, &mut disabled_memory, &theme);
    let disabled = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut disabled_value,
        &mut disabled_state,
        NumericScrubInputConfig::new(1.0)
            .disabled(true)
            .read_only(true),
    );
    let frame = ui.finish_output();
    let node = frame.semantics.get(id).expect("disabled numeric node");
    assert!(disabled.read_only);
    assert!(node.state.disabled);
    assert!(!node.focusable);
    assert!(!has_action(node, &SemanticActionKind::SetText));
    assert!(!has_action(node, &SemanticActionKind::SetValue));
    assert_eq!(disabled_memory.focused(), None);
    assert_eq!(disabled_memory.text_input_owner(), None);
}

#[test]
#[allow(clippy::too_many_lines)]
fn scrub_access_modes_cover_read_only_navigation_viewport_history_and_disabled_inertia() {
    let theme = default_dark_theme();
    let id = root_child("number");

    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    state.insert_text("!");
    assert!(state.undo());
    state.set_caret(state.text.len());
    let mut expected_history = state.clone();
    let mut value = 10.0;
    let input = canonical([
        UiInputEvent::Key(KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift(),
            false,
        )),
        UiInputEvent::Key(KeyEvent::new(
            Key::Character("c".to_owned()),
            KeyState::Pressed,
            ctrl(),
            false,
        )),
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(Vec2::new(-24.0, 0.0)),
            position: Some(Point::new(8.0, 8.0)),
        },
    ]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        Rect::new(0.0, 0.0, 72.0, 24.0),
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0).read_only(true),
    );
    let frame = ui.finish_output();
    assert!(!output.scrubbed);
    assert_eq!(value, 10.0);
    assert_eq!(state.text, expected_history.text);
    assert!(!state.selection.is_caret());
    assert!(memory.scroll_offset(id).x > 0.0);
    assert!(
        frame
            .platform_requests
            .contains(&PlatformRequest::CopyToClipboard("z".to_owned()))
    );
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );
    assert!(
        output
            .input
            .field
            .widget
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Rect(_)))
            .count()
            >= 2
    );
    expected_history.set_selection(state.selection);
    let mut actual_history = state.clone();
    for operation in [TextEditState::undo, TextEditState::redo] {
        assert_eq!(
            operation(&mut actual_history),
            operation(&mut expected_history)
        );
        assert_eq!(actual_history, expected_history);
    }

    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("disabled");
    state.set_selection(TextSelection::new(0, state.text.len()));
    let expected = state.clone();
    let mut value = 4.0;
    let input = canonical([
        press(8.0, 8.0, 1),
        moved(18.0, 8.0, 10.0),
        release(18.0, 8.0, 1),
        UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        UiInputEvent::Key(KeyEvent::new(
            Key::Character("c".to_owned()),
            KeyState::Pressed,
            ctrl(),
            false,
        )),
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(Vec2::new(-24.0, 0.0)),
            position: Some(Point::new(8.0, 8.0)),
        },
    ]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0).disabled(true),
    );
    let frame = ui.finish_output();
    assert!(!output.scrubbed);
    assert_eq!(value, 4.0);
    assert_eq!(state, expected);
    assert_eq!(memory.focused(), None);
    assert_eq!(memory.text_input_owner(), None);
    assert_eq!(memory.scroll_offset(id), Vec2::ZERO);
    assert!(
        frame
            .platform_requests
            .iter()
            .all(|request| { matches!(request, PlatformRequest::StopTextInput) })
    );
    assert_eq!(
        output
            .input
            .field
            .widget
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Rect(_)))
            .count(),
        1
    );
}

#[test]
fn invalid_editable_scrub_keeps_text_semantics_without_set_value() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("bad draft");
    let mut value = 7.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let frame = ui.finish_output();
    let node = frame
        .semantics
        .get(root_child("number"))
        .expect("numeric semantics");
    assert_eq!(
        node.state.value,
        Some(SemanticValue::Text("bad draft".to_owned()))
    );
    assert!(has_action(node, &SemanticActionKind::SetText));
    assert!(!has_action(node, &SemanticActionKind::SetValue));
}

#[test]
fn valid_editable_scrub_exposes_numeric_value_and_both_mutation_actions() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("7");
    let mut value = 7.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0).with_range(0.0, 10.0),
    );
    let frame = ui.finish_output();
    let node = frame
        .semantics
        .get(root_child("number"))
        .expect("numeric semantics");
    assert_eq!(
        node.state.value,
        Some(SemanticValue::Number {
            current: 7.0,
            min: 0.0,
            max: 10.0,
        })
    );
    assert!(has_action(node, &SemanticActionKind::SetText));
    assert!(has_action(node, &SemanticActionKind::SetValue));
}

#[test]
fn ui_numeric_commit_and_revert_intents_remain_editable_only() {
    let theme = default_dark_theme();
    let id = root_child("number");

    let input = canonical([UiInputEvent::Key(KeyEvent::new(
        Key::Enter,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    ))]);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("42");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_input("number", FIELD_RECT, &mut state, false);
    let _ = ui.finish_output();
    assert!(output.policy.commit_requested);
    assert!(!output.policy.revert_requested);

    let input = canonical([UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    ))]);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("bad draft");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_input("number", FIELD_RECT, &mut state, false);
    let _ = ui.finish_output();
    assert!(!output.policy.commit_requested);
    assert!(output.policy.revert_requested);

    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("42");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_input("number", FIELD_RECT, &mut state, true);
    let _ = ui.finish_output();
    assert!(!output.policy.commit_requested);
    assert!(!output.policy.revert_requested);
}

#[test]
fn migrated_search_clear_and_numeric_invalid_enter_report_exact_outputs() {
    let theme = default_dark_theme();

    let search_id = root_child("search");
    let mut memory = UiMemory::new();
    memory.focus(search_id);
    let mut state = TextEditState::new("find");
    state.set_selection(TextSelection::new(0, state.text.len()));
    let input = canonical([UiInputEvent::Key(KeyEvent::new(
        Key::Backspace,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    ))]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.search_field("search", FIELD_RECT, &mut state, false);
    let _ = ui.finish_output();
    assert!(output.empty);
    assert!(output.query.is_empty());
    assert!(output.field.changed);

    for (key, draft) in [("invalid", "not-a-number"), ("empty", "")] {
        let id = root_child(key);
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new(draft);
        let input = canonical([UiInputEvent::Key(KeyEvent::new(
            Key::Enter,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        ))]);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let output = ui.numeric_input(key, FIELD_RECT, &mut state, false);
        let _ = ui.finish_output();
        assert!(!output.policy.commit_requested);
        assert_eq!(output.valid, draft.is_empty());
        assert_eq!(state.text, draft);
    }
}

#[test]
fn search_and_numeric_wrappers_share_one_ordered_owner_in_both_call_orders() {
    let theme = default_dark_theme();
    let search_rect = Rect::new(0.0, 0.0, 160.0, 24.0);
    let numeric_rect = Rect::new(0.0, 32.0, 160.0, 24.0);
    for target_numeric in [false, true] {
        let target_y = if target_numeric { 40.0 } else { 8.0 };
        let input = canonical([
            press(8.0, target_y, 1),
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        ]);

        for numeric_first in [false, true] {
            let mut memory = UiMemory::new();
            let mut search_state = TextEditState::new("find");
            let mut numeric_state = TextEditState::new("12");
            let mut ui = Ui::new(&input, &mut memory, &theme);
            if numeric_first {
                ui.numeric_input("number", numeric_rect, &mut numeric_state, false);
                ui.search_field("search", search_rect, &mut search_state, false);
            } else {
                ui.search_field("search", search_rect, &mut search_state, false);
                ui.numeric_input("number", numeric_rect, &mut numeric_state, false);
            }
            let frame = ui.finish_output();
            let target_id = root_child(if target_numeric { "number" } else { "search" });

            assert_eq!(search_state.text.contains('X'), !target_numeric);
            assert_eq!(numeric_state.text.contains('X'), target_numeric);
            assert_eq!(memory.focused(), Some(target_id));
            assert_eq!(memory.text_input_owner(), Some(target_id));
            assert_eq!(
                frame
                    .platform_requests
                    .iter()
                    .filter(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
                    .count(),
                1
            );
            assert!(frame.warnings.is_empty());
        }
    }
}

#[test]
fn focus_loss_cancel_threshold_and_later_press_never_place_a_scrub_caret() {
    let theme = default_dark_theme();
    let cases = [
        canonical([
            press(8.0, 8.0, 1),
            UiInputEvent::WindowFocusChanged(false),
            release(9.0, 8.0, 1),
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        ]),
        canonical([
            press(8.0, 8.0, 1),
            UiInputEvent::PointerReleaseAll {
                position: Some(Point::new(9.0, 8.0)),
            },
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        ]),
        canonical([
            press(8.0, 8.0, 1),
            release(12.0, 8.0, 1),
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        ]),
        canonical([
            press(8.0, 8.0, 1),
            release(9.0, 8.0, 1),
            UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
            press(220.0, 8.0, 1),
        ]),
    ];

    for (index, input) in cases.into_iter().enumerate() {
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("4");
        let mut value = 4.0;
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let output = ui.numeric_scrub_input(
            "number",
            FIELD_RECT,
            &mut value,
            &mut state,
            NumericScrubInputConfig::new(1.0),
        );
        let frame = ui.finish_output();

        assert_eq!(memory.focused(), None, "case {index}");
        assert!(!state.text.contains('X'), "case {index}");
        assert!(
            !frame
                .platform_requests
                .iter()
                .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
        );
        if index == 2 {
            assert!(output.scrubbed);
            assert_eq!(value, 8.0);
        } else {
            assert!(!output.scrubbed);
            assert_eq!(value, 4.0);
        }
    }
}

#[test]
fn path_runtime_preserves_child_ids_copy_policy_and_open_intent() {
    let theme = default_dark_theme();
    let path_id = root_child("path");
    let text_id = path_id.child("text");
    let browse_id = path_id.child("browse");
    let mut memory = UiMemory::new();
    memory.focus(text_id);
    let mut state = TextEditState::new("src/main.rs");
    state.set_selection(TextSelection::new(0, 3));
    let input = copy_input();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().read_only(true).open(true),
    );
    let frame = ui.finish_output();

    assert!(!output.browse_requested);
    assert!(!output.open_requested);
    assert!(frame.semantics.get(text_id).is_some());
    let browse = frame.semantics.get(browse_id).expect("browse semantic");
    assert!(browse.state.disabled);
    assert!(
        frame
            .platform_requests
            .contains(&PlatformRequest::CopyToClipboard("src".to_owned()))
    );
    assert!(frame.warnings.is_empty());

    let input = canonical([press(8.0, 8.0, 2), release(8.0, 8.0, 2)]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("src/main.rs");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let frame = ui.finish_output();
    assert!(output.open_requested);
    assert!(!output.browse_requested);
    assert!(!state.selection.is_caret());
    assert!(frame.warnings.is_empty());
}

#[test]
fn path_runtime_targets_clipboard_to_text_child_and_access_modes_reject_mutation() {
    let theme = default_dark_theme();
    let text_id = root_child("path").child("text");
    let other = root_child("other");

    let mut memory = UiMemory::new();
    memory.focus(text_id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));
    let input = canonical([
        UiInputEvent::ClipboardText(ClipboardText::new(other, "wrong")),
        UiInputEvent::ClipboardText(ClipboardText::new(text_id, "XY")),
    ]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let _ = ui.finish_output();
    assert!(output.changed);
    assert_eq!(state.text, "aXYd");
    assert!(!output.browse_requested);
    assert!(!output.open_requested);

    for (key, config) in [
        ("read-only", PathFieldConfig::new().read_only(true)),
        ("disabled", PathFieldConfig::new().disabled(true)),
    ] {
        let text_id = root_child(key).child("text");
        let mut memory = UiMemory::new();
        memory.focus(text_id);
        memory.set_text_input_owner(text_id);
        let mut state = TextEditState::new("keep");
        state.set_selection(TextSelection::new(0, state.text.len()));
        let expected = state.clone();
        let input = canonical([
            UiInputEvent::ClipboardText(ClipboardText::new(text_id, "replace")),
            press(140.0, 8.0, 1),
            release(140.0, 8.0, 1),
        ]);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let output = ui.path_field(key, FIELD_RECT, "Source", &mut state, config.open(true));
        let frame = ui.finish_output();
        assert_eq!(state, expected);
        assert!(!output.changed);
        assert!(!output.browse_requested);
        assert!(!output.open_requested);
        assert!(
            !frame
                .platform_requests
                .iter()
                .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
        );
    }
}

#[test]
fn vector_runtime_isolates_target_and_read_only_copy_by_exact_child_id() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let component_rects = vector3_component_rects(rect, VectorComponentLayout::default());
    let target = component_rects[1].value_rect.center();
    let input = canonical([
        press(target.x, target.y, 1),
        moved(target.x + 6.0, target.y, 6.0),
        release(target.x + 6.0, target.y, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut values = [1.0, 2.0, 3.0];
    let mut states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.vector3_scrub_input(
        "vector",
        rect,
        "Position",
        &mut values,
        &mut states,
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(0.5)),
    );
    let frame = ui.finish_output();
    assert!(output.components[1].scrubbed);
    assert_eq!(values, [1.0, 5.0, 3.0]);
    assert_eq!(states[0].text, "1");
    assert_eq!(states[1].text, "5");
    assert_eq!(states[2].text, "3");
    assert!(
        frame
            .semantics
            .get(root_child("vector").child("X"))
            .is_some()
    );
    assert!(
        frame
            .semantics
            .get(root_child("vector").child("Y"))
            .is_some()
    );
    assert!(
        frame
            .semantics
            .get(root_child("vector").child("Z"))
            .is_some()
    );
    assert!(frame.warnings.is_empty());

    let y_id = root_child("vector").child("Y");
    let mut memory = UiMemory::new();
    memory.focus(y_id);
    let mut values = [1.0, 2.0, 3.0];
    let mut states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    states[1].set_selection(TextSelection::new(0, 1));
    let input = copy_input();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.vector3_scrub_input(
        "vector",
        rect,
        "Position",
        &mut values,
        &mut states,
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(0.5)).read_only(true),
    );
    let frame = ui.finish_output();
    let node = frame.semantics.get(y_id).expect("Y semantic");
    assert!(output.read_only);
    assert_eq!(values, [1.0, 2.0, 3.0]);
    assert!(node.focusable);
    assert!(!node.state.disabled);
    assert!(!has_action(node, &SemanticActionKind::SetText));
    assert!(!has_action(node, &SemanticActionKind::SetValue));
    assert!(
        frame
            .platform_requests
            .contains(&PlatformRequest::CopyToClipboard("2".to_owned()))
    );
    assert!(frame.warnings.is_empty());
}

#[test]
fn vector_runtime_applies_disabled_over_nested_and_outer_read_only_precedence() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 220.0, 24.0);
    let cases = [
        (
            VectorScrubInputConfig::default().read_only(true),
            false,
            true,
        ),
        (
            VectorScrubInputConfig::new(NumericScrubInputConfig::default().read_only(true)),
            false,
            true,
        ),
        (
            VectorScrubInputConfig::new(NumericScrubInputConfig::default().read_only(true))
                .disabled(true),
            true,
            true,
        ),
        (
            VectorScrubInputConfig::new(NumericScrubInputConfig::default().disabled(true))
                .read_only(true),
            true,
            true,
        ),
        (
            VectorScrubInputConfig::default().disabled(true),
            true,
            false,
        ),
    ];

    for (config, expected_disabled, expected_read_only) in cases {
        let mut memory = UiMemory::new();
        let mut values = [1.0, 2.0];
        let mut states = [TextEditState::new("1"), TextEditState::new("2")];
        let input = UiInput::default();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let output =
            ui.vector2_scrub_input("vector", rect, "Offset", &mut values, &mut states, config);
        let frame = ui.finish_output();
        assert_eq!(output.disabled, expected_disabled);
        assert_eq!(output.read_only, expected_read_only);
        assert!(output.components.iter().all(|component| {
            component.read_only == expected_read_only
                && component
                    .input
                    .field
                    .widget
                    .response
                    .is_some_and(|response| response.state.disabled == expected_disabled)
        }));
        assert_eq!(values, [1.0, 2.0]);
        assert_eq!(states[0].text, "1");
        assert_eq!(states[1].text, "2");
        assert!(frame.warnings.is_empty());
    }
}

#[test]
fn narrow_canonical_wrappers_never_emit_zero_area_text_clips() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut values = [1.0, 2.0, 3.0];
    let mut states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.vector3_scrub_input(
        "vector",
        Rect::new(0.0, 0.0, 30.0, 24.0),
        "Position",
        &mut values,
        &mut states,
        VectorScrubInputConfig::default(),
    );
    let frame = ui.finish_output();
    assert!(frame.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            kinetik_ui_core::Primitive::ClipBegin { rect, .. }
                if rect.width <= 0.0 || rect.height <= 0.0
        )
    }));
    assert!(frame.warnings.is_empty());
}

#[test]
fn every_migrated_wrapper_publishes_caret_geometry_not_field_geometry() {
    let theme = default_dark_theme();

    let input = canonical([press(8.0, 8.0, 1)]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("query");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.search_field("search", FIELD_RECT, &mut state, false);
    assert_caret_start(&ui.finish_output(), FIELD_RECT);

    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("42");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_input("number", FIELD_RECT, &mut state, false);
    assert_caret_start(&ui.finish_output(), FIELD_RECT);

    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("src/lib.rs");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let path = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new(),
    );
    assert_caret_start(
        &ui.finish_output(),
        path.field.widget.response.unwrap().rect,
    );

    let vector_rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let component_rects = vector3_component_rects(vector_rect, VectorComponentLayout::default());
    let target = component_rects[0].value_rect.center();
    let input = canonical([
        press(target.x, target.y, 1),
        release(target.x + 1.0, target.y, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut values = [1.0, 2.0, 3.0];
    let mut states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.vector3_scrub_input(
        "vector",
        vector_rect,
        "Position",
        &mut values,
        &mut states,
        VectorScrubInputConfig::default(),
    );
    assert_caret_start(&ui.finish_output(), component_rects[0].value_rect);
}

#[test]
#[allow(clippy::too_many_lines)]
fn migrated_wrappers_retain_hidden_caret_viewports_then_publish_next_frame() {
    let theme = default_dark_theme();
    let narrow = Rect::new(0.0, 0.0, 52.0, 24.0);

    for kind in ["search", "numeric", "path"] {
        let id = if kind == "path" {
            root_child(kind).child("text")
        } else {
            root_child(kind)
        };
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("123456789012345678901234567890");
        state.set_caret(state.text.len());
        let mut value = 1.0;
        let input = UiInput::default();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        match kind {
            "search" => {
                let _ = ui.search_field(kind, narrow, &mut state, false);
            }
            "numeric" => {
                let _ = ui.numeric_scrub_input(
                    kind,
                    narrow,
                    &mut value,
                    &mut state,
                    NumericScrubInputConfig::default(),
                );
            }
            "path" => {
                let _ = ui.path_field(
                    kind,
                    narrow,
                    "Source",
                    &mut state,
                    PathFieldConfig::new().browse(false),
                );
            }
            _ => unreachable!(),
        }
        let first = ui.finish_output();
        assert!(memory.scroll_offset(id).x > 0.0, "{kind}");
        assert!(
            !first
                .platform_requests
                .iter()
                .any(|request| matches!(request, PlatformRequest::StartTextInput { .. })),
            "{kind}"
        );

        let input = UiInput::default();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        match kind {
            "search" => {
                let _ = ui.search_field(kind, narrow, &mut state, false);
            }
            "numeric" => {
                let _ = ui.numeric_scrub_input(
                    kind,
                    narrow,
                    &mut value,
                    &mut state,
                    NumericScrubInputConfig::default(),
                );
            }
            "path" => {
                let _ = ui.path_field(
                    kind,
                    narrow,
                    "Source",
                    &mut state,
                    PathFieldConfig::new().browse(false),
                );
            }
            _ => unreachable!(),
        }
        assert_caret_start(&ui.finish_output(), narrow);
    }

    let rect = Rect::new(0.0, 0.0, 120.0, 24.0);
    let component_rects = vector2_component_rects(rect, VectorComponentLayout::default());
    let id = root_child("vector").child("X");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut values = [1.0, 2.0];
    let mut states = [
        TextEditState::new("12345678901234567890"),
        TextEditState::new("2"),
    ];
    states[0].set_caret(states[0].text.len());
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let _ = ui.vector2_scrub_input(
        "vector",
        rect,
        "Offset",
        &mut values,
        &mut states,
        VectorScrubInputConfig::default(),
    );
    let first = ui.finish_output();
    assert!(memory.scroll_offset(id).x > 0.0);
    assert!(
        !first
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let _ = ui.vector2_scrub_input(
        "vector",
        rect,
        "Offset",
        &mut values,
        &mut states,
        VectorScrubInputConfig::default(),
    );
    assert_caret_start(&ui.finish_output(), component_rects[0].value_rect);
}

#[test]
fn domain_drag_access_transition_fences_then_recovers_selection() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("12");
    let mut value = 12.0;

    let input = canonical([press(8.0, 8.0, 1)]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    assert_eq!(memory.focused(), None);

    let input = canonical([moved(10.0, 8.0, 2.0)]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let transition = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0).read_only(true),
    );
    let frame = ui.finish_output();
    assert!(!transition.scrubbed);
    assert_eq!(memory.focused(), None);
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );

    let input = canonical([press(8.0, 8.0, 1), release(9.0, 8.0, 1)]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let recovered = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0).read_only(true),
    );
    let frame = ui.finish_output();
    assert!(!recovered.scrubbed);
    assert_eq!(memory.focused(), Some(root_child("number")));
    assert!(
        !frame
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn nonfinite_and_overflowing_scrub_arithmetic_fails_closed() {
    let theme = default_dark_theme();
    let id = root_child("number");
    let input = canonical([press(8.0, 8.0, 1), moved(14.0, 8.0, 6.0)]);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("a");
    state.insert_text("b");
    state.insert_text("c");
    assert!(state.undo());
    state.set_selection(TextSelection::new(0, 1));
    let mut value = 8.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    let expected = state.clone();
    let expected_value = value;

    let input = canonical([
        UiInputEvent::PointerMoved {
            position: Point::new(16.0, 8.0),
            delta: Vec2::new(f32::INFINITY, 0.0),
        },
        UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
        UiInputEvent::Key(KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )),
        UiInputEvent::ModifiersChanged(ctrl()),
    ]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    assert!(output.scrub_response.dragged);
    assert!(!output.scrubbed);
    assert_eq!(output.step, 10.0);
    assert_eq!(value, expected_value);
    assert_eq!(state, expected);
    assert!(memory.claim_text_input_events(id));
    let mut actual_history = state.clone();
    let mut expected_history = expected.clone();
    for operation in [TextEditState::undo, TextEditState::redo] {
        assert_eq!(
            operation(&mut actual_history),
            operation(&mut expected_history)
        );
        assert_eq!(actual_history, expected_history);
    }

    let input = canonical([
        press(8.0, 8.0, 1),
        moved(14.0, 8.0, 6.0),
        UiInputEvent::Text(TextInputEvent::Commit("X".to_owned())),
    ]);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("20");
    state.set_selection(TextSelection::new(0, 1));
    let expected = state.clone();
    let mut value = 20.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(f32::MAX),
    );
    let _ = ui.finish_output();
    assert!(output.scrub_response.dragged);
    assert!(!output.scrubbed);
    assert_eq!(value, 20.0);
    assert_eq!(state, expected);

    let replacement = format!("{}", f32::MAX);
    let input = canonical([
        press(8.0, 8.0, 1),
        moved(14.0, 8.0, 6.0),
        UiInputEvent::Text(TextInputEvent::Commit(replacement)),
    ]);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("1");
    state.set_selection(TextSelection::new(0, 1));
    let expected = state.clone();
    let mut value = 1.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(f32::MAX / 8.0),
    );
    let _ = ui.finish_output();
    assert!(output.scrub_response.dragged);
    assert!(!output.scrubbed);
    assert_eq!(value, 1.0);
    assert_eq!(state, expected);
    assert!(memory.claim_text_input_events(id));
}

#[test]
fn browse_press_preempts_path_text_owner_and_emits_only_browse() {
    let theme = default_dark_theme();
    let text_id = root_child("path").child("text");
    let mut memory = UiMemory::new();
    memory.focus(text_id);
    memory.set_text_input_owner(text_id);
    let mut state = TextEditState::new("src/main.rs");
    let input = canonical([press(140.0, 8.0, 1), release(140.0, 8.0, 1)]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let frame = ui.finish_output();

    assert!(output.browse_requested);
    assert!(!output.open_requested);
    assert_eq!(memory.focused(), None);
    assert_eq!(memory.text_input_owner(), None);
    assert!(frame.warnings.is_empty());
}

#[test]
fn later_browse_transaction_discards_earlier_path_double_click_open() {
    let theme = default_dark_theme();
    let input = canonical([
        press(8.0, 8.0, 2),
        release(8.0, 8.0, 2),
        press(140.0, 8.0, 1),
        release(140.0, 8.0, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("src/main.rs");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let frame = ui.finish_output();

    assert!(output.browse_requested);
    assert!(!output.open_requested);
    assert!(state.selection.is_caret());
    assert_eq!(memory.focused(), None);
    assert!(frame.warnings.is_empty());
}

#[test]
fn path_open_requires_an_accepted_completed_double_click_release() {
    let theme = default_dark_theme();
    let text_id = root_child("path").child("text");
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("src/main.rs");

    let input = canonical([press(8.0, 8.0, 2)]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let pressed = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let _ = ui.finish_output();
    assert!(!pressed.open_requested);
    assert_eq!(memory.focused(), Some(text_id));

    let input = canonical([release(8.0, 8.0, 2)]);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let released = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let frame = ui.finish_output();
    assert!(released.open_requested);
    assert!(!released.browse_requested);
    assert!(!state.selection.is_caret());
    assert!(frame.warnings.is_empty());

    let input = canonical([press(8.0, 8.0, 2), UiInputEvent::WindowFocusChanged(false)]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("src/main.rs");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let cancelled = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let _ = ui.finish_output();
    assert!(!cancelled.open_requested);
}

#[test]
fn scrub_place_caret_uses_causal_release_click_count_and_modifiers() {
    let theme = default_dark_theme();
    let id = root_child("number");

    let input = canonical([
        press(58.0, 8.0, 1),
        release(58.0, 8.0, 2),
        UiInputEvent::ModifiersChanged(ctrl()),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("alpha beta");
    let mut value = 1.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    assert!(!output.scrubbed);
    assert_eq!(state.selected_text(), Some("beta"));
    assert_eq!(memory.focused(), Some(id));

    let input = canonical([press(58.0, 8.0, 2), release(58.0, 8.0, 1)]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("alpha beta");
    let mut value = 1.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    assert!(state.selection.is_caret());

    let input = canonical([
        press(8.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(shift()),
        release(9.0, 8.0, 1),
        UiInputEvent::ModifiersChanged(ctrl()),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("alpha beta");
    let mut value = 1.0;
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "number",
        FIELD_RECT,
        &mut value,
        &mut state,
        NumericScrubInputConfig::new(1.0),
    );
    let _ = ui.finish_output();
    assert!(!state.selection.is_caret());
    assert_eq!(state.selection.anchor, "alpha beta".len());
}

#[test]
fn path_open_uses_the_accepted_release_not_press_or_aggregate_evidence() {
    let theme = default_dark_theme();

    for (press_count, release_count, expected_open) in [(1, 2, true), (2, 1, false)] {
        let input = canonical([
            press(24.0, 8.0, press_count),
            release(24.0, 8.0, release_count),
        ]);
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("src/main.rs");
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let output = ui.path_field(
            "path",
            FIELD_RECT,
            "Source",
            &mut state,
            PathFieldConfig::new().open(true),
        );
        let _ = ui.finish_output();
        assert_eq!(output.open_requested, expected_open);
        assert!(!output.browse_requested);
    }

    let input = canonical([
        press(24.0, 8.0, 2),
        release(24.0, 8.0, 2),
        press(28.0, 8.0, 1),
        release(28.0, 8.0, 1),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("src/main.rs");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.path_field(
        "path",
        FIELD_RECT,
        "Source",
        &mut state,
        PathFieldConfig::new().open(true),
    );
    let _ = ui.finish_output();
    assert!(!output.open_requested);
    assert!(state.selection.is_caret());
}
