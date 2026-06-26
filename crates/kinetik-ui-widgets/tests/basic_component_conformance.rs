//! Windowless conformance tests for the Stage 9 basic component set.

use kinetik_ui_core::{
    Brush, Color, CursorShape, IconId, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect, RepaintRequest,
    Response, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue, Theme, UiInput,
    UiMemory, Vec2, WidgetId, default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{
    AssetSlotAsset, AssetSlotConfig, ColorFieldConfig, DropdownItem, DropdownItemId, DropdownModel,
    NumericScrubInputConfig, PropertyGridAffordanceLayout, PropertyGridRow, RadioGroupChoice,
    SelectFieldConfig, SliderStep, Ui, WidgetOutput, asset_slot_field, button, checkbox_with_label,
    color_field, icon_button_with_label, label, panel, property_grid_row_affordance_controls,
    property_grid_row_affordance_rects, radio_button_with_label, select_field, slider_with_label,
    toggle_with_label,
};

fn pointer_input(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pressed_at(x: f32, y: f32) -> UiInput {
    pointer_input(x, y, true, true, false)
}

fn dragged_at(x: f32, y: f32, delta_x: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            delta: Vec2::new(delta_x, 0.0),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn released_at(x: f32, y: f32) -> UiInput {
    pointer_input(x, y, false, false, true)
}

fn double_released_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(false, false, true),
            click_count: 2,
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pressed_key(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn stage9_rect() -> Rect {
    Rect::new(0.0, 0.0, 120.0, 28.0)
}

fn interactive_request(output: &kinetik_ui_widgets::WidgetOutput, cursor: CursorShape) -> bool {
    output
        .platform_requests
        .contains(&PlatformRequest::SetCursor(cursor))
}

#[derive(Clone, Copy)]
enum BasicComponentCase {
    Button,
    IconButton,
    Checkbox,
    Radio,
    Toggle,
    Slider,
}

impl BasicComponentCase {
    const fn name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::IconButton => "icon button",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Toggle => "toggle",
            Self::Slider => "slider",
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::IconButton => "icon",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Toggle => "toggle",
            Self::Slider => "slider",
        }
    }

    fn role(self) -> SemanticRole {
        match self {
            Self::Button => SemanticRole::Button,
            Self::IconButton => SemanticRole::IconButton,
            Self::Checkbox => SemanticRole::CheckBox,
            Self::Radio => SemanticRole::RadioButton,
            Self::Toggle => SemanticRole::Toggle,
            Self::Slider => SemanticRole::Slider,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Button => "Run",
            Self::IconButton => "Save",
            Self::Checkbox => "Snap",
            Self::Radio => "Mode",
            Self::Toggle => "Loop",
            Self::Slider => "Opacity",
        }
    }
}

fn component_output(
    case: BasicComponentCase,
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    match case {
        BasicComponentCase::Button => button(id, rect, "Run", input, memory, theme, disabled),
        BasicComponentCase::IconButton => icon_button_with_label(
            id,
            rect,
            IconId::from_raw(7),
            "Save",
            input,
            memory,
            theme,
            disabled,
        ),
        BasicComponentCase::Checkbox => {
            checkbox_with_label(id, rect, "Snap", false, input, memory, theme, disabled)
        }
        BasicComponentCase::Radio => {
            radio_button_with_label(id, rect, "Mode", false, input, memory, theme, disabled)
        }
        BasicComponentCase::Toggle => {
            toggle_with_label(id, rect, "Loop", false, input, memory, theme, disabled)
        }
        BasicComponentCase::Slider => {
            let mut value = 0.25;
            slider_with_label(
                id,
                rect,
                "Opacity",
                &mut value,
                0.0..=1.0,
                input,
                memory,
                theme,
                disabled,
            )
        }
    }
}

fn assert_disabled_not_focused(name: &str, output: &kinetik_ui_widgets::WidgetOutput) {
    let response = output.response.as_ref().expect(name);
    assert!(response.state.disabled, "{name}");
    assert!(!response.state.focused, "{name}");
    assert!(!response.state.active, "{name}");
    assert!(!response.state.pressed, "{name}");
    assert!(!output.semantics[0].state.focused, "{name}");
    assert!(!output.semantics[0].state.pressed, "{name}");
}

fn assert_disabled_after_enabled_press<F, G, H>(
    name: &str,
    mut enabled_press: F,
    mut disabled_held: G,
    mut fresh_disabled: H,
) where
    F: FnMut(&mut UiMemory) -> kinetik_ui_widgets::WidgetOutput,
    G: FnMut(&mut UiMemory) -> kinetik_ui_widgets::WidgetOutput,
    H: FnMut(&mut UiMemory) -> kinetik_ui_widgets::WidgetOutput,
{
    let mut memory = UiMemory::new();
    let enabled = enabled_press(&mut memory);
    let enabled_response = enabled.response.as_ref().expect(name);
    assert!(enabled_response.state.active, "{name}");
    assert!(enabled_response.state.pressed, "{name}");

    let disabled = disabled_held(&mut memory);
    let mut fresh_memory = UiMemory::new();
    let fresh = fresh_disabled(&mut fresh_memory);

    let response = disabled.response.as_ref().expect(name);
    assert!(response.state.disabled, "{name}");
    assert!(!response.state.active, "{name}");
    assert!(!response.state.pressed, "{name}");
    assert!(!response.state.focused, "{name}");
    assert!(!response.clicked, "{name}");
    assert!(!response.dragged, "{name}");
    assert!(disabled.platform_requests.is_empty(), "{name}");

    assert!(disabled.semantics[0].state.disabled, "{name}");
    assert!(!disabled.semantics[0].state.focused, "{name}");
    assert!(!disabled.semantics[0].state.pressed, "{name}");
    assert_eq!(disabled.primitives, fresh.primitives, "{name}");
    assert_eq!(disabled.semantics, fresh.semantics, "{name}");
}

fn assert_disabled_component_clears_retained_active(case: BasicComponentCase) {
    let theme = default_dark_theme();
    let rect = stage9_rect();
    let held = pointer_input(4.0, 4.0, true, false, false);
    let pressed = pressed_at(4.0, 4.0);
    let key = format!("retained-{}", case.key());

    assert_disabled_after_enabled_press(
        case.name(),
        |memory| {
            component_output(
                case,
                WidgetId::from_key(&key),
                rect,
                &pressed,
                memory,
                &theme,
                false,
            )
        },
        |memory| {
            component_output(
                case,
                WidgetId::from_key(&key),
                rect,
                &held,
                memory,
                &theme,
                true,
            )
        },
        |memory| {
            component_output(
                case,
                WidgetId::from_key(&key),
                rect,
                &held,
                memory,
                &theme,
                true,
            )
        },
    );
}

fn assert_selection_control_clicks_and_respects_disabled(case: BasicComponentCase) {
    let name = case.name();
    let theme = default_dark_theme();
    let rect = stage9_rect();
    let id = WidgetId::from_key(case.key());
    let mut memory = UiMemory::new();
    let pressed = pressed_at(4.0, 4.0);

    component_output(case, id, rect, &pressed, &mut memory, &theme, false);

    let released = released_at(4.0, 4.0);
    let output = component_output(case, id, rect, &released, &mut memory, &theme, false);
    let response = output.response.expect(name);
    assert!(response.clicked, "{name}");
    assert!(response.state.selected, "{name}");
    assert_eq!(output.semantics[0].state.checked, Some(true), "{name}");

    let disabled = component_output(
        case,
        WidgetId::from_key(format!("{}-disabled", case.key())),
        rect,
        &pressed_at(4.0, 4.0),
        &mut UiMemory::new(),
        &theme,
        true,
    );
    let response = disabled.response.expect(name);
    assert!(response.state.disabled, "{name}");
    assert!(!response.clicked, "{name}");
    assert!(!response.state.pressed, "{name}");
    assert!(!response.state.selected, "{name}");
    assert_eq!(disabled.semantics[0].state.checked, Some(false), "{name}");
    assert!(disabled.platform_requests.is_empty(), "{name}");
}

fn has_semantic_action(node: &SemanticNode, kind: &SemanticActionKind) -> bool {
    node.actions.iter().any(|action| action.kind == *kind)
}

fn radio_group_choices() -> Vec<RadioGroupChoice<u8>> {
    vec![
        RadioGroupChoice::new("first", Rect::new(0.0, 0.0, 20.0, 20.0), "First", 1),
        RadioGroupChoice::new("second", Rect::new(0.0, 28.0, 20.0, 20.0), "Second", 2),
        RadioGroupChoice::new("third", Rect::new(0.0, 56.0, 20.0, 20.0), "Third", 3),
    ]
}

fn checked_radio_labels(output: &kinetik_ui_core::FrameOutput) -> Vec<&str> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::RadioButton)
        .filter(|node| node.state.selected && node.state.checked == Some(true))
        .filter_map(|node| node.label.as_deref())
        .collect()
}

