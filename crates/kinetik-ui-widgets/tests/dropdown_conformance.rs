//! Windowless dropdown/select model and lifecycle conformance tests.

use kinetik_ui_core::{Point, Rect, Size, WidgetId};
use kinetik_ui_widgets::{
    DropdownCloseReason, DropdownItem, DropdownItemId, DropdownModel, DropdownOverlay,
    OverlayDismissal, OverlayId, OverlayKind, OverlayStack, PopoverPlacement,
    dropdown_visible_range,
};

fn id(raw: u64) -> DropdownItemId {
    DropdownItemId::from_raw(raw)
}

fn overlay_id(raw: u64) -> OverlayId {
    OverlayId::from_raw(raw)
}

fn item(raw: u64, label: &str) -> DropdownItem {
    DropdownItem::new(id(raw), label)
}

fn disabled_item(raw: u64, label: &str) -> DropdownItem {
    item(raw, label).with_enabled(false)
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON);
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

#[test]
fn select_trigger_presentation_uses_selected_label_or_placeholder() {
    let mut dropdown = DropdownModel::from_items([item(1, "Normal"), item(2, "Screen")]);

    assert_eq!(dropdown.trigger_label("Blend mode"), "Blend mode");
    let placeholder = dropdown.trigger_presentation("Blend mode", false, false);
    assert_eq!(placeholder.label, "Blend mode");
    assert_eq!(placeholder.selected_id, None);
    assert!(placeholder.placeholder);
    assert!(!placeholder.selected());
    assert!(placeholder.can_invoke());

    assert!(dropdown.set_selected_id(id(2)));
    let selected = dropdown.trigger_presentation("Blend mode", false, true);
    assert_eq!(selected.label, "Screen");
    assert_eq!(selected.selected_id, Some(id(2)));
    assert!(!selected.placeholder);
    assert!(selected.selected());
    assert!(!selected.disabled);
    assert!(selected.open);
    assert_eq!(dropdown.trigger_label("Blend mode"), "Screen");
}

#[test]
fn select_trigger_falls_back_when_selection_disappears_or_becomes_disabled() {
    let mut dropdown = DropdownModel::from_items([item(1, "Draft"), item(2, "Final")]);

    assert!(dropdown.set_selected_id(id(2)));
    dropdown.replace_items([item(1, "Draft")]);
    let missing = dropdown.trigger_presentation("Quality", false, false);
    assert_eq!(missing.label, "Quality");
    assert_eq!(missing.selected_id, None);
    assert!(missing.placeholder);

    assert!(dropdown.set_selected_id(id(1)));
    dropdown.replace_items([disabled_item(1, "Draft")]);
    let disabled = dropdown.trigger_presentation("Quality", false, false);
    assert_eq!(disabled.label, "Quality");
    assert_eq!(disabled.selected_id, None);
    assert!(disabled.placeholder);
}

#[test]
fn disabled_select_trigger_presentation_is_non_invokable_metadata() {
    let mut dropdown = DropdownModel::from_items([item(1, "Output")]);
    assert!(dropdown.set_selected_id(id(1)));

    let presentation = dropdown.trigger_presentation("Target", true, false);

    assert_eq!(presentation.label, "Output");
    assert_eq!(presentation.selected_id, Some(id(1)));
    assert!(presentation.selected());
    assert!(presentation.disabled);
    assert!(!presentation.open);
    assert!(!presentation.can_invoke());
}

#[test]
fn open_helper_describes_dropdown_overlay_with_trigger_identity() {
    let trigger = WidgetId::from_key("blend-mode-trigger");
    let dropdown = DropdownOverlay::anchored(
        overlay_id(10),
        trigger,
        DropdownModel::from_items([item(1, "Normal"), item(2, "Screen")]),
        Rect::new(185.0, 70.0, 10.0, 10.0),
        Size::new(40.0, 32.0),
        PopoverPlacement::Right,
        6.0,
        true,
        Rect::new(100.0, 50.0, 100.0, 90.0),
        OverlayDismissal::OutsideClickOrEscape,
    );
    let mut stack = OverlayStack::new();

    dropdown.open_in(&mut stack);

    assert_eq!(dropdown.trigger_id, trigger);
    assert_eq!(dropdown.entry.kind, OverlayKind::Dropdown);
    assert_eq!(dropdown.entry.rect, Rect::new(139.0, 70.0, 40.0, 32.0));
    assert_eq!(stack.focus_target(), Some(overlay_id(10)));
    assert_eq!(stack.top().map(|entry| entry.id), Some(overlay_id(10)));
}

