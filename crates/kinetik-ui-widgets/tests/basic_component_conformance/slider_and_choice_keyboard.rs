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
    has_semantic_action, icon_button_with_label, interactive_request, label, panel, pointer_input,
    pressed_at, pressed_key, property_grid_row_affordance_controls,
    property_grid_row_affordance_rects, radio_button_with_label, radio_group_choices, released_at,
    select_field, slider_semantic_current, slider_with_label, stage9_rect, toggle_with_label,
};

#[test]
fn stage2_slider_keyboard_uses_default_and_configured_steps() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);

    let mut value = 0.5;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("slider"));
    let input = pressed_key(Key::ArrowRight);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.slider("slider", rect, &mut value, 0.0..=1.0, false);
    let output = ui.finish_output();

    assert!(response.keyboard_activated);
    assert!(!response.clicked);
    assert!((value - 0.51).abs() < f32::EPSILON);
    assert!((frame_slider_current(&output) - 0.51).abs() < f32::EPSILON);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut configured_value = 0.5;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("configured-slider"));
    let input = pressed_key(Key::ArrowUp);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.slider_with_step(
        "configured-slider",
        rect,
        &mut configured_value,
        0.0..=1.0,
        SliderStep::new(0.25),
        false,
    );
    let output = ui.finish_output();

    assert!(response.keyboard_activated);
    assert!((configured_value - 0.75).abs() < f32::EPSILON);
    assert!((frame_slider_current(&output) - 0.75).abs() < f32::EPSILON);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn stage2_slider_home_end_and_page_keys_clamp_to_bounds() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);

    let mut home_value = 0.4;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("home-slider"));
    let input = pressed_key(Key::Home);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider("home-slider", rect, &mut home_value, 0.0..=1.0, false);
    let output = ui.finish_output();
    assert!(home_value.abs() < f32::EPSILON);
    assert!(frame_slider_current(&output).abs() < f32::EPSILON);

    let mut end_value = 0.4;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("end-slider"));
    let input = pressed_key(Key::End);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider("end-slider", rect, &mut end_value, 0.0..=1.0, false);
    let output = ui.finish_output();
    assert!((end_value - 1.0).abs() < f32::EPSILON);
    assert!((frame_slider_current(&output) - 1.0).abs() < f32::EPSILON);

    let mut page_up_value = 0.5;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("page-up-slider"));
    let input = pressed_key(Key::PageUp);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider("page-up-slider", rect, &mut page_up_value, 0.0..=1.0, false);
    let output = ui.finish_output();
    assert!((page_up_value - 0.6).abs() < f32::EPSILON);
    assert!((frame_slider_current(&output) - 0.6).abs() < f32::EPSILON);

    let mut page_down_value = 0.05;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("page-down-slider"));
    let input = pressed_key(Key::PageDown);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider(
        "page-down-slider",
        rect,
        &mut page_down_value,
        0.0..=1.0,
        false,
    );
    let output = ui.finish_output();
    assert!(page_down_value.abs() < f32::EPSILON);
    assert!(frame_slider_current(&output).abs() < f32::EPSILON);
}

#[test]
fn stage2_disabled_slider_ignores_keyboard_and_does_not_report_focus() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
    let mut value = 0.5;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("slider"));
    let input = pressed_key(Key::ArrowRight);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let response = ui.slider("slider", rect, &mut value, 0.0..=1.0, true);
    let output = ui.finish_output();

    assert!(response.state.disabled);
    assert!(!response.state.focused);
    assert!(!response.keyboard_activated);
    assert!((value - 0.5).abs() < f32::EPSILON);
    assert_eq!(output.repaint, RepaintRequest::None);
    assert!(!output.semantics.nodes()[0].state.focused);
}

#[test]
fn stage2_slider_keyboard_keeps_invalid_ranges_finite_and_deterministic() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 100.0, 12.0);

    let mut invalid_value = f32::NAN;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("invalid-slider"));
    let input = pressed_key(Key::ArrowRight);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider(
        "invalid-slider",
        rect,
        &mut invalid_value,
        f32::NAN..=f32::INFINITY,
        false,
    );
    let output = ui.finish_output();
    assert!(invalid_value.is_finite());
    assert!(frame_slider_current(&output).is_finite());

    let mut equal_range_value = 8.0;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("equal-range-slider"));
    let input = pressed_key(Key::ArrowRight);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.slider(
        "equal-range-slider",
        rect,
        &mut equal_range_value,
        4.0..=4.0,
        false,
    );
    let output = ui.finish_output();
    assert!((equal_range_value - 4.0).abs() < f32::EPSILON);
    assert!((frame_slider_current(&output) - 4.0).abs() < f32::EPSILON);
}

