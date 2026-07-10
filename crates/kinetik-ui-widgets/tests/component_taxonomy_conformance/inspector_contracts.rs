use super::{
    COMPONENT_METADATA, Color, ColorFieldConfig, ComponentCategory, ComponentConformanceStatus,
    NumericScrubInputConfig, PropertyGridAffordanceLayout, PropertyGridLayout, PropertyGridRow,
    PropertyGridRowAffordances, PropertyGridRowState, PropertyGridRowStatus,
    PropertyGridStatusSeverity, Rect, SemanticActionKind, SemanticRole, SemanticValue,
    TextEditState, Ui, UiInput, UiMemory, VectorComponentLayout, VectorScrubInputConfig, WidgetId,
    assert_close, assert_entry, component_metadata, components_by_category, default_dark_theme,
    property_grid_row_affordance_controls, property_grid_row_affordance_rects,
    property_grid_row_status_semantics, vector4_component_rects,
};

#[test]
fn stage2_property_grid_experimental_status_is_backed_by_layout_and_row_state_metadata() {
    assert_entry(
        "PropertyGrid",
        ComponentCategory::Inspector,
        ComponentConformanceStatus::Experimental,
    );

    let rows = [
        PropertyGridRow::section(kinetik_ui_widgets::ItemId::from_raw(1), "Transform")
            .with_help_text("Object transform"),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(2), "Position X", 1)
            .with_required(true)
            .with_status(PropertyGridRowStatus::warning("Outside guide range")),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(3), "Script", 0).with_state(
            PropertyGridRowState::neutral()
                .with_read_only(true)
                .with_status(PropertyGridRowStatus::severity(
                    PropertyGridStatusSeverity::Info,
                )),
        ),
    ];
    let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);
    let rects = layout.visible_row_rects(Rect::new(10.0, 20.0, 220.0, 80.0), &rows, 0.0, 0);

    assert_eq!(PropertyGridLayout::validate_rows(&rows), Ok(()));
    assert_close(layout.content_height(&rows), 64.0);
    assert_eq!(rects.len(), rows.len());
    assert_close(rects[1].label_rect.x, 22.0);
    assert_close(rects[1].value_rect.x, 120.0);
    assert!(rows[1].state.required);
    assert_eq!(
        rows[1].state.status.severity,
        PropertyGridStatusSeverity::Warning
    );
    assert!(!rows[2].is_editable());
}
#[test]
fn property_grid_experimental_status_includes_affordance_request_contracts() {
    assert_entry(
        "PropertyGrid",
        ComponentCategory::Inspector,
        ComponentConformanceStatus::Experimental,
    );

    let theme = default_dark_theme();
    let row = PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(7), "Exposure", 0)
        .with_status(PropertyGridRowStatus::warning("Preview range exceeded"))
        .with_resettable(true, false)
        .with_keyframeable(true, true);
    let rects = property_grid_row_affordance_rects(
        &row,
        Rect::new(0.0, 0.0, 120.0, 20.0),
        PropertyGridAffordanceLayout::new(18.0, 4.0),
    );
    let output = property_grid_row_affordance_controls(
        WidgetId::from_key("exposure-affordances"),
        &row,
        rects,
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
    );

    assert_eq!(
        row.state.affordances,
        PropertyGridRowAffordances::neutral()
            .with_reset(true, false)
            .with_keyframe(true, true)
    );
    assert!(row.can_request_reset());
    assert!(row.can_request_keyframe_toggle());
    assert!(rects.value_rect.width < 120.0);
    assert_eq!(output.widget.semantics.len(), 2);
    assert!(output.widget.semantics.iter().any(|node| {
        node.role == SemanticRole::IconButton
            && node.label.as_deref() == Some("Reset Exposure to default")
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
    }));
    assert!(output.widget.semantics.iter().any(|node| {
        node.label.as_deref() == Some("Toggle keyframe for Exposure") && node.state.selected
    }));
    assert_eq!(
        row.state.status.presentation(),
        PropertyGridStatusSeverity::Warning.presentation()
    );
}

