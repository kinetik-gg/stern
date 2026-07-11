//! Conformance coverage for typed wheel normalization and compatibility input.

use kinetik_ui_core::{
    InputWheelDelta, Point, Primitive, Rect, ScriptedInput, Size, Transform, UiInput, UiInputEvent,
    UiMemory, UiTestHarness, Vec2, WidgetId, scrollable,
};

const VIEWPORT: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);
const CONTENT: Size = Size::new(500.0, 500.0);

fn scroll_once(input: &UiInput, initial: Vec2) -> kinetik_ui_core::ScrollResponse {
    let id = WidgetId::from_key("wheel-target");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(id, initial);
    scrollable(id, VIEWPORT, CONTENT, input, &mut memory, false)
}

fn wheel_input(events: impl IntoIterator<Item = InputWheelDelta>) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.position = Some(Point::new(20.0, 20.0));
    for delta in events {
        input.push_event(UiInputEvent::Wheel {
            delta,
            position: input.pointer.position,
        });
    }
    input
}

#[test]
fn canonical_line_and_pixel_wheels_convert_independently_and_invert_once() {
    let input = wheel_input([
        InputWheelDelta::Lines(Vec2::new(1.0, -0.5)),
        InputWheelDelta::Pixels(Vec2::new(2.0, -3.0)),
    ]);

    let response = scroll_once(&input, Vec2::new(200.0, 200.0));

    assert_eq!(response.delta, Vec2::new(-42.0, 23.0));
    assert_eq!(response.offset, Vec2::new(158.0, 223.0));
}

#[test]
fn canonical_wheels_override_compatibility_snapshot_and_nonwheel_stream_is_zero() {
    let mut input = wheel_input([InputWheelDelta::Lines(Vec2::new(0.0, -1.0))]);
    input.pointer.wheel_delta = Vec2::new(900.0, 900.0);
    let response = scroll_once(&input, Vec2::new(100.0, 100.0));
    assert_eq!(response.delta, Vec2::new(0.0, 40.0));

    let mut nonwheel = UiInput::default();
    nonwheel.pointer.position = Some(Point::new(20.0, 20.0));
    nonwheel.push_event(UiInputEvent::PointerMoved {
        position: Point::new(20.0, 20.0),
        delta: Vec2::ZERO,
    });
    nonwheel.pointer.wheel_delta = Vec2::new(0.0, -80.0);
    assert_eq!(
        scroll_once(&nonwheel, Vec2::new(100.0, 100.0)).delta,
        Vec2::ZERO
    );
}

#[test]
fn canonical_empty_fallback_preserves_legacy_logical_magnitude() {
    let mut input = UiInput::default();
    input.pointer.position = Some(Point::new(20.0, 20.0));
    input.pointer.wheel_delta = Vec2::new(3.0, -7.0);

    assert_eq!(
        scroll_once(&input, Vec2::new(100.0, 100.0)).delta,
        Vec2::new(-3.0, 7.0)
    );
}

#[test]
fn wheel_normalization_sanitizes_components_products_and_accumulation() {
    let input = wheel_input([
        InputWheelDelta::Lines(Vec2::new(f32::NAN, f32::INFINITY)),
        InputWheelDelta::Pixels(Vec2::new(-4.0, 6.0)),
        InputWheelDelta::Pixels(Vec2::new(f32::MAX, 0.0)),
        InputWheelDelta::Pixels(Vec2::new(f32::MAX, 0.0)),
    ]);

    let response = scroll_once(&input, Vec2::new(100.0, 100.0));

    assert!(response.delta.x.is_finite());
    assert!(response.delta.y.is_finite());
    assert_eq!(response.delta, Vec2::new(0.0, -6.0));
}

#[test]
fn nested_nonuniform_scope_transforms_pixels_once_but_keeps_lines_invariant() {
    let input = wheel_input([
        InputWheelDelta::Lines(Vec2::new(-1.0, 0.0)),
        InputWheelDelta::Pixels(Vec2::new(-8.0, 0.0)),
    ]);
    let mut harness = UiTestHarness::new();
    *harness.input_mut() = input;

    let (response, _) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 4.0,
        ))));
        let id = WidgetId::from_key("scoped-wheel");
        ui.memory_mut()
            .set_scroll_offset(id, Vec2::new(100.0, 100.0));
        let response = {
            let (input, memory) = ui.input_and_memory_mut();
            scrollable(id, VIEWPORT, CONTENT, input, memory, false)
        };
        ui.push_primitive(Primitive::TransformEnd);
        response
    });

    assert_eq!(response.delta, Vec2::new(44.0, 0.0));
}

#[test]
fn harness_legacy_wheel_aliases_preserve_pixel_magnitude() {
    let mut direct = UiTestHarness::new();
    direct.set_pointer_position(Point::new(20.0, 20.0));
    direct.wheel(Vec2::new(2.0, -3.0));
    assert!(matches!(
        direct.input().events.last(),
        Some(UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(Vec2 { x: 2.0, y: -3.0 }),
            ..
        })
    ));

    let mut scripted = UiTestHarness::new();
    scripted.apply_scripted_input(ScriptedInput::Wheel(Vec2::new(4.0, -5.0)));
    assert!(matches!(
        scripted.input().events.last(),
        Some(UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(Vec2 { x: 4.0, y: -5.0 }),
            ..
        })
    ));
}