#[test]
fn stage9_value_helpers_reflect_same_frame_changes_and_request_repaint() {
    let theme = default_dark_theme();
    let rect = stage9_rect();

    let mut checkbox_value = false;
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.checkbox_value("checkbox", rect, &mut checkbox_value, false);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.checkbox_value("checkbox", rect, &mut checkbox_value, false);
    let output = ui.finish_output();
    assert!(response.clicked);
    assert!(response.state.selected);
    assert!(checkbox_value);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut radio_value = 0_u8;
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.radio_button_value("radio", rect, &mut radio_value, 2, false);
    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.radio_button_value("radio", rect, &mut radio_value, 2, false);
    let output = ui.finish_output();
    assert!(response.clicked);
    assert!(response.state.selected);
    assert_eq!(radio_value, 2);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut toggle_value = false;
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.toggle_value("toggle", rect, &mut toggle_value, false);
    let input = released_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.toggle_value("toggle", rect, &mut toggle_value, false);
    let output = ui.finish_output();
    assert!(response.clicked);
    assert!(response.state.selected);
    assert!(toggle_value);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut slider_value = 0.0;
    let mut memory = UiMemory::new();
    let input = pressed_at(60.0, 6.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.slider(
        "slider",
        Rect::new(0.0, 0.0, 100.0, 12.0),
        &mut slider_value,
        0.0..=1.0,
        false,
    );
    let output = ui.finish_output();
    assert!(response.state.active);
    assert!((slider_value - 0.6).abs() < f32::EPSILON);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert!(
        output.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Slider
                && matches!(node.state.value, Some(SemanticValue::Number { current, .. }) if (current - 0.6).abs() < f32::EPSILON)
        })
    );
}

#[test]
fn stage2_choice_value_helpers_activate_from_keyboard() {
    let theme = default_dark_theme();
    let rect = stage9_rect();

    for key in [Key::Space, Key::Enter] {
        let mut checkbox_value = false;
        let mut memory = UiMemory::new();
        memory.focus(WidgetId::from_key("root").child("checkbox"));
        let input = pressed_key(key.clone());
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let response = ui.checkbox_value("checkbox", rect, &mut checkbox_value, false);
        let output = ui.finish_output();
        assert!(response.keyboard_activated);
        assert!(response.state.selected);
        assert!(checkbox_value);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);

        let mut radio_value = 0_u8;
        let mut memory = UiMemory::new();
        memory.focus(WidgetId::from_key("root").child("radio"));
        let input = pressed_key(key.clone());
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let response = ui.radio_button_value("radio", rect, &mut radio_value, 2, false);
        let output = ui.finish_output();
        assert!(response.keyboard_activated);
        assert!(response.state.selected);
        assert_eq!(radio_value, 2);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);

        let mut toggle_value = false;
        let mut memory = UiMemory::new();
        memory.focus(WidgetId::from_key("root").child("toggle"));
        let input = pressed_key(key);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let response = ui.toggle_value("toggle", rect, &mut toggle_value, false);
        let output = ui.finish_output();
        assert!(response.keyboard_activated);
        assert!(response.state.selected);
        assert!(toggle_value);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
    }
}

#[test]
fn stage2_disabled_choice_value_helpers_ignore_keyboard_activation() {
    let theme = default_dark_theme();
    let rect = stage9_rect();
    let input = pressed_key(Key::Space);

    let mut checkbox_value = false;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("checkbox"));
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.checkbox_value("checkbox", rect, &mut checkbox_value, true);
    assert!(response.state.disabled);
    assert!(!response.keyboard_activated);
    assert!(!response.clicked);
    assert!(!response.state.focused);
    assert!(!checkbox_value);

    let mut radio_value = 0_u8;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("radio"));
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.radio_button_value("radio", rect, &mut radio_value, 2, true);
    assert!(response.state.disabled);
    assert!(!response.keyboard_activated);
    assert!(!response.clicked);
    assert!(!response.state.focused);
    assert_eq!(radio_value, 0);

    let mut toggle_value = false;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("toggle"));
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.toggle_value("toggle", rect, &mut toggle_value, true);
    assert!(response.state.disabled);
    assert!(!response.keyboard_activated);
    assert!(!response.clicked);
    assert!(!response.state.focused);
    assert!(!toggle_value);
}

