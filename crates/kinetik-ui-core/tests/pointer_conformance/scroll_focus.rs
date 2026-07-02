use kinetik_ui_core::{
    Key, Modifiers, MouseButton, Point, Size, UiTestHarness, Vec2, context_menu_trigger,
    context_menu_trigger_transformed, focusable_transformed, scrollable_transformed,
    selectable_transformed, tooltip_trigger_transformed,
};

use crate::support::{
    context_menu_response, press_transform, pressable_response, rect, scroll_response,
    tooltip_response,
};

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