#[test]
fn outside_click_and_escape_close_dropdown_with_trigger_focus_return() {
    let trigger = WidgetId::from_key("quality-trigger");
    let dropdown = DropdownOverlay::anchored(
        overlay_id(20),
        trigger,
        DropdownModel::from_items([item(1, "Draft"), item(2, "Final")]),
        Rect::new(12.0, 12.0, 80.0, 24.0),
        Size::new(120.0, 80.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 300.0, 180.0),
        OverlayDismissal::OutsideClickOrEscape,
    );
    let mut stack = OverlayStack::new();

    dropdown.open_in(&mut stack);
    let close = dropdown
        .dismiss_in(&mut stack, Some(Point::new(280.0, 170.0)), false)
        .expect("outside click closes dropdown");

    assert_eq!(close.overlay_id, overlay_id(20));
    assert_eq!(close.reason, DropdownCloseReason::OutsideClick);
    assert_eq!(close.focus_return, trigger);
    assert_eq!(close.selected_id, None);
    assert!(stack.entries().is_empty());

    dropdown.open_in(&mut stack);
    let close = dropdown
        .dismiss_in(&mut stack, None, true)
        .expect("escape closes dropdown");

    assert_eq!(close.reason, DropdownCloseReason::Escape);
    assert_eq!(close.focus_return, trigger);
    assert!(stack.entries().is_empty());
}

#[test]
fn selection_closes_dropdown_and_returns_selected_id() {
    let trigger = WidgetId::from_key("resolution-trigger");
    let mut dropdown = DropdownOverlay::anchored(
        overlay_id(30),
        trigger,
        DropdownModel::from_items([item(1, "720p"), item(2, "1080p"), disabled_item(3, "8K")]),
        Rect::new(12.0, 12.0, 80.0, 24.0),
        Size::new(120.0, 80.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 300.0, 180.0),
        OverlayDismissal::OutsideClickOrEscape,
    );
    let mut stack = OverlayStack::new();

    dropdown.open_in(&mut stack);
    assert_eq!(dropdown.select_and_close(id(3), &mut stack), None);
    assert_eq!(stack.top().map(|entry| entry.id), Some(overlay_id(30)));

    let close = dropdown
        .select_and_close(id(2), &mut stack)
        .expect("enabled selection closes dropdown");

    assert_eq!(close.reason, DropdownCloseReason::Selection(id(2)));
    assert_eq!(close.selected_id, Some(id(2)));
    assert_eq!(close.focus_return, trigger);
    assert_eq!(dropdown.model.selected_id(), Some(id(2)));
    assert!(stack.entries().is_empty());
}

#[test]
fn selection_close_failure_keeps_selection_and_highlight_unchanged() {
    let trigger = WidgetId::from_key("closed-dropdown-trigger");
    let mut dropdown = DropdownOverlay::anchored(
        overlay_id(31),
        trigger,
        DropdownModel::from_items([item(1, "Low"), item(2, "High")]),
        Rect::new(12.0, 12.0, 80.0, 24.0),
        Size::new(120.0, 80.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 300.0, 180.0),
        OverlayDismissal::OutsideClickOrEscape,
    );
    let mut stack = OverlayStack::new();

    assert!(dropdown.model.set_selected_id(id(1)));
    assert!(dropdown.model.set_highlighted_id(id(1)));

    assert_eq!(dropdown.select_and_close(id(2), &mut stack), None);
    assert_eq!(dropdown.model.selected_id(), Some(id(1)));
    assert_eq!(dropdown.model.highlighted_id(), Some(id(1)));
    assert!(stack.entries().is_empty());
}

#[test]
fn highlighted_selection_close_failure_keeps_selection_and_highlight_unchanged() {
    let trigger = WidgetId::from_key("closed-highlighted-dropdown-trigger");
    let mut dropdown = DropdownOverlay::anchored(
        overlay_id(32),
        trigger,
        DropdownModel::from_items([item(1, "Low"), item(2, "High")]),
        Rect::new(12.0, 12.0, 80.0, 24.0),
        Size::new(120.0, 80.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 300.0, 180.0),
        OverlayDismissal::OutsideClickOrEscape,
    );
    let mut stack = OverlayStack::new();

    assert!(dropdown.model.set_selected_id(id(1)));
    assert!(dropdown.model.set_highlighted_id(id(2)));

    assert_eq!(dropdown.select_highlighted_and_close(&mut stack), None);
    assert_eq!(dropdown.model.selected_id(), Some(id(1)));
    assert_eq!(dropdown.model.highlighted_id(), Some(id(2)));
    assert!(stack.entries().is_empty());
}

#[test]
fn long_menu_visible_range_clamps_invalid_short_and_overscrolled_offsets() {
    assert_eq!(dropdown_visible_range(0, 20.0, 80.0, 0.0).range(), 0..0);
    assert_eq!(
        dropdown_visible_range(10, f32::NAN, 80.0, 0.0).range(),
        0..0
    );
    assert_eq!(
        dropdown_visible_range(10, 20.0, f32::INFINITY, 0.0).range(),
        0..0
    );

    let short = dropdown_visible_range(3, 20.0, 120.0, 500.0);
    assert_f32_eq(short.scroll_offset, 0.0);
    assert_eq!(short.range(), 0..3);

    let invalid_offset = dropdown_visible_range(10, 20.0, 60.0, f32::NAN);
    assert_f32_eq(invalid_offset.scroll_offset, 0.0);
    assert_eq!(invalid_offset.range(), 0..3);

    let overscrolled = dropdown_visible_range(10, 20.0, 60.0, 500.0);
    assert_f32_eq(overscrolled.scroll_offset, 140.0);
    assert_eq!(overscrolled.range(), 7..10);
}