#[test]
fn property_grid_status_semantics_include_warning_error_and_info_metadata() {
    assert_entry(
        "PropertyGrid",
        ComponentCategory::Inspector,
        ComponentConformanceStatus::Experimental,
    );

    let rows = [
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(1), "Mode", 0),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(2), "Hint", 0)
            .with_status(PropertyGridRowStatus::info("Inherited from parent")),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(3), "Exposure", 0)
            .with_status(PropertyGridRowStatus::warning("Preview range exceeded")),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(4), "Mass", 0)
            .with_status(PropertyGridRowStatus::error("Mass must be positive")),
    ];
    let plain_rows = [
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(1), "Mode", 0),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(2), "Hint", 0),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(3), "Exposure", 0),
        PropertyGridRow::property(kinetik_ui_widgets::ItemId::from_raw(4), "Mass", 0),
    ];
    let layout = PropertyGridLayout::new(20.0, 24.0, 90.0, 8.0, 12.0);
    let bounds = Rect::new(10.0, 20.0, 260.0, 80.0);
    let rects = layout.visible_row_rects(bounds, &rows, 0.0, 0);

    assert_eq!(rects, layout.visible_row_rects(bounds, &plain_rows, 0.0, 0));
    assert!(
        property_grid_row_status_semantics(WidgetId::from_key("mode"), &rows[0], rects[0])
            .is_none()
    );

    for (index, expected) in [
        (1, "Info: Inherited from parent"),
        (2, "Warning: Preview range exceeded"),
        (3, "Error: Mass must be positive"),
    ] {
        let node = property_grid_row_status_semantics(
            WidgetId::from_key(rows[index].label.as_str()),
            &rows[index],
            rects[index],
        )
        .expect("status semantics");

        assert_eq!(node.role, SemanticRole::Label);
        assert_eq!(node.description.as_deref(), Some(expected));
        assert_eq!(
            node.state.value,
            Some(SemanticValue::Text(expected.to_owned()))
        );
    }
}

#[test]
fn stage7_vector_and_color_statuses_are_backed_by_public_contracts() {
    let rect = Rect::new(0.0, 0.0, 220.0, 24.0);
    let layout = VectorComponentLayout::new(4.0, 10.0, 2.0, 20.0);
    let components = vector4_component_rects(rect, layout);
    assert_eq!(components[0].label, "X");
    assert_eq!(components[3].label, "W");
    assert!(
        components
            .windows(2)
            .all(|pair| pair[0].rect.max_x() <= pair[1].rect.x)
    );

    let theme = default_dark_theme();
    let mut vector_values = [0.0, 1.0, 2.0];
    let mut vector_states = [
        TextEditState::new("0"),
        TextEditState::new("1"),
        TextEditState::new("2"),
    ];
    let mut memory = UiMemory::new();
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let vector = ui.vector3_scrub_input(
        "position",
        rect,
        "Position",
        &mut vector_values,
        &mut vector_states,
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(0.1)),
    );
    let frame = ui.finish_output();
    assert_eq!(vector.components.len(), 3);
    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::TextField && node.label.as_deref() == Some("Position Z")
    }));

    let mut memory = UiMemory::new();
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let color = ui.color_field(
        "material-color",
        rect,
        "Material color",
        Color::rgba(0.2, 0.4, 0.6, 1.0),
        ColorFieldConfig::default(),
    );
    let frame = ui.finish_output();
    assert!(!color.open_requested);
    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Button
            && node.label.as_deref() == Some("Material color")
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Open)
    }));
}

#[test]
fn lookup_by_name_returns_registry_entry() {
    for metadata in COMPONENT_METADATA {
        assert_eq!(component_metadata(metadata.name), Some(metadata));
    }

    assert_eq!(component_metadata("UnknownComponent"), None);
}

#[test]
fn filtering_by_category_returns_only_matching_entries() {
    let docking = components_by_category(ComponentCategory::Docking).collect::<Vec<_>>();
    assert!(!docking.is_empty());
    assert!(
        docking
            .iter()
            .all(|metadata| metadata.category == ComponentCategory::Docking)
    );
    assert!(docking.iter().any(|metadata| metadata.name == "Dock"));

    for category in [
        ComponentCategory::Display,
        ComponentCategory::Control,
        ComponentCategory::Input,
        ComponentCategory::TextEditing,
        ComponentCategory::Collection,
        ComponentCategory::Docking,
        ComponentCategory::Overlay,
        ComponentCategory::Viewport,
        ComponentCategory::Inspector,
        ComponentCategory::System,
    ] {
        let filtered = components_by_category(category).collect::<Vec<_>>();
        assert!(
            filtered
                .iter()
                .all(|metadata| metadata.category == category),
            "{category:?}"
        );
    }
}
