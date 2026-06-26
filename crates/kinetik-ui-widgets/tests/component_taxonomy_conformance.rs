//! Data-only component taxonomy conformance tests.

use std::collections::BTreeSet;

use kinetik_ui_core::{
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point, Rect, SemanticActionKind,
    SemanticRole, Size, UiInput, UiMemory, WidgetId, default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{
    COMPONENT_METADATA, ComponentCategory, ComponentConformanceStatus, ComponentMetadata,
    DropdownCloseReason, DropdownItem, DropdownItemId, DropdownModel, DropdownOverlay, OverlayId,
    OverlayStack, PanelId, PopoverPlacement, PropertyGridLayout, PropertyGridRow,
    PropertyGridRowState, PropertyGridRowStatus, PropertyGridStatusSeverity, RadioGroupChoice,
    SliderStep, TabStrip, Ui, classify_numeric_input_draft, component_metadata,
    components_by_category, numeric_input, slider_with_step,
};

fn entry(name: &str) -> &'static ComponentMetadata {
    component_metadata(name).unwrap_or_else(|| panic!("missing metadata for {name}"))
}

fn assert_entry(name: &str, category: ComponentCategory, status: ComponentConformanceStatus) {
    let metadata = entry(name);
    assert_eq!(metadata.category, category, "{name} category");
    assert_eq!(metadata.status, status, "{name} status");
}

fn item(raw: u64, label: &str) -> DropdownItem {
    DropdownItem::new(DropdownItemId::from_raw(raw), label)
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

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

#[test]
fn registry_contains_unique_component_names() {
    let mut names = BTreeSet::new();

    for metadata in COMPONENT_METADATA {
        assert!(names.insert(metadata.name), "duplicate {}", metadata.name);
    }
}

#[test]
fn registry_contains_unique_component_slugs() {
    let mut slugs = BTreeSet::new();

    for metadata in COMPONENT_METADATA {
        assert!(slugs.insert(metadata.slug), "duplicate {}", metadata.slug);
    }
}

#[test]
fn every_metadata_entry_has_stable_non_empty_fields() {
    for metadata in COMPONENT_METADATA {
        assert!(!metadata.name.is_empty(), "{metadata:?}");
        assert!(!metadata.slug.is_empty(), "{metadata:?}");
        assert!(!metadata.category.as_str().is_empty(), "{metadata:?}");
        assert!(!metadata.status.as_str().is_empty(), "{metadata:?}");
        assert!(
            metadata
                .slug
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '-'),
            "{metadata:?}"
        );
        assert!(!metadata.slug.starts_with('-'), "{metadata:?}");
        assert!(!metadata.slug.ends_with('-'), "{metadata:?}");
    }
}

#[test]
fn representative_components_report_honest_categories_and_statuses() {
    assert_entry(
        "Button",
        ComponentCategory::Control,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "TextField",
        ComponentCategory::TextEditing,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "Dock",
        ComponentCategory::Docking,
        ComponentConformanceStatus::Partial,
    );
    assert_entry(
        "Table",
        ComponentCategory::Collection,
        ComponentConformanceStatus::Partial,
    );
    assert_entry(
        "CommandPalette",
        ComponentCategory::Overlay,
        ComponentConformanceStatus::Partial,
    );
    assert_entry(
        "Viewport",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Partial,
    );
    assert_entry(
        "StatusBar",
        ComponentCategory::System,
        ComponentConformanceStatus::Partial,
    );
}

#[test]
fn stage9_basic_components_report_current_conformance_statuses() {
    assert_entry(
        "Label",
        ComponentCategory::Display,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "Button",
        ComponentCategory::Control,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "IconButton",
        ComponentCategory::Control,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "Checkbox",
        ComponentCategory::Input,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "RadioButton",
        ComponentCategory::Input,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "Toggle",
        ComponentCategory::Input,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "Slider",
        ComponentCategory::Input,
        ComponentConformanceStatus::Implemented,
    );
    assert_entry(
        "Panel",
        ComponentCategory::Docking,
        ComponentConformanceStatus::Partial,
    );
}

