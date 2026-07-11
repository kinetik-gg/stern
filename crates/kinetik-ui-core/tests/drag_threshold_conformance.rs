//! Ordered drag-threshold and captured-selection gesture conformance.

use kinetik_ui_core::{
    ClipId, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point, Primitive, Rect,
    SelectionGesturePhase, TextInputEvent, UiInputEvent, UiTestHarness, Vec2, draggable,
    drop_target, pressable,
};

const FULL: Rect = Rect::new(0.0, 0.0, 160.0, 80.0);

fn run_drag(harness: &mut UiTestHarness) -> kinetik_ui_core::Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, FULL, input, memory, false)
        })
        .0
}

fn run_press(harness: &mut UiTestHarness) -> kinetik_ui_core::Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("press");
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, FULL, input, memory, false)
        })
        .0
}

#[test]
fn below_threshold_release_clicks_and_exact_threshold_suppresses_click() {
    let mut below = UiTestHarness::new();
    below.set_pointer_position(Point::new(10.0, 10.0));
    below.pointer_press(MouseButton::Primary);
    let pressed = run_drag(&mut below);
    below.set_pointer_position(Point::new(13.0, 10.0));
    let moved = run_drag(&mut below);
    assert!(!moved.dragged);
    assert_eq!(below.memory().drag_source(), None);
    below.pointer_release(MouseButton::Primary);
    let released = run_drag(&mut below);
    assert!(released.clicked);
    assert!(!released.double_clicked);
    assert_eq!(below.memory().released_drag_source(), None);
    assert_eq!(below.memory().pointer_capture(), None);

    let mut exact = UiTestHarness::new();
    exact.set_pointer_position(Point::new(10.0, 10.0));
    exact.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut exact);
    exact.set_pointer_position(Point::new(14.0, 10.0));
    let crossing = run_drag(&mut exact);
    assert!(crossing.dragged);
    assert_eq!(crossing.drag_delta, Vec2::new(4.0, 0.0));
    assert_eq!(exact.memory().drag_source(), Some(pressed.id));
    exact.pointer_release(MouseButton::Primary);
    let released = run_drag(&mut exact);
    assert!(!released.clicked);
    assert_eq!(exact.memory().released_drag_source(), Some(released.id));
}

#[test]
fn crossing_reports_full_displacement_then_subsequent_delta_and_never_unlatches() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut harness);

    harness.set_pointer_position(Point::new(12.0, 10.0));
    assert!(!run_drag(&mut harness).dragged);

    harness.set_pointer_position(Point::new(14.0, 10.0));
    let crossing = run_drag(&mut harness);
    assert!(crossing.dragged);
    assert_eq!(crossing.drag_delta, Vec2::new(4.0, 0.0));

    harness.set_pointer_position(Point::new(17.0, 10.0));
    let later = run_drag(&mut harness);
    assert!(later.dragged);
    assert_eq!(later.drag_delta, Vec2::new(3.0, 0.0));

    harness.set_pointer_position(Point::new(11.0, 10.0));
    let moved_back = run_drag(&mut harness);
    assert!(moved_back.dragged);
    assert_eq!(moved_back.drag_delta, Vec2::new(-6.0, 0.0));

    harness.pointer_release(MouseButton::Primary);
    let released = run_drag(&mut harness);
    assert!(!released.clicked);
    assert_eq!(harness.memory().released_drag_source(), Some(released.id));
}

#[test]
fn same_frame_crossing_release_reports_inside_motion_but_outside_only_cleans_up() {
    let mut inside = UiTestHarness::new();
    inside.set_pointer_position(Point::new(10.0, 10.0));
    inside.pointer_press(MouseButton::Primary);
    inside.set_pointer_position(Point::new(14.0, 10.0));
    inside.pointer_release(MouseButton::Primary);
    let response = run_drag(&mut inside);
    assert!(response.dragged);
    assert_eq!(response.drag_delta, Vec2::new(4.0, 0.0));
    assert!(!response.clicked);
    assert_eq!(inside.memory().released_drag_source(), Some(response.id));

    let mut outside = UiTestHarness::new();
    outside.set_pointer_position(Point::new(10.0, 10.0));
    outside.pointer_press(MouseButton::Primary);
    outside.set_pointer_position(Point::new(200.0, 10.0));
    outside.pointer_release(MouseButton::Primary);
    let response = run_drag(&mut outside);
    assert!(!response.dragged);
    assert_eq!(response.drag_delta, Vec2::ZERO);
    assert!(!response.clicked);
    assert_eq!(outside.memory().pointer_capture(), None);
    assert_eq!(outside.memory().released_drag_source(), Some(response.id));
}

#[test]
fn release_event_position_can_cross_threshold_without_a_move_event() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(14.0, 10.0)),
    });

    let response = run_drag(&mut harness);
    assert!(response.dragged);
    assert_eq!(response.drag_delta, Vec2::new(4.0, 0.0));
    assert!(!response.clicked);
    assert_eq!(harness.memory().released_drag_source(), Some(response.id));
}

