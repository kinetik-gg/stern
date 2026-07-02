//! Fixed-extent collection virtualization conformance tests.

use std::collections::BTreeMap;
use std::ops::Range;

use kinetik_ui_core::Rect;
use kinetik_ui_widgets::{
    CollectionProjection, ItemId, ListLayout, Selection, TableColumn, TableColumnConstraints,
    TableLayout, TreeExpansion, TreeItem, TreeLayout, TreeModel, TreeRow, VirtualRangeRequest,
    VirtualWindow, VirtualWindowRequest, virtual_range, virtual_window,
};

fn request(
    item_count: usize,
    scroll_offset: f32,
    viewport_extent: f32,
    item_extent: f32,
    overscan: usize,
) -> VirtualWindowRequest {
    VirtualWindowRequest {
        item_count,
        scroll_offset,
        viewport_extent,
        item_extent,
        overscan,
    }
}

fn assert_window_finite(window: &VirtualWindow) {
    assert!(window.content_extent.is_finite(), "{window:?}");
    assert!(window.max_scroll_offset.is_finite(), "{window:?}");
    assert!(window.clamped_scroll_offset.is_finite(), "{window:?}");
    assert!(window.content_extent >= 0.0, "{window:?}");
    assert!(window.max_scroll_offset >= 0.0, "{window:?}");
    assert!(window.clamped_scroll_offset >= 0.0, "{window:?}");
}

fn assert_rect_finite(rect: Rect) {
    assert!(rect.x.is_finite(), "{rect:?}");
    assert!(rect.y.is_finite(), "{rect:?}");
    assert!(rect.width.is_finite(), "{rect:?}");
    assert!(rect.height.is_finite(), "{rect:?}");
    assert!(rect.width >= 0.0, "{rect:?}");
    assert!(rect.height >= 0.0, "{rect:?}");
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

fn assert_empty_finite_window(window: &VirtualWindow) {
    assert_window_finite(window);
    assert_approx(window.content_extent, 0.0);
    assert_approx(window.max_scroll_offset, 0.0);
    assert_approx(window.clamped_scroll_offset, 0.0);
    assert_eq!(window.visible_range, 0..0);
    assert_eq!(window.materialized_range, 0..0);
}

fn assert_range(range: Range<usize>, expected: Range<usize>) {
    assert_eq!(range, expected);
}

fn table_layout() -> TableLayout {
    TableLayout {
        columns: vec![
            TableColumn {
                id: ItemId::from_raw(1),
                header: "Name".to_owned(),
                width: 80.0,
            },
            TableColumn {
                id: ItemId::from_raw(2),
                header: "State".to_owned(),
                width: 40.0,
            },
        ],
        header_height: 25.0,
        row_height: 10.0,
        sort: None,
    }
}

fn tree_rows() -> Vec<TreeRow> {
    vec![
        TreeRow {
            row: 0,
            item_index: 0,
            id: ItemId::from_raw(1),
            parent: None,
            depth: 0,
            has_children: true,
            expanded: true,
        },
        TreeRow {
            row: 1,
            item_index: 1,
            id: ItemId::from_raw(2),
            parent: Some(ItemId::from_raw(1)),
            depth: 1,
            has_children: true,
            expanded: true,
        },
        TreeRow {
            row: 2,
            item_index: 2,
            id: ItemId::from_raw(3),
            parent: Some(ItemId::from_raw(2)),
            depth: 2,
            has_children: false,
            expanded: false,
        },
        TreeRow {
            row: 3,
            item_index: 3,
            id: ItemId::from_raw(4),
            parent: None,
            depth: 0,
            has_children: false,
            expanded: false,
        },
        TreeRow {
            row: 4,
            item_index: 4,
            id: ItemId::from_raw(5),
            parent: None,
            depth: 0,
            has_children: false,
            expanded: false,
        },
        TreeRow {
            row: 5,
            item_index: 5,
            id: ItemId::from_raw(6),
            parent: None,
            depth: 0,
            has_children: false,
            expanded: false,
        },
    ]
}

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn tree_model() -> TreeModel {
    TreeModel::new(vec![
        TreeItem {
            id: id(1),
            parent: None,
            has_children: false,
        },
        TreeItem {
            id: id(2),
            parent: Some(id(1)),
            has_children: false,
        },
        TreeItem {
            id: id(3),
            parent: Some(id(2)),
            has_children: false,
        },
        TreeItem {
            id: id(4),
            parent: None,
            has_children: true,
        },
    ])
}

fn table_constraints(
    entries: impl IntoIterator<Item = (ItemId, TableColumnConstraints)>,
) -> BTreeMap<ItemId, TableColumnConstraints> {
    entries.into_iter().collect()
}

#[path = "collections_virtualization_conformance/composition.rs"]
mod composition;
#[path = "collections_virtualization_conformance/table_layouts.rs"]
mod table_layouts;
#[path = "collections_virtualization_conformance/tree_and_selection.rs"]
mod tree_and_selection;
#[path = "collections_virtualization_conformance/windows_and_ranges.rs"]
mod windows_and_ranges;
