//! Canonical ordered input and compatibility-projection conformance coverage.

use kinetik_ui_core::{
    ClipboardText, InputStreamConflict, InputWheelDelta, Key, KeyEvent, KeyState, Modifiers,
    MouseButton, PhysicalKey, Point, TextInputEvent, UiInput, UiInputEvent, UiTestHarness, Vec2,
    WidgetId,
};

fn hardware_key(text: &str) -> KeyEvent {
    KeyEvent::with_physical_key(
        Key::Character(text.to_owned()),
        PhysicalKey::KeyA,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )
    .with_text(text)
}

#[test]
fn push_event_preserves_order_event_time_geometry_and_typed_wheel_provenance() {
    let field = WidgetId::from_key("field");
    let mut input = UiInput::default();
    let events = vec![
        UiInputEvent::PointerMoved {
            position: Point::new(10.0, 20.0),
            delta: Vec2::ZERO,
        },
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 2,
            position: Some(Point::new(10.0, 20.0)),
        },
        UiInputEvent::PointerMoved {
            position: Point::new(14.0, 27.0),
            delta: Vec2::new(4.0, 7.0),
        },
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Lines(Vec2::new(0.0, -1.0)),
            position: Some(Point::new(14.0, 27.0)),
        },
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(Vec2::new(3.0, -8.0)),
            position: Some(Point::new(14.0, 27.0)),
        },
        UiInputEvent::Text(TextInputEvent::Commit("before".to_owned())),
        UiInputEvent::Key(hardware_key("a")),
        UiInputEvent::ClipboardText(ClipboardText::new(field, "clip")),
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: false,
            click_count: 2,
            position: Some(Point::new(14.0, 27.0)),
        },
        UiInputEvent::WindowFocusChanged(false),
    ];

    for event in events.clone() {
        input.push_event(event);
    }

    assert_eq!(input.events, events);
    assert_eq!(input.pointer.position, Some(Point::new(14.0, 27.0)));
    assert_eq!(input.pointer.delta, Vec2::new(4.0, 7.0));
    assert_eq!(input.pointer.wheel_delta, Vec2::new(3.0, -9.0));
    assert!(input.pointer.primary.pressed);
    assert!(input.pointer.primary.released);
    assert!(!input.pointer.primary.down);
    assert_eq!(input.pointer.click_count, 2);
    assert_eq!(input.keyboard.events, vec![hardware_key("a")]);
    assert_eq!(input.text_events.len(), 1);
    assert_eq!(input.clipboard_text.len(), 1);
    assert!(!input.window_focused);
    assert_eq!(input.validate_event_stream(), Ok(()));
}

#[test]
fn positional_pointer_events_project_definitive_evidence_and_none_retains_position() {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(1.0, 2.0)),
    });
    assert_eq!(input.pointer.position, Some(Point::new(1.0, 2.0)));

    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Lines(Vec2::new(0.0, -1.0)),
        position: None,
    });
    assert_eq!(input.pointer.position, Some(Point::new(1.0, 2.0)));

    input.push_event(UiInputEvent::PointerReleaseAll {
        position: Some(Point::new(3.0, 4.0)),
    });
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Pixels(Vec2::new(2.0, 3.0)),
        position: None,
    });
    assert_eq!(input.pointer.position, Some(Point::new(3.0, 4.0)));

    input.push_event(UiInputEvent::PointerLeft);
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Lines(Vec2::new(1.0, 0.0)),
        position: None,
    });
    assert_eq!(input.pointer.position, None);
    assert_eq!(input.validate_event_stream(), Ok(()));
}

#[test]
fn root_validation_rejects_mutated_final_pointer_position() {
    let mut positioned = UiInput::default();
    positioned.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Pixels(Vec2::new(1.0, 2.0)),
        position: Some(Point::new(4.0, 5.0)),
    });
    positioned.pointer.position = Some(Point::new(9.0, 9.0));
    assert_eq!(
        positioned.validate_event_stream(),
        Err(InputStreamConflict::Pointer)
    );

    let mut left = UiInput::default();
    left.push_event(UiInputEvent::PointerLeft);
    left.pointer.position = Some(Point::new(1.0, 1.0));
    assert_eq!(
        left.validate_event_stream(),
        Err(InputStreamConflict::Pointer)
    );
}

