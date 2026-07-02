use kinetik_ui_core::{
    MouseButton, Point, Rect, Response, ScrollResponse, Size, Transform, Ui, UiTestHarness, Vec2,
    WidgetId, context_menu_trigger, draggable, draggable_transformed, pressable, scrollable,
    tooltip_trigger,
};

pub(crate) fn rect() -> Rect {
    Rect::new(0.0, 0.0, 100.0, 40.0)
}

pub(crate) fn source_rect() -> Rect {
    Rect::new(0.0, 0.0, 40.0, 40.0)
}

pub(crate) fn target_rect() -> Rect {
    Rect::new(80.0, 0.0, 40.0, 40.0)
}

pub(crate) fn local_target_rect() -> Rect {
    Rect::new(0.0, 0.0, 40.0, 40.0)
}

pub(crate) fn translated(offset_x: f32, offset_y: f32) -> Transform {
    Transform::translation(Vec2::new(offset_x, offset_y))
}

pub(crate) fn press_transform() -> Transform {
    translated(100.0, 50.0)
}

pub(crate) fn transformed_source_transform() -> Transform {
    translated(100.0, 0.0)
}

pub(crate) fn transformed_target_transform() -> Transform {
    translated(200.0, 0.0)
}

pub(crate) fn pressable_response(harness: &mut UiTestHarness, disabled: bool) -> Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("pressable");
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, rect(), input, memory, disabled)
        })
        .0
}

pub(crate) fn context_menu_response(harness: &mut UiTestHarness, disabled: bool) -> Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("menu");
            let (input, memory) = ui.input_and_memory_mut();
            context_menu_trigger(id, rect(), input, memory, disabled)
        })
        .0
}

pub(crate) fn tooltip_response(harness: &mut UiTestHarness, disabled: bool) -> Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("tooltip");
            let (input, memory) = ui.input_and_memory_mut();
            tooltip_trigger(id, rect(), input, memory, disabled)
        })
        .0
}

pub(crate) fn scroll_response(harness: &mut UiTestHarness, disabled: bool) -> ScrollResponse {
    harness
        .run_frame(|ui| {
            let id = ui.id("scroll");
            let (input, memory) = ui.input_and_memory_mut();
            scrollable(id, rect(), Size::new(150.0, 200.0), input, memory, disabled)
        })
        .0
}

pub(crate) fn source_id(ui: &mut Ui<'_>) -> WidgetId {
    ui.id("source")
}

pub(crate) fn target_id(ui: &mut Ui<'_>) -> WidgetId {
    ui.id("target")
}

pub(crate) fn start_drag_over_target(harness: &mut UiTestHarness) -> WidgetId {
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

pub(crate) fn start_transformed_drag_over_target(harness: &mut UiTestHarness) -> WidgetId {
    press_transformed_source(harness, Point::new(110.0, 10.0));
    drag_transformed_source_to(harness, Point::new(210.0, 10.0))
}

pub(crate) fn press_transformed_source(harness: &mut UiTestHarness, point: Point) {
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

pub(crate) fn drag_transformed_source_to(harness: &mut UiTestHarness, point: Point) -> WidgetId {
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
