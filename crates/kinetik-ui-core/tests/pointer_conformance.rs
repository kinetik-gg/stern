//! Windowless pointer interaction conformance coverage.

use std::time::Duration;

use kinetik_ui_core::{
    CursorShape, Key, Modifiers, MouseButton, PlatformRequest, Point, Rect, Response,
    ScrollResponse, Size, Transform, Ui, UiTestHarness, Vec2, WidgetId, context_menu_trigger,
    context_menu_trigger_transformed, draggable, draggable_transformed, drop_target,
    drop_target_transformed, focusable_transformed, pressable, pressable_transformed, scrollable,
    scrollable_transformed, selectable_transformed, tooltip_trigger, tooltip_trigger_transformed,
};

fn rect() -> Rect {
    Rect::new(0.0, 0.0, 100.0, 40.0)
}

fn source_rect() -> Rect {
    Rect::new(0.0, 0.0, 40.0, 40.0)
}

fn target_rect() -> Rect {
    Rect::new(80.0, 0.0, 40.0, 40.0)
}

fn local_target_rect() -> Rect {
    Rect::new(0.0, 0.0, 40.0, 40.0)
}

fn translated(offset_x: f32, offset_y: f32) -> Transform {
    Transform::translation(Vec2::new(offset_x, offset_y))
}

fn press_transform() -> Transform {
    translated(100.0, 50.0)
}

fn transformed_source_transform() -> Transform {
    translated(100.0, 0.0)
}

fn transformed_target_transform() -> Transform {
    translated(200.0, 0.0)
}

fn pressable_response(harness: &mut UiTestHarness, disabled: bool) -> Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, rect(), input, memory, disabled)
        })
        .0
}

fn context_menu_response(harness: &mut UiTestHarness, disabled: bool) -> Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("menu");
            let (input, memory) = ui.input_and_memory_mut();
            context_menu_trigger(id, rect(), input, memory, disabled)
        })
        .0
}

fn tooltip_response(harness: &mut UiTestHarness, disabled: bool) -> Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("tooltip");
            let (input, memory) = ui.input_and_memory_mut();
            tooltip_trigger(id, rect(), input, memory, disabled)
        })
        .0
}

fn scroll_response(harness: &mut UiTestHarness, disabled: bool) -> ScrollResponse {
    harness
        .run_frame(|ui| {
            let id = ui.id("scroll");
            let (input, memory) = ui.input_and_memory_mut();
            scrollable(id, rect(), Size::new(150.0, 200.0), input, memory, disabled)
        })
        .0
}

fn source_id(ui: &mut Ui<'_>) -> WidgetId {
    ui.id("source")
}

fn target_id(ui: &mut Ui<'_>) -> WidgetId {
    ui.id("target")
}

fn start_drag_over_target(harness: &mut UiTestHarness) -> WidgetId {
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let dragged = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(source, source_rect(), input, memory, false)
        })
        .0;

    assert!(dragged.dragged);
    assert_eq!(harness.memory().drag_source(), Some(dragged.id));
    dragged.id
}

fn start_transformed_drag_over_target(harness: &mut UiTestHarness) -> WidgetId {
    press_transformed_source(harness, Point::new(110.0, 10.0));
    drag_transformed_source_to(harness, Point::new(210.0, 10.0))
}

fn press_transformed_source(harness: &mut UiTestHarness, point: Point) {
    harness.set_pointer_position(point);
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable_transformed(
            source,
            source_rect(),
            transformed_source_transform(),
            input,
            memory,
            false,
        )
    });
}

fn drag_transformed_source_to(harness: &mut UiTestHarness, point: Point) -> WidgetId {
    harness.set_pointer_position(point);
    let dragged = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable_transformed(
                source,
                source_rect(),
                transformed_source_transform(),
                input,
                memory,
                false,
            )
        })
        .0;

    assert!(dragged.dragged);
    assert_eq!(harness.memory().drag_source(), Some(dragged.id));
    dragged.id
}

