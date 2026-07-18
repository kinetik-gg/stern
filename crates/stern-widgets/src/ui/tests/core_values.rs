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
fn ui_request_repaint_exposes_app_state_dirty_path() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.request_repaint(RepaintRequest::NextFrame);

    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_exposes_custom_semantics_and_platform_requests() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let id = ui.id("custom-button");

    ui.push_semantic_node(
        SemanticNode::new(
            id,
            SemanticRole::IconButton,
            Rect::new(0.0, 0.0, 24.0, 24.0),
        )
        .with_label("Custom"),
    );
    ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
    let output = ui.finish_output();

    assert!(output.semantics.nodes().iter().any(|node| {
        node.id == id
            && node.role == SemanticRole::IconButton
            && node.label.as_deref() == Some("Custom")
    }));
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
}

#[test]
fn ui_toggle_value_mutates_and_reflects_clicked_state_same_frame() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 54.0, 24.0);
    let mut memory = UiMemory::new();
    let mut value = false;

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let pressed = ui.toggle_value("toggle", rect, &mut value, false);
    assert!(pressed.state.pressed);
    assert!(!value);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let clicked = ui.toggle_value("toggle", rect, &mut value, false);
    let output = ui.finish_output();

    assert!(clicked.clicked);
    assert!(clicked.state.selected);
    assert!(value);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_labeled_value_helpers_return_mutated_state_same_frame() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 120.0, 28.0);

    let mut checked = false;
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.checkbox_value_with_label("checkbox", rect, "Checkbox", &mut checked, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let checkbox = ui.checkbox_value_with_label("checkbox", rect, "Checkbox", &mut checked, false);
    let output = ui.finish_output();
    assert!(checkbox.clicked);
    assert!(checkbox.state.selected);
    assert!(checked);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut on = false;
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.toggle_value_with_label("toggle", rect, "Toggle", &mut on, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let toggle = ui.toggle_value_with_label("toggle", rect, "Toggle", &mut on, false);
    let output = ui.finish_output();
    assert!(toggle.clicked);
    assert!(toggle.state.selected);
    assert!(on);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut selected = 0_usize;
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.radio_button_value_with_label("radio", rect, "Radio", &mut selected, 1, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let radio = ui.radio_button_value_with_label("radio", rect, "Radio", &mut selected, 1, false);
    let output = ui.finish_output();
    assert!(radio.clicked);
    assert!(radio.state.selected);
    assert_eq!(selected, 1);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_selection_value_helpers_mutate_and_reflect_clicked_state_same_frame() {
    let theme = default_dark_theme();
    let mut selected = 0_usize;

    let mut memory = UiMemory::new();
    let rect = Rect::new(0.0, 0.0, 120.0, 28.0);
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.list_row_value("row", rect, "Row", &mut selected, 1, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let row = ui.list_row_value("row", rect, "Row", &mut selected, 1, false);
    let output = ui.finish_output();
    assert!(row.clicked);
    assert!(row.state.selected);
    assert_eq!(selected, 1);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.tab_button_value("tab", rect, "Tab", &mut selected, 2, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let tab = ui.tab_button_value("tab", rect, "Tab", &mut selected, 2, false);
    let output = ui.finish_output();
    assert!(tab.clicked);
    assert!(tab.state.selected);
    assert_eq!(selected, 2);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.selectable_value("asset", rect, &mut selected, 3, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::None);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let asset = ui.selectable_value("asset", rect, &mut selected, 3, false);
    let output = ui.finish_output();
    assert!(asset.clicked);
    assert!(asset.state.selected);
    assert_eq!(selected, 3);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.radio_button_value("radio", rect, &mut selected, 4, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let radio = ui.radio_button_value("radio", rect, &mut selected, 4, false);
    let output = ui.finish_output();
    assert!(radio.clicked);
    assert!(radio.state.selected);
    assert_eq!(selected, 4);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.image_icon_button_value(
        "image-icon",
        rect,
        ImageId::from_raw(7),
        "Tool",
        &mut selected,
        5,
        false,
    );
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let icon = ui.image_icon_button_value(
        "image-icon",
        rect,
        ImageId::from_raw(7),
        "Tool",
        &mut selected,
        5,
        false,
    );
    let output = ui.finish_output();
    assert!(icon.clicked);
    assert!(icon.state.selected);
    assert_eq!(selected, 5);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_sized_image_icon_value_helper_mutates_and_reflects_clicked_state_same_frame() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 120.0, 28.0);
    let mut selected = 0_usize;
    let mut memory = UiMemory::new();

    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.image_icon_button_value_sized(
        "image-icon-sized",
        rect,
        ImageId::from_raw(8),
        "Sized tool",
        24.0,
        &mut selected,
        6,
        false,
    );
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let icon = ui.image_icon_button_value_sized(
        "image-icon-sized",
        rect,
        ImageId::from_raw(8),
        "Sized tool",
        24.0,
        &mut selected,
        6,
        false,
    );
    let output = ui.finish_output();

    assert!(icon.clicked);
    assert!(icon.state.selected);
    assert_eq!(selected, 6);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}
