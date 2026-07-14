use super::*;

#[test]
fn label_emits_text() {
    let output = label(
        Rect::new(0.0, 0.0, 100.0, 20.0),
        "Name",
        &default_dark_theme(),
    );

    assert!(matches!(output.primitives[0], Primitive::Text(_)));
    assert!(output.response.is_none());
}

#[test]
fn panel_frame_resolves_clamped_body_rect() {
    let frame = PanelFrame::new(Rect::new(10.0, 20.0, 100.0, 50.0), Insets::all(12.0));

    assert_eq!(frame.outer, Rect::new(10.0, 20.0, 100.0, 50.0));
    assert_eq!(frame.body, Rect::new(22.0, 32.0, 76.0, 26.0));

    let clamped = PanelFrame::new(Rect::new(0.0, 0.0, 10.0, 8.0), Insets::all(20.0));
    assert_eq!(clamped.body, Rect::new(20.0, 20.0, 0.0, 0.0));
}

#[test]
fn button_emits_surface_and_text_and_clicks() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let id = WidgetId::from_key("button");
    let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
    let mut input = input_at(4.0, 4.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    let _ = button(id, rect, "Run", &input, &mut memory, &theme, false);
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let output = button(id, rect, "Run", &input, &mut memory, &theme, false);

    assert_eq!(output.primitives.len(), 2);
    assert!(output.response.expect("button response").clicked);
}

#[test]
fn icon_button_emits_vector_fallback_symbol() {
    let output = icon_button(
        WidgetId::from_key("icon"),
        Rect::new(0.0, 0.0, 24.0, 24.0),
        IconId::from_raw(1),
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );

    assert_eq!(output.primitives.len(), 3);
    assert!(matches!(output.primitives[0], Primitive::Rect(_)));
    assert!(matches!(output.primitives[1], Primitive::Path(_)));
    assert!(matches!(output.primitives[2], Primitive::Line(_)));
}

#[test]
fn icon_button_with_label_preserves_accessible_name() {
    let output = icon_button_with_label(
        WidgetId::from_key("icon"),
        Rect::new(0.0, 0.0, 24.0, 24.0),
        IconId::from_raw(1),
        "Save project",
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );

    assert_eq!(output.semantics[0].role, SemanticRole::IconButton);
    assert_eq!(output.semantics[0].label.as_deref(), Some("Save project"));
}

#[test]
fn image_icon_button_emits_button_surface_and_image() {
    let output = image_icon_button(
        WidgetId::from_key("bitmap-icon"),
        Rect::new(0.0, 0.0, 24.0, 24.0),
        ImageId::from_raw(99),
        "Save project",
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );

    assert_eq!(output.primitives.len(), 2);
    assert!(matches!(output.primitives[0], Primitive::Rect(_)));
    let Primitive::Image(image) = output.primitives[1] else {
        panic!("expected image primitive");
    };
    assert_eq!(image.tint, None);
    assert_eq!(output.semantics[0].role, SemanticRole::IconButton);
    assert_eq!(output.semantics[0].label.as_deref(), Some("Save project"));
}

#[test]
fn image_icon_button_uses_common_scale_integer_icon_size() {
    let output = image_icon_button(
        WidgetId::from_key("bitmap-icon"),
        Rect::new(0.0, 0.0, 28.0, 28.0),
        ImageId::from_raw(99),
        "Save project",
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );
    let Primitive::Image(image) = output.primitives[1] else {
        panic!("expected image primitive");
    };

    assert_approx(image.rect.width, 16.0);
    assert_approx(image.rect.height, 16.0);
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        assert_approx((image.rect.width * scale).fract(), 0.0);
        assert_approx((image.rect.height * scale).fract(), 0.0);
    }
}

#[test]
fn sized_image_icon_button_uses_requested_common_scale_icon_size() {
    let output = image_icon_button_sized(
        WidgetId::from_key("bitmap-icon"),
        Rect::new(0.0, 0.0, 30.0, 26.0),
        ImageId::from_raw(99),
        "Save project",
        24.0,
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );
    let Primitive::Image(image) = output.primitives[1] else {
        panic!("expected image primitive");
    };

    assert_approx(image.rect.width, 24.0);
    assert_approx(image.rect.height, 24.0);
    assert_approx(image.rect.x, 3.0);
    assert_approx(image.rect.y, 1.0);
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        assert_approx((image.rect.width * scale).fract(), 0.0);
        assert_approx((image.rect.height * scale).fract(), 0.0);
    }
}

