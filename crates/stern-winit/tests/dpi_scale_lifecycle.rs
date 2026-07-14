//! Winit-to-core scale lifecycle evidence for issue #651.

use std::time::Instant;

use stern_core::{
    CapturedDomainDragGesture, DomainDragGesturePhase, Point, Primitive, Rect, Response,
    ScaleFactor, Transform, Ui, UiInputEvent, UiTestHarness, Vec2, ViewportInfo, WidgetId,
    focusable,
};
use stern_winit::{WinitInputAdapter, viewport_from_winit};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
};

const OWNER: WidgetId = WidgetId::from_raw(0xd91);
const LOCAL_BOUNDS: Rect = Rect::new(0.0, 0.0, 40.0, 20.0);

fn assert_point_close(actual: Point, expected: Point) {
    assert!((actual.x - expected.x).abs() < 1.0e-4, "x: {actual:?}");
    assert!((actual.y - expected.y).abs() < 1.0e-4, "y: {actual:?}");
}

fn push_nested_scope(ui: &mut Ui<'_>) {
    ui.push_primitive(Primitive::TransformBegin(Transform::translation(
        Vec2::new(10.0, 20.0),
    )));
    ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
        2.0, 4.0,
    ))));
}

fn pop_nested_scope(ui: &mut Ui<'_>) {
    ui.push_primitive(Primitive::TransformEnd);
    ui.push_primitive(Primitive::TransformEnd);
}

fn nested_focusable(ui: &mut Ui<'_>) -> Response {
    ui.register_id(OWNER);
    push_nested_scope(ui);
    let response = {
        let (input, memory) = ui.input_and_memory_mut();
        focusable(OWNER, LOCAL_BOUNDS, input, memory, false)
    };
    pop_nested_scope(ui);
    response
}

fn nested_domain_drag(ui: &mut Ui<'_>) -> CapturedDomainDragGesture {
    ui.register_id(OWNER);
    push_nested_scope(ui);
    let gesture = ui.captured_domain_drag_gesture(OWNER, LOCAL_BOUNDS, false);
    pop_nested_scope(ui);
    gesture
}

fn prepare_frame(harness: &mut UiTestHarness, adapter: &WinitInputAdapter, viewport: ViewportInfo) {
    harness.set_viewport(viewport.logical_size, viewport.scale_factor);
    *harness.input_mut() = adapter.input().clone();
}

fn assert_retained_owner(harness: &UiTestHarness) {
    assert_eq!(harness.memory().focused(), Some(OWNER));
    assert_eq!(harness.memory().pointer_capture(), Some(OWNER));
    assert_eq!(harness.memory().drag_source(), Some(OWNER));
}

fn establish_nested_drag(
    adapter: &mut WinitInputAdapter,
    harness: &mut UiTestHarness,
    viewport: ViewportInfo,
) {
    adapter.set_window_focused(true);
    adapter.pointer_moved(PhysicalPosition::new(24.0, 32.0));
    adapter.mouse_button(MouseButton::Left, ElementState::Pressed, 1);
    adapter.mouse_button(MouseButton::Left, ElementState::Released, 1);
    prepare_frame(harness, adapter, viewport);
    let (focus, output) = harness.run_frame(nested_focusable);
    assert!(focus.clicked);
    assert_eq!(harness.memory().focused(), Some(OWNER));
    assert!(output.warnings.is_empty());

    adapter.begin_frame();
    adapter.mouse_button(MouseButton::Left, ElementState::Pressed, 1);
    prepare_frame(harness, adapter, viewport);
    let (pressed, output) = harness.run_frame(nested_domain_drag);
    assert_eq!(pressed.actions[0].phase, DomainDragGesturePhase::Press);
    assert_eq!(harness.memory().pointer_capture(), Some(OWNER));
    assert_eq!(harness.memory().focused(), Some(OWNER));
    assert!(output.warnings.is_empty());

    adapter.begin_frame();
    adapter.pointer_moved(PhysicalPosition::new(32.0, 32.0));
    prepare_frame(harness, adapter, viewport);
    let (crossed, output) = harness.run_frame(nested_domain_drag);
    assert!(crossed.response.dragged);
    assert_eq!(crossed.response.drag_delta, Vec2::new(5.0, 0.0));
    assert_retained_owner(harness);
    assert!(output.warnings.is_empty());
}