#[test]
fn pressable_suppresses_threshold_release_without_becoming_a_drag_source() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_press(&mut harness);
    harness.set_pointer_position(Point::new(14.0, 10.0));
    let moved = run_press(&mut harness);
    assert!(!moved.dragged);
    assert_eq!(harness.memory().drag_source(), None);
    harness.pointer_release(MouseButton::Primary);
    let released = run_press(&mut harness);
    assert!(!released.clicked);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn below_threshold_canonical_double_click_preserves_live_click_count() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    let _ = run_press(&mut harness);
    harness.set_pointer_position(Point::new(12.0, 10.0));
    let _ = run_press(&mut harness);
    harness.pointer_release(MouseButton::Primary);
    harness.set_click_count(2);
    let response = run_press(&mut harness);
    assert!(response.clicked);
    assert!(response.double_clicked);
}

#[test]
fn captured_selection_preserves_root_ordinals_without_domain_drag_or_replay() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit("x".to_owned())));
    harness.set_pointer_position(Point::new(18.0, 10.0));
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    harness.pointer_release(MouseButton::Primary);

    let ((first, second), _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        let first = ui.captured_selection_gesture(id, FULL, false);
        let second = ui.captured_selection_gesture(id, FULL, false);
        (first, second)
    });

    assert_eq!(
        first
            .actions
            .iter()
            .map(|action| (action.ordinal, action.phase))
            .collect::<Vec<_>>(),
        vec![
            (Some(1), SelectionGesturePhase::Press),
            (Some(3), SelectionGesturePhase::Move),
            (Some(5), SelectionGesturePhase::Release),
        ]
    );
    assert_eq!(first.actions[1].delta, Vec2::new(8.0, 0.0));
    assert!(second.actions.is_empty());
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn spatial_filtering_keeps_original_action_ordinals_with_gaps() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(200.0, 10.0));
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit(
            "before".to_owned(),
        )));
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let clip = ClipId::from_raw(91);
    let (gesture, _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: FULL,
        });
        let gesture = ui.captured_selection_gesture(id, FULL, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        gesture
    });

    assert_eq!(
        gesture
            .actions
            .iter()
            .map(|action| action.ordinal)
            .collect::<Vec<_>>(),
        vec![Some(3), Some(5), Some(6)]
    );
    assert!(gesture.response.clicked);
}

#[test]
fn release_all_emits_one_original_ordinal_cancel_and_clears_selection_capture() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(
        harness.memory().pointer_capture(),
        Some(pressed.response.id)
    );

    harness.input_mut().release_pointer_buttons();
    let cancelled = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, Some(0));
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
}

#[test]
fn drop_is_ineligible_below_threshold_and_order_independent_after_crossing() {
    let mut below = UiTestHarness::new();
    below.set_pointer_position(Point::new(10.0, 10.0));
    below.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut below);
    below.set_pointer_position(Point::new(13.0, 10.0));
    let _ = run_drag(&mut below);
    below.pointer_release(MouseButton::Primary);
    let ((drop, source), _) = below.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (drop, source)
    });
    assert_eq!(drop.source, None);
    assert!(!drop.dropped);
    assert!(source.clicked);

    let mut crossed = UiTestHarness::new();
    crossed.set_pointer_position(Point::new(10.0, 10.0));
    crossed.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut crossed);
    crossed.set_pointer_position(Point::new(14.0, 10.0));
    let _ = run_drag(&mut crossed);
    crossed.pointer_release(MouseButton::Primary);
    let ((drop, source), _) = crossed.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (drop, source)
    });
    assert_eq!(drop.source, Some(source.id));
    assert!(drop.dropped);
    assert!(!source.clicked);
}

#[test]
fn conflicted_release_cleans_existing_capture_without_click_drag_or_drop() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = run_drag(&mut harness);
    assert_eq!(harness.memory().pointer_capture(), Some(pressed.id));

    harness.pointer_release(MouseButton::Primary);
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);
    let ((response, drop), output) = harness.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let (input, memory) = ui.input_and_memory_mut();
        let response = draggable(source, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        (response, drop)
    });

    assert!(!response.clicked);
    assert!(!response.dragged);
    assert_eq!(drop.source, None);
    assert!(!drop.dropped);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(output.warnings.len(), 1);
}

#[test]
fn plain_pointer_capture_release_cleans_without_synthesizing_a_click() {
    let mut harness = UiTestHarness::new();
    let owner = kinetik_ui_core::WidgetId::from_key("plain-capture");
    harness.memory_mut().capture_pointer(owner);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let response = harness
        .run_frame(|ui| {
            ui.register_id(owner);
            let (input, memory) = ui.input_and_memory_mut();
            pressable(owner, FULL, input, memory, false)
        })
        .0;
    assert!(!response.clicked);
    assert_eq!(harness.memory().pointer_capture(), None);
}