#[test]
fn icon_button_uses_registered_vector_icon() {
    let mut icons = IconLibrary::new();
    let icon = IconId::from_raw(7);
    icons.register(icon, check_icon());

    let output = icon_button_with_library(
        WidgetId::from_key("icon"),
        Rect::new(0.0, 0.0, 24.0, 24.0),
        icon,
        "Check",
        &icons,
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );

    assert!(icons.has_icon(icon));
    assert_eq!(output.primitives.len(), 2);
    assert!(matches!(output.primitives[0], Primitive::Rect(_)));
    assert!(matches!(output.primitives[1], Primitive::Path(_)));
}

#[test]
fn tab_and_row_surfaces_are_not_button_clones() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let input = input_at(4.0, 4.0);
    let tab = tab_button(
        WidgetId::from_key("tab"),
        Rect::new(0.0, 0.0, 90.0, 28.0),
        "Tab",
        true,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let row = list_row(
        WidgetId::from_key("row"),
        Rect::new(0.0, 32.0, 140.0, 26.0),
        "Row",
        true,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(tab.response.expect("tab response").state.selected);
    assert!(row.response.expect("row response").state.selected);
    assert_ne!(tab.primitives.len(), row.primitives.len());
}

#[test]
fn tab_and_row_reflect_clicked_selection_same_frame() {
    let theme = default_dark_theme();
    let mut tab_memory = UiMemory::new();
    let tab_id = WidgetId::from_key("tab");
    let tab_rect = Rect::new(0.0, 0.0, 90.0, 28.0);
    let mut input = input_at(4.0, 4.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    tab_button(
        tab_id,
        tab_rect,
        "Tab",
        false,
        &input,
        &mut tab_memory,
        &theme,
        false,
    );
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let tab = tab_button(
        tab_id,
        tab_rect,
        "Tab",
        false,
        &input,
        &mut tab_memory,
        &theme,
        false,
    );

    let tab_response = tab.response.expect("tab response");
    assert!(tab_response.clicked);
    assert!(tab_response.state.selected);
    assert!(tab.semantics[0].state.selected);

    let mut row_memory = UiMemory::new();
    let row_id = WidgetId::from_key("row");
    let row_rect = Rect::new(0.0, 32.0, 140.0, 26.0);
    let mut input = input_at(4.0, 36.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    list_row(
        row_id,
        row_rect,
        "Row",
        false,
        &input,
        &mut row_memory,
        &theme,
        false,
    );
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let row = list_row(
        row_id,
        row_rect,
        "Row",
        false,
        &input,
        &mut row_memory,
        &theme,
        false,
    );

    let row_response = row.response.expect("row response");
    assert!(row_response.clicked);
    assert!(row_response.state.selected);
    assert!(row.semantics[0].state.selected);

    let mut icon_memory = UiMemory::new();
    let icon_id = WidgetId::from_key("image-icon");
    let icon_rect = Rect::new(0.0, 64.0, 28.0, 28.0);
    let mut input = input_at(4.0, 68.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    image_icon_selectable_button(
        icon_id,
        icon_rect,
        ImageId::from_raw(7),
        "Tool",
        false,
        &input,
        &mut icon_memory,
        &theme,
        false,
    );
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let icon = image_icon_selectable_button(
        icon_id,
        icon_rect,
        ImageId::from_raw(7),
        "Tool",
        false,
        &input,
        &mut icon_memory,
        &theme,
        false,
    );

    let icon_response = icon.response.expect("icon response");
    assert!(icon_response.clicked);
    assert!(icon_response.state.selected);
    assert!(icon.semantics[0].state.selected);
}

#[test]
fn checkbox_and_toggle_reflect_selection() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let checkbox = checkbox(
        WidgetId::from_key("check"),
        Rect::new(0.0, 0.0, 20.0, 20.0),
        true,
        &input_at(1.0, 1.0),
        &mut memory,
        &theme,
        false,
    );
    let toggle = toggle(
        WidgetId::from_key("toggle"),
        Rect::new(0.0, 0.0, 36.0, 18.0),
        true,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );

    assert!(checkbox.response.expect("checkbox response").state.selected);
    assert_eq!(toggle.primitives.len(), 2);
}

#[test]
fn controls_emit_stable_response_flags_and_semantic_states() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("row");
    let rect = Rect::new(0.0, 0.0, 140.0, 26.0);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let input = input_at(4.0, 4.0);

    let row = list_row(id, rect, "Asset", true, &input, &mut memory, &theme, true);
    let response = row.response.expect("row response");

    assert_eq!(response.id, id);
    assert_eq!(response.rect, rect);
    assert!(response.state.disabled);
    assert!(response.state.selected);
    assert!(response.state.focused);
    assert!(!response.state.hovered);
    assert!(!response.state.active);
    assert!(!response.state.pressed);
    assert!(!response.clicked);
    assert!(!response.double_clicked);
    assert!(!response.secondary_clicked);
    assert!(!response.dragged);
    assert!(!response.keyboard_activated);

    let node = &row.semantics[0];
    assert_eq!(node.role, SemanticRole::ListItem);
    assert_eq!(node.label.as_deref(), Some("Asset"));
    assert!(!node.focusable);
    assert!(node.state.disabled);
    assert!(node.state.selected);
    assert!(!node.state.focused);
    assert!(!node.state.pressed);
    assert!(
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Invoke)
    );
}

#[test]
fn checkbox_and_toggle_reflect_clicked_selection_same_frame() {
    let theme = default_dark_theme();
    let mut checkbox_memory = UiMemory::new();
    let check_id = WidgetId::from_key("check");
    let check_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut input = input_at(10.0, 10.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    checkbox(
        check_id,
        check_rect,
        false,
        &input,
        &mut checkbox_memory,
        &theme,
        false,
    );
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let checkbox = checkbox(
        check_id,
        check_rect,
        false,
        &input,
        &mut checkbox_memory,
        &theme,
        false,
    );

    let checkbox_response = checkbox.response.expect("checkbox response");
    assert!(checkbox_response.clicked);
    assert!(checkbox_response.state.selected);
    assert_eq!(checkbox.semantics[0].state.checked, Some(true));

    let mut toggle_memory = UiMemory::new();
    let toggle_id = WidgetId::from_key("toggle");
    let toggle_rect = Rect::new(0.0, 0.0, 36.0, 18.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    toggle(
        toggle_id,
        toggle_rect,
        false,
        &input,
        &mut toggle_memory,
        &theme,
        false,
    );
    input.pointer.primary = PointerButtonState::new(false, false, true);
    let toggle = toggle(
        toggle_id,
        toggle_rect,
        false,
        &input,
        &mut toggle_memory,
        &theme,
        false,
    );

    let toggle_response = toggle.response.expect("toggle response");
    assert!(toggle_response.clicked);
    assert!(toggle_response.state.selected);
    assert_eq!(toggle.semantics[0].state.checked, Some(true));
    assert!(matches!(
        toggle.primitives[1],
        Primitive::Rect(RectPrimitive { rect, .. }) if rect.x > toggle_rect.x
    ));
}

#[test]
fn labeled_controls_preserve_accessible_names() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let input = UiInput::default();
    let mut slider_value = 0.5;

    let checkbox = checkbox_with_label(
        WidgetId::from_key("check"),
        Rect::new(0.0, 0.0, 20.0, 20.0),
        "Enable snapping",
        true,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let radio = radio_button_with_label(
        WidgetId::from_key("radio"),
        Rect::new(0.0, 24.0, 20.0, 20.0),
        "Blend mode",
        true,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let toggle = toggle_with_label(
        WidgetId::from_key("toggle"),
        Rect::new(0.0, 48.0, 36.0, 18.0),
        "Loop playback",
        true,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let slider = slider_with_label(
        WidgetId::from_key("slider"),
        Rect::new(0.0, 72.0, 100.0, 12.0),
        "Brush opacity",
        &mut slider_value,
        0.0..=1.0,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert_eq!(
        checkbox.semantics[0].label.as_deref(),
        Some("Enable snapping")
    );
    assert_eq!(radio.semantics[0].role, SemanticRole::RadioButton);
    assert_eq!(radio.semantics[0].label.as_deref(), Some("Blend mode"));
    assert_eq!(toggle.semantics[0].label.as_deref(), Some("Loop playback"));
    assert_eq!(slider.semantics[0].label.as_deref(), Some("Brush opacity"));
}

#[test]
fn slider_updates_value_from_pointer_position() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("slider");
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
    let mut memory = UiMemory::new();
    let mut value = 0.0;
    let mut input = input_at(50.0, 6.0);

    input.pointer.primary = PointerButtonState::new(true, true, false);
    slider(
        id,
        rect,
        &mut value,
        0.0..=1.0,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!((value - 0.5).abs() < f32::EPSILON);
}

#[test]
fn focused_slider_keyboard_activation_does_not_write_from_stale_pointer() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("slider");
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut value = 2.0;
    let mut input = input_at(rect.max_x() + 500.0, 6.0);
    input.keyboard = KeyboardInput {
        events: vec![KeyEvent::new(
            Key::Enter,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
        ..KeyboardInput::default()
    };

    let output = slider(
        id,
        rect,
        &mut value,
        0.0..=1.0,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let response = output.response.expect("slider response");

    assert!(response.clicked);
    assert!(response.keyboard_activated);
    assert!((value - 2.0).abs() < f32::EPSILON);
    assert_approx(rect_width(&output.primitives[1]), rect.width);
    assert!(matches!(
        output.semantics[0].state.value,
        Some(SemanticValue::Number { current, min, max })
            if (current - 1.0).abs() < f32::EPSILON
                && min.abs() < f32::EPSILON
                && (max - 1.0).abs() < f32::EPSILON
    ));
}

#[test]
fn slider_degenerate_geometry_and_range_stay_finite() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("slider");
    let mut memory = UiMemory::new();
    let mut input = input_at(50.0, 6.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);

    let mut zero_width_value = 15.0;
    let zero_width = slider(
        id,
        Rect::new(0.0, 0.0, 0.0, 12.0),
        &mut zero_width_value,
        10.0..=20.0,
        &input,
        &mut memory,
        &theme,
        false,
    );
    assert!((zero_width_value - 15.0).abs() < f32::EPSILON);
    assert!(rect_width(&zero_width.primitives[1]).is_finite());

    let mut equal_range_value = 12.0;
    let equal_range = slider(
        WidgetId::from_key("equal_range_slider"),
        Rect::new(0.0, 0.0, 100.0, 12.0),
        &mut equal_range_value,
        4.0..=4.0,
        &input,
        &mut memory,
        &theme,
        false,
    );
    assert!((equal_range_value - 4.0).abs() < f32::EPSILON);
    assert!(rect_width(&equal_range.primitives[1]).abs() < f32::EPSILON);
}

#[test]
fn slider_clamps_edge_values_for_display_semantics_and_mapping() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("slider");
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
    let mut memory = UiMemory::new();

    let mut above_range = 2.0;
    let output = slider(
        id,
        rect,
        &mut above_range,
        0.0..=1.0,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!((above_range - 2.0).abs() < f32::EPSILON);
    assert_approx(rect_width(&output.primitives[1]), rect.width);
    assert!(matches!(
        output.semantics[0].state.value,
        Some(SemanticValue::Number { current, min, max })
            if (current - 1.0).abs() < f32::EPSILON
                && min.abs() < f32::EPSILON
                && (max - 1.0).abs() < f32::EPSILON
    ));

    let mut non_finite = f32::NAN;
    let output = slider(
        WidgetId::from_key("nan_slider"),
        rect,
        &mut non_finite,
        f32::NAN..=f32::INFINITY,
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert!(non_finite.is_nan());
    assert!(rect_width(&output.primitives[1]).is_finite());
    assert!(matches!(
        output.semantics[0].state.value,
        Some(SemanticValue::Number { current, min, max })
            if current.is_finite() && min.is_finite() && max.is_finite()
    ));

    let mut descending = 5.0;
    let mut input = input_at(rect.max_x() - 0.001, 6.0);
    input.pointer.primary = PointerButtonState::new(true, true, false);
    let output = slider(
        WidgetId::from_key("descending_slider"),
        rect,
        &mut descending,
        10.0..=0.0,
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert!(descending < 0.001);
    assert!(matches!(
        output.semantics[0].state.value,
        Some(SemanticValue::Number { current, min, max })
            if current < 0.001
                && min.abs() < f32::EPSILON
                && (max - 10.0).abs() < f32::EPSILON
    ));
}

fn rect_width(primitive: &Primitive) -> f32 {
    match primitive {
        Primitive::Rect(rect) => rect.rect.width,
        _ => panic!("expected rect primitive"),
    }
}

#[test]
fn panel_emits_exact_recipe_rectangle_without_shadow_and_images_stay_single() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let recipe = theme.panel();
    let panel = panel(rect, &theme);

    assert_eq!(
        panel.primitives,
        vec![Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        })]
    );
    assert!(panel.response.is_none());
    assert!(
        panel
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Shadow(_)))
    );
    assert!(matches!(
        image(rect, ImageId::from_raw(1)).primitives.as_slice(),
        [Primitive::Image(_)]
    ));
}
