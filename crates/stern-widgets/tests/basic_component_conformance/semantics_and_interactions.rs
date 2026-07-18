#[allow(unused_imports)]
use super::{
    AssetSlotAsset, AssetSlotConfig, BasicComponentCase, Brush, Color, ColorFieldConfig,
    CursorShape, DropdownItem, DropdownItemId, DropdownModel, IconId, Key, KeyEvent, KeyState,
    KeyboardInput, Modifiers, NumericScrubInputConfig, PlatformRequest, Point, PointerButtonState,
    PointerInput, Primitive, PropertyGridAffordanceLayout, PropertyGridRow, RadioGroupChoice, Rect,
    RepaintRequest, Response, SelectFieldConfig, SemanticActionKind, SemanticNode, SemanticRole,
    SemanticValue, SliderStep, TextEditState, Theme, Ui, UiInput, UiMemory, Vec2, WidgetId,
    WidgetOutput, assert_disabled_basic_control_semantics,
    assert_disabled_component_clears_retained_active, assert_disabled_not_focused,
    assert_enabled_basic_control_semantics, assert_selection_control_clicks_and_respects_disabled,
    asset_slot_field, button, checkbox_with_label, checked_radio_labels, color_field,
    component_output, default_dark_theme, double_released_at, dragged_at, frame_slider_current,
    has_semantic_action, icon_button, interactive_request, label, panel, pointer_input, pressed_at,
    pressed_key, property_grid_row_affordance_controls, property_grid_row_affordance_rects,
    radio_button_with_label, radio_group_choices, released_at, select_field,
    slider_semantic_current, slider_with_label, stage9_rect, toggle_with_label,
};

#[test]
fn stage9_basic_components_expose_semantic_roles_states_and_values() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut slider_value = 0.62;
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Title");
    ui.button("button", Rect::new(0.0, 24.0, 90.0, 28.0), "Run", false);
    ui.icon_button(
        "icon",
        Rect::new(0.0, 56.0, 28.0, 28.0),
        stern_icons_phosphor::regular::CHECK,
        "Save project",
        false,
    );
    ui.checkbox_with_label(
        "checkbox",
        Rect::new(0.0, 92.0, 20.0, 20.0),
        "Enable snapping",
        true,
        false,
    );
    ui.radio_button_with_label(
        "radio",
        Rect::new(0.0, 120.0, 20.0, 20.0),
        "Blend mode",
        true,
        false,
    );
    ui.toggle_with_label(
        "toggle",
        Rect::new(0.0, 148.0, 36.0, 18.0),
        "Loop playback",
        true,
        false,
    );
    ui.slider_with_label(
        "slider",
        Rect::new(0.0, 176.0, 120.0, 12.0),
        "Opacity",
        &mut slider_value,
        0.0..=1.0,
        false,
    );
    ui.panel(Rect::new(0.0, 200.0, 160.0, 80.0));

    let output = ui.finish_output();
    let nodes = output.semantics.nodes();
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::Label && node.label.as_deref() == Some("Title")
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::Button
            && node.label.as_deref() == Some("Run")
            && node.focusable
            && !node.state.disabled
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::IconButton && node.label.as_deref() == Some("Save project")
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::CheckBox
            && node.label.as_deref() == Some("Enable snapping")
            && node.state.checked == Some(true)
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::RadioButton
            && node.label.as_deref() == Some("Blend mode")
            && node.state.selected
            && node.state.checked == Some(true)
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::Toggle
            && node.label.as_deref() == Some("Loop playback")
            && node.state.checked == Some(true)
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::Slider
            && node.label.as_deref() == Some("Opacity")
            && matches!(
                node.state.value,
                Some(SemanticValue::Number { current, min, max })
                    if (current - 0.62).abs() < f32::EPSILON
                        && (min - 0.0).abs() < f32::EPSILON
                        && (max - 1.0).abs() < f32::EPSILON
            )
    }));
    assert!(nodes.iter().any(|node| {
        node.role == SemanticRole::Panel && node.label.as_deref() == Some("Panel")
    }));
}

