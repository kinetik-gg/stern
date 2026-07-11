use kinetik_ui_core::{
    ClipId, FrameWarning, InputStreamConflict, Key, KeyEvent, KeyState, Modifiers, MouseButton,
    PhysicalKey, Point, Primitive, Rect, TextInputEvent, Transform, UiInput, UiInputEvent,
    UiMemory, UiTestHarness, Vec2, WidgetId, default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{multi_line_text_field, text_field};

fn hardware_event(key: Key, text: &str) -> UiInputEvent {
    UiInputEvent::Key(
        KeyEvent::with_physical_key(
            key,
            PhysicalKey::Unidentified,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )
        .with_text(text),
    )
}

fn focused_memory(id: WidgetId) -> UiMemory {
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    memory
}

#[test]
fn focused_field_claims_canonical_input_once_even_when_called_repeatedly() {
    let id = WidgetId::from_key("field");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));
    let mut memory = focused_memory(id);
    let mut state = TextEditState::new("");

    let first = text_field(
        id,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let second = text_field(
        id,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(first.changed);
    assert!(!second.changed);
    assert_eq!(state.text, "x");
}

#[test]
fn preclaim_handoff_routes_to_new_owner_and_postclaim_handoff_never_replays() {
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));

    let mut preclaim_memory = focused_memory(first);
    preclaim_memory.focus(second);
    preclaim_memory.set_text_input_owner(second);
    let mut first_state = TextEditState::new("");
    let mut second_state = TextEditState::new("");
    let first_output = text_field(
        first,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut first_state,
        &input,
        &mut preclaim_memory,
        &theme,
        false,
    );
    let second_output = text_field(
        second,
        Rect::new(0.0, 30.0, 120.0, 24.0),
        &mut second_state,
        &input,
        &mut preclaim_memory,
        &theme,
        false,
    );
    assert!(!first_output.changed);
    assert!(second_output.changed);
    assert_eq!(first_state.text, "");
    assert_eq!(second_state.text, "x");

    let mut postclaim_memory = focused_memory(first);
    let mut first_state = TextEditState::new("");
    let mut second_state = TextEditState::new("");
    let first_output = text_field(
        first,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut first_state,
        &input,
        &mut postclaim_memory,
        &theme,
        false,
    );
    postclaim_memory.focus(second);
    postclaim_memory.set_text_input_owner(second);
    let second_output = text_field(
        second,
        Rect::new(0.0, 30.0, 120.0, 24.0),
        &mut second_state,
        &input,
        &mut postclaim_memory,
        &theme,
        false,
    );
    assert!(first_output.changed);
    assert!(!second_output.changed);
    assert_eq!(first_state.text, "x");
    assert_eq!(second_state.text, "");
}

#[test]
fn mixed_mode_conflict_claims_but_applies_no_ordered_editing() {
    let id = WidgetId::from_key("field");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));
    input
        .text_events
        .push(TextInputEvent::Commit("legacy".to_owned()));
    let mut memory = focused_memory(id);
    let mut state = TextEditState::new("base");

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "base");
    assert!(!memory.claim_text_input_events(id));
}

#[test]
fn standalone_unvalidated_memory_rejects_pointer_projection_conflict() {
    let id = WidgetId::from_key("standalone-conflict");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(2.0, 3.0),
        delta: Vec2::ZERO,
    });
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));
    input.pointer.position = Some(Point::new(8.0, 9.0));
    let mut memory = focused_memory(id);
    let mut state = TextEditState::new("base");

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "base");
    assert!(!memory.claim_text_input_events(id));
}

fn assert_scoped_text_survives_suppressed_pointer_end(pointer_end: UiInputEvent) {
    let id = WidgetId::from_key("scoped-field");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(5.0, 5.0)),
    });
    input.push_event(pointer_end);
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));
    assert_eq!(input.validate_event_stream(), Ok(()));

    let mut harness = UiTestHarness::new();
    *harness.input_mut() = input;
    harness.memory_mut().focus(id);
    harness.memory_mut().set_text_input_owner(id);
    let mut state = TextEditState::new("");

    let ((localized, first_changed, second_changed), frame) = harness.run_frame(|ui| {
        ui.register_id(id);
        ui.push_primitive(Primitive::ClipBegin {
            id: ClipId::from_raw(91),
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
        });
        let localized = ui.input().clone();
        let (input, memory) = ui.input_and_memory_mut();
        let first = text_field(
            id,
            Rect::new(0.0, 0.0, 18.0, 18.0),
            &mut state,
            input,
            memory,
            &theme,
            false,
        );
        let second = text_field(
            id,
            Rect::new(0.0, 0.0, 18.0, 18.0),
            &mut state,
            input,
            memory,
            &theme,
            false,
        );
        ui.push_primitive(Primitive::ClipEnd {
            id: ClipId::from_raw(91),
        });
        (localized, first.changed, second.changed)
    });

    assert_eq!(
        localized.validate_event_stream(),
        Err(InputStreamConflict::Pointer)
    );
    assert!(first_changed);
    assert!(!second_changed);
    assert_eq!(state.text, "x");
    assert!(frame.warnings.is_empty());
}

