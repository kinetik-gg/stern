use std::time::Duration;

use kinetik_ui_core::{
    CursorShape, MouseButton, PlatformRequest, Point, UiTestHarness, Vec2, WidgetId, draggable,
    draggable_transformed, drop_target, pressable, pressable_transformed,
};

use crate::support::{
    context_menu_response, local_target_rect, source_id, source_rect, target_id, target_rect,
    transformed_source_transform, transformed_target_transform,
};

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
    let ((drop, source), output) = harness.run_frame(|ui| {
        let source = source_id(ui);
        let target = target_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        (
            drop_target(target, target_rect(), input, memory, false),
            draggable(source, source_rect(), input, memory, false),
        )
    });

    assert!(!drop.dropped);
    assert!(!drop.response.state.hovered);
    assert!(!source.clicked);
    assert!(!source.state.active);
    assert!(!source.dragged);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
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
    let (frame_start_memory, output) = harness.run_frame(|ui| {
        (
            ui.memory().pointer_capture(),
            ui.memory().drag_source(),
            ui.memory().released_drag_source(),
            ui.memory().pointer_interaction_cancelled(),
        )
    });

    assert_eq!(frame_start_memory.0, None);
    assert_eq!(frame_start_memory.1, None);
    assert_eq!(frame_start_memory.2, None);
    assert!(frame_start_memory.3);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
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