fn slider_semantic_current(node: &SemanticNode) -> f32 {
    match &node.state.value {
        Some(SemanticValue::Number { current, .. }) => *current,
        _ => panic!("expected numeric slider semantic value"),
    }
}

fn frame_slider_current(output: &kinetik_ui_core::FrameOutput) -> f32 {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::Slider)
        .map(slider_semantic_current)
        .expect("slider semantic node")
}

fn assert_enabled_basic_control_semantics(
    case: BasicComponentCase,
    node: &SemanticNode,
    response: Response,
) {
    assert_eq!(node.role, case.role(), "{}", case.name());
    assert_eq!(node.label.as_deref(), Some(case.label()), "{}", case.name());
    assert!(node.focusable, "{}", case.name());
    assert!(!node.state.disabled, "{}", case.name());
    assert!(!response.state.disabled, "{}", case.name());
    assert!(
        has_semantic_action(node, &SemanticActionKind::Focus),
        "{}",
        case.name()
    );

    match case {
        BasicComponentCase::Button | BasicComponentCase::IconButton => {
            assert!(has_semantic_action(node, &SemanticActionKind::Invoke));
            assert_eq!(node.state.checked, None, "{}", case.name());
            assert!(!node.state.selected, "{}", case.name());
            assert_eq!(node.state.value, None, "{}", case.name());
        }
        BasicComponentCase::Checkbox | BasicComponentCase::Radio | BasicComponentCase::Toggle => {
            assert!(has_semantic_action(node, &SemanticActionKind::Invoke));
            assert_eq!(node.state.checked, Some(false), "{}", case.name());
            assert!(!node.state.selected, "{}", case.name());
            assert_eq!(node.state.value, None, "{}", case.name());
        }
        BasicComponentCase::Slider => {
            assert!(has_semantic_action(node, &SemanticActionKind::Increment));
            assert!(has_semantic_action(node, &SemanticActionKind::Decrement));
            assert!(has_semantic_action(node, &SemanticActionKind::SetValue));
            assert!(matches!(
                node.state.value,
                Some(SemanticValue::Number { current, min, max })
                    if (current - 0.25).abs() < f32::EPSILON
                        && (min - 0.0).abs() < f32::EPSILON
                        && (max - 1.0).abs() < f32::EPSILON
            ));
        }
    }
}