#[test]
fn stage2_choice_label_targets_activate_paired_controls_deterministically() {
    let theme = default_dark_theme();
    let control_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let label_rect = Rect::new(28.0, 0.0, 92.0, 20.0);
    let press = pressed_at(40.0, 8.0);
    let release = released_at(40.0, 8.0);

    let mut checkbox_value = false;
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&press, &mut memory, &theme);
    ui.checkbox_value_with_label_target(
        "checkbox",
        control_rect,
        label_rect,
        "Enable snapping",
        &mut checkbox_value,
        false,
    );
    let mut ui = Ui::new(&release, &mut memory, &theme);
    let response = ui.checkbox_value_with_label_target(
        "checkbox",
        control_rect,
        label_rect,
        "Enable snapping",
        &mut checkbox_value,
        false,
    );
    assert!(response.clicked);
    assert!(response.state.selected);
    assert!(checkbox_value);

    let mut radio_value = 0_u8;
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&press, &mut memory, &theme);
    ui.radio_button_value_with_label_target(
        "radio",
        control_rect,
        label_rect,
        "Blend mode",
        &mut radio_value,
        2,
        false,
    );
    let mut ui = Ui::new(&release, &mut memory, &theme);
    let response = ui.radio_button_value_with_label_target(
        "radio",
        control_rect,
        label_rect,
        "Blend mode",
        &mut radio_value,
        2,
        false,
    );
    assert!(response.clicked);
    assert!(response.state.selected);
    assert_eq!(radio_value, 2);

    let mut toggle_value = false;
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&press, &mut memory, &theme);
    ui.toggle_value_with_label_target(
        "toggle",
        control_rect,
        label_rect,
        "Loop playback",
        &mut toggle_value,
        false,
    );
    let mut ui = Ui::new(&release, &mut memory, &theme);
    let response = ui.toggle_value_with_label_target(
        "toggle",
        control_rect,
        label_rect,
        "Loop playback",
        &mut toggle_value,
        false,
    );
    assert!(response.clicked);
    assert!(response.state.selected);
    assert!(toggle_value);
}

#[test]
fn stage2_disabled_choice_label_targets_do_not_activate() {
    let theme = default_dark_theme();
    let control_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let label_rect = Rect::new(28.0, 0.0, 92.0, 20.0);
    let press = pressed_at(40.0, 8.0);

    let mut checkbox_value = false;
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&press, &mut memory, &theme);
    let response = ui.checkbox_value_with_label_target(
        "checkbox",
        control_rect,
        label_rect,
        "Enable snapping",
        &mut checkbox_value,
        true,
    );
    assert!(response.state.disabled);
    assert!(!response.state.pressed);
    assert!(!response.clicked);
    assert!(!checkbox_value);

    let mut radio_value = 0_u8;
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&press, &mut memory, &theme);
    let response = ui.radio_button_value_with_label_target(
        "radio",
        control_rect,
        label_rect,
        "Blend mode",
        &mut radio_value,
        2,
        true,
    );
    assert!(response.state.disabled);
    assert!(!response.state.pressed);
    assert!(!response.clicked);
    assert_eq!(radio_value, 0);

    let mut toggle_value = false;
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&press, &mut memory, &theme);
    let response = ui.toggle_value_with_label_target(
        "toggle",
        control_rect,
        label_rect,
        "Loop playback",
        &mut toggle_value,
        true,
    );
    assert!(response.state.disabled);
    assert!(!response.state.pressed);
    assert!(!response.clicked);
    assert!(!toggle_value);
}