#[test]
fn mixed_canonical_and_direct_projection_mutation_fails_deterministically() {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Key(hardware_key("a")));
    input
        .text_events
        .push(TextInputEvent::Commit("legacy".to_owned()));

    assert_eq!(
        input.validate_event_stream(),
        Err(InputStreamConflict::TextEvents)
    );
    assert_eq!(
        input.effective_text_events(),
        Err(InputStreamConflict::TextEvents)
    );
}

#[test]
fn false_final_focus_without_an_ordered_focus_loss_fails_closed() {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Key(hardware_key("a")));
    input.window_focused = false;

    assert_eq!(
        input.validate_event_stream(),
        Err(InputStreamConflict::WindowFocus)
    );
}

#[test]
fn legacy_text_synthesis_matches_the_previous_domain_order() {
    let field = WidgetId::from_key("field");
    let clipboard = KeyEvent::with_physical_key(
        Key::Character("v".to_owned()),
        PhysicalKey::KeyV,
        KeyState::Pressed,
        Modifiers::new(false, true, false, false),
        false,
    );
    let edit = KeyEvent::new(
        Key::Backspace,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    );
    let input = UiInput {
        keyboard: kinetik_ui_core::KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![edit.clone(), clipboard.clone()],
        },
        text_events: vec![TextInputEvent::Commit("typed".to_owned())],
        clipboard_text: vec![ClipboardText::new(field, "pasted")],
        window_focused: false,
        ..UiInput::default()
    };

    assert_eq!(
        input.effective_text_events().expect("legacy synthesis"),
        vec![
            UiInputEvent::WindowFocusChanged(false),
            UiInputEvent::Key(clipboard),
            UiInputEvent::Text(TextInputEvent::Commit("typed".to_owned())),
            UiInputEvent::ClipboardText(ClipboardText::new(field, "pasted")),
            UiInputEvent::Key(edit),
        ]
    );
}

#[test]
fn begin_frame_clears_stream_and_transient_projections_together() {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(4.0, 5.0),
        delta: Vec2::new(1.0, 2.0),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(4.0, 5.0)),
    });
    input.push_event(UiInputEvent::ModifiersChanged(Modifiers::new(
        true, false, false, false,
    )));
    input.push_event(UiInputEvent::Key(hardware_key("a")));

    input.begin_frame();

    assert!(input.events.is_empty());
    assert_eq!(input.pointer.position, Some(Point::new(4.0, 5.0)));
    assert!(input.pointer.primary.down);
    assert!(!input.pointer.primary.pressed);
    assert_eq!(input.pointer.delta, Vec2::ZERO);
    assert!(input.keyboard.events.is_empty());
    assert_eq!(input.keyboard.modifiers, Modifiers::default());
}

#[test]
fn harness_official_producers_keep_canonical_projections_valid() {
    let mut harness = UiTestHarness::new();
    harness.set_window_focused(true);
    harness.set_pointer_position(Point::new(2.0, 3.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    harness.set_pointer_position(Point::new(8.0, 13.0));
    harness.wheel_lines(Vec2::new(0.0, -1.0));
    harness.wheel_pixels(Vec2::new(4.0, -6.0));
    harness.pointer_release(MouseButton::Primary);
    harness.set_modifiers(Modifiers::new(false, false, true, false));
    harness.key_event_with_text(
        Key::Unidentified,
        PhysicalKey::Unidentified,
        KeyState::Pressed,
        false,
        "é",
    );

    let input = harness.input();
    assert_eq!(input.validate_event_stream(), Ok(()));
    assert!(matches!(
        input.events[2],
        UiInputEvent::PointerButton {
            position: Some(Point { x: 2.0, y: 3.0 }),
            click_count: 2,
            ..
        }
    ));
    assert!(matches!(
        input.events[4],
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Lines(_),
            ..
        }
    ));
    assert!(matches!(
        input.events[5],
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(_),
            ..
        }
    ));
}

#[test]
fn text_event_claim_is_owner_only_single_use_and_handoff_safe() {
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let mut memory = kinetik_ui_core::UiMemory::new();

    memory.focus(first);
    memory.set_text_input_owner(first);
    memory.focus(second);
    memory.set_text_input_owner(second);
    assert!(!memory.claim_text_input_events(first));
    assert!(memory.claim_text_input_events(second));
    assert!(!memory.claim_text_input_events(second));

    memory.focus(first);
    memory.set_text_input_owner(first);
    assert!(!memory.claim_text_input_events(first));

    memory.begin_frame();
    assert!(memory.claim_text_input_events(first));
}