#[test]
fn stage1_basic_control_matrix_reports_complete_statuses() {
    for (name, category) in [
        ("TextField", ComponentCategory::TextEditing),
        ("MultiLineTextField", ComponentCategory::TextEditing),
        ("SearchField", ComponentCategory::TextEditing),
        ("NumericInput", ComponentCategory::Input),
        ("Button", ComponentCategory::Control),
        ("IconButton", ComponentCategory::Control),
        ("Checkbox", ComponentCategory::Input),
        ("RadioButton", ComponentCategory::Input),
        ("Toggle", ComponentCategory::Input),
        ("Slider", ComponentCategory::Input),
    ] {
        assert_entry(name, category, ComponentConformanceStatus::Implemented);
    }
}

#[test]
fn stage2_control_taxonomy_reports_honest_statuses() {
    for (name, category, status) in [
        (
            "Dropdown",
            ComponentCategory::Overlay,
            ComponentConformanceStatus::Partial,
        ),
        (
            "Slider",
            ComponentCategory::Input,
            ComponentConformanceStatus::Implemented,
        ),
        (
            "NumericInput",
            ComponentCategory::Input,
            ComponentConformanceStatus::Implemented,
        ),
        (
            "RadioButton",
            ComponentCategory::Input,
            ComponentConformanceStatus::Implemented,
        ),
        (
            "PropertyGrid",
            ComponentCategory::Inspector,
            ComponentConformanceStatus::Partial,
        ),
    ] {
        assert_entry(name, category, status);
    }
}

#[test]
fn component_taxonomy_conformance_reports_stage6_status_bar_partial() {
    assert_entry(
        "StatusBar",
        ComponentCategory::System,
        ComponentConformanceStatus::Partial,
    );
}

#[test]
fn component_taxonomy_conformance_reports_stage6_tabs_partial() {
    assert_entry(
        "Tabs",
        ComponentCategory::Docking,
        ComponentConformanceStatus::Partial,
    );

    let strip = TabStrip::from_tabs([
        kinetik_ui_widgets::FrameTab {
            panel: PanelId::from_raw(1),
            title: "Viewport".to_owned(),
            active: true,
            close_visible: true,
            draggable: true,
        },
        kinetik_ui_widgets::FrameTab {
            panel: PanelId::from_raw(2),
            title: "Inspector".to_owned(),
            active: false,
            close_visible: false,
            draggable: true,
        },
    ]);

    assert_eq!(strip.active_panel(), Some(PanelId::from_raw(1)));
    assert_eq!(
        strip
            .activation_target_by_panel(PanelId::from_raw(2))
            .map(|target| target.index),
        Some(1)
    );
}