fn assert_disabled_basic_control_semantics(
    case: BasicComponentCase,
    node: &SemanticNode,
    response: Response,
) {
    assert_eq!(node.role, case.role(), "{}", case.name());
    assert_eq!(node.label.as_deref(), Some(case.label()), "{}", case.name());
    assert!(response.state.disabled, "{}", case.name());
    assert!(node.state.disabled, "{}", case.name());
    assert!(!node.focusable, "{}", case.name());
    assert!(
        !has_semantic_action(node, &SemanticActionKind::Focus),
        "{}",
        case.name()
    );
    assert!(!node.state.focused, "{}", case.name());
}

#[test]
fn stage9_basic_components_emit_stable_primitive_categories() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let id = WidgetId::from_key("component");
    let rect = stage9_rect();
    let mut value = 0.5;

    let label = label(rect, "Project", &theme);
    assert!(matches!(label.primitives.as_slice(), [Primitive::Text(_)]));

    let button = button(id, rect, "Run", &input, &mut memory, &theme, false);
    assert!(matches!(
        button.primitives.as_slice(),
        [Primitive::Rect(_), Primitive::Text(_)]
    ));

    let icon = icon_button_with_label(
        id,
        rect,
        IconId::from_raw(7),
        "Save",
        &input,
        &mut memory,
        &theme,
        false,
    );
    assert!(matches!(icon.primitives.first(), Some(Primitive::Rect(_))));
    assert!(
        icon.primitives[1..]
            .iter()
            .any(|primitive| { matches!(primitive, Primitive::Path(_) | Primitive::Line(_)) })
    );

    let checkbox = checkbox_with_label(id, rect, "Snap", true, &input, &mut memory, &theme, false);
    assert!(matches!(
        checkbox.primitives.as_slice(),
        [Primitive::Rect(_)]
    ));

    let radio = radio_button_with_label(id, rect, "Mode", true, &input, &mut memory, &theme, false);
    assert!(matches!(radio.primitives.as_slice(), [Primitive::Rect(_)]));

    let toggle = toggle_with_label(id, rect, "Loop", true, &input, &mut memory, &theme, false);
    assert!(matches!(
        toggle.primitives.as_slice(),
        [Primitive::Rect(_), Primitive::Rect(_)]
    ));

    let slider = slider_with_label(
        id,
        rect,
        "Opacity",
        &mut value,
        0.0..=1.0,
        &input,
        &mut memory,
        &theme,
        false,
    );
    assert!(matches!(
        slider.primitives.as_slice(),
        [Primitive::Rect(_), Primitive::Rect(_)]
    ));

    let panel = panel(rect, &theme);
    assert!(matches!(panel.primitives.last(), Some(Primitive::Rect(_))));
    assert!(
        panel
            .primitives
            .iter()
            .take(panel.primitives.len().saturating_sub(1))
            .all(|primitive| matches!(primitive, Primitive::Shadow(_)))
    );
}