#[test]
fn pointer_interaction_pressable_press_release_click_and_double_click_are_frame_driven() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = pressable_response(&mut harness, false);

    assert!(pressed.state.hovered);
    assert!(pressed.state.active);
    assert!(pressed.state.pressed);
    assert!(harness.memory().has_pointer_capture(pressed.id));

    harness.set_click_count(1);
    harness.pointer_release(MouseButton::Primary);
    let clicked = pressable_response(&mut harness, false);

    assert!(clicked.clicked);
    assert!(!clicked.double_clicked);
    assert!(!clicked.state.active);
    assert!(!clicked.state.pressed);
    assert_eq!(harness.memory().pointer_capture(), None);

    harness.advance_frame(Duration::from_millis(16));
    harness.pointer_press(MouseButton::Primary);
    let pressed_again = pressable_response(&mut harness, false);
    assert!(pressed_again.state.pressed);

    harness.set_click_count(2);
    harness.pointer_release(MouseButton::Primary);
    let double_clicked = pressable_response(&mut harness, false);

    assert!(double_clicked.clicked);
    assert!(double_clicked.double_clicked);
}

#[test]
fn pointer_interaction_pressable_release_outside_does_not_click() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = pressable_response(&mut harness, false);
    assert!(pressed.state.pressed);

    harness.set_pointer_position(Point::new(140.0, 10.0));
    harness.pointer_release(MouseButton::Primary);
    let released_outside = pressable_response(&mut harness, false);

    assert!(!released_outside.state.hovered);
    assert!(!released_outside.clicked);
    assert!(!released_outside.double_clicked);
    assert_eq!(harness.memory().active(), None);
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn pointer_interaction_transformed_pressable_clicks_hits_and_rejects_misses() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(110.0, 60.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;

    assert!(pressed.state.hovered);
    assert!(pressed.state.pressed);
    assert!(harness.memory().has_pointer_capture(pressed.id));

    harness.pointer_release(MouseButton::Primary);
    let clicked = harness
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;

    assert!(clicked.clicked);
    assert_eq!(harness.memory().pointer_capture(), None);

    let mut missed = UiTestHarness::new();
    missed.set_pointer_position(Point::new(90.0, 60.0));
    missed.pointer_press(MouseButton::Primary);
    let miss = missed
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;

    assert!(!miss.state.hovered);
    assert!(!miss.state.pressed);
    assert_eq!(missed.memory().pointer_capture(), None);
}

#[test]
fn pointer_interaction_transformed_pressable_release_outside_does_not_click() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(110.0, 60.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;
    assert!(pressed.state.pressed);

    harness.set_pointer_position(Point::new(205.0, 60.0));
    harness.pointer_release(MouseButton::Primary);
    let released_outside = harness
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;

    assert!(!released_outside.state.hovered);
    assert!(!released_outside.clicked);
    assert!(!released_outside.double_clicked);
    assert_eq!(harness.memory().active(), None);
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn pointer_interaction_context_menu_secondary_click_requires_matching_press_owner() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Secondary);
    let pressed = context_menu_response(&mut harness, false);
    assert!(!pressed.context_requested);

    harness.pointer_release(MouseButton::Secondary);
    let clicked = context_menu_response(&mut harness, false);
    assert!(clicked.secondary_clicked);
    assert!(clicked.context_requested);
    assert_eq!(harness.memory().secondary_pressed(), None);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Secondary);
    let _ = harness.run_frame(|ui| {
        let first = ui.id("first");
        let (input, memory) = ui.input_and_memory_mut();
        context_menu_trigger(first, rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(210.0, 10.0));
    harness.pointer_release(MouseButton::Secondary);
    let (first, second) = harness
        .run_frame(|ui| {
            let first = ui.id("first");
            let second = ui.id("second");
            let first_rect = rect();
            let second_rect = Rect::new(200.0, 0.0, 100.0, 40.0);
            let (input, memory) = ui.input_and_memory_mut();
            (
                context_menu_trigger(first, first_rect, input, memory, false),
                context_menu_trigger(second, second_rect, input, memory, false),
            )
        })
        .0;

    assert!(!first.context_requested);
    assert!(!second.secondary_clicked);
    assert!(!second.context_requested);
}

#[test]
fn pointer_interaction_draggable_starts_updates_and_ends_with_capture_outside_rect() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let id = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, source_rect(), input, memory, false)
        })
        .0;

    assert!(pressed.state.pressed);
    assert!(!pressed.dragged);
    assert!(harness.memory().has_pointer_capture(pressed.id));
    assert_eq!(harness.memory().drag_source(), None);

    harness.set_pointer_position(Point::new(70.0, 10.0));
    let dragged = harness
        .run_frame(|ui| {
            let id = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, source_rect(), input, memory, false)
        })
        .0;

    assert!(!dragged.state.hovered);
    assert!(dragged.state.active);
    assert!(dragged.dragged);
    assert_eq!(dragged.drag_delta, Vec2::new(60.0, 0.0));
    assert_eq!(harness.memory().drag_source(), Some(dragged.id));
    assert_eq!(harness.memory().released_drag_source(), None);

    harness.pointer_release(MouseButton::Primary);
    let released = harness
        .run_frame(|ui| {
            let id = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, source_rect(), input, memory, false)
        })
        .0;

    assert!(!released.clicked);
    assert!(!released.state.active);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), Some(released.id));
}

