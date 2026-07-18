#[allow(unused_imports)]
use super::{
    ActionContext, ActionDescriptor, ActionSource, Brush, Color, CursorShape, FrameContext,
    FrameOutput, FrameWarning, IconId, ImageId, Insets, Key, Modifiers, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, Primitive, Rect, RepaintRequest, ScaleFactor,
    SemanticNode, SemanticRole, Size, TextEditState, TextInputEvent, TextLayoutKey,
    TextLayoutStore, TextPrimitive, TextRange, TextStyle, TimeInfo, Ui, UiInput, UiMemory, Vec2,
    ViewportInfo, WidgetId, committed_text, default_dark_theme, frame_context, frame_context_at,
    held_at, input_at, pressed_at, pressed_key, released_at, scrolled_at, text_field_has_caret,
};

#[test]
fn ui_forwards_widget_platform_requests() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let input = input_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.button("run", Rect::new(0.0, 0.0, 80.0, 28.0), "Run", false);
    let output = ui.finish_output();

    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
}

#[test]
fn ui_widget_cursor_requests_respect_pointer_capture_owner() {
    let theme = default_dark_theme();
    let capture_owner = WidgetId::from_key("root").child("drag-source");
    let mut memory = UiMemory::new();
    memory.capture_pointer(capture_owner);
    let input = input_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.button("run", Rect::new(0.0, 0.0, 80.0, 28.0), "Run", false);
    let output = ui.finish_output();

    assert!(
        !output
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::SetCursor(_)))
    );
}

#[test]
fn ui_widget_cursor_requests_are_suppressed_on_cancellation_frame() {
    let theme = default_dark_theme();
    let owner = WidgetId::from_key("root").child("run");
    let mut memory = UiMemory::new();
    memory.activate(owner);
    memory.press(owner);
    memory.capture_pointer(owner);
    let mut input = input_at(4.0, 4.0);
    input.release_pointer_buttons();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.button("run", Rect::new(0.0, 0.0, 80.0, 28.0), "Run", false);
    let output = ui.finish_output();

    assert!(
        !output
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::SetCursor(_)))
    );
}

#[test]
fn ui_widget_cursor_routing_preserves_resize_and_text_cursors() {
    let theme = default_dark_theme();

    let mut value = 0.5;
    let mut memory = UiMemory::new();
    let input = input_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider(
        "gain",
        Rect::new(0.0, 0.0, 120.0, 12.0),
        &mut value,
        0.0..=1.0,
        false,
    );
    let output = ui.finish_output();
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::ResizeHorizontal))
    );

    let mut state = TextEditState::new("abc");
    let mut memory = UiMemory::new();
    let input = input_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
    let output = ui.finish_output();
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::Text))
    );
}