#[test]
fn scale_changes_preserve_retained_nested_drag_ownership_across_release_scales() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::new(0.8));
    let mut harness = UiTestHarness::new();
    let initial_viewport = viewport_from_winit(PhysicalSize::new(640, 480), 0.8);
    establish_nested_drag(&mut adapter, &mut harness, initial_viewport);

    let mut screen_position = Point::new(40.0, 40.0);

    for (scale, physical_size) in [
        (1.0, PhysicalSize::new(800, 600)),
        (1.25, PhysicalSize::new(1000, 750)),
        (1.5, PhysicalSize::new(1200, 900)),
        (2.0, PhysicalSize::new(1600, 1200)),
    ] {
        adapter.begin_frame();
        adapter.set_scale_factor(ScaleFactor::new(scale));

        assert_eq!(adapter.input().pointer.position, None);
        assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
        assert!(adapter.input().pointer.primary.down);
        assert!(matches!(
            adapter.input().events.last(),
            Some(UiInputEvent::PointerLeft)
        ));

        let viewport = viewport_from_winit(physical_size, scale);
        assert_eq!(viewport.logical_size, stern_core::Size::new(800.0, 600.0));
        adapter.pointer_moved(PhysicalPosition::new(
            f64::from(screen_position.x) * scale,
            f64::from(screen_position.y) * scale,
        ));
        assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);

        prepare_frame(&mut harness, &adapter, viewport);
        let (rebased, output) = harness.run_frame(nested_domain_drag);
        let rebased_move = rebased
            .actions
            .iter()
            .find(|action| action.phase == DomainDragGesturePhase::Move)
            .expect("rebase move reaches retained drag owner");
        assert_point_close(
            rebased_move.position.expect("rebase position"),
            Point::new(
                (screen_position.x - 10.0) / 2.0,
                (screen_position.y - 20.0) / 4.0,
            ),
        );
        assert_eq!(rebased_move.delta, Vec2::ZERO);
        assert_retained_owner(&harness);
        assert!(output.warnings.is_empty());

        adapter.begin_frame();
        screen_position.x += 2.0;
        screen_position.y += 4.0;
        adapter.pointer_moved(PhysicalPosition::new(
            f64::from(screen_position.x) * scale,
            f64::from(screen_position.y) * scale,
        ));
        assert_eq!(adapter.input().pointer.delta, Vec2::new(2.0, 4.0));

        prepare_frame(&mut harness, &adapter, viewport);
        let (moved, output) = harness.run_frame(nested_domain_drag);
        let localized_move = moved
            .actions
            .iter()
            .find(|action| action.phase == DomainDragGesturePhase::Move)
            .expect("nonzero move reaches retained drag owner");
        assert_point_close(
            localized_move.position.expect("localized position"),
            Point::new(
                (screen_position.x - 10.0) / 2.0,
                (screen_position.y - 20.0) / 4.0,
            ),
        );
        assert_eq!(localized_move.delta, Vec2::new(1.0, 1.0));
        assert_retained_owner(&harness);
        assert!(output.warnings.is_empty());
    }

    adapter.begin_frame();
    adapter.set_scale_factor(ScaleFactor::new(2.0));
    assert!(adapter.input().events.is_empty());
    assert_eq!(adapter.input().pointer.position, Some(screen_position));

    adapter.set_scale_factor(ScaleFactor::new(f64::NAN));
    assert!(matches!(
        adapter.input().events.last(),
        Some(UiInputEvent::PointerLeft)
    ));
    assert_eq!(adapter.input().pointer.position, None);
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);

    adapter.begin_frame();
    adapter.set_scale_factor(ScaleFactor::new(f64::NAN));
    assert!(adapter.input().events.is_empty());
    assert_eq!(harness.memory().focused(), Some(OWNER));
    assert_eq!(harness.memory().pointer_capture(), Some(OWNER));
    assert_eq!(harness.memory().drag_source(), Some(OWNER));
}

#[test]
fn first_move_after_scale_change_starts_a_fresh_click_coordinate_basis() {
    let started = Instant::now();
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    adapter.mouse_button_at(MouseButton::Left, ElementState::Pressed, started);
    adapter.mouse_button_at(MouseButton::Left, ElementState::Released, started);

    adapter.begin_frame();
    adapter.set_scale_factor(ScaleFactor::new(1.25));
    adapter.pointer_moved(PhysicalPosition::new(12.5, 12.5));
    adapter.mouse_button_at(MouseButton::Left, ElementState::Pressed, started);

    assert_eq!(
        adapter.input().pointer.position,
        Some(Point::new(10.0, 10.0))
    );
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
    assert!(matches!(
        adapter.input().events.first(),
        Some(UiInputEvent::PointerLeft)
    ));
    assert!(matches!(
        adapter.input().events.last(),
        Some(UiInputEvent::PointerButton { click_count: 1, .. })
    ));
}
