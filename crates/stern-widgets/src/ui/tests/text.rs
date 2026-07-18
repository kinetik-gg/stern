#[allow(unused_imports)]
use super::{
    ActionContext, ActionDescriptor, ActionSource, Brush, Color, CursorShape, Duration,
    FrameContext, FrameOutput, FrameWarning, IconId, ImageId, Insets, Key, Modifiers, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, Primitive, Rect, RepaintRequest, ScaleFactor,
    SemanticNode, SemanticRole, Size, TextEditState, TextInputEvent, TextLayoutKey,
    TextLayoutStore, TextPrimitive, TextRange, TextStyle, TimeInfo, Ui, UiInput, UiMemory, Vec2,
    ViewportInfo, WidgetId, committed_text, default_dark_theme, frame_context, frame_context_at,
    held_at, input_at, pressed_at, pressed_key, released_at, scrolled_at, text_field_has_caret,
};

#[test]
fn ui_text_field_requests_platform_text_input_when_focused() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
    let press_output = ui.finish_output();
    assert!(press_output.platform_requests.iter().any(|request| {
        matches!(
            request,
            PlatformRequest::StartTextInput {
                rect: Some(rect),
            } if *rect == Rect::new(4.0, 4.0, 1.0, 16.0)
        )
    }));

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::UpdateTextInputRect {
                rect: Rect::new(4.0, 4.0, 1.0, 16.0),
            })
    );
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
    )));
}

#[test]
fn ui_text_field_interaction_requests_followup_repaint() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");
    let rect = Rect::new(0.0, 0.0, 120.0, 24.0);

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", rect, &mut state, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.text_field("field", rect, &mut state, false);
    assert!(output.widget.response.expect("text field response").clicked);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_text_field_changes_request_followup_repaint() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");
    let rect = Rect::new(0.0, 0.0, 120.0, 24.0);

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", rect, &mut state, false);
    let _ = ui.finish_output();

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", rect, &mut state, false);
    let _ = ui.finish_output();

    let input = committed_text("d");
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.text_field("field", rect, &mut state, false);

    assert!(output.changed);
    assert_eq!(state.text, "dabc");
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_text_field_caret_motion_requests_followup_repaint() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");
    let field = WidgetId::from_key("root").child("field");
    memory.focus(field);
    let input = pressed_key(Key::ArrowLeft);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);

    assert!(!output.changed);
    assert_eq!(state.caret(), 2);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_text_field_composition_requests_followup_repaint() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");
    let field = WidgetId::from_key("root").child("field");
    memory.focus(field);
    let input = UiInput {
        text_events: vec![TextInputEvent::Composition {
            text: "pre".to_owned(),
            selection: Some(TextRange::new(1, 2)),
        }],
        ..UiInput::default()
    };
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);

    assert!(!output.changed);
    assert_eq!(state.text, "abc");
    assert!(state.composition.is_some());
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_text_field_blinks_caret_and_schedules_repaint() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let field = WidgetId::from_key("root").child("field");
    memory.focus(field);

    let mut state = TextEditState::new("abc");
    let mut ui = Ui::begin_frame(
        frame_context_at(Duration::from_millis(0)),
        &mut memory,
        &theme,
    );
    ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
    let output = ui.finish_output();
    assert!(text_field_has_caret(&output));
    assert_eq!(
        output.repaint,
        RepaintRequest::After(Duration::from_millis(500))
    );

    let mut ui = Ui::begin_frame(
        frame_context_at(Duration::from_millis(500)),
        &mut memory,
        &theme,
    );
    ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
    let output = ui.finish_output();
    assert!(!text_field_has_caret(&output));
    assert_eq!(
        output.repaint,
        RepaintRequest::After(Duration::from_millis(500))
    );
}

#[test]
fn ui_text_field_schedules_partial_blink_delay() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let field = WidgetId::from_key("root").child("field");
    memory.focus(field);
    let mut state = TextEditState::new("abc");
    let mut ui = Ui::begin_frame(
        frame_context_at(Duration::from_millis(750)),
        &mut memory,
        &theme,
    );

    ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert!(!text_field_has_caret(&output));
    assert_eq!(
        output.repaint,
        RepaintRequest::After(Duration::from_millis(250))
    );
}

#[test]
fn ui_text_fields_use_public_text_widget() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);

    assert!(!output.changed);
    assert!(!ui.finish().is_empty());
}

#[test]
fn ui_multi_line_text_field_uses_public_widget() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("one\ntwo");
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output =
        ui.multi_line_text_field("field", Rect::new(0.0, 0.0, 160.0, 80.0), &mut state, false);

    assert_eq!(output.visible_lines, 2);
    assert!(!ui.finish().is_empty());
}