#[test]
fn stage1_basic_control_matrix_exposes_semantic_contracts() {
    let theme = default_dark_theme();
    let rect = stage9_rect();

    for case in [
        BasicComponentCase::Button,
        BasicComponentCase::IconButton,
        BasicComponentCase::Checkbox,
        BasicComponentCase::Radio,
        BasicComponentCase::Toggle,
        BasicComponentCase::Slider,
    ] {
        let mut memory = UiMemory::new();
        let output = component_output(
            case,
            WidgetId::from_key(format!("{}-enabled", case.key())),
            rect,
            &UiInput::default(),
            &mut memory,
            &theme,
            false,
        );
        let response = output
            .response
            .unwrap_or_else(|| panic!("{} response", case.name()));
        let node = output
            .semantics
            .first()
            .unwrap_or_else(|| panic!("{} semantic node", case.name()));
        assert_enabled_basic_control_semantics(case, node, response);

        let disabled_id = WidgetId::from_key(format!("{}-disabled", case.key()));
        let mut memory = UiMemory::new();
        memory.focus(disabled_id);
        let disabled = component_output(
            case,
            disabled_id,
            rect,
            &pressed_at(4.0, 4.0),
            &mut memory,
            &theme,
            true,
        );
        let disabled_response = disabled
            .response
            .unwrap_or_else(|| panic!("{} disabled response", case.name()));
        let disabled_node = disabled
            .semantics
            .first()
            .unwrap_or_else(|| panic!("{} disabled semantic node", case.name()));
        assert_disabled_basic_control_semantics(case, disabled_node, disabled_response);
    }
}

#[test]
fn stage9_button_and_icon_button_click_and_disabled_paths_are_deterministic() {
    let theme = default_dark_theme();
    let rect = stage9_rect();

    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let pressed = button(
        WidgetId::from_key("button"),
        rect,
        "Run",
        &input,
        &mut memory,
        &theme,
        false,
    );
    assert!(pressed.response.unwrap().state.pressed);
    assert!(interactive_request(&pressed, CursorShape::PointingHand));

    let input = released_at(4.0, 4.0);
    let released = button(
        WidgetId::from_key("button"),
        rect,
        "Run",
        &input,
        &mut memory,
        &theme,
        false,
    );
    assert!(released.response.unwrap().clicked);

    let icon_id = WidgetId::from_key("icon-button");
    let mut icon_memory = UiMemory::new();
    let pressed = icon_button(
        icon_id,
        rect,
        stern_icons_phosphor::regular::CHECK,
        "Save",
        &pressed_at(4.0, 4.0),
        &mut icon_memory,
        &theme,
        false,
    );
    assert!(pressed.response.unwrap().state.pressed);
    assert!(interactive_request(&pressed, CursorShape::PointingHand));

    let released = icon_button(
        icon_id,
        rect,
        stern_icons_phosphor::regular::CHECK,
        "Save",
        &released_at(4.0, 4.0),
        &mut icon_memory,
        &theme,
        false,
    );
    assert!(released.response.unwrap().clicked);

    for (name, output) in [
        (
            "button",
            button(
                WidgetId::from_key("button-disabled"),
                rect,
                "Run",
                &pressed_at(4.0, 4.0),
                &mut UiMemory::new(),
                &theme,
                true,
            ),
        ),
        (
            "icon button",
            icon_button(
                WidgetId::from_key("icon-disabled"),
                rect,
                stern_icons_phosphor::regular::CHECK,
                "Save",
                &pressed_at(4.0, 4.0),
                &mut UiMemory::new(),
                &theme,
                true,
            ),
        ),
    ] {
        let response = output.response.expect(name);
        assert!(response.state.disabled, "{name}");
        assert!(!response.clicked, "{name}");
        assert!(!response.state.hovered, "{name}");
        assert!(!response.state.pressed, "{name}");
        assert!(output.platform_requests.is_empty(), "{name}");
        assert!(output.semantics[0].state.disabled, "{name}");
    }
}