#[test]
fn pointer_interaction_drag_capture_suppresses_other_hover() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let id = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(id, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let (source, other) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let other = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable(source, source_rect(), input, memory, false),
                pressable(other, target_rect(), input, memory, false),
            )
        })
        .0;

    assert!(source.dragged);
    assert!(!other.state.hovered);
    assert_eq!(harness.memory().hovered(), None);
    assert_eq!(harness.memory().pointer_capture(), Some(source.id));
}

#[test]
fn pointer_interaction_transformed_drag_capture_uses_screen_delta_and_suppresses_other() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(110.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable_transformed(
                source,
                source_rect(),
                transformed_source_transform(),
                input,
                memory,
                false,
            )
        })
        .0;

    assert!(pressed.state.pressed);
    assert!(harness.memory().has_pointer_capture(pressed.id));

    harness.set_pointer_position(Point::new(210.0, 10.0));
    let (source, other) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let other = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
                pressable_transformed(
                    other,
                    local_target_rect(),
                    transformed_target_transform(),
                    input,
                    memory,
                    false,
                ),
            )
        })
        .0;

    assert!(source.dragged);
    assert_eq!(source.drag_delta, Vec2::new(100.0, 0.0));
    assert_eq!(harness.memory().drag_source(), Some(source.id));
    assert!(!other.state.hovered);
    assert!(!other.clicked);
    assert_eq!(harness.memory().hovered(), None);
    assert_eq!(harness.memory().pointer_capture(), Some(source.id));
}

#[test]
fn pointer_interaction_capture_blocks_other_active_click_and_cursor_stealing() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let id = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(id, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let ((source, other, source_cursor, other_cursor), drag_output) = harness.run_frame(|ui| {
        let source = source_id(ui);
        let other = target_id(ui);
        let (source_response, other_response) = {
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable(source, source_rect(), input, memory, false),
                pressable(other, target_rect(), input, memory, false),
            )
        };
        let source_cursor = ui.request_cursor_for(source, CursorShape::Grabbing);
        let other_cursor = ui.request_cursor_for(other, CursorShape::PointingHand);
        (source_response, other_response, source_cursor, other_cursor)
    });

    assert!(source.dragged);
    assert!(source.state.active);
    assert!(!source.state.hovered);
    assert!(!other.state.hovered);
    assert!(!other.state.active);
    assert!(!other.state.pressed);
    assert!(!other.clicked);
    assert!(source_cursor);
    assert!(!other_cursor);
    assert_eq!(
        drag_output.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::Grabbing)]
    );

    harness.pointer_release(MouseButton::Primary);
    let ((source, other, source_cursor, other_cursor), release_output) = harness.run_frame(|ui| {
        let source = source_id(ui);
        let other = target_id(ui);
        let (source_response, other_response) = {
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable(source, source_rect(), input, memory, false),
                pressable(other, target_rect(), input, memory, false),
            )
        };
        let source_cursor = ui.request_cursor_for(source, CursorShape::Grabbing);
        let other_cursor = ui.request_cursor_for(other, CursorShape::PointingHand);
        (source_response, other_response, source_cursor, other_cursor)
    });

    assert!(!source.clicked);
    assert!(!source.state.active);
    assert!(!other.state.hovered);
    assert!(!other.state.active);
    assert!(!other.state.pressed);
    assert!(!other.clicked);
    assert!(!source_cursor);
    assert!(!other_cursor);
    assert!(release_output.platform_requests.is_empty());
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().active(), None);
}

