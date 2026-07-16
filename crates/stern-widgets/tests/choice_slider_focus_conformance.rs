//! Focus-ring conformance for canonical choice and slider widget outputs.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use stern_core::{
    Brush, Color, ComponentState, CornerRadius, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    Point, PointerInput, Primitive, Rect, RectPrimitive, StrokeScale, Theme, ThemeColors, UiInput,
    UiMemory, WidgetId, default_dark_theme,
};
use stern_widgets::{
    RadioGroupChoice, Ui, WidgetOutput, checkbox_with_label_target, radio_button_with_label_target,
    slider, toggle_with_label_target,
};

fn sentinel_theme() -> Theme {
    let mut colors = ThemeColors::default_dark();
    colors.focus.indicator = Color::rgb8(0x12, 0x34, 0x56);
    colors.focus.separator = Color::rgb8(0xA1, 0xB2, 0xC3);
    default_dark_theme()
        .with_colors(colors)
        .with_strokes(StrokeScale::from_values(0.5, 1.5, 2.5, 3.5, 4.5))
}

fn focused_memory(id: WidgetId) -> UiMemory {
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory
}

fn rect_primitive(primitive: &Primitive) -> RectPrimitive {
    let Primitive::Rect(primitive) = primitive else {
        panic!("expected rectangle primitive");
    };
    *primitive
}

fn assert_additive_focus_pair(
    focused: &WidgetOutput,
    unfocused: &WidgetOutput,
    theme: &Theme,
    base_rect: Rect,
    radius: CornerRadius,
) {
    let expected = theme
        .focus_ring(true)
        .expect("visible focus recipe")
        .primitives(base_rect, radius);
    assert_eq!(focused.primitives[0], expected[0]);
    assert_eq!(focused.primitives[1], expected[1]);
    assert_eq!(&focused.primitives[2..], unfocused.primitives.as_slice());
    assert_eq!(
        focused.response.as_ref().expect("focused response").rect,
        unfocused
            .response
            .as_ref()
            .expect("unfocused response")
            .rect
    );
    assert_eq!(focused.semantics[0].bounds, unfocused.semantics[0].bounds);
}

