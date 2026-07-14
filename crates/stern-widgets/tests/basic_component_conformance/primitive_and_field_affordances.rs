#[allow(unused_imports)]
use stern_core::RectPrimitive;

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
    assert!(matches!(panel.primitives.as_slice(), [Primitive::Rect(_)]));
}

#[test]
fn public_passive_panel_preserves_rect_and_emits_exact_flat_recipe() {
    let theme = default_dark_theme();
    let rect = Rect::new(13.0, 17.0, 211.0, 89.0);
    let recipe = theme.panel();

    let output = panel(rect, &theme);

    assert_eq!(
        output.primitives,
        vec![Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        })]
    );
    let [Primitive::Rect(surface)] = output.primitives.as_slice() else {
        panic!("passive panel must emit exactly one rectangle");
    };
    assert_eq!(surface.rect, rect);
    assert!(output.response.is_none());
    assert!(
        output
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Shadow(_)))
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
    let row = PropertyGridRow::property(stern_widgets::ItemId::from_raw(42), "Exposure", 0)
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
    let row = PropertyGridRow::property(stern_widgets::ItemId::from_raw(43), "Script", 0)
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
