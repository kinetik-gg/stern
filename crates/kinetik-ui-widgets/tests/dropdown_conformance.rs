//! Windowless dropdown/select model conformance tests.

use kinetik_ui_widgets::{DropdownItem, DropdownItemId, DropdownModel};

fn id(raw: u64) -> DropdownItemId {
    DropdownItemId::from_raw(raw)
}

fn item(raw: u64, label: &str) -> DropdownItem {
    DropdownItem::new(id(raw), label)
}

fn disabled_item(raw: u64, label: &str) -> DropdownItem {
    item(raw, label).with_enabled(false)
}

#[test]
fn selected_by_id_survives_reorder() {
    let mut dropdown =
        DropdownModel::from_items([item(1, "First"), item(2, "Second"), item(3, "Third")]);

    assert!(dropdown.set_selected_id(id(2)));
    dropdown.replace_items([item(3, "Third"), item(1, "First"), item(2, "Second")]);

    assert_eq!(dropdown.selected_id(), Some(id(2)));
    assert_eq!(
        dropdown.selected_item().map(|item| item.label.as_str()),
        Some("Second")
    );
}

#[test]
fn arrow_down_and_arrow_up_skip_disabled_items() {
    let mut dropdown = DropdownModel::from_items([
        item(1, "First"),
        disabled_item(2, "Second"),
        item(3, "Third"),
        disabled_item(4, "Fourth"),
        item(5, "Fifth"),
    ]);

    assert_eq!(dropdown.highlight_next(), Some(id(1)));
    assert_eq!(dropdown.highlight_next(), Some(id(3)));
    assert_eq!(dropdown.highlight_next(), Some(id(5)));
    assert_eq!(dropdown.highlight_next(), Some(id(5)));

    assert_eq!(dropdown.highlight_previous(), Some(id(3)));
    assert_eq!(dropdown.highlight_previous(), Some(id(1)));
    assert_eq!(dropdown.highlight_previous(), Some(id(1)));
}

#[test]
fn home_and_end_skip_disabled_items() {
    let mut dropdown = DropdownModel::from_items([
        disabled_item(1, "First"),
        item(2, "Second"),
        disabled_item(3, "Third"),
        item(4, "Fourth"),
        disabled_item(5, "Fifth"),
    ]);

    assert_eq!(dropdown.highlight_last(), Some(id(4)));
    assert_eq!(dropdown.highlighted_id(), Some(id(4)));
    assert_eq!(dropdown.highlight_first(), Some(id(2)));
    assert_eq!(dropdown.highlighted_id(), Some(id(2)));
}

#[test]
fn enter_style_selection_uses_highlighted_enabled_item() {
    let mut dropdown = DropdownModel::from_items([
        item(1, "First"),
        disabled_item(2, "Second"),
        item(3, "Third"),
    ]);

    assert!(!dropdown.set_highlighted_id(id(2)));
    assert_eq!(dropdown.highlighted_id(), None);
    assert_eq!(dropdown.highlight_last(), Some(id(3)));
    assert_eq!(dropdown.select_highlighted(), Some(id(3)));

    assert_eq!(dropdown.selected_id(), Some(id(3)));
    assert_eq!(
        dropdown.selected_item().map(|item| item.label.as_str()),
        Some("Third")
    );
    assert!(!dropdown.set_selected_id(id(2)));
    assert_eq!(dropdown.selected_id(), Some(id(3)));
}

#[test]
fn empty_and_all_disabled_lists_have_deterministic_no_selection_state() {
    let mut empty = DropdownModel::new();

    assert_eq!(empty.items(), &[]);
    assert_eq!(empty.highlight_next(), None);
    assert_eq!(empty.highlight_previous(), None);
    assert_eq!(empty.highlight_first(), None);
    assert_eq!(empty.highlight_last(), None);
    assert_eq!(empty.select_highlighted(), None);
    assert_eq!(empty.selected_id(), None);
    assert_eq!(empty.highlighted_id(), None);
    assert!(!empty.set_selected_id(id(1)));
    assert!(!empty.set_highlighted_id(id(1)));

    let mut disabled =
        DropdownModel::from_items([disabled_item(1, "First"), disabled_item(2, "Second")]);

    assert_eq!(disabled.highlight_next(), None);
    assert_eq!(disabled.highlight_last(), None);
    assert_eq!(disabled.select_highlighted(), None);
    assert_eq!(disabled.selected_id(), None);
    assert_eq!(disabled.highlighted_id(), None);
    assert!(!disabled.set_selected_id(id(1)));
    assert!(!disabled.set_highlighted_id(id(2)));
}

#[test]
fn replacing_items_clears_now_disabled_selection_and_highlight() {
    let mut dropdown = DropdownModel::from_items([item(1, "First"), item(2, "Second")]);

    assert!(dropdown.set_selected_id(id(1)));
    assert!(dropdown.set_highlighted_id(id(2)));
    dropdown.replace_items([disabled_item(1, "First"), disabled_item(2, "Second")]);

    assert_eq!(dropdown.selected_id(), None);
    assert_eq!(dropdown.highlighted_id(), None);
    assert_eq!(dropdown.selected_item(), None);
    assert_eq!(dropdown.highlighted_item(), None);
}
