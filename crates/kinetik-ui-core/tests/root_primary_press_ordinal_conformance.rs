//! Conformance coverage for cached root primary-Press provenance.

use kinetik_ui_core::{
    ClipId, ClipboardText, FrameWarning, InputStreamConflict, InputWheelDelta, Key, KeyEvent,
    KeyState, LayerId, Modifiers, MouseButton, Point, PointerButtonState, Primitive, Rect,
    TextInputEvent, Transform, UiInputEvent, UiTestHarness, Vec2, WidgetId,
};

fn pointer_button(button: MouseButton, down: bool, position: Option<Point>) -> UiInputEvent {
    UiInputEvent::PointerButton {
        button,
        down,
        click_count: 1,
        position,
    }
}

#[test]
fn empty_legacy_and_canonical_streams_without_primary_press_return_none() {
    let mut harness = UiTestHarness::new();
    let (legacy, _) = harness.run_frame(|ui| ui.last_root_primary_press_ordinal());
    assert_eq!(legacy, None);

    let mut legacy_snapshot = UiTestHarness::new();
    legacy_snapshot.input_mut().pointer.position = Some(Point::new(3.0, 4.0));
    legacy_snapshot.input_mut().pointer.primary = PointerButtonState::new(true, true, false);
    let (snapshot_only, _) = legacy_snapshot.run_frame(|ui| ui.last_root_primary_press_ordinal());
    assert_eq!(snapshot_only, None);

    harness.input_mut().push_event(pointer_button(
        MouseButton::Secondary,
        true,
        Some(Point::new(1.0, 1.0)),
    ));
    harness.input_mut().push_event(pointer_button(
        MouseButton::Primary,
        false,
        Some(Point::new(1.0, 1.0)),
    ));
    let (without_press, _) = harness.run_frame(|ui| ui.last_root_primary_press_ordinal());
    assert_eq!(without_press, None);
}

#[test]
fn every_non_primary_press_event_class_is_ignored() {
    let mut harness = UiTestHarness::new();
    let input = harness.input_mut();
    input.push_event(pointer_button(
        MouseButton::Other(9),
        true,
        Some(Point::new(1.0, 1.0)),
    ));
    input.push_event(UiInputEvent::PointerReleaseAll {
        position: Some(Point::new(1.0, 1.0)),
    });
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Pixels(Vec2::new(2.0, -3.0)),
        position: Some(Point::new(1.0, 1.0)),
    });
    input.push_event(UiInputEvent::ModifiersChanged(Modifiers::new(
        true, false, false, false,
    )));
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Character("x".into()),
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    input.push_event(UiInputEvent::Text(TextInputEvent::Commit("x".into())));
    input.push_event(UiInputEvent::ClipboardText(ClipboardText::new(
        WidgetId::from_key("target"),
        "clipboard",
    )));
    input.push_event(UiInputEvent::ImeEnabled(true));
    input.push_event(UiInputEvent::WindowFocusChanged(false));

    let (ordinal, _) = harness.run_frame(|ui| ui.last_root_primary_press_ordinal());
    assert_eq!(ordinal, None);
}

#[test]
fn one_primary_press_returns_its_exact_root_ordinal() {
    let mut harness = UiTestHarness::new();
    harness.input_mut().push_event(UiInputEvent::PointerMoved {
        position: Point::new(2.0, 3.0),
        delta: Vec2::ZERO,
    });
    harness.input_mut().push_event(pointer_button(
        MouseButton::Primary,
        true,
        Some(Point::new(2.0, 3.0)),
    ));

    let (ordinal, _) = harness.run_frame(|ui| ui.last_root_primary_press_ordinal());
    assert_eq!(ordinal, Some(1));
}

#[test]
fn multiple_primary_presses_choose_the_final_root_ordinal() {
    let mut harness = UiTestHarness::new();
    let input = harness.input_mut();
    input.push_event(pointer_button(
        MouseButton::Primary,
        true,
        Some(Point::new(1.0, 1.0)),
    ));
    input.push_event(pointer_button(
        MouseButton::Primary,
        false,
        Some(Point::new(1.0, 1.0)),
    ));
    input.push_event(pointer_button(
        MouseButton::Middle,
        true,
        Some(Point::new(3.0, 3.0)),
    ));
    input.push_event(pointer_button(MouseButton::Primary, true, None));
    input.push_event(pointer_button(
        MouseButton::Secondary,
        false,
        Some(Point::new(4.0, 4.0)),
    ));

    let (ordinal, _) = harness.run_frame(|ui| ui.last_root_primary_press_ordinal());
    assert_eq!(ordinal, Some(3));
}

#[test]
fn spatial_layer_and_retained_capture_never_change_cached_root_provenance() {
    let mut harness = UiTestHarness::new();
    let captured = WidgetId::from_key("captured");
    harness.memory_mut().capture_pointer(captured);
    harness.input_mut().push_event(pointer_button(
        MouseButton::Primary,
        true,
        Some(Point::new(50.0, 50.0)),
    ));

    let ((root, layered, transformed, clipped), _) = harness.run_frame(|ui| {
        ui.register_id(captured);
        let root = ui.last_root_primary_press_ordinal();
        ui.push_primitive(Primitive::LayerBegin {
            id: LayerId::from_raw(701),
        });
        let layered = ui.last_root_primary_press_ordinal();
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 2.0,
        ))));
        let transformed = ui.last_root_primary_press_ordinal();
        ui.push_primitive(Primitive::ClipBegin {
            id: ClipId::from_raw(700),
            rect: Rect::new(0.0, 0.0, 5.0, 5.0),
        });
        assert!(ui.input().events.is_empty());
        let clipped = ui.last_root_primary_press_ordinal();
        ui.push_primitive(Primitive::ClipEnd {
            id: ClipId::from_raw(700),
        });
        ui.push_primitive(Primitive::TransformEnd);
        ui.push_primitive(Primitive::LayerEnd {
            id: LayerId::from_raw(701),
        });
        (root, layered, transformed, clipped)
    });

    assert_eq!(
        (root, layered, transformed, clipped),
        (Some(0), Some(0), Some(0), Some(0))
    );
}

#[test]
fn conflicted_projection_does_not_rewrite_root_press_provenance() {
    let mut harness = UiTestHarness::new();
    let input = harness.input_mut();
    input.push_event(pointer_button(
        MouseButton::Primary,
        true,
        Some(Point::new(5.0, 5.0)),
    ));
    input.pointer.position = Some(Point::new(9.0, 9.0));
    assert_eq!(
        input.validate_event_stream(),
        Err(InputStreamConflict::Pointer)
    );

    let (ordinal, frame) = harness.run_frame(|ui| ui.last_root_primary_press_ordinal());

    assert_eq!(ordinal, Some(0));
    assert_eq!(
        frame.warnings,
        vec![FrameWarning::InputStreamConflict {
            conflict: InputStreamConflict::Pointer,
        }]
    );
}