#[test]
fn pointer_interaction_focus_loss_cancels_capture_without_synthesizing_drop() {
    let mut harness = UiTestHarness::new();
    let retained_focus = WidgetId::from_key("retained-focus");
    harness.memory_mut().focus(retained_focus);
    harness.memory_mut().set_text_input_owner(retained_focus);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_window_focused(false);
    let (drop, source) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                drop_target(target, target_rect(), input, memory, false),
                draggable(source, source_rect(), input, memory, false),
            )
        })
        .0;

    assert!(!drop.dropped);
    assert!(!drop.response.state.hovered);
    assert!(!source.clicked);
    assert!(!source.state.active);
    assert!(!source.dragged);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(harness.memory().focused(), Some(retained_focus));
    assert_eq!(harness.memory().text_input_owner(), Some(retained_focus));
}

#[test]
fn pointer_interaction_focus_loss_clears_capture_without_participating_primitive() {
    let mut harness = UiTestHarness::new();
    let retained_focus = WidgetId::from_key("retained-focus");
    harness.memory_mut().focus(retained_focus);
    harness.memory_mut().set_text_input_owner(retained_focus);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_window_focused(false);
    let frame_start_memory = harness
        .run_frame(|ui| {
            (
                ui.memory().pointer_capture(),
                ui.memory().drag_source(),
                ui.memory().released_drag_source(),
                ui.memory().focused(),
                ui.memory().text_input_owner(),
                ui.memory().pointer_interaction_cancelled(),
            )
        })
        .0;

    assert_eq!(frame_start_memory.0, None);
    assert_eq!(frame_start_memory.1, None);
    assert_eq!(frame_start_memory.2, None);
    assert_eq!(frame_start_memory.3, Some(retained_focus));
    assert_eq!(frame_start_memory.4, Some(retained_focus));
    assert!(frame_start_memory.5);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(harness.memory().focused(), Some(retained_focus));
    assert_eq!(harness.memory().text_input_owner(), Some(retained_focus));
}

#[test]
fn pointer_interaction_release_all_clears_capture_without_participating_primitive() {
    let mut harness = UiTestHarness::new();
    let retained_focus = WidgetId::from_key("retained-focus");
    harness.memory_mut().focus(retained_focus);
    harness.memory_mut().set_text_input_owner(retained_focus);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.input_mut().release_pointer_buttons();
    let frame_start_memory = harness
        .run_frame(|ui| {
            (
                ui.input().window_focused,
                ui.memory().pointer_capture(),
                ui.memory().drag_source(),
                ui.memory().released_drag_source(),
                ui.memory().focused(),
                ui.memory().text_input_owner(),
                ui.memory().pointer_interaction_cancelled(),
            )
        })
        .0;

    assert!(frame_start_memory.0);
    assert_eq!(frame_start_memory.1, None);
    assert_eq!(frame_start_memory.2, None);
    assert_eq!(frame_start_memory.3, None);
    assert_eq!(frame_start_memory.4, Some(retained_focus));
    assert_eq!(frame_start_memory.5, Some(retained_focus));
    assert!(frame_start_memory.6);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(harness.memory().focused(), Some(retained_focus));
    assert_eq!(harness.memory().text_input_owner(), Some(retained_focus));
}

#[test]
fn pointer_interaction_release_all_cancels_disabled_owner_without_drop_or_cursor() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.input_mut().release_pointer_buttons();
    let ((source, target, source_cursor), output) = harness.run_frame(|ui| {
        let source = source_id(ui);
        let target = target_id(ui);
        let (source_response, target_response) = {
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable(source, source_rect(), input, memory, true),
                drop_target(target, target_rect(), input, memory, false),
            )
        };
        let source_cursor = ui.request_cursor_for(source, CursorShape::Grabbing);
        (source_response, target_response, source_cursor)
    });

    assert!(source.state.disabled);
    assert!(!source.state.active);
    assert!(!source.dragged);
    assert!(!source.clicked);
    assert!(!target.response.state.hovered);
    assert_eq!(target.source, None);
    assert!(!target.dropped);
    assert!(!source_cursor);
    assert!(output.platform_requests.is_empty());
    assert!(harness.memory().pointer_interaction_cancelled());
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn pointer_interaction_cancellation_flag_and_cursor_suppression_are_frame_local() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.input_mut().release_pointer_buttons();
    let ((cancelled, source_cursor, target_cursor), cancel_output) = harness.run_frame(|ui| {
        let source = source_id(ui);
        let target = target_id(ui);
        let _ = {
            let (input, memory) = ui.input_and_memory_mut();
            draggable(source, source_rect(), input, memory, false)
        };
        let source_cursor = ui.request_cursor_for(source, CursorShape::Grabbing);
        let _ = {
            let (input, memory) = ui.input_and_memory_mut();
            pressable(target, target_rect(), input, memory, false)
        };
        let target_cursor = ui.request_cursor_for(target, CursorShape::PointingHand);
        (
            ui.memory().pointer_interaction_cancelled(),
            source_cursor,
            target_cursor,
        )
    });

    assert!(cancelled);
    assert!(!source_cursor);
    assert!(!target_cursor);
    assert!(cancel_output.platform_requests.is_empty());

    harness.advance_frame(Duration::from_millis(16));
    let ((cancelled, target, target_cursor), normal_output) = harness.run_frame(|ui| {
        let target = target_id(ui);
        let target_response = {
            let (input, memory) = ui.input_and_memory_mut();
            pressable(target, target_rect(), input, memory, false)
        };
        let target_cursor = ui.request_cursor_for(target, CursorShape::PointingHand);
        (
            ui.memory().pointer_interaction_cancelled(),
            target_response,
            target_cursor,
        )
    });

    assert!(!cancelled);
    assert!(target.state.hovered);
    assert!(target_cursor);
    assert_eq!(
        normal_output.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
    );
}