#[test]
fn stage2_radio_group_activation_leaves_exactly_one_selected_option() {
    let theme = default_dark_theme();
    let choices = radio_group_choices();
    let mut selected = 1_u8;
    let mut memory = UiMemory::new();

    let press = pressed_at(4.0, 32.0);
    let mut ui = Ui::new(&press, &mut memory, &theme);
    ui.radio_group_value("modes", &mut selected, &choices);
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);

    let release = released_at(4.0, 32.0);
    let mut ui = Ui::new(&release, &mut memory, &theme);
    let group = ui.radio_group_value("modes", &mut selected, &choices);
    let output = ui.finish_output();

    assert!(group.changed);
    assert_eq!(selected, 2);
    assert_eq!(group.selected, 2);
    assert_eq!(group.selected_index, Some(1));
    assert_eq!(group.activated, Some(2));
    assert_eq!(group.activated_index, Some(1));
    assert_eq!(
        group
            .responses
            .iter()
            .filter(|response| response.state.selected)
            .count(),
        1
    );
    assert_eq!(checked_radio_labels(&output), vec!["Second"]);
}

#[test]
fn stage2_radio_group_disabled_options_cannot_become_selected() {
    let theme = default_dark_theme();
    let choices = vec![
        RadioGroupChoice::new("first", Rect::new(0.0, 0.0, 20.0, 20.0), "First", 1),
        RadioGroupChoice::new("second", Rect::new(0.0, 28.0, 20.0, 20.0), "Second", 2)
            .disabled(true),
        RadioGroupChoice::new("third", Rect::new(0.0, 56.0, 20.0, 20.0), "Third", 3),
    ];
    let mut selected = 1_u8;
    let mut memory = UiMemory::new();

    let press = pressed_at(4.0, 32.0);
    let mut ui = Ui::new(&press, &mut memory, &theme);
    let pressed = ui.radio_group_value("modes", &mut selected, &choices);
    assert!(!pressed.responses[1].state.pressed);
    assert!(pressed.responses[1].state.disabled);
    assert_eq!(selected, 1);

    let release = released_at(4.0, 32.0);
    let mut ui = Ui::new(&release, &mut memory, &theme);
    let group = ui.radio_group_value("modes", &mut selected, &choices);
    let output = ui.finish_output();

    assert!(!group.changed);
    assert_eq!(selected, 1);
    assert_eq!(group.selected_index, Some(0));
    assert_eq!(group.activated, None);
    assert_eq!(checked_radio_labels(&output), vec!["First"]);
}

#[test]
fn stage2_radio_group_reselecting_current_option_is_stable() {
    let theme = default_dark_theme();
    let choices = radio_group_choices();
    let mut selected = 2_u8;
    let mut memory = UiMemory::new();

    let press = pressed_at(4.0, 32.0);
    let mut ui = Ui::new(&press, &mut memory, &theme);
    ui.radio_group_value("modes", &mut selected, &choices);

    let release = released_at(4.0, 32.0);
    let mut ui = Ui::new(&release, &mut memory, &theme);
    let group = ui.radio_group_value("modes", &mut selected, &choices);
    let output = ui.finish_output();

    assert!(!group.changed);
    assert_eq!(selected, 2);
    assert_eq!(group.selected_index, Some(1));
    assert_eq!(group.activated, Some(2));
    assert_eq!(checked_radio_labels(&output), vec!["Second"]);
}

#[test]
fn stage2_radio_group_keyboard_activation_uses_choice_control_semantics() {
    let theme = default_dark_theme();
    let choices = radio_group_choices();
    let mut selected = 1_u8;
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("root").child("modes").child("third"));

    let input = pressed_key(Key::Space);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let group = ui.radio_group_value("modes", &mut selected, &choices);
    let output = ui.finish_output();

    assert!(group.changed);
    assert_eq!(selected, 3);
    assert_eq!(group.selected_index, Some(2));
    assert!(group.responses[2].keyboard_activated);
    assert_eq!(checked_radio_labels(&output), vec!["Third"]);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut scrub_value = 2.0;
    let mut scrub_state = TextEditState::new("2");
    let rect = stage9_rect();
    let mut memory = UiMemory::new();
    let input = pressed_at(4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.numeric_scrub_input(
        "scrub",
        rect,
        &mut scrub_value,
        &mut scrub_state,
        NumericScrubInputConfig::new(0.5),
    );
    assert_eq!(ui.finish_output().repaint, RepaintRequest::NextFrame);
    let input = dragged_at(8.0, 4.0, 4.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let scrub = ui.numeric_scrub_input(
        "scrub",
        rect,
        &mut scrub_value,
        &mut scrub_state,
        NumericScrubInputConfig::new(0.5),
    );
    let output = ui.finish_output();
    assert!(scrub.scrubbed);
    assert!((scrub_value - 4.0).abs() < f32::EPSILON);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}
