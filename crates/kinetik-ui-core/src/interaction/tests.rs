use super::{
    clamp_scroll_offset, context_menu_trigger, draggable, drop_target, focusable, hit_test,
    max_scroll_offset, pressable, scrollable, selectable, tooltip_trigger,
};
use crate::Size;
use crate::{
    Key, KeyEvent, KeyState, Modifiers, Point, PointerButtonState, PointerInput, Rect, UiInput,
    UiMemory, Vec2, WidgetId,
};

fn input_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            ..PointerInput::default()
        },
        window_focused: true,
        ..UiInput::default()
    }
}

#[test]
fn hit_testing_uses_rect_containment() {
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);

    assert!(hit_test(rect, &input_at(5.0, 5.0)));
    assert!(!hit_test(rect, &input_at(10.0, 5.0)));
}

#[test]
fn pressable_tracks_hover_press_and_click() {
    let id = WidgetId::from_key("button");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();

    let mut input = input_at(5.0, 5.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    let response = pressable(id, rect, &input, &mut memory, false);
    assert!(response.state.hovered);
    assert!(response.state.active);
    assert!(response.state.pressed);
    assert!(memory.has_pointer_capture(id));

    input.pointer.primary = PointerButtonState::new(false, false, true);
    let response = pressable(id, rect, &input, &mut memory, false);
    assert!(response.clicked);
    assert!(!response.state.active);
    assert!(!response.state.pressed);
    assert_eq!(memory.active(), None);
    assert_eq!(memory.pointer_capture(), None);
}

#[test]
fn pressable_detects_double_and_secondary_clicks() {
    let id = WidgetId::from_key("button");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    input.pointer.click_count = 2;
    pressable(id, rect, &input, &mut memory, false);
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let response = pressable(id, rect, &input, &mut memory, false);
    assert!(response.double_clicked);

    input.pointer.primary = PointerButtonState::default();
    input.pointer.secondary = PointerButtonState::new(false, false, true);
    let response = pressable(id, rect, &input, &mut memory, false);
    assert!(!response.secondary_clicked);

    input.pointer.secondary = PointerButtonState::new(true, true, false);
    pressable(id, rect, &input, &mut memory, false);
    input.pointer.secondary = PointerButtonState::new(false, false, true);
    let response = pressable(id, rect, &input, &mut memory, false);
    assert!(response.secondary_clicked);
}

#[test]
fn pointer_capture_suppresses_hover_on_other_widgets() {
    let owner = WidgetId::from_key("owner");
    let other = WidgetId::from_key("other");
    let owner_rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let other_rect = Rect::new(20.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    let owner_press = pressable(owner, owner_rect, &input, &mut memory, false);
    assert!(owner_press.state.hovered);
    assert!(memory.has_pointer_capture(owner));

    memory.begin_frame();
    input.pointer.position = Some(Point::new(25.0, 5.0));
    input.pointer.primary = PointerButtonState::new(true, false, false);
    let other_response = pressable(other, other_rect, &input, &mut memory, false);

    assert!(!other_response.state.hovered);
    assert_eq!(memory.hovered(), None);
    assert_eq!(memory.pointer_capture(), Some(owner));
}

#[test]
fn secondary_click_requires_matching_press_owner() {
    let id = WidgetId::from_key("button");
    let other = WidgetId::from_key("other");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);
    memory.press_secondary(other);
    input.pointer.secondary = PointerButtonState::new(false, false, true);

    let response = pressable(id, rect, &input, &mut memory, false);

    assert!(!response.secondary_clicked);
    assert_eq!(memory.secondary_pressed(), Some(other));
}

#[test]
fn disabled_pressable_does_not_click() {
    let id = WidgetId::from_key("button");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);
    input.pointer.primary = PointerButtonState::new(false, false, true);

    let response = pressable(id, rect, &input, &mut memory, true);

    assert!(response.state.disabled);
    assert!(!response.clicked);
    assert!(!response.state.hovered);
}

#[test]
fn focusable_sets_focus_on_click() {
    let id = WidgetId::from_key("field");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    focusable(id, rect, &input, &mut memory, false);
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let response = focusable(id, rect, &input, &mut memory, false);

    assert!(response.state.focused);
    assert_eq!(memory.focused(), Some(id));
}

#[test]
fn focused_pressable_activates_from_keyboard() {
    let id = WidgetId::from_key("button");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let input = UiInput {
        keyboard: crate::KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                Key::Space,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    };

    let response = pressable(id, rect, &input, &mut memory, false);

    assert!(response.clicked);
    assert!(response.keyboard_activated);
    assert!(!response.state.pressed);
}

