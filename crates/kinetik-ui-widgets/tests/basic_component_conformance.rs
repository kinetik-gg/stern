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

#[path = "basic_component_conformance/primitive_and_field_affordances.rs"]
mod primitive_and_field_affordances;
#[path = "basic_component_conformance/semantics_and_interactions.rs"]
mod semantics_and_interactions;
#[path = "basic_component_conformance/slider_and_choice_keyboard.rs"]
mod slider_and_choice_keyboard;