#[test]
fn valid_root_scoped_text_ignores_held_pointer_suppressed_outside_clip() {
    assert_scoped_text_survives_suppressed_pointer_end(UiInputEvent::PointerMoved {
        position: Point::new(30.0, 30.0),
        delta: Vec2::new(25.0, 25.0),
    });
}

#[test]
fn valid_root_scoped_text_ignores_uncaptured_release_suppressed_outside_clip() {
    assert_scoped_text_survives_suppressed_pointer_end(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(30.0, 30.0)),
    });
}

#[test]
fn root_conflict_warns_once_and_blocks_all_text_owner_claims() {
    let first = WidgetId::from_key("first-conflicted");
    let second = WidgetId::from_key("second-conflicted");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(2.0, 3.0),
        delta: Vec2::ZERO,
    });
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));
    input.pointer.position = Some(Point::new(9.0, 9.0));

    let mut harness = UiTestHarness::new();
    *harness.input_mut() = input;
    harness.memory_mut().focus(first);
    harness.memory_mut().set_text_input_owner(first);
    let mut first_state = TextEditState::new("one");
    let mut second_state = TextEditState::new("two");

    let ((first_changed, second_changed), frame) = harness.run_frame(|ui| {
        ui.register_id(first);
        ui.register_id(second);
        let (input, memory) = ui.input_and_memory_mut();
        let first_output = text_field(
            first,
            Rect::new(0.0, 0.0, 120.0, 24.0),
            &mut first_state,
            input,
            memory,
            &theme,
            false,
        );
        memory.focus(second);
        memory.set_text_input_owner(second);
        let second_output = text_field(
            second,
            Rect::new(0.0, 30.0, 120.0, 24.0),
            &mut second_state,
            input,
            memory,
            &theme,
            false,
        );
        (first_output.changed, second_output.changed)
    });

    assert!(!first_changed);
    assert!(!second_changed);
    assert_eq!(first_state.text, "one");
    assert_eq!(second_state.text, "two");
    assert_eq!(
        frame.warnings,
        vec![FrameWarning::InputStreamConflict {
            conflict: InputStreamConflict::Pointer,
        }]
    );
}

#[test]
fn combined_text_and_pointer_conflict_preserves_scoped_snapshot_and_fails_closed() {
    let id = WidgetId::from_key("combined-conflict");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(12.0, 12.0),
        delta: Vec2::new(4.0, 6.0),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(12.0, 12.0)),
    });
    input.push_event(hardware_event(Key::Character("x".to_owned()), "x"));
    input
        .text_events
        .push(TextInputEvent::Commit("legacy".to_owned()));
    input.pointer.delta = Vec2::new(50.0, 90.0);
    input.pointer.click_count = 7;
    assert_eq!(
        input.validate_event_stream(),
        Err(InputStreamConflict::TextEvents)
    );

    let mut harness = UiTestHarness::new();
    *harness.input_mut() = input;
    harness.memory_mut().focus(id);
    harness.memory_mut().set_text_input_owner(id);
    let mut state = TextEditState::new("base");

    let ((localized, changed), frame) = harness.run_frame(|ui| {
        ui.register_id(id);
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 3.0,
        ))));
        ui.push_primitive(Primitive::ClipBegin {
            id: ClipId::from_raw(92),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        let localized = ui.input().clone();
        let (input, memory) = ui.input_and_memory_mut();
        let field = text_field(
            id,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            &mut state,
            input,
            memory,
            &theme,
            false,
        );
        ui.push_primitive(Primitive::ClipEnd {
            id: ClipId::from_raw(92),
        });
        ui.push_primitive(Primitive::TransformEnd);
        (localized, field.changed)
    });

    assert_eq!(localized.pointer.position, Some(Point::new(6.0, 4.0)));
    assert_eq!(localized.pointer.delta, Vec2::new(25.0, 30.0));
    assert_eq!(localized.pointer.click_count, 7);
    assert!(localized.events.iter().any(|event| matches!(
        event,
        UiInputEvent::PointerMoved {
            position: Point { x: 6.0, y: 4.0 },
            delta: Vec2 { x: 2.0, y: 2.0 },
        }
    )));
    assert!(localized.events.iter().any(|event| matches!(
        event,
        UiInputEvent::PointerButton {
            click_count: 1,
            position: Some(Point { x: 6.0, y: 4.0 }),
            ..
        }
    )));
    assert!(!changed);
    assert_eq!(state.text, "base");
    assert_eq!(
        frame.warnings,
        vec![FrameWarning::InputStreamConflict {
            conflict: InputStreamConflict::TextEvents,
        }]
    );
}

#[test]
fn canonical_multiline_enter_is_inserted_once_at_its_stream_position() {
    let id = WidgetId::from_key("multiline");
    let theme = default_dark_theme();
    let mut input = UiInput::default();
    input.push_event(hardware_event(Key::Character("a".to_owned()), "a"));
    input.push_event(hardware_event(Key::Enter, "\r"));
    input.push_event(hardware_event(Key::Character("b".to_owned()), "b"));
    let mut memory = focused_memory(id);
    let mut state = TextEditState::new("");

    let output = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 120.0, 80.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "a\nb");
}
