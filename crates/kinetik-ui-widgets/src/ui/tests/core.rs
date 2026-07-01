#[allow(unused_imports)]
use super::{
    ActionContext, ActionDescriptor, ActionSource, Brush, Color, CursorShape, FrameContext,
    FrameOutput, FrameWarning, IconId, IconLibrary, ImageId, Insets, Key, Modifiers, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, Primitive, Rect, RepaintRequest, ScaleFactor,
    SemanticNode, SemanticRole, Size, TextEditState, TextInputEvent, TextLayoutKey,
    TextLayoutStore, TextPrimitive, TextRange, TextStyle, TimeInfo, Ui, UiInput, UiMemory, Vec2,
    ViewportInfo, WidgetId, check_icon, committed_text, default_dark_theme, frame_context,
    frame_context_at, held_at, input_at, pressed_at, pressed_key, released_at, scrolled_at,
    text_field_has_caret,
};

#[test]
fn ui_begin_frame_preserves_full_runtime_context() {
    let theme = default_dark_theme();
    let context = frame_context();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context.clone(), &mut memory, &theme);

    assert_eq!(ui.context(), &context);
    assert_eq!(ui.viewport(), context.viewport);
    assert_eq!(ui.time(), context.time);
    assert!(ui.output().primitives.is_empty());

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
    assert_eq!(ui.output().primitives.len(), 1);
}

#[test]
fn ui_begin_frame_with_text_layouts_attaches_layouts() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut text_layouts = TextLayoutStore::new();
    let mut ui =
        Ui::begin_frame_with_text_layouts(frame_context(), &mut memory, &theme, &mut text_layouts);

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
    let output = ui.finish_output();

    assert!(matches!(
        output.primitives.first(),
        Some(Primitive::Text(text)) if text.layout.is_some()
    ));
    assert_eq!(text_layouts.len(), 1);
}

#[test]
fn ui_collects_widget_primitives() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
    ui.panel(Rect::new(0.0, 24.0, 120.0, 48.0));
    let primitives = ui.finish();

    assert!(
        primitives
            .iter()
            .any(|item| matches!(item, Primitive::Text(_)))
    );
    assert!(
        primitives
            .iter()
            .any(|item| matches!(item, Primitive::Rect(_)))
    );
}

#[test]
fn ui_attaches_shaped_text_layouts_when_store_is_available() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut text_layouts = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut text_layouts);

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
    let output = ui.finish_output();

    let text = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text),
            _ => None,
        })
        .expect("label emits text");
    let layout = text.layout.expect("text layout is attached");
    assert!(text_layouts.layout(layout).is_some());
    assert_eq!(text_layouts.len(), 1);
}

#[test]
fn ui_uses_text_primitive_style_for_attached_layouts() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut text_layouts = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut text_layouts);

    ui.primitive(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(0.0, 16.0),
        text: "Styled".to_owned(),
        family: "monospace".to_owned(),
        size: 12.0,
        line_height: 17.0,
        brush: Brush::Solid(Color::WHITE),
    }));
    let output = ui.finish_output();
    let layout = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => text.layout,
            _ => None,
        })
        .expect("text layout is attached");
    let expected = text_layouts.layout_id(TextLayoutKey::new(
        "Styled",
        TextStyle::new("monospace", 12.0, 17.0),
        0.0,
        false,
    ));

    assert_eq!(layout, expected);
    assert_eq!(text_layouts.len(), 1);
}

#[test]
fn ui_text_field_uses_shaped_text_layout_store_for_caret_geometry() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("Wide text");
    let mut text_layouts = TextLayoutStore::new();
    memory.focus(WidgetId::from_key("root").child("field"));
    state.set_caret(4);
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut text_layouts);

    let output = ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 28.0), &mut state, false);
    assert!(!output.changed);
    let frame = ui.finish_output();

    assert!(!text_layouts.is_empty());
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.layout.is_some()))
    );
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if (rect.rect.width - 1.0).abs() < f32::EPSILON
                && rect.rect.height > theme.text_size
    )));
}