#[test]
fn draggable_reports_delta_while_active() {
    let id = WidgetId::from_key("handle");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    draggable(id, rect, &input, &mut memory, false);

    input.pointer.primary = PointerButtonState::new(true, false, false);
    input.pointer.position = Some(Point::new(20.0, 20.0));
    input.pointer.delta = Vec2::new(15.0, 15.0);
    let response = draggable(id, rect, &input, &mut memory, false);

    assert!(response.dragged);
    assert_eq!(response.drag_delta, Vec2::new(15.0, 15.0));
    assert_eq!(memory.drag_source(), Some(id));
}

#[test]
fn draggable_finishes_drag_on_release_for_drop_targets() {
    let id = WidgetId::from_key("handle");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    draggable(id, rect, &input, &mut memory, false);
    input.pointer.primary = PointerButtonState::new(true, false, false);
    input.pointer.delta = Vec2::new(1.0, 0.0);
    draggable(id, rect, &input, &mut memory, false);

    input.pointer.primary = PointerButtonState::new(false, false, true);
    draggable(id, rect, &input, &mut memory, false);

    assert_eq!(memory.drag_source(), None);
    assert_eq!(memory.released_drag_source(), Some(id));
}

#[test]
fn pointer_cancellation_suppresses_events_without_releasing_drag_source() {
    let focused = WidgetId::from_key("focused");
    let source = WidgetId::from_key("source");
    let target = WidgetId::from_key("target");
    let source_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(30.0, 0.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    memory.focus(focused);
    memory.set_text_input_owner(focused);
    let mut input = input_at(5.0, 5.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    draggable(source, source_rect, &input, &mut memory, false);

    input.pointer.position = Some(Point::new(35.0, 5.0));
    input.pointer.delta = Vec2::new(30.0, 0.0);
    input.pointer.primary = PointerButtonState::new(true, false, false);
    draggable(source, source_rect, &input, &mut memory, false);

    input.pointer.delta = Vec2::ZERO;
    input.release_pointer_buttons();
    memory.cancel_pointer_interaction();

    let drop = drop_target(target, target_rect, &input, &mut memory, false);
    let cancelled = draggable(source, source_rect, &input, &mut memory, false);

    assert!(!drop.dropped);
    assert!(!drop.response.state.hovered);
    assert!(!cancelled.clicked);
    assert!(!cancelled.state.active);
    assert_eq!(memory.pointer_capture(), None);
    assert_eq!(memory.drag_source(), None);
    assert_eq!(memory.released_drag_source(), None);
    assert_eq!(memory.focused(), Some(focused));
    assert_eq!(memory.text_input_owner(), Some(focused));
}

#[test]
fn selectable_preserves_selected_state() {
    let id = WidgetId::from_key("row");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();

    let response = selectable(id, rect, &input_at(5.0, 5.0), &mut memory, true, false);

    assert!(response.state.selected);
    assert!(response.state.hovered);
}

#[test]
fn scroll_helpers_clamp_offsets_to_content_bounds() {
    assert_eq!(
        max_scroll_offset(Size::new(100.0, 80.0), Size::new(140.0, 200.0)),
        Vec2::new(40.0, 120.0)
    );
    assert_eq!(
        clamp_scroll_offset(
            Vec2::new(f32::INFINITY, -20.0),
            Size::new(100.0, 80.0),
            Size::new(140.0, 200.0),
        ),
        Vec2::new(0.0, 0.0)
    );
    assert_eq!(
        clamp_scroll_offset(
            Vec2::new(90.0, 180.0),
            Size::new(100.0, 80.0),
            Size::new(140.0, 200.0),
        ),
        Vec2::new(40.0, 120.0)
    );
}

#[test]
fn scrollable_applies_hovered_wheel_delta_and_stores_offset() {
    let id = WidgetId::from_key("scroll");
    let rect = Rect::new(0.0, 0.0, 100.0, 80.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(20.0, 20.0);
    input.pointer.wheel_delta = Vec2::new(0.0, -30.0);

    let output = scrollable(
        id,
        rect,
        Size::new(100.0, 200.0),
        &input,
        &mut memory,
        false,
    );

    assert!(output.response.state.hovered);
    assert_eq!(output.offset, Vec2::new(0.0, 30.0));
    assert_eq!(output.delta, Vec2::new(0.0, 30.0));
    assert_eq!(memory.scroll_offset(id), Vec2::new(0.0, 30.0));
}

#[test]
fn scrollable_ignores_wheel_when_not_hovered_or_disabled() {
    let id = WidgetId::from_key("scroll");
    let rect = Rect::new(0.0, 0.0, 100.0, 80.0);
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(id, Vec2::new(0.0, 40.0));
    let mut input = input_at(120.0, 20.0);
    input.pointer.wheel_delta = Vec2::new(0.0, -30.0);

    let output = scrollable(
        id,
        rect,
        Size::new(100.0, 200.0),
        &input,
        &mut memory,
        false,
    );

    assert!(!output.response.state.hovered);
    assert_eq!(output.offset, Vec2::new(0.0, 40.0));

    let disabled = scrollable(
        id,
        rect,
        Size::new(100.0, 200.0),
        &input_at(20.0, 20.0),
        &mut memory,
        true,
    );
    assert!(disabled.response.state.disabled);
    assert_eq!(disabled.offset, Vec2::new(0.0, 40.0));
}

#[test]
fn context_menu_trigger_uses_secondary_click_and_shift_f10() {
    let id = WidgetId::from_key("menu");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(5.0, 5.0);
    input.pointer.secondary = PointerButtonState::new(true, true, false);
    context_menu_trigger(id, rect, &input, &mut memory, false);
    input.pointer.secondary = PointerButtonState::new(false, false, true);

    let pointer = context_menu_trigger(id, rect, &input, &mut memory, false);
    assert!(pointer.context_requested);

    memory.focus(id);
    let input = UiInput {
        keyboard: crate::KeyboardInput {
            modifiers: Modifiers::new(true, false, false, false),
            events: vec![KeyEvent::new(
                Key::Function(10),
                KeyState::Pressed,
                Modifiers::new(true, false, false, false),
                false,
            )],
        },
        ..UiInput::default()
    };
    let keyboard = context_menu_trigger(id, rect, &input, &mut memory, false);
    assert!(keyboard.context_requested);
}

#[test]
fn tooltip_trigger_reports_idle_hover_only() {
    let id = WidgetId::from_key("tip");
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut memory = UiMemory::new();

    let response = tooltip_trigger(id, rect, &input_at(5.0, 5.0), &mut memory, false);
    assert!(response.tooltip_requested);

    let mut pressed = input_at(5.0, 5.0);
    pressed.pointer.primary = PointerButtonState::new(true, true, false);
    let response = tooltip_trigger(id, rect, &pressed, &mut memory, false);
    assert!(!response.tooltip_requested);
}

#[test]
fn drop_target_reports_drag_source_released_over_target() {
    let source = WidgetId::from_key("source");
    let target = WidgetId::from_key("target");
    let source_rect = Rect::new(30.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    let mut input = input_at(35.0, 5.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    draggable(source, source_rect, &input, &mut memory, false);

    input.pointer.primary = PointerButtonState::new(true, false, false);
    input.pointer.delta = Vec2::new(5.0, 0.0);
    draggable(source, source_rect, &input, &mut memory, false);

    input.pointer.position = Some(Point::new(5.0, 5.0));
    input.pointer.primary = PointerButtonState::new(false, false, true);

    let output = drop_target(target, target_rect, &input, &mut memory, false);

    assert_eq!(output.source, Some(source));
    assert!(output.dropped);
    assert!(output.response.state.hovered);
}

#[test]
fn drop_target_does_not_accept_plain_pointer_capture() {
    let source = WidgetId::from_key("source");
    let target = WidgetId::from_key("target");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    memory.capture_pointer(source);
    let mut input = input_at(5.0, 5.0);
    input.pointer.primary = PointerButtonState::new(false, false, true);

    let output = drop_target(target, rect, &input, &mut memory, false);

    assert_eq!(output.source, None);
    assert!(!output.dropped);
}

#[test]
fn drop_target_ignores_self_capture_disabled_and_misses() {
    let target = WidgetId::from_key("target");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    memory.start_drag(target);
    let mut input = input_at(5.0, 5.0);
    input.pointer.primary = PointerButtonState::new(false, false, true);

    let own_capture = drop_target(target, rect, &input, &mut memory, false);
    assert_eq!(own_capture.source, None);
    assert!(!own_capture.dropped);

    memory.start_drag(WidgetId::from_key("source"));
    let disabled = drop_target(target, rect, &input, &mut memory, true);
    assert!(!disabled.dropped);

    let missed = drop_target(target, rect, &released_at(40.0, 40.0), &mut memory, false);
    assert!(!missed.dropped);
}

fn released_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        window_focused: true,
        ..UiInput::default()
    }
}