#[test]
fn stage2_dropdown_partial_status_is_backed_by_public_model_and_lifecycle() {
    let mut model = DropdownModel::from_items([
        item(1, "Source"),
        item(2, "Composite").with_enabled(false),
        item(3, "Output"),
    ]);

    assert_eq!(model.highlight_first(), Some(DropdownItemId::from_raw(1)));
    assert_eq!(model.highlight_next(), Some(DropdownItemId::from_raw(3)));
    assert_eq!(
        model.select_highlighted(),
        Some(DropdownItemId::from_raw(3))
    );
    assert_eq!(
        model.selected_item().map(|item| item.label.as_str()),
        Some("Output")
    );
    assert!(!model.set_selected_id(DropdownItemId::from_raw(2)));

    model.replace_items([item(1, "Source")]);
    assert_eq!(model.selected_id(), None);

    let trigger = WidgetId::from_key("dropdown-trigger");
    let mut stack = OverlayStack::new();
    let mut dropdown = DropdownOverlay::anchored(
        OverlayId::from_raw(4),
        trigger,
        DropdownModel::from_items([item(1, "Source"), item(2, "Output")]),
        Rect::new(20.0, 20.0, 120.0, 24.0),
        Size::new(160.0, 72.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        kinetik_ui_widgets::OverlayDismissal::OutsideClickOrEscape,
    );

    dropdown.open_in(&mut stack);
    assert_eq!(
        stack.top().map(|entry| entry.id),
        Some(OverlayId::from_raw(4))
    );
    let closed = dropdown
        .dismiss_in(&mut stack, Some(Point::new(2.0, 2.0)), false)
        .expect("outside click closes dropdown");
    assert_eq!(closed.reason, DropdownCloseReason::OutsideClick);
    assert_eq!(closed.focus_return, trigger);

    dropdown.open_in(&mut stack);
    let selected = dropdown
        .select_and_close(DropdownItemId::from_raw(2), &mut stack)
        .expect("enabled selection closes dropdown");
    assert_eq!(
        selected.reason,
        DropdownCloseReason::Selection(DropdownItemId::from_raw(2))
    );
    assert_eq!(selected.selected_id, Some(DropdownItemId::from_raw(2)));
    assert_eq!(
        dropdown.model.selected_id(),
        Some(DropdownItemId::from_raw(2))
    );
}

#[test]
fn stage2_slider_and_numeric_input_statuses_are_backed_by_public_contracts() {
    let theme = default_dark_theme();
    let slider_id = WidgetId::from_key("stage2-slider");
    let mut memory = UiMemory::new();
    memory.focus(slider_id);
    let mut value = 0.5;
    let slider = slider_with_step(
        slider_id,
        Rect::new(0.0, 0.0, 160.0, 20.0),
        &mut value,
        0.0..=1.0,
        SliderStep::new(0.25).with_page_step(0.5),
        &pressed_key(Key::ArrowRight),
        &mut memory,
        &theme,
        false,
    );

    assert!((value - 0.75).abs() < f32::EPSILON);
    let slider_semantics = slider
        .semantics
        .iter()
        .find(|node| node.role == SemanticRole::Slider)
        .expect("slider semantics");
    assert!(
        slider_semantics
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::SetValue)
    );

    assert_eq!(classify_numeric_input_draft("42.5").value(), Some(42.5));
    assert!(!classify_numeric_input_draft("42 px").is_acceptable());
    assert!(classify_numeric_input_draft("  ").is_empty());

    let numeric_id = WidgetId::from_key("stage2-number");
    let mut numeric_memory = UiMemory::new();
    numeric_memory.focus(numeric_id);
    numeric_memory.set_text_input_owner(numeric_id);
    let mut state = TextEditState::new("12.5");
    let numeric = numeric_input(
        numeric_id,
        Rect::new(0.0, 28.0, 120.0, 24.0),
        &mut state,
        &pressed_key(Key::Enter),
        &mut numeric_memory,
        &theme,
        false,
    );

    assert_eq!(numeric.value, Some(12.5));
    assert!(numeric.valid);
    assert!(numeric.policy.commit_requested);
    assert!(!numeric.policy.revert_requested);
}

#[test]
fn stage2_radio_button_status_is_backed_by_label_target_group_contract() {
    assert_entry(
        "RadioButton",
        ComponentCategory::Input,
        ComponentConformanceStatus::Implemented,
    );

    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut selected = 99_u8;
    let choices = [
        RadioGroupChoice::new("first", Rect::new(0.0, 0.0, 20.0, 20.0), "First", 1)
            .with_label_rect(Rect::new(24.0, 0.0, 80.0, 20.0)),
        RadioGroupChoice::new("second", Rect::new(0.0, 28.0, 20.0, 20.0), "Second", 2)
            .with_label_rect(Rect::new(24.0, 28.0, 80.0, 20.0))
            .disabled(true),
        RadioGroupChoice::new("third", Rect::new(0.0, 56.0, 20.0, 20.0), "Third", 3)
            .with_label_rect(Rect::new(24.0, 56.0, 80.0, 20.0)),
    ];
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let group = ui.radio_group_value("stage2-radio", &mut selected, &choices);
    let output = ui.finish_output();

    assert_eq!(group.selected, 1);
    assert_eq!(group.selected_index, Some(0));
    assert!(group.changed);
    assert_eq!(
        output
            .semantics
            .nodes()
            .iter()
            .filter(|node| node.role == SemanticRole::RadioButton)
            .count(),
        3
    );
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::RadioButton
            && node.label.as_deref() == Some("Second")
            && node.state.disabled
    }));
}

#[test]
fn stage2_property_grid_partial_status_is_backed_by_layout_and_row_state_metadata() {
    assert_entry(
        "PropertyGrid",
        ComponentCategory::Inspector,
        ComponentConformanceStatus::Partial,
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