#[test]
fn pointer_interaction_secondary_press_owner_is_cleared_by_cancellation() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Secondary);
    let pressed = context_menu_response(&mut harness, false);
    assert!(!pressed.secondary_clicked);
    assert_eq!(harness.memory().secondary_pressed(), Some(pressed.id));

    harness.set_window_focused(false);
    harness.pointer_release(MouseButton::Secondary);
    let cancelled = context_menu_response(&mut harness, false);

    assert!(!cancelled.secondary_clicked);
    assert!(!cancelled.context_requested);
    assert_eq!(harness.memory().secondary_pressed(), None);
    assert!(harness.memory().pointer_interaction_cancelled());

    harness.set_window_focused(true);
    harness.advance_frame(Duration::from_millis(16));
    let cleared = harness
        .run_frame(|ui| ui.memory().pointer_interaction_cancelled())
        .0;
    assert!(!cleared);
}

#[test]
fn pointer_interaction_drop_target_reports_active_drag_source_over_target() {
    let mut harness = UiTestHarness::new();
    let source = start_drag_over_target(&mut harness);

    let drop = harness
        .run_frame(|ui| {
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(target, target_rect(), input, memory, false)
        })
        .0;

    assert_eq!(drop.source, Some(source));
    assert!(!drop.dropped);
    assert!(drop.response.state.hovered);
    assert_eq!(harness.memory().drag_source(), Some(source));
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn pointer_interaction_drop_target_accepts_released_drag_source_over_target() {
    for source_first in [true, false] {
        let mut harness = UiTestHarness::new();
        let source = start_drag_over_target(&mut harness);

        harness.pointer_release(MouseButton::Primary);
        let (source_response, target_response) = harness
            .run_frame(|ui| {
                let source = source_id(ui);
                let target = target_id(ui);
                let (input, memory) = ui.input_and_memory_mut();

                if source_first {
                    let source_response = draggable(source, source_rect(), input, memory, false);
                    let target_response = drop_target(target, target_rect(), input, memory, false);
                    (source_response, target_response)
                } else {
                    let target_response = drop_target(target, target_rect(), input, memory, false);
                    let source_response = draggable(source, source_rect(), input, memory, false);
                    (source_response, target_response)
                }
            })
            .0;

        assert_eq!(source_response.id, source);
        assert_eq!(target_response.source, Some(source));
        assert!(target_response.dropped);
        assert!(target_response.response.state.hovered);
        assert_eq!(harness.memory().drag_source(), None);
        assert_eq!(harness.memory().released_drag_source(), Some(source));

        harness.advance_frame(Duration::from_millis(16));
        let released_source = harness.run_frame(|ui| ui.memory().released_drag_source()).0;
        assert_eq!(released_source, None);
    }
}

#[test]
fn pointer_interaction_transformed_drop_target_reports_active_and_released_sources() {
    let mut harness = UiTestHarness::new();
    let source = start_transformed_drag_over_target(&mut harness);

    let active = harness
        .run_frame(|ui| {
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            drop_target_transformed(
                target,
                local_target_rect(),
                transformed_target_transform(),
                input,
                memory,
                false,
            )
        })
        .0;

    assert_eq!(active.source, Some(source));
    assert!(!active.dropped);
    assert!(active.response.state.hovered);
    assert_eq!(harness.memory().pointer_capture(), Some(source));
    assert_eq!(harness.memory().drag_source(), Some(source));

    for source_first in [true, false] {
        let mut released = UiTestHarness::new();
        let source = start_transformed_drag_over_target(&mut released);

        released.pointer_release(MouseButton::Primary);
        let (source_response, target_response) = released
            .run_frame(|ui| {
                let source = source_id(ui);
                let target = target_id(ui);
                let (input, memory) = ui.input_and_memory_mut();

                if source_first {
                    let source_response = draggable_transformed(
                        source,
                        source_rect(),
                        transformed_source_transform(),
                        input,
                        memory,
                        false,
                    );
                    let target_response = drop_target_transformed(
                        target,
                        local_target_rect(),
                        transformed_target_transform(),
                        input,
                        memory,
                        false,
                    );
                    (source_response, target_response)
                } else {
                    let target_response = drop_target_transformed(
                        target,
                        local_target_rect(),
                        transformed_target_transform(),
                        input,
                        memory,
                        false,
                    );
                    let source_response = draggable_transformed(
                        source,
                        source_rect(),
                        transformed_source_transform(),
                        input,
                        memory,
                        false,
                    );
                    (source_response, target_response)
                }
            })
            .0;

        assert_eq!(source_response.id, source);
        assert_eq!(target_response.source, Some(source));
        assert!(target_response.dropped);
        assert!(target_response.response.state.hovered);
        assert_eq!(released.memory().released_drag_source(), Some(source));
    }
}

#[test]
fn pointer_interaction_drop_target_rejects_self_disabled_and_missed_releases() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    harness.set_pointer_position(Point::new(20.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    harness.pointer_release(MouseButton::Primary);
    let self_drop = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            let source_response = draggable(source, source_rect(), input, memory, false);
            let target_response = drop_target(source, source_rect(), input, memory, false);
            (source_response, target_response)
        })
        .0;
    assert_eq!(self_drop.1.source, None);
    assert!(!self_drop.1.dropped);

    let mut disabled = UiTestHarness::new();
    disabled.set_pointer_position(Point::new(10.0, 10.0));
    disabled.pointer_press(MouseButton::Primary);
    let _ = disabled.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    disabled.set_pointer_position(Point::new(50.0, 10.0));
    let _ = disabled.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    disabled.set_pointer_position(Point::new(90.0, 10.0));
    disabled.pointer_release(MouseButton::Primary);
    let disabled_drop = disabled
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(source, source_rect(), input, memory, false);
            drop_target(target, target_rect(), input, memory, true)
        })
        .0;
    assert!(disabled_drop.response.state.disabled);
    assert_eq!(disabled_drop.source, None);
    assert!(!disabled_drop.dropped);

    let mut missed = UiTestHarness::new();
    missed.set_pointer_position(Point::new(10.0, 10.0));
    missed.pointer_press(MouseButton::Primary);
    let _ = missed.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    missed.set_pointer_position(Point::new(50.0, 10.0));
    let _ = missed.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    missed.set_pointer_position(Point::new(160.0, 10.0));
    missed.pointer_release(MouseButton::Primary);
    let (_, missed_drop) = missed
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable(source, source_rect(), input, memory, false),
                drop_target(target, target_rect(), input, memory, false),
            )
        })
        .0;
    assert_eq!(missed_drop.source, None);
    assert!(!missed_drop.dropped);
}

