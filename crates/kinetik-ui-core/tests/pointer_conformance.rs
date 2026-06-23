//! Windowless pointer interaction conformance coverage.

use std::time::Duration;

use kinetik_ui_core::{
    Key, Modifiers, MouseButton, Point, Rect, Response, ScrollResponse, Size, Ui, UiTestHarness,
    Vec2, WidgetId, context_menu_trigger, draggable, drop_target, pressable, scrollable,
    tooltip_trigger,
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
fn pointer_interaction_drop_target_accepts_released_drag_source_over_target() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(50.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });

    harness.set_pointer_position(Point::new(90.0, 10.0));
    harness.pointer_release(MouseButton::Primary);
    let drop = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            let source_response = draggable(source, source_rect(), input, memory, false);
            let target_response = drop_target(target, target_rect(), input, memory, false);
            (source_response, target_response)
        })
        .0;

    assert_eq!(drop.1.source, Some(drop.0.id));
    assert!(drop.1.dropped);
    assert!(drop.1.response.state.hovered);
    assert_eq!(harness.memory().released_drag_source(), Some(drop.0.id));
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
    let (missed_source, missed_drop) = missed
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
    assert_eq!(missed_drop.source, Some(missed_source.id));
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