#[test]
fn focused_choice_and_slider_outputs_prepend_exact_rings_without_changing_bounds_or_content() {
    let theme = sentinel_theme();
    let input = UiInput::default();

    let check_id = WidgetId::from_key("focused-check");
    let check_control = Rect::new(10.0, 20.0, 20.0, 20.0);
    let check_label = Rect::new(36.0, 20.0, 70.0, 20.0);
    let check_size = theme.checkbox(ComponentState::default()).size;
    let check_base = Rect::new(check_control.x, check_control.y, check_size, check_size);
    let mut focused = focused_memory(check_id);
    let focused_check = checkbox_with_label_target(
        check_id,
        check_control,
        check_label,
        "Checked",
        true,
        &input,
        &mut focused,
        &theme,
        false,
    );
    let mut unfocused = UiMemory::new();
    let unfocused_check = checkbox_with_label_target(
        check_id,
        check_control,
        check_label,
        "Checked",
        true,
        &input,
        &mut unfocused,
        &theme,
        false,
    );
    assert_additive_focus_pair(
        &focused_check,
        &unfocused_check,
        &theme,
        check_base,
        theme.radii.sm,
    );
    assert_eq!(
        focused_check.response.as_ref().expect("response").rect,
        check_control.union(check_label)
    );
    assert_eq!(
        focused_check.semantics[0].bounds,
        check_control.union(check_label)
    );

    let radio_id = WidgetId::from_key("focused-radio");
    let radio_control = Rect::new(10.0, 50.0, 20.0, 20.0);
    let radio_label = Rect::new(36.0, 50.0, 70.0, 20.0);
    let radio_size = theme.radio_button(ComponentState::default()).size;
    let radio_base = Rect::new(radio_control.x, radio_control.y, radio_size, radio_size);
    let mut focused = focused_memory(radio_id);
    let focused_radio = radio_button_with_label_target(
        radio_id,
        radio_control,
        radio_label,
        "Selected",
        true,
        &input,
        &mut focused,
        &theme,
        false,
    );
    let mut unfocused = UiMemory::new();
    let unfocused_radio = radio_button_with_label_target(
        radio_id,
        radio_control,
        radio_label,
        "Selected",
        true,
        &input,
        &mut unfocused,
        &theme,
        false,
    );
    assert_additive_focus_pair(
        &focused_radio,
        &unfocused_radio,
        &theme,
        radio_base,
        theme.radii.full,
    );
    assert_eq!(
        focused_radio.response.as_ref().expect("response").rect,
        radio_control.union(radio_label)
    );
    assert_eq!(
        focused_radio.semantics[0].bounds,
        radio_control.union(radio_label)
    );

    let toggle_id = WidgetId::from_key("focused-toggle");
    let toggle_rect = Rect::new(10.0, 80.0, 40.0, 20.0);
    let toggle_label = Rect::new(56.0, 80.0, 70.0, 20.0);
    let toggle_radius = CornerRadius::all(toggle_rect.height * 0.5);
    let mut focused = focused_memory(toggle_id);
    let focused_toggle = toggle_with_label_target(
        toggle_id,
        toggle_rect,
        toggle_label,
        "On",
        true,
        &input,
        &mut focused,
        &theme,
        false,
    );
    let mut unfocused = UiMemory::new();
    let unfocused_toggle = toggle_with_label_target(
        toggle_id,
        toggle_rect,
        toggle_label,
        "On",
        true,
        &input,
        &mut unfocused,
        &theme,
        false,
    );
    assert_additive_focus_pair(
        &focused_toggle,
        &unfocused_toggle,
        &theme,
        toggle_rect,
        toggle_radius,
    );

    let slider_id = WidgetId::from_key("focused-slider");
    let slider_rect = Rect::new(10.0, 110.0, 120.0, 12.0);
    let mut focused_value = 0.75;
    let mut focused = focused_memory(slider_id);
    let focused_slider = slider(
        slider_id,
        slider_rect,
        &mut focused_value,
        0.0..=1.0,
        &input,
        &mut focused,
        &theme,
        false,
    );
    let mut unfocused_value = 0.75;
    let mut unfocused = UiMemory::new();
    let unfocused_slider = slider(
        slider_id,
        slider_rect,
        &mut unfocused_value,
        0.0..=1.0,
        &input,
        &mut unfocused,
        &theme,
        false,
    );
    assert_additive_focus_pair(
        &focused_slider,
        &unfocused_slider,
        &theme,
        slider_rect,
        theme.radii.full,
    );
    assert_eq!(focused_value, unfocused_value);
}

#[test]
fn focused_hovered_selected_checkbox_preserves_focus_layers_and_selected_paint() {
    let theme = sentinel_theme();
    let id = WidgetId::from_key("focused-hovered-selected-check");
    let rect = Rect::new(10.0, 20.0, 20.0, 20.0);
    let check_size = theme.checkbox(ComponentState::default()).size;
    let base_rect = Rect::new(rect.x, rect.y, check_size, check_size);
    let mut focused_state = focused_memory(id);
    let focused = checkbox_with_label_target(
        id,
        rect,
        Rect::ZERO,
        "Selected",
        true,
        &UiInput::default(),
        &mut focused_state,
        &theme,
        false,
    );
    let hover_input = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(rect.x + 1.0, rect.y + 1.0)),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut hovered_state = focused_memory(id);
    let hovered = checkbox_with_label_target(
        id,
        rect,
        Rect::ZERO,
        "Selected",
        true,
        &hover_input,
        &mut hovered_state,
        &theme,
        false,
    );

    assert!(
        !focused
            .response
            .as_ref()
            .expect("focused response")
            .state
            .hovered
    );
    let hovered_response = hovered.response.as_ref().expect("hovered response");
    assert!(hovered_response.state.focused);
    assert!(hovered_response.state.hovered);
    assert!(hovered_response.state.selected);
    let expected = theme
        .focus_ring(true)
        .expect("visible focus recipe")
        .primitives(base_rect, theme.radii.sm);
    assert_eq!(hovered.primitives[0], expected[0]);
    assert_eq!(hovered.primitives[1], expected[1]);
    assert_eq!(hovered.primitives, focused.primitives);
    let base = rect_primitive(&hovered.primitives[2]);
    assert_eq!(base.fill, Some(Brush::Solid(theme.colors.accent.default)));
    assert_eq!(
        base.stroke.expect("neutral base border").brush,
        Brush::Solid(theme.colors.border.default)
    );
    assert_eq!(hovered.semantics[0].state.checked, Some(true));
}