#[test]
fn pointer_interaction_transformed_drop_target_rejects_self_source() {
    let mut harness = UiTestHarness::new();
    press_transformed_source(&mut harness, Point::new(110.0, 10.0));
    drag_transformed_source_to(&mut harness, Point::new(120.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let (_, target_response) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
                drop_target_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
            )
        })
        .0;

    assert_eq!(target_response.source, None);
    assert!(!target_response.dropped);
}

#[test]
fn pointer_interaction_transformed_drop_target_rejects_disabled_target() {
    let mut harness = UiTestHarness::new();
    let _ = start_transformed_drag_over_target(&mut harness);
    harness.pointer_release(MouseButton::Primary);

    let disabled_drop = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable_transformed(
                source,
                source_rect(),
                transformed_source_transform(),
                input,
                memory,
                false,
            );
            drop_target_transformed(
                target,
                local_target_rect(),
                transformed_target_transform(),
                input,
                memory,
                true,
            )
        })
        .0;

    assert!(disabled_drop.response.state.disabled);
    assert_eq!(disabled_drop.source, None);
    assert!(!disabled_drop.dropped);
}

#[test]
fn pointer_interaction_transformed_drop_target_rejects_missed_target() {
    let mut harness = UiTestHarness::new();
    press_transformed_source(&mut harness, Point::new(110.0, 10.0));
    drag_transformed_source_to(&mut harness, Point::new(400.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let (_, missed_drop) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
                drop_target_transformed(
                    target,
                    local_target_rect(),
                    transformed_target_transform(),
                    input,
                    memory,
                    false,
                ),
            )
        })
        .0;

    assert_eq!(missed_drop.source, None);
    assert!(!missed_drop.dropped);
}

