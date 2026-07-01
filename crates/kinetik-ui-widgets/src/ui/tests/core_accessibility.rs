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
fn ui_labeled_controls_preserve_accessible_names() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut value = 0.4;
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.checkbox_with_label(
        "snap",
        Rect::new(0.0, 0.0, 20.0, 20.0),
        "Enable snapping",
        true,
        false,
    );
    ui.radio_button_with_label(
        "blend",
        Rect::new(0.0, 24.0, 20.0, 20.0),
        "Blend mode",
        true,
        false,
    );
    ui.toggle_with_label(
        "loop",
        Rect::new(0.0, 48.0, 36.0, 18.0),
        "Loop playback",
        true,
        false,
    );
    ui.slider_with_label(
        "opacity",
        Rect::new(0.0, 72.0, 100.0, 12.0),
        "Brush opacity",
        &mut value,
        0.0..=1.0,
        false,
    );
    ui.icon_button_with_label(
        "save",
        Rect::new(0.0, 96.0, 24.0, 24.0),
        IconId::from_raw(1),
        "Save project",
        false,
    );
    let output = ui.finish_output();

    let labels = output
        .semantics
        .nodes()
        .iter()
        .filter_map(|node| node.label.as_deref())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Enable snapping"));
    assert!(labels.contains(&"Blend mode"));
    assert!(labels.contains(&"Loop playback"));
    assert!(labels.contains(&"Brush opacity"));
    assert!(labels.contains(&"Save project"));
}

#[test]
fn ui_icon_buttons_use_registered_vector_icons() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut icons = IconLibrary::new();
    let icon = IconId::from_raw(7);
    icons.register(icon, check_icon());
    let mut ui = Ui::new(&input, &mut memory, &theme).with_icons(&icons);

    ui.icon_button_with_label(
        "check",
        Rect::new(0.0, 0.0, 24.0, 24.0),
        icon,
        "Apply",
        false,
    );
    let output = ui.finish_output();

    assert_eq!(output.primitives.len(), 2);
    assert!(matches!(output.primitives[1], Primitive::Path(_)));
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::IconButton && node.label.as_deref() == Some("Apply")
    }));
}

#[test]
fn ui_image_icon_button_uses_bitmap_icon_widget() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.image_icon_button(
        "save",
        Rect::new(0.0, 0.0, 24.0, 24.0),
        ImageId::from_raw(7),
        "Save project",
        false,
    );
    let output = ui.finish_output();

    assert!(
        output
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Image(_)))
    );
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::IconButton && node.label.as_deref() == Some("Save project")
    }));
}