#[test]
fn stage9_disabled_components_do_not_report_focus_when_already_focused() {
    let theme = default_dark_theme();
    let rect = stage9_rect();

    let id = WidgetId::from_key("focused-disabled-button");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let output = button(
        id,
        rect,
        "Run",
        &UiInput::default(),
        &mut memory,
        &theme,
        true,
    );
    assert_disabled_not_focused("button", &output);

    let id = WidgetId::from_key("focused-disabled-icon");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let output = icon_button(
        id,
        rect,
        stern_icons_phosphor::regular::CHECK,
        "Save",
        &UiInput::default(),
        &mut memory,
        &theme,
        true,
    );
    assert_disabled_not_focused("icon button", &output);

    let id = WidgetId::from_key("focused-disabled-checkbox");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let output = checkbox_with_label(
        id,
        rect,
        "Snap",
        false,
        &UiInput::default(),
        &mut memory,
        &theme,
        true,
    );
    assert_disabled_not_focused("checkbox", &output);

    let id = WidgetId::from_key("focused-disabled-radio");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let output = radio_button_with_label(
        id,
        rect,
        "Mode",
        false,
        &UiInput::default(),
        &mut memory,
        &theme,
        true,
    );
    assert_disabled_not_focused("radio", &output);

    let id = WidgetId::from_key("focused-disabled-toggle");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let output = toggle_with_label(
        id,
        rect,
        "Loop",
        false,
        &UiInput::default(),
        &mut memory,
        &theme,
        true,
    );
    assert_disabled_not_focused("toggle", &output);

    let id = WidgetId::from_key("focused-disabled-slider");
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut value = 0.25;
    let output = slider_with_label(
        id,
        rect,
        "Opacity",
        &mut value,
        0.0..=1.0,
        &UiInput::default(),
        &mut memory,
        &theme,
        true,
    );
    assert_disabled_not_focused("slider", &output);
}

#[test]
fn stage9_disabled_components_do_not_report_retained_active_or_pressed() {
    for case in [
        BasicComponentCase::Button,
        BasicComponentCase::IconButton,
        BasicComponentCase::Checkbox,
        BasicComponentCase::Radio,
        BasicComponentCase::Toggle,
        BasicComponentCase::Slider,
    ] {
        assert_disabled_component_clears_retained_active(case);
    }
}

#[test]
fn stage9_selection_controls_click_toggle_and_respect_disabled_state() {
    for case in [
        BasicComponentCase::Checkbox,
        BasicComponentCase::Radio,
        BasicComponentCase::Toggle,
    ] {
        assert_selection_control_clicks_and_respects_disabled(case);
    }
}

#[test]
fn stage9_slider_updates_finitely_and_respects_disabled_state() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("slider");
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
    let mut memory = UiMemory::new();
    let mut value = 0.0;

    let output = slider_with_label(
        id,
        rect,
        "Opacity",
        &mut value,
        0.0..=1.0,
        &pressed_at(75.0, 6.0),
        &mut memory,
        &theme,
        false,
    );
    let response = output.response.expect("slider response");
    assert!(response.state.active);
    assert!((value - 0.75).abs() < f32::EPSILON);
    assert!(interactive_request(&output, CursorShape::ResizeHorizontal));
    assert!(
        matches!(output.semantics[0].state.value, Some(SemanticValue::Number { current, .. }) if (current - 0.75).abs() < f32::EPSILON)
    );

    let mut degenerate_value = f32::NAN;
    let output = slider_with_label(
        WidgetId::from_key("degenerate-slider"),
        Rect::new(0.0, 0.0, 0.0, 12.0),
        "Degenerate",
        &mut degenerate_value,
        f32::NAN..=f32::INFINITY,
        &pressed_at(75.0, 6.0),
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert!(
        matches!(output.semantics[0].state.value, Some(SemanticValue::Number { current, min, max })
            if current.is_finite() && min.is_finite() && max.is_finite())
    );

    let mut disabled_value = 0.25;
    let disabled = slider_with_label(
        WidgetId::from_key("disabled-slider"),
        rect,
        "Opacity",
        &mut disabled_value,
        0.0..=1.0,
        &pressed_at(80.0, 6.0),
        &mut UiMemory::new(),
        &theme,
        true,
    );
    let response = disabled.response.expect("disabled slider response");
    assert!(response.state.disabled);
    assert!(!response.state.active);
    assert!(!response.clicked);
    assert!(!response.dragged);
    assert!((disabled_value - 0.25).abs() < f32::EPSILON);
    assert!(disabled.platform_requests.is_empty());
}
