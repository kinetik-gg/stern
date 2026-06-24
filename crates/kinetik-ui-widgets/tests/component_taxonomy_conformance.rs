//! Data-only component taxonomy conformance tests.

use std::collections::BTreeSet;

use kinetik_ui_widgets::{
    COMPONENT_METADATA, ComponentCategory, ComponentConformanceStatus, ComponentMetadata,
    component_metadata, components_by_category,
};

fn entry(name: &str) -> &'static ComponentMetadata {
    component_metadata(name).unwrap_or_else(|| panic!("missing metadata for {name}"))
}

fn assert_entry(name: &str, category: ComponentCategory, status: ComponentConformanceStatus) {
    let metadata = entry(name);
    assert_eq!(metadata.category, category, "{name} category");
    assert_eq!(metadata.status, status, "{name} status");
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