#[test]
fn pointer_interaction_tooltip_idle_hover_requires_no_active_buttons_and_enabled_target() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    let idle = tooltip_response(&mut harness, false);

    assert!(idle.state.hovered);
    assert!(idle.tooltip_requested);

    harness.pointer_press(MouseButton::Primary);
    let primary_down = tooltip_response(&mut harness, false);
    assert!(!primary_down.tooltip_requested);

    harness.pointer_release(MouseButton::Primary);
    tooltip_response(&mut harness, false);
    harness.pointer_press(MouseButton::Secondary);
    let secondary_down = tooltip_response(&mut harness, false);
    assert!(!secondary_down.tooltip_requested);

    harness.pointer_release(MouseButton::Secondary);
    tooltip_response(&mut harness, false);
    let disabled = tooltip_response(&mut harness, true);
    assert!(disabled.state.disabled);
    assert!(!disabled.state.hovered);
    assert!(!disabled.tooltip_requested);
}

#[test]
fn pointer_interaction_scrollable_wheel_only_when_hovered_and_clamps_offset() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.wheel(Vec2::new(-80.0, -500.0));
    let scrolled = scroll_response(&mut harness, false);

    assert!(scrolled.response.state.hovered);
    assert_eq!(scrolled.max_offset, Vec2::new(50.0, 160.0));
    assert_eq!(scrolled.offset, Vec2::new(50.0, 160.0));
    assert_eq!(scrolled.delta, Vec2::new(50.0, 160.0));

    harness.set_pointer_position(Point::new(220.0, 10.0));
    harness.wheel(Vec2::new(80.0, 500.0));
    let outside = scroll_response(&mut harness, false);

    assert!(!outside.response.state.hovered);
    assert_eq!(outside.offset, Vec2::new(50.0, 160.0));
    assert_eq!(outside.delta, Vec2::ZERO);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.wheel(Vec2::new(80.0, 500.0));
    let disabled = scroll_response(&mut harness, true);

    assert!(disabled.response.state.disabled);
    assert!(!disabled.response.state.hovered);
    assert_eq!(disabled.offset, Vec2::new(50.0, 160.0));
    assert_eq!(disabled.delta, Vec2::ZERO);
}

#[test]
fn pointer_interaction_transformed_scrollable_wheel_only_when_hit_and_clamps_offset() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(110.0, 60.0));
    harness.wheel(Vec2::new(-80.0, -500.0));
    let scrolled = harness
        .run_frame(|ui| {
            let id = ui.id("scroll");
            let (input, memory) = ui.input_and_memory_mut();
            scrollable_transformed(
                id,
                rect(),
                press_transform(),
                Size::new(150.0, 200.0),
                input,
                memory,
                false,
            )
        })
        .0;

    assert!(scrolled.response.state.hovered);
    assert_eq!(scrolled.max_offset, Vec2::new(50.0, 160.0));
    assert_eq!(scrolled.offset, Vec2::new(50.0, 160.0));
    assert_eq!(scrolled.delta, Vec2::new(50.0, 160.0));

    harness.set_pointer_position(Point::new(90.0, 60.0));
    harness.wheel(Vec2::new(80.0, 500.0));
    let outside = harness
        .run_frame(|ui| {
            let id = ui.id("scroll");
            let (input, memory) = ui.input_and_memory_mut();
            scrollable_transformed(
                id,
                rect(),
                press_transform(),
                Size::new(150.0, 200.0),
                input,
                memory,
                false,
            )
        })
        .0;

    assert!(!outside.response.state.hovered);
    assert_eq!(outside.offset, Vec2::new(50.0, 160.0));
    assert_eq!(outside.delta, Vec2::ZERO);
}

