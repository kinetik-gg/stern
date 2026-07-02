use std::time::Duration;

use kinetik_ui_core::{
    MouseButton, Point, Rect, UiTestHarness, context_menu_trigger, pressable_transformed,
};

use crate::support::{context_menu_response, press_transform, pressable_response, rect};

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