#[test]
fn color_field_emits_swatch_semantics_and_open_intent() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("albedo");
    let rect = Rect::new(0.0, 0.0, 180.0, 24.0);
    let color = Color::rgba(0.25, 0.5, 0.75, 1.0);
    let mut memory = UiMemory::new();

    let _ = color_field(
        id,
        rect,
        "Albedo",
        color,
        ColorFieldConfig::default(),
        &pressed_at(8.0, 8.0),
        &mut memory,
        &theme,
    );
    let output = color_field(
        id,
        rect,
        "Albedo",
        color,
        ColorFieldConfig::default(),
        &released_at(8.0, 8.0),
        &mut memory,
        &theme,
    );

    assert!(output.response.clicked);
    assert!(output.open_requested);
    assert_eq!(output.color, color);
    assert!(matches!(
        output.widget.primitives.as_slice(),
        [Primitive::Rect(_), Primitive::Rect(_), Primitive::Text(_)]
    ));
    assert!(output.widget.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.fill == Some(Brush::Solid(color))
                    && rect.rect.width > 0.0
                    && rect.rect.height > 0.0
        )
    }));

    let node = &output.widget.semantics[0];
    assert_eq!(node.role, SemanticRole::Button);
    assert_eq!(node.label.as_deref(), Some("Albedo"));
    assert!(node.focusable);
    assert!(!node.state.disabled);
    assert!(
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
    assert_eq!(
        node.state.value,
        Some(SemanticValue::Text(
            "rgba(0.250, 0.500, 0.750, 1.000)".to_owned()
        ))
    );
}

#[test]
fn property_grid_affordance_controls_report_app_owned_requests() {
    let theme = default_dark_theme();
    let row = PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(42), "Exposure", 0)
        .with_resettable(true, false)
        .with_keyframeable(true, false);
    let rects = property_grid_row_affordance_rects(
        &row,
        Rect::new(0.0, 0.0, 96.0, 20.0),
        PropertyGridAffordanceLayout::new(18.0, 4.0),
    );
    let keyframe_center = rects.keyframe_rect.expect("keyframe rect").center();
    let mut memory = UiMemory::new();

    let _ = property_grid_row_affordance_controls(
        WidgetId::from_key("exposure"),
        &row,
        rects,
        &pressed_at(keyframe_center.x, keyframe_center.y),
        &mut memory,
        &theme,
    );
    let output = property_grid_row_affordance_controls(
        WidgetId::from_key("exposure"),
        &row,
        rects,
        &released_at(keyframe_center.x, keyframe_center.y),
        &mut memory,
        &theme,
    );

    assert!(!output.reset_requested);
    assert!(output.keyframe_toggle_requested);
    assert!(output.requested_keyed);
    assert!(!row.state.affordances.keyframe.keyed);
    assert!(output.widget.semantics.iter().any(|node| {
        node.role == SemanticRole::IconButton
            && node.label.as_deref() == Some("Toggle keyframe for Exposure")
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
    }));
}

#[test]
fn read_only_property_grid_affordances_do_not_emit_mutation_requests() {
    let theme = default_dark_theme();
    let row = PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(43), "Script", 0)
        .with_read_only(true)
        .with_resettable(true, false)
        .with_keyframeable(true, false);
    let rects = property_grid_row_affordance_rects(
        &row,
        Rect::new(0.0, 0.0, 96.0, 20.0),
        PropertyGridAffordanceLayout::new(18.0, 4.0),
    );
    let reset_center = rects.reset_rect.expect("reset rect").center();
    let output = property_grid_row_affordance_controls(
        WidgetId::from_key("script"),
        &row,
        rects,
        &pressed_at(reset_center.x, reset_center.y),
        &mut UiMemory::new(),
        &theme,
    );

    assert!(!output.reset_requested);
    assert!(!output.keyframe_toggle_requested);
    assert!(
        output
            .reset_response
            .expect("reset response")
            .state
            .disabled
    );
    assert!(
        output
            .keyframe_response
            .expect("keyframe response")
            .state
            .disabled
    );
}