#[test]
fn pointer_interaction_transformed_focus_select_context_and_tooltip_use_transformed_hits() {
    let mut focus = UiTestHarness::new();
    focus.set_pointer_position(Point::new(110.0, 60.0));
    focus.pointer_press(MouseButton::Primary);
    let _ = focus.run_frame(|ui| {
        let id = ui.id("field");
        let (input, memory) = ui.input_and_memory_mut();
        focusable_transformed(id, rect(), press_transform(), input, memory, false)
    });
    focus.pointer_release(MouseButton::Primary);
    let focused = focus
        .run_frame(|ui| {
            let id = ui.id("field");
            let (input, memory) = ui.input_and_memory_mut();
            focusable_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;
    assert!(focused.state.focused);
    assert_eq!(focus.memory().focused(), Some(focused.id));

    let mut selectable = UiTestHarness::new();
    selectable.set_pointer_position(Point::new(110.0, 60.0));
    let selected = selectable
        .run_frame(|ui| {
            let id = ui.id("row");
            let (input, memory) = ui.input_and_memory_mut();
            selectable_transformed(id, rect(), press_transform(), input, memory, true, false)
        })
        .0;
    assert!(selected.state.hovered);
    assert!(selected.state.selected);

    let mut menu = UiTestHarness::new();
    menu.set_pointer_position(Point::new(110.0, 60.0));
    menu.pointer_press(MouseButton::Secondary);
    let _ = menu.run_frame(|ui| {
        let id = ui.id("menu");
        let (input, memory) = ui.input_and_memory_mut();
        context_menu_trigger_transformed(id, rect(), press_transform(), input, memory, false)
    });
    menu.pointer_release(MouseButton::Secondary);
    let context = menu
        .run_frame(|ui| {
            let id = ui.id("menu");
            let (input, memory) = ui.input_and_memory_mut();
            context_menu_trigger_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;
    assert!(context.secondary_clicked);
    assert!(context.context_requested);

    let mut tooltip = UiTestHarness::new();
    tooltip.set_pointer_position(Point::new(110.0, 60.0));
    let shown = tooltip
        .run_frame(|ui| {
            let id = ui.id("tooltip");
            let (input, memory) = ui.input_and_memory_mut();
            tooltip_trigger_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;
    assert!(shown.state.hovered);
    assert!(shown.tooltip_requested);

    tooltip.set_pointer_position(Point::new(90.0, 60.0));
    let missed = tooltip
        .run_frame(|ui| {
            let id = ui.id("tooltip");
            let (input, memory) = ui.input_and_memory_mut();
            tooltip_trigger_transformed(id, rect(), press_transform(), input, memory, false)
        })
        .0;
    assert!(!missed.state.hovered);
    assert!(!missed.tooltip_requested);
}

#[test]
fn pointer_interaction_disabled_press_context_and_scroll_primitives_do_not_interact() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let press = pressable_response(&mut harness, true);

    assert!(press.state.disabled);
    assert!(!press.state.hovered);
    assert!(!press.state.pressed);
    assert!(!press.clicked);
    assert_eq!(harness.memory().active(), None);
    assert_eq!(harness.memory().pointer_capture(), None);

    harness.pointer_release(MouseButton::Primary);
    pressable_response(&mut harness, true);
    harness.pointer_press(MouseButton::Secondary);
    context_menu_response(&mut harness, true);
    harness.pointer_release(MouseButton::Secondary);
    let menu = context_menu_response(&mut harness, true);

    assert!(menu.state.disabled);
    assert!(!menu.secondary_clicked);
    assert!(!menu.context_requested);
    assert_eq!(harness.memory().secondary_pressed(), None);

    harness.wheel(Vec2::new(0.0, -40.0));
    let scroll = scroll_response(&mut harness, true);
    assert!(scroll.response.state.disabled);
    assert_eq!(scroll.offset, Vec2::ZERO);
    assert_eq!(scroll.delta, Vec2::ZERO);
}

#[test]
fn pointer_interaction_focused_context_menu_trigger_accepts_shift_f10() {
    let mut harness = UiTestHarness::new();

    harness.set_modifiers(Modifiers::new(true, false, false, false));
    harness.key_press(Key::Function(10));
    let response = harness
        .run_frame(|ui| {
            let id = ui.id("menu");
            ui.memory_mut().focus(id);
            let (input, memory) = ui.input_and_memory_mut();
            context_menu_trigger(id, rect(), input, memory, false)
        })
        .0;

    assert!(response.context_requested);
}