#[test]
fn disabled_choice_and_slider_controls_suppress_retained_focus_rings() {
    let theme = sentinel_theme();
    let input = UiInput::default();
    let rect = Rect::new(0.0, 0.0, 40.0, 20.0);

    let check_id = WidgetId::from_key("disabled-check");
    let mut memory = focused_memory(check_id);
    let check = checkbox_with_label_target(
        check_id,
        rect,
        Rect::ZERO,
        "Check",
        true,
        &input,
        &mut memory,
        &theme,
        true,
    );
    assert_eq!(check.primitives.len(), 1);

    let radio_id = WidgetId::from_key("disabled-radio");
    let mut memory = focused_memory(radio_id);
    let radio = radio_button_with_label_target(
        radio_id,
        rect,
        Rect::ZERO,
        "Radio",
        true,
        &input,
        &mut memory,
        &theme,
        true,
    );
    assert_eq!(radio.primitives.len(), 1);

    let toggle_id = WidgetId::from_key("disabled-toggle");
    let mut memory = focused_memory(toggle_id);
    let toggle = toggle_with_label_target(
        toggle_id,
        rect,
        Rect::ZERO,
        "Toggle",
        true,
        &input,
        &mut memory,
        &theme,
        true,
    );
    assert_eq!(toggle.primitives.len(), 2);

    let slider_id = WidgetId::from_key("disabled-slider");
    let mut value = 0.5;
    let mut memory = focused_memory(slider_id);
    let slider = slider(
        slider_id,
        rect,
        &mut value,
        0.0..=1.0,
        &input,
        &mut memory,
        &theme,
        true,
    );
    assert_eq!(slider.primitives.len(), 2);
}

#[test]
fn focused_radio_group_normalization_preserves_ring_layers_and_radio_shape() {
    let theme = sentinel_theme();
    let choices = vec![
        RadioGroupChoice::new("first", Rect::new(0.0, 0.0, 20.0, 20.0), "First", 1_u8),
        RadioGroupChoice::new("second", Rect::new(0.0, 28.0, 20.0, 20.0), "Second", 2_u8),
        RadioGroupChoice::new("third", Rect::new(0.0, 56.0, 20.0, 20.0), "Third", 3_u8),
    ];
    let mut selected = 1_u8;
    let focused_id = WidgetId::from_key("root").child("modes").child("third");
    let mut memory = focused_memory(focused_id);
    let input = UiInput {
        keyboard: KeyboardInput {
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
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let group = ui.radio_group_value("modes", &mut selected, &choices);
    let output = ui.finish_output();

    assert_eq!(group.selected, 3);
    assert_eq!(group.selected_index, Some(2));
    assert_eq!(output.primitives.len(), 5);
    let radio_size = theme.radio_button(ComponentState::default()).size;
    let base_rect = Rect::new(choices[2].rect.x, choices[2].rect.y, radio_size, radio_size);
    let expected = theme
        .focus_ring(true)
        .expect("visible focus recipe")
        .primitives(base_rect, theme.radii.full);
    assert_eq!(output.primitives[2], expected[0]);
    assert_eq!(output.primitives[3], expected[1]);
    let base = rect_primitive(&output.primitives[4]);
    assert_eq!(base.rect, base_rect);
    assert_eq!(base.radius, theme.radii.full);
    assert!(base.fill.is_some());
    assert_eq!(
        base.stroke.expect("neutral base border").brush,
        Brush::Solid(theme.colors.border.default)
    );
}