#[test]
fn ui_scroll_area_clips_translates_content_and_stores_offset() {
    let theme = default_dark_theme();
    let input = scrolled_at(8.0, 8.0, Vec2::new(0.0, -24.0));
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.scroll_area(
        "area",
        Rect::new(0.0, 0.0, 100.0, 80.0),
        Size::new(100.0, 200.0),
        false,
        |ui, offset| {
            ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Inside");
            offset
        },
    );
    assert_eq!(output.inner, Vec2::new(0.0, 24.0));
    assert_eq!(output.scroll.offset, Vec2::new(0.0, 24.0));

    let frame = ui.finish_output();
    assert_eq!(
        memory.scroll_offset(output.scroll.response.id),
        Vec2::new(0.0, 24.0)
    );
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);
    assert!(matches!(frame.primitives[0], Primitive::ClipBegin { .. }));
    assert!(matches!(
        frame.primitives[1],
        Primitive::TransformBegin(transform)
            if transform.dx.abs() < f32::EPSILON && (transform.dy + 24.0).abs() < f32::EPSILON
    ));
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(_)))
    );
    assert!(matches!(
        frame.primitives[frame.primitives.len() - 2],
        Primitive::TransformEnd
    ));
    assert!(matches!(
        frame.primitives[frame.primitives.len() - 1],
        Primitive::ClipEnd { .. }
    ));
}

#[test]
fn ui_panel_body_emits_balanced_body_clip() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let body = ui.panel_body(
        "inspector",
        Rect::new(10.0, 20.0, 120.0, 80.0),
        Insets::new(8.0, 10.0, 12.0, 14.0),
        |ui, body| {
            ui.label(body, "Inside");
            body
        },
    );
    let frame = ui.finish_output();

    assert_eq!(body, Rect::new(18.0, 32.0, 102.0, 54.0));
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Rect(_)))
    );
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::ClipBegin { rect, .. } if *rect == body
    )));
    assert!(matches!(
        frame.primitives.last(),
        Some(Primitive::ClipEnd { .. })
    ));
    assert!(frame.warnings.is_empty());
}

#[test]
fn ui_routes_button_interaction_through_memory() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let response = ui.button("run", Rect::new(0.0, 0.0, 80.0, 28.0), "Run", false);

    assert!(response.state.hovered);
    assert!(response.state.pressed);
}

#[test]
fn ui_action_button_queues_action_invocation_on_click() {
    let theme = default_dark_theme();
    let action = ActionDescriptor::new("run", "Run");
    let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
    let mut memory = UiMemory::new();

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let pressed = ui
        .action_button("run", rect, &action, ActionContext::Global)
        .expect("visible action");
    assert!(pressed.state.pressed);
    assert!(ui.finish_output().actions.is_empty());

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let clicked = ui
        .action_button("run", rect, &action, ActionContext::Global)
        .expect("visible action");
    let mut output = ui.finish_output();

    assert!(clicked.clicked);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert_eq!(output.actions.len(), 1);
    let invocation = output.actions.pop_front().expect("queued action");
    assert_eq!(invocation.action_id, action.id);
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Global);
}

#[test]
fn ui_action_button_respects_hidden_and_disabled_action_state() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
    let mut hidden = ActionDescriptor::new("hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = ActionDescriptor::new("disabled", "Disabled");
    disabled.state.enabled = false;

    let mut memory = UiMemory::new();
    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    assert!(
        ui.action_button("hidden", rect, &hidden, ActionContext::Global)
            .is_none()
    );
    let response = ui
        .action_button("disabled", rect, &disabled, ActionContext::Global)
        .expect("disabled action is visible");
    let output = ui.finish_output();

    assert!(response.state.disabled);
    assert!(output.actions.is_empty());
    assert!(!output.primitives.is_empty());
}

#[test]
fn ui_can_invoke_action_descriptors_without_a_button_surface() {
    let theme = default_dark_theme();
    let mut action = ActionDescriptor::new("export", "Export");
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    assert!(ui.invoke_action_descriptor(&action, ActionSource::Menu, ActionContext::Global));
    action.state.enabled = false;
    assert!(!ui.invoke_action_descriptor(&action, ActionSource::Menu, ActionContext::Global));

    let output = ui.finish_output();
    assert_eq!(output.actions.len(), 1);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_interactive_press_and_click_request_followup_repaint() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
    let mut memory = UiMemory::new();

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.button("run", rect, "Run", false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = held_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let held = ui.button("run", rect, "Run", false);
    assert!(held.state.pressed);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::None);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.button("run", rect, "Run", false);
    let output = ui.finish_output();

    assert!(response.clicked);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}