#[test]
fn disabled_and_read_only_color_fields_do_not_request_open() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 180.0, 24.0);
    let color = Color::rgba(1.2, f32::NAN, 0.25, 1.5);

    let disabled = color_field(
        WidgetId::from_key("disabled-color"),
        rect,
        "Disabled color",
        color,
        ColorFieldConfig::default().disabled(true),
        &pressed_at(8.0, 8.0),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(!disabled.open_requested);
    assert_eq!(disabled.color, Color::rgba(1.0, 0.0, 0.25, 1.0));
    assert!(disabled.response.state.disabled);
    assert!(!disabled.response.state.pressed);
    assert!(disabled.widget.semantics[0].state.disabled);
    assert!(!disabled.widget.semantics[0].focusable);
    assert!(
        !disabled.widget.semantics[0]
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
    assert!(disabled.widget.platform_requests.is_empty());

    let read_only = color_field(
        WidgetId::from_key("read-only-color"),
        rect,
        "Read-only color",
        Color::rgba(0.1, 0.2, 0.3, 1.0),
        ColorFieldConfig::default().read_only(true),
        &pressed_at(8.0, 8.0),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(read_only.read_only);
    assert!(!read_only.open_requested);
    assert!(read_only.response.state.disabled);
    assert!(read_only.widget.semantics[0].state.disabled);
    assert!(
        !read_only.widget.semantics[0]
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
}

#[test]
fn select_field_uses_dropdown_presentation_and_open_intent() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("material-select");
    let rect = Rect::new(0.0, 0.0, 180.0, 24.0);
    let mut model = DropdownModel::from_items([
        DropdownItem::new(DropdownItemId::from_raw(7), "Matte"),
        DropdownItem::new(DropdownItemId::from_raw(8), "Glossy"),
    ]);
    assert!(model.set_selected_id(DropdownItemId::from_raw(8)));
    let mut memory = UiMemory::new();

    let _ = select_field(
        id,
        rect,
        "Material",
        &model,
        SelectFieldConfig::new("Choose material").open(true),
        &pressed_at(8.0, 8.0),
        &mut memory,
        &theme,
    );
    let output = select_field(
        id,
        rect,
        "Material",
        &model,
        SelectFieldConfig::new("Choose material").open(true),
        &released_at(8.0, 8.0),
        &mut memory,
        &theme,
    );

    assert_eq!(output.presentation.label, "Glossy");
    assert_eq!(
        output.presentation.selected_id,
        Some(DropdownItemId::from_raw(8))
    );
    assert!(output.presentation.selected());
    assert!(output.presentation.open);
    assert!(output.open_requested);
    assert_eq!(output.widget.semantics[0].role, SemanticRole::Button);
    assert_eq!(
        output.widget.semantics[0].label.as_deref(),
        Some("Material")
    );
    assert_eq!(output.widget.semantics[0].state.expanded, Some(true));
    assert_eq!(
        output.widget.semantics[0].state.value,
        Some(SemanticValue::Text("Glossy".to_owned()))
    );
    assert!(
        output.widget.semantics[0]
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
}

#[test]
fn select_field_empty_all_disabled_and_read_only_states_are_non_invokable() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 180.0, 24.0);
    let disabled_model = DropdownModel::from_items([
        DropdownItem::new(DropdownItemId::from_raw(1), "Box").with_enabled(false),
        DropdownItem::new(DropdownItemId::from_raw(2), "Sphere").with_enabled(false),
    ]);

    for (key, model, config) in [
        (
            "empty-select",
            DropdownModel::new(),
            SelectFieldConfig::new("None"),
        ),
        (
            "all-disabled-select",
            disabled_model,
            SelectFieldConfig::new("Collider"),
        ),
        (
            "read-only-select",
            DropdownModel::from_items([DropdownItem::new(DropdownItemId::from_raw(3), "Mesh")]),
            SelectFieldConfig::new("Collider").read_only(true),
        ),
    ] {
        let output = select_field(
            WidgetId::from_key(key),
            rect,
            "Collider",
            &model,
            config,
            &released_at(8.0, 8.0),
            &mut UiMemory::new(),
            &theme,
        );

        assert!(!output.open_requested);
        assert!(output.response.state.disabled);
        assert!(output.presentation.placeholder);
        assert!(output.presentation.disabled);
        assert!(output.widget.semantics[0].state.disabled);
        assert!(!output.widget.semantics[0].focusable);
    }
}

#[test]
fn asset_slot_field_reports_empty_filled_disabled_read_only_and_drop_states() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 220.0, 24.0);
    let asset = AssetSlotAsset::new("asset://night_sky", "night_sky")
        .with_kind("texture")
        .with_icon(IconId::from_raw(3));

    let empty = asset_slot_field(
        WidgetId::from_key("empty-asset"),
        rect,
        "Texture",
        None,
        AssetSlotConfig::new("None").accepts_drop(true),
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(!empty.filled);
    assert!(!empty.pick_requested);
    assert!(!empty.open_requested);
    assert!(empty.drop_target.is_some());
    assert_eq!(
        empty.widget.semantics[0].state.value,
        Some(SemanticValue::Text("None".to_owned()))
    );

    let id = WidgetId::from_key("filled-asset");
    let mut memory = UiMemory::new();
    let _ = asset_slot_field(
        id,
        rect,
        "Texture",
        Some(&asset),
        AssetSlotConfig::default(),
        &pressed_at(8.0, 8.0),
        &mut memory,
        &theme,
    );
    let filled = asset_slot_field(
        id,
        rect,
        "Texture",
        Some(&asset),
        AssetSlotConfig::default(),
        &double_released_at(8.0, 8.0),
        &mut memory,
        &theme,
    );
    assert!(filled.filled);
    assert!(filled.open_requested);
    assert!(!filled.pick_requested);
    assert!(filled.widget.semantics[0].state.selected);
    assert!(
        filled.widget.semantics[0]
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );

    let disabled = asset_slot_field(
        WidgetId::from_key("disabled-asset"),
        rect,
        "Texture",
        Some(&asset),
        AssetSlotConfig::default().disabled(true),
        &pressed_at(8.0, 8.0),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(!disabled.pick_requested);
    assert!(disabled.response.state.disabled);
    assert!(disabled.widget.semantics[0].state.disabled);

    let read_only = asset_slot_field(
        WidgetId::from_key("read-only-asset"),
        rect,
        "Texture",
        Some(&asset),
        AssetSlotConfig::default().read_only(true),
        &pressed_at(8.0, 8.0),
        &mut UiMemory::new(),
        &theme,
    );
    assert!(read_only.read_only);
    assert!(!read_only.pick_requested);
    assert!(read_only.response.state.disabled);

    let mut drop_memory = UiMemory::new();
    drop_memory.start_drag(WidgetId::from_key("asset-source"));
    let dropped = asset_slot_field(
        WidgetId::from_key("drop-asset"),
        rect,
        "Texture",
        None,
        AssetSlotConfig::default().accepts_drop(true),
        &released_at(8.0, 8.0),
        &mut drop_memory,
        &theme,
    );
    assert!(dropped.drop_received);
    assert_eq!(
        dropped.drop_target.and_then(|drop| drop.source),
        Some(WidgetId::from_key("asset-source"))
    );
}

#[test]
fn stage9_basic_components_expose_semantic_roles_states_and_values() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut slider_value = 0.62;
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Title");
    ui.button("button", Rect::new(0.0, 24.0, 90.0, 28.0), "Run", false);
    ui.icon_button_with_label(
        "icon",
        Rect::new(0.0, 56.0, 28.0, 28.0),
        IconId::from_raw(7),
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
    let pressed = icon_button_with_label(
        icon_id,
        rect,
        IconId::from_raw(7),
        "Save",
        &pressed_at(4.0, 4.0),
        &mut icon_memory,
        &theme,
        false,
    );
    assert!(pressed.response.unwrap().state.pressed);
    assert!(interactive_request(&pressed, CursorShape::PointingHand));

    let released = icon_button_with_label(
        icon_id,
        rect,
        IconId::from_raw(7),
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
            icon_button_with_label(
                WidgetId::from_key("icon-disabled"),
                rect,
                IconId::from_raw(7),
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
    let output = icon_button_with_label(
        id,
        rect,
        IconId::from_raw(7),
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
